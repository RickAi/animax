// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

use crate::layer::{draw_layer_list, hit_test_layers, RenderContext};
use animax_animation::ValueAnimator;
use animax_base::{
    compute_fit_transform, Event, EventError, EventKeys, EventParamMap, EventParamValue,
    ObjectFit, ObjectPosition, VisibilityState, VisibilityStateFlags,
};
use animax_model::{CompositionModel, RectF};
use animax_parser::{parse_lottie_json, parse_zip_bundle};
use animax_property::{PropertyStore, PropertyUpdateRequest, PropertyUpdateResponse};
use animax_skity::{AlphaVideoCompositor, FontManager, Matrix, PixelFormat, Pixmap, Surface};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct PlayerEvent {
    pub event: Event,
    pub params: EventParamMap,
}

/// Complete pure Rust AnimaX Player replacing `AnimaXPlayer`, `AnimaXRenderer`,
/// `AnimaXMainController`, `AnimaXEventDispatcher`, and Skity surface management.
pub struct AnimaXPlayer {
    composition: Option<CompositionModel>,
    animator: ValueAnimator,
    surface: Option<Surface>,
    images: HashMap<String, Pixmap>,
    video_frames: HashMap<String, Pixmap>,
    font_manager: FontManager,
    props: PropertyStore,
    visibility: VisibilityStateFlags,
    object_fit: ObjectFit,
    object_position: ObjectPosition,
    device_scale: f32,
    autoplay: bool,
    keep_last_frame: bool,
    first_frame_flushed: bool,
    subscribed_frames: HashSet<i32>,
    last_emitted_frame: Option<i32>,
    events: Vec<PlayerEvent>,
    last_json: Option<String>,
}

impl Default for AnimaXPlayer {
    fn default() -> Self {
        Self::new(1.0)
    }
}

impl AnimaXPlayer {
    pub fn new(device_scale: f32) -> Self {
        Self {
            composition: None,
            animator: ValueAnimator::new(),
            surface: None,
            images: HashMap::new(),
            video_frames: HashMap::new(),
            font_manager: FontManager::new(),
            props: PropertyStore::new(),
            visibility: VisibilityStateFlags::new(),
            object_fit: ObjectFit::Contain,
            object_position: ObjectPosition::Center,
            device_scale: if device_scale > 0.0 { device_scale } else { 1.0 },
            autoplay: true,
            keep_last_frame: true,
            first_frame_flushed: false,
            subscribed_frames: HashSet::new(),
            last_emitted_frame: None,
            events: Vec::new(),
            last_json: None,
        }
    }

    pub fn set_device_scale(&mut self, scale: f32) {
        if scale > 0.0 {
            self.device_scale = scale;
        }
    }

    pub fn set_surface_size(&mut self, width: u32, height: u32) -> bool {
        if width == 0 || height == 0 {
            return false;
        }
        let ok = match &mut self.surface {
            Some(s) => s.resize(width, height),
            None => {
                self.surface = Surface::new(width, height);
                self.surface.is_some()
            }
        };
        if ok && self.composition.is_some() {
            self.emit(Event::Ready, EventParamMap::new());
            self.render_current_frame();
        }
        ok
    }

    pub fn surface_width(&self) -> u32 {
        self.surface.as_ref().map_or(0, |s| s.width())
    }

    pub fn surface_height(&self) -> u32 {
        self.surface.as_ref().map_or(0, |s| s.height())
    }

    pub fn set_json(&mut self, json: &str) -> Result<(), String> {
        self.last_json = Some(json.to_string());
        match parse_lottie_json(json) {
            Ok(comp) => {
                self.set_composition(comp);
                Ok(())
            }
            Err(e) => {
                let mut params = EventParamMap::new();
                params.insert(
                    EventKeys::CODE.to_string(),
                    EventParamValue::Int32(EventError::ResourceNotFound as i32),
                );
                params.insert(
                    EventKeys::MESSAGE.to_string(),
                    EventParamValue::String(e.to_string()),
                );
                self.emit(Event::Error, params);
                Err(e.to_string())
            }
        }
    }

    pub fn set_zip_bytes(&mut self, bytes: &[u8]) -> Result<(), String> {
        match parse_zip_bundle(bytes) {
            Ok(bundle) => {
                self.set_composition(bundle.composition);
                Ok(())
            }
            Err(e) => {
                let mut params = EventParamMap::new();
                params.insert(
                    EventKeys::CODE.to_string(),
                    EventParamValue::Int32(EventError::ResourceNotFound as i32),
                );
                params.insert(
                    EventKeys::MESSAGE.to_string(),
                    EventParamValue::String(e.to_string()),
                );
                self.emit(Event::Error, params);
                Err(e.to_string())
            }
        }
    }

    pub fn set_composition(&mut self, comp: CompositionModel) {
        self.images.clear();
        // Decode any embedded images (data URIs or ZIP-bundled images) directly in pure Rust
        for (id, asset) in &comp.images {
            if let Some(bytes) = &asset.embedded_data {
                if let Some(px) = Pixmap::decode_encoded_bytes(bytes) {
                    self.images.insert(id.clone(), px);
                }
            }
        }
        self.animator
            .set_composition_range(comp.start_frame, comp.end_frame, comp.frame_rate);
        self.composition = Some(comp);
        self.first_frame_flushed = false;
        self.last_emitted_frame = None;
        self.emit(Event::CompositionReady, EventParamMap::new());
        if self.surface.is_some() {
            self.emit(Event::Ready, EventParamMap::new());
        }
        if self.autoplay && self.visibility.is_visible() {
            self.animator.play();
        }
        self.render_current_frame();
    }

    pub fn reload(&mut self) {
        if let Some(json) = self.last_json.clone() {
            let _ = self.set_json(&json);
        } else if let Some(comp) = self.composition.clone() {
            self.set_composition(comp);
        }
    }

    pub fn register_image_encoded(&mut self, id: &str, bytes: &[u8]) -> bool {
        if let Some(px) = Pixmap::decode_encoded_bytes(bytes) {
            self.images.insert(id.to_string(), px);
            true
        } else {
            false
        }
    }

    pub fn register_image_rgba(&mut self, id: &str, width: u32, height: u32, rgba: &[u8]) -> bool {
        if let Some(px) = Pixmap::from_straight_rgba(width, height, rgba) {
            self.images.insert(id.to_string(), px);
            true
        } else {
            false
        }
    }

    pub fn register_font(&mut self, name: &str, ttf_bytes: &[u8]) -> bool {
        self.font_manager.register_font(name, ttf_bytes)
    }

    /// Uploads a decoded split Alpha Video RGBA frame and composites RGB+Alpha in pure Rust.
    pub fn submit_video_frame_rgba(
        &mut self,
        video_id: &str,
        frame_w: u32,
        frame_h: u32,
        rgba: &[u8],
    ) -> bool {
        let (rgb_rect, alpha_rect) = self.video_split_rects(frame_w, frame_h);
        if let Some(px) =
            AlphaVideoCompositor::compose_rgba_split(rgba, frame_w, frame_h, rgb_rect, alpha_rect)
        {
            self.video_frames.insert(video_id.to_string(), px);
            true
        } else {
            false
        }
    }

    /// Uploads a decoded split Alpha Video YUV420p frame and composites RGB+Alpha in pure Rust.
    pub fn submit_video_frame_yuv420p(
        &mut self,
        video_id: &str,
        frame_w: u32,
        frame_h: u32,
        y: &[u8],
        u: &[u8],
        v: &[u8],
    ) -> bool {
        let (rgb_rect, alpha_rect) = self.video_split_rects(frame_w, frame_h);
        if let Some(px) = AlphaVideoCompositor::compose_yuv420p_split(
            y, u, v, frame_w, frame_h, rgb_rect, alpha_rect,
        ) {
            self.video_frames.insert(video_id.to_string(), px);
            true
        } else {
            false
        }
    }

    fn video_split_rects(&self, fw: u32, fh: u32) -> (RectF, RectF) {
        if let Some(cfg) = self
            .composition
            .as_ref()
            .and_then(|c| c.alpha_player_config.as_ref())
        {
            if let Some(item) = cfg.portrait.as_ref().or(cfg.landscape.as_ref()) {
                if !item.rgb_frame.is_empty() && !item.alpha_frame.is_empty() {
                    return (item.rgb_frame, item.alpha_frame);
                }
            }
        }
        let half_w = (fw / 2) as f32;
        (
            RectF::new(0.0, 0.0, half_w, fh as f32),
            RectF::new(half_w, 0.0, fw as f32, fh as f32),
        )
    }

    // --- Playback Controls ---
    pub fn set_loop(&mut self, loop_forever: bool) {
        self.animator.set_loop(loop_forever);
    }

    pub fn set_loop_count(&mut self, count: i32) {
        self.animator.set_loop_count(count);
    }

    pub fn set_auto_reverse(&mut self, auto_reverse: bool) {
        self.animator.set_auto_reverse(auto_reverse);
    }

    pub fn set_autoplay(&mut self, autoplay: bool) {
        self.autoplay = autoplay;
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.animator.set_speed(speed);
    }

    pub fn set_object_fit(&mut self, fit: ObjectFit) {
        self.object_fit = fit;
    }

    pub fn set_object_position(&mut self, pos: ObjectPosition) {
        self.object_position = pos;
    }

    pub fn set_keep_last_frame(&mut self, keep: bool) {
        self.keep_last_frame = keep;
    }

    pub fn set_start_frame(&mut self, frame: f32) {
        self.animator.set_start_frame(frame);
    }

    pub fn set_end_frame(&mut self, frame: f32) {
        self.animator.set_end_frame(frame);
    }

    pub fn set_max_frame_rate(&mut self, fps: f32) {
        self.animator.set_max_frame_rate(fps);
    }

    pub fn set_progress(&mut self, progress: f32) {
        self.animator.set_progress(progress);
        self.render_current_frame();
    }

    pub fn seek(&mut self, frame: f32) {
        self.animator.seek(frame);
        self.render_current_frame();
    }

    pub fn play(&mut self) {
        self.animator.play();
    }

    pub fn play_segment(&mut self, start_frame: f32, end_frame: f32) {
        self.animator.play_segment(start_frame, end_frame);
    }

    pub fn pause(&mut self) {
        self.animator.pause();
    }

    pub fn resume(&mut self) {
        self.animator.resume();
    }

    pub fn stop(&mut self) {
        if self.animator.stop() {
            self.emit(Event::Cancel, EventParamMap::new());
        }
        if !self.keep_last_frame {
            if let Some(s) = &mut self.surface {
                s.clear();
            }
        }
    }

    pub fn duration_ms(&self) -> i64 {
        self.composition.as_ref().map_or(0, |c| c.duration_ms())
    }

    pub fn is_animating(&self) -> bool {
        self.animator.is_animating()
    }

    pub fn current_frame(&self) -> f32 {
        self.animator.current_frame()
    }

    pub fn progress(&self) -> f32 {
        self.animator.progress()
    }

    // --- Visibility & Interaction ---
    pub fn on_show(&mut self, state: VisibilityState) {
        self.visibility.on_show(state);
        if self.visibility.is_visible() && self.autoplay && !self.animator.is_animating() {
            self.animator.resume();
        }
    }

    pub fn on_hide(&mut self, state: VisibilityState) {
        self.visibility.on_hide(state);
        if !self.visibility.is_visible() && self.animator.is_animating() {
            self.animator.pause();
        }
    }

    pub fn update_visibility_states(&mut self, flags: u8) {
        self.visibility.set_flags(flags);
        if !self.visibility.is_visible() && self.animator.is_animating() {
            self.animator.pause();
        }
    }

    pub fn on_tap(&mut self, x: f32, y: f32) -> Vec<String> {
        let (Some(comp), Some(surface)) = (&self.composition, &self.surface) else {
            return Vec::new();
        };
        let root_mat = self.compute_root_matrix(comp, surface.width(), surface.height());
        let ctx = RenderContext {
            composition: comp,
            images: &self.images,
            video_frames: &self.video_frames,
            font_manager: &self.font_manager,
            props: &self.props,
        };
        let hits = hit_test_layers(&comp.layers, &root_mat, self.animator.current_frame(), x, y, &ctx);
        if !hits.is_empty() {
            let mut params = EventParamMap::new();
            params.insert(
                EventKeys::LAYER_LIST.to_string(),
                EventParamValue::StringVector(hits.clone()),
            );
            self.emit(Event::TapLayer, params);
        }
        hits
    }

    // --- Dynamic Properties & Subscriptions ---
    pub fn update_properties(&mut self, req: PropertyUpdateRequest) -> PropertyUpdateResponse {
        let mut count = 0;
        for ov in req.layer_updates {
            self.props.apply_override(ov);
            count += 1;
        }
        self.render_current_frame();
        PropertyUpdateResponse {
            success: true,
            updated_count: count,
            error_message: None,
        }
    }

    pub fn subscribe_update_event(&mut self, frame: i32) {
        self.subscribed_frames.insert(frame);
    }

    pub fn unsubscribe_update_event(&mut self, frame: i32) {
        self.subscribed_frames.remove(&frame);
    }

    /// Advances playback to `timestamp_ns` (VSync monotonic timestamp) and renders if a new frame is due.
    pub fn tick(&mut self, timestamp_ns: i64) -> bool {
        if !self.visibility.is_visible() {
            return false;
        }
        let res = self.animator.on_frame(timestamp_ns);
        if !res.rendered {
            return false;
        }
        if res.started {
            self.emit(Event::Start, EventParamMap::new());
        }
        if let Some(loop_idx) = res.repeated {
            let mut params = EventParamMap::new();
            params.insert(
                EventKeys::LOOP_INDEX.to_string(),
                EventParamValue::Int32(loop_idx),
            );
            self.emit(Event::Repeat, params);
        }

        self.render_current_frame();

        let frame_i = res.current_frame.round() as i32;
        if self.subscribed_frames.contains(&frame_i) && self.last_emitted_frame != Some(frame_i) {
            self.last_emitted_frame = Some(frame_i);
            let mut params = EventParamMap::new();
            params.insert(EventKeys::CURRENT.to_string(), EventParamValue::Int32(frame_i));
            if let Some(c) = &self.composition {
                params.insert(
                    EventKeys::TOTAL.to_string(),
                    EventParamValue::Int32(c.duration_frames().round() as i32),
                );
            }
            self.emit(Event::Update, params);
        }

        if res.completed {
            self.emit(Event::Completion, EventParamMap::new());
        }
        true
    }

    /// Renders the current frame immediately into the internal pure Rust `animax-skity::Surface`.
    pub fn render_current_frame(&mut self) -> bool {
        let Some(comp) = &self.composition else {
            return false;
        };
        let Some(surface) = &mut self.surface else {
            return false;
        };
        let sw = surface.width();
        let sh = surface.height();
        let root_mat = compute_root_matrix_helper(
            comp,
            sw,
            sh,
            self.device_scale,
            self.object_fit,
            self.object_position,
        );
        surface.clear();

        let ctx = RenderContext {
            composition: comp,
            images: &self.images,
            video_frames: &self.video_frames,
            font_manager: &self.font_manager,
            props: &self.props,
        };
        draw_layer_list(
            &comp.layers,
            surface.canvas_mut(),
            &root_mat,
            1.0,
            self.animator.current_frame(),
            &ctx,
        );

        if !self.first_frame_flushed {
            self.first_frame_flushed = true;
            self.emit(Event::FirstFrame, EventParamMap::new());
        }
        true
    }

    pub fn pixels(&mut self, format: PixelFormat) -> Option<&[u8]> {
        self.surface.as_mut().map(|s| s.pixels_as(format))
    }

    pub fn drain_events(&mut self) -> Vec<PlayerEvent> {
        std::mem::take(&mut self.events)
    }

    fn emit(&mut self, event: Event, params: EventParamMap) {
        self.events.push(PlayerEvent { event, params });
    }

    fn compute_root_matrix(&self, comp: &CompositionModel, sw: u32, sh: u32) -> Matrix {
        compute_root_matrix_helper(
            comp,
            sw,
            sh,
            self.device_scale,
            self.object_fit,
            self.object_position,
        )
    }
}

fn compute_root_matrix_helper(
    comp: &CompositionModel,
    sw: u32,
    sh: u32,
    device_scale: f32,
    fit: ObjectFit,
    pos: ObjectPosition,
) -> Matrix {
    let tf = compute_fit_transform(
        comp.width(),
        comp.height(),
        sw as f32,
        sh as f32,
        device_scale,
        fit,
        pos,
    );
    let mut m = Matrix::identity();
    m.pre_translate(tf.translate_x, tf.translate_y);
    m.pre_scale(tf.scale_x, tf.scale_y);
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    use animax_base::Color;
    use animax_property::{AnimaXKeyPath, AnimaXValueParam};

    const TEST_LOTTIE: &str = r#"{
        "v": "5.7.4",
        "fr": 60,
        "ip": 0,
        "op": 60,
        "w": 100,
        "h": 100,
        "layers": [{
            "ty": 4,
            "ind": 1,
            "nm": "BoxLayer",
            "ip": 0,
            "op": 60,
            "ks": {
                "o": {"a": 0, "k": 100},
                "p": {"a": 1, "k": [
                    {"t": 0, "s": [25, 50], "e": [75, 50]},
                    {"t": 60, "s": [75, 50]}
                ]},
                "s": {"a": 0, "k": [100, 100]}
            },
            "shapes": [{
                "ty": "gr",
                "nm": "Group",
                "it": [
                    {"ty": "rc", "nm": "Rect", "p": {"a": 0, "k": [0, 0]}, "s": {"a": 0, "k": [40, 40]}, "r": {"a": 0, "k": 0}},
                    {"ty": "fl", "nm": "Fill", "c": {"a": 0, "k": [1, 0, 0, 1]}, "o": {"a": 0, "k": 100}}
                ]
            }]
        }]
    }"#;

    #[test]
    fn end_to_end_lottie_render_animate_hittest_and_dynamic_property() {
        let mut player = AnimaXPlayer::new(1.0);
        assert!(player.set_surface_size(100, 100));
        player.set_json(TEST_LOTTIE).expect("valid lottie");

        // Frame 0: red square centered at (25, 50), size 40x40 -> pixel (25, 50) is red!
        let px0 = player.pixels(PixelFormat::Rgba8888Premul).unwrap();
        let off_left = (50 * 100 + 25) * 4;
        assert_eq!(&px0[off_left..off_left + 4], &[255, 0, 0, 255]);

        // Hit-test at (25, 50) hits "BoxLayer"
        let hits = player.on_tap(25.0, 50.0);
        assert_eq!(hits, vec!["BoxLayer"]);

        // Tick 0.5s -> frame 30 -> square moves to center (50, 50)
        assert!(player.tick(0));
        assert!(player.tick(500_000_000));
        let px_mid = player.pixels(PixelFormat::Rgba8888Premul).unwrap();
        let off_center = (50 * 100 + 50) * 4;
        assert_eq!(&px_mid[off_center..off_center + 4], &[255, 0, 0, 255]);

        // Dynamic property override: recolor Fill to pure green!
        let mut req = PropertyUpdateRequest::default();
        req.add_layer_property(
            AnimaXKeyPath::from_slices(&["BoxLayer", "Group", "Fill"]),
            animax_property::LayerPropertyType::Color,
            AnimaXValueParam::from_color(
                Color::pack_argb(255, 0, 255, 0),
                AnimaXValueParam::FRAME_INDEX_ALL,
            ),
        );
        let resp = player.update_properties(req);
        assert!(resp.success);
        let px_green = player.pixels(PixelFormat::Rgba8888Premul).unwrap();
        assert_eq!(&px_green[off_center..off_center + 4], &[0, 255, 0, 255]);
    }

    #[test]
    fn end_to_end_alpha_video_compositing() {
        let cfg = r#"{
            "portrait": {
                "path": "demo.mp4",
                "v": 1,
                "f": 30,
                "videoW": 4,
                "videoH": 2,
                "w": 2,
                "h": 2,
                "rgbFrame": [0, 0, 2, 2],
                "aFrame": [2, 0, 2, 2]
            }
        }"#;
        let mut player = AnimaXPlayer::new(1.0);
        assert!(player.set_surface_size(2, 2));
        player.set_json(cfg).expect("alpha config");

        // 4x2 RGBA frame: left 2x2 is pure blue RGB(0,0,255), right 2x2 has R=128 alpha
        let mut frame = vec![0u8; 4 * 2 * 4];
        for y in 0..2 {
            for x in 0..2 {
                let rgb_o = (y * 4 + x) * 4;
                frame[rgb_o..rgb_o + 4].copy_from_slice(&[0, 0, 255, 255]);
                let alp_o = (y * 4 + (x + 2)) * 4;
                frame[alp_o..alp_o + 4].copy_from_slice(&[128, 128, 128, 255]);
            }
        }
        assert!(player.submit_video_frame_rgba("0", 4, 2, &frame));
        assert!(player.render_current_frame());
        let px = player.pixels(PixelFormat::Rgba8888Premul).unwrap();
        assert_eq!(px[0], 0);
        assert_eq!(px[1], 0);
        assert!((px[2] as i32 - 128).abs() <= 1);
        assert_eq!(px[3], 128);
    }
}
