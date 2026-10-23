//! A gain that moves linearly towards a target in real time (SPEC-11 REQ-AUD-020 to
//! REQ-AUD-023).

/// Seconds a fade over the full range takes: 200 frames at 60 per second.
pub const FULL_FADE_SECONDS: f64 = 200.0 / 60.0;

/// A gain with a target and the range a full fade covers.
#[derive(Debug, Clone, Copy)]
pub struct FadeGain {
    gain: f64,
    target: f64,
    range: f64,
}

impl FadeGain {
    /// Creates a gain settled at `value`.
    #[must_use]
    pub const fn new(value: f64) -> Self {
        Self {
            gain: value,
            target: value,
            range: value.max(0.0),
        }
    }

    /// The current gain.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // the C# `(float)` of the double gain
    pub const fn value(&self) -> f32 {
        self.gain as f32
    }

    /// The gain being faded to.
    #[must_use]
    pub const fn target(&self) -> f64 {
        self.target
    }

    /// Whether the gain has reached its target.
    #[must_use]
    pub fn is_settled(&self) -> bool {
        (self.gain - self.target).abs() <= 1e-9
    }

    /// Fades to `target`, a full `range` taking [`FULL_FADE_SECONDS`].
    pub const fn fade_to(&mut self, target: f64, range: f64) {
        self.target = target;
        self.range = range.max(0.0);
    }

    /// Like [`fade_to`](Self::fade_to), but jumps to the target at once when the gain had
    /// settled (a volume change does not cut a fade in progress, REQ-AUD-023).
    pub fn retarget(&mut self, target: f64, range: f64) {
        let settled = self.is_settled();
        self.fade_to(target, range);
        if settled {
            self.gain = target;
        }
    }

    /// Jumps to `target` with no fade.
    pub const fn snap(&mut self, target: f64, range: f64) {
        self.fade_to(target, range);
        self.gain = target;
    }

    /// Advances the fade by `real_seconds`.
    pub fn advance(&mut self, real_seconds: f64) {
        if real_seconds <= 0.0 || self.is_settled() {
            return;
        }
        let step = self.range / FULL_FADE_SECONDS * real_seconds;
        if step <= 0.0 {
            self.gain = self.target;
            return;
        }
        self.gain = if self.gain < self.target {
            self.target.min(self.gain + step)
        } else {
            self.target.max(self.gain - step)
        };
    }
}
