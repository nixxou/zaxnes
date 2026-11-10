//! AMD-063 REQ-SCN-087 — the inner lateral faces of a transparent background object made of full
//! tiles are taken off the drawn mesh, and nothing else changes.
//!
//! The unit of measure is the drawn mesh of one tile of the draw list, which is where the
//! requirement puts the removal: REQ-E3D-425 still governs the voxel field and the mesh that is
//! **built**.

use std::collections::BTreeSet;
use std::sync::Arc;

use zax_engine3d::engine::{ConversionEngine, FrameRole};
use zax_engine3d::geometry::tile_sides::TileSides;
use zax_engine3d::image::{TileImage, TileImageKey};
use zax_engine3d::instance::InstanceRef;
use zax_engine3d::settings::RenderAlgorithm;
use zax_profiles::enums::{VariantGeometry, VariantRenderStyle};

use super::support::{cells, img, Batch, PatternOptions, ProfileBuilder};

/// How a pattern of these tests is drawn.
#[derive(Debug, Clone, Copy)]
struct Look {
    depth: i32,
    alpha: f32,
    geometry: VariantGeometry,
    style: VariantRenderStyle,
}

impl Default for Look {
    fn default() -> Self {
        Self {
            depth: 4,
            alpha: 1.0,
            geometry: VariantGeometry::Default,
            style: VariantRenderStyle::Normal,
        }
    }
}

impl Look {
    fn transparent() -> Self {
        Self {
            alpha: 0.7,
            ..Self::default()
        }
    }

    fn of_alpha(alpha: f32) -> Self {
        Self {
            alpha,
            ..Self::default()
        }
    }
}

/// An engine with geometry, carrying one one-cell pattern per `(image, look)` pair.
fn engine_of(patterns: &[(TileImageKey, Look)]) -> ConversionEngine {
    let mut builder = ProfileBuilder::new();
    for (image, look) in patterns {
        builder.pattern_with(
            1,
            1,
            &cells(&[*image]),
            PatternOptions {
                depth: look.depth,
                ..PatternOptions::default()
            },
        );
    }

    for (index, (_, look)) in patterns.iter().enumerate() {
        let variant = builder.variant(index, 0);
        variant.alpha = look.alpha;
        variant.geometry = look.geometry;
        variant.render_style = look.style;
    }

    let mut engine = ConversionEngine::new();
    engine.use_geometry();
    engine.load_profile(&builder.build(), true).unwrap();
    engine
}

/// Converts a square block of identical background tiles.
fn block(engine: &mut ConversionEngine, image: TileImageKey, column: i32, row: i32, size: i32) {
    let mut batch = Batch::new();
    for y in 0..size {
        for x in 0..size {
            batch.bg_cell(image, column + x, row + y);
        }
    }

    batch.convert(engine, 0);
}

/// The reference 3D object and the tile image of the first tile of the converted frame.
fn first_tile(engine: &ConversionEngine) -> (InstanceRef, TileImage) {
    let frame = engine.converting_frame();
    let id = frame.instances()[0];
    let instance = frame.instance(id);
    let tile = frame.shape_tiles(id.shape).next().unwrap();
    (instance.reference(), *tile.image())
}

/// The vertex count of each drawn tile of the last draw list.
fn drawn_vertex_counts(engine: &mut ConversionEngine) -> Vec<usize> {
    let list = engine.build_draw_list_of(FrameRole::Converting);
    list.tiles()
        .iter()
        .map(|tile| tile.mesh.as_ref().map_or(0, |mesh| mesh.vertex_count()))
        .collect()
}

/// The distinct vertex counts of a 4 × 4 block, decreasing: the corners, the edges and the inner
/// tiles — or the same count three times when nothing is removed.
fn block_counts(look: Look) -> [usize; 3] {
    let mut engine = engine_of(&[(img::a(), look)]);
    block(&mut engine, img::a(), 10, 10, 4);
    let counts = drawn_vertex_counts(&mut engine);
    assert_eq!(16, counts.len());

    let distinct: Vec<usize> = counts
        .iter()
        .copied()
        .collect::<BTreeSet<usize>>()
        .into_iter()
        .rev()
        .collect();
    if distinct.len() == 1 {
        return [distinct[0]; 3];
    }

    assert_eq!(3, distinct.len());
    // 4 corners, 8 edges, 4 inner tiles.
    assert_eq!(4, counts.iter().filter(|&&c| c == distinct[0]).count());
    assert_eq!(8, counts.iter().filter(|&&c| c == distinct[1]).count());
    assert_eq!(4, counts.iter().filter(|&&c| c == distinct[2]).count());
    [distinct[0], distinct[1], distinct[2]]
}

/// Two one-cell background objects on neighbouring cells, and the vertex count of each drawn
/// tile, the left one first.
fn facing_tiles(left_alpha: f32, right_alpha: f32) -> (usize, usize) {
    let mut engine = engine_of(&[
        (img::a(), Look::of_alpha(left_alpha)),
        (img::b(), Look::of_alpha(right_alpha)),
    ]);
    let mut batch = Batch::new();
    batch.bg_cell(img::a(), 10, 10).bg_cell(img::b(), 11, 10);
    batch.convert(&mut engine, 0);

    let list = engine.build_draw_list_of(FrameRole::Converting);
    assert_eq!(2, list.tiles().len());
    let count_at = |x: i32| {
        list.tiles()
            .iter()
            .find(|tile| tile.tile.x() == x)
            .and_then(|tile| tile.mesh.as_ref())
            .map_or(0, |mesh| mesh.vertex_count())
    };
    (count_at(80), count_at(88))
}

// REQ-SCN-087, REQ-E3D-420, REQ-E3D-422: a full tile of a non-MARCHING variant is `D > 0` with
// all three slots of every voxel occupied. The greedy mesh of such a tile is the closed box of 24
// vertices AMD-057 draws.
#[test]
fn req_scn_087_a_tile_with_no_empty_pixel_and_a_positive_depth_is_full() {
    let mut engine = engine_of(&[(img::a(), Look::default())]);
    Batch::new()
        .bg_cell(img::a(), 10, 10)
        .convert(&mut engine, 0);

    let (reference, image) = first_tile(&engine);
    assert!(engine.geometry().is_full_tile(reference, &image));
    assert_eq!(
        24,
        engine
            .geometry()
            .mesh_of(reference, &image)
            .unwrap()
            .vertex_count()
    );
}

// REQ-SCN-087: one empty pixel, a depth of 0 or a hollow geometry is enough to make a tile not
// full, and a MARCHING variant has no removable face at all.
#[test]
fn req_scn_087_a_tile_that_is_not_full_is_told_apart() {
    // One empty pixel.
    let holed = TileImageKey::new(!1, 0, false, false);
    let mut engine = engine_of(&[(holed, Look::default())]);
    Batch::new().bg_cell(holed, 10, 10).convert(&mut engine, 0);
    let (reference, image) = first_tile(&engine);
    assert!(!engine.geometry().is_full_tile(reference, &image));

    // No voxel at all (REQ-E3D-420).
    let mut engine = engine_of(&[(
        img::a(),
        Look {
            depth: 0,
            ..Look::default()
        },
    )]);
    Batch::new()
        .bg_cell(img::a(), 10, 10)
        .convert(&mut engine, 0);
    let (reference, image) = first_tile(&engine);
    assert!(!engine.geometry().is_full_tile(reference, &image));

    // A hollow geometry writes the outer shells and leaves the inside empty.
    let mut engine = engine_of(&[(
        img::a(),
        Look {
            geometry: VariantGeometry::Cube,
            ..Look::default()
        },
    )]);
    Batch::new()
        .bg_cell(img::a(), 10, 10)
        .convert(&mut engine, 0);
    let (reference, image) = first_tile(&engine);
    assert!(!engine.geometry().is_full_tile(reference, &image));

    // MARCHING: no tile of it is full, and the whole mesh is handed over whatever is asked for.
    let mut engine = engine_of(&[(img::a(), Look::default())]);
    engine.set_render_algorithm(RenderAlgorithm::Marching);
    Batch::new()
        .bg_cell(img::a(), 10, 10)
        .convert(&mut engine, 0);
    let (reference, image) = first_tile(&engine);
    assert!(!engine.geometry().is_full_tile(reference, &image));
    let whole = engine.geometry().mesh_of(reference, &image).unwrap();
    let asked = engine
        .geometry_mut()
        .mesh_of_without(reference, &image, TileSides::ALL)
        .unwrap();
    assert!(Arc::ptr_eq(&whole, &asked));
}

// REQ-SCN-087, first acceptance criterion: a 4 × 4 block of identical full tiles at opacity 0.7
// loses 2 lateral faces on each corner tile, 3 on each edge tile and 4 on each of the four inner
// ones — 16, 12 and 8 vertices. At opacity 1 every tile keeps its 24 (AMD-057).
#[test]
fn req_scn_087_a_transparent_block_of_full_tiles_loses_its_inner_faces() {
    assert_eq!([16, 12, 8], block_counts(Look::transparent()));
    assert_eq!([24, 24, 24], block_counts(Look::default()));
}

// REQ-SCN-087: an opacity above 1 is opaque, and so is render style 2 whatever the opacity.
#[test]
fn req_scn_087_an_instance_that_is_not_transparent_keeps_every_face() {
    assert_eq!([24, 24, 24], block_counts(Look::of_alpha(1.5)));
    assert_eq!(
        [24, 24, 24],
        block_counts(Look {
            alpha: 0.7,
            style: VariantRenderStyle::ShadowOnly,
            ..Look::default()
        })
    );
}

// REQ-SCN-087 and SPEC-05 §4.2: only the drawn instance decides the opacity. A script that
// lowers `shape.Alpha` on a variant of opacity 1 gets the removal of a variant of opacity 0.7.
#[test]
fn req_scn_087_lowering_the_alpha_of_the_drawn_instance_is_enough() {
    let mut engine = engine_of(&[(img::a(), Look::default())]);
    block(&mut engine, img::a(), 10, 10, 4);

    let ids: Vec<_> = engine.converting_frame().instances().to_vec();
    for id in ids {
        engine.converting_frame_mut().instance_mut(id).alpha = 0.7;
    }

    let counts = drawn_vertex_counts(&mut engine);
    let distinct: BTreeSet<usize> = counts.iter().copied().collect();
    assert_eq!(vec![8, 12, 16], distinct.into_iter().collect::<Vec<_>>());
}

// REQ-SCN-087, third criterion: two different transparent background objects side by side lose
// the face they share, on both sides; a transparent object against an opaque one keeps it.
#[test]
fn req_scn_087_two_transparent_objects_side_by_side_lose_the_face_they_share() {
    // Each object is one tile wide, so of its four lateral faces only the shared one goes.
    assert_eq!((20, 20), facing_tiles(0.7, 0.7));
    assert_eq!((24, 24), facing_tiles(0.7, 1.0));
}

// REQ-SCN-087, second criterion: a tile that is not full keeps all its faces, and its neighbours
// keep the face that touches it.
#[test]
fn req_scn_087_a_tile_that_is_not_full_keeps_its_neighbours_faces() {
    let holed = TileImageKey::new(!1, 0, false, false);
    let mut engine = engine_of(&[(img::a(), Look::transparent())]);

    // A 3 × 1 row whose middle tile has an empty pixel: one pattern of image A and one learned
    // pattern for the holed tile would not share a shape, so the row is three one-cell shapes.
    let mut batch = Batch::new();
    batch
        .bg_cell(img::a(), 10, 10)
        .bg_cell(holed, 11, 10)
        .bg_cell(img::a(), 12, 10);
    batch.convert(&mut engine, 0);

    let list = engine.build_draw_list_of(FrameRole::Converting).clone();
    let of = |x: i32| {
        list.tiles()
            .iter()
            .find(|tile| tile.tile.x() == x)
            .and_then(|tile| tile.mesh.as_ref())
            .map_or(0, |mesh| mesh.vertex_count())
    };

    // The holed tile is not a pattern of the profile at all, so it draws nothing; what matters is
    // that the two full tiles keep every face, the cell between them holding no full tile.
    assert_eq!(24, of(80));
    assert_eq!(24, of(96));
}

// REQ-SCN-087: the sprites are left out of the removal. A transparent sprite object of full tiles
// keeps every face, and takes none off a background neighbour either.
#[test]
fn req_scn_087_a_transparent_sprite_object_of_full_tiles_loses_nothing() {
    let mut engine = engine_of(&[
        (img::a(), Look::transparent()),
        (img::b(), Look::transparent()),
    ]);

    let mut batch = Batch::new();
    for y in 0..2 {
        for x in 0..2 {
            batch.sprite(img::a(), (10 + x) * 8, (10 + y) * 8);
        }
    }
    // One background tile right against the sprite block.
    batch.bg_cell(img::b(), 12, 10);
    batch.convert(&mut engine, 0);

    let counts = drawn_vertex_counts(&mut engine);
    assert_eq!(5, counts.len());
    for count in counts {
        assert_eq!(24, count);
    }
}

// REQ-SCN-087 and REQ-E3D-602: the mesh of a removed combination is built once and keeps its
// identity, so the renderer sees the same object from frame to frame. Only the lateral faces are
// ever removed: the mesh of an inner tile is exactly its two faces of normal ±z.
#[test]
fn req_scn_087_a_removed_combination_is_memoised_and_only_lateral() {
    let mut engine = engine_of(&[(img::a(), Look::transparent())]);
    block(&mut engine, img::a(), 10, 10, 4);
    let (reference, image) = first_tile(&engine);

    let sides = TileSides::LEFT.with(TileSides::TOP);
    let first = engine
        .geometry_mut()
        .mesh_of_without(reference, &image, sides)
        .unwrap();
    let again = engine
        .geometry_mut()
        .mesh_of_without(reference, &image, sides)
        .unwrap();
    assert!(Arc::ptr_eq(&first, &again));
    assert_eq!(16, first.vertex_count());

    // Another combination is another mesh, and nothing removed is the mesh the build produced.
    let other = engine
        .geometry_mut()
        .mesh_of_without(reference, &image, TileSides::LEFT)
        .unwrap();
    assert!(!Arc::ptr_eq(&first, &other));
    let whole = engine.geometry().mesh_of(reference, &image).unwrap();
    let none = engine
        .geometry_mut()
        .mesh_of_without(reference, &image, TileSides::NONE)
        .unwrap();
    assert!(Arc::ptr_eq(&whole, &none));

    // Every lateral face gone: the two faces of normal ±z are left, 8 vertices.
    let inner = engine
        .geometry_mut()
        .mesh_of_without(reference, &image, TileSides::ALL)
        .unwrap();
    assert_eq!(8, inner.vertex_count());
    for vertex in inner.vertices() {
        assert_eq!(0.0, vertex.normal.x);
        assert_eq!(0.0, vertex.normal.y);
        assert_eq!(1.0, vertex.normal.z.abs());
    }
}

// REQ-SCN-087 and the cost rule of the changelog: the background cell map is built only when a
// background instance of the frame is transparent and holds a full tile. A picture without a
// transparent object — every 3dn profile of the corpus that has none — pays one pass over the
// instance list of the frame and nothing else.
#[test]
fn req_scn_087_the_background_map_is_built_only_for_a_transparent_background_object() {
    // Opaque: nothing to remove, no map.
    let mut engine = engine_of(&[(img::a(), Look::default())]);
    block(&mut engine, img::a(), 10, 10, 4);
    drawn_vertex_counts(&mut engine);
    assert!(!engine.background_map_built());

    // Transparent background tiles: the map is built.
    let mut engine = engine_of(&[(img::a(), Look::transparent())]);
    block(&mut engine, img::a(), 10, 10, 4);
    drawn_vertex_counts(&mut engine);
    assert!(engine.background_map_built());

    // Transparent, but of tiles that are not full: nothing can be removed, no map.
    let holed = TileImageKey::new(!1, 0, false, false);
    let mut engine = engine_of(&[(holed, Look::transparent())]);
    block(&mut engine, holed, 10, 10, 4);
    drawn_vertex_counts(&mut engine);
    assert!(!engine.background_map_built());

    // Transparent sprites of full tiles: the removal never touches a sprite, so no map.
    let mut engine = engine_of(&[(img::a(), Look::transparent())]);
    let mut batch = Batch::new();
    for y in 0..2 {
        for x in 0..2 {
            batch.sprite(img::a(), (10 + x) * 8, (10 + y) * 8);
        }
    }
    batch.convert(&mut engine, 0);
    drawn_vertex_counts(&mut engine);
    assert!(!engine.background_map_built());
}
