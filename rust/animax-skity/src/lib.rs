// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

//! Pure Rust Skity-compatible 2D graphics engine embedded directly in AnimaX.
//!
//! Replaces the C++ Skity library (`third_party/skity`) and `src/render/` with a
//! self-contained Rust implementation backed by `tiny-skia`, `fontdue`, and `image`,
//! targeting Android, iOS, HarmonyOS, WebAssembly, and Desktop uniformly.

pub mod canvas;
pub mod geometry;
pub mod paint;
pub mod path;
pub mod path_measure;
pub mod pixmap;
pub mod surface;
pub mod text;

pub use canvas::Canvas;
pub use geometry::{Camera, Matrix, Quaternion};
pub use paint::{
    ColorFilter, MaskFilter, Paint, PaintStyle, PathEffect, Shader, StrokeCap, StrokeJoin, TileMode,
};
pub use path::{FillType, Path, PathBuilder, PathOp, PathVerb};
pub use path_measure::PathMeasure;
pub use pixmap::{AlphaVideoCompositor, Pixmap};
pub use surface::{PixelFormat, Surface};
pub use text::{Font, FontManager, FontMetrics};
