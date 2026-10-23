//! The fixed geometry of the conversion: grid sizes, scene height and the screen-to-cell
//! relations of SPEC-04 §2 and SPEC-04B REQ-E3D-410.

use crate::vectors::Cell;

/// Background grid width in cells: the core's 33 useful columns plus one cell of margin on
/// each side (SPEC-02 REQ-CORE-031, SPEC-04 §2).
pub const WIDTH: i32 = 35;

/// Background grid height in cells: the core's 31 useful rows plus two margins.
pub const HEIGHT: i32 = 33;

/// Number of cells of either grid.
#[allow(clippy::cast_sign_loss)] // A positive constant.
pub const CELL_COUNT: usize = (WIDTH * HEIGHT) as usize;

/// Scene height `Hs = 248`: the constant that flips the y axis (REQ-E3D-205, REQ-E3D-410).
pub const SCENE_HEIGHT: i32 = 248;

/// Largest tile box width that still counts as "not big" (REQ-E3D-045).
pub const BIG_WIDTH: i32 = 28;

/// Largest tile box height that still counts as "not big" (REQ-E3D-045).
pub const BIG_HEIGHT: i32 = 26;

/// The neighbour offsets in the examination order of REQ-E3D-090: right, down, up, left.
/// Index `3 − d` is the opposite direction.
pub const NEIGHBOUR_OFFSETS: [i32; 4] = [1, WIDTH, -WIDTH, -1];

/// The background cell of a tile whose top-left pixel is at (`x`, `y`):
/// `(⌊(x+7)/8⌋ + 1, ⌊(y+7)/8⌋ + 1)`, division rounding towards −∞ (SPEC-04 §2, REQ-E3D-005).
#[must_use]
pub const fn background_cell(x: i32, y: i32) -> Cell {
    Cell::new(((x + 7) >> 3) + 1, ((y + 7) >> 3) + 1)
}

/// The sprite cell of a sprite at (`x`, `y`): `(⌊x/8⌋, ⌊y/8⌋)`, without margin (SPEC-04 §2,
/// REQ-E3D-005).
#[must_use]
pub const fn sprite_cell(x: i32, y: i32) -> Cell {
    Cell::new(x >> 3, y >> 3)
}

/// The index of a cell in a grid array; negative or past the end for a cell outside the grid.
#[must_use]
pub const fn index(x: i32, y: i32) -> i32 {
    x + (y * WIDTH)
}

/// The index of a cell in a grid array.
#[must_use]
pub const fn index_of(cell: Cell) -> i32 {
    index(cell.x, cell.y)
}

/// The slot of a grid index in a grid array, or `None` when it is outside the array.
#[must_use]
pub fn slot(index: i32) -> Option<usize> {
    usize::try_from(index)
        .ok()
        .filter(|&slot| slot < CELL_COUNT)
}

/// True when the cell is inside the grid, margins included.
#[must_use]
pub const fn contains(cell: Cell) -> bool {
    cell.x >= 0 && cell.x < WIDTH && cell.y >= 0 && cell.y < HEIGHT
}
