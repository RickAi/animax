// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

use crate::asset::{
    AlphaPlayerConfig, AudioAsset, FontCharacter, FontModel, ImageAsset, MarkerModel, VideoAsset,
};
use crate::layer::LayerModel;
use crate::value::RectF;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Root Lottie / Alpha Video composition model matching `lynx::animax::CompositionModel`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CompositionModel {
    pub version: String,
    pub name: String,
    pub bounds: RectF,
    pub start_frame: f32,
    pub end_frame: f32,
    pub frame_rate: f32,
    pub layers: Vec<LayerModel>,
    pub precomps: HashMap<String, Vec<LayerModel>>,
    pub images: HashMap<String, ImageAsset>,
    pub videos: HashMap<String, VideoAsset>,
    pub audios: HashMap<String, AudioAsset>,
    pub fonts: HashMap<String, FontModel>,
    pub characters: HashMap<String, FontCharacter>,
    pub markers: Vec<MarkerModel>,
    pub alpha_player_config: Option<AlphaPlayerConfig>,
}

impl CompositionModel {
    pub fn duration_frames(&self) -> f32 {
        (self.end_frame - self.start_frame).max(0.0)
    }

    pub fn duration_ms(&self) -> i64 {
        if self.frame_rate <= 0.0 {
            return 0;
        }
        ((self.duration_frames() / self.frame_rate) * 1000.0).round() as i64
    }

    pub fn width(&self) -> f32 {
        self.bounds.width()
    }

    pub fn height(&self) -> f32 {
        self.bounds.height()
    }

    pub fn progress_to_frame(&self, progress: f32) -> f32 {
        self.start_frame + progress.clamp(0.0, 1.0) * self.duration_frames()
    }

    pub fn frame_to_progress(&self, frame: f32) -> f32 {
        let dur = self.duration_frames();
        if dur <= 0.0 {
            0.0
        } else {
            ((frame - self.start_frame) / dur).clamp(0.0, 1.0)
        }
    }

    pub fn find_marker(&self, name: &str) -> Option<&MarkerModel> {
        self.markers.iter().find(|m| m.comment == name)
    }
}
