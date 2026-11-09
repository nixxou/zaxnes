//! Faxanadu (USA): the margins of an area, decoded from its screens in PRG-ROM.
//!
//! The world is cut into screens of 16 x 13 blocks of 16 x 16 pixels, under a status bar of four
//! tile rows. The game keeps only the screen on display in RAM (`$0600`, 16 blocks a row), and
//! when the player walks past an edge it slides to the screen next to it, drawing it column by
//! column at 4 pixels a frame; at rest the other nametable holds whatever was there before, not
//! a neighbour. The decoder draws the neighbours from the level data, as the game's own screen
//! loader (`$D1F6`, fixed bank) reads them:
//!
//! * screen `s` is compressed at `$8000 +` the word at `($7A) + 2 s`, in PRG bank `$8C` (both
//!   set by the game for the area): for each of the 256 block positions in turn, 2 bits (most
//!   significant first): 0, 1 or 2 copy the block 1, 16 or 17 positions back (left, above,
//!   above left), 3 reads the block from the next 8 bits; positions `$F0`-`$FF` are cleared;
//! * the area `$24`'s tables are in bank 3: the word at `$8000` leads to a table of one word per
//!   area, each to the area's header, whose first word leads to five words: the blocks'
//!   attributes, then the four tables of their tiles (top left, top right, bottom left, bottom
//!   right), one byte per block; the header's third word leads to the screens' links, 4 bytes
//!   per screen (left, right, above, below; `$FF` none). The links of the screen on display are
//!   also in RAM (`$66`-`$69`);
//! * a block's sub-palette is the low two bits of its attribute byte.
//!
//! `$63` is the screen on display, or the one the game slides to from the moment it chose it
//! (some 17 frames before the slide starts); `$0D` counts the screens slid through (bit 0 the
//! nametable of the one at rest) and `$0C` the pixels of the slide under way, `$54` its direction
//! (0 left, 1 right, 2 up, 3 down, `$FF` none). Past an
//! area's end (no link) the margin stays empty. A row whose screen edges are not what the decoder
//! reads there (a vertical slide, a building's inside the decoder does not know, the text box
//! drawn over the screen's edge) keeps the nametables'.

use super::{BackgroundRow, DecodedTile, DecoderMemory, MarginDecoder};

/// The decoder.
pub struct Faxanadu;

/// CRC-32 of the ROM's data, iNES header excluded. Every build has the screens' loader, the
/// areas' tables (banks 0-3) and the RAM of (USA); the variants come by their pack's signatures.
pub const SOURCE_CRC: u32 = 0x57DD_23D1;
/// (USA) (Rev 1), (Europe) and (Japan).
pub const SOURCE_CRC_REV1: u32 = 0xE71D_B268;
pub const SOURCE_CRC_EUROPE: u32 = 0x76C1_61E3;
pub const SOURCE_CRC_JAPAN: u32 = 0xA80F_A181;

const AREA: u16 = 0x0024;
const SCREEN: u16 = 0x0063;
const LINK_LEFT: u16 = 0x0066;
const LINK_RIGHT: u16 = 0x0067;
const SCREEN_BANK: u16 = 0x008C;
const SCREEN_TABLE: u16 = 0x007A;
const SLIDE_X: u16 = 0x000C;
const SLIDE_SCREENS: u16 = 0x000D;
const SLIDE_DIRECTION: u16 = 0x0054;
const SCROLL_Y: u16 = 0x0057;
/// `$54`: a slide to the left, to the right, or none under way.
const LEFT: u8 = 0;
const RIGHT: u8 = 1;
const AT_REST: u8 = 0xFF;

/// The bank of the areas' block tables and links.
const AREA_BANK: usize = 3;
const BANK_SIZE: usize = 0x4000;
/// The status bar's tile rows, above the screen's 13 rows of blocks.
const STATUS_ROWS: i32 = 4;
const BLOCK_ROWS: i32 = 13;
/// No screen there.
const NONE: u8 = 0xFF;
/// The screens a margin reaches at most on each side (512 pixels).
const MAX_STEPS: i32 = 2;

impl MarginDecoder for Faxanadu {
    fn decode_row(
        &self,
        memory: &DecoderMemory<'_>,
        row: &BackgroundRow,
        columns: &mut dyn Iterator<Item = i32>,
        out: &mut Vec<DecodedTile>,
    ) -> bool {
        let tile_row = i32::from(row.coarse_y()) - STATUS_ROWS;
        if !(0..BLOCK_ROWS * 2).contains(&tile_row) || memory.ram(SCROLL_Y) != 0 {
            return false;
        }
        let Some(area) = Area::new(memory) else {
            return false;
        };
        // The work RAM is a frame ahead of the picture, whose rows were drawn with the scroll of
        // the frame before; so the slide is placed from its direction and the row's own scroll,
        // not from `$0C`. Once the game has chosen the next screen, `$63` is that screen, about
        // 17 frames before the slide starts, and keeps a nametable until its end: the one `$0D`
        // moves to, next to the one at rest (`$0D` moves on at the start of a slide to the left,
        // at the end of one to the right).
        let screens_slid = i32::from(memory.ram(SLIDE_SCREENS));
        let direction = memory.ram(SLIDE_DIRECTION);
        let slide_x = memory.ram(SLIDE_X);
        let (screen_nametable, side) = match direction {
            LEFT if slide_x != 0 => (screens_slid & 1, LEFT),
            LEFT | RIGHT => ((screens_slid + 1) & 1, direction),
            _ => (screens_slid & 1, AT_REST),
        };
        // The tile column, counted from the screen `$63`'s left edge, at the picture's left: the
        // picture shows `$63` and the screen it is slid in from, to its right for a slide to the
        // left (0 to 32 columns on), to its left for a slide to the right (0 to 32 columns back).
        let ring = ((row.ring_x() >> 3) - screen_nametable * 32).rem_euclid(64);
        let first = match side {
            LEFT => ring,
            RIGHT if ring > 0 => ring - 64,
            RIGHT => 0,
            _ => (ring + 32).rem_euclid(64) - 32,
        };

        let current = memory.ram(SCREEN);
        let mut cache: Vec<(i32, Option<Vec<u8>>)> = Vec::new();
        let mut tile_at = |world: i32| -> Option<(u8, u8)> {
            let step = world.div_euclid(32);
            if step.abs() > MAX_STEPS {
                return None;
            }
            if !cache.iter().any(|(s, _)| *s == step) {
                let screen = area.screen_at(memory, current, step);
                let blocks = screen.and_then(|screen| area.decode(screen));
                cache.push((step, blocks));
            }
            let blocks = cache.iter().find(|(s, _)| *s == step)?.1.as_ref()?;
            let column = world.rem_euclid(32);
            let index = usize::try_from((tile_row / 2) * 16 + column / 2).ok()?;
            let block = *blocks.get(index)?;
            area.tile(block, column & 1, tile_row & 1)
        };

        // The screen's edge columns must show what the level data gives there: column 1 for
        // the left, column 0 being the one a slide to the left draws after it shows (redrawn).
        for column in [1, 31] {
            let shown = (memory.nametable)(nametable_address(*row, column));
            if tile_at(first + column).map(|(tile, _)| tile) != Some(shown) {
                return false;
            }
        }
        for column in columns {
            if let Some((tile, palette)) = tile_at(first + column) {
                out.push(DecodedTile {
                    column,
                    tile,
                    palette,
                });
            }
        }
        true
    }

    /// A slide to the left shows the column it brings in a frame before drawing it.
    fn redrawn_left_columns(&self) -> i32 {
        1
    }
}

/// The PPU address of screen column `column` of `row` on the plane of two nametables.
fn nametable_address(row: BackgroundRow, column: i32) -> u16 {
    let ring = ((row.ring_x() >> 3) + column).rem_euclid(64);
    let nametable = u16::try_from(ring / 32).unwrap_or(0);
    let x = u16::try_from(ring % 32).unwrap_or(0);
    0x2000 | (nametable << 10) | (u16::from(row.coarse_y()) << 5) | x
}

/// The tables of the area being played.
struct Area<'a> {
    /// Bank 3.
    tables: &'a [u8],
    /// The bank of the area's screens.
    screens: &'a [u8],
    /// Offsets in `screens` of the table of the screens' pointers.
    screen_table: usize,
    /// Offsets in `tables` of the attributes, the four tile tables and the links.
    attributes: usize,
    tiles: [usize; 4],
    links: usize,
}

impl<'a> Area<'a> {
    fn new(memory: &DecoderMemory<'a>) -> Option<Self> {
        let tables = memory
            .prg_rom
            .get(AREA_BANK * BANK_SIZE..(AREA_BANK + 1) * BANK_SIZE)?;
        let bank = usize::from(memory.ram(SCREEN_BANK));
        let screens = memory
            .prg_rom
            .get(bank * BANK_SIZE..(bank + 1) * BANK_SIZE)?;
        let screen_table = (usize::from(memory.ram(SCREEN_TABLE))
            | (usize::from(memory.ram(SCREEN_TABLE + 1)) << 8))
            .checked_sub(0x8000)?;
        let areas = word(tables, 0)?;
        let header = word(tables, areas + 2 * usize::from(memory.ram(AREA)))?;
        let sets = word(tables, header)?;
        let links = word(tables, header + 4)?;
        let attributes = word(tables, sets)?;
        let tiles = [
            word(tables, sets + 2)?,
            word(tables, sets + 4)?,
            word(tables, sets + 6)?,
            word(tables, sets + 8)?,
        ];
        Some(Self {
            tables,
            screens,
            screen_table,
            attributes,
            tiles,
            links,
        })
    }

    /// The screen `step` screens right (left when negative) of `current`.
    fn screen_at(&self, memory: &DecoderMemory<'_>, current: u8, step: i32) -> Option<u8> {
        let screen = match step {
            0 => current,
            -1 => memory.ram(LINK_LEFT),
            1 => memory.ram(LINK_RIGHT),
            _ => {
                let side = usize::from(step > 0);
                let near = self.screen_at(memory, current, step - step.signum())?;
                *self.tables.get(self.links + 4 * usize::from(near) + side)?
            }
        };
        (screen != NONE).then_some(screen)
    }

    /// The 256 blocks of `screen`, as the game's loader lays them out.
    fn decode(&self, screen: u8) -> Option<Vec<u8>> {
        let pointer = word(self.screens, self.screen_table + 2 * usize::from(screen))?;
        let mut bits = Bits {
            data: self.screens,
            byte: pointer,
            bit: 0,
        };
        let mut blocks = vec![0u8; 256];
        for position in 0..256 {
            let code = bits.take(2)?;
            blocks[position] = if code == 3 {
                bits.take(8)?
            } else {
                let back = [1, 16, 17][usize::from(code)];
                blocks[(position + 256 - back) % 256]
            };
        }
        blocks[0xF0..].fill(0);
        Some(blocks)
    }

    /// The CHR tile and sub-palette of corner (`x`, `y`) of `block`.
    fn tile(&self, block: u8, x: i32, y: i32) -> Option<(u8, u8)> {
        let corner = usize::try_from(y * 2 + x).ok()?;
        let tile = *self.tables.get(self.tiles[corner] + usize::from(block))?;
        let attribute = *self.tables.get(self.attributes + usize::from(block))?;
        Some((tile, attribute & 3))
    }
}

/// The word at `offset` of a bank, as an offset in the bank (the game's pointers are relative
/// to `$8000`).
fn word(bank: &[u8], offset: usize) -> Option<usize> {
    let low = usize::from(*bank.get(offset)?);
    let high = usize::from(*bank.get(offset + 1)?);
    Some(((high << 8) | low) & (BANK_SIZE - 1))
}

/// A bit reader, most significant bit first.
struct Bits<'a> {
    data: &'a [u8],
    byte: usize,
    bit: u8,
}

impl Bits<'_> {
    fn take(&mut self, count: u8) -> Option<u8> {
        let mut value = 0u8;
        for _ in 0..count {
            let byte = *self.data.get(self.byte)?;
            value = (value << 1) | ((byte >> (7 - self.bit)) & 1);
            self.bit += 1;
            if self.bit == 8 {
                self.bit = 0;
                self.byte += 1;
            }
        }
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screens_copy_left_above_or_read_a_block() {
        // Block 5 (3, then 8 bits), then copy left (0) for the rest of the row, then copy above
        // (1) from position 16 on: every position is block 5.
        let mut data = vec![0u8; 0x200];
        // Pointer of screen 0 at offset 0: data at offset 2.
        data[0] = 0x02;
        data[1] = 0x00;
        // 11 0000 0101, then 15 x "00", then 1 x "01"...: build it bit by bit.
        let mut bits: Vec<u8> = vec![1, 1, 0, 0, 0, 0, 0, 1, 0, 1];
        bits.extend(std::iter::repeat_n(0, 15 * 2));
        for _ in 16..256 {
            bits.extend([0, 1]);
        }
        for (index, bit) in bits.iter().enumerate() {
            data[2 + index / 8] |= bit << (7 - index % 8);
        }
        let area = Area {
            tables: &[],
            screens: &data,
            screen_table: 0,
            attributes: 0,
            tiles: [0; 4],
            links: 0,
        };
        let blocks = area.decode(0).unwrap_or_default();
        assert_eq!(blocks.len(), 256);
        assert!(blocks[..0xF0].iter().all(|&block| block == 5));
        assert!(blocks[0xF0..].iter().all(|&block| block == 0));
    }
}
