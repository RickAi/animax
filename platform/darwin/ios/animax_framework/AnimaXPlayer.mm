// Copyright 2025 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

#import <AnimaX/AnimaXPlayer.h>

#import <AnimaX/AnimaXKeyPathListCallback.h>
#import <AnimaX/AnimaXPropertyCallback.h>
#import <AnimaX/AnimaXResourceFactoryService.h>
#import <AnimaX/AnimaXSurfaceDrawable.h>
#import "AnimaXComposition+Internal.h"
#import "AnimaXContext+Internal.h"
#import "AnimaXPropertyUpdater.h"
#import "AnimaXResourceLoaderHolder.h"
#import "AnimaXSurfaceDrawable+Internal.h"
#import "AnimaXRustCoreIOS.h"

#if ANIMAX_USE_RUST_CORE

#include <algorithm>
#include <cstring>
#include <memory>
#include <string>
#include <unordered_map>
#include <vector>

@interface AnimaXRustDisplayLinkProxy : NSObject
@property(nonatomic, weak) id target;
@property(nonatomic, assign) SEL selector;
@end

@implementation AnimaXRustDisplayLinkProxy
- (void)tick:(CADisplayLink *)link {
  if (self.target && [self.target respondsToSelector:self.selector]) {
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Warc-performSelector-leaks"
    [self.target performSelector:self.selector withObject:link];
#pragma clang diagnostic pop
  } else {
    [link invalidate];
  }
}
@end

@interface AnimaXPlayer ()
@property(nonatomic) AnimaXResourceLoaderHolder *resourceLoader;
@property(nonatomic, strong, readwrite) AnimaXContext *context;
@property(nonatomic, assign, readwrite) CGFloat scale;
@property(nonatomic, strong, nullable) AnimaXSurfaceDrawable *surfaceDrawable;
@property(nonatomic, strong, nullable) CADisplayLink *displayLink;
@property(nonatomic, strong, nullable) NSBundle *assetBundle;
@end

@implementation AnimaXPlayer {
  std::shared_ptr<lynx::animax::RustPlayerBridge> _rustPlayer;
  std::unordered_map<std::string, std::string> _srcPolyfill;
  std::vector<uint8_t> _pixelScratch;
  AnimaXPropertyUpdater *_propertyUpdater;
  int _visibilityFlag;
}

- (instancetype)initWithContext:(AnimaXContext *)context {
  CGFloat scale = context.scale > 0 ? context.scale : [UIScreen mainScreen].scale;
  return [self initWithContext:context scale:scale];
}

- (instancetype)initWithContext:(AnimaXContext *)context scale:(CGFloat)scale {
  if (self = [super init]) {
    _context = context;
    _scale = scale > 0 ? scale : 1.0;
    _rustPlayer = std::make_shared<lynx::animax::RustPlayerBridge>(static_cast<float>(_scale));
    _resourceLoader = [AnimaXResourceLoaderHolder loaderForPlayer:nullptr context:_context];
    [[self.context monitorDelegate] setAnimaXPlayer:&_rustPlayer];
    [[NSNotificationCenter defaultCenter] addObserver:self
                                             selector:@selector(enterBackground)
                                                 name:UIApplicationDidEnterBackgroundNotification
                                               object:nil];
    [[NSNotificationCenter defaultCenter] addObserver:self
                                             selector:@selector(enterForeground)
                                                 name:UIApplicationWillEnterForegroundNotification
                                               object:nil];
  }
  return self;
}

- (void)dealloc {
  [[NSNotificationCenter defaultCenter] removeObserver:self];
  [self stopDisplayLink];
  [[self.context monitorDelegate] onRelease];
  if (_rustPlayer) {
    _rustPlayer->Stop();
    _rustPlayer.reset();
  }
}

- (void *)getPlayerRaw {
  return _rustPlayer.get();
}

#pragma mark - DisplayLink & Presentation

- (void)ensureDisplayLink {
  if (!_rustPlayer || !_rustPlayer->IsAnimating()) {
    return;
  }
  if (![NSThread isMainThread]) {
    dispatch_async(dispatch_get_main_queue(), ^{
      [self ensureDisplayLink];
    });
    return;
  }
  if (!self.displayLink) {
    AnimaXRustDisplayLinkProxy *proxy = [[AnimaXRustDisplayLinkProxy alloc] init];
    proxy.target = self;
    proxy.selector = @selector(onDisplayLinkTick:);
    CADisplayLink *link = [CADisplayLink displayLinkWithTarget:proxy selector:@selector(tick:)];
    [link addToRunLoop:[NSRunLoop mainRunLoop] forMode:NSRunLoopCommonModes];
    self.displayLink = link;
  }
  self.displayLink.paused = NO;
}

- (void)stopDisplayLink {
  if (self.displayLink) {
    [self.displayLink invalidate];
    self.displayLink = nil;
  }
}

- (void)onDisplayLinkTick:(CADisplayLink *)link {
  if (!_rustPlayer) {
    [self stopDisplayLink];
    return;
  }
  int64_t ts_ns = static_cast<int64_t>(link.timestamp * 1e9);
  if (_rustPlayer->Tick(ts_ns)) {
    [self presentCurrentFrame];
  }
  [self drainAndDispatchEvents];
  if (!_rustPlayer->IsAnimating()) {
    self.displayLink.paused = YES;
  }
}

- (void)presentCurrentFrame {
  if (!_rustPlayer || !self.surfaceDrawable) {
    return;
  }
  AnimaXSurfaceDrawable *drawable = self.surfaceDrawable;
  uint32_t w = static_cast<uint32_t>(std::max(1.0, static_cast<double>(drawable.size.width)));
  uint32_t h = static_cast<uint32_t>(std::max(1.0, static_cast<double>(drawable.size.height)));
  const size_t stride = static_cast<size_t>(w) * 4;
  const size_t needed = stride * static_cast<size_t>(h);
  if (_pixelScratch.size() < needed) {
    _pixelScratch.resize(needed);
  }
  if (!_rustPlayer->CopyPixelsToBuffer(_pixelScratch.data(), static_cast<uint32_t>(stride), h,
                                       ANIMAX_PIXEL_FORMAT_BGRA8888_PREMUL)) {
    return;
  }

  if (drawable.target == Layer) {
    CALayer *layer = drawable.layer;
    if (!layer) return;
    CGImageRef img = lynx::animax::RustCreateCGImageFromBGRA(_pixelScratch.data(), w, h);
    if (!img) return;
    auto apply = ^{
      [CATransaction begin];
      [CATransaction setDisableActions:YES];
      layer.contents = (__bridge id)img;
      [CATransaction commit];
      CGImageRelease(img);
    };
    if ([NSThread isMainThread]) {
      apply();
    } else {
      dispatch_async(dispatch_get_main_queue(), apply);
    }
  } else if (drawable.bufferWrapper) {
    [drawable.bufferWrapper notifyBufferUpdateWithGeneration:drawable.bufferWrapper.generation
                                                   srcPixels:_pixelScratch.data()
                                                       width:w
                                                      height:h
                                                      stride:stride];
  }
}

- (void)drainAndDispatchEvents {
  if (!_rustPlayer) return;
  std::string eventsJson = _rustPlayer->DrainEventsJson();
  if (eventsJson.size() <= 2) return;

  NSData *data = [NSData dataWithBytes:eventsJson.data() length:eventsJson.size()];
  id parsed = [NSJSONSerialization JSONObjectWithData:data options:0 error:nil];
  if (![parsed isKindOfClass:[NSArray class]]) return;

  double curFrame = _rustPlayer->GetCurrentFrame();
  double totalFrames = static_cast<double>(animax_player_get_end_frame(_rustPlayer->raw()) -
                                           animax_player_get_start_frame(_rustPlayer->raw()));

  for (id item in (NSArray *)parsed) {
    if (![item isKindOfClass:[NSDictionary class]]) continue;
    NSDictionary *dict = (NSDictionary *)item;
    NSNumber *evNum = dict[@"event"];
    if (!evNum) continue;
    auto ev = static_cast<lynx::animax::RustEvent>(evNum.unsignedCharValue);
    NSMutableDictionary *params = [NSMutableDictionary dictionary];
    params[@"current"] = @(curFrame);
    params[@"total"] = @(totalFrames);
    if (dict[@"params"] && [dict[@"params"] isKindOfClass:[NSDictionary class]]) {
      [params addEntriesFromDictionary:dict[@"params"]];
    }
    NSDictionary *copyParams = [params copy];
    __weak typeof(self) weakSelf = self;
    dispatch_async(dispatch_get_main_queue(), ^{
      __strong typeof(weakSelf) strongSelf = weakSelf;
      if (!strongSelf) return;
      [strongSelf onRustEvent:ev params:copyParams];
    });
  }
}

- (void)onRustEvent:(lynx::animax::RustEvent)event params:(NSDictionary *)params {
  if (event == lynx::animax::RustEvent::kCompositionReady) {
    [_compositionListener onCompositionReady];
  }
  if (self.context.ability) {
    void *eventPtr = (void *)(uintptr_t)event;
    [self.context.ability handleEvent:eventPtr params:params];
  }
}

#pragma mark - IAnimaXPlayer

- (void)setImageFolder:(NSString *)imageFolder {
  (void)imageFolder;
}

- (void)setSrc:(NSString *)src {
  [self setSrc:src inBundle:nil];
}

- (void)setSrc:(NSString *)src inBundle:(NSBundle *)bundle {
  if (![src length] || !_rustPlayer) {
    return;
  }
  self.assetBundle = bundle;
  [_resourceLoader setAssetBundle:bundle];
  [[self.context monitorDelegate] updateUrl:src];

  NSString *resolved = src;
  std::string key = src.UTF8String ? std::string(src.UTF8String) : "";
  auto it = _srcPolyfill.find(key);
  if (it != _srcPolyfill.end()) {
    resolved = [NSString stringWithUTF8String:it->second.c_str()];
  }

  __weak typeof(self) weakSelf = self;
  lynx::animax::RustLoadUriData(resolved, bundle, ^(NSData *data, NSError *err) {
    __strong typeof(weakSelf) self = weakSelf;
    if (!self || !self->_rustPlayer) return;
    if (err || !data.length) {
      [self onRustEvent:lynx::animax::RustEvent::kError
                 params:@{@"code" : @(-1), @"msg" : err.localizedDescription ?: @"Load failed"}];
      return;
    }
    const auto *bytes = static_cast<const uint8_t *>(data.bytes);
    const size_t len = data.length;
    if (len >= 4 && bytes[0] == 0x50 && bytes[1] == 0x4B && bytes[2] == 0x03 && bytes[3] == 0x04) {
      self->_rustPlayer->SetZip(bytes, len);
    } else {
      self->_rustPlayer->SetJson(std::string(reinterpret_cast<const char *>(bytes), len));
    }
    [self presentCurrentFrame];
    [self drainAndDispatchEvents];
    [self ensureDisplayLink];
  });
}

- (void)setComposition:(AnimaXComposition *)composition {
  if (!composition || !_rustPlayer) return;
  auto model =
      reinterpret_cast<std::shared_ptr<lynx::animax::RustCompositionModel> *>([composition internalModel]);
  if (model && *model) {
    if ((*model)->is_zip) {
      _rustPlayer->SetZip((*model)->zip_bytes.data(), (*model)->zip_bytes.size());
    } else {
      _rustPlayer->SetJson((*model)->json);
    }
    [self presentCurrentFrame];
    [self drainAndDispatchEvents];
    [self ensureDisplayLink];
  }
  [[self.context monitorDelegate] updateUrl:@"composition"];
}

- (void)setPolyfill:(NSDictionary *)polyfill {
  _srcPolyfill.clear();
  [polyfill enumerateKeysAndObjectsUsingBlock:^(id key, id obj, BOOL *stop) {
    if ([key isKindOfClass:[NSString class]] && [obj isKindOfClass:[NSString class]]) {
      const char *k = [key UTF8String];
      const char *v = [obj UTF8String];
      if (k && v) {
        self->_srcPolyfill[std::string(k)] = std::string(v);
      }
    }
  }];
}

- (void)setJson:(NSString *)json {
  [self setJson:json inBundle:nil];
}

- (void)setJson:(NSString *)json inBundle:(NSBundle *)bundle {
  if (![json length] || !_rustPlayer) return;
  self.assetBundle = bundle;
  [_resourceLoader setAssetBundle:bundle];
  _rustPlayer->SetJson(std::string([json UTF8String]));
  [[self.context monitorDelegate] updateUrl:@"json"];
  [self presentCurrentFrame];
  [self drainAndDispatchEvents];
  [self ensureDisplayLink];
}

- (void)setLoop:(BOOL)enable {
  if (_rustPlayer) _rustPlayer->SetLoop(enable ? true : false);
}

- (void)setStartFrame:(double)startFrame {
  if (_rustPlayer) _rustPlayer->SetStartFrame(static_cast<float>(startFrame));
}

- (void)setEndFrame:(double)endFrame {
  if (_rustPlayer) _rustPlayer->SetEndFrame(static_cast<float>(endFrame));
}

- (void)setAutoReverse:(BOOL)enable {
  if (_rustPlayer) _rustPlayer->SetAutoReverse(enable ? true : false);
}

- (void)setProgress:(double)progress {
  if (_rustPlayer) {
    _rustPlayer->SetProgress(static_cast<float>(progress));
    [self presentCurrentFrame];
  }
}

- (void)setLoopCount:(int)loopCount {
  if (_rustPlayer) _rustPlayer->SetLoopCount(loopCount);
}

- (void)setObjectfit:(NSString *)objectfit {
  if (![objectfit length] || !_rustPlayer) return;
  uint8_t fit = 1;  // Contain
  if ([@"fill" isEqualToString:objectfit]) {
    fit = 0;
  } else if ([@"contain" isEqualToString:objectfit]) {
    fit = 1;
  } else if ([@"cover" isEqualToString:objectfit]) {
    fit = 2;
  } else if ([@"center" isEqualToString:objectfit]) {
    fit = 3;
  } else if ([@"scale-down" isEqualToString:objectfit]) {
    fit = 4;
  }
  _rustPlayer->SetObjectFit(fit);
  _rustPlayer->Render();
  [self presentCurrentFrame];
}

- (void)setObjectPosition:(NSString *)objectPosition {
  if (![objectPosition length] || !_rustPlayer) return;
  uint8_t pos = 0;  // Center
  if ([@"top" isEqualToString:objectPosition]) pos = 1;
  else if ([@"bottom" isEqualToString:objectPosition]) pos = 2;
  else if ([@"left" isEqualToString:objectPosition]) pos = 3;
  else if ([@"right" isEqualToString:objectPosition]) pos = 4;
  else if ([@"top-left" isEqualToString:objectPosition]) pos = 5;
  else if ([@"top-right" isEqualToString:objectPosition]) pos = 6;
  else if ([@"bottom-left" isEqualToString:objectPosition]) pos = 7;
  else if ([@"bottom-right" isEqualToString:objectPosition]) pos = 8;
  _rustPlayer->SetObjectPosition(pos);
  _rustPlayer->Render();
  [self presentCurrentFrame];
}

- (void)setAutoplay:(BOOL)enable {
  if (_rustPlayer) {
    _rustPlayer->SetAutoplay(enable ? true : false);
    if (enable) [self ensureDisplayLink];
  }
}

- (void)setSpeed:(double)speed {
  if (_rustPlayer) _rustPlayer->SetSpeed(static_cast<float>(speed));
}

- (void)setFPSEventInterval:(long)interval {
  (void)interval;
}

- (void)setMaxFrameRate:(double)maxFrameRate {
  if (_rustPlayer) _rustPlayer->SetMaxFrameRate(static_cast<float>(maxFrameRate));
}

- (void)setDynamicResource:(BOOL)enable {
  (void)enable;
}

- (void)subscribeUpdateEvent:(int)frame {
  if (_rustPlayer) _rustPlayer->SubscribeUpdateEvent(frame);
}

- (void)unsubscribeUpdateEvent:(int)frame {
  if (_rustPlayer) _rustPlayer->UnsubscribeUpdateEvent(frame);
}

- (void)subscribeUpdateEvents:(NSArray *)frames subscribe:(BOOL)subscribe {
  if (!_rustPlayer) return;
  for (NSNumber *frame in frames) {
    if (subscribe) {
      _rustPlayer->SubscribeUpdateEvent(frame.intValue);
    } else {
      _rustPlayer->UnsubscribeUpdateEvent(frame.intValue);
    }
  }
}

- (void)play {
  [[self.context monitorDelegate] onPlay];
  if (_rustPlayer) {
    _rustPlayer->Play();
    [self ensureDisplayLink];
    [self drainAndDispatchEvents];
  }
}

- (void)playFrom:(double)startFrame to:(double)endFrame {
  [[self.context monitorDelegate] onPlaySegment];
  if (_rustPlayer) {
    _rustPlayer->PlaySegment(static_cast<float>(startFrame), static_cast<float>(endFrame));
    [self ensureDisplayLink];
    [self drainAndDispatchEvents];
  }
}

- (void)resume {
  [[self.context monitorDelegate] onResume];
  if (_rustPlayer) {
    _rustPlayer->Resume();
    [self ensureDisplayLink];
  }
}

- (void)stop {
  if (_rustPlayer) {
    _rustPlayer->Stop();
    self.displayLink.paused = YES;
    [self presentCurrentFrame];
    [self drainAndDispatchEvents];
  }
}

- (void)pause {
  if (_rustPlayer) {
    _rustPlayer->Pause();
    self.displayLink.paused = YES;
  }
}

- (void)seekTo:(double)frame {
  if (_rustPlayer) {
    _rustPlayer->Seek(static_cast<float>(frame));
    [self presentCurrentFrame];
  }
}

- (double)durationInMS {
  return _rustPlayer ? _rustPlayer->GetDurationMs() : 0.0;
}

- (BOOL)isAnimating {
  return _rustPlayer ? (_rustPlayer->IsAnimating() ? YES : NO) : NO;
}

- (double)progress {
  return _rustPlayer ? _rustPlayer->GetProgress() : 0.0;
}

- (double)currentFrame {
  return _rustPlayer ? _rustPlayer->GetCurrentFrame() : 0.0;
}

- (int64_t)memoryUsageBytes {
  return _rustPlayer ? static_cast<int64_t>(animax_player_get_memory_usage_bytes(_rustPlayer->raw()))
                     : 0;
}

- (void)setMuted:(BOOL)mute {
  (void)mute;
}

- (void)setEnableAudio:(BOOL)enable {
  (void)enable;
}

- (void)reload {
  if (_rustPlayer) {
    _rustPlayer->Reload();
    [self presentCurrentFrame];
    [self ensureDisplayLink];
  }
}

- (void)handleTapAtPoint:(CGPoint)point {
  if (_rustPlayer) {
    _rustPlayer->OnTap(static_cast<float>(point.x), static_cast<float>(point.y));
    [self drainAndDispatchEvents];
  }
}

- (void)onHide:(VisibilityState)state {
  if (_rustPlayer) {
    _rustPlayer->OnHide(static_cast<uint8_t>(state));
    if (!_rustPlayer->IsAnimating()) self.displayLink.paused = YES;
  }
}

- (void)onShow:(VisibilityState)state {
  if (_rustPlayer) {
    _rustPlayer->OnShow(static_cast<uint8_t>(state));
    [self ensureDisplayLink];
  }
}

- (void)enterForeground {
  [self updateVisibilityState:YES forType:VisibilityState::kBackground];
}

- (void)enterBackground {
  [self updateVisibilityState:NO forType:VisibilityState::kBackground];
}

- (BOOL)updateVisibilityFlagIfChanged:(BOOL)isVisible forType:(VisibilityState)state {
  BOOL currentIsVisible = (_visibilityFlag & state) == 0;
  if (isVisible == currentIsVisible) {
    return NO;
  }
  _visibilityFlag ^= state;
  return YES;
}

- (void)updateVisibilityState:(BOOL)visible forType:(VisibilityState)state {
  if (![self updateVisibilityFlagIfChanged:visible forType:state]) {
    return;
  }
  if (visible) {
    [self onShow:state];
  } else {
    [self onHide:state];
  }
}

- (void)adoptAnimaXSurfaceDrawable:(AnimaXSurfaceDrawable *)drawable {
  self.surfaceDrawable = drawable;
  if (!_rustPlayer || !drawable) return;
  uint32_t w = static_cast<uint32_t>(std::max(1.0, static_cast<double>(drawable.size.width)));
  uint32_t h = static_cast<uint32_t>(std::max(1.0, static_cast<double>(drawable.size.height)));
  _rustPlayer->SetSurfaceSize(w, h);
  [self presentCurrentFrame];
  [self drainAndDispatchEvents];
  [self ensureDisplayLink];
}

- (void)addAnimationEventListener:(id<AnimaXAnimationListener>)listener {
  [self.context.ability addAnimationListener:listener];
}

- (void)removeAnimationEventListener:(id<AnimaXAnimationListener>)listener {
  [self.context.ability removeAnimationListener:listener];
}

- (AnimaXContext *)getAnimaXContext {
  return _context;
}

- (void)updateLayerProperty:(AnimaXKeyPath *)keyPath
               propertyType:(LayerPropertyType)propertyType
                      value:(AnimaXValueParam *)value
                   callback:(nullable id<AnimaXPropertyCallback>)callback {
  [self ensurePropertyUpdater];
  [_propertyUpdater updateLayerProperty:keyPath
                           propertyType:propertyType
                                  value:value
                               callback:callback];
  [self presentCurrentFrame];
}

- (void)updateResourceProperty:(AnimaXKeyPath *)keyPath
                  propertyType:(ResourcePropertyType)propertyType
                         value:(AnimaXValueParam *)value
                      callback:(nullable id<AnimaXPropertyCallback>)callback {
  [self ensurePropertyUpdater];
  [_propertyUpdater updateResourceProperty:keyPath
                              propertyType:propertyType
                                     value:value
                                  callback:callback];
}

- (void)addLayerPropertyCallback:(LayerPropertyType)propertyType
                         keyPath:(AnimaXKeyPath *)keyPath
                   valueCallback:(AnimaXValueCallback *)valueCallback
                        callback:(nullable id<AnimaXPropertyCallback>)callback {
  [self ensurePropertyUpdater];
  [_propertyUpdater addLayerPropertyCallback:propertyType
                                     keyPath:keyPath
                               valueCallback:valueCallback
                                    callback:callback];
}

- (void)getKeysForKeyPath:(AnimaXKeyPath *)keyPath
                 callback:(id<AnimaXKeyPathListCallback>)callback {
  [self ensurePropertyUpdater];
  [_propertyUpdater getKeysForKeyPath:keyPath callback:callback];
}

- (void)ensurePropertyUpdater {
  if (!_propertyUpdater) {
    _propertyUpdater = [[AnimaXPropertyUpdater alloc] initWithPlayer:&_rustPlayer];
  }
}

@end

#else  // !ANIMAX_USE_RUST_CORE

#include <map>
#include <string>

#include "include/player/animax_player.h"
#include "include/player/animax_player_builder.h"
#include "src/base/log/log.h"
#include "src/base/thread/task_runner.h"
#include "src/base/util/ios/lifecycle_manager.h"
#include "src/player/ios/animax_ability_ios.h"
#include "src/player/ios/animax_surface_ios.h"

namespace lynx {
namespace animax {
static id EventParamValueToNSObject(const EventParamValue &value) {
  switch (value.type) {
    case EventParamValue::Type::kInt32:
      if (value.int_val) {
        return @(*value.int_val);
      }
      break;
    case EventParamValue::Type::kDouble:
      if (value.double_val) {
        return @(*value.double_val);
      }
      break;
    case EventParamValue::Type::kString:
      if (value.string_val) {
        return [NSString stringWithUTF8String:value.string_val->c_str()];
      }
      break;
    case EventParamValue::Type::kStringVector:
      if (value.string_vector_val) {
        NSMutableArray *array = [NSMutableArray arrayWithCapacity:value.string_vector_val->size()];
        for (const auto &str : *value.string_vector_val) {
          [array addObject:[NSString stringWithUTF8String:str.c_str()]];
        }
        return array;
      }
      break;
    default:
      break;
  }
  return nil;
}

static NSDictionary<NSString *, id> *MapToNSDictionary(const lynx::animax::EventParamMap &map) {
  NSMutableDictionary<NSString *, id> *dict =
      [NSMutableDictionary dictionaryWithCapacity:map.size()];
  for (const auto &pair : map) {
    NSString *key = [NSString stringWithUTF8String:pair.first.c_str()];
    id value = EventParamValueToNSObject(pair.second);
    if (key && value) {
      dict[key] = value;
    }
  }
  return [dict copy];
}

}  // namespace animax
}  // namespace lynx

@interface AnimaXPlayer ()
@property(nonatomic) AnimaXResourceLoaderHolder *resourceLoader;
@property(nonatomic, strong, readwrite) AnimaXContext *context;
@property(nonatomic, assign, readwrite) CGFloat scale;
@end

@implementation AnimaXPlayer {
  std::shared_ptr<lynx::animax::AnimaXPlayer> _player;
  AnimaXPropertyUpdater *_propertyUpdater;
  CGFloat _width;
  CGFloat _height;
  int _visibilityFlag;
}

- (instancetype)initWithContext:(AnimaXContext *)context {
  CGFloat scale = context.scale > 0 ? context.scale : [UIScreen mainScreen].scale;
  return [self initWithContext:context scale:scale];
}

- (instancetype)initWithContext:(AnimaXContext *)context scale:(CGFloat)scale {
  if (self = [super init]) {
    _context = context;
    _scale = scale;
    [self createPlayerWithScale:scale];
  }
  return self;
}

- (void)dealloc {
  [[self.context monitorDelegate] onRelease];

  ANIMAX_LOGI("AnimaXPlayer dealloc"
              << ", this: " << [self getPlayerRaw]);

  if (_player) {
    _player->Destroy();
    _player = nullptr;
  }
}

- (void *)getPlayerRaw {
  return _player.get();
}

- (std::shared_ptr<lynx::animax::AnimaXPlayer>)getPlayerShared {
  return _player;
}

#pragma mark - IAnimaXPlayer

- (void)setImageFolder:(NSString *)imageFolder {
  if (![imageFolder length]) {
    return;
  }
  _player->SetImageFolder([imageFolder UTF8String]);
}

- (void)setSrc:(NSString *)src {
  [self setSrc:src inBundle:nil];
}

- (void)setSrc:(NSString *)src inBundle:(NSBundle *)bundle {
  if (![src length]) {
    return;
  }
  [_resourceLoader setAssetBundle:bundle];

  _player->SetSrc([src UTF8String]);
  [[self.context monitorDelegate] updateUrl:src];
}

- (void)setComposition:(AnimaXComposition *)composition {
  if (!composition) {
    return;
  }

  auto model = reinterpret_cast<std::shared_ptr<lynx::animax::CompositionModel> *>(
      [composition internalModel]);
  if (model) {
    _player->SetComposition(*model);
  }
  [[self.context monitorDelegate] updateUrl:@"composition"];
}

- (void)setPolyfill:(NSDictionary *)polyfill {
  __block std::unordered_map<std::string, std::string> stdPolyfill;
  [polyfill
      enumerateKeysAndObjectsUsingBlock:^(id _Nonnull key, id _Nonnull obj, BOOL *_Nonnull stop) {
        if ([key isKindOfClass:[NSString class]] && [obj isKindOfClass:[NSString class]]) {
          const char *keyString = [key UTF8String];
          const char *valueString = [obj UTF8String];
          if (keyString && valueString) {
            stdPolyfill[std::string(keyString)] = std::string(valueString);
          }
        }
      }];
  _player->SetSrcPolyfill(stdPolyfill);
}

- (void)setJson:(NSString *)json {
  [self setJson:json inBundle:nil];
}

- (void)setJson:(NSString *)json inBundle:(NSBundle *)bundle {
  if (![json length]) {
    return;
  }
  [_resourceLoader setAssetBundle:bundle];

  _player->SetJson([json UTF8String]);
  [[self.context monitorDelegate] updateUrl:@"json"];
}

- (void)setLoop:(BOOL)enable {
  _player->SetLoop(enable);
}

- (void)setStartFrame:(double)startFrame {
  _player->SetStartFrame(startFrame);
}

- (void)setEndFrame:(double)endFrame {
  _player->SetEndFrame(endFrame);
}

- (void)setAutoReverse:(BOOL)enable {
  _player->SetAutoReverse(enable);
}

- (void)setProgress:(double)progress {
  _player->SetProgress(progress);
}

- (void)setLoopCount:(int)loopCount {
  _player->SetLoopCount(loopCount);
}

- (void)setObjectfit:(NSString *)objectfit {
  if (![objectfit length]) {
    return;
  }
  lynx::animax::ObjectFit objectFit = lynx::animax::ObjectFit::kContain;
  if ([@"cover" isEqualToString:objectfit]) {
    objectFit = lynx::animax::ObjectFit::kCover;
  } else if ([@"center" isEqualToString:objectfit]) {
    objectFit = lynx::animax::ObjectFit::kCenter;
  } else if ([@"fill" isEqualToString:objectfit]) {
    objectFit = lynx::animax::ObjectFit::kFill;
  } else if ([@"scale-down" isEqualToString:objectfit]) {
    objectFit = lynx::animax::ObjectFit::kScaleDown;
  }
  _player->SetObjectFit(objectFit);
}

- (void)setObjectPosition:(NSString *)objectPosition {
  if (![objectPosition length]) {
    return;
  }
  lynx::animax::ObjectPosition objectPos = lynx::animax::ObjectPosition::kCenter;
  if ([@"left" isEqualToString:objectPosition]) {
    objectPos = lynx::animax::ObjectPosition::kLeft;
  } else if ([@"right" isEqualToString:objectPosition]) {
    objectPos = lynx::animax::ObjectPosition::kRight;
  } else if ([@"top" isEqualToString:objectPosition]) {
    objectPos = lynx::animax::ObjectPosition::kTop;
  } else if ([@"bottom" isEqualToString:objectPosition]) {
    objectPos = lynx::animax::ObjectPosition::kBottom;
  } else if ([@"top-left" isEqualToString:objectPosition]) {
    objectPos = lynx::animax::ObjectPosition::kTopLeft;
  } else if ([@"top-right" isEqualToString:objectPosition]) {
    objectPos = lynx::animax::ObjectPosition::kTopRight;
  } else if ([@"bottom-left" isEqualToString:objectPosition]) {
    objectPos = lynx::animax::ObjectPosition::kBottomLeft;
  } else if ([@"bottom-right" isEqualToString:objectPosition]) {
    objectPos = lynx::animax::ObjectPosition::kBottomRight;
  }
  _player->SetObjectPosition(objectPos);
}

- (void)setAutoplay:(BOOL)enable {
  _player->SetAutoplay(enable);
}

- (void)setSpeed:(double)speed {
  _player->SetSpeed(speed);
}

- (void)setFPSEventInterval:(long)interval {
  _player->SetFpsEventInterval(interval);
}

- (void)setMaxFrameRate:(double)maxFrameRate {
  _player->SetMaxFrameRate(maxFrameRate);
}

- (void)setDynamicResource:(BOOL)enable {
  _player->SetDynamicResource(enable);
}

- (void)subscribeUpdateEvent:(int)frame {
  _player->SubscribeUpdateEvent(frame);
}

- (void)unsubscribeUpdateEvent:(int)frame {
  _player->UnsubscribeUpdateEvent(frame);
}

- (void)subscribeUpdateEvents:(NSArray *)frames subscribe:(BOOL)subscribe {
  std::unordered_set<int> frames_set;
  for (NSNumber *frame in frames) {
    frames_set.insert(frame.intValue);
  }
  _player->SubscribeUpdateEvents(std::move(frames_set), subscribe);
}

- (void)play {
  [[self.context monitorDelegate] onPlay];
  _player->Play();
}

- (void)playFrom:(double)startFrame to:(double)endFrame {
  [[self.context monitorDelegate] onPlaySegment];
  _player->PlaySegment(startFrame, endFrame);
}

- (void)resume {
  [[self.context monitorDelegate] onResume];
  _player->Resume();
}

- (void)stop {
  _player->Stop();
}

- (void)pause {
  _player->Pause();
}

- (void)seekTo:(double)frame {
  _player->Seek(frame);
}

- (double)durationInMS {
  return _player->GetDurationMs();
}

- (BOOL)isAnimating {
  return _player->IsAnimating();
}

- (double)progress {
  return _player->GetProgress();
}

- (double)currentFrame {
  return _player->GetCurrentFrame();
}

- (int64_t)memoryUsageBytes {
  return _player->GetEstimatedMemoryUsage();
}

#pragma mark - Audio
- (void)setMuted:(BOOL)mute {
  _player->SetMuted(mute);
}

- (void)setEnableAudio:(BOOL)enable {
  _player->SetEnableAudio(enable);
}

#pragma mark - Reload
- (void)reload {
  _player->Reload();
}

#pragma mark - Tap
- (void)handleTapAtPoint:(CGPoint)point {
  if (_player) {
    _player->OnTap(point.x, point.y);
  }
}

#pragma mark - Native AnimaXPlayer
- (void)createPlayerWithScale:(CGFloat)scale {
  using lynx::animax::AnimaXPlayer;
  using lynx::animax::Event;
  using lynx::animax::EventParamMap;

  lynx::animax::AnimaXPlayerBuilder builder;
  builder.SetScale(static_cast<float>(scale))
      .EnableMultiThreadAccelerate(_context.enableMultiThreadAccelerate)
      .DisableRenderInBackground(true)
      .DisablePlaybackOnAssetLoadFailure(_context.disablePlaybackOnAssetLoadFailure)
      .SetAbility(std::make_shared<lynx::animax::AnimaXAbilityIOS>(_context));

  __weak typeof(self) weakSelf = self;
  builder.AddEventListener([weakSelf](std::weak_ptr<AnimaXPlayer> /*player*/, const Event event,
                                      const EventParamMap &params) {
    NSDictionary *eventParams = lynx::animax::MapToNSDictionary(params);
    __strong typeof(weakSelf) self = weakSelf;
    if (!self) {
      return;
    }
    dispatch_async(dispatch_get_main_queue(), ^{
      __strong typeof(weakSelf) self = weakSelf;
      if (!self) {
        return;
      }
      [self onEvent:event params:eventParams];
    });
  });

  _resourceLoader = [AnimaXResourceLoaderHolder loaderForPlayer:&builder context:_context];

  _player = builder.Build();
  [[self.context monitorDelegate] setAnimaXPlayer:&_player];
  lynx::animax::LifecycleManager::Instance().AddListener(self);
}

#pragma mark - Visibility
- (void)onHide:(VisibilityState)state {
  _player->OnHide(static_cast<lynx::animax::VisibilityState>(state));
}

- (void)onShow:(VisibilityState)state {
  _player->OnShow(static_cast<lynx::animax::VisibilityState>(state));
}

- (void)enterForeground {
  [self updateVisibilityState:YES forType:VisibilityState::kBackground];
}

- (void)enterBackground {
  [self updateVisibilityState:NO forType:VisibilityState::kBackground];
}

- (void)onAppEnterForeground {
  _player->OnAppEnterForeground();
}

- (void)onAppEnterBackground {
  _player->OnAppEnterBackground();
}

- (BOOL)updateVisibilityFlagIfChanged:(BOOL)isVisible forType:(VisibilityState)state {
  BOOL currentIsVisible = (_visibilityFlag & state) == 0;
  if (isVisible == currentIsVisible) {
    return NO;
  }
  _visibilityFlag ^= state;
  return YES;
}

- (void)updateVisibilityState:(BOOL)visible forType:(VisibilityState)state {
  if (![self updateVisibilityFlagIfChanged:visible forType:state]) {
    return;
  }
  if (visible) {
    [self onShow:state];
  } else {
    [self onHide:state];
  }
}

#pragma mark - Event
- (void)onEvent:(lynx::animax::Event)event params:(NSDictionary *_Nonnull)params {
  // internal listeners
  if (event == lynx::animax::Event::kCompositionReady) {
    [_compositionListener onCompositionReady];
  }

  // external listeners
  if (self.context.ability) {
    void *eventPtr = (void *)(uintptr_t)event;
    [self.context.ability handleEvent:eventPtr params:params];
  }
}

- (void)adoptAnimaXSurfaceDrawable:(AnimaXSurfaceDrawable *)drawable {
  _player->UpdateSurface([drawable](std::unique_ptr<lynx::animax::AnimaXSurface> old_surface) {
    if (old_surface) {
      return lynx::animax::AnimaXSurfaceIOS::Reconfigure(std::move(old_surface),
                                                         {.size = drawable.size});
    } else {
      return lynx::animax::AnimaXSurfaceIOS::Make(drawable);
    }
  });
}

- (void)addAnimationEventListener:(id<AnimaXAnimationListener>)listener {
  [self.context.ability addAnimationListener:listener];
}

- (void)removeAnimationEventListener:(id<AnimaXAnimationListener>)listener {
  [self.context.ability removeAnimationListener:listener];
}

- (AnimaXContext *)getAnimaXContext {
  return _context;
}

#pragma mark - Property Updates
- (void)updateLayerProperty:(AnimaXKeyPath *)keyPath
               propertyType:(LayerPropertyType)propertyType
                      value:(AnimaXValueParam *)value
                   callback:(nullable id<AnimaXPropertyCallback>)callback {
  [self executePropertyOperation:callback
                       operation:^(AnimaXPropertyUpdater *updater) {
                         [updater updateLayerProperty:keyPath
                                         propertyType:propertyType
                                                value:value
                                             callback:callback];
                       }];
}

- (void)updateResourceProperty:(AnimaXKeyPath *)keyPath
                  propertyType:(ResourcePropertyType)propertyType
                         value:(AnimaXValueParam *)value
                      callback:(nullable id<AnimaXPropertyCallback>)callback {
  [self executePropertyOperation:callback
                       operation:^(AnimaXPropertyUpdater *updater) {
                         [updater updateResourceProperty:keyPath
                                            propertyType:propertyType
                                                   value:value
                                                callback:callback];
                       }];
}

- (void)addLayerPropertyCallback:(LayerPropertyType)propertyType
                         keyPath:(AnimaXKeyPath *)keyPath
                   valueCallback:(AnimaXValueCallback *)valueCallback
                        callback:(nullable id<AnimaXPropertyCallback>)callback {
  [self executePropertyOperation:callback
                       operation:^(AnimaXPropertyUpdater *updater) {
                         [updater addLayerPropertyCallback:propertyType
                                                   keyPath:keyPath
                                             valueCallback:valueCallback
                                                  callback:callback];
                       }];
}

- (void)executePropertyOperation:(nullable id<AnimaXPropertyCallback>)callback
                       operation:(void (^)(AnimaXPropertyUpdater *))operation {
  [self ensurePropertyUpdater];
  if (operation) {
    operation(_propertyUpdater);
  }
}

- (void)getKeysForKeyPath:(AnimaXKeyPath *)keyPath
                 callback:(id<AnimaXKeyPathListCallback>)callback {
  [self ensurePropertyUpdater];
  [_propertyUpdater getKeysForKeyPath:keyPath callback:callback];
}

- (void)ensurePropertyUpdater {
  if (!_propertyUpdater) {
    _propertyUpdater = [[AnimaXPropertyUpdater alloc] initWithPlayer:&_player];
  }
}

@end

#endif  // ANIMAX_USE_RUST_CORE
