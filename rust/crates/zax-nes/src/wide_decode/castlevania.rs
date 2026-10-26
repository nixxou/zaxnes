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
//!   PPUCTRL nametable bit), read there too: their rows repeat, so trying both ways would at
//!   times take the wrong one;
//! * the map's first block row is nametable row 4; its top two tile rows are under the status
//!   bar, which the game keeps in nametable rows 0-5 (the playfield shows from row 6).
//!
//! A margin column the nametables still hold for its place, as the game drew it, is taken from
//! them (the game redraws some blocks there: crushers, broken walls); leading screens of filler (block numbers past the
//! stage's blocks, behind a wall the camera never passes) are not the room.
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
    /// The table of the stages that swap the nametables.
    swaps: u16,
}

/// The base build.
pub static USA: Castlevania = Castlevania {
    maps: 0xFAFF,
    blocks: 0xF9D6,
    lengths: 0xFB8C,
    swaps: 0xFBB2,
};

/// Rev 1.
pub static USA_REV1: Castlevania = Castlevania {
    maps: 0xFB23,
    blocks: 0xF9FA,
    lengths: 0xFBB0,
    swaps: 0xFBD6,
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
/// The stages, 0-18.
const STAGES: usize = 19;
/// The map column of room x 0.
const FIRST_MAP_COLUMN: usize = 4;
/// The end of a map.
const END: u8 = 0xFF;

/// How many of the centre's 32 columns must show the map for a row to be decoded.
const CENTRE_MATCHES: usize = 24;

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
        // on the ring of two nametables.
        let swap = level.swap * 256;
        let first_column = (resolve_ring(row.ring_x(), 512, level.camera + swap) - swap) >> 3;
        // What the PPU shows in the centre must be what the map gives, give or take the blocks the
        // game redraws itself (crushers, a wall broken: a column it holds for its place).
        let matching = (0..32)
            .filter(|&column| {
                let world = first_column + column;
                level.tile_at(world, coarse_y).is_some_and(|(tile, _)| {
                    (memory.nametable)(nametable_address(*row, column)) == tile
                        || column_is_current(memory, &level, *row, column, world)
                })
            })
            .count();
        if matching < CENTRE_MATCHES {
            return false;
        }
        for column in columns {
            let world = first_column + column;
            if level.tile_at(world, coarse_y).is_none() {
                continue;
            }
            // A column the nametables still hold for this very place shows what the game drew
            // there: its crushers going down, a wall broken.
            let decoded = if column_is_current(memory, &level, *row, column, world) {
                Some(nametable_tile(memory, *row, column))
            } else {
                level.tile_at(world, coarse_y)
            };
            if let Some((tile, palette)) = decoded {
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

/// How many of a column's playfield rows must show the map for the nametables to hold it (a
/// crusher down redraws 9 of the 22).
const CURRENT_ROWS: i32 = 8;

/// The screen columns the nametables hold whole: the game draws a column 96 pixels ahead of the
/// right edge a block at a time and keeps those behind the left one, so within 64 pixels of both
/// edges every column is one it finished drawing for its place.
const HELD_COLUMNS: std::ops::Range<i32> = -8..40;

/// Whether screen column `column` of the nametables (on the plane of two) holds room tile column
/// `world` as the game drew it: a column within 64 pixels of the screen enough of whose playfield
/// rows show what the map gives there (the game redraws some blocks itself, crushers, broken
/// walls, after it drew the column).
fn column_is_current(
    memory: &DecoderMemory<'_>,
    level: &Level<'_>,
    row: BackgroundRow,
    column: i32,
    world: i32,
) -> bool {
    if !HELD_COLUMNS.contains(&column) {
        return false;
    }
    let matching = (PLAYFIELD_TOP..MAP_TOP + 4 * BLOCK_ROWS)
        .filter(|&tile_row| {
            #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)] // 6..28.
            let v = (row.v & !0x03E0) | ((tile_row as u16) << 5);
            let address = nametable_address(BackgroundRow { v, ..row }, column);
            level
                .tile_at(world, tile_row)
                .is_some_and(|(tile, _)| (memory.nametable)(address) == tile)
        })
        .count();
    i32::try_from(matching).unwrap_or(0) >= CURRENT_ROWS
}

/// The tile and sub-palette the nametables hold at screen column `column` of `row`.
fn nametable_tile(memory: &DecoderMemory<'_>, row: BackgroundRow, column: i32) -> (u8, u8) {
    let address = nametable_address(row, column);
    let tile = (memory.nametable)(address);
    let table = address & 0x2C00;
    let tile_column = address & 0x1F;
    let tile_row = (address >> 5) & 0x1F;
    let attribute =
        (memory.nametable)(table | 0x03C0 | ((tile_row >> 2) << 3) | (tile_column >> 2));
    let shift = ((tile_row & 2) << 1) | (tile_column & 2);
    (tile, (attribute >> shift) & 3)
}

/// The room in play: its map and its stage's blocks, in PRG bank 4.
struct Level<'a> {
    bank: &'a [u8],
    map: usize,
    blocks: usize,
    camera: i32,
    /// Where the room starts and ends, in tile columns.
    start: i32,
    end: i32,
    /// 1 when the stage draws room x 0 in the right nametable.
    swap: i32,
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
        // The stage's blocks end where the next block set of the table starts (the last set, at
        // the end of the bank).
        let next_set = (0..STAGES)
            .filter_map(|other| {
                let low = fixed_byte(build.blocks, 2 * other)?;
                let high = fixed_byte(build.blocks, 2 * other + 1)?;
                Some(usize::from(low) | (usize::from(high) << 8))
            })
            .filter(|&set| set > blocks)
            .min()
            .unwrap_or(0x8000 + BANK_SIZE);
        let mut level = Self {
            bank,
            map,
            blocks: blocks.checked_sub(0x8000)?,
            camera,
            start: 0,
            end: (i32::from(last_screen) + 1) * 32,
            swap: i32::from(fixed_byte(build.swaps, stage)? & 1),
        };
        // A room may open on a screen of filler behind a wall that the camera never shows (stage
        // 4): its map holds block numbers past the stage's blocks there. Such leading screens are
        // not the room.
        let count = (next_set - blocks) / BLOCK_BYTES;
        while level.start < level.end && level.screen_has_filler(level.start, count) {
            level.start += 32;
        }
        Some(level)
    }

    /// Whether the screen at tile column `first` holds a block number past the stage's `count`.
    fn screen_has_filler(&self, first: i32, count: usize) -> bool {
        let Ok(first) = usize::try_from(first) else {
            return false;
        };
        (first >> 2..(first >> 2) + 8).any(|column| {
            (0..6).any(|row: usize| {
                let cell = (self.map + (column + FIRST_MAP_COLUMN) * 6 + row)
                    .checked_sub(0x8000)
                    .and_then(|cell| self.bank.get(cell));
                cell.is_some_and(|&block| block != END && usize::from(block) >= count)
            })
        })
    }

    /// CHR tile and sub-palette of room tile column `world`, nametable row `tile_row`; `None`
    /// outside the room or under the playfield.
    fn tile_at(&self, world: i32, tile_row: i32) -> Option<(u8, u8)> {
        if world < self.start || world >= self.end {
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
            start: 0,
            end: 32,
            swap: 0,
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
