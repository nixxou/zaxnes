//! The four lateral sides of a tile — the ones a neighbouring tile of the same object can be
//! against — as a bit mask (Q-067).
//!
//! SPEC-04B REQ-E3D-501 closes every tile mesh on itself, so two full tiles side by side keep
//! a pair of coincident walls between them, inside the solid the two form. Those walls are
//! what shows at every joint of a flat surface: a dark line where they win the depth test
//! along the shared edge, and a fin sticking out of the silhouette wherever the surface ends
//! (SPEC-06 REQ-SCN-065). A side listed in a mask is one the draw list has proved covered, so
//! the mesh variant of Q-067 leaves its wall out.

use crate::geometry::tile_voxels::{TileVoxels, AXIS_X, AXIS_Y, SIZE_X, SIZE_Y};

/// The `x = 0` side of the tile, towards the neighbour on its left.
pub const MINUS_X: u8 = 1 << 0;

/// The `x = 8` side, towards the neighbour on its right.
pub const PLUS_X: u8 = 1 << 1;

/// The `y = 0` side, towards the neighbour below it in the scene.
pub const MINUS_Y: u8 = 1 << 2;

/// The `y = 8` side, towards the neighbour above it in the scene.
pub const PLUS_Y: u8 = 1 << 3;

/// Every lateral side; the Z sides are never sealed.
pub const ALL: u8 = MINUS_X | PLUS_X | MINUS_Y | PLUS_Y;

/// The bit of the side at the `low` end of `axis` (0 for X, 1 for Y; anything else has no
/// bit). `low` is true for the `0` side, false for the `size` side.
#[must_use]
pub const fn bit(axis: usize, low: bool) -> u8 {
    match (axis, low) {
        (0, true) => MINUS_X,
        (0, false) => PLUS_X,
        (1, true) => MINUS_Y,
        (1, false) => PLUS_Y,
        _ => 0,
    }
}

/// The side opposite `side`, which the neighbour presents.
#[must_use]
pub const fn opposite(side: u8) -> u8 {
    match side {
        MINUS_X => PLUS_X,
        PLUS_X => MINUS_X,
        MINUS_Y => PLUS_Y,
        PLUS_Y => MINUS_Y,
        _ => 0,
    }
}

/// Whether the voxel at `index` along `axis`, which is outside the tile when it leaves
/// `[0, size)`, belongs to a sealed neighbour.
#[must_use]
pub const fn is_sealed(sealed_sides: u8, axis: usize, index: i32, size: i32) -> bool {
    if sealed_sides == 0 || axis > 1 {
        return false;
    }

    if index < 0 {
        return sealed_sides & bit(axis, true) != 0;
    }

    index >= size && sealed_sides & bit(axis, false) != 0
}

/// The sides of `field` whose whole plane is occupied: those are the only ones a neighbour can
/// seal without opening a hole, since the two walls then coincide cell for cell (Q-067).
#[must_use]
pub fn full_sides(field: &TileVoxels) -> u8 {
    if field.is_empty() || field.depth() <= 0 {
        return 0;
    }

    let mut mask = ALL;
    for k in 0..field.depth() {
        for n in 0..SIZE_Y {
            if !field.occupied(0, n, k, AXIS_X) {
                mask &= !MINUS_X;
            }

            if !field.occupied(SIZE_X - 1, n, k, AXIS_X) {
                mask &= !PLUS_X;
            }
        }

        for n in 0..SIZE_X {
            if !field.occupied(n, 0, k, AXIS_Y) {
                mask &= !MINUS_Y;
            }

            if !field.occupied(n, SIZE_Y - 1, k, AXIS_Y) {
                mask &= !PLUS_Y;
            }
        }

        if mask == 0 {
            return 0;
        }
    }

    mask
}
