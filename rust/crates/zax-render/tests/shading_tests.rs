//! The light and the shading of SPEC-06 §3.9 to §3.11 and the test vectors of §5.2 (port of
//! `ShadingTests.cs`).

mod common;

use common::assert_close;
use zax_engine3d::palette::{nes_color, Rgb24};
use zax_render::color;
use zax_render::numerics::Vector3;
use zax_render::scene::RenderSettings;

/// The default light direction of REQ-SCN-025, white, intensity 1.
const DIRECTION: Vector3 = Vector3::new(1.0, -1.0, 1.0);
const WHITE: Rgb24 = Rgb24::new(255, 255, 255);

fn close(expected: f32, actual: f32, tolerance: f64) {
    assert_close(f64::from(expected), f64::from(actual), tolerance);
}

#[track_caller]
fn in_range(value: u8, low: i32, high: i32) {
    let value = i32::from(value);
    assert!((low..=high).contains(&value), "{value} not in {low}..={high}");
}

#[track_caller]
fn assert_shade(albedo: Rgb24, normal: Vector3, shadow: f32, r: i32, g: i32, b: i32) {
    let shaded = color::shade(albedo, normal, DIRECTION, WHITE, 1.0, shadow);
    in_range(shaded.r, r - 3, r + 3);
    in_range(shaded.g, g - 3, g + 3);
    in_range(shaded.b, b - 3, b + 3);
}

#[test]
fn req_scn_027_the_ambient_term_is_0_1812_in_linear_space() {
    close(0.1812, color::ambient_linear(), 1e-4);
}

#[test]
fn req_scn_025_the_three_faces_towards_the_light_get_the_same_illumination() {
    for normal in [
        Vector3::new(0.0, 0.0, -1.0),
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(-1.0, 0.0, 0.0),
    ] {
        let ndl = Vector3::dot(normal, (-DIRECTION).normalize());
        close(0.577, ndl, 1e-3);
    }

    // The opposite faces only get the ambient light (REQ-SCN-025 criterion).
    for normal in [
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, -1.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
    ] {
        let ndl = Vector3::dot(normal, (-DIRECTION).normalize()).max(0.0);
        close(0.0, ndl, 1e-6);
    }
}

#[test]
fn section_5_2_linear_factors_are_0_7585_0_2966_and_0_1812() {
    let ndl = 0.577_350_3_f32;
    close(0.7585, color::ambient_linear() + ndl, 1e-3);
    close(0.2966, color::ambient_linear() + (ndl * color::SHADOW_FACTOR), 1e-3);
    close(0.1812, color::ambient_linear(), 1e-3);
}

#[test]
fn section_5_2_shading_vectors() {
    // NES colour, sRGB input, lit, in shadow, ambient only (SPEC-06 §5.2, ±3 per channel).
    let rows: [(usize, [u8; 3], [i32; 3], [i32; 3], [i32; 3]); 4] = [
        (0x30, [252, 248, 252], [223, 219, 223], [146, 144, 146], [117, 115, 117]),
        (0x22, [104, 136, 252], [91, 120, 223], [57, 76, 146], [44, 60, 117]),
        (0x17, [228, 96, 24], [202, 84, 20], [132, 52, 9], [105, 40, 5]),
        (0x29, [188, 248, 24], [166, 219, 20], [108, 144, 9], [85, 115, 5]),
    ];
    for (nes, input, lit, shadow, ambient) in rows {
        let albedo = nes_color(nes);
        assert_eq!(Rgb24::new(input[0], input[1], input[2]), albedo);

        let front = Vector3::new(0.0, 0.0, -1.0);
        assert_shade(albedo, front, 1.0, lit[0], lit[1], lit[2]);
        assert_shade(albedo, front, color::SHADOW_FACTOR, shadow[0], shadow[1], shadow[2]);
        let back = Vector3::new(0.0, 0.0, 1.0);
        assert_shade(albedo, back, 1.0, ambient[0], ambient[1], ambient[2]);
    }
}

#[test]
fn section_5_2_pure_white_vector() {
    let albedo = Rgb24::new(255, 255, 255);
    assert_shade(albedo, Vector3::new(0.0, 0.0, -1.0), 1.0, 226, 226, 226);
    assert_shade(albedo, Vector3::new(0.0, 0.0, -1.0), color::SHADOW_FACTOR, 148, 148, 148);
    assert_shade(albedo, Vector3::new(0.0, 0.0, 1.0), 1.0, 118, 118, 118);
}

#[test]
fn req_scn_027_a_white_face_lit_only_by_the_ambient_shows_srgb_117() {
    let shaded = color::shade(
        nes_color(0x30),
        Vector3::new(0.0, 0.0, 1.0),
        DIRECTION,
        WHITE,
        1.0,
        1.0,
    );

    in_range(shaded.r, 115, 119);
    in_range(shaded.g, 113, 117);
    in_range(shaded.b, 115, 119);
}

/// Q-072, lot FIX-4: what the original's own 1080 p capture shows on the unlit side faces of
/// the extruded HUD glyphs of Super Mario Bros. 3. The white glyphs are NES 0x30 over a cyan
/// panel; the faces the light does not reach come out at sRGB 117 in the original and 116 here,
/// so the ambient term of REQ-SCN-027 is the one the original uses and an "ambient intensity of
/// 0,3" — whichever way it is read, 0,054 or 0,3 in linear space — is not.
#[test]
fn q_072_only_the_ambient_of_req_scn_027_matches_the_reference_capture() {
    // linear ambient, expected sRGB of a white face lit by the ambient alone
    for (ambient_linear, expected) in [
        (0.1812_f32, 117), // REQ-SCN-027, what both the original and Zaxnes show
        (0.0544, 65),      // 0,3 read as a factor on the ambient colour: far too dark
        (0.3, 147),        // 0,3 read as the linear ambient itself: too light
    ] {
        // The shading of REQ-SCN-028 for a face with N·L = 0, spelled out so that the three
        // candidate ambients can be compared without changing the renderer's constant.
        let albedo = color::to_linear(f32::from(nes_color(0x30).r) / 255.0);
        let shown = (color::to_srgb(albedo * ambient_linear) * 255.0).round_ties_even() as i32;
        assert_eq!(expected, shown);

        // And the one the renderer actually uses is the first of the three.
        if expected == 117 {
            close(ambient_linear, color::ambient_linear(), 1e-4);
        }
    }
}

#[test]
fn req_scn_031_a_white_front_face_goes_from_223_to_146_in_shadow() {
    // A face facing the viewer: N = (0, 0, −1), so N·L = 0.577 (REQ-SCN-031 criterion).
    let normal = Vector3::new(0.0, 0.0, -1.0);
    let lit = color::shade(nes_color(0x30), normal, DIRECTION, WHITE, 1.0, 1.0);
    let shadowed = color::shade(
        nes_color(0x30),
        normal,
        DIRECTION,
        WHITE,
        1.0,
        color::SHADOW_FACTOR,
    );

    in_range(lit.r, 220, 226);
    in_range(shadowed.r, 143, 149);
}

#[test]
fn dec_06_q2_the_intensity_goes_through_the_same_srgb_curve_as_the_colours() {
    // Intensity 1.5 in sRGB becomes about 1.5^2.4 in linear space, not 1.5.
    let linear = color::to_linear(1.5);
    close(((1.5_f32 + 0.055) / 1.055).powf(2.4), linear, 1e-5);
    close(1.0, color::to_linear(1.0), 1e-6);
    close(0.214_041_1, color::to_linear(0.5), 1e-5);
}

#[test]
fn req_scn_026_the_intensity_setting_is_clamped_to_its_bounds() {
    let mut settings = RenderSettings::default();

    settings.set_light_intensity(9.0);
    close(1.5, settings.light_intensity(), 0.0);
    settings.set_light_intensity(0.0);
    close(0.5, settings.light_intensity(), 0.0);
}

#[test]
fn req_scn_028_the_result_is_saturated_without_tone_mapping() {
    let shaded = color::shade(
        Rgb24::new(255, 255, 255),
        (-DIRECTION).normalize(),
        DIRECTION,
        WHITE,
        1.5,
        1.0,
    );

    assert_eq!(255, shaded.r);
}

#[test]
fn section_2_the_transfer_functions_are_inverses_of_each_other() {
    for step in 0..=255_u8 {
        let value = f32::from(step) / 255.0;
        close(value, color::to_srgb(color::to_linear(value)), 1e-5);
    }
}
