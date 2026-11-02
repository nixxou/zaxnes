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

/// Perceptual roughness of the backdrop material (AMD-049 REQ-SCN-075): almost matt.
pub const BACKDROP_ROUGHNESS: f32 = 0.99;

/// Normal reflectance of the backdrop material, and the `0.96` the diffuse part keeps
/// (REQ-SCN-075: a non-metallic surface reflects 4 % at normal incidence, and the diffuse part is
/// reduced by as much).
pub const BACKDROP_REFLECTANCE: f32 = 0.04;

/// The backdrop lighting of REQ-SCN-075 (AMD-049), in **linear** space, for one point.
///
/// This is the CPU twin of `shaders/backdrop.frag`: the same model, written once here so the
/// acceptance vectors of AMD-049 — which are stated as linear values before encoding — can be
/// checked without a GL context, and so the renderer and this function cannot drift apart
/// unnoticed (a test renders the same configuration and compares).
///
/// * `albedo_linear` — `c`, `Backdrop.Color` converted to linear;
/// * `normal` — `N`, the unit normal of the face, already transported by the curvature when
///   REQ-SCN-068 is active;
/// * `to_eye` — `V`, the unit vector from the point towards the eye;
/// * `to_light` — `L`, `−Light.Direction` normalised;
/// * `light_linear` — `E`, the light colour times the intensity, converted to linear
///   (REQ-SCN-028, DEC-06-Q2);
/// * `ambient` — `Amb`, the ambient of REQ-SCN-027 ([`ambient_linear`]);
/// * `shadow` — `s`, 1 in the light and [`SHADOW_FACTOR`] in shadow (REQ-SCN-031). It scales the
///   specular term as well as the direct diffuse one, because the specular term *is* direct
///   light; there is no environment reflection.
#[must_use]
pub fn backdrop_linear(
    albedo_linear: Vector3,
    normal: Vector3,
    to_eye: Vector3,
    to_light: Vector3,
    light_linear: Vector3,
    ambient: f32,
    shadow: f32,
) -> Vector3 {
    // The one-letter names are the ones REQ-SCN-075 itself uses for these vectors; renaming them
    // would make the model harder to check against the amendment, not easier.
    #[allow(clippy::many_single_char_names)]
    let (n, v, l) = (
        unit_or(normal, Vector3::new(0.0, 0.0, -1.0)),
        unit_or(to_eye, Vector3::new(0.0, 0.0, -1.0)),
        unit_or(to_light, Vector3::new(0.0, 0.0, -1.0)),
    );
    // REQ-SCN-075: H = (L + V) normalised, and H = N when L + V = 0 (the degenerate case R2
    // raised as 49-8, which happens when the eye looks from exactly the far side of the light).
    let half = unit_or(l + v, n);

    let nl = Vector3::dot(n, l).max(0.0);
    let nv = Vector3::dot(n, v).abs();
    let nh = Vector3::dot(n, half).max(0.0);
    let lh = Vector3::dot(l, half).max(0.0);

    let roughness = BACKDROP_ROUGHNESS;
    let alpha = roughness * roughness;

    // The diffuse term.
    let f90 = 0.5 + 2.0 * lh * lh * roughness;
    let fresnel = |cosine: f32| 1.0 + (f90 - 1.0) * (1.0 - cosine).powi(5);
    let fd = fresnel(nl) * fresnel(nv);

    // The specular term.
    let visibility =
        0.5 / (nl * (nv * (1.0 - alpha) + alpha) + nv * (nl * (1.0 - alpha) + alpha) + 1.0e-5);
    let denominator = nh * nh * (alpha * alpha - 1.0) + 1.0;
    let distribution = alpha * alpha / (std::f32::consts::PI * denominator * denominator);
    let specular = visibility * distribution * std::f32::consts::PI * nl;
    let reflected = BACKDROP_REFLECTANCE + (1.0 - BACKDROP_REFLECTANCE) * (1.0 - lh).powi(5);

    let diffuse = albedo_linear
        * (1.0 - BACKDROP_REFLECTANCE)
        * (Vector3::splat(ambient) + light_linear * (shadow * fd * nl));
    diffuse + light_linear * (shadow * specular * reflected)
}

/// `vector` normalised, or `fallback` when it has no length.
fn unit_or(vector: Vector3, fallback: Vector3) -> Vector3 {
    if vector.length_squared() > 0.0 {
        vector.normalize()
    } else {
        fallback
    }
}

/// A 0..1 value as a byte, rounded half to even like `MathF.Round`.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // Clamped to 0..255 first.
fn quantise(value: f32) -> u8 {
    (value * 255.0).round_ties_even().clamp(0.0, 255.0) as u8
}
