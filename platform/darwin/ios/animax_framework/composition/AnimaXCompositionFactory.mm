// Copyright 2025 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

#import <AnimaX/AnimaXComposition.h>
#import <AnimaX/AnimaXCompositionFactory.h>
#import <AnimaX/AnimaXServiceScope.h>
#import <AnimaX/DefaultAnimaXAbility.h>
#import "AnimaXComposition+Internal.h"
#import "AnimaXResourceLoaderHolder+Internal.h"
#import "AnimaXResourceLoaderHolder.h"
#import "AnimaXRustCoreIOS.h"
#import "AnimaXThreadSafeDictionary.h"
#if !ANIMAX_USE_RUST_CORE
#include "src/resource/composition_loader/animax_composition_factory.h"
#endif

@interface AnimaXCompositionFactory ()
@property(nonatomic, strong)
    AnimaXThreadSafeDictionary<AnimaXServiceScope *, AnimaXResourceLoaderHolder *> *loaderMap;
@end

@implementation AnimaXCompositionFactory

+ (AnimaXCompositionFactory *)shared {
  static AnimaXCompositionFactory *sInstance = nil;
  static dispatch_once_t onceToken;
  dispatch_once(&onceToken, ^{
    sInstance = [[AnimaXCompositionFactory alloc] initPrivate];
  });
  return sInstance;
}

- (instancetype)initPrivate {
  self = [super init];
  if (self) {
    _loaderMap = [[AnimaXThreadSafeDictionary alloc] init];
  }
  return self;
}

- (void)ensureLoaderForContext:(AnimaXContext *)context {
  AnimaXServiceScope *scope = context.ability.getServiceScope;
  if (!self.loaderMap[scope]) {
    self.loaderMap[scope] = [AnimaXResourceLoaderHolder loaderForCompositionFactory:context];
  }
}

#if !ANIMAX_USE_RUST_CORE
- (void)handleCompositionModel:(std::shared_ptr<lynx::animax::CompositionModel>)model
                         error:(const std::string &)errorMessage
                    completion:(AnimaXCompositionCompletionBlock)completion {
  if (!completion) {
    return;
  }

  if (!model || !errorMessage.empty()) {
    NSError *error = nil;
    if (!errorMessage.empty()) {
      error = [NSError errorWithDomain:@"AnimaXCompositionErrorDomain"
                                  code:-1
                              userInfo:@{NSLocalizedDescriptionKey : @(errorMessage.c_str())}];
    }
    completion(nil, error);
    return;
  }

  AnimaXComposition *composition = [[AnimaXComposition alloc] initWithSharedModel:&model];
  completion(composition, nil);
}
#endif

- (void)loadJson:(NSString *)json
     withContext:(AnimaXContext *)context
      completion:(AnimaXCompositionCompletionBlock)completion {
  [self ensureLoaderForContext:context];
#if ANIMAX_USE_RUST_CORE
  if (!completion) return;
  std::string s = json ? std::string(json.UTF8String) : "";
  lynx::animax::RustPlayerBridge probe(static_cast<float>(context.scale > 0 ? context.scale : 1.0));
  if (probe.SetJson(s)) {
    auto model = std::make_shared<lynx::animax::RustCompositionModel>();
    model->json = std::move(s);
    model->start_frame = animax_player_get_start_frame(probe.raw());
    model->end_frame = animax_player_get_end_frame(probe.raw());
    AnimaXComposition *comp = [[AnimaXComposition alloc] initWithSharedModel:&model];
    completion(comp, nil);
  } else {
    NSError *err = [NSError errorWithDomain:@"AnimaXCompositionErrorDomain"
                                       code:-1
                                   userInfo:@{NSLocalizedDescriptionKey : @"Invalid Lottie JSON"}];
    completion(nil, err);
  }
#else
  AnimaXServiceScope *scope = context.ability.getServiceScope;
  __weak typeof(self) weakSelf = self;
  lynx::animax::AnimaXCompositionFactory::Instance().LoadJson(
      scope.description.UTF8String, json.UTF8String, context.scale,
      [weakSelf, completion](std::shared_ptr<lynx::animax::CompositionModel> model,
                             const std::string &error) {
        [weakSelf handleCompositionModel:std::move(model) error:error completion:completion];
      });
#endif
}

- (void)loadJson:(NSString *)json completion:(AnimaXCompositionCompletionBlock)completion {
  AnimaXContext *context =
      [[AnimaXContext alloc] initWithAbility:[[DefaultAnimaXAbility alloc] init]];
  [self loadJson:json withContext:context completion:completion];
}

- (void)loadUri:(NSString *)uri
    withContext:(AnimaXContext *)context
     completion:(AnimaXCompositionCompletionBlock)completion {
  [self ensureLoaderForContext:context];
#if ANIMAX_USE_RUST_CORE
  float scale = static_cast<float>(context.scale > 0 ? context.scale : 1.0);
  lynx::animax::RustLoadUriData(uri, nil, ^(NSData *data, NSError *err) {
    if (!completion) return;
    if (err || !data.length) {
      completion(nil, err ?: [NSError errorWithDomain:@"AnimaXCompositionErrorDomain"
                                                 code:-1
                                             userInfo:@{NSLocalizedDescriptionKey : @"Empty URI data"}]);
      return;
    }
    const auto *bytes = static_cast<const uint8_t *>(data.bytes);
    const size_t len = data.length;
    auto model = std::make_shared<lynx::animax::RustCompositionModel>();
    lynx::animax::RustPlayerBridge probe(scale);
    bool ok = false;
    if (len >= 4 && bytes[0] == 0x50 && bytes[1] == 0x4B && bytes[2] == 0x03 && bytes[3] == 0x04) {
      model->is_zip = true;
      model->zip_bytes.assign(bytes, bytes + len);
      ok = probe.SetZip(bytes, len);
    } else {
      model->json.assign(reinterpret_cast<const char *>(bytes), len);
      ok = probe.SetJson(model->json);
    }
    if (ok) {
      model->start_frame = animax_player_get_start_frame(probe.raw());
      model->end_frame = animax_player_get_end_frame(probe.raw());
      AnimaXComposition *comp = [[AnimaXComposition alloc] initWithSharedModel:&model];
      completion(comp, nil);
    } else {
      completion(nil, [NSError errorWithDomain:@"AnimaXCompositionErrorDomain"
                                          code:-1
                                      userInfo:@{NSLocalizedDescriptionKey : @"Failed to parse composition"}]);
    }
  });
#else
  AnimaXServiceScope *scope = context.ability.getServiceScope;
  __weak typeof(self) weakSelf = self;
  lynx::animax::AnimaXCompositionFactory::Instance().LoadUri(
      scope.description.UTF8String, uri.UTF8String, context.scale,
      [weakSelf, completion](std::shared_ptr<lynx::animax::CompositionModel> model,
                             const std::string &error) {
        [weakSelf handleCompositionModel:std::move(model) error:error completion:completion];
      });
#endif
}

- (void)loadUri:(NSString *)uri completion:(AnimaXCompositionCompletionBlock)completion {
  AnimaXContext *context =
      [[AnimaXContext alloc] initWithAbility:[[DefaultAnimaXAbility alloc] init]];
  [self loadUri:uri withContext:context completion:completion];
}

- (void)releaseAll {
  [_loaderMap removeAllObjects];
#if !ANIMAX_USE_RUST_CORE
  lynx::animax::AnimaXCompositionFactory::Instance().Release();
#endif
}

@end
