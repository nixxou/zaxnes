//! The sRGB ↔ linear conversions of SPEC-06 §2 and REQ-SCN-035, and the scene constants that
//! depend on them.
//!
//! Shading is computed in linear space and re-encoded to sRGB for display (REQ-SCN-028). The
//! same curve is applied to the light intensity (DEC-06-Q2). The index and alpha channels are
//! never converted (REQ-SCN-035).

use zax_engine3d::palette::Rgb24;

use crate::numerics::Vector3;

/// The ambient colour, sRGB (118, 118, 118) (REQ-SCN-027).
pub const AMBIENT: Rgb24 = Rgb24::new(118, 118, 118);

/// The direct-light factor inside a shadow: 1 − 0.8 (REQ-SCN-031).
pub const SHADOW_FACTOR: f32 = 0.2;

/// The ambient term in linear space, 0.1812 (REQ-SCN-027).
#[must_use]
pub fn ambient_linear() -> f32 {
    to_linear(118.0 / 255.0)
}

/// The standard sRGB → linear transfer function (IEC 61966-2-1).
#[must_use]
pub fn to_linear(value: f32) -> f32 {
    if value <= 0.040_45 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

/// The standard linear → sRGB transfer function.
#[must_use]
pub fn to_srgb(value: f32) -> f32 {
    if value <= 0.003_130_8 {
        value * 12.92
    } else {
        (1.055 * value.powf(1.0 / 2.4)) - 0.055
    }
}

/// The linear colour of an sRGB colour given as bytes.
#[must_use]
pub fn rgb_to_linear(color: Rgb24) -> Vector3 {
    Vector3::new(
        to_linear(f32::from(color.r) / 255.0),
        to_linear(f32::from(color.g) / 255.0),
        to_linear(f32::from(color.b) / 255.0),
    )
}

/// The sRGB bytes of a linear colour, saturated to 1 without tone mapping.
#[must_use]
pub fn to_srgb_bytes(linear: Vector3) -> Rgb24 {
    Rgb24::new(
        quantise(to_srgb(linear.x.clamp(0.0, 1.0))),
        quantise(to_srgb(linear.y.clamp(0.0, 1.0))),
        quantise(to_srgb(linear.z.clamp(0.0, 1.0))),
    )
}

/// The sRGB components of an sRGB colour, in the 0..1 range.
#[must_use]
pub fn to_vector(color: Rgb24) -> Vector3 {
    Vector3::new(
        f32::from(color.r) / 255.0,
        f32::from(color.g) / 255.0,
        f32::from(color.b) / 255.0,
    )
}

/// The diffuse Lambert shading of REQ-SCN-028:
/// `albedo_lin × (ambient_lin + max(0, N·L) × light_lin × S)`, encoded back to sRGB.
///
/// `normal` is the mesh normal in the scene frame (unchanged by the deformation),
/// `light_direction` the propagation direction of the rays (REQ-SCN-025), `intensity` the
/// light intensity of REQ-SCN-026 and `shadow` the factor `S`: 1 in the light, 0.2 in shadow.
#[must_use]
pub fn shade(
    albedo: Rgb24,
    normal: Vector3,
    light_direction: Vector3,
    light_color: Rgb24,
    intensity: f32,
    shadow: f32,
) -> Rgb24 {
    let light = rgb_to_linear(light_color) * to_linear(intensity);
    let mut ndl = 0.0;
    if normal.length_squared() > 0.0 && light_direction.length_squared() > 0.0 {
        ndl = Vector3::dot(normal.normalize(), (-light_direction).normalize()).max(0.0);
    }

    let factor = Vector3::splat(ambient_linear()) + (light * (ndl * shadow));
    to_srgb_bytes(rgb_to_linear(albedo) * factor)
}

/// A 0..1 value as a byte, rounded half to even like `MathF.Round`.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // Clamped to 0..255 first.
fn quantise(value: f32) -> u8 {
    (value * 255.0).round_ties_even().clamp(0.0, 255.0) as u8
}
