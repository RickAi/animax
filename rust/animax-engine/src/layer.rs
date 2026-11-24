// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

use crate::content::{compute_content_bounds, draw_content_items};
use animax_animation::{
    eval_color, eval_document, eval_float, eval_shape, eval_transform,
};
use animax_base::Color;
use animax_model::*;
use animax_property::{LayerPropertyType, PropertyStore};
use animax_skity::{Canvas, FontManager, MaskFilter, Matrix, Paint, PaintStyle, Path, Pixmap};
use std::collections::HashMap;

/// Runtime resource context passed down during layer tree traversal.
pub struct RenderContext<'a> {
    pub composition: &'a CompositionModel,
    pub images: &'a HashMap<String, Pixmap>,
    pub video_frames: &'a HashMap<String, Pixmap>,
    pub font_manager: &'a FontManager,
    pub props: &'a PropertyStore,
}

/// Renders a list of layers (composition root or precomp) onto `canvas`.
pub fn draw_layer_list(
    layers: &[LayerModel],
    canvas: &mut Canvas,
    parent_matrix: &Matrix,
    parent_alpha: f32,
    frame: f32,
    ctx: &RenderContext<'_>,
) {
    // Build id -> index map for parent chain & matte parent lookups
    let mut id_to_idx: HashMap<i32, usize> = HashMap::with_capacity(layers.len());
    for (idx, l) in layers.iter().enumerate() {
        if l.id >= 0 {
            id_to_idx.insert(l.id, idx);
        }
    }

    // Identify matte source layers so they aren't rendered directly
    let mut is_matte_source = vec![false; layers.len()];
    let mut matte_for_layer: Vec<Option<usize>> = vec![None; layers.len()];
    for (idx, l) in layers.iter().enumerate() {
        if l.matte_type != MatteType::None && l.matte_type != MatteType::Unknown {
            let matte_idx = if l.matte_parent >= 0 {
                id_to_idx.get(&l.matte_parent).copied()
            } else if idx > 0 {
                Some(idx - 1)
            } else {
                None
            };
            if let Some(mi) = matte_idx {
                is_matte_source[mi] = true;
                matte_for_layer[idx] = Some(mi);
            }
        }
        if l.is_matte_target {
            is_matte_source[idx] = true;
        }
    }

    // Lottie draws layers from bottom (last in JSON array) to top (index 0)
    for idx in (0..layers.len()).rev() {
        if is_matte_source[idx] {
            continue;
        }
        let layer = &layers[idx];
        if let Some(matte_idx) = matte_for_layer[idx] {
            draw_layer_with_matte(
                layer,
                &layers[matte_idx],
                layers,
                &id_to_idx,
                canvas,
                parent_matrix,
                parent_alpha,
                frame,
                ctx,
            );
        } else {
            draw_single_layer(layer, layers, &id_to_idx, canvas, parent_matrix, parent_alpha, frame, ctx);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_layer_with_matte(
    layer: &LayerModel,
    matte_layer: &LayerModel,
    all_layers: &[LayerModel],
    id_to_idx: &HashMap<i32, usize>,
    canvas: &mut Canvas,
    parent_matrix: &Matrix,
    parent_alpha: f32,
    frame: f32,
    ctx: &RenderContext<'_>,
) {
    // 1. Save an isolated offscreen layer for content + matte compositing
    let mut outer_paint = Paint::default();
    outer_paint.blend_mode = layer.blend_mode;
    canvas.save_layer(None, &outer_paint);

    // 2. Render the main layer into the offscreen buffer
    draw_single_layer(layer, all_layers, id_to_idx, canvas, parent_matrix, parent_alpha, frame, ctx);

    // 3. Composite the matte layer onto it with DstIn / DstOut
    let mut matte_paint = Paint::default();
    matte_paint.blend_mode = match matte_layer.matte_type {
        _ => match layer.matte_type {
            MatteType::Alpha | MatteType::Luma => BlendMode::DstIn,
            MatteType::AlphaInverted | MatteType::LumaInverted => BlendMode::DstOut,
            _ => BlendMode::DstIn,
        },
    };
    canvas.save_layer(None, &matte_paint);
    draw_single_layer(matte_layer, all_layers, id_to_idx, canvas, parent_matrix, 1.0, frame, ctx);
    if matches!(layer.matte_type, MatteType::Luma | MatteType::LumaInverted) {
        canvas.convert_top_layer_to_luma_alpha(layer.matte_type == MatteType::LumaInverted);
    }
    canvas.restore(); // composites matte with DstIn/DstOut
    canvas.restore(); // composites masked layer onto parent
}

#[allow(clippy::too_many_arguments)]
fn draw_single_layer(
    layer: &LayerModel,
    all_layers: &[LayerModel],
    id_to_idx: &HashMap<i32, usize>,
    canvas: &mut Canvas,
    parent_matrix: &Matrix,
    parent_alpha: f32,
    frame: f32,
    ctx: &RenderContext<'_>,
) {
    if !layer.is_visible_at_frame(frame) {
        return;
    }

    let path_slice = [layer.name.as_str()];
    if let Some(vis) = ctx
        .props
        .resolve(&path_slice, LayerPropertyType::Visibility, frame.round() as i32)
        .and_then(|v| v.as_bool())
    {
        if !vis {
            return;
        }
    }

    let (world_matrix, local_opacity) =
        compute_layer_world_transform(layer, all_layers, id_to_idx, parent_matrix, frame, ctx);
    let alpha = parent_alpha * (local_opacity as f32 / 100.0);
    if alpha <= 0.0 {
        return;
    }

    let needs_layer_isolation = layer.blend_mode != BlendMode::SrcOver
        || layer.blur_effect.is_some()
        || !layer.masks.is_empty();

    if needs_layer_isolation {
        let mut lp = Paint::default();
        lp.blend_mode = layer.blend_mode;
        if let Some(blur) = &layer.blur_effect {
            let r = eval_float(&blur.blurriness, frame, 0.0);
            if r > 0.5 {
                lp.mask_filter = Some(MaskFilter::Blur { radius: r * 0.5 });
            }
        }
        canvas.save_layer(None, &lp);
    } else {
        canvas.save();
    }

    // Apply layer masks
    for mask in &layer.masks {
        if mask.mode == MaskMode::None {
            continue;
        }
        let shape = eval_shape(&mask.mask_path, frame);
        let mask_path = Path::from_shape_data(&shape);
        if !mask_path.is_empty() {
            canvas.set_matrix(world_matrix);
            canvas.clip_path(&mask_path, true);
        }
    }

    // Render optional drop shadow beneath the layer
    if let Some(ds) = &layer.drop_shadow_effect {
        let ds_color = eval_color(&ds.color, frame, Color::BLACK);
        let ds_opacity = (eval_float(&ds.opacity, frame, 100.0) / 255.0).clamp(0.0, 1.0);
        let ds_dir = eval_float(&ds.direction, frame, 0.0).to_radians();
        let ds_dist = eval_float(&ds.distance, frame, 0.0);
        let ds_rad = eval_float(&ds.radius, frame, 0.0);
        let dx = ds_dir.sin() * ds_dist;
        let dy = ds_dir.cos() * ds_dist;
        let mut shadow_mat = world_matrix;
        shadow_mat.pre_translate(dx, dy);
        let mut sp = Paint::default();
        sp.set_alpha_f(alpha * ds_opacity);
        if ds_rad > 0.5 {
            sp.mask_filter = Some(MaskFilter::Blur { radius: ds_rad * 0.5 });
        }
        sp.color_filter = Some(animax_skity::ColorFilter {
            color: ds_color,
            mode: BlendMode::SrcIn,
        });
        canvas.save_layer(None, &sp);
        draw_layer_body(layer, canvas, &shadow_mat, 1.0, frame, ctx);
        canvas.restore();
    }

    draw_layer_body(layer, canvas, &world_matrix, alpha, frame, ctx);

    canvas.restore();
}

fn draw_layer_body(
    layer: &LayerModel,
    canvas: &mut Canvas,
    world_matrix: &Matrix,
    alpha: f32,
    frame: f32,
    ctx: &RenderContext<'_>,
) {
    match layer.layer_type {
        LayerType::Shape => {
            let mut key_path = vec![layer.name.clone()];
            draw_content_items(
                &layer.shapes,
                canvas,
                world_matrix,
                alpha,
                frame,
                &mut key_path,
                ctx.props,
                &[],
            );
        }
        LayerType::Solid => {
            if layer.solid_width > 0 && layer.solid_height > 0 {
                let mut color = layer.solid_color;
                let path_slice = [layer.name.as_str()];
                if let Some(ov) = ctx
                    .props
                    .resolve(&path_slice, LayerPropertyType::Color, frame.round() as i32)
                    .and_then(|v| v.as_color())
                {
                    color = ov;
                }
                color.a = ((color.a as f32) * alpha).clamp(0.0, 255.0).round() as u8;
                let mut paint = Paint::default();
                paint.style = PaintStyle::Fill;
                paint.color = color;
                canvas.save();
                canvas.set_matrix(*world_matrix);
                canvas.draw_rect(
                    RectF::from_wh(layer.solid_width as f32, layer.solid_height as f32),
                    &paint,
                );
                canvas.restore();
            }
        }
        LayerType::PreComp => {
            if let Some(sub_layers) = ctx.composition.precomps.get(&layer.ref_id) {
                let sub_frame = if let Some(tm) = &layer.time_remapping {
                    eval_float(tm, frame, 0.0) * ctx.composition.frame_rate
                } else {
                    (frame - layer.start_frame) / layer.time_stretch
                };
                canvas.save();
                if layer.pre_comp_width > 0.0 && layer.pre_comp_height > 0.0 {
                    canvas.set_matrix(*world_matrix);
                    canvas.clip_rect(RectF::from_wh(layer.pre_comp_width, layer.pre_comp_height));
                }
                draw_layer_list(sub_layers, canvas, world_matrix, alpha, sub_frame, ctx);
                canvas.restore();
            }
        }
        LayerType::Image => {
            if let Some(pixmap) = ctx.images.get(&layer.ref_id) {
                let mut paint = Paint::default();
                paint.color = Color::WHITE;
                paint.set_alpha_f(alpha);
                let src = RectF::from_wh(pixmap.width() as f32, pixmap.height() as f32);
                let dst = if let Some(asset) = ctx.composition.images.get(&layer.ref_id) {
                    if asset.width > 0 && asset.height > 0 {
                        RectF::from_wh(asset.width as f32, asset.height as f32)
                    } else {
                        src
                    }
                } else {
                    src
                };
                canvas.save();
                canvas.set_matrix(*world_matrix);
                canvas.draw_image_rect(pixmap, src, dst, &paint);
                canvas.restore();
            }
        }
        LayerType::AlphaVideo => {
            if let Some(pixmap) = ctx.video_frames.get(&layer.ref_id) {
                let mut paint = Paint::default();
                paint.color = Color::WHITE;
                paint.set_alpha_f(alpha);
                let src = RectF::from_wh(pixmap.width() as f32, pixmap.height() as f32);
                let dst = if ctx.composition.width() > 0.0 && ctx.composition.height() > 0.0 {
                    RectF::from_wh(ctx.composition.width(), ctx.composition.height())
                } else {
                    src
                };
                canvas.save();
                canvas.set_matrix(*world_matrix);
                canvas.draw_image_rect(pixmap, src, dst, &paint);
                canvas.restore();
            }
        }
        LayerType::Text => {
            draw_text_layer(layer, canvas, world_matrix, alpha, frame, ctx);
        }
        LayerType::Null | LayerType::Audio | LayerType::Camera => {}
    }
}

fn draw_text_layer(
    layer: &LayerModel,
    canvas: &mut Canvas,
    world_matrix: &Matrix,
    alpha: f32,
    frame: f32,
    ctx: &RenderContext<'_>,
) {
    let Some(text_anim) = &layer.text else {
        return;
    };
    let mut doc = eval_document(text_anim, frame);
    let path_slice = [layer.name.as_str()];
    if let Some(ov) = ctx
        .props
        .resolve(&path_slice, LayerPropertyType::TextValue, frame.round() as i32)
        .and_then(|v| v.as_str())
    {
        doc.text = ov.to_string();
    }
    if let Some(ov) = ctx
        .props
        .resolve(&path_slice, LayerPropertyType::TextSize, frame.round() as i32)
        .and_then(|v| v.as_f32())
    {
        doc.size = ov;
    }
    if let Some(ov) = ctx
        .props
        .resolve(&path_slice, LayerPropertyType::TextColor, frame.round() as i32)
        .and_then(|v| v.as_color())
    {
        doc.color = ov.to_i32();
    }

    let mut fill_color = Color::from_i32(doc.color);
    fill_color.a = ((fill_color.a as f32) * alpha).clamp(0.0, 255.0).round() as u8;
    let tracking = (doc.tracking as f32 / 10.0) * (doc.size / 100.0);
    let line_height = if doc.line_height > 0.0 {
        doc.line_height
    } else {
        doc.size * 1.2
    };

    // 1. Try rasterizing with registered FontManager font (`fontdue`)
    if let Some(font) = ctx.font_manager.get(&doc.font_name) {
        let mut paint = Paint::default();
        paint.color = fill_color;
        canvas.save();
        canvas.set_matrix(*world_matrix);
        for (line_idx, line) in doc.text.split(['\r', '\n']).enumerate() {
            let line_w = font.measure_text(line, doc.size, tracking);
            let x_off = match doc.justification {
                DocumentJustification::Center | DocumentJustification::JustifyWithLastLineCenter => {
                    -line_w * 0.5
                }
                DocumentJustification::RightAlign | DocumentJustification::JustifyWithLastLineRight => {
                    -line_w
                }
                _ => 0.0,
            };
            let y_off = (line_idx as f32) * line_height - doc.baseline_shift;
            canvas.draw_text(line, x_off, y_off, font, doc.size, tracking, &paint);
        }
        canvas.restore();
        return;
    }

    // 2. Fallback to embedded Lottie `chars` vector glyph outlines
    if let Some(font_model) = ctx.composition.fonts.get(&doc.font_name) {
        let glyph_scale = doc.size / 100.0;
        let mut paint = Paint::default();
        paint.style = PaintStyle::Fill;
        paint.color = fill_color;
        for (line_idx, line) in doc.text.split(['\r', '\n']).enumerate() {
            let mut cursor_x = 0.0f32;
            let y_off = (line_idx as f32) * line_height - doc.baseline_shift;
            for ch in line.chars() {
                let key = format!("{}|{}|{}", ch, font_model.family, font_model.style);
                if let Some(fc) = ctx.composition.characters.get(&key) {
                    let mut m = *world_matrix;
                    m.pre_translate(cursor_x, y_off);
                    m.pre_scale(glyph_scale, glyph_scale);
                    for sg in &fc.shapes {
                        let mut kp = vec![layer.name.clone()];
                        draw_content_items(
                            &sg.items,
                            canvas,
                            &m,
                            alpha,
                            frame,
                            &mut kp,
                            ctx.props,
                            &[],
                        );
                    }
                    cursor_x += fc.width * glyph_scale + tracking;
                } else {
                    cursor_x += doc.size * 0.5 + tracking;
                }
            }
        }
    }
}

fn compute_layer_world_transform(
    layer: &LayerModel,
    all_layers: &[LayerModel],
    id_to_idx: &HashMap<i32, usize>,
    root_matrix: &Matrix,
    frame: f32,
    ctx: &RenderContext<'_>,
) -> (Matrix, i32) {
    // Walk parent chain up to 32 levels to guard against cycles
    let mut chain = Vec::new();
    let mut cur_parent = layer.parent_id;
    for _ in 0..32 {
        if cur_parent < 0 {
            break;
        }
        if let Some(&pidx) = id_to_idx.get(&cur_parent) {
            chain.push(pidx);
            cur_parent = all_layers[pidx].parent_id;
        } else {
            break;
        }
    }

    let mut mat = *root_matrix;
    for &pidx in chain.iter().rev() {
        let p_ev = eval_transform(&all_layers[pidx].transform, frame);
        mat.pre_concat(&p_ev.matrix);
    }

    let mut ev = eval_transform(&layer.transform, frame);
    let path_slice = [layer.name.as_str()];
    if let Some(op) = ctx
        .props
        .resolve(&path_slice, LayerPropertyType::TransformOpacity, frame.round() as i32)
        .and_then(|v| v.as_i32())
    {
        ev.opacity = op.clamp(0, 100);
    }
    mat.pre_concat(&ev.matrix);
    (mat, ev.opacity)
}

/// Hit-tests `(x, y)` in surface pixel coordinates against visible layers and returns matching layer names (topmost first).
pub fn hit_test_layers(
    layers: &[LayerModel],
    parent_matrix: &Matrix,
    frame: f32,
    x: f32,
    y: f32,
    ctx: &RenderContext<'_>,
) -> Vec<String> {
    let mut id_to_idx: HashMap<i32, usize> = HashMap::with_capacity(layers.len());
    for (idx, l) in layers.iter().enumerate() {
        if l.id >= 0 {
            id_to_idx.insert(l.id, idx);
        }
    }
    let mut hits = Vec::new();
    for layer in layers {
        if !layer.is_visible_at_frame(frame) {
            continue;
        }
        let (world_mat, _) =
            compute_layer_world_transform(layer, layers, &id_to_idx, parent_matrix, frame, ctx);
        let local_bounds = match layer.layer_type {
            LayerType::Shape => compute_content_bounds(&layer.shapes, frame),
            LayerType::Solid => RectF::from_wh(layer.solid_width as f32, layer.solid_height as f32),
            LayerType::Image => ctx
                .composition
                .images
                .get(&layer.ref_id)
                .map_or(RectF::default(), |a| RectF::from_wh(a.width as f32, a.height as f32)),
            LayerType::PreComp => RectF::from_wh(layer.pre_comp_width, layer.pre_comp_height),
            LayerType::AlphaVideo => ctx.composition.bounds,
            _ => RectF::default(),
        };
        if !local_bounds.is_empty() {
            let screen_bounds = world_mat.map_rect(local_bounds);
            if screen_bounds.contains(x, y) && !layer.name.is_empty() {
                hits.push(layer.name.clone());
            }
        }
    }
    hits
}
