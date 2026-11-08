//! Lot MF-LEG-2: the MMC5's extended attribute mode (`$5104` = %01) and its vertical split
//! (`$5200`-`$5202`), the two features of `docs/review/LEGACY-XCHECK.md` entry **MMC5-1**.
//!
//! Every test here drives a **rendered frame** rather than the board's methods: both features
//! are about what the PPU fetches, and the per-tile state they need — the `ExRAM` index of
//! mode 1, the split's tile counter and its row — is latched on the board's own bus while a
//! line is drawn. A test that called `Cartridge::read_nametable_from_vram` by hand would be
//! testing an expression, not the feature; a test that reads the picture back cannot pass
//! unless the PPU, the board and the two hooks between them agree.
//!
//! Every assertion cites the nesdev *MMC5* page. Where the page is silent or the five
//! cross-checked emulators disagree, the banner in `mmc5.rs` records the vote and the choice,
//! and the test says which reading it pins.
//!
//! **How the picture is read back.** The test image's 4 KiB CHR bank `b` holds, at tile `t`,
//! eight identical rows of `(b << 4) | (t & 0x0F)` in plane 0 and **zero** in plane 1, so every
//! background pixel is colour 0 (transparent) or colour 1 of its sub-palette. Palette entry 1
//! of sub-palette `p` is `$p1` and the universal backdrop is `$0F`, so one pixel says two
//! things at once: *which sub-palette* the tile was drawn with, and *which bit* of the fetched
//! pattern byte was set — plane-0 bit `n` being the pixel at `x = 7 - n` of the tile. That is
//! enough to name the bank, the tile and the palette of every tile on screen.
//!
//! The whole file is behind `board-mmc5-exattr-split`. The default build's side of the
//! decision — `$5200` ignored and mode 1 rendering like mode 0 — is `mapper_tests.rs`'s
//! `mf_leg_2_mmc5_*` pair, which runs in **both** configurations.
#![cfg(feature = "board-mmc5-exattr-split")]
// Tile numbers, bank numbers and pixel coordinates are all small.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_lossless,
    clippy::cast_sign_loss,
    clippy::expect_used
)]

mod common;

use common::{machine_with, TileFixture};
use zax_nes::ppu::{PIXEL_COLOR, SCREEN_WIDTH};
use zax_nes::NesCore;

/// 4 KiB CHR banks in the test image. Four is the fewest that tells "the window's bank", "the
/// next one" and "a bank only `ExRAM` can reach" apart.
const CHR_BANKS_4K: usize = 4;

/// The universal backdrop ($3F00), i.e. the colour of a transparent background pixel.
const BACKDROP: u8 = 0x0F;

/// A tile whose pattern byte has bits 0 to 3 set in **every** bank, so that the four pixels at
/// `x = 4..=7` of a column are opaque whatever bank it came from.
const OPAQUE_TILE: u8 = 0x0F;

/// Plane-0 byte of every row of tile `tile` of 4 KiB bank `bank`: the bank in the high nibble
/// and the tile in the low one, so one fetched byte names both.
const fn pattern_byte(bank: usize, tile: usize) -> u8 {
    ((bank as u8) << 4) | ((tile as u8) & 0x0F)
}

/// Screen x of plane-0 bit `bit` of the tile at screen column `column`.
const fn bit_x(column: usize, bit: usize) -> usize {
    (column * 8) + (7 - bit)
}

/// A mapper-5 image: 64 KiB of PRG and [`CHR_BANKS_4K`] 4 KiB banks of CHR laid out as the
/// module comment describes.
fn mmc5_image() -> Vec<u8> {
    const PRG_SIZE: usize = 4 * 16 * 1024;
    let chr_size = CHR_BANKS_4K * 4 * 1024;
    let mut image = vec![0u8; 16 + PRG_SIZE + chr_size];
    image[..4].copy_from_slice(b"NES\x1A");
    image[4] = (PRG_SIZE / (16 * 1024)) as u8;
    image[5] = (chr_size / (8 * 1024)) as u8;
    // Byte 6: bits 7-4 are the mapper's low four bits (5), bit 0 the vertical-mirroring flag.
    image[6] = 0x51;

    // `JMP $8000`, so the CPU parks and the test alone drives the registers (REQ-CORE-082).
    image[16..19].copy_from_slice(&[0x4C, 0x00, 0x80]);
    let vectors = 16 + PRG_SIZE - 6;
    for slot in 0..3 {
        image[vectors + 2 * slot] = 0x00;
        image[vectors + 2 * slot + 1] = 0x80;
    }

    let chr = 16 + PRG_SIZE;
    for bank in 0..CHR_BANKS_4K {
        for tile in 0..256usize {
            let offset = chr + (bank * 4096) + (tile * 16);
            for row in 0..8 {
                image[offset + row] = pattern_byte(bank, tile);
                // Plane 1 stays zero, so every pixel is colour 0 or colour 1: a pixel is either
                // the backdrop or entry 1 of its sub-palette, and nothing else.
            }
        }
    }

    image
}

/// The fixture: the image above, the PPU warm-up passed, and the palette the module comment
/// describes.
fn mmc5_fixture() -> TileFixture {
    let mut core = machine_with(&mmc5_image(), "Mmc5ExAttrSplit.nes");
    for _ in 0..3 {
        core.run_frame(0);
    }

    let mut fixture = TileFixture { core };
    fixture.fill_oam(0xFF);
    fixture.write_vram(0x3F00, &[BACKDROP]);
    for palette in 0..4u16 {
        // Entry 1 of sub-palette p is `$p1`, each a legal NES colour.
        fixture.write_vram(0x3F01 + (palette * 4), &[((palette as u8) << 4) | 0x01]);
    }
    fixture
}

/// The NES colour of the pixel at `(x, y)` of the frame just run (REQ-CORE-048).
fn pixel_at(core: &NesCore, x: usize, y: usize) -> u8 {
    core.pixel_frame_with_mask()[(y * SCREEN_WIDTH) + x] & PIXEL_COLOR
}

/// The sub-palette the **opaque** pixel at `(x, y)` was drawn with; panics on a transparent
/// one, so a test cannot read a palette out of the backdrop by accident.
fn palette_at(core: &NesCore, x: usize, y: usize) -> u8 {
    let pixel = pixel_at(core, x, y);
    assert_ne!(
        BACKDROP, pixel,
        "the pixel at ({x}, {y}) is transparent: it carries no palette"
    );
    assert_eq!(
        0x01,
        pixel & 0x0F,
        "the pixel at ({x}, {y}) is not entry 1 of a sub-palette"
    );
    pixel >> 4
}

/// Whether the pixel at `(x, y)` is opaque, i.e. plane-0 bit `7 - (x % 8)` of the pattern byte
/// the PPU fetched for that tile was set.
fn opaque_at(core: &NesCore, x: usize, y: usize) -> bool {
    pixel_at(core, x, y) != BACKDROP
}

/// Writes `values` into `ExRAM` at `$5C00 + offset`. `$5104` goes to **mode 2** for the writes
/// — the only mode in which the CPU may store a byte outside rendering (MR-5-2) — and then to
/// `mode`.
fn write_ex_ram(fixture: &mut TileFixture, offset: u16, values: &[u8], mode: u8) {
    fixture.core.write_memory(0x5104, 0x02);
    for (index, &value) in values.iter().enumerate() {
        fixture
            .core
            .write_memory(0x5C00 + offset + index as u16, value);
    }
    fixture.core.write_memory(0x5104, mode);
}

/// An `ExRAM` image for the split: `tile(row, column)` at each of the 30 x 32 cells, and an
/// attribute table of `$55` — palette 1 in all four quadrants, so a split tile is recognisable
/// by its palette alone.
fn split_ex_ram(tile: impl Fn(usize, usize) -> u8) -> [u8; 1024] {
    let mut ex_ram = [0u8; 1024];
    for row in 0..30usize {
        for column in 0..32usize {
            ex_ram[(row * 32) + column] = tile(row, column);
        }
    }
    for byte in &mut ex_ram[0x03C0..0x0400] {
        *byte = 0x55;
    }

    ex_ram
}

/// Puts the board in a state where a frame renders a plain background: CHR mode 3 with the
/// eight 1 KiB windows on banks 0 to 7 (4 KiB banks 0 and 1), all four nametables on CIRAM
/// page 0, the whole nametable one tile with palette 0, background on.
fn plain_background(fixture: &mut TileFixture, tile: u8) {
    for window in 0..8u16 {
        fixture.core.write_memory(0x5120 + window, window as u8);
    }
    fixture.core.write_memory(0x5101, 0x03); // CHR mode 3: eight 1 KiB windows.
    fixture.core.write_memory(0x5105, 0x00); // All four tables: CIRAM page 0.
    fixture.core.write_memory(0x5130, 0x00); // No global upper CHR bank bits.
    fixture.fill_vram(0x2000, 960, tile);
    fixture.fill_vram(0x23C0, 64, 0x00); // Attribute table: palette 0 everywhere.
    fixture.set_scroll(0, 0);
    fixture.set_control(0x00); // Background patterns at $0000.
    fixture.set_mask(0x0A); // Background on, left column shown.
}

/// Arms the split: `$5200` = enable | side | threshold, `$5201`, `$5202`, and an `ExRAM` whose
/// every cell is [`OPAQUE_TILE`].
fn arm_split(fixture: &mut TileFixture, side_right: bool, threshold: u8, scroll: u8, bank: u8) {
    write_ex_ram(fixture, 0, &split_ex_ram(|_, _| OPAQUE_TILE), 0x00);
    let control = 0x80 | (u8::from(side_right) << 6) | (threshold & 0x1F);
    fixture.core.write_memory(0x5200, control);
    fixture.core.write_memory(0x5201, scroll);
    fixture.core.write_memory(0x5202, bank);
}

// ----------------------------------------------------------------- extended attribute mode

/// nesdev ("MMC5"), `$5104` mode %01: "each byte of the MMC5's internal extended RAM is used to
/// enhance the background tile at the corresponding nametable address", as `AACC CCCC` whose
/// bits 7-6 "select palette to use with specified tile".
///
/// So the palette is per **tile**, not per 16x16 attribute quadrant: the four tiles below are
/// neighbours inside **one** quadrant, which no attribute byte could colour differently.
#[test]
fn mf_leg_2_mode_1_takes_the_palette_of_every_tile_from_ex_ram() {
    let mut fixture = mmc5_fixture();
    plain_background(&mut fixture, OPAQUE_TILE);
    write_ex_ram(&mut fixture, 0, &[0x00, 0x40, 0x80, 0xC0], 0x01);
    fixture.next_batch();

    for (column, expected) in [0u8, 1, 2, 3].into_iter().enumerate() {
        assert_eq!(
            expected,
            palette_at(&fixture.core, bit_x(column, 0), 4),
            "tile {column} of the top row takes palette {expected} from its own ExRAM byte"
        );
    }
}

/// The attribute table is not merely overridden where `ExRAM` says something: it is **not
/// consulted at all**. The table below says palette 3 everywhere and `ExRAM` says palette 0,
/// and palette 0 is what must show.
#[test]
fn mf_leg_2_mode_1_ignores_the_nametables_own_attribute_table() {
    let mut fixture = mmc5_fixture();
    plain_background(&mut fixture, OPAQUE_TILE);
    fixture.fill_vram(0x23C0, 64, 0xFF); // Palette 3 in every quadrant.
    fixture.set_scroll(0, 0); // The $2006 pair above moved `t`; put the scroll back.
    write_ex_ram(&mut fixture, 0, &[0x00; 64], 0x01);
    fixture.next_batch();

    assert_eq!(
        0,
        palette_at(&fixture.core, bit_x(3, 0), 4),
        "in mode 1 the attribute byte is replaced, not combined"
    );
}

/// The same `ExRAM` byte's other field: "bits 0-5 specified here are used for selecting a 4KB
/// CHR bank on a per-tile basis", and "the pattern fetches ignore the standard CHR banking
/// bits".
///
/// The eight 1 KiB windows are on banks 0 to 7, i.e. 4 KiB banks 0 and 1, so a tile that
/// renders from bank 2 or 3 can only have got there through `ExRAM`. Bank `b` of tile `$0F` has
/// pattern byte `(b << 4) | 0x0F`, so plane-0 bit 4 names banks 1 and 3 and bit 5 names banks 2
/// and 3 — the four banks told apart by two pixels.
#[test]
fn mf_leg_2_mode_1_takes_the_four_kib_chr_bank_of_every_tile_from_ex_ram() {
    let mut fixture = mmc5_fixture();
    plain_background(&mut fixture, OPAQUE_TILE);
    write_ex_ram(&mut fixture, 0, &[0x00, 0x01, 0x02, 0x03], 0x01);
    fixture.next_batch();

    for (column, bank) in [0usize, 1, 2, 3].into_iter().enumerate() {
        assert_eq!(
            bank & 1 != 0,
            opaque_at(&fixture.core, bit_x(column, 4), 4),
            "tile {column} renders from bank {bank}: plane-0 bit 4 of its pattern byte"
        );
        assert_eq!(
            bank & 2 != 0,
            opaque_at(&fixture.core, bit_x(column, 5), 4),
            "tile {column} renders from bank {bank}: plane-0 bit 5 of its pattern byte"
        );
    }
}

/// "The two bits in `$5130` are used globally as CHR bank bits 6 and 7."
///
/// Four 4 KiB banks is all the image has, so bit 6 adds 64 and the wrap brings every bank back
/// to itself: the honest assertion is that the picture does **not** move, which is what a bank
/// bit above the image's width must do — and what a build that folded `$5130` into the *low*
/// six bits would not do, there the banks shifting by two.
#[test]
fn mf_leg_2_mode_1_adds_the_two_global_bank_bits_of_5130() {
    let mut fixture = mmc5_fixture();
    plain_background(&mut fixture, OPAQUE_TILE);
    write_ex_ram(&mut fixture, 0, &[0x01, 0x02], 0x01);

    fixture.core.write_memory(0x5130, 0x00);
    fixture.next_batch();
    let without = fixture.core.pixel_frame_with_mask().to_vec();

    fixture.core.write_memory(0x5130, 0x01); // Bank bit 6: +64 banks.
    fixture.next_batch();

    assert_eq!(
        without,
        fixture.core.pixel_frame_with_mask(),
        "$5130 bit 6 is bank bit 6, so in a four-bank image it wraps to the same banks"
    );
}

/// "In Mode 1, nametable fetches are processed normally, and can come from CIRAM nametables,
/// fill mode, or even Expansion RAM, but attribute fetches are replaced by data from Expansion
/// RAM."
///
/// So the *tile* byte is the nametable's. The nametable here holds tile `$02` and `ExRAM` holds
/// `$3F` — palette 0, bank 63, which wraps to bank 3 of four — so the pattern byte must be
/// `$32`: bit 1 set, bit 0 clear. Tile `$3F` of any bank would have bit 0 set.
#[test]
fn mf_leg_2_mode_1_does_not_replace_the_nametable_tile_byte() {
    let mut fixture = mmc5_fixture();
    plain_background(&mut fixture, 0x02);
    write_ex_ram(&mut fixture, 0, &[0x3F; 64], 0x01);
    fixture.next_batch();

    assert!(
        !opaque_at(&fixture.core, bit_x(0, 0), 4),
        "the tile is the nametable's $02, whose pattern byte has bit 0 clear"
    );
    assert!(
        opaque_at(&fixture.core, bit_x(0, 1), 4),
        "and bit 1 set, which tile $3F would not have"
    );
}

/// "CHR mode always applies to sprite pattern fetches": mode 1 is a **background** mode, and a
/// sprite keeps the bank the windows give it and the palette its OAM attribute gives it.
#[test]
fn mf_leg_2_mode_1_leaves_sprites_alone() {
    let mut fixture = mmc5_fixture();
    plain_background(&mut fixture, 0x00); // A transparent background, so the sprite is alone.
                                          // Every ExRAM byte: palette 3, bank 3. A sprite that read one would show both.
    write_ex_ram(&mut fixture, 0, &[0xC3; 64], 0x01);
    // Entry 1 of sprite sub-palette 0 is $3F11, a colour no background sub-palette has.
    fixture.write_vram(0x3F11, &[0x2C]);
    fixture.set_sprite(0, 63, OPAQUE_TILE, 0x00, 64);
    fixture.set_mask(0x1E); // Background and sprites on.
    fixture.next_batch();

    assert_eq!(
        0x2C,
        pixel_at(&fixture.core, bit_x(8, 0), 64),
        "a sprite takes its palette from OAM, never from ExRAM"
    );
}

/// `$5104` modes %10 and %11 make `ExRAM` the CPU's RAM and the PPU cannot see it (MR-5-5), so
/// extended attributes are a mode-%01 feature: mode %10 must render exactly as mode %00 does.
#[test]
fn mf_leg_2_extended_attributes_are_mode_1_only() {
    let mut fixture = mmc5_fixture();
    plain_background(&mut fixture, OPAQUE_TILE);
    write_ex_ram(&mut fixture, 0, &[0x00, 0x40, 0x80, 0xC0], 0x00);

    fixture.next_batch();
    let mode0 = fixture.core.pixel_frame_with_mask().to_vec();

    fixture.core.write_memory(0x5104, 0x02);
    fixture.next_batch();
    let mode2 = fixture.core.pixel_frame_with_mask().to_vec();

    fixture.core.write_memory(0x5104, 0x01);
    fixture.next_batch();

    assert_eq!(mode0, mode2, "mode 2 renders as mode 0 does");
    assert_ne!(
        mode0,
        fixture.core.pixel_frame_with_mask(),
        "and mode 1 does not, or the assertion above proves nothing"
    );
    assert_eq!(
        1,
        palette_at(&fixture.core, bit_x(1, 0), 4),
        "mode 1 gives tile 1 the palette its ExRAM byte $40 names"
    );
}

// ------------------------------------------------------------------------- vertical split

/// nesdev ("MMC5"), `$5200`: "`ESxW WWWW`" — bit 7 enables, bit 6 is the side and bits 4-0 the
/// "vertical split threshold tile count". Side 0 puts the split region on the **left**.
///
/// The region test is `column < threshold` on the left: FCEUX (`mmc5.cpp:186-193`), Nestopia
/// (`NstBoardMmc5.cpp:856-857`) and Mesen (`MMC5.h:487-495`, a toggle at the delimiter that
/// starts inside at column 0) all three, with puNES alone using `<=` (`mapper_005.c:475`).
/// **Four to one**; this test pins the four, and so the threshold column itself is outside.
#[test]
fn mf_leg_2_the_split_covers_the_columns_left_of_the_threshold() {
    let mut fixture = mmc5_fixture();
    plain_background(&mut fixture, OPAQUE_TILE);
    arm_split(&mut fixture, false, 4, 0, 0);
    fixture.next_batch();

    for column in 0..4usize {
        assert_eq!(
            1,
            palette_at(&fixture.core, bit_x(column, 0), 4),
            "column {column} is left of threshold 4, so it is in the split region"
        );
    }

    assert_eq!(
        0,
        palette_at(&fixture.core, bit_x(4, 0), 4),
        "column 4 is the threshold itself, so it is outside the split (`<`, not `<=`)"
    );
    assert_eq!(
        0,
        palette_at(&fixture.core, bit_x(31, 0), 4),
        "and so is the last column"
    );
}

/// The same register's bit 6 set: "specifies the vertical split screen side (0:left; 1:right)",
/// the region then being `column >= threshold`.
#[test]
fn mf_leg_2_the_split_covers_the_columns_right_of_the_threshold() {
    let mut fixture = mmc5_fixture();
    plain_background(&mut fixture, OPAQUE_TILE);
    arm_split(&mut fixture, true, 28, 0, 0);
    fixture.next_batch();

    assert_eq!(
        0,
        palette_at(&fixture.core, bit_x(27, 0), 4),
        "column 27 is left of threshold 28, so it is outside a right-side split"
    );
    for column in 28..32usize {
        assert_eq!(
            1,
            palette_at(&fixture.core, bit_x(column, 0), 4),
            "column {column} is at or right of threshold 28, so it is in the split region"
        );
    }
}

/// The tile counter the page calls the "nametable fetch count (horizontally)": every column of
/// the split region reads the `ExRAM` cell of **its own** column, so an `ExRAM` whose cells
/// carry their column number makes the counter itself observable — a counter one ahead or one
/// behind would draw a neighbour's tile.
///
/// Column `c` holds tile `c & 0x0F`, whose pattern byte in bank 0 is `$0c`, so plane-0 bit `n`
/// is set exactly where bit `n` of `c` is: four pixels per column name the column the chip
/// thought it was on.
#[test]
fn mf_leg_2_the_split_reads_the_ex_ram_cell_of_its_own_column() {
    let mut fixture = mmc5_fixture();
    plain_background(&mut fixture, 0x00);
    arm_split(&mut fixture, false, 16, 0, 0);
    write_ex_ram(
        &mut fixture,
        0,
        &split_ex_ram(|_, column| (column & 0x0F) as u8),
        0x00,
    );
    fixture.next_batch();

    for column in 1..16usize {
        for bit in 0..4usize {
            assert_eq!(
                (column >> bit) & 1 == 1,
                opaque_at(&fixture.core, bit_x(column, bit), 4),
                "split column {column} draws ExRAM tile {column}: plane-0 bit {bit}"
            );
        }
    }
}

/// "MMC5 internal extended RAM is always used as the nametable in split screen mode": the tile
/// **and** its attribute come from `ExRAM`, the attribute from the attribute table of that same
/// nametable (Nestopia's `GetSpliterAttribute`, puNES `mapper_005.c:479-480`, Mesen
/// `MMC5.h:506-509`, the three agreeing to the expression).
///
/// The ordinary nametable is tile `$00` here — pattern byte `$00`, fully transparent — so every
/// opaque pixel inside the region came from `ExRAM`, and its palette 1 came from `ExRAM`'s own
/// attribute table rather than from the nametable's all-zero one.
#[test]
fn mf_leg_2_the_split_region_takes_its_tile_and_attribute_from_ex_ram() {
    let mut fixture = mmc5_fixture();
    plain_background(&mut fixture, 0x00);
    arm_split(&mut fixture, false, 8, 0, 0);
    fixture.next_batch();

    assert_eq!(
        1,
        palette_at(&fixture.core, bit_x(3, 0), 4),
        "inside the split: ExRAM's tile, drawn with ExRAM's own attribute table"
    );
    assert!(
        !opaque_at(&fixture.core, bit_x(9, 0), 4),
        "outside it the ordinary nametable's transparent tile $00 is drawn"
    );

    // And the attribute really is read per quadrant out of ExRAM: $3C0 is the byte covering
    // columns 0-3 of rows 0-3, and changing it alone moves those tiles and no others.
    write_ex_ram(&mut fixture, 0x03C0, &[0xAA], 0x00); // Palette 2 in all four quadrants.
    fixture.next_batch();
    assert_eq!(
        2,
        palette_at(&fixture.core, bit_x(1, 0), 4),
        "the split's attribute byte is ExRAM's $3C0-$3FF, indexed as a nametable's would be"
    );
    assert_eq!(
        1,
        palette_at(&fixture.core, bit_x(1, 0), 60),
        "and only the quadrant that byte covers moves"
    );
}

/// "`$5202`: all eight bits select a 4 KB CHR bank" — the whole split region's patterns come
/// from it, whatever the eight CHR windows say.
#[test]
fn mf_leg_2_the_split_region_takes_its_patterns_from_the_5202_bank() {
    let mut fixture = mmc5_fixture();
    plain_background(&mut fixture, 0x00);
    arm_split(&mut fixture, false, 8, 0, 0);

    for bank in 0..CHR_BANKS_4K {
        fixture.core.write_memory(0x5202, bank as u8);
        fixture.next_batch();
        assert_eq!(
            bank & 1 != 0,
            opaque_at(&fixture.core, bit_x(2, 4), 4),
            "$5202 = {bank}: tile $0F of that bank, plane-0 bit 4"
        );
        assert_eq!(
            bank & 2 != 0,
            opaque_at(&fixture.core, bit_x(2, 5), 4),
            "$5202 = {bank}: tile $0F of that bank, plane-0 bit 5"
        );
    }
}

/// "The MMC5 has a separate v-split scanline counter that resets to the vertical scrolling
/// value in `$5201` each v-blank": visible line L shows the region's row `$5201 + L`, modulo
/// 240.
///
/// puNES (`mapper_005.c:396-405`) and Nestopia (`NstBoardMmc5.cpp:681-692`) both do exactly
/// that; Mesen is one line later and FCEUX one earlier and to tile granularity. **Q-177**
/// records the disagreement; this test pins the page's reading and the two that follow it.
///
/// The observable is the `ExRAM` **row** the split reads: row R holds tile `R & 0x0F` in every
/// column, so the pattern byte names the row.
#[test]
fn mf_leg_2_the_split_region_scrolls_vertically_from_5201() {
    let mut fixture = mmc5_fixture();
    plain_background(&mut fixture, 0x00);
    arm_split(&mut fixture, false, 8, 0, 0);
    write_ex_ram(
        &mut fixture,
        0,
        &split_ex_ram(|row, _| (row & 0x0F) as u8),
        0x00,
    );

    // Scroll 0: line 4 is the region's row 4, in ExRAM row 0 — tile 0, pattern byte $00, which
    // is transparent throughout.
    fixture.core.write_memory(0x5201, 0x00);
    fixture.next_batch();
    for bit in 0..4usize {
        assert!(
            !opaque_at(&fixture.core, bit_x(1, bit), 4),
            "scroll 0, line 4: ExRAM row 0, tile 0, transparent at plane-0 bit {bit}"
        );
    }

    // Scroll 8: the region moves up a tile, so line 4 is its row 12, in ExRAM row 1 — tile 1,
    // whose pattern byte $01 has bit 0 set and nothing else.
    fixture.core.write_memory(0x5201, 0x08);
    fixture.next_batch();
    assert!(
        opaque_at(&fixture.core, bit_x(1, 0), 4),
        "scroll 8, line 4: ExRAM row 1, tile 1, plane-0 bit 0 set"
    );
    for bit in 1..4usize {
        assert!(
            !opaque_at(&fixture.core, bit_x(1, bit), 4),
            "and tile 1's pattern byte $01 has bit {bit} clear"
        );
    }

    // Scroll 16: ExRAM row 2, tile 2 — bit 1 and nothing else, the complement of the pair above.
    fixture.core.write_memory(0x5201, 0x10);
    fixture.next_batch();
    assert!(
        !opaque_at(&fixture.core, bit_x(1, 0), 4),
        "scroll 16, line 4: ExRAM row 2, tile 2, plane-0 bit 0 clear"
    );
    assert!(
        opaque_at(&fixture.core, bit_x(1, 1), 4),
        "and plane-0 bit 1 set"
    );
}

/// "...and the lowest 3 address bits of the pattern data": the split region's **fine** vertical
/// scroll, which Mesen substitutes (`MMC5.h:513`) and puNES (`:452`) and FCEUX (`:208`) do not,
/// scrolling their regions in whole tiles only. This build follows the page and Mesen; Q-177
/// records the split.
///
/// With `$5201` = 4 the region's rows 4 to 11 fall on screen lines 0 to 7, so the `ExRAM` row
/// boundary is crossed **inside** the first screen tile row: lines 0-3 take pattern rows 4-7 of
/// `ExRAM` row 0's tile and lines 4-7 pattern rows 0-3 of row 1's. A build that kept the PPU's
/// own low three bits would put that boundary at line 8.
#[test]
fn mf_leg_2_the_split_region_substitutes_the_patterns_low_three_address_bits() {
    let mut fixture = mmc5_fixture();
    plain_background(&mut fixture, 0x00);
    arm_split(&mut fixture, false, 8, 4, 0);
    // ExRAM row 0: tile 1 ($01, plane-0 bit 0 only). Row 1: tile 2 ($02, bit 1 only).
    write_ex_ram(
        &mut fixture,
        0,
        &split_ex_ram(|row, _| match row {
            0 => 0x01,
            1 => 0x02,
            _ => 0x00,
        }),
        0x00,
    );
    fixture.next_batch();

    for line in 0..4usize {
        assert!(
            opaque_at(&fixture.core, bit_x(1, 0), line),
            "line {line} is the region's row {}, in ExRAM row 0: tile 1, bit 0 set",
            line + 4
        );
    }
    for line in 4..8usize {
        assert!(
            opaque_at(&fixture.core, bit_x(1, 1), line),
            "line {line} is the region's row {}, in ExRAM row 1: tile 2, bit 1 set",
            line + 4
        );
        assert!(
            !opaque_at(&fixture.core, bit_x(1, 0), line),
            "and tile 2's pattern byte $02 has bit 0 clear"
        );
    }
}

/// `$5200` bit 7 clear: no split at all, and the frame is the one a build without the feature
/// draws.
#[test]
fn mf_leg_2_the_split_does_nothing_with_bit_7_clear() {
    let mut fixture = mmc5_fixture();
    plain_background(&mut fixture, OPAQUE_TILE);
    arm_split(&mut fixture, false, 8, 0, 0);

    fixture.next_batch();
    let armed = fixture.core.pixel_frame_with_mask().to_vec();

    fixture.core.write_memory(0x5200, 0x08); // Threshold 8, enable bit clear.
    fixture.next_batch();

    assert_ne!(
        armed,
        fixture.core.pixel_frame_with_mask(),
        "clearing bit 7 must change the picture, or the loop below proves nothing"
    );
    for column in 0..32usize {
        assert_eq!(
            0,
            palette_at(&fixture.core, bit_x(column, 0), 4),
            "column {column}: with $5200 bit 7 clear every tile is the ordinary background's"
        );
    }
}

/// The split reads `ExRAM` through the PPU, which only the two nametable modes allow: in modes
/// %10 and %11 the 1 KiB is the CPU's RAM. Mesen gates on `_extendedRamMode <= 1`
/// (`MMC5.h:480`) and FCEUX on `!(MMC5HackCHRMode & 2)` (`mmc5.cpp:181`) — the same test.
#[test]
fn mf_leg_2_the_split_is_off_in_the_two_cpu_ram_ex_ram_modes() {
    let mut fixture = mmc5_fixture();
    plain_background(&mut fixture, OPAQUE_TILE);
    arm_split(&mut fixture, false, 8, 0, 0);

    for mode in [0x02u8, 0x03] {
        fixture.core.write_memory(0x5104, mode);
        fixture.next_batch();
        for column in 0..8usize {
            assert_eq!(
                0,
                palette_at(&fixture.core, bit_x(column, 0), 4),
                "column {column}: in ExRAM mode {mode} the PPU cannot see ExRAM, so no split"
            );
        }
    }
}

/// The two composed, as the page describes them: "when using mode %01 with vertical split
/// mode, the non-split region gains extended attributes, and the split region does not use
/// extended attributes." Nestopia's `FetchNtExtSplit` (`NstBoardMmc5.cpp:938-955`) is that
/// sentence in code, and this build's `substituted_nametable_byte` tests the split first for
/// the same reason.
#[test]
fn mf_leg_2_the_split_region_does_not_take_extended_attributes() {
    let mut fixture = mmc5_fixture();
    plain_background(&mut fixture, OPAQUE_TILE);
    arm_split(&mut fixture, false, 4, 0, 0);
    // The same 1 KiB read two ways: inside the split, cell `c` is a tile number and the
    // attribute comes from $3C0-$3FF (palette 1); outside it, cell `c` is an `AACC CCCC` byte
    // whose bits 7-6 are the palette — and `OPAQUE_TILE` is `$0F`, so that palette is 0.
    fixture.core.write_memory(0x5104, 0x01);
    fixture.next_batch();

    assert_eq!(
        1,
        palette_at(&fixture.core, bit_x(0, 0), 4),
        "column 0 is in the split: its palette is ExRAM's attribute table's"
    );
    assert_eq!(
        0,
        palette_at(&fixture.core, bit_x(8, 0), 4),
        "column 8 is outside it: its palette is bits 7-6 of its own ExRAM byte"
    );

    // And the two fields really are read from the same byte: putting palette 3 in bits 7-6 of
    // the cells outside the split moves them and leaves the split's columns alone.
    write_ex_ram(
        &mut fixture,
        0,
        &split_ex_ram(|_, column| if column < 4 { OPAQUE_TILE } else { 0xCF }),
        0x01,
    );
    fixture.next_batch();
    assert_eq!(
        1,
        palette_at(&fixture.core, bit_x(0, 0), 4),
        "the split's columns still read their cell as a tile number"
    );
    assert_eq!(
        3,
        palette_at(&fixture.core, bit_x(8, 0), 4),
        "and the others read theirs as `AACC CCCC`"
    );
}

/// A `$2007` write to a nametable the board is answering must still reach CIRAM: with either
/// mode armed the MMC5 reports `SERVED_BY_CARTRIDGE` for all four tables, and
/// `Mapper::nametable_write_offset` is what keeps the console's VRAM reachable through it.
///
/// No cross-checked emulator has to think about this — their PPUs hold one pointer per table
/// rather than a page number — so the assertion is this build's own invariant, not the page's.
#[test]
fn mf_leg_2_a_nametable_write_still_reaches_ciram_while_the_board_answers() {
    let mut fixture = mmc5_fixture();
    plain_background(&mut fixture, 0x00);
    write_ex_ram(&mut fixture, 0, &[0x00; 64], 0x01); // Mode 1: all four tables answered.

    // The tile byte is the nametable's in mode 1, so a write to CIRAM must show on screen.
    fixture.write_vram(0x2000, &[OPAQUE_TILE]);
    fixture.next_batch();

    assert!(
        opaque_at(&fixture.core, bit_x(0, 0), 4),
        "a $2007 write to a CIRAM-backed table reaches CIRAM even while the MMC5 answers it"
    );
    assert!(
        !opaque_at(&fixture.core, bit_x(1, 0), 4),
        "and only the one byte written moves"
    );
}

// ------------------------------------------------------------------------------- savestate

/// `$5200`-`$5202` are registers a game writes once at setup, so they are in the board's
/// savestate block; the rest of the split's state is re-derived inside one line and is not
/// (REQ-CORE-091).
#[test]
fn mf_leg_2_the_split_registers_survive_a_savestate_round_trip() {
    let mut fixture = mmc5_fixture();
    plain_background(&mut fixture, 0x00);
    arm_split(&mut fixture, true, 19, 0x37, 0x02);
    fixture.next_batch();
    let before = fixture.core.pixel_frame_with_mask().to_vec();

    let state = fixture.core.save_state(false).expect("a savestate");

    fixture.core.write_memory(0x5200, 0x00);
    fixture.core.write_memory(0x5201, 0x00);
    fixture.core.write_memory(0x5202, 0x00);
    fixture.next_batch();
    assert_ne!(
        before,
        fixture.core.pixel_frame_with_mask(),
        "clearing the three registers must change the picture, or the test proves nothing"
    );

    fixture
        .core
        .try_load_state(&state, 0)
        .expect("the state loads back");
    fixture.next_batch();
    assert_eq!(
        before,
        fixture.core.pixel_frame_with_mask(),
        "$5200-$5202 come back with the state, and the frame is the one that was saved"
    );
}

// ------------------------------------------------------- the 3D view (REQ-CORE-037, SPEC-04)

/// Asserts that **every pixel of every background tile record agrees with the 2D picture**: the
/// records are what `zax-engine3d` builds its blocks from, so if a record's palette or pattern
/// differed from the frame the 3D view and the 2D picture would show different colours for the
/// same tile.
///
/// The check is pixel for pixel rather than palette for palette because the record carries both
/// halves of the answer — `TileRecord::palette` and the two bit planes — and the two substituting
/// modes move both: mode 1 the palette *and* the CHR bank, the split the tile itself *and* its
/// attribute. A per-palette assertion would pass on a record that had taken the right palette
/// from `ExRAM` and the wrong bank.
fn assert_records_agree_with_the_picture(fixture: &mut TileFixture, what: &str) {
    let batch = fixture.next_batch();
    let records = batch.background();
    assert_eq!(
        30 * 32,
        records.len(),
        "{what}: a full frame with fine X 0 emits 32 records a row for 30 rows (REQ-CORE-038)"
    );

    let mut opaque = 0usize;
    for record in &records {
        for py in 0..8usize {
            for px in 0..8usize {
                let x = i32::from(record.x) + px as i32;
                let y = i32::from(record.y) + py as i32;
                if !(0..256).contains(&x) || !(0..240).contains(&y) {
                    continue;
                }

                let pixel = record.displayed_pixel(px, py).expect("a tile position");
                // Plane 1 of the test image is zero, so a pixel is colour 0 or colour 1 and the
                // expected colour is the backdrop or entry 1 of the record's own sub-palette.
                let expected = if pixel == 0 {
                    BACKDROP
                } else {
                    (record.palette << 4) | 0x01
                };
                opaque += usize::from(pixel != 0);
                assert_eq!(
                    expected,
                    pixel_at(&fixture.core, x as usize, y as usize),
                    "{what}: the record at ({}, {}) disagrees with the picture at ({x}, {y})",
                    record.x,
                    record.y
                );
            }
        }
    }

    assert!(
        opaque > 1000,
        "{what}: {opaque} opaque pixels is too few for the comparison to mean anything"
    );
}

/// Extended attribute mode: the per-tile palette **and** the per-tile 4 KiB bank reach the tile
/// records, so a 3D block is coloured as the 2D tile under it.
///
/// The `ExRAM` image gives every tile of the screen its own palette and its own bank — the
/// palette cycling with the column and the bank with the row — so no two neighbouring blocks
/// agree by accident and a record that had read the wrong `ExRAM` cell would show.
#[test]
fn mf_leg_2_mode_1_reaches_the_tile_records_with_the_same_palette_and_bank() {
    let mut fixture = mmc5_fixture();
    plain_background(&mut fixture, OPAQUE_TILE);

    let mut ex_ram = [0u8; 1024];
    for row in 0..30usize {
        for column in 0..32usize {
            let palette = (column % 4) as u8;
            let bank = (row % CHR_BANKS_4K) as u8;
            ex_ram[(row * 32) + column] = (palette << 6) | bank;
        }
    }
    write_ex_ram(&mut fixture, 0, &ex_ram, 0x01);

    assert_records_agree_with_the_picture(&mut fixture, "ExRAM mode 1");
}

/// The vertical split: the region's tiles, their `ExRAM` attribute and `$5202`'s bank reach the
/// records too, so the 3D blocks of the split region show the split's artwork and not the
/// ordinary background's.
///
/// `$5201` is a multiple of 8 here, and that is the lot's one honest limitation: a record is a
/// whole 8x8 tile, so a region scrolled by a fraction of a tile straddles two `ExRAM` rows
/// inside one band and no single record can carry both. The region's **tile** scroll reaches the
/// records (the second half of this test moves it by one tile and checks the picture again); its
/// fine scroll does not, and `docs/progress/MF-LEG-2.md` step 2 records that.
#[test]
fn mf_leg_2_the_split_reaches_the_tile_records_with_its_own_tiles() {
    let mut fixture = mmc5_fixture();
    plain_background(&mut fixture, OPAQUE_TILE);
    arm_split(&mut fixture, false, 12, 0, 1);
    // Every ExRAM cell its own tile, so each column and each row of the region differs.
    write_ex_ram(
        &mut fixture,
        0,
        &split_ex_ram(|row, column| (((row + column) % 15) + 1) as u8),
        0x00,
    );

    assert_records_agree_with_the_picture(&mut fixture, "the split, $5201 = 0");

    fixture.core.write_memory(0x5201, 0x08);
    assert_records_agree_with_the_picture(&mut fixture, "the split, $5201 = 8");

    fixture.core.write_memory(0x5200, 0x80 | 0x40 | 0x14); // Right side, threshold 20.
    assert_records_agree_with_the_picture(&mut fixture, "the split, right side");
}

/// The two composed, which is the state *Bandit Kings of Ancient China* reaches: the records
/// must agree with the picture when a line is partly the split's and partly mode 1's.
#[test]
fn mf_leg_2_the_two_modes_composed_reach_the_tile_records() {
    let mut fixture = mmc5_fixture();
    plain_background(&mut fixture, OPAQUE_TILE);
    arm_split(&mut fixture, false, 8, 8, 2);
    // Cells 0-7 of each row are tile numbers for the split; the rest are `AACC CCCC` bytes for
    // mode 1, which reads the same 1 KiB the other way.
    write_ex_ram(
        &mut fixture,
        0,
        &split_ex_ram(|row, column| {
            if column < 8 {
                (((row + column) % 15) + 1) as u8
            } else {
                (((column % 4) as u8) << 6) | ((row % CHR_BANKS_4K) as u8)
            }
        }),
        0x01,
    );

    assert_records_agree_with_the_picture(&mut fixture, "mode 1 and the split together");
}

/// And the counterpart that makes the three above worth having: with neither mode armed the
/// records still agree, so the comparison is not passing because the capture and the picture
/// share a bug.
#[test]
fn mf_leg_2_the_records_agree_with_the_picture_with_neither_mode_armed() {
    let mut fixture = mmc5_fixture();
    plain_background(&mut fixture, OPAQUE_TILE);
    fixture.fill_vram(0x23C0, 64, 0xE4); // Palettes 0, 1, 2 and 3 across each quadrant row.
    fixture.set_scroll(0, 0);

    assert_records_agree_with_the_picture(&mut fixture, "neither mode armed");
}
