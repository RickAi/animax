// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

#include "src/rust_bridge/animax_rust_bridge.h"

#include <algorithm>

namespace lynx {
namespace animax {

RustPlayerBridge::RustPlayerBridge(float device_scale)
    : handle_(animax_player_create(device_scale)) {}

RustPlayerBridge::~RustPlayerBridge() {
  if (handle_) {
    animax_player_destroy(handle_);
    handle_ = nullptr;
  }
}

RustPlayerBridge::RustPlayerBridge(RustPlayerBridge&& other) noexcept
    : handle_(other.handle_) {
  other.handle_ = nullptr;
}

RustPlayerBridge& RustPlayerBridge::operator=(
    RustPlayerBridge&& other) noexcept {
  if (this != &other) {
    if (handle_) {
      animax_player_destroy(handle_);
    }
    handle_ = other.handle_;
    other.handle_ = nullptr;
  }
  return *this;
}

bool RustPlayerBridge::SetSurfaceSize(uint32_t width, uint32_t height) {
  return animax_player_set_surface_size(handle_, width, height);
}

bool RustPlayerBridge::SetJson(const std::string& json) {
  return animax_player_set_json(handle_, json.data(), json.size());
}

bool RustPlayerBridge::SetZip(const uint8_t* data, size_t len) {
  return animax_player_set_zip(handle_, data, len);
}

void RustPlayerBridge::Reload() { animax_player_reload(handle_); }
void RustPlayerBridge::Play() { animax_player_play(handle_); }
void RustPlayerBridge::Pause() { animax_player_pause(handle_); }
void RustPlayerBridge::Resume() { animax_player_resume(handle_); }
void RustPlayerBridge::Stop() { animax_player_stop(handle_); }
void RustPlayerBridge::Seek(float frame) { animax_player_seek(handle_, frame); }
void RustPlayerBridge::SetProgress(float progress) {
  animax_player_set_progress(handle_, progress);
}
void RustPlayerBridge::PlaySegment(float start_frame, float end_frame) {
  animax_player_play_segment(handle_, start_frame, end_frame);
}

void RustPlayerBridge::SetLoop(bool loop) {
  animax_player_set_loop(handle_, loop);
}
void RustPlayerBridge::SetLoopCount(int32_t count) {
  animax_player_set_loop_count(handle_, count);
}
void RustPlayerBridge::SetAutoReverse(bool auto_reverse) {
  animax_player_set_auto_reverse(handle_, auto_reverse);
}
void RustPlayerBridge::SetAutoplay(bool autoplay) {
  animax_player_set_autoplay(handle_, autoplay);
}
void RustPlayerBridge::SetSpeed(float speed) {
  animax_player_set_speed(handle_, speed);
}
void RustPlayerBridge::SetStartFrame(float frame) {
  animax_player_set_start_frame(handle_, frame);
}
void RustPlayerBridge::SetEndFrame(float frame) {
  animax_player_set_end_frame(handle_, frame);
}
void RustPlayerBridge::SetMaxFrameRate(float fps) {
  animax_player_set_max_frame_rate(handle_, fps);
}
void RustPlayerBridge::SetKeepLastFrame(bool keep) {
  animax_player_set_keep_last_frame(handle_, keep);
}
void RustPlayerBridge::SetObjectFit(uint8_t fit) {
  animax_player_set_object_fit(handle_, fit);
}
void RustPlayerBridge::SetObjectPosition(uint8_t pos) {
  animax_player_set_object_position(handle_, pos);
}

int64_t RustPlayerBridge::GetDurationMs() const {
  return animax_player_get_duration_ms(handle_);
}
bool RustPlayerBridge::IsAnimating() const {
  return animax_player_is_animating(handle_);
}
float RustPlayerBridge::GetCurrentFrame() const {
  return animax_player_get_current_frame(handle_);
}
float RustPlayerBridge::GetProgress() const {
  return animax_player_get_progress(handle_);
}

void RustPlayerBridge::OnShow(uint8_t state) {
  animax_player_on_show(handle_, state);
}
void RustPlayerBridge::OnHide(uint8_t state) {
  animax_player_on_hide(handle_, state);
}
void RustPlayerBridge::UpdateVisibilityStates(uint8_t flags) {
  animax_player_update_visibility_states(handle_, flags);
}
size_t RustPlayerBridge::OnTap(float x, float y) {
  return animax_player_on_tap(handle_, x, y);
}

bool RustPlayerBridge::RegisterImageEncoded(const std::string& id,
                                            const uint8_t* data, size_t len) {
  return animax_player_register_image_encoded(handle_, id.data(), id.size(),
                                              data, len);
}

bool RustPlayerBridge::RegisterImageRgba(const std::string& id, uint32_t w,
                                         uint32_t h, const uint8_t* rgba,
                                         size_t len) {
  return animax_player_register_image_rgba(handle_, id.data(), id.size(), w, h,
                                           rgba, len);
}

bool RustPlayerBridge::RegisterFont(const std::string& name, const uint8_t* ttf,
                                    size_t len) {
  return animax_player_register_font(handle_, name.data(), name.size(), ttf,
                                     len);
}

bool RustPlayerBridge::SubmitVideoFrameRgba(const std::string& video_id,
                                            uint32_t frame_w, uint32_t frame_h,
                                            const uint8_t* rgba, size_t len) {
  return animax_player_submit_video_frame_rgba(
      handle_, video_id.data(), video_id.size(), frame_w, frame_h, rgba, len);
}

bool RustPlayerBridge::SubmitVideoFrameYuv420p(
    const std::string& video_id, uint32_t frame_w, uint32_t frame_h,
    const uint8_t* y, size_t y_len, const uint8_t* u, size_t u_len,
    const uint8_t* v, size_t v_len) {
  return animax_player_submit_video_frame_yuv420p(
      handle_, video_id.data(), video_id.size(), frame_w, frame_h, y, y_len, u,
      u_len, v, v_len);
}

bool RustPlayerBridge::SetLayerColor(const std::string& key_path_csv,
                                     uint16_t property_type, int32_t argb_color,
                                     int32_t target_frame) {
  return animax_player_set_layer_color(handle_, key_path_csv.c_str(),
                                       property_type, argb_color, target_frame);
}

bool RustPlayerBridge::SetLayerNumber(const std::string& key_path_csv,
                                      uint16_t property_type, double value,
                                      int32_t target_frame) {
  return animax_player_set_layer_number(handle_, key_path_csv.c_str(),
                                        property_type, value, target_frame);
}

bool RustPlayerBridge::SetLayerBool(const std::string& key_path_csv,
                                    uint16_t property_type, bool value,
                                    int32_t target_frame) {
  return animax_player_set_layer_bool(handle_, key_path_csv.c_str(),
                                      property_type, value, target_frame);
}

bool RustPlayerBridge::SetLayerString(const std::string& key_path_csv,
                                      uint16_t property_type,
                                      const std::string& value,
                                      int32_t target_frame) {
  return animax_player_set_layer_string(handle_, key_path_csv.c_str(),
                                        property_type, value.c_str(),
                                        target_frame);
}

bool RustPlayerBridge::SetLayerCoord(const std::string& key_path_csv,
                                     uint16_t property_type, double x, double y,
                                     double z, int32_t target_frame) {
  return animax_player_set_layer_coord(handle_, key_path_csv.c_str(),
                                       property_type, x, y, z, target_frame);
}

void RustPlayerBridge::SubscribeUpdateEvent(int32_t frame) {
  animax_player_subscribe_update_event(handle_, frame);
}

void RustPlayerBridge::UnsubscribeUpdateEvent(int32_t frame) {
  animax_player_unsubscribe_update_event(handle_, frame);
}

bool RustPlayerBridge::Tick(int64_t timestamp_ns) {
  return animax_player_tick(handle_, timestamp_ns);
}

bool RustPlayerBridge::Render() { return animax_player_render(handle_); }

bool RustPlayerBridge::CopyPixelsToBuffer(uint8_t* dst,
                                          uint32_t dst_stride_bytes,
                                          uint32_t dst_height,
                                          AnimaXPixelFormat format) const {
  if (!dst || !handle_) {
    return false;
  }
  uint32_t src_w = 0, src_h = 0;
  animax_player_get_surface_size(handle_, &src_w, &src_h);
  size_t len = 0;
  const uint8_t* src =
      animax_player_get_pixels(handle_, static_cast<uint8_t>(format), &len);
  if (!src || src_w == 0 || src_h == 0) {
    return false;
  }
  const uint32_t src_row_bytes = src_w * 4;
  const uint32_t copy_bytes = std::min(src_row_bytes, dst_stride_bytes);
  const uint32_t copy_rows = std::min(src_h, dst_height);
  if (copy_bytes == src_row_bytes && dst_stride_bytes == src_row_bytes) {
    std::memcpy(dst, src, static_cast<size_t>(copy_bytes) * copy_rows);
  } else {
    for (uint32_t row = 0; row < copy_rows; ++row) {
      std::memcpy(dst + static_cast<size_t>(row) * dst_stride_bytes,
                  src + static_cast<size_t>(row) * src_row_bytes, copy_bytes);
    }
  }
  return true;
}

std::string RustPlayerBridge::DrainEventsJson() {
  char* raw_json = animax_player_drain_events_json(handle_);
  if (!raw_json) {
    return "[]";
  }
  std::string out(raw_json);
  animax_string_free(raw_json);
  return out;
}

}  // namespace animax
}  // namespace lynx
