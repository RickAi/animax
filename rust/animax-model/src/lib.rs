// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

//! Pure Rust data models for Lottie compositions, layers, shapes, keyframes,
//! values, gradients, text documents, and Alpha Video assets.

pub mod animatable;
pub mod asset;
pub mod composition;
pub mod content;
pub mod effect;
pub mod keyframe;
pub mod layer;
pub mod value;

pub use animatable::*;
pub use asset::*;
pub use composition::CompositionModel;
pub use content::*;
pub use effect::*;
pub use keyframe::Keyframe;
pub use layer::{LayerModel, LayerType, MatteType};
pub use value::*;
