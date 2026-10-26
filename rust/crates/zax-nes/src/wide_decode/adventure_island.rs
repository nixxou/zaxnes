//! Adventure Island (USA): the margins of a round, laid out from its tables in PRG-ROM.
//!
//! The game draws a round one tile column at a time, a little past the right edge of the screen
//! (`$B4C3`, `$B4E9`, `$B53A`, `$B55A`, `$B397`), from tables whose addresses it keeps in RAM for
//! the round (`$1A`-`$27`):
//!
//! * the round is a list of sections of 16 tile columns (128 pixels), 3 bytes each at
//!   `($1A) + 3 s`: the first byte's high six bits are the section's screen, its low two the
//!   camera's vertical move; the second byte's high three bits are the slope of the ground (see
//!   below);
//! * screen `n` is 3 bytes at `($1C) + 3 n`: four structures of 4 tile columns (32 pixels), six
//!   bits each, the first in the high bits;
//! * structure `t` is 2 bytes at `($1E) + 2 t`: its top part (6 bytes at `($20) + 6 top`) and its
//!   bottom part (5 bytes at `($22) + 5 bottom`), lists of 32 x 32 metatiles from the top down. A
//!   tile column is 56 tiles high: the top list's first metatile three times, its other five, the
//!   bottom list's five, and its last again;
//! * metatile `m` is 16 tiles at `($24) + 16 m`, row by row, and its attribute byte, the
//!   sub-palettes of its four 16 x 16 quarters as an attribute table byte has them, at
//!   `($26) + m`.
//!
//! A tile column is put on a plane of rows the camera moves over: its first tile at the row
//! `$31` holds when the game draws it, the nametables holding the rows of that plane modulo 30
//! (the game scrolls vertically too, the rows it shows starting near `$32`). A section with a
//! slope moves `$31` by a step every 8 or 16 columns (`$BE0D`: masks `$BE5D`, steps `$BE63`), so
//! the row of every column follows from the one being drawn: the column of world tile
//! `16 $30 + ($35 & 15)`, at `$31`.
//!
//! The game also puts things over the tables as it draws a column (the cloud platforms of the sea
//! rounds, `$A629`, from the round's object list). A margin column the game drew already (left of
//! the one being drawn) is shown as the nametables hold it: a tile that differs from the tables is
//! kept when it was written in or after the frame of the column's rows that match them, an older
//! one belonging to another column of the ring. A column not drawn yet comes from the tables.
//!
//! The decoder checks the centre of each row: a row whose centre does not show what the tables
//! give, or what the game drew over them, is left to the nametables (the title, the screens between
//! rounds).
//!
//! The rows of the vertical margin come from the tables: the column's rows past the screen (and
//! past its top and bottom, its first and last metatiles going on), while the screen's nearest
//! row shows the round.

use super::{resolve_ring, BackgroundRow, DecodedTile, DecoderMemory, MarginDecoder};

/// The decoder.
pub struct AdventureIsland;

/// CRC-32 of the ROM's data, iNES header excluded.
pub const SOURCE_CRC: u32 = 0xF8A7_13BE;

/// The round's tables: sections, screens, structures, top and bottom parts, metatiles, their
/// attributes (pointers, low byte first).
const TABLES: u16 = 0x001A;
const SECTION: u16 = 0x0030;
const COLUMN_ROW: u16 = 0x0031;
const WINDOW_ROW: u16 = 0x0032;
const COLUMN: u16 = 0x0035;
/// The round's length in sections.
const SECTIONS: u16 = 0x002F;
/// The masks and steps of the slopes (`$BE5D`, `$BE63`), indexed by the slope minus one.
const SLOPE_MASKS: u16 = 0xBE5D;
const SLOPE_STEPS: u16 = 0xBE63;
const SLOPES: u8 = 6;

/// The tile rows of a column.
const COLUMN_TILES: i32 = 56;
/// The rows of the plane the nametables hold.
const RING_ROWS: i32 = 30;
/// How far left of the column being drawn the screen's left edge is, in pixels.
const DRAW_LEAD: i32 = 264;
/// Screen columns whose tiles are checked against the nametables.
const CHECKED: [i32; 4] = [3, 11, 19, 27];

impl MarginDecoder for AdventureIsland {
    fn decode_row(
        &self,
        memory: &DecoderMemory<'_>,
        row: &BackgroundRow,
        columns: &mut dyn Iterator<Item = i32>,
        out: &mut Vec<DecodedTile>,
    ) -> bool {
        if i32::from(row.coarse_y()) >= RING_ROWS {
            return false;
        }
        let view = View::new(memory);
        let first_column = view.first_column(*row);
        let plane_row = View::plane_row(memory, *row);
        if !view.centre_matches(memory, *row, first_column, plane_row) {
            return false;
        }
        for column in columns {
            let world = first_column + column;
            // A column the game drew already shows what it drew, what it puts over the tables
            // included; one it has not drawn yet, the tables.
            let drawn = (world < view.drawing)
                .then(|| view.drawn(memory, *row, column, world, plane_row))
                .flatten();
            let decoded = drawn
                .as_ref()
                .and_then(|drawn| drawn.cell(memory, *row, column))
                .or_else(|| view.tile(world, plane_row));
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

    /// The rows above and below the screen: the plane's, from the tables (the nametables hold the
    /// screen's rows only), while the screen's nearest row shows the round.
    fn decode_extra_row(
        &self,
        memory: &DecoderMemory<'_>,
        row: &BackgroundRow,
        columns: &mut dyn Iterator<Item = i32>,
        out: &mut Vec<DecodedTile>,
    ) -> bool {
        let view = View::new(memory);
        // The screen's row nearest this one.
        let rows_away = if row.line < 0 {
            -((-row.line + 7) / 8)
        } else {
            (row.line - (VISIBLE_LINES - 8)).max(0) / 8
        };
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0..30.
        let screen_y = (i32::from(row.coarse_y()) - rows_away).rem_euclid(RING_ROWS) as u16;
        let screen_row = BackgroundRow {
            line: row.line - (8 * rows_away),
            v: (row.v & !0x03E0) | (screen_y << 5),
            fine_x: row.fine_x,
        };
        let first_column = view.first_column(*row);
        if !view.centre_matches(
            memory,
            screen_row,
            first_column,
            View::plane_row(memory, screen_row),
        ) {
            return false;
        }
        let plane_row = View::plane_row(memory, *row);
        for column in columns {
            if let Some((tile, palette)) = view.tile(first_column + column, plane_row) {
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

/// The lines of the screen.
const VISIBLE_LINES: i32 = 240;

/// The round as the screen shows it this frame.
struct View<'a> {
    round: Round<'a>,
    rows: ColumnRows,
    /// The world tile column the game is drawing.
    drawing: i32,
}

impl<'a> View<'a> {
    fn new(memory: &DecoderMemory<'a>) -> Self {
        let round = Round::new(memory);
        let drawing = (i32::from(memory.ram(SECTION)) * 16) + i32::from(memory.ram(COLUMN) & 15);
        let rows = ColumnRows::new(&round, drawing, i32::from(memory.ram(COLUMN_ROW)));
        Self {
            round,
            rows,
            drawing,
        }
    }

    /// The world tile column at the left edge of `row`.
    fn first_column(&self, row: BackgroundRow) -> i32 {
        resolve_ring(row.ring_x(), 512, (self.drawing * 8) - DRAW_LEAD) >> 3
    }

    /// The row of the plane `row` shows: the one congruent to its nametable row nearest the rows
    /// the game keeps drawn.
    fn plane_row(memory: &DecoderMemory<'_>, row: BackgroundRow) -> i32 {
        resolve_ring(
            i32::from(row.coarse_y()),
            RING_ROWS,
            i32::from(memory.ram(WINDOW_ROW)) + (row.line >> 3),
        )
    }

    /// CHR tile and sub-palette of world tile column `world` at row `plane_row` of the plane.
    fn tile(&self, world: i32, plane_row: i32) -> Option<(u8, u8)> {
        self.round.tile(world, plane_row - self.rows.at(world))
    }

    /// Whether the centre of `row` shows what the tables give, or what the game drew over them
    /// when it drew the column (the cloud platforms of the sea rounds).
    fn centre_matches(
        &self,
        memory: &DecoderMemory<'_>,
        row: BackgroundRow,
        first_column: i32,
        plane_row: i32,
    ) -> bool {
        CHECKED.iter().all(|&column| {
            self.drawn(memory, row, column, first_column + column, plane_row)
                .and_then(|drawn| drawn.cell(memory, row, column))
                .is_some()
        })
    }
}

/// The round being played: its tables, as the game's RAM points to them, in PRG-ROM.
struct Round<'a> {
    prg_rom: &'a [u8],
    /// `$1A`-`$27`: sections, screens, structures, top parts, bottom parts, metatiles,
    /// attributes.
    tables: [u16; 7],
    /// Its length in tile columns.
    columns: i32,
}

impl<'a> Round<'a> {
    fn new(memory: &DecoderMemory<'a>) -> Self {
        let mut tables = [0; 7];
        for (index, table) in (0u16..).zip(tables.iter_mut()) {
            *table = u16::from_le_bytes([
                memory.ram(TABLES + (2 * index)),
                memory.ram(TABLES + (2 * index) + 1),
            ]);
        }
        Self {
            prg_rom: memory.prg_rom,
            tables,
            columns: i32::from(memory.ram(SECTIONS)) * 16,
        }
    }

    /// The PRG-ROM byte at CPU address `address` (`$8000-$FFFF`).
    fn byte(&self, address: u32) -> Option<u8> {
        let offset = usize::try_from(address.checked_sub(0x8000)?).ok()?;
        self.prg_rom.get(offset).copied()
    }

    /// Byte `index` of table `table`.
    fn table(&self, table: usize, index: u32) -> Option<u8> {
        self.byte(u32::from(self.tables[table]) + index)
    }

    /// Byte `index` (0-2) of section `section`.
    fn section(&self, section: i32, index: u32) -> Option<u8> {
        let section = u32::try_from(section).ok()?;
        self.table(0, (3 * section) + index)
    }

    /// CHR tile and sub-palette of tile row `tile_row` of world tile column `world` (0-55: the
    /// column; past its top and bottom its first and last metatiles go on, as the game repeats
    /// them within the column); `None` off the round.
    fn tile(&self, world: i32, tile_row: i32) -> Option<(u8, u8)> {
        if !(0..self.columns).contains(&world) {
            return None;
        }
        let tile_row = if tile_row < 0 {
            tile_row.rem_euclid(4)
        } else if tile_row >= COLUMN_TILES {
            COLUMN_TILES - 4 + (tile_row - COLUMN_TILES).rem_euclid(4)
        } else {
            tile_row
        };
        let screen = u32::from(self.section(world >> 4, 0)? >> 2);
        let structures = (0..3).try_fold(0u32, |bits, index| {
            Some((bits << 8) | u32::from(self.table(1, (3 * screen) + index)?))
        })?;
        let shift = 18 - (6 * ((world >> 2) & 3));
        let structure = (structures >> shift) & 0x3F;
        // The column's metatiles, top down: the top part's first three times, its other five,
        // the bottom part's five and its last again.
        let metatile_row = tile_row >> 2;
        let metatile = if metatile_row < 8 {
            let top = u32::from(self.table(2, 2 * structure)?);
            let index = u32::try_from(metatile_row - 2).unwrap_or(0);
            self.table(3, (6 * top) + index)?
        } else {
            let bottom = u32::from(self.table(2, (2 * structure) + 1)?);
            let index = u32::try_from(metatile_row - 8).map_or(0, |index| index.min(4));
            self.table(4, (5 * bottom) + index)?
        };
        let column = u32::try_from(world & 3).ok()?;
        let row = u32::try_from(tile_row & 3).ok()?;
        let tile = self.table(5, (16 * u32::from(metatile)) + (4 * row) + column)?;
        let attribute = self.table(6, u32::from(metatile))?;
        let shift = (u8::from(tile_row & 2 != 0) * 4) + (u8::from(world & 2 != 0) * 2);
        Some((tile, (attribute >> shift) & 3))
    }

    /// The slope of section `section`: its mask and step, or `None` on flat ground.
    fn slope(&self, section: i32) -> Option<(i32, i32)> {
        let slope = self.section(section, 1)? >> 5;
        if slope == 0 || slope > SLOPES {
            return None;
        }
        let mask = self.byte(u32::from(SLOPE_MASKS + u16::from(slope) - 1))?;
        let step = self.byte(u32::from(SLOPE_STEPS + u16::from(slope) - 1))?;
        #[allow(clippy::cast_possible_wrap)] // A signed step.
        Some((i32::from(mask), i32::from(step as i8)))
    }
}

/// The row of the plane each tile column starts at, worked out from the column being drawn.
struct ColumnRows {
    /// The first column known, and the row of each column from it.
    first: i32,
    rows: Vec<i32>,
}

/// Columns worked out on each side of the one being drawn: past them, a column takes the row
/// of the nearest known one.
const KNOWN_COLUMNS: i32 = 160;

impl ColumnRows {
    fn new(round: &Round<'_>, drawn: i32, row: i32) -> Self {
        // A column moves the row by its section's step when its index is a multiple of the
        // section's mask (`$BE0D`, run when the game moves on to that column).
        let step = |world: i32| {
            round
                .slope(world >> 4)
                .filter(|(mask, _)| world & mask == 0)
                .map_or(0, |(_, step)| step)
        };
        let first = drawn - KNOWN_COLUMNS;
        let mut rows = vec![0; usize::try_from(2 * KNOWN_COLUMNS + 1).unwrap_or(0)];
        let at = usize::try_from(KNOWN_COLUMNS).unwrap_or(0);
        rows[at] = row;
        for index in at + 1..rows.len() {
            let world = first + i32::try_from(index).unwrap_or(0);
            rows[index] = rows[index - 1] + step(world);
        }
        for index in (0..at).rev() {
            let world = first + i32::try_from(index).unwrap_or(0) + 1;
            rows[index] = rows[index + 1] - step(world);
        }
        Self { first, rows }
    }

    /// The row of the plane world tile column `world` starts at.
    fn at(&self, world: i32) -> i32 {
        let last = self.rows.len().saturating_sub(1);
        let index = usize::try_from(world - self.first).map_or(0, |index| index.min(last));
        self.rows.get(index).copied().unwrap_or(0)
    }
}

impl View<'_> {
    /// What the nametables hold of world tile column `world` at screen column `column`: the frame
    /// the game drew it in, the newest of its rows that show what the tables give, when enough of
    /// them do. What the game drew over the tables then (or later) is newer than that; the rows
    /// the column was not drawn on (the plane's rows past the screen when the game drew it) are
    /// older.
    fn drawn(
        &self,
        memory: &DecoderMemory<'_>,
        row: BackgroundRow,
        column: i32,
        world: i32,
        plane_row: i32,
    ) -> Option<DrawnColumn> {
        let coarse_y = i32::from(row.coarse_y());
        let mut frame = 0u32;
        let mut matching = 0;
        for tile_row in 0..RING_ROWS {
            let address = nametable_address(row_at(row, tile_row), column);
            // The rows of the screen are those of the plane around this one.
            let offset =
                (tile_row - coarse_y + (RING_ROWS / 2)).rem_euclid(RING_ROWS) - (RING_ROWS / 2);
            if let Some((tile, _)) = self.tile(world, plane_row + offset) {
                if (memory.nametable)(address) == tile {
                    matching += 1;
                    frame = frame.max((memory.nametable_written)(address));
                }
            }
        }
        (frame > 0 && matching >= 4).then(|| DrawnColumn {
            frame,
            tables: self.tile(world, plane_row),
        })
    }
}

/// A column the nametables hold as the game drew it.
struct DrawnColumn {
    /// The frame the game drew it in.
    frame: u32,
    /// What the tables give at the row asked for.
    tables: Option<(u8, u8)>,
}

impl DrawnColumn {
    /// The tile and sub-palette at screen column `column` of `row`: the nametables', when they
    /// show the tables or something the game drew over them since the column; `None` for a
    /// tile older than the column (another column's, before the ring came round).
    fn cell(
        &self,
        memory: &DecoderMemory<'_>,
        row: BackgroundRow,
        column: i32,
    ) -> Option<(u8, u8)> {
        let address = nametable_address(row, column);
        let shown = nametable_tile(memory, row, column);
        match self.tables {
            // The tables' sub-palette: the attribute bytes of the margins may be another column's.
            Some(tables) if tables.0 == shown.0 => Some(tables),
            _ => ((memory.nametable_written)(address) >= self.frame).then_some(shown),
        }
    }
}

/// `row` at tile row `tile_row` of its nametable.
fn row_at(row: BackgroundRow, tile_row: i32) -> BackgroundRow {
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)] // 0..30.
    let v = (row.v & !0x03E0) | ((tile_row as u16) << 5);
    BackgroundRow { v, ..row }
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

    /// A round of two sections in a 32 KiB PRG-ROM: its tables at `$9000` up, the slopes'
    /// tables where the game has them.
    fn round_image() -> (Vec<u8>, [u8; 0x800]) {
        let mut prg = vec![0u8; 0x8000];
        let mut ram = [0u8; 0x800];
        let tables: [u16; 7] = [0x9000, 0x9100, 0x9200, 0x9300, 0x9400, 0x9500, 0x9600];
        for (index, table) in tables.iter().enumerate() {
            ram[0x1A + (2 * index)..0x1C + (2 * index)].copy_from_slice(&table.to_le_bytes());
        }
        ram[usize::from(SECTIONS)] = 2;
        let at = |address: u16| usize::from(address - 0x8000);
        prg[at(SLOPE_MASKS)..at(SLOPE_MASKS) + 6].copy_from_slice(&[7, 7, 15, 15, 15, 15]);
        prg[at(SLOPE_STEPS)..at(SLOPE_STEPS) + 6].copy_from_slice(&[0xFE, 2, 0xFC, 4, 0xF8, 8]);
        // Section 0: screen 1, flat; section 1: screen 0, slope 2 (2 rows down every 8 columns).
        prg[at(0x9000)..at(0x9006)].copy_from_slice(&[1 << 2, 0, 0, 0, 2 << 5, 0]);
        // Screen 1: structures 3, 0, 0, 0 (six bits each, the first in the high bits).
        prg[at(0x9103)..at(0x9106)].copy_from_slice(&[3 << 2, 0, 0]);
        // Structure 3: top part 1, bottom part 2.
        prg[at(0x9206)..at(0x9208)].copy_from_slice(&[1, 2]);
        // Top part 1: metatiles 4 (three times), 5, 5, 5, 5, 6; bottom part 2: 7, 7, 7, 7, 8.
        prg[at(0x9306)..at(0x930C)].copy_from_slice(&[4, 5, 5, 5, 5, 6]);
        prg[at(0x940A)..at(0x940F)].copy_from_slice(&[7, 7, 7, 7, 8]);
        // Metatile m: tile 16 m + 4 row + column; its quarters' sub-palettes 0, 1, 2, 3.
        for metatile in 0u8..16 {
            for index in 0u8..16 {
                prg[at(0x9500) + (16 * usize::from(metatile)) + usize::from(index)] =
                    metatile.wrapping_mul(16).wrapping_add(index);
            }
            prg[at(0x9600) + usize::from(metatile)] = 0b11_10_01_00;
        }
        (prg, ram)
    }

    fn memory<'a>(prg: &'a [u8], ram: &'a [u8]) -> DecoderMemory<'a> {
        DecoderMemory {
            ram,
            prg_ram: &[],
            prg_rom: prg,
            nametable: &|_| 0,
            nametable_written: &|_| 0,
        }
    }

    #[test]
    fn a_column_is_56_tiles_of_its_structure_s_metatiles() {
        let (prg, ram) = round_image();
        let round = Round::new(&memory(&prg, &ram));
        // World column 1 is column 1 of structure 3 (the first of screen 1).
        assert_eq!(
            round.tile(1, 0),
            Some((0x41, 0)),
            "the top part's first metatile"
        );
        assert_eq!(
            round.tile(1, 11),
            Some((0x4D, 2)),
            "repeated down to row 11"
        );
        assert_eq!(round.tile(1, 12), Some((0x51, 0)), "then its second");
        assert_eq!(
            round.tile(3, 31),
            Some((0x6F, 3)),
            "its last, lower right quarter"
        );
        assert_eq!(round.tile(1, 32), Some((0x71, 0)), "the bottom part");
        assert_eq!(round.tile(1, 52), Some((0x81, 0)), "its last metatile");
        assert_eq!(round.tile(1, 55), Some((0x8D, 2)), "repeated to the bottom");
        assert_eq!(
            round.tile(1, 56),
            Some((0x81, 0)),
            "the last metatile goes on below"
        );
        assert_eq!(round.tile(1, -3), Some((0x45, 0)), "the first one above");
        assert_eq!(round.tile(32, 0), None, "past the round");
        // Columns 4-15 are structure 0, all metatile 0.
        assert_eq!(round.tile(6, 2), Some((0x0A, 3)), "lower right quarter");
    }

    #[test]
    fn slopes_move_the_rows_of_the_columns() {
        let (prg, ram) = round_image();
        let round = Round::new(&memory(&prg, &ram));
        // The column being drawn is world column 10, at row 100; section 1 (from 16) moves the
        // row by 2 at columns 16 and 24.
        let rows = ColumnRows::new(&round, 10, 100);
        assert_eq!(rows.at(0), 100);
        assert_eq!(rows.at(15), 100);
        assert_eq!(rows.at(16), 102);
        assert_eq!(rows.at(23), 102);
        assert_eq!(rows.at(24), 104);
        assert_eq!(rows.at(31), 104);
        let behind = ColumnRows::new(&round, 30, 104);
        assert_eq!(behind.at(16), 102);
        assert_eq!(behind.at(15), 100);
    }
}
