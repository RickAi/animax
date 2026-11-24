// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

use animax_base::Color;
use animax_model::{BlendMode, PointF};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PaintStyle {
    #[default]
    Fill,
    Stroke,
    StrokeAndFill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StrokeCap {
    #[default]
    Butt,
    Round,
    Square,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StrokeJoin {
    #[default]
    Miter,
    Round,
    Bevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TileMode {
    #[default]
    Clamp,
    Repeat,
    Mirror,
    Decal,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Shader {
    LinearGradient {
        start: PointF,
        end: PointF,
        colors: Vec<Color>,
        positions: Vec<f32>,
        tile_mode: TileMode,
    },
    RadialGradient {
        center: PointF,
        radius: f32,
        colors: Vec<Color>,
        positions: Vec<f32>,
        tile_mode: TileMode,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum PathEffect {
    Dash { intervals: Vec<f32>, phase: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MaskFilter {
    Blur { radius: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorFilter {
    pub color: Color,
    pub mode: BlendMode,
}

/// Pure Rust paint configuration matching `skity::Paint`.
#[derive(Debug, Clone, PartialEq)]
pub struct Paint {
    pub color: Color,
    pub style: PaintStyle,
    pub stroke_width: f32,
    pub stroke_cap: StrokeCap,
    pub stroke_join: StrokeJoin,
    pub stroke_miter: f32,
    pub blend_mode: BlendMode,
    pub shader: Option<Shader>,
    pub path_effect: Option<PathEffect>,
    pub mask_filter: Option<MaskFilter>,
    pub color_filter: Option<ColorFilter>,
    pub anti_alias: bool,
}

impl Default for Paint {
    fn default() -> Self {
        Self {
            color: Color::BLACK,
            style: PaintStyle::Fill,
            stroke_width: 1.0,
            stroke_cap: StrokeCap::Butt,
            stroke_join: StrokeJoin::Miter,
            stroke_miter: 4.0,
            blend_mode: BlendMode::SrcOver,
            shader: None,
            path_effect: None,
            mask_filter: None,
            color_filter: None,
            anti_alias: true,
        }
    }
}

impl Paint {
    pub fn set_alpha(&mut self, alpha: u8) {
        self.color.a = alpha;
    }

    pub fn set_alpha_f(&mut self, alpha: f32) {
        self.color.a = (alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
    }

    pub fn alpha_f(&self) -> f32 {
        self.color.a as f32 / 255.0
    }
}
