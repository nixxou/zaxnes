//! Offscreen captures of synthetic frames, checked by pixel statistics: default framing, shading
//! vectors, sprite nudge ordering, shadows per layer, clipping, HUD and background colour
//! (SPEC-06 §5, SPEC-01 REQ-ARCH-031; port of `CaptureTests.cs`).
//!
//! Every test renders through the GL fixture, which hands the renderer a framebuffer it did not
//! create — the contract of REQ-SCN-057 and REQ-ARCH-003.

mod common;

use common::{solid, Built, Capture, GlFixture, PatternOptions, SceneFixture};
use zax_engine3d::palette::Rgb24;
use zax_engine3d::settings::{ConversionSettings, ZLayer};
use zax_profiles::enums::{ProfileLayer, VariantRenderStyle};
use zax_profiles::values::Vec4F;
use zax_render::numerics::Vector3;
use zax_render::scene::SkyboxKind;

const BLUE_22: i32 = (104 << 16) | (136 << 8) | 252;
const BLACK: i32 = 0;

/// The lit front face of the NES 0x30 floor: (223, 219, 223) (SPEC-06 §5.2).
const LIT_FLOOR: (i32, i32, i32) = (223, 219, 223);

/// The same face in shadow: (146, 144, 146) (SPEC-06 §5.2, REQ-SCN-031).
const SHADOWED_FLOOR: (i32, i32, i32) = (146, 144, 146);

#[test]
fn req_scn_046_the_background_is_the_universal_nes_colour_of_the_frame() {
    let built = flat(0x22, 0x30);
    let gl = GlFixture::new();
    let mut renderer = gl.renderer();

    let capture = gl.render_built(&mut renderer, &built);

    // The capture corners are outside the NES picture, so they show the clear colour.
    assert_eq!(BLUE_22, capture.packed(0, 0));
    assert_eq!(BLUE_22, capture.packed(capture.width - 1, capture.height - 1));
}

#[test]
fn req_scn_046_a_skybox_of_color_uses_the_custom_colour_and_a_stopped_game_is_black() {
    let mut built = flat(0x22, 0x30);
    built.scene.settings.skybox = SkyboxKind::Color;
    built.scene.settings.custom_background_color = Rgb24::new(255, 0, 0);

    let gl = GlFixture::new();
    let mut renderer = gl.renderer();

    let custom = gl.render_built(&mut renderer, &built);
    assert_eq!(255 << 16, custom.packed(0, 0));

    // REQ-SCN-047: outside a running game the 3D scene is not drawn at all.
    built.scene.settings.skybox = SkyboxKind::Default;
    built.scene.game_running = false;
    let stopped = gl.render_built(&mut renderer, &built);
    assert_eq!(BLACK, stopped.packed(0, 0));
    assert_eq!(1, stopped.distinct_colors().len());
}

#[test]
fn req_scn_012_the_default_view_frames_the_nes_picture_centred_and_about_87_percent_high() {
    let built = flat(0x22, 0x30);
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();

    let capture = gl.render_built(&mut renderer, &built);
    let (first_row, last_row) = capture.row_span(BLUE_22);
    let (first_column, last_column) = capture.column_span(BLUE_22);

    let height_ratio = f64::from(last_row - first_row + 1) / f64::from(capture.height);
    assert!((0.85..=0.93).contains(&height_ratio), "height ratio {height_ratio}");

    // 4:3 here, so the picture is wider on screen than the 52 % of the 16:9 reference view.
    let width_ratio = f64::from(last_column - first_column + 1) / f64::from(capture.width);
    assert!((0.65..=0.78).contains(&width_ratio), "width ratio {width_ratio}");

    // Centred: the margins left and right, top and bottom, match within 2 % of the side.
    let horizontal = (first_column - (capture.width - 1 - last_column)).abs();
    assert!(horizontal <= capture.width * 2 / 100, "horizontal offset {horizontal}");
    let vertical = (first_row - (capture.height - 1 - last_row)).abs();
    assert!(vertical <= capture.height * 2 / 100, "vertical offset {vertical}");
}

#[test]
fn section_5_2_a_lit_white_front_face_is_srgb_223() {
    let built = flat(0x0F, 0x30);
    let gl = GlFixture::new();
    let mut renderer = gl.renderer();

    let capture = gl.render_built(&mut renderer, &built);
    let (r, g, b) = capture.at(capture.width / 2, capture.height / 2);

    // NES 0x30 is (252, 248, 252); lit by (1, -1, 1) at N.L = 0.577 it is (223, 219, 223).
    assert!((220..=226).contains(&r), "r {r}");
    assert!((216..=222).contains(&g), "g {g}");
    assert!((220..=226).contains(&b), "b {b}");
}

#[test]
fn section_5_2_a_lit_blue_front_face_is_srgb_91_120_223() {
    let built = flat(0x0F, 0x22);
    let gl = GlFixture::new();
    let mut renderer = gl.renderer();

    let capture = gl.render_built(&mut renderer, &built);
    let (r, g, b) = capture.at(capture.width / 2, capture.height / 2);

    assert!((88..=94).contains(&r), "r {r}");
    assert!((117..=123).contains(&g), "g {g}");
    assert!((220..=226).contains(&b), "b {b}");
}

#[test]
fn req_scn_026_the_intensity_setting_brightens_and_darkens_the_direct_light() {
    let mut built = flat(0x0F, 0x30);
    let gl = GlFixture::new();
    let mut renderer = gl.renderer();
    let (x, y) = (gl.width() / 2, gl.height() / 2);

    built.scene.settings.set_light_intensity(1.0);
    let normal = gl.render_built(&mut renderer, &built).luma(x, y);

    built.scene.settings.set_light_intensity(0.5);
    let dim = gl.render_built(&mut renderer, &built).luma(x, y);

    built.scene.settings.set_light_intensity(1.5);
    let bright = gl.render_built(&mut renderer, &built).luma(x, y);

    assert!(dim < normal);
    assert!(bright > normal);
}

#[test]
fn req_scn_037_a_flat_sprite_stays_in_front_of_a_background_tile_of_the_same_depth() {
    let built = nudge_scene(false);
    let gl = GlFixture::new();
    let mut renderer = gl.renderer();

    let first = gl.render_built(&mut renderer, &built);
    let second = gl.render_built(&mut renderer, &built);

    let (x, y) = (first.width / 2, first.height / 2);
    let (r, g, b) = first.at(x, y);

    // The sprite is 0x16 (red, lit), the background tile 0x21 (blue, lit): red wins.
    assert!(r > g && r > b, "expected the sprite's red in front, got ({r}, {g}, {b})");

    // No flicker: the same pixel comes out the same on two consecutive frames.
    assert_eq!(first.packed(x, y), second.packed(x, y));
}

#[test]
fn req_scn_037_a_behind_background_sprite_is_pushed_behind_instead() {
    let gl = GlFixture::new();
    let mut renderer = gl.renderer();

    let front = gl.render_built(&mut renderer, &nudge_scene(false));
    let behind = gl.render_built(&mut renderer, &nudge_scene(true));

    let (x, y) = (front.width / 2, front.height / 2);
    assert_ne!(front.packed(x, y), behind.packed(x, y));

    let (r, g, b) = behind.at(x, y);
    assert!(b > r, "expected the background tile's blue in front, got ({r}, {g}, {b})");
}

#[test]
fn req_scn_030_the_cast_shadow_is_present_and_its_edge_is_sharp() {
    let gl = GlFixture::new();
    let mut renderer = gl.renderer();

    let lit = gl.render_built(&mut renderer, &shadow_scene(false, true, VariantRenderStyle::Normal));
    let shadowed =
        gl.render_built(&mut renderer, &shadow_scene(true, true, VariantRenderStyle::Normal));

    // The floor is NES 0x30 seen from the front: (223, 219, 223) lit, (146, 144, 146) in
    // shadow (SPEC-06 §5.2).
    let shadow_pixels = count_near(&shadowed, SHADOWED_FLOOR, 4);
    assert!(
        shadow_pixels > 200,
        "expected a cast shadow, found {shadow_pixels} shadowed pixels"
    );
    assert_eq!(0, count_near(&lit, SHADOWED_FLOOR, 4));

    // A hard edge: the lit floor turns into the shadowed floor within two pixels.
    let edge = edge_width(&shadowed);
    assert!((0..=2).contains(&edge), "edge width {edge}");
}

#[test]
fn req_scn_031_a_slanted_lit_surface_shows_no_shadow_acne() {
    // The biases of REQ-SCN-031 are there "pour éviter l'acné d'ombre". Even with the
    // 4096-texel map of AMD-034 §2, one texel over a whole scene spans far more than 0.02 unit,
    // so a front face would shadow itself in a stipple pattern; only the back faces fill the
    // map.
    let gl = GlFixture::create(240, 240);
    let mut renderer = gl.renderer();

    // One big slab of 3-voxel-deep tiles, every one of them a caster and a receiver, seen from
    // an angle so that its top face is lit and visible.
    let mut built = SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x30)
        .pattern(solid(1, 0), 3, ProfileLayer::L1)
        .rect(solid(1, 0), 4, 6, 24, 18)
        .build();
    built.scene.camera.current_mut().angles = Vector3::new(25.0, -20.0, 0.0);
    built.scene.camera.snap_to_setpoints();

    let lit = gl.render_built(&mut renderer, &built);
    assert!(renderer.statistics().shadow_pass);

    built.scene.settings.shadows = false;
    let flat = gl.render_built(&mut renderer, &built);

    // Self-shadowing would double the number of distinct greys of the slab; without acne the
    // two captures hold the same handful of colours.
    let (with, without) = (lit.distinct_colors().len(), flat.distinct_colors().len());
    assert!(
        with <= without + 2,
        "shadow acne: {with} colours with shadows, {without} without"
    );
}

#[test]
fn req_scn_030_turning_the_shadows_off_removes_every_shadow() {
    let gl = GlFixture::new();
    let mut renderer = gl.renderer();

    let off = gl.render_built(&mut renderer, &shadow_scene(false, true, VariantRenderStyle::Normal));
    let on = gl.render_built(&mut renderer, &shadow_scene(true, true, VariantRenderStyle::Normal));

    assert!(off.distinct_colors().len() < on.distinct_colors().len());
}

#[test]
fn req_scn_032_clearing_the_casting_flag_of_a_layer_removes_its_shadows() {
    let gl = GlFixture::new();
    let mut renderer = gl.renderer();

    let casting =
        gl.render_built(&mut renderer, &shadow_scene(true, true, VariantRenderStyle::Normal));
    let not_casting =
        gl.render_built(&mut renderer, &shadow_scene(true, false, VariantRenderStyle::Normal));

    assert!(count_near(&casting, SHADOWED_FLOOR, 4) > 200);
    assert_eq!(0, count_near(&not_casting, SHADOWED_FLOOR, 4));
}

#[test]
fn req_scn_036_a_fragment_outside_the_visible_rectangle_is_dropped() {
    let mut built = SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x30)
        .pattern(solid(1, 0), 8, ProfileLayer::L1)
        .rect(solid(1, 0), 0, 8, 6, 14)
        .build();
    let gl = GlFixture::new();
    let mut renderer = gl.renderer();

    built.scene.clipping = ConversionSettings::DEFAULT_CLIPPING;
    let wide = gl.render_built(&mut renderer, &built);
    let (wide_first, _) = wide.column_span(BLACK);

    // REQ-SCN-039: the rectangle is recomputed every rendered frame from the margins.
    built.scene.clipping = Vec4F::new(24.1, 8.1, 8.1, 0.1);
    let narrow = gl.render_built(&mut renderer, &built);
    let (narrow_first, _) = narrow.column_span(BLACK);

    assert!(
        narrow_first > wide_first + 20,
        "the left edge of R did not move: {wide_first} then {narrow_first}"
    );
    assert!(narrow.count_other(BLACK) < wide.count_other(BLACK));
}

#[test]
fn req_scn_044_a_hud_pattern_outside_the_visible_rectangle_stays_whole() {
    // Screen row 0 is scene y in [240, 248], above the top edge of R (REQ-E3D-410).
    let built = SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x30)
        .pattern(solid(1, 0), 2, ProfileLayer::Ui)
        .rect(solid(1, 0), 4, 0, 8, 1)
        .build();
    let gl = GlFixture::new();
    let mut renderer = gl.renderer();

    let capture = gl.render_built(&mut renderer, &built);
    let drawn = capture.count_other(BLACK);

    assert!(drawn > 500, "the HUD strip was clipped away, only {drawn} pixels drawn");
}

#[test]
fn req_scn_040_a_transparent_pattern_lets_the_wall_behind_it_show_through() {
    let built = SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x16)
        .palette(2, 0x30)
        .pattern(solid(1, 0), 1, ProfileLayer::L2)
        .pattern_with(solid(2, 0), 6, ProfileLayer::L1, alpha(0.5))
        .rect(solid(1, 0), 8, 10, 16, 10)
        .rect(solid(2, 0), 13, 13, 6, 4)
        .build();
    let gl = GlFixture::new();
    let mut renderer = gl.renderer();

    let capture = gl.render_built(&mut renderer, &built);
    let (r, g, b) = capture.at(capture.width / 2, capture.height / 2);

    // Half white over a lit red wall: still clearly reddish, but much lighter than the wall.
    let (wall_r, wall_g, wall_b) = capture.at(capture.width / 2, (capture.height / 2) + 90);
    assert!(
        r > wall_r - 40 && g > wall_g + 20 && b > wall_b + 20,
        "blend looks wrong: pattern ({r}, {g}, {b}) over wall ({wall_r}, {wall_g}, {wall_b})"
    );
}

#[test]
fn req_scn_041_a_shadow_only_pattern_is_invisible_but_still_casts() {
    let normal = shadow_scene(true, true, VariantRenderStyle::Normal);
    let shadow_only = shadow_scene(true, true, VariantRenderStyle::ShadowOnly);

    let gl = GlFixture::new();
    let mut renderer = gl.renderer();

    let visible = gl.render_built(&mut renderer, &normal);
    let invisible = gl.render_built(&mut renderer, &shadow_only);

    // The red block is gone...
    assert!(
        count_reddish(&invisible) < count_reddish(&visible) / 10,
        "the shadow-only block is still drawn: {} red pixels",
        count_reddish(&invisible)
    );

    // ... but its shadow is still on the floor.
    assert!(count_near(&invisible, SHADOWED_FLOOR, 4) > 200);
}

#[test]
fn req_scn_057_two_renderers_can_share_a_context_without_interfering() {
    let built = flat(0x22, 0x30);
    let gl = GlFixture::new();
    let mut first = gl.renderer();
    let mut second = gl.renderer();

    let one = gl.render_built(&mut first, &built);
    let two = gl.render_built(&mut second, &built);

    assert_eq!(one.pixels, two.pixels);
}

// ------------------------------------------------------------------ AMD-021 R2-009/R2-010

#[test] // AMD-021 R2-009, first half of the criterion
fn amd_021_r2_009_a_half_opaque_motif_on_black_is_srgb_158_124_145() {
    // NES 0x35 is (244, 192, 224); its lit front face is linear (0.9047, 0.5273, 0.7455)
    // x (0.1812 + 0.577) = (0.6859, 0.3998, 0.5653). Half of that over a black backdrop is
    // (0.3430, 0.1999, 0.2826) in linear, that is sRGB (158, 124, 145). Blended in sRGB
    // instead it would come out (108, 85, 99).
    let built = SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x35)
        .pattern_with(solid(1, 0), 2, ProfileLayer::L1, alpha(0.5))
        .rect(solid(1, 0), 8, 8, 16, 14)
        .build();

    let gl = GlFixture::new();
    let mut renderer = gl.renderer();

    let capture = gl.render_built(&mut renderer, &built);
    let (r, g, b) = capture.at(capture.width / 2, capture.height / 2);

    assert!((156..=160).contains(&r), "r {r}");
    assert!((122..=126).contains(&g), "g {g}");
    assert!((143..=147).contains(&b), "b {b}");
}

#[test] // AMD-021 R2-009, second half of the criterion
fn amd_021_r2_009_the_same_motif_over_srgb_0_120_131_gives_g_147() {
    // The wall is NES 0x1C (0, 136, 148), whose lit front face is sRGB (0, 120, 131) — the
    // background colour the criterion names. Blended in linear, the green channel comes out at
    // 147; blended in sRGB it would be 134.
    //
    // The motif in front is laid down as sprite records: two background records on the same
    // cell would replace one another and leave a hole in the wall, which is exactly what this
    // measurement must not have.
    let mut fixture = SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x1C)
        .palette(18, 0x35)
        .pattern(solid(1, 0), 1, ProfileLayer::L2)
        .pattern_with(solid(2, 0), 6, ProfileLayer::L1, alpha(0.5))
        .rect(solid(1, 0), 6, 6, 20, 18);
    for row in 12..18 {
        for column in 12..20 {
            fixture.push_sprite(solid(2, 0), column * 8, row * 8, 0, false);
        }
    }
    let built = fixture.build();

    let gl = GlFixture::new();
    let mut renderer = gl.renderer();

    let capture = gl.render_built(&mut renderer, &built);

    // The wall on its own, below the transparent motif: the named background colour.
    let (wall_r, wall_g, wall_b) = capture.at(capture.width / 2, (capture.height / 2) + 110);
    assert!((0..=2).contains(&wall_r), "wall r {wall_r}");
    assert!((118..=122).contains(&wall_g), "wall g {wall_g}");
    assert!((129..=133).contains(&wall_b), "wall b {wall_b}");

    let (_, g, _) = capture.at(capture.width / 2, capture.height / 2);
    assert!((145..=149).contains(&g), "g {g}");
}

#[test] // AMD-021 R2-010
fn amd_021_r2_010_a_transparent_receiver_takes_no_shadow() {
    // The synthetic shadow scene of REQ-SCN-030 with the receiving floor at alpha 0.5: the
    // shadow of the block must not appear on it at all, so the whole floor is one colour.
    let render = |floor_alpha: f32| {
        let mut built = SceneFixture::new()
            .background(0x0F)
            .palette(1, 0x30)
            .palette(2, 0x16)
            .pattern_with(solid(1, 0), 1, ProfileLayer::L2, alpha(floor_alpha))
            .pattern(solid(2, 0), 8, ProfileLayer::L1)
            .rect(solid(1, 0), 8, 10, 16, 10)
            .rect(solid(2, 0), 14, 13, 2, 2)
            .build_with(|settings| settings.shadow = true);
        built.scene.settings.shadows = true;
        let gl = GlFixture::new();
        let mut renderer = gl.renderer();
        gl.render_built(&mut renderer, &built)
    };

    let opaque = render(1.0);
    let transparent = render(0.5);

    // The opaque floor does carry the shadow: that is REQ-SCN-030.
    assert!(
        count_near(&opaque, SHADOWED_FLOOR, 4) > 100,
        "the opaque floor shows no shadow"
    );

    // The transparent one does not: no pixel of the floor is darker than the unshadowed colour
    // of the same tile, which over a black backdrop is half of the lit floor in linear space.
    let (lit_r, lit_g, lit_b) =
        transparent.at(transparent.width / 2, (transparent.height / 2) + 100);

    let mut darker = 0;
    for y in 0..transparent.height {
        for x in 0..transparent.width {
            let (r, g, b) = transparent.at(x, y);
            if r == 0 && g == 0 && b == 0 {
                continue; // The backdrop.
            }

            if r < lit_r - 6 && g < lit_g - 6 && b < lit_b - 6 && r > 10 {
                darker += 1;
            }
        }
    }

    assert_eq!(0, darker);
    assert_eq!(0, count_near(&transparent, SHADOWED_FLOOR, 4));
}

// ------------------------------------------------------------------ scenes

fn alpha(alpha: f32) -> PatternOptions {
    PatternOptions {
        alpha,
        ..PatternOptions::default()
    }
}

fn flat(background: u8, tile_color: u8) -> Built {
    SceneFixture::new()
        .background(background)
        .palette(1, tile_color)
        .rect(solid(1, 0), 0, 1, 32, 29)
        .build()
}

fn nudge_scene(behind_background: bool) -> Built {
    let mut fixture = SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x21)
        .palette(18, 0x16)
        .pattern(solid(1, 0), 1, ProfileLayer::L1)
        .pattern(solid(2, 0), 1, ProfileLayer::L1)
        .rect(solid(1, 0), 12, 12, 8, 6);

    for row in 13..17 {
        for column in 13..19 {
            fixture.push_sprite(solid(2, 0), column * 8, row * 8, 0, behind_background);
        }
    }

    fixture.build()
}

fn shadow_scene(shadows: bool, layer_casts: bool, block_style: VariantRenderStyle) -> Built {
    let mut built = SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x30)
        .palette(2, 0x16)
        .pattern(solid(1, 0), 1, ProfileLayer::L2)
        .pattern_with(
            solid(2, 0),
            8,
            ProfileLayer::L1,
            PatternOptions {
                style: block_style,
                ..PatternOptions::default()
            },
        )
        .rect(solid(1, 0), 8, 10, 16, 10)
        .rect(solid(2, 0), 14, 13, 2, 2)
        .build_with(|settings| {
            settings.shadow = shadows;
            if !layer_casts {
                settings.layer_mut(ZLayer::L1).cast_shadow = false;
            }
        });

    built.scene.settings.shadows = shadows;
    built
}

// ------------------------------------------------------------------ measures

/// Pixels within `tolerance` of `color`.
fn count_near(capture: &Capture, color: (i32, i32, i32), tolerance: i32) -> usize {
    let mut found = 0;
    for y in 0..capture.height {
        for x in 0..capture.width {
            if near(capture.at(x, y), color, tolerance) {
                found += 1;
            }
        }
    }
    found
}

fn near(value: (i32, i32, i32), target: (i32, i32, i32), tolerance: i32) -> bool {
    (value.0 - target.0).abs() <= tolerance
        && (value.1 - target.1).abs() <= tolerance
        && (value.2 - target.2).abs() <= tolerance
}

fn count_reddish(capture: &Capture) -> usize {
    let mut found = 0;
    for y in 0..capture.height {
        for x in 0..capture.width {
            let (r, g, b) = capture.at(x, y);
            if r > 80 && r > g * 2 && r > b * 2 {
                found += 1;
            }
        }
    }
    found
}

/// The widest run of pixels that are neither the lit floor nor the shadowed floor, between a lit
/// pixel and a shadowed one on the same row: the width of the shadow edge (REQ-SCN-030 asks for
/// at most 2 pixels on a 640 × 480 capture).
fn edge_width(capture: &Capture) -> i32 {
    let mut worst = 0;

    for y in 0..capture.height {
        let mut run = -1;
        for x in 0..capture.width {
            let pixel = capture.at(x, y);
            let is_lit = near(pixel, LIT_FLOOR, 4);
            let is_shadow = near(pixel, SHADOWED_FLOOR, 4);

            if is_lit {
                run = 0;
            } else if run >= 0 && is_shadow {
                worst = worst.max(run);
                run = 0;
            } else if run >= 0 {
                run += 1;
                if run > 8 {
                    // Something else is in the way (the block itself); stop this run.
                    run = -1;
                }
            }
        }
    }

    worst
}
