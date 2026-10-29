//! Mega Man 2 (USA): the margins of a stage, decoded from its layout in PRG-ROM.
//!
//! A stage lives in PRG bank `$2A & 7` (`$2A` the stage), as the game's own collision lookup
//! (`$CBC3`) and column drawer (`$C96B`) read it:
//!
//! * screen `s` of the stage is 64 bytes at `$8500 + 64 s`, 8 columns of 8 structures (32 x 32
//!   pixels), column first: byte `8 column + row`;
//! * structure `n` is 4 bytes at `$8000 + 4 n`, its four 16 x 16 blocks column first (top left,
//!   bottom left, top right, bottom right); its attribute byte is at `$8400 + n`;
//! * block `b` is CHR tiles `4 b` to `4 b + 3`, column first too (top left, bottom left, top
//!   right, bottom right); the low six bits of the block byte are `b`, the high two its
//!   collision.
//!
//! The level is a straight list of screens: world x / 256 is the screen. The camera (`$20:$1F`)
//! is kept between screens `$14` and `$15` of the section being played; the screens past them
//! belong to the next section (above, below, past a door), so nothing is drawn there.
//!
//! The decoder checks itself against the PPU: a row whose centre does not show what the layout
//! gives (stage select, boss intro, a vertical transition, the pause menu) is left to the
//! nametables.

use super::{resolve_ring, BackgroundRow, DecodedTile, DecoderMemory, MarginDecoder};

/// The decoder.
pub struct MegaMan2;

/// CRC-32 of the ROM's data, iNES header excluded.
pub const SOURCE_CRC: u32 = 0x0FCF_C04D;
/// CRC-32 of the European build's data: the same stage layouts.
pub const SOURCE_CRC_EUROPE: u32 = 0xA663_8CBA;
/// CRC-32 of Rockman 2 (Japan)'s data: the same stage layouts and RAM.
pub const SOURCE_CRC_JAPAN: u32 = 0x6150_517C;

const STAGE: u16 = 0x002A;
const CAMERA_LOW: u16 = 0x001F;
const CAMERA_HIGH: u16 = 0x0020;
const SECTION_FIRST: u16 = 0x0014;
const SECTION_LAST: u16 = 0x0015;

const STRUCTURES: usize = 0x0000;
const ATTRIBUTES: usize = 0x0400;
const SCREENS: usize = 0x0500;
const BANK_SIZE: usize = 0x4000;

/// Screen columns whose tiles are checked against the nametables.
const CHECKED: [i32; 4] = [3, 11, 19, 27];

impl MarginDecoder for MegaMan2 {
    fn decode_row(
        &self,
        memory: &DecoderMemory<'_>,
        row: &BackgroundRow,
        columns: &mut dyn Iterator<Item = i32>,
        out: &mut Vec<DecodedTile>,
    ) -> bool {
        let coarse_y = i32::from(row.coarse_y());
        if coarse_y >= 30 {
            return false;
        }
        let bank = usize::from(memory.ram(STAGE) & 7);
        let Some(stage) = memory.prg_rom.get(bank * BANK_SIZE..(bank + 1) * BANK_SIZE) else {
            return false;
        };
        let camera = (i32::from(memory.ram(CAMERA_HIGH)) << 8) | i32::from(memory.ram(CAMERA_LOW));
        let first_column = resolve_ring(row.ring_x(), 512, camera) >> 3;

        // What the PPU shows in the centre must be what the layout gives.
        for column in CHECKED {
            let world = first_column + column;
            let Some((tile, _)) = tile_at(stage, world, coarse_y) else {
                return false;
            };
            if (memory.nametable)(nametable_address(*row, column)) != tile {
                return false;
            }
        }

        // The section's screens; right after a vertical transition the game updates them some
        // frames after the camera (Wood Man's falls: 33 frames), the camera then outside them:
        // until it does, the camera's screen alone (the screen it left lies above or below, not
        // beside it).
        let (mut first_screen, mut last_screen) = (
            i32::from(memory.ram(SECTION_FIRST)),
            i32::from(memory.ram(SECTION_LAST)),
        );
        let camera_screen = camera >> 8;
        if !(first_screen..=last_screen).contains(&camera_screen) {
            (first_screen, last_screen) = (camera_screen, camera_screen);
        }
        let first = first_screen * 32;
        let end = (last_screen + 1) * 32;
        for column in columns {
            let world = first_column + column;
            if !(first..end).contains(&world) {
                continue;
            }
            // A column the nametables hold for this very place shows what the game drew there,
            // things objects draw into the background included (Air Man's cloud devils).
            let decoded = if column_is_current(memory, stage, *row, column, world) {
                Some(nametable_tile(memory, *row, column))
            } else {
                tile_at(stage, world, coarse_y)
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

/// Whether screen column `column` of the nametables (on the plane of two) holds world tile column
/// `world`, not an older column of the ring. The game draws the level a 32-pixel block at a time;
/// what an object draws over it (Air Man's cloud devils, Bubble Man's lantern fish) comes later. So
/// the column is current when its rows that differ from the layout were all written after every
/// row that matches it (and enough rows match); an older column of the ring was drawn in one go,
/// its differences among the rows that happen to match.
fn column_is_current(
    memory: &DecoderMemory<'_>,
    stage: &[u8],
    row: BackgroundRow,
    column: i32,
    world: i32,
) -> bool {
    let mut newest_match = 0u32;
    let mut oldest_difference = u32::MAX;
    let mut matching = 0;
    for tile_row in 0i32..30 {
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)] // 0..30.
        let v = (row.v & !0x03E0) | ((tile_row as u16) << 5);
        let address = nametable_address(BackgroundRow { v, ..row }, column);
        let written = (memory.nametable_written)(address);
        let same = tile_at(stage, world, tile_row)
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

/// CHR tile and sub-palette of world tile column `world`, tile row `tile_row` (0-29), from the
/// stage bank `stage`; `None` before the level or past the bank.
fn tile_at(stage: &[u8], world: i32, tile_row: i32) -> Option<(u8, u8)> {
    let world = usize::try_from(world).ok()?;
    let tile_row = usize::try_from(tile_row).ok()?;
    let screen = world >> 5;
    let column = world & 31;
    let structure = *stage.get(SCREENS + (screen * 64) + ((column >> 2) * 8) + (tile_row >> 2))?;
    let right = (column >> 1) & 1;
    let lower = (tile_row >> 1) & 1;
    let block = *stage.get(STRUCTURES + (usize::from(structure) * 4) + (right * 2) + lower)? & 0x3F;
    #[allow(clippy::cast_possible_truncation)] // One bit each.
    let tile = (block << 2) | ((column as u8 & 1) << 1) | (tile_row as u8 & 1);
    let attribute = *stage.get(ATTRIBUTES + usize::from(structure))?;
    let palette = (attribute >> ((lower * 4) + (right * 2))) & 3;
    Some((tile, palette))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiles_come_from_screens_structures_and_blocks() {
        let mut stage = vec![0u8; BANK_SIZE];
        // Screen 1, structure column 2, structure row 3: structure 5.
        stage[SCREENS + 64 + 16 + 3] = 5;
        // Structure 5: blocks TL 1, BL 2, TR 3, BR $C4 (collision bits on block 4).
        stage[STRUCTURES + 20..STRUCTURES + 24].copy_from_slice(&[1, 2, 3, 0xC4]);
        // Its attribute: TL 0, TR 1, BL 2, BR 3.
        stage[ATTRIBUTES + 5] = 0b11_10_01_00;
        // World tile column 32 + 8 = 40 is structure column 2 of screen 1; tile row 12 is
        // structure row 3.
        assert_eq!(tile_at(&stage, 40, 12), Some((4, 0)), "block 1, top left");
        assert_eq!(
            tile_at(&stage, 40, 13),
            Some((5, 0)),
            "block 1, bottom left"
        );
        assert_eq!(tile_at(&stage, 41, 12), Some((6, 0)), "block 1, top right");
        assert_eq!(
            tile_at(&stage, 42, 12),
            Some((12, 1)),
            "block 3, top right block"
        );
        assert_eq!(
            tile_at(&stage, 40, 14),
            Some((8, 2)),
            "block 2, bottom left block"
        );
        assert_eq!(
            tile_at(&stage, 43, 15),
            Some((19, 3)),
            "block 4 without its collision"
        );
        assert_eq!(tile_at(&stage, -1, 0), None);
    }

    #[test]
    fn nametable_addresses_wrap_on_two_nametables() {
        let row = BackgroundRow {
            line: 0,
            v: (1 << 10) | (4 << 5) | 0x1E,
            fine_x: 0,
        };
        assert_eq!(nametable_address(row, 0), 0x2400 | (4 << 5) | 0x1E);
        assert_eq!(nametable_address(row, 2), 0x2000 | (4 << 5));
    }
}
