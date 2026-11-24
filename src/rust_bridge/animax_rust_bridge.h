// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

#ifndef ANIMAX_SRC_RUST_BRIDGE_ANIMAX_RUST_BRIDGE_H_
#define ANIMAX_SRC_RUST_BRIDGE_ANIMAX_RUST_BRIDGE_H_

#include <cstdint>
#include <cstring>
#include <string>
#include <vector>

#include "include/animax_rust_ffi.h"

namespace lynx {
namespace animax {

/**
 * RAII C++ bridge over the pure Rust AnimaX core & pure Rust Skity renderer
 * (`libanimax_ffi.a`). Shared by Android JNI, iOS ObjC++, HarmonyOS NAPI, and
 * Desktop hosts.
 */
class RustPlayerBridge {
 public:
  explicit RustPlayerBridge(float device_scale = 1.0f);
  ~RustPlayerBridge();

  RustPlayerBridge(const RustPlayerBridge&) = delete;
  RustPlayerBridge& operator=(const RustPlayerBridge&) = delete;
  RustPlayerBridge(RustPlayerBridge&& other) noexcept;
  RustPlayerBridge& operator=(RustPlayerBridge&& other) noexcept;

  AnimaXPlayerHandle* raw() const { return handle_; }

  bool SetSurfaceSize(uint32_t width, uint32_t height);
  bool SetJson(const std::string& json);
  bool SetZip(const uint8_t* data, size_t len);
  void Reload();

  void Play();
  void Pause();
  void Resume();
  void Stop();
  void Seek(float frame);
  void SetProgress(float progress);
  void PlaySegment(float start_frame, float end_frame);

  void SetLoop(bool loop);
  void SetLoopCount(int32_t count);
  void SetAutoReverse(bool auto_reverse);
  void SetAutoplay(bool autoplay);
  void SetSpeed(float speed);
  void SetStartFrame(float frame);
  void SetEndFrame(float frame);
  void SetMaxFrameRate(float fps);
  void SetKeepLastFrame(bool keep);
  void SetObjectFit(uint8_t fit);
  void SetObjectPosition(uint8_t pos);

  int64_t GetDurationMs() const;
  bool IsAnimating() const;
  float GetCurrentFrame() const;
  float GetProgress() const;

  void OnShow(uint8_t state);
  void OnHide(uint8_t state);
  void UpdateVisibilityStates(uint8_t flags);
  size_t OnTap(float x, float y);

  bool RegisterImageEncoded(const std::string& id, const uint8_t* data,
                            size_t len);
  bool RegisterImageRgba(const std::string& id, uint32_t w, uint32_t h,
                         const uint8_t* rgba, size_t len);
  bool RegisterFont(const std::string& name, const uint8_t* ttf, size_t len);
  bool SubmitVideoFrameRgba(const std::string& video_id, uint32_t frame_w,
                            uint32_t frame_h, const uint8_t* rgba, size_t len);
  bool SubmitVideoFrameYuv420p(const std::string& video_id, uint32_t frame_w,
                               uint32_t frame_h, const uint8_t* y, size_t y_len,
                               const uint8_t* u, size_t u_len, const uint8_t* v,
                               size_t v_len);

  bool SetLayerColor(const std::string& key_path_csv, uint16_t property_type,
                     int32_t argb_color, int32_t target_frame = -1);
  bool SetLayerNumber(const std::string& key_path_csv, uint16_t property_type,
                      double value, int32_t target_frame = -1);
  bool SetLayerBool(const std::string& key_path_csv, uint16_t property_type,
                    bool value, int32_t target_frame = -1);
  bool SetLayerString(const std::string& key_path_csv, uint16_t property_type,
                      const std::string& value, int32_t target_frame = -1);
  bool SetLayerCoord(const std::string& key_path_csv, uint16_t property_type,
                     double x, double y, double z, int32_t target_frame = -1);

  void SubscribeUpdateEvent(int32_t frame);
  void UnsubscribeUpdateEvent(int32_t frame);

  bool Tick(int64_t timestamp_ns);
  bool Render();

  /**
   * Copies the rendered frame into a platform-owned pixel buffer (e.g.,
   * Android ANativeWindow_Buffer, iOS CVPixelBuffer/MTLTexture staging buffer,
   * or HarmonyOS OH_NativeWindow buffer) respecting row stride.
   */
  bool CopyPixelsToBuffer(uint8_t* dst, uint32_t dst_stride_bytes,
                          uint32_t dst_height,
                          AnimaXPixelFormat format) const;

  std::string DrainEventsJson();

 private:
  AnimaXPlayerHandle* handle_ = nullptr;
};

}  // namespace animax
}  // namespace lynx

#endif  // ANIMAX_SRC_RUST_BRIDGE_ANIMAX_RUST_BRIDGE_H_
