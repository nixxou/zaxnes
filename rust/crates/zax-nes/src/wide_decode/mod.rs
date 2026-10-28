//! Margin decoders: widescreen scenery read from a game's own level data instead of the PPU.
//!
//! The PPU holds two screens of background at most, and most games only keep a few columns
//! past the screen drawn there. A game that keeps its whole area in RAM can do better: a decoder
//! made for it turns that RAM into the tiles of the margins, as wide as the host asks.
//!
//! Decoders are optional and per game. For each background row of a widescreen frame the core
//! asks the game's decoder, if it has one, for the tiles of the margins; a row the decoder does
//! not handle (status bar, title screen, a mode it does not know) keeps the columns captured
//! from the nametables, as for any game without a decoder. The centre 256 pixels always come
//! from the PPU.
//!
//! A decoder applies to the ROMs a built-in patch is made for, while the built-in patches are
//! on (see [`crate::builtin_patches`]).

mod adventure_island;
mod blastermaster;
mod castlevania;
mod excitebike;
mod ff1;
pub mod guest;
mod megaman2;
mod metalforce;
mod metroid;
mod ng3;
mod smb1;
mod smb3;
mod zelda2;

/// What a decoder may read once the frame has run.
#[derive(Clone, Copy)]
pub struct DecoderMemory<'a> {
    /// CPU RAM, `$0000-$07FF`.
    pub ram: &'a [u8],
    /// PRG-RAM, `$6000-$7FFF`.
    pub prg_ram: &'a [u8],
    /// The whole PRG-ROM image, as the cartridge runs it (built-in patch applied).
    pub prg_rom: &'a [u8],
    /// A nametable byte (`$2000-$2FFF`) as the PPU reads it now, mirroring and layout applied:
    /// a decoder can check what it decodes against what the PPU shows.
    pub nametable: &'a dyn Fn(u16) -> u8,
    /// When a nametable byte was last written, in rendered frames (0: not since power-on or a
    /// state load): what the game drew together shares a frame, what an object drew over it later
    /// is newer.
    pub nametable_written: &'a dyn Fn(u16) -> u32,
}

impl std::fmt::Debug for DecoderMemory<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DecoderMemory")
            .field("ram", &self.ram.len())
            .field("prg_ram", &self.prg_ram.len())
            .field("prg_rom", &self.prg_rom.len())
            .finish_non_exhaustive()
    }
}

impl DecoderMemory<'_> {
    /// CPU RAM byte at `address` (0 past the end).
    #[must_use]
    pub fn ram(&self, address: u16) -> u8 {
        self.ram.get(usize::from(address)).copied().unwrap_or(0)
    }
}

/// One captured background row: where the PPU was reading when it drew it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackgroundRow {
    /// Line the row was captured on.
    pub line: i32,
    /// The PPU address register (`v`) at the start of that line.
    pub v: u16,
    /// Fine X scroll.
    pub fine_x: u8,
}

impl BackgroundRow {
    /// Tile column of the left edge, 0-31.
    #[must_use]
    pub const fn coarse_x(&self) -> u8 {
        (self.v & 0x1F) as u8
    }

    /// Tile row, 0-29 (30-31 for rows past a nametable).
    #[must_use]
    pub const fn coarse_y(&self) -> u8 {
        ((self.v >> 5) & 0x1F) as u8
    }

    /// Horizontal nametable bit.
    #[must_use]
    pub const fn nametable_x(&self) -> u8 {
        ((self.v >> 10) & 1) as u8
    }

    /// Vertical nametable bit.
    #[must_use]
    pub const fn nametable_y(&self) -> u8 {
        ((self.v >> 11) & 1) as u8
    }

    /// The pixel of the 512-pixel horizontal plane of two nametables at the left edge.
    #[must_use]
    pub fn ring_x(&self) -> i32 {
        (i32::from(self.nametable_x()) << 8)
            | (i32::from(self.coarse_x()) << 3)
            | i32::from(self.fine_x)
    }
}

/// One tile a decoder hands back: screen column `column` (x = 8 × column − fine X, 0 being the
/// leftmost column of the 256-pixel picture), CHR tile `tile` of the row's background pattern
/// table, in sub-palette `palette`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodedTile {
    pub column: i32,
    pub tile: u8,
    pub palette: u8,
}

/// A margin decoder for one game.
pub trait MarginDecoder: Sync {
    /// Decodes the tiles of `row` at screen columns `columns` (the margins of the frame) into
    /// `out`. Returns `false` when the decoder does not handle this row: the core then keeps
    /// what it captured from the nametables. A column left out shows nothing (backdrop).
    fn decode_row(
        &self,
        memory: &DecoderMemory<'_>,
        row: &BackgroundRow,
        columns: &mut dyn Iterator<Item = i32>,
        out: &mut Vec<DecodedTile>,
    ) -> bool;

    /// Whether the decoder also draws screen column 0 of the rows it decodes while the game
    /// hides the leftmost 8 pixels of the background (PPUMASK bit 1 clear): such a game keeps
    /// there the column it is drawing next (Super Mario Bros. 3), and the wide picture, which
    /// shows those pixels, would show it. `false` by default: column 0 stays the nametables'.
    fn decodes_masked_column(&self) -> bool {
        false
    }

    /// How many of the centre's rightmost columns the decoder redraws on the rows it decodes
    /// (0 by default): a game that shows there colours of the column it is drawing next (Super
    /// Mario Bros. 3: the attributes it writes ahead of the right edge also cover the visible
    /// columns next to it). Their nametable records are replaced, and the wide picture composes
    /// those columns of the decoded rows from the records instead of the PPU's pixels.
    fn redrawn_right_columns(&self) -> i32 {
        0
    }

    /// Decodes the whole of `row`, a row the PPU did not draw: above or below the screen, in
    /// the vertical margin (its `line` is then negative or past 239, and its PPU address is the
    /// one it would have on the plane of two stacked nametables). Returns `false`, as by
    /// default, when the decoder cannot: the vertical margin then stays empty.
    fn decode_extra_row(
        &self,
        _memory: &DecoderMemory<'_>,
        _row: &BackgroundRow,
        _columns: &mut dyn Iterator<Item = i32>,
        _out: &mut Vec<DecodedTile>,
    ) -> bool {
        false
    }

    /// Whether `decode_extra_row` also covers the screen's overscan: the rows asked for then
    /// fill the vertical margin whole whatever the fine Y scroll, the screen's first and last
    /// rows (across its top or bottom edge, or ending on it) are decoded whole width (their
    /// centre too), and the 2D picture composes the screen's lines such rows cover from the
    /// records rather than the PPU's pixels. For a game that shows another row of its nametable
    /// ring there (Adventure Island scrolling vertically). Off by default: `margin_y / 8` rows
    /// each side, the screen the PPU's.
    fn covers_overscan(&self) -> bool {
        false
    }

    /// With `covers_overscan`, the lines from the screen's top and bottom edges within which a
    /// captured row (starting within them at the top, ending within them at the bottom) is
    /// decoded whole width too, and composed from its records in the 2D picture: for a game whose
    /// attribute bytes for the row coming in at one edge also colour the next row, on the screen
    /// (Blaster Master, 8: a television's overscan hid it). 0 by default: the edge rows only.
    fn overscan_lines(&self) -> i32 {
        0
    }
}

/// The decoder made for an unpatched ROM whose data (iNES header excluded) has CRC-32 `crc`, if
/// any: a game a decoder alone widens.
#[must_use]
pub fn for_rom(crc: u32) -> Option<&'static dyn MarginDecoder> {
    match crc {
        megaman2::SOURCE_CRC => Some(&megaman2::MegaMan2),
        smb1::SOURCE_CRC => Some(&smb1::SuperMarioBros),
        adventure_island::SOURCE_CRC => Some(&adventure_island::AdventureIsland),
        ng3::SOURCE_CRC => Some(&ng3::NinjaGaiden3),
        smb3::SOURCE_CRC => Some(&smb3::SuperMarioBros3),
        castlevania::SOURCE_CRC => Some(&castlevania::USA),
        castlevania::SOURCE_CRC_REV1 => Some(&castlevania::USA_REV1),
        excitebike::SOURCE_CRC => Some(&excitebike::Excitebike),
        ff1::SOURCE_CRC => Some(&ff1::FinalFantasy),
        metalforce::SOURCE_CRC => Some(&metalforce::MetalForce),
        blastermaster::SOURCE_CRC => Some(&blastermaster::BlasterMaster),
        metroid::SOURCE_CRC => Some(&metroid::Metroid),
        _ => None,
    }
}

/// The decoder made for the ROM a built-in patch named `patch` applies to, if any.
#[must_use]
pub fn for_patch(patch: &str) -> Option<&'static dyn MarginDecoder> {
    match patch {
        crate::builtin_patches::ZELDA2_USA => Some(&zelda2::Zelda2),
        crate::builtin_patches::MEGAMAN2_USA => Some(&megaman2::MegaMan2),
        crate::builtin_patches::SMB1_WORLD => Some(&smb1::SuperMarioBros),
        crate::builtin_patches::ADVENTURE_ISLAND_USA => Some(&adventure_island::AdventureIsland),
        crate::builtin_patches::EXCITEBIKE => Some(&excitebike::Excitebike),
        crate::builtin_patches::NG3_USA => Some(&ng3::NinjaGaiden3),
        crate::builtin_patches::SMB3_USA => Some(&smb3::SuperMarioBros3),
        crate::builtin_patches::CASTLEVANIA_USA => Some(&castlevania::USA),
        crate::builtin_patches::CASTLEVANIA_USA_REV1 => Some(&castlevania::USA_REV1),
        crate::builtin_patches::FINAL_FANTASY_USA => Some(&ff1::FinalFantasy),
        crate::builtin_patches::METALFORCE_KOREA => Some(&metalforce::MetalForce),
        crate::builtin_patches::BLASTERMASTER_USA => Some(&blastermaster::BlasterMaster),
        _ => None,
    }
}

/// The value congruent to `ring_position` modulo `ring` nearest to `estimate` (ties to the
/// lower one): picks which instance of a wrapping coordinate an estimate stands for.
#[must_use]
pub fn resolve_ring(ring_position: i32, ring: i32, estimate: i32) -> i32 {
    let base = estimate - (estimate - ring_position).rem_euclid(ring);
    let up = base + ring;
    if estimate - base <= up - estimate {
        base
    } else {
        up
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rings_resolve_to_the_nearest_instance() {
        assert_eq!(resolve_ring(10, 512, 0), 10);
        assert_eq!(resolve_ring(500, 512, 0), -12);
        assert_eq!(resolve_ring(10, 512, 600), 522);
        assert_eq!(resolve_ring(0, 256, 128), 0);
    }

    #[test]
    fn rows_read_the_ppu_address() {
        let row = BackgroundRow {
            line: 33,
            v: 0x1000 | 0x0800 | 0x0400 | (5 << 5) | 3,
            fine_x: 6,
        };
        assert_eq!(row.coarse_x(), 3);
        assert_eq!(row.coarse_y(), 5);
        assert_eq!(row.nametable_x(), 1);
        assert_eq!(row.nametable_y(), 1);
        assert_eq!(row.ring_x(), 256 + 24 + 6);
    }
}
