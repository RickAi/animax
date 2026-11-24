// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

#pragma once

#ifndef ANIMAX_USE_RUST_CORE
#define ANIMAX_USE_RUST_CORE 1
#endif

#if ANIMAX_USE_RUST_CORE

#import <CoreGraphics/CoreGraphics.h>
#import <CoreVideo/CoreVideo.h>
#import <Foundation/Foundation.h>
#import <QuartzCore/QuartzCore.h>
#import <UIKit/UIKit.h>

#include <cstdint>
#include <cstring>
#include <memory>
#include <sstream>
#include <string>
#include <vector>

#include "include/animax_rust_ffi.h"
#include "src/rust_bridge/animax_rust_bridge.h"

#ifndef ANIMAX_LOGI
#define ANIMAX_LOGI(msg)                                             \
  do {                                                               \
    std::ostringstream _animax_oss;                                  \
    _animax_oss << msg;                                              \
    NSLog(@"[AnimaXRust][I] %s", _animax_oss.str().c_str());         \
  } while (0);
#endif

#ifndef ANIMAX_LOGW
#define ANIMAX_LOGW(msg)                                             \
  do {                                                               \
    std::ostringstream _animax_oss;                                  \
    _animax_oss << msg;                                              \
    NSLog(@"[AnimaXRust][W] %s", _animax_oss.str().c_str());         \
  } while (0);
#endif

#ifndef ANIMAX_LOGE
#define ANIMAX_LOGE(msg)                                             \
  do {                                                               \
    std::ostringstream _animax_oss;                                  \
    _animax_oss << msg;                                              \
    NSLog(@"[AnimaXRust][E] %s", _animax_oss.str().c_str());         \
  } while (0);
#endif

#ifndef DCHECK
#define DCHECK(cond) ((void)(cond))
#endif

namespace lynx {
namespace animax {

enum class RustEvent : uint8_t {
  kCompletion = 0,
  kStart = 1,
  kRepeat = 2,
  kCancel = 3,
  kReady = 4,
  kUpdate = 5,
  kError = 6,
  kFps = 7,
  kCompositionReady = 8,
  kTapLayer = 9,
  kFirstFrame = 10,
  kWarning = 11,
};

struct RustCompositionModel {
  std::string json;
  std::vector<uint8_t> zip_bytes;
  bool is_zip = false;
  double start_frame = 0.0;
  double end_frame = 0.0;
};

static inline void RustCopyBuffer(const uint8_t *src, uint8_t *dst, size_t height,
                                  size_t src_stride, size_t dst_stride) {
  if (!src || !dst || height == 0) return;
  if (src_stride == dst_stride) {
    std::memcpy(dst, src, height * src_stride);
    return;
  }
  const size_t row_bytes = src_stride < dst_stride ? src_stride : dst_stride;
  for (size_t y = 0; y < height; ++y) {
    std::memcpy(dst + y * dst_stride, src + y * src_stride, row_bytes);
  }
}

static inline CAMetalLayer *RustCreateDisplayLayer(CGRect frame, CGFloat scale) {
  CAMetalLayer *layer = [CAMetalLayer layer];
  layer.frame = CGRectMake(0, 0, frame.size.width, frame.size.height);
  layer.contentsScale = scale > 0 ? scale : [UIScreen mainScreen].scale;
  layer.opaque = NO;
  layer.backgroundColor = [UIColor clearColor].CGColor;
  return layer;
}

static inline void RustResizeCALayer(CALayer *layer, CGSize size) {
  if (!layer) return;
  [CATransaction begin];
  [CATransaction setDisableActions:YES];
  layer.frame = CGRectMake(0, 0, size.width, size.height);
  [CATransaction commit];
}

static inline BOOL RustIsApplicationActive(void) {
  if ([NSThread isMainThread]) {
    return [UIApplication sharedApplication].applicationState != UIApplicationStateBackground;
  }
  return YES;
}

static inline void RustLoadUriData(
    NSString *uri, NSBundle *bundle,
    void (^completion)(NSData *_Nullable data, NSError *_Nullable error)) {
  if (!uri.length) {
    if (completion) {
      completion(nil, [NSError errorWithDomain:@"AnimaXRust"
                                          code:-1
                                      userInfo:@{NSLocalizedDescriptionKey : @"Empty URI"}]);
    }
    return;
  }

  // data: URI
  if ([uri hasPrefix:@"data:"]) {
    NSRange comma = [uri rangeOfString:@","];
    if (comma.location != NSNotFound) {
      NSString *payload = [uri substringFromIndex:comma.location + 1];
      NSData *decoded =
          [[NSData alloc] initWithBase64EncodedString:payload
                                              options:NSDataBase64DecodingIgnoreUnknownCharacters];
      if (!decoded) {
        decoded = [payload dataUsingEncoding:NSUTF8StringEncoding];
      }
      if (completion) completion(decoded, nil);
      return;
    }
  }

  // http:// or https://
  if ([uri hasPrefix:@"http://"] || [uri hasPrefix:@"https://"]) {
    NSURL *url = [NSURL URLWithString:uri];
    if (!url) {
      if (completion) {
        completion(nil, [NSError errorWithDomain:@"AnimaXRust"
                                            code:-2
                                        userInfo:@{NSLocalizedDescriptionKey : @"Invalid URL"}]);
      }
      return;
    }
    NSURLSessionDataTask *task =
        [[NSURLSession sharedSession] dataTaskWithURL:url
                                    completionHandler:^(NSData *data, NSURLResponse *resp,
                                                        NSError *err) {
                                      dispatch_async(dispatch_get_main_queue(), ^{
                                        if (completion) completion(data, err);
                                      });
                                    }];
    [task resume];
    return;
  }

  // file:// URL
  if ([uri hasPrefix:@"file://"]) {
    NSURL *fileUrl = [NSURL URLWithString:uri];
    NSError *err = nil;
    NSData *data = fileUrl ? [NSData dataWithContentsOfURL:fileUrl options:0 error:&err] : nil;
    if (completion) completion(data, err);
    return;
  }

  // Direct filesystem path
  if ([[NSFileManager defaultManager] fileExistsAtPath:uri]) {
    NSError *err = nil;
    NSData *data = [NSData dataWithContentsOfFile:uri options:0 error:&err];
    if (completion) completion(data, err);
    return;
  }

  // asset:// or relative bundle resource path
  NSString *relPath = uri;
  if ([relPath hasPrefix:@"asset://"]) {
    relPath = [relPath substringFromIndex:8];
  }
  NSBundle *targetBundle = bundle ?: [NSBundle mainBundle];
  NSString *resourcePath = [[targetBundle resourcePath] stringByAppendingPathComponent:relPath];
  if ([[NSFileManager defaultManager] fileExistsAtPath:resourcePath]) {
    NSError *err = nil;
    NSData *data = [NSData dataWithContentsOfFile:resourcePath options:0 error:&err];
    if (completion) completion(data, err);
    return;
  }
  NSString *name = [relPath stringByDeletingPathExtension];
  NSString *ext = [relPath pathExtension];
  NSString *found = [targetBundle pathForResource:[name lastPathComponent]
                                           ofType:ext.length ? ext : nil
                                      inDirectory:[name stringByDeletingLastPathComponent]];
  if (!found) {
    found = [targetBundle pathForResource:[name lastPathComponent] ofType:ext.length ? ext : nil];
  }
  if (found) {
    NSError *err = nil;
    NSData *data = [NSData dataWithContentsOfFile:found options:0 error:&err];
    if (completion) completion(data, err);
    return;
  }

  if (completion) {
    completion(nil, [NSError errorWithDomain:@"AnimaXRust"
                                        code:-3
                                    userInfo:@{
                                      NSLocalizedDescriptionKey :
                                          [NSString stringWithFormat:@"Resource not found: %@", uri]
                                    }]);
  }
}

static inline CGImageRef _Nullable RustCreateCGImageFromBGRA(const uint8_t *bgra_data, uint32_t w,
                                                             uint32_t h) {
  if (!bgra_data || w == 0 || h == 0) return NULL;
  const size_t byte_len = static_cast<size_t>(w) * static_cast<size_t>(h) * 4;
  NSData *nsData = [NSData dataWithBytes:bgra_data length:byte_len];
  CGDataProviderRef provider = CGDataProviderCreateWithCFData((__bridge CFDataRef)nsData);
  if (!provider) return NULL;
  CGColorSpaceRef cs = CGColorSpaceCreateDeviceRGB();
  CGBitmapInfo info = kCGBitmapByteOrder32Little | kCGImageAlphaPremultipliedFirst;
  CGImageRef img = CGImageCreate(w, h, 8, 32, static_cast<size_t>(w) * 4, cs, info, provider, NULL,
                                 false, kCGRenderingIntentDefault);
  CGColorSpaceRelease(cs);
  CGDataProviderRelease(provider);
  return img;
}

}  // namespace animax
}  // namespace lynx

#endif  // ANIMAX_USE_RUST_CORE
