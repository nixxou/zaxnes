//! AMD-057: the walls between neighbouring tiles are **kept**. The drawn mesh of a tile is its
//! mesh of SPEC-04B REQ-E3D-501, every face included — no lateral wall is left out on the
//! grounds that a neighbouring tile covers it.
//!
//! This replaces the Q-067 seal (`sealed_side_tests.rs`, which AMD-057 §2 withdrew together with
//! AMD-027 §1 and §2). REQ-SCN-065 is still required and is now held by the depth of the inner
//! walls in the scene vertex shader (`zax-render`: `geometry::WALL_DEPTH_BIAS`,
//! `shaders/scene.vert`), which is what `zax-render/tests/seam_tests.rs` measures.

use zax_engine3d::engine::{ConversionEngine, FrameRole};
use zax_engine3d::geometry::greedy;
use zax_engine3d::geometry::texture::ObjectTexture;
use zax_engine3d::geometry::tile_voxels::TileVoxels;
use zax_engine3d::image::TileImageKey;
use zax_engine3d::pattern::EnginePattern;
use zax_engine3d::settings::RenderAlgorithm;

use super::support::geo::{
    self, object_3d, rectangles, rectangles_with_normal, MINUS_X, MINUS_Y, MINUS_Z, PLUS_X, PLUS_Y,
    PLUS_Z,
};
use super::support::{Batch, PatternOptions, ProfileBuilder};

/// A solid 8 × 8 tile: every lateral side is a full plane, so this is the tile the Q-067 seal
/// used to act on.
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
fn req_e3d_501_a_full_tile_is_a_closed_box_whatever_its_neighbours() {
    // The mesher has no way left to ask for anything else: `greedy::build` lost the mask.
    let (field, texture) = field_and_texture(&geo::one_tile(solid()), 8);
    let mesh = greedy::build(&field, &texture, false);

    assert_eq!(6, rectangles(&mesh));
    assert_eq!(24, mesh.vertex_count());
    assert_eq!(12, mesh.triangle_count());
    for normal in [PLUS_X, MINUS_X, PLUS_Y, MINUS_Y, PLUS_Z, MINUS_Z] {
        assert_eq!(
            1,
            rectangles_with_normal(&mesh, normal),
            "the face of normal {normal:?} is there"
        );
    }
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

/// The first acceptance criterion of AMD-057 §2, read on the draw list rather than on
/// `--dump-meshes`: a scene where two full tiles of the same object touch gives each of them
/// 24 vertices, the complete box.
#[test]
fn amd_057_two_full_tiles_of_one_object_that_touch_each_keep_the_whole_box() {
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
        let mesh = tile.mesh.as_ref().unwrap();
        assert_eq!(24, mesh.vertex_count(), "every drawn tile is the whole box");
        assert_eq!(12, mesh.triangle_count());
        assert_eq!(6, rectangles(mesh));
    }

    // A tile on its own is the same box: before AMD-057 these two cases differed (6 faces
    // alone, 4 in the middle of a surface), which is exactly what the amendment removed.
    let mut alone = surface_engine(1, 1);
    let mut single = Batch::new();
    single.bg(solid(), 0, 0, 0);
    single.convert(&mut alone, 0);
    let list = alone.build_draw_list_of(FrameRole::Converting);
    assert_eq!(1, list.tiles().len());
    assert_eq!(24, list.tiles()[0].mesh.as_ref().unwrap().vertex_count());
}

/// Every tile of one object draws the mesh the object owns, the one REQ-E3D-540 and REQ-E3D-523
/// are written against: the same `Arc`, not a variant of it.
#[test]
fn req_e3d_540_a_repeated_tile_draws_the_very_mesh_its_object_owns() {
    let mut engine = surface_engine(2, 2);
    let mut batch = Batch::new();
    batch
        .bg(solid(), 0, 0, 0)
        .bg(solid(), 8, 0, 0)
        .bg(solid(), 0, 8, 0)
        .bg(solid(), 8, 8, 0);
    batch.convert(&mut engine, 0);
    let list = engine.build_draw_list_of(FrameRole::Converting).clone();

    let first = list.tiles()[0].mesh.clone().unwrap();
    for tile in list.tiles() {
        assert!(
            std::sync::Arc::ptr_eq(&first, tile.mesh.as_ref().unwrap()),
            "the four tiles share one mesh — no variant is built"
        );
    }
}
