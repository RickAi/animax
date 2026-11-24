// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

use animax_base::Color;
use animax_model::{PointF, ScaleF};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ColorFilterParam {
    pub color: i32,
    pub mode: i32,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum ValueParamData {
    #[default]
    Null,
    String(String),
    Number(f64),
    Boolean(bool),
    Coordinate { x: f64, y: f64, z: f64 },
    Color(i32),
    ColorFilter(ColorFilterParam),
}

/// Dynamic property value parameter matching `lynx::animax::AnimaXValueParam`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimaXValueParam {
    pub data: ValueParamData,
    pub target_frame: i32,
}

impl AnimaXValueParam {
    pub const FRAME_INDEX_ALL: i32 = -1;

    pub fn null() -> Self {
        Self {
            data: ValueParamData::Null,
            target_frame: Self::FRAME_INDEX_ALL,
        }
    }

    pub fn from_string(v: impl Into<String>, target_frame: i32) -> Self {
        Self {
            data: ValueParamData::String(v.into()),
            target_frame,
        }
    }

    pub fn from_number(v: f64, target_frame: i32) -> Self {
        Self {
            data: ValueParamData::Number(v),
            target_frame,
        }
    }

    pub fn from_bool(v: bool, target_frame: i32) -> Self {
        Self {
            data: ValueParamData::Boolean(v),
            target_frame,
        }
    }

    pub fn from_coord(x: f64, y: f64, z: f64, target_frame: i32) -> Self {
        Self {
            data: ValueParamData::Coordinate { x, y, z },
            target_frame,
        }
    }

    pub fn from_color(argb: i32, target_frame: i32) -> Self {
        Self {
            data: ValueParamData::Color(argb),
            target_frame,
        }
    }

    pub fn from_color_filter(color: i32, mode: i32, target_frame: i32) -> Self {
        Self {
            data: ValueParamData::ColorFilter(ColorFilterParam { color, mode }),
            target_frame,
        }
    }

    pub fn has_target_frame(&self) -> bool {
        self.target_frame != Self::FRAME_INDEX_ALL
    }

    pub fn as_f32(&self) -> Option<f32> {
        match self.data {
            ValueParamData::Number(n) => Some(n as f32),
            ValueParamData::Boolean(b) => Some(if b { 1.0 } else { 0.0 }),
            _ => None,
        }
    }

    pub fn as_i32(&self) -> Option<i32> {
        match self.data {
            ValueParamData::Number(n) => Some(n as i32),
            ValueParamData::Boolean(b) => Some(if b { 1 } else { 0 }),
            ValueParamData::Color(c) => Some(c),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self.data {
            ValueParamData::Boolean(b) => Some(b),
            ValueParamData::Number(n) => Some(n != 0.0),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match &self.data {
            ValueParamData::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_color(&self) -> Option<Color> {
        match &self.data {
            ValueParamData::Color(c) => Some(Color::from_i32(*c)),
            ValueParamData::String(s) => Some(Color::from_hex(s)),
            _ => None,
        }
    }

    pub fn as_point(&self) -> Option<PointF> {
        match self.data {
            ValueParamData::Coordinate { x, y, z } => Some(PointF::new_3d(x as f32, y as f32, z as f32)),
            _ => None,
        }
    }

    pub fn as_scale(&self) -> Option<ScaleF> {
        match self.data {
            ValueParamData::Coordinate { x, y, z } => Some(ScaleF::new_3d(x as f32, y as f32, z as f32)),
            _ => None,
        }
    }
}
