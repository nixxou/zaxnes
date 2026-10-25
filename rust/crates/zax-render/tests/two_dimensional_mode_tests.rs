//! The 2D mode of SPEC-06 REQ-SCN-055 as AMD-009 (Q-022) settles it: the core's 256 × 240
//! pixel frame mapped on the NES plane of REQ-E3D-410, drawn by the same 3D pipeline (port of
//! `TwoDimensionalModeTests.cs`).

// Test helpers fail loudly on a broken fixture, as the C# assertions do.
#![allow(clippy::expect_used, clippy::panic)]

mod common;

use common::{solid, GlFixture, SceneFixture};
use zax_engine3d::palette::Rgb24;
use zax_profiles::enums::ProfileLayer;
use zax_profiles::values::Vec4F;
use zax_render::scene::{SceneFrame, SceneState};

const WIDTH: usize = 256;
const HEIGHT: usize = 240;

#[test]
fn amd_009_the_pixel_frame_replaces_the_tile_list_with_a_single_quad() {
    let gl = GlFixture::create(320, 300);
    let mut renderer = gl.renderer();

    // A frame built from tile records, so that the draw list is not empty: in 2D mode the
    // renderer must ignore it and draw the picture instead (REQ-E3D-381 no longer applies).
    let mut fixture =
        SceneFixture::new()
            .background(0x0F)
            .palette(1, 0x17)
            .rect(solid(1, 0), 4, 4, 8, 8);
    fixture.two_dimensional_mode = true;
    let mut built = fixture.build();
    built.scene.settings.shadows = false;
    built.scene.two_dimensional_mode = true;
    let picture = picture(0x30);

    let frame = SceneFrame {
        draw_list: Some(&built.draw_list),
        nes_picture: &picture,
        nes_picture_height: 0,
    };
    gl.render(&mut renderer, &built.scene, &frame);

    let statistics = renderer.statistics();
    assert_eq!(1, statistics.opaque_tiles);
    assert_eq!(0, statistics.transparent_tiles);
    assert_eq!(2, statistics.triangles);
    assert!(!statistics.shadow_pass);
}

#[test]
fn amd_009_a_white_picture_comes_out_at_the_lit_srgb_of_section_5_2() {
    let gl = GlFixture::create(320, 300);
    let mut renderer = gl.renderer();

    let picture = picture(0x30);
    let capture = gl.render(&mut renderer, &scene(), &frame(&picture));

    // The quad faces the default camera, so N·L = 0.577 and the factor is 0.7585: NES 0x30
    // (252, 248, 252) comes out at (223, 219, 223), the vector of §5.2.
    let (r, g, b) = capture.at(capture.width / 2, capture.height / 2);
    assert!((223 - r).abs() <= 3, "r {r}");
    assert!((219 - g).abs() <= 3, "g {g}");
    assert!((223 - b).abs() <= 3, "b {b}");
}

#[test]
fn amd_009_a_blue_picture_comes_out_at_the_lit_srgb_of_section_5_2() {
    let gl = GlFixture::create(320, 300);
    let mut renderer = gl.renderer();

    let picture = picture(0x22);
    let capture = gl.render(&mut renderer, &scene(), &frame(&picture));

    let (r, g, b) = capture.at(capture.width / 2, capture.height / 2);
    assert!((91 - r).abs() <= 3, "r {r}");
    assert!((120 - g).abs() <= 3, "g {g}");
    assert!((223 - b).abs() <= 3, "b {b}");
}

#[test]
fn req_e3d_410_the_picture_is_not_turned_upside_down() {
    let gl = GlFixture::create(320, 300);
    let mut renderer = gl.renderer();

    // NES row 0 is red, row 239 is green; row 0 is at the top of the screen.
    let mut picture = picture(0x0F);
    for x in 0..WIDTH {
        for y in 0..40 {
            picture[(y * WIDTH) + x] = 0x16;
            picture[((HEIGHT - 1 - y) * WIDTH) + x] = 0x1A;
        }
    }

    let capture = gl.render(&mut renderer, &scene(), &frame(&picture));

    let middle = capture.width / 2;
    let (top_r, _, _) = capture.at(middle, 30);
    let (bottom_r, bottom_g, _) = capture.at(middle, capture.height - 40);
    assert!(
        top_r > 100,
        "the top of the capture should show the red rows of the picture"
    );
    assert!(
        bottom_g > bottom_r,
        "the bottom of the capture should show the green rows"
    );
}

#[test]
fn req_scn_036_the_picture_is_cut_by_the_visible_rectangle() {
    let gl = GlFixture::create(320, 300);
    let mut renderer = gl.renderer();
    let picture = picture(0x30);

    let mut wide = scene();
    wide.clipping = Vec4F::new(0.1, 8.1, 8.1, 0.1);
    let full = gl
        .render(&mut renderer, &wide, &frame(&picture))
        .column_span(0x00_0000);

    let mut narrow = scene();
    narrow.clipping = Vec4F::new(64.1, 8.1, 64.1, 0.1);
    let cut = gl
        .render(&mut renderer, &narrow, &frame(&picture))
        .column_span(0x00_0000);

    assert!(
        cut.0 > full.0,
        "the left margin should bite into the picture"
    );
    assert!(
        cut.1 < full.1,
        "the right margin should bite into the picture"
    );
}

#[test]
fn amd_009_a_picture_of_the_wrong_size_falls_back_to_the_tile_list() {
    let gl = GlFixture::create(160, 150);
    let mut renderer = gl.renderer();

    let mut built = SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x30)
        .pattern(solid(1, 0), 1, ProfileLayer::L1)
        .rect(solid(1, 0), 4, 4, 8, 8)
        .build();
    built.scene.settings.shadows = false;
    built.scene.two_dimensional_mode = true;

    let short = [0_u8; 16];
    let frame = SceneFrame {
        draw_list: Some(&built.draw_list),
        nes_picture: &short,
        nes_picture_height: 0,
    };
    gl.render(&mut renderer, &built.scene, &frame);

    assert!(renderer.statistics().opaque_tiles > 1);
}

/// The head-on picture (Rust-only): the first row the game shows (row 8, under the default
/// margins) meets the top of the view and row 239 its bottom, where the camera's own view
/// shows neither.
#[test]
fn the_picture_seen_head_on_fills_the_height_of_the_view() {
    let gl = GlFixture::create(320, 300);
    let mut renderer = gl.renderer();
    // White rows at the top, hidden ones included, red ones at the bottom, black between.
    let mut picture = picture(0x0F);
    picture[..12 * WIDTH].fill(0x30);
    picture[(HEIGHT - 4) * WIDTH..].fill(0x16);
    let white = |(r, g, b): (i32, i32, i32)| r > 150 && g > 150 && b > 150;
    let red = |(r, g, b): (i32, i32, i32)| r > 100 && g < 80 && b < 80;

    let mut head_on = scene();
    head_on.picture_head_on = true;
    let capture = gl.render(&mut renderer, &head_on, &frame(&picture));
    let (middle, last) = (capture.width / 2, capture.height - 1);
    assert!(white(capture.at(middle, 0)), "{:?}", capture.at(middle, 0));
    assert!(
        red(capture.at(middle, last)),
        "{:?}",
        capture.at(middle, last)
    );

    head_on.picture_head_on = false;
    let capture = gl.render(&mut renderer, &head_on, &frame(&picture));
    assert!(!white(capture.at(middle, 0)), "{:?}", capture.at(middle, 0));
    assert!(
        !red(capture.at(middle, last)),
        "{:?}",
        capture.at(middle, last)
    );
}

fn scene() -> SceneState {
    let mut scene = SceneState {
        game_running: true,
        two_dimensional_mode: true,
        nes_background_color: Rgb24::default(),
        ..SceneState::default()
    };
    scene.settings.shadows = false;
    scene.camera.snap_to_setpoints();
    scene
}

fn frame(picture: &[u8]) -> SceneFrame<'_> {
    SceneFrame {
        draw_list: None,
        nes_picture: picture,
        nes_picture_height: 0,
    }
}

fn picture(nes_color: u8) -> Vec<u8> {
    vec![nes_color; WIDTH * HEIGHT]
}
