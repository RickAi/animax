// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

use crate::interpolator::cubic_bezier_ease;
use animax_base::{gamma_evaluate_color, lerp_f32, lerp_i32, Color};
use animax_model::*;
use animax_skity::{Path, PathMeasure};

fn find_keyframe<T>(keyframes: &[Keyframe<T>], frame: f32) -> Option<(&Keyframe<T>, f32)> {
    if keyframes.is_empty() {
        return None;
    }
    if keyframes.len() == 1 {
        return Some((&keyframes[0], 0.0));
    }
    if frame <= keyframes[0].start_frame {
        return Some((&keyframes[0], 0.0));
    }
    let last = &keyframes[keyframes.len() - 1];
    if frame >= last.start_frame {
        return Some((last, 1.0));
    }
    // Binary search for the active segment
    let idx = keyframes
        .partition_point(|kf| kf.start_frame <= frame)
        .saturating_sub(1);
    let kf = &keyframes[idx];
    let ef = kf
        .end_frame
        .unwrap_or_else(|| keyframes.get(idx + 1).map_or(kf.start_frame, |n| n.start_frame));
    let span = ef - kf.start_frame;
    let linear = if span <= 0.0 {
        0.0
    } else {
        ((frame - kf.start_frame) / span).clamp(0.0, 1.0)
    };
    Some((kf, linear))
}

#[inline]
fn eased_progress<T>(kf: &Keyframe<T>, linear: f32) -> f32 {
    if kf.hold || kf.end_value.is_none() {
        return 0.0;
    }
    match (kf.cp1, kf.cp2) {
        (Some(cp1), Some(cp2)) => cubic_bezier_ease(cp1, cp2, linear),
        _ => linear,
    }
}

pub fn eval_float(anim: &AnimatableFloat, frame: f32, default: f32) -> f32 {
    let Some((kf, linear)) = find_keyframe(&anim.keyframes, frame) else {
        return default;
    };
    let Some(end) = kf.end_value else {
        return kf.start_value;
    };
    let t = eased_progress(kf, linear);
    lerp_f32(kf.start_value, end, t)
}

pub fn eval_i32(anim: &AnimatableInteger, frame: f32, default: i32) -> i32 {
    let Some((kf, linear)) = find_keyframe(&anim.keyframes, frame) else {
        return default;
    };
    let Some(end) = kf.end_value else {
        return kf.start_value;
    };
    let t = eased_progress(kf, linear);
    lerp_i32(kf.start_value, end, t)
}

pub fn eval_color(anim: &AnimatableColor, frame: f32, default: Color) -> Color {
    let Some((kf, linear)) = find_keyframe(&anim.keyframes, frame) else {
        return default;
    };
    let Some(end) = kf.end_value else {
        return kf.start_value;
    };
    let t = eased_progress(kf, linear);
    gamma_evaluate_color(kf.start_value, end, t)
}

pub fn eval_point(anim: &AnimatablePoint, frame: f32, default: PointF) -> PointF {
    let Some((kf, linear)) = find_keyframe(&anim.keyframes, frame) else {
        return default;
    };
    let Some(end) = kf.end_value else {
        return kf.start_value;
    };
    if kf.hold {
        return kf.start_value;
    }
    if kf.has_multi_dim_interpolator() {
        let tx = cubic_bezier_ease(kf.x_cp1.unwrap(), kf.x_cp2.unwrap(), linear);
        let ty = cubic_bezier_ease(kf.y_cp1.unwrap(), kf.y_cp2.unwrap(), linear);
        let tz = eased_progress(kf, linear);
        return PointF::new_3d(
            lerp_f32(kf.start_value.x, end.x, tx),
            lerp_f32(kf.start_value.y, end.y, ty),
            lerp_f32(kf.start_value.z, end.z, tz),
        );
    }
    let t = eased_progress(kf, linear);
    kf.start_value.lerp(end, t)
}

pub fn eval_scale(anim: &AnimatableScale, frame: f32) -> ScaleF {
    let Some((kf, linear)) = find_keyframe(&anim.keyframes, frame) else {
        return ScaleF::default();
    };
    let Some(end) = kf.end_value else {
        return kf.start_value;
    };
    let t = eased_progress(kf, linear);
    kf.start_value.lerp(end, t)
}

pub fn eval_gradient(anim: &AnimatableGradient, frame: f32) -> GradientColor {
    let Some((kf, linear)) = find_keyframe(&anim.keyframes, frame) else {
        return GradientColor::default();
    };
    let Some(end) = &kf.end_value else {
        return kf.start_value.clone();
    };
    let t = eased_progress(kf, linear);
    let mut out = GradientColor::default();
    out.lerp_color(&kf.start_value, end, t);
    out
}

pub fn eval_shape(anim: &AnimatableShape, frame: f32) -> ShapeData {
    let Some((kf, linear)) = find_keyframe(&anim.keyframes, frame) else {
        return ShapeData::default();
    };
    let Some(end) = &kf.end_value else {
        return kf.start_value.clone();
    };
    let t = eased_progress(kf, linear);
    let mut out = ShapeData::default();
    out.interpolate_between(&kf.start_value, end, t);
    out
}

pub fn eval_document(anim: &AnimatableDocument, frame: f32) -> DocumentData {
    let Some((kf, linear)) = find_keyframe(&anim.keyframes, frame) else {
        return DocumentData::default();
    };
    if linear >= 1.0 {
        if let Some(end) = &kf.end_value {
            return end.clone();
        }
    }
    kf.start_value.clone()
}

pub fn eval_position(anim: &AnimatablePosition, frame: f32) -> PointF {
    eval_position_with_tangent(anim, frame).0
}

/// Evaluates position (spatial cubic Bezier motion path or split X/Y/Z channels)
/// and returns `(position, optional_tangent_angle_deg)` for auto-orient layers.
pub fn eval_position_with_tangent(anim: &AnimatablePosition, frame: f32) -> (PointF, Option<f32>) {
    match anim {
        AnimatablePosition::Split { x, y, z } => {
            let px = eval_float(x, frame, 0.0);
            let py = eval_float(y, frame, 0.0);
            let pz = z.as_ref().map_or(0.0, |za| eval_float(za, frame, 0.0));
            (PointF::new_3d(px, py, pz), None)
        }
        AnimatablePosition::Path(pts) => {
            let Some((kf, linear)) = find_keyframe(&pts.keyframes, frame) else {
                return (PointF::default(), None);
            };
            let Some(end) = kf.end_value else {
                return (kf.start_value, None);
            };
            if kf.hold {
                return (kf.start_value, None);
            }
            let t = eased_progress(kf, linear);
            let has_spatial = kf.path_cp1.is_some_and(|p| p.x != 0.0 || p.y != 0.0)
                || kf.path_cp2.is_some_and(|p| p.x != 0.0 || p.y != 0.0);

            if has_spatial {
                let s = kf.start_value;
                let cp1 = kf.path_cp1.unwrap_or_default();
                let cp2 = kf.path_cp2.unwrap_or_default();
                let mut path = Path::new();
                path.move_to(s.x, s.y);
                path.cubic_to(
                    s.x + cp1.x,
                    s.y + cp1.y,
                    end.x + cp2.x,
                    end.y + cp2.y,
                    end.x,
                    end.y,
                );
                let pm = PathMeasure::from_path(&path, false);
                let len = pm.length();
                if len > 0.0 {
                    if let Some((pos, tan)) = pm.get_pos_tan(t * len) {
                        let angle = tan.y.atan2(tan.x).to_degrees();
                        return (
                            PointF::new_3d(pos.x, pos.y, lerp_f32(s.z, end.z, t)),
                            Some(angle),
                        );
                    }
                }
            }
            let p = eval_point(pts, frame, PointF::default());
            let dx = end.x - kf.start_value.x;
            let dy = end.y - kf.start_value.y;
            let angle = if dx.abs() > 1e-5 || dy.abs() > 1e-5 {
                Some(dy.atan2(dx).to_degrees())
            } else {
                None
            };
            (p, angle)
        }
    }
}
