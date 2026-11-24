// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

use crate::path::{Path, PathVerb};
use animax_model::PointF;

#[derive(Debug, Clone, Copy)]
enum Segment {
    Line([f32; 2], [f32; 2]),
    Quad([f32; 2], [f32; 2], [f32; 2]),
    Cubic([f32; 2], [f32; 2], [f32; 2], [f32; 2]),
}

#[derive(Debug, Clone)]
struct MeasuredSegment {
    seg: Segment,
    /// Cumulative arc-length samples at uniform parameter t = i / SUBDIVISIONS.
    lut: [f32; SUBDIVISIONS + 1],
    start_dist: f32,
    end_dist: f32,
}

const SUBDIVISIONS: usize = 16;

#[derive(Debug, Clone)]
struct Contour {
    segments: Vec<MeasuredSegment>,
    total_length: f32,
    closed: bool,
}

/// Pure Rust arc-length path measurer & sub-path extractor matching `skity::PathMeasure`.
#[derive(Debug, Clone, Default)]
pub struct PathMeasure {
    contours: Vec<Contour>,
    current: usize,
}

impl PathMeasure {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_path(path: &Path, force_closed: bool) -> Self {
        let mut pm = Self::new();
        pm.set_path(path, force_closed);
        pm
    }

    pub fn set_path(&mut self, path: &Path, force_closed: bool) {
        self.contours.clear();
        self.current = 0;

        let mut cur_segments: Vec<MeasuredSegment> = Vec::new();
        let mut start = [0.0f32; 2];
        let mut cur = [0.0f32; 2];
        let mut has_move = false;
        let mut dist = 0.0f32;

        let flush_contour = |contours: &mut Vec<Contour>,
                             segs: &mut Vec<MeasuredSegment>,
                             dist: &mut f32,
                             closed: bool,
                             start_pt: [f32; 2],
                             cur_pt: [f32; 2]| {
            if segs.is_empty() {
                return;
            }
            if (closed || force_closed) && dist_pts(cur_pt, start_pt) > 1e-5 {
                let ms = measure_segment(Segment::Line(cur_pt, start_pt), *dist);
                *dist = ms.end_dist;
                segs.push(ms);
            }
            contours.push(Contour {
                segments: std::mem::take(segs),
                total_length: *dist,
                closed: closed || force_closed,
            });
            *dist = 0.0;
        };

        for &verb in path.verbs() {
            match verb {
                PathVerb::MoveTo(x, y) => {
                    flush_contour(&mut self.contours, &mut cur_segments, &mut dist, false, start, cur);
                    start = [x, y];
                    cur = [x, y];
                    has_move = true;
                }
                PathVerb::LineTo(x, y) => {
                    if !has_move {
                        start = [0.0, 0.0];
                        cur = [0.0, 0.0];
                        has_move = true;
                    }
                    let next = [x, y];
                    if dist_pts(cur, next) > 1e-6 {
                        let ms = measure_segment(Segment::Line(cur, next), dist);
                        dist = ms.end_dist;
                        cur_segments.push(ms);
                    }
                    cur = next;
                }
                PathVerb::QuadTo(cx, cy, x, y) => {
                    if !has_move {
                        start = [0.0, 0.0];
                        cur = [0.0, 0.0];
                        has_move = true;
                    }
                    let next = [x, y];
                    let ms = measure_segment(Segment::Quad(cur, [cx, cy], next), dist);
                    dist = ms.end_dist;
                    cur_segments.push(ms);
                    cur = next;
                }
                PathVerb::CubicTo(x1, y1, x2, y2, x, y) => {
                    if !has_move {
                        start = [0.0, 0.0];
                        cur = [0.0, 0.0];
                        has_move = true;
                    }
                    let next = [x, y];
                    let ms = measure_segment(Segment::Cubic(cur, [x1, y1], [x2, y2], next), dist);
                    dist = ms.end_dist;
                    cur_segments.push(ms);
                    cur = next;
                }
                PathVerb::Close => {
                    flush_contour(&mut self.contours, &mut cur_segments, &mut dist, true, start, cur);
                    cur = start;
                    has_move = false;
                }
            }
        }
        flush_contour(&mut self.contours, &mut cur_segments, &mut dist, false, start, cur);
    }

    pub fn length(&self) -> f32 {
        self.contours
            .get(self.current)
            .map_or(0.0, |c| c.total_length)
    }

    pub fn is_closed(&self) -> bool {
        self.contours.get(self.current).is_some_and(|c| c.closed)
    }

    pub fn next_contour(&mut self) -> bool {
        if self.current + 1 < self.contours.len() {
            self.current += 1;
            true
        } else {
            false
        }
    }

    pub fn get_pos_tan(&self, distance: f32) -> Option<(PointF, PointF)> {
        let contour = self.contours.get(self.current)?;
        if contour.segments.is_empty() || contour.total_length <= 0.0 {
            return None;
        }
        let d = distance.clamp(0.0, contour.total_length);
        let idx = find_segment(&contour.segments, d);
        let ms = &contour.segments[idx];
        let t = distance_to_t(ms, d);
        let pos = eval_segment(&ms.seg, t);
        let tan = eval_tangent(&ms.seg, t);
        let len = tan[0].hypot(tan[1]);
        let unit_tan = if len > 1e-6 {
            [tan[0] / len, tan[1] / len]
        } else {
            [1.0, 0.0]
        };
        Some((
            PointF::new(pos[0], pos[1]),
            PointF::new(unit_tan[0], unit_tan[1]),
        ))
    }

    /// Extracts sub-path between `start_d` and `stop_d` into `dst` using exact de Casteljau splitting.
    pub fn get_segment(
        &self,
        start_d: f32,
        stop_d: f32,
        dst: &mut Path,
        start_with_move_to: bool,
    ) -> bool {
        let Some(contour) = self.contours.get(self.current) else {
            return false;
        };
        if contour.segments.is_empty() || contour.total_length <= 0.0 {
            return false;
        }
        let s = start_d.max(0.0);
        let e = stop_d.min(contour.total_length);
        if s >= e {
            return false;
        }

        let start_idx = find_segment(&contour.segments, s);
        let stop_idx = find_segment(&contour.segments, e);
        let mut need_move = start_with_move_to;

        for idx in start_idx..=stop_idx {
            let ms = &contour.segments[idx];
            let t0 = if idx == start_idx {
                distance_to_t(ms, s)
            } else {
                0.0
            };
            let t1 = if idx == stop_idx {
                distance_to_t(ms, e)
            } else {
                1.0
            };
            if t1 <= t0 + 1e-6 {
                continue;
            }
            let sub = sub_segment(&ms.seg, t0, t1);
            emit_segment(&sub, dst, need_move);
            need_move = false;
        }
        !need_move || !start_with_move_to
    }
}

#[inline]
fn dist_pts(a: [f32; 2], b: [f32; 2]) -> f32 {
    (b[0] - a[0]).hypot(b[1] - a[1])
}

#[inline]
fn lerp2(a: [f32; 2], b: [f32; 2], t: f32) -> [f32; 2] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}

fn eval_segment(seg: &Segment, t: f32) -> [f32; 2] {
    match *seg {
        Segment::Line(p0, p1) => lerp2(p0, p1, t),
        Segment::Quad(p0, p1, p2) => {
            let a = lerp2(p0, p1, t);
            let b = lerp2(p1, p2, t);
            lerp2(a, b, t)
        }
        Segment::Cubic(p0, p1, p2, p3) => {
            let a = lerp2(p0, p1, t);
            let b = lerp2(p1, p2, t);
            let c = lerp2(p2, p3, t);
            let d = lerp2(a, b, t);
            let e = lerp2(b, c, t);
            lerp2(d, e, t)
        }
    }
}

fn eval_tangent(seg: &Segment, t: f32) -> [f32; 2] {
    match *seg {
        Segment::Line(p0, p1) => [p1[0] - p0[0], p1[1] - p0[1]],
        Segment::Quad(p0, p1, p2) => {
            let mt = 1.0 - t;
            [
                2.0 * mt * (p1[0] - p0[0]) + 2.0 * t * (p2[0] - p1[0]),
                2.0 * mt * (p1[1] - p0[1]) + 2.0 * t * (p2[1] - p1[1]),
            ]
        }
        Segment::Cubic(p0, p1, p2, p3) => {
            let mt = 1.0 - t;
            let dx = 3.0 * mt * mt * (p1[0] - p0[0])
                + 6.0 * mt * t * (p2[0] - p1[0])
                + 3.0 * t * t * (p3[0] - p2[0]);
            let dy = 3.0 * mt * mt * (p1[1] - p0[1])
                + 6.0 * mt * t * (p2[1] - p1[1])
                + 3.0 * t * t * (p3[1] - p2[1]);
            if dx.hypot(dy) < 1e-6 {
                [p3[0] - p0[0], p3[1] - p0[1]]
            } else {
                [dx, dy]
            }
        }
    }
}

fn measure_segment(seg: Segment, start_dist: f32) -> MeasuredSegment {
    let mut lut = [0.0f32; SUBDIVISIONS + 1];
    let mut prev = eval_segment(&seg, 0.0);
    let mut acc = 0.0f32;
    for i in 1..=SUBDIVISIONS {
        let t = (i as f32) / (SUBDIVISIONS as f32);
        let p = eval_segment(&seg, t);
        acc += dist_pts(prev, p);
        lut[i] = acc;
        prev = p;
    }
    MeasuredSegment {
        seg,
        lut,
        start_dist,
        end_dist: start_dist + acc,
    }
}

fn find_segment(segments: &[MeasuredSegment], d: f32) -> usize {
    let idx = segments.partition_point(|s| s.end_dist < d);
    idx.min(segments.len() - 1)
}

fn distance_to_t(ms: &MeasuredSegment, d: f32) -> f32 {
    let local = (d - ms.start_dist).clamp(0.0, ms.end_dist - ms.start_dist);
    if local <= 0.0 {
        return 0.0;
    }
    let total = ms.lut[SUBDIVISIONS];
    if total <= 0.0 || local >= total {
        return 1.0;
    }
    let idx = ms.lut.partition_point(|&v| v < local).clamp(1, SUBDIVISIONS);
    let d0 = ms.lut[idx - 1];
    let d1 = ms.lut[idx];
    let frac = if d1 > d0 { (local - d0) / (d1 - d0) } else { 0.0 };
    (((idx - 1) as f32) + frac) / (SUBDIVISIONS as f32)
}

fn split_cubic(
    p0: [f32; 2],
    p1: [f32; 2],
    p2: [f32; 2],
    p3: [f32; 2],
    t: f32,
) -> (([f32; 2], [f32; 2], [f32; 2], [f32; 2]), ([f32; 2], [f32; 2], [f32; 2], [f32; 2])) {
    let a = lerp2(p0, p1, t);
    let b = lerp2(p1, p2, t);
    let c = lerp2(p2, p3, t);
    let d = lerp2(a, b, t);
    let e = lerp2(b, c, t);
    let f = lerp2(d, e, t);
    ((p0, a, d, f), (f, e, c, p3))
}

fn split_quad(
    p0: [f32; 2],
    p1: [f32; 2],
    p2: [f32; 2],
    t: f32,
) -> (([f32; 2], [f32; 2], [f32; 2]), ([f32; 2], [f32; 2], [f32; 2])) {
    let a = lerp2(p0, p1, t);
    let b = lerp2(p1, p2, t);
    let c = lerp2(a, b, t);
    ((p0, a, c), (c, b, p2))
}

fn sub_segment(seg: &Segment, t0: f32, t1: f32) -> Segment {
    match *seg {
        Segment::Line(p0, p1) => Segment::Line(lerp2(p0, p1, t0), lerp2(p0, p1, t1)),
        Segment::Quad(p0, p1, p2) => {
            let (_, right) = split_quad(p0, p1, p2, t0);
            let nt = if t0 < 1.0 { (t1 - t0) / (1.0 - t0) } else { 0.0 };
            let (mid, _) = split_quad(right.0, right.1, right.2, nt);
            Segment::Quad(mid.0, mid.1, mid.2)
        }
        Segment::Cubic(p0, p1, p2, p3) => {
            let (_, right) = split_cubic(p0, p1, p2, p3, t0);
            let nt = if t0 < 1.0 { (t1 - t0) / (1.0 - t0) } else { 0.0 };
            let (mid, _) = split_cubic(right.0, right.1, right.2, right.3, nt);
            Segment::Cubic(mid.0, mid.1, mid.2, mid.3)
        }
    }
}

fn emit_segment(seg: &Segment, dst: &mut Path, start_with_move: bool) {
    match *seg {
        Segment::Line(p0, p1) => {
            if start_with_move {
                dst.move_to(p0[0], p0[1]);
            }
            dst.line_to(p1[0], p1[1]);
        }
        Segment::Quad(p0, p1, p2) => {
            if start_with_move {
                dst.move_to(p0[0], p0[1]);
            }
            dst.quad_to(p1[0], p1[1], p2[0], p2[1]);
        }
        Segment::Cubic(p0, p1, p2, p3) => {
            if start_with_move {
                dst.move_to(p0[0], p0[1]);
            }
            dst.cubic_to(p1[0], p1[1], p2[0], p2[1], p3[0], p3[1]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measure_line_and_extract_segment() {
        let mut p = Path::new();
        p.move_to(0.0, 0.0);
        p.line_to(100.0, 0.0);
        p.line_to(100.0, 100.0);
        let pm = PathMeasure::from_path(&p, false);
        assert!((pm.length() - 200.0).abs() < 1e-3);

        let (pos, tan) = pm.get_pos_tan(50.0).unwrap();
        assert!((pos.x - 50.0).abs() < 1e-3);
        assert!(pos.y.abs() < 1e-3);
        assert!((tan.x - 1.0).abs() < 1e-3);

        let mut seg = Path::new();
        assert!(pm.get_segment(50.0, 150.0, &mut seg, true));
        let b = seg.bounds();
        assert!((b.left - 50.0).abs() < 1e-2);
        assert!((b.right - 100.0).abs() < 1e-2);
        assert!((b.bottom - 50.0).abs() < 1e-2);
    }
}
