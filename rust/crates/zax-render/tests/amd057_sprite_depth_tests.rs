//! AMD-057 §2, REQ-SCN-037 amended: the depth nudge of a sprite tile is
//! `n = 0.001 × (i mod 64) ∓ 0.1`, where `i` is the sprite index of REQ-CORE-043.
//!
//! The consequence is the point of the amendment: where two sprites overlap at the same depth,
//! the one the NES gives priority to — the lowest index — ends up in front, and neither of them
//! flickers.
//!
//! The nudge is measured where the renderer applies it, on the scene positions
//! `scene_export::objects` produces: the same `FramePlan::collect` the GPU path runs, without a
//! GL context.

// Test helpers fail loudly on a broken fixture, as the C# assertions do.
#![allow(clippy::expect_used, clippy::panic)]

mod common;

use common::{solid, GlFixture, SceneFixture};
use zax_profiles::enums::ProfileLayer;
use zax_render::camera::SceneCamera;
use zax_render::geometry;
use zax_render::scene_export;

/// The formula itself, as AMD-057 §2 writes it.
#[test]
fn req_scn_037_the_offset_is_a_thousandth_of_the_index_either_side_of_the_tenth() {
    // The three cases the amended TV-W2 names.
    assert!((geometry::sprite_nudge(0, false) - -0.100).abs() < 1e-6);
    assert!((geometry::sprite_nudge(5, false) - -0.095).abs() < 1e-6);
    // The lower half of an 8 × 16 is index 64 + n, and takes the same offset as its upper half.
    assert!((geometry::sprite_nudge(69, false) - -0.095).abs() < 1e-6);
    assert!((geometry::sprite_nudge(5, false) - geometry::sprite_nudge(69, false)).abs() < 1e-9);

    // Behind the background, the formula as AMD-057 §2 writes it: the same step, **added** to
    // +0.1 rather than to −0.1. A higher index is therefore further away on that side too, so
    // the priority order is the same whichever side of the background a sprite is on.
    assert!((geometry::sprite_nudge(0, true) - 0.100).abs() < 1e-6);
    assert!((geometry::sprite_nudge(5, true) - 0.105).abs() < 1e-6);
    assert!(geometry::sprite_nudge(3, true) < geometry::sprite_nudge(10, true));

    // The eight sprite tiles AMD-057 §1 measured on Contra, frame 599.
    for (index, expected) in [
        (5, -0.095),
        (9, -0.091),
        (20, -0.080),
        (24, -0.076),
        (35, -0.065),
        (39, -0.061),
        (50, -0.050),
        (54, -0.046),
    ] {
        let found = geometry::sprite_nudge(index, false);
        assert!(
            (found - expected).abs() < 1e-6,
            "sprite {index}: {found} instead of {expected}"
        );
    }

    // The highest index of the range still leaves the sprites in front of the background: the
    // whole span of the offset is 0.063 of a unit, under the 0.1 that separates it from 0.
    assert!(geometry::sprite_nudge(63, false) < 0.0);
    assert!(geometry::sprite_nudge(127, true) > 0.0);
}

/// The nudge reaches the drawn geometry, measured on the front face of the sprite's box: TV-W2
/// as AMD-057 §2 restates it.
#[test]
fn tv_w2_the_front_face_of_a_sprite_sits_at_the_offset_of_its_index() {
    // The front face of a tile of depth D is its lowest z in the local frame, and the placement
    // of REQ-E3D-406 puts a L1 FRONT tile's front face at z = 0 before the nudge.
    let front_of = |index: i32, behind: bool| -> f32 {
        let built = SceneFixture::new()
            .background(0x0F)
            .palette(5, 0x16)
            .pattern(solid(1, 0), 4, ProfileLayer::L1)
            .sprite_indexed(solid(1, 0), 96, 112, 1, behind, index)
            .build();
        let mut camera = SceneCamera::new();
        camera.snap_to_setpoints();
        let view = camera.build_view(built.scene.view_clipping(), 4.0 / 3.0);
        let exported = scene_export::objects(&built.scene, &built.frame(), &view);
        assert_eq!(1, exported.len(), "one sprite object");
        exported[0]
            .positions
            .iter()
            .map(|position| position.z)
            .fold(f32::MAX, f32::min)
    };

    let close = |found: f32, expected: f32, what: &str| {
        assert!(
            (found - expected).abs() < 1e-3,
            "{what}: the front face is at {found}, not {expected}"
        );
    };

    close(front_of(0, false), -0.100, "sprite of index 0, in front");
    close(front_of(5, false), -0.095, "sprite of index 5");
    close(front_of(69, false), -0.095, "lower half of index 69");
    close(front_of(0, true), 0.100, "sprite of index 0, behind");

    // A background tile is not nudged at all (REQ-SCN-037: "les tuiles de fond ne sont pas
    // décalées").
    let background = SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x30)
        .pattern(solid(1, 0), 4, ProfileLayer::L1)
        .tile(solid(1, 0), 96, 112, 0)
        .build();
    let mut camera = SceneCamera::new();
    camera.snap_to_setpoints();
    let view = camera.build_view(background.scene.view_clipping(), 4.0 / 3.0);
    let exported = scene_export::objects(&background.scene, &background.frame(), &view);
    let front = exported[0]
        .positions
        .iter()
        .map(|position| position.z)
        .fold(f32::MAX, f32::min);
    close(front, 0.0, "a background tile");
}

/// The second criterion of AMD-057 §2: two flat sprites that overlap, indices 3 and 10 — the
/// sprite of index 3 is drawn in front, and nothing flickers (the depths differ by a fixed
/// 0.007, so no frame can swap them).
#[test]
fn amd_057_of_two_overlapping_sprites_the_lower_index_is_in_front() {
    let third = geometry::sprite_nudge(3, false);
    let tenth = geometry::sprite_nudge(10, false);
    assert!(
        third < tenth,
        "sprite 3 is nearer the eye than sprite 10: {third} against {tenth}"
    );
    assert!((tenth - third - 0.007).abs() < 1e-6, "a fixed 0.007 apart");

    // On the geometry, which is where the criterion really lives: the two front faces are
    // 0.007 unit apart, sprite 3 in front, and the gap is a constant of the two indices — so no
    // frame can swap them, which is what "sans scintillement" means.
    //
    // Sprite sub-palette 0 is PPU entries 0x10..0x13 (the shader reads `4 * P + (t mod 4)` with
    // `P = palette + 4` for a sprite): colour 1 is red, colour 2 is white.
    let mut built = SceneFixture::new()
        .background(0x0F)
        .palette(0x11, 0x16)
        .palette(0x12, 0x30)
        .pattern(solid(1, 0), 1, ProfileLayer::L1)
        .pattern(solid(2, 1), 1, ProfileLayer::L1)
        .sprite_indexed(solid(2, 1), 96, 112, 0, false, 10)
        .sprite_indexed(solid(1, 0), 96, 112, 0, false, 3)
        .build();

    let front_faces: Vec<f32> = {
        let mut camera = SceneCamera::new();
        camera.snap_to_setpoints();
        let view = camera.build_view(built.scene.view_clipping(), 4.0 / 3.0);
        scene_export::objects(&built.scene, &built.frame(), &view)
            .iter()
            .map(|object| {
                object
                    .positions
                    .iter()
                    .map(|position| position.z)
                    .fold(f32::MAX, f32::min)
            })
            .collect()
    };
    assert_eq!(2, front_faces.len());
    // The list order is the sprite order, so sprite 10 is first.
    assert!((front_faces[0] - tenth).abs() < 1e-3, "{front_faces:?}");
    assert!((front_faces[1] - third).abs() < 1e-3, "{front_faces:?}");

    // And on the GPU, the nearer one wins every pixel of the overlap. The camera is brought in
    // to 32 units for this: at the default setpoint distance (about 148 units) the 0.007 unit
    // gap is **1.6 least significant bits** of the 24-bit depth buffer, because the projection
    // of REQ-SCN-004 (near 0.3, far 5000) spends almost all of its range in front of the scene.
    // That is a property of the formula AMD-057 measured on the original, not of this port — it
    // is recorded here so the frailty is on the record rather than hidden by a lucky capture.
    built.scene.camera.current_mut().set_distance(32.0);
    built.scene.camera.snap_to_setpoints();

    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();
    let capture = gl.render_built(&mut renderer, &built);
    assert_eq!(2, renderer.statistics().opaque_tiles, "both sprites drawn");

    // The colour the two overlap in is sprite 3's, whichever order the draws came in.
    let (x0, x1) = capture.column_span(0);
    let (y0, y1) = capture.row_span(0);
    let (cx, cy) = ((x0 + x1) / 2, (y0 + y1) / 2);
    let (r, g, b) = capture.at(cx, cy);
    assert!(
        r > g + 20 && r > b + 20,
        "the overlap is sprite 3's red, not sprite 10's white: ({r}, {g}, {b})"
    );
}
