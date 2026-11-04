//! AMD-049 REQ-SCN-075: the backdrop is lit with the almost-matt reflection model of the
//! amendment instead of the plain diffuse term of REQ-SCN-028, so an albedo of zero is no longer
//! rendered black and a large backdrop takes a little of the light's own tint.
//!
//! The model is checked twice: on the CPU twin (`color::backdrop_linear`), where the amendment's
//! vectors are stated as linear values before encoding, and on the GPU, where the capture is
//! compared with that twin — the two cannot drift apart unnoticed.

// Test helpers fail loudly on a broken fixture, as the C# assertions do.
#![allow(clippy::expect_used, clippy::panic)]

mod common;

use common::{solid, Built, GlFixture, SceneFixture};
use zax_engine3d::palette::Rgb24;
use zax_profiles::enums::ProfileLayer;
use zax_render::color;
use zax_render::numerics::Vector3;
use zax_render::shaders;

/// The four axes these tests vary, as the named-field variant of `shaders::scene`. AMD-055 added
/// two more axes (the voxel grid of REQ-SCN-081 and the pixel shadows of REQ-SCN-082); both are
/// off here, which is what every assertion of this file is about.
#[allow(clippy::fn_params_excessive_bools)] // The four axes, as `shaders::scene` had them.
fn variant(solid: bool, srgb_palette: bool, bend: bool, centroid: bool) -> shaders::SceneVariant {
    shaders::SceneVariant {
        solid,
        srgb_palette,
        bend,
        centroid,
        ..shaders::SceneVariant::FLAT
    }
}

/// Tolerance of AMD-049's automatic vectors: ±0.001 on a linear value.
const LINEAR_TOLERANCE: f32 = 0.001;

fn close(found: f32, expected: f32, what: &str) {
    assert!(
        (found - expected).abs() < LINEAR_TOLERANCE,
        "{what}: {found} instead of {expected}"
    );
}

/// The configuration of AMD-049's two automatic criteria: the front face of the box seen head on,
/// the light coming straight from the eye, white light of intensity 1.
///
/// `N = (0, 0, −1)`, `V = (0, 0, −1)`, `Light.Direction = (0, 0, 1)` so `L = (0, 0, −1)`.
fn head_on(albedo_linear: f32, shadow: f32) -> Vector3 {
    color::backdrop_linear(
        Vector3::splat(albedo_linear),
        Vector3::new(0.0, 0.0, -1.0),
        Vector3::new(0.0, 0.0, -1.0),
        Vector3::new(0.0, 0.0, -1.0),
        Vector3::splat(1.0),
        color::ambient_linear(),
        shadow,
    )
}

/// First automatic criterion of AMD-049: the whole chain of products at normal incidence, and the
/// two albedo values the amendment publishes.
#[test]
fn req_scn_075_the_head_on_vector_of_amd_049() {
    // `nl = nv = nh = lh = 1`, `Fd = 1`, `Vis = 0.25`, `D = 0.3314`, `Spec = 0.2603`, `F = 0.04`,
    // so the colour is `c · 0.96 · 1.1812 + 0.01041` per channel.
    let ambient = color::ambient_linear();
    close(ambient, 0.1812, "the ambient of REQ-SCN-027");

    for albedo in [0.0_f32, 0.2, 0.5, 1.0] {
        let expected = albedo * 0.96 * (ambient + 1.0) + 0.010_412;
        let found = head_on(albedo, 1.0);
        for (channel, value) in [("r", found.x), ("g", found.y), ("b", found.z)] {
            close(
                value,
                expected,
                &format!("albedo {albedo}, channel {channel}"),
            );
        }
    }

    // The value AMD-049 writes out: albedo 0.2 gives 0.2372 out of shadow.
    close(head_on(0.2, 1.0).x, 0.2372, "albedo 0.2, out of shadow");

    // And in shadow (s = 0.2): `c · 0.96 · (0.1812 + 0.2) + 0.2 · 0.01041`, 0.0753 at c = 0.2.
    close(
        head_on(0.2, color::SHADOW_FACTOR).x,
        0.0753,
        "albedo 0.2, in shadow",
    );
}

/// Second automatic criterion: an albedo of zero is **not** black any more — 0.01041 linear, sRGB
/// 26, and 0.00208 / sRGB 7 in shadow. This is the whole point of the amendment (AMD-049 §1).
#[test]
fn req_scn_075_a_zero_albedo_is_no_longer_black() {
    let lit = head_on(0.0, 1.0);
    close(lit.x, 0.010_412, "zero albedo, linear, out of shadow");
    let lit_srgb = color::to_srgb_bytes(lit);
    assert!(
        (i32::from(lit_srgb.r) - 26).abs() <= 2,
        "zero albedo out of shadow is sRGB 26 ± 2, not {}",
        lit_srgb.r
    );
    assert_eq!(lit_srgb.r, lit_srgb.g);
    assert_eq!(lit_srgb.g, lit_srgb.b);

    let shadowed = head_on(0.0, color::SHADOW_FACTOR);
    close(shadowed.x, 0.002_082, "zero albedo, linear, in shadow");
    let shadowed_srgb = color::to_srgb_bytes(shadowed);
    assert!(
        (i32::from(shadowed_srgb.r) - 7).abs() <= 2,
        "zero albedo in shadow is sRGB 7 ± 2, not {}",
        shadowed_srgb.r
    );

    // The same albedo under the diffuse model of REQ-SCN-028 is pure black, which is the
    // difference AMD-049 §1 describes and what made the acceptance criterion of REQ-SCN-048
    // unverifiable (R2's point 49-3).
    let diffuse = color::shade(
        Rgb24::new(0, 0, 0),
        Vector3::new(0.0, 0.0, -1.0),
        Vector3::new(0.0, 0.0, 1.0),
        Rgb24::new(255, 255, 255),
        1.0,
        1.0,
    );
    assert_eq!((0, 0, 0), (diffuse.r, diffuse.g, diffuse.b));
}

/// The default light of REQ-SCN-025 — direction `(1, −1, 1)`, white, intensity 1 — on a black
/// panel seen head on: the values AMD-049 §3 point 2 puts into the amended criterion of
/// REQ-SCN-048, sRGB ≈ 21 out of shadow and ≈ 5 in shadow.
#[test]
fn req_scn_048_the_black_panel_of_the_amended_criterion() {
    let front = Vector3::new(0.0, 0.0, -1.0);
    let to_light = (-Vector3::new(1.0, -1.0, 1.0)).normalize();
    let lit = color::backdrop_linear(
        Vector3::ZERO,
        front,
        front,
        to_light,
        Vector3::splat(1.0),
        color::ambient_linear(),
        1.0,
    );
    let shadowed = color::backdrop_linear(
        Vector3::ZERO,
        front,
        front,
        to_light,
        Vector3::splat(1.0),
        color::ambient_linear(),
        color::SHADOW_FACTOR,
    );

    // R2's own recomputation: nl = 0.5774, Spec × F = 0.007534.
    close(lit.x, 0.007_534, "the black panel, linear, out of shadow");
    let lit_srgb = i32::from(color::to_srgb_bytes(lit).r);
    let shadowed_srgb = i32::from(color::to_srgb_bytes(shadowed).r);
    assert!(
        (lit_srgb - 21).abs() <= 2,
        "out of shadow the panel is sRGB 21 ± 2, not {lit_srgb}"
    );
    assert!(
        (shadowed_srgb - 5).abs() <= 2,
        "in shadow the panel is sRGB 5 ± 2, not {shadowed_srgb}"
    );
    // The point R2 makes in 49-3: with this model the shadow on the panel becomes observable at
    // all, which under the diffuse model it never was.
    assert!(shadowed_srgb < lit_srgb);
}

/// The degenerate case R2 raised as 49-8: `L + V = 0` has no half vector, and REQ-SCN-075 says
/// `H = N` there. The model has to stay finite and sane.
#[test]
fn req_scn_075_the_half_vector_falls_back_to_the_normal() {
    let found = color::backdrop_linear(
        Vector3::splat(0.2),
        Vector3::new(0.0, 0.0, -1.0),
        Vector3::new(0.0, 0.0, -1.0),
        Vector3::new(0.0, 0.0, 1.0),
        Vector3::splat(1.0),
        color::ambient_linear(),
        1.0,
    );
    for value in [found.x, found.y, found.z] {
        assert!(value.is_finite(), "finite, not {value}");
        assert!((0.0..=2.0).contains(&value), "sane, not {value}");
    }

    // L is the far side of the face, so nl = 0: only the ambient part of the diffuse term is
    // left and the specular term vanishes with nl.
    close(
        found.x,
        0.2 * 0.96 * color::ambient_linear(),
        "lit from behind",
    );
}

/// The specular term is **not** tinted by the albedo (AMD-049 §1: "un reflet spéculaire faible,
/// de la couleur de la lumière"), which is the structural difference from REQ-SCN-028 — there,
/// every term is multiplied by the albedo.
#[test]
fn req_scn_075_the_specular_term_carries_the_light_colour_not_the_albedo() {
    let front = Vector3::new(0.0, 0.0, -1.0);
    // A pink light, as the Punch-Out!! profile sets (Light.Color = (1, 0.8, 0.8)).
    let light = Vector3::new(1.0, 0.64, 0.64);
    let found = color::backdrop_linear(
        Vector3::ZERO,
        front,
        front,
        front,
        light,
        color::ambient_linear(),
        1.0,
    );

    // A zero albedo leaves the specular term alone, so what is left is the light's own colour.
    assert!(found.x > found.y, "the red channel leads: {found:?}");
    close(found.x / found.y, 1.0 / 0.64, "the ratio of the light");
}

/// The `[ROM]` criterion of AMD-049, as far as it can be checked without the ROM: the front face
/// of the Punch-Out!! box is sRGB (26, 100, 209) ±3 for every camera pitch from 0° to 45°.
///
/// The profile's script sets `Backdrop.Color = Palette.FrameColor(0)` = NES `0x0078FC`,
/// `Light.Color = (1, 0.8, 0.8)`, and the profile's light direction is `(1, 1, 2)` at intensity 1.
/// R2's point 49-2 is that the published figure cannot be recovered from the amendment's wording
/// alone — the eye position is missing — so what is pinned here is the **pitch sweep**: the value
/// R2 recomputed, and that it holds across the range the criterion names. R0 runs the capture.
#[test]
fn req_scn_075_the_punch_out_review_vector_over_the_pitch_sweep() {
    let albedo = color::rgb_to_linear(Rgb24::new(0x00, 0x78, 0xFC));
    let light = Vector3::new(
        color::to_linear(1.0),
        color::to_linear(0.8),
        color::to_linear(0.8),
    );
    let to_light = (-Vector3::new(1.0, 1.0, 2.0)).normalize();
    let front = Vector3::new(0.0, 0.0, -1.0);

    for pitch in [0.0_f32, 15.0, 30.0, 45.0] {
        // The eye of a camera that has pitched down by `pitch` and still looks at the centre of
        // the face: V is the unit vector from the face towards it.
        let radians = pitch.to_radians();
        let to_eye = Vector3::new(0.0, radians.sin(), -radians.cos());
        let found = color::to_srgb_bytes(color::backdrop_linear(
            albedo,
            front,
            to_eye,
            to_light,
            light,
            color::ambient_linear(),
            1.0,
        ));
        for (channel, value, want) in [
            ("r", i32::from(found.r), 26),
            ("g", i32::from(found.g), 100),
            ("b", i32::from(found.b), 209),
        ] {
            assert!(
                (value - want).abs() <= 3,
                "pitch {pitch}°, channel {channel}: {value} instead of {want} ± 3"
            );
        }
    }
}

// ------------------------------------------------------------ the GPU side

/// A scene whose only object is the backdrop of REQ-SCN-048, seen head on, with the light coming
/// straight from the eye — the configuration of AMD-049's automatic criteria, so the capture can
/// be compared with [`head_on`] directly.
///
/// `lighting` is `render.backdrop_lighting`; `albedo` the NES colour index of `Backdrop.Color`.
fn backdrop_scene(albedo: Rgb24, lighting: bool) -> Built {
    let mut built = SceneFixture::new()
        .background(0x0F)
        .palette(3, 0x30)
        .pattern(solid(3, 0), 4, ProfileLayer::L1)
        .build();
    built.scene.settings.backdrop_lighting = lighting;
    built.scene.settings.shadows = false;
    built.scene.backdrop.enabled = true;
    built.scene.backdrop.color = albedo;
    built.scene.backdrop.receive_shadow = false;
    // The light straight down the view axis of the default setpoint, which looks along +z.
    built.scene.light.direction = Vector3::new(0.0, 0.0, 1.0);
    built.scene.light.color = Rgb24::new(255, 255, 255);
    // Head on: no pitch, no yaw, so the eye is on the axis and looks along +z (REQ-SCN-006).
    built.scene.camera.current_mut().angles = Vector3::ZERO;
    built.scene.camera.snap_to_setpoints();
    built
}

/// The centre of the capture, which the backdrop fills.
fn centre(capture: &common::Capture) -> (i32, i32, i32) {
    capture.at(capture.width / 2, capture.height / 2)
}

/// The GPU draws what the CPU twin says, for the head-on configuration AMD-049 publishes — the
/// check that `shaders/backdrop.frag` and `color::backdrop_linear` cannot drift apart.
#[test]
fn req_scn_075_the_gpu_draws_what_the_cpu_twin_says() {
    let gl = GlFixture::create(320, 240);
    let mut renderer = gl.renderer();

    for index in [0x0F_u8, 0x30, 0x12, 0x16] {
        let albedo = zax_engine3d::palette::nes_color(usize::from(index));
        let built = backdrop_scene(albedo, true);
        let capture = gl.render_built(&mut renderer, &built);

        let expected = color::to_srgb_bytes(color::backdrop_linear(
            color::rgb_to_linear(albedo),
            Vector3::new(0.0, 0.0, -1.0),
            Vector3::new(0.0, 0.0, -1.0),
            Vector3::new(0.0, 0.0, -1.0),
            Vector3::splat(1.0),
            color::ambient_linear(),
            1.0,
        ));
        let (r, g, b) = centre(&capture);
        for (channel, found, want) in [
            ("r", r, i32::from(expected.r)),
            ("g", g, i32::from(expected.g)),
            ("b", b, i32::from(expected.b)),
        ] {
            assert!(
                (found - want).abs() <= 2,
                "NES {index:#04x}, channel {channel}: the GPU drew {found} and the twin says \
                 {want} (sRGB, ±2)"
            );
        }
    }
}

/// The second automatic criterion of AMD-049 on the GPU: a backdrop of albedo zero comes out
/// sRGB 26 and **not black**, which is the visible difference the amendment is about.
#[test]
fn req_scn_075_a_black_backdrop_is_drawn_srgb_twenty_six_and_not_black() {
    let gl = GlFixture::create(320, 240);
    let mut renderer = gl.renderer();

    let built = backdrop_scene(Rgb24::new(0, 0, 0), true);
    let capture = gl.render_built(&mut renderer, &built);
    let (r, g, b) = centre(&capture);
    for (channel, found) in [("r", r), ("g", g), ("b", b)] {
        assert!(
            (found - 26).abs() <= 2,
            "channel {channel}: {found} instead of 26 ± 2"
        );
    }

    // And with the setting off, the diffuse model of REQ-SCN-048 gives black — the behaviour of
    // before the amendment, which the handheld default keeps (AMD-049's priority clause T-1).
    let plain = backdrop_scene(Rgb24::new(0, 0, 0), false);
    let capture = gl.render_built(&mut renderer, &plain);
    assert_eq!(
        (0, 0, 0),
        centre(&capture),
        "the diffuse model draws it black"
    );
}

/// The scope of the setting: it changes the backdrop and **nothing else**. The tiles of the scene
/// are drawn by the same programs either way, so every pixel that is not the backdrop is
/// identical.
#[test]
fn req_scn_075_the_setting_changes_the_backdrop_and_nothing_else() {
    let gl = GlFixture::create(320, 240);
    let mut renderer = gl.renderer();

    // A scene with tiles in front of a **black** backdrop, over a scene background that is not
    // black (REQ-SCN-046 clears with it): so the three regions of the picture — background,
    // backdrop, tiles — are told apart by colour alone.
    let background = zax_engine3d::palette::nes_color(0x11);
    let scene_of = |lighting: bool| -> Built {
        let mut built = SceneFixture::new()
            .background(0x11)
            .palette(3, 0x30)
            .pattern(solid(3, 0), 6, ProfileLayer::L1)
            .rect(solid(3, 0), 14, 14, 4, 4)
            .build();
        built.scene.settings.backdrop_lighting = lighting;
        built.scene.settings.custom_background_color = background;
        built.scene.backdrop.enabled = true;
        built.scene.backdrop.color = Rgb24::new(0, 0, 0);
        built.scene.backdrop.offset = Vector3::new(0.0, 0.0, 26.0);
        built
    };

    let plain = gl.render_built(&mut renderer, &scene_of(false));
    let lit = gl.render_built(&mut renderer, &scene_of(true));

    // Under the diffuse model the black backdrop is pure black, and nothing else in the picture
    // is: the clear colour is NES 0x11 and the tiles are lit white. So "was black" names the
    // backdrop exactly, and every other pixel has to be untouched by the setting.
    let mut tile_or_background = 0;
    let mut backdrop_pixels = 0;
    for y in 0..plain.height {
        for x in 0..plain.width {
            let before = plain.packed(x, y);
            let after = lit.packed(x, y);
            if before == 0 {
                assert_ne!(0, after, "the backdrop at ({x}, {y}) is no longer black");
                backdrop_pixels += 1;
            } else {
                assert_eq!(
                    before, after,
                    "a pixel that is not the backdrop changed at ({x}, {y})"
                );
                tile_or_background += 1;
            }
        }
    }
    assert!(
        backdrop_pixels > 1000,
        "the backdrop shows: {backdrop_pixels}"
    );
    assert!(
        tile_or_background > 1000,
        "the tiles and the background show: {tile_or_background}"
    );
}

/// The requirement of the brief, and the reason REQ-SCN-075 is a program of its own rather than a
/// test on `uColorMode`: **no tile fragment ever sees the model**. The scene sources carry none
/// of it, and the backdrop program carries no atlas lookup.
#[test]
fn req_scn_075_no_tile_fragment_carries_the_model() {
    const BACKDROP_ONLY: [&str; 6] = [
        "ZAX_BACKDROP_ROUGHNESS",
        "ZAX_BACKDROP_REFLECTANCE",
        "zaxBackdropFresnel",
        "zaxPow5",
        "uEye",
        "ZAX_PI",
    ];

    for embedded in [false, true] {
        for solid_variant in [false, true] {
            for srgb in [false, true] {
                for bend in [false, true] {
                    for centroid in [false, true] {
                        let source = shaders::scene(variant(solid_variant, srgb, bend, centroid));
                        for (fragment, stage) in [(false, &source.vertex), (true, &source.fragment)]
                        {
                            let assembled = shaders::assemble(stage, fragment, embedded);
                            for forbidden in BACKDROP_ONLY {
                                assert!(
                                    !common::mentions(&assembled, forbidden),
                                    "{}: the tile shader is given {forbidden}",
                                    source.name
                                );
                            }
                        }
                    }
                }
            }
        }

        // And the other way: the backdrop fragment reads one flat colour, so it holds none of the
        // atlas machinery of REQ-SCN-034 — no palette lookup, no index texture, no texel
        // coordinate and no rejected fragment.
        for bend in [false, true] {
            let source = shaders::backdrop(bend, false);
            let fragment = shaders::assemble(&source.fragment, true, embedded);
            for forbidden in [
                "uIndexTexture",
                "uColorTexture",
                "uLinearPalette",
                "uPaletteTable",
                "uColorMode",
                "discard;",
                "vUv",
            ] {
                assert!(
                    !fragment.contains(forbidden),
                    "{}: the backdrop shader is given {forbidden}",
                    source.name
                );
            }
            assert!(fragment.contains("uEye"), "it reads the eye");
            assert_eq!(bend, fragment.contains("#define ZAX_BEND"));
        }
    }
}

/// The two backdrop variants have names of their own, so the on-disk program cache files them
/// apart from the scene programs (`gl/program_cache.rs`).
#[test]
fn req_scn_075_the_backdrop_variants_have_names_of_their_own() {
    let mut names = vec![
        shaders::backdrop(false, false).name,
        shaders::backdrop(true, false).name,
    ];
    for solid_variant in [false, true] {
        for srgb in [false, true] {
            for bend in [false, true] {
                for centroid in [false, true] {
                    names.push(shaders::scene(variant(solid_variant, srgb, bend, centroid)).name);
                }
            }
        }
    }
    names.push(shaders::shadow(false).name);
    names.push(shaders::shadow(true).name);

    let count = names.len();
    names.sort_unstable();
    names.dedup();
    assert_eq!(count, names.len(), "every program name is distinct");
}

/// The curvature of REQ-SCN-068 reaches the backdrop under the new model too: the backdrop bends
/// with the scene, so its program has the curvature axis and `select_backdrop_variant` picks the
/// bent one when a curvature is set.
#[test]
fn req_scn_075_the_backdrop_model_bends_with_the_scene() {
    let gl = GlFixture::create(320, 240);
    let mut renderer = gl.renderer();

    let mut built = backdrop_scene(Rgb24::new(0, 0, 0), true);
    let flat = gl.render_built(&mut renderer, &built);
    assert!((centre(&flat).0 - 26).abs() <= 2);

    // A curvature of 90° left-right, the setting the criteria of REQ-SCN-068 use. The box is
    // subdivided and bent, so the normals it shades with are the transported ones — the centre
    // of the picture still faces the eye, so it keeps its colour.
    built.scene.bend = zax_engine3d::space_bend::SpaceBend::new(90.0, 0.0, 0.0, 0.0, 1.0);
    let bent = gl.render_built(&mut renderer, &built);
    assert!(
        (centre(&bent).0 - 26).abs() <= 3,
        "the bent backdrop still takes the model: {:?}",
        centre(&bent)
    );
}

/// What the head-on fixture is really worth, stated rather than assumed: with the setpoint angles
/// at zero the eye looks straight along +z from in front of the scene, so `V` is within a degree
/// of `(0, 0, −1)` at the centre of the picture — close enough for the published vectors to be
/// compared to the capture, which is why the GPU comparison above is to ±2 sRGB levels and not
/// exact.
///
/// It is **not** exactly on the axis through `C = (128, 120, 0)` (REQ-E3D-410): the camera aims at
/// the centre of the *visible rectangle*, which the default clip margins put at (124, 124)
/// (REQ-SCN-003). Four units of offset at 190 units of distance is 1.2°.
#[test]
fn the_head_on_fixture_really_looks_down_the_axis() {
    let mut built = backdrop_scene(Rgb24::new(0, 0, 0), true);
    let clipping = built.scene.view_clipping();
    let camera = &mut built.scene.camera;
    camera.snap_to_setpoints();
    let view = camera.build_view(clipping, 4.0 / 3.0);

    assert!(view.eye.z < 0.0, "in front of the scene: {:?}", view.eye);
    let offset = (view.eye.x - 128.0).hypot(view.eye.y - 120.0);
    let angle = offset.atan2(view.eye.z.abs()).to_degrees();
    assert!(
        angle < 2.0,
        "within 2° of the axis through C, not {angle}° (eye {:?})",
        view.eye
    );
}

/// RU-7 step 4e: the model of REQ-SCN-075 is in the `wgpu` backend too, and it is still a
/// **variant** there — no tile fragment carries any of it, which is the requirement of the lot
/// that introduced it and the reason it is a program of its own on the OpenGL side.
///
/// Step 1 of RU-7 measured what its absence cost: with `render.backdrop_lighting` on — the desktop
/// default since this amendment — the two backends agreed on **58.930 %** of the `backdrop`
/// capture. The owner put the port into that lot; this is the static half of the check, and
/// `zax-harness/tests/parity_wgpu_tests.rs` is the measured half.
///
/// Whether the WGSL compiles is not a question this side can answer — naga validates at module
/// creation — which is why the figures live in the harness file.
#[cfg(feature = "wgpu")]
#[test]
fn req_scn_075_the_wgpu_backend_carries_the_model_as_a_variant_of_its_own() {
    const SCENE: &str = include_str!("../src/wgpu_backend/shaders/scene.wgsl");

    // The model itself: its two constants, its Fresnel, its half-vector fallback, the specular
    // term that carries the light colour and not the albedo, and the `1 - F0` the diffuse part
    // gives up.
    assert!(SCENE.contains("ZAX_BACKDROP_ROUGHNESS: f32 = 0.99"));
    assert!(SCENE.contains("ZAX_BACKDROP_REFLECTANCE: f32 = 0.04"));
    assert!(SCENE.contains("fn zax_backdrop_fresnel("));
    assert!(SCENE.contains("fn zax_pow5("));
    assert!(SCENE.contains("select(n, normalize(sum), dot(sum, sum) > 0.0)"));
    assert!(SCENE.contains("frame.light_ambient.xyz * (shadow * spec * f)"));
    assert!(SCENE.contains("(1.0 - ZAX_BACKDROP_REFLECTANCE)"));
    // And the eye the view vector comes from, which the OpenGL path gives to this program alone.
    assert!(SCENE.contains("frame.eye.xyz - in.scene"));

    // It is behind its own marker, so the module a tile draws with carries none of it: expanded
    // without `ZAX_BACKDROP`, not one of the names above survives.
    assert!(SCENE.contains("//#ifdef ZAX_BACKDROP"));
    let plain = zax_render::wgpu_backend::expand_wgsl(SCENE, &[]);
    for forbidden in [
        "ZAX_BACKDROP_ROUGHNESS",
        "ZAX_BACKDROP_REFLECTANCE",
        "zax_backdrop_fresnel",
        "zax_pow5",
        "frame.eye",
        "ZAX_PI",
    ] {
        assert!(
            !common::mentions(&plain, forbidden),
            "the tile variant of the wgpu scene module is given {forbidden}"
        );
    }
    // And the other way round: the backdrop variant reads one flat colour, so it holds none of the
    // atlas machinery of REQ-SCN-034. The names are written with their parenthesis, which is what
    // makes them **calls**: the file's own header mentions the two texel functions in prose, to
    // say where they come from, and that sentence is what a reader wants.
    let backdrop = zax_render::wgpu_backend::expand_wgsl(SCENE, &["ZAX_BACKDROP"]);
    for forbidden in [
        "zax_index_texel(",
        "zax_color_texel(",
        "textureLoad(palette",
        "zax_mod(",
    ] {
        assert!(
            !backdrop.contains(forbidden),
            "the backdrop variant of the wgpu scene module is given {forbidden}"
        );
    }
}
