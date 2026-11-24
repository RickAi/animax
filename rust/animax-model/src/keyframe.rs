// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

use crate::value::PointF;
use serde::{Deserialize, Serialize};

/// Keyframe with optional cubic Bezier easing (single or split X/Y) and spatial tangents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Keyframe<T> {
    pub start_value: T,
    pub end_value: Option<T>,
    pub start_frame: f32,
    pub end_frame: Option<f32>,
    pub hold: bool,
    /// Single cubic Bezier control points (in `[0, 1]` range typically).
    pub cp1: Option<PointF>,
    pub cp2: Option<PointF>,
    /// Multi-dimensional X/Y cubic Bezier control points.
    pub x_cp1: Option<PointF>,
    pub x_cp2: Option<PointF>,
    pub y_cp1: Option<PointF>,
    pub y_cp2: Option<PointF>,
    /// Spatial path tangents (`to` = out tangent, `ti` = in tangent).
    pub path_cp1: Option<PointF>,
    pub path_cp2: Option<PointF>,
    /// Composition frame range cached for progress calculations.
    pub comp_start_frame: f32,
    pub comp_end_frame: f32,
}

impl<T> Keyframe<T> {
    pub fn static_value(value: T) -> Self {
        Self {
            start_value: value,
            end_value: None,
            start_frame: 0.0,
            end_frame: None,
            hold: false,
            cp1: None,
            cp2: None,
            x_cp1: None,
            x_cp2: None,
            y_cp1: None,
            y_cp2: None,
            path_cp1: None,
            path_cp2: None,
            comp_start_frame: 0.0,
            comp_end_frame: 1.0,
        }
    }

    pub fn is_static(&self) -> bool {
        self.end_value.is_none() || self.end_frame.is_none()
    }

    pub fn has_multi_dim_interpolator(&self) -> bool {
        self.x_cp1.is_some() && self.x_cp2.is_some() && self.y_cp1.is_some() && self.y_cp2.is_some()
    }

    pub fn start_progress(&self) -> f32 {
        let dur = self.comp_end_frame - self.comp_start_frame;
        if dur <= 0.0 {
            0.0
        } else {
            (self.start_frame - self.comp_start_frame) / dur
        }
    }

    pub fn end_progress(&self) -> f32 {
        match self.end_frame {
            None => 1.0,
            Some(ef) => {
                let dur = self.comp_end_frame - self.comp_start_frame;
                if dur <= 0.0 {
                    1.0
                } else {
                    (ef - self.comp_start_frame) / dur
                }
            }
        }
    }

    pub fn contains_progress(&self, progress: f32) -> bool {
        progress >= self.start_progress() && progress < self.end_progress()
    }
}
