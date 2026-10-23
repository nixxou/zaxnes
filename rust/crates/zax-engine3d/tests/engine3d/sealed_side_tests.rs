//! Q-067 (SPEC-06 REQ-SCN-065): the wall that closes a tile against a neighbour of the same
//! object is left out of the *drawn* mesh when the neighbour covers it entirely, so the joints
//! of a flat surface show neither a line nor a fin.
//!
//! The mesh of REQ-E3D-501 is not touched: a mask of 0 — what every existing test vector asks
//! for — still gives the tile closed on itself, which is what
//! `a_sealed_side_changes_nothing_when_the_mask_is_empty` checks.

use zax_engine3d::engine::{ConversionEngine, FrameRole};
use zax_engine3d::geometry::greedy;
use zax_engine3d::geometry::tile_sides;
use zax_engine3d::geometry::tile_voxels::TileVoxels;
use zax_engine3d::image::TileImageKey;
use zax_engine3d::geometry::texture::ObjectTexture;
use zax_engine3d::pattern::EnginePattern;
use zax_engine3d::settings::RenderAlgorithm;

use super::support::geo::{self, object_3d, rectangles, rectangles_with_normal, MINUS_X, MINUS_Y, MINUS_Z, PLUS_X, PLUS_Y, PLUS_Z};
use super::support::{Batch, PatternOptions, ProfileBuilder};

/// A solid 8 × 8 tile: every lateral side is a full plane.
fn solid() -> TileImageKey {
    geo::tile(&["11111111"; 8])
}

fn field_and_texture(pattern: &EnginePattern, depth: i32) -> (TileVoxels, ObjectTexture) {
    let instance = object_3d(depth);
    let texture = geo::texture_of(pattern, &instance);
    let (mut fields, _) = geo::fields(pattern, &instance, RenderAlgorithm::Greedy);
    (fields.swap_remove(0), texture)
}

#[test]
fn a_sealed_side_changes_nothing_when_the_mask_is_empty() {
    // REQ-E3D-501, REQ-E3D-523: the default is the mesh closed on itself.
    let (field, texture) = field_and_texture(&geo::one_tile(solid()), 8);

    let closed = greedy::build(&field, &texture, false, 0);
    let same = greedy::build(&field, &texture, false, 0);

    assert_eq!(6, rectangles(&closed));
    assert_eq!(closed.triangle_count(), same.triangle_count());
    assert_eq!(closed.vertex_count(), same.vertex_count());
}

#[test]
fn a_sealed_side_drops_only_the_wall_it_names() {
    // Q-067: sealing +X removes the wall at x = 8 and nothing else; the five other faces of
    // the box keep the positions and the texture coordinates of REQ-E3D-504 and 506.
    let (field, texture) = field_and_texture(&geo::one_tile(solid()), 8);

    let sealed_right = greedy::build(&field, &texture, false, tile_sides::PLUS_X);

    assert_eq!(5, rectangles(&sealed_right));
    assert_eq!(0, rectangles_with_normal(&sealed_right, PLUS_X));
    assert_eq!(1, rectangles_with_normal(&sealed_right, MINUS_X));
    assert_eq!(1, rectangles_with_normal(&sealed_right, MINUS_Y));
    assert_eq!(1, rectangles_with_normal(&sealed_right, PLUS_Y));
    assert_eq!(1, rectangles_with_normal(&sealed_right, MINUS_Z));
    assert_eq!(1, rectangles_with_normal(&sealed_right, PLUS_Z));

    // The two tiles of a flat surface seal each other, so the pair keeps no wall at all.
    let sealed_both = greedy::build(&field, &texture, false, tile_sides::ALL);
    assert_eq!(2, rectangles(&sealed_both));
    assert_eq!(1, rectangles_with_normal(&sealed_both, MINUS_Z));
    assert_eq!(1, rectangles_with_normal(&sealed_both, PLUS_Z));
}

#[test]
fn a_side_that_is_not_a_full_plane_is_never_sealed() {
    // Safety rule of Q-067: only a side whose whole plane is occupied may be sealed, since the
    // two walls then coincide cell for cell and dropping them cannot open a hole.
    let mut rows = ["11111111"; 8];
    rows[7] = "1111111.";
    let (field, _) = field_and_texture(&geo::one_tile(geo::tile(&rows)), 4);

    let full = tile_sides::full_sides(&field);
    let kept = tile_sides::MINUS_X | tile_sides::PLUS_Y;
    assert_eq!(kept, full & kept);
    assert_eq!(0, full & tile_sides::PLUS_X);
    assert_eq!(0, full & tile_sides::MINUS_Y);
}

#[test]
fn an_empty_tile_seals_nothing() {
    let (field, _) = field_and_texture(&geo::one_tile(geo::tile(&["........"; 8])), 4);
    assert_eq!(0, tile_sides::full_sides(&field));
}

#[test]
fn two_full_tiles_of_one_object_drop_the_wall_between_them() {
    // REQ-SCN-065 at the source: a 2 × 1 surface of identical full tiles. The draw list asks
    // for the variant whose shared wall is gone, so the pair draws 5 + 5 rectangles instead
    // of 6 + 6.
    let pattern = geo::pattern(2, 1, &[Some(solid()), Some(solid())]);
    let (field, texture) = field_and_texture(&pattern, 8);

    assert_eq!(tile_sides::ALL, tile_sides::full_sides(&field));

    let left = greedy::build(&field, &texture, false, tile_sides::PLUS_X);
    let right = greedy::build(&field, &texture, false, tile_sides::MINUS_X);
    assert_eq!(5, rectangles(&left));
    assert_eq!(5, rectangles(&right));

    // No face is left on the plane the two tiles share.
    assert_eq!(0, rectangles_with_normal(&left, PLUS_X));
    assert_eq!(0, rectangles_with_normal(&right, MINUS_X));
}

fn surface_engine(width: i32, height: i32) -> ConversionEngine {
    let count = usize::try_from(width * height).unwrap();
    let mut engine = ConversionEngine::new();
    engine.use_geometry();
    engine
        .load_profile(
            &ProfileBuilder::new()
                .pattern_with(
                    width,
                    height,
                    &vec![Some(solid()); count],
                    PatternOptions {
                        depth: 8,
                        ..PatternOptions::default()
                    },
                )
                .build(),
            true,
        )
        .unwrap();
    engine
}

#[test]
fn the_draw_list_seals_the_joints_of_a_surface_and_keeps_its_border() {
    // End to end: a 2 x 2 surface of one full image. Each tile loses the two walls it shares
    // with its neighbours and keeps the two that are on the border of the surface, so every
    // tile draws four rectangles instead of six.
    let mut engine = surface_engine(2, 2);

    let mut batch = Batch::new();
    batch
        .bg(solid(), 0, 0, 0)
        .bg(solid(), 8, 0, 0)
        .bg(solid(), 0, 8, 0)
        .bg(solid(), 8, 8, 0);
    batch.convert(&mut engine, 0);
    let list = engine.build_draw_list_of(FrameRole::Converting);

    assert_eq!(4, list.tiles().len());
    for tile in list.tiles() {
        assert_eq!(4, rectangles(tile.mesh.as_ref().unwrap()));
    }

    // One tile on its own keeps the six faces of REQ-E3D-501.
    let mut alone = surface_engine(1, 1);
    let mut single = Batch::new();
    single.bg(solid(), 0, 0, 0);
    single.convert(&mut alone, 0);
    let list = alone.build_draw_list_of(FrameRole::Converting);
    assert_eq!(1, list.tiles().len());
    assert_eq!(6, rectangles(list.tiles()[0].mesh.as_ref().unwrap()));
}

#[test]
fn sealing_is_symmetric_between_the_two_sides_of_a_joint() {
    assert_eq!(tile_sides::PLUS_X, tile_sides::opposite(tile_sides::MINUS_X));
    assert_eq!(tile_sides::MINUS_X, tile_sides::opposite(tile_sides::PLUS_X));
    assert_eq!(tile_sides::PLUS_Y, tile_sides::opposite(tile_sides::MINUS_Y));
    assert_eq!(tile_sides::MINUS_Y, tile_sides::opposite(tile_sides::PLUS_Y));

    // The Z sides are never sealed: nothing is ever against them inside one object.
    assert!(!tile_sides::is_sealed(tile_sides::ALL, 2, -1, 8));
    assert!(tile_sides::is_sealed(tile_sides::MINUS_X, 0, -1, 8));
    assert!(!tile_sides::is_sealed(tile_sides::MINUS_X, 0, 8, 8));
    assert!(tile_sides::is_sealed(tile_sides::PLUS_Y, 1, 8, 8));
}
