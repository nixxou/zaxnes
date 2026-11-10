//! AMD-057 §2: the walls between neighbouring tiles are kept, and REQ-SCN-065 holds anyway.
//!
//! `seam_tests.rs` measures REQ-SCN-065 itself. These are the two things AMD-057 adds to it:
//! that the surface it measures really is made of **whole boxes** (so it cannot pass for the
//! wrong reason), and that the means which replaces the Q-067 seal — the depth of the inner
//! walls — is what makes it pass.

// Test helpers fail loudly on a broken fixture, as the C# assertions do.
#![allow(clippy::expect_used, clippy::panic)]

mod common;

use common::{solid, Built, Capture, GlFixture, PatternOptions, SceneFixture};
use zax_profiles::enums::ProfileLayer;
use zax_render::geometry;
use zax_render::numerics::Vector3;

const BLACK: i32 = 0;

/// AMD-012's surface, which REQ-SCN-065's criterion is written against: 4 × 4 identical tiles,
/// lit, casting and receiving shadows.
fn surface() -> Built {
    SceneFixture::new()
        .background(0x0F)
        .palette(3, 0x30)
        .pattern(solid(3, 0), 6, ProfileLayer::L1)
        .rect(solid(3, 0), 14, 14, 4, 4)
        .build()
}

/// The same surface shared between **two** objects of 2 × 4 tiles, the second criterion of
/// AMD-058 — kept here too, because two objects of equal depth and layer have the same joint and
/// the Q-067 seal never covered them.
fn surface_over_two_objects() -> Built {
    SceneFixture::new()
        .background(0x0F)
        .palette(3, 0x30)
        .pattern_block(solid(3, 0), 2, 4, 6, ProfileLayer::L1, 1.0)
        .pattern_block(solid(3, 1), 2, 4, 6, ProfileLayer::L1, 1.0)
        .rect(solid(3, 0), 14, 14, 2, 4)
        .rect(solid(3, 1), 16, 14, 2, 4)
        .build()
}

/// `solid(3, 0)` with the pixel at bit 0 left transparent: a tile AMD-063 REQ-SCN-087 does **not**
/// call full, so no lateral face is taken off it and the wall bias is still the means that holds
/// REQ-SCN-040 for it.
fn holed() -> (u64, u64) {
    let (low, high) = solid(3, 0);
    (low & !1, high & !1)
}

/// How many pixels of the interior of the drawn shape depart from its commonest colour by more
/// than two levels — the measure of REQ-SCN-065 and AMD-012.
fn offenders(capture: &Capture) -> usize {
    let (first_x, last_x) = capture.column_span(BLACK);
    let (first_y, last_y) = capture.row_span(BLACK);
    assert!(
        last_x - first_x > 24 && last_y - first_y > 24,
        "the surface is too small to measure"
    );
    let inset_x = (last_x - first_x + 1) / 12;
    let inset_y = (last_y - first_y + 1) / 12;
    let (x0, y0, x1, y1) = (
        first_x + inset_x,
        first_y + inset_y,
        last_x - inset_x,
        last_y - inset_y,
    );

    let mut counts = std::collections::HashMap::new();
    for y in y0..=y1 {
        for x in x0..=x1 {
            *counts.entry(capture.packed(x, y)).or_insert(0usize) += 1;
        }
    }
    let surface = counts
        .iter()
        .max_by_key(|&(_, count)| *count)
        .map(|(&color, _)| color)
        .expect("a non-empty interior");

    let deviation = |left: i32, right: i32| {
        let channel = |shift: i32| (((left >> shift) & 0xFF) - ((right >> shift) & 0xFF)).abs();
        channel(16).max(channel(8)).max(channel(0))
    };
    (y0..=y1)
        .flat_map(|y| (x0..=x1).map(move |x| (x, y)))
        .filter(|&(x, y)| deviation(capture.packed(x, y), surface) > 2)
        .count()
}

/// The first criterion of AMD-057 §2, read on the meshes the renderer is actually given: every
/// tile of a surface whose tiles touch is the complete box of REQ-E3D-501, 24 vertices and
/// 12 triangles.
fn assert_whole_boxes(built: &Built) {
    assert!(!built.draw_list.tiles().is_empty());
    for tile in built.draw_list.tiles() {
        let mesh = tile.mesh.as_ref().expect("a mesh");
        assert_eq!(
            24,
            mesh.vertex_count(),
            "a drawn tile of a touching surface is the whole box"
        );
        assert_eq!(12, mesh.triangle_count());
    }
}

#[test]
fn amd_057_the_surface_of_req_scn_065_passes_with_every_wall_kept() {
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();
    let mut built = surface();

    // The criterion the amendment names: "le critère de REQ-SCN-065 passe avec les parois
    // conservées". Both halves are asserted, the geometry first.
    assert_whole_boxes(&built);
    for map_size in [4096, 1024, 256] {
        built.scene.settings.set_shadow_map_size(map_size);
        let capture = gl.render_built(&mut renderer, &built);
        assert!(renderer.statistics().shadow_pass);
        assert_eq!(
            0,
            offenders(&capture),
            "with a {map_size}-texel shadow map the joints of the surface show"
        );
    }
}

#[test]
fn amd_057_a_surface_split_over_two_objects_passes_too() {
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();
    let built = surface_over_two_objects();
    assert_whole_boxes(&built);

    let capture = gl.render_built(&mut renderer, &built);
    assert_eq!(
        0,
        offenders(&capture),
        "the joint between the two objects shows"
    );
}

/// The measure that **bites**: the two tests above cannot fail by themselves if the means of
/// REQ-SCN-065 is removed, so this one shows which means is holding them. With the wall bias at
/// 0 — the walls kept and nothing done about them — the joints come back.
/// The measure that **bites**. With the walls kept, the synthetic criterion of REQ-SCN-065
/// passes even with no bias at all — on a surface of one object seen from the default view the
/// coincident walls lose the fill rule anyway. What does not pass is **REQ-SCN-040**: in the
/// equal-depth colour pass of a translucent object, a wall that is coincident with the face of
/// its neighbour is kept by `depth_func(EQUAL)` and that pixel is blended twice.
///
/// So this is where the bias earns its keep, and the same finding as the .NET lot V2-29 ("avec
/// `WallDepthBias = 0` les deux tests de jointure passent encore mais
/// `REQ_SCN_040_…_shows_no_internal_face` échoue"). On real scenes, which hold hundreds of
/// objects of different depths, it also erases the one-pixel dark lines Q-067 reported.
///
/// **Amended by AMD-063 REQ-SCN-087**: on a transparent object of **full** tiles the coincident
/// walls are no longer drawn at all, so the bias has nothing left to hold there — that is what
/// `req_scn_087_a_transparent_surface_of_full_tiles_needs_no_bias` below shows. The surface here
/// is therefore made of tiles that are **not** full (one pixel of colour 0), the case the
/// removal leaves alone and where the bias is still the means.
#[test]
fn req_scn_040_the_wall_depth_is_what_keeps_a_coincident_wall_out_of_the_blend() {
    let gl = GlFixture::create(640, 480);
    let mut built = SceneFixture::new()
        .background(0x0F)
        .palette(3, 0x30)
        .pattern_block(holed(), 4, 4, 24, ProfileLayer::L1, 0.5)
        .rect(holed(), 14, 14, 4, 4)
        .build();

    // Seen from a corner, so the walls between the tiles have a real width on screen: head-on
    // they are edge-on and hide themselves.
    built.scene.camera.current_mut().angles = Vector3::new(8.0, 8.0, 0.0);
    built.scene.camera.snap_to_setpoints();

    let mut renderer = gl.renderer();
    let with_bias = offenders(&gl.render_built(&mut renderer, &built));
    assert_eq!(1, renderer.statistics().transparent_objects);
    assert_eq!(16, renderer.statistics().transparent_tiles);
    assert!(
        with_bias <= 2,
        "the default keeps the internal faces out of the blend: {with_bias} pixel(s)"
    );

    let mut plain = gl.renderer();
    plain.set_wall_depth_bias(0.0);
    let without = offenders(&gl.render_built(&mut plain, &built));
    assert!(
        without > with_bias,
        "with the bias at 0 a coincident wall is blended twice: {without} pixel(s) against          {with_bias}"
    );

    // The requirement is an inequality, so a larger bias holds too: nothing depends on the
    // exact quarter unit.
    let mut larger = gl.renderer();
    larger.set_wall_depth_bias(geometry::WALL_DEPTH_BIAS * 4.0);
    assert!(offenders(&gl.render_built(&mut larger, &built)) <= 2);
}

/// AMD-063 REQ-SCN-087 and REQ-SCN-065 on a **transparent** surface: the same scene made of
/// **full** tiles shows no joint even with the wall bias at 0, because the coincident walls are
/// no longer in the drawn mesh at all. That is the amendment's own reading of REQ-SCN-040 — "its
/// depth pass is not enough to hide the walls that fall on a joint; REQ-SCN-087 removes them".
#[test]
fn req_scn_087_a_transparent_surface_of_full_tiles_needs_no_bias() {
    let gl = GlFixture::create(640, 480);
    let mut built = SceneFixture::new()
        .background(0x0F)
        .palette(3, 0x30)
        .pattern_block(solid(3, 0), 4, 4, 24, ProfileLayer::L1, 0.5)
        .rect(solid(3, 0), 14, 14, 4, 4)
        .build();

    built.scene.camera.current_mut().angles = Vector3::new(8.0, 8.0, 0.0);
    built.scene.camera.snap_to_setpoints();

    let mut renderer = gl.renderer();
    assert_eq!(0, offenders(&gl.render_built(&mut renderer, &built)));
    assert_eq!(1, renderer.statistics().transparent_objects);
    assert_eq!(16, renderer.statistics().transparent_tiles);

    // With no bias at all: still nothing, the walls being gone rather than pushed.
    let mut plain = gl.renderer();
    plain.set_wall_depth_bias(0.0);
    assert_eq!(0, offenders(&gl.render_built(&mut plain, &built)));

    // And the drawn surface is still the same silhouette: the removal takes faces that fall on a
    // joint, never one of the outline (the block is 32 by 32 pixels of the picture).
    let capture = gl.render_built(&mut plain, &built);
    let (first_x, last_x) = capture.column_span(BLACK);
    let (first_y, last_y) = capture.row_span(BLACK);
    assert!(last_x - first_x > 24 && last_y - first_y > 24);
}

/// AMD-057 forbids reaching REQ-SCN-065 by taking the walls out, so the geometry has to be
/// **unchanged** by the bias: it moves depth only, never a silhouette.
#[test]
fn amd_057_the_wall_depth_moves_no_vertex_and_no_silhouette() {
    let gl = GlFixture::create(640, 480);
    let mut built = surface();
    built.scene.camera.current_mut().angles = Vector3::new(14.0, 14.0, 0.0);
    built.scene.camera.snap_to_setpoints();

    let mut renderer = gl.renderer();
    let with_bias = gl.render_built(&mut renderer, &built);
    let drawn = renderer.statistics().triangles;

    let mut plain = gl.renderer();
    plain.set_wall_depth_bias(0.0);
    let without = gl.render_built(&mut plain, &built);

    // Same draws, same triangles: the bias is not geometry.
    assert_eq!(drawn, plain.statistics().triangles);
    assert_eq!(
        renderer.statistics().draw_calls,
        plain.statistics().draw_calls
    );

    // Same silhouette, to the pixel: only the depth of the inner walls moved.
    assert_eq!(with_bias.column_span(BLACK), without.column_span(BLACK));
    assert_eq!(with_bias.row_span(BLACK), without.row_span(BLACK));
}

/// A smoothed surface has no joint to hide (REQ-E3D-520 merges the tiles) and its normals are
/// not axis aligned, so the `|normal.z|` test would be meaningless there: a MARCHING draw takes
/// no bias at all.
#[test]
fn amd_057_a_smoothed_surface_takes_no_wall_bias() {
    use zax_engine3d::settings::RenderAlgorithm;

    let mut built = SceneFixture::new()
        .background(0x0F)
        .palette(3, 0x30)
        .pattern_with(solid(3, 0), 6, ProfileLayer::L1, PatternOptions::default())
        .rect(solid(3, 0), 14, 14, 4, 4)
        .build();
    // REQ-E3D-340: the whole geometry is rebuilt with the smoothed algorithm.
    built.engine.set_render_algorithm(RenderAlgorithm::Marching);
    built.rebuild_draw_list();

    let smoothed = built
        .draw_list
        .tiles()
        .iter()
        .filter_map(|tile| tile.mesh.as_ref())
        .any(|mesh| !mesh.has_axis_aligned_normals());
    assert!(smoothed, "MARCHING produced a smoothed mesh");

    // The picture does not depend on the bias at all, which is the renderer saying it applies
    // none: every pixel is the same with the bias and without it.
    let gl = GlFixture::create(640, 480);
    let mut renderer = gl.renderer();
    let with_bias = gl.render_built(&mut renderer, &built);
    let mut plain = gl.renderer();
    plain.set_wall_depth_bias(0.0);
    let without = gl.render_built(&mut plain, &built);
    for y in 0..gl.height() {
        for x in 0..gl.width() {
            assert_eq!(
                with_bias.packed(x, y),
                without.packed(x, y),
                "a smoothed surface moved at ({x}, {y})"
            );
        }
    }
}
