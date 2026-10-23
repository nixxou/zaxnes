//! The parts of the procedural skyboxes that do not need a GPU: the Gradient sky of SPEC-06
//! REQ-SCN-050 and the sun, moon and clouds of AMD-024 REQ-PUB-003.

use zax_engine3d::palette::Rgb24;

use crate::numerics::Vector3;

/// The stars of the Gradient sky turn 3° per second of real time.
pub const STAR_DEGREES_PER_SECOND: f32 = 3.0;

/// Grey level of `M` at or below which stars are added.
pub const STAR_GREY_THRESHOLD: f32 = 0.4;

/// Factor applied to `M` at the zenith, `Hc = 1.4 M`.
pub const ZENITH_FACTOR: f32 = 1.4;

/// Factor applied to `M` at the nadir, `Bc = 0.6 M`.
pub const NADIR_FACTOR: f32 = 0.6;

/// The linear factor the "×2" of REQ-SCN-051 stands for: what doubling an sRGB value does to
/// its linear value, under the sRGB curve SPEC-06 §2 declares normative.
///
/// REQ-SCN-051 writes two numbers in the same sentence: « multiplier la teinte linéarisée par
/// **4,595** (doublement en sRGB) ». 4.595 is 2^2.2, the pure power law; the piecewise sRGB
/// curve of §2 gives `lin(1) / lin(0.5) = 4.672`. AMD-019 (Q-045) settles it: the curve of §2
/// is the authority, the exact value is the one it gives, « ×2 » was an approximation, and the
/// criterion carries a ±2 % tolerance (Q-045, Q-055).
pub const SRGB_DOUBLE: f32 = 4.672;

/// The grey level of REQ-SCN-050: `0.299 R + 0.587 G + 0.114 B` on sRGB 0–1.
#[must_use]
pub fn grey(color: Rgb24) -> f32 {
    ((0.299 * f32::from(color.r)) + (0.587 * f32::from(color.g)) + (0.114 * f32::from(color.b)))
        / 255.0
}

/// Whether the Gradient sky shows stars for the background colour `m`.
#[must_use]
pub fn stars_visible(m: Rgb24) -> bool {
    grey(m) <= STAR_GREY_THRESHOLD
}

/// The colour of the Gradient sky in the direction whose vertical component is `y`, before the
/// stars are mixed in: `g` of REQ-SCN-050, in sRGB and **not** saturated. `m` is the universal
/// NES background colour.
#[must_use]
pub fn sky(m: Rgb24, y: f32) -> (f32, f32, f32) {
    let channel = |value: u8| {
        let value = f32::from(value) / 255.0;
        let up = y.max(0.0).powf(2.5);
        let down = (-y).max(0.0).powf(2.5);
        let c = value + (((ZENITH_FACTOR * value) - value) * up);
        c + (((NADIR_FACTOR * value) - c) * down)
    };

    (channel(m.r), channel(m.g), channel(m.b))
}

/// The celestial bodies of the procedural skyboxes of AMD-024 REQ-PUB-003: a sun disc, a moon
/// disc and a cloud layer, all produced by the skybox shader and driven from here.
///
/// Q-064: REQ-PUB-003 asks the procedural skyboxes to carry « soleil et lune, nuages » among
/// their functional parameters, while SPEC-06 REQ-SCN-050 states that the sun, the moon and the
/// clouds of the original skybox pack **are disabled** — and gives an acceptance criterion
/// (horizon, zenith, nadir) that a visible sun would break. The two are reconciled by making the
/// three bodies real, parametric and **off by default**: every gain starts at 0, so the default
/// image is the gradient and the stars of REQ-SCN-050.
///
/// The bodies are fixed in the scene frame, like the rest of the skybox (REQ-SCN-049): nothing
/// here follows the clock.
#[derive(Debug, Clone, PartialEq)]
pub struct SkyBodySettings {
    sun_gain: f32,
    moon_gain: f32,
    cloud_gain: f32,
    sun_radius_degrees: f32,
    moon_radius_degrees: f32,
    cloud_coverage: f32,
    sun_direction: Vector3,
    moon_direction: Vector3,
    /// Colour of the sun disc, sRGB.
    pub sun_color: Rgb24,
    /// Colour of the moon disc, sRGB.
    pub moon_color: Rgb24,
    /// Colour of the clouds, sRGB.
    pub cloud_color: Rgb24,
    /// How far the edge of a disc is feathered, as a fraction of its radius. 0 is a hard edge.
    pub disc_softness: f32,
    /// Angular scale of the cloud pattern; larger values give smaller cells.
    pub cloud_scale: f32,
}

impl Default for SkyBodySettings {
    fn default() -> Self {
        Self {
            sun_gain: 0.0,
            moon_gain: 0.0,
            cloud_gain: 0.0,
            sun_radius_degrees: 2.4,
            moon_radius_degrees: 3.2,
            cloud_coverage: 0.45,
            sun_direction: Vector3::new(-0.45, 0.62, 1.0).normalize(),
            moon_direction: Vector3::new(0.7, 0.42, -1.0).normalize(),
            sun_color: Rgb24::new(255, 243, 214),
            moon_color: Rgb24::new(226, 230, 238),
            cloud_color: Rgb24::new(242, 244, 248),
            disc_softness: 0.45,
            cloud_scale: 2.6,
        }
    }
}

impl SkyBodySettings {
    /// Weight of the sun disc in the sky, 0 (the default) to 1.
    #[must_use]
    pub const fn sun_gain(&self) -> f32 {
        self.sun_gain
    }

    /// Sets the weight of the sun disc, clamped to 0..1.
    pub fn set_sun_gain(&mut self, value: f32) {
        self.sun_gain = value.clamp(0.0, 1.0);
    }

    /// Weight of the moon disc, 0 (the default) to 1.
    #[must_use]
    pub const fn moon_gain(&self) -> f32 {
        self.moon_gain
    }

    /// Sets the weight of the moon disc, clamped to 0..1.
    pub fn set_moon_gain(&mut self, value: f32) {
        self.moon_gain = value.clamp(0.0, 1.0);
    }

    /// Weight of the cloud layer, 0 (the default) to 1.
    #[must_use]
    pub const fn cloud_gain(&self) -> f32 {
        self.cloud_gain
    }

    /// Sets the weight of the cloud layer, clamped to 0..1.
    pub fn set_cloud_gain(&mut self, value: f32) {
        self.cloud_gain = value.clamp(0.0, 1.0);
    }

    /// Direction of the sun in scene coordinates, normalised.
    #[must_use]
    pub const fn sun_direction(&self) -> Vector3 {
        self.sun_direction
    }

    /// Sets the direction of the sun; a zero vector points it straight up.
    pub fn set_sun_direction(&mut self, value: Vector3) {
        self.sun_direction = safe_direction(value);
    }

    /// Direction of the moon in scene coordinates, normalised.
    #[must_use]
    pub const fn moon_direction(&self) -> Vector3 {
        self.moon_direction
    }

    /// Sets the direction of the moon; a zero vector points it straight up.
    pub fn set_moon_direction(&mut self, value: Vector3) {
        self.moon_direction = safe_direction(value);
    }

    /// Angular radius of the sun disc in degrees, 0.2 to 30.
    #[must_use]
    pub const fn sun_radius_degrees(&self) -> f32 {
        self.sun_radius_degrees
    }

    /// Sets the angular radius of the sun disc, clamped to 0.2..30 degrees.
    pub fn set_sun_radius_degrees(&mut self, value: f32) {
        self.sun_radius_degrees = value.clamp(0.2, 30.0);
    }

    /// Angular radius of the moon disc in degrees, 0.2 to 30.
    #[must_use]
    pub const fn moon_radius_degrees(&self) -> f32 {
        self.moon_radius_degrees
    }

    /// Sets the angular radius of the moon disc, clamped to 0.2..30 degrees.
    pub fn set_moon_radius_degrees(&mut self, value: f32) {
        self.moon_radius_degrees = value.clamp(0.2, 30.0);
    }

    /// How much of the sky the clouds cover, 0 to 1. Low values leave a few wisps, high values
    /// an overcast sky.
    #[must_use]
    pub const fn cloud_coverage(&self) -> f32 {
        self.cloud_coverage
    }

    /// Sets the cloud coverage, clamped to 0..1.
    pub fn set_cloud_coverage(&mut self, value: f32) {
        self.cloud_coverage = value.clamp(0.0, 1.0);
    }

    /// Whether anything at all is drawn, that is whether some gain is not zero.
    #[must_use]
    pub fn any_visible(&self) -> bool {
        self.sun_gain > 0.0 || self.moon_gain > 0.0 || self.cloud_gain > 0.0
    }

    /// Puts every parameter back to its default, which draws no body at all.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

fn safe_direction(value: Vector3) -> Vector3 {
    let length = value.length();
    if length > 1e-6 {
        value / length
    } else {
        Vector3::UNIT_Y
    }
}
