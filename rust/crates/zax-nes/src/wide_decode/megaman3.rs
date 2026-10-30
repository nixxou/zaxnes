//! Mega Man 3 (USA): the margins of a stage, decoded from its layout in PRG-ROM; and the layout
//! the later games keep (Mega Man 4 to 6: [`Layout`], their addresses in their own modules).
//!
//! A stage lives in the 8 KiB PRG bank the fixed bank's table at `$C8B9` gives for the stage
//! (`$22`), which the game's column drawer (`$E882`) maps at `$A000`:
//!
//! * screen `s` of the stage is laid out as room `$AA00[s]`, 64 bytes at `$AF00 + 64 room`: 8 rows
//!   of 8 structures (32 x 32 pixels), row first, byte `8 row + column` (the terrain lookup at
//!   `$E913` reads it so);
//! * structure `n` is 4 bytes at `$B700 + 4 n`, its four 16 x 16 blocks row first (top left, top
//!   right, bottom left, bottom right);
//! * block `b` is CHR tiles `$BB00[b]` (top left), `$BC00[b]` (top right), `$BD00[b]` (bottom
//!   left), `$BE00[b]` (bottom right), in sub-palette `$BF00[b] & 3` (the high bits its
//!   collision).
//!
//! The level is a straight list of screens: world x / 256 is the screen. The camera is
//! `$F9:$FC`; the section being played is `$2C + 1` screens, the camera on its screen `$2D`, so
//! from screen `$F9 - $2D` on; nothing is drawn past it (the next screens belong to another
//! section, above or below). While the game scrolls to such a section (`$FA` non-zero) the
//! margins stay empty.

use super::{megaman, BackgroundRow, DecodedTile, DecoderMemory, MarginDecoder};

/// CRC-32 of the ROM's data, iNES header excluded.
pub const SOURCE_CRC: u32 = 0x603A_AA57;
/// CRC-32 of the other builds' data with the same stage layouts and stage table: Rockman 3
/// (Japan) and Mega Man 3 (Europe).
pub const OTHER_CRCS: [u32; 2] = [0xD9F1_E47C, 0x837A_3D8A];
/// CRC-32 of Mega Man 3 (Europe) (Rev 1)'s data: its stage table moved.
pub const SOURCE_CRC_EUROPE_REV1: u32 = 0x0013_88B3;

/// Where a game of the series keeps a stage, as offsets in the stage's 8 KiB bank (seen at
/// `$A000`), and its camera and section in RAM.
pub struct Layout {
    /// Which 8 KiB bank holds the stage.
    pub stage_bank: StageBank,
    /// The stage number.
    pub stage: u16,
    /// The camera's x: pixel and screen.
    pub camera_low: u16,
    pub camera_high: u16,
    /// The vertical scroll: non-zero only while the game scrolls to another section, above or
    /// below (the camera and the section are the new one's, the rows on the screen both's).
    pub scroll_y: u16,
    /// The section being played: its last screen, counted from its first, and the camera's.
    pub section_length: u16,
    pub section_screen: u16,
    /// Screen to room, one byte a screen.
    pub rooms: usize,
    /// The rooms' layouts, 64 bytes each.
    pub layouts: usize,
    /// The structures, 4 bytes each.
    pub structures: usize,
    /// The blocks' corners, four tables of 256 (top left, top right, bottom left, bottom right).
    pub tiles: usize,
    /// The blocks' sub-palettes (low two bits).
    pub palettes: usize,
}

/// How a game finds a stage's bank.
pub enum StageBank {
    /// From a table of `count` banks, by stage, `offset` bytes before the end of the PRG-ROM.
    Table { offset: usize, count: usize },
    /// The stage number with these bits set.
    Or(u8),
    /// The bank RAM byte `bank` holds; while RAM byte `switch` is `value`, each screen's own,
    /// from the table `per_screen` bytes before the end of the PRG-ROM (Mega Man 5).
    Ram {
        bank: u16,
        switch: u16,
        value: u8,
        per_screen: usize,
    },
}

/// Mega Man 3 (USA).
pub const MEGAMAN3: Layout = Layout {
    stage_bank: StageBank::Table {
        offset: 0x4000 - 0x08B9,
        count: 20,
    },
    stage: 0x0022,
    camera_low: 0x00FC,
    camera_high: 0x00F9,
    scroll_y: 0x00FA,
    section_length: 0x002C,
    section_screen: 0x002D,
    rooms: 0x0A00,
    layouts: 0x0F00,
    structures: 0x1700,
    tiles: 0x1B00,
    palettes: 0x1F00,
};

/// Mega Man 3 (Europe) (Rev 1): its stage table $38 bytes further, as the code around it.
pub const MEGAMAN3_EUROPE_REV1: Layout = Layout {
    stage_bank: StageBank::Table {
        offset: 0x4000 - 0x08F1,
        count: 20,
    },
    ..MEGAMAN3
};

const BANK_SIZE: usize = 0x2000;

impl MarginDecoder for Layout {
    fn decode_row(
        &self,
        memory: &DecoderMemory<'_>,
        row: &BackgroundRow,
        columns: &mut dyn Iterator<Item = i32>,
        out: &mut Vec<DecodedTile>,
    ) -> bool {
        if memory.ram(self.scroll_y) != 0 || self.stage_bank(memory, 0).is_none() {
            return false;
        }
        let screen = i32::from(memory.ram(self.camera_high));
        let camera = (screen << 8) | i32::from(memory.ram(self.camera_low));
        let first = screen - i32::from(memory.ram(self.section_screen));
        let last = first + i32::from(memory.ram(self.section_length));
        megaman::decode_row(
            memory,
            *row,
            columns,
            out,
            camera,
            first * 32..(last + 1) * 32,
            &|world, tile_row| {
                let stage = self.stage_bank(memory, world >> 5)?;
                self.tile_at(stage, world, tile_row)
            },
        )
    }

    /// The games hide the screen's leftmost 8 pixels while they scroll, and keep there a column
    /// they are drawing (Mega Man 5 shows it in the middle of a wide view otherwise).
    fn decodes_masked_column(&self) -> bool {
        true
    }

    /// The column the game draws past the screen's right edge takes its colours from attribute
    /// bytes it writes later: the screen's last column shows them first (a stripe of the next
    /// structure's colours at the edge of the original picture).
    fn redrawn_right_columns(&self) -> i32 {
        1
    }

    /// The game draws the rows coming in at the screen's top or bottom edge a piece at a time
    /// while it scrolls to another section (which a television hid): those 8 lines stay hidden
    /// with a vertical margin as without one, when the host hides them.
    fn hides_overscan(&self) -> bool {
        true
    }
}

impl Layout {
    /// The 8 KiB bank screen `screen` of the stage is laid out in, as the game's column drawer
    /// maps it.
    fn stage_bank<'a>(&self, memory: &DecoderMemory<'a>, screen: i32) -> Option<&'a [u8]> {
        let prg_rom = memory.prg_rom;
        let stage = memory.ram(self.stage);
        let bank = match self.stage_bank {
            StageBank::Table { offset, count } => {
                if usize::from(stage) >= count {
                    return None;
                }
                let table = prg_rom.len().checked_sub(offset)?;
                usize::from(*prg_rom.get(table + usize::from(stage))?)
            }
            StageBank::Or(bits) => usize::from(stage | bits),
            StageBank::Ram {
                bank,
                switch,
                value,
                per_screen,
            } => {
                if memory.ram(switch) == value {
                    let table = prg_rom.len().checked_sub(per_screen)?;
                    usize::from(*prg_rom.get(table + usize::try_from(screen).ok()?)?)
                } else {
                    usize::from(memory.ram(bank))
                }
            }
        };
        prg_rom.get(bank * BANK_SIZE..(bank + 1) * BANK_SIZE)
    }

    /// CHR tile and sub-palette of world tile column `world`, tile row `tile_row` (0-29), from
    /// the stage bank `stage`; `None` before the level or past the bank.
    fn tile_at(&self, stage: &[u8], world: i32, tile_row: i32) -> Option<(u8, u8)> {
        let screen = usize::try_from(world >> 5).ok()?;
        let row = usize::try_from(tile_row).ok()?;
        let column = usize::try_from(world & 31).ok()?;
        let room = usize::from(*stage.get(self.rooms + screen)?);
        let structure =
            usize::from(*stage.get(self.layouts + (room * 64) + ((row >> 2) * 8) + (column >> 2))?);
        let quarter = (((row >> 1) & 1) * 2) + ((column >> 1) & 1);
        let block = usize::from(*stage.get(self.structures + (structure * 4) + quarter)?);
        let corner = ((row & 1) * 2) + (column & 1);
        let tile = *stage.get(self.tiles + (corner * 0x100) + block)?;
        let palette = *stage.get(self.palettes + block)? & 3;
        Some((tile, palette))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screens_rooms_structures_and_blocks() {
        let layout = &MEGAMAN3;
        let mut stage = vec![0u8; BANK_SIZE];
        // Screen 1 is room 2; its structure row 3, column 2 is structure 5.
        stage[layout.rooms + 1] = 2;
        stage[layout.layouts + 128 + 24 + 2] = 5;
        // Structure 5: blocks TL 1, TR 2, BL 3, BR 4.
        stage[layout.structures + 20..layout.structures + 24].copy_from_slice(&[1, 2, 3, 4]);
        // Block 3: tiles $30-$33, sub-palette 2 (collision bits above).
        for corner in 0..4 {
            stage[layout.tiles + corner * 0x100 + 3] = 0x30 + u8::try_from(corner).unwrap();
        }
        stage[layout.palettes + 3] = 0x42;
        // World tile column 32 + 8 is structure column 2 of screen 1; tile row 14 is structure
        // row 3, its lower half.
        assert_eq!(
            layout.tile_at(&stage, 40, 14),
            Some((0x30, 2)),
            "block 3, top left"
        );
        assert_eq!(layout.tile_at(&stage, 41, 14), Some((0x31, 2)), "top right");
        assert_eq!(
            layout.tile_at(&stage, 40, 15),
            Some((0x32, 2)),
            "bottom left"
        );
        assert_eq!(
            layout.tile_at(&stage, 41, 15),
            Some((0x33, 2)),
            "bottom right"
        );
        assert_eq!(layout.tile_at(&stage, -1, 0), None);
    }

    #[test]
    fn stage_banks_from_a_table_bits_or_ram() {
        let mut prg = vec![0u8; 8 * BANK_SIZE];
        let end = prg.len();
        prg[end - 0x100 + 2] = 5;
        prg[end - 0x80 + 3] = 6;
        prg[5 * BANK_SIZE] = 0xAB;
        prg[6 * BANK_SIZE] = 0xCD;
        let mut ram = vec![0u8; 0x800];
        ram[0x22] = 2;
        ram[0x27] = 5;
        let nametable = |_| 0;
        let written = |_| 0;
        let memory = |ram: &[u8]| -> Vec<u8> { ram.to_vec() };
        let ram_copy = memory(&ram);
        let decoder_memory = DecoderMemory {
            ram: &ram_copy,
            prg_ram: &[],
            prg_rom: &prg,
            nametable: &nametable,
            nametable_written: &written,
        };
        let table = Layout {
            stage_bank: StageBank::Table {
                offset: 0x100,
                count: 4,
            },
            ..MEGAMAN3
        };
        assert_eq!(
            table.stage_bank(&decoder_memory, 0).map(|bank| bank[0]),
            Some(0xAB)
        );
        // Stage 2 with bit 2: bank 6.
        let bits = Layout {
            stage_bank: StageBank::Or(0x04),
            ..MEGAMAN3
        };
        assert_eq!(
            bits.stage_bank(&decoder_memory, 0).map(|bank| bank[0]),
            Some(0xCD)
        );
        let in_ram = Layout {
            stage_bank: StageBank::Ram {
                bank: 0x0027,
                switch: 0x0026,
                value: 0x0E,
                per_screen: 0x80,
            },
            ..MEGAMAN3
        };
        assert_eq!(
            in_ram.stage_bank(&decoder_memory, 3).map(|bank| bank[0]),
            Some(0xAB)
        );
        ram[0x26] = 0x0E;
        let ram_copy = memory(&ram);
        let decoder_memory = DecoderMemory {
            ram: &ram_copy,
            ..decoder_memory
        };
        assert_eq!(
            in_ram.stage_bank(&decoder_memory, 3).map(|bank| bank[0]),
            Some(0xCD)
        );
    }
}
