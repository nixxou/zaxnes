//! Metal Force (Korea): the margins of a level, decoded from its map in PRG-RAM.
//!
//! The game keeps the whole level in the 8 KiB of PRG-RAM, as its column drawer (`$E3DB`) and
//! the pointer it uses (`$E529`) read it:
//!
//! * the map is a grid of 16 x 16 metatiles at `$6000`, column after column, `$1800` bytes; a
//!   column is `H` metatiles high, `H` = 16, 24, 32, 48, 64, 80, 96, 112, 128, 144, 160 for the
//!   level shape `$0352` = 0 to 10, 40 past it. Map rows count **upwards**: row `m` of a column
//!   is above row `m - 1`;
//! * metatile `n` is 5 bytes at `$7800 + 5 n`: its CHR tiles top left, top right, bottom left,
//!   bottom right, then its sub-palette in the low two bits.
//!
//! The camera is `$35:$34` (world x of the screen's left edge, pixels) and `$3D`, the map row
//! at the bottom of the screen. The PPU shows one nametable (one-screen mirroring): a ring of
//! 32 x 15 metatiles where map row `m` is nametable metatile row `(14 - m) mod 15`.
//!
//! A level is made of rooms: the game keeps the camera within `$0356:$0355`-`$035A:$0359` (x)
//! and `$0358:$0357`-`$035C:$035B` (y of the screen's bottom edge, upwards), the map past them
//! holding the other rooms. The margins show the room only: x up to the last camera x + 256, y
//! up to the last camera y + 240.
//!
//! The decoder checks itself against the PPU: a row whose centre does not show what the map
//! gives (the mission title, a transition) is left to the nametables.

use super::{resolve_ring, BackgroundRow, DecodedTile, DecoderMemory, MarginDecoder};

/// The decoder.
pub struct MetalForce;

/// CRC-32 of the ROM's data, iNES header excluded.
pub const SOURCE_CRC: u32 = 0xEC6D_910E;

const CAMERA_LOW: u16 = 0x0034;
const CAMERA_HIGH: u16 = 0x0035;
const BOTTOM_ROW: u16 = 0x003D;
const SHAPE: u16 = 0x0352;
const ROOM_X_MIN: u16 = 0x0355;
const ROOM_Y_MIN: u16 = 0x0357;
const ROOM_X_MAX: u16 = 0x0359;
const ROOM_Y_MAX: u16 = 0x035B;

/// Offsets in PRG-RAM.
const MAP: usize = 0x0000;
const MAP_SIZE: usize = 0x1800;
const METATILES: usize = 0x1800;

/// Metatile rows of the nametable ring.
const RING_ROWS: i32 = 15;

/// Screen columns whose tiles are checked against the nametables.
const CHECKED: [i32; 4] = [3, 11, 19, 27];

impl MarginDecoder for MetalForce {
    fn decode_row(
        &self,
        memory: &DecoderMemory<'_>,
        row: &BackgroundRow,
        columns: &mut dyn Iterator<Item = i32>,
        out: &mut Vec<DecodedTile>,
    ) -> bool {
        let Some(level) = Level::of(memory) else {
            return false;
        };
        let first_column = level.first_column(*row);
        let map_row = level.map_row(*row, row.line);
        if !level.centre_matches(memory, *row, first_column, map_row) {
            return false;
        }
        level.decode(first_column, map_row, columns, out);
        true
    }

    fn decode_extra_row(
        &self,
        memory: &DecoderMemory<'_>,
        row: &BackgroundRow,
        columns: &mut dyn Iterator<Item = i32>,
        out: &mut Vec<DecodedTile>,
    ) -> bool {
        let Some(level) = Level::of(memory) else {
            return false;
        };
        // The nametable row this one would read is on the screen too, one ring away: the
        // map must match it there.
        let first_column = level.first_column(*row);
        let shown = level.map_row(*row, row.line.clamp(0, 239));
        if !level.centre_matches(memory, *row, first_column, shown) {
            return false;
        }
        level.decode(first_column, level.map_row(*row, row.line), columns, out);
        true
    }

    /// The one nametable: the screen's edge tiles show the column the game is drawing for the
    /// other side. Column 0, hidden by PPUMASK, is decoded...
    fn decodes_masked_column(&self) -> bool {
        true
    }

    /// ...and the last one, whose right part comes from the 33rd tile of a line, the same
    /// nametable column as the first.
    fn redrawn_right_columns(&self) -> i32 {
        1
    }
}

/// The level in play.
struct Level<'a> {
    map: &'a [u8],
    metatiles: &'a [u8],
    height: i32,
    camera: i32,
    bottom_row: i32,
    /// The room, in world pixels: x from the first to past the last, y (upwards) the same.
    room_x: (i32, i32),
    room_y: (i32, i32),
}

impl<'a> Level<'a> {
    fn of(memory: &DecoderMemory<'a>) -> Option<Self> {
        let map = memory.prg_ram.get(MAP..MAP + MAP_SIZE)?;
        let metatiles = memory.prg_ram.get(METATILES..METATILES + 256 * 5)?;
        Some(Self {
            map,
            metatiles,
            height: column_height(memory.ram(SHAPE)),
            camera: (i32::from(memory.ram(CAMERA_HIGH)) << 8) | i32::from(memory.ram(CAMERA_LOW)),
            bottom_row: i32::from(memory.ram(BOTTOM_ROW)),
            room_x: (word(memory, ROOM_X_MIN), word(memory, ROOM_X_MAX) + 256),
            room_y: (word(memory, ROOM_Y_MIN), word(memory, ROOM_Y_MAX) + 240),
        })
    }

    /// World tile column of the row's screen column 0.
    fn first_column(&self, row: BackgroundRow) -> i32 {
        resolve_ring(row.ring_x() & 0xFF, 256, self.camera) >> 3
    }

    /// World tile row (map row x 2 + 1 for the upper tile) of the row, `line` being where it
    /// shows on the screen.
    fn map_row(&self, row: BackgroundRow, line: i32) -> i32 {
        let tile_row = i32::from(row.coarse_y());
        let ring = (14 - (tile_row >> 1)).rem_euclid(RING_ROWS);
        let estimate = self.bottom_row + 14 - line.div_euclid(16);
        resolve_ring(ring, RING_ROWS, estimate) * 2 + 1 - (tile_row & 1)
    }

    fn centre_matches(
        &self,
        memory: &DecoderMemory<'_>,
        row: BackgroundRow,
        first_column: i32,
        tile_row: i32,
    ) -> bool {
        CHECKED.iter().all(|&column| {
            self.tile_at(first_column + column, tile_row)
                .is_some_and(|(tile, _)| (memory.nametable)(nametable_address(row, column)) == tile)
        })
    }

    fn decode(
        &self,
        first_column: i32,
        tile_row: i32,
        columns: &mut dyn Iterator<Item = i32>,
        out: &mut Vec<DecodedTile>,
    ) {
        for column in columns {
            if !self.in_room(first_column + column, tile_row) {
                continue;
            }
            if let Some((tile, palette)) = self.tile_at(first_column + column, tile_row) {
                out.push(DecodedTile {
                    column,
                    tile,
                    palette,
                });
            }
        }
    }

    /// Whether the tile at world tile column `column`, world tile row `tile_row` (upwards) shows
    /// any of the room.
    fn in_room(&self, column: i32, tile_row: i32) -> bool {
        let (x, y) = (column * 8, tile_row * 8);
        x + 8 > self.room_x.0 && x < self.room_x.1 && y + 8 > self.room_y.0 && y < self.room_y.1
    }

    /// CHR tile and sub-palette at world tile column `column`, world tile row `tile_row`
    /// (counting upwards, 2 m + 1 the upper tile of map row m); `None` off the map.
    fn tile_at(&self, column: i32, tile_row: i32) -> Option<(u8, u8)> {
        if column < 0 || tile_row < 0 || tile_row >= self.height * 2 {
            return None;
        }
        let index = usize::try_from((column >> 1) * self.height + (tile_row >> 1)).ok()?;
        let metatile = usize::from(*self.map.get(index)?) * 5;
        let lower = usize::from(tile_row & 1 == 0);
        let right = usize::try_from(column & 1).ok()?;
        let tile = *self.metatiles.get(metatile + lower * 2 + right)?;
        let palette = self.metatiles.get(metatile + 4)? & 3;
        Some((tile, palette))
    }
}

/// The 16-bit value at `address` (low byte first).
fn word(memory: &DecoderMemory<'_>, address: u16) -> i32 {
    i32::from(memory.ram(address)) | (i32::from(memory.ram(address + 1)) << 8)
}

/// Metatiles in a map column for level shape `shape` (`$E529`).
fn column_height(shape: u8) -> i32 {
    match shape {
        0 => 16,
        1 => 24,
        2 => 32,
        3 => 48,
        4 => 64,
        5 => 80,
        6 => 96,
        7 => 112,
        8 => 128,
        9 => 144,
        10 => 160,
        _ => 40,
    }
}

/// The nametable address of screen column `column` (0 = the left edge) of `row`, on the one
/// nametable the game shows.
fn nametable_address(row: BackgroundRow, column: i32) -> u16 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0..32.
    let tile_column = (i32::from(row.coarse_x()) + column).rem_euclid(32) as u16;
    0x2000 | (u16::from(row.coarse_y()) << 5) | tile_column
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level<'a>(map: &'a [u8], metatiles: &'a [u8]) -> Level<'a> {
        Level {
            map,
            metatiles,
            height: 16,
            camera: 0,
            bottom_row: 0,
            room_x: (0, i32::MAX),
            room_y: (0, i32::MAX),
        }
    }

    /// Map columns of a 16-metatile-high level.
    const MAP_COLUMNS_16: i32 = 0x1800 / 16;

    #[test]
    fn tiles_come_from_the_map_and_metatiles() {
        let mut map = vec![0u8; MAP_SIZE];
        let mut metatiles = vec![0u8; 256 * 5];
        // Map column 3, row 2 (from the bottom): metatile 7.
        map[3 * 16 + 2] = 7;
        metatiles[35..40].copy_from_slice(&[0x10, 0x11, 0x20, 0x21, 0x02]);
        let level = level(&map, &metatiles);
        assert_eq!(level.tile_at(6, 5), Some((0x10, 2)), "top left");
        assert_eq!(level.tile_at(7, 5), Some((0x11, 2)), "top right");
        assert_eq!(level.tile_at(6, 4), Some((0x20, 2)), "bottom left");
        assert_eq!(level.tile_at(7, 4), Some((0x21, 2)), "bottom right");
        assert_eq!(level.tile_at(-1, 4), None);
        assert_eq!(level.tile_at(6, 32), None, "above the map");
        assert_eq!(level.tile_at(2 * MAP_COLUMNS_16, 4), None, "past it");
    }

    #[test]
    fn the_margins_show_the_room_only() {
        let map = vec![0u8; MAP_SIZE];
        let metatiles = vec![0u8; 256 * 5];
        let mut level = level(&map, &metatiles);
        // A room of one screen: camera x 512 only, y 0 to 16.
        level.room_x = (512, 512 + 256);
        level.room_y = (0, 16 + 240);
        assert!(!level.in_room(63, 0), "left of the room");
        assert!(level.in_room(64, 0));
        assert!(level.in_room(95, 0));
        assert!(!level.in_room(96, 0), "right of it");
        assert!(level.in_room(64, 31));
        assert!(!level.in_room(64, 32), "above it");
    }

    #[test]
    fn map_rows_count_up_the_nametable_ring() {
        let map = vec![0u8; MAP_SIZE];
        let metatiles = vec![0u8; 256 * 5];
        let mut level = level(&map, &metatiles);
        level.height = 48;
        level.bottom_row = 24;
        // Map row 24 is nametable metatile row (14 - 24) mod 15 = 5, tiles 10 and 11.
        let row = |tile_row: u16| BackgroundRow {
            line: 232,
            v: tile_row << 5,
            fine_x: 0,
        };
        assert_eq!(level.map_row(row(10), 224), 49, "upper tile of row 24");
        assert_eq!(level.map_row(row(11), 232), 48, "lower tile of row 24");
        // Twelve tile rows higher on the screen, six map rows up.
        assert_eq!(level.map_row(row(28), 128), 61);
    }
}
