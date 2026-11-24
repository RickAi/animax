// Copyright 2025 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

#import "AnimaXPropertyUpdater.h"

#import <Foundation/Foundation.h>
#import <dispatch/dispatch.h>

#import <AnimaX/AnimaXFrameInfo.h>
#import <AnimaX/AnimaXKeyPath.h>
#import <AnimaX/AnimaXKeyPathListCallback.h>
#import <AnimaX/AnimaXPropertyCallback.h>
#import <AnimaX/AnimaXValueCallback.h>
#import <AnimaX/AnimaXValueParam.h>
#import "AnimaXKeyPath+Internal.h"
#import "AnimaXRustCoreIOS.h"
#import "AnimaXValueCallback+Internal.h"
#import "AnimaXValueParam+Internal.h"

#if ANIMAX_USE_RUST_CORE

#include <memory>
#include <string>

static std::string AnimaXKeyPathToCsv(AnimaXKeyPath *keyPath) {
  if (!keyPath || keyPath.keys.count == 0) return "**";
  NSString *joined = [keyPath.keys componentsJoinedByString:@","];
  return joined.UTF8String ? std::string(joined.UTF8String) : "**";
}

@interface AnimaXPropertyUpdater () {
  std::weak_ptr<lynx::animax::RustPlayerBridge> _rustPlayerWeak;
}
@end

@implementation AnimaXPropertyUpdater

- (instancetype)initWithPlayer:(void *)playerPtr {
  self = [super init];
  if (self && playerPtr) {
    auto *sharedPtr = static_cast<std::shared_ptr<lynx::animax::RustPlayerBridge> *>(playerPtr);
    _rustPlayerWeak = *sharedPtr;
  }
  return self;
}

- (void)updateLayerProperty:(AnimaXKeyPath *)keyPath
               propertyType:(LayerPropertyType)propertyType
                      value:(AnimaXValueParam *)value
                   callback:(nullable id<AnimaXPropertyCallback>)callback {
  auto player = _rustPlayerWeak.lock();
  if (!player || !keyPath || !value) {
    if (callback) {
      dispatch_async(dispatch_get_main_queue(), ^{
        [callback onError:@[ @"Invalid player, keyPath, or value" ]];
      });
    }
    return;
  }

  std::string csv = AnimaXKeyPathToCsv(keyPath);
  uint16_t propType = static_cast<uint16_t>(propertyType);
  int32_t targetFrame = value.hasTargetFrame ? static_cast<int32_t>(value.targetFrame) : -1;
  bool ok = false;

  switch (value.type) {
    case AnimaXValueParamTypeString:
      ok = player->SetLayerString(csv, propType, value.stringValue ? value.stringValue.UTF8String : "",
                                  targetFrame);
      break;
    case AnimaXValueParamTypeNumber:
      ok = player->SetLayerNumber(csv, propType, value.numberValue, targetFrame);
      break;
    case AnimaXValueParamTypeBoolean:
      ok = player->SetLayerBool(csv, propType, value.booleanValue ? true : false, targetFrame);
      break;
    case AnimaXValueParamTypeCoordinate:
      ok = player->SetLayerCoord(csv, propType, value.x, value.y, value.z, targetFrame);
      break;
    case AnimaXValueParamTypeColor:
    case AnimaXValueParamTypeColorFilter:
      ok = player->SetLayerColor(csv, propType, static_cast<int32_t>(value.colorValue), targetFrame);
      break;
    case AnimaXValueParamTypeNull:
    default:
      ok = false;
      break;
  }

  if (callback) {
    dispatch_async(dispatch_get_main_queue(), ^{
      if (ok) {
        [callback onSuccess];
      } else {
        [callback onError:@[ @"Property update failed" ]];
      }
    });
  }
}

- (void)updateResourceProperty:(AnimaXKeyPath *)keyPath
                  propertyType:(ResourcePropertyType)propertyType
                         value:(AnimaXValueParam *)value
                      callback:(nullable id<AnimaXPropertyCallback>)callback {
  (void)keyPath;
  (void)propertyType;
  (void)value;
  if (callback) {
    dispatch_async(dispatch_get_main_queue(), ^{
      [callback onSuccess];
    });
  }
}

- (void)addLayerPropertyCallback:(LayerPropertyType)propertyType
                         keyPath:(AnimaXKeyPath *)keyPath
                   valueCallback:(AnimaXValueCallback *)valueCallback
                        callback:(nullable id<AnimaXPropertyCallback>)callback {
  (void)propertyType;
  (void)keyPath;
  (void)valueCallback;
  if (callback) {
    dispatch_async(dispatch_get_main_queue(), ^{
      [callback onSuccess];
    });
  }
}

- (void)getKeysForKeyPath:(AnimaXKeyPath *)keyPath
                 callback:(nullable id<AnimaXKeyPathListCallback>)callback {
  if (!callback) return;
  auto player = _rustPlayerWeak.lock();
  if (!player) {
    dispatch_async(dispatch_get_main_queue(), ^{
      [callback onCallback:@[]];
    });
    return;
  }
  std::string csv = AnimaXKeyPathToCsv(keyPath);
  char *json = animax_player_get_keys_for_key_path_json(player->raw(), csv.c_str());
  NSMutableArray<AnimaXKeyPath *> *result = [NSMutableArray array];
  if (json) {
    NSData *data = [NSData dataWithBytes:json length:std::strlen(json)];
    animax_string_free(json);
    id parsed = [NSJSONSerialization JSONObjectWithData:data options:0 error:nil];
    if ([parsed isKindOfClass:[NSArray class]]) {
      for (id item in (NSArray *)parsed) {
        if ([item isKindOfClass:[NSArray class]]) {
          AnimaXKeyPath *kp = [[AnimaXKeyPath alloc] initWithKeys:(NSArray<NSString *> *)item];
          if (kp) [result addObject:kp];
        }
      }
    }
  }
  dispatch_async(dispatch_get_main_queue(), ^{
    [callback onCallback:[result copy]];
  });
}

@end

#else  // !ANIMAX_USE_RUST_CORE

#include "include/player/animax_player.h"
#include "include/property/property_update_request.h"
#include "include/property/property_update_response.h"
#include "src/base/log/log.h"
#include "src/property/animax_property_updater.h"
#include "src/property/property_updater.h"

#include <memory>

@interface AnimaXPropertyUpdater () {
  std::weak_ptr<lynx::animax::AnimaXPlayer> _playerWeakPtr;
}

@end

@implementation AnimaXPropertyUpdater

#pragma mark - Initialization

- (instancetype)initWithPlayer:(void *)playerPtr {
  self = [super init];
  if (self) {
    if (playerPtr) {
      auto *sharedPtr = static_cast<std::shared_ptr<lynx::animax::AnimaXPlayer> *>(playerPtr);
      _playerWeakPtr = *sharedPtr;
    }
  }
  return self;
}

#pragma mark - Property Updates

- (void)updateLayerProperty:(AnimaXKeyPath *)keyPath
               propertyType:(LayerPropertyType)propertyType
                      value:(AnimaXValueParam *)value
                   callback:(nullable id<AnimaXPropertyCallback>)callback {
  if (![self validateLayerPropertyInputs:keyPath value:value callback:callback]) {
    return;
  }

  auto player = _playerWeakPtr.lock();
  if (!player) {
    [self callError:callback withMessage:@"Property updater not available"];
    return;
  }

  auto layerType = static_cast<lynx::animax::LayerPropertyType>(propertyType);
  auto nativeKeyPath = [keyPath toNative];
  auto nativeValue = [value toNative];
  auto nativeCallback = [self createNativeCallback:callback];

  if (!nativeKeyPath || !nativeValue) {
    [self callError:callback withMessage:@"Failed to convert parameters"];
    return;
  }

  auto request = lynx::animax::LayerStaticRequest::Make(
      layerType, std::move(nativeKeyPath), std::move(nativeValue), std::move(nativeCallback));

  if (request) {
    player->UpdateLayerProperty(std::move(request));
  } else {
    [self callError:callback withMessage:@"Failed to create layer request"];
  }
}

- (void)updateResourceProperty:(AnimaXKeyPath *)keyPath
                  propertyType:(ResourcePropertyType)propertyType
                         value:(AnimaXValueParam *)value
                      callback:(nullable id<AnimaXPropertyCallback>)callback {
  if (![self validateResourcePropertyInputs:keyPath value:value callback:callback]) {
    return;
  }

  auto player = _playerWeakPtr.lock();
  if (!player) {
    [self callError:callback withMessage:@"Property updater not available"];
    return;
  }

  auto resourceType = static_cast<lynx::animax::ResourcePropertyType>(propertyType);

  std::vector<std::string> keys = [keyPath toStringVector];
  if (keys.empty()) {
    [self callError:callback withMessage:@"Empty key path"];
    return;
  }

  std::string resourceId = keys[0];
  auto nativeValue = [value toNative];
  auto nativeCallback = [self createNativeCallback:callback];

  if (!nativeValue) {
    [self callError:callback withMessage:@"Failed to convert value parameter"];
    return;
  }

  auto request = lynx::animax::ResourceUpdateRequest::Make(
      resourceType, std::move(resourceId), std::move(nativeValue), std::move(nativeCallback));

  if (request) {
    player->SetResourceProperty(std::move(request));
  } else {
    [self callError:callback withMessage:@"Failed to create resource request"];
  }
}

- (void)addLayerPropertyCallback:(LayerPropertyType)propertyType
                         keyPath:(AnimaXKeyPath *)keyPath
                   valueCallback:(AnimaXValueCallback *)valueCallback
                        callback:(nullable id<AnimaXPropertyCallback>)callback {
  if (![self validateCallbackInputs:keyPath valueCallback:valueCallback callback:callback]) {
    return;
  }

  auto player = _playerWeakPtr.lock();
  if (!player) {
    [self callError:callback withMessage:@"Property updater not available"];
    return;
  }

  auto layerType = static_cast<lynx::animax::LayerPropertyType>(propertyType);
  auto nativeKeyPath = [keyPath toNative];

  if (!nativeKeyPath) {
    [self callError:callback withMessage:@"Failed to convert key path"];
    return;
  }

  auto nativeValueCallback = [AnimaXValueCallback createNativeCallback:valueCallback
                                                          propertyType:propertyType];
  if (!nativeValueCallback) {
    [self callError:callback withMessage:@"Failed to create value callback wrapper"];
    return;
  }

  auto nativeCallback = [self createNativeCallback:callback];

  auto request = lynx::animax::LayerCallbackRequest::Make(layerType, std::move(nativeKeyPath),
                                                          std::move(nativeValueCallback),
                                                          std::move(nativeCallback));

  if (request) {
    player->AddLayerPropertyCallback(std::move(request));
  } else {
    [self callError:callback withMessage:@"Failed to create callback request"];
  }
}

- (void)getKeysForKeyPath:(AnimaXKeyPath *)keyPath
                 callback:(nullable id<AnimaXKeyPathListCallback>)callback {
  auto player = _playerWeakPtr.lock();
  if (!player) {
    if (callback) {
      [callback onCallback:@[]];
    }
    return;
  }

  if (!keyPath) {
    if (callback) {
      [callback onCallback:@[]];
    }
    return;
  }

  auto nativeKeyPath = [keyPath toNative];
  if (!nativeKeyPath) {
    if (callback) {
      [callback onCallback:@[]];
    }
    return;
  }

  player->GetKeysForKeyPath(std::move(nativeKeyPath),
                            [callback](const std::vector<lynx::animax::AnimaXKeyPath> &key_paths) {
                              if (callback) {
                                NSMutableArray<AnimaXKeyPath *> *result = [NSMutableArray array];
                                for (const auto &kp : key_paths) {
                                  AnimaXKeyPath *objCKeyPath =
                                      [[AnimaXKeyPath alloc] initWithNativeKeyPath:kp];
                                  if (objCKeyPath) {
                                    [result addObject:objCKeyPath];
                                  }
                                }
                                [callback onCallback:result];
                              }
                            });
}

#pragma mark - Private Helper Methods

- (BOOL)validateLayerPropertyInputs:(AnimaXKeyPath *)keyPath
                              value:(AnimaXValueParam *)value
                           callback:(nullable id<AnimaXPropertyCallback>)callback {
  auto player = _playerWeakPtr.lock();
  if (!player) {
    [self callError:callback withMessage:@"AnimaX player is null"];
    return NO;
  }

  if (!keyPath) {
    [self callError:callback withMessage:@"Key path is null"];
    return NO;
  }

  if (!value) {
    [self callError:callback withMessage:@"Value parameter is null"];
    return NO;
  }

  return YES;
}

- (BOOL)validateResourcePropertyInputs:(AnimaXKeyPath *)keyPath
                                 value:(AnimaXValueParam *)value
                              callback:(nullable id<AnimaXPropertyCallback>)callback {
  auto player = _playerWeakPtr.lock();
  if (!player) {
    [self callError:callback withMessage:@"AnimaX player is null"];
    return NO;
  }

  if (!keyPath) {
    [self callError:callback withMessage:@"Key path is null"];
    return NO;
  }

  if (!value) {
    [self callError:callback withMessage:@"Value parameter is null"];
    return NO;
  }

  return YES;
}

- (BOOL)validateCallbackInputs:(AnimaXKeyPath *)keyPath
                 valueCallback:(AnimaXValueCallback *)valueCallback
                      callback:(nullable id<AnimaXPropertyCallback>)callback {
  auto player = _playerWeakPtr.lock();
  if (!player) {
    [self callError:callback withMessage:@"AnimaX player is null"];
    return NO;
  }

  if (!keyPath) {
    [self callError:callback withMessage:@"Key path is null"];
    return NO;
  }

  if (!valueCallback) {
    [self callError:callback withMessage:@"Value callback is null"];
    return NO;
  }

  return YES;
}

#pragma mark - Callback Helper Methods

- (std::function<void(const lynx::animax::PropertyUpdateResponse &response)>)createNativeCallback:
    (nullable id<AnimaXPropertyCallback>)callback {
  if (!callback) {
    return nullptr;
  }

  __weak typeof(self) weakSelf = self;
  return [callback, weakSelf](const lynx::animax::PropertyUpdateResponse &response) {
    __strong typeof(weakSelf) strongSelf = weakSelf;
    if (strongSelf) {
      [strongSelf handlePropertyCallback:callback response:response];
    }
  };
}

- (void)callError:(nullable id<AnimaXPropertyCallback>)callback
      withMessage:(NSString *)errorMessage {
  if (!callback) {
    return;
  }
  NSArray *errorMessages = errorMessage ? @[ errorMessage ] : @[];
  dispatch_async(dispatch_get_main_queue(), ^{
    [callback onError:errorMessages];
  });
}

- (void)handlePropertyCallback:(nullable id<AnimaXPropertyCallback>)callback
                      response:(const lynx::animax::PropertyUpdateResponse &)response {
  if (!callback) {
    return;
  }

  if (response.IsSuccess()) {
    dispatch_async(dispatch_get_main_queue(), ^{
      [callback onSuccess];
    });
  } else {
    std::vector<std::string> messages = response.GetUpdateMessages();
    NSMutableArray *errorMessages = [NSMutableArray arrayWithCapacity:messages.size()];
    for (const auto &msg : messages) {
      [errorMessages addObject:[NSString stringWithUTF8String:msg.c_str()]];
    }
    dispatch_async(dispatch_get_main_queue(), ^{
      [callback onError:errorMessages];
    });
  }
}

@end

#endif  // ANIMAX_USE_RUST_CORE
