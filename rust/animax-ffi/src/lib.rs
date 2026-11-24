// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

//! Zero-copy C ABI exposing the pure Rust AnimaX core & pure Rust Skity renderer
//! to Android (JNI), iOS (ObjC++), HarmonyOS (NAPI/OHOS), Desktop, and WebAssembly.

use animax_base::{ColorUtil, ObjectFit, ObjectPosition, VisibilityState};
use animax_engine::AnimaXPlayer;
use animax_property::{
    AnimaXKeyPath, AnimaXValueParam, LayerPropertyType, PropertyUpdateRequest,
};
use animax_skity::PixelFormat;
use serde::Serialize;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::slice;

/// Opaque handle to a Rust `AnimaXPlayer` instance.
pub struct AnimaXPlayerHandle {
    inner: AnimaXPlayer,
}

#[derive(Serialize)]
struct SerializedEvent {
    event: u8,
    params: animax_base::EventParamMap,
}

unsafe fn str_from_ptr<'a>(ptr: *const c_char, len: usize) -> Option<&'a str> {
    if ptr.is_null() {
        return None;
    }
    if len == 0 {
        CStr::from_ptr(ptr).to_str().ok()
    } else {
        let bytes = slice::from_raw_parts(ptr as *const u8, len);
        std::str::from_utf8(bytes).ok()
    }
}

unsafe fn bytes_from_ptr<'a>(ptr: *const u8, len: usize) -> Option<&'a [u8]> {
    if ptr.is_null() || len == 0 {
        None
    } else {
        Some(slice::from_raw_parts(ptr, len))
    }
}

fn parse_pixel_format(fmt: u8) -> PixelFormat {
    match fmt {
        1 => PixelFormat::Rgba8888Straight,
        2 => PixelFormat::Bgra8888Premul,
        _ => PixelFormat::Rgba8888Premul,
    }
}

#[no_mangle]
pub extern "C" fn animax_player_create(device_scale: f32) -> *mut AnimaXPlayerHandle {
    Box::into_raw(Box::new(AnimaXPlayerHandle {
        inner: AnimaXPlayer::new(device_scale),
    }))
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_destroy(handle: *mut AnimaXPlayerHandle) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_set_device_scale(
    handle: *mut AnimaXPlayerHandle,
    scale: f32,
) {
    if let Some(p) = handle.as_mut() {
        p.inner.set_device_scale(scale);
    }
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_set_surface_size(
    handle: *mut AnimaXPlayerHandle,
    width: u32,
    height: u32,
) -> bool {
    handle
        .as_mut()
        .is_some_and(|p| p.inner.set_surface_size(width, height))
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_get_surface_size(
    handle: *const AnimaXPlayerHandle,
    out_width: *mut u32,
    out_height: *mut u32,
) {
    if let Some(p) = handle.as_ref() {
        if !out_width.is_null() {
            *out_width = p.inner.surface_width();
        }
        if !out_height.is_null() {
            *out_height = p.inner.surface_height();
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_set_json(
    handle: *mut AnimaXPlayerHandle,
    json_ptr: *const c_char,
    json_len: usize,
) -> bool {
    let (Some(p), Some(json)) = (handle.as_mut(), str_from_ptr(json_ptr, json_len)) else {
        return false;
    };
    p.inner.set_json(json).is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_set_zip(
    handle: *mut AnimaXPlayerHandle,
    zip_ptr: *const u8,
    zip_len: usize,
) -> bool {
    let (Some(p), Some(bytes)) = (handle.as_mut(), bytes_from_ptr(zip_ptr, zip_len)) else {
        return false;
    };
    p.inner.set_zip_bytes(bytes).is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_reload(handle: *mut AnimaXPlayerHandle) {
    if let Some(p) = handle.as_mut() {
        p.inner.reload();
    }
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_play(handle: *mut AnimaXPlayerHandle) {
    if let Some(p) = handle.as_mut() {
        p.inner.play();
    }
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_pause(handle: *mut AnimaXPlayerHandle) {
    if let Some(p) = handle.as_mut() {
        p.inner.pause();
    }
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_resume(handle: *mut AnimaXPlayerHandle) {
    if let Some(p) = handle.as_mut() {
        p.inner.resume();
    }
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_stop(handle: *mut AnimaXPlayerHandle) {
    if let Some(p) = handle.as_mut() {
        p.inner.stop();
    }
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_seek(handle: *mut AnimaXPlayerHandle, frame: f32) {
    if let Some(p) = handle.as_mut() {
        p.inner.seek(frame);
    }
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_set_progress(handle: *mut AnimaXPlayerHandle, progress: f32) {
    if let Some(p) = handle.as_mut() {
        p.inner.set_progress(progress);
    }
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_play_segment(
    handle: *mut AnimaXPlayerHandle,
    start_frame: f32,
    end_frame: f32,
) {
    if let Some(p) = handle.as_mut() {
        p.inner.play_segment(start_frame, end_frame);
    }
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_set_loop(handle: *mut AnimaXPlayerHandle, loop_forever: bool) {
    if let Some(p) = handle.as_mut() {
        p.inner.set_loop(loop_forever);
    }
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_set_loop_count(handle: *mut AnimaXPlayerHandle, count: i32) {
    if let Some(p) = handle.as_mut() {
        p.inner.set_loop_count(count);
    }
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_set_auto_reverse(
    handle: *mut AnimaXPlayerHandle,
    auto_reverse: bool,
) {
    if let Some(p) = handle.as_mut() {
        p.inner.set_auto_reverse(auto_reverse);
    }
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_set_autoplay(handle: *mut AnimaXPlayerHandle, autoplay: bool) {
    if let Some(p) = handle.as_mut() {
        p.inner.set_autoplay(autoplay);
    }
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_set_speed(handle: *mut AnimaXPlayerHandle, speed: f32) {
    if let Some(p) = handle.as_mut() {
        p.inner.set_speed(speed);
    }
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_set_start_frame(handle: *mut AnimaXPlayerHandle, frame: f32) {
    if let Some(p) = handle.as_mut() {
        p.inner.set_start_frame(frame);
    }
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_set_end_frame(handle: *mut AnimaXPlayerHandle, frame: f32) {
    if let Some(p) = handle.as_mut() {
        p.inner.set_end_frame(frame);
    }
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_set_max_frame_rate(handle: *mut AnimaXPlayerHandle, fps: f32) {
    if let Some(p) = handle.as_mut() {
        p.inner.set_max_frame_rate(fps);
    }
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_set_keep_last_frame(
    handle: *mut AnimaXPlayerHandle,
    keep: bool,
) {
    if let Some(p) = handle.as_mut() {
        p.inner.set_keep_last_frame(keep);
    }
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_set_object_fit(handle: *mut AnimaXPlayerHandle, fit: u8) {
    if let Some(p) = handle.as_mut() {
        p.inner.set_object_fit(ObjectFit::from_u8(fit));
    }
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_set_object_position(
    handle: *mut AnimaXPlayerHandle,
    pos: u8,
) {
    if let Some(p) = handle.as_mut() {
        p.inner.set_object_position(ObjectPosition::from_u8(pos));
    }
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_get_duration_ms(handle: *const AnimaXPlayerHandle) -> i64 {
    handle.as_ref().map_or(0, |p| p.inner.duration_ms())
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_is_animating(handle: *const AnimaXPlayerHandle) -> bool {
    handle.as_ref().is_some_and(|p| p.inner.is_animating())
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_get_current_frame(handle: *const AnimaXPlayerHandle) -> f32 {
    handle.as_ref().map_or(0.0, |p| p.inner.current_frame())
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_get_progress(handle: *const AnimaXPlayerHandle) -> f32 {
    handle.as_ref().map_or(0.0, |p| p.inner.progress())
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_on_show(handle: *mut AnimaXPlayerHandle, state: u8) {
    if let (Some(p), Some(s)) = (handle.as_mut(), VisibilityState::from_u8(state)) {
        p.inner.on_show(s);
    }
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_on_hide(handle: *mut AnimaXPlayerHandle, state: u8) {
    if let (Some(p), Some(s)) = (handle.as_mut(), VisibilityState::from_u8(state)) {
        p.inner.on_hide(s);
    }
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_update_visibility_states(
    handle: *mut AnimaXPlayerHandle,
    flags: u8,
) {
    if let Some(p) = handle.as_mut() {
        p.inner.update_visibility_states(flags);
    }
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_on_tap(
    handle: *mut AnimaXPlayerHandle,
    x: f32,
    y: f32,
) -> usize {
    handle
        .as_mut()
        .map_or(0, |p| p.inner.on_tap(x, y).len())
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_tick(
    handle: *mut AnimaXPlayerHandle,
    timestamp_ns: i64,
) -> bool {
    handle.as_mut().is_some_and(|p| p.inner.tick(timestamp_ns))
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_render(handle: *mut AnimaXPlayerHandle) -> bool {
    handle
        .as_mut()
        .is_some_and(|p| p.inner.render_current_frame())
}

/// Returns a pointer to the rendered pixel buffer in the requested `PixelFormat`
/// (`0 = RGBA8888Premul`, `1 = RGBA8888Straight`, `2 = BGRA8888Premul`).
/// Valid until the next mutating call on `handle`.
#[no_mangle]
pub unsafe extern "C" fn animax_player_get_pixels(
    handle: *mut AnimaXPlayerHandle,
    pixel_format: u8,
    out_len: *mut usize,
) -> *const u8 {
    let Some(p) = handle.as_mut() else {
        if !out_len.is_null() {
            *out_len = 0;
        }
        return std::ptr::null();
    };
    let Some(bytes) = p.inner.pixels(parse_pixel_format(pixel_format)) else {
        if !out_len.is_null() {
            *out_len = 0;
        }
        return std::ptr::null();
    };
    if !out_len.is_null() {
        *out_len = bytes.len();
    }
    bytes.as_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_register_image_encoded(
    handle: *mut AnimaXPlayerHandle,
    id_ptr: *const c_char,
    id_len: usize,
    data_ptr: *const u8,
    data_len: usize,
) -> bool {
    let (Some(p), Some(id), Some(bytes)) = (
        handle.as_mut(),
        str_from_ptr(id_ptr, id_len),
        bytes_from_ptr(data_ptr, data_len),
    ) else {
        return false;
    };
    p.inner.register_image_encoded(id, bytes)
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_register_image_rgba(
    handle: *mut AnimaXPlayerHandle,
    id_ptr: *const c_char,
    id_len: usize,
    width: u32,
    height: u32,
    rgba_ptr: *const u8,
    rgba_len: usize,
) -> bool {
    let (Some(p), Some(id), Some(bytes)) = (
        handle.as_mut(),
        str_from_ptr(id_ptr, id_len),
        bytes_from_ptr(rgba_ptr, rgba_len),
    ) else {
        return false;
    };
    p.inner.register_image_rgba(id, width, height, bytes)
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_register_font(
    handle: *mut AnimaXPlayerHandle,
    name_ptr: *const c_char,
    name_len: usize,
    ttf_ptr: *const u8,
    ttf_len: usize,
) -> bool {
    let (Some(p), Some(name), Some(bytes)) = (
        handle.as_mut(),
        str_from_ptr(name_ptr, name_len),
        bytes_from_ptr(ttf_ptr, ttf_len),
    ) else {
        return false;
    };
    p.inner.register_font(name, bytes)
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_submit_video_frame_rgba(
    handle: *mut AnimaXPlayerHandle,
    video_id_ptr: *const c_char,
    video_id_len: usize,
    frame_w: u32,
    frame_h: u32,
    rgba_ptr: *const u8,
    rgba_len: usize,
) -> bool {
    let (Some(p), Some(id), Some(bytes)) = (
        handle.as_mut(),
        str_from_ptr(video_id_ptr, video_id_len),
        bytes_from_ptr(rgba_ptr, rgba_len),
    ) else {
        return false;
    };
    p.inner.submit_video_frame_rgba(id, frame_w, frame_h, bytes)
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_submit_video_frame_yuv420p(
    handle: *mut AnimaXPlayerHandle,
    video_id_ptr: *const c_char,
    video_id_len: usize,
    frame_w: u32,
    frame_h: u32,
    y_ptr: *const u8,
    y_len: usize,
    u_ptr: *const u8,
    u_len: usize,
    v_ptr: *const u8,
    v_len: usize,
) -> bool {
    let (Some(p), Some(id), Some(y), Some(u), Some(v)) = (
        handle.as_mut(),
        str_from_ptr(video_id_ptr, video_id_len),
        bytes_from_ptr(y_ptr, y_len),
        bytes_from_ptr(u_ptr, u_len),
        bytes_from_ptr(v_ptr, v_len),
    ) else {
        return false;
    };
    p.inner
        .submit_video_frame_yuv420p(id, frame_w, frame_h, y, u, v)
}

fn parse_key_path_csv(csv: &str) -> AnimaXKeyPath {
    AnimaXKeyPath::new(csv.split(',').map(|s| s.to_string()).collect())
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_set_layer_color(
    handle: *mut AnimaXPlayerHandle,
    key_path_csv: *const c_char,
    property_type: u16,
    argb_color: i32,
    target_frame: i32,
) -> bool {
    let (Some(p), Some(kp)) = (handle.as_mut(), str_from_ptr(key_path_csv, 0)) else {
        return false;
    };
    let mut req = PropertyUpdateRequest::default();
    req.add_layer_property(
        parse_key_path_csv(kp),
        LayerPropertyType::from_u16(property_type),
        AnimaXValueParam::from_color(argb_color, target_frame),
    );
    p.inner.update_properties(req).success
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_set_layer_number(
    handle: *mut AnimaXPlayerHandle,
    key_path_csv: *const c_char,
    property_type: u16,
    value: f64,
    target_frame: i32,
) -> bool {
    let (Some(p), Some(kp)) = (handle.as_mut(), str_from_ptr(key_path_csv, 0)) else {
        return false;
    };
    let mut req = PropertyUpdateRequest::default();
    req.add_layer_property(
        parse_key_path_csv(kp),
        LayerPropertyType::from_u16(property_type),
        AnimaXValueParam::from_number(value, target_frame),
    );
    p.inner.update_properties(req).success
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_set_layer_bool(
    handle: *mut AnimaXPlayerHandle,
    key_path_csv: *const c_char,
    property_type: u16,
    value: bool,
    target_frame: i32,
) -> bool {
    let (Some(p), Some(kp)) = (handle.as_mut(), str_from_ptr(key_path_csv, 0)) else {
        return false;
    };
    let mut req = PropertyUpdateRequest::default();
    req.add_layer_property(
        parse_key_path_csv(kp),
        LayerPropertyType::from_u16(property_type),
        AnimaXValueParam::from_bool(value, target_frame),
    );
    p.inner.update_properties(req).success
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_set_layer_string(
    handle: *mut AnimaXPlayerHandle,
    key_path_csv: *const c_char,
    property_type: u16,
    value: *const c_char,
    target_frame: i32,
) -> bool {
    let (Some(p), Some(kp), Some(val)) = (
        handle.as_mut(),
        str_from_ptr(key_path_csv, 0),
        str_from_ptr(value, 0),
    ) else {
        return false;
    };
    let mut req = PropertyUpdateRequest::default();
    req.add_layer_property(
        parse_key_path_csv(kp),
        LayerPropertyType::from_u16(property_type),
        AnimaXValueParam::from_string(val, target_frame),
    );
    p.inner.update_properties(req).success
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_set_layer_coord(
    handle: *mut AnimaXPlayerHandle,
    key_path_csv: *const c_char,
    property_type: u16,
    x: f64,
    y: f64,
    z: f64,
    target_frame: i32,
) -> bool {
    let (Some(p), Some(kp)) = (handle.as_mut(), str_from_ptr(key_path_csv, 0)) else {
        return false;
    };
    let mut req = PropertyUpdateRequest::default();
    req.add_layer_property(
        parse_key_path_csv(kp),
        LayerPropertyType::from_u16(property_type),
        AnimaXValueParam::from_coord(x, y, z, target_frame),
    );
    p.inner.update_properties(req).success
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_subscribe_update_event(
    handle: *mut AnimaXPlayerHandle,
    frame: i32,
) {
    if let Some(p) = handle.as_mut() {
        p.inner.subscribe_update_event(frame);
    }
}

#[no_mangle]
pub unsafe extern "C" fn animax_player_unsubscribe_update_event(
    handle: *mut AnimaXPlayerHandle,
    frame: i32,
) {
    if let Some(p) = handle.as_mut() {
        p.inner.unsubscribe_update_event(frame);
    }
}

/// Drains queued events as a newly allocated NUL-terminated JSON string.
/// Caller must release with `animax_string_free`.
#[no_mangle]
pub unsafe extern "C" fn animax_player_drain_events_json(
    handle: *mut AnimaXPlayerHandle,
) -> *mut c_char {
    let Some(p) = handle.as_mut() else {
        return std::ptr::null_mut();
    };
    let events: Vec<SerializedEvent> = p
        .inner
        .drain_events()
        .into_iter()
        .map(|e| SerializedEvent {
            event: e.event as u8,
            params: e.params,
        })
        .collect();
    let json = serde_json::to_string(&events).unwrap_or_else(|_| "[]".to_string());
    CString::new(json)
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn animax_string_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(CString::from_raw(ptr));
    }
}

#[no_mangle]
pub unsafe extern "C" fn animax_parse_hex_color(hex: *const c_char) -> i32 {
    str_from_ptr(hex, 0).map_or(0, ColorUtil::parse_hex_color)
}
