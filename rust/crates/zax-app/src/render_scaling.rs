//! How big the offscreen buffer the scene is drawn into has to be, from the render method and
//! the resolution scale of SPEC-09 §4.1.1 (REQ-SCN-063, AMD-032 §3).
//!
//! The application always draws the scene into its own framebuffer and blits it onto the window.
//! The two settings only decide how big that framebuffer is: with `video.render_method` = 0,
//! "Direct", it has the size of the view and `video.res_scale` has no effect at all; with 1,
//! "Render Tex", it is the view scaled by `video.res_scale_factor` (1, 2/3, 1/2, 1/3 or 1/4) and
//! the blit stretches it back, filtered bilinearly (REQ-SCN-063).

use std::time::Duration;

/// The render method that draws at the view's own resolution.
pub const DIRECT_METHOD: i32 = 0;
/// The render method the resolution scale applies to.
pub const RENDER_TEXTURE_METHOD: i32 = 1;

/// Whether the resolution scale has any effect, which is what the options window greys the row
/// out on.
#[must_use]
pub const fn scale_applies(render_method: i32) -> bool {
    render_method == RENDER_TEXTURE_METHOD
}

/// The size of the offscreen buffer for a view of `view_width` x `view_height` pixels: at least
/// one pixel in each direction, whatever the factor.
#[must_use]
pub fn scene_size(
    view_width: i32,
    view_height: i32,
    render_method: i32,
    factor: f32,
) -> (i32, i32) {
    let width = view_width.max(1);
    let height = view_height.max(1);
    if !scale_applies(render_method) || !factor.is_finite() || factor >= 1.0 || factor <= 0.0 {
        return (width, height);
    }
    // Rounding rather than truncating keeps 2/3 of 1080 at 720 instead of 719, so the factors
    // stay the exact fractions SPEC-06 names wherever the view divides evenly.
    (scaled(width, factor), scaled(height, factor))
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
// the C# `(int)MathF.Round(size * factor)` of a view size below 2^24
fn scaled(size: i32, factor: f32) -> i32 {
    ((size as f32 * factor).round() as i32).max(1)
}

/// The lowest scale of the automatic resolution (`video.dynamic_resolution`, Rust-only).
const LOWEST_SCALE: f32 = 0.5;

/// The scale it starts at: the handheld default before it existed.
const START_SCALE: f32 = 2.0 / 3.0;

/// A rise: one hundredth of the view, every few frames (+1 % every 50 ms at 60 Hz).
const RISE_STEP: f32 = 0.01;
const RISE_FRAMES: u32 = 3;

/// Within this of the scale that last fell, one rise a second only: the scale does not keep
/// climbing back into the same late frames.
const NEAR_FAILED: f32 = 0.025;
const NEAR_RISE_FRAMES: u32 = 60;

/// Frames after which the scale that fell is forgotten (10 s at 60 Hz).
const FORGET_FRAMES: u32 = 600;

/// A fall multiplies the scale by this.
const FALL_FACTOR: f32 = 0.9;

/// Frames late because of the drawing, within [`FALL_WINDOW`] frames of the first, that make
/// the scale fall: a lone hitch does not.
const FALL_MISSES: u32 = 2;
const FALL_WINDOW: u32 = 10;

/// Frames without a rise after a fall.
const PAUSE_FRAMES: u32 = 30;

/// A frame is late past this share of its budget (a missed vertical blank lasts two).
const LATE_SHARE: f64 = 1.25;

/// The CPU work of a late frame above this share of its budget is what made it late.
const WORK_SHARE: f64 = 0.9;

/// How a displayed frame went, for the automatic resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameOutcome {
    /// On time.
    OnTime,
    /// Late while the work of the CPU fit in the budget: the drawing made it late (the GPU, or
    /// the driver waiting for it), which a lower resolution relieves.
    LateDrawing,
    /// Late because the work of the CPU alone took the budget (the emulation, the scripts, the
    /// conversion): a lower resolution would not help.
    LateWork,
}

/// The outcome of a displayed frame that lasted `interval` (start to start, the swap included)
/// and whose work before the swap took `work`, for a frame `budget`.
#[must_use]
pub fn frame_outcome(interval: Duration, work: Duration, budget: Duration) -> FrameOutcome {
    let budget = budget.as_secs_f64();
    if interval.as_secs_f64() <= budget * LATE_SHARE {
        FrameOutcome::OnTime
    } else if work.as_secs_f64() > budget * WORK_SHARE {
        FrameOutcome::LateWork
    } else {
        FrameOutcome::LateDrawing
    }
}

/// The automatic resolution: a scale of the view between 50 and 100 %, which climbs by 1 %
/// every few frames on time and falls by a tenth when frames come late because of the drawing.
/// Near the scale that last fell it climbs slowly, so that it settles just under what the GPU
/// holds instead of swinging around it; frames late because of the CPU leave it alone.
#[derive(Debug, Clone)]
pub struct AutomaticScale {
    scale: f32,
    /// Frames on time since the last change.
    since_change: u32,
    /// Frames late because of the drawing in the current window, and the window's age.
    misses: u32,
    window: u32,
    /// The scale that last fell, and the frames since.
    failed: Option<(f32, u32)>,
    /// Frames left without a rise.
    pause: u32,
}

impl Default for AutomaticScale {
    fn default() -> Self {
        Self {
            scale: START_SCALE,
            since_change: 0,
            misses: 0,
            window: 0,
            failed: None,
            pause: 0,
        }
    }
}

impl AutomaticScale {
    /// The scale in force.
    #[must_use]
    pub const fn factor(&self) -> f32 {
        self.scale
    }

    /// Takes the outcome of one displayed frame. Returns whether the scale changed.
    pub fn record(&mut self, outcome: FrameOutcome) -> bool {
        if let Some((_, age)) = &mut self.failed {
            *age += 1;
            if *age >= FORGET_FRAMES {
                self.failed = None;
            }
        }
        if self.misses > 0 {
            self.window += 1;
            if self.window > FALL_WINDOW {
                self.misses = 0;
            }
        }
        match outcome {
            FrameOutcome::LateDrawing => {
                if self.misses == 0 {
                    self.window = 0;
                }
                self.misses += 1;
                if self.misses < FALL_MISSES {
                    return false;
                }
                let from = self.scale;
                self.scale = (from * FALL_FACTOR).max(LOWEST_SCALE);
                self.misses = 0;
                self.since_change = 0;
                self.pause = PAUSE_FRAMES;
                self.failed = Some((from, 0));
                return self.scale < from;
            }
            FrameOutcome::LateWork => return false,
            FrameOutcome::OnTime => {}
        }
        if self.pause > 0 {
            self.pause -= 1;
            return false;
        }
        if self.scale >= 1.0 {
            return false;
        }
        self.since_change += 1;
        let near = self
            .failed
            .is_some_and(|(failed, _)| self.scale + RISE_STEP > failed - NEAR_FAILED);
        let every = if near { NEAR_RISE_FRAMES } else { RISE_FRAMES };
        if self.since_change < every {
            return false;
        }
        self.since_change = 0;
        self.scale = (self.scale + RISE_STEP).min(1.0);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUDGET: Duration = Duration::from_micros(16_667);

    fn ms(value: u64) -> Duration {
        Duration::from_millis(value)
    }

    fn close(scale: &AutomaticScale, expected: f32) -> bool {
        (scale.factor() - expected).abs() < 1.0e-4
    }

    #[test]
    fn a_frame_is_late_by_the_drawing_only_when_the_cpu_work_fit() {
        assert_eq!(FrameOutcome::OnTime, frame_outcome(ms(17), ms(16), BUDGET));
        assert_eq!(
            FrameOutcome::LateDrawing,
            frame_outcome(ms(33), ms(9), BUDGET)
        );
        assert_eq!(
            FrameOutcome::LateWork,
            frame_outcome(ms(33), ms(16), BUDGET)
        );
    }

    #[test]
    fn the_scale_climbs_one_percent_every_three_frames_up_to_native() {
        let mut scale = AutomaticScale::default();
        assert!(close(&scale, 2.0 / 3.0));
        assert!(!scale.record(FrameOutcome::OnTime));
        assert!(!scale.record(FrameOutcome::OnTime));
        assert!(scale.record(FrameOutcome::OnTime));
        assert!(close(&scale, 2.0 / 3.0 + 0.01));
        for _ in 0..200 {
            scale.record(FrameOutcome::OnTime);
        }
        assert!(close(&scale, 1.0));
        assert!(!scale.record(FrameOutcome::OnTime));
    }

    #[test]
    fn two_frames_late_by_the_drawing_make_it_fall_a_tenth_then_pause() {
        let mut scale = AutomaticScale::default();
        // A lone hitch does nothing.
        assert!(!scale.record(FrameOutcome::LateDrawing));
        for _ in 0..FALL_WINDOW {
            scale.record(FrameOutcome::LateWork);
        }
        assert!(!scale.record(FrameOutcome::LateDrawing));
        // A second one within the window falls.
        assert!(scale.record(FrameOutcome::LateDrawing));
        assert!(close(&scale, 2.0 / 3.0 * 0.9));
        // No rise during the pause.
        for _ in 0..PAUSE_FRAMES {
            assert!(!scale.record(FrameOutcome::OnTime));
        }
        assert!(!scale.record(FrameOutcome::OnTime));
        assert!(!scale.record(FrameOutcome::OnTime));
        assert!(scale.record(FrameOutcome::OnTime));
    }

    #[test]
    fn frames_late_because_of_the_cpu_leave_the_scale_alone() {
        let mut scale = AutomaticScale::default();
        for _ in 0..1000 {
            assert!(!scale.record(FrameOutcome::LateWork));
        }
        assert!(close(&scale, 2.0 / 3.0));
    }

    #[test]
    fn near_the_scale_that_fell_it_climbs_slowly_until_it_is_forgotten() {
        let mut scale = AutomaticScale::default();
        for _ in 0..200 {
            scale.record(FrameOutcome::OnTime);
        }
        assert!(scale.record(FrameOutcome::LateDrawing) || scale.record(FrameOutcome::LateDrawing));
        assert!(close(&scale, 0.9));
        for _ in 0..PAUSE_FRAMES {
            scale.record(FrameOutcome::OnTime);
        }
        // From 0.90 to 0.97, one rise every three frames: 21 frames.
        for _ in 0..21 {
            scale.record(FrameOutcome::OnTime);
        }
        assert!(close(&scale, 0.97));
        // Within 2.5 % of 1.0, which fell: one rise a second.
        for _ in 0..NEAR_RISE_FRAMES - 1 {
            assert!(!scale.record(FrameOutcome::OnTime));
        }
        assert!(scale.record(FrameOutcome::OnTime));
        assert!(close(&scale, 0.98));
    }

    #[test]
    fn the_scale_never_falls_under_one_half() {
        let mut scale = AutomaticScale::default();
        for _ in 0..100 {
            scale.record(FrameOutcome::LateDrawing);
        }
        assert!(close(&scale, 0.5));
    }
}
