//! The rate controller of the audio output (Q-153 §3): it keeps the core's sample ring at the
//! configured latency by nudging the resampling ratio, within the ±0,5 % REQ-AUD-005 allows.
//!
//! The device's clock and the emulation's clock are never exactly equal. A crystal off by
//! 0,03 % — ordinary for a sound card — drains or fills the ring by 0,3 ms every second, which
//! is 18 ms a minute: without correction the ring either empties into droughts or fills until
//! the oldest samples are dropped.
//!
//! What v0.4.2 did was proportional only, with a gain of 0,002 against a relative error, and it
//! measured the ring alone while the SDL queue held most of the latency. A proportional term
//! cannot hold a steady offset: it needs a standing error to produce any correction at all, and
//! that standing error was paid in blocks of silence. So:
//!
//! * the error is a **latency in milliseconds**, measured by the audio thread on what it really
//!   found in the ring and averaged over about a second;
//! * a **dead band** of [`DEAD_BAND_MS`] keeps the proportional term quiet while the latency is
//!   near enough, so the pitch does not wander for nothing;
//! * a **capped proportional** term reacts to a real excursion;
//! * a **slowly accumulating** term integrates the raw error — inside the dead band too — and
//!   that is what finally cancels the clock offset, so the latency settles *on* its target
//!   instead of below it;
//! * a drift of [`RESYNC_MS`] is not a clock offset but an accident (a long stall, a
//!   fast-forward burst): the controller asks for a resynchronisation rather than trying to
//!   claw it back at 0,25 %.
//!
//! The correction only runs at 100 % speed and outside a recording; elsewhere it is neutral,
//! because the core is retiming its own resampler for the speed factor (REQ-AUD-011) and a
//! recorded movie must not depend on this machine's sound card.

/// Latency error, in milliseconds, the proportional term ignores.
pub const DEAD_BAND_MS: f64 = 3.0;
/// Proportional gain, per millisecond of error.
pub const PROPORTIONAL_GAIN: f64 = 1.25e-4;
/// Largest correction the proportional term alone may ask for: 0,25 %.
pub const PROPORTIONAL_CAP: f64 = 0.002_5;
/// Accumulation, per millisecond of error and per second.
pub const OFFSET_GAIN: f64 = 4.0e-5;
/// How fast the accumulated term bleeds away, per second.
///
/// An accumulator with a dead band in front of the proportional term has no damping of its own:
/// the ring's fill and the accumulated correction form an oscillator of period
/// `2π / √(1000 · OFFSET_GAIN)` ≈ 31 s, and inside the dead band nothing opposes it. The leak is
/// that damping. It costs a residual error of `LEAK · drift / OFFSET_GAIN` — about 0,4 ms for a
/// sound card off by 0,03 % — which is far inside the dead band and invisible.
pub const OFFSET_LEAK: f64 = 0.05;
/// Largest correction the accumulated term may reach: 0,06 %. A sound card off by more than
/// that is beyond what REQ-AUD-005 lets us hide.
pub const OFFSET_CAP: f64 = 0.000_6;
/// Latency error that calls for a resynchronisation rather than a correction.
pub const RESYNC_MS: f64 = 50.0;
/// How long after a resynchronisation another one is refused, in seconds: the measured average
/// needs about a second to catch up, and asking twice would stutter.
pub const RESYNC_COOLDOWN_SECONDS: f64 = 2.0;

/// What one step of the controller decided.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Correction {
    /// The fine adjustment of the resampling ratio to ask the core for.
    pub adjustment: f64,
    /// Whether the ring should be emptied and prefilled again.
    pub resynchronise: bool,
}

/// The controller's state: only the accumulated term and the resynchronisation cooldown.
#[derive(Debug, Clone, Copy, Default)]
pub struct RateControl {
    offset: f64,
    cooldown: f64,
    adjustment: f64,
}

impl RateControl {
    /// A controller that has not corrected anything yet.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            offset: 0.0,
            cooldown: 0.0,
            adjustment: 0.0,
        }
    }

    /// The correction asked for at the last step, for the overlay.
    #[must_use]
    pub const fn adjustment(&self) -> f64 {
        self.adjustment
    }

    /// The accumulated term, which is this machine's clock offset once it has settled.
    #[must_use]
    pub const fn offset(&self) -> f64 {
        self.offset
    }

    /// Stops correcting without forgetting the clock offset: a pause, a speed other than 100 %
    /// or a recording. The accumulated term is kept because the sound card has not changed.
    pub fn idle(&mut self, seconds: f64) -> Correction {
        self.cooldown = (self.cooldown - seconds).max(0.0);
        self.adjustment = 0.0;
        Correction {
            adjustment: 0.0,
            resynchronise: false,
        }
    }

    /// Forgets the accumulated term: the ring was emptied and refilled, so what it had learnt
    /// about the drift no longer applies to the new fill.
    pub fn restart(&mut self) {
        self.offset = 0.0;
        self.cooldown = RESYNC_COOLDOWN_SECONDS;
        self.adjustment = 0.0;
    }

    /// One step. `error_ms` is the measured latency minus the target, in milliseconds, and
    /// `seconds` the real time since the previous step. `settled` says whether the stream is
    /// past its prefill: while it is not, the measurement means nothing and only time passes.
    pub fn step(&mut self, error_ms: f64, seconds: f64, settled: bool, cap: f64) -> Correction {
        self.cooldown = (self.cooldown - seconds).max(0.0);
        if !settled {
            self.adjustment = 0.0;
            return Correction {
                adjustment: 0.0,
                resynchronise: false,
            };
        }
        if error_ms.abs() >= RESYNC_MS && self.cooldown <= 0.0 {
            self.restart();
            return Correction {
                adjustment: 0.0,
                resynchronise: true,
            };
        }
        // A ring that is too full means the core produces faster than the device consumes, so
        // the ratio has to come down: the correction is the opposite sign of the error.
        let proportional = if error_ms.abs() > DEAD_BAND_MS {
            (-error_ms * PROPORTIONAL_GAIN).clamp(-PROPORTIONAL_CAP, PROPORTIONAL_CAP)
        } else {
            0.0
        };
        // The accumulated term integrates the raw error, the dead band included: that is the
        // whole point of it, since a steady error of a millisecond or two is exactly what the
        // proportional term is built to ignore. The leak damps the oscillation the dead band
        // would otherwise leave undamped.
        let integrated = self.offset - (error_ms * OFFSET_GAIN * seconds);
        self.offset =
            (integrated - (self.offset * OFFSET_LEAK * seconds)).clamp(-OFFSET_CAP, OFFSET_CAP);
        self.adjustment = (proportional + self.offset).clamp(-cap, cap);
        Correction {
            adjustment: self.adjustment,
            resynchronise: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CAP: f64 = 0.005;

    #[test]
    fn the_dead_band_asks_for_nothing_at_once() {
        let mut control = RateControl::new();
        let correction = control.step(1.0, 1.0 / 60.0, true, CAP);
        assert!(!correction.resynchronise);
        // The proportional term is silent; only the accumulation has moved, by a hair.
        assert!(correction.adjustment.abs() < 1e-6, "{correction:?}");
    }

    #[test]
    fn a_large_excursion_is_capped_and_opposes_the_error() {
        let mut control = RateControl::new();
        let full = control.step(40.0, 1.0 / 60.0, true, CAP);
        assert!(full.adjustment < 0.0, "a full ring must slow production");
        assert!(full.adjustment >= -PROPORTIONAL_CAP - OFFSET_CAP);
        let empty = RateControl::new().step(-40.0, 1.0 / 60.0, true, CAP);
        assert!(empty.adjustment > 0.0, "an empty ring must speed it up");
    }

    #[test]
    fn the_accumulated_term_absorbs_a_steady_offset_inside_the_dead_band() {
        let mut control = RateControl::new();
        // Two milliseconds short, for ten seconds, at sixty steps a second.
        for _ in 0..600 {
            control.step(-2.0, 1.0 / 60.0, true, CAP);
        }
        // The accumulation points the way that refills the ring, and stays within its cap.
        assert!(control.offset() > 3e-4, "{}", control.offset());
        assert!(control.offset() <= OFFSET_CAP);
    }

    /// The loop closed over a device whose clock is `drift` off the emulation's, stepped at
    /// 60 Hz for `seconds`. Returns the error at the end and the worst error seen after the
    /// first minute, both in milliseconds, and the least fill reached as a latency.
    fn simulate(drift: f64, seconds: f64, target_ms: f64) -> (f64, f64, f64) {
        let mut control = RateControl::new();
        let step = 1.0 / 60.0;
        // The audio thread's average lags the truth by about a second; that lag is part of the
        // loop and is what could make it ring, so it is simulated too.
        let mut measured = target_ms;
        let mut fill = target_ms;
        let mut worst = 0.0_f64;
        let mut least = target_ms;
        let mut elapsed = 0.0;
        let mut adjustment = 0.0;
        while elapsed < seconds {
            // The ring gains what the core produced beyond what the device consumed.
            fill += (adjustment - drift) * 1000.0 * step;
            least = least.min(fill);
            let weight = (step / 1.0).min(1.0);
            measured += (fill - measured) * weight;
            adjustment = control
                .step(measured - target_ms, step, true, CAP)
                .adjustment;
            elapsed += step;
            if elapsed > 60.0 {
                worst = worst.max((fill - target_ms).abs());
            }
        }
        (fill - target_ms, worst, least)
    }

    #[test]
    fn a_device_clock_off_by_three_hundredths_of_a_percent_settles_inside_the_dead_band() {
        // An hour of play, the device a little fast then a little slow, at the default latency.
        for drift in [0.000_3, -0.000_3] {
            let (final_error, worst, least) = simulate(drift, 3600.0, 60.0);
            assert!(
                final_error.abs() < DEAD_BAND_MS,
                "drift {drift}: settled at {final_error} ms"
            );
            assert!(
                worst < DEAD_BAND_MS,
                "drift {drift}: wandered to {worst} ms after the first minute"
            );
            // Never near empty, so the stream has nothing to under-run on.
            assert!(least > 40.0, "drift {drift}: the ring fell to {least} ms");
        }
    }

    #[test]
    fn a_tenth_of_a_percent_is_still_held_without_emptying_the_ring() {
        // Ten times the usual drift: the accumulation saturates at its cap, the proportional
        // term carries the rest, and the ring keeps a comfortable margin.
        let (final_error, _, least) = simulate(0.001, 600.0, 60.0);
        assert!(final_error.abs() < 10.0, "settled at {final_error} ms");
        assert!(least > 30.0, "the ring fell to {least} ms");
    }

    #[test]
    fn a_far_drift_asks_for_a_resynchronisation_once() {
        let mut control = RateControl::new();
        let first = control.step(RESYNC_MS + 1.0, 1.0 / 60.0, true, CAP);
        assert!(first.resynchronise);
        // The cooldown refuses a second one straight away.
        let second = control.step(RESYNC_MS + 1.0, 1.0 / 60.0, true, CAP);
        assert!(!second.resynchronise);
    }

    #[test]
    fn a_stream_still_prefilling_is_not_corrected() {
        let mut control = RateControl::new();
        let correction = control.step(-60.0, 1.0 / 60.0, false, CAP);
        assert_eq!(correction.adjustment, 0.0);
        assert!(!correction.resynchronise);
        assert_eq!(control.offset(), 0.0);
    }

    #[test]
    fn idling_keeps_what_was_learnt_of_the_clock() {
        let mut control = RateControl::new();
        for _ in 0..600 {
            control.step(-2.0, 1.0 / 60.0, true, CAP);
        }
        let learnt = control.offset();
        control.idle(1.0);
        assert_eq!(control.offset(), learnt);
        assert_eq!(control.adjustment(), 0.0);
    }
}
