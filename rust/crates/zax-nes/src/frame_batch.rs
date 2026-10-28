use crate::TileRecord;

/// For each band of 8 lines (row 0 = lines 0 to 7), the range of screen x `[low, high)` the
/// tiles of that band could show in: what the game keeps drawn there. A shape that reaches the
/// edge of its band's range may be cut there.
pub type VisibleRows = [(i16, i16); 30];

/// The rows of a standard 256-pixel frame.
pub const STANDARD_ROWS: VisibleRows = [(0, 256); 30];

/// The ordered tile records of one emulated frame, together with the frame palette and the
/// unreliable border widths, handed to the video callback (REQ-CORE-030).
///
/// The records are borrowed: a host that keeps the data must copy it (REQ-CORE-030).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameBatch<'a> {
    /// Tile records in capture order (REQ-CORE-044): chronological by capture line, and for
    /// one line the sprites before the background row.
    pub records: &'a [TileRecord],
    /// The 32 bytes of PPU palette RAM ($3F00-$3F1F) at the end of the frame
    /// (REQ-CORE-045). Each value is a NES colour index, 0 to 63.
    pub palette: [u8; 32],
    /// The margin decoder's own background palette for the frame (16 NES colour indices, as
    /// `$3F00-$3F0F`): background records in sub-palettes 4-7 are in its sub-palettes 0-3.
    /// `None` for a frame without (its records stay in sub-palettes 0-3).
    pub margin_palette: Option<[u8; 16]>,
    /// Width in NES pixels of the unreliable bands on the left *and* right edges
    /// (REQ-CORE-046).
    pub mask_x: i32,
    /// Width in NES pixels of the unreliable bands on the top *and* bottom edges
    /// (REQ-CORE-046).
    pub mask_y: i32,
    /// Widescreen margin of the frame, in NES pixels on each side: 0 for a standard frame.
    /// Background records then reach `-margin - 7` and sprite records `-margin`, and both go
    /// up to `255 + margin`.
    pub margin: i32,
    /// The range of x each band of the frame showed tiles in: [`STANDARD_ROWS`] for a standard
    /// frame, from `-margin` to `256 + margin` at most for a widescreen one.
    pub visible_rows: VisibleRows,
    /// Widescreen margin above and below, in NES pixels: background records then reach
    /// `-margin_y - 8` and `239 + margin_y`. 0 unless a margin decoder fills it.
    pub margin_y: i32,
}
