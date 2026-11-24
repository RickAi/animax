// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

use animax_model::PointF;

/// Evaluates a cubic Bezier easing curve `(0, 0) -> (cp1.x, cp1.y) -> (cp2.x, cp2.y) -> (1, 1)`
/// at horizontal progress `x`, matching `PathInterpolator::GetInterpolation`.
pub fn cubic_bezier_ease(cp1: PointF, cp2: PointF, x: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let x1 = cp1.x.clamp(0.0, 1.0);
    let x2 = cp2.x.clamp(0.0, 1.0);
    if (x1 - cp1.y).abs() < 1e-5 && (x2 - cp2.y).abs() < 1e-5 {
        return x;
    }

    // Solve B_x(t) = x using Newton-Raphson with bisection fallback
    let mut lo = 0.0f32;
    let mut hi = 1.0f32;
    let mut t = x;

    for _ in 0..8 {
        let bx = sample_bezier(x1, x2, t) - x;
        if bx.abs() < 1e-5 {
            break;
        }
        let d = sample_bezier_deriv(x1, x2, t);
        if d.abs() < 1e-6 {
            break;
        }
        let next = t - bx / d;
        if next >= lo && next <= hi {
            t = next;
        } else {
            if bx < 0.0 {
                lo = t;
            } else {
                hi = t;
            }
            t = 0.5 * (lo + hi);
        }
    }

    // Refine via bisection if still not converged
    for _ in 0..8 {
        let bx = sample_bezier(x1, x2, t) - x;
        if bx.abs() < 1e-5 {
            break;
        }
        if bx < 0.0 {
            lo = t;
        } else {
            hi = t;
        }
        t = 0.5 * (lo + hi);
    }

    sample_bezier(cp1.y, cp2.y, t)
}

#[inline]
fn sample_bezier(p1: f32, p2: f32, t: f32) -> f32 {
    let mt = 1.0 - t;
    3.0 * mt * mt * t * p1 + 3.0 * mt * t * t * p2 + t * t * t
}

#[inline]
fn sample_bezier_deriv(p1: f32, p2: f32, t: f32) -> f32 {
    let mt = 1.0 - t;
    3.0 * mt * mt * p1 + 6.0 * mt * t * (p2 - p1) + 3.0 * t * t * (1.0 - p2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bezier_endpoints_and_midpoint() {
        assert_eq!(cubic_bezier_ease(PointF::new(0.42, 0.0), PointF::new(0.58, 1.0), 0.0), 0.0);
        assert_eq!(cubic_bezier_ease(PointF::new(0.42, 0.0), PointF::new(0.58, 1.0), 1.0), 1.0);
        // Symmetric ease-in-out passes through (0.5, 0.5)
        let mid = cubic_bezier_ease(PointF::new(0.42, 0.0), PointF::new(0.58, 1.0), 0.5);
        assert!((mid - 0.5).abs() < 1e-3);
    }
}
