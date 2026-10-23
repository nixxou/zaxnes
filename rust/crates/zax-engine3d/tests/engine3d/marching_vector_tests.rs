//! The test vectors of SPEC-04B §5.4: the smoothed surfaces of MARCHING (REQ-E3D-520 to 524).

use zax_engine3d::geometry::greedy;
use zax_engine3d::geometry::texture::ObjectTexture;
use zax_engine3d::geometry::tile_mesh::TileMesh;
use zax_engine3d::geometry::tile_voxels::{TileVoxels, MAX_DEPTH, SIZE_X, SIZE_Y};
use zax_engine3d::image::TileImageKey;
use zax_engine3d::settings::RenderAlgorithm;
use zax_engine3d::vectors::{Vec2, Vec3};

use super::support::assert_close;
use super::support::geo::{
    self, close, close_uv, mesh_of, object_3d, object_with, rectangles, Object,
};

const MARCHING: RenderAlgorithm = RenderAlgorithm::Marching;

fn corners(mesh: &TileMesh, n: usize) -> [Vec3; 3] {
    let triangles = mesh.triangles();
    [0, 1, 2].map(|c| mesh.vertices()[triangles[n + c] as usize].position)
}

fn normal_of(mesh: &TileMesh, n: usize) -> Vec3 {
    mesh.vertices()[mesh.triangles()[n] as usize].normal
}

#[test]
fn tv_m1_a_single_pixel_becomes_a_regular_octahedron() {
    // REQ-E3D-521, 522.
    let mesh = mesh_of(geo::tile(&["1......."]), &object_3d(1), MARCHING);

    assert_eq!(8, mesh.triangle_count());
    assert_eq!(24, mesh.vertex_count());

    // Vertices of the octahedron centred on (−3.5, 3.5, 0), "radius" 1. The outward
    // orientation of the normals is the gradient rule of Q-016.
    let expected = [
        Vec3::new(-4.5, 3.5, 0.0),
        Vec3::new(-2.5, 3.5, 0.0),
        Vec3::new(-3.5, 2.5, 0.0),
        Vec3::new(-3.5, 4.5, 0.0),
        Vec3::new(-3.5, 3.5, -1.0),
        Vec3::new(-3.5, 3.5, 1.0),
    ];

    let root = 1.0 / 3f32.sqrt();
    for vertex in mesh.vertices() {
        assert!(expected
            .iter()
            .any(|&candidate| close(candidate, vertex.position)));

        // uv (0.5, 0.5) for every vertex, and normals (±1, ±1, ±1)/sqrt(3).
        assert!(close_uv(Vec2::new(0.5, 0.5), vertex.uv));
        assert_close(root, vertex.normal.x.abs(), 3);
        assert_close(root, vertex.normal.y.abs(), 3);
        assert_close(root, vertex.normal.z.abs(), 3);
    }

    // One listed triangle, with its outward normal.
    assert_triangle(
        &mesh,
        [
            Vec3::new(-4.5, 3.5, 0.0),
            Vec3::new(-3.5, 3.5, -1.0),
            Vec3::new(-3.5, 2.5, 0.0),
        ],
        Vec3::new(-1.0, -1.0, -1.0) * root,
    );

    // Every normal looks away from the centre of the octahedron.
    let centre = Vec3::new(-3.5, 3.5, 0.0);
    for vertex in mesh.vertices() {
        let outward = vertex.position - centre;
        assert!(
            Vec3::dot(outward, vertex.normal) > 0.0,
            "inward normal {} at {}",
            vertex.normal,
            vertex.position
        );
    }
}

#[test]
fn tv_m2_two_adjacent_pixels_of_one_colour_give_sixteen_triangles() {
    let mesh = mesh_of(geo::tile(&["11......"]), &object_3d(1), MARCHING);

    assert_eq!(16, count_non_degenerate(&mesh));
    assert!(close(Vec3::new(-4.5, 2.5, -1.0), mesh.bounds_min()));
    assert!(close(Vec3::new(-1.5, 4.5, 1.0), mesh.bounds_max()));
}

#[test]
fn tv_m3_a_full_tile_gives_a_closed_surface_that_joins_its_neighbour() {
    // REQ-E3D-520: the neighbourhood is read in the next tile of the pattern.
    let alone = mesh_of(geo::solid(1), &object_3d(2), MARCHING);

    assert_eq!(380, count_non_degenerate(&alone));
    assert!(close(Vec3::new(-4.5, -4.5, -1.5), alone.bounds_min()));
    assert!(close(Vec3::new(4.5, 4.5, 1.5), alone.bounds_max()));

    // Two full tiles side by side, distinct as images but identical in content — a flip of a
    // symmetric tile is another image (SPEC-04 §2) — so each has its own mesh and its own
    // neighbourhood. Neither keeps a face in the junction plane.
    let flipped = TileImageKey {
        flip_x: true,
        ..geo::solid(1)
    };
    let pair = geo::pattern(2, 1, &[Some(geo::solid(1)), Some(flipped)]);
    assert_eq!(2, pair.images().len());

    let instance = object_3d(2);
    let left = geo::mesh(&pair, &instance, MARCHING, 0);
    let right = geo::mesh(&pair, &instance, MARCHING, 1);

    // A lone tile closes itself with a wall at x = 4.5; the left tile of the pair has none,
    // and neither has the right tile at x = −4.5. The caps at the front and the back still
    // reach those planes, so what counts is a triangle lying entirely in them.
    assert!(has_wall_at(&alone, 0, 4.5));
    assert!(!has_wall_at(&left, 0, 4.5));
    assert!(!has_wall_at(&right, 0, -4.5));

    // The left tile keeps its own outer walls.
    assert!(has_wall_at(&left, 0, -4.5));
    assert!(count_non_degenerate(&left) < count_non_degenerate(&alone));
}

/// True when at least one triangle lies entirely in the plane `axis = coordinate`, which is
/// what a wall between two tiles looks like.
fn has_wall_at(mesh: &TileMesh, axis: usize, coordinate: f32) -> bool {
    (0..mesh.triangles().len()).step_by(3).any(|n| {
        let wall = corners(mesh, n).iter().all(|position| {
            let value = match axis {
                0 => position.x,
                1 => position.y,
                _ => position.z,
            };
            (value - coordinate).abs() < 1e-3
        });
        wall && normal_of(mesh, n) != Vec3::ZERO
    })
}

#[test]
fn tv_m4_an_opaque_empty_colour_adds_the_legacy_zero_surface() {
    // REQ-E3D-521 notes: with e != 0 the empty corners count as the value 0 and produce their
    // own surface; here eight of its triangles have zero area.
    let mesh = mesh_of(
        geo::tile(&["1......."]),
        &object_with(
            1,
            Object {
                empty_color: 4,
                ..Object::default()
            },
        ),
        MARCHING,
    );

    assert_eq!(16, mesh.triangle_count());
    assert_eq!(8, count_non_degenerate(&mesh));
}

#[test]
fn req_e3d_524_the_mesher_accepts_more_than_sixty_five_thousand_vertices() {
    // A 3D checkerboard injected straight into the field: 18 432 rectangles, 73 728 vertices.
    let mut field = TileVoxels::new(MAX_DEPTH, true);
    for k in 0..MAX_DEPTH {
        for j in 0..SIZE_Y {
            for i in 0..SIZE_X {
                if (i + j + k) % 2 == 0 {
                    field.write_all(i, j, k, 0, 0, false);
                }
            }
        }
    }

    let pattern = geo::one_tile(geo::solid(1));
    let mesh = greedy::build(&field, &ObjectTexture::from_pattern(&pattern), false, 0);

    assert_eq!(18432, rectangles(&mesh));
    assert_eq!(73728, mesh.vertex_count());
    assert_eq!(36864, mesh.triangle_count());
}

fn count_non_degenerate(mesh: &TileMesh) -> usize {
    (0..mesh.triangles().len())
        .step_by(3)
        .filter(|&n| normal_of(mesh, n) != Vec3::ZERO)
        .count()
}

fn assert_triangle(mesh: &TileMesh, expected: [Vec3; 3], normal: Vec3) {
    for n in (0..mesh.triangles().len()).step_by(3) {
        let actual = corners(mesh, n);
        if !expected
            .iter()
            .all(|&corner| actual.iter().any(|&candidate| close(candidate, corner)))
        {
            continue;
        }

        let found = normal_of(mesh, n);
        assert_close(normal.x, found.x, 3);
        assert_close(normal.y, found.y, 3);
        assert_close(normal.z, found.z, 3);
        return;
    }

    panic!("no triangle with the corners {expected:?}");
}
