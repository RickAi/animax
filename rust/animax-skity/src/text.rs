// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, Default)]
pub struct FontMetrics {
    pub ascent: f32,
    pub descent: f32,
    pub line_gap: f32,
}

/// Pure Rust font wrapper backed by `fontdue`, replacing `skity::Font` + Textra shaping.
#[derive(Clone)]
pub struct Font {
    inner: Arc<fontdue::Font>,
}

impl Font {
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let font = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()).ok()?;
        Some(Self {
            inner: Arc::new(font),
        })
    }

    pub fn metrics(&self, px_size: f32) -> FontMetrics {
        if let Some(lm) = self.inner.horizontal_line_metrics(px_size) {
            FontMetrics {
                ascent: lm.ascent,
                descent: lm.descent,
                line_gap: lm.line_gap,
            }
        } else {
            FontMetrics {
                ascent: px_size * 0.8,
                descent: -px_size * 0.2,
                line_gap: 0.0,
            }
        }
    }

    pub fn rasterize(&self, ch: char, px_size: f32) -> (fontdue::Metrics, Vec<u8>) {
        self.inner.rasterize(ch, px_size.max(1.0))
    }

    pub fn measure_text(&self, text: &str, px_size: f32, tracking: f32) -> f32 {
        let mut width = 0.0f32;
        for ch in text.chars() {
            let m = self.inner.metrics(ch, px_size.max(1.0));
            width += m.advance_width + tracking;
        }
        width
    }
}

/// Registry of dynamically registered fonts across Android/iOS/Harmony/Web.
#[derive(Default, Clone)]
pub struct FontManager {
    fonts: HashMap<String, Font>,
    default_font: Option<Font>,
}

impl FontManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_font(&mut self, name: impl Into<String>, bytes: &[u8]) -> bool {
        if let Some(font) = Font::from_bytes(bytes) {
            let key = name.into();
            if self.default_font.is_none() {
                self.default_font = Some(font.clone());
            }
            self.fonts.insert(key, font);
            true
        } else {
            false
        }
    }

    pub fn get(&self, name: &str) -> Option<&Font> {
        self.fonts.get(name).or(self.default_font.as_ref())
    }
}
