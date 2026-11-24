// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

use crate::geometry::Matrix;
use crate::paint::{
    MaskFilter, Paint, PaintStyle, PathEffect, Shader, StrokeCap, StrokeJoin, TileMode,
};
use crate::path::{FillType, Path};
use crate::pixmap::Pixmap;
use crate::text::Font;
use animax_base::Color;
use animax_model::{BlendMode, RectF};

struct LayerEntry {
    pixmap: tiny_skia::Pixmap,
    paint: Paint,
}

struct SaveState {
    matrix: Matrix,
    clip_mask: Option<tiny_skia::Mask>,
    layer_depth: usize,
}

/// Pure Rust 2D Canvas matching `skity::Canvas` + `lynx::animax::Canvas`.
pub struct Canvas {
    width: u32,
    height: u32,
    layers: Vec<LayerEntry>,
    states: Vec<SaveState>,
    current_matrix: Matrix,
    current_clip: Option<tiny_skia::Mask>,
}

impl Canvas {
    pub fn new(width: u32, height: u32) -> Option<Self> {
        let w = width.max(1);
        let h = height.max(1);
        let root = tiny_skia::Pixmap::new(w, h)?;
        Some(Self {
            width: w,
            height: h,
            layers: vec![LayerEntry {
                pixmap: root,
                paint: Paint::default(),
            }],
            states: Vec::new(),
            current_matrix: Matrix::identity(),
            current_clip: None,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn clear(&mut self, color: Color) {
        if let Some(top) = self.layers.last_mut() {
            top.pixmap.fill(to_skia_color(color));
        }
    }

    pub fn pixels(&self) -> &[u8] {
        self.layers[0].pixmap.data()
    }

    pub fn to_pixmap(&self) -> Pixmap {
        Pixmap {
            inner: self.layers[0].pixmap.clone(),
        }
    }

    pub fn save(&mut self) {
        self.states.push(SaveState {
            matrix: self.current_matrix,
            clip_mask: self.current_clip.clone(),
            layer_depth: self.layers.len(),
        });
    }

    pub fn save_layer(&mut self, _bounds: Option<RectF>, paint: &Paint) {
        self.states.push(SaveState {
            matrix: self.current_matrix,
            clip_mask: self.current_clip.clone(),
            layer_depth: self.layers.len(),
        });
        if let Some(offscreen) = tiny_skia::Pixmap::new(self.width, self.height) {
            self.layers.push(LayerEntry {
                pixmap: offscreen,
                paint: paint.clone(),
            });
        }
    }

    pub fn restore(&mut self) {
        let Some(state) = self.states.pop() else {
            return;
        };
        while self.layers.len() > state.layer_depth {
            if let Some(mut finished) = self.layers.pop() {
                if let Some(MaskFilter::Blur { radius }) = finished.paint.mask_filter {
                    let mut px = Pixmap {
                        inner: finished.pixmap,
                    };
                    px.apply_blur(radius);
                    finished.pixmap = px.inner;
                }
                if let Some(cf) = finished.paint.color_filter {
                    apply_color_filter_pixmap(&mut finished.pixmap, cf.color, cf.mode);
                }
                if let Some(parent) = self.layers.last_mut() {
                    let pp = tiny_skia::PixmapPaint {
                        opacity: finished.paint.alpha_f(),
                        blend_mode: to_skia_blend_mode(finished.paint.blend_mode),
                        quality: tiny_skia::FilterQuality::Nearest,
                    };
                    parent.pixmap.draw_pixmap(
                        0,
                        0,
                        finished.pixmap.as_ref(),
                        &pp,
                        tiny_skia::Transform::identity(),
                        state.clip_mask.as_ref(),
                    );
                }
            }
        }
        self.current_matrix = state.matrix;
        self.current_clip = state.clip_mask;
    }

    pub fn reset_matrix(&mut self) {
        self.current_matrix.reset();
    }

    pub fn set_matrix(&mut self, matrix: Matrix) {
        self.current_matrix = matrix;
    }

    pub fn matrix(&self) -> Matrix {
        self.current_matrix
    }

    pub fn concat(&mut self, matrix: &Matrix) {
        self.current_matrix.pre_concat(matrix);
    }

    pub fn translate(&mut self, dx: f32, dy: f32) {
        self.current_matrix.pre_translate(dx, dy);
    }

    pub fn scale(&mut self, sx: f32, sy: f32) {
        self.current_matrix.pre_scale(sx, sy);
    }

    pub fn clip_rect(&mut self, rect: RectF) -> bool {
        let mut p = Path::new();
        p.add_rect(rect);
        self.clip_path(&p, false)
    }

    pub fn clip_path(&mut self, path: &Path, anti_alias: bool) -> bool {
        let Some(sk_path) = path.to_tiny_skia_path() else {
            return false;
        };
        let fill_rule = to_skia_fill_rule(path.fill_type);
        let ts = self.current_matrix.to_tiny_skia();
        match &mut self.current_clip {
            Some(existing) => {
                existing.intersect_path(&sk_path, fill_rule, anti_alias, ts);
            }
            None => {
                if let Some(mut mask) = tiny_skia::Mask::new(self.width, self.height) {
                    mask.fill_path(&sk_path, fill_rule, anti_alias, ts);
                    self.current_clip = Some(mask);
                }
            }
        }
        true
    }

    pub fn draw_rect(&mut self, rect: RectF, paint: &Paint) {
        let mut p = Path::new();
        p.add_rect(rect);
        self.draw_path(&p, paint);
    }

    pub fn draw_path(&mut self, path: &Path, paint: &Paint) {
        if path.is_empty() || paint.color.a == 0 && paint.shader.is_none() {
            return;
        }
        let Some(sk_path) = path.to_tiny_skia_path() else {
            return;
        };
        let sk_paint = build_tiny_skia_paint(paint);
        let ts = self.current_matrix.to_tiny_skia();
        let fill_rule = to_skia_fill_rule(path.fill_type);

        // If blur mask filter is requested on a single draw call, route through a temporary layer.
        if let Some(MaskFilter::Blur { radius }) = paint.mask_filter {
            if radius > 0.5 {
                let mut layer_paint = Paint::default();
                layer_paint.mask_filter = Some(MaskFilter::Blur { radius });
                layer_paint.blend_mode = paint.blend_mode;
                self.save_layer(None, &layer_paint);
                let mut inner_paint = paint.clone();
                inner_paint.mask_filter = None;
                inner_paint.blend_mode = BlendMode::SrcOver;
                self.draw_path(path, &inner_paint);
                self.restore();
                return;
            }
        }

        let Some(target) = self.layers.last_mut() else {
            return;
        };

        match paint.style {
            PaintStyle::Fill | PaintStyle::StrokeAndFill => {
                target.pixmap.fill_path(
                    &sk_path,
                    &sk_paint,
                    fill_rule,
                    ts,
                    self.current_clip.as_ref(),
                );
            }
            PaintStyle::Stroke => {}
        }

        if matches!(paint.style, PaintStyle::Stroke | PaintStyle::StrokeAndFill)
            && paint.stroke_width > 0.0
        {
            let stroke = build_tiny_skia_stroke(paint);
            target.pixmap.stroke_path(
                &sk_path,
                &sk_paint,
                &stroke,
                ts,
                self.current_clip.as_ref(),
            );
        }
    }

    pub fn draw_image_rect(&mut self, image: &Pixmap, src: RectF, dst: RectF, paint: &Paint) {
        if src.is_empty() || dst.is_empty() || paint.color.a == 0 {
            return;
        }
        let sx = dst.width() / src.width();
        let sy = dst.height() / src.height();
        let mut mat = self.current_matrix;
        mat.pre_translate(dst.left - src.left * sx, dst.top - src.top * sy);
        mat.pre_scale(sx, sy);

        let pp = tiny_skia::PixmapPaint {
            opacity: paint.alpha_f(),
            blend_mode: to_skia_blend_mode(paint.blend_mode),
            quality: tiny_skia::FilterQuality::Bilinear,
        };
        if let Some(target) = self.layers.last_mut() {
            target.pixmap.draw_pixmap(
                0,
                0,
                image.inner.as_ref(),
                &pp,
                mat.to_tiny_skia(),
                self.current_clip.as_ref(),
            );
        }
    }

    /// Rasterizes a UTF-8 text run using a `fontdue` font onto the canvas.
    pub fn draw_text(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        font: &Font,
        px_size: f32,
        tracking: f32,
        paint: &Paint,
    ) {
        if text.is_empty() || px_size <= 0.0 || paint.color.a == 0 {
            return;
        }
        let mut cursor_x = x;
        for ch in text.chars() {
            let (metrics, bitmap) = font.rasterize(ch, px_size);
            if metrics.width > 0 && metrics.height > 0 {
                let mut glyph_data = vec![0u8; metrics.width * metrics.height * 4];
                let cr = paint.color.r as u16;
                let cg = paint.color.g as u16;
                let cb = paint.color.b as u16;
                let ca = paint.color.a as u16;
                for (i, &cov) in bitmap.iter().enumerate() {
                    let a = (cov as u16 * ca + 127) / 255;
                    let off = i * 4;
                    glyph_data[off] = ((cr * a + 127) / 255) as u8;
                    glyph_data[off + 1] = ((cg * a + 127) / 255) as u8;
                    glyph_data[off + 2] = ((cb * a + 127) / 255) as u8;
                    glyph_data[off + 3] = a as u8;
                }
                if let Some(glyph_px) = Pixmap::from_premultiplied_rgba(
                    metrics.width as u32,
                    metrics.height as u32,
                    glyph_data,
                ) {
                    let gx = cursor_x + metrics.xmin as f32;
                    let gy = y - (metrics.height as f32 + metrics.ymin as f32);
                    let src = RectF::from_wh(metrics.width as f32, metrics.height as f32);
                    let dst = RectF::new(
                        gx,
                        gy,
                        gx + metrics.width as f32,
                        gy + metrics.height as f32,
                    );
                    let mut gp = Paint::default();
                    gp.color = Color::WHITE;
                    gp.blend_mode = paint.blend_mode;
                    self.draw_image_rect(&glyph_px, src, dst, &gp);
                }
            }
            cursor_x += metrics.advance_width + tracking;
        }
    }

    /// Applies a luma-to-alpha conversion on the top layer (used for Lottie Luma / LumaInverted mattes).
    pub fn convert_top_layer_to_luma_alpha(&mut self, inverted: bool) {
        if let Some(top) = self.layers.last_mut() {
            for px in top.pixmap.data_mut().chunks_exact_mut(4) {
                let r = px[0] as f32;
                let g = px[1] as f32;
                let b = px[2] as f32;
                let mut luma = (0.2126 * r + 0.7152 * g + 0.0722 * b).clamp(0.0, 255.0) as u8;
                if inverted {
                    luma = 255 - luma;
                }
                px[0] = luma;
                px[1] = luma;
                px[2] = luma;
                px[3] = luma;
            }
        }
    }
}

fn to_skia_color(c: Color) -> tiny_skia::Color {
    tiny_skia::Color::from_rgba8(c.r, c.g, c.b, c.a)
}

fn to_skia_fill_rule(ft: FillType) -> tiny_skia::FillRule {
    match ft {
        FillType::Winding => tiny_skia::FillRule::Winding,
        FillType::EvenOdd => tiny_skia::FillRule::EvenOdd,
    }
}

fn to_skia_spread(tm: TileMode) -> tiny_skia::SpreadMode {
    match tm {
        TileMode::Clamp | TileMode::Decal => tiny_skia::SpreadMode::Pad,
        TileMode::Repeat => tiny_skia::SpreadMode::Repeat,
        TileMode::Mirror => tiny_skia::SpreadMode::Reflect,
    }
}

fn to_skia_blend_mode(bm: BlendMode) -> tiny_skia::BlendMode {
    match bm {
        BlendMode::Clear => tiny_skia::BlendMode::Clear,
        BlendMode::Src => tiny_skia::BlendMode::Source,
        BlendMode::Dst => tiny_skia::BlendMode::Destination,
        BlendMode::SrcOver => tiny_skia::BlendMode::SourceOver,
        BlendMode::DstOver => tiny_skia::BlendMode::DestinationOver,
        BlendMode::SrcIn => tiny_skia::BlendMode::SourceIn,
        BlendMode::DstIn => tiny_skia::BlendMode::DestinationIn,
        BlendMode::SrcOut => tiny_skia::BlendMode::SourceOut,
        BlendMode::DstOut => tiny_skia::BlendMode::DestinationOut,
        BlendMode::SrcATop => tiny_skia::BlendMode::SourceAtop,
        BlendMode::DstATop => tiny_skia::BlendMode::DestinationAtop,
        BlendMode::Xor => tiny_skia::BlendMode::Xor,
        BlendMode::Plus => tiny_skia::BlendMode::Plus,
        BlendMode::Modulate => tiny_skia::BlendMode::Modulate,
        BlendMode::Screen => tiny_skia::BlendMode::Screen,
        BlendMode::Overlay => tiny_skia::BlendMode::Overlay,
        BlendMode::Darken => tiny_skia::BlendMode::Darken,
        BlendMode::Lighten => tiny_skia::BlendMode::Lighten,
        BlendMode::ColorDodge => tiny_skia::BlendMode::ColorDodge,
        BlendMode::ColorBurn => tiny_skia::BlendMode::ColorBurn,
        BlendMode::HardLight => tiny_skia::BlendMode::HardLight,
        BlendMode::SoftLight => tiny_skia::BlendMode::SoftLight,
        BlendMode::Difference => tiny_skia::BlendMode::Difference,
        BlendMode::Exclusion => tiny_skia::BlendMode::Exclusion,
        BlendMode::Multiply => tiny_skia::BlendMode::Multiply,
        BlendMode::Hue => tiny_skia::BlendMode::Hue,
        BlendMode::Saturation => tiny_skia::BlendMode::Saturation,
        BlendMode::Color => tiny_skia::BlendMode::Color,
        BlendMode::Luminosity => tiny_skia::BlendMode::Luminosity,
    }
}

fn build_tiny_skia_paint(paint: &Paint) -> tiny_skia::Paint<'static> {
    let mut sk = tiny_skia::Paint::default();
    sk.anti_alias = paint.anti_alias;
    sk.blend_mode = to_skia_blend_mode(paint.blend_mode);

    let alpha_scale = paint.alpha_f();
    if let Some(shader) = &paint.shader {
        match shader {
            Shader::LinearGradient {
                start,
                end,
                colors,
                positions,
                tile_mode,
            } => {
                let stops = build_gradient_stops(colors, positions, alpha_scale);
                if let Some(grad) = tiny_skia::LinearGradient::new(
                    tiny_skia::Point::from_xy(start.x, start.y),
                    tiny_skia::Point::from_xy(end.x, end.y),
                    stops,
                    to_skia_spread(*tile_mode),
                    tiny_skia::Transform::identity(),
                ) {
                    sk.shader = grad;
                    return sk;
                }
            }
            Shader::RadialGradient {
                center,
                radius,
                colors,
                positions,
                tile_mode,
            } => {
                let stops = build_gradient_stops(colors, positions, alpha_scale);
                let pt = tiny_skia::Point::from_xy(center.x, center.y);
                if let Some(grad) = tiny_skia::RadialGradient::new(
                    pt,
                    pt,
                    radius.max(0.001),
                    stops,
                    to_skia_spread(*tile_mode),
                    tiny_skia::Transform::identity(),
                ) {
                    sk.shader = grad;
                    return sk;
                }
            }
        }
    }

    let c = if let Some(cf) = paint.color_filter {
        match cf.mode {
            BlendMode::SrcATop | BlendMode::SrcIn | BlendMode::Src => Color::from_argb(
                ((paint.color.a as u16 * cf.color.a as u16) / 255) as u8,
                cf.color.r,
                cf.color.g,
                cf.color.b,
            ),
            _ => paint.color,
        }
    } else {
        paint.color
    };
    sk.set_color(to_skia_color(c));
    sk
}

fn build_gradient_stops(
    colors: &[Color],
    positions: &[f32],
    alpha_scale: f32,
) -> Vec<tiny_skia::GradientStop> {
    let n = colors.len().min(positions.len());
    let mut stops = Vec::with_capacity(n.max(2));
    for i in 0..n {
        let mut c = colors[i];
        c.a = ((c.a as f32) * alpha_scale).clamp(0.0, 255.0).round() as u8;
        stops.push(tiny_skia::GradientStop::new(
            positions[i].clamp(0.0, 1.0),
            to_skia_color(c),
        ));
    }
    if stops.is_empty() {
        stops.push(tiny_skia::GradientStop::new(0.0, tiny_skia::Color::TRANSPARENT));
        stops.push(tiny_skia::GradientStop::new(1.0, tiny_skia::Color::TRANSPARENT));
    } else if stops.len() == 1 {
        let only = stops[0];
        stops.push(only);
    }
    stops
}

fn build_tiny_skia_stroke(paint: &Paint) -> tiny_skia::Stroke {
    let mut stroke = tiny_skia::Stroke {
        width: paint.stroke_width,
        miter_limit: paint.stroke_miter,
        line_cap: match paint.stroke_cap {
            StrokeCap::Butt => tiny_skia::LineCap::Butt,
            StrokeCap::Round => tiny_skia::LineCap::Round,
            StrokeCap::Square => tiny_skia::LineCap::Square,
        },
        line_join: match paint.stroke_join {
            StrokeJoin::Miter => tiny_skia::LineJoin::Miter,
            StrokeJoin::Round => tiny_skia::LineJoin::Round,
            StrokeJoin::Bevel => tiny_skia::LineJoin::Bevel,
        },
        dash: None,
    };
    if let Some(PathEffect::Dash { intervals, phase }) = &paint.path_effect {
        if !intervals.is_empty() && intervals.iter().all(|&v| v >= 0.0) && intervals.iter().any(|&v| v > 0.0) {
            stroke.dash = tiny_skia::StrokeDash::new(intervals.clone(), *phase);
        }
    }
    stroke
}

fn apply_color_filter_pixmap(pixmap: &mut tiny_skia::Pixmap, color: Color, mode: BlendMode) {
    if !matches!(mode, BlendMode::SrcATop | BlendMode::SrcIn | BlendMode::Multiply) {
        return;
    }
    for px in pixmap.data_mut().chunks_exact_mut(4) {
        let a = px[3] as u16;
        if a == 0 {
            continue;
        }
        match mode {
            BlendMode::SrcATop | BlendMode::SrcIn => {
                let fa = (a * color.a as u16 + 127) / 255;
                px[0] = ((color.r as u16 * fa + 127) / 255) as u8;
                px[1] = ((color.g as u16 * fa + 127) / 255) as u8;
                px[2] = ((color.b as u16 * fa + 127) / 255) as u8;
                px[3] = fa as u8;
            }
            BlendMode::Multiply => {
                px[0] = ((px[0] as u16 * color.r as u16 + 127) / 255) as u8;
                px[1] = ((px[1] as u16 * color.g as u16 + 127) / 255) as u8;
                px[2] = ((px[2] as u16 * color.b as u16 + 127) / 255) as u8;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canvas_draws_rect_and_save_layer_opacity() {
        let mut canvas = Canvas::new(64, 64).unwrap();
        canvas.clear(Color::TRANSPARENT);

        let mut layer_paint = Paint::default();
        layer_paint.set_alpha(128);
        canvas.save_layer(None, &layer_paint);

        let mut fill = Paint::default();
        fill.color = Color::from_rgb(255, 0, 0);
        canvas.draw_rect(RectF::new(10.0, 10.0, 50.0, 50.0), &fill);
        canvas.restore();

        // Center pixel (30, 30) should have ~50% premultiplied red
        let off = (30 * 64 + 30) * 4;
        let px = &canvas.pixels()[off..off + 4];
        assert!((px[0] as i32 - 128).abs() <= 2);
        assert_eq!(px[1], 0);
        assert_eq!(px[2], 0);
        assert!((px[3] as i32 - 128).abs() <= 2);
    }
}
