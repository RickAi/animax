// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

use crate::keyframe::Keyframe;
use crate::value::{DocumentData, GradientColor, PointF, ScaleF, ShapeData};
use animax_base::Color;
use serde::{Deserialize, Serialize};

/// Generic keyframed animatable property.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimatableValue<T> {
    pub keyframes: Vec<Keyframe<T>>,
}

impl<T> Default for AnimatableValue<T> {
    fn default() -> Self {
        Self { keyframes: Vec::new() }
    }
}

impl<T> AnimatableValue<T> {
    pub fn from_static(value: T) -> Self {
        Self {
            keyframes: vec![Keyframe::static_value(value)],
        }
    }

    pub fn from_keyframes(keyframes: Vec<Keyframe<T>>) -> Self {
        Self { keyframes }
    }

    pub fn is_empty(&self) -> bool {
        self.keyframes.is_empty()
    }

    pub fn is_static(&self) -> bool {
        self.keyframes.is_empty()
            || (self.keyframes.len() == 1 && self.keyframes[0].is_static())
    }
}

pub type AnimatableFloat = AnimatableValue<f32>;
pub type AnimatableInteger = AnimatableValue<i32>;
pub type AnimatableColor = AnimatableValue<Color>;
pub type AnimatablePoint = AnimatableValue<PointF>;
pub type AnimatableScale = AnimatableValue<ScaleF>;
pub type AnimatableShape = AnimatableValue<ShapeData>;
pub type AnimatableGradient = AnimatableValue<GradientColor>;
pub type AnimatableDocument = AnimatableValue<DocumentData>;

/// Position property: either a spatial motion path (`PointF` keyframes) or split X/Y/Z channels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AnimatablePosition {
    Path(AnimatablePoint),
    Split {
        x: AnimatableFloat,
        y: AnimatableFloat,
        z: Option<AnimatableFloat>,
    },
}

impl Default for AnimatablePosition {
    fn default() -> Self {
        Self::Path(AnimatableValue::from_static(PointF::default()))
    }
}

/// Complete 2D/3D layer & group transform model matching `AnimatableTransformModel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AnimatableTransform {
    pub anchor_point: Option<AnimatablePosition>,
    pub position: Option<AnimatablePosition>,
    pub scale: Option<AnimatableScale>,
    pub rotation: Option<AnimatableFloat>,
    pub rotation_x: Option<AnimatableFloat>,
    pub rotation_y: Option<AnimatableFloat>,
    pub rotation_z: Option<AnimatableFloat>,
    pub opacity: Option<AnimatableInteger>,
    pub start_opacity: Option<AnimatableFloat>,
    pub end_opacity: Option<AnimatableFloat>,
    pub skew: Option<AnimatableFloat>,
    pub skew_angle: Option<AnimatableFloat>,
    pub auto_orient: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TextRangeSelector {
    pub start: Option<AnimatableFloat>,
    pub end: Option<AnimatableFloat>,
    pub offset: Option<AnimatableFloat>,
    pub units: i32,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AnimatableTextProperties {
    pub color: Option<AnimatableColor>,
    pub stroke_color: Option<AnimatableColor>,
    pub stroke_width: Option<AnimatableFloat>,
    pub tracking: Option<AnimatableFloat>,
    pub opacity: Option<AnimatableInteger>,
    pub position: Option<AnimatablePosition>,
    pub scale: Option<AnimatableScale>,
    pub rotation: Option<AnimatableFloat>,
    pub range_selector: Option<TextRangeSelector>,
}
