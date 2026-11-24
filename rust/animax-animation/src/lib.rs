// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

//! Pure Rust animation interpolation, keyframe evaluation, transform computation,
//! and VSync playback clock replacing `src/animation/` and `src/animator/`.

pub mod evaluator;
pub mod interpolator;
pub mod transform;
pub mod value_animator;

pub use evaluator::{
    eval_color, eval_document, eval_float, eval_gradient, eval_i32, eval_point, eval_position,
    eval_position_with_tangent, eval_scale, eval_shape,
};
pub use interpolator::cubic_bezier_ease;
pub use transform::{eval_repeater_transform, eval_transform, EvaluatedTransform};
pub use value_animator::{AnimatorTickResult, ValueAnimator};
