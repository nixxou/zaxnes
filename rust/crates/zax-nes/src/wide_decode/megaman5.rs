//! Mega Man 5 (USA): the margins of a stage, decoded from its layout in PRG-ROM.
//!
//! Mega Man 3's layout (see [`super::megaman3`]) at other addresses. A stage lives in the 8 KiB
//! PRG bank `$27`, which the game's column drawer (`$D758`) maps at `$A000` (while `$26` is `$0E`,
//! each screen's own bank, from the fixed bank's table at `$D793`): screen `s` is room `$A900[s]`,
//! 64 structure bytes at `$B600 + 64 room`; structure `n` is 4 block bytes at `$B200 + 4 n`;
//! block `b` is CHR tiles `$AD00[b]` (top left), `$AE00[b]` (top right), `$AF00[b]` (bottom
//! left), `$B000[b]` (bottom right), in sub-palette `$B100[b] & 3`. The camera is `$F9:$FC`; the
//! section being played is `$2A + 1` screens, the camera on its screen `$2B`.

use super::megaman3::{Layout, StageBank};

/// CRC-32 of the ROM's data, iNES header excluded.
pub const SOURCE_CRC: u32 = 0x3EDC_F7E8;

/// The decoder.
pub const MEGAMAN5: Layout = Layout {
    stage_bank: StageBank::Ram {
        bank: 0x0027,
        switch: 0x0026,
        value: 0x0E,
        per_screen: 0x4000 - 0x1793,
    },
    stage: 0x0027,
    camera_low: 0x00FC,
    camera_high: 0x00F9,
    section_length: 0x002A,
    section_screen: 0x002B,
    rooms: 0x0900,
    layouts: 0x1600,
    structures: 0x1200,
    tiles: 0x0D00,
    palettes: 0x1100,
};
