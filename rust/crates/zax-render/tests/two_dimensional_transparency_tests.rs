//! AMD-050 REQ-SCN-076: in the 2D mode, a pixel whose opacity bit of REQ-CORE-049 is 0 — the
//! universal backdrop for want of an opaque pixel — is **not drawn**, and the scene shows
//! through. A pixel that is opaque stays drawn even when its palette colour is the universal
//! backdrop's, which is what the index image alone cannot express.
//!
//! The pixels are synthesised here rather than emulated: what is under test is the rendering of
//! the rectangle, and the core's side of the mask is checked in `zax-nes`
//! (`opacity_mask_tests.rs`, vector V11 of SPEC-02 §5.3).
//!
//! Conditioning of every criterion, which the amendment spells out: the default settings force
//! `scene.use_nes_bg_color` true at each profile load, and the scene background then equals the
//! NES backdrop, so the transparency is **not observable**. These tests therefore turn it off
//! and put a magenta custom colour behind the rectangle.

// Test helpers fail loudly on a broken fixture, as the C# assertions do.
#![allow(clippy::expect_used, clippy::panic)]

mod common;

use common::{Capture, GlFixture};
use zax_engine3d::palette::{nes_color, Rgb24};
use zax_render::camera::{to_gl, CameraView};
use zax_render::interop::{OPAQUE_PIXEL, PIXEL_COLOR};
use zax_render::numerics::Vector3;
use zax_render::scene::{SceneFrame, SceneState};

const WIDTH: usize = 256;
const HEIGHT: usize = 240;

/// The magenta the criteria put behind the rectangle.
const MAGENTA: Rgb24 = Rgb24::new(255, 0, 255);

/// The universal backdrop of the synthetic frames, NES `$01` (a dark blue), chosen so that it is
/// clearly neither magenta nor the colour of an opaque pixel.
const BACKDROP: u8 = 0x01;

/// A 2D-mode scene with the conditioning of the criteria: the custom background colour shown
/// instead of the NES one.
fn scene() -> SceneState {
    let mut scene = SceneState {
        game_running: true,
        two_dimensional_mode: true,
        // The criteria's conditioning: without this the scene background equals the NES backdrop
        // and nothing of the transparency can be seen.
        use_nes_background_color: false,
        ..SceneState::default()
    };
    scene.settings.shadows = false;
    scene.settings.custom_background_color = MAGENTA;
    scene
}

/// A picture of `256 × 240` bytes, every pixel the universal backdrop and **transparent**.
fn transparent_picture() -> Vec<u8> {
    vec![BACKDROP; WIDTH * HEIGHT]
}

/// The byte of an **opaque** pixel of NES colour `index`.
fn opaque(index: u8) -> u8 {
    (index & PIXEL_COLOR) | OPAQUE_PIXEL
}

fn frame(picture: &[u8]) -> SceneFrame<'_> {
    SceneFrame {
        draw_list: None,
        nes_picture: picture,
        nes_picture_height: 0,
    }
}

/// Whether `(r, g, b)` is `colour` to within 2 sRGB levels per channel.
fn close_to(got: (i32, i32, i32), colour: Rgb24) -> bool {
    let (r, g, b) = got;
    (r - i32::from(colour.r)).abs() <= 2
        && (g - i32::from(colour.g)).abs() <= 2
        && (b - i32::from(colour.b)).abs() <= 2
}

/// The colour of the capture where the NES pixel `(px, py)` of the rectangle lands, projected
/// through the camera the frame was rendered with.
///
/// The rectangle does **not** fill the view — the camera frames the visible rectangle of
/// REQ-SCN-036, and the projection is perspective — so the pixel's scene centre is put through
/// the very view-projection the renderer used (`SceneRenderer::last_view`), as the camera and
/// Zapper tests do.
fn at_nes_pixel(capture: &Capture, view: &CameraView, px: usize, py: usize) -> (i32, i32, i32) {
    // REQ-E3D-410: the NES pixel (px, py) occupies [px, px + 1] × [247 − py, 248 − py].
    #[allow(clippy::cast_precision_loss)] // 0 to 255.
    let centre = Vector3::new(px as f32 + 0.5, 247.5 - py as f32, 0.0);
    let gl = to_gl(centre);
    let m = view.view_projection();
    let clip_x = gl.x * m.m11 + gl.y * m.m21 + gl.z * m.m31 + m.m41;
    let clip_y = gl.x * m.m12 + gl.y * m.m22 + gl.z * m.m32 + m.m42;
    let clip_w = gl.x * m.m14 + gl.y * m.m24 + gl.z * m.m34 + m.m44;
    let (ndc_x, ndc_y) = (clip_x / clip_w, clip_y / clip_w);
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)] // capture sizes.
    let x = (ndc_x.midpoint(1.0) * capture.width as f32) as i32;
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)] // capture sizes.
    let y = ((1.0 - ndc_y) * 0.5 * capture.height as f32) as i32;
    capture.at(
        x.clamp(0, capture.width - 1),
        y.clamp(0, capture.height - 1),
    )
}

/// REQ-SCN-076, first criterion: a tile with two pixels of value 0 and two of value 3, the
/// palette entry 3 being equal to the entry 0 — so the **image** shows the same colour for all
/// four. The two transparent ones give the scene background (magenta); the two opaque ones are
/// drawn with the universal backdrop's colour.
#[test]
fn req_scn_076_an_opaque_pixel_of_the_backdrop_colour_is_drawn_and_a_transparent_one_is_not() {
    let gl = GlFixture::create(512, 480);
    let mut renderer = gl.renderer();

    let mut picture = transparent_picture();
    // Four pixels of one tile, in the middle of the picture where the capture is reliable: two
    // transparent, two opaque, all four of the same NES colour.
    let put = |picture: &mut Vec<u8>, px: usize, py: usize, byte: u8| {
        picture[py * WIDTH + px] = byte;
    };
    put(&mut picture, 120, 100, BACKDROP);
    put(&mut picture, 121, 100, BACKDROP);
    put(&mut picture, 120, 101, opaque(BACKDROP));
    put(&mut picture, 121, 101, opaque(BACKDROP));

    let capture = gl.render(&mut renderer, &scene(), &frame(&picture));
    let view = *renderer.last_view();

    // The two transparent pixels: the scene's own background shows through.
    for px in [120, 121] {
        let got = at_nes_pixel(&capture, &view, px, 100);
        assert!(
            close_to(got, MAGENTA),
            "the transparent pixel ({px}, 100) should show the scene background, got {got:?}"
        );
    }

    // The two opaque ones: drawn, with the universal backdrop's colour — lit, so not magenta and
    // not black either.
    let backdrop = nes_color(usize::from(BACKDROP));
    for px in [120, 121] {
        let got = at_nes_pixel(&capture, &view, px, 101);
        assert!(
            !close_to(got, MAGENTA),
            "the opaque pixel ({px}, 101) is drawn, not the background: {got:?}"
        );
        // Lit as a tile (REQ-SCN-028), so darker than the raw colour but of the same hue: blue
        // dominates, as the raw `$01` does.
        assert!(
            got.2 > got.0 && got.2 > got.1,
            "the opaque pixel keeps the backdrop's hue (raw {backdrop:?}), got {got:?}"
        );
    }
}

/// REQ-SCN-076, second criterion, first half: with both PPUMASK layers off, every pixel is
/// transparent (V11's fourth case) and the capture is the scene background over the whole
/// rectangle.
#[test]
fn req_scn_076_a_wholly_transparent_picture_shows_only_the_scene() {
    let gl = GlFixture::create(512, 480);
    let mut renderer = gl.renderer();

    let picture = transparent_picture();
    let capture = gl.render(&mut renderer, &scene(), &frame(&picture));
    let view = *renderer.last_view();

    for (px, py) in [(40, 60), (128, 120), (200, 180), (10, 230)] {
        let got = at_nes_pixel(&capture, &view, px, py);
        assert!(
            close_to(got, MAGENTA),
            "({px}, {py}) should be the scene background, got {got:?}"
        );
    }
}

/// REQ-SCN-076, second criterion, second half, with the conditioning Q-085 settles: clip margins
/// `(0.1, 8.1, 0.1, 0.1)` so that the right columns are inside the visible rectangle, and the
/// rows 8 and 239 for the edges. The eight **left** columns are transparent and the rest of the
/// rectangle is unchanged — the right columns and the edges included.
#[test]
fn req_scn_076_only_the_eight_left_columns_go_with_the_ppumask_bit() {
    let gl = GlFixture::create(512, 480);
    let mut renderer = gl.renderer();
    let mut scene = scene();
    // Q-085: with the default margins the NES columns 248 to 255 are outside R anyway, so the
    // criterion is only observable with the right margin opened up.
    scene.clipping = zax_profiles::values::Vec4F::new(0.1, 8.1, 0.1, 0.1);

    // Everything opaque, then the left eight columns of every row made transparent, as the
    // left-column mask of PPUMASK does.
    let mut masked = vec![opaque(0x30); WIDTH * HEIGHT];
    for row in 0..HEIGHT {
        for column in 0..8 {
            masked[row * WIDTH + column] = BACKDROP;
        }
    }
    let whole = vec![opaque(0x30); WIDTH * HEIGHT];

    let with_mask = gl.render(&mut renderer, &scene, &frame(&masked));
    let view = *renderer.last_view();
    let without = gl.render(&mut renderer, &scene, &frame(&whole));

    // The eight left columns: the scene shows through.
    for px in 0..8 {
        let got = at_nes_pixel(&with_mask, &view, px, 120);
        assert!(
            close_to(got, MAGENTA),
            "the masked column {px} should be transparent, got {got:?}"
        );
    }

    // The right columns and the top and bottom rows R keeps: drawn, and **unchanged** against
    // the same capture without the masking.
    for (px, py) in [(248, 120), (255, 120), (128, 8), (128, 239), (8, 120)] {
        let masked_pixel = at_nes_pixel(&with_mask, &view, px, py);
        let whole_pixel = at_nes_pixel(&without, &view, px, py);
        assert_eq!(
            whole_pixel, masked_pixel,
            "({px}, {py}) is outside the mask and must be untouched"
        );
        assert!(
            !close_to(masked_pixel, MAGENTA),
            "({px}, {py}) is drawn, got {masked_pixel:?}"
        );
    }
}

/// REQ-SCN-076, third criterion: the result does not depend on `video.anti_alias`. The same
/// scene rendered at 0, 1 and 2 gives the **same set of drawn pixels**, and no pixel on the edge
/// of a transparent run is partly transparent — the rejection is per fragment, so a pixel is
/// either the scene's background or the rectangle's colour, never a blend of the two.
#[test]
fn req_scn_076_the_drawn_pixels_do_not_depend_on_the_anti_aliasing() {
    let gl = GlFixture::create(512, 480);
    let mut renderer = gl.renderer();

    // A chequer of 8 × 8 blocks, so that every block edge inside the rectangle is a boundary
    // between a drawn run and a transparent one.
    let mut picture = transparent_picture();
    for row in 0..HEIGHT {
        for column in 0..WIDTH {
            if ((row / 8) + (column / 8)) % 2 == 0 {
                picture[row * WIDTH + column] = opaque(0x30);
            }
        }
    }

    let drawn_sets: Vec<Vec<bool>> = [0, 1, 2]
        .into_iter()
        .map(|level| {
            let mut scene = scene();
            // AMD-032 §2: 0 off, 1 is MSAA 4x, 2 is MSAA 8x.
            scene.settings.set_anti_alias_samples(match level {
                0 => 1,
                1 => 4,
                _ => 8,
            });
            let capture = gl.render(&mut renderer, &scene, &frame(&picture));
            let view = *renderer.last_view();
            // Sampled at the centre of each NES pixel of a band well inside the rectangle, so
            // the rectangle's own border (which the anti-aliasing does blend) is left out.
            (64..192)
                .flat_map(|py| (64..192).map(move |px| (px, py)))
                .map(|(px, py)| !close_to(at_nes_pixel(&capture, &view, px, py), MAGENTA))
                .collect()
        })
        .collect();

    assert!(
        drawn_sets[0].iter().any(|drawn| *drawn) && drawn_sets[0].iter().any(|drawn| !*drawn),
        "the chequer really has both kinds of pixel"
    );
    assert_eq!(
        drawn_sets[0], drawn_sets[1],
        "MSAA 4x draws the same pixels as no anti-aliasing"
    );
    assert_eq!(drawn_sets[0], drawn_sets[2], "and so does MSAA 8x");
}

/// REQ-SCN-076 read with REQ-SCN-074: the transparency holds whether the rectangle bends or not.
#[test]
fn req_scn_076_the_transparency_holds_bent_as_well_as_flat() {
    let gl = GlFixture::create(512, 480);
    let mut renderer = gl.renderer();

    let picture = transparent_picture();
    for bend in [
        zax_engine3d::space_bend::SpaceBend::NONE,
        zax_engine3d::space_bend::SpaceBend::new(90.0, 0.0, 0.0, 0.0, 1.0),
    ] {
        let mut scene = scene();
        scene.bend = bend;
        let capture = gl.render(&mut renderer, &scene, &frame(&picture));
        let view = *renderer.last_view();
        // The centre of the picture is the fixed point of the curvature, so it is the one pixel
        // that lands in the same place either way.
        let got = at_nes_pixel(&capture, &view, 128, 120);
        assert!(
            close_to(got, MAGENTA),
            "a wholly transparent rectangle shows the scene, bent or not: {got:?}"
        );
    }
}

/// The renderer writes the layout of a picture byte out for itself (it does not depend on the
/// core), so the two definitions have to agree.
#[test]
fn req_core_049_the_two_definitions_of_the_picture_byte_agree() {
    assert_eq!(zax_nes::ppu::PIXEL_COLOR, PIXEL_COLOR);
    assert_eq!(zax_nes::ppu::OPAQUE_PIXEL, OPAQUE_PIXEL);
    assert_eq!(0, PIXEL_COLOR & OPAQUE_PIXEL, "the two fields are disjoint");
    // And `HIDDEN_PIXEL`, the renderer's own value in the same byte, is not a value the core can
    // ever produce: bit 7 is free (Rust-only, the overscan's cut sides).
    assert_ne!(
        0,
        zax_render::interop::HIDDEN_PIXEL & !(PIXEL_COLOR | OPAQUE_PIXEL),
        "HIDDEN_PIXEL uses a bit the core never sets"
    );
}
