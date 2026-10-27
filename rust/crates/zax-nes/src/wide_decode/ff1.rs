//! Final Fantasy (USA): the margins of the world map and of the towns, castles and dungeons,
//! decoded from their maps.
//!
//! Both kinds of map are grids of tiles of 16 x 16 pixels, drawn through the tileset the game
//! keeps in RAM: tile `t` is the CHR tiles at `$0500 + t` (top left), `$0580 + t` (top right),
//! `$0600 + t` (bottom left) and `$0680 + t` (bottom right), its attribute byte (the same
//! sub-palette in its four quarters) at `$0700 + t` (the game's drawing, `$D1E4`).
//!
//! * The world map (`$2D` bit 0 clear) is 256 x 256 tiles, a torus. Its rows are run-length
//!   encoded from PRG bank 1 (`$D157`): row `y` starts at the pointer at `$8000 + 2 y` and goes
//!   on into bank 2 past `$BFFF`; a byte below `$80` is one tile, `$80 + t` then a count is `t`
//!   that many times (0 for 256), `$FF` ends the row. The game unpacks only the 16 rows around
//!   the screen (to `$7000 + 256 (y & 15)`); the decoder unpacks any row from the ROM, so the
//!   vertical margin goes on too. The camera's top left tile is `$27`, `$28`.
//! * A standard map (`$2D` bit 0 set) is 64 x 64 tiles at `$7000 + 64 y + x` of PRG-RAM, the
//!   game's own copy (with what it changed: chests opened, doors), wrapping as the game reads
//!   it. The camera's top left tile is `$29`, `$2A`.
//!
//! The screen is the plane of two nametables side by side: 32 tiles across, so its x is the
//! map's modulo 512 pixels, and 15 tiles down, the game keeping the nametable row of the
//! camera's top tile in `$2F` (0-14): the map's rows are the nametable's from there.
//!
//! The game's 15 rows all show: scrolling vertically, it draws the row coming in over the one
//! leaving, at the screen's top or bottom edge. The screen's first and last rows are decoded
//! whole width too, with the vertical margin (`covers_overscan`).
//!
//! The decoder checks itself against the PPU: a frame whose centre does not show the map at
//! three rows at least at one of them (a battle, a menu, the title) is left to the nametables;
//! the rows the game scrolls otherwise to show a dialog box over the map are placed as the camera
//! in RAM says, and get the map in their margins.

use super::{resolve_ring, BackgroundRow, DecodedTile, DecoderMemory, MarginDecoder};

/// The decoder.
pub struct FinalFantasy;

/// CRC-32 of the ROM's data, iNES header excluded: Final Fantasy (USA).
pub const SOURCE_CRC: u32 = 0xCEBD_2A31;

const MAP_FLAGS: u16 = 0x002D;
const WORLD_SCROLL_X: u16 = 0x0027;
const WORLD_SCROLL_Y: u16 = 0x0028;
const STANDARD_SCROLL_X: u16 = 0x0029;
const STANDARD_SCROLL_Y: u16 = 0x002A;
/// The nametable row (of 16 x 16 tiles, 0-14) of the camera's top tile.
const SCROLL_ROW: u16 = 0x002F;
/// The camera's fine scroll within a tile, in pixels.
const FINE_X: u16 = 0x0035;
const FINE_Y: u16 = 0x0036;

const TILESET: u16 = 0x0500;

/// The world map's row pointers, at `$8000` of PRG bank 1.
const WORLD_ROWS: usize = 0x4000;
const WORLD_SIZE: i32 = 256;
/// The standard map, `$7000` of PRG-RAM.
const STANDARD_MAP: usize = 0x1000;
const STANDARD_SIZE: i32 = 64;

/// Pixels of the plane: two nametables across, one down.
const PLANE_WIDTH: i32 = 512;
const PLANE_HEIGHT: i32 = 240;
const MAP_ROWS: i32 = 15;

/// Screen columns whose tiles are checked against the nametables.
const CHECKED: [i32; 4] = [3, 11, 19, 27];
/// Lines of the screen whose rows are checked: one of them showing the map is enough.
const CHECKED_LINES: [i32; 3] = [25, 121, 217];

/// The map of the frame.
enum Map<'a> {
    /// The world map's packed rows.
    World(&'a [u8]),
    /// A standard map in PRG-RAM.
    Standard(&'a [u8]),
}

impl Map<'_> {
    /// The map tile at tile `x`, `y` (16 x 16 pixels), wrapping.
    fn tile(&self, x: i32, y: i32) -> Option<u8> {
        match self {
            Map::World(rom) => world_tile(rom, x.rem_euclid(WORLD_SIZE), y.rem_euclid(WORLD_SIZE)),
            Map::Standard(map) => {
                let at =
                    (y.rem_euclid(STANDARD_SIZE) * STANDARD_SIZE) + x.rem_euclid(STANDARD_SIZE);
                map.get(STANDARD_MAP + usize::try_from(at).ok()?).copied()
            }
        }
    }
}

/// Tile `x` of the world map's row `y`, unpacked from the ROM as `$D157` does.
fn world_tile(rom: &[u8], x: i32, y: i32) -> Option<u8> {
    let entry = WORLD_ROWS + (2 * usize::try_from(y).ok()?);
    let pointer = usize::from(u16::from_le_bytes([*rom.get(entry)?, *rom.get(entry + 1)?]));
    // Bank 1 at $8000, bank 2 after it: the row goes on in the next bank past $BFFF.
    let mut at = WORLD_ROWS + pointer.checked_sub(0x8000)?;
    let mut column = 0;
    while column < WORLD_SIZE {
        let byte = *rom.get(at)?;
        at += 1;
        let (tile, count) = if byte == 0xFF {
            return None;
        } else if byte & 0x80 == 0 {
            (byte, 1)
        } else {
            let count = *rom.get(at)?;
            at += 1;
            (byte & 0x7F, if count == 0 { 256 } else { i32::from(count) })
        };
        if x < column + count {
            return Some(tile);
        }
        column += count;
    }
    None
}

/// Where the screen lies, on the plane of the nametables and on the map: the pixel at the left
/// edge (x = 0) of the screen, and the one on its line 0.
#[derive(Clone, Copy)]
struct Placement {
    plane_left: i32,
    map_left: i32,
    plane_top: i32,
    map_top: i32,
}

/// The frame as the decoder reads it: the map, and the camera.
struct View<'a> {
    map: Map<'a>,
    tileset: &'a [u8],
    /// The camera's top left pixel on the map, as the game writes its scroll (`$C346`, `$CCA6`):
    /// x `16 $27 + $35` (`$29` in a standard map), y `16 $28 + $36` (`$2A`).
    camera_x: i32,
    camera_y: i32,
    /// The nametable's pixel row of the camera's top line, `16 $2F + $36`.
    camera_plane_y: i32,
}

impl<'a> View<'a> {
    fn of(memory: &DecoderMemory<'a>) -> Option<View<'a>> {
        let standard = memory.ram(MAP_FLAGS) & 1 != 0;
        let (map, scroll_x, scroll_y) = if standard {
            (
                Map::Standard(memory.prg_ram),
                STANDARD_SCROLL_X,
                STANDARD_SCROLL_Y,
            )
        } else {
            (Map::World(memory.prg_rom), WORLD_SCROLL_X, WORLD_SCROLL_Y)
        };
        let row = i32::from(memory.ram(SCROLL_ROW));
        if row >= MAP_ROWS {
            return None;
        }
        let fine_x = i32::from(memory.ram(FINE_X) & 15);
        let fine_y = i32::from(memory.ram(FINE_Y) & 15);
        Some(View {
            map,
            tileset: memory
                .ram
                .get(usize::from(TILESET)..usize::from(TILESET) + 0x280)?,
            camera_x: (i32::from(memory.ram(scroll_x)) * 16) + fine_x,
            camera_y: (i32::from(memory.ram(scroll_y)) * 16) + fine_y,
            camera_plane_y: (row * 16) + fine_y,
        })
    }

    /// The CHR tile and sub-palette at map tile column `x`, tile row `y` (8 x 8 pixels).
    fn chr_at(&self, x: i32, y: i32) -> Option<(u8, u8)> {
        let tile = usize::from(self.map.tile(x >> 1, y >> 1)?);
        let quarter = usize::try_from(((y & 1) << 1) | (x & 1)).ok()?;
        let pattern = *self.tileset.get((quarter * 0x80) + tile)?;
        let attribute = *self.tileset.get((4 * 0x80) + tile)?;
        Some((pattern, attribute & 3))
    }

    /// The placement `row`'s PPU address gives, the camera picking the instance on the map.
    /// RAM is read after the frame: the PPU's position is the frame's.
    fn placement_of(&self, row: BackgroundRow) -> Placement {
        let plane_left = row.ring_x();
        let plane_y = plane_y(row);
        // The map's rows are the nametable's from row $2F on: the same modulo 240.
        let position = (plane_y + self.camera_y - self.camera_plane_y).rem_euclid(PLANE_HEIGHT);
        let map_y = resolve_ring(position, PLANE_HEIGHT, self.camera_y + row.line);
        Placement {
            plane_left,
            map_left: resolve_ring(plane_left, PLANE_WIDTH, self.camera_x),
            plane_top: plane_y - row.line,
            map_top: map_y - row.line,
        }
    }

    /// The placement of the camera in RAM: the map's, for a row the game scrolls otherwise (a
    /// dialog box).
    fn camera_placement(&self) -> Placement {
        Placement {
            plane_left: self.camera_x.rem_euclid(PLANE_WIDTH),
            map_left: self.camera_x,
            plane_top: self.camera_plane_y,
            map_top: self.camera_y,
        }
    }

    /// Whether the PPU shows the map, so placed, at screen line `line`.
    fn shows(&self, memory: &DecoderMemory<'_>, placement: Placement, line: i32) -> bool {
        let tile_row = (placement.map_top + line) >> 3;
        let nametable_row = (placement.plane_top + line).rem_euclid(PLANE_HEIGHT) >> 3;
        let plane_column = placement.plane_left >> 3;
        let first = placement.map_left >> 3;
        CHECKED.iter().all(|&column| {
            self.chr_at(first + column, tile_row).map(|(tile, _)| tile)
                == Some((memory.nametable)(nametable_address(
                    plane_column + column,
                    nametable_row,
                )))
        })
    }

    /// Whether the frame shows the map so placed: one of a few rows across the screen does.
    fn frame_shows(&self, memory: &DecoderMemory<'_>, placement: Placement) -> bool {
        CHECKED_LINES
            .iter()
            .any(|&line| self.shows(memory, placement, line))
    }

    /// Where `row` shows the map: as its PPU address says, or for a row scrolled otherwise (a
    /// dialog box over the map), as the camera says; `None` when the frame shows no map.
    fn placement(&self, memory: &DecoderMemory<'_>, row: BackgroundRow) -> Option<Placement> {
        let own = self.placement_of(row);
        if self.frame_shows(memory, own) {
            return Some(own);
        }
        let camera = self.camera_placement();
        self.frame_shows(memory, camera).then_some(camera)
    }

    fn decode(
        &self,
        placement: Placement,
        row: BackgroundRow,
        columns: &mut dyn Iterator<Item = i32>,
        out: &mut Vec<DecodedTile>,
    ) {
        // Screen column c starts at x = 8 c - fine X.
        let first = (placement.map_left - i32::from(row.fine_x)).div_euclid(8);
        let tile_row = (placement.map_top + row.line) >> 3;
        for column in columns {
            if let Some((tile, palette)) = self.chr_at(first + column, tile_row) {
                out.push(DecodedTile {
                    column,
                    tile,
                    palette,
                });
            }
        }
    }
}

/// The plane's pixel row of `row` (on the plane of two stacked nametables for a row past the
/// screen: the same modulo 240).
fn plane_y(row: BackgroundRow) -> i32 {
    (i32::from(row.nametable_y()) * PLANE_HEIGHT)
        + (i32::from(row.coarse_y()) * 8)
        + i32::from((row.v >> 12) & 7)
}

/// The nametable address of tile column `column` (0-63, wrapping) of the plane of two
/// nametables side by side, at nametable row `nametable_row` (0-29).
fn nametable_address(column: i32, nametable_row: i32) -> u16 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0..64, 0..30.
    let (column, nametable_row) = (column.rem_euclid(64) as u16, nametable_row as u16);
    0x2000 | ((column >> 5) << 10) | (nametable_row << 5) | (column & 31)
}

impl MarginDecoder for FinalFantasy {
    fn decode_row(
        &self,
        memory: &DecoderMemory<'_>,
        row: &BackgroundRow,
        columns: &mut dyn Iterator<Item = i32>,
        out: &mut Vec<DecodedTile>,
    ) -> bool {
        if row.coarse_y() >= 30 {
            return false;
        }
        let Some(view) = View::of(memory) else {
            return false;
        };
        let Some(placement) = view.placement(memory, *row) else {
            return false;
        };
        view.decode(placement, *row, columns, out);
        true
    }

    fn decode_extra_row(
        &self,
        memory: &DecoderMemory<'_>,
        row: &BackgroundRow,
        columns: &mut dyn Iterator<Item = i32>,
        out: &mut Vec<DecodedTile>,
    ) -> bool {
        let Some(view) = View::of(memory) else {
            return false;
        };
        let Some(placement) = view.placement(memory, *row) else {
            return false;
        };
        view.decode(placement, *row, columns, out);
        true
    }

    fn covers_overscan(&self) -> bool {
        // The game's ring is the 15 rows of one nametable, all on the screen: scrolling
        // vertically, it draws the row coming in over the one at the other edge, in the lines a
        // television hides.
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_rows_unpack_runs() {
        let mut rom = vec![0u8; WORLD_ROWS + 0x200];
        // Row 0 at $8100: tile 5, tile $17 x 3, tile $76 x 252, end.
        rom[WORLD_ROWS..WORLD_ROWS + 2].copy_from_slice(&[0x00, 0x81]);
        let row = WORLD_ROWS + 0x100;
        rom[row..row + 6].copy_from_slice(&[0x05, 0x97, 0x03, 0xF6, 252, 0xFF]);
        assert_eq!(world_tile(&rom, 0, 0), Some(0x05));
        assert_eq!(world_tile(&rom, 1, 0), Some(0x17));
        assert_eq!(world_tile(&rom, 3, 0), Some(0x17));
        assert_eq!(world_tile(&rom, 4, 0), Some(0x76));
        assert_eq!(world_tile(&rom, 255, 0), Some(0x76));
    }

    #[test]
    fn tiles_come_from_the_tileset_quarters() {
        let mut ram = vec![0u8; 0x800];
        let mut prg_ram = vec![0u8; 0x2000];
        // Standard map tile (2, 1) is tile $10.
        prg_ram[STANDARD_MAP + 64 + 2] = 0x10;
        for (quarter, pattern) in [0x40u8, 0x41, 0x42, 0x43].into_iter().enumerate() {
            ram[0x500 + (quarter * 0x80) + 0x10] = pattern;
        }
        ram[0x700 + 0x10] = 0xAA;
        let view = View {
            map: Map::Standard(&prg_ram),
            tileset: &ram[0x500..0x780],
            camera_x: 0,
            camera_y: 0,
            camera_plane_y: 0,
        };
        assert_eq!(view.chr_at(4, 2), Some((0x40, 2)), "top left");
        assert_eq!(view.chr_at(5, 2), Some((0x41, 2)), "top right");
        assert_eq!(view.chr_at(4, 3), Some((0x42, 2)), "bottom left");
        assert_eq!(view.chr_at(5, 3), Some((0x43, 2)), "bottom right");
        assert_eq!(view.chr_at(4 + 128, 2 + 128), Some((0x40, 2)), "wraps");
    }
}
