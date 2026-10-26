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
//! one belonging to another column of the ring. The column the game is putting together comes
//! from its buffer once complete (`$0460`); a column not drawn yet from the tables, with the
//! platforms the game will draw over it (`Platforms`): the one being drawn, the one asked for,
//! and those still in the round's object lists, read ahead.
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
        let platforms = Platforms::new(memory, &view);
        for column in columns {
            let decoded = view.decoded(memory, &platforms, *row, column, first_column, plane_row);
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

    /// The rows at the screen's top and bottom edges may show a row of the game's ring that is
    /// not the plane's (past the screen, or a whole row when the vertical scroll is a multiple
    /// of 8 and the 30 rows of the ring all show): the decoder gives them.
    fn covers_overscan(&self) -> bool {
        true
    }

    /// The rows above and below the screen: the plane's, from the tables (the nametables hold the
    /// screen's rows only), while the screen's nearest row shows the round. A row at the screen's
    /// edge (its first or last) is decoded as the margins of `decode_row` are, the nametables
    /// kept where they show the round, while the next row towards the centre does.
    fn decode_extra_row(
        &self,
        memory: &DecoderMemory<'_>,
        row: &BackgroundRow,
        columns: &mut dyn Iterator<Item = i32>,
        out: &mut Vec<DecodedTile>,
    ) -> bool {
        let view = View::new(memory);
        // The screen's row nearest this one, whole on the screen (a row across its edge shows,
        // past it, a row the game does not show); for a row at the edge, the next one in.
        let edge = row.line > -8 && row.line < VISIBLE_LINES;
        let rows_away = if edge {
            if row.line < VISIBLE_LINES / 2 {
                -1
            } else {
                1
            }
        } else if row.line < 0 {
            -((-row.line + 7) / 8)
        } else {
            (row.line - (VISIBLE_LINES - 8) + 7).max(0) / 8
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
        let platforms = Platforms::new(memory, &view);
        for column in columns {
            let decoded = if edge {
                view.decoded(memory, &platforms, *row, column, first_column, plane_row)
            } else {
                view.tile(first_column + column, plane_row)
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

    /// The tile and sub-palette of screen column `column` of `row` (world tile column
    /// `first_column + column`): a column the game drew already shows what it drew, what it puts
    /// over the tables included; one it has not drawn yet, the tables and the platforms to come.
    fn decoded(
        &self,
        memory: &DecoderMemory<'_>,
        platforms: &Platforms,
        row: BackgroundRow,
        column: i32,
        first_column: i32,
        plane_row: i32,
    ) -> Option<(u8, u8)> {
        let world = first_column + column;
        let drawn = (world < self.drawing)
            .then(|| self.drawn(memory, row, column, world, plane_row))
            .flatten();
        match drawn.as_ref() {
            Some(drawn) => drawn.cell(memory, row, column),
            None => self
                .buffered(memory, world, plane_row)
                .or_else(|| platforms.tile(self, world, plane_row))
                .or_else(|| self.tile(world, plane_row)),
        }
        .or_else(|| self.tile(world, plane_row))
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

/// The column the game is drawing, as it puts it together (`$0460`, from the column's first tile
/// on): complete, platforms included, from its third step (`$06 & 7 = 3`, `$B397`) to the next
/// column, while the nametables may not have it yet.
const COLUMN_BUFFER: u16 = 0x0460;

impl View<'_> {
    /// The tile and sub-palette of world tile column `world` at row `plane_row` from the game's
    /// column buffer, when `world` is the column it has put together; a tile the game put over
    /// the tables takes the platforms' sub-palette.
    fn buffered(&self, memory: &DecoderMemory<'_>, world: i32, plane_row: i32) -> Option<(u8, u8)> {
        if world != self.drawing || memory.ram(0x0006) & 7 < 3 {
            return None;
        }
        let index = u16::try_from(plane_row - self.rows.at(world))
            .ok()
            .filter(|&index| i32::from(index) < COLUMN_TILES)?;
        let tile = memory.ram(COLUMN_BUFFER + index);
        let tables = self.tile(world, plane_row)?;
        if tables.0 == tile {
            return Some(tables);
        }
        let style = u32::from(memory.ram(0x008E));
        let palette = self.round.byte(PLATFORM_PALETTES + style)? & 3;
        Some((tile, palette))
    }

    /// What the nametables hold of world tile column `world` at screen column `column`: the frame
    /// each of its rows that show what the tables give was written in, when enough of them do (0
    /// for a row not written since a state was loaded: the state holds it as drawn). What the
    /// game drew over the tables with the column (or later) is as new as the rows around it; a
    /// row the column was not drawn on (the plane's rows past the screen when the game drew it)
    /// is older. The game also draws whole rows as it scrolls vertically: those are newer than
    /// the rest of the column, so a tile is compared with its nearest matching rows, not with the
    /// newest.
    fn drawn(
        &self,
        memory: &DecoderMemory<'_>,
        row: BackgroundRow,
        column: i32,
        world: i32,
        plane_row: i32,
    ) -> Option<DrawnColumn> {
        let coarse_y = i32::from(row.coarse_y());
        let mut frames = [None; 30];
        let mut matching = 0;
        for (tile_row, frame) in (0..RING_ROWS).zip(frames.iter_mut()) {
            let address = nametable_address(row_at(row, tile_row), column);
            // The rows of the screen are those of the plane around this one.
            let offset =
                (tile_row - coarse_y + (RING_ROWS / 2)).rem_euclid(RING_ROWS) - (RING_ROWS / 2);
            if let Some((tile, _)) = self.tile(world, plane_row + offset) {
                if (memory.nametable)(address) == tile {
                    matching += 1;
                    *frame = Some((memory.nametable_written)(address));
                }
            }
        }
        // Frame 0: not written since the state was loaded (or power-on), the state's own.
        (matching >= 4).then(|| DrawnColumn {
            frames,
            tables: self.tile(world, plane_row),
            ring: [
                self.tile(world, plane_row - RING_ROWS),
                self.tile(world, plane_row + RING_ROWS),
            ],
        })
    }
}

/// A column the nametables hold as the game drew it.
struct DrawnColumn {
    /// The frame each nametable row (0-29) that shows the tables was written in.
    frames: [Option<u32>; 30],
    /// What the tables give at the row asked for.
    tables: Option<(u8, u8)>,
    /// And at the rows of the plane the nametables hold at the same place (30 rows up and down).
    ring: [Option<(u8, u8)>; 2],
}

impl DrawnColumn {
    /// The tile and sub-palette at screen column `column` of `row`: the nametables', when they
    /// show the tables or something the game drew over them since the column; the tables', when
    /// they show the plane's row 30 rows up or down (the game draws the rows the screen scrolls
    /// to over the screen's columns only: past them a row stays as the column left it); `None`
    /// for a tile older than the rows around it that show the tables (the older of the nearest
    /// above and below: another column's, before the ring came round).
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
            Some(tables) if self.ring.iter().flatten().any(|&(tile, _)| tile == shown.0) => {
                Some(tables)
            }
            _ => ((memory.nametable_written)(address) >= self.around(row)).then_some(shown),
        }
    }

    /// The older of the frames of the nearest rows above and below `row` that show the tables.
    fn around(&self, row: BackgroundRow) -> u32 {
        let coarse_y = i32::from(row.coarse_y());
        let nearest = |step: i32| {
            (1..RING_ROWS).find_map(|distance| {
                let index = usize::try_from((coarse_y + (step * distance)).rem_euclid(RING_ROWS));
                index.ok().and_then(|index| self.frames[index])
            })
        };
        match (nearest(-1), nearest(1)) {
            (Some(above), Some(below)) => above.min(below),
            (Some(frame), None) | (None, Some(frame)) => frame,
            (None, None) => 0,
        }
    }
}

/// The platforms the game draws over the tables (`$A629`): kind `$3B` with its extra byte's low
/// three bits 0 (the cloud platforms of the sea rounds, rock ledges elsewhere). Created as an
/// object (`$A8CF`), such a platform asks for its drawing (`$052E` = 4, `$052F` its extra byte);
/// at the third step of each tile column the game draws (`$06 & 7 = 3`, `$BD3C`) the request
/// counts down, and from the column where it reaches 0 the game writes two tiles over each of 8
/// columns: `tbl_A80F[$8E]` + the column (top) and + 8 more (below), at the row of the plane
/// `$32 + $8D` (`$8D` from the ground's y under its first column, `$A607`), the sub-palette
/// `tbl_A81F[$8E]` over the 16 lines of attribute row `$34 + $0530`. The decoder shows in the margin
/// columns the game has not drawn yet the platform being drawn (`$88` columns left, its data at
/// `$89`), the one asked for, and those still in the round's object lists (read ahead as `$8C5A`
/// reads them), each at the column and row the game will give it.
struct Platforms {
    list: Vec<Platform>,
}

/// One platform: its first world tile column and the data index (0-7) of it, the columns left,
/// its tiles (CPU address of the top row's first), its row of the plane and its sub-palette's.
struct Platform {
    first: i32,
    index: i32,
    columns: i32,
    tiles: u16,
    row: i32,
    palette_row: i32,
    palette: u8,
}

/// The platforms' kind, the tiles and sub-palettes of each style (`$8E`), the third of the
/// ground's y they take (by `$2B`).
const PLATFORM_KIND: u8 = 0x3B;
const PLATFORM_TILES: u32 = 0xA80F;
const PLATFORM_PALETTES: u32 = 0xA81F;
const PLATFORM_LIFT: u32 = 0xA604;
/// Their columns.
const PLATFORM_COLUMNS: i32 = 8;
/// The object lists: read pointers (`$7C` + list), bits left in the byte read (`$FF`: over) and
/// that byte (`$0546`, `$0547`), the next entry's position (`$0540`); lists 0, 2 and 4.
const LIST_POINTER: u16 = 0x007C;
const LIST_BITS_LEFT: u16 = 0x0546;
const LIST_BITS: u16 = 0x0547;
const LIST_NEXT: u16 = 0x0540;
/// Entries read ahead per list at most.
const LIST_AHEAD: usize = 8;
/// How far past the column being drawn platforms are looked for.
const PLATFORM_REACH: i32 = 48;
/// The ground: the y, shape and part-3 section of the game's 5 parts of 128 pixels, the change of
/// y per section (`$BF43`).
const GROUND_Y: u16 = 0x0506;
const GROUND_SHAPE: u16 = 0x050B;
const GROUND_SECTION: u16 = 0x003B;
const GROUND_RISE: u32 = 0xBF43;

impl Platforms {
    fn new(memory: &DecoderMemory<'_>, view: &View<'_>) -> Self {
        let mut list = Vec::new();
        let ram = |address: u16| memory.ram(address);
        let style = ram(0x008E);
        let byte = |address: u32| view.round.byte(address);
        let (Some(tiles_low), Some(tiles_high), Some(palette)) = (
            byte(PLATFORM_TILES + (2 * u32::from(style))),
            byte(PLATFORM_TILES + (2 * u32::from(style)) + 1),
            byte(PLATFORM_PALETTES + u32::from(style)),
        ) else {
            return Self { list };
        };
        let scroll = (i32::from(ram(0x003A)) << 8) | i32::from(ram(0x0006));
        // The next column whose third step is to come.
        let next = view.drawing + i32::from(scroll & 7 >= 3);
        // The platform being drawn.
        let left = i32::from(ram(0x0088));
        if left > 0 {
            let row = i32::from(ram(WINDOW_ROW)) + i32::from(ram(0x008D));
            list.push(Platform {
                first: next,
                index: PLATFORM_COLUMNS - left,
                columns: left,
                tiles: u16::from_le_bytes([ram(0x0089), ram(0x008A)]),
                row,
                palette_row: 2 * (i32::from(ram(0x0034)) + i32::from(ram(0x0530))),
                palette: palette & 3,
            });
        }
        let tiles = u16::from_le_bytes([tiles_low, tiles_high]);
        let mut add = |first: i32, extra: u8| {
            if let Some((row, palette_row)) = Self::row(memory, view, scroll, first, extra) {
                list.push(Platform {
                    first,
                    index: 0,
                    columns: PLATFORM_COLUMNS,
                    tiles,
                    row,
                    palette_row,
                    palette: palette & 3,
                });
            }
        };
        // The one asked for: drawn from the column where the count reaches 0.
        let asked = i32::from(ram(0x052E));
        if asked > 0 {
            add(next + asked - 1, ram(0x052F));
        }
        // Those of the lists: created when the screen's left edge comes 64 pixels near their
        // position (its 16-pixel part), their request counted down from the column then drawn.
        for list_index in [0u16, 2, 4] {
            let mut reader = ListReader::new(memory, list_index);
            for _ in 0..LIST_AHEAD {
                let Some((position, kind, extra)) = reader.entry() else {
                    break;
                };
                let first = ((position & 0xFFF0) >> 3) - 4;
                if first > view.drawing + PLATFORM_REACH {
                    break;
                }
                if kind == PLATFORM_KIND && extra.trailing_zeros() >= 3 {
                    add(first, extra);
                }
                if !reader.next() {
                    break;
                }
            }
        }
        Self { list }
    }

    /// The row of the plane a platform starting at world tile column `first` gets (`$A654`), and
    /// the first of the two rows its sub-palette goes over, as if drawn with the screen's vertical
    /// position of now.
    fn row(
        memory: &DecoderMemory<'_>,
        view: &View<'_>,
        scroll: i32,
        first: i32,
        extra: u8,
    ) -> Option<(i32, i32)> {
        let ram = |address: u16| memory.ram(address);
        // `$A607`: the ground at screen x 265 when the column is drawn (the screen's left edge then
        // at 8 first - 5), the screen being where it is now.
        let x = (8 * first) + 260 - scroll;
        let ground = ground_y(memory, &view.round, x)?;
        let third = ram(0x002B);
        let lift = view.round.byte(PLATFORM_LIFT + u32::from(third))?;
        let mut y = ground.wrapping_sub(0x10).wrapping_add(lift);
        if y & 7 >= 4 {
            y = y.wrapping_add(3);
        }
        let mut rows = (y & 0xF8).wrapping_sub((extra << 1) & 0xF0) >> 3;
        if ram(0x008E) == 0 {
            rows = rows.wrapping_add(1);
        }
        let palette_rows = (rows.wrapping_add(u8::from(third == 2))) >> 1;
        Some((
            i32::from(ram(WINDOW_ROW)) + i32::from(rows),
            2 * (i32::from(ram(0x0034)) + i32::from(palette_rows)),
        ))
    }

    /// The tile and sub-palette a platform puts at world tile column `world`, row `plane_row`.
    fn tile(&self, view: &View<'_>, world: i32, plane_row: i32) -> Option<(u8, u8)> {
        self.list.iter().find_map(|platform| {
            let column = world - platform.first;
            let below = plane_row - platform.row;
            if !(0..platform.columns).contains(&column) || !(0..2).contains(&below) {
                return None;
            }
            let index = u32::try_from(platform.index + column + (8 * below)).ok()?;
            let tile = view.round.byte(u32::from(platform.tiles) + index)?;
            let palette = if (0..2).contains(&(plane_row - platform.palette_row)) {
                platform.palette
            } else {
                view.tile(world, plane_row)
                    .map_or(0, |(_, palette)| palette)
            };
            Some((tile, palette))
        })
    }
}

/// The ground's y at screen x `x` as `sub_C1EA` gives it: from the game's 5 parts of 128 pixels
/// (`((left edge | 128) + x) / 128`), and past them from the round's sections as `$BEDE` makes the
/// next part (the last one's y plus `$BF43` of the section's second byte, the shape from its
/// third); `None` past the round or 3 parts on.
fn ground_y(memory: &DecoderMemory<'_>, round: &Round<'_>, x: i32) -> Option<u8> {
    let ram = |address: u16| memory.ram(address);
    let sum = i32::from(ram(0x0006) | 0x80) + x;
    let part = sum >> 7;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0..128.
    let position = (sum & 0x7F) as u8;
    let (y, shape) = match part {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0..5.
        0..=4 => (ram(GROUND_Y + part as u16), ram(GROUND_SHAPE + part as u16)),
        5..=7 => {
            let mut y = ram(GROUND_Y + 4);
            let mut shape = 0;
            for part in 5..=part {
                let section = i32::from(ram(GROUND_SECTION)) - 3 + part;
                if section >= i32::from(ram(SECTIONS)) {
                    return None;
                }
                let rise = (round.section(section, 1)? >> 2) & 7;
                y = y.wrapping_add(round.byte(GROUND_RISE + u32::from(rise))?);
                shape = round.section(section, 2)? & 0x0F;
            }
            (y, shape)
        }
        _ => return None,
    };
    Some(match shape {
        2 => y.wrapping_sub(position >> 2),
        3 => y.wrapping_add(position >> 2),
        4..=6 if position >= 0x40 => y.wrapping_add(if shape == 4 { 0xE0 } else { 0x20 }),
        _ => y,
    })
}

/// One of the round's object lists read ahead from its state in RAM, as `sub_8D74` reads it.
struct ListReader<'a> {
    prg_rom: &'a [u8],
    pointer: u16,
    bits_left: u8,
    bits: u8,
    position: i32,
    over: bool,
}

impl<'a> ListReader<'a> {
    fn new(memory: &DecoderMemory<'a>, list: u16) -> Self {
        let ram = |address: u16| memory.ram(address);
        Self {
            prg_rom: memory.prg_rom,
            pointer: u16::from_le_bytes([ram(LIST_POINTER + list), ram(LIST_POINTER + list + 1)]),
            bits_left: ram(LIST_BITS_LEFT + list),
            bits: ram(LIST_BITS + list),
            position: i32::from(u16::from_le_bytes([
                ram(LIST_NEXT + list),
                ram(LIST_NEXT + list + 1),
            ])),
            over: ram(LIST_BITS_LEFT + list) & 0x80 != 0,
        }
    }

    /// `count` bits, the first the highest; `None` past the PRG-ROM.
    fn read(&mut self, count: u32) -> Option<i32> {
        let mut value = 0;
        for _ in 0..count {
            if self.bits_left == 0 {
                let offset = usize::from(self.pointer.checked_sub(0x8000)?);
                self.bits = *self.prg_rom.get(offset)?;
                self.pointer = self.pointer.wrapping_add(1);
                self.bits_left = 8;
            }
            value = (value << 1) | i32::from(self.bits >> 7);
            self.bits <<= 1;
            self.bits_left -= 1;
        }
        Some(value)
    }

    /// The next entry: its position, kind and first extra byte (kinds `$38` and up; a second for
    /// `$3E` and up).
    fn entry(&mut self) -> Option<(i32, u8, u8)> {
        if self.over {
            return None;
        }
        let kind = u8::try_from(self.read(6)?).ok()?;
        let mut extra = 0;
        if kind >= 0x38 {
            extra = u8::try_from(self.read(8)?).ok()?;
            if kind >= 0x3E {
                self.read(8)?;
            }
        }
        Some((self.position, kind, extra))
    }

    /// On to the position of the entry after (1 bit, then 10 bits, 0 ending the list, or 6 bits
    /// added; x 8); false when the list is over.
    fn next(&mut self) -> bool {
        let step = match self.read(1) {
            Some(1) => match self.read(10) {
                Some(0) | None => None,
                Some(at) => {
                    self.position = at * 8;
                    Some(())
                }
            },
            Some(_) => self.read(6).map(|add| self.position += add * 8),
            None => None,
        };
        self.over = step.is_none();
        !self.over
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
    fn an_object_list_is_read_ahead_as_the_game_reads_it() {
        let (mut prg, mut ram) = round_image();
        // List 2 at $9800, its next entry at position $0120: kind $3B with extra byte $08, then 6
        // bits added (2: 16 pixels on), kind $01, then 10 bits of 0: the list is over.
        let bits = "111011 00001000 0 000010 000001 1 0000000000";
        let bits: Vec<u8> = bits
            .bytes()
            .filter(|&b| b != b' ')
            .map(|b| b - b'0')
            .collect();
        for (index, chunk) in bits.chunks(8).enumerate() {
            let byte = chunk
                .iter()
                .enumerate()
                .fold(0u8, |byte, (bit, &value)| byte | (value << (7 - bit)));
            prg[0x1800 + index] = byte;
        }
        ram[usize::from(LIST_POINTER) + 2..usize::from(LIST_POINTER) + 4]
            .copy_from_slice(&0x9800u16.to_le_bytes());
        ram[usize::from(LIST_BITS_LEFT) + 2] = 0;
        ram[usize::from(LIST_NEXT) + 2..usize::from(LIST_NEXT) + 4]
            .copy_from_slice(&0x0120u16.to_le_bytes());
        let memory = memory(&prg, &ram);
        let mut reader = ListReader::new(&memory, 2);
        assert_eq!(reader.entry(), Some((0x120, 0x3B, 0x08)));
        assert!(reader.next());
        assert_eq!(reader.entry(), Some((0x130, 0x01, 0)));
        assert!(!reader.next(), "a 10-bit position of 0 ends the list");
        assert_eq!(reader.entry(), None);
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
