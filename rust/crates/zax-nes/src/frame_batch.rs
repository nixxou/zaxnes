use crate::TileRecord;

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
}
