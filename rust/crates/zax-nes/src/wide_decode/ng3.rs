//! Ninja Gaiden III (USA): the margins of an area, decoded from its layout in PRG-ROM.
//!
//! The game draws its areas from PRG banks 0 to 3 (`$C01E`, in the fixed bank, with banks 0 and 1
//! at `$8000` and banks 2 and 3 at `$A000` in turn):
//!
//! * area `a` (`$5F`) starts at the address `$B960 + a` / `$B990 + a` of bank 1 gives (in banks 0
//!   and 1 seen at `$8000-$BFFF`); its flags are `$A0F0 + a` of bank 6 (bit 7: a vertical area,
//!   drawn by rows of 8 blocks), its last camera screen the low nibble of `$A1B0 + a`;
//! * a horizontal area is a list of columns of 32 pixels, 6 bytes each, one block of 32 x 32
//!   pixels per byte from the top of the playfield (nametable rows 0-23) down;
//! * block `b` is 4 sub-blocks of 16 x 16 pixels at `($51) + 4 b` (top left, top right, bottom
//!   left, bottom right), its attribute byte at `($53) + b`; sub-block `s` is 4 tiles at
//!   `($55) + 4 s`, in the same order. Those three pointers (`$51`-`$56`) are set from the area's
//!   flags and point into banks 2 and 3, seen at `$8000-$BFFF`.
//!
//! The camera is `$DC:$DB`. Each row's centre is checked against the nametables
//! (a cut scene, a transition, a vertical area are left to the nametables); a margin column the
//! nametables still hold for that place is taken from them, with what the game drew over the
//! layout since. Rows 24 and past (the status bar) are left to the nametables.

use super::{resolve_ring, BackgroundRow, DecodedTile, DecoderMemory, MarginDecoder};

/// The decoder.
pub struct NinjaGaiden3;

/// CRC-32 of the ROM's data, iNES header excluded.
pub const SOURCE_CRC: u32 = 0x902E_3168;

const AREA: u16 = 0x005F;
const CAMERA_LOW: u16 = 0x00DB;
const CAMERA_HIGH: u16 = 0x00DC;
const BLOCKS: u16 = 0x0051;
const ATTRIBUTES: u16 = 0x0053;
const SUB_BLOCKS: u16 = 0x0055;

/// Area start addresses, low and high bytes (bank 1 at `$A000`).
const AREA_START_LOW: usize = 0x2000 + 0x1960;
const AREA_START_HIGH: usize = 0x2000 + 0x1990;
/// Area flags (bank 6 at `$A000`): bit 7, a vertical area.
const AREA_FLAGS: usize = 0xC000 + 0x00F0;
/// The area's last camera screen in the low nibble (bank 6 at `$A000`).
const AREA_SCREENS: usize = 0xC000 + 0x01B0;
const AREAS: u8 = 48;
/// Banks 0-1 (the layouts) and 2-3 (the blocks) in PRG-ROM, each pair seen at `$8000-$BFFF`.
const LAYOUT_BANKS: usize = 0x0000;
const GRAPHICS_BANKS: usize = 0x4000;
const BANK_PAIR: usize = 0x4000;

/// Tile rows of the playfield: 6 blocks of 4.
const ROWS: i32 = 24;
/// Columns of the centre (of 32) that must show the layout's tile.
const MATCHING: usize = 20;

/// The horizontal area being played, as read from PRG-ROM and RAM.
struct Area<'a> {
    layout: &'a [u8],
    graphics: &'a [u8],
    /// Offset of the area's first column in `layout`.
    start: usize,
    /// Tile columns in the area.
    columns: i32,
    blocks: usize,
    attributes: usize,
    sub_blocks: usize,
}

impl<'a> Area<'a> {
    fn of(memory: &DecoderMemory<'a>) -> Option<Self> {
        let area = memory.ram(AREA);
        if area >= AREAS {
            return None;
        }
        let rom = memory.prg_rom;
        let index = usize::from(area);
        if rom.get(AREA_FLAGS + index)? & 0x80 != 0 {
            return None;
        }
        let address = u16::from(*rom.get(AREA_START_LOW + index)?)
            | (u16::from(*rom.get(AREA_START_HIGH + index)?) << 8);
        let start = usize::from(address.checked_sub(0x8000)?);
        let screens = i32::from(rom.get(AREA_SCREENS + index)? & 0x0F) + 1;
        let pointer = |at: u16| {
            let address = u16::from(memory.ram(at)) | (u16::from(memory.ram(at + 1)) << 8);
            address.checked_sub(0x8000).map(usize::from)
        };
        Some(Self {
            layout: rom.get(LAYOUT_BANKS..LAYOUT_BANKS + BANK_PAIR)?,
            graphics: rom.get(GRAPHICS_BANKS..GRAPHICS_BANKS + BANK_PAIR)?,
            start,
            columns: screens * 32,
            blocks: pointer(BLOCKS)?,
            attributes: pointer(ATTRIBUTES)?,
            sub_blocks: pointer(SUB_BLOCKS)?,
        })
    }

    /// CHR tile and sub-palette of world tile column `world`, tile row `tile_row` (0-23); `None`
    /// outside the area.
    fn tile_at(&self, world: i32, tile_row: i32) -> Option<(u8, u8)> {
        if !(0..self.columns).contains(&world) || !(0..ROWS).contains(&tile_row) {
            return None;
        }
        let world = usize::try_from(world).ok()?;
        let tile_row = usize::try_from(tile_row).ok()?;
        let block = *self
            .layout
            .get(self.start + ((world >> 2) * 6) + (tile_row >> 2))?;
        let quarter = (((tile_row >> 1) & 1) * 2) + ((world >> 1) & 1);
        let sub_block = *self
            .graphics
            .get(self.blocks + (usize::from(block) * 4) + quarter)?;
        let tile = *self.graphics.get(
            self.sub_blocks + (usize::from(sub_block) * 4) + ((tile_row & 1) * 2) + (world & 1),
        )?;
        let attribute = *self.graphics.get(self.attributes + usize::from(block))?;
        let shift = ((tile_row & 2) << 1) | (world & 2);
        Some((tile, (attribute >> shift) & 3))
    }
}

impl MarginDecoder for NinjaGaiden3 {
    fn decode_row(
        &self,
        memory: &DecoderMemory<'_>,
        row: &BackgroundRow,
        columns: &mut dyn Iterator<Item = i32>,
        out: &mut Vec<DecodedTile>,
    ) -> bool {
        let tile_row = i32::from(row.coarse_y());
        if tile_row >= ROWS {
            return false;
        }
        let Some(area) = Area::of(memory) else {
            return false;
        };
        let camera = (i32::from(memory.ram(CAMERA_HIGH)) << 8) | i32::from(memory.ram(CAMERA_LOW));
        // A row at the camera's scroll shows the area where the camera is. A band the game
        // scrolls on its own (the parallax of skies and mountains, by the IRQ) shows the ring of
        // 64 tile columns as the game drew it around the camera: each of its columns is the
        // layout's column of that ring position nearest to the camera (such a band's layout repeats
        // itself, so the ring always looks whole).
        let placements = [
            Placement {
                first: resolve_ring(row.ring_x(), 512, camera) >> 3,
                ring_near: None,
            },
            Placement {
                first: row.ring_x() >> 3,
                // The game draws nothing past the ends of the area: there the ring holds what it
                // drew within.
                ring_near: Some(((camera >> 3) + 16).clamp(32, (area.columns - 32).max(32))),
            },
        ];
        let Some(placement) = placements
            .into_iter()
            .find(|placement| centre_matches(memory, &area, *row, placement))
        else {
            return false;
        };

        for column in columns {
            let world = placement.world(column);
            // A column the nametables hold for this very place shows what the game drew there.
            let decoded = if column_is_current(memory, &area, *row, column, world) {
                Some(nametable_tile(memory, *row, column))
            } else {
                area.tile_at(world, tile_row)
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

/// Where the screen columns of a row are in the area.
#[derive(Clone, Copy)]
struct Placement {
    /// World tile column of screen column 0.
    first: i32,
    /// For a band scrolled on its own: the world tile column the ring of 64 is taken around.
    ring_near: Option<i32>,
}

impl Placement {
    fn world(self, column: i32) -> i32 {
        let world = self.first + column;
        self.ring_near
            .map_or(world, |near| resolve_ring(world.rem_euclid(64), 64, near))
    }
}

/// Whether the centre of `row` shows what the layout gives there, placed as `placement` says: at
/// 20 columns of 32 at least (the game animates some tiles, the sand of the desert, draws into the
/// background, and a band it scrolls on its own may show a seam where the ring was drawn at two
/// moments; another stretch of the area, a cut scene or a transition differ nearly everywhere).
fn centre_matches(
    memory: &DecoderMemory<'_>,
    area: &Area<'_>,
    row: BackgroundRow,
    placement: &Placement,
) -> bool {
    let tile_row = i32::from(row.coarse_y());
    let matching = (0..32)
        .filter(|&column| {
            area.tile_at(placement.world(column), tile_row)
                .is_some_and(|(tile, _)| (memory.nametable)(nametable_address(row, column)) == tile)
        })
        .count();
    matching >= MATCHING
}

/// Whether screen column `column` of the nametables (on the plane of two) holds world tile column
/// `world`, not an older column of the ring: its rows that differ from the layout were all written
/// after every row that matches it, and enough rows match (Mega Man 2's rule).
fn column_is_current(
    memory: &DecoderMemory<'_>,
    area: &Area<'_>,
    row: BackgroundRow,
    column: i32,
    world: i32,
) -> bool {
    if !(0..area.columns).contains(&world) {
        return false;
    }
    let mut newest_match = 0u32;
    let mut oldest_difference = u32::MAX;
    let mut matching = 0;
    for tile_row in 0..ROWS {
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)] // 0..24.
        let v = (row.v & !0x03E0) | ((tile_row as u16) << 5);
        let address = nametable_address(BackgroundRow { v, ..row }, column);
        let written = (memory.nametable_written)(address);
        let same = area
            .tile_at(world, tile_row)
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

    #[test]
    fn tiles_come_from_columns_blocks_and_sub_blocks() {
        let mut layout = vec![0u8; BANK_PAIR];
        let mut graphics = vec![0u8; BANK_PAIR];
        // Column 2 of an area starting at $0100 (offset), block row 3: block 5.
        layout[0x0100 + (2 * 6) + 3] = 5;
        // Block 5: sub-blocks TL 1, TR 2, BL 3, BR 4; attribute TL 0, TR 1, BL 2, BR 3.
        graphics[0x1C00 + 20..0x1C00 + 24].copy_from_slice(&[1, 2, 3, 4]);
        graphics[0x3A00 + 5] = 0b11_10_01_00;
        // Sub-block 3: tiles $30-$33; sub-block 2: tiles $20-$23.
        graphics[12..16].copy_from_slice(&[0x30, 0x31, 0x32, 0x33]);
        graphics[8..12].copy_from_slice(&[0x20, 0x21, 0x22, 0x23]);
        let area = Area {
            layout: &layout,
            graphics: &graphics,
            start: 0x0100,
            columns: 32,
            blocks: 0x1C00,
            attributes: 0x3A00,
            sub_blocks: 0,
        };
        // World tile column 8 is block column 2; tile row 12 is block row 3.
        assert_eq!(
            area.tile_at(8, 14),
            Some((0x30, 2)),
            "bottom left sub-block, its top left"
        );
        assert_eq!(
            area.tile_at(9, 15),
            Some((0x33, 2)),
            "its bottom right tile"
        );
        assert_eq!(area.tile_at(10, 12), Some((0x20, 1)), "top right sub-block");
        assert_eq!(area.tile_at(32, 12), None, "past the area");
        assert_eq!(area.tile_at(8, 24), None, "past the playfield");
    }

    #[test]
    fn a_band_scrolled_on_its_own_reads_the_ring_around_the_camera() {
        let placement = Placement {
            first: 61,
            ring_near: Some(80),
        };
        assert_eq!(placement.world(3), 64);
        // World column 41 is ring position 41, held around the camera as column 105.
        assert_eq!(placement.world(-20), 105);
        let plain = Placement {
            first: 61,
            ring_near: None,
        };
        assert_eq!(plain.world(3), 64);
        assert_eq!(plain.world(-5), 56);
    }
}
