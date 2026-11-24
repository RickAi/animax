// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

use serde::{Deserialize, Serialize};

/// Object fit modes matching `lynx::animax::ObjectFit`.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ObjectFit {
    Center = 0,
    Cover = 1,
    #[default]
    Contain = 2,
    Fill = 3,
    ScaleDown = 4,
}

impl ObjectFit {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Center,
            1 => Self::Cover,
            2 => Self::Contain,
            3 => Self::Fill,
            4 => Self::ScaleDown,
            _ => Self::Contain,
        }
    }
}

/// Object alignment modes matching `lynx::animax::ObjectPosition`.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ObjectPosition {
    #[default]
    Center = 0,
    Left = 1,
    Right = 2,
    Top = 3,
    Bottom = 4,
    TopLeft = 5,
    TopRight = 6,
    BottomLeft = 7,
    BottomRight = 8,
}

impl ObjectPosition {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Center,
            1 => Self::Left,
            2 => Self::Right,
            3 => Self::Top,
            4 => Self::Bottom,
            5 => Self::TopLeft,
            6 => Self::TopRight,
            7 => Self::BottomLeft,
            8 => Self::BottomRight,
            _ => Self::Center,
        }
    }
}

/// Layer bounds coordinate space matching `lynx::animax::LayerBoundsSpace`.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LayerBoundsSpace {
    #[default]
    Parent = 0,
    Root = 1,
}

/// Computed 2D scale + translation mapping composition coordinates to surface pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FitTransform {
    pub scale_x: f32,
    pub scale_y: f32,
    pub translate_x: f32,
    pub translate_y: f32,
}

/// Computes `(scale_x, scale_y, translate_x, translate_y)` for rendering a composition
/// of size `(comp_w, comp_h)` onto a surface of size `(surf_w, surf_h)`.
pub fn compute_fit_transform(
    comp_w: f32,
    comp_h: f32,
    surf_w: f32,
    surf_h: f32,
    device_scale: f32,
    fit: ObjectFit,
    position: ObjectPosition,
) -> FitTransform {
    if comp_w <= 0.0 || comp_h <= 0.0 || surf_w <= 0.0 || surf_h <= 0.0 {
        return FitTransform {
            scale_x: 1.0,
            scale_y: 1.0,
            translate_x: 0.0,
            translate_y: 0.0,
        };
    }

    let ratio_x = surf_w / comp_w;
    let ratio_y = surf_h / comp_h;
    let dpr = if device_scale > 0.0 { device_scale } else { 1.0 };

    let (scale_x, scale_y) = match fit {
        ObjectFit::Center => (dpr, dpr),
        ObjectFit::Cover => {
            let s = ratio_x.max(ratio_y);
            (s, s)
        }
        ObjectFit::Contain => {
            let s = ratio_x.min(ratio_y);
            (s, s)
        }
        ObjectFit::Fill => (ratio_x, ratio_y),
        ObjectFit::ScaleDown => {
            let s = ratio_x.min(ratio_y).min(dpr);
            (s, s)
        }
    };

    let drawn_w = comp_w * scale_x;
    let drawn_h = comp_h * scale_y;
    let dx = surf_w - drawn_w;
    let dy = surf_h - drawn_h;

    let (translate_x, translate_y) = match position {
        ObjectPosition::Center => (dx * 0.5, dy * 0.5),
        ObjectPosition::Left => (0.0, dy * 0.5),
        ObjectPosition::Right => (dx, dy * 0.5),
        ObjectPosition::Top => (dx * 0.5, 0.0),
        ObjectPosition::Bottom => (dx * 0.5, dy),
        ObjectPosition::TopLeft => (0.0, 0.0),
        ObjectPosition::TopRight => (dx, 0.0),
        ObjectPosition::BottomLeft => (0.0, dy),
        ObjectPosition::BottomRight => (dx, dy),
    };

    FitTransform {
        scale_x,
        scale_y,
        translate_x,
        translate_y,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fit_contain_center() {
        let tf = compute_fit_transform(100.0, 50.0, 200.0, 200.0, 1.0, ObjectFit::Contain, ObjectPosition::Center);
        assert!((tf.scale_x - 2.0).abs() < 1e-6);
        assert!((tf.scale_y - 2.0).abs() < 1e-6);
        assert!((tf.translate_x - 0.0).abs() < 1e-6);
        assert!((tf.translate_y - 50.0).abs() < 1e-6);
    }

    #[test]
    fn test_fit_cover_top_left() {
        let tf = compute_fit_transform(100.0, 50.0, 200.0, 200.0, 1.0, ObjectFit::Cover, ObjectPosition::TopLeft);
        assert!((tf.scale_x - 4.0).abs() < 1e-6);
        assert!((tf.translate_x - 0.0).abs() < 1e-6);
        assert!((tf.translate_y - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_fit_scale_down() {
        let tf = compute_fit_transform(100.0, 100.0, 400.0, 400.0, 1.0, ObjectFit::ScaleDown, ObjectPosition::BottomRight);
        assert!((tf.scale_x - 1.0).abs() < 1e-6);
        assert!((tf.translate_x - 300.0).abs() < 1e-6);
        assert!((tf.translate_y - 300.0).abs() < 1e-6);
    }
}
