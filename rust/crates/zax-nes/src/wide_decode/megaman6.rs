//! Mega Man 6 (USA): the margins of a stage, decoded from its map in PRG-ROM.
//!
//! The game keeps where the stage's data is in RAM, as its column drawer (`$D1FB`) and the map
//! lookup (`$D8AB`, `$D900`) read it, with the 8 KiB banks `$6E` at `$8000` and `$6F` at `$A000`:
//!
//! * the map at `($52)`: one 256-byte page a row of 32 x 32 structures (8 rows a screen, row `y / 32`
//!   of the screen), the screen's 8 structures at `8 s` in the page, `s` the screen (the camera's
//!   screen x plus its screen y: a stage is a straight list of screens, going right or down);
//! * structure `n` is 4 bytes at `($66) + 4 n`, its four 16 x 16 blocks row first (top left, top
//!   right, bottom left, bottom right);
//! * block `b` is CHR tiles `($62)[b]` (top left), `($62)[256 + b]` (top right), `($62)[512 + b]`
//!   (bottom left), `($62)[768 + b]` (bottom right), in sub-palette `($62)[1024 + b] & 3`.
//!
//! The camera is `$57:$56` across, `$59:$58` down. Only rows the camera shows whole screens of
//! (its y on a screen's top) are decoded; while it scrolls down a shaft the margins stay the
//! nametables'. The camera stops at a screen's edge unless `$71` lets it on (bit 0 right, bit 1
//! left; objects of the stage set it as Mega Man goes): past such an edge the map holds another
//! part of the stage, and nothing is drawn. Where `$71` lets it on, it goes as far as the next
//! controller of the stage's list (see [`List`]) that will stop it when it spawns: the screen
//! before or after in the list may be above or below, not beside.

use super::{megaman, BackgroundRow, DecodedTile, DecoderMemory, MarginDecoder};

/// The decoder.
pub struct MegaMan6;

/// CRC-32 of the ROM's data, iNES header excluded.
pub const SOURCE_CRC: u32 = 0x9887_98A8;

const CAMERA_X_LOW: u16 = 0x0056;
const CAMERA_X_HIGH: u16 = 0x0057;
const CAMERA_Y_LOW: u16 = 0x0058;
const CAMERA_Y_HIGH: u16 = 0x0059;
const MAP: u16 = 0x0052;
const TILES: u16 = 0x0062;
const STRUCTURES: u16 = 0x0066;
const BANK_8000: u16 = 0x006E;
const BANK_A000: u16 = 0x006F;
const SCROLL: u16 = 0x0071;
const LIST_SCREENS: u16 = 0x0074;
const LIST_XS: u16 = 0x0078;
const LIST_YS: u16 = 0x007C;
const LIST_TYPES: u16 = 0x007E;
const LIST_RIGHT: u16 = 0x0649;
const LIST_LEFT: u16 = 0x064A;

const BANK_SIZE: usize = 0x2000;
/// The bank of the stage lists, at `$8000` while the game walks them.
const LIST_BANK: usize = 0x3A;
/// The bank of the controllers' script and settings, at `$8000` while it runs.
const CONTROLLER_BANK: usize = 0x39;
/// A controller's type in the stage list.
const CONTROLLER: u8 = 0x80;

impl MarginDecoder for MegaMan6 {
    fn decode_row(
        &self,
        memory: &DecoderMemory<'_>,
        row: &BackgroundRow,
        columns: &mut dyn Iterator<Item = i32>,
        out: &mut Vec<DecodedTile>,
    ) -> bool {
        if memory.ram(CAMERA_Y_LOW) != 0 {
            return false;
        }
        let camera =
            (i32::from(memory.ram(CAMERA_X_HIGH)) << 8) | i32::from(memory.ram(CAMERA_X_LOW));
        let screen_y = i32::from(memory.ram(CAMERA_Y_HIGH));
        // The camera goes on past its screen's edges only where $71 lets it, and as far as the
        // next controller of the stage's list that will stop it.
        let scroll = memory.ram(SCROLL);
        let screen = camera >> 8;
        let list = List::new(memory);
        let first = if scroll & 2 != 0 {
            list.left_stop().map_or(0, |stop| stop * 32)
        } else {
            screen * 32
        };
        let end = if scroll & 1 != 0 {
            list.right_stop().map_or(32 * 64, |stop| (stop + 2) * 32)
        } else {
            (screen + i32::from(camera & 0xFF != 0) + 1) * 32
        };
        let stage = Stage::new(memory);
        megaman::decode_row(
            memory,
            *row,
            columns,
            out,
            camera,
            first..end,
            &|world, tile_row| stage.tile_at(world, screen_y, tile_row),
        )
    }

    /// The game hides the screen's leftmost 8 pixels while it scrolls, and keeps there a column
    /// it is drawing.
    fn decodes_masked_column(&self) -> bool {
        true
    }

    /// The column the game draws past the screen's right edge takes its colours from attribute
    /// bytes it writes later: the screen's last column shows them first (a stripe of the next
    /// structure's colours at the edge of the original picture).
    fn redrawn_right_columns(&self) -> i32 {
        1
    }

    /// The game draws the column coming past the right edge into a single nametable's width
    /// (its 32 columns wrap), over the screen's first columns: the hidden 8 pixels cover column 0
    /// and, with a fine X, part of column 1, which showed that column's colours.
    fn redrawn_left_columns(&self) -> i32 {
        2
    }

    /// The game draws the rows coming in at the screen's top or bottom edge a piece at a time
    /// while it scrolls to another section (which a television hid): those 8 lines stay hidden
    /// with a vertical margin as without one, when the host hides them.
    fn hides_overscan(&self) -> bool {
        true
    }
}

/// Where the stage's data is, as the game keeps it in RAM.
struct Stage<'a> {
    memory: &'a DecoderMemory<'a>,
    map: u16,
    tiles: u16,
    structures: u16,
}

impl<'a> Stage<'a> {
    fn new(memory: &'a DecoderMemory<'a>) -> Self {
        let word = |at: u16| u16::from(memory.ram(at)) | (u16::from(memory.ram(at + 1)) << 8);
        Self {
            memory,
            map: word(MAP),
            tiles: word(TILES),
            structures: word(STRUCTURES) & !3,
        }
    }

    /// The byte at CPU address `address` of `$8000-$BFFF`, through the banks the drawer maps.
    fn read(&self, address: u16) -> Option<u8> {
        let bank = match address {
            0x8000..=0x9FFF => self.memory.ram(BANK_8000),
            0xA000..=0xBFFF => self.memory.ram(BANK_A000),
            _ => return None,
        };
        let offset = usize::from(bank) * BANK_SIZE + usize::from(address & 0x1FFF);
        self.memory.prg_rom.get(offset).copied()
    }

    /// CHR tile and sub-palette of world tile column `world`, tile row `tile_row` (0-29) of the
    /// screens at screen y `screen_y`.
    fn tile_at(&self, world: i32, screen_y: i32, tile_row: i32) -> Option<(u8, u8)> {
        if world < 0 || !(0..30).contains(&tile_row) {
            return None;
        }
        let screen = u16::try_from((world >> 5) + screen_y).ok()?;
        let column = u16::try_from(world & 31).ok()?;
        let row = u16::try_from(tile_row).ok()?;
        // The page of the structure row, the screen's 8 structures in it (wrapping in the page).
        let low = (self.map & 0xFF).wrapping_add(screen * 8 + (column >> 2)) & 0xFF;
        let page = (self.map & 0xFF00).wrapping_add((row >> 2) << 8);
        let structure = u16::from(self.read(page | low)?);
        let quarter = (((row >> 1) & 1) * 2) + ((column >> 1) & 1);
        let block = u16::from(self.read(self.structures.wrapping_add(structure * 4 + quarter))?);
        let corner = ((row & 1) * 2) + (column & 1);
        let tile = self.read(self.tiles.wrapping_add(corner * 0x100 + block))?;
        let palette = self.read(self.tiles.wrapping_add(0x400 + block))? & 3;
        Some((tile, palette))
    }
}

/// The stage's list of objects as the game walks it (bank `$3A` at `$8000`): entry `i` is screen
/// `($74)[i]`, x `($78)[i]`, y `($7C)[i]`, type `($7E)[i]`, sorted by place, `$FF` past both
/// ends; `$0649` is the next entry right of the spawn window, `$064A` the one past its left edge.
/// A controller (type `$80`, script `$824D` of bank `$39`) sets `$71` when it spawns, from its
/// settings: entry `y` of the tables `$82EB` (low) and `$836B` (high) of bank `$39`, whose first
/// byte is `$71`, or, with bit 7 set (a routine's address first), its third.
struct List<'a> {
    memory: &'a DecoderMemory<'a>,
    screens: u16,
    xs: u16,
    ys: u16,
    types: u16,
}

impl<'a> List<'a> {
    fn new(memory: &'a DecoderMemory<'a>) -> Self {
        let word = |at: u16| u16::from(memory.ram(at)) | (u16::from(memory.ram(at + 1)) << 8);
        Self {
            memory,
            screens: word(LIST_SCREENS),
            xs: word(LIST_XS),
            ys: word(LIST_YS),
            types: word(LIST_TYPES),
        }
    }

    fn rom(&self, bank: usize, address: u16) -> Option<u8> {
        let offset = bank * BANK_SIZE + usize::from(address & 0x1FFF);
        self.memory.prg_rom.get(offset).copied()
    }

    /// Entry `index`'s x and type; `None` past an end of the list.
    fn entry(&self, index: u8) -> Option<(i32, u8)> {
        let at = |list: u16| list.wrapping_add(u16::from(index));
        let screen = self.rom(LIST_BANK, at(self.screens))?;
        if screen == 0xFF {
            return None;
        }
        let x = self.rom(LIST_BANK, at(self.xs))?;
        let kind = self.rom(LIST_BANK, at(self.types))?;
        Some(((i32::from(screen) << 8) | i32::from(x), kind))
    }

    /// The `$71` controller `index` sets.
    fn scroll(&self, index: u8) -> Option<u8> {
        let setting = usize::from(self.rom(LIST_BANK, self.ys.wrapping_add(u16::from(index)))?);
        let low = self.rom(CONTROLLER_BANK, 0x82EB + u16::try_from(setting).ok()?)?;
        let high = self.rom(CONTROLLER_BANK, 0x836B + u16::try_from(setting).ok()?)?;
        let at = u16::from(low) | (u16::from(high) << 8);
        let first = self.rom(CONTROLLER_BANK, at)?;
        if first & 0x80 == 0 {
            Some(first)
        } else {
            self.rom(CONTROLLER_BANK, at.wrapping_add(2))
        }
    }

    /// Going left, the screen the camera stops on: that of the first controller left of the
    /// spawn window that does not let it on left (it spawns as the window's left edge, the
    /// camera + 8, reaches it). `None`: the stage's start.
    fn left_stop(&self) -> Option<i32> {
        let mut index = self.memory.ram(LIST_LEFT);
        while index > 0 {
            index -= 1;
            let (x, kind) = self.entry(index)?;
            if kind == CONTROLLER && self.scroll(index)? & 2 == 0 {
                return Some((x - 8).max(0) >> 8);
            }
        }
        None
    }

    /// Going right, the screen the camera last stops in: that of the first controller right of
    /// the spawn window that does not let it on right (it spawns as the window's right edge, the
    /// camera + 254, reaches it; the camera then goes on to its screen's end). `None`: the map's.
    fn right_stop(&self) -> Option<i32> {
        let mut index = self.memory.ram(LIST_RIGHT);
        loop {
            let (x, kind) = self.entry(index)?;
            if kind == CONTROLLER && self.scroll(index)? & 1 == 0 {
                return Some((x - 254).max(0) >> 8);
            }
            index = index.checked_add(1)?;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_map_structures_and_blocks_through_the_rams_pointers() {
        // Banks 2 ($8000) and 3 ($A000); the map at $B000, the blocks at $9200, the structures
        // at $9700.
        let mut prg = vec![0u8; 4 * BANK_SIZE];
        let at = |address: usize, bank: usize| bank * BANK_SIZE + (address & 0x1FFF);
        // Screen 1, structure row 3, column 2: structure 5.
        prg[at(0xB30A, 3)] = 5;
        // Structure 5: blocks TL 1, TR 2, BL 3, BR 4.
        prg[at(0x9714, 2)..at(0x9718, 2)].copy_from_slice(&[1, 2, 3, 4]);
        // Block 3: tiles $30-$33, sub-palette 2.
        for corner in 0..4 {
            prg[at(0x9203 + corner * 0x100, 2)] = 0x30 + u8::try_from(corner).unwrap();
        }
        prg[at(0x9603, 2)] = 0x42;
        let mut ram = vec![0u8; 0x800];
        ram[0x52..0x54].copy_from_slice(&[0x00, 0xB0]);
        ram[0x62..0x64].copy_from_slice(&[0x00, 0x92]);
        ram[0x66..0x68].copy_from_slice(&[0x00, 0x97]);
        ram[0x6E] = 2;
        ram[0x6F] = 3;
        let nametable = |_| 0;
        let written = |_| 0;
        let memory = DecoderMemory {
            ram: &ram,
            prg_ram: &[],
            prg_rom: &prg,
            nametable: &nametable,
            nametable_written: &written,
        };
        let stage = Stage::new(&memory);
        // World tile column 32 + 8 is structure column 2 of screen 1; tile row 14 is structure
        // row 3, its lower half.
        assert_eq!(stage.tile_at(40, 0, 14), Some((0x30, 2)), "top left");
        assert_eq!(stage.tile_at(41, 0, 14), Some((0x31, 2)), "top right");
        assert_eq!(stage.tile_at(40, 0, 15), Some((0x32, 2)), "bottom left");
        assert_eq!(stage.tile_at(41, 0, 15), Some((0x33, 2)), "bottom right");
        // The screen below is the next one of the list.
        assert_eq!(stage.tile_at(8, 1, 14), Some((0x30, 2)), "screen y 1");
        assert_eq!(stage.tile_at(-1, 0, 0), None);
    }

    #[test]
    fn the_camera_stops_at_the_next_controller_that_does_not_let_it_on() {
        // The list in bank $3A: screens at $8100, x at $8200, y at $8300, types at $8400.
        let mut prg = vec![0u8; 0x3B * BANK_SIZE];
        let list = |address: usize| LIST_BANK * BANK_SIZE + (address & 0x1FFF);
        // Entries: an enemy on screen 2, a controller on screen 3 at x $40 (setting 1: $71 = 1,
        // right only), the camera, a controller on screen 6 at x $80 (setting 2: through a
        // routine's address, $71 = 2), one on screen 8 (setting 3: $71 = 3), then the end.
        let entries: [(u8, u8, u8, u8); 4] = [
            (2, 0x10, 0, 0x14),
            (3, 0x40, 1, 0x80),
            (6, 0x80, 2, 0x80),
            (8, 0x00, 3, 0x80),
        ];
        prg[list(0x8100)] = 0xFF;
        for (index, &(screen, x, setting, kind)) in entries.iter().enumerate() {
            prg[list(0x8101 + index)] = screen;
            prg[list(0x8201 + index)] = x;
            prg[list(0x8301 + index)] = setting;
            prg[list(0x8401 + index)] = kind;
        }
        prg[list(0x8105)] = 0xFF;
        // The settings in bank $39.
        let settings = |address: usize| CONTROLLER_BANK * BANK_SIZE + (address & 0x1FFF);
        for (setting, at, bytes) in [
            (1usize, 0x9000u16, &[0x01u8][..]),
            (2, 0x9010, &[0x90, 0x00, 0x02]),
            (3, 0x9020, &[0x03]),
        ] {
            prg[settings(0x82EB + setting)] = (at & 0xFF) as u8;
            prg[settings(0x836B + setting)] = (at >> 8) as u8;
            let start = settings(usize::from(at));
            prg[start..start + bytes.len()].copy_from_slice(bytes);
        }
        let mut ram = vec![0u8; 0x800];
        for (zero_page, address) in [
            (0x74, 0x8101u16),
            (0x78, 0x8201),
            (0x7C, 0x8301),
            (0x7E, 0x8401),
        ] {
            ram[zero_page] = (address & 0xFF) as u8;
            ram[zero_page + 1] = (address >> 8) as u8;
        }
        // The window between entries 1 and 2: $064A = 2 (entries 0-1 left), $0649 = 2.
        ram[0x64A] = 2;
        ram[0x649] = 2;
        let nametable = |_| 0;
        let written = |_| 0;
        let memory = DecoderMemory {
            ram: &ram,
            prg_ram: &[],
            prg_rom: &prg,
            nametable: &nametable,
            nametable_written: &written,
        };
        let list = List::new(&memory);
        // Left: the controller of screen 3 lets the camera on right only: it stops on the screen
        // of $340 - 8.
        assert_eq!(list.left_stop(), Some(3));
        // Right: that of screen 6 does not let it on right: the camera last stops in the screen
        // of $680 - 254 (screen 5).
        assert_eq!(list.right_stop(), Some(5));
        // Past the controllers that let it on, the list's ends.
        let mut past = ram.clone();
        past[0x64A] = 1;
        past[0x649] = 4;
        let memory = DecoderMemory {
            ram: &past,
            ..memory
        };
        let list = List::new(&memory);
        assert_eq!(list.left_stop(), None);
        assert_eq!(list.right_stop(), None);
    }
}
