//! Tiles and the tile grid of a pattern (SPEC-03 REQ-PROF-012, REQ-PROF-030 to REQ-PROF-032).

use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::enums::TileFlip;
use crate::layout::FieldMask;

/// One 8x8 NES pattern stored in a profile: two bit planes and the mirroring flags.
///
/// The planes are held in clear: the reversible masking of the file belongs to the wire codec.
/// Bit `y * 8 + (7 - x)` of a plane is the bit of pixel (x, y), with (0, 0) at the top left and
/// without mirroring applied, the same packing as the tiles produced by the core.
///
/// Two tiles are the same tile exactly when their planes and their flip are equal
/// (REQ-PROF-032), which is what `==` implements. Sharing inside a file is a separate matter:
/// a profile may carry one tile in several grid cells (an [`Arc`] held by several cells), and
/// the reader preserves that identity (REQ-PROF-011), which is what the writer uses to rebuild
/// the shared references.
///
/// Everything else about a tile (pixel count, bounding box, colours used, density,
/// connectivity) is derived and recomputed by the conversion engine; none of it is stored.
#[derive(Debug, Clone, Copy)]
pub struct ProfileTile {
    /// Low bit plane, in clear.
    pub low_plane: u64,
    /// High bit plane, in clear.
    pub high_plane: u64,
    /// Mirroring flags.
    pub flip: TileFlip,
    /// The fields the file carried, when they differ from the schema (Q-010).
    pub recorded_fields: Option<FieldMask>,
}

impl ProfileTile {
    /// Creates a tile from its planes in clear.
    #[must_use]
    pub const fn new(low_plane: u64, high_plane: u64, flip: TileFlip) -> Self {
        Self {
            low_plane,
            high_plane,
            flip,
            recorded_fields: None,
        }
    }

    /// True when the horizontal mirror bit is set.
    #[must_use]
    pub const fn flip_x(&self) -> bool {
        self.flip.contains(TileFlip::HORIZONTAL)
    }

    /// True when the vertical mirror bit is set.
    #[must_use]
    pub const fn flip_y(&self) -> bool {
        self.flip.contains(TileFlip::VERTICAL)
    }

    /// Colour index, 0 to 3, of the displayed pixel at (x, y), mirroring applied
    /// (REQ-PROF-031). Colour 0 is transparent. None when a coordinate is outside 0..7.
    #[must_use]
    pub fn color_at(&self, x: u32, y: u32) -> Option<u32> {
        if x > 7 || y > 7 {
            return None;
        }
        let source_x = if self.flip_x() { 7 - x } else { x };
        let source_y = if self.flip_y() { 7 - y } else { y };
        let bit = source_y * 8 + 7 - source_x;
        let low = (self.low_plane >> bit) & 1;
        let high = (self.high_plane >> bit) & 1;
        u32::try_from(2 * high + low).ok()
    }
}

impl PartialEq for ProfileTile {
    fn eq(&self, other: &Self) -> bool {
        self.low_plane == other.low_plane
            && self.high_plane == other.high_plane
            && self.flip == other.flip
    }
}

impl Eq for ProfileTile {}

impl Hash for ProfileTile {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.low_plane.hash(state);
        self.high_plane.hash(state);
        self.flip.hash(state);
    }
}

/// The rectangular tile grid of a pattern (SPEC-03 REQ-PROF-012): `width x height` cells row by
/// row, each either a tile or empty.
///
/// Cell (cx, cy) is at index `cx + cy * width`. The size comes from the pattern's `tSize`; the
/// built-in profiles always hold exactly `width x height` cells, but a file that holds another
/// number is still read (the cell list is authoritative, and [`Self::is_rectangular`] says
/// whether the two agree).
#[derive(Debug, Clone)]
pub struct PatternGrid {
    width: i32,
    height: i32,
    cells: Vec<Option<Arc<ProfileTile>>>,
}

impl PatternGrid {
    /// Creates a grid from its cells in file order, row by row; a `None` entry is an empty
    /// cell. `width` and `height` come from the pattern's `tSize`.
    #[must_use]
    pub fn new(width: i32, height: i32, cells: Vec<Option<Arc<ProfileTile>>>) -> Self {
        Self {
            width,
            height,
            cells,
        }
    }

    /// Width in tiles, as declared by the pattern.
    #[must_use]
    pub fn width(&self) -> i32 {
        self.width
    }

    /// Height in tiles, as declared by the pattern.
    #[must_use]
    pub fn height(&self) -> i32 {
        self.height
    }

    /// Cells in file order, row by row; a `None` entry is an empty cell.
    #[must_use]
    pub fn cells(&self) -> &[Option<Arc<ProfileTile>>] {
        &self.cells
    }

    /// True when the number of cells is exactly `width x height`.
    #[must_use]
    pub fn is_rectangular(&self) -> bool {
        i64::try_from(self.cells.len())
            .is_ok_and(|count| count == i64::from(self.width) * i64::from(self.height))
    }

    /// Number of cells that hold a tile.
    #[must_use]
    pub fn filled_cell_count(&self) -> usize {
        self.cells.iter().flatten().count()
    }

    /// The tile at (cx, cy), or none when the cell is empty or outside the stored cells.
    #[must_use]
    pub fn get(&self, cx: i32, cy: i32) -> Option<&Arc<ProfileTile>> {
        if !(0..self.width).contains(&cx) || !(0..self.height).contains(&cy) {
            return None;
        }
        let index = usize::try_from(i64::from(cx) + i64::from(cy) * i64::from(self.width)).ok()?;
        self.cells.get(index)?.as_ref()
    }
}
