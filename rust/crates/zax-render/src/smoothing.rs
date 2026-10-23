//! The critical smoothing filter of SPEC-06 REQ-SCN-019.

/// A critically damped spring that never overshoots, with characteristic time `Tₛ`
/// (REQ-SCN-019).
///
/// One instance holds the internal velocity of one scalar. The camera applies it separately to
/// `rx`, `ry`, `rz` (without folding modulo 360°), to the distance and to the three components
/// of the target point (REQ-SCN-019).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CriticalSmoothing {
    velocity: f32,
}

impl CriticalSmoothing {
    /// The default characteristic time, 0.4 s (REQ-SCN-019).
    pub const DEFAULT_SMOOTH_TIME: f32 = 0.4;

    /// The internal velocity, zero before the first step.
    #[must_use]
    pub const fn velocity(&self) -> f32 {
        self.velocity
    }

    /// Forgets the velocity, so the next step starts from rest.
    pub fn reset(&mut self) {
        self.velocity = 0.0;
    }

    /// Moves `value` towards `target` over `delta_seconds`, with the characteristic time
    /// `smooth_time`, and returns the new value.
    pub fn step(&mut self, value: f32, target: f32, smooth_time: f32, delta_seconds: f32) -> f32 {
        if delta_seconds <= 0.0 {
            return value;
        }

        // ω = 2 / Tₛ, x = ω·Δt, k = 1 / (1 + x + 0.48 x² + 0.235 x³) (REQ-SCN-019).
        let omega = 2.0 / smooth_time.max(1e-4);
        let x = omega * delta_seconds;
        let k = 1.0 / (1.0 + x + (0.48 * x * x) + (0.235 * x * x * x));

        let delta = value - target;
        let w = (self.velocity + (omega * delta)) * delta_seconds;
        self.velocity = (self.velocity - (omega * w)) * k;
        let result = target + ((delta + w) * k);

        // The setpoint must never be overshot (REQ-SCN-019).
        if (target - value > 0.0) == (result > target) {
            self.velocity = 0.0;
            return target;
        }

        result
    }
}
