// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

use animax_model::RectF;

/// Premultiplied RGBA8888 pixel buffer matching `skity::Pixmap` / `skity::Image`.
#[derive(Debug, Clone)]
pub struct Pixmap {
    pub(crate) inner: tiny_skia::Pixmap,
}

impl Pixmap {
    pub fn new(width: u32, height: u32) -> Option<Self> {
        Some(Self {
            inner: tiny_skia::Pixmap::new(width.max(1), height.max(1))?,
        })
    }

    pub fn from_premultiplied_rgba(width: u32, height: u32, data: Vec<u8>) -> Option<Self> {
        let size = tiny_skia::IntSize::from_wh(width, height)?;
        Some(Self {
            inner: tiny_skia::Pixmap::from_vec(data, size)?,
        })
    }

    /// Creates a premultiplied `Pixmap` from straight (unpremultiplied) RGBA8888 bytes.
    pub fn from_straight_rgba(width: u32, height: u32, rgba: &[u8]) -> Option<Self> {
        let expected = (width as usize).checked_mul(height as usize)?.checked_mul(4)?;
        if rgba.len() < expected {
            return None;
        }
        let mut premul = vec![0u8; expected];
        for (dst, src) in premul.chunks_exact_mut(4).zip(rgba[..expected].chunks_exact(4)) {
            let a = src[3] as u16;
            dst[0] = ((src[0] as u16 * a + 127) / 255) as u8;
            dst[1] = ((src[1] as u16 * a + 127) / 255) as u8;
            dst[2] = ((src[2] as u16 * a + 127) / 255) as u8;
            dst[3] = src[3];
        }
        Self::from_premultiplied_rgba(width, height, premul)
    }

    /// Decodes PNG / JPEG / WebP image bytes directly in pure Rust.
    pub fn decode_encoded_bytes(bytes: &[u8]) -> Option<Self> {
        let img = image::load_from_memory(bytes).ok()?.to_rgba8();
        let (w, h) = img.dimensions();
        Self::from_straight_rgba(w, h, img.as_raw())
    }

    pub fn width(&self) -> u32 {
        self.inner.width()
    }

    pub fn height(&self) -> u32 {
        self.inner.height()
    }

    pub fn data(&self) -> &[u8] {
        self.inner.data()
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        self.inner.data_mut()
    }

    pub fn clear(&mut self) {
        self.inner.fill(tiny_skia::Color::TRANSPARENT);
    }

    /// Copies premultiplied RGBA8888 into straight RGBA8888 (for Web `ImageData`, PNG export, etc.).
    pub fn copy_to_straight_rgba(&self, out: &mut [u8]) {
        for (dst, src) in out.chunks_exact_mut(4).zip(self.data().chunks_exact(4)) {
            let a = src[3];
            if a == 0 {
                dst.copy_from_slice(&[0, 0, 0, 0]);
            } else if a == 255 {
                dst.copy_from_slice(src);
            } else {
                let a32 = a as u32;
                dst[0] = ((src[0] as u32 * 255 + a32 / 2) / a32).min(255) as u8;
                dst[1] = ((src[1] as u32 * 255 + a32 / 2) / a32).min(255) as u8;
                dst[2] = ((src[2] as u32 * 255 + a32 / 2) / a32).min(255) as u8;
                dst[3] = a;
            }
        }
    }

    /// Copies premultiplied RGBA8888 into premultiplied BGRA8888 (for iOS CoreGraphics/Metal `bgra8Unorm`).
    pub fn copy_to_premultiplied_bgra(&self, out: &mut [u8]) {
        for (dst, src) in out.chunks_exact_mut(4).zip(self.data().chunks_exact(4)) {
            dst[0] = src[2];
            dst[1] = src[1];
            dst[2] = src[0];
            dst[3] = src[3];
        }
    }

    /// Separable 3-pass box blur approximating Gaussian blur (pure Rust, in-place).
    pub fn apply_blur(&mut self, radius: f32) {
        let r = radius.round() as i32;
        if r <= 0 {
            return;
        }
        let w = self.width() as usize;
        let h = self.height() as usize;
        if w == 0 || h == 0 {
            return;
        }
        let r = (r as usize).min(w.max(h));
        let mut tmp = vec![0u8; w * h * 4];
        let src = self.data_mut();
        for _ in 0..2 {
            // Horizontal pass
            for y in 0..h {
                let row = y * w * 4;
                for x in 0..w {
                    let x0 = x.saturating_sub(r);
                    let x1 = (x + r).min(w - 1);
                    let count = (x1 - x0 + 1) as u32;
                    let mut acc = [0u32; 4];
                    for kx in x0..=x1 {
                        let off = row + kx * 4;
                        for c in 0..4 {
                            acc[c] += src[off + c] as u32;
                        }
                    }
                    let out_off = row + x * 4;
                    for c in 0..4 {
                        tmp[out_off + c] = (acc[c] / count) as u8;
                    }
                }
            }
            // Vertical pass
            for y in 0..h {
                let y0 = y.saturating_sub(r);
                let y1 = (y + r).min(h - 1);
                let count = (y1 - y0 + 1) as u32;
                for x in 0..w {
                    let mut acc = [0u32; 4];
                    for ky in y0..=y1 {
                        let off = (ky * w + x) * 4;
                        for c in 0..4 {
                            acc[c] += tmp[off + c] as u32;
                        }
                    }
                    let out_off = (y * w + x) * 4;
                    for c in 0..4 {
                        src[out_off + c] = (acc[c] / count) as u8;
                    }
                }
            }
        }
    }
}

/// Pure Rust Alpha Video compositor replacing GPU GLSL/MSL/WGSL YUV+Alpha shaders.
/// Combines split RGB + Alpha video frames (RGBA or YUV420p) into a premultiplied RGBA8888 `Pixmap`.
pub struct AlphaVideoCompositor;

impl AlphaVideoCompositor {
    /// Composites an RGBA8888 video frame containing separate `rgb_rect` and `alpha_rect` sub-regions
    /// into a premultiplied RGBA8888 `Pixmap`.
    pub fn compose_rgba_split(
        frame_rgba: &[u8],
        frame_w: u32,
        frame_h: u32,
        rgb_rect: RectF,
        alpha_rect: RectF,
    ) -> Option<Pixmap> {
        let out_w = rgb_rect.width().round().max(1.0) as u32;
        let out_h = rgb_rect.height().round().max(1.0) as u32;
        let fw = frame_w as usize;
        let fh = frame_h as usize;
        if frame_rgba.len() < fw * fh * 4 {
            return None;
        }
        let mut out = vec![0u8; (out_w as usize) * (out_h as usize) * 4];
        let rx0 = rgb_rect.left.max(0.0) as usize;
        let ry0 = rgb_rect.top.max(0.0) as usize;
        let ax0 = alpha_rect.left.max(0.0) as usize;
        let ay0 = alpha_rect.top.max(0.0) as usize;

        for y in 0..(out_h as usize) {
            let sy_rgb = (ry0 + y).min(fh.saturating_sub(1));
            let sy_alp = (ay0 + y).min(fh.saturating_sub(1));
            for x in 0..(out_w as usize) {
                let sx_rgb = (rx0 + x).min(fw.saturating_sub(1));
                let sx_alp = (ax0 + x).min(fw.saturating_sub(1));
                let rgb_off = (sy_rgb * fw + sx_rgb) * 4;
                let alp_off = (sy_alp * fw + sx_alp) * 4;
                let a = frame_rgba[alp_off] as u16; // R channel carries luma/alpha in split alpha video
                let dst_off = (y * (out_w as usize) + x) * 4;
                out[dst_off] = ((frame_rgba[rgb_off] as u16 * a + 127) / 255) as u8;
                out[dst_off + 1] = ((frame_rgba[rgb_off + 1] as u16 * a + 127) / 255) as u8;
                out[dst_off + 2] = ((frame_rgba[rgb_off + 2] as u16 * a + 127) / 255) as u8;
                out[dst_off + 3] = a as u8;
            }
        }
        Pixmap::from_premultiplied_rgba(out_w, out_h, out)
    }

    /// Converts YUV420p (BT.601/BT.709) split alpha video planes into premultiplied RGBA8888 `Pixmap`.
    pub fn compose_yuv420p_split(
        y_plane: &[u8],
        u_plane: &[u8],
        v_plane: &[u8],
        frame_w: u32,
        frame_h: u32,
        rgb_rect: RectF,
        alpha_rect: RectF,
    ) -> Option<Pixmap> {
        let fw = frame_w as usize;
        let fh = frame_h as usize;
        let cw = (fw + 1) / 2;
        if y_plane.len() < fw * fh || u_plane.len() < cw * ((fh + 1) / 2) || v_plane.len() < cw * ((fh + 1) / 2) {
            return None;
        }
        let out_w = rgb_rect.width().round().max(1.0) as u32;
        let out_h = rgb_rect.height().round().max(1.0) as u32;
        let mut out = vec![0u8; (out_w as usize) * (out_h as usize) * 4];

        let rx0 = rgb_rect.left.max(0.0) as usize;
        let ry0 = rgb_rect.top.max(0.0) as usize;
        let ax0 = alpha_rect.left.max(0.0) as usize;
        let ay0 = alpha_rect.top.max(0.0) as usize;

        for y in 0..(out_h as usize) {
            let sy_rgb = (ry0 + y).min(fh - 1);
            let sy_alp = (ay0 + y).min(fh - 1);
            for x in 0..(out_w as usize) {
                let sx_rgb = (rx0 + x).min(fw - 1);
                let sx_alp = (ax0 + x).min(fw - 1);

                let y_val = y_plane[sy_rgb * fw + sx_rgb] as f32;
                let u_val = u_plane[(sy_rgb / 2) * cw + (sx_rgb / 2)] as f32 - 128.0;
                let v_val = v_plane[(sy_rgb / 2) * cw + (sx_rgb / 2)] as f32 - 128.0;

                let r = (y_val + 1.402 * v_val).clamp(0.0, 255.0) as u16;
                let g = (y_val - 0.344136 * u_val - 0.714136 * v_val).clamp(0.0, 255.0) as u16;
                let b = (y_val + 1.772 * u_val).clamp(0.0, 255.0) as u16;
                let a = y_plane[sy_alp * fw + sx_alp] as u16;

                let dst_off = (y * (out_w as usize) + x) * 4;
                out[dst_off] = ((r * a + 127) / 255) as u8;
                out[dst_off + 1] = ((g * a + 127) / 255) as u8;
                out[dst_off + 2] = ((b * a + 127) / 255) as u8;
                out[dst_off + 3] = a as u8;
            }
        }
        Pixmap::from_premultiplied_rgba(out_w, out_h, out)
    }
}
