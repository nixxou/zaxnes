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
//! nametables'. The margins show the screens beside the camera's, as far as a join of two
//! screens that are not side by side (see [`List`]: the screen before or after in the list may be
//! above or below, or behind a door to another screen); past it nothing is drawn. `$71`, which
//! lets the camera on past its screen's edges (bit 0 right, bit 1 left), only says where it may
//! go now: a screen beside, past a lock, shows.

use super::{megaman, BackgroundRow, DecodedTile, DecoderMemory, MarginDecoder};

/// The decoder.
pub struct MegaMan6;

/// CRC-32 of the ROM's data, iNES header excluded.
pub const SOURCE_CRC: u32 = 0x9887_98A8;

const CAMERA_X_LOW: u16 = 0x0056;
const CAMERA_X_HIGH: u16 = 0x0057;
const CAMERA_Y_LOW: u16 = 0x0058;
const CAMERA_Y_HIGH: u16 = 0x0059;
/// The vertical scroll's nametable (bit 0), as the NMI sets $2000 from it.
const CAMERA_Y_SCROLL: u16 = 0x00FA;
const MAP: u16 = 0x0052;
const TILES: u16 = 0x0062;
const STRUCTURES: u16 = 0x0066;
const BANK_8000: u16 = 0x006E;
const BANK_A000: u16 = 0x006F;
const LIST_SCREENS: u16 = 0x0074;
const LIST_XS: u16 = 0x0078;
const LIST_YS: u16 = 0x007C;
const LIST_TYPES: u16 = 0x007E;

/// The game's raster effect: on, its first interrupt handler, the line of the first interrupt.
const RASTER: u16 = 0x00F4;
const RASTER_HANDLERS: u16 = 0x0677;
const RASTER_LINE: u16 = 0x0672;
/// The sunset's handlers, and the lines its band of sky takes (2, then 20 times 3).
const SUNSET_HANDLERS: u8 = 0x0E;
const SUNSET_LINES: i32 = 62;

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
        // The screens side by side with the camera's: as far as the next join of the stage's
        // list that is not (see `List::apart`), the margins showing at most one screen past the
        // two the camera may straddle.
        let screen = camera >> 8;
        let list = List::new(memory);
        let mut first = screen;
        if first > 0 && !list.apart(first - 1) {
            first -= 1;
        }
        let mut end_screen = screen + i32::from(camera & 0xFF != 0);
        if !list.apart(end_screen) {
            end_screen += 1;
        }
        let (first, end) = (first * 32, (end_screen + 1) * 32);
        // The sunset's band of sky, which the game scrolls apart (its raster effect), is a layer:
        // it goes on as the nametables do; so is a row the game shows from the other nametable
        // than the stage's (`$FA` bit 0, as its NMI sets $2000): the empty band its raster effect
        // opens above the submarine, pushing the stage's lower rows down as it rises and sinks
        // (the stage's layout there showed blocks of ice afloat beside it).
        if sky_band(memory).contains(&i32::from(row.coarse_y()))
            || row.nametable_y() != memory.ram(CAMERA_Y_SCROLL) & 1
        {
            megaman::decode_layer(memory, *row, columns, out);
            return true;
        }
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

/// The tile rows of the sunset's band of sky, when the game shows it: its raster effect on
/// (`$F4`), its interrupt handlers from 14 on (`$0677`, the table at `$C116`): from the line after
/// `$0672` the band's scroll is 0, then 0 or 1 every 3 lines 20 times (`$C2FF`, `$C31B`, the
/// shimmer), then the camera's again (`$C2F0`, `$06A7`). Empty otherwise.
fn sky_band(memory: &DecoderMemory<'_>) -> std::ops::Range<i32> {
    if memory.ram(RASTER) == 0 || memory.ram(RASTER_HANDLERS) != SUNSET_HANDLERS {
        return 0..0;
    }
    let line = i32::from(memory.ram(RASTER_LINE)) + 1;
    line / 8..(line + SUNSET_LINES + 7) / 8
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

/// The stage's list of objects (bank `$3A` at `$8000` while the game walks it): entry `i` is
/// screen `($74)[i]`, x `($78)[i]`, y `($7C)[i]`, type `($7E)[i]`, sorted by place, `$FF` past
/// its end.
///
/// A stage is a straight list of screens: whether screen `s + 1` is beside `s` or above or below
/// it, the map does not say. The stage's controllers (type `$80`, script `$824D` of bank `$39`)
/// do: each one's settings, entry `y` of the tables `$82EB` (low) and `$836B` (high) of bank `$39`,
/// are `$71` (or, after bit 7, a routine's address then `$71`), two bytes, then its actions, three
/// bytes each (kind, `$73`, `$72`) and a screen of their own with bit 7, up to a 0 or a byte with
/// bit 7: 1 and 3 Mega Man at the screen's bottom scroll down to the screen after or before it
/// (`$D583`, `$D53C`), 2 and 4 at a ladder's top scroll up to the next or previous (`$D60E`,
/// `$D5CD`), 5 and 6 at its right or left edge scroll across to a screen (`$D64F`, `$D6AF`). An
/// action without a screen of its own starts from the camera's, which rests on the controller's
/// screen while it acts; one that scrolls to the next screen (1, 2) makes the controller the
/// join's, and its scroll back (3, 4) acts from that next screen (the game takes the controller
/// along). Checked on movies of a stage against every change of screen they show.
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

    /// Entry `index`'s x, y and type; `None` past the list's end.
    fn entry(&self, index: u8) -> Option<(i32, u8, u8)> {
        let at = |list: u16| list.wrapping_add(u16::from(index));
        let screen = self.rom(LIST_BANK, at(self.screens))?;
        if screen == 0xFF {
            return None;
        }
        let x = self.rom(LIST_BANK, at(self.xs))?;
        let y = self.rom(LIST_BANK, at(self.ys))?;
        let kind = self.rom(LIST_BANK, at(self.types))?;
        Some(((i32::from(screen) << 8) | i32::from(x), y, kind))
    }

    /// The actions of settings `setting` without a screen of their own (kinds 1-6), and whether
    /// one scrolls to the next screen; `None` for settings not of a scroll controller (`$71`
    /// past 3, or a kind past 6: a boss's).
    fn actions(&self, setting: u8) -> Option<(Vec<(u8, bool)>, bool)> {
        let low = self.rom(CONTROLLER_BANK, 0x82EB + u16::from(setting))?;
        let high = self.rom(CONTROLLER_BANK, 0x836B + u16::from(setting))?;
        let start = u16::from(low) | (u16::from(high) << 8);
        let byte = |offset: u16| self.rom(CONTROLLER_BANK, start.wrapping_add(offset));
        let (scroll, mut at) = if byte(0)? & 0x80 == 0 {
            (byte(0)?, 3)
        } else {
            (byte(2)?, 5)
        };
        if scroll > 3 {
            return None;
        }
        let mut kinds = Vec::new();
        let mut forward = false;
        for _ in 0..16 {
            let kind = byte(at)?;
            if kind == 0 || kind & 0x80 != 0 {
                break;
            }
            if kind > 6 {
                return None;
            }
            let tiles = byte(at + 1)? & 0x80 != 0;
            at += 3;
            if byte(at)? & 0x80 != 0 {
                at += 1;
                continue;
            }
            forward |= matches!(kind, 1 | 2);
            kinds.push((kind, tiles));
        }
        Some((kinds, forward))
    }

    /// Whether screens `screen` and `screen + 1` are not side by side: a controller scrolls
    /// between them up or down, or leaves one by a door to another screen; or they do not show
    /// together: an action across their join loads the stage's other tiles (its `$73` with bit 7,
    /// a set of background tiles the game stores in `$064F`: Mega Man 6's machine rooms, whose
    /// door out swaps their tiles for the stage's, and the room drawn past the door showed as a
    /// jumble, a ghost of the machine).
    fn apart(&self, screen: i32) -> bool {
        for index in 0..=u8::MAX {
            let Some((x, y, kind)) = self.entry(index) else {
                return false;
            };
            if kind != CONTROLLER || !(4..=0xFE).contains(&(x & 0xFF)) {
                continue;
            }
            let Some((kinds, forward)) = self.actions(y) else {
                continue;
            };
            let own = x >> 8;
            let join = |kind: u8| match kind {
                1 | 2 | 5 => own,
                3 | 4 if forward => own,
                _ => own - 1,
            };
            if kinds
                .iter()
                .any(|&(kind, tiles)| (kind <= 4 || tiles) && join(kind) == screen)
            {
                return true;
            }
            if self.door(y, own, screen) {
                return true;
            }
        }
        false
    }

    /// Whether settings `setting` of a controller on screen `own` hold a door to another screen
    /// (5 or 6 with a screen of its own) across the join after `screen`.
    fn door(&self, setting: u8, own: i32, screen: i32) -> bool {
        let Some(low) = self.rom(CONTROLLER_BANK, 0x82EB + u16::from(setting)) else {
            return false;
        };
        let Some(high) = self.rom(CONTROLLER_BANK, 0x836B + u16::from(setting)) else {
            return false;
        };
        let start = u16::from(low) | (u16::from(high) << 8);
        let byte = |offset: u16| self.rom(CONTROLLER_BANK, start.wrapping_add(offset));
        let Some(first) = byte(0) else {
            return false;
        };
        let mut at = if first & 0x80 == 0 { 3 } else { 5 };
        for _ in 0..16 {
            let Some(kind) = byte(at) else {
                return false;
            };
            if kind == 0 || kind & 0x80 != 0 || kind > 6 {
                return false;
            }
            at += 3;
            if byte(at).is_some_and(|target| target & 0x80 != 0) {
                at += 1;
                if (kind == 5 && own == screen) || (kind == 6 && own - 1 == screen) {
                    return true;
                }
            }
        }
        false
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
    fn screens_apart_where_a_controller_scrolls_up_or_down_or_through_a_door() {
        // The list in bank $3A: screens at $8100, x at $8200, y at $8300, types at $8400.
        let mut prg = vec![0u8; 0x3B * BANK_SIZE];
        let list = |address: usize| LIST_BANK * BANK_SIZE + (address & 0x1FFF);
        // Controllers on screen 2 (setting 1: falls to the next screen, and back up its ladder),
        // 5 (setting 2: falls to the previous), 7 (setting 3: a door right to screen $20) and 9
        // (setting 4: $71 of 24, a boss's: left out) and 10 (setting 5: a door right loading
        // other tiles, a door left that does not), an enemy on screen 3, then the end.
        let entries: [(u8, u8, u8, u8); 6] = [
            (2, 0x80, 1, 0x80),
            (3, 0x10, 0, 0x14),
            (5, 0x40, 2, 0x80),
            (7, 0xC0, 3, 0x80),
            (9, 0x40, 4, 0x80),
            (10, 0x80, 5, 0x80),
        ];
        for (index, &(screen, x, setting, kind)) in entries.iter().enumerate() {
            prg[list(0x8100 + index)] = screen;
            prg[list(0x8200 + index)] = x;
            prg[list(0x8300 + index)] = setting;
            prg[list(0x8400 + index)] = kind;
        }
        prg[list(0x8106)] = 0xFF;
        // The settings in bank $39: $71, two bytes, then the actions.
        let settings = |address: usize| CONTROLLER_BANK * BANK_SIZE + (address & 0x1FFF);
        let all: [(usize, u16, &[u8]); 5] = [
            (1, 0x9000, &[0x00, 0, 0, 1, 0, 0, 4, 0, 0, 0]),
            (2, 0x9010, &[0x01, 0, 0, 3, 0, 0, 0]),
            (3, 0x9020, &[0x90, 0x00, 0x02, 0, 0, 5, 0, 0, 0xA0, 0]),
            (4, 0x9040, &[0x18, 0, 0, 1, 0, 0, 0]),
            (5, 0x9050, &[0x01, 0, 0, 5, 0x95, 0, 6, 0x00, 0, 0]),
        ];
        for (setting, at, bytes) in all {
            prg[settings(0x82EB + setting)] = (at & 0xFF) as u8;
            prg[settings(0x836B + setting)] = (at >> 8) as u8;
            let start = settings(usize::from(at));
            prg[start..start + bytes.len()].copy_from_slice(bytes);
        }
        let mut ram = vec![0u8; 0x800];
        for (zero_page, address) in [
            (0x74, 0x8100u16),
            (0x78, 0x8200),
            (0x7C, 0x8300),
            (0x7E, 0x8400),
        ] {
            ram[zero_page] = (address & 0xFF) as u8;
            ram[zero_page + 1] = (address >> 8) as u8;
        }
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
        let apart: Vec<i32> = (0..12).filter(|&screen| list.apart(screen)).collect();
        // 2|3 (down and back up, both the controller of 2's), 4|5 (down to the previous from 5),
        // 7|8 (the door to $20), 10|11 (a door loading other tiles); 9|10 not (a boss's settings,
        // and a door left without tiles of its own).
        assert_eq!(apart, [2, 4, 7, 10]);
    }
}
