//! Exponentially smoothed frame time, so that the displayed FPS does not jitter.

const SMOOTHING: f64 = 0.1;

/// The smoothed duration of the displayed frames.
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameRateCounter {
    smoothed_seconds: f64,
}

impl FrameRateCounter {
    /// Creates a counter that has seen no frame.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            smoothed_seconds: 0.0,
        }
    }

    /// Records the duration of the frame that just finished, in seconds.
    pub fn add(&mut self, delta_seconds: f64) {
        if delta_seconds <= 0.0 || delta_seconds.is_nan() {
            return;
        }
        self.smoothed_seconds = if self.smoothed_seconds <= 0.0 {
            delta_seconds
        } else {
            self.smoothed_seconds + ((delta_seconds - self.smoothed_seconds) * SMOOTHING)
        };
    }

    /// Smoothed frames per second, or zero before the first frame.
    #[must_use]
    pub fn frames_per_second(&self) -> f64 {
        if self.smoothed_seconds > 0.0 {
            1.0 / self.smoothed_seconds
        } else {
            0.0
        }
    }

    /// Smoothed frame duration in milliseconds.
    #[must_use]
    pub fn frame_milliseconds(&self) -> f64 {
        self.smoothed_seconds * 1000.0
    }
}
