// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

/// Playback events emitted by a single `ValueAnimator::on_frame` tick.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AnimatorTickResult {
    pub rendered: bool,
    pub started: bool,
    pub repeated: Option<i32>,
    pub completed: bool,
    pub current_frame: f32,
    pub progress: f32,
}

/// Pure Rust animation clock & playback state machine matching `lynx::animax::ValueAnimator`.
#[derive(Debug, Clone)]
pub struct ValueAnimator {
    frame_rate: f64,
    start_frame: f64,
    end_frame: f64,
    min_frame: f64,
    max_frame: f64,
    current_frame: f64,
    speed: f64,
    loop_count: i32, // <= 0 means loop forever if loop_enabled, or 1 if !loop_enabled
    loop_forever: bool,
    current_loop: i32,
    auto_reverse: bool,
    reversed_direction: bool,
    animating: bool,
    paused: bool,
    has_emitted_start: bool,
    last_time_ns: Option<i64>,
    max_frame_rate: f64,
    accumulator_ns: f64,
}

impl Default for ValueAnimator {
    fn default() -> Self {
        Self {
            frame_rate: 60.0,
            start_frame: 0.0,
            end_frame: 0.0,
            min_frame: 0.0,
            max_frame: 0.0,
            current_frame: 0.0,
            speed: 1.0,
            loop_count: 1,
            loop_forever: false,
            current_loop: 0,
            auto_reverse: false,
            reversed_direction: false,
            animating: false,
            paused: false,
            has_emitted_start: false,
            last_time_ns: None,
            max_frame_rate: 0.0,
            accumulator_ns: 0.0,
        }
    }
}

impl ValueAnimator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_composition_range(&mut self, start_frame: f32, end_frame: f32, frame_rate: f32) {
        self.start_frame = start_frame as f64;
        self.end_frame = end_frame as f64;
        self.min_frame = self.start_frame;
        self.max_frame = self.end_frame;
        self.frame_rate = if frame_rate > 0.0 { frame_rate as f64 } else { 60.0 };
        self.current_frame = self.min_frame;
    }

    pub fn set_start_frame(&mut self, frame: f32) {
        self.min_frame = (frame as f64).clamp(self.start_frame, self.end_frame);
        if self.current_frame < self.min_frame {
            self.current_frame = self.min_frame;
        }
    }

    pub fn set_end_frame(&mut self, frame: f32) {
        self.max_frame = (frame as f64).clamp(self.start_frame, self.end_frame);
        if self.current_frame > self.max_frame {
            self.current_frame = self.max_frame;
        }
    }

    pub fn set_loop(&mut self, loop_forever: bool) {
        self.loop_forever = loop_forever;
    }

    pub fn set_loop_count(&mut self, count: i32) {
        if count <= 0 {
            self.loop_forever = true;
            self.loop_count = -1;
        } else {
            self.loop_forever = false;
            self.loop_count = count;
        }
    }

    pub fn set_auto_reverse(&mut self, auto_reverse: bool) {
        self.auto_reverse = auto_reverse;
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed as f64;
    }

    pub fn set_max_frame_rate(&mut self, max_fps: f32) {
        self.max_frame_rate = (max_fps as f64).max(0.0);
        self.accumulator_ns = 0.0;
    }

    pub fn is_animating(&self) -> bool {
        self.animating && !self.paused
    }

    pub fn current_frame(&self) -> f32 {
        self.current_frame as f32
    }

    pub fn progress(&self) -> f32 {
        let span = self.end_frame - self.start_frame;
        if span <= 0.0 {
            0.0
        } else {
            ((self.current_frame - self.start_frame) / span).clamp(0.0, 1.0) as f32
        }
    }

    pub fn set_progress(&mut self, progress: f32) {
        let p = (progress as f64).clamp(0.0, 1.0);
        self.current_frame = self.start_frame + p * (self.end_frame - self.start_frame);
    }

    pub fn seek(&mut self, frame: f32) {
        self.current_frame = (frame as f64).clamp(self.min_frame, self.max_frame);
    }

    pub fn play(&mut self) {
        self.animating = true;
        self.paused = false;
        self.current_loop = 0;
        self.has_emitted_start = false;
        self.last_time_ns = None;
        self.reversed_direction = false;
        self.current_frame = if self.speed >= 0.0 {
            self.min_frame
        } else {
            self.max_frame
        };
    }

    pub fn play_segment(&mut self, start_frame: f32, end_frame: f32) {
        let s = (start_frame as f64).clamp(self.start_frame, self.end_frame);
        let e = (end_frame as f64).clamp(self.start_frame, self.end_frame);
        self.min_frame = s.min(e);
        self.max_frame = s.max(e);
        self.play();
    }

    pub fn pause(&mut self) {
        self.paused = true;
        self.last_time_ns = None;
    }

    pub fn resume(&mut self) {
        if self.animating || self.paused {
            self.animating = true;
            self.paused = false;
            self.last_time_ns = None;
        }
    }

    pub fn stop(&mut self) -> bool {
        let was_animating = self.animating;
        self.animating = false;
        self.paused = false;
        self.last_time_ns = None;
        self.current_loop = 0;
        self.has_emitted_start = false;
        self.reversed_direction = false;
        self.current_frame = self.min_frame;
        was_animating
    }

    /// Advances playback to `timestamp_ns` (monotonic nanoseconds).
    pub fn on_frame(&mut self, timestamp_ns: i64) -> AnimatorTickResult {
        let mut res = AnimatorTickResult::default();
        if !self.is_animating() {
            return res;
        }

        let effective_speed = if self.reversed_direction {
            -self.speed
        } else {
            self.speed
        };

        match self.last_time_ns {
            None => {
                self.last_time_ns = Some(timestamp_ns);
                if !self.has_emitted_start {
                    self.has_emitted_start = true;
                    res.started = true;
                }
            }
            Some(prev_ns) => {
                let dt_ns = (timestamp_ns - prev_ns).max(0) as f64;
                if self.max_frame_rate > 0.0 {
                    let min_interval = 1e9 / self.max_frame_rate;
                    self.accumulator_ns += dt_ns;
                    if self.accumulator_ns + 1.0 < min_interval {
                        return res;
                    }
                    self.accumulator_ns %= min_interval;
                }
                self.last_time_ns = Some(timestamp_ns);
                let dt_ms = dt_ns / 1e6;
                self.current_frame += effective_speed * dt_ms * self.frame_rate / 1000.0;
            }
        }

        let span = self.max_frame - self.min_frame;
        if span > 0.0 {
            if self.current_frame > self.max_frame {
                if !self.loop_forever && self.current_loop >= self.loop_count - 1 {
                    self.current_frame = self.max_frame;
                    res.completed = true;
                    self.animating = false;
                } else {
                    self.current_loop += 1;
                    res.repeated = Some(self.current_loop);
                    if self.auto_reverse {
                        self.reversed_direction = !self.reversed_direction;
                        let overshoot = (self.current_frame - self.max_frame) % span;
                        self.current_frame = self.max_frame - overshoot;
                    } else {
                        self.current_frame =
                            self.min_frame + ((self.current_frame - self.min_frame) % span);
                    }
                }
            } else if self.current_frame < self.min_frame {
                if !self.loop_forever && self.current_loop >= self.loop_count - 1 {
                    self.current_frame = self.min_frame;
                    res.completed = true;
                    self.animating = false;
                } else {
                    self.current_loop += 1;
                    res.repeated = Some(self.current_loop);
                    if self.auto_reverse {
                        self.reversed_direction = !self.reversed_direction;
                        let overshoot = (self.min_frame - self.current_frame) % span;
                        self.current_frame = self.min_frame + overshoot;
                    } else {
                        let rem = (self.min_frame - self.current_frame) % span;
                        self.current_frame = self.max_frame - rem;
                    }
                }
            }
        }

        res.rendered = true;
        res.current_frame = self.current_frame as f32;
        res.progress = self.progress();
        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn animator_plays_loops_and_completes() {
        let mut anim = ValueAnimator::new();
        anim.set_composition_range(0.0, 60.0, 60.0);
        anim.set_loop_count(2);
        anim.play();

        let t0 = anim.on_frame(0);
        assert!(t0.started);
        assert!(t0.rendered);

        // 0.5s -> frame 30
        let t1 = anim.on_frame(500_000_000);
        assert!((t1.current_frame - 30.0).abs() < 1e-2);
        assert!(!t1.completed);

        // 1.1s -> loops once to frame 6
        let t2 = anim.on_frame(1_100_000_000);
        assert_eq!(t2.repeated, Some(1));
        assert!((t2.current_frame - 6.0).abs() < 1e-2);

        // 2.2s -> completes on loop 2
        let t3 = anim.on_frame(2_200_000_000);
        assert!(t3.completed);
        assert!((t3.current_frame - 60.0).abs() < 1e-2);
        assert!(!anim.is_animating());
    }
}
