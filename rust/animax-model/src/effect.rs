// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

use crate::animatable::{AnimatableColor, AnimatableFloat, AnimatableInteger, AnimatableShape};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MaskMode {
    #[default]
    Add,
    Subtract,
    Intersect,
    None,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MaskModel {
    pub mode: MaskMode,
    pub mask_path: AnimatableShape,
    pub opacity: AnimatableInteger,
    pub inverted: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct BlurEffectModel {
    pub blurriness: AnimatableFloat,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DropShadowEffectModel {
    pub color: AnimatableColor,
    pub opacity: AnimatableFloat,
    pub direction: AnimatableFloat,
    pub distance: AnimatableFloat,
    pub radius: AnimatableFloat,
}
