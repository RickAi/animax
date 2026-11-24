// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

use crate::animatable::{
    AnimatableDocument, AnimatableFloat, AnimatableTextProperties, AnimatableTransform,
};
use crate::content::ContentModel;
use crate::effect::{BlurEffectModel, DropShadowEffectModel, MaskModel};
use crate::value::BlendMode;
use animax_base::Color;
use serde::{Deserialize, Serialize};

/// Layer types matching `lynx::animax::LayerType`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LayerType {
    PreComp = 0,
    Solid = 1,
    Image = 2,
    #[default]
    Null = 3,
    Shape = 4,
    Text = 5,
    Audio = 6,
    Camera = 13,
    AlphaVideo = 1009,
}

impl LayerType {
    pub fn from_i32(v: i32) -> Self {
        match v {
            0 => Self::PreComp,
            1 => Self::Solid,
            2 => Self::Image,
            3 => Self::Null,
            4 => Self::Shape,
            5 => Self::Text,
            6 => Self::Audio,
            13 => Self::Camera,
            1009 => Self::AlphaVideo,
            _ => Self::Null,
        }
    }
}

/// Matte modes matching `lynx::animax::MatteType`.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MatteType {
    #[default]
    None = 0,
    Alpha = 1,
    AlphaInverted = 2,
    Luma = 3,
    LumaInverted = 4,
    Unknown = 5,
}

impl MatteType {
    pub fn from_i32(v: i32) -> Self {
        match v {
            0 => Self::None,
            1 => Self::Alpha,
            2 => Self::AlphaInverted,
            3 => Self::Luma,
            4 => Self::LumaInverted,
            _ => Self::None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LayerModel {
    pub name: String,
    pub layer_type: LayerType,
    pub id: i32,
    pub parent_id: i32,
    pub ref_id: String,
    pub blend_mode: BlendMode,
    pub transform: AnimatableTransform,
    pub shapes: Vec<ContentModel>,
    pub masks: Vec<MaskModel>,
    pub solid_width: i32,
    pub solid_height: i32,
    pub solid_color: Color,
    pub time_stretch: f32,
    pub start_frame: f32,
    pub in_frame: f32,
    pub out_frame: f32,
    pub pre_comp_width: f32,
    pub pre_comp_height: f32,
    pub hidden: bool,
    pub enable_3d: bool,
    pub text: Option<AnimatableDocument>,
    pub text_properties: Vec<AnimatableTextProperties>,
    pub matte_type: MatteType,
    pub matte_parent: i32,
    pub is_matte_target: bool,
    pub time_remapping: Option<AnimatableFloat>,
    pub blur_effect: Option<BlurEffectModel>,
    pub drop_shadow_effect: Option<DropShadowEffectModel>,
    pub perspective: Option<AnimatableFloat>,
}

impl LayerModel {
    pub fn is_visible_at_frame(&self, frame: f32) -> bool {
        !self.hidden && frame >= self.in_frame && frame < self.out_frame
    }
}
