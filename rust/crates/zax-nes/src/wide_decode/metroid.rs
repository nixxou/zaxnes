//! Metroid (USA): the margins of the world, decoded from its map and its rooms.
//!
//! The world is a map of 32 x 32 rooms, one screen (256 x 240) each, its room numbers at `$7000`
//! in PRG-RAM (`$FF`: no room); a room number is an entry of the area's room table. The game
//! builds the rooms it shows into PRG-RAM, `$6000` for nametable 0 and `$6400` for the other,
//! tiles then attribute bytes as in a nametable (`SetupRoom`, `$EA2B`), and copies them to the
//! PPU a column or a row at a time as it scrolls. A room's data (`RoomPtrTable`, `$3B`) is its
//! default sub-palette, then objects: a position `%yyyyxxxx` in metatiles, a structure (from
//! `StructPtrTable`, `$3D`) and its sub-palette, until `$FD` or `$FF`. A structure is rows of
//! metatiles: a byte `%xxxxnnnn` (offset, count, 0 for 16) then the metatiles, until `$FF`; a
//! metatile is 4 CHR tiles at `MetatileDefs` (`$3F`) + 4 n (`DrawStruct`, `$EF8C`). The tables
//! are in the area's PRG bank (`InArea`, `$74`, through `$CA30`).
//!
//! The game scrolls one way at a time (`ScrollDir`, `$49`: 0 up, 1 down, 2 left, 3 right): across,
//! two rooms side by side on the two nametables (vertical mirroring), or up and down, two rooms
//! stacked (horizontal mirroring). `MapPosX`/`MapPosY` (`$50`/`$4F`) is the room the screen
//! scrolls into: the right (bottom) one of the two on the screen scrolling right (down), the left
//! (top) one scrolling left (up). `PPUCTRL` (`$FF`, both nametable bits toggled together) gives the
//! nametable of the screen's left (top) edge, `$FD`/`$FC` the scroll.
//!
//! The map's rooms belong to five areas, each with its own tables, and the map does not say which
//! a room is in: the margins show the rooms of the area in play only, those its starting room
//! (`$95D7`/`$95D8` of its bank) or the screen's room reach through the edges two neighbours
//! share a way through (a tile at or past `$A0`, or a block the area's bombs blast, `$D651`, on
//! both sides), up and down only between two rooms walled on the left and the right (the vertical
//! shafts: a corridor's ceiling may happen to meet a room of another area built with this one's
//! tables), never across an elevator room's top or bottom (the elevators lead to the other
//! areas).
//!
//! The margins show the map's rooms around the screen, those behind a door too. The room on
//! the screen's left (top) edge, and the other one while both show, come from PRG-RAM (what the
//! game changed in them included); the others are built from the area's data as the game would.
//!
//! The decoder checks itself against the PPU: a row whose centre does not show what the map gives
//! is left to the nametables (the title, a transition between areas, the password screen).

use std::collections::HashMap;
use std::sync::Mutex;

use super::{resolve_ring, BackgroundRow, DecodedTile, DecoderMemory, MarginDecoder};

/// The decoder.
pub struct Metroid;

/// CRC-32 of the ROM's data, iNES header excluded.
pub const SOURCE_CRC: u32 = 0x7008_0810;

const GAME_MODE: u16 = 0x001D;
const AREA_POINTERS: u16 = 0x003B;
const SCROLL_DIR: u16 = 0x0049;
const MAP_Y: u16 = 0x004F;
const MAP_X: u16 = 0x0050;
const IN_AREA: u16 = 0x0074;
const SCROLL_Y: u16 = 0x00FC;
const SCROLL_X: u16 = 0x00FD;
const PPUCTRL: u16 = 0x00FF;

/// `$7000`, in PRG-RAM.
const WORLD_MAP: usize = 0x1000;
/// The size of a room in PRG-RAM (`$6000`, `$6400`): its tiles, then its attribute bytes.
const ROOM_SIZE: usize = 0x400;
const ATTRIBUTES: usize = 0x3C0;
/// Past this offset in a room, a structure stops (`$EF2B`).
const LAST_METATILE: usize = 0x3A0;
/// The PRG bank of each area (`InArea` low nibble, `$CA30`).
const AREA_BANKS: [usize; 5] = [1, 2, 4, 3, 5];

const BANK_SIZE: usize = 0x4000;
const ROOM_WIDTH: i32 = 256;
const ROOM_HEIGHT: i32 = 240;
const MAP_SIZE: i32 = 32;
/// The last capture line of a row whose 8 lines end before the screen's last.
const LAST_INNER_LINE: i32 = 232;
/// Of the screen's 32 columns, how many must show the room's tile for a row to be the map's (a
/// few may show something the game changed: a door, a blasted block).
const MATCHING_COLUMNS: usize = 28;

/// Rooms built from the area's data: by PRG bank and address of the room's data.
static ROOMS: Mutex<Option<BuiltRooms>> = Mutex::new(None);
type BuiltRooms = HashMap<(usize, usize), Box<[u8; ROOM_SIZE]>>;
/// Rooms kept at most: a few areas' worth.
const MAX_ROOMS: usize = 256;

/// The rooms of the area in play (bit `x` of word `y`), by area bank and the room they were
/// reached from.
static AREA: Mutex<Option<(AreaKey, AreaRooms)>> = Mutex::new(None);
/// Area bank, the screen's room, and the side it was entered from when it is a palette change
/// room.
type AreaKey = (usize, i32, i32, i32);

/// The rooms of the area in play, and those of them in its other palette: bit `x` of word `y`.
#[derive(Clone, Copy)]
struct AreaRooms {
    area: [u32; 32],
    other_palette: [u32; 32],
}

const PALETTE_TOGGLE: u16 = 0x0076;
/// The side of the room Samus entered a palette change room from: its `ScrollDir` then (`$EEFA`;
/// 0 once she has left it).
const PALETTE_CHANGE_DIR: u16 = 0x0091;
/// In an area bank: the pointers to the area's palettes (PPU strings, `$C1E0`); the room
/// palettes are 0 and 5 (`PaletteToggle`, `$76`, holds one plus one of them).
const PALETTE_POINTERS: usize = 0x9560;
const ROOM_PALETTES: [u8; 2] = [0, 5];
/// In an area bank: the pointer to its special items (`$ED98`): rows of the map, each `y`, the
/// next row's address (`$FFFF`: none), then rooms, each `x`, the offset of the next (`$FF`:
/// none), then items until 0, item type in the low nibble.
const SPECIAL_ITEMS: usize = 0x9598;
/// The special item that toggles the room palette when Samus goes through its room (`$EEFA`,
/// `$8C84`).
const PALETTE_CHANGE: u8 = 0x0A;
/// The bytes of the special items before the palette changes can be told apart: squeept (3),
/// power-up (3), mellows (1), elevator (2), door (2).
const SPECIAL_ITEM_SIZES: [(u8, usize); 5] = [(1, 3), (2, 3), (3, 1), (4, 2), (9, 2)];
/// In an area bank: the map position of the area's starting room.
const AREA_START: usize = 0x95D7;
/// A tile Samus goes through: at or past this one (`$E7FD`).
const WALKABLE: u8 = 0xA0;
/// The blocks the bombs blast, from the area's first (`CheckBlastTile`, `$D651`) to `$97`.
const LAST_BLAST: u8 = 0x97;
/// Of a room's 30 rows, how many may go through its left or right edge for it to be a shaft (a
/// door's opening).
const SHAFT_OPENING: usize = 6;

/// The area's tables (CPU addresses in its bank).
#[derive(Clone, Copy)]
struct Tables {
    rooms: usize,
    structures: usize,
    metatiles: usize,
}

/// The world as the screen shows it.
struct View<'a> {
    memory: &'a DecoderMemory<'a>,
    bank_index: usize,
    tables: Tables,
    /// Scrolling across (two rooms side by side) or up and down.
    across: bool,
    /// The ring of the two nametables: 512 x 240 across, 256 x 480 up and down.
    ring_width: i32,
    ring_height: i32,
    /// The ring's pixel minus the world's.
    offset_x: i32,
    offset_y: i32,
    /// The world pixel at the screen's top left corner, as the game's variables give it.
    camera_x: i32,
    camera_y: i32,
    /// The rooms in PRG-RAM the screen shows: map x, map y, nametable (0: `$6000`).
    loaded: [Option<(i32, i32, usize)>; 2],
    /// The rooms of the area in play, and those in its other palette.
    rooms: AreaRooms,
}

impl<'a> View<'a> {
    fn of(memory: &'a DecoderMemory<'a>) -> Option<Self> {
        if memory.ram(GAME_MODE) != 0 {
            return None;
        }
        let bank_index = *AREA_BANKS.get(usize::from(memory.ram(IN_AREA) & 0x0F))?;
        let pointer = |index: u16| {
            usize::from(memory.ram(AREA_POINTERS + 2 * index))
                | (usize::from(memory.ram(AREA_POINTERS + 2 * index + 1)) << 8)
        };
        let tables = Tables {
            rooms: pointer(0),
            structures: pointer(1),
            metatiles: pointer(2),
        };
        let direction = memory.ram(SCROLL_DIR);
        let across = direction & 2 != 0;
        let (map_x, map_y) = (i32::from(memory.ram(MAP_X)), i32::from(memory.ram(MAP_Y)));
        let (scroll_x, scroll_y) = (
            i32::from(memory.ram(SCROLL_X)),
            i32::from(memory.ram(SCROLL_Y)),
        );
        let nametable = usize::from(memory.ram(PPUCTRL) & 1);
        // `MapPos` is the room scrolled into: the second of the two on the screen scrolling
        // right or down.
        let view = if across {
            let first = map_x - i32::from(direction == 3 && scroll_x != 0);
            let first_ring = i32::from(nametable == 1) * ROOM_WIDTH;
            Self {
                memory,
                bank_index,
                tables,
                across,
                ring_width: 2 * ROOM_WIDTH,
                ring_height: ROOM_HEIGHT,
                offset_x: (first_ring - first * ROOM_WIDTH).rem_euclid(2 * ROOM_WIDTH),
                offset_y: (-map_y * ROOM_HEIGHT).rem_euclid(ROOM_HEIGHT),
                camera_x: first * ROOM_WIDTH + scroll_x,
                camera_y: map_y * ROOM_HEIGHT + scroll_y,
                loaded: [
                    Some((first, map_y, nametable)),
                    (scroll_x != 0).then_some((first + 1, map_y, nametable ^ 1)),
                ],
                rooms: AreaRooms {
                    area: [0; 32],
                    other_palette: [0; 32],
                },
            }
        } else {
            let first = map_y - i32::from(direction == 1 && scroll_y != 0);
            let first_ring = i32::from(nametable == 1) * ROOM_HEIGHT;
            Self {
                memory,
                bank_index,
                tables,
                across,
                ring_width: ROOM_WIDTH,
                ring_height: 2 * ROOM_HEIGHT,
                offset_x: (-map_x * ROOM_WIDTH).rem_euclid(ROOM_WIDTH),
                offset_y: (first_ring - first * ROOM_HEIGHT).rem_euclid(2 * ROOM_HEIGHT),
                camera_x: map_x * ROOM_WIDTH + scroll_x,
                camera_y: first * ROOM_HEIGHT + scroll_y,
                loaded: [
                    Some((map_x, first, nametable)),
                    (scroll_y != 0).then_some((map_x, first + 1, nametable ^ 1)),
                ],
                rooms: AreaRooms {
                    area: [0; 32],
                    other_palette: [0; 32],
                },
            }
        };
        let (x, y, _) = view.loaded[0]?;
        let rooms = view.area_rooms(x, y)?;
        Some(Self { rooms, ..view })
    }

    /// The row's place on the ring: its left edge across, its top down (a whole tile).
    fn ring_position(&self, row: BackgroundRow) -> (i32, i32) {
        let x = (row.ring_x() - i32::from(row.fine_x)).rem_euclid(self.ring_width);
        let y = if self.across {
            i32::from(row.coarse_y()) * 8
        } else {
            i32::from(row.nametable_y()) * ROOM_HEIGHT + i32::from(row.coarse_y()) * 8
        };
        (x, y)
    }

    /// World tile column of the row's screen column 0, and world tile row of the row (`line`
    /// being where it shows on the screen).
    fn world_tile(&self, row: BackgroundRow, line: i32) -> (i32, i32) {
        let (x, y) = self.ring_position(row);
        let x = resolve_ring(
            (x - self.offset_x).rem_euclid(self.ring_width),
            self.ring_width,
            self.camera_x - i32::from(row.fine_x),
        );
        let y = resolve_ring(
            (y - self.offset_y).rem_euclid(self.ring_height),
            self.ring_height,
            self.camera_y + line - 1,
        );
        (x >> 3, y.div_euclid(8))
    }

    /// CHR tile and sub-palette of world tile `(x, y)`; `None` off the map or where it has no
    /// room.
    fn tile_at(&self, rooms: &mut RoomCache<'a>, x: i32, y: i32) -> Option<(u8, u8)> {
        let (columns, rows) = (ROOM_WIDTH / 8, ROOM_HEIGHT / 8);
        let (room_x, room_y) = (x.div_euclid(columns), y.div_euclid(rows));
        let room = rooms.get(self, room_x, room_y)?;
        #[allow(clippy::cast_sign_loss)] // rem_euclid of positive divisors.
        let (column, row) = (x.rem_euclid(columns) as usize, y.rem_euclid(rows) as usize);
        let tile = *room.get(row * 32 + column)?;
        let attribute = *room.get(ATTRIBUTES + (row / 4) * 8 + column / 4)?;
        let shift = ((row & 2) << 1) | (column & 2);
        // A room in the area's other palette: sub-palettes 4-7, the decoder's margin palette.
        let other = bit(&self.rooms.other_palette, room_x, room_y);
        Some((tile, ((attribute >> shift) & 3) + if other { 4 } else { 0 }))
    }

    /// The room at map `(x, y)`: from PRG-RAM when the screen shows it, else built.
    fn room(&self, x: i32, y: i32) -> Option<Room<'a>> {
        if !(0..MAP_SIZE).contains(&x) || !(0..MAP_SIZE).contains(&y) {
            return None;
        }
        #[allow(clippy::cast_sign_loss)] // 0..32.
        let cell = WORLD_MAP + (y as usize) * 32 + x as usize;
        let number = *self.memory.prg_ram.get(cell)?;
        if number == 0xFF {
            return None;
        }
        let in_area = bit(&self.rooms.area, x, y);
        if let Some((_, _, nametable)) = self
            .loaded
            .iter()
            .flatten()
            .find(|(room_x, room_y, _)| (*room_x, *room_y) == (x, y))
        {
            let start = nametable * ROOM_SIZE;
            return self
                .memory
                .prg_ram
                .get(start..start + ROOM_SIZE)
                .map(Room::Ram);
        }
        if !in_area {
            return None;
        }
        let bank = self.bank()?;
        let data = room_data(bank, self.tables, number)?;
        let mut rooms = ROOMS.lock().ok()?;
        let rooms = rooms.get_or_insert_with(HashMap::new);
        let key = (self.bank_index, data);
        if !rooms.contains_key(&key) {
            if rooms.len() >= MAX_ROOMS {
                rooms.clear();
            }
            rooms.insert(key, build_room(bank, self.tables, data)?);
        }
        rooms.get(&key).map(|room| Room::Built(room.clone()))
    }

    fn bank(&self) -> Option<&'a [u8]> {
        self.memory
            .prg_rom
            .get(self.bank_index * BANK_SIZE..(self.bank_index + 1) * BANK_SIZE)
    }

    /// The rooms of the area in play, reached from its starting room and from the screen's room
    /// at map `(x, y)`, and those of them in the area's other palette (kept while the screen
    /// stays there).
    fn area_rooms(&self, x: i32, y: i32) -> Option<AreaRooms> {
        // A palette change room's other side is the one Samus did not come from.
        let entered_from = match self.memory.ram(PALETTE_CHANGE_DIR) {
            3 => -1,
            2 => 1,
            _ => 0,
        };
        let key = (self.bank_index, x, y, entered_from);
        let mut cached = AREA.lock().ok()?;
        if let Some((at, rooms)) = cached.as_ref() {
            if *at == key {
                return Some(*rooms);
            }
        }
        let bank = self.bank()?;
        let map = self.memory.prg_ram.get(WORLD_MAP..WORLD_MAP + 1024)?;
        let start = (
            i32::from(read(bank, AREA_START)?),
            i32::from(read(bank, AREA_START + 1)?),
        );
        let first_blast = match self.bank_index {
            1 => 0x80,
            3 => WALKABLE,
            _ => 0x70,
        };
        let area = reach(bank, self.tables, map, first_blast, &[start, (x, y)]);
        let changes = palette_changes(bank).unwrap_or_default();
        let rooms = AreaRooms {
            area,
            other_palette: palette_sides(&area, &changes, (x, y), entered_from),
        };
        *cached = Some((key, rooms));
        Some(rooms)
    }

    fn centre_matches(
        &self,
        rooms: &mut RoomCache<'a>,
        row: BackgroundRow,
        (first_column, tile_row): (i32, i32),
    ) -> bool {
        (0..32)
            .filter(|&column| {
                self.tile_at(rooms, first_column + column, tile_row)
                    .is_some_and(|(tile, _)| {
                        (self.memory.nametable)(nametable_address(row, column)) == tile
                    })
            })
            .count()
            >= MATCHING_COLUMNS
    }

    /// The screen's row nearest `row` that reaches neither its top edge nor its bottom one (its
    /// top, the line before the capture line, within 1-231), on the same ring.
    fn inner(&self, row: BackgroundRow) -> BackgroundRow {
        let rows = if row.line < 2 {
            (2 - row.line + 7) / 8
        } else if row.line > LAST_INNER_LINE {
            -((row.line - LAST_INNER_LINE + 7) / 8)
        } else {
            0
        };
        let (_, y) = self.ring_position(row);
        let y = (y + 8 * rows).rem_euclid(self.ring_height);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0..60.
        let (nametable_y, coarse_y) = ((y / ROOM_HEIGHT) as u16, ((y % ROOM_HEIGHT) / 8) as u16);
        let v = if self.across {
            (row.v & !0x03E0) | (coarse_y << 5)
        } else {
            (row.v & !0x0BE0) | (nametable_y << 11) | (coarse_y << 5)
        };
        BackgroundRow {
            line: row.line + 8 * rows,
            v,
            fine_x: row.fine_x,
        }
    }
}

/// A room's 1 KiB: from PRG-RAM, or built.
#[derive(Clone)]
enum Room<'a> {
    Ram(&'a [u8]),
    Built(Box<[u8; ROOM_SIZE]>),
}

impl Room<'_> {
    fn get(&self, index: usize) -> Option<&u8> {
        match self {
            Room::Ram(bytes) => bytes.get(index),
            Room::Built(bytes) => bytes.get(index),
        }
    }
}

/// The rooms a row has looked up, by map position.
struct RoomCache<'a> {
    rooms: Vec<((i32, i32), Option<Room<'a>>)>,
}

impl<'a> RoomCache<'a> {
    fn new() -> Self {
        Self { rooms: Vec::new() }
    }

    fn get(&mut self, view: &View<'a>, x: i32, y: i32) -> Option<&Room<'a>> {
        let index = if let Some(index) = self.rooms.iter().position(|(at, _)| *at == (x, y)) {
            index
        } else {
            self.rooms.push(((x, y), view.room(x, y)));
            self.rooms.len() - 1
        };
        self.rooms[index].1.as_ref()
    }
}

/// Bit `(x, y)` of a set of map rooms (bit `x` of word `y`); false off the map.
fn bit(set: &[u32; 32], x: i32, y: i32) -> bool {
    usize::try_from(y)
        .ok()
        .and_then(|row| set.get(row))
        .is_some_and(|word| (0..MAP_SIZE).contains(&x) && word & (1 << x) != 0)
}

/// The map positions of the area's palette change rooms (its special items, `$ED98`).
fn palette_changes(bank: &[u8]) -> Option<Vec<(i32, i32)>> {
    let mut found = Vec::new();
    let mut row = read_word(bank, SPECIAL_ITEMS)?;
    for _ in 0..32 {
        let y = read(bank, row)?;
        let mut room = row + 3;
        for _ in 0..32 {
            let x = read(bank, room)?;
            let mut item = room + 2;
            for _ in 0..8 {
                let kind = read(bank, item)? & 0x0F;
                if kind == 0 {
                    break;
                }
                if kind == PALETTE_CHANGE {
                    found.push((i32::from(x), i32::from(y)));
                }
                let Some(&(_, size)) = SPECIAL_ITEM_SIZES.iter().find(|(k, _)| *k == kind) else {
                    break;
                };
                item += size;
            }
            match read(bank, room + 1)? {
                0xFF => break,
                next => room += usize::from(next),
            }
        }
        match read_word(bank, row + 1)? {
            0xFFFF => break,
            next => row = next,
        }
    }
    Some(found)
}

/// The rooms of `area` in the other palette than the screen's room `from`: those past a palette
/// change room, on its side across from the one Samus enters it from (`$8C84`: going through it,
/// she leaves by the other door and the game toggles the palette). `entered_from` is that side
/// for `from` itself (-1 left, 1 right, 0: not one).
fn palette_sides(
    area: &[u32; 32],
    changes: &[(i32, i32)],
    from: (i32, i32),
    entered_from: i32,
) -> [u32; 32] {
    let mut other = [0u32; 32];
    if changes.is_empty() || !bit(area, from.0, from.1) {
        return other;
    }
    // Each room once: its palette (true: the other) and, for a palette change room, the side
    // it was entered from.
    let mut seen = [0u32; 32];
    let set = |set: &mut [u32; 32], x: i32, y: i32| {
        #[allow(clippy::cast_sign_loss)] // 0..32.
        {
            set[y as usize] |= 1 << x;
        }
    };
    set(&mut seen, from.0, from.1);
    let mut queue = vec![(from, false, entered_from)];
    while let Some(((x, y), toggled, side)) = queue.pop() {
        let change = changes.contains(&(x, y));
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let (nx, ny) = (x + dx, y + dy);
            if !bit(area, nx, ny) || bit(&seen, nx, ny) {
                continue;
            }
            set(&mut seen, nx, ny);
            let crossed = change && side != 0 && dx == -side;
            let toggled = toggled ^ crossed;
            if toggled {
                set(&mut other, nx, ny);
            }
            queue.push(((nx, ny), toggled, -dx));
        }
    }
    other
}

/// The background colours (`$3F00-$3F0F`) of the area's room palette `palette` (0 or 5): its
/// palette 0, then palette 5's PPU strings over it for 5 (`$C1FF`).
fn room_palette(bank: &[u8], palette: u8) -> Option<[u8; 16]> {
    let mut colours = [0x0F; 16];
    let strings = |id: u8, colours: &mut [u8; 16]| -> Option<()> {
        let mut at = read_word(bank, PALETTE_POINTERS + 2 * usize::from(id))?;
        for _ in 0..16 {
            let high = read(bank, at)?;
            if high == 0 {
                break;
            }
            let address = (usize::from(high) << 8) | usize::from(read(bank, at + 1)?);
            let length = usize::from(read(bank, at + 2)?);
            for index in 0..length {
                let target = address + index;
                if (0x3F00..0x3F10).contains(&target) {
                    colours[target - 0x3F00] = read(bank, at + 3 + index)?;
                }
            }
            at += 3 + length;
        }
        Some(())
    };
    strings(ROOM_PALETTES[0], &mut colours)?;
    if palette != ROOM_PALETTES[0] {
        strings(palette, &mut colours)?;
    }
    Some(colours)
}

/// A room built for the reach of an area: its tiles, and whether it has an elevator.
type ReachRoom = Option<(Box<[u8; ROOM_SIZE]>, bool)>;

/// The rooms of `map` reached from `from` through the edges two neighbours share a way through:
/// bit `x` of word `y`.
fn reach(
    bank: &[u8],
    tables: Tables,
    map: &[u8],
    first_blast: u8,
    from: &[(i32, i32)],
) -> [u32; 32] {
    let goes_through = |tile: u8| tile >= WALKABLE || (first_blast..=LAST_BLAST).contains(&tile);
    let number = |x: i32, y: i32| {
        if !(0..MAP_SIZE).contains(&x) || !(0..MAP_SIZE).contains(&y) {
            return None;
        }
        usize::try_from(y * MAP_SIZE + x)
            .ok()
            .and_then(|cell| map.get(cell).copied())
            .filter(|number| *number != 0xFF)
    };
    let mut built: HashMap<u8, ReachRoom> = HashMap::new();
    let mut room = |number: u8| -> ReachRoom {
        built
            .entry(number)
            .or_insert_with(|| {
                let data = room_data(bank, tables, number)?;
                Some((build_room(bank, tables, data)?, has_elevator(bank, data)?))
            })
            .clone()
    };
    let shaft = |tiles: &[u8; ROOM_SIZE]| {
        [0, 31].iter().all(|&column| {
            (0..30)
                .filter(|row| goes_through(tiles[row * 32 + column]))
                .count()
                <= SHAFT_OPENING
        })
    };
    let mut reached = [0u32; 32];
    let mut queue: Vec<(i32, i32)> = Vec::new();
    for &(x, y) in from {
        if number(x, y).is_some() {
            #[allow(clippy::cast_sign_loss)] // 0..32.
            let (column, row) = (x as u32, y as usize);
            reached[row] |= 1 << column;
            queue.push((x, y));
        }
    }
    while let Some((x, y)) = queue.pop() {
        let Some((tiles, elevator)) = number(x, y).and_then(&mut room) else {
            continue;
        };
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let (nx, ny) = (x + dx, y + dy);
            let Some(other) = number(nx, ny) else {
                continue;
            };
            #[allow(clippy::cast_sign_loss)] // 0..32.
            let (column, row) = (nx as u32, ny as usize);
            if reached[row] & (1 << column) != 0 {
                continue;
            }
            let Some((other_tiles, other_elevator)) = room(other) else {
                continue;
            };
            let open = if dx == 0 {
                let (here, there) = if dy > 0 { (29, 0) } else { (0, 29) };
                !elevator
                    && !other_elevator
                    && shaft(&tiles)
                    && shaft(&other_tiles)
                    && (0..32).any(|column| {
                        goes_through(tiles[here * 32 + column])
                            && goes_through(other_tiles[there * 32 + column])
                    })
            } else {
                let (here, there) = if dx > 0 { (31, 0) } else { (0, 31) };
                (0..30).any(|row| {
                    goes_through(tiles[row * 32 + here])
                        && goes_through(other_tiles[row * 32 + there])
                })
            };
            if open {
                reached[row] |= 1 << column;
                queue.push((nx, ny));
            }
        }
    }
    reached
}

/// Whether the room whose data is at `data` has an elevator among its objects (`$EAD4`: after
/// `$FD`, entries of 3 bytes for an enemy or a pipe of bugs, 2 for a door or an elevator, 1 for
/// the statues).
fn has_elevator(bank: &[u8], data: usize) -> Option<bool> {
    let mut at = data + 1;
    for _ in 0..256 {
        match read(bank, at)? {
            0xFF => return Some(false),
            0xFD => break,
            0xFE => at += 1,
            _ => at += 3,
        }
    }
    at += 1;
    for _ in 0..64 {
        let entry = read(bank, at)?;
        if entry == 0xFF {
            return Some(false);
        }
        match entry & 0x0F {
            4 => return Some(true),
            1 | 7 => at += 3,
            2 => at += 2,
            _ => at += 1,
        }
    }
    Some(false)
}

/// Reads the byte at CPU address `address` of an area bank (`$8000-$BFFF`).
fn read(bank: &[u8], address: usize) -> Option<u8> {
    bank.get(address.checked_sub(0x8000)?).copied()
}

fn read_word(bank: &[u8], address: usize) -> Option<usize> {
    Some(usize::from(read(bank, address)?) | (usize::from(read(bank, address + 1)?) << 8))
}

/// The address of room `number`'s data (the room table is read with an 8-bit index, `$EA42`).
fn room_data(bank: &[u8], tables: Tables, number: u8) -> Option<usize> {
    read_word(bank, tables.rooms + usize::from(number.wrapping_mul(2)))
}

/// Builds the room whose data is at `data` as `SetupRoom` (`$EA2B`) does in PRG-RAM.
fn build_room(bank: &[u8], tables: Tables, data: usize) -> Option<Box<[u8; ROOM_SIZE]>> {
    let mut room = Box::new([0xFF; ROOM_SIZE]);
    let palette = read(bank, data)?;
    room[ATTRIBUTES..].fill([0x00, 0x55, 0xAA, 0xFF][usize::from(palette & 3)]);
    let mut at = data + 1;
    // A room has a few dozen objects at most.
    for _ in 0..256 {
        let position = read(bank, at)?;
        match position {
            0xFF | 0xFD => break,
            0xFE => {
                at += 1;
                continue;
            }
            _ => {}
        }
        let structure = read(bank, at + 1)?;
        let object_palette = read(bank, at + 2)?;
        at += 3;
        let structure = read_word(
            bank,
            tables.structures + usize::from(structure.wrapping_mul(2)),
        )?;
        let work = usize::from(position >> 4) * 0x40 + usize::from(position & 0x0F) * 2;
        draw_structure(
            &mut room,
            bank,
            tables.metatiles,
            structure,
            work,
            (object_palette, palette),
        )?;
    }
    Some(room)
}

/// Draws the structure at `structure` from room offset `work` (`DrawStruct`, `$EF8C`).
fn draw_structure(
    room: &mut [u8; ROOM_SIZE],
    bank: &[u8],
    metatiles: usize,
    mut structure: usize,
    mut work: usize,
    palettes: (u8, u8),
) -> Option<()> {
    // A structure has 15 rows at most on a screen.
    for _ in 0..32 {
        let head = read(bank, structure)?;
        if head == 0xFF {
            return Some(());
        }
        let mut count = usize::from(head & 0x0F);
        if count == 0 {
            count = 16;
        }
        let mut at = work + usize::from(head >> 4) * 2;
        let mut index = 0;
        loop {
            if at >= LAST_METATILE {
                return Some(());
            }
            index += 1;
            let mut metatile = (usize::from(read(bank, structure + index)?) * 4) & 0xFF;
            for offset in [0x00, 0x01, 0x20, 0x21] {
                room[at + offset] = read(bank, metatiles + metatile)?;
                metatile = (metatile + 1) & 0xFF;
            }
            update_attribute(room, at, palettes);
            at += 2;
            if at.trailing_zeros() >= 5 {
                // The row would wrap: its other metatiles are skipped.
                index += count - 1;
                break;
            }
            count -= 1;
            if count == 0 {
                break;
            }
        }
        structure += index + 1;
        work += 0x40;
    }
    Some(())
}

/// Sets the sub-palette of the metatile at room offset `at` (`UpdateAttrib`, `$EF9E`).
fn update_attribute(room: &mut [u8; ROOM_SIZE], at: usize, (object, default): (u8, u8)) {
    if object == default {
        return;
    }
    let index = ATTRIBUTES + ((at >> 7) & 7) * 8 + ((at >> 2) & 7);
    let quarter = usize::from(at & 0x40 != 0) * 2 + usize::from(at & 2 != 0);
    let mut bits = object;
    for _ in 0..quarter {
        let carry = bits & 0x40 != 0;
        bits <<= 2;
        if carry {
            break;
        }
    }
    room[index] = (room[index] & [0xFC, 0xF3, 0xCF, 0x3F][quarter]) | bits;
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

impl Metroid {
    fn decode_any(
        memory: &DecoderMemory<'_>,
        row: BackgroundRow,
        columns: &mut dyn Iterator<Item = i32>,
        out: &mut Vec<DecodedTile>,
    ) -> bool {
        if row.coarse_y() >= 30 {
            return false;
        }
        let Some(view) = View::of(memory) else {
            return false;
        };
        let mut rooms = RoomCache::new();
        // A row at or past the screen's edge may be the one the game draws coming in: the check
        // is made on the screen's nearest row wholly inside it.
        let checked = view.inner(row);
        if !view.centre_matches(&mut rooms, checked, view.world_tile(checked, checked.line)) {
            return false;
        }
        let (first_column, tile_row) = view.world_tile(row, row.line);
        for column in columns {
            if let Some((tile, palette)) = view.tile_at(&mut rooms, first_column + column, tile_row)
            {
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

impl MarginDecoder for Metroid {
    fn decode_row(
        &self,
        memory: &DecoderMemory<'_>,
        row: &BackgroundRow,
        columns: &mut dyn Iterator<Item = i32>,
        out: &mut Vec<DecodedTile>,
    ) -> bool {
        Self::decode_any(memory, *row, columns, out)
    }

    fn decode_extra_row(
        &self,
        memory: &DecoderMemory<'_>,
        row: &BackgroundRow,
        columns: &mut dyn Iterator<Item = i32>,
        out: &mut Vec<DecodedTile>,
    ) -> bool {
        Self::decode_any(memory, *row, columns, out)
    }

    fn covers_overscan(&self) -> bool {
        // Scrolling up and down, the game draws the row coming in at the screen's edge, in the
        // lines a television hides.
        true
    }

    fn margin_palette(&self, memory: &DecoderMemory<'_>) -> Option<[u8; 16]> {
        // The area's other room palette, for the rooms past a palette change room.
        if memory.ram(GAME_MODE) != 0 {
            return None;
        }
        let bank_index = *AREA_BANKS.get(usize::from(memory.ram(IN_AREA) & 0x0F))?;
        let bank = memory
            .prg_rom
            .get(bank_index * BANK_SIZE..(bank_index + 1) * BANK_SIZE)?;
        if palette_changes(bank)?.is_empty() {
            return None;
        }
        let current = memory.ram(PALETTE_TOGGLE).wrapping_sub(1);
        let other = if current == ROOM_PALETTES[1] {
            ROOM_PALETTES[0]
        } else {
            ROOM_PALETTES[1]
        };
        room_palette(bank, other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rooms_past_a_palette_change_are_in_the_other_palette() {
        // A corridor of five rooms on map row 0, the middle one a palette change room.
        let area = {
            let mut area = [0u32; 32];
            area[0] = 0b1_1111;
            area
        };
        let changes = [(2, 0)];
        let other = palette_sides(&area, &changes, (0, 0), 0);
        assert_eq!(other[0], 0b1_1000);
        // In the change room itself, entered from the right: the left side is the other.
        let other = palette_sides(&area, &changes, (2, 0), 1);
        assert_eq!(other[0], 0b0_0011);
        // An area without one: none.
        assert_eq!(palette_sides(&area, &[], (0, 0), 0), [0; 32]);
    }

    #[test]
    fn rooms_are_built_from_structures_of_metatiles() {
        let mut bank = vec![0u8; BANK_SIZE];
        let (rooms, structures, metatiles) = (0x8100, 0x8200, 0x8300);
        let tables = Tables {
            rooms,
            structures,
            metatiles,
        };
        // Room 1: default sub-palette 2, one object at metatile (x 3, y 2), structure 0 in
        // sub-palette 1, then the end of the objects.
        bank[0x0102..0x0104].copy_from_slice(&[0x00, 0x84]);
        bank[0x0400..0x0405].copy_from_slice(&[0x02, 0x23, 0x00, 0x01, 0xFD]);
        // Structure 0: a row of two metatiles (5, 6) one metatile in, then a row of one (7).
        bank[0x0200..0x0202].copy_from_slice(&[0x00, 0x85]);
        bank[0x0500..0x0506].copy_from_slice(&[0x12, 0x05, 0x06, 0x01, 0x07, 0xFF]);
        for metatile in 5..8 {
            for tile in 0..4 {
                bank[0x0300 + metatile * 4 + tile] = u8::try_from(metatile * 16 + tile).unwrap();
            }
        }
        let data = room_data(&bank, tables, 1).unwrap();
        let room = build_room(&bank, tables, data).unwrap();
        // Metatile 5 at tile (8, 4): upper left, upper right, lower left, lower right.
        assert_eq!(&room[4 * 32 + 8..4 * 32 + 12], &[0x50, 0x51, 0x60, 0x61]);
        assert_eq!(&room[5 * 32 + 8..5 * 32 + 12], &[0x52, 0x53, 0x62, 0x63]);
        // The second row, one metatile lower, has no offset.
        assert_eq!(&room[6 * 32 + 6..6 * 32 + 8], &[0x70, 0x71]);
        assert_eq!(room[4 * 32 + 7], 0xFF);
        // Tiles (8, 4) and (10, 4): attribute byte 2 of row 1, its two upper quarters; the rest
        // keeps the room's sub-palette 2.
        assert_eq!(room[ATTRIBUTES + 8 + 2], 0b1010_0101);
        assert_eq!(room[ATTRIBUTES], 0xAA);
    }
}
