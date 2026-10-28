//! Mega Man 4 (USA), both builds: the margins of a stage, decoded from its layout in PRG-ROM.
//!
//! Mega Man 3's layout (see [`super::megaman3`]) at other addresses. A stage lives in the 8 KiB
//! PRG bank `$20 | $22` (`$22` the stage), which the game's column drawer (`$D295`) maps at
//! `$A000`: screen `s` is room `$B500[s]`, 64 structure bytes at `$A900 + 64 room`; structure
//! `n` is 4 block bytes at `$A500 + 4 n`; block `b` is CHR tiles `$A000[b]` (top left),
//! `$A100[b]` (top right), `$A200[b]` (bottom left), `$A300[b]` (bottom right), in sub-palette
//! `$A400[b] & 3`. The camera is `$F9:$FC`; the section being played is `$2D + 1` screens, the
//! camera on its screen `$2E`. The Rev 1 build differs from the first in three bytes, none of
//! them here.

use super::megaman3::{Layout, StageBank};

/// CRC-32 of the ROM's data, iNES header excluded.
pub const SOURCE_CRC: u32 = 0x2BC6_7AA8;
/// The same for the Rev 1 build.
pub const SOURCE_CRC_REV1: u32 = 0x18A2_E74F;

/// The decoder.
pub const MEGAMAN4: Layout = Layout {
    stage_bank: StageBank::Or(0x20),
    stage: 0x0022,
    camera_low: 0x00FC,
    camera_high: 0x00F9,
    section_length: 0x002D,
    section_screen: 0x002E,
    rooms: 0x1500,
    layouts: 0x0900,
    structures: 0x0500,
    tiles: 0x0000,
    palettes: 0x0400,
};
