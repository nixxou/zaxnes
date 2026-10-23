/// One 8x8 tile displayed during a frame (REQ-CORE-034, REQ-CORE-035, SPEC-02 §4.1).
///
/// The two bit planes hold the 16 CHR bytes of the pattern *without flips applied*. The byte
/// of row `r` (0 = top) occupies bits `8r` to `8r+7`; inside that byte bit 7 is the leftmost
/// pixel (column 0) and bit 0 is column 7. The colour of the source pixel at (column c,
/// row r) is `bit(plane0, 8r + 7 - c) + 2 * bit(plane1, 8r + 7 - c)`, between 0 and 3, where
/// 0 is transparent. The pixel shown at tile position (px, py) is the source pixel of column
/// `mirror_h ? 7 - px : px` and row `mirror_v ? 7 - py : py`.
///
/// This encoding is also the one used by the pattern planes stored in profiles (SPEC-03),
/// which are the recognition key of the conversion engine (SPEC-04); it must be respected to
/// the bit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[allow(clippy::struct_excessive_bools)] // The record of SPEC-02 §4.1 has four flags.
pub struct TileRecord {
    /// Low bit plane, 8 CHR bytes packed little-endian by row.
    pub plane0: u64,
    /// High bit plane, 8 CHR bytes packed little-endian by row.
    pub plane1: u64,
    /// Screen x of the tile's top-left pixel. Background: -7..255. Sprite: 0..255.
    pub x: i16,
    /// Screen y of the tile's top-left pixel. Background: -1..238. Sprite: 1..239.
    pub y: i16,
    /// Background: REQ-CORE-040. Sprite: 0..127 (REQ-CORE-043).
    pub index: i32,
    /// Horizontal flip. Sprite: bit 6 of the OAM attribute. Background: false.
    pub mirror_h: bool,
    /// Vertical flip. Sprite: bit 7 of the OAM attribute. Background: false.
    pub mirror_v: bool,
    /// Sprite priority bit "behind background" (bit 5 of the attribute). Always false for
    /// background.
    pub hidden: bool,
    /// Sub-palette number, 0 to 3; background sub-palette for background tiles.
    pub palette: u8,
    /// True for a background tile, false for a sprite tile.
    pub background: bool,
}

impl TileRecord {
    /// Creates a tile record.
    #[must_use]
    #[allow(clippy::too_many_arguments)] // The ten fields of SPEC-02 §4.1, in their order.
    #[allow(clippy::fn_params_excessive_bools)] // Same.
    pub const fn new(
        plane0: u64,
        plane1: u64,
        x: i16,
        y: i16,
        index: i32,
        mirror_h: bool,
        mirror_v: bool,
        hidden: bool,
        palette: u8,
        background: bool,
    ) -> Self {
        Self {
            plane0,
            plane1,
            x,
            y,
            index,
            mirror_h,
            mirror_v,
            hidden,
            palette,
            background,
        }
    }

    /// Colour (0 to 3) of the *source* pixel at (`column`, `row`), flips not applied. 0 is
    /// transparent. `None` when a coordinate is outside 0..=7.
    #[must_use]
    pub const fn source_pixel(&self, column: usize, row: usize) -> Option<u8> {
        if column > 7 || row > 7 {
            return None;
        }

        let bit = (8 * row) + 7 - column;
        #[allow(clippy::cast_possible_truncation)] // Each term is 0 or 1.
        let pixel = ((self.plane0 >> bit) & 1) as u8 + 2 * ((self.plane1 >> bit) & 1) as u8;
        Some(pixel)
    }

    /// Colour (0 to 3) of the pixel *displayed* at tile position (`px`, `py`), flips
    /// applied. `None` when a coordinate is outside 0..=7.
    #[must_use]
    pub const fn displayed_pixel(&self, px: usize, py: usize) -> Option<u8> {
        if px > 7 || py > 7 {
            return None;
        }

        self.source_pixel(
            if self.mirror_h { 7 - px } else { px },
            if self.mirror_v { 7 - py } else { py },
        )
    }

    /// True when both planes are zero, i.e. the tile draws nothing.
    ///
    /// Empty records are still emitted by the core (REQ-CORE-036); filtering is the
    /// consumer's job (SPEC-04).
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        (self.plane0 | self.plane1) == 0
    }

    /// Packs eight CHR bytes of one plane into the 64-bit representation of REQ-CORE-034:
    /// the byte of row `r` goes to bits `8r`..`8r+7`.
    #[must_use]
    pub const fn pack_plane(rows: [u8; 8]) -> u64 {
        u64::from_le_bytes(rows)
    }
}
