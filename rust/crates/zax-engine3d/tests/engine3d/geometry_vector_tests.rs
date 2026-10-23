//! The test vectors of SPEC-04B §5.1 and §5.2: extrusion and textured box
//! (`ExtrusionAndCubeVectorTests`).

use std::sync::Arc;

use zax_engine3d::geometry::tile_mesh::TileMesh;
use zax_engine3d::image::TileImageKey;
use zax_engine3d::settings::{Geo3DType, RenderAlgorithm};
use zax_engine3d::vectors::{Vec2, Vec3};
use zax_profiles::enums::TextureFormat;
use zax_profiles::variant::{TexturePixels, VariantTexture};

use super::support::geo::{
    self, assert_rectangle, close, mesh_of, object_3d, object_with, rectangles,
    rectangles_with_normal, with_normal, Object, MINUS_X, MINUS_Y, MINUS_Z, PLUS_X, PLUS_Y,
    PLUS_Z,
};

const GREEDY: RenderAlgorithm = RenderAlgorithm::Greedy;

fn cube(depth: i32) -> zax_engine3d::instance::PatternInstance {
    object_with(
        depth,
        Object {
            geometry: Geo3DType::Cube,
            ..Object::default()
        },
    )
}

fn bounds(mesh: &TileMesh, min: Vec3, max: Vec3) {
    assert!(close(min, mesh.bounds_min()), "{}", mesh.bounds_min());
    assert!(close(max, mesh.bounds_max()), "{}", mesh.bounds_max());
}

#[test]
fn tv_g1_a_single_pixel_becomes_a_prism_of_six_rectangles() {
    // REQ-E3D-421, 440, 501, 502, 504, 505, 506.
    let tile = geo::tile(&["1......."]);
    let mesh = mesh_of(tile, &object_3d(1), GREEDY);

    assert_eq!(6, rectangles(&mesh));
    bounds(&mesh, Vec3::new(-4.0, 3.0, -0.5), Vec3::new(-3.0, 4.0, 0.5));

    assert_rectangle(&mesh, MINUS_X, &[
        (-4.0, 3.0, -0.5, 0.0, 0.0),
        (-4.0, 3.0, 0.5, 1.0, 0.0),
        (-4.0, 4.0, -0.5, 0.0, 1.0),
        (-4.0, 4.0, 0.5, 1.0, 1.0),
    ]);
    assert_rectangle(&mesh, MINUS_Y, &[
        (-4.0, 3.0, -0.5, 0.0, 0.0),
        (-3.0, 3.0, -0.5, 1.0, 0.0),
        (-4.0, 3.0, 0.5, 0.0, 1.0),
        (-3.0, 3.0, 0.5, 1.0, 1.0),
    ]);
    assert_rectangle(&mesh, MINUS_Z, &[
        (-4.0, 3.0, -0.5, 0.0, 0.0),
        (-4.0, 4.0, -0.5, 0.0, 1.0),
        (-3.0, 3.0, -0.5, 1.0, 0.0),
        (-3.0, 4.0, -0.5, 1.0, 1.0),
    ]);
    assert_rectangle(&mesh, PLUS_X, &[
        (-3.0, 3.0, -0.5, 0.0, 0.0),
        (-3.0, 3.0, 0.5, 1.0, 0.0),
        (-3.0, 4.0, -0.5, 0.0, 1.0),
        (-3.0, 4.0, 0.5, 1.0, 1.0),
    ]);
    assert_rectangle(&mesh, PLUS_Y, &[
        (-4.0, 4.0, -0.5, 0.0, 0.0),
        (-3.0, 4.0, -0.5, 1.0, 0.0),
        (-4.0, 4.0, 0.5, 0.0, 1.0),
        (-3.0, 4.0, 0.5, 1.0, 1.0),
    ]);
    assert_rectangle(&mesh, PLUS_Z, &[
        (-4.0, 3.0, 0.5, 0.0, 0.0),
        (-4.0, 4.0, 0.5, 0.0, 1.0),
        (-3.0, 3.0, 0.5, 1.0, 0.0),
        (-3.0, 4.0, 0.5, 1.0, 1.0),
    ]);

    // Same pixel with D = 4: the same six rectangles, z in [−2, 2].
    let deeper = mesh_of(tile, &object_3d(4), GREEDY);
    assert_eq!(6, rectangles(&deeper));
    bounds(&deeper, Vec3::new(-4.0, 3.0, -2.0), Vec3::new(-3.0, 4.0, 2.0));
}

#[test]
fn tv_g2_greedy_ignores_colour_and_multicolour_does_not() {
    // REQ-E3D-502, 503, 506.
    let tile = geo::tile(&["12......"]);

    let greedy = mesh_of(tile, &object_3d(2), GREEDY);
    assert_eq!(6, rectangles(&greedy));
    bounds(&greedy, Vec3::new(-4.0, 3.0, -1.0), Vec3::new(-2.0, 4.0, 1.0));
    assert_rectangle(&greedy, MINUS_Z, &[
        (-4.0, 3.0, -1.0, 0.0, 0.0),
        (-4.0, 4.0, -1.0, 0.0, 1.0),
        (-2.0, 3.0, -1.0, 2.0, 0.0),
        (-2.0, 4.0, -1.0, 2.0, 1.0),
    ]);
    assert_rectangle(&greedy, PLUS_X, &[
        (-2.0, 3.0, -1.0, 1.0, 0.0),
        (-2.0, 3.0, 1.0, 2.0, 0.0),
        (-2.0, 4.0, -1.0, 1.0, 1.0),
        (-2.0, 4.0, 1.0, 2.0, 1.0),
    ]);

    let multicolor = mesh_of(tile, &object_3d(2), RenderAlgorithm::GreedyMulticolor);
    assert_eq!(10, rectangles(&multicolor));
    assert_eq!(1, rectangles_with_normal(&multicolor, MINUS_X));
    assert_eq!(1, rectangles_with_normal(&multicolor, PLUS_X));
    assert_eq!(2, rectangles_with_normal(&multicolor, MINUS_Y));
    assert_eq!(2, rectangles_with_normal(&multicolor, PLUS_Y));
    assert_eq!(2, rectangles_with_normal(&multicolor, MINUS_Z));
    assert_eq!(2, rectangles_with_normal(&multicolor, PLUS_Z));

    // The two halves of the −Z face, uv (0,0)–(1,1) then (1,0)–(2,1).
    let minus_z = with_normal(&multicolor, MINUS_Z);
    assert!(minus_z.contains(&(Vec3::new(-4.0, 3.0, -1.0), Vec2::new(0.0, 0.0))));
    assert!(minus_z.contains(&(Vec3::new(-3.0, 4.0, -1.0), Vec2::new(1.0, 1.0))));
    assert!(minus_z.contains(&(Vec3::new(-3.0, 3.0, -1.0), Vec2::new(1.0, 0.0))));
    assert!(minus_z.contains(&(Vec3::new(-2.0, 4.0, -1.0), Vec2::new(2.0, 1.0))));
}

#[test]
fn tv_g3_a_full_tile_is_six_rectangles_whatever_the_colours() {
    // REQ-E3D-502: GREEDY merges across colours.
    let mesh = mesh_of(geo::solid(1), &object_3d(8), GREEDY);
    assert_eq!(6, rectangles(&mesh));
    bounds(&mesh, Vec3::new(-4.0, -4.0, -4.0), Vec3::new(4.0, 4.0, 4.0));

    let striped = mesh_of(geo::rows("12121212"), &object_3d(2), GREEDY);
    assert_eq!(6, rectangles(&striped));
}

#[test]
fn tv_g4_an_l_shape_gives_ten_rectangles() {
    // REQ-E3D-502, 506.
    let mesh = mesh_of(geo::tile(&["11......", "1......."]), &object_3d(1), GREEDY);

    assert_eq!(10, rectangles(&mesh));
    assert_eq!(1, rectangles_with_normal(&mesh, MINUS_X));
    assert_eq!(2, rectangles_with_normal(&mesh, MINUS_Y));
    assert_eq!(2, rectangles_with_normal(&mesh, MINUS_Z));
    assert_eq!(2, rectangles_with_normal(&mesh, PLUS_X));
    assert_eq!(1, rectangles_with_normal(&mesh, PLUS_Y));
    assert_eq!(2, rectangles_with_normal(&mesh, PLUS_Z));

    assert_rectangle(&mesh, MINUS_X, &[
        (-4.0, 2.0, -0.5, 0.0, 2.0),
        (-4.0, 2.0, 0.5, 1.0, 2.0),
        (-4.0, 4.0, -0.5, 0.0, 0.0),
        (-4.0, 4.0, 0.5, 1.0, 0.0),
    ]);
    assert_rectangle(&mesh, PLUS_Y, &[
        (-4.0, 4.0, -0.5, 0.0, 0.0),
        (-2.0, 4.0, -0.5, 2.0, 0.0),
        (-4.0, 4.0, 0.5, 0.0, 1.0),
        (-2.0, 4.0, 0.5, 2.0, 1.0),
    ]);
}

#[test]
fn tv_g5_empty_colour_four_makes_every_pixel_opaque() {
    // REQ-E3D-424: e = 4 means none of the colours 0 to 3 is transparent.
    let mesh = mesh_of(
        geo::tile(&["10......"]),
        &object_with(
            1,
            Object {
                empty_color: 4,
                ..Object::default()
            },
        ),
        GREEDY,
    );

    assert_eq!(6, rectangles(&mesh));
    bounds(&mesh, Vec3::new(-4.0, -4.0, -0.5), Vec3::new(4.0, 4.0, 0.5));
}

#[test]
fn tv_g6_full_warp_puts_the_side_faces_on_the_back_half() {
    // REQ-E3D-441.
    let mesh = mesh_of(
        geo::tile(&["1......."]),
        &object_with(
            2,
            Object {
                full_warp: true,
                ..Object::default()
            },
        ),
        GREEDY,
    );

    assert_eq!(6, rectangles(&mesh));

    // ±Z cover z = ∓1 over the pixel.
    assert_rectangle(&mesh, MINUS_Z, &[
        (-4.0, 3.0, -1.0, 0.0, 0.0),
        (-4.0, 4.0, -1.0, 0.0, 1.0),
        (-3.0, 3.0, -1.0, 1.0, 0.0),
        (-3.0, 4.0, -1.0, 1.0, 1.0),
    ]);

    // ±X and ±Y only exist on the back half, z in [0, 1].
    assert_rectangle(&mesh, MINUS_X, &[
        (-4.0, 3.0, 0.0, 0.0, 0.0),
        (-4.0, 3.0, 1.0, 1.0, 0.0),
        (-4.0, 4.0, 0.0, 0.0, 1.0),
        (-4.0, 4.0, 1.0, 1.0, 1.0),
    ]);
}

#[test]
fn tv_g7_two_different_full_tiles_keep_the_wall_between_them() {
    // REQ-E3D-501 notes: each tile mesh is closed on itself.
    let pattern = geo::pattern(2, 1, &[Some(geo::solid(1)), Some(geo::solid(2))]);
    let instance = object_3d(1);

    assert_eq!(2, pattern.images().len());
    for image in 0..2 {
        let mesh = geo::mesh(&pattern, &instance, GREEDY, image);
        assert_eq!(6, rectangles(&mesh));
        bounds(&mesh, Vec3::new(-4.0, -4.0, -0.5), Vec3::new(4.0, 4.0, 0.5));
        assert_eq!(1, rectangles_with_normal(&mesh, PLUS_X));
        assert_eq!(1, rectangles_with_normal(&mesh, MINUS_X));
    }
}

#[test]
fn tv_g8_multicolour_stripes_give_thirty_four_rectangles() {
    // REQ-E3D-503.
    let mesh = mesh_of(geo::rows("12121212"), &object_3d(2), RenderAlgorithm::GreedyMulticolor);

    assert_eq!(34, rectangles(&mesh));
    assert_eq!(1, rectangles_with_normal(&mesh, MINUS_X));
    assert_eq!(1, rectangles_with_normal(&mesh, PLUS_X));
    assert_eq!(8, rectangles_with_normal(&mesh, MINUS_Y));
    assert_eq!(8, rectangles_with_normal(&mesh, PLUS_Y));
    assert_eq!(8, rectangles_with_normal(&mesh, MINUS_Z));
    assert_eq!(8, rectangles_with_normal(&mesh, PLUS_Z));
}

#[test]
fn tv_c1_a_cube_of_a_full_tile_has_twelve_rectangles() {
    // REQ-E3D-443: six outer faces and six inner ones.
    let mesh = mesh_of(geo::solid(1), &cube(4), GREEDY);

    assert_eq!(12, rectangles(&mesh));
    for normal in [MINUS_X, PLUS_X, MINUS_Y, PLUS_Y, MINUS_Z, PLUS_Z] {
        assert_eq!(2, rectangles_with_normal(&mesh, normal));
    }

    bounds(&mesh, Vec3::new(-4.0, -4.0, -2.0), Vec3::new(4.0, 4.0, 2.0));
}

#[test]
fn tv_c2_a_cube_of_a_ring_has_forty_rectangles() {
    // REQ-E3D-443 with transparent texels inside the box.
    let ring = geo::tile(&[
        "11111111",
        "1......1",
        "1......1",
        "1......1",
        "1......1",
        "1......1",
        "1......1",
        "11111111",
    ]);
    let mesh = mesh_of(ring, &cube(4), GREEDY);

    assert_eq!(40, rectangles(&mesh));
    assert_eq!(6, rectangles_with_normal(&mesh, MINUS_X));
    assert_eq!(6, rectangles_with_normal(&mesh, PLUS_X));
    assert_eq!(6, rectangles_with_normal(&mesh, MINUS_Y));
    assert_eq!(6, rectangles_with_normal(&mesh, PLUS_Y));
    assert_eq!(8, rectangles_with_normal(&mesh, MINUS_Z));
    assert_eq!(8, rectangles_with_normal(&mesh, PLUS_Z));
}

#[test]
fn tv_c3_two_cells_of_the_same_tile_share_one_mesh_with_both_walls() {
    // REQ-E3D-425: the shared field receives the writes of both cells.
    let shared = geo::pattern(2, 1, &[Some(geo::solid(1)), Some(geo::solid(1))]);
    let instance = cube(2);

    assert_eq!(1, shared.images().len());
    let mesh = geo::mesh(&shared, &instance, GREEDY, 0);
    assert_eq!(10, rectangles(&mesh));
    assert_eq!(2, rectangles_with_normal(&mesh, MINUS_X));
    assert_eq!(2, rectangles_with_normal(&mesh, PLUS_X));
    assert_eq!(2, rectangles_with_normal(&mesh, MINUS_Y));
    assert_eq!(2, rectangles_with_normal(&mesh, PLUS_Y));
    assert_eq!(1, rectangles_with_normal(&mesh, MINUS_Z));
    assert_eq!(1, rectangles_with_normal(&mesh, PLUS_Z));

    // Both walls are present: x = −4 and x = +4.
    let left = with_normal(&mesh, MINUS_X);
    assert!(left.iter().any(|(position, _)| (position.x + 4.0).abs() < 1e-3));
    assert!(left.iter().any(|(position, _)| (position.x - 3.0).abs() < 1e-3));

    // Two different tiles instead: two meshes of eight rectangles each.
    let distinct = geo::pattern(2, 1, &[Some(geo::solid(1)), Some(geo::solid(2))]);
    assert_eq!(2, distinct.images().len());
    for image in 0..2 {
        let one = geo::mesh(&distinct, &instance, GREEDY, image);
        assert_eq!(8, rectangles(&one));
        assert_eq!(1, rectangles_with_normal(&one, MINUS_X));
        assert_eq!(1, rectangles_with_normal(&one, PLUS_X));
        assert_eq!(2, rectangles_with_normal(&one, MINUS_Y));
        assert_eq!(2, rectangles_with_normal(&one, PLUS_Y));
    }
}

#[test]
fn tv_c4_a_cube_of_one_opaque_row_has_ten_rectangles() {
    // REQ-E3D-443, 506: the side faces show the columns D − 1 to 0.
    let mesh = mesh_of(geo::tile(&["11111111"]), &cube(2), GREEDY);

    assert_eq!(10, rectangles(&mesh));
    assert_eq!(2, rectangles_with_normal(&mesh, MINUS_X));
    assert_eq!(2, rectangles_with_normal(&mesh, PLUS_X));
    assert_eq!(2, rectangles_with_normal(&mesh, MINUS_Y));
    assert_eq!(2, rectangles_with_normal(&mesh, PLUS_Y));
    assert_eq!(1, rectangles_with_normal(&mesh, MINUS_Z));
    assert_eq!(1, rectangles_with_normal(&mesh, PLUS_Z));

    // −X at x = −4: y in [3, 4], z in [−1, 1], uv (2,0), (0,0), (2,1), (0,1).
    let minus_x = with_normal(&mesh, MINUS_X);
    assert!(minus_x.contains(&(Vec3::new(-4.0, 3.0, -1.0), Vec2::new(2.0, 0.0))));
    assert!(minus_x.contains(&(Vec3::new(-4.0, 3.0, 1.0), Vec2::new(0.0, 0.0))));
    assert!(minus_x.contains(&(Vec3::new(-4.0, 4.0, -1.0), Vec2::new(2.0, 1.0))));
    assert!(minus_x.contains(&(Vec3::new(-4.0, 4.0, 1.0), Vec2::new(0.0, 1.0))));

    // The bottom face only shows row 0 of the texture: z in [−1, 0].
    let minus_y = with_normal(&mesh, MINUS_Y);
    let has = |point: Vec3| minus_y.iter().any(|(position, _)| close(*position, point));
    assert!(has(Vec3::new(-4.0, -4.0, -1.0)));
    assert!(has(Vec3::new(4.0, -4.0, 0.0)));

    // The top face shows the same rows the other way round: z in [0, 1].
    assert!(has(Vec3::new(-4.0, 3.0, 0.0)));
    assert!(has(Vec3::new(4.0, 3.0, 1.0)));
}

#[test]
fn req_e3d_420_a_depth_of_zero_produces_no_geometry() {
    let mesh = mesh_of(geo::solid(1), &object_3d(0), GREEDY);
    assert!(mesh.is_empty());
    assert_eq!(0, mesh.triangle_count());
}

#[test]
fn req_e3d_442_marching_full_warp_takes_the_lowest_opaque_texel() {
    // Column of three pixels of colours 1, 2, 3 top to bottom, D = 3.
    let pattern = geo::one_tile(geo::tile(&["1.......", "2.......", "3......."]));
    let instance = object_with(
        3,
        Object {
            full_warp: true,
            ..Object::default()
        },
    );
    let (fields, _) = geo::fields(&pattern, &instance, RenderAlgorithm::Marching);
    let texture = geo::texture_of(&pattern, &instance);

    // The slice k = 1 of the top pixel takes the colour 2, the slice k = 2 the colour 3.
    let (tx1, ty1) = fields[0].texel(0, 7, 1, 0).unwrap();
    assert_eq!(2, texture.texel(tx1, ty1));
    let (tx2, ty2) = fields[0].texel(0, 7, 2, 0).unwrap();
    assert_eq!(3, texture.texel(tx2, ty2));
}

fn modulo(value: f32, size: f32) -> f32 {
    let result = value % size;
    if result < 0.0 {
        result + size
    } else {
        result
    }
}

fn build_solid_indexed(width: usize, height: usize, value: i32) -> Vec<i32> {
    let mut data = vec![0; (width * height).div_ceil(16)];
    for n in 0..width * height {
        data[n / 16] |= (value & 3) << (2 * (n % 16));
    }
    data
}

#[test]
fn req_e3d_426_a_texture_smaller_than_the_pattern_repeats_over_it() {
    // An 8 x 8 texture on an 8 x 192 pattern of 24 distinct tiles: the 24 meshes carry the
    // same texture coordinates modulo 8, each block of 8 rows reading the same texture.
    let cells: Vec<Option<TileImageKey>> = (0..24)
        .map(|n| {
            // Distinct images: a full tile of colour 1 with one marker pixel of colour 2.
            let rows: Vec<&str> = (0..8)
                .map(|y| if y == n % 8 { "21111111" } else { "11111111" })
                .collect();
            let key = geo::tile(&rows);
            Some(TileImageKey {
                flip_x: (8..16).contains(&n),
                flip_y: n >= 16,
                ..key
            })
        })
        .collect();

    let pattern = geo::pattern(1, 24, &cells);
    assert_eq!(24, pattern.images().len());

    let mut instance = object_3d(1);
    instance.texture = Some(Arc::new(VariantTexture::new(
        8,
        8,
        Some(TexturePixels::new(TextureFormat::Indexed, build_solid_indexed(8, 8, 1), Vec::new())),
    )));

    let texture = geo::texture_of(&pattern, &instance);
    assert_eq!(8, texture.width());
    assert_eq!(8, texture.height());

    let mut reference: Option<Vec<String>> = None;
    for image in 0..pattern.images().len() {
        let mesh = geo::mesh(&pattern, &instance, GREEDY, image);

        // The texture is uniform, so every pixel is opaque and the tile is a full prism.
        assert_eq!(6, rectangles(&mesh));

        let mut reduced: Vec<String> = mesh
            .vertices()
            .iter()
            .map(|vertex| {
                format!(
                    "{} {} {}",
                    vertex.position,
                    modulo(vertex.uv.x, 8.0),
                    modulo(vertex.uv.y, 8.0)
                )
            })
            .collect();
        reduced.sort();
        match &reference {
            None => reference = Some(reduced),
            Some(expected) => assert_eq!(expected, &reduced),
        }
    }
}
