//! Super Mario Bros. (World) and (Europe): the margins of an area, from the game's own area
//! parser.
//!
//! The game keeps no level map: its area parser (`AreaParserCore`, `$93FC`) reads the area's
//! objects as the screen scrolls and writes one column of 13 metatiles at a time
//! (`MetatileBuffer`, `$06A1`; 16 x 16 pixels each, from tile row 4 down), only a little past the
//! right edge. So the decoder runs that parser itself, in a sandbox over a copy of the RAM
//! ([`super::guest`]), from the start of the area: `InitializeArea` (`$8FE4`, with no halfway or
//! entrance page, so from page 0), then `AreaParserCore` and `IncrementColumnPos` (`$92DB`) for
//! each of 512 columns (32 pages), as the game's own task list does. The console is not touched.
//! The result is kept until the area changes; an area is known by its data pointer (`$E7/$E8`)
//! and type (`$074E`), not by `AreaPointer`, which may name another area (the next one while
//! Mario goes down a pipe, anything on the title screen): after `InitializeArea` the sandbox takes
//! the game's pointer and type, and reads the area header again from there (`$9C58`, within
//! `GetAreaDataAddrs`).
//!
//! A metatile is `%ppnnnnnn`: sub-palette `p`, number `n` in the table of that palette
//! (`MetatileGraphics_Low/High`, `$8B08`/`$8B0C`), 4 tiles: top left, bottom left, top right,
//! bottom right. The palette comes from the metatile, not from the attribute table, which the game
//! writes later than the tiles.
//!
//! What the game changed since (a brick broken, a coin taken, a block used) is in the nametables:
//! a column the nametables hold for that very place is shown as they hold it. Rows 0-3 are the
//! status bar, its margins left empty. In a room the area's data locks the screen on (a scroll
//! lock object, as the parse records it per column: an underground bonus room, whose area holds
//! other rooms beside it) the margins stay empty. A row whose centre does not show what the parser gives
//! (a transition) is left to the nametables, as is any row while the game is not running an area
//! (`OperMode` 0 or 1 with `OperMode_Task` 3: the black "WORLD 1-1" screen shares its blank tile
//! with the sky, which the check would not tell).

use std::sync::Mutex;

use super::guest::Guest;
use super::{resolve_ring, BackgroundRow, DecodedTile, DecoderMemory, MarginDecoder};

/// The decoder.
pub struct SuperMarioBros;

/// CRC-32 of the ROM's data, iNES header excluded.
pub const SOURCE_CRC: u32 = 0xD445_F698;
/// The same of Super Mario Bros. (Europe): its area parser and metatile tables at the World's
/// places (the code it moved is later), the same RAM.
pub const SOURCE_CRC_EUROPE: u32 = 0x9A2D_B086;

const INITIALIZE_AREA: u16 = 0x8FE4;
const AREA_PARSER_CORE: u16 = 0x93FC;
const INCREMENT_COLUMN_POS: u16 = 0x92DB;
/// Within `GetAreaDataAddrs`: reads the area header at `AreaData` and moves past it.
const READ_AREA_HEADER: u16 = 0x9C58;
/// `MetatileGraphics_Low`, then `MetatileGraphics_High` 4 bytes on, in PRG-ROM.
const METATILE_GRAPHICS: usize = 0x0B08;

const AREA_DATA_LOW: u16 = 0x00E7;
const AREA_DATA_HIGH: u16 = 0x00E8;
const AREA_TYPE: u16 = 0x074E;
const HALFWAY_PAGE: u16 = 0x075B;
const ALT_ENTRANCE_CONTROL: u16 = 0x0752;
const CURRENT_PAGE: u16 = 0x0725;
const CURRENT_COLUMN: u16 = 0x0726;
const METATILE_BUFFER: u16 = 0x06A1;
const SCREEN_LEFT_PAGE: u16 = 0x071A;
const SCREEN_LEFT_X: u16 = 0x071C;
const SCROLL_LOCK: u16 = 0x0723;
const OPER_MODE: u16 = 0x0770;
const OPER_MODE_TASK: u16 = 0x0772;
/// The task of the title screen and of the game that runs the area (`GameMenuRoutine`,
/// `GameCoreRoutine`); the others draw other screens (the black "WORLD 1-1" one shares its blank
/// tile with the sky).
const PLAYING: u8 = 3;

/// Metatile columns parsed: 32 pages.
const COLUMNS: usize = 512;
/// Metatile rows of a column.
const ROWS: usize = 13;
/// The first tile row of the playfield; rows 0-3 are the status bar.
const FIRST_ROW: i32 = 4;
/// Screen columns whose tiles are checked against the nametables.
const CHECKED: [i32; 4] = [3, 11, 19, 27];

/// An area as the parser lays it out.
struct Area {
    /// `$E7`, `$E8`, `$074E` when it was parsed.
    key: [u8; 3],
    /// Each metatile column's 13 metatiles, from page 0; empty when the area could not be parsed.
    columns: Vec<[u8; ROWS]>,
    /// For each column, whether the area's own data had locked the scrolling there (a scroll lock
    /// object: the underground bonus rooms, a warp zone, rooms one screen wide that share an area).
    locked: Vec<bool>,
}

static AREA: Mutex<Option<Area>> = Mutex::new(None);

impl MarginDecoder for SuperMarioBros {
    fn decode_row(
        &self,
        memory: &DecoderMemory<'_>,
        row: &BackgroundRow,
        columns: &mut dyn Iterator<Item = i32>,
        out: &mut Vec<DecodedTile>,
    ) -> bool {
        let coarse_y = i32::from(row.coarse_y());
        if coarse_y >= 30 || memory.ram(OPER_MODE) > 1 || memory.ram(OPER_MODE_TASK) != PLAYING {
            return false;
        }
        if coarse_y < FIRST_ROW {
            // The status bar: nothing beside it.
            return true;
        }
        let key = [
            memory.ram(AREA_DATA_LOW),
            memory.ram(AREA_DATA_HIGH),
            memory.ram(AREA_TYPE),
        ];
        let Ok(mut cached) = AREA.lock() else {
            return false;
        };
        if cached.as_ref().is_none_or(|area| area.key != key) {
            *cached = Some(parse(memory, key));
        }
        let Some(area) = cached.as_ref().filter(|area| !area.columns.is_empty()) else {
            return false;
        };
        let camera =
            (i32::from(memory.ram(SCREEN_LEFT_PAGE)) << 8) | i32::from(memory.ram(SCREEN_LEFT_X));
        let first_column = resolve_ring(row.ring_x(), 512, camera) >> 3;
        // A room the area's data locks the screen on (a bonus room) shows nothing past the screen:
        // the next room of the area is its neighbour there. A lock that is not the area's (the
        // flagpole, a fall) keeps the margins.
        let room = (memory.ram(SCROLL_LOCK) != 0
            && usize::try_from((camera + 128) >> 4)
                .ok()
                .and_then(|column| area.locked.get(column))
                .copied()
                .unwrap_or(false))
        .then_some((camera >> 3)..((camera >> 3) + 32));
        let tiles = Tiles {
            area,
            prg_rom: memory.prg_rom,
        };

        // What the PPU shows in the centre must be what the parser gives (one column may differ:
        // a brick broken, a coin taken).
        let matching = CHECKED
            .iter()
            .filter(|&&column| {
                tiles
                    .at(first_column + column, coarse_y)
                    .is_some_and(|(tile, _)| {
                        (memory.nametable)(nametable_address(*row, column)) == tile
                    })
            })
            .count();
        if matching + 1 < CHECKED.len() {
            return false;
        }

        for column in columns {
            let world = first_column + column;
            if room.as_ref().is_some_and(|room| !room.contains(&world)) {
                continue;
            }
            let Some((tile, palette)) = tiles.at(world, coarse_y) else {
                continue;
            };
            let tile = if tiles.column_is_current(memory, *row, column, world) {
                (memory.nametable)(nametable_address(*row, column))
            } else {
                tile
            };
            out.push(DecodedTile {
                column,
                tile,
                palette,
            });
        }
        true
    }
}

/// Runs the game's area parser over the area in play, from page 0, in a sandbox.
fn parse(memory: &DecoderMemory<'_>, key: [u8; 3]) -> Area {
    let mut area = Area {
        key,
        columns: Vec::new(),
        locked: Vec::new(),
    };
    let mut guest = Guest::new(memory.ram, memory.prg_rom);
    guest.poke_ram(HALFWAY_PAGE, 0);
    guest.poke_ram(ALT_ENTRANCE_CONTROL, 0);
    if !guest.call(INITIALIZE_AREA, 200_000) {
        return area;
    }
    // `InitializeArea` found the area through `AreaPointer`, which may name another one (Mario
    // going down a pipe, the title screen): the area is the one whose data the game reads, its
    // header read again from there.
    let [low, high] = (u16::from_le_bytes([key[0], key[1]]).wrapping_sub(2)).to_le_bytes();
    guest.poke_ram(AREA_DATA_LOW, low);
    guest.poke_ram(AREA_DATA_HIGH, high);
    guest.poke_ram(AREA_TYPE, key[2]);
    if !guest.call(READ_AREA_HEADER, 1_000) {
        return area;
    }
    let mut columns = Vec::with_capacity(COLUMNS);
    for index in 0..COLUMNS {
        let place = (usize::from(guest.peek_ram(CURRENT_PAGE)) * 16)
            + usize::from(guest.peek_ram(CURRENT_COLUMN) & 0x0F);
        if place != index || !guest.call(AREA_PARSER_CORE, 100_000) {
            break;
        }
        let mut column = [0; ROWS];
        for (offset, metatile) in (0u16..).zip(column.iter_mut()) {
            *metatile = guest.peek_ram(METATILE_BUFFER + offset);
        }
        columns.push(column);
        area.locked.push(guest.peek_ram(SCROLL_LOCK) != 0);
        if !guest.call(INCREMENT_COLUMN_POS, 1_000) {
            break;
        }
    }
    area.columns = columns;
    area
}

/// The tiles of a parsed area.
struct Tiles<'a> {
    area: &'a Area,
    prg_rom: &'a [u8],
}

impl Tiles<'_> {
    /// CHR tile and sub-palette at world tile column `world`, tile row `tile_row`; `None` off
    /// the area or the playfield.
    fn at(&self, world: i32, tile_row: i32) -> Option<(u8, u8)> {
        if !(FIRST_ROW..30).contains(&tile_row) {
            return None;
        }
        let column = self.area.columns.get(usize::try_from(world >> 1).ok()?)?;
        #[allow(clippy::cast_sign_loss)] // Checked above.
        let row = (tile_row - FIRST_ROW) as usize;
        let metatile = *column.get(row >> 1)?;
        let palette = metatile >> 6;
        let low = *self.prg_rom.get(METATILE_GRAPHICS + usize::from(palette))?;
        let high = *self
            .prg_rom
            .get(METATILE_GRAPHICS + 4 + usize::from(palette))?;
        let table = usize::from(u16::from_le_bytes([low, high])).checked_sub(0x8000)?;
        let corner = (usize::from(world & 1 != 0) * 2) + (row & 1);
        let tile = *self
            .prg_rom
            .get(table + (usize::from(metatile & 0x3F) * 4) + corner)?;
        Some((tile, palette))
    }

    /// Whether screen column `column` of the nametables holds world tile column `world` (as Mega
    /// Man 2's decoder tells it): its playfield rows that differ from the parser's were all
    /// written after every row that matches it, and enough rows match.
    fn column_is_current(
        &self,
        memory: &DecoderMemory<'_>,
        row: BackgroundRow,
        column: i32,
        world: i32,
    ) -> bool {
        let mut newest_match = 0u32;
        let mut oldest_difference = u32::MAX;
        let mut matching = 0;
        for tile_row in FIRST_ROW..30 {
            #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)] // 4..30.
            let v = (row.v & !0x03E0) | ((tile_row as u16) << 5);
            let address = nametable_address(BackgroundRow { v, ..row }, column);
            let written = (memory.nametable_written)(address);
            let same = self
                .at(world, tile_row)
                .is_some_and(|(tile, _)| (memory.nametable)(address) == tile);
            if same {
                matching += 1;
                newest_match = newest_match.max(written);
            } else {
                oldest_difference = oldest_difference.min(written);
            }
        }
        newest_match > 0 && matching >= 4 && oldest_difference > newest_match
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
