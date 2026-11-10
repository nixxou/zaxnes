//! The four lateral sides of a tile a transparent background instance may have taken off
//! (AMD-063 REQ-SCN-087 step 2). Front and back are never removable.

/// A set of lateral sides of a tile.
///
/// The names are the screen's: [`TileSides::TOP`] is the side the cell above shares, which is
/// `+y` in the tile's own frame, since `j` runs upwards there (SPEC-04B REQ-E3D-420).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TileSides(u8);

impl TileSides {
    /// Nothing removed: the whole mesh of REQ-E3D-501, as AMD-057 draws it.
    pub const NONE: Self = Self(0);
    /// The `−x` side, shared with the cell to the left.
    pub const LEFT: Self = Self(1);
    /// The `+x` side, shared with the cell to the right.
    pub const RIGHT: Self = Self(2);
    /// The `−y` side, shared with the cell below on screen.
    pub const BOTTOM: Self = Self(4);
    /// The `+y` side, shared with the cell above on screen.
    pub const TOP: Self = Self(8);
    /// All four lateral sides.
    pub const ALL: Self = Self(0b1111);

    /// How many distinct sets of sides there are, which is how many mesh variants a full tile
    /// can have: 16, the first being the mesh itself.
    pub const COMBINATIONS: usize = 16;

    /// The raw bits, which index the memoised mesh variants of a full tile.
    #[must_use]
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// True when nothing is removed.
    #[must_use]
    pub const fn is_none(self) -> bool {
        self.0 == 0
    }

    /// True when every side of `other` is in this set.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// This set with the sides of `other` added.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}
