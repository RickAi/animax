// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

use crate::animatable::{
    AnimatableColor, AnimatableFloat, AnimatableGradient, AnimatableInteger, AnimatablePoint,
    AnimatablePosition, AnimatableShape, AnimatableTransform,
};
use crate::value::GradientType;
use serde::{Deserialize, Serialize};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FillRule {
    #[default]
    Winding = 1,
    EvenOdd = 2,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LineCapType {
    #[default]
    Butt = 1,
    Round = 2,
    Square = 3,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LineJoinType {
    #[default]
    Miter = 1,
    Round = 2,
    Bevel = 3,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PolystarType {
    #[default]
    Star = 1,
    Polygon = 2,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MergePathsMode {
    #[default]
    Merge = 1,
    Add = 2,
    Subtract = 3,
    Intersect = 4,
    ExcludeIntersections = 5,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TrimPathType {
    #[default]
    Simultaneously = 1,
    Individually = 2,
}

/// All Lottie vector shape & modifier content models.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ContentModel {
    ShapeGroup(ShapeGroupModel),
    ShapePath(ShapePathModel),
    ShapeFill(ShapeFillModel),
    ShapeStroke(ShapeStrokeModel),
    GradientFill(GradientFillModel),
    GradientStroke(GradientStrokeModel),
    Rectangle(RectangleShapeModel),
    Ellipse(CircleShapeModel),
    Polystar(PolystarShapeModel),
    MergePaths(MergePathsModel),
    TrimPath(TrimPathModel),
    RoundedCorners(RoundedCornersModel),
    Repeater(RepeaterModel),
    Transform(AnimatableTransform),
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ShapeGroupModel {
    pub name: String,
    pub items: Vec<ContentModel>,
    pub hidden: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ShapePathModel {
    pub name: String,
    pub index: i32,
    pub shape: AnimatableShape,
    pub hidden: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ShapeFillModel {
    pub name: String,
    pub fill_enabled: bool,
    pub fill_rule: FillRule,
    pub color: Option<AnimatableColor>,
    pub opacity: Option<AnimatableInteger>,
    pub hidden: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ShapeStrokeModel {
    pub name: String,
    pub color: AnimatableColor,
    pub opacity: AnimatableInteger,
    pub width: AnimatableFloat,
    pub cap_type: LineCapType,
    pub join_type: LineJoinType,
    pub miter_limit: f32,
    pub dash_pattern: Vec<AnimatableFloat>,
    pub dash_offset: Option<AnimatableFloat>,
    pub hidden: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GradientFillModel {
    pub name: String,
    pub gradient_type: GradientType,
    pub fill_rule: FillRule,
    pub gradient_color: AnimatableGradient,
    pub opacity: AnimatableInteger,
    pub start_point: AnimatablePoint,
    pub end_point: AnimatablePoint,
    pub highlight_length: Option<AnimatableFloat>,
    pub highlight_angle: Option<AnimatableFloat>,
    pub hidden: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GradientStrokeModel {
    pub name: String,
    pub gradient_type: GradientType,
    pub gradient_color: AnimatableGradient,
    pub opacity: AnimatableInteger,
    pub start_point: AnimatablePoint,
    pub end_point: AnimatablePoint,
    pub width: AnimatableFloat,
    pub cap_type: LineCapType,
    pub join_type: LineJoinType,
    pub miter_limit: f32,
    pub dash_pattern: Vec<AnimatableFloat>,
    pub dash_offset: Option<AnimatableFloat>,
    pub hidden: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RectangleShapeModel {
    pub name: String,
    pub position: AnimatablePosition,
    pub size: AnimatablePoint,
    pub corner_radius: AnimatableFloat,
    pub reversed: bool,
    pub hidden: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CircleShapeModel {
    pub name: String,
    pub position: AnimatablePosition,
    pub size: AnimatablePoint,
    pub reversed: bool,
    pub hidden: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PolystarShapeModel {
    pub name: String,
    pub polystar_type: PolystarType,
    pub points: AnimatableFloat,
    pub position: AnimatablePosition,
    pub rotation: AnimatableFloat,
    pub inner_radius: Option<AnimatableFloat>,
    pub outer_radius: AnimatableFloat,
    pub inner_roundedness: Option<AnimatableFloat>,
    pub outer_roundedness: AnimatableFloat,
    pub reversed: bool,
    pub hidden: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MergePathsModel {
    pub name: String,
    pub mode: MergePathsMode,
    pub hidden: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TrimPathModel {
    pub name: String,
    pub trim_type: TrimPathType,
    pub start: AnimatableFloat,
    pub end: AnimatableFloat,
    pub offset: AnimatableFloat,
    pub hidden: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RoundedCornersModel {
    pub name: String,
    pub corner_radius: AnimatableFloat,
    pub hidden: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RepeaterModel {
    pub name: String,
    pub copies: AnimatableFloat,
    pub offset: AnimatableFloat,
    pub transform: AnimatableTransform,
    pub hidden: bool,
}
