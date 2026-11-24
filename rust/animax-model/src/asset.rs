// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

use crate::content::ShapeGroupModel;
use crate::value::RectF;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ImageAsset {
    pub id: String,
    pub width: i32,
    pub height: i32,
    pub file_name: String,
    pub dir_name: String,
    /// Optional embedded data URL or decoded RGBA8888 bytes.
    pub embedded_data: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct VideoAsset {
    pub id: String,
    pub width: i32,
    pub height: i32,
    pub file_name: String,
    pub dir_name: String,
    pub alpha_layout: AlphaVideoLayout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AlphaVideoLayout {
    /// Left half RGB, right half Alpha (or configured via `rgb_frame`/`alpha_frame`).
    #[default]
    LeftRgbRightAlpha,
    LeftAlphaRightRgb,
    TopRgbBottomAlpha,
    TopAlphaBottomRgb,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AlphaPlayerItem {
    pub path: String,
    pub align: i32,
    pub version: i32,
    pub total_frames: i32,
    pub video_width: i32,
    pub video_height: i32,
    pub actual_width: i32,
    pub actual_height: i32,
    pub rgb_frame: RectF,
    pub alpha_frame: RectF,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AlphaPlayerConfig {
    pub portrait: Option<AlphaPlayerItem>,
    pub landscape: Option<AlphaPlayerItem>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AudioAsset {
    pub id: String,
    pub file_name: String,
    pub dir_name: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct FontModel {
    pub family: String,
    pub name: String,
    pub style: String,
    pub ascent: f32,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct FontCharacter {
    pub ch: char,
    pub font_family: String,
    pub style: String,
    pub size: f32,
    pub width: f32,
    pub shapes: Vec<ShapeGroupModel>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MarkerModel {
    pub comment: String,
    pub start_frame: f32,
    pub duration_frames: f32,
}
