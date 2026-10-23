// Portions adapted from 3DNes_Engine (c) Geod Studio, GPL-3.0

//! Turns a voxel field into rectangular faces merged greedily (SPEC-04B REQ-E3D-501 to 506):
//! `GREEDY` merges without looking at colour, `GREEDY_MULTICOLOR` only merges cells of equal
//! texel value.
//!
//! The voxels of the neighbouring tiles are never consulted: each tile mesh is closed on
//! itself, which is why two full tiles side by side keep the wall between them (REQ-E3D-501
//! notes, TV-G7).
//!
//! Q-067: the `sealed_sides` mask is the one exception, and it is a *rendering* variant, not
//! a change to REQ-E3D-501. A side listed there is one the draw list has found to be entirely
//! covered by the neighbouring tile of the same object; the wall between the two is then
//! strictly inside the solid they form, and the mesh of that variant leaves it out so that it
//! cannot show at the joint. The default mask of 0 gives back the mesh REQ-E3D-501 and
//! REQ-E3D-523 describe, bit for bit.

use crate::geometry::texture::ObjectTexture;
use crate::geometry::tile_mesh::{TileMesh, TileMeshBuilder};
use crate::geometry::tile_sides;
use crate::geometry::tile_voxels::{pack, unpack, TileVoxels, SIZE_X, SIZE_Y};
use crate::vectors::{Vec2, Vec3};

const EMPTY: i32 = -1;

/// One plane of faces: its axis, the two axes it spans, their sizes, its position and the
/// direction of its normal.
#[derive(Debug, Clone, Copy)]
struct Plane {
    axis: usize,
    u: usize,
    v: usize,
    su: i32,
    sv: i32,
    position: i32,
    sign: i32,
    depth: i32,
}

/// Builds the mesh of one tile. `field` is the voxel field of the tile (three slots per
/// voxel), `texture` the object's texture, read for the multicolour merge; `multicolor` is
/// true for `GREEDY_MULTICOLOR` (REQ-E3D-503). `sealed_sides` lists the lateral sides a
/// neighbouring tile covers entirely, as bits of [`tile_sides`]; 0 closes the tile on itself
/// as REQ-E3D-501 states (Q-067).
#[must_use]
pub fn build(
    field: &TileVoxels,
    texture: &ObjectTexture,
    multicolor: bool,
    sealed_sides: u8,
) -> TileMesh {
    if field.is_empty() {
        return TileMesh::empty();
    }

    let mut builder = TileMeshBuilder::default();
    let sizes = [SIZE_X, SIZE_Y, field.depth()];
    let max_cells = (sizes[0] * sizes[1])
        .max(sizes[1] * sizes[2])
        .max(sizes[2] * sizes[0]);
    #[allow(clippy::cast_sign_loss)] // A product of positive sizes.
    let mut mask = vec![EMPTY; max_cells as usize];

    for axis in 0..3 {
        let u = (axis + 1) % 3;
        let v = (axis + 2) % 3;
        for sign in [-1, 1] {
            for plane in 0..=sizes[axis] {
                let plane = Plane {
                    axis,
                    u,
                    v,
                    su: sizes[u],
                    sv: sizes[v],
                    position: plane,
                    sign,
                    depth: field.depth(),
                };
                if fill_mask(field, &mut mask, plane, sealed_sides, sizes[axis]) {
                    partition(&mut builder, texture, &mut mask, plane, multicolor);
                }
            }
        }
    }

    builder.build()
}

/// Fills `mask` with the face cells of one plane and one direction (REQ-E3D-501), and answers
/// whether there is at least one.
fn fill_mask(
    field: &TileVoxels,
    mask: &mut [i32],
    plane: Plane,
    sealed_sides: u8,
    axis_size: i32,
) -> bool {
    let mut any = false;
    let mut cell = [0i32; 3];
    let mut n = 0;
    for vi in 0..plane.sv {
        for ui in 0..plane.su {
            cell[plane.u] = ui;
            cell[plane.v] = vi;

            // The owner of the face is the voxel on the side the normal comes from.
            cell[plane.axis] = if plane.sign > 0 {
                plane.position - 1
            } else {
                plane.position
            };
            let owner = field.texel(cell[0], cell[1], cell[2], plane.axis);

            cell[plane.axis] = if plane.sign > 0 {
                plane.position
            } else {
                plane.position - 1
            };

            // Q-067: on a sealed side the tile next door fills the whole plane, so the voxel
            // just outside counts as occupied and no wall is emitted there.
            let neighbour =
                tile_sides::is_sealed(sealed_sides, plane.axis, cell[plane.axis], axis_size)
                    || field.occupied(cell[0], cell[1], cell[2], plane.axis);

            mask[n] = match owner {
                Some((tx, ty)) if !neighbour => {
                    any = true;
                    pack(tx, ty)
                }
                _ => EMPTY,
            };
            n += 1;
        }
    }

    any
}

/// Groups the face cells of a plane into maximal rectangles, `v` ascending then `u` ascending
/// (REQ-E3D-502, REQ-E3D-503), and emits each one.
#[allow(clippy::cast_sign_loss)] // Offsets inside the plane are non-negative.
fn partition(
    builder: &mut TileMeshBuilder,
    texture: &ObjectTexture,
    mask: &mut [i32],
    plane: Plane,
    multicolor: bool,
) {
    let su = plane.su as usize;
    for vi in 0..plane.sv {
        let row = vi as usize * su;
        let mut ui = 0;
        while ui < plane.su {
            let start = row + ui as usize;
            if mask[start] == EMPTY {
                ui += 1;
                continue;
            }

            // GREEDY merges any two cells that both exist; GREEDY_MULTICOLOR also demands the
            // texel value of the rectangle's first cell. A position without a cell is never
            // merged, which leaves out the legacy "missing counts as 0" case of the
            // REQ-E3D-503 notes (P3, class D, DEC-04B-Q6).
            let reference = if multicolor {
                value(texture, mask[start])
            } else {
                0
            };
            let compatible =
                |cell: i32| cell != EMPTY && (!multicolor || value(texture, cell) == reference);

            let mut width = 0;
            while ui + width < plane.su && compatible(mask[start + width as usize]) {
                width += 1;
            }

            let mut height = 1;
            while vi + height < plane.sv {
                let candidate = start + su * height as usize;
                if !(0..width as usize).all(|c| compatible(mask[candidate + c])) {
                    break;
                }

                height += 1;
            }

            let top_row = start + su * (height as usize - 1);
            let last = width as usize - 1;
            emit_rectangle(
                builder,
                plane,
                (ui, vi, width, height),
                [
                    mask[start],
                    mask[top_row],
                    mask[top_row + last],
                    mask[start + last],
                ],
            );

            for c in 0..width as usize {
                for r in 0..height as usize {
                    mask[start + c + su * r] = EMPTY;
                }
            }

            ui += width;
        }
    }
}

/// Emits one rectangle `(u0, v0, width, height)`: its four corners in local coordinates
/// (REQ-E3D-504) and their texture coordinates (REQ-E3D-506). `cells` are the packed texels
/// of the bottom-left, top-left, top-right and bottom-right cells.
fn emit_rectangle(
    builder: &mut TileMeshBuilder,
    plane: Plane,
    rectangle: (i32, i32, i32, i32),
    cells: [i32; 4],
) {
    let (u0, v0, width, height) = rectangle;
    let corners = [
        corner(plane, u0, v0),
        corner(plane, u0, v0 + height),
        corner(plane, u0 + width, v0 + height),
        corner(plane, u0 + width, v0),
    ];

    let (mut blx, mut bly) = unpack(cells[0]);
    let (mut tlx, mut tly) = unpack(cells[1]);
    let (mut trx, mut tr_y) = unpack(cells[2]);
    let (mut brx, mut bry) = unpack(cells[3]);

    let axis_is_z = plane.axis == 2;

    // Column component (REQ-E3D-506).
    if trx >= blx {
        trx += 1;
        if axis_is_z {
            brx += 1;
        } else {
            tlx += 1;
        }
    } else {
        blx += 1;
        if axis_is_z {
            tlx += 1;
        } else {
            brx += 1;
        }
    }

    // Row component (REQ-E3D-506).
    if tr_y >= bly {
        tr_y += 1;
        if axis_is_z {
            tly += 1;
        } else {
            bry += 1;
        }
    } else {
        bly += 1;
        if axis_is_z {
            bry += 1;
        } else {
            tly += 1;
        }
    }

    #[allow(clippy::cast_precision_loss)] // Texel coordinates and ±1.
    let (uvs, sign) = (
        [
            Vec2::new(blx as f32, bly as f32),
            Vec2::new(tlx as f32, tly as f32),
            Vec2::new(trx as f32, tr_y as f32),
            Vec2::new(brx as f32, bry as f32),
        ],
        plane.sign as f32,
    );
    let normal = match plane.axis {
        0 => Vec3::new(sign, 0.0, 0.0),
        1 => Vec3::new(0.0, sign, 0.0),
        _ => Vec3::new(0.0, 0.0, sign),
    };

    builder.add_quad(corners, uvs, normal);
}

/// The local position of the voxel corner of integer indices `(i, j, k)`:
/// `(i − 4, j − 4, k − D/2)` (REQ-E3D-504).
#[allow(clippy::cast_precision_loss)] // Small voxel indices.
fn corner(plane: Plane, ui: i32, vi: i32) -> Vec3 {
    let mut index = [0i32; 3];
    index[plane.axis] = plane.position;
    index[plane.u] = ui;
    index[plane.v] = vi;
    Vec3::new(
        index[0] as f32 - 4.0,
        index[1] as f32 - 4.0,
        index[2] as f32 - (plane.depth as f32 / 2.0),
    )
}

fn value(texture: &ObjectTexture, cell: i32) -> i32 {
    let (tx, ty) = unpack(cell);
    texture.texel(tx, ty)
}
