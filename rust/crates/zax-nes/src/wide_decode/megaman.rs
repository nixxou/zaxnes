//! What the Mega Man decoders share: a stage is a straight list of screens of 32 tile columns,
//! the camera kept within the screens of the section being played, and the margins drawn from the
//! stage's layout, or from the nametables where the game still holds the column for that place.
//!
//! Each game gives the layout (`tile_at`: the CHR tile and sub-palette of a world tile column and
//! a tile row), the camera and the section; the rest is the Mega Man 2 decoder's method (see
//! `megaman2.rs`): a row whose centre does not show the layout at four columns is left to the
//! nametables (stage select, boss intro, a vertical transition, the pause menu).

use super::{resolve_ring, BackgroundRow, DecodedTile, DecoderMemory};

/// Screen columns whose tiles are checked against the nametables.
const CHECKED: [i32; 4] = [3, 11, 19, 27];

/// Decodes the margins of `row` for a stage whose camera's left edge is world pixel `camera`,
/// within world tile columns `section` (the section being played).
pub(super) fn decode_row(
    memory: &DecoderMemory<'_>,
    row: BackgroundRow,
    columns: &mut dyn Iterator<Item = i32>,
    out: &mut Vec<DecodedTile>,
    camera: i32,
    section: std::ops::Range<i32>,
    tile_at: &dyn Fn(i32, i32) -> Option<(u8, u8)>,
) -> bool {
    let coarse_y = i32::from(row.coarse_y());
    if coarse_y >= 30 {
        return false;
    }
    // The row's place: the instance of its pixel on the plane nearest the camera, modulo 256
    // pixels (the camera in RAM is a few pixels from the scroll the frame used at most; and a
    // game may show the same nametable on both sides, Mega Man 3 at the end of a section, where
    // the plane's two instances lie as far from the camera). The centre must show what the layout
    // gives there.
    let first_column = resolve_ring(row.ring_x() & 255, 256, camera) >> 3;
    let centre_matches = CHECKED.iter().all(|&column| {
        tile_at(first_column + column, coarse_y)
            .is_some_and(|(tile, _)| (memory.nametable)(nametable_address(row, column)) == tile)
    });
    if !centre_matches {
        return false;
    }

    // With the same nametable on both sides (mirrored), a margin's nametable column is a copy of
    // one on the screen: never the margin's own, though a repeating wall may match the layout.
    let mirrored = (0..32).all(|column| {
        (memory.nametable)(nametable_address(row, column))
            == (memory.nametable)(nametable_address(row, column + 32))
    });

    for column in columns {
        let world = first_column + column;
        if !section.contains(&world) {
            continue;
        }
        // A column the nametables hold for this very place shows what the game drew there,
        // things objects draw into the background included; its colours are the layout's, the
        // game writing a column's attribute bytes after its tiles (the column it is drawing past
        // the screen's edge showed in another column's colours).
        let layout = tile_at(world, coarse_y);
        let own_column = !mirrored || (0..32).contains(&column);
        let decoded = if own_column && column_is_current(memory, row, column, world, tile_at) {
            let (tile, palette) = nametable_tile(memory, row, column);
            Some((tile, layout.map_or(palette, |(_, palette)| palette)))
        } else {
            layout
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

/// Whether screen column `column` of the nametables (on the plane of two) holds world tile column
/// `world`, not an older column of the ring: its rows that differ from the layout were all written
/// after every row that matches it, and enough rows match.
fn column_is_current(
    memory: &DecoderMemory<'_>,
    row: BackgroundRow,
    column: i32,
    world: i32,
    tile_at: &dyn Fn(i32, i32) -> Option<(u8, u8)>,
) -> bool {
    let mut newest_match = 0u32;
    let mut oldest_difference = u32::MAX;
    let mut matching = 0;
    for tile_row in 0i32..30 {
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)] // 0..30.
        let v = (row.v & !0x03E0) | ((tile_row as u16) << 5);
        let address = nametable_address(BackgroundRow { v, ..row }, column);
        let written = (memory.nametable_written)(address);
        let same =
            tile_at(world, tile_row).is_some_and(|(tile, _)| (memory.nametable)(address) == tile);
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
pub(super) fn nametable_address(row: BackgroundRow, column: i32) -> u16 {
    let plane = (i32::from(row.nametable_x()) * 32) + i32::from(row.coarse_x()) + column;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0..64.
    let plane = plane.rem_euclid(64) as u16;
    0x2000
        | (u16::from(row.nametable_y()) << 11)
        | ((plane >> 5) << 10)
        | (u16::from(row.coarse_y()) << 5)
        | (plane & 31)
}

/// The tile of a 32 x 32 structure: its four 16 x 16 blocks column first (top left, bottom left,
/// top right, bottom right) in `blocks`, block `b` being CHR tiles `4 b` to `4 b + 3` column
/// first too; `attribute` its attribute byte. `column` and `tile_row` are world tile coordinates.
pub(super) fn structure_tile(
    blocks: [u8; 4],
    attribute: u8,
    column: i32,
    tile_row: i32,
) -> (u8, u8) {
    let right = usize::from(column & 2 != 0);
    let lower = usize::from(tile_row & 2 != 0);
    let block = blocks[(right * 2) + lower] & 0x3F;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // One bit each.
    let tile = (block << 2) | (((column & 1) as u8) << 1) | ((tile_row & 1) as u8);
    let palette = (attribute >> ((lower * 4) + (right * 2))) & 3;
    (tile, palette)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structures_lay_blocks_out_column_first() {
        let blocks = [1, 2, 3, 0xC4];
        let attribute = 0b11_10_01_00;
        assert_eq!(structure_tile(blocks, attribute, 40, 12), (4, 0));
        assert_eq!(structure_tile(blocks, attribute, 40, 13), (5, 0));
        assert_eq!(structure_tile(blocks, attribute, 41, 12), (6, 0));
        assert_eq!(structure_tile(blocks, attribute, 42, 12), (12, 1));
        assert_eq!(structure_tile(blocks, attribute, 40, 14), (8, 2));
        assert_eq!(structure_tile(blocks, attribute, 43, 15), (19, 3));
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
