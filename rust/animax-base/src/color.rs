// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

use serde::{Deserialize, Serialize};

/// ARGB 8-bit-per-channel color matching `lynx::animax::Color`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Color {
    pub a: u8,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const TRANSPARENT: Self = Self { a: 0, r: 0, g: 0, b: 0 };
    pub const BLACK: Self = Self { a: 255, r: 0, g: 0, b: 0 };
    pub const WHITE: Self = Self { a: 255, r: 255, g: 255, b: 255 };

    #[inline]
    pub const fn from_argb(a: u8, r: u8, g: u8, b: u8) -> Self {
        Self { a, r, g, b }
    }

    #[inline]
    pub const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self { a: 255, r, g, b }
    }

    #[inline]
    pub const fn from_i32(value: i32) -> Self {
        let u = value as u32;
        Self {
            a: ((u >> 24) & 0xff) as u8,
            r: ((u >> 16) & 0xff) as u8,
            g: ((u >> 8) & 0xff) as u8,
            b: (u & 0xff) as u8,
        }
    }

    #[inline]
    pub const fn to_i32(self) -> i32 {
        Self::pack_argb(self.a, self.r, self.g, self.b)
    }

    #[inline]
    pub const fn pack_argb(a: u8, r: u8, g: u8, b: u8) -> i32 {
        (((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)) as i32
    }

    /// Construct from normalized `[0.0, 1.0]` RGBA floats (Lottie JSON format).
    #[inline]
    pub fn from_normalized_rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        let to_u8 = |v: f32| -> u8 {
            if v <= 1.0 {
                (v.clamp(0.0, 1.0) * 255.0).round() as u8
            } else {
                v.clamp(0.0, 255.0).round() as u8
            }
        };
        Self {
            a: to_u8(a),
            r: to_u8(r),
            g: to_u8(g),
            b: to_u8(b),
        }
    }

    pub fn from_hex(hex: &str) -> Self {
        Self::from_i32(ColorUtil::parse_hex_color(hex))
    }
}

/// Hex color parser matching `lynx::animax::ColorUtil::ParseHexColor`.
pub struct ColorUtil;

impl ColorUtil {
    /// Parses `#RRGGBB`, `RRGGBB`, `#AARRGGBB`, or `AARRGGBB` into packed ARGB `i32`.
    /// Returns `0` on invalid input.
    pub fn parse_hex_color(color_string: &str) -> i32 {
        if color_string.is_empty() {
            return 0;
        }
        let s = color_string.strip_prefix('#').unwrap_or(color_string);
        let bytes = s.as_bytes();
        let (a, offset) = match bytes.len() {
            6 => (255u8, 0usize),
            8 => {
                let Some(alpha) = parse_hex_byte(bytes[0], bytes[1]) else {
                    return 0;
                };
                (alpha, 2usize)
            }
            _ => return 0,
        };
        let (Some(r), Some(g), Some(b)) = (
            parse_hex_byte(bytes[offset], bytes[offset + 1]),
            parse_hex_byte(bytes[offset + 2], bytes[offset + 3]),
            parse_hex_byte(bytes[offset + 4], bytes[offset + 5]),
        ) else {
            return 0;
        };
        Color::pack_argb(a, r, g, b)
    }
}

#[inline]
fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[inline]
fn parse_hex_byte(hi: u8, lo: u8) -> Option<u8> {
    Some((hex_nibble(hi)? << 4) | hex_nibble(lo)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_6_and_8_digits() {
        assert_eq!(ColorUtil::parse_hex_color("#FF0000"), Color::pack_argb(255, 255, 0, 0));
        assert_eq!(ColorUtil::parse_hex_color("00ff00"), Color::pack_argb(255, 0, 255, 0));
        assert_eq!(ColorUtil::parse_hex_color("#80112233"), Color::pack_argb(0x80, 0x11, 0x22, 0x33));
        assert_eq!(ColorUtil::parse_hex_color("00000000"), 0);
    }

    #[test]
    fn parse_hex_invalid() {
        assert_eq!(ColorUtil::parse_hex_color(""), 0);
        assert_eq!(ColorUtil::parse_hex_color("#123"), 0);
        assert_eq!(ColorUtil::parse_hex_color("#GG0000"), 0);
        assert_eq!(ColorUtil::parse_hex_color("#123456789"), 0);
    }
}
