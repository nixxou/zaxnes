//! The pull on the head-on 2D picture (Rust-only, owner request): in 2D seen head on the camera
//! does not turn, but a rotation asked for (the camera actions of a stick or keys, a drag of the
//! mouse) fills a gauge, the picture tilting a little its way like a rubber band. Past
//! [`RELEASE_DEGREES`] the picture gives way: the 2D is drawn through the 3D camera from then
//! on, which starts from the head-on view and turns. The gauge empties by itself once nothing
//! pulls, so a brush of a stick never adds up to a release. The head-on view comes back with the
//! default view and each time the game goes into 2D.

/// The rotation, in degrees, the gauge holds when the picture gives way: 160 pixels of drag, or
/// a stick held about half a second (owner request: 8 was too soft).
pub const RELEASE_DEGREES: f32 = 16.0;

/// The tilt of the picture, in degrees, when the gauge is full.
pub const MAX_TILT_DEGREES: f32 = 2.0;

/// Seconds for the gauge to lose half of what it holds once nothing pulls.
pub const HALF_LIFE_SECONDS: f32 = 0.15;

/// What a frame of pull comes to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Pull {
    /// The picture stays head on, tilted by `(rx, ry)` degrees.
    Held([f32; 2]),
    /// The picture gives way: the camera starts tilted by the first `(rx, ry)` and heads for
    /// the second, the whole rotation asked for.
    Released([f32; 2], [f32; 2]),
}

/// The gauge, `(rx, ry)` in degrees, as the camera's setpoint angles count them.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct HeadOnPull {
    gauge: [f32; 2],
}

impl HeadOnPull {
    /// An empty gauge.
    #[must_use]
    pub const fn new() -> Self {
        Self { gauge: [0.0; 2] }
    }

    /// Empties the gauge (the picture is head on again).
    pub const fn clear(&mut self) {
        self.gauge = [0.0; 2];
    }

    /// One frame of `seconds` with the rotation `asked` (degrees, `(rx, ry)`) this frame.
    pub fn update(&mut self, asked: [f32; 2], seconds: f32) -> Pull {
        if asked.iter().all(|degrees| degrees.abs() < f32::EPSILON) {
            let kept = 0.5_f32.powf(seconds.max(0.0) / HALF_LIFE_SECONDS);
            self.gauge = self.gauge.map(|degrees| degrees * kept);
        } else {
            self.gauge = [self.gauge[0] + asked[0], self.gauge[1] + asked[1]];
        }
        let length = self.gauge[0].hypot(self.gauge[1]);
        let tilt = self
            .gauge
            .map(|degrees| degrees * MAX_TILT_DEGREES / RELEASE_DEGREES);
        if length >= RELEASE_DEGREES {
            let whole = self.gauge;
            self.clear();
            Pull::Released(tilt, whole)
        } else {
            Pull::Held(tilt)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: f32 = 1.0 / 60.0;

    #[test]
    fn a_brush_of_a_stick_never_adds_up_to_a_release() {
        let mut pull = HeadOnPull::new();
        // A stick at 30 degrees a second, a fifth of a second every half second, for a minute.
        for frame in 0..3600 {
            let asked = if frame % 30 < 12 { 30.0 * FRAME } else { 0.0 };
            assert!(matches!(pull.update([0.0, asked], FRAME), Pull::Held(_)));
        }
    }

    #[test]
    fn a_stick_held_half_a_second_releases_the_picture() {
        let mut pull = HeadOnPull::new();
        let mut frames = 0;
        loop {
            frames += 1;
            match pull.update([0.0, 30.0 * FRAME], FRAME) {
                Pull::Held(tilt) => assert!(tilt[1] > 0.0 && tilt[1] < MAX_TILT_DEGREES),
                Pull::Released(tilt, whole) => {
                    assert!((tilt[1] - MAX_TILT_DEGREES).abs() < 0.1);
                    assert!((whole[1] - RELEASE_DEGREES).abs() < 0.6);
                    break;
                }
            }
        }
        assert!((31..=33).contains(&frames), "{frames}");
        assert_eq!(Pull::Held([0.0, 0.0]), pull.update([0.0; 2], FRAME));
    }

    #[test]
    fn the_gauge_empties_by_half_in_its_half_life() {
        let mut pull = HeadOnPull::new();
        pull.update([4.0, 0.0], FRAME);
        let Pull::Held(tilt) = pull.update([0.0; 2], HALF_LIFE_SECONDS) else {
            panic!("released");
        };
        assert!((tilt[0] - 0.25).abs() < 1e-4, "{tilt:?}");
    }
}
