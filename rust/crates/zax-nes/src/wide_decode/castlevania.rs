//! Castlevania (USA), both builds: the margins of a stage, decoded from its layout in PRG-ROM.
//!
//! The two USA builds share their data banks 0-4 byte for byte; only the code (banks 5-7) was
//! rebuilt, which moved the tables of the fixed bank. So one decoder serves both, given where
//! its build keeps them.
//!
//! The level is drawn from PRG bank 4, as the game's own block drawer (`$F67A`, base build)
//! reads it:
//!
//! * room `$46` of stage `$28` has a map, pointed at by entry `$46` of the list the fixed
//!   bank's maps table gives for the stage (`$CFD4`, which keeps it in `$33`, but only once the
//!   camera moves): a column of 32 pixels is 6 block numbers, top first, the column at room x
//!   `32 c` at `map + 6 (c + 4)`; the camera's x in the room is `$2F:$2E`;
//! * the stage's blocks (`$28` the stage) are pointed at by the fixed bank's table, entry
//!   `$28`: block `n` is 17 bytes at that pointer `+ 17 n`, its attribute byte, then its 16
//!   tiles row first (32 x 32 pixels, one attribute cell);
//! * a room is as long as its camera goes, plus a screen: the camera stops at the screen the
//!   fixed bank's length table gives for the stage (`$CFF0`: the high nibble for room `$46`
//!   0, the low one for room 1);
//! * some stages draw room x 0 in the right nametable (`$FBB2`, which `$F5EB` turns into the
//!   PPUCTRL nametable bit);
//! * the map's first block row is nametable row 4; its top two tile rows are under the status
//!   bar, which the game keeps in nametable rows 0-5 (the playfield shows from row 6).
//!
//! The decoder checks itself against the PPU: a row whose centre does not show what the map
//! gives (the title, a transition, the status bar) is left to the nametables.

use super::{resolve_ring, BackgroundRow, DecodedTile, DecoderMemory, MarginDecoder};

/// The decoder for one build: where its fixed bank keeps the tables of the stages' maps, of
/// their blocks and of their rooms' lengths.
pub struct Castlevania {
    /// The table of the stages' maps.
    maps: u16,
    /// The table of the stages' blocks.
    blocks: u16,
    /// The table of the stages' rooms' lengths.
    lengths: u16,
}

/// The base build.
pub static USA: Castlevania = Castlevania {
    maps: 0xFAFF,
    blocks: 0xF9D6,
    lengths: 0xFB8C,
};

/// Rev 1.
pub static USA_REV1: Castlevania = Castlevania {
    maps: 0xFB23,
    blocks: 0xF9FA,
    lengths: 0xFBB0,
};

/// CRC-32 of the base build's data (no CHR-ROM, iNES header excluded).
pub const SOURCE_CRC: u32 = 0x0AC1_AA8F;
/// CRC-32 of Rev 1's data.
pub const SOURCE_CRC_REV1: u32 = 0xB668_C7FC;

const STAGE: u16 = 0x0028;
const ROOM: u16 = 0x0046;
const CAMERA_LOW: u16 = 0x002E;
const CAMERA_HIGH: u16 = 0x002F;

const BANK_SIZE: usize = 0x4000;
const LEVEL_BANK: usize = 4;
const FIXED_BANK: usize = 7;
/// The nametable row of the map's first block row, and the first row of the playfield.
const MAP_TOP: i32 = 4;
const PLAYFIELD_TOP: i32 = 6;
const BLOCK_ROWS: i32 = 6;
const BLOCK_BYTES: usize = 17;
/// The map column of room x 0.
const FIRST_MAP_COLUMN: usize = 4;
/// The end of a map.
const END: u8 = 0xFF;

/// Screen columns whose tiles are checked against the nametables.
const CHECKED: [i32; 4] = [3, 11, 19, 27];

impl MarginDecoder for Castlevania {
    fn decode_row(
        &self,
        memory: &DecoderMemory<'_>,
        row: &BackgroundRow,
        columns: &mut dyn Iterator<Item = i32>,
        out: &mut Vec<DecodedTile>,
    ) -> bool {
        let coarse_y = i32::from(row.coarse_y());
        if !(PLAYFIELD_TOP..MAP_TOP + 4 * BLOCK_ROWS).contains(&coarse_y) {
            return false;
        }
        let Some(level) = Level::of(self, memory) else {
            return false;
        };
        // Some stages draw room x 0 in the right nametable: the camera is then 256 pixels away
        // on the ring of two nametables. The one the centre shows is the one.
        let Some(first_column) = [0, 256].into_iter().find_map(|swap| {
            let first = (resolve_ring(row.ring_x(), 512, level.camera + swap) - swap) >> 3;
            CHECKED
                .iter()
                .all(|&column| {
                    level
                        .tile_at(first + column, coarse_y)
                        .is_some_and(|(tile, _)| {
                            (memory.nametable)(nametable_address(*row, column)) == tile
                        })
                })
                .then_some(first)
        }) else {
            return false;
        };
        for column in columns {
            if let Some((tile, palette)) = level.tile_at(first_column + column, coarse_y) {
                out.push(DecodedTile {
                    column,
                    tile,
                    palette,
                });
            }
        }
        true
    }
}

/// The room in play: its map and its stage's blocks, in PRG bank 4.
struct Level<'a> {
    bank: &'a [u8],
    map: usize,
    blocks: usize,
    camera: i32,
    /// Where the room ends, in tile columns.
    end: i32,
}

impl<'a> Level<'a> {
    fn of(build: &Castlevania, memory: &DecoderMemory<'a>) -> Option<Self> {
        let bank = memory
            .prg_rom
            .get(LEVEL_BANK * BANK_SIZE..(LEVEL_BANK + 1) * BANK_SIZE)?;
        let fixed = memory
            .prg_rom
            .get(FIXED_BANK * BANK_SIZE..(FIXED_BANK + 1) * BANK_SIZE)?;
        let fixed_byte = |address: u16, offset: usize| {
            fixed
                .get(usize::from(address.checked_sub(0xC000)?) + offset)
                .copied()
        };
        let stage = usize::from(memory.ram(STAGE));
        let blocks = usize::from(fixed_byte(build.blocks, 2 * stage)?)
            | (usize::from(fixed_byte(build.blocks, 2 * stage + 1)?) << 8);
        // The camera's last screen: the high nibble for the stage's first room, the low one
        // for its second.
        let lengths = fixed_byte(build.lengths, stage)?;
        let last_screen = if memory.ram(ROOM) == 0 {
            lengths >> 4
        } else {
            lengths & 0x0F
        };
        // The room's map, as `$CFD4` finds it: `$33` is only set once the camera moves.
        let maps = usize::from(fixed_byte(build.maps, 2 * stage)?)
            | (usize::from(fixed_byte(build.maps, 2 * stage + 1)?) << 8);
        let room = maps.checked_sub(0x8000)? + 2 * usize::from(memory.ram(ROOM));
        let map = usize::from(*bank.get(room)?) | (usize::from(*bank.get(room + 1)?) << 8);
        let camera = (i32::from(memory.ram(CAMERA_HIGH)) << 8) | i32::from(memory.ram(CAMERA_LOW));
        Some(Self {
            bank,
            map,
            blocks: blocks.checked_sub(0x8000)?,
            camera,
            end: (i32::from(last_screen) + 1) * 32,
        })
    }

    /// CHR tile and sub-palette of room tile column `world`, nametable row `tile_row`; `None`
    /// outside the room or under the playfield.
    fn tile_at(&self, world: i32, tile_row: i32) -> Option<(u8, u8)> {
        if world >= self.end {
            return None;
        }
        let world = usize::try_from(world).ok()?;
        let in_map = usize::try_from(tile_row - MAP_TOP).ok()?;
        let block_row = in_map >> 2;
        if block_row >= 6 {
            return None;
        }
        let cell =
            (self.map + ((world >> 2) + FIRST_MAP_COLUMN) * 6 + block_row).checked_sub(0x8000)?;
        let block = *self.bank.get(cell)?;
        if block == END {
            return None;
        }
        let at = self.blocks + usize::from(block) * BLOCK_BYTES;
        let attribute = *self.bank.get(at)?;
        let tile = *self.bank.get(at + 1 + (in_map & 3) * 4 + (world & 3))?;
        let palette = (attribute >> (((in_map & 2) << 1) | (world & 2))) & 3;
        Some((tile, palette))
    }
}

/// The nametable address of screen column `column` (0 = the left edge) of `row`, on the plane
/// of two nametables side by side.
fn nametable_address(row: BackgroundRow, column: i32) -> u16 {
    let plane = (i32::from(row.nametable_x()) * 32) + i32::from(row.coarse_x()) + column;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0..64.
    let plane = plane.rem_euclid(64) as u16;
    0x2000
        | (u16::from(row.nametable_y()) << 11)
        | ((plane >> 5) << 10)
        | (u16::from(row.coarse_y()) << 5)
        | (plane & 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiles_come_from_the_map_and_the_blocks() {
        let mut bank = vec![0u8; BANK_SIZE];
        // A map at $8100 (its room x 0 four columns in), blocks at $8200.
        let map = 0x8100;
        let blocks = 0x200;
        // Room column 1 (tile columns 4-7), block row 2: block 3.
        bank[0x100 + (1 + FIRST_MAP_COLUMN) * 6 + 2] = 3;
        // Block 3: attribute TL 0, TR 1, BL 2, BR 3; tile 16 r + c.
        let at = blocks + 3 * BLOCK_BYTES;
        bank[at] = 0b11_10_01_00;
        for (index, tile) in bank[at + 1..at + 17].iter_mut().enumerate() {
            *tile = u8::try_from(index).unwrap() + 0x40;
        }
        // The next column ends the map.
        bank[0x100 + (2 + FIRST_MAP_COLUMN) * 6 + 2] = END;
        let level = Level {
            bank: &bank,
            map,
            blocks,
            camera: 0,
            end: 32,
        };
        // Block row 2 is nametable rows 12-15.
        assert_eq!(level.tile_at(4, 12), Some((0x40, 0)), "top left");
        assert_eq!(level.tile_at(7, 12), Some((0x43, 1)), "top right");
        assert_eq!(level.tile_at(5, 14), Some((0x49, 2)), "bottom left");
        assert_eq!(level.tile_at(6, 15), Some((0x4E, 3)), "bottom right");
        assert_eq!(level.tile_at(8, 12), None, "the end of the map");
        assert_eq!(level.tile_at(-1, 12), None, "before the room");
        assert_eq!(level.tile_at(4, 28), None, "under the playfield");
        let short = Level { end: 4, ..level };
        assert_eq!(short.tile_at(4, 12), None, "past the room");
    }
}
