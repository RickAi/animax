// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

use serde::{Deserialize, Serialize};

/// Layer & content dynamic property types matching `lynx::animax::LayerPropertyType` discriminants.
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LayerPropertyType {
    #[default]
    Unknown = 0,
    Visibility = 1,

    TransformOpacity = 2,
    TransformAnchor = 3,
    TransformPosition = 4,
    TransformScale = 5,
    TransformRotation = 6,
    TransformSkew = 7,
    TransformSkewAngle = 8,
    TransformStartOpacity = 9,
    TransformEndOpacity = 10,
    TransformRotationX = 11,
    TransformRotationY = 12,
    TransformRotationZ = 13,

    TextValue = 101,
    TextSize = 102,
    TextColor = 103,
    TextTracking = 104,
    TextSkew = 105,
    TextRangeOffset = 106,
    TextRangeStart = 107,
    TextRangeEnd = 108,

    Color = 201,
    ColorFilter = 202,
    StrokeColor = 203,
    StrokeWidth = 204,
    Opacity = 205,
    BlurRadius = 206,
    EllipseSize = 207,
    RectangleSize = 208,
    CornerRadius = 209,
    Position = 210,
    RepeaterCopies = 211,
    RepeaterOffset = 212,
    PolyStarPoints = 213,
    PolyStarRotation = 214,
    PolyStarInnerRadius = 215,
    PolyStarOuterRadius = 216,
    PolyStarInnerRounded = 217,
    PolyStarOuterRounded = 218,
    DropShadowColor = 219,
    DropShadowOpacity = 220,
    DropShadowDirection = 221,
    DropShadowDistance = 222,
    DropShadowRadius = 223,
}

impl LayerPropertyType {
    pub fn from_u16(v: u16) -> Self {
        match v {
            1 => Self::Visibility,
            2 => Self::TransformOpacity,
            3 => Self::TransformAnchor,
            4 => Self::TransformPosition,
            5 => Self::TransformScale,
            6 => Self::TransformRotation,
            7 => Self::TransformSkew,
            8 => Self::TransformSkewAngle,
            9 => Self::TransformStartOpacity,
            10 => Self::TransformEndOpacity,
            11 => Self::TransformRotationX,
            12 => Self::TransformRotationY,
            13 => Self::TransformRotationZ,
            101 => Self::TextValue,
            102 => Self::TextSize,
            103 => Self::TextColor,
            104 => Self::TextTracking,
            105 => Self::TextSkew,
            106 => Self::TextRangeOffset,
            107 => Self::TextRangeStart,
            108 => Self::TextRangeEnd,
            201 => Self::Color,
            202 => Self::ColorFilter,
            203 => Self::StrokeColor,
            204 => Self::StrokeWidth,
            205 => Self::Opacity,
            206 => Self::BlurRadius,
            207 => Self::EllipseSize,
            208 => Self::RectangleSize,
            209 => Self::CornerRadius,
            210 => Self::Position,
            211 => Self::RepeaterCopies,
            212 => Self::RepeaterOffset,
            213 => Self::PolyStarPoints,
            214 => Self::PolyStarRotation,
            215 => Self::PolyStarInnerRadius,
            216 => Self::PolyStarOuterRadius,
            217 => Self::PolyStarInnerRounded,
            218 => Self::PolyStarOuterRounded,
            219 => Self::DropShadowColor,
            220 => Self::DropShadowOpacity,
            221 => Self::DropShadowDirection,
            222 => Self::DropShadowDistance,
            223 => Self::DropShadowRadius,
            _ => Self::Unknown,
        }
    }
}

/// Resource dynamic property types matching `lynx::animax::ResourcePropertyType`.
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ResourcePropertyType {
    #[default]
    Unknown = 0,
    ImageDirName = 1,
    ImageFileName = 2,
    ImageWidth = 3,
    ImageHeight = 4,

    FontFamily = 101,
    FontStyle = 102,
    FontAscent = 103,
    FontPath = 104,

    VideoDirName = 201,
    VideoFileName = 202,
    VideoWidth = 203,
    VideoHeight = 204,

    AudioDirName = 301,
    AudioFileName = 302,
}
