// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

use crate::evaluator::{eval_float, eval_i32, eval_position, eval_position_with_tangent, eval_scale};
use animax_model::{AnimatableTransform, PointF, ScaleF};
use animax_skity::Matrix;

#[derive(Debug, Clone, Copy)]
pub struct EvaluatedTransform {
    pub matrix: Matrix,
    /// Opacity in `[0, 100]`.
    pub opacity: i32,
}

/// Evaluates an `AnimatableTransform` at `frame`, matching `TransformKeyframeAnimation::GetMatrix`.
pub fn eval_transform(tr: &AnimatableTransform, frame: f32) -> EvaluatedTransform {
    let mut matrix = Matrix::identity();

    let (pos, auto_angle) = tr
        .position
        .as_ref()
        .map_or((PointF::default(), None), |p| eval_position_with_tangent(p, frame));
    if pos.x != 0.0 || pos.y != 0.0 || pos.z != 0.0 {
        matrix.pre_translate_3d(pos.x, pos.y, pos.z);
    }

    let rx = tr.rotation_x.as_ref().map_or(0.0, |r| eval_float(r, frame, 0.0));
    let ry = tr.rotation_y.as_ref().map_or(0.0, |r| eval_float(r, frame, 0.0));
    let rz = tr
        .rotation_z
        .as_ref()
        .or(tr.rotation.as_ref())
        .map_or(0.0, |r| eval_float(r, frame, 0.0))
        + if tr.auto_orient {
            auto_angle.unwrap_or(0.0)
        } else {
            0.0
        };

    if rx != 0.0 || ry != 0.0 {
        matrix.pre_rotate_xyz(rx, ry, rz);
    } else if rz != 0.0 {
        matrix.pre_rotate(rz);
    }

    let skew = tr.skew.as_ref().map_or(0.0, |s| eval_float(s, frame, 0.0));
    if skew != 0.0 {
        let skew_angle = tr
            .skew_angle
            .as_ref()
            .map_or(0.0, |sa| eval_float(sa, frame, 0.0));
        let (m_sin, m_cos) = (-skew_angle).to_radians().sin_cos();
        let tan_skew = skew.to_radians().tan();

        let rot1 = Matrix::from_9(&[m_cos, m_sin, 0.0, -m_sin, m_cos, 0.0, 0.0, 0.0, 1.0]);
        let sk = Matrix::from_9(&[1.0, tan_skew, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);
        let rot2 = Matrix::from_9(&[m_cos, -m_sin, 0.0, m_sin, m_cos, 0.0, 0.0, 0.0, 1.0]);
        matrix.pre_concat(&rot2);
        matrix.pre_concat(&sk);
        matrix.pre_concat(&rot1);
    }

    let scale = tr
        .scale
        .as_ref()
        .map_or(ScaleF::default(), |s| eval_scale(s, frame));
    if scale.x != 1.0 || scale.y != 1.0 || scale.z != 1.0 {
        matrix.pre_scale_3d(scale.x, scale.y, scale.z);
    }

    let anchor = tr
        .anchor_point
        .as_ref()
        .map_or(PointF::default(), |a| eval_position(a, frame));
    if anchor.x != 0.0 || anchor.y != 0.0 || anchor.z != 0.0 {
        matrix.pre_translate_3d(-anchor.x, -anchor.y, -anchor.z);
    }

    let opacity = tr
        .opacity
        .as_ref()
        .map_or(100, |o| eval_i32(o, frame, 100))
        .clamp(0, 100);

    EvaluatedTransform { matrix, opacity }
}

/// Evaluates the N-th repeater copy transform matrix matching `TransformKeyframeAnimation::GetMatrixForRepeater`.
pub fn eval_repeater_transform(tr: &AnimatableTransform, frame: f32, amount: f32) -> Matrix {
    let mut matrix = Matrix::identity();

    let pos = tr
        .position
        .as_ref()
        .map_or(PointF::default(), |p| eval_position(p, frame));
    matrix.pre_translate(pos.x * amount, pos.y * amount);

    let anchor = tr
        .anchor_point
        .as_ref()
        .map_or(PointF::default(), |a| eval_position(a, frame));
    let scale = tr
        .scale
        .as_ref()
        .map_or(ScaleF::default(), |s| eval_scale(s, frame));
    matrix.pre_scale(scale.x.powf(amount), scale.y.powf(amount));

    let rot = tr.rotation.as_ref().map_or(0.0, |r| eval_float(r, frame, 0.0));
    if rot != 0.0 {
        matrix.pre_rotate_at(rot * amount, anchor.x, anchor.y);
    }

    matrix
}
