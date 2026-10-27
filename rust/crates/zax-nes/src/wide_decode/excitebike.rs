//! Excitebike (Japan, USA): the margins of a race, from the track as the game's own routine lays
//! it out.
//!
//! The screen is the stands, the sky with its signs and the grass above the track (tile rows
//! 0-7), the track (rows 8-21), the grass below it (rows 22-23) and the status bar (24-29). Only
//! the track changes as the screen scrolls: the game draws it one tile column at a time
//! (`$F4FF`, once a frame from the race's loop), 14 tiles from row 8, at the column `$EA:$E9` of
//! the ring of two nametables side by side (64 columns), about 19 columns past the screen's right
//! edge. The other rows are drawn once and repeat as the ring does.
//!
//! The track is a stream of bytes (`$ED3A/$ED40` indexed by the track, `$43`): pieces (a ramp, a
//! stretch of mud, plain track...) and how many times each repeats, each piece a list of columns.
//! `$F4FF` keeps its place in it in `$ED` (the byte), `$EC` (the repetitions left), `$EF` (the
//! piece's column) and `$EE` (the piece), and goes back to the stream's start at the finish line,
//! a lap later. The decoder runs it in a sandbox ([`super::guest`]) from the state a race starts
//! with (`$C869` clears `$4C-$EF`), one lap of columns, kept until the track changes. The column
//! the game draws next is the lap's column whose state is the game's now: the columns past it are
//! the lap's next ones, those the ring no longer holds (64 columns behind) its previous ones.
//! Within the ring the nametables are shown as they are.
//!
//! The rows outside the track show the ring of nametables around them; beside the status bar its
//! background goes on (the tile at the screen's edge). A race is told by the centre of the track: what the nametables show there must be the
//! lap's columns, in a mode that runs a race (`$41`), otherwise (the title, the menus, the design
//! mode's editor) the rows are left to the nametables.

use std::sync::Mutex;

use super::guest::Guest;
use super::{resolve_ring, BackgroundRow, DecodedTile, DecoderMemory, MarginDecoder};

/// The decoder.
pub struct Excitebike;

/// CRC-32 of the ROM's data, iNES header excluded.
pub const SOURCE_CRC: u32 = 0x49AE_B3A6;

/// The routine that draws the track's next column, when `$EB` reaches `$E0`.
const DRAW_COLUMN: u16 = 0xF4FF;
/// The column it writes: tiles of rows 8-21 in the VRAM buffer.
const COLUMN_TILES: u16 = 0x0304;
/// The track's tile rows.
const FIRST_TRACK_ROW: i32 = 8;
const TRACK_ROWS: usize = 14;
/// The status bar's first row.
const STATUS_BAR: i32 = 24;
/// The ring of two nametables side by side, in tile columns.
const RING: i32 = 64;
/// How far past the screen's right edge the game draws (about).
const DRAW_LEAD: i32 = 19;

const TRACK: u16 = 0x0043;
const TRACK_POINTERS_LOW: u32 = 0xED3A;
const TRACK_POINTERS_HIGH: u32 = 0xED40;
/// The stream's state: byte, repetitions left, piece's column, piece.
const STATE: [u16; 4] = [0x00ED, 0x00EC, 0x00EF, 0x00EE];
/// The column to draw (`$E8` the game's RAM copy of the track, `$E9`/`$EA` the nametable
/// address), and the column that makes the routine draw (`$EB`, against the player's `$E0`).
const RAM_COLUMN: u16 = 0x00E8;
const NEXT_COLUMN: u16 = 0x00E9;
const NEXT_TABLE: u16 = 0x00EA;
const DRAW_AT: u16 = 0x00EB;
const PLAYER_COLUMN: u16 = 0x00E0;
/// The game's main mode (`$41`, the routines of the table at `$C000`): those that run a race
/// (`$C99A`; `$C97C` for a track tried in the design mode) draw the track; the title, the menus
/// and the editor do not.
const MODE: u16 = 0x0041;
const RACING_MODES: [u8; 4] = [2, 5, 9, 10];
/// The track's scroll: x and nametable.
const SCROLL_X: u16 = 0x0012;
const SCROLL_TABLE: u16 = 0x004D;
/// What the routine counts over a race (finish lines passed, the letters of the finish line's
/// banner, the last lap, the lane lines' alternation), as a race starts.
const LAP_COUNTERS: [u16; 5] = [0x03A4, 0x03A5, 0x03BC, 0x03A8, 0x03A2];
/// The longest stream read for the key; the most columns laid out.
const STREAM_MAX: usize = 512;
const COLUMNS_MAX: usize = 4096;
/// Screen columns of the track's middle row checked against the lap.
const CHECKED: [i32; 4] = [3, 11, 19, 27];
const CHECKED_ROW: i32 = 15;

/// One lap of the track as the game draws it.
struct Lap {
    /// The track's stream (the key).
    stream: Vec<u8>,
    /// Each column's 14 tiles, and the stream's state before it was drawn.
    columns: Vec<[u8; TRACK_ROWS]>,
    states: Vec<[u8; 4]>,
    /// The lap's length in columns: from the second column on, the stream's states come back
    /// every `length` columns (the first lap alone starts by reading the lap count); 0 when they
    /// did not within the columns laid out.
    length: usize,
}

static LAP: Mutex<Option<Lap>> = Mutex::new(None);

impl MarginDecoder for Excitebike {
    fn decode_row(
        &self,
        memory: &DecoderMemory<'_>,
        row: &BackgroundRow,
        columns: &mut dyn Iterator<Item = i32>,
        out: &mut Vec<DecodedTile>,
    ) -> bool {
        let coarse_y = i32::from(row.coarse_y());
        if coarse_y >= 30 || !RACING_MODES.contains(&memory.ram(MODE)) {
            return false;
        }
        let stream = track_stream(memory);
        let Ok(mut cached) = LAP.lock() else {
            return false;
        };
        if cached.as_ref().is_none_or(|lap| lap.stream != stream) {
            *cached = Some(lay_out(memory, stream));
        }
        let Some(lap) = cached.as_ref().filter(|lap| !lap.columns.is_empty()) else {
            return false;
        };
        let state = STATE.map(|address| memory.ram(address));
        let Some(next) = lap.states.iter().position(|known| *known == state) else {
            return false;
        };
        let place = Place::new(memory, lap, next);
        if !place.racing(memory) {
            return false;
        }
        // The row's left edge on the track, near the race's scroll.
        let first = resolve_ring(
            (i32::from(row.nametable_x()) * 32) + i32::from(row.coarse_x()),
            RING,
            place.left,
        );
        let track = (FIRST_TRACK_ROW..FIRST_TRACK_ROW + 14).contains(&coarse_y);
        if coarse_y >= STATUS_BAR {
            // The status bar: its background (the screen's edge columns) goes on.
            for column in columns {
                let (tile, palette) = nametable_tile(memory, *row, if column < 0 { 0 } else { 31 });
                out.push(DecodedTile {
                    column,
                    tile,
                    palette,
                });
            }
            return true;
        }
        for column in columns {
            let shown = nametable_tile(memory, *row, column);
            let tile = if track {
                place.tile(first + column, coarse_y).unwrap_or(shown.0)
            } else {
                shown.0
            };
            out.push(DecodedTile {
                column,
                tile,
                palette: shown.1,
            });
        }
        true
    }
}

/// Where the race is on the lap.
struct Place<'a> {
    lap: &'a Lap,
    /// The lap's column the game draws next, and its place on the ring.
    next: usize,
    next_ring: i32,
    /// The ring's column at the track's left edge, as the race scrolls.
    left: i32,
}

impl<'a> Place<'a> {
    fn new(memory: &DecoderMemory<'_>, lap: &'a Lap, next: usize) -> Self {
        let next_ring = i32::from(memory.ram(NEXT_COLUMN) & 31)
            + (32 * i32::from((memory.ram(NEXT_TABLE) >> 2) & 1));
        let left =
            (i32::from(memory.ram(SCROLL_TABLE) & 1) * 32) + i32::from(memory.ram(SCROLL_X) >> 3);
        Self {
            lap,
            next,
            next_ring,
            left,
        }
    }

    /// The lap's tile at track column `screen` (from the ring's column 0 the left edge is in),
    /// row `coarse_y` (8-21), when the nametables no longer or not yet hold it: its distance
    /// from the next column to draw (some 19 past the right edge, the ring's 64 columns ending
    /// there).
    fn tile(&self, screen: i32, coarse_y: i32) -> Option<u8> {
        let next = resolve_ring(self.next_ring, RING, self.left + 32 + DRAW_LEAD);
        let distance = screen - next;
        if (-RING..0).contains(&distance) {
            return None;
        }
        self.at(distance, coarse_y)
    }

    /// The lap's tile `distance` columns from the next one to draw: past the columns laid out,
    /// or before them, the same place of the next or previous lap (from the second column on).
    fn at(&self, distance: i32, coarse_y: i32) -> Option<u8> {
        let laid_out = i32::try_from(self.lap.columns.len()).ok()?;
        let length = i32::try_from(self.lap.length).ok()?;
        let mut index = i32::try_from(self.next).ok()? + distance;
        if (index < 1 || index >= laid_out) && length > 0 {
            index = 1 + (index - 1).rem_euclid(length);
        }
        let row = usize::try_from(coarse_y - FIRST_TRACK_ROW).ok()?;
        self.lap
            .columns
            .get(usize::try_from(index).ok()?)
            .and_then(|column| column.get(row))
            .copied()
    }

    /// Whether the track's centre shows the lap: a race in play (one column may differ, the
    /// finish line's lettering).
    fn racing(&self, memory: &DecoderMemory<'_>) -> bool {
        let next = resolve_ring(self.next_ring, RING, self.left + 32 + DRAW_LEAD);
        let matching = CHECKED
            .iter()
            .filter(|&&column| {
                let screen = self.left + column;
                let ring = screen.rem_euclid(RING);
                let address = 0x2000
                    | (u16::try_from(ring >> 5).unwrap_or(0) << 10)
                    | (u16::try_from(CHECKED_ROW).unwrap_or(0) << 5)
                    | u16::try_from(ring & 31).unwrap_or(0);
                self.at(screen - next, CHECKED_ROW)
                    .is_some_and(|tile| (memory.nametable)(address) == tile)
            })
            .count();
        matching + 1 >= CHECKED.len()
    }
}

/// The track's stream: from its pointer, up to its 0 (or `STREAM_MAX` bytes).
fn track_stream(memory: &DecoderMemory<'_>) -> Vec<u8> {
    let prg = |address: u32| {
        let len = memory.prg_rom.len();
        (len > 0).then(|| memory.prg_rom[usize::try_from(address - 0x8000).unwrap_or(0) % len])
    };
    let track = u32::from(memory.ram(TRACK));
    let (Some(low), Some(high)) = (
        prg(TRACK_POINTERS_LOW + track),
        prg(TRACK_POINTERS_HIGH + track),
    ) else {
        return Vec::new();
    };
    let pointer = u16::from_le_bytes([low, high]);
    let byte = |offset: usize| {
        let address = pointer.wrapping_add(u16::try_from(offset).unwrap_or(0));
        if address < 0x0800 {
            Some(memory.ram(address))
        } else if address >= 0x8000 {
            prg(u32::from(address))
        } else {
            None
        }
    };
    let mut stream = vec![low, high];
    for offset in 0..STREAM_MAX {
        let Some(value) = byte(offset) else { break };
        stream.push(value);
        if offset > 0 && value == 0 {
            break;
        }
    }
    stream
}

/// The track as `$F4FF` draws it from the start of a race: its first column, then one lap.
fn lay_out(memory: &DecoderMemory<'_>, stream: Vec<u8>) -> Lap {
    let mut lap = Lap {
        stream,
        columns: Vec::new(),
        states: Vec::new(),
        length: 0,
    };
    let mut guest = Guest::new(memory.ram, memory.prg_rom);
    for address in STATE {
        guest.poke_ram(address, 0);
    }
    // As `$C859` leaves them; the routine only writes the RAM copy and the VRAM buffer with them.
    guest.poke_ram(RAM_COLUMN, 0x3F);
    guest.poke_ram(NEXT_COLUMN, 0x1F);
    guest.poke_ram(NEXT_TABLE, 0x25);
    guest.poke_ram(DRAW_AT, 0x0A);
    for address in LAP_COUNTERS {
        guest.poke_ram(address, 0);
    }
    for _ in 0..COLUMNS_MAX {
        let state = STATE.map(|address| guest.peek_ram(address));
        let draw_at = guest.peek_ram(DRAW_AT);
        guest.poke_ram(PLAYER_COLUMN, draw_at);
        if !guest.call(DRAW_COLUMN, 50_000) || guest.peek_ram(DRAW_AT) == draw_at {
            break;
        }
        let mut column = [0; TRACK_ROWS];
        for (offset, tile) in (0u16..).zip(column.iter_mut()) {
            *tile = guest.peek_ram(COLUMN_TILES + offset);
        }
        if lap.states.len() > 1 && lap.states.get(1) == Some(&state) {
            lap.length = lap.states.len() - 1;
            break;
        }
        lap.states.push(state);
        lap.columns.push(column);
    }
    lap
}

/// The tile and sub-palette the nametables hold at screen column `column` of `row`.
fn nametable_tile(memory: &DecoderMemory<'_>, row: BackgroundRow, column: i32) -> (u8, u8) {
    let plane = (i32::from(row.nametable_x()) * 32) + i32::from(row.coarse_x()) + column;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0..64.
    let plane = plane.rem_euclid(RING) as u16;
    let table = 0x2000 | ((plane >> 5) << 10);
    let tile_row = u16::from(row.coarse_y());
    let tile_column = plane & 31;
    let tile = (memory.nametable)(table | (tile_row << 5) | tile_column);
    let attribute =
        (memory.nametable)(table | 0x03C0 | ((tile_row >> 2) << 3) | (tile_column >> 2));
    let shift = ((tile_row & 2) << 1) | (tile_column & 2);
    (tile, (attribute >> shift) & 3)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A lap of 100 columns whose tile on every row is its index.
    fn lap() -> Lap {
        let columns: Vec<[u8; TRACK_ROWS]> = (0..100u8).map(|index| [index; TRACK_ROWS]).collect();
        // Column 0 the race's start, columns 1-99 a lap of 99.
        Lap {
            stream: Vec::new(),
            states: vec![[0; 4]; columns.len()],
            length: columns.len() - 1,
            columns,
        }
    }

    #[test]
    fn the_columns_past_the_ring_are_the_lap_s_around_the_next_one() {
        let lap = lap();
        // The screen's left edge at ring column 10; the game draws lap column 50 next, at ring
        // column 61 (19 past the right edge at 42).
        let place = Place {
            lap: &lap,
            next: 50,
            next_ring: 61,
            left: 10,
        };
        assert_eq!(place.tile(61, 12), Some(50), "the next column to draw");
        assert_eq!(place.tile(63, 12), Some(52), "past it");
        assert_eq!(place.tile(60, 12), None, "drawn: the nametables'");
        assert_eq!(
            place.tile(-3, 12),
            None,
            "64 columns back: still the ring's"
        );
        assert_eq!(
            place.tile(-4, 12),
            Some(84),
            "past the ring, the lap's earlier ones"
        );
        assert_eq!(place.tile(61, 7), None, "not a row of the track");
    }

    #[test]
    fn a_lap_goes_round() {
        let lap = lap();
        let place = Place {
            lap: &lap,
            next: 98,
            next_ring: 61,
            left: 10,
        };
        assert_eq!(
            place.tile(64, 12),
            Some(2),
            "past the lap's end: its start (column 1)"
        );
    }
}
