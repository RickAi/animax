// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

use crate::color::Color;

/// Linear interpolation between two `f32` values: `a + percentage * (b - a)`.
#[inline]
pub fn lerp_f32(a: f32, b: f32, percentage: f32) -> f32 {
    a + percentage * (b - a)
}

/// Linear interpolation between two `i32` values, matching C++ truncation towards zero.
#[inline]
pub fn lerp_i32(a: i32, b: i32, percentage: f32) -> i32 {
    let prod = percentage * ((b - a) as f32);
    let delta = if prod.is_finite() && prod >= (i32::MIN as f32) && prod < 2147483648.0_f32 {
        prod as i32
    } else {
        i32::MIN
    };
    a.wrapping_add(delta)
}

/// Floor modulus matching `lynx::animax::FloorMod`.
#[inline]
pub fn floor_mod(x: i32, y: i32) -> i32 {
    if y == 0 {
        return 0;
    }
    let mut r = x / y;
    if (x ^ y) < 0 && (r * y != x) {
        r -= 1;
    }
    x - r * y
}

/// Channel-wise color interpolation matching `lynx::animax::GammaEvaluate(int32_t, int32_t, float)`.
#[inline]
pub fn gamma_evaluate(start_color: i32, end_color: i32, progress: f32) -> i32 {
    let a = lerp_i32((start_color >> 24) & 0xff, (end_color >> 24) & 0xff, progress) as u8;
    let r = lerp_i32((start_color >> 16) & 0xff, (end_color >> 16) & 0xff, progress) as u8;
    let g = lerp_i32((start_color >> 8) & 0xff, (end_color >> 8) & 0xff, progress) as u8;
    let b = lerp_i32(start_color & 0xff, end_color & 0xff, progress) as u8;
    Color::pack_argb(a, r, g, b)
}

/// Channel-wise `Color` interpolation matching `lynx::animax::GammaEvaluate(Color, Color, float)`.
#[inline]
pub fn gamma_evaluate_color(start_color: Color, end_color: Color, progress: f32) -> Color {
    Color {
        a: lerp_f32(start_color.a as f32, end_color.a as f32, progress) as u8,
        r: lerp_f32(start_color.r as f32, end_color.r as f32, progress) as u8,
        g: lerp_f32(start_color.g as f32, end_color.g as f32, progress) as u8,
        b: lerp_f32(start_color.b as f32, end_color.b as f32, progress) as u8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lerp() {
        assert!((lerp_f32(0.0, 10.0, 0.5) - 5.0).abs() < 1e-6);
        assert_eq!(lerp_i32(0, 100, 0.25), 25);
        assert_eq!(lerp_i32(100, 0, 0.5), 50);
    }

    #[test]
    fn test_floor_mod() {
        assert_eq!(floor_mod(5, 3), 2);
        assert_eq!(floor_mod(-1, 3), 2);
        assert_eq!(floor_mod(-3, 3), 0);
        assert_eq!(floor_mod(1, -3), -2);
        assert_eq!(floor_mod(-5, -3), -2);
    }

    #[test]
    fn test_gamma_evaluate() {
        let c1 = Color::pack_argb(255, 0, 0, 0);
        let c2 = Color::pack_argb(255, 254, 100, 200);
        let mid = gamma_evaluate(c1, c2, 0.5);
        assert_eq!(Color::from_i32(mid), Color::from_argb(255, 127, 50, 100));
    }
}
