//! The two skyboxes of SPEC-06 §3.14: Gradient (REQ-SCN-050) and Fantasy (REQ-SCN-051) (port
//! of `SkyboxTests.cs`).
//!
//! The acceptance criterion of REQ-SCN-051 asks for uniform test cubemaps injected in place of
//! the ones of `assets-neutral/`; [`UniformAssets`] is that injection, which the renderer
//! accepts because it takes its assets from the host (REQ-SCN-057).

mod common;

use std::sync::Arc;

use common::{solid, Capture, GlFixture, SceneFixture};
use zax_engine3d::draw_list::FrameDrawList;
use zax_engine3d::geometry::tile_mesh::TileMesh;
use zax_engine3d::palette::{nes_color, Rgb24};
use zax_profiles::enums::ProfileLayer;
use zax_render::assets::{CubeFace, ImageData, SceneAssets};
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

#[test]
fn req_scn_050_the_stars_only_show_on_a_dark_background() {
    // Grey level 0.299 R + 0.587 G + 0.114 B on the sRGB values, threshold 0.4.
    assert!(!sky::stars_visible(blue()));
    assert!(sky::stars_visible(nes_color(0x0F)));
    assert!((sky::STAR_GREY_THRESHOLD - 0.4).abs() < 5e-6);
    assert!((0.4..=1.0).contains(&sky::grey(blue())));
}

#[test]
fn req_scn_050_the_horizon_is_the_background_colour_and_the_poles_scale_it() {
    let gl = GlFixture::create(96, 96);
    let mut renderer = gl.renderer_with(UniformAssets::black());

    // Horizon: a direction with y = 0 shows M itself.
    assert_eq!(srgb(blue()), centre(&gl, &mut renderer, 0.0, blue()));

    // Zenith: srgb(lin(1.4 M)) component by component, saturated at display.
    assert_eq!(scaled(blue(), 1.4), centre(&gl, &mut renderer, -90.0, blue()));

    // Nadir: srgb(lin(0.6 M)).
    assert_eq!(scaled(blue(), 0.6), centre(&gl, &mut renderer, 90.0, blue()));
}

#[test]
fn req_scn_050_a_dark_background_lights_the_stars_of_the_cubemap() {
    let gl = GlFixture::create(96, 96);

    // A white Stars_LP_01 makes s = 1 everywhere, so k = min(|0.2 y|, 1) and the zenith, where
    // y = 1, mixes 20 % of the slightly grey white E.
    let mut stars = gl.renderer_with(UniformAssets::with("Stars_LP_01", [255, 255, 255]));
    let zenith = centre(&gl, &mut stars, -90.0, nes_color(0x0F));

    // AMD-019 (Q-045): the "×2" factor is the one the sRGB curve of SPEC-06 §2 gives.
    let e = color::to_linear(0.5) * sky::SRGB_DOUBLE;
    let expected = (color::to_srgb(0.2 * e) * 255.0).round_ties_even() as i32;
    assert!((expected - zenith.0).abs() <= 2, "r {} vs {expected}", zenith.0);
    assert!((expected - zenith.1).abs() <= 2, "g {} vs {expected}", zenith.1);
    assert!((expected - zenith.2).abs() <= 2, "b {} vs {expected}", zenith.2);

    // The same sky without the cubemap stays pure black at the zenith.
    let mut none = gl.renderer_with(UniformAssets::black());
    assert_eq!((0, 0, 0), centre(&gl, &mut none, -90.0, nes_color(0x0F)));
}

#[test]
fn req_scn_051_a_all_cubemaps_null_shows_f0_in_every_direction() {
    let gl = GlFixture::create(64, 64);
    let mut renderer = gl.renderer_with(UniformAssets::black());

    let capture = render(&gl, &mut renderer, SkyboxKind::Fantasy, blue(), 0.0, None);
    assert_eq!(
        vec![blue().packed()],
        capture.distinct_colors().into_iter().collect::<Vec<_>>()
    );
}

#[test]
fn req_scn_051_b_a_green_diffusion_of_one_half_multiplies_f0_by_2_2975() {
    let gl = GlFixture::create(64, 64);
    let mut renderer = gl.renderer_with(UniformAssets::with("NebulaDiffusion", [0, 128, 0]));

    // F0 = grey sRGB 0.2: only the ambient halo contributes, with the weight 1.
    let grey = Rgb24::new(51, 51, 51);
    let capture = render(&gl, &mut renderer, SkyboxKind::Fantasy, grey, 0.0, None);

    let expected =
        (color::to_srgb(2.2975 * color::to_linear(51.0 / 255.0)) * 255.0).round_ties_even() as i32;
    let (r, g, b) = capture.at(32, 32);
    assert!((expected - r).abs() <= 2, "r {r} vs {expected}");
    assert!((expected - g).abs() <= 2, "g {g} vs {expected}");
    assert!((expected - b).abs() <= 2, "b {b} vs {expected}");
}

#[test]
fn req_scn_051_c_a_light_background_takes_nothing_from_the_starfield() {
    let gl = GlFixture::create(64, 64);

    // A white Starfield would otherwise push the whole sky towards the star tint F1 / 2; with
    // gamma > 0.4 that tint is transparent, so the sky stays F0.
    let mut renderer = gl.renderer_with(UniformAssets::with("Starfield", [255, 255, 255]));
    let light = Rgb24::new(220, 220, 220);
    let capture = render(&gl, &mut renderer, SkyboxKind::Fantasy, light, 0.0, Some(0x30));
    assert_eq!(
        vec![light.packed()],
        capture.distinct_colors().into_iter().collect::<Vec<_>>()
    );

    // The same starfield over a dark background does take the star tint.
    let dark = render(
        &gl,
        &mut renderer,
        SkyboxKind::Fantasy,
        nes_color(0x0F),
        0.0,
        Some(0x30),
    );
    assert!(!dark.distinct_colors().contains(&nes_color(0x0F).packed()));
}

#[test]
fn req_scn_049_the_scene_is_drawn_in_front_of_the_skybox() {
    let gl = GlFixture::create(128, 128);
    let mut renderer = gl.renderer_with(UniformAssets::black());

    let mut built = SceneFixture::new()
        .background(0x0F)
        .pattern(solid(1, 0), 1, ProfileLayer::L1)
        .rect(solid(1, 0), 10, 10, 8, 8)
        .palette(1, 0x30)
        .build();
    built.scene.settings.skybox = SkyboxKind::Gradient;
    built.scene.settings.shadows = false;

    let capture = gl.render_built(&mut renderer, &built);

    // The tiles are white and lit; the sky around them is the black background colour.
    assert!(capture.count(0x000000) > 0);
    assert!(capture.count_other(0x000000) > 0);
    assert!(renderer.statistics().skybox_pass);
}

#[test]
fn req_scn_049_default_and_color_draw_no_skybox_at_all() {
    let gl = GlFixture::create(32, 32);
    let mut renderer = gl.renderer_with(UniformAssets::black());

    let mut scene = SceneState {
        game_running: true,
        nes_background_color: blue(),
        ..SceneState::default()
    };
    scene.camera.snap_to_setpoints();

    scene.settings.skybox = SkyboxKind::Default;
    gl.render_scene(&mut renderer, &scene);
    assert!(!renderer.statistics().skybox_pass);

    scene.settings.skybox = SkyboxKind::Gradient;
    gl.render_scene(&mut renderer, &scene);
    assert!(renderer.statistics().skybox_pass);
}

fn centre(gl: &GlFixture, renderer: &mut SceneRenderer, rx: f32, background: Rgb24) -> (i32, i32, i32) {
    let capture = render(gl, renderer, SkyboxKind::Gradient, background, rx, None);
    capture.at(capture.width / 2, capture.height / 2)
}

/// A frame that only carries a palette whose entries 1 to 3 are `entry`: the Fantasy sky needs
/// F1 to F3, which come from the frame palette of the draw list.
fn palette_frame(entry: u8) -> FrameDrawList {
    SceneFixture::new()
        .palette(1, entry)
        .palette(2, entry)
        .palette(3, entry)
        .build()
        .draw_list
}

/// Renders an empty scene with the given skybox, looking along `rx`. FPV is used so that the
/// camera really points along `q·(0,0,1)` (REQ-SCN-010) instead of being turned back towards
/// `P*`.
fn render(
    gl: &GlFixture,
    renderer: &mut SceneRenderer,
    kind: SkyboxKind,
    background: Rgb24,
    rx: f32,
    palette: Option<u8>,
) -> Capture {
    let mut scene = SceneState {
        game_running: true,
        nes_background_color: background,
        ..SceneState::default()
    };
    scene.settings.skybox = kind;
    scene.camera.set_mode(CameraMode::Fpv);
    scene.camera.current_mut().angles = Vector3::new(rx, 0.0, 0.0);
    scene.camera.current_mut().set_distance(0.0);
    scene.camera.snap_to_setpoints();

    let list = palette.map(palette_frame);
    let frame = SceneFrame {
        draw_list: list.as_ref(),
        nes_picture: &[],
    };
    gl.render(renderer, &scene, &frame)
}

fn srgb(color: Rgb24) -> (i32, i32, i32) {
    (i32::from(color.r), i32::from(color.g), i32::from(color.b))
}

fn scaled(color: Rgb24, factor: f32) -> (i32, i32, i32) {
    // REQ-SCN-050: the factor is applied to the sRGB value without saturating it, the result is
    // linearised and re-encoded, and only then saturated for display.
    let channel = |value: u8| {
        let srgb = f32::from(value) / 255.0 * factor;
        let linear = color::to_linear(srgb);
        (color::to_srgb(linear.clamp(0.0, 1.0)) * 255.0)
            .round_ties_even()
            .clamp(0.0, 255.0) as i32
    };

    (channel(color.r), channel(color.g), channel(color.b))
}

/// Cubemaps of one constant colour, the test injection of REQ-SCN-051. Every face of every
/// cubemap is a single texel; the cubemaps not named are black.
struct UniformAssets {
    name: &'static str,
    rgb: [u8; 3],
}

impl UniformAssets {
    /// Everything black: the "no cubemap" case of both skyboxes.
    fn black() -> Box<dyn SceneAssets> {
        Self::with("", [0, 0, 0])
    }

    /// The cubemap `name` uniformly `rgb`, every other one black.
    fn with(name: &'static str, rgb: [u8; 3]) -> Box<dyn SceneAssets> {
        Box::new(Self { name, rgb })
    }
}

impl SceneAssets for UniformAssets {
    fn cubemap_face(&mut self, cubemap: &str, _face: CubeFace) -> Option<ImageData> {
        let [r, g, b] = if cubemap == self.name {
            self.rgb
        } else {
            [0, 0, 0]
        };
        Some(ImageData::single(r, g, b, 255))
    }

    fn texture(&mut self, _name: &str) -> Option<Arc<ImageData>> {
        None
    }

    fn mesh(&mut self, _name: &str) -> Option<Arc<TileMesh>> {
        None
    }
}
