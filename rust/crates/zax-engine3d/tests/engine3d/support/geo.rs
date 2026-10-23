//! Builds the little patterns and 3D objects the test vectors of SPEC-04B §5 describe, and
//! reads the results back in the shape those vectors use (port of `GeometrySupport.cs`).
//!
//! Conventions of §5: a tile image is given as eight rows, row 0 at the top, `.` for the
//! colour 0 and a digit for the colours 1 to 3. Unless said otherwise a vector uses a one-tile
//! pattern, `emptyColor = 0`, `GREEDY` and the pattern image as texture.

use zax_engine3d::geometry::marching;
use zax_engine3d::geometry::pattern_voxels::PatternVoxels;
use zax_engine3d::geometry::texture::ObjectTexture;
use zax_engine3d::geometry::tile_mesh::TileMesh;
use zax_engine3d::geometry::tile_voxels::{TileVoxels, AXIS_X, SIZE_X, SIZE_Y};
use zax_engine3d::geometry::{greedy, voxel_construction};
use zax_engine3d::image::{ImageRegistry, TileImageKey};
use zax_engine3d::instance::PatternInstance;
use zax_engine3d::palette::nes_color;
use zax_engine3d::pattern::EnginePattern;
use zax_engine3d::settings::{Geo3DType, RenderAlgorithm};
use zax_engine3d::vectors::{Vec2, Vec3};

/// A tile image given row by row, row 0 at the top (§5).
pub fn tile(rows: &[&str]) -> TileImageKey {
    let mut low = 0u64;
    let mut high = 0u64;
    for (y, row) in rows.iter().take(8).enumerate() {
        let bytes = row.as_bytes();
        for x in 0..8 {
            let color = match bytes.get(x) {
                Some(&c) if c != b'.' => c - b'0',
                _ => 0,
            };
            let bit = (y << 3) | (7 - x);
            if color & 1 != 0 {
                low |= 1 << bit;
            }

            if color & 2 != 0 {
                high |= 1 << bit;
            }
        }
    }

    TileImageKey::new(low, high, false, false)
}

/// A tile whose eight rows are all `row`.
pub fn rows(row: &str) -> TileImageKey {
    tile(&[row; 8])
}

/// A full tile of the colour `color`.
pub fn solid(color: u8) -> TileImageKey {
    rows(&char::from(b'0' + color).to_string().repeat(8))
}

/// The empty tile.
pub fn blank() -> TileImageKey {
    tile(&[])
}

/// A pattern of `width` × `height` cells, row by row; a `None` entry is an empty cell.
pub fn pattern(width: u16, height: u16, cells: &[Option<TileImageKey>]) -> EnginePattern {
    let mut registry = ImageRegistry::new();
    let images: Vec<_> = cells
        .iter()
        .map(|cell| cell.map(|key| registry.intern(key)))
        .collect();
    EnginePattern::new(width, height, images, true)
}

/// A one-cell pattern carrying `tile`.
pub fn one_tile(tile: TileImageKey) -> EnginePattern {
    pattern(1, 1, &[Some(tile)])
}

/// The options of a 3D object, with the defaults of REQ-E3D-453.
#[derive(Debug, Clone, Copy, Default)]
#[allow(clippy::struct_excessive_bools)] // The flags of SPEC-04 §4.3.
pub struct Object {
    pub geometry: Geo3DType,
    pub solid: bool,
    pub full_warp: bool,
    pub flip: bool,
    pub anti_shadow: bool,
    pub empty_color: i32,
}

/// A 3D object with the defaults of REQ-E3D-453 and the given depth.
pub fn object_3d(depth: i32) -> PatternInstance {
    object_with(depth, Object::default())
}

/// A 3D object with the given depth and options.
pub fn object_with(depth: i32, options: Object) -> PatternInstance {
    let mut instance = PatternInstance::new();
    instance.depth = depth;
    instance.geometry = options.geometry;
    instance.solid = options.solid;
    instance.full_warp = options.full_warp;
    instance.flip = options.flip;
    instance.anti_shadow = options.anti_shadow;
    instance.empty_color = options.empty_color;
    instance
}

/// The texture of a 3D object of `pattern` (REQ-E3D-426).
pub fn texture_of(pattern: &EnginePattern, instance: &PatternInstance) -> ObjectTexture {
    ObjectTexture::for_object(pattern, instance.texture.as_deref(), nes_color(0x0F))
}

/// Builds the voxel fields of a 3D object and reports its effective depth.
pub fn fields(
    pattern: &EnginePattern,
    instance: &PatternInstance,
    algorithm: RenderAlgorithm,
) -> (Vec<TileVoxels>, i32) {
    let built =
        voxel_construction::build(pattern, &texture_of(pattern, instance), instance, algorithm);
    (built.fields, built.effective_depth)
}

/// Builds the mesh of one tile image of a 3D object.
pub fn mesh(
    pattern: &EnginePattern,
    instance: &PatternInstance,
    algorithm: RenderAlgorithm,
    image_index: usize,
) -> TileMesh {
    let texture = texture_of(pattern, instance);
    let built = voxel_construction::build(pattern, &texture, instance, algorithm);
    if algorithm == RenderAlgorithm::Marching {
        let voxels = PatternVoxels::new(pattern, &texture, &built.fields);
        return marching::build(
            &voxels,
            image_index,
            ObjectTexture::empty_color(instance.empty_color),
        );
    }

    greedy::build(
        &built.fields[image_index],
        &texture,
        algorithm == RenderAlgorithm::GreedyMulticolor,
        0,
    )
}

/// The mesh of a one-tile pattern, the usual case of §5.
pub fn mesh_of(
    tile: TileImageKey,
    instance: &PatternInstance,
    algorithm: RenderAlgorithm,
) -> TileMesh {
    mesh(&one_tile(tile), instance, algorithm, 0)
}

/// Number of rectangles of a GREEDY mesh: each one is four vertices, two triangles.
pub fn rectangles(mesh: &TileMesh) -> usize {
    mesh.triangle_count() / 2
}

/// The vertices carrying the normal `normal`, as (position, uv) pairs. Vertices are never
/// shared, so a GREEDY rectangle contributes exactly four of them.
pub fn with_normal(mesh: &TileMesh, normal: Vec3) -> Vec<(Vec3, Vec2)> {
    mesh.vertices()
        .iter()
        .filter(|vertex| vertex.normal == normal)
        .map(|vertex| (vertex.position, vertex.uv))
        .collect()
}

/// Number of rectangles whose normal is `normal`.
pub fn rectangles_with_normal(mesh: &TileMesh, normal: Vec3) -> usize {
    with_normal(mesh, normal).len() / 4
}

/// The six unit normals, in the listing order of §5.
pub const MINUS_X: Vec3 = Vec3::new(-1.0, 0.0, 0.0);
pub const PLUS_X: Vec3 = Vec3::new(1.0, 0.0, 0.0);
pub const MINUS_Y: Vec3 = Vec3::new(0.0, -1.0, 0.0);
pub const PLUS_Y: Vec3 = Vec3::new(0.0, 1.0, 0.0);
pub const MINUS_Z: Vec3 = Vec3::new(0.0, 0.0, -1.0);
pub const PLUS_Z: Vec3 = Vec3::new(0.0, 0.0, 1.0);

/// Asserts that the rectangle of normal `normal` is exactly the four (position, uv) pairs
/// given, in the order the vectors of §5 use: BL, TL, BR, TR. The comparison ignores the
/// order of the vertices (REQ-E3D-523).
pub fn assert_rectangle(mesh: &TileMesh, normal: Vec3, corners: &[(f32, f32, f32, f32, f32)]) {
    let actual = with_normal(mesh, normal);
    assert_eq!(4, actual.len());
    for &(x, y, z, u, v) in corners {
        let position = Vec3::new(x, y, z);
        let uv = Vec2::new(u, v);
        assert!(
            actual
                .iter()
                .any(|&(p, t)| close(p, position) && close_uv(t, uv)),
            "no vertex {position} uv {uv} with normal {normal}; got {actual:?}"
        );
    }
}

/// The voxels of one line of a cylindrical section, as the vectors of §5.3 list them:
/// `(i, k)` and the texel column for the V types, `(j, k)` and the texel row for the H types.
pub fn section(field: &TileVoxels, line: i32, vertical: bool) -> Vec<(i32, i32, i32)> {
    let mut found = Vec::new();
    for i in 0..SIZE_X {
        for j in 0..SIZE_Y {
            for k in 0..field.depth() {
                let Some((tx, ty)) = field.texel(i, j, k, AXIS_X) else {
                    continue;
                };

                if vertical {
                    // V types keep the whole section in the row of the line.
                    assert_eq!(7 - (line & 7), j);
                    found.push((i, k, tx));
                } else {
                    assert_eq!(line & 7, i);
                    found.push((j, k, ty));
                }
            }
        }
    }

    found.sort_unstable();
    found
}

/// Formats the expected voxel list of a §5.3 vector.
pub fn voxels(list: &[(i32, i32, i32)]) -> Vec<(i32, i32, i32)> {
    let mut sorted = list.to_vec();
    sorted.sort_unstable();
    sorted
}

/// True when the two positions agree to 10⁻³ (REQ-E3D-523).
pub fn close(a: Vec3, b: Vec3) -> bool {
    (a.x - b.x).abs() < 1e-3 && (a.y - b.y).abs() < 1e-3 && (a.z - b.z).abs() < 1e-3
}

/// True when the two texture coordinates agree to 10⁻³ texel (REQ-E3D-523).
pub fn close_uv(a: Vec2, b: Vec2) -> bool {
    (a.x - b.x).abs() < 1e-3 && (a.y - b.y).abs() < 1e-3
}
