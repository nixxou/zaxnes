//! Blaster Master (USA): the margins of an area, decoded from its map in PRG-ROM.
//!
//! An area is 2048 x 2048 pixels, 8 x 8 screens of 256 x 256, laid out in four levels as the
//! game's column and row drawers (`$DA15`, `$DAC4`) read them through the pointers `$D975` copies
//! into `$27-$30` from the area's header:
//!
//! * the map (`$2F/$30`) is 32 x 32 blocks of 64 x 64 pixels, row first;
//! * block `b` is 4 blocks of 32 x 32 at `($2D) + 4 b` (top left, top right, bottom left, bottom
//!   right), each of them 4 metatiles of 16 x 16 at `($2B) + 4 n`, each of them 4 CHR tiles at
//!   `($29) + 4 m`, in the same order;
//! * metatile `m`'s sub-palette is the low two bits of `($27) + m`.
//!
//! The tables are in the PRG bank `$C9F4[$14] >> 4` gives for the area `$14` (`$EA3C`).
//!
//! The camera is `$1D:$1C` and `$1F:$1E` (sixteenths of a pixel), the world x and y of the
//! screen's top left corner. The game shows two nametables side by side: a ring of 512 x 240
//! pixels, drawn at a fixed offset from the world (the scroll `$FD` with `$F1` bit 0 and `$FC`
//! follow the camera's moves), the rows modulo 240.
//!
//! The camera stays within a room: a screen's boundary with its neighbour may be closed
//! (`$CC6D`: the bits of the 16 bytes `$CA04[$14]` points to, `8 + y` for the boundary left of
//! screen `(x, y)`, `y` for the one above it, bit `7 - x`), and the camera stops in the middle of
//! a screen with a closed boundary ahead. The margins show the screens the camera's screens reach
//! through open boundaries only. In the overhead view (areas `$14` 0-7) the camera goes across
//! closed boundaries: the margins show every screen.
//!
//! The decoder checks itself against the PPU: a row whose centre does not show what the map gives,
//! nor its two ends (the game draws some rooms of the overhead view over the map), is left to the
//! nametables (the title, the area's name, a transition, the pause screen).

use super::{resolve_ring, BackgroundRow, DecodedTile, DecoderMemory, MarginDecoder};

/// The decoder.
pub struct BlasterMaster;

/// CRC-32 of the ROM's data, iNES header excluded.
pub const SOURCE_CRC: u32 = 0x3F0F_D764;

const AREA: u16 = 0x0014;
const PALETTES: u16 = 0x0027;
const TILES: u16 = 0x0029;
const METATILES: u16 = 0x002B;
const BLOCKS: u16 = 0x002D;
const MAP: u16 = 0x002F;
const CAMERA_X: u16 = 0x001C;
const CAMERA_Y: u16 = 0x001E;
const SCROLL_X: u16 = 0x00FD;
const SCROLL_Y: u16 = 0x00FC;
const NAMETABLE: u16 = 0x00F1;

/// In the fixed bank: per area, the bank and entry of its header and of its boundaries.
const HEADERS: usize = 0xC9F4;
const BOUNDARIES: usize = 0xCA04;

const BANK_SIZE: usize = 0x4000;
const PLANE_WIDTH: i32 = 512;
const PLANE_HEIGHT: i32 = 240;
const SCREEN: i32 = 256;
const SCREENS: i32 = 8;
/// The last capture line of a row whose 8 lines end before the screen's last.
const LAST_INNER_LINE: i32 = 232;

/// Of the screen's 32 columns, how many must show the map's tile for a row to be the map's (a few
/// may show something the game changed: a door, a block).
const MATCHING_COLUMNS: usize = 28;
/// The screen's first and last four columns, checked when the others are not the map's.
const EDGES: [i32; 8] = [0, 1, 2, 3, 28, 29, 30, 31];
/// The first area of the side view (`$14`): below are the overhead view's.
const FIRST_SIDE_AREA: u8 = 8;

/// The area in play.
struct Area<'a> {
    bank: &'a [u8],
    palettes: usize,
    tiles: usize,
    metatiles: usize,
    blocks: usize,
    map: usize,
    /// Bit `y * 8 + x`: the screen shows in the margins.
    shown: u64,
    camera_x: i32,
    camera_y: i32,
    /// The ring's pixel minus the world's, across (0-511) and down (0-239).
    offset_x: i32,
    offset_y: i32,
}

impl<'a> Area<'a> {
    fn of(memory: &DecoderMemory<'a>, margin_x: i32, margin_y: i32) -> Option<Self> {
        let rom = memory.prg_rom;
        let fixed = rom.len().checked_sub(BANK_SIZE)?;
        let area = usize::from(memory.ram(AREA));
        let header = *rom.get(fixed + HEADERS - 0xC000 + area)?;
        let bank_index = usize::from(header >> 4);
        let bank = rom.get(bank_index * BANK_SIZE..(bank_index + 1) * BANK_SIZE)?;
        let pointer = |address: u16| -> Option<usize> {
            let value =
                usize::from(memory.ram(address)) | (usize::from(memory.ram(address + 1)) << 8);
            value
                .checked_sub(0x8000)
                .filter(|offset| *offset < BANK_SIZE)
        };
        let camera_x = word(memory, CAMERA_X) >> 4;
        let camera_y = word(memory, CAMERA_Y) >> 4;
        let scroll_x = i32::from(memory.ram(SCROLL_X)) + 256 * i32::from(memory.ram(NAMETABLE) & 1);
        let boundaries = boundaries(rom, *rom.get(fixed + BOUNDARIES - 0xC000 + area)?)?;
        Some(Self {
            bank,
            palettes: pointer(PALETTES)?,
            tiles: pointer(TILES)?,
            metatiles: pointer(METATILES)?,
            blocks: pointer(BLOCKS)?,
            map: pointer(MAP)?,
            // The overhead view's camera goes across closed boundaries: there they are not a
            // room's.
            shown: if memory.ram(AREA) < FIRST_SIDE_AREA {
                u64::MAX
            } else {
                shown_screens(&boundaries, camera_x, camera_y, margin_x, margin_y)
            },
            camera_x,
            camera_y,
            offset_x: (scroll_x - camera_x).rem_euclid(PLANE_WIDTH),
            offset_y: (i32::from(memory.ram(SCROLL_Y)) - camera_y).rem_euclid(PLANE_HEIGHT),
        })
    }

    /// World tile column of the row's screen column 0.
    fn first_column(&self, row: BackgroundRow) -> i32 {
        let ring = (row.ring_x() - i32::from(row.fine_x) - self.offset_x).rem_euclid(PLANE_WIDTH);
        resolve_ring(ring, PLANE_WIDTH, self.camera_x - i32::from(row.fine_x)) >> 3
    }

    /// World tile row of the row, `line` being where it shows on the screen.
    fn tile_row(&self, row: BackgroundRow, line: i32) -> i32 {
        let plane = (i32::from(row.coarse_y()) * 8 - self.offset_y).rem_euclid(PLANE_HEIGHT);
        resolve_ring(plane, PLANE_HEIGHT, self.camera_y + line - (line & 7)) >> 3
    }

    fn centre_matches(
        &self,
        memory: &DecoderMemory<'_>,
        row: BackgroundRow,
        first_column: i32,
        tile_row: i32,
    ) -> bool {
        let matches = |column: i32| {
            self.tile_at(first_column + column, tile_row)
                .filter(|(tile, _)| (memory.nametable)(nametable_address(row, column)) == *tile)
                .map(|(tile, _)| tile)
        };
        // A few columns would not do: the game blanks the nametables around a boss, where the
        // map has tiles every few columns.
        if (0..32).filter(|&column| matches(column).is_some()).count() >= MATCHING_COLUMNS {
            return true;
        }
        // The game may draw something of its own over the map in the middle of the screen (a
        // room of the overhead view): the row is the map's still when both of its ends are, in
        // more than one tile.
        let ends: Option<Vec<u8>> = EDGES.iter().map(|&column| matches(column)).collect();
        ends.is_some_and(|tiles| tiles.iter().any(|tile| *tile != tiles[0]))
    }

    fn decode(
        &self,
        first_column: i32,
        tile_row: i32,
        columns: &mut dyn Iterator<Item = i32>,
        out: &mut Vec<DecodedTile>,
    ) {
        for column in columns {
            let x = first_column + column;
            if !self.shows(x, tile_row) {
                continue;
            }
            if let Some((tile, palette)) = self.tile_at(x, tile_row) {
                out.push(DecodedTile {
                    column,
                    tile,
                    palette,
                });
            }
        }
    }

    /// Whether world tile `(x, y)` is in a screen the margins show.
    fn shows(&self, x: i32, y: i32) -> bool {
        let (sx, sy) = (x * 8 / SCREEN, y * 8 / SCREEN);
        (0..SCREENS).contains(&sx)
            && (0..SCREENS).contains(&sy)
            && x >= 0
            && y >= 0
            && self.shown & (1 << (sy * SCREENS + sx)) != 0
    }

    /// CHR tile and sub-palette of world tile `(x, y)` (0-255 each); `None` off the area.
    fn tile_at(&self, x: i32, y: i32) -> Option<(u8, u8)> {
        if !(0..256).contains(&x) || !(0..256).contains(&y) {
            return None;
        }
        #[allow(clippy::cast_sign_loss)] // 0..256.
        let (x, y) = (x as usize, y as usize);
        let quarter = |bit: usize| ((y >> bit) & 1) * 2 + ((x >> bit) & 1);
        let block = usize::from(*self.bank.get(self.map + (y >> 3) * 32 + (x >> 3))?);
        let block32 = usize::from(*self.bank.get(self.blocks + block * 4 + quarter(2))?);
        let metatile = usize::from(*self.bank.get(self.metatiles + block32 * 4 + quarter(1))?);
        let tile = *self.bank.get(self.tiles + metatile * 4 + quarter(0))?;
        let palette = self.bank.get(self.palettes + metatile)? & 3;
        Some((tile, palette))
    }
}

/// The area's 16 boundary bytes: entry `code & 15` of the pointers at `$8000` of PRG bank
/// `code >> 4` (`$EA3C`).
fn boundaries(rom: &[u8], code: u8) -> Option<[u8; 16]> {
    let bank = usize::from(code >> 4) * BANK_SIZE;
    let entry = bank + usize::from(code & 15) * 2;
    let pointer = usize::from(*rom.get(entry)?) | (usize::from(*rom.get(entry + 1)?) << 8);
    let at = bank + pointer.checked_sub(0x8000)?;
    rom.get(at..at + 16)?.try_into().ok()
}

/// Bit `y * 8 + x`: the screens the view (the camera's screen widened by the margins) touches
/// that the camera's own screens reach through open boundaries.
fn shown_screens(
    boundaries: &[u8; 16],
    camera_x: i32,
    camera_y: i32,
    margin_x: i32,
    margin_y: i32,
) -> u64 {
    let bit = |index: i32, x: i32| {
        usize::try_from(index)
            .ok()
            .and_then(|index| boundaries.get(index))
            .is_some_and(|byte| byte & (0x80 >> x) != 0)
    };
    let open_left = |x: i32, y: i32| !bit(8 + y, x);
    let open_above = |x: i32, y: i32| !bit(y, x);
    let screens = |from: i32, to: i32| {
        (
            from.div_euclid(SCREEN).max(0),
            to.div_euclid(SCREEN).min(SCREENS - 1),
        )
    };
    let (left, right) = screens(camera_x - margin_x, camera_x + SCREEN - 1 + margin_x);
    let (top, bottom) = screens(camera_y - margin_y, camera_y + PLANE_HEIGHT - 1 + margin_y);
    let in_view = |x: i32, y: i32| (left..=right).contains(&x) && (top..=bottom).contains(&y);
    let (first_x, last_x) = screens(camera_x, camera_x + SCREEN - 1);
    let (first_y, last_y) = screens(camera_y, camera_y + PLANE_HEIGHT - 1);
    let mut shown = 0u64;
    let mut stack = Vec::new();
    for y in first_y..=last_y {
        for x in first_x..=last_x {
            stack.push((x, y));
        }
    }
    while let Some((x, y)) = stack.pop() {
        let bit = 1u64 << (y * SCREENS + x);
        if !in_view(x, y) || shown & bit != 0 {
            continue;
        }
        shown |= bit;
        if x > 0 && open_left(x, y) {
            stack.push((x - 1, y));
        }
        if x + 1 < SCREENS && open_left(x + 1, y) {
            stack.push((x + 1, y));
        }
        if y > 0 && open_above(x, y) {
            stack.push((x, y - 1));
        }
        if y + 1 < SCREENS && open_above(x, y + 1) {
            stack.push((x, y + 1));
        }
    }
    shown
}

/// The 16-bit value at `address` (low byte first).
fn word(memory: &DecoderMemory<'_>, address: u16) -> i32 {
    i32::from(memory.ram(address)) | (i32::from(memory.ram(address + 1)) << 8)
}

/// The nametable address of screen column `column` (0 = the left edge) of `row`, on the plane
/// of two nametables side by side.
fn nametable_address(row: BackgroundRow, column: i32) -> u16 {
    let plane = (i32::from(row.nametable_x()) * 32) + i32::from(row.coarse_x()) + column;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0..64.
    let plane = plane.rem_euclid(64) as u16;
    0x2000 | ((plane >> 5) << 10) | (u16::from(row.coarse_y()) << 5) | (plane & 31)
}

impl BlasterMaster {
    fn decode_any(
        memory: &DecoderMemory<'_>,
        row: BackgroundRow,
        columns: &mut dyn Iterator<Item = i32>,
        out: &mut Vec<DecodedTile>,
    ) -> bool {
        if row.coarse_y() >= 30 {
            return false;
        }
        // The widest margins the host may ask for: the screens they reach are worked out once
        // for any of them.
        let Some(area) = Area::of(memory, 248, 240) else {
            return false;
        };
        let first_column = area.first_column(row);
        let tile_row = area.tile_row(row, row.line);
        // A row at or past the screen's edge may be the row the game keeps for the overscan (the
        // one it draws coming in, or a stale one: a band across the picture), so the check is
        // made on the screen's nearest row wholly inside it, the same distance away on the ring;
        // the margins of a row across the edge (decode_row) as well as its centre and the rows
        // past it (decode_extra_row).
        let checked = Self::inner(row);
        let checked_row = area.tile_row(checked, checked.line);
        if !area.centre_matches(memory, checked, first_column, checked_row) {
            return false;
        }
        area.decode(first_column, tile_row, columns, out);
        true
    }

    /// The screen's row nearest `row` that reaches neither its top edge nor its bottom one (its
    /// top, the line before the capture line, within 1-231), on the same ring of 30 rows.
    fn inner(row: BackgroundRow) -> BackgroundRow {
        let rows = if row.line < 2 {
            (2 - row.line + 7) / 8
        } else if row.line > LAST_INNER_LINE {
            -((row.line - LAST_INNER_LINE + 7) / 8)
        } else {
            0
        };
        let coarse_y = (i32::from(row.coarse_y()) + rows).rem_euclid(30);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0..30.
        let coarse_y = coarse_y as u16;
        BackgroundRow {
            line: row.line + 8 * rows,
            v: (row.v & !0x03E0) | (coarse_y << 5),
            fine_x: row.fine_x,
        }
    }
}

impl MarginDecoder for BlasterMaster {
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
        // The ring is 30 rows, all on the screen: scrolling vertically, the game draws the row
        // coming in over the one at the other edge, in the lines a television hides.
        true
    }

    fn overscan_lines(&self) -> i32 {
        // The attribute bytes the game writes for that row also colour the next one, on the
        // screen within 8 lines of the edge.
        8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiles_come_through_four_levels() {
        let mut bank = vec![0u8; BANK_SIZE];
        let (palettes, tiles, metatiles, blocks, map) = (0x100, 0x200, 0x600, 0xA00, 0x1000);
        // World tile (13, 3): block (1, 0) of the map, its top right 32-pixel block (x bit 2
        // set, y bit 2 clear), that one's bottom left metatile (y bit 1 set, x bit 1 clear), its
        // bottom right tile (bits 0 set).
        bank[map + 1] = 2;
        bank[blocks + 2 * 4 + 1] = 3;
        bank[metatiles + 3 * 4 + 2] = 5;
        bank[tiles + 5 * 4 + 3] = 0x42;
        bank[palettes + 5] = 0xF6;
        let area = Area {
            bank: &bank,
            palettes,
            tiles,
            metatiles,
            blocks,
            map,
            shown: u64::MAX,
            camera_x: 0,
            camera_y: 0,
            offset_x: 0,
            offset_y: 0,
        };
        assert_eq!(area.tile_at(13, 3), Some((0x42, 2)));
        assert_eq!(area.tile_at(256, 0), None, "past the area");
    }

    #[test]
    fn closed_boundaries_hide_the_next_room() {
        // Column 3 of screens is another room: the boundaries left of its screens are closed
        // (rows 1 to 3, those the view reaches). The boundary above (2, 2) is closed too, but
        // (2, 1) is reached around it, through (1, 2) and (1, 1): the same room.
        let mut boundaries = [0u8; 16];
        for row in 1..=3 {
            boundaries[8 + row] = 0x80 >> 3;
        }
        boundaries[2] = 0x80 >> 2;
        // The camera on screen (2, 2) exactly, a margin of 248 across and 240 down.
        let shown = shown_screens(&boundaries, 512, 512, 248, 240);
        let at = |x: i32, y: i32| shown & (1 << (y * 8 + x)) != 0;
        assert!(at(2, 2));
        assert!(at(1, 2), "open to the left");
        assert!(!at(3, 2), "closed to the right");
        assert!(at(2, 1), "reached around");
        assert!(!at(3, 1), "another room");
        assert!(at(2, 3), "open below");
    }

    #[test]
    fn edge_rows_are_checked_one_row_in() {
        let row = |line: i32, coarse_y: u16| BackgroundRow {
            line,
            v: 0x0405 | (coarse_y << 5),
            fine_x: 3,
        };
        // The last row, ending on the screen's edge: the row above, on the ring.
        let inner = BlasterMaster::inner(row(233, 0));
        assert_eq!(
            (inner.line, inner.coarse_y(), inner.v & 0x041F),
            (225, 29, 0x0405)
        );
        // A row across the top edge: the next one down.
        let inner = BlasterMaster::inner(row(-3, 29));
        assert_eq!((inner.line, inner.coarse_y()), (5, 0));
        // Two rows below the screen: three rows up.
        let inner = BlasterMaster::inner(row(249, 4));
        assert_eq!((inner.line, inner.coarse_y()), (225, 1));
        // A row inside the screen stays.
        assert_eq!(BlasterMaster::inner(row(120, 15)).line, 120);
    }
}
