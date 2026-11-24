// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

use animax_base::{gamma_evaluate, lerp_f32};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct PointF {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl PointF {
    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y, z: 0.0 }
    }

    #[inline]
    pub const fn new_3d(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    #[inline]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            x: lerp_f32(self.x, other.x, t),
            y: lerp_f32(self.y, other.y, t),
            z: lerp_f32(self.z, other.z, t),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ScaleF {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Default for ScaleF {
    fn default() -> Self {
        Self { x: 1.0, y: 1.0, z: 1.0 }
    }
}

impl ScaleF {
    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y, z: 1.0 }
    }

    #[inline]
    pub const fn new_3d(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    #[inline]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            x: lerp_f32(self.x, other.x, t),
            y: lerp_f32(self.y, other.y, t),
            z: lerp_f32(self.z, other.z, t),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct RectF {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl RectF {
    #[inline]
    pub const fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self { left, top, right, bottom }
    }

    #[inline]
    pub const fn from_wh(width: f32, height: f32) -> Self {
        Self { left: 0.0, top: 0.0, right: width, bottom: height }
    }

    #[inline]
    pub fn width(&self) -> f32 {
        (self.right - self.left).max(0.0)
    }

    #[inline]
    pub fn height(&self) -> f32 {
        (self.bottom - self.top).max(0.0)
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.right <= self.left || self.bottom <= self.top
    }

    #[inline]
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.left && x <= self.right && y >= self.top && y <= self.bottom
    }

    #[inline]
    pub fn union(&mut self, other: &RectF) {
        if other.is_empty() {
            return;
        }
        if self.is_empty() {
            *self = *other;
            return;
        }
        self.left = self.left.min(other.left);
        self.top = self.top.min(other.top);
        self.right = self.right.max(other.right);
        self.bottom = self.bottom.max(other.bottom);
    }
}

/// Blend modes matching `lynx::animax::BlendModeType` and `skity::BlendMode`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BlendMode {
    Clear = 0,
    Src = 1,
    Dst = 2,
    #[default]
    SrcOver = 3,
    DstOver = 4,
    SrcIn = 5,
    DstIn = 6,
    SrcOut = 7,
    DstOut = 8,
    SrcATop = 9,
    DstATop = 10,
    Xor = 11,
    Plus = 12,
    Modulate = 13,
    Screen = 14,
    Overlay = 15,
    Darken = 16,
    Lighten = 17,
    ColorDodge = 18,
    ColorBurn = 19,
    HardLight = 20,
    SoftLight = 21,
    Difference = 22,
    Exclusion = 23,
    Multiply = 24,
    Hue = 25,
    Saturation = 26,
    Color = 27,
    Luminosity = 28,
}

impl BlendMode {
    /// Maps Lottie `bm` integer index to `BlendMode`.
    pub fn from_lottie_bm(bm: i32) -> Self {
        match bm {
            0 => Self::SrcOver,
            1 => Self::Multiply,
            2 => Self::Screen,
            3 => Self::Overlay,
            4 => Self::Darken,
            5 => Self::Lighten,
            6 => Self::ColorDodge,
            7 => Self::ColorBurn,
            8 => Self::HardLight,
            9 => Self::SoftLight,
            10 => Self::Difference,
            11 => Self::Exclusion,
            12 => Self::Hue,
            13 => Self::Saturation,
            14 => Self::Color,
            15 => Self::Luminosity,
            16 => Self::Plus,
            _ => Self::SrcOver,
        }
    }

    pub fn from_i32(v: i32) -> Self {
        match v {
            0 => Self::Clear,
            1 => Self::Src,
            2 => Self::Dst,
            3 => Self::SrcOver,
            4 => Self::DstOver,
            5 => Self::SrcIn,
            6 => Self::DstIn,
            7 => Self::SrcOut,
            8 => Self::DstOut,
            9 => Self::SrcATop,
            10 => Self::DstATop,
            11 => Self::Xor,
            12 => Self::Plus,
            13 => Self::Modulate,
            14 => Self::Screen,
            15 => Self::Overlay,
            16 => Self::Darken,
            17 => Self::Lighten,
            18 => Self::ColorDodge,
            19 => Self::ColorBurn,
            20 => Self::HardLight,
            21 => Self::SoftLight,
            22 => Self::Difference,
            23 => Self::Exclusion,
            24 => Self::Multiply,
            25 => Self::Hue,
            26 => Self::Saturation,
            27 => Self::Color,
            28 => Self::Luminosity,
            _ => Self::SrcOver,
        }
    }
}

/// Cubic Bezier segment for `ShapeData`.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct CubicCurve {
    pub cp1: PointF,
    pub cp2: PointF,
    pub vertex: PointF,
}

/// Bezier contour matching `lynx::animax::ShapeData`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ShapeData {
    pub initial_point: PointF,
    pub curves: Vec<CubicCurve>,
    pub closed: bool,
}

impl ShapeData {
    pub fn is_empty(&self) -> bool {
        self.curves.is_empty() && self.initial_point == PointF::default() && !self.closed
    }

    /// Interpolates between two `ShapeData` instances into `self`.
    pub fn interpolate_between(&mut self, s1: &ShapeData, s2: &ShapeData, percentage: f32) {
        self.closed = s1.closed || s2.closed;
        let point_size = s1.curves.len().min(s2.curves.len());
        self.initial_point = s1.initial_point.lerp(s2.initial_point, percentage);
        self.curves.clear();
        self.curves.reserve(point_size);
        for i in 0..point_size {
            let c1 = &s1.curves[i];
            let c2 = &s2.curves[i];
            self.curves.push(CubicCurve {
                cp1: c1.cp1.lerp(c2.cp1, percentage),
                cp2: c1.cp2.lerp(c2.cp2, percentage),
                vertex: c1.vertex.lerp(c2.vertex, percentage),
            });
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum GradientType {
    #[default]
    Linear = 0,
    Radial = 1,
    Conic = 2,
}

/// Gradient color stops matching `lynx::animax::GradientColor`, using
/// single-pass O(N1 + N2) two-pointer merge & sampling for mismatched stop counts.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GradientColor {
    pub positions: Vec<f32>,
    pub colors: Vec<i32>,
}

impl GradientColor {
    pub fn new(positions: Vec<f32>, colors: Vec<i32>) -> Self {
        let len = positions.len().min(colors.len());
        Self {
            positions: positions.into_iter().take(len).collect(),
            colors: colors.into_iter().take(len).collect(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.positions.is_empty() || self.colors.is_empty()
    }

    pub fn len(&self) -> usize {
        self.positions.len().min(self.colors.len())
    }

    /// Linearly interpolates between `gc1` and `gc2` at `progress`.
    /// Uses O(N) fast path when stop counts match and O(N1 + N2) two-pointer merge
    /// when stop counts differ (preserving duplicate hard stops).
    pub fn lerp_color(&mut self, gc1: &GradientColor, gc2: &GradientColor, progress: f32) {
        if gc1.is_empty() || gc2.is_empty() {
            return;
        }
        let n1 = gc1.len();
        let n2 = gc2.len();
        self.positions.clear();
        self.colors.clear();

        if n1 == n2 {
            self.positions.reserve(n1);
            self.colors.reserve(n1);
            for i in 0..n1 {
                self.positions
                    .push(lerp_f32(gc1.positions[i], gc2.positions[i], progress));
                self.colors
                    .push(gamma_evaluate(gc1.colors[i], gc2.colors[i], progress));
            }
            return;
        }

        let pos1 = &gc1.positions[..n1];
        let col1 = &gc1.colors[..n1];
        let pos2 = &gc2.positions[..n2];
        let col2 = &gc2.colors[..n2];

        self.positions.reserve(n1 + n2);
        self.colors.reserve(n1 + n2);

        if is_sorted_no_nan(pos1) && is_sorted_no_nan(pos2) {
            let mut i1 = 0usize;
            let mut i2 = 0usize;
            while i1 < n1 || i2 < n2 {
                let take_first = i2 >= n2 || (i1 < n1 && pos1[i1] < pos2[i2]);
                let position = if take_first { pos1[i1] } else { pos2[i2] };

                let s1 = i1;
                while i1 < n1 && pos1[i1] == position {
                    i1 += 1;
                }
                let count1 = i1 - s1;

                let s2 = i2;
                while i2 < n2 && pos2[i2] == position {
                    i2 += 1;
                }
                let count2 = i2 - s2;

                let count = count1.max(count2);
                let c1_interp = if count1 == 0 {
                    sample_between_sorted(pos1, col1, i1, position)
                } else {
                    0
                };
                let c2_interp = if count2 == 0 {
                    sample_between_sorted(pos2, col2, i2, position)
                } else {
                    0
                };

                for occ in 0..count {
                    let c1 = if count1 > 0 {
                        col1[s1 + occ.min(count1 - 1)]
                    } else {
                        c1_interp
                    };
                    let c2 = if count2 > 0 {
                        col2[s2 + occ.min(count2 - 1)]
                    } else {
                        c2_interp
                    };
                    self.positions.push(position);
                    self.colors.push(gamma_evaluate(c1, c2, progress));
                }
            }
            return;
        }

        // Fallback for unsorted or NaN inputs
        let mut i1 = 0usize;
        let mut i2 = 0usize;
        while i1 < n1 || i2 < n2 {
            let take_first = i2 >= n2 || (i1 < n1 && pos1[i1] < pos2[i2]);
            let position = if take_first { pos1[i1] } else { pos2[i2] };

            let mut count1 = 0usize;
            while i1 < n1 && pos1[i1] == position {
                count1 += 1;
                i1 += 1;
            }
            let mut count2 = 0usize;
            while i2 < n2 && pos2[i2] == position {
                count2 += 1;
                i2 += 1;
            }
            if count1 == 0 && count2 == 0 {
                if take_first && i1 < n1 {
                    count1 = 1;
                    i1 += 1;
                } else if i2 < n2 {
                    count2 = 1;
                    i2 += 1;
                } else {
                    break;
                }
            }
            let count = count1.max(count2);
            for occ in 0..count {
                let c1 = sample_fallback(pos1, col1, position, occ);
                let c2 = sample_fallback(pos2, col2, position, occ);
                self.positions.push(position);
                self.colors.push(gamma_evaluate(c1, c2, progress));
            }
        }
    }
}

#[inline]
fn is_sorted_no_nan(pos: &[f32]) -> bool {
    if pos.is_empty() {
        return true;
    }
    let mut prev = pos[0];
    if prev.is_nan() {
        return false;
    }
    for &p in &pos[1..] {
        if !(p >= prev) {
            return false;
        }
        prev = p;
    }
    true
}

#[inline]
fn sample_between_sorted(pos: &[f32], col: &[i32], upper: usize, position: f32) -> i32 {
    if upper == 0 {
        return col[0];
    }
    if upper == pos.len() {
        return col[pos.len() - 1];
    }
    let lower = upper - 1;
    let start_pos = pos[lower];
    let end_pos = pos[upper];
    if end_pos <= start_pos {
        return col[lower];
    }
    let segment_progress = (position - start_pos) / (end_pos - start_pos);
    gamma_evaluate(col[lower], col[upper], segment_progress)
}

fn sample_fallback(pos: &[f32], col: &[i32], position: f32, occurrence: usize) -> i32 {
    let size = pos.len();
    if size == 0 {
        return 0;
    }
    let mut exact_index = 0;
    while exact_index < size && pos[exact_index] < position {
        exact_index += 1;
    }
    if exact_index < size && pos[exact_index] == position {
        let mut exact_count = 0;
        while exact_index + exact_count < size && pos[exact_index + exact_count] == position {
            exact_count += 1;
        }
        let sampled_occurrence = occurrence.min(exact_count - 1);
        return col[exact_index + sampled_occurrence];
    }
    let mut upper_index = 0;
    while upper_index < size && pos[upper_index] < position {
        upper_index += 1;
    }
    sample_between_sorted(pos, col, upper_index, position)
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DocumentJustification {
    #[default]
    LeftAlign = 0,
    RightAlign = 1,
    Center = 2,
    JustifyWithLastLineLeft = 3,
    JustifyWithLastLineRight = 4,
    JustifyWithLastLineCenter = 5,
    JustifyWithLastLineFull = 6,
}

impl DocumentJustification {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::LeftAlign,
            1 => Self::RightAlign,
            2 => Self::Center,
            3 => Self::JustifyWithLastLineLeft,
            4 => Self::JustifyWithLastLineRight,
            5 => Self::JustifyWithLastLineCenter,
            6 => Self::JustifyWithLastLineFull,
            _ => Self::LeftAlign,
        }
    }
}

/// Text document data matching `lynx::animax::DocumentData`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DocumentData {
    pub text: String,
    pub font_name: String,
    pub size: f32,
    pub justification: DocumentJustification,
    pub tracking: i32,
    pub line_height: f32,
    pub baseline_shift: f32,
    pub color: i32,
    pub stroke_color: i32,
    pub stroke_width: f32,
    pub stroke_overfill: bool,
    pub box_position: Option<PointF>,
    pub box_size: Option<PointF>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use animax_base::Color;

    #[test]
    fn gradient_lerp_mismatched_and_hard_stops() {
        let g1 = GradientColor::new(
            vec![0.0, 0.5, 1.0],
            vec![
                Color::pack_argb(255, 255, 255, 255),
                Color::pack_argb(128, 0, 0, 255),
                Color::pack_argb(64, 0, 0, 0),
            ],
        );
        let g2 = GradientColor::new(
            vec![0.0, 0.25, 0.5, 0.75, 1.0],
            vec![
                Color::pack_argb(128, 0, 0, 0),
                Color::pack_argb(255, 0, 255, 0),
                Color::pack_argb(255, 255, 0, 0),
                Color::pack_argb(128, 0, 0, 255),
                Color::pack_argb(255, 255, 255, 255),
            ],
        );
        let mut out = GradientColor::default();
        out.lerp_color(&g1, &g2, 0.5);
        assert_eq!(out.positions, vec![0.0, 0.25, 0.5, 0.75, 1.0]);
        let sampled_025 = gamma_evaluate(g1.colors[0], g1.colors[1], 0.5);
        assert_eq!(out.colors[1], gamma_evaluate(sampled_025, g2.colors[1], 0.5));

        // Duplicate positions preserve hard stops
        let h1 = GradientColor::new(
            vec![0.0, 0.5, 0.5, 1.0],
            vec![
                Color::pack_argb(255, 0, 0, 0),
                Color::pack_argb(255, 255, 0, 0),
                Color::pack_argb(255, 0, 0, 255),
                Color::pack_argb(255, 0, 0, 255),
            ],
        );
        let h2 = GradientColor::new(
            vec![0.0, 0.5, 1.0],
            vec![
                Color::pack_argb(255, 0, 0, 0),
                Color::pack_argb(255, 255, 255, 255),
                Color::pack_argb(255, 255, 255, 255),
            ],
        );
        out.lerp_color(&h1, &h2, 0.5);
        assert_eq!(out.positions, vec![0.0, 0.5, 0.5, 1.0]);
        assert_ne!(out.colors[1], out.colors[2]);
    }

    #[test]
    fn shape_data_interpolate() {
        let s1 = ShapeData {
            initial_point: PointF::new(0.0, 0.0),
            curves: vec![CubicCurve {
                cp1: PointF::new(0.0, 10.0),
                cp2: PointF::new(10.0, 10.0),
                vertex: PointF::new(10.0, 0.0),
            }],
            closed: false,
        };
        let s2 = ShapeData {
            initial_point: PointF::new(10.0, 20.0),
            curves: vec![CubicCurve {
                cp1: PointF::new(20.0, 30.0),
                cp2: PointF::new(30.0, 30.0),
                vertex: PointF::new(30.0, 20.0),
            }],
            closed: true,
        };
        let mut out = ShapeData::default();
        out.interpolate_between(&s1, &s2, 0.5);
        assert!(out.closed);
        assert_eq!(out.initial_point, PointF::new(5.0, 10.0));
        assert_eq!(out.curves[0].vertex, PointF::new(20.0, 10.0));
    }
}
