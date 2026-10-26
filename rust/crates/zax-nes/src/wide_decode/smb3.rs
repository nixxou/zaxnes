//! Super Mario Bros. 3 (USA) (Rev 1): the margins of a level, decoded from its tile map in
//! PRG-RAM.
//!
//! The game lays a whole level out in PRG-RAM before it starts (`Tile_Mem`, `$6000-$794F`) and
//! keeps it up to date as blocks are hit, coins taken or bricks broken; the nametables only show
//! the screen around the camera. A horizontal level is up to 15 screens of 16 x 27 tiles of 16 x 16
//! pixels: screen `s` is `$1B0` bytes at `$6000 + $1B0 s`, row first (`16 row + column`).
//!
//! A tile `t` is four CHR tiles, `layout[256 q + t]` for quarter `q` (0 top left, 1 bottom left,
//! 2 top right, 3 bottom right) of the tileset's layout table, and its sub-palette is `t >> 6`.
//! The layout of tileset `n` (`$070A`) is the pointer `TileLayout_ByTileset[n]` (`$99F7` of PRG
//! bank 30, which the game keeps at `$8000`): at `$A000` of bank `PAGE_A000_ByTileset[n]`
//! (`$83E9`), or at `$C000` of bank `PAGE_C000_ByTileset[n]` (`$83D6`). Banks 0-29 keep their
//! place with the built-in patch; banks 30 and 31 are the last two.
//!
//! The 27 rows of a level are the 54 tile rows of the plane of two stacked nametables (the
//! game's horizontal mirroring), from the top; the camera's y in the level is `Vert_Scroll`
//! (`$FC`, 0 to 239) and its x `Horz_Scroll_Hi:Horz_Scroll` (`$12:$FD`). The status bar is drawn
//! in the rows past the level (54-59).
//!
//! The vertical margin is the level above the screen; below it, under the status bar, it stays
//! empty.
//!
//! A world map is in the same tile map (rows 16-26, up to four screens, those past it filled with
//! tile `$02`), with the layout of tileset 0, in bank 12. A vertical level is not decoded.
//!
//! The decoder checks itself against the PPU: a row whose centre does not show what the tile map
//! gives (the title, a transition) is left to the nametables. Column 0, which the game hides
//! because it holds the column drawn next, and the last 3 columns of the centre, whose colours
//! the attributes written for that column change, are redrawn from the tile map too.

use super::{resolve_ring, BackgroundRow, DecodedTile, DecoderMemory, MarginDecoder};

/// The decoder.
pub struct SuperMarioBros3;

/// CRC-32 of the ROM's data, iNES header excluded: Super Mario Bros. 3 (USA) (Rev 1).
pub const SOURCE_CRC: u32 = 0x2E63_01ED;

const HORZ_SCROLL: u16 = 0x00FD;
const HORZ_SCROLL_HI: u16 = 0x0012;
const VERT_SCROLL: u16 = 0x00FC;
const LEVEL_WIDTH: u16 = 0x0022;
const LEVEL_VERTICAL: u16 = 0x03EF;
const LEVEL_TILESET: u16 = 0x070A;

/// Offsets in PRG bank 30 of the game's tables. The bank is the one before the last, which MMC3
/// fixes at `$8000` in the game's PRG mode: 16 KiB before the end of the PRG-ROM, with the
/// built-in patch (which adds a bank before those two) or without.
const TILE_LAYOUT_BY_TILESET: usize = 0x19F7;
const PAGE_C000_BY_TILESET: usize = 0x03D6;
const PAGE_A000_BY_TILESET: usize = 0x03E9;
const TILESETS: u8 = 19;
const BANK_SIZE: usize = 0x2000;

const SCREEN_BYTES: usize = 0x1B0;
const SCREENS: i32 = 15;
/// Tile rows of a level (27 rows of 16 x 16).
const LEVEL_ROWS: i32 = 54;
/// Tile rows of the plane of two stacked nametables.
const PLANE_ROWS: i32 = 60;

/// A world map: at most 4 screens; its first row under the top border, and the tile of the
/// screens past it.
const MAP_SCREENS: usize = 4;
const MAP_FIRST_ROW: usize = 17 * 16;
const MAP_FILL: u8 = 0x02;
const MAP_LAYOUT_BANK: usize = 12;

/// Lines of the screen.
const SCREEN_LINES: i32 = 240;

/// A line of the playfield (above the status bar) whatever the scroll.
const MIDDLE_LINE: i32 = 96;

/// The centre's rightmost columns redrawn from the tile map.
const RIGHT_COLUMNS: i32 = 3;

/// Screen columns whose tiles are checked against the nametables.
const CHECKED: [i32; 4] = [3, 11, 19, 27];

/// One level as the decoder reads it this frame.
struct Level<'a> {
    tiles: &'a [u8],
    layout: &'a [u8],
    /// Its last screen.
    last_screen: i32,
}

impl Level<'_> {
    fn of<'a>(memory: &DecoderMemory<'a>) -> Option<Level<'a>> {
        if memory.ram(LEVEL_VERTICAL) != 0 {
            return None;
        }
        let tileset = memory.ram(LEVEL_TILESET);
        if tileset >= TILESETS {
            return None;
        }
        let rom = memory.prg_rom;
        let bank_30 = rom.len().checked_sub(2 * BANK_SIZE)?;
        let entry = bank_30 + TILE_LAYOUT_BY_TILESET + (2 * usize::from(tileset));
        let pointer = u16::from_le_bytes([*rom.get(entry)?, *rom.get(entry + 1)?]);
        let pages = bank_30
            + if pointer >= 0xC000 {
                PAGE_C000_BY_TILESET
            } else {
                PAGE_A000_BY_TILESET
            };
        // The world map's entry of the page table is not used: its layout is in bank 12.
        let bank = if tileset == 0 {
            MAP_LAYOUT_BANK
        } else {
            usize::from(*rom.get(pages + usize::from(tileset))?)
        };
        let start = (bank * BANK_SIZE) + (usize::from(pointer) & (BANK_SIZE - 1));
        let layout = rom.get(start..start + 0x400)?;
        let tiles = memory.prg_ram.get(..SCREENS as usize * SCREEN_BYTES)?;
        let last_screen = if tileset == 0 {
            map_last_screen(tiles)
        } else {
            i32::from(memory.ram(LEVEL_WIDTH)).min(SCREENS - 1)
        };
        Some(Level {
            tiles,
            layout,
            last_screen,
        })
    }

    /// CHR tile and sub-palette of level tile column `column`, tile row `row` (0-53); `None`
    /// outside the level.
    fn tile_at(&self, column: i32, row: i32) -> Option<(u8, u8)> {
        if column < 0 || !(0..LEVEL_ROWS).contains(&row) {
            return None;
        }
        let screen = column >> 5;
        if screen > self.last_screen {
            return None;
        }
        let at = (usize::try_from(screen).ok()? * SCREEN_BYTES)
            + (usize::try_from(row >> 1).ok()? * 16)
            + usize::try_from((column >> 1) & 15).ok()?;
        let tile = *self.tiles.get(at)?;
        let quarter = usize::try_from(((column & 1) << 1) | (row & 1)).ok()?;
        let pattern = *self.layout.get((quarter * 256) + usize::from(tile))?;
        Some((pattern, tile >> 6))
    }
}

/// The last screen of a world map: the game fills the screens past it with tile `$02`, the first
/// row under the map's top border (row 17) included.
fn map_last_screen(tiles: &[u8]) -> i32 {
    (0..MAP_SCREENS)
        .take_while(|&screen| tiles.get((screen * SCREEN_BYTES) + MAP_FIRST_ROW) != Some(&MAP_FILL))
        .count()
        .try_into()
        .map_or(0, |screens: i32| (screens - 1).max(0))
}

impl Level<'_> {
    /// Whether the PPU shows the tile map at `row` (level tile row `tile_row`, level tile column
    /// `first` at its left edge), checked at a few columns of the centre; one may lag behind a
    /// block the game has just changed.
    fn shows(
        &self,
        memory: &DecoderMemory<'_>,
        row: BackgroundRow,
        first: i32,
        tile_row: i32,
    ) -> bool {
        let differing = CHECKED
            .iter()
            .filter(|&&column| {
                self.tile_at(first + column, tile_row).map(|(tile, _)| tile)
                    != Some((memory.nametable)(nametable_address(row, column)))
            })
            .count();
        differing <= 1
    }
}

/// The level tile row of `row` (on the plane of two stacked nametables), the camera's y picking
/// the instance: rows past the level (the status bar, above its top) are outside `0..54`.
fn level_row(memory: &DecoderMemory<'_>, row: BackgroundRow) -> i32 {
    let plane = (i32::from(row.nametable_y()) * 30) + i32::from(row.coarse_y());
    let estimate = (i32::from(memory.ram(VERT_SCROLL)) + row.line) >> 3;
    resolve_ring(plane, PLANE_ROWS, estimate)
}

/// The level tile column at the left edge of `row`, the camera's x picking the instance.
fn first_column(memory: &DecoderMemory<'_>, row: BackgroundRow) -> i32 {
    let camera = (i32::from(memory.ram(HORZ_SCROLL_HI)) << 8) | i32::from(memory.ram(HORZ_SCROLL));
    resolve_ring(row.ring_x() & 0xFF, 256, camera) >> 3
}

impl MarginDecoder for SuperMarioBros3 {
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
        let tile_row = level_row(memory, *row);
        if !(0..LEVEL_ROWS).contains(&tile_row) {
            return false;
        }
        let first = first_column(memory, *row);

        if !level.shows(memory, *row, first, tile_row) {
            return false;
        }

        for column in columns {
            if let Some((tile, palette)) = level.tile_at(first + column, tile_row) {
                out.push(DecodedTile {
                    column,
                    tile,
                    palette,
                });
            }
        }
        true
    }

    fn decodes_masked_column(&self) -> bool {
        // The game hides its left 8 pixels: with horizontal mirroring, the column it draws
        // ahead of the right edge is the same nametable column as the one at the left edge.
        true
    }

    fn redrawn_right_columns(&self) -> i32 {
        // The attribute bytes the game writes for the column it draws past the right edge
        // cover the last visible columns too (up to 24 pixels with the fine scroll).
        RIGHT_COLUMNS
    }

    fn decode_extra_row(
        &self,
        memory: &DecoderMemory<'_>,
        row: &BackgroundRow,
        columns: &mut dyn Iterator<Item = i32>,
        out: &mut Vec<DecodedTile>,
    ) -> bool {
        // Below the screen is below the status bar, which the game always draws at the bottom:
        // the level there would stick to it, and nothing of the level goes on under it.
        if row.line >= SCREEN_LINES {
            return true;
        }
        let Some(level) = Level::of(memory) else {
            return false;
        };
        // The PPU did not draw this row: the screen must show the level, checked on a row it
        // did draw, halfway down the playfield.
        let middle_row = (i32::from(memory.ram(VERT_SCROLL)) + MIDDLE_LINE) >> 3;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0..60.
        let plane = middle_row.rem_euclid(PLANE_ROWS) as u16;
        let middle = BackgroundRow {
            line: MIDDLE_LINE,
            v: (row.v & !0x0BE0) | ((plane / 30) << 11) | ((plane % 30) << 5),
            fine_x: row.fine_x,
        };
        let first = first_column(memory, *row);
        if !level.shows(memory, middle, first, middle_row) {
            return false;
        }
        let tile_row = level_row(memory, *row);
        for column in columns {
            if let Some((tile, palette)) = level.tile_at(first + column, tile_row) {
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

    fn level<'a>(tiles: &'a [u8], layout: &'a [u8]) -> Level<'a> {
        Level {
            tiles,
            layout,
            last_screen: 1,
        }
    }

    #[test]
    fn tiles_come_from_the_tile_map_and_the_layout() {
        let mut tiles = vec![0u8; SCREENS as usize * SCREEN_BYTES];
        let mut layout = vec![0u8; 0x400];
        // Screen 1, row 13, column 5: tile $45 (sub-palette 1).
        tiles[SCREEN_BYTES + (13 * 16) + 5] = 0x45;
        for (quarter, pattern) in [0x10u8, 0x11, 0x12, 0x13].into_iter().enumerate() {
            layout[(quarter * 256) + 0x45] = pattern;
        }
        let level = level(&tiles, &layout);
        // Tile column 32 + 10 = 42 is the left half of column 5 of screen 1; tile row 26 the
        // top half of row 13.
        assert_eq!(level.tile_at(42, 26), Some((0x10, 1)), "top left");
        assert_eq!(level.tile_at(42, 27), Some((0x11, 1)), "bottom left");
        assert_eq!(level.tile_at(43, 26), Some((0x12, 1)), "top right");
        assert_eq!(level.tile_at(43, 27), Some((0x13, 1)), "bottom right");
        assert_eq!(level.tile_at(-1, 0), None, "left of the level");
        assert_eq!(level.tile_at(64, 0), None, "past its last screen");
        assert_eq!(level.tile_at(0, 54), None, "below it");
    }
}
