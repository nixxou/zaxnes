//! Kirby's Adventure: the margins of a room, read from its map in PRG-RAM.
//!
//! The game unpacks the whole room into PRG-RAM when it enters it, so the decoder reads nothing
//! from the ROM:
//!
//! * the room is a grid of screens of 16 x 12 blocks of 16 x 16 pixels; screen `k` is at
//!   `$68C8 + $C0 k`, its blocks row by row (16 a row), 16 screens at most;
//! * `$67EE` is the room's width in screens, and `$676D + r` the number of the first screen of
//!   screen row `r` (the game's own lookup, `$ECF1` in the fixed bank);
//! * a block's four tiles are at `$7A00`, `$7B00`, `$7C00` and `$7D00` (top left, top right,
//!   bottom left, bottom right) indexed by the block, its sub-palette in the low two bits of
//!   `$7E00 +` the block;
//! * the camera is `$DE:$DD` + `$01A3` across and `$E1:$E0` + `$01A4` down, in the room's pixels,
//!   the picture's top left corner (the frame being drawn's own: the game writes the scroll from
//!   them in its NMI).
//!
//! A row whose two edge columns are not what the map gives there (the status bar under the
//! playfield, a cut scene, the title) keeps the nametables'.

use super::{BackgroundRow, DecodedTile, DecoderMemory, MarginDecoder};

/// The decoder.
pub struct Kirby;

/// CRC-32 of the ROM's data, iNES header excluded: (USA) (Rev 1).
pub const SOURCE_CRC: u32 = 0x5ED6_F221;

const CAMERA_X: u16 = 0x00DD;
const CAMERA_X_HIGH: u16 = 0x00DE;
const CAMERA_Y: u16 = 0x00E0;
const CAMERA_Y_HIGH: u16 = 0x00E1;
const CAMERA_X_FINE: u16 = 0x01A3;
const CAMERA_Y_FINE: u16 = 0x01A4;

/// PRG-RAM, from `$6000`.
const ROOM_WIDTH: usize = 0x07EE;
const SCREEN_ROWS: usize = 0x076D;
const SCREENS: usize = 0x08C8;
const SCREEN_SIZE: usize = 0xC0;
const MAX_SCREENS: usize = 16;
const SCREEN_HEIGHT: i32 = 192;
const BLOCK_TILES: usize = 0x1A00;
const BLOCK_ATTRIBUTES: usize = 0x1E00;

impl MarginDecoder for Kirby {
    fn decode_row(
        &self,
        memory: &DecoderMemory<'_>,
        row: &BackgroundRow,
        columns: &mut dyn Iterator<Item = i32>,
        out: &mut Vec<DecodedTile>,
    ) -> bool {
        let Some(room) = Room::new(memory) else {
            return false;
        };
        // `$01A3` and `$01A4` shake the picture (when Kirby lands hard, a few pixels either
        // way): signed.
        let camera_x = (i32::from(memory.ram(CAMERA_X_HIGH)) << 8)
            + i32::from(memory.ram(CAMERA_X))
            + i32::from(i8::from_ne_bytes([memory.ram(CAMERA_X_FINE)]));
        let camera_y = (i32::from(memory.ram(CAMERA_Y_HIGH)) << 8)
            + i32::from(memory.ram(CAMERA_Y))
            + i32::from(i8::from_ne_bytes([memory.ram(CAMERA_Y_FINE)]));
        // While the camera moves, the work RAM is a frame ahead of the picture: the row's own
        // scroll places it. The game draws the room in one nametable's width (the horizontal
        // nametable bit plays no part), so that scroll repeats every 256 pixels across the room,
        // and the camera only picks the repeat (the one whose left edge is nearest to it).
        let ring = row.ring_x() & 0xFF;
        // (A shake may put the picture a few pixels before the room's start: the columns there
        // have nothing.)
        let left = camera_x - (camera_x - ring + 128).rem_euclid(256) + 128;
        let first = left >> 3;
        // Down, the same, but the game lays the room out in two ways: a screen of the room (192
        // lines) in the nametable's first 24 tile rows while the camera rests on it, and the
        // nametable's 30 tile rows in turn while it scrolls down the room (falling into the hub).
        // The row's tile row repeats every 24 or every 30 rows, the camera (a frame early, a few
        // lines off at most) picks the repeat, and the row's edge columns, which must show what
        // the map gives there, pick the layout.
        let fine_y = i32::from(row.v >> 12);
        let ring_row = i32::from(row.coarse_y());
        let near = (camera_y + row.line - fine_y) >> 3;
        // The candidate of each layout, the nearest to the camera's row first (on a plain sky
        // both may pass).
        let mut candidates = [24, 30].map(|repeat| {
            (ring_row < repeat)
                .then(|| near - (near - ring_row + repeat / 2).rem_euclid(repeat) + repeat / 2)
        });
        candidates
            .sort_by_key(|tile_row| tile_row.map_or(i32::MAX, |tile_row| (tile_row - near).abs()));
        let Some(tile_row) = candidates
            .into_iter()
            .flatten()
            .find(|&tile_row| tile_row >= 0 && edges_match(&room, memory, *row, first, tile_row))
        else {
            return false;
        };

        for column in columns {
            if let Some((tile, palette)) = room.tile(first + column, tile_row) {
                out.push(DecodedTile {
                    column,
                    tile,
                    palette,
                });
            }
        }
        true
    }

    /// The status bar's split comes 1 to 7 lines under the playfield's last row the PPU kept,
    /// depending on the camera's height: those lines would have no margins.
    fn decodes_split_strips(&self) -> bool {
        true
    }

    /// The game hides the background's leftmost 8 pixels (PPUMASK `$18`) and draws there the
    /// column coming in.
    fn decodes_masked_column(&self) -> bool {
        true
    }

    /// The attribute byte the game writes for the column coming in under the mask also colours
    /// the visible column next to it (scrolling right) ...
    fn redrawn_left_columns(&self) -> i32 {
        2
    }

    /// ... and, scrolling left, the column at the right edge sharing its place in the nametable.
    fn redrawn_right_columns(&self) -> i32 {
        2
    }
}

/// Whether the row shows what the map gives at tile row `tile_row`, the row's first tile column
/// being `first`: every column from the third to the thirtieth, one aside (an object the game
/// draws in the nametable, a block being broken). Two columns alone were not enough: a screen
/// that is not the room (the intro, a level's title card) matched the last room's map there now
/// and then, and its rows got that room's margins.
fn edges_match(
    room: &Room<'_>,
    memory: &DecoderMemory<'_>,
    row: BackgroundRow,
    first: i32,
    tile_row: i32,
) -> bool {
    // Not the first two columns: turning round to scroll left, the game draws the column coming
    // in a frame late, and for that frame the second column (under the mask but for its last two
    // pixels) still shows the other side of the room; nor the last one, coming in scrolling
    // right.
    let mut misses = 0;
    for column in 2..=29 {
        let shown = (memory.nametable)(nametable_address(row, column));
        if room.tile(first + column, tile_row).map(|(tile, _)| tile) != Some(shown) {
            misses += 1;
            if misses > 1 {
                return false;
            }
        }
    }
    true
}

/// The PPU address of screen column `column` of `row`, on the plane of two nametables side by
/// side.
fn nametable_address(row: BackgroundRow, column: i32) -> u16 {
    let ring = ((row.ring_x() >> 3) + column).rem_euclid(64);
    let nametable = u16::try_from(ring / 32).unwrap_or(0);
    let x = u16::try_from(ring % 32).unwrap_or(0);
    0x2000
        | (u16::from(row.nametable_y()) << 11)
        | (nametable << 10)
        | (u16::from(row.coarse_y()) << 5)
        | x
}

/// The room in PRG-RAM.
struct Room<'a> {
    ram: &'a [u8],
    width: i32,
}

impl<'a> Room<'a> {
    fn new(memory: &DecoderMemory<'a>) -> Option<Self> {
        let ram = memory.prg_ram;
        let width = i32::from(*ram.get(ROOM_WIDTH)?);
        (width > 0 && ram.len() > BLOCK_ATTRIBUTES + 0xFF).then_some(Self { ram, width })
    }

    /// The tile and sub-palette at tile column `x`, tile row `y` of the room.
    fn tile(&self, x: i32, y: i32) -> Option<(u8, u8)> {
        if x < 0 || y < 0 {
            return None;
        }
        let screen_x = x >> 5;
        if screen_x >= self.width {
            return None;
        }
        let pixel_y = y << 3;
        let screen_row = usize::try_from(pixel_y / SCREEN_HEIGHT).ok()?;
        let block_row = usize::try_from((pixel_y % SCREEN_HEIGHT) >> 4).ok()?;
        let first = usize::from(*self.ram.get(SCREEN_ROWS + screen_row)?);
        let screen = first + usize::try_from(screen_x).ok()?;
        if screen >= MAX_SCREENS {
            return None;
        }
        let block_column = usize::try_from((x >> 1) & 15).ok()?;
        let block = usize::from(
            *self
                .ram
                .get(SCREENS + screen * SCREEN_SIZE + block_row * 16 + block_column)?,
        );
        let quarter = usize::try_from((y & 1) * 2 + (x & 1)).ok()?;
        let tile = *self.ram.get(BLOCK_TILES + quarter * 0x100 + block)?;
        let palette = self.ram.get(BLOCK_ATTRIBUTES + block)? & 3;
        Some((tile, palette))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_block_s_tiles_come_from_its_screen_and_its_four_tables() {
        let mut ram = vec![0u8; 0x2000];
        ram[ROOM_WIDTH] = 2;
        ram[SCREEN_ROWS] = 0;
        ram[SCREEN_ROWS + 1] = 2;
        // Screen 1 (the second across), block row 3, block column 5: block 7.
        ram[SCREENS + SCREEN_SIZE + 3 * 16 + 5] = 7;
        for quarter in 0..4 {
            ram[BLOCK_TILES + quarter * 0x100 + 7] = 0x40 + u8::try_from(quarter).unwrap();
        }
        ram[BLOCK_ATTRIBUTES + 7] = 0xF2;
        let room = Room {
            ram: &ram,
            width: 2,
        };
        // Tile column 32 + 10 + 1, tile row 6 + 1: the block's bottom right.
        assert_eq!(room.tile(43, 7), Some((0x43, 2)));
        assert_eq!(room.tile(42, 6), Some((0x40, 2)));
        assert_eq!(room.tile(64, 6), None);
    }
}
