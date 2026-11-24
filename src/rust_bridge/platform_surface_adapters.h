// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

#ifndef ANIMAX_SRC_RUST_BRIDGE_PLATFORM_SURFACE_ADAPTERS_H_
#define ANIMAX_SRC_RUST_BRIDGE_PLATFORM_SURFACE_ADAPTERS_H_

#include "src/rust_bridge/animax_rust_bridge.h"

#if defined(__ANDROID__)
#include <android/native_window.h>
#endif

#if defined(__APPLE__)
#include <CoreVideo/CoreVideo.h>
#endif

#if defined(__OHOS__)
#include <native_window/external_window.h>
#endif

namespace lynx {
namespace animax {

/**
 * Platform-specific zero-Skity surface presentation helpers that blit
 * rendered frames directly from `RustPlayerBridge` (`animax-skity`) to
 * native Android, iOS, and HarmonyOS surfaces.
 */
class PlatformSurfaceAdapters {
 public:
#if defined(__ANDROID__)
  static bool PresentToAndroidNativeWindow(const RustPlayerBridge& player,
                                           ANativeWindow* window) {
    if (!window) {
      return false;
    }
    ANativeWindow_Buffer buf{};
    if (ANativeWindow_lock(window, &buf, nullptr) != 0) {
      return false;
    }
    bool ok = player.CopyPixelsToBuffer(
        static_cast<uint8_t*>(buf.bits),
        static_cast<uint32_t>(buf.stride) * 4,
        static_cast<uint32_t>(buf.height),
        ANIMAX_PIXEL_FORMAT_RGBA8888_PREMUL);
    ANativeWindow_unlockAndPost(window);
    return ok;
  }
#endif

#if defined(__APPLE__)
  static bool PresentToIOSCVPixelBuffer(const RustPlayerBridge& player,
                                        CVPixelBufferRef pixel_buffer) {
    if (!pixel_buffer) {
      return false;
    }
    if (CVPixelBufferLockBaseAddress(pixel_buffer, 0) != kCVReturnSuccess) {
      return false;
    }
    auto* base = static_cast<uint8_t*>(CVPixelBufferGetBaseAddress(pixel_buffer));
    auto stride = static_cast<uint32_t>(CVPixelBufferGetBytesPerRow(pixel_buffer));
    auto height = static_cast<uint32_t>(CVPixelBufferGetHeight(pixel_buffer));
    bool ok = player.CopyPixelsToBuffer(base, stride, height,
                                        ANIMAX_PIXEL_FORMAT_BGRA8888_PREMUL);
    CVPixelBufferUnlockBaseAddress(pixel_buffer, 0);
    return ok;
  }
#endif

#if defined(__OHOS__)
  static bool PresentToHarmonyNativeWindow(const RustPlayerBridge& player,
                                           OHNativeWindow* window) {
    if (!window) {
      return false;
    }
    OHNativeWindowBuffer* buffer = nullptr;
    int fence_fd = -1;
    if (OH_NativeWindow_NativeWindowRequestBuffer(window, &buffer, &fence_fd) != 0 ||
        !buffer) {
      return false;
    }
    BufferHandle* handle = OH_NativeWindow_GetBufferHandleFromNative(buffer);
    bool ok = false;
    if (handle && handle->virAddr) {
      ok = player.CopyPixelsToBuffer(
          static_cast<uint8_t*>(handle->virAddr),
          static_cast<uint32_t>(handle->stride),
          static_cast<uint32_t>(handle->height),
          ANIMAX_PIXEL_FORMAT_RGBA8888_PREMUL);
    }
    Region region{nullptr, 0};
    OH_NativeWindow_NativeWindowFlushBuffer(window, buffer, fence_fd, region);
    return ok;
  }
#endif
};

}  // namespace animax
}  // namespace lynx

#endif  // ANIMAX_SRC_RUST_BRIDGE_PLATFORM_SURFACE_ADAPTERS_H_
