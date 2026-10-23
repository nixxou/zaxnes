//! The emulated frame clock of SPEC-07 §3.2: frames are due at `t0 + n / f` on a monotonic
//! clock, late frames are caught up to a limit of five, and the clock can be reset without any
//! catch-up.
//!
//! The nominal rate is 60 frames/s in NTSC and 50 in PAL and Dendy (DEC-R2-T3); the speed factor
//! `k` of REQ-LOOP-015 multiplies it. The pacing is deliberately host-side only: the kernel
//! exposes "run exactly one emulated frame" and nothing else (REQ-LOOP-062).

use std::time::Instant;

/// A monotonic clock in seconds. A test passes its own so that the pacing is checked without
/// waiting.
pub type Clock = Box<dyn Fn() -> f64>;

/// Highest number of emulated frames that may be late (REQ-LOOP-012).
pub const MAX_BACKLOG: u32 = 5;
/// A frame may start up to 1 ms early (REQ-LOOP-011).
pub const EARLY_START_SECONDS: f64 = 0.001;

/// A clock that starts at zero now and follows the system's monotonic time.
pub fn system_clock() -> Clock {
    let start = Instant::now();
    Box::new(move || start.elapsed().as_secs_f64())
}

/// The frame pacer.
pub struct FramePacer {
    clock: Clock,
    next_due: f64,
    /// Nominal rate of the active region, in emulated frames per second.
    pub nominal_rate: f64,
    /// Speed factor `k` of REQ-LOOP-015, 1 by default.
    pub speed: f64,
    frame_count: u64,
    dropped_frames: u64,
}

impl FramePacer {
    /// Creates a pacer at `nominal_rate` frames per second (60 or 50, DEC-R2-T3) on `clock`.
    #[must_use]
    pub fn new(nominal_rate: f64, clock: Clock) -> Self {
        let next_due = clock();
        Self {
            clock,
            next_due,
            nominal_rate,
            speed: 1.0,
            frame_count: 0,
            dropped_frames: 0,
        }
    }

    /// The effective rate, `nominal x k`.
    #[must_use]
    pub fn effective_rate(&self) -> f64 {
        (self.nominal_rate * self.speed).max(1.0)
    }

    /// Elapsed real time in seconds on the pacer's clock.
    #[must_use]
    pub fn elapsed_seconds(&self) -> f64 {
        (self.clock)()
    }

    /// Emulated frames the pacer has handed out.
    #[must_use]
    pub const fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Frames dropped because the backlog went past the limit (REQ-LOOP-012).
    #[must_use]
    pub const fn dropped_frames(&self) -> u64 {
        self.dropped_frames
    }

    /// Resets the clock, so that nothing is caught up: what a resume, a reset, the end of a load
    /// and a rollback restore do (REQ-LOOP-013).
    pub fn reset(&mut self) {
        self.next_due = (self.clock)();
    }

    /// How many emulated frames are due now, at most [`MAX_BACKLOG`]. Any further backlog is
    /// abandoned, so the game slows down instead of suddenly speeding up.
    pub fn due_frames(&mut self) -> u32 {
        let now = (self.clock)();
        let period = 1.0 / self.effective_rate();
        let mut due = 0;
        while now + EARLY_START_SECONDS >= self.next_due {
            self.next_due += period;
            due += 1;
            if due < MAX_BACKLOG {
                continue;
            }
            // REQ-LOOP-012: the backlog never exceeds five frames; the rest is dropped.
            while now + EARLY_START_SECONDS >= self.next_due {
                self.next_due += period;
                self.dropped_frames += 1;
            }
            break;
        }
        self.frame_count += u64::from(due);
        due
    }
}
