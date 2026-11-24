// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

#ifndef ANIMAX_INCLUDE_ANIMAX_RUST_FFI_H_
#define ANIMAX_INCLUDE_ANIMAX_RUST_FFI_H_

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct AnimaXPlayerHandle AnimaXPlayerHandle;

enum AnimaXPixelFormat {
  ANIMAX_PIXEL_FORMAT_RGBA8888_PREMUL = 0,
  ANIMAX_PIXEL_FORMAT_RGBA8888_STRAIGHT = 1,
  ANIMAX_PIXEL_FORMAT_BGRA8888_PREMUL = 2,
};

AnimaXPlayerHandle* animax_player_create(float device_scale);
void animax_player_destroy(AnimaXPlayerHandle* handle);

void animax_player_set_device_scale(AnimaXPlayerHandle* handle, float scale);
bool animax_player_set_surface_size(AnimaXPlayerHandle* handle, uint32_t width,
                                    uint32_t height);
void animax_player_get_surface_size(const AnimaXPlayerHandle* handle,
                                    uint32_t* out_width, uint32_t* out_height);

bool animax_player_set_json(AnimaXPlayerHandle* handle, const char* json_ptr,
                            size_t json_len);
bool animax_player_set_zip(AnimaXPlayerHandle* handle, const uint8_t* zip_ptr,
                           size_t zip_len);
void animax_player_reload(AnimaXPlayerHandle* handle);

void animax_player_play(AnimaXPlayerHandle* handle);
void animax_player_pause(AnimaXPlayerHandle* handle);
void animax_player_resume(AnimaXPlayerHandle* handle);
void animax_player_stop(AnimaXPlayerHandle* handle);
void animax_player_seek(AnimaXPlayerHandle* handle, float frame);
void animax_player_set_progress(AnimaXPlayerHandle* handle, float progress);
void animax_player_play_segment(AnimaXPlayerHandle* handle, float start_frame,
                                float end_frame);

void animax_player_set_loop(AnimaXPlayerHandle* handle, bool loop_forever);
void animax_player_set_loop_count(AnimaXPlayerHandle* handle, int32_t count);
void animax_player_set_auto_reverse(AnimaXPlayerHandle* handle,
                                    bool auto_reverse);
void animax_player_set_autoplay(AnimaXPlayerHandle* handle, bool autoplay);
void animax_player_set_speed(AnimaXPlayerHandle* handle, float speed);
void animax_player_set_start_frame(AnimaXPlayerHandle* handle, float frame);
void animax_player_set_end_frame(AnimaXPlayerHandle* handle, float frame);
void animax_player_set_max_frame_rate(AnimaXPlayerHandle* handle, float fps);
void animax_player_set_keep_last_frame(AnimaXPlayerHandle* handle, bool keep);
void animax_player_set_object_fit(AnimaXPlayerHandle* handle, uint8_t fit);
void animax_player_set_object_position(AnimaXPlayerHandle* handle, uint8_t pos);

int64_t animax_player_get_duration_ms(const AnimaXPlayerHandle* handle);
bool animax_player_is_animating(const AnimaXPlayerHandle* handle);
float animax_player_get_current_frame(const AnimaXPlayerHandle* handle);
float animax_player_get_progress(const AnimaXPlayerHandle* handle);

void animax_player_on_show(AnimaXPlayerHandle* handle, uint8_t state);
void animax_player_on_hide(AnimaXPlayerHandle* handle, uint8_t state);
void animax_player_update_visibility_states(AnimaXPlayerHandle* handle,
                                            uint8_t flags);
size_t animax_player_on_tap(AnimaXPlayerHandle* handle, float x, float y);

bool animax_player_tick(AnimaXPlayerHandle* handle, int64_t timestamp_ns);
bool animax_player_render(AnimaXPlayerHandle* handle);
const uint8_t* animax_player_get_pixels(AnimaXPlayerHandle* handle,
                                        uint8_t pixel_format, size_t* out_len);

bool animax_player_register_image_encoded(AnimaXPlayerHandle* handle,
                                          const char* id_ptr, size_t id_len,
                                          const uint8_t* data_ptr,
                                          size_t data_len);
bool animax_player_register_image_rgba(AnimaXPlayerHandle* handle,
                                       const char* id_ptr, size_t id_len,
                                       uint32_t width, uint32_t height,
                                       const uint8_t* rgba_ptr,
                                       size_t rgba_len);
bool animax_player_register_font(AnimaXPlayerHandle* handle,
                                 const char* name_ptr, size_t name_len,
                                 const uint8_t* ttf_ptr, size_t ttf_len);
bool animax_player_submit_video_frame_rgba(AnimaXPlayerHandle* handle,
                                           const char* video_id_ptr,
                                           size_t video_id_len,
                                           uint32_t frame_w, uint32_t frame_h,
                                           const uint8_t* rgba_ptr,
                                           size_t rgba_len);
bool animax_player_submit_video_frame_yuv420p(
    AnimaXPlayerHandle* handle, const char* video_id_ptr, size_t video_id_len,
    uint32_t frame_w, uint32_t frame_h, const uint8_t* y_ptr, size_t y_len,
    const uint8_t* u_ptr, size_t u_len, const uint8_t* v_ptr, size_t v_len);

bool animax_player_set_layer_color(AnimaXPlayerHandle* handle,
                                   const char* key_path_csv,
                                   uint16_t property_type, int32_t argb_color,
                                   int32_t target_frame);
bool animax_player_set_layer_number(AnimaXPlayerHandle* handle,
                                    const char* key_path_csv,
                                    uint16_t property_type, double value,
                                    int32_t target_frame);
bool animax_player_set_layer_bool(AnimaXPlayerHandle* handle,
                                  const char* key_path_csv,
                                  uint16_t property_type, bool value,
                                  int32_t target_frame);
bool animax_player_set_layer_string(AnimaXPlayerHandle* handle,
                                    const char* key_path_csv,
                                    uint16_t property_type, const char* value,
                                    int32_t target_frame);
bool animax_player_set_layer_coord(AnimaXPlayerHandle* handle,
                                   const char* key_path_csv,
                                   uint16_t property_type, double x, double y,
                                   double z, int32_t target_frame);

void animax_player_subscribe_update_event(AnimaXPlayerHandle* handle,
                                          int32_t frame);
void animax_player_unsubscribe_update_event(AnimaXPlayerHandle* handle,
                                            int32_t frame);

char* animax_player_drain_events_json(AnimaXPlayerHandle* handle);
void animax_string_free(char* ptr);

int32_t animax_parse_hex_color(const char* hex);

#ifdef __cplusplus
}
#endif

#endif  // ANIMAX_INCLUDE_ANIMAX_RUST_FFI_H_
