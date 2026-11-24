// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

use crate::geometry::Matrix;
use animax_model::{PointF, RectF, ShapeData};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FillType {
    #[default]
    Winding,
    EvenOdd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathOp {
    Merge,
    Union,
    Difference,
    Intersect,
    Xor,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathVerb {
    MoveTo(f32, f32),
    LineTo(f32, f32),
    QuadTo(f32, f32, f32, f32),
    CubicTo(f32, f32, f32, f32, f32, f32),
    Close,
}

/// Pure Rust 2D vector path matching `skity::Path`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Path {
    verbs: Vec<PathVerb>,
    pub fill_type: FillType,
}

impl Path {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.verbs.is_empty()
    }

    pub fn clear(&mut self) {
        self.verbs.clear();
    }

    pub fn verbs(&self) -> &[PathVerb] {
        &self.verbs
    }

    pub fn move_to(&mut self, x: f32, y: f32) {
        self.verbs.push(PathVerb::MoveTo(x, y));
    }

    pub fn line_to(&mut self, x: f32, y: f32) {
        self.verbs.push(PathVerb::LineTo(x, y));
    }

    pub fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.verbs.push(PathVerb::QuadTo(cx, cy, x, y));
    }

    pub fn cubic_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.verbs.push(PathVerb::CubicTo(x1, y1, x2, y2, x, y));
    }

    pub fn close(&mut self) {
        self.verbs.push(PathVerb::Close);
    }

    pub fn add_rect(&mut self, rect: RectF) {
        self.move_to(rect.left, rect.top);
        self.line_to(rect.right, rect.top);
        self.line_to(rect.right, rect.bottom);
        self.line_to(rect.left, rect.bottom);
        self.close();
    }

    pub fn add_rounded_rect(&mut self, rect: RectF, rx: f32, ry: f32) {
        let rx = rx.min(rect.width() * 0.5).max(0.0);
        let ry = ry.min(rect.height() * 0.5).max(0.0);
        if rx <= 1e-4 || ry <= 1e-4 {
            self.add_rect(rect);
            return;
        }
        const K: f32 = 0.55228475;
        let ox = rx * K;
        let oy = ry * K;
        self.move_to(rect.left + rx, rect.top);
        self.line_to(rect.right - rx, rect.top);
        self.cubic_to(
            rect.right - rx + ox,
            rect.top,
            rect.right,
            rect.top + ry - oy,
            rect.right,
            rect.top + ry,
        );
        self.line_to(rect.right, rect.bottom - ry);
        self.cubic_to(
            rect.right,
            rect.bottom - ry + oy,
            rect.right - rx + ox,
            rect.bottom,
            rect.right - rx,
            rect.bottom,
        );
        self.line_to(rect.left + rx, rect.bottom);
        self.cubic_to(
            rect.left + rx - ox,
            rect.bottom,
            rect.left,
            rect.bottom - ry + oy,
            rect.left,
            rect.bottom - ry,
        );
        self.line_to(rect.left, rect.top + ry);
        self.cubic_to(
            rect.left,
            rect.top + ry - oy,
            rect.left + rx - ox,
            rect.top,
            rect.left + rx,
            rect.top,
        );
        self.close();
    }

    pub fn add_oval(&mut self, rect: RectF) {
        let cx = (rect.left + rect.right) * 0.5;
        let cy = (rect.top + rect.bottom) * 0.5;
        let rx = rect.width() * 0.5;
        let ry = rect.height() * 0.5;
        const K: f32 = 0.55228475;
        let ox = rx * K;
        let oy = ry * K;
        self.move_to(cx, rect.top);
        self.cubic_to(cx + ox, rect.top, rect.right, cy - oy, rect.right, cy);
        self.cubic_to(rect.right, cy + oy, cx + ox, rect.bottom, cx, rect.bottom);
        self.cubic_to(cx - ox, rect.bottom, rect.left, cy + oy, rect.left, cy);
        self.cubic_to(rect.left, cy - oy, cx - ox, rect.top, cx, rect.top);
        self.close();
    }

    pub fn from_shape_data(shape: &ShapeData) -> Self {
        let mut path = Path::new();
        if shape.curves.is_empty() {
            return path;
        }
        let mut cur = shape.initial_point;
        path.move_to(cur.x, cur.y);
        for curve in &shape.curves {
            let cp1 = curve.cp1;
            let cp2 = curve.cp2;
            let v = curve.vertex;
            if (cp1.x - cur.x).abs() < 1e-5
                && (cp1.y - cur.y).abs() < 1e-5
                && (cp2.x - v.x).abs() < 1e-5
                && (cp2.y - v.y).abs() < 1e-5
            {
                path.line_to(v.x, v.y);
            } else {
                path.cubic_to(cp1.x, cp1.y, cp2.x, cp2.y, v.x, v.y);
            }
            cur = v;
        }
        if shape.closed {
            path.close();
        }
        path
    }

    pub fn add_path(&mut self, other: &Path) {
        self.verbs.extend_from_slice(&other.verbs);
    }

    pub fn add_path_with_matrix(&mut self, other: &Path, matrix: &Matrix) {
        if matrix.is_identity() {
            self.add_path(other);
            return;
        }
        for &verb in &other.verbs {
            match verb {
                PathVerb::MoveTo(x, y) => {
                    let (nx, ny) = matrix.map_xy(x, y);
                    self.move_to(nx, ny);
                }
                PathVerb::LineTo(x, y) => {
                    let (nx, ny) = matrix.map_xy(x, y);
                    self.line_to(nx, ny);
                }
                PathVerb::QuadTo(cx, cy, x, y) => {
                    let (ncx, ncy) = matrix.map_xy(cx, cy);
                    let (nx, ny) = matrix.map_xy(x, y);
                    self.quad_to(ncx, ncy, nx, ny);
                }
                PathVerb::CubicTo(x1, y1, x2, y2, x, y) => {
                    let (nx1, ny1) = matrix.map_xy(x1, y1);
                    let (nx2, ny2) = matrix.map_xy(x2, y2);
                    let (nx, ny) = matrix.map_xy(x, y);
                    self.cubic_to(nx1, ny1, nx2, ny2, nx, ny);
                }
                PathVerb::Close => self.close(),
            }
        }
    }

    pub fn transform(&self, matrix: &Matrix) -> Path {
        let mut out = Path {
            verbs: Vec::with_capacity(self.verbs.len()),
            fill_type: self.fill_type,
        };
        out.add_path_with_matrix(self, matrix);
        out
    }

    pub fn bounds(&self) -> RectF {
        let mut left = f32::INFINITY;
        let mut top = f32::INFINITY;
        let mut right = f32::NEG_INFINITY;
        let mut bottom = f32::NEG_INFINITY;
        let mut include = |x: f32, y: f32| {
            if x.is_finite() && y.is_finite() {
                left = left.min(x);
                top = top.min(y);
                right = right.max(x);
                bottom = bottom.max(y);
            }
        };
        for &verb in &self.verbs {
            match verb {
                PathVerb::MoveTo(x, y) | PathVerb::LineTo(x, y) => include(x, y),
                PathVerb::QuadTo(cx, cy, x, y) => {
                    include(cx, cy);
                    include(x, y);
                }
                PathVerb::CubicTo(x1, y1, x2, y2, x, y) => {
                    include(x1, y1);
                    include(x2, y2);
                    include(x, y);
                }
                PathVerb::Close => {}
            }
        }
        if left > right || top > bottom {
            RectF::default()
        } else {
            RectF::new(left, top, right, bottom)
        }
    }

    pub fn to_tiny_skia_path(&self) -> Option<tiny_skia::Path> {
        let mut pb = tiny_skia::PathBuilder::new();
        for &verb in &self.verbs {
            match verb {
                PathVerb::MoveTo(x, y) => pb.move_to(x, y),
                PathVerb::LineTo(x, y) => pb.line_to(x, y),
                PathVerb::QuadTo(cx, cy, x, y) => pb.quad_to(cx, cy, x, y),
                PathVerb::CubicTo(x1, y1, x2, y2, x, y) => pb.cubic_to(x1, y1, x2, y2, x, y),
                PathVerb::Close => pb.close(),
            }
        }
        pb.finish()
    }

    pub fn contains_point(&self, pt: PointF) -> bool {
        let b = self.bounds();
        b.contains(pt.x, pt.y)
    }
}

/// Builder wrapper matching `skity::PathBuilder`.
#[derive(Debug, Default)]
pub struct PathBuilder {
    path: Path,
}

impl PathBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn move_to(&mut self, x: f32, y: f32) -> &mut Self {
        self.path.move_to(x, y);
        self
    }

    pub fn line_to(&mut self, x: f32, y: f32) -> &mut Self {
        self.path.line_to(x, y);
        self
    }

    pub fn cubic_to(
        &mut self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        x: f32,
        y: f32,
    ) -> &mut Self {
        self.path.cubic_to(x1, y1, x2, y2, x, y);
        self
    }

    pub fn close(&mut self) -> &mut Self {
        self.path.close();
        self
    }

    pub fn finish(self) -> Path {
        self.path
    }
}
