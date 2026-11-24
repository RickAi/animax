// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Event types emitted by AnimaXPlayer, matching `lynx::animax::Event` discriminants.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Event {
    Completion = 0,
    Start = 1,
    Repeat = 2,
    Cancel = 3,
    Ready = 4,
    Update = 5,
    Error = 6,
    Fps = 7,
    CompositionReady = 8,
    TapLayer = 9,
    FirstFrame = 10,
    Warning = 11,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventError {
    NoError = 0,
    ResourceNotFound = 1,
    AssetLoadFailed = 5,
    SurfaceInitError = 10,
    LibraryInitError = 12,
    DefaultFontNotFound = 13,
    VideoPlayerError = 100,
    VideoPlayerErrorHasOccurred = 101,
    VideoPlayerSoftwareRenderingNotSupported = 102,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventWarning {
    NoWarning = 0,
    InvalidLottieFormat = 101,
    IllegalAlphaVideoSize = 102,
    AssetCountOverLimit = 103,
    DynamicResourceCannotBeLoaded = 104,
    AssetLoadFailed = 105,
    TextLayerFontInvalid = 106,
    StartFrameGreaterThanEndFrame = 201,
    IllegalSubscribedStartFrame = 202,
    ExecuteBeforeReady = 301,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EventParamValue {
    None,
    Int32(i32),
    Double(f64),
    String(String),
    StringVector(Vec<String>),
}

pub type EventParamMap = BTreeMap<String, EventParamValue>;

pub struct EventKeys;

impl EventKeys {
    pub const CODE: &'static str = "code";
    pub const MESSAGE: &'static str = "msg";
    pub const FPS: &'static str = "fps";
    pub const MAX_DROP_RATE: &'static str = "max_drop_rate";
    pub const LAYER_LIST: &'static str = "layerList";
    pub const ANIMATION_ID: &'static str = "animationID";
    pub const CURRENT: &'static str = "current";
    pub const TOTAL: &'static str = "total";
    pub const LOOP_INDEX: &'static str = "loopIndex";
    pub const PLAYER_ID: &'static str = "elementID";
}
