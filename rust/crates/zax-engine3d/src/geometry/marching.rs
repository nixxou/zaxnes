// Portions adapted from 3DNes_Engine (c) Geod Studio, GPL-3.0

//! Turns a voxel field into a smoothed surface by marching cubes, one surface per colour
//! (SPEC-04B REQ-E3D-520 to 522).
//!
//! The grid is the *dual* of the voxels: a cell's eight corners are voxel centres, so the
//! surface floats half a voxel outside the field, which REQ-E3D-408 compensates by shrinking
//! the instance scale. Corners outside the tile are read in the neighbouring tile of the
//! pattern, which is what makes two adjacent tiles join without a wall (TV-M3).
//!
//! Each vertex is placed *exactly on the exterior corner* of its edge rather than
//! interpolated, which is the behaviour of the reference; it produces zero-area triangles in
//! some configurations, which REQ-E3D-522 allows and REQ-E3D-523 ignores when comparing.

use crate::geometry::marching_tables::{
    CORNER_COUNT, CORNER_OFFSETS, EDGE_CORNERS, EDGE_COUNT, TRIANGULATION,
};
use crate::geometry::pattern_voxels::PatternVoxels;
use crate::geometry::tile_mesh::{TileMesh, TileMeshBuilder};
use crate::geometry::tile_voxels::{SIZE_X, SIZE_Y};
use crate::vectors::{Vec2, Vec3};

/// The eight corners of one cell: grid position, texel and value.
#[derive(Debug, Clone, Copy, Default)]
struct Corner {
    position: (i32, i32, i32),
    texel: (i32, i32),
    value: i32,
}

/// Builds the mesh of the tile of index `image_index`. `voxels` holds the fields of the whole
/// pattern, for the neighbourhood; `empty_color` is the transparent colour `e`
/// (REQ-E3D-424).
#[must_use]
pub fn build(voxels: &PatternVoxels<'_>, image_index: usize, empty_color: i32) -> TileMesh {
    let field = voxels.field(image_index);
    if field.is_empty() {
        return TileMesh::empty();
    }

    let depth = field.depth();
    let mut builder = TileMeshBuilder::default();
    let mut corners = [Corner::default(); CORNER_COUNT];
    let mut colors = [0i32; CORNER_COUNT];

    for k in -1..depth {
        for j in -1..SIZE_Y {
            for i in -1..SIZE_X {
                // The cells of the leftmost column belong to the leftmost occurrence of the
                // tile only (REQ-E3D-520).
                if i == -1 && !voxels.meshes_left_edge(image_index, j) {
                    continue;
                }

                let mut color_count = 0;
                for (corner, (dx, dy, dz)) in corners.iter_mut().zip(CORNER_OFFSETS) {
                    let (x, y, z) = (i + dx, j + dy, k + dz);
                    let texel = voxels.corner(image_index, x, y, z);
                    *corner = Corner {
                        position: (x, y, z),
                        texel: texel.unwrap_or((0, 0)),
                        // An empty corner counts as the value 0 (REQ-E3D-521).
                        value: texel.map_or(0, |(tx, ty)| voxels.texture().texel(tx, ty)),
                    };

                    if corner.value != empty_color && !colors[..color_count].contains(&corner.value)
                    {
                        colors[color_count] = corner.value;
                        color_count += 1;
                    }
                }

                for &color in &colors[..color_count] {
                    surface(&mut builder, color, &corners, depth);
                }
            }
        }
    }

    builder.build()
}

/// Emits the surface that separates the corners of value `color` from the others, inside one
/// cell (REQ-E3D-521, REQ-E3D-522).
fn surface(
    builder: &mut TileMeshBuilder,
    color: i32,
    corners: &[Corner; CORNER_COUNT],
    depth: i32,
) {
    let mut configuration = 0usize;
    let mut highest = None;
    let mut interior = [false; CORNER_COUNT];
    for (c, corner) in corners.iter().enumerate() {
        interior[c] = corner.value == color;
        if interior[c] {
            configuration |= 1 << c;
            highest = Some(c);
        }
    }

    // A cell whose eight corners carry the colour produces nothing.
    let Some(highest) = highest.filter(|_| configuration != 0xFF) else {
        return;
    };

    // The texel of the interior corner of highest number (REQ-E3D-522).
    #[allow(clippy::cast_precision_loss)] // Texel coordinates.
    let uv = Vec2::new(
        corners[highest].texel.0 as f32 + 0.5,
        corners[highest].texel.1 as f32 + 0.5,
    );

    let mut edge_vertex = [Vec3::ZERO; EDGE_COUNT];
    let mut edge_outward = [Vec3::ZERO; EDGE_COUNT];
    for (e, &(first, second)) in EDGE_CORNERS.iter().enumerate() {
        let a = position(corners[first].position, depth);
        let b = position(corners[second].position, depth);
        if !interior[first] {
            edge_vertex[e] = a;
            edge_outward[e] = if interior[second] { a - b } else { Vec3::ZERO };
        } else if !interior[second] {
            edge_vertex[e] = b;
            edge_outward[e] = b - a;
        } else {
            edge_vertex[e] = (a + b) * 0.5;
            edge_outward[e] = Vec3::ZERO;
        }
    }

    for triangle in TRIANGULATION[configuration].chunks_exact(3) {
        let [Ok(e0), Ok(e1), Ok(e2)] = [triangle[0], triangle[1], triangle[2]].map(usize::try_from)
        else {
            break;
        };

        // The surface must look away from the region of colour c; averaging the
        // interior-to-exterior directions of the three edges gives that side. REQ-E3D-522
        // states the intention, not a construction: the vertices sit on exterior corners, so
        // the triangle alone does not say which way is out (REQ-E3D-505, Q-016).
        let outward = edge_outward[e0] + edge_outward[e1] + edge_outward[e2];
        builder.add_triangle(
            edge_vertex[e0],
            edge_vertex[e1],
            edge_vertex[e2],
            uv,
            outward,
        );
    }
}

/// The local position of a voxel centre: `(i, j, k) − (3.5, 3.5, (D − 1)/2)` (REQ-E3D-522).
#[allow(clippy::cast_precision_loss)] // Small voxel indices.
fn position((i, j, k): (i32, i32, i32), depth: i32) -> Vec3 {
    Vec3::new(
        i as f32 - 3.5,
        j as f32 - 3.5,
        k as f32 - ((depth - 1) as f32 / 2.0),
    )
}
