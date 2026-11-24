// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

use animax_base::{Color, ColorUtil};
use animax_model::*;
use base64::Engine;
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone)]
pub struct ParseError(pub String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AnimaX ParseError: {}", self.0)
    }
}

impl std::error::Error for ParseError {}

struct CompCtx {
    start_frame: f32,
    end_frame: f32,
}

/// Parses a Lottie JSON string (or auto-detects an AlphaPlayer `config.json`) into `CompositionModel`.
pub fn parse_lottie_json(json: &str) -> Result<CompositionModel, ParseError> {
    let root: Value =
        serde_json::from_str(json).map_err(|e| ParseError(format!("invalid json: {e}")))?;
    let obj = root
        .as_object()
        .ok_or_else(|| ParseError("root must be a JSON object".into()))?;

    // Auto-detect AlphaPlayer config.json (`portrait` / `landscape` without Lottie `layers`).
    if !obj.contains_key("layers")
        && (obj.contains_key("portrait") || obj.contains_key("landscape"))
    {
        return crate::alpha_player::parse_alpha_player_value(&root);
    }

    let width = get_f32(&root, "w", 0.0);
    let height = get_f32(&root, "h", 0.0);
    let start_frame = get_f32(&root, "ip", 0.0);
    let end_frame = get_f32(&root, "op", 0.0);
    let frame_rate = get_f32(&root, "fr", 60.0);
    let version = get_str(&root, "v");
    let name = get_str(&root, "nm");

    let ctx = CompCtx {
        start_frame,
        end_frame,
    };

    let mut comp = CompositionModel {
        version,
        name,
        bounds: RectF::from_wh(width, height),
        start_frame,
        end_frame,
        frame_rate,
        ..Default::default()
    };

    if let Some(assets) = root.get("assets").and_then(|v| v.as_array()) {
        parse_assets(assets, &ctx, &mut comp);
    }
    if let Some(fonts) = root
        .get("fonts")
        .and_then(|f| f.get("list"))
        .and_then(|v| v.as_array())
    {
        for f in fonts {
            let model = FontModel {
                family: get_str(f, "fFamily"),
                name: get_str(f, "fName"),
                style: get_str(f, "fStyle"),
                ascent: get_f32(f, "ascent", 0.0),
                path: get_str(f, "fPath"),
            };
            if !model.name.is_empty() {
                comp.fonts.insert(model.name.clone(), model);
            }
        }
    }
    if let Some(chars) = root.get("chars").and_then(|v| v.as_array()) {
        for c in chars {
            let ch_str = get_str(c, "ch");
            if let Some(ch) = ch_str.chars().next() {
                let mut shapes = Vec::new();
                if let Some(items) = c
                    .get("data")
                    .and_then(|d| d.get("shapes"))
                    .and_then(|s| s.as_array())
                {
                    let parsed = parse_content_items(items, &ctx);
                    shapes.push(ShapeGroupModel {
                        name: ch_str.clone(),
                        items: parsed,
                        hidden: false,
                    });
                }
                let family = get_str(c, "fFamily");
                let style = get_str(c, "style");
                let key = format!("{ch}|{family}|{style}");
                comp.characters.insert(
                    key,
                    FontCharacter {
                        ch,
                        font_family: family,
                        style,
                        size: get_f32(c, "size", 0.0),
                        width: get_f32(c, "w", 0.0),
                        shapes,
                    },
                );
            }
        }
    }
    if let Some(markers) = root.get("markers").and_then(|v| v.as_array()) {
        for m in markers {
            comp.markers.push(MarkerModel {
                comment: get_str(m, "cm"),
                start_frame: get_f32(m, "tm", 0.0),
                duration_frames: get_f32(m, "dr", 0.0),
            });
        }
    }
    if let Some(layers) = root.get("layers").and_then(|v| v.as_array()) {
        comp.layers = parse_layers(layers, &ctx);
    }

    Ok(comp)
}

fn parse_assets(assets: &[Value], ctx: &CompCtx, comp: &mut CompositionModel) {
    for a in assets {
        let id = get_str(a, "id");
        if id.is_empty() {
            continue;
        }
        if let Some(layers) = a.get("layers").and_then(|v| v.as_array()) {
            comp.precomps.insert(id, parse_layers(layers, ctx));
            continue;
        }
        let file_name = get_str(a, "p");
        let dir_name = get_str(a, "u");
        let lower = file_name.to_ascii_lowercase();
        if lower.ends_with(".mp4") || lower.ends_with(".webm") || lower.ends_with(".mov") {
            comp.videos.insert(
                id.clone(),
                VideoAsset {
                    id,
                    width: get_i32(a, "w", 0),
                    height: get_i32(a, "h", 0),
                    file_name,
                    dir_name,
                    alpha_layout: AlphaVideoLayout::LeftRgbRightAlpha,
                },
            );
        } else if lower.ends_with(".mp3") || lower.ends_with(".wav") || lower.ends_with(".aac") {
            comp.audios.insert(
                id.clone(),
                AudioAsset {
                    id,
                    file_name,
                    dir_name,
                },
            );
        } else if !file_name.is_empty() {
            let embedded_data = decode_data_uri(&file_name);
            comp.images.insert(
                id.clone(),
                ImageAsset {
                    id,
                    width: get_i32(a, "w", 0),
                    height: get_i32(a, "h", 0),
                    file_name,
                    dir_name,
                    embedded_data,
                },
            );
        }
    }
}

fn decode_data_uri(s: &str) -> Option<Vec<u8>> {
    let rest = s.strip_prefix("data:")?;
    let (_, b64) = rest.split_once(";base64,")?;
    base64::engine::general_purpose::STANDARD.decode(b64.trim()).ok()
}

fn parse_layers(layers: &[Value], ctx: &CompCtx) -> Vec<LayerModel> {
    let mut out = Vec::with_capacity(layers.len());
    for l in layers {
        let layer_type = LayerType::from_i32(get_i32(l, "ty", 3));
        let solid_color = l
            .get("sc")
            .and_then(|v| v.as_str())
            .map(|s| Color::from_i32(ColorUtil::parse_hex_color(s)))
            .unwrap_or(Color::TRANSPARENT);
        let transform = l
            .get("ks")
            .map(|ks| parse_transform(ks, ctx, get_i32(l, "ao", 0) == 1))
            .unwrap_or_default();
        let shapes = l
            .get("shapes")
            .and_then(|v| v.as_array())
            .map(|arr| parse_content_items(arr, ctx))
            .unwrap_or_default();
        let masks = l
            .get("masksProperties")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().map(|m| parse_mask(m, ctx)).collect())
            .unwrap_or_default();

        let (text, text_properties) = if let Some(t) = l.get("t") {
            let doc = t.get("d").map(|d| parse_animatable_document(d, ctx));
            let props = t
                .get("a")
                .and_then(|a| a.as_array())
                .map(|arr| arr.iter().map(|p| parse_text_animator(p, ctx)).collect())
                .unwrap_or_default();
            (doc, props)
        } else {
            (None, Vec::new())
        };

        let (blur_effect, drop_shadow_effect) = parse_effects(l.get("ef"), ctx);

        out.push(LayerModel {
            name: get_str(l, "nm"),
            layer_type,
            id: get_i32(l, "ind", -1),
            parent_id: get_i32(l, "parent", -1),
            ref_id: get_str(l, "refId"),
            blend_mode: BlendMode::from_lottie_bm(get_i32(l, "bm", 0)),
            transform,
            shapes,
            masks,
            solid_width: get_i32(l, "sw", 0),
            solid_height: get_i32(l, "sh", 0),
            solid_color,
            time_stretch: {
                let sr = get_f32(l, "sr", 1.0);
                if sr == 0.0 { 1.0 } else { sr }
            },
            start_frame: get_f32(l, "st", 0.0),
            in_frame: get_f32(l, "ip", ctx.start_frame),
            out_frame: get_f32(l, "op", ctx.end_frame),
            pre_comp_width: get_f32(l, "w", 0.0),
            pre_comp_height: get_f32(l, "h", 0.0),
            hidden: get_bool(l, "hd", false),
            enable_3d: get_i32(l, "ddd", 0) == 1,
            text,
            text_properties,
            matte_type: MatteType::from_i32(get_i32(l, "tt", 0)),
            matte_parent: get_i32(l, "tp", -1),
            is_matte_target: get_i32(l, "td", 0) == 1,
            time_remapping: l.get("tm").map(|v| parse_animatable_float(v, ctx, 1.0)),
            blur_effect,
            drop_shadow_effect,
            perspective: l.get("pe").map(|v| parse_animatable_float(v, ctx, 1.0)),
        });
    }
    out
}

fn parse_effects(
    ef_val: Option<&Value>,
    ctx: &CompCtx,
) -> (Option<BlurEffectModel>, Option<DropShadowEffectModel>) {
    let mut blur = None;
    let mut drop_shadow = None;
    let Some(arr) = ef_val.and_then(|v| v.as_array()) else {
        return (None, None);
    };
    for ef in arr {
        let ty = get_i32(ef, "ty", 0);
        let Some(props) = ef.get("ef").and_then(|v| v.as_array()) else {
            continue;
        };
        if ty == 29 {
            // Gaussian Blur effect
            if let Some(p0) = props.first().and_then(|p| p.get("v")) {
                blur = Some(BlurEffectModel {
                    blurriness: parse_animatable_float(p0, ctx, 1.0),
                });
            }
        } else if ty == 25 {
            // Drop Shadow effect
            let mut ds = DropShadowEffectModel::default();
            for p in props {
                let nm = get_str(p, "nm");
                let Some(v) = p.get("v") else { continue };
                match nm.as_str() {
                    "Shadow Color" => ds.color = parse_animatable_color(v, ctx),
                    "Opacity" => ds.opacity = parse_animatable_float(v, ctx, 1.0),
                    "Direction" => ds.direction = parse_animatable_float(v, ctx, 1.0),
                    "Distance" => ds.distance = parse_animatable_float(v, ctx, 1.0),
                    "Softness" => ds.radius = parse_animatable_float(v, ctx, 1.0),
                    _ => {}
                }
            }
            drop_shadow = Some(ds);
        }
    }
    (blur, drop_shadow)
}

fn parse_mask(m: &Value, ctx: &CompCtx) -> MaskModel {
    let mode = match get_str(m, "mode").as_str() {
        "a" => MaskMode::Add,
        "s" => MaskMode::Subtract,
        "i" => MaskMode::Intersect,
        "n" => MaskMode::None,
        _ => MaskMode::Add,
    };
    MaskModel {
        mode,
        mask_path: m
            .get("pt")
            .map(|v| parse_animatable_shape(v, ctx))
            .unwrap_or_default(),
        opacity: m
            .get("o")
            .map(|v| parse_animatable_i32(v, ctx, 100))
            .unwrap_or_else(|| AnimatableValue::from_static(100)),
        inverted: get_bool(m, "inv", false),
    }
}

fn parse_content_items(items: &[Value], ctx: &CompCtx) -> Vec<ContentModel> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let ty = get_str(item, "ty");
        let hidden = get_bool(item, "hd", false);
        let name = get_str(item, "nm");
        match ty.as_str() {
            "gr" => {
                let sub = item
                    .get("it")
                    .and_then(|v| v.as_array())
                    .map(|a| parse_content_items(a, ctx))
                    .unwrap_or_default();
                out.push(ContentModel::ShapeGroup(ShapeGroupModel {
                    name,
                    items: sub,
                    hidden,
                }));
            }
            "sh" => {
                out.push(ContentModel::ShapePath(ShapePathModel {
                    name,
                    index: get_i32(item, "ind", 0),
                    shape: item
                        .get("ks")
                        .map(|v| parse_animatable_shape(v, ctx))
                        .unwrap_or_default(),
                    hidden,
                }));
            }
            "fl" => {
                let fill_rule = if get_i32(item, "r", 1) == 2 {
                    FillRule::EvenOdd
                } else {
                    FillRule::Winding
                };
                out.push(ContentModel::ShapeFill(ShapeFillModel {
                    name,
                    fill_enabled: get_bool(item, "fillEnabled", true),
                    fill_rule,
                    color: item.get("c").map(|v| parse_animatable_color(v, ctx)),
                    opacity: item.get("o").map(|v| parse_animatable_i32(v, ctx, 100)),
                    hidden,
                }));
            }
            "st" => {
                let (dash_pattern, dash_offset) = parse_dashes(item.get("d"), ctx);
                out.push(ContentModel::ShapeStroke(ShapeStrokeModel {
                    name,
                    color: item
                        .get("c")
                        .map(|v| parse_animatable_color(v, ctx))
                        .unwrap_or_else(|| AnimatableValue::from_static(Color::BLACK)),
                    opacity: item
                        .get("o")
                        .map(|v| parse_animatable_i32(v, ctx, 100))
                        .unwrap_or_else(|| AnimatableValue::from_static(100)),
                    width: item
                        .get("w")
                        .map(|v| parse_animatable_float(v, ctx, 1.0))
                        .unwrap_or_else(|| AnimatableValue::from_static(1.0)),
                    cap_type: parse_line_cap(get_i32(item, "lc", 1)),
                    join_type: parse_line_join(get_i32(item, "lj", 1)),
                    miter_limit: get_f32(item, "ml", 4.0),
                    dash_pattern,
                    dash_offset,
                    hidden,
                }));
            }
            "gf" => {
                let gtype = if get_i32(item, "t", 1) == 2 {
                    GradientType::Radial
                } else {
                    GradientType::Linear
                };
                let fill_rule = if get_i32(item, "r", 1) == 2 {
                    FillRule::EvenOdd
                } else {
                    FillRule::Winding
                };
                out.push(ContentModel::GradientFill(GradientFillModel {
                    name,
                    gradient_type: gtype,
                    fill_rule,
                    gradient_color: item
                        .get("g")
                        .map(|v| parse_animatable_gradient(v, ctx))
                        .unwrap_or_default(),
                    opacity: item
                        .get("o")
                        .map(|v| parse_animatable_i32(v, ctx, 100))
                        .unwrap_or_else(|| AnimatableValue::from_static(100)),
                    start_point: item
                        .get("s")
                        .map(|v| parse_animatable_point(v, ctx))
                        .unwrap_or_default(),
                    end_point: item
                        .get("e")
                        .map(|v| parse_animatable_point(v, ctx))
                        .unwrap_or_default(),
                    highlight_length: item.get("h").map(|v| parse_animatable_float(v, ctx, 1.0)),
                    highlight_angle: item.get("a").map(|v| parse_animatable_float(v, ctx, 1.0)),
                    hidden,
                }));
            }
            "gs" => {
                let gtype = if get_i32(item, "t", 1) == 2 {
                    GradientType::Radial
                } else {
                    GradientType::Linear
                };
                let (dash_pattern, dash_offset) = parse_dashes(item.get("d"), ctx);
                out.push(ContentModel::GradientStroke(GradientStrokeModel {
                    name,
                    gradient_type: gtype,
                    gradient_color: item
                        .get("g")
                        .map(|v| parse_animatable_gradient(v, ctx))
                        .unwrap_or_default(),
                    opacity: item
                        .get("o")
                        .map(|v| parse_animatable_i32(v, ctx, 100))
                        .unwrap_or_else(|| AnimatableValue::from_static(100)),
                    start_point: item
                        .get("s")
                        .map(|v| parse_animatable_point(v, ctx))
                        .unwrap_or_default(),
                    end_point: item
                        .get("e")
                        .map(|v| parse_animatable_point(v, ctx))
                        .unwrap_or_default(),
                    width: item
                        .get("w")
                        .map(|v| parse_animatable_float(v, ctx, 1.0))
                        .unwrap_or_else(|| AnimatableValue::from_static(1.0)),
                    cap_type: parse_line_cap(get_i32(item, "lc", 1)),
                    join_type: parse_line_join(get_i32(item, "lj", 1)),
                    miter_limit: get_f32(item, "ml", 4.0),
                    dash_pattern,
                    dash_offset,
                    hidden,
                }));
            }
            "rc" => {
                out.push(ContentModel::Rectangle(RectangleShapeModel {
                    name,
                    position: item
                        .get("p")
                        .map(|v| parse_animatable_position(v, ctx))
                        .unwrap_or_default(),
                    size: item
                        .get("s")
                        .map(|v| parse_animatable_point(v, ctx))
                        .unwrap_or_default(),
                    corner_radius: item
                        .get("r")
                        .map(|v| parse_animatable_float(v, ctx, 1.0))
                        .unwrap_or_else(|| AnimatableValue::from_static(0.0)),
                    reversed: get_i32(item, "d", 1) == 3,
                    hidden,
                }));
            }
            "el" => {
                out.push(ContentModel::Ellipse(CircleShapeModel {
                    name,
                    position: item
                        .get("p")
                        .map(|v| parse_animatable_position(v, ctx))
                        .unwrap_or_default(),
                    size: item
                        .get("s")
                        .map(|v| parse_animatable_point(v, ctx))
                        .unwrap_or_default(),
                    reversed: get_i32(item, "d", 1) == 3,
                    hidden,
                }));
            }
            "sr" => {
                out.push(ContentModel::Polystar(PolystarShapeModel {
                    name,
                    polystar_type: if get_i32(item, "sy", 1) == 2 {
                        PolystarType::Polygon
                    } else {
                        PolystarType::Star
                    },
                    points: item
                        .get("pt")
                        .map(|v| parse_animatable_float(v, ctx, 1.0))
                        .unwrap_or_else(|| AnimatableValue::from_static(5.0)),
                    position: item
                        .get("p")
                        .map(|v| parse_animatable_position(v, ctx))
                        .unwrap_or_default(),
                    rotation: item
                        .get("r")
                        .map(|v| parse_animatable_float(v, ctx, 1.0))
                        .unwrap_or_else(|| AnimatableValue::from_static(0.0)),
                    inner_radius: item.get("ir").map(|v| parse_animatable_float(v, ctx, 1.0)),
                    outer_radius: item
                        .get("or")
                        .map(|v| parse_animatable_float(v, ctx, 1.0))
                        .unwrap_or_else(|| AnimatableValue::from_static(0.0)),
                    inner_roundedness: item.get("is").map(|v| parse_animatable_float(v, ctx, 1.0)),
                    outer_roundedness: item
                        .get("os")
                        .map(|v| parse_animatable_float(v, ctx, 1.0))
                        .unwrap_or_else(|| AnimatableValue::from_static(0.0)),
                    reversed: get_i32(item, "d", 1) == 3,
                    hidden,
                }));
            }
            "mm" => {
                let mode = match get_i32(item, "mm", 1) {
                    2 => MergePathsMode::Add,
                    3 => MergePathsMode::Subtract,
                    4 => MergePathsMode::Intersect,
                    5 => MergePathsMode::ExcludeIntersections,
                    _ => MergePathsMode::Merge,
                };
                out.push(ContentModel::MergePaths(MergePathsModel {
                    name,
                    mode,
                    hidden,
                }));
            }
            "tm" => {
                let trim_type = if get_i32(item, "m", 1) == 2 {
                    TrimPathType::Individually
                } else {
                    TrimPathType::Simultaneously
                };
                out.push(ContentModel::TrimPath(TrimPathModel {
                    name,
                    trim_type,
                    start: item
                        .get("s")
                        .map(|v| parse_animatable_float(v, ctx, 1.0))
                        .unwrap_or_else(|| AnimatableValue::from_static(0.0)),
                    end: item
                        .get("e")
                        .map(|v| parse_animatable_float(v, ctx, 1.0))
                        .unwrap_or_else(|| AnimatableValue::from_static(100.0)),
                    offset: item
                        .get("o")
                        .map(|v| parse_animatable_float(v, ctx, 1.0))
                        .unwrap_or_else(|| AnimatableValue::from_static(0.0)),
                    hidden,
                }));
            }
            "rd" => {
                out.push(ContentModel::RoundedCorners(RoundedCornersModel {
                    name,
                    corner_radius: item
                        .get("r")
                        .map(|v| parse_animatable_float(v, ctx, 1.0))
                        .unwrap_or_else(|| AnimatableValue::from_static(0.0)),
                    hidden,
                }));
            }
            "rp" => {
                out.push(ContentModel::Repeater(RepeaterModel {
                    name,
                    copies: item
                        .get("c")
                        .map(|v| parse_animatable_float(v, ctx, 1.0))
                        .unwrap_or_else(|| AnimatableValue::from_static(1.0)),
                    offset: item
                        .get("o")
                        .map(|v| parse_animatable_float(v, ctx, 1.0))
                        .unwrap_or_else(|| AnimatableValue::from_static(0.0)),
                    transform: item
                        .get("tr")
                        .map(|tr| parse_transform(tr, ctx, false))
                        .unwrap_or_default(),
                    hidden,
                }));
            }
            "tr" => {
                out.push(ContentModel::Transform(parse_transform(item, ctx, false)));
            }
            _ => {}
        }
    }
    out
}

fn parse_transform(v: &Value, ctx: &CompCtx, auto_orient: bool) -> AnimatableTransform {
    AnimatableTransform {
        anchor_point: v.get("a").map(|x| parse_animatable_position(x, ctx)),
        position: v.get("p").map(|x| parse_animatable_position(x, ctx)),
        scale: v.get("s").map(|x| parse_animatable_scale(x, ctx)),
        rotation: v
            .get("r")
            .or_else(|| v.get("rz"))
            .map(|x| parse_animatable_float(x, ctx, 1.0)),
        rotation_x: v.get("rx").map(|x| parse_animatable_float(x, ctx, 1.0)),
        rotation_y: v.get("ry").map(|x| parse_animatable_float(x, ctx, 1.0)),
        rotation_z: v.get("rz").map(|x| parse_animatable_float(x, ctx, 1.0)),
        opacity: v.get("o").map(|x| parse_animatable_i32(x, ctx, 100)),
        start_opacity: v.get("so").map(|x| parse_animatable_float(x, ctx, 1.0)),
        end_opacity: v.get("eo").map(|x| parse_animatable_float(x, ctx, 1.0)),
        skew: v.get("sk").map(|x| parse_animatable_float(x, ctx, 1.0)),
        skew_angle: v.get("sa").map(|x| parse_animatable_float(x, ctx, 1.0)),
        auto_orient,
    }
}

fn parse_text_animator(v: &Value, ctx: &CompCtx) -> AnimatableTextProperties {
    let a = v.get("a");
    let s = v.get("s");
    AnimatableTextProperties {
        color: a.and_then(|x| x.get("fc")).map(|x| parse_animatable_color(x, ctx)),
        stroke_color: a.and_then(|x| x.get("sc")).map(|x| parse_animatable_color(x, ctx)),
        stroke_width: a.and_then(|x| x.get("sw")).map(|x| parse_animatable_float(x, ctx, 1.0)),
        tracking: a.and_then(|x| x.get("t")).map(|x| parse_animatable_float(x, ctx, 1.0)),
        opacity: a.and_then(|x| x.get("o")).map(|x| parse_animatable_i32(x, ctx, 100)),
        position: a.and_then(|x| x.get("p")).map(|x| parse_animatable_position(x, ctx)),
        scale: a.and_then(|x| x.get("s")).map(|x| parse_animatable_scale(x, ctx)),
        rotation: a.and_then(|x| x.get("r")).map(|x| parse_animatable_float(x, ctx, 1.0)),
        range_selector: s.map(|sel| TextRangeSelector {
            start: sel.get("s").map(|x| parse_animatable_float(x, ctx, 1.0)),
            end: sel.get("e").map(|x| parse_animatable_float(x, ctx, 1.0)),
            offset: sel.get("o").map(|x| parse_animatable_float(x, ctx, 1.0)),
            units: get_i32(sel, "r", 1),
        }),
    }
}

fn parse_dashes(d: Option<&Value>, ctx: &CompCtx) -> (Vec<AnimatableFloat>, Option<AnimatableFloat>) {
    let mut pattern = Vec::new();
    let mut offset = None;
    let Some(arr) = d.and_then(|v| v.as_array()) else {
        return (pattern, offset);
    };
    for entry in arr {
        let n = get_str(entry, "n");
        let Some(v) = entry.get("v") else { continue };
        let anim = parse_animatable_float(v, ctx, 1.0);
        if n == "o" {
            offset = Some(anim);
        } else if n == "d" || n == "g" {
            pattern.push(anim);
        }
    }
    (pattern, offset)
}

fn parse_line_cap(v: i32) -> LineCapType {
    match v {
        2 => LineCapType::Round,
        3 => LineCapType::Square,
        _ => LineCapType::Butt,
    }
}

fn parse_line_join(v: i32) -> LineJoinType {
    match v {
        2 => LineJoinType::Round,
        3 => LineJoinType::Bevel,
        _ => LineJoinType::Miter,
    }
}

fn parse_keyframes<T: Clone, F: Fn(&Value) -> T>(
    v: &Value,
    ctx: &CompCtx,
    parse_val: F,
) -> AnimatableValue<T> {
    let Some(k) = v.get("k") else {
        return AnimatableValue::default();
    };
    let animated = v.get("a").and_then(|a| a.as_i64()).unwrap_or(0) == 1
        || (k.is_array() && k.as_array().is_some_and(|arr| arr.first().is_some_and(|e| e.get("t").is_some())));

    if !animated {
        return AnimatableValue::from_static(parse_val(k));
    }
    let Some(arr) = k.as_array() else {
        return AnimatableValue::from_static(parse_val(k));
    };
    let mut frames: Vec<Keyframe<T>> = Vec::with_capacity(arr.len());
    for (idx, kf) in arr.iter().enumerate() {
        let start_frame = get_f32(kf, "t", 0.0);
        let hold = get_i32(kf, "h", 0) == 1;
        let start_value = kf
            .get("s")
            .map(&parse_val)
            .unwrap_or_else(|| parse_val(kf));
        let end_value = kf.get("e").map(&parse_val).or_else(|| {
            arr.get(idx + 1)
                .and_then(|next| next.get("s"))
                .map(&parse_val)
        });
        let end_frame = arr.get(idx + 1).map(|next| get_f32(next, "t", start_frame));

        let (cp1, x_cp1, y_cp1) = parse_easing_point(kf.get("o"));
        let (cp2, x_cp2, y_cp2) = parse_easing_point(kf.get("i"));
        let path_cp1 = kf.get("to").map(parse_point_val);
        let path_cp2 = kf.get("ti").map(parse_point_val);

        frames.push(Keyframe {
            start_value,
            end_value: if hold { None } else { end_value },
            start_frame,
            end_frame,
            hold,
            cp1,
            cp2,
            x_cp1,
            x_cp2,
            y_cp1,
            y_cp2,
            path_cp1,
            path_cp2,
            comp_start_frame: ctx.start_frame,
            comp_end_frame: ctx.end_frame,
        });
    }
    // Remove trailing dummy keyframe that only marks end_frame when previous keyframe already captured it.
    if frames.len() >= 2 && frames.last().is_some_and(|last| last.end_frame.is_none() && last.end_value.is_none()) {
        // Keep if needed as clamp sentinel; standard Lottie ends on the last keyframe's start_value.
    }
    AnimatableValue::from_keyframes(frames)
}

fn parse_easing_point(v: Option<&Value>) -> (Option<PointF>, Option<PointF>, Option<PointF>) {
    let Some(obj) = v.and_then(|x| x.as_object()) else {
        return (None, None, None);
    };
    let (xv, yv) = match (obj.get("x"), obj.get("y")) {
        (Some(x), Some(y)) => (x, y),
        _ => return (None, None, None),
    };
    if let (Some(xa), Some(ya)) = (xv.as_array(), yv.as_array()) {
        let x0 = xa.first().and_then(|n| n.as_f64()).unwrap_or(0.0) as f32;
        let y0 = ya.first().and_then(|n| n.as_f64()).unwrap_or(0.0) as f32;
        if xa.len() >= 2 && ya.len() >= 2 {
            let x1 = xa[1].as_f64().unwrap_or(0.0) as f32;
            let y1 = ya[1].as_f64().unwrap_or(0.0) as f32;
            return (
                Some(PointF::new(x0, y0)),
                Some(PointF::new(x0, y0)),
                Some(PointF::new(x1, y1)),
            );
        }
        return (Some(PointF::new(x0, y0)), None, None);
    }
    let x = xv.as_f64().unwrap_or(0.0) as f32;
    let y = yv.as_f64().unwrap_or(0.0) as f32;
    (Some(PointF::new(x, y)), None, None)
}

fn parse_animatable_float(v: &Value, ctx: &CompCtx, scale: f32) -> AnimatableFloat {
    parse_keyframes(v, ctx, |val| parse_float_val(val) * scale)
}

fn parse_animatable_i32(v: &Value, ctx: &CompCtx, default: i32) -> AnimatableInteger {
    if v.get("k").is_none() {
        return AnimatableValue::from_static(default);
    }
    parse_keyframes(v, ctx, |val| parse_float_val(val).round() as i32)
}

fn parse_animatable_color(v: &Value, ctx: &CompCtx) -> AnimatableColor {
    parse_keyframes(v, ctx, parse_color_val)
}

fn parse_animatable_point(v: &Value, ctx: &CompCtx) -> AnimatablePoint {
    parse_keyframes(v, ctx, parse_point_val)
}

fn parse_animatable_position(v: &Value, ctx: &CompCtx) -> AnimatablePosition {
    if get_bool(v, "s", false) {
        let x = v
            .get("x")
            .map(|p| parse_animatable_float(p, ctx, 1.0))
            .unwrap_or_else(|| AnimatableValue::from_static(0.0));
        let y = v
            .get("y")
            .map(|p| parse_animatable_float(p, ctx, 1.0))
            .unwrap_or_else(|| AnimatableValue::from_static(0.0));
        let z = v.get("z").map(|p| parse_animatable_float(p, ctx, 1.0));
        AnimatablePosition::Split { x, y, z }
    } else {
        AnimatablePosition::Path(parse_animatable_point(v, ctx))
    }
}

fn parse_animatable_scale(v: &Value, ctx: &CompCtx) -> AnimatableScale {
    parse_keyframes(v, ctx, |val| {
        let p = parse_point_val(val);
        ScaleF::new_3d(p.x / 100.0, p.y / 100.0, if p.z == 0.0 { 1.0 } else { p.z / 100.0 })
    })
}

fn parse_animatable_shape(v: &Value, ctx: &CompCtx) -> AnimatableShape {
    parse_keyframes(v, ctx, parse_shape_val)
}

fn parse_animatable_gradient(v: &Value, ctx: &CompCtx) -> AnimatableGradient {
    let color_points = get_i32(v, "p", 0).max(0) as usize;
    let Some(k) = v.get("k") else {
        return AnimatableValue::default();
    };
    parse_keyframes(k, ctx, |val| parse_gradient_val(val, color_points))
}

fn parse_animatable_document(v: &Value, ctx: &CompCtx) -> AnimatableDocument {
    parse_keyframes(v, ctx, parse_document_val)
}

fn parse_float_val(v: &Value) -> f32 {
    if let Some(n) = v.as_f64() {
        return n as f32;
    }
    if let Some(arr) = v.as_array() {
        return arr.first().and_then(|x| x.as_f64()).unwrap_or(0.0) as f32;
    }
    0.0
}

fn parse_point_val(v: &Value) -> PointF {
    if let Some(arr) = v.as_array() {
        let x = arr.first().and_then(|n| n.as_f64()).unwrap_or(0.0) as f32;
        let y = arr.get(1).and_then(|n| n.as_f64()).unwrap_or(0.0) as f32;
        let z = arr.get(2).and_then(|n| n.as_f64()).unwrap_or(0.0) as f32;
        return PointF::new_3d(x, y, z);
    }
    if let Some(obj) = v.as_object() {
        let x = obj.get("x").and_then(|n| n.as_f64()).unwrap_or(0.0) as f32;
        let y = obj.get("y").and_then(|n| n.as_f64()).unwrap_or(0.0) as f32;
        return PointF::new(x, y);
    }
    PointF::default()
}

fn parse_color_val(v: &Value) -> Color {
    let Some(arr) = v.as_array() else {
        return Color::BLACK;
    };
    let r = arr.first().and_then(|n| n.as_f64()).unwrap_or(0.0) as f32;
    let g = arr.get(1).and_then(|n| n.as_f64()).unwrap_or(0.0) as f32;
    let b = arr.get(2).and_then(|n| n.as_f64()).unwrap_or(0.0) as f32;
    let a = arr.get(3).and_then(|n| n.as_f64()).unwrap_or(1.0) as f32;
    Color::from_normalized_rgba(r, g, b, a)
}

fn parse_shape_val(v: &Value) -> ShapeData {
    let obj = if let Some(arr) = v.as_array() {
        arr.first().unwrap_or(v)
    } else {
        v
    };
    let closed = get_bool(obj, "c", false);
    let verts = obj.get("v").and_then(|x| x.as_array());
    let ins = obj.get("i").and_then(|x| x.as_array());
    let outs = obj.get("o").and_then(|x| x.as_array());

    let (Some(verts), Some(ins), Some(outs)) = (verts, ins, outs) else {
        return ShapeData::default();
    };
    let len = verts.len().min(ins.len()).min(outs.len());
    if len == 0 {
        return ShapeData::default();
    }

    let pts: Vec<PointF> = (0..len).map(|i| parse_point_val(&verts[i])).collect();
    let in_t: Vec<PointF> = (0..len).map(|i| parse_point_val(&ins[i])).collect();
    let out_t: Vec<PointF> = (0..len).map(|i| parse_point_val(&outs[i])).collect();

    let mut curves = Vec::with_capacity(if closed { len } else { len.saturating_sub(1) });
    for i in 1..len {
        let prev = pts[i - 1];
        let cur = pts[i];
        let cp1 = PointF::new(prev.x + out_t[i - 1].x, prev.y + out_t[i - 1].y);
        let cp2 = PointF::new(cur.x + in_t[i].x, cur.y + in_t[i].y);
        curves.push(CubicCurve {
            cp1,
            cp2,
            vertex: cur,
        });
    }
    if closed && len > 0 {
        let prev = pts[len - 1];
        let cur = pts[0];
        let cp1 = PointF::new(prev.x + out_t[len - 1].x, prev.y + out_t[len - 1].y);
        let cp2 = PointF::new(cur.x + in_t[0].x, cur.y + in_t[0].y);
        curves.push(CubicCurve {
            cp1,
            cp2,
            vertex: cur,
        });
    }

    ShapeData {
        initial_point: pts[0],
        curves,
        closed,
    }
}

fn parse_gradient_val(v: &Value, color_points: usize) -> GradientColor {
    let Some(arr) = v.as_array() else {
        return GradientColor::default();
    };
    let nums: Vec<f32> = arr.iter().filter_map(|x| x.as_f64().map(|n| n as f32)).collect();
    let cp_count = if color_points > 0 {
        color_points.min(nums.len() / 4)
    } else {
        nums.len() / 4
    };
    let mut positions = Vec::with_capacity(cp_count);
    let mut colors = Vec::with_capacity(cp_count);

    for i in 0..cp_count {
        let base = i * 4;
        let pos = nums[base];
        let r = (nums[base + 1].clamp(0.0, 1.0) * 255.0).round() as u8;
        let g = (nums[base + 2].clamp(0.0, 1.0) * 255.0).round() as u8;
        let b = (nums[base + 3].clamp(0.0, 1.0) * 255.0).round() as u8;
        positions.push(pos);
        colors.push(Color::pack_argb(255, r, g, b));
    }

    // Apply optional opacity stops appended after `cp_count * 4`.
    let alpha_start = cp_count * 4;
    if nums.len() >= alpha_start + 2 {
        let mut alpha_stops = Vec::new();
        let mut idx = alpha_start;
        while idx + 1 < nums.len() {
            alpha_stops.push((nums[idx], nums[idx + 1].clamp(0.0, 1.0)));
            idx += 2;
        }
        for i in 0..positions.len() {
            let p = positions[i];
            let alpha = sample_alpha_stops(&alpha_stops, p);
            let mut c = Color::from_i32(colors[i]);
            c.a = (alpha * 255.0).round() as u8;
            colors[i] = c.to_i32();
        }
    }

    GradientColor::new(positions, colors)
}

fn sample_alpha_stops(stops: &[(f32, f32)], pos: f32) -> f32 {
    if stops.is_empty() {
        return 1.0;
    }
    if pos <= stops[0].0 {
        return stops[0].1;
    }
    for w in stops.windows(2) {
        if pos <= w[1].0 {
            let span = w[1].0 - w[0].0;
            if span <= 1e-6 {
                return w[0].1;
            }
            let t = (pos - w[0].0) / span;
            return w[0].1 + (w[1].1 - w[0].1) * t;
        }
    }
    stops.last().map_or(1.0, |s| s.1)
}

fn parse_document_val(v: &Value) -> DocumentData {
    let fc = v
        .get("fc")
        .map(parse_color_val)
        .unwrap_or(Color::BLACK)
        .to_i32();
    let sc = v
        .get("sc")
        .map(parse_color_val)
        .unwrap_or(Color::TRANSPARENT)
        .to_i32();
    DocumentData {
        text: get_str(v, "t"),
        font_name: get_str(v, "f"),
        size: get_f32(v, "s", 12.0),
        justification: DocumentJustification::from_u8(get_i32(v, "j", 0) as u8),
        tracking: get_i32(v, "tr", 0),
        line_height: get_f32(v, "lh", 0.0),
        baseline_shift: get_f32(v, "ls", 0.0),
        color: fc,
        stroke_color: sc,
        stroke_width: get_f32(v, "sw", 0.0),
        stroke_overfill: get_bool(v, "of", true),
        box_position: v.get("ps").map(parse_point_val),
        box_size: v.get("sz").map(parse_point_val),
    }
}

pub(crate) fn get_f32(v: &Value, key: &str, default: f32) -> f32 {
    v.get(key).and_then(|x| x.as_f64()).map(|n| n as f32).unwrap_or(default)
}

pub(crate) fn get_i32(v: &Value, key: &str, default: i32) -> i32 {
    v.get(key).and_then(|x| x.as_i64()).map(|n| n as i32).unwrap_or(default)
}

pub(crate) fn get_bool(v: &Value, key: &str, default: bool) -> bool {
    match v.get(key) {
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_i64().unwrap_or(0) != 0,
        _ => default,
    }
}

pub(crate) fn get_str(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or_default()
        .to_string()
}

/// Helper to construct a map of extracted assets by filename.
pub fn index_assets_by_name(files: HashMap<String, Vec<u8>>) -> HashMap<String, Vec<u8>> {
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_lottie_shape_layer() {
        let json = r#"{
            "v": "5.7.4",
            "fr": 30,
            "ip": 0,
            "op": 60,
            "w": 200,
            "h": 200,
            "nm": "TestComp",
            "layers": [{
                "ty": 4,
                "ind": 1,
                "nm": "Shape Layer 1",
                "ip": 0,
                "op": 60,
                "ks": {
                    "o": {"a": 0, "k": 100},
                    "p": {"a": 1, "k": [
                        {"t": 0, "s": [50, 50], "e": [150, 150], "o": {"x": 0.5, "y": 0}, "i": {"x": 0.5, "y": 1}},
                        {"t": 60, "s": [150, 150]}
                    ]},
                    "s": {"a": 0, "k": [100, 100]}
                },
                "shapes": [{
                    "ty": "gr",
                    "nm": "Group 1",
                    "it": [
                        {"ty": "rc", "nm": "Rect", "p": {"a": 0, "k": [0, 0]}, "s": {"a": 0, "k": [80, 60]}, "r": {"a": 0, "k": 8}},
                        {"ty": "fl", "nm": "Fill", "c": {"a": 0, "k": [1, 0, 0, 1]}, "o": {"a": 0, "k": 100}}
                    ]
                }]
            }]
        }"#;

        let comp = parse_lottie_json(json).expect("should parse");
        assert_eq!(comp.name, "TestComp");
        assert_eq!(comp.width(), 200.0);
        assert_eq!(comp.duration_ms(), 2000);
        assert_eq!(comp.layers.len(), 1);
        assert_eq!(comp.layers[0].layer_type, LayerType::Shape);
        assert_eq!(comp.layers[0].shapes.len(), 1);
    }
}
