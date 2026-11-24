// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

use crate::canvas::Canvas;
use animax_base::Color;

/// Target pixel layout for cross-platform surface presentation.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PixelFormat {
    /// Premultiplied RGBA8888 (native tiny-skia, Android GL/ANativeWindow, HarmonyOS).
    #[default]
    Rgba8888Premul = 0,
    /// Straight RGBA8888 (Web Canvas2D `ImageData`).
    Rgba8888Straight = 1,
    /// Premultiplied BGRA8888 (iOS/macOS CoreGraphics & Metal `MTLPixelFormatBGRA8Unorm`).
    Bgra8888Premul = 2,
}

/// Pure Rust rendering surface replacing `skity::GPUSurface` / `SurfaceGL` / `SurfaceMTL` / `SurfaceSW`.
pub struct Surface {
    canvas: Canvas,
    conversion_buf: Vec<u8>,
}

impl Surface {
    pub fn new(width: u32, height: u32) -> Option<Self> {
        let canvas = Canvas::new(width, height)?;
        Some(Self {
            canvas,
            conversion_buf: Vec::new(),
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) -> bool {
        let w = width.max(1);
        let h = height.max(1);
        if self.canvas.width() == w && self.canvas.height() == h {
            return true;
        }
        if let Some(c) = Canvas::new(w, h) {
            self.canvas = c;
            true
        } else {
            false
        }
    }

    pub fn width(&self) -> u32 {
        self.canvas.width()
    }

    pub fn height(&self) -> u32 {
        self.canvas.height()
    }

    pub fn clear(&mut self) {
        self.canvas.clear(Color::TRANSPARENT);
    }

    pub fn canvas_mut(&mut self) -> &mut Canvas {
        &mut self.canvas
    }

    pub fn pixels_rgba_premul(&self) -> &[u8] {
        self.canvas.pixels()
    }

    /// Returns pixels converted to the requested platform pixel format.
    pub fn pixels_as(&mut self, format: PixelFormat) -> &[u8] {
        match format {
            PixelFormat::Rgba8888Premul => self.canvas.pixels(),
            PixelFormat::Rgba8888Straight => {
                let src = self.canvas.pixels();
                self.conversion_buf.resize(src.len(), 0);
                for (dst, s) in self.conversion_buf.chunks_exact_mut(4).zip(src.chunks_exact(4)) {
                    let a = s[3];
                    if a == 0 {
                        dst.copy_from_slice(&[0, 0, 0, 0]);
                    } else if a == 255 {
                        dst.copy_from_slice(s);
                    } else {
                        let a32 = a as u32;
                        dst[0] = ((s[0] as u32 * 255 + a32 / 2) / a32).min(255) as u8;
                        dst[1] = ((s[1] as u32 * 255 + a32 / 2) / a32).min(255) as u8;
                        dst[2] = ((s[2] as u32 * 255 + a32 / 2) / a32).min(255) as u8;
                        dst[3] = a;
                    }
                }
                &self.conversion_buf
            }
            PixelFormat::Bgra8888Premul => {
                let src = self.canvas.pixels();
                self.conversion_buf.resize(src.len(), 0);
                for (dst, s) in self.conversion_buf.chunks_exact_mut(4).zip(src.chunks_exact(4)) {
                    dst[0] = s[2];
                    dst[1] = s[1];
                    dst[2] = s[0];
                    dst[3] = s[3];
                }
                &self.conversion_buf
            }
        }
    }
}
