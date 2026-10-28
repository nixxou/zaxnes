//! Mega Man (USA): the margins of a stage, decoded from its layout in PRG-ROM.
//!
//! A stage lives in PRG bank `$31` (the stage; the Wily stages, 6 and up, in bank `$31 - 6`), as
//! the game's own column drawer (`$CE5A`, fixed bank) reads it:
//!
//! * screen `s` of the stage is laid out as room `$8C00[s]`, whose 64 bytes the pointer at
//!   `$8C30 + 2 room` gives: 8 columns of 8 structures (32 x 32 pixels), column first, byte
//!   `8 column + row`;
//! * structure `n` is 4 bytes at `$8000 + 4 n`, its four 16 x 16 blocks column first (top left,
//!   bottom left, top right, bottom right); its attribute byte is at `$8300 + n`;
//! * block `b` is CHR tiles `4 b` to `4 b + 3`, column first too; the low six bits of the block
//!   byte are `b`, the high two its collision.
//!
//! The level is a straight list of screens: world x / 256 is the screen. The camera (`$1B:$1A`)
//! is kept between screens `$27` and `$28` of the section being played; the screens past them
//! belong to the next section (above, below, past a door), so nothing is drawn there.

use super::{megaman, BackgroundRow, DecodedTile, DecoderMemory, MarginDecoder};

/// The decoder.
pub struct MegaMan1;

/// CRC-32 of the ROM's data, iNES header excluded.
pub const SOURCE_CRC: u32 = 0x6EE4_BB0A;

const STAGE: u16 = 0x0031;
const CAMERA_LOW: u16 = 0x001A;
const CAMERA_HIGH: u16 = 0x001B;
const SECTION_FIRST: u16 = 0x0027;
const SECTION_LAST: u16 = 0x0028;

const STRUCTURES: usize = 0x0000;
const ATTRIBUTES: usize = 0x0300;
const ROOMS: usize = 0x0C00;
const ROOM_POINTERS: usize = 0x0C30;
const BANK_SIZE: usize = 0x4000;
/// The robot masters' stages; the Wily stages share their banks.
const STAGE_BANKS: u8 = 6;

impl MarginDecoder for MegaMan1 {
    fn decode_row(
        &self,
        memory: &DecoderMemory<'_>,
        row: &BackgroundRow,
        columns: &mut dyn Iterator<Item = i32>,
        out: &mut Vec<DecodedTile>,
    ) -> bool {
        let bank = usize::from(memory.ram(STAGE) % STAGE_BANKS);
        let Some(stage) = memory.prg_rom.get(bank * BANK_SIZE..(bank + 1) * BANK_SIZE) else {
            return false;
        };
        let camera = (i32::from(memory.ram(CAMERA_HIGH)) << 8) | i32::from(memory.ram(CAMERA_LOW));
        let first = i32::from(memory.ram(SECTION_FIRST)) * 32;
        let end = (i32::from(memory.ram(SECTION_LAST)) + 1) * 32;
        megaman::decode_row(
            memory,
            *row,
            columns,
            out,
            camera,
            first..end,
            &|world, tile_row| tile_at(stage, world, tile_row),
        )
    }
}

/// CHR tile and sub-palette of world tile column `world`, tile row `tile_row` (0-29), from the
/// stage bank `stage`; `None` before the level or past the bank.
fn tile_at(stage: &[u8], world: i32, tile_row: i32) -> Option<(u8, u8)> {
    let screen = usize::try_from(world >> 5).ok()?;
    let row = usize::try_from(tile_row).ok()?;
    let column = usize::try_from(world & 31).ok()?;
    let room = usize::from(*stage.get(ROOMS + screen)?);
    let pointer = usize::from(*stage.get(ROOM_POINTERS + (room * 2))?)
        | (usize::from(*stage.get(ROOM_POINTERS + (room * 2) + 1)?) << 8);
    let layout = pointer.checked_sub(0x8000)?;
    let structure = usize::from(*stage.get(layout + ((column >> 2) * 8) + (row >> 2))?);
    let blocks = stage.get(STRUCTURES + (structure * 4)..STRUCTURES + (structure * 4) + 4)?;
    let attribute = *stage.get(ATTRIBUTES + structure)?;
    Some(megaman::structure_tile(
        [blocks[0], blocks[1], blocks[2], blocks[3]],
        attribute,
        world,
        tile_row,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screens_go_through_the_room_table() {
        let mut stage = vec![0u8; BANK_SIZE];
        // Screen 1 is room 3, laid out at $9000.
        stage[ROOMS + 1] = 3;
        stage[ROOM_POINTERS + 6] = 0x00;
        stage[ROOM_POINTERS + 7] = 0x90;
        // Structure column 2, structure row 3: structure 5.
        stage[0x1000 + 16 + 3] = 5;
        stage[STRUCTURES + 20..STRUCTURES + 24].copy_from_slice(&[1, 2, 3, 0xC4]);
        stage[ATTRIBUTES + 5] = 0b11_10_01_00;
        assert_eq!(tile_at(&stage, 40, 12), Some((4, 0)));
        assert_eq!(tile_at(&stage, 43, 15), Some((19, 3)));
        assert_eq!(tile_at(&stage, -1, 0), None);
    }
}
