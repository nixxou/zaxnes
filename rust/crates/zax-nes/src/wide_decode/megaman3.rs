//! Mega Man 3 (USA): the margins of a stage, decoded from its layout in PRG-ROM.
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
//! section, above or below).

use super::{megaman, BackgroundRow, DecodedTile, DecoderMemory, MarginDecoder};

/// The decoder.
pub struct MegaMan3;

/// CRC-32 of the ROM's data, iNES header excluded.
pub const SOURCE_CRC: u32 = 0x603A_AA57;

const STAGE: u16 = 0x0022;
const CAMERA_LOW: u16 = 0x00FC;
const CAMERA_HIGH: u16 = 0x00F9;
const SECTION_LENGTH: u16 = 0x002C;
const SECTION_SCREEN: u16 = 0x002D;

/// The fixed bank's table of each stage's layout bank, from the end of the PRG-ROM.
const STAGE_BANKS: usize = 0x4000 - 0x08B9;
const STAGE_BANK_COUNT: usize = 20;
const BANK_SIZE: usize = 0x2000;

const ROOMS: usize = 0x0A00;
const LAYOUTS: usize = 0x0F00;
const STRUCTURES: usize = 0x1700;
const TILES: usize = 0x1B00;
const PALETTES: usize = 0x1F00;

impl MarginDecoder for MegaMan3 {
    fn decode_row(
        &self,
        memory: &DecoderMemory<'_>,
        row: &BackgroundRow,
        columns: &mut dyn Iterator<Item = i32>,
        out: &mut Vec<DecodedTile>,
    ) -> bool {
        let Some(stage) = stage_bank(memory.prg_rom, memory.ram(STAGE)) else {
            return false;
        };
        let screen = i32::from(memory.ram(CAMERA_HIGH));
        let camera = (screen << 8) | i32::from(memory.ram(CAMERA_LOW));
        let first = screen - i32::from(memory.ram(SECTION_SCREEN));
        let last = first + i32::from(memory.ram(SECTION_LENGTH));
        megaman::decode_row(
            memory,
            *row,
            columns,
            out,
            camera,
            first * 32..(last + 1) * 32,
            &|world, tile_row| tile_at(stage, world, tile_row),
        )
    }
}

/// The 8 KiB bank stage `stage` is laid out in, as the game's column drawer maps it.
fn stage_bank(prg_rom: &[u8], stage: u8) -> Option<&[u8]> {
    let stage = usize::from(stage);
    if stage >= STAGE_BANK_COUNT {
        return None;
    }
    let table = prg_rom.len().checked_sub(STAGE_BANKS)?;
    let bank = usize::from(*prg_rom.get(table + stage)?);
    prg_rom.get(bank * BANK_SIZE..(bank + 1) * BANK_SIZE)
}

/// CHR tile and sub-palette of world tile column `world`, tile row `tile_row` (0-29), from the
/// stage bank `stage`; `None` before the level or past the bank.
fn tile_at(stage: &[u8], world: i32, tile_row: i32) -> Option<(u8, u8)> {
    let screen = usize::try_from(world >> 5).ok()?;
    let row = usize::try_from(tile_row).ok()?;
    let column = usize::try_from(world & 31).ok()?;
    let room = usize::from(*stage.get(ROOMS + screen)?);
    let structure =
        usize::from(*stage.get(LAYOUTS + (room * 64) + ((row >> 2) * 8) + (column >> 2))?);
    let quarter = (((row >> 1) & 1) * 2) + ((column >> 1) & 1);
    let block = usize::from(*stage.get(STRUCTURES + (structure * 4) + quarter)?);
    let corner = ((row & 1) * 2) + (column & 1);
    let tile = *stage.get(TILES + (corner * 0x100) + block)?;
    let palette = *stage.get(PALETTES + block)? & 3;
    Some((tile, palette))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screens_rooms_structures_and_blocks() {
        let mut stage = vec![0u8; BANK_SIZE];
        // Screen 1 is room 2; its structure row 3, column 2 is structure 5.
        stage[ROOMS + 1] = 2;
        stage[LAYOUTS + 128 + 24 + 2] = 5;
        // Structure 5: blocks TL 1, TR 2, BL 3, BR 4.
        stage[STRUCTURES + 20..STRUCTURES + 24].copy_from_slice(&[1, 2, 3, 4]);
        // Block 3: tiles $30-$33, sub-palette 2 (collision bits above).
        for corner in 0..4 {
            stage[TILES + corner * 0x100 + 3] = 0x30 + u8::try_from(corner).unwrap();
        }
        stage[PALETTES + 3] = 0x42;
        // World tile column 32 + 8 is structure column 2 of screen 1; tile row 14 is structure
        // row 3, its lower half.
        assert_eq!(
            tile_at(&stage, 40, 14),
            Some((0x30, 2)),
            "block 3, top left"
        );
        assert_eq!(tile_at(&stage, 41, 14), Some((0x31, 2)), "top right");
        assert_eq!(tile_at(&stage, 40, 15), Some((0x32, 2)), "bottom left");
        assert_eq!(tile_at(&stage, 41, 15), Some((0x33, 2)), "bottom right");
        assert_eq!(tile_at(&stage, -1, 0), None);
    }
}
