//! The free assets of AMD-024: the procedural skyboxes of REQ-PUB-003, which must render with
//! **no asset source at all** (port of `ProceduralAssetsTests.cs`). Every renderer here is
//! built without assets: nothing can read a file, so what the captures show is what the code
//! produces.

// Test helpers fail loudly on a broken fixture, as the C# assertions do.
#![allow(clippy::expect_used, clippy::panic)]

mod common;

use common::{Capture, GlFixture, SceneFixture};
use zax_engine3d::draw_list::FrameDrawList;
use zax_engine3d::palette::{nes_color, Rgb24};
use zax_render::camera::CameraMode;
use zax_render::color;
use zax_render::numerics::Vector3;
use zax_render::scene::{SceneFrame, SceneState, SkyboxKind};
use zax_render::sky;
use zax_render::SceneRenderer;

/// NES 0x22, the sky blue of the reference views.
fn blue() -> Rgb24 {
    nes_color(0x22)
}

/// NES 0x0F, black: the background that lights the stars (REQ-SCN-050).
fn black() -> Rgb24 {
    nes_color(0x0F)
}

#[test] // REQ-PUB-003, REQ-SCN-050
fn req_pub_003_the_gradient_sky_renders_with_no_image_file() {
    let gl = GlFixture::create(96, 96);
    let mut renderer = gl.renderer();

    // The gradient formula of REQ-SCN-050 is untouched by the change of field source: the
    // horizon is M, the zenith 1.4 M and the nadir 0.6 M, and a bright M shows no star.
    assert_eq!(srgb(blue()), centre(&gl, &mut renderer, 0.0));
    assert_eq!(scaled(blue(), 1.4), centre(&gl, &mut renderer, -90.0));
    assert_eq!(scaled(blue(), 0.6), centre(&gl, &mut renderer, 90.0));
}

#[test] // REQ-PUB-003, REQ-SCN-050
fn req_pub_003_the_procedural_stars_only_appear_over_a_dark_background() {
    let gl = GlFixture::create(192, 192);
    let mut renderer = gl.renderer();

    // Looking at the zenith, where the |0.2 y| factor is at its largest.
    let dark = render(
        &gl,
        &mut renderer,
        SkyboxKind::Gradient,
        black(),
        -90.0,
        None,
        0.0,
    );
    let lit = dark.count_other(0x00_0000);
    assert!(lit > 0, "a dark sky should carry stars");
    assert!(
        lit < usize::try_from(dark.width * dark.height / 2).expect("size"),
        "the star field should stay sparse"
    );

    // A bright background turns them off entirely (grey level above 0.4): the image is then the
    // smooth gradient alone, which the twinkle of REQ-SCN-050 cannot touch.
    assert!(!sky::stars_visible(blue()));
    let bright = render(
        &gl,
        &mut renderer,
        SkyboxKind::Gradient,
        blue(),
        -90.0,
        None,
        0.0,
    );
    let bright_later = render(
        &gl,
        &mut renderer,
        SkyboxKind::Gradient,
        blue(),
        -90.0,
        None,
        20.0,
    );
    assert_eq!(bright.pixels, bright_later.pixels);
}

#[test] // REQ-PUB-003, REQ-SCN-059
fn req_pub_003_the_stars_twinkle_with_real_time() {
    let gl = GlFixture::create(128, 128);
    let mut renderer = gl.renderer();

    let first = render(
        &gl,
        &mut renderer,
        SkyboxKind::Gradient,
        black(),
        -90.0,
        None,
        0.0,
    );
    let later = render(
        &gl,
        &mut renderer,
        SkyboxKind::Gradient,
        black(),
        -90.0,
        None,
        20.0,
    );

    // 20 s at 3°/s turns the twinkle mask by 60°, which changes the brightness of the stars
    // without moving them.
    assert_ne!(first.pixels, later.pixels);
}

#[test] // REQ-PUB-003, REQ-SCN-051
fn req_pub_003_the_fantasy_sky_has_structure_and_follows_the_palette() {
    let gl = GlFixture::create(128, 128);
    let mut renderer = gl.renderer();

    let nebula = render(
        &gl,
        &mut renderer,
        SkyboxKind::Fantasy,
        blue(),
        0.0,
        Some(0x21),
        0.0,
    );
    assert!(
        nebula.distinct_colors().len() > 8,
        "the nebula should not be a flat colour"
    );

    // REQ-SCN-052: the sky follows the colour of the displayed frame, so another background
    // gives another image.
    let other = render(
        &gl,
        &mut renderer,
        SkyboxKind::Fantasy,
        nes_color(0x16),
        0.0,
        Some(0x21),
        0.0,
    );
    assert_ne!(nebula.pixels, other.pixels);
}

#[test] // REQ-PUB-003 (Q-064)
fn req_pub_003_the_sun_the_moon_and_the_clouds_are_off_by_default_and_change_the_image() {
    let gl = GlFixture::create(128, 128);
    let mut renderer = gl.renderer();

    let mut scene = sky_scene(blue(), 0.0);

    // REQ-SCN-050: by default the sun, the moon and the clouds are disabled, so the sky is the
    // pure gradient — one colour per row of the capture.
    assert!(!scene.settings.sky_bodies.any_visible());
    let plain = gl.render_scene(&mut renderer, &scene);
    let centre_luma = |capture: &Capture| capture.luma(capture.width / 2, capture.height / 2);

    // The sun, placed straight ahead, is a bright disc the gradient cannot produce.
    let bodies = &mut scene.settings.sky_bodies;
    bodies.set_sun_direction(Vector3::new(0.0, 0.0, 1.0));
    bodies.set_sun_radius_degrees(8.0);
    bodies.set_sun_gain(1.0);
    let sun = gl.render_scene(&mut renderer, &scene);
    assert_ne!(plain.pixels, sun.pixels);
    assert!(
        centre_luma(&sun) > centre_luma(&plain) + 0.2,
        "the sun disc should brighten the centre of the view"
    );

    // The moon, on the other side, is another disc.
    let bodies = &mut scene.settings.sky_bodies;
    bodies.reset();
    bodies.set_moon_direction(Vector3::new(0.0, 0.0, 1.0));
    bodies.set_moon_radius_degrees(8.0);
    bodies.set_moon_gain(1.0);
    let moon = gl.render_scene(&mut renderer, &scene);
    assert_ne!(plain.pixels, moon.pixels);

    // The clouds are a band around the horizon, which is where this view looks.
    let bodies = &mut scene.settings.sky_bodies;
    bodies.reset();
    bodies.set_cloud_gain(1.0);
    bodies.set_cloud_coverage(0.9);
    let clouds = gl.render_scene(&mut renderer, &scene);
    assert_ne!(plain.pixels, clouds.pixels);

    // Back to the defaults, the image is the one of the start: nothing is stateful.
    scene.settings.sky_bodies.reset();
    assert_eq!(plain.pixels, gl.render_scene(&mut renderer, &scene).pixels);
}

fn centre(gl: &GlFixture, renderer: &mut SceneRenderer, rx: f32) -> (i32, i32, i32) {
    let capture = render(gl, renderer, SkyboxKind::Gradient, blue(), rx, None, 0.0);
    capture.at(capture.width / 2, capture.height / 2)
}

fn render(
    gl: &GlFixture,
    renderer: &mut SceneRenderer,
    kind: SkyboxKind,
    background: Rgb24,
    rx: f32,
    palette: Option<u8>,
    real_time: f32,
) -> Capture {
    let mut scene = sky_scene(background, rx);
    scene.settings.skybox = kind;
    scene.real_time = real_time;
    let list = palette.map(palette_frame);
    let frame = SceneFrame {
        draw_list: list.as_ref(),
        nes_picture: &[],
        nes_picture_height: 0,
    };
    gl.render(renderer, &scene, &frame)
}

fn palette_frame(entry: u8) -> FrameDrawList {
    SceneFixture::new()
        .palette(1, entry)
        .palette(2, entry)
        .palette(3, entry)
        .build()
        .draw_list
}

/// An empty scene looking along `rx` with the Gradient sky. FPV is used so that the camera
/// really points along `q·(0,0,1)` (REQ-SCN-010).
fn sky_scene(background: Rgb24, rx: f32) -> SceneState {
    let mut scene = SceneState {
        game_running: true,
        nes_background_color: background,
        ..SceneState::default()
    };
    scene.settings.skybox = SkyboxKind::Gradient;
    scene.camera.set_mode(CameraMode::Fpv);
    scene.camera.current_mut().angles = Vector3::new(rx, 0.0, 0.0);
    scene.camera.current_mut().set_distance(0.0);
    scene.camera.snap_to_setpoints();
    scene
}

fn srgb(color: Rgb24) -> (i32, i32, i32) {
    (i32::from(color.r), i32::from(color.g), i32::from(color.b))
}

#[allow(clippy::cast_possible_truncation)] // Rounded and clamped to 0..255 first.
fn scaled(color: Rgb24, factor: f32) -> (i32, i32, i32) {
    let channel = |value: u8| {
        let srgb = f32::from(value) / 255.0 * factor;
        let linear = color::to_linear(srgb);
        (color::to_srgb(linear.clamp(0.0, 1.0)) * 255.0)
            .round_ties_even()
            .clamp(0.0, 255.0) as i32
    };

    (channel(color.r), channel(color.g), channel(color.b))
}
