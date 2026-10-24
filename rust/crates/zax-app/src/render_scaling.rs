//! How big the offscreen buffer the scene is drawn into has to be, from the render method and
//! the resolution scale of SPEC-09 §4.1.1 (REQ-SCN-063, AMD-032 §3).
//!
//! The application always draws the scene into its own framebuffer and blits it onto the window.
//! The two settings only decide how big that framebuffer is: with `video.render_method` = 0,
//! "Direct", it has the size of the view and `video.res_scale` has no effect at all; with 1,
//! "Render Tex", it is the view scaled by `video.res_scale_factor` (1, 2/3, 1/2, 1/3 or 1/4) and
//! the blit stretches it back, filtered bilinearly (REQ-SCN-063).

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

/// The scales of the automatic resolution (`video.dynamic_resolution`, Rust-only), native first.
const DYNAMIC_SCALES: [f32; 4] = [1.0, 5.0 / 6.0, 2.0 / 3.0, 0.5];

/// The scale the automatic resolution starts at: the handheld default before it existed.
const DYNAMIC_START: usize = 2;

/// Share of the frame budget above which the GPU is late.
const BUSY_SHARE: f64 = 0.85;

/// Late measures in a row that lower the scale.
const BUSY_MEASURES: u32 = 3;

/// Share of the frame budget the next scale up has to fit in, estimated from its pixel count.
const ROOM_SHARE: f64 = 0.7;

/// Measures in a row with that room that raise the scale (about a second at 60 Hz).
const ROOM_MEASURES: u32 = 60;

/// Measures in a row without a late one, at a reduced scale, before the next scale up is tried
/// whatever the estimate says (about 5 s at 60 Hz). A handheld's GPU lowers its clock when the
/// load is light, so the time it measures overstates the cost of the next scale: the estimate
/// alone would never let the scale rise.
const PROBE_MEASURES: u32 = 300;

/// The longest wait before a probe, which doubles after each probe that did not hold (about
/// 80 s at 60 Hz).
const PROBE_MEASURES_MAX: u32 = 4800;

/// Measures after a rise during which the GPU may bring its clock up: the scale then falls
/// back only after [`SETTLE_BUSY_MEASURES`] late measures in a row.
const SETTLE_MEASURES: u32 = 90;

/// Late measures in a row that lower the scale while it settles after a rise.
const SETTLE_BUSY_MEASURES: u32 = 10;

/// The automatic resolution: the render scale follows the GPU time of the scene passes, one
/// step at a time, with a margin between lowering and raising so that it does not flicker. It
/// rises when the next scale fits by estimate for a second, or, every few seconds without a
/// late frame, by trying it: a try that does not hold falls back and the next waits twice as
/// long.
#[derive(Debug, Clone)]
pub struct DynamicResolution {
    step: usize,
    busy: u32,
    room: u32,
    /// Measures in a row without a late one.
    calm: u32,
    /// The calm measures the next probe waits for.
    probe_wait: u32,
    /// Measures since the last rise, while it settles.
    settling: Option<u32>,
    /// Whether the last rise was a probe.
    probed: bool,
}

impl Default for DynamicResolution {
    fn default() -> Self {
        Self {
            step: DYNAMIC_START,
            busy: 0,
            room: 0,
            calm: 0,
            probe_wait: PROBE_MEASURES,
            settling: None,
            probed: false,
        }
    }
}

impl DynamicResolution {
    /// The scale in force.
    #[must_use]
    pub const fn factor(&self) -> f32 {
        DYNAMIC_SCALES[self.step]
    }

    /// Takes one measure: the GPU time of a frame's scene passes, for a frame `budget`. Returns
    /// whether the scale changed.
    pub fn record(&mut self, gpu: std::time::Duration, budget: std::time::Duration) -> bool {
        let share = gpu.as_secs_f64() / budget.as_secs_f64().max(f64::EPSILON);
        let settled = self.settle();
        if share > BUSY_SHARE {
            self.room = 0;
            self.calm = 0;
            self.busy += 1;
            let needed = if settled {
                BUSY_MEASURES
            } else {
                SETTLE_BUSY_MEASURES
            };
            if self.busy >= needed && self.step + 1 < DYNAMIC_SCALES.len() {
                if !settled && self.probed {
                    // The try did not hold: the next one waits longer.
                    self.probe_wait = (self.probe_wait * 2).min(PROBE_MEASURES_MAX);
                }
                self.step += 1;
                self.busy = 0;
                self.settling = None;
                self.probed = false;
                return true;
            }
            return false;
        }
        self.busy = 0;
        self.calm += 1;
        let Some(up) = self.step.checked_sub(1) else {
            return false;
        };
        let ratio = f64::from(DYNAMIC_SCALES[up] / DYNAMIC_SCALES[self.step]);
        if share * ratio * ratio < ROOM_SHARE {
            self.room += 1;
            if self.room >= ROOM_MEASURES {
                self.rise(up, false);
                return true;
            }
        } else {
            self.room = 0;
        }
        if self.calm >= self.probe_wait {
            self.rise(up, true);
            return true;
        }
        false
    }

    /// Counts one measure of the settling after a rise; true once it is over (or when there
    /// was none). A probe that held through it resets the wait of the next one.
    fn settle(&mut self) -> bool {
        let Some(measures) = self.settling else {
            return true;
        };
        if measures + 1 < SETTLE_MEASURES {
            self.settling = Some(measures + 1);
            return false;
        }
        self.settling = None;
        if std::mem::take(&mut self.probed) {
            self.probe_wait = PROBE_MEASURES;
        }
        true
    }

    fn rise(&mut self, up: usize, probed: bool) {
        self.step = up;
        self.room = 0;
        self.calm = 0;
        self.busy = 0;
        self.settling = Some(0);
        self.probed = probed;
    }
}

/// The longest GPU time of a frame's scene passes that is taken as a measure: a timer that
/// returns more (a driver that answers with the largest value of the result, as some do) or
/// nothing at all is not measuring.
pub const PLAUSIBLE_GPU_TIME: std::time::Duration = std::time::Duration::from_secs(1);

/// Whether `gpu` can be a GPU time of a frame's scene passes.
#[must_use]
pub fn plausible_gpu_time(gpu: std::time::Duration) -> bool {
    !gpu.is_zero() && gpu < PLAUSIBLE_GPU_TIME
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    const BUDGET: Duration = Duration::from_micros(16_667);

    fn ms(value: u64) -> Duration {
        Duration::from_millis(value)
    }

    #[test]
    fn the_scale_drops_after_three_late_frames_and_stops_at_one_half() {
        let mut scale = DynamicResolution::default();
        assert!((scale.factor() - 2.0 / 3.0).abs() < 1.0e-6);
        assert!(!scale.record(ms(16), BUDGET));
        assert!(!scale.record(ms(16), BUDGET));
        assert!(scale.record(ms(16), BUDGET));
        assert!((scale.factor() - 0.5).abs() < 1.0e-6);
        for _ in 0..10 {
            scale.record(ms(30), BUDGET);
        }
        assert!((scale.factor() - 0.5).abs() < 1.0e-6);
    }

    #[test]
    fn the_scale_rises_when_the_next_step_fits_for_a_second() {
        let mut scale = DynamicResolution::default();
        // 2/3 -> 5/6 costs (5/4)^2 = 1.5625 times more: 7 ms becomes 10.9 ms, under 70 %.
        for _ in 0..ROOM_MEASURES - 1 {
            assert!(!scale.record(ms(7), BUDGET));
        }
        assert!(scale.record(ms(7), BUDGET));
        assert!((scale.factor() - 5.0 / 6.0).abs() < 1.0e-6);
        // 9 ms at 5/6 would be 13 ms at native: over 70 % of the budget, it stays.
        for _ in 0..ROOM_MEASURES * 2 {
            assert!(!scale.record(ms(9), BUDGET));
        }
        assert!((scale.factor() - 5.0 / 6.0).abs() < 1.0e-6);
    }

    #[test]
    fn without_room_by_estimate_the_next_scale_is_tried_after_five_calm_seconds() {
        let mut scale = DynamicResolution::default();
        // 10 ms at 2/3 would be 15.6 ms at 5/6: no room by estimate, but no late frame either.
        for _ in 0..PROBE_MEASURES - 1 {
            assert!(!scale.record(ms(10), BUDGET));
        }
        assert!(scale.record(ms(10), BUDGET));
        assert!((scale.factor() - 5.0 / 6.0).abs() < 1.0e-6);
    }

    #[test]
    fn a_try_that_does_not_hold_falls_back_and_the_next_waits_twice_as_long() {
        let mut scale = DynamicResolution::default();
        for _ in 0..PROBE_MEASURES {
            scale.record(ms(10), BUDGET);
        }
        assert!((scale.factor() - 5.0 / 6.0).abs() < 1.0e-6);
        // While it settles, three late frames are not enough to fall back...
        for _ in 0..BUSY_MEASURES {
            assert!(!scale.record(ms(16), BUDGET));
        }
        // ...ten are.
        for _ in BUSY_MEASURES..SETTLE_BUSY_MEASURES - 1 {
            assert!(!scale.record(ms(16), BUDGET));
        }
        assert!(scale.record(ms(16), BUDGET));
        assert!((scale.factor() - 2.0 / 3.0).abs() < 1.0e-6);
        // The next try waits twice as long.
        for _ in 0..PROBE_MEASURES * 2 - 1 {
            assert!(!scale.record(ms(10), BUDGET));
        }
        assert!(scale.record(ms(10), BUDGET));
        assert!((scale.factor() - 5.0 / 6.0).abs() < 1.0e-6);
    }

    #[test]
    fn a_try_that_holds_reaches_native_and_resets_the_wait() {
        let mut scale = DynamicResolution::default();
        for _ in 0..PROBE_MEASURES {
            scale.record(ms(10), BUDGET);
        }
        // It holds through the settling, then the next try comes after the base wait.
        for _ in 0..PROBE_MEASURES - 1 {
            assert!(!scale.record(ms(11), BUDGET));
        }
        assert!(scale.record(ms(11), BUDGET));
        assert!((scale.factor() - 1.0).abs() < 1.0e-6);
        // At native there is nothing above.
        for _ in 0..PROBE_MEASURES * 2 {
            assert!(!scale.record(ms(11), BUDGET));
        }
    }

    #[test]
    fn only_a_plausible_gpu_time_is_a_measure() {
        assert!(plausible_gpu_time(ms(12)));
        assert!(!plausible_gpu_time(Duration::ZERO));
        assert!(!plausible_gpu_time(Duration::from_nanos(u64::from(
            u32::MAX
        ))));
    }

    #[test]
    fn a_late_frame_resets_the_count_to_rise() {
        let mut scale = DynamicResolution::default();
        for _ in 0..ROOM_MEASURES - 1 {
            scale.record(ms(7), BUDGET);
        }
        scale.record(ms(16), BUDGET);
        assert!(!scale.record(ms(7), BUDGET));
        assert!((scale.factor() - 2.0 / 3.0).abs() < 1.0e-6);
    }
}
