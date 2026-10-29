//! Zelda II - The Adventure of Link (USA): the margins of the side view and of the world map,
//! decoded from the level data the game keeps in RAM.
//!
//! Adapted from the widescreen margins of z2rs (<https://github.com/troyedwardsjr/z2rs>,
//! `crates/z2-core/src/wide_margins.rs` and `overworld_map.rs`, MIT licence, Copyright (c) 2026
//! troyedwardsjr), which found where everything lives:
//!
//! * side view (game mode `$0B`): the area, 4 pages of 13 rows of 16 metatiles, at `$6000`
//!   (`page × $D0 + row × 16 + column`). A metatile code gives four CHR tiles through the
//!   pointer table at `$8500` of the area's PRG bank (`$0769`), column first (top left,
//!   bottom left, top right, bottom right), and its sub-palette is `code >> 6`. Level row `r`
//!   covers nametable rows `4 + 2r` and `5 + 2r`; rows 0-3 are the status bar;
//! * world map (game mode `$05`): one terrain nibble per 16 × 16 cell, run-length coded per
//!   map row in PRG-RAM, the game's own table of row starts at `$6000`.
//!
//! The position comes from the PPU (the row's place on the plane of nametables) and RAM only
//! says which instance of that wrapping plane is meant ([`super::resolve_ring`]): RAM is read
//! once the frame has run and may already be a step ahead of the scroll the frame used.

use super::{
    resolve_ring, BackgroundRow, DecodedTile, DecoderMemory, MarginDecoder, MaskingSprites,
};

/// The decoder.
pub struct Zelda2;

const GAME_MODE: u16 = 0x0736;
const MODE_WORLD_MAP: u8 = 0x05;
const MODE_SIDE_VIEW: u8 = 0x0B;
/// Pause or spell pane open (drawn into the nametables only).
const MENU: u16 = 0x0524;
/// PRG bank of the area, which holds the metatile tables.
const AREA_BANK: u16 = 0x0769;
/// Ground type: 0 fills the outside of the area with metatile `$42`, anything else `$40`.
const GROUND_TYPE: u16 = 0x010C;
const SCROLL_HIGH: u16 = 0x072A;
const SCROLL_LOW: u16 = 0x072C;
/// World map: Link's cell row (map row + `$1E`) and column.
const MAP_ROW: u16 = 0x0073;
const MAP_COLUMN: u16 = 0x0074;
/// World map: the first of the sprites hiding x 248-255 in the page the game copies to OAM.
const MASK_SPRITE: u16 = 0x0208;
/// World map: pixels left in the step under way, and its direction (1 right, 2 left).
const STEP_PIXELS: u16 = 0x007D;
const STEP_FACING: u16 = 0x0562;
/// World map region: 0 West, 1 Death Mountain and Maze Island, 2 East.
const REGION: u16 = 0x0706;

const METATILE_POINTERS: u16 = 0x8500;
const PAGE_SIZE: usize = 0xD0;
const LEVEL_ROWS: usize = 13;
const LEVEL_COLUMNS: usize = 16;
const LEVEL_TOP_ROW: u8 = 4;
/// Tile columns of an area: 4 pages of 32.
const AREA_TILE_COLUMNS: i32 = 128;
const BANK_SIZE: usize = 0x4000;

const MAP_WIDTH: i32 = 64;
const MAP_ROWS: usize = 75;
const MAP_ROWS_I32: i32 = 75;
const MAP_ROW_BASE: i32 = 0x1E;
/// The run-length map when the game's table of row starts cannot be used.
const MAP_RLE: usize = 0x7C00 - 0x6000;
const MAP_TABLE_LOW: usize = 0x7000;
const TERRAIN_MOUNTAIN: u8 = 0x0B;
const TERRAIN_WATER: u8 = 0x0C;

/// CHR tiles of each terrain, column first.
const TERRAIN_TILES: [[u8; 4]; 16] = [
    [0x5C, 0x5D, 0x5E, 0x5F], // town
    [0xF4, 0xF4, 0xF4, 0xF4], // cave
    [0x60, 0x61, 0x62, 0x63], // palace
    [0x5A, 0x5A, 0x5B, 0x5B], // bridge
    [0x6C, 0x6C, 0x6C, 0x6C], // desert
    [0x6D, 0x6D, 0x6D, 0x6D], // grass
    [0x68, 0x69, 0x6A, 0x6B], // forest
    [0x6F, 0x6F, 0x6F, 0x6F], // swamp
    [0x70, 0x71, 0xFE, 0xFE], // graveyard
    [0xFE, 0xFE, 0xFE, 0xFE], // road
    [0x6E, 0x6E, 0x6E, 0x6E], // lava
    [0x64, 0x65, 0x66, 0x67], // mountain
    [0x6E, 0x6E, 0x6E, 0x6E], // water
    [0x6E, 0x6E, 0x6E, 0x6E], // walkable water
    [0x56, 0x57, 0x58, 0x59], // rock
    [0x40, 0x41, 0x42, 0x43], // spider
];

/// Sub-palette of each terrain.
const TERRAIN_PALETTES: [u8; 16] = [2, 1, 2, 1, 3, 0, 0, 0, 1, 1, 1, 1, 3, 3, 1, 1];

impl MarginDecoder for Zelda2 {
    fn decode_row(
        &self,
        memory: &DecoderMemory<'_>,
        row: &BackgroundRow,
        columns: &mut dyn Iterator<Item = i32>,
        out: &mut Vec<DecodedTile>,
    ) -> bool {
        match memory.ram(GAME_MODE) {
            MODE_SIDE_VIEW => side_view(memory, *row, columns, out),
            MODE_WORLD_MAP => world_map(memory, *row, columns, out),
            _ => false,
        }
    }

    /// The world map only: the side view is as tall as the screen.
    fn decode_extra_row(
        &self,
        memory: &DecoderMemory<'_>,
        row: &BackgroundRow,
        columns: &mut dyn Iterator<Item = i32>,
        out: &mut Vec<DecodedTile>,
    ) -> bool {
        memory.ram(GAME_MODE) == MODE_WORLD_MAP && world_map(memory, *row, columns, out)
    }

    /// The world map hides x 248-255, the column it draws as it goes, under 15 sprites (`$877C`:
    /// sprites 2-16, tile `$FF`, x 248) that the wide picture would show as a band down its
    /// middle. The widescreen hack puts them out of sight itself (their y `$F8`).
    fn masking_sprites(&self, memory: &DecoderMemory<'_>) -> Option<MaskingSprites> {
        (memory.ram(GAME_MODE) == MODE_WORLD_MAP
            && memory.ram(MASK_SPRITE + 1) == 0xFF
            && memory.ram(MASK_SPRITE + 3) == 0xF8
            && memory.ram(MASK_SPRITE) < 0xEF)
            .then_some(MaskingSprites {
                x: 248,
                first: 2,
                last: 16,
            })
    }
}

/// Byte at CPU `address` with PRG bank `bank` at `$8000` and the last bank at `$C000`.
fn prg_read(prg: &[u8], bank: u8, address: u16) -> u8 {
    let offset = match address {
        0x8000..=0xBFFF => usize::from(bank) * BANK_SIZE + usize::from(address - 0x8000),
        0xC000..=0xFFFF => match prg.len().checked_sub(BANK_SIZE) {
            Some(last) => last + usize::from(address - 0xC000),
            None => return 0,
        },
        _ => return 0,
    };
    prg.get(offset).copied().unwrap_or(0)
}

fn side_view(
    memory: &DecoderMemory<'_>,
    row: BackgroundRow,
    columns: &mut dyn Iterator<Item = i32>,
    out: &mut Vec<DecodedTile>,
) -> bool {
    let coarse_y = row.coarse_y();
    if memory.ram(MENU) != 0 || !(LEVEL_TOP_ROW..30).contains(&coarse_y) {
        return false;
    }

    let estimate = (i32::from(memory.ram(SCROLL_HIGH)) << 8) | i32::from(memory.ram(SCROLL_LOW));
    let first_column = resolve_ring(row.ring_x(), 512, estimate) >> 3;
    let bank = memory.ram(AREA_BANK);
    let fill = if memory.ram(GROUND_TYPE) == 0 {
        0x42
    } else {
        0x40
    };
    let relative = coarse_y - LEVEL_TOP_ROW;
    let level_row = usize::from(relative >> 1);
    let lower_half = u16::from(relative & 1);

    for column in columns {
        let world = first_column + column;
        let code = if (0..AREA_TILE_COLUMNS).contains(&world) && level_row < LEVEL_ROWS {
            #[allow(clippy::cast_sign_loss)] // 0..128.
            let world = world as usize;
            let offset = (world >> 5) * PAGE_SIZE
                + level_row * LEVEL_COLUMNS
                + ((world >> 1) & (LEVEL_COLUMNS - 1));
            memory.prg_ram.get(offset).copied().unwrap_or(fill)
        } else {
            fill
        };
        let group = code >> 6;
        let pointer = u16::from(prg_read(
            memory.prg_rom,
            bank,
            METATILE_POINTERS + (u16::from(group) * 2),
        )) | (u16::from(prg_read(
            memory.prg_rom,
            bank,
            METATILE_POINTERS + (u16::from(group) * 2) + 1,
        )) << 8);
        #[allow(clippy::cast_sign_loss)] // Masked to one bit.
        let right_half = (world & 1) as u16;
        let entry = pointer
            .wrapping_add(u16::from(code & 0x3F) * 4)
            .wrapping_add((right_half * 2) + lower_half);
        out.push(DecodedTile {
            column,
            tile: prg_read(memory.prg_rom, bank, entry),
            palette: group,
        });
    }
    true
}

fn world_map(
    memory: &DecoderMemory<'_>,
    row: BackgroundRow,
    columns: &mut dyn Iterator<Item = i32>,
    out: &mut Vec<DecodedTile>,
) -> bool {
    let coarse_y = row.coarse_y();
    let link_row = i32::from(memory.ram(MAP_ROW)) - MAP_ROW_BASE;
    // On the first frame of the mode Link is not placed yet: nothing to extend.
    if coarse_y >= 30 || !(0..MAP_ROWS_I32).contains(&link_row) {
        return false;
    }
    let Some(map) = WorldMap::new(memory) else {
        return false;
    };

    // Horizontally: Link's column, 8 cells from the left edge, moved by the step under way.
    let step = i32::from(memory.ram(STEP_PIXELS));
    let estimate = 16 * (i32::from(memory.ram(MAP_COLUMN)) - 8)
        + match memory.ram(STEP_FACING) {
            1 => -step,
            2 => step,
            _ => 0,
        };
    let ring_x = (i32::from(row.coarse_x()) << 3) | i32::from(row.fine_x);
    let first_column = resolve_ring(ring_x, 256, estimate) >> 3;

    // Vertically: the row's place in the two stacked nametables (30 cells), the instance
    // nearest to where Link's row puts this line.
    let stack = (i32::from(row.nametable_y()) * 30) + i32::from(coarse_y);
    let estimate = link_row - 7 + (row.line >> 4);
    let map_row = resolve_ring(stack >> 1, 30, estimate);
    let lower_half = usize::from(stack & 1 == 1);

    for column in columns {
        let world = first_column + column;
        let terrain = usize::from(map.terrain(world.div_euclid(2), map_row) & 0x0F);
        let right_half = usize::from(world & 1 == 1);
        out.push(DecodedTile {
            column,
            tile: TERRAIN_TILES[terrain][(right_half * 2) + lower_half],
            palette: TERRAIN_PALETTES[terrain] & 3,
        });
    }
    true
}

/// The run-length coded world map: per map row, a run of (length − 1) << 4 | terrain bytes.
struct WorldMap<'a> {
    runs: &'a [u8],
    rows: [usize; MAP_ROWS],
    region: u8,
}

impl<'a> WorldMap<'a> {
    fn new(memory: &DecoderMemory<'a>) -> Option<Self> {
        let prg_ram = memory.prg_ram;
        let region = memory.ram(REGION);
        let mut rows = [0usize; MAP_ROWS];

        // The game's own table of row starts at $6000, when all of it points into $7000-$7FFF
        // (a map moved elsewhere in PRG-RAM, as randomisers do, is then still read right).
        let address = |row: usize| -> Option<usize> {
            Some(
                usize::from(*prg_ram.get(2 * row)?)
                    | (usize::from(*prg_ram.get((2 * row) + 1)?) << 8),
            )
        };
        let table = (0..MAP_ROWS).all(|row| {
            address(row).is_some_and(|address| (MAP_TABLE_LOW..0x8000).contains(&address))
        });
        if table {
            for (row, start) in rows.iter_mut().enumerate() {
                *start = address(row)? - MAP_TABLE_LOW;
            }
            return Some(Self {
                runs: prg_ram.get(MAP_TABLE_LOW - 0x6000..0x2000)?,
                rows,
                region,
            });
        }

        // Otherwise the rows are found again in the map the game copies to $7C00: each row
        // stops at the run that would reach the 65th column.
        let runs = prg_ram.get(MAP_RLE..0x2000)?;
        let mut offset = 0;
        for start in &mut rows {
            *start = offset;
            let mut consumed = 0;
            while let Some(&byte) = runs.get(offset) {
                let length = usize::from(byte >> 4) + 1;
                if consumed + length >= 65 {
                    break;
                }
                consumed += length;
                offset += 1;
            }
        }
        Some(Self { runs, rows, region })
    }

    /// Terrain nibble of cell (`column`, `row`): mountains left of the map in the first two
    /// regions, water anywhere else outside it.
    fn terrain(&self, column: i32, row: i32) -> u8 {
        let Some(&start) = usize::try_from(row).ok().and_then(|row| self.rows.get(row)) else {
            return TERRAIN_WATER;
        };
        if column < 0 {
            return if self.region < 2 {
                TERRAIN_MOUNTAIN
            } else {
                TERRAIN_WATER
            };
        }
        if column >= MAP_WIDTH {
            return TERRAIN_WATER;
        }
        #[allow(clippy::cast_sign_loss)] // 0..64.
        let column = column as usize;
        let mut reached = 0;
        for &byte in self.runs.get(start..).unwrap_or_default() {
            reached += usize::from(byte >> 4) + 1;
            if reached > column {
                return byte & 0x0F;
            }
        }
        TERRAIN_WATER
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_prg_window_maps_the_area_bank_and_the_fixed_bank() {
        let mut prg = vec![0u8; 8 * BANK_SIZE];
        prg[(3 * BANK_SIZE) + 0x500] = 0x34;
        prg[(7 * BANK_SIZE) + 0x2AE0] = 0x77;
        assert_eq!(prg_read(&prg, 3, 0x8500), 0x34);
        assert_eq!(prg_read(&prg, 0, 0xEAE0), 0x77);
        assert_eq!(prg_read(&prg, 3, 0x6000), 0);
    }

    #[test]
    fn the_side_view_reads_the_level_ram() {
        let mut ram = vec![0u8; 0x800];
        ram[usize::from(GAME_MODE)] = MODE_SIDE_VIEW;
        ram[usize::from(GROUND_TYPE)] = 1;
        // Camera at the start of page 1 (world tile column 32).
        ram[usize::from(SCROLL_HIGH)] = 1;
        let mut prg_ram = vec![0u8; 0x2000];
        // Page 1, level row 2, metatile column 3: code $45 (group 1, entry 5).
        prg_ram[PAGE_SIZE + (2 * LEVEL_COLUMNS) + 3] = 0x45;
        let mut prg = vec![0u8; 8 * BANK_SIZE];
        // Group 1 table at $9000; entry 5 = [TL, BL, TR, BR] = [1, 2, 3, 4].
        prg[0x502] = 0x00;
        prg[0x503] = 0x90;
        prg[0x1000 + 20..0x1000 + 24].copy_from_slice(&[1, 2, 3, 4]);
        let memory = DecoderMemory {
            ram: &ram,
            prg_ram: &prg_ram,
            prg_rom: &prg,
            nametable: &|_| 0,
            nametable_written: &|_| 0,
        };
        // The row of level row 2's top, the left edge at pixel 256 of the plane.
        let row = BackgroundRow {
            line: 0,
            v: 0x0400 | (8 << 5),
            fine_x: 0,
        };
        let mut out = Vec::new();
        assert!(side_view(&memory, row, &mut [6, 7].into_iter(), &mut out));
        assert_eq!(out[0].tile, 1, "left half, top");
        assert_eq!(out[1].tile, 3, "right half, top");
        assert_eq!(out[0].palette, 1);
    }

    #[test]
    fn the_status_bar_and_other_modes_are_left_to_the_ppu() {
        let mut ram = vec![0u8; 0x800];
        ram[usize::from(GAME_MODE)] = MODE_SIDE_VIEW;
        let memory = DecoderMemory {
            ram: &ram,
            prg_ram: &[0; 0x2000],
            prg_rom: &vec![0; 8 * BANK_SIZE],
            nametable: &|_| 0,
            nametable_written: &|_| 0,
        };
        let status = BackgroundRow {
            line: 0,
            v: 2 << 5,
            fine_x: 0,
        };
        let mut out = Vec::new();
        assert!(!Zelda2.decode_row(&memory, &status, &mut (0..2), &mut out));
        let mut ram = ram.clone();
        ram[usize::from(GAME_MODE)] = 0x10;
        let memory = DecoderMemory {
            ram: &ram,
            ..memory
        };
        let level = BackgroundRow {
            v: 8 << 5,
            ..status
        };
        assert!(!Zelda2.decode_row(&memory, &level, &mut (0..2), &mut out));
    }

    #[test]
    fn the_world_maps_masking_column_is_reported_while_shown() {
        let mut ram = vec![0u8; 0x800];
        ram[usize::from(GAME_MODE)] = MODE_WORLD_MAP;
        let sprite = usize::from(MASK_SPRITE);
        ram[sprite..sprite + 4].copy_from_slice(&[0x10, 0xFF, 0x01, 0xF8]);
        let memory = DecoderMemory {
            ram: &ram,
            prg_ram: &[0; 0x2000],
            prg_rom: &vec![0; 8 * BANK_SIZE],
            nametable: &|_| 0,
            nametable_written: &|_| 0,
        };
        assert_eq!(
            Zelda2.masking_sprites(&memory),
            Some(MaskingSprites {
                x: 248,
                first: 2,
                last: 16
            })
        );
        // Out of sight (the widescreen hack), or the side view: none.
        let mut hidden = ram.clone();
        hidden[sprite] = 0xF8;
        assert_eq!(
            Zelda2.masking_sprites(&DecoderMemory {
                ram: &hidden,
                ..memory
            }),
            None
        );
        let mut side = ram.clone();
        side[usize::from(GAME_MODE)] = MODE_SIDE_VIEW;
        assert_eq!(
            Zelda2.masking_sprites(&DecoderMemory {
                ram: &side,
                ..memory
            }),
            None
        );
    }

    #[test]
    fn world_map_rows_decode_their_runs() {
        let mut prg_ram = vec![0u8; 0x2000];
        // No usable row table: the rows come from $7C00. Row 0: 3 cells of forest then 62 of
        // grass, 64 in all, then row 1 starts.
        prg_ram[MAP_RLE] = 0x26;
        prg_ram[MAP_RLE + 1] = 0xF5;
        prg_ram[MAP_RLE + 2] = 0xF5;
        prg_ram[MAP_RLE + 3] = 0xF5;
        prg_ram[MAP_RLE + 4] = 0xC5;
        prg_ram[MAP_RLE + 5] = 0x0C;
        let ram = vec![0u8; 0x800];
        let memory = DecoderMemory {
            ram: &ram,
            prg_ram: &prg_ram,
            prg_rom: &[],
            nametable: &|_| 0,
            nametable_written: &|_| 0,
        };
        let map = WorldMap::new(&memory).unwrap_or_else(|| unreachable!());
        assert_eq!(map.terrain(0, 0), 6);
        assert_eq!(map.terrain(3, 0), 5);
        assert_eq!(map.terrain(0, 1), TERRAIN_WATER);
        assert_eq!(map.terrain(-1, 0), TERRAIN_MOUNTAIN);
        assert_eq!(map.terrain(64, 0), TERRAIN_WATER);
    }
}
