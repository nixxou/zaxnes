//! Lot MF-LEG-3: the MMC5's two substituting modes are **armed at run time**.
//!
//! A game cannot reach extended attribute mode or the vertical split without writing `$5104`
//! with mode %01 or `$5200` with bit 7 set, and both writes go through the board. So the
//! cartridge is created on `Board::Mmc5` — `04731c86`'s MMC5, to the instruction, which is what
//! *Castlevania III* runs — and the write that arms a mode replaces it with `Board::Mmc5Ext`,
//! every register and memory carried across. The owner's decision of 2026-11-09, taken on
//! MF-LEG-2's figure of **+2.992 %** for a game that uses neither mode.
//!
//! **The variant is read off the cartridge and not inferred**, because the switch is designed to
//! be invisible from outside: no register is lost by a board that has not switched (the banner in
//! `mmc5.rs` says why `$5201` and `$5202` arm it as well), so "did it switch?" cannot be answered
//! by looking at the picture. `NesCore::cartridge_mut` and `Cartridge::board` are both public and
//! answer it exactly.
//!
//! What the two modes *do* once armed is lot MF-LEG-2's file beside this one — twenty-three tests,
//! every one of them driving a rendered frame, and every one of them now reaching the full
//! variant through this switch.
#![cfg(feature = "board-mmc5-exattr-split")]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::expect_used
)]

mod common;

use common::{machine_with, TileFixture};
use zax_nes::cartridges::Board;
use zax_nes::ppu::{SCREEN_HEIGHT, SCREEN_WIDTH};
use zax_nes::NesCore;

/// 4 KiB CHR banks in the test image, as MF-LEG-2's: four is the fewest that tells "the window's
/// bank", "the next one" and "a bank only `ExRAM` can reach" apart.
const CHR_BANKS_4K: usize = 4;

/// The universal backdrop ($3F00), i.e. the colour of a transparent background pixel.
const BACKDROP: u8 = 0x0F;

/// A tile whose pattern byte has bits 0 to 3 set in **every** bank, so that the four pixels at
/// `x = 4..=7` of a column are opaque whatever bank it came from.
const OPAQUE_TILE: u8 = 0x0F;

/// `$5203`'s target in the mid-frame test: the scanline the board's own IRQ flag is raised on.
const ARMING_TARGET: u8 = 60;

/// The visible line the arming write lands on, **measured** rather than derived: the flag is
/// raised at the detection point of the line the counter matches on and the program's poll loop
/// takes a few CPU cycles more to see it and to store `$5200`. The test finds the line from the
/// picture and compares it with this, so a scanline detection that moved by a line would say so
/// here and not in the structure it asserts.
const ARMING_LINE: usize = ARMING_TARGET as usize;

/// `$5200` the mid-frame test arms: enabled, right side, threshold 16 — so the region is the
/// right half of the screen, whose tiles are fetched well after the write.
const ARMING_CONTROL: u8 = 0x80 | 0x40 | 16;

/// Plane-0 byte of every row of tile `tile` of 4 KiB bank `bank`.
const fn pattern_byte(bank: usize, tile: usize) -> u8 {
    ((bank as u8) << 4) | ((tile as u8) & 0x0F)
}

/// The 6502 program the mid-frame test needs, at `$E000` — the one window the MMC5 maps to ROM at
/// power-on (`prg_bank[3]` = `$FF`, bit 7 set), so it runs without the test having to map
/// anything first.
///
/// It waits on a flag in RAM, so that the fixture can do its own set-up with the CPU parked, then
/// polls `$5204`'s pending flag with the IRQ **disabled** and writes `$5200` the moment the
/// scanline counter reaches `$5203` — which is inside the visible dots of line
/// [`ARMING_LINE`], the worst case for the counters the light board does not keep.
fn arming_program() -> Vec<u8> {
    vec![
        0xAD,
        0x00,
        0x07, // $E000  LDA $0700      ; the test's go flag
        0xF0,
        0xFB, //       $E003  BEQ $E000
        0xA9,
        ARMING_TARGET, // $E005  LDA #target
        0x8D,
        0x03,
        0x52, // $E007  STA $5203
        0xA9,
        0x00, //       $E00A  LDA #$00       ; the IRQ line stays down; we poll the flag
        0x8D,
        0x04,
        0x52, // $E00C  STA $5204
        0xAD,
        0x04,
        0x52, // $E00F  LDA $5204      ; discard a flag set before the frame
        0xAD,
        0x04,
        0x52, // $E012  LDA $5204      ; poll
        0x10,
        0xFB, //       $E015  BPL $E012
        0xA9,
        ARMING_CONTROL, // $E017  LDA #control
        0x8D,
        0x00,
        0x52, // $E019  STA $5200      ; **mid-frame arming**
        0x4C,
        0x1C,
        0xE0, // $E01C  JMP $E01C
    ]
}

/// A mapper-5 image: 64 KiB of PRG, [`CHR_BANKS_4K`] 4 KiB banks of CHR laid out as MF-LEG-2's
/// file describes, and `program` in the last 8 KiB bank at `$E000` with all three vectors on it.
fn mmc5_image(program: &[u8]) -> Vec<u8> {
    const PRG_SIZE: usize = 4 * 16 * 1024;
    let chr_size = CHR_BANKS_4K * 4 * 1024;
    let mut image = vec![0u8; 16 + PRG_SIZE + chr_size];
    image[..4].copy_from_slice(b"NES\x1A");
    image[4] = (PRG_SIZE / (16 * 1024)) as u8;
    image[5] = (chr_size / (8 * 1024)) as u8;
    // Byte 6: bits 7-4 are the mapper's low four bits (5), bit 0 the vertical-mirroring flag.
    image[6] = 0x51;

    // The last 8 KiB bank is $E000-$FFFF at power-on, so the program is written there and the
    // vectors point at its first byte.
    let last_bank = 16 + PRG_SIZE - 8192;
    image[last_bank..last_bank + program.len()].copy_from_slice(program);
    let vectors = 16 + PRG_SIZE - 6;
    for slot in 0..3 {
        image[vectors + 2 * slot] = 0x00;
        image[vectors + 2 * slot + 1] = 0xE0;
    }

    let chr = 16 + PRG_SIZE;
    for bank in 0..CHR_BANKS_4K {
        for tile in 0..256usize {
            let offset = chr + (bank * 4096) + (tile * 16);
            for row in 0..8 {
                image[offset + row] = pattern_byte(bank, tile);
            }
        }
    }

    image
}

/// The fixture: the image above, the PPU warm-up passed, and MF-LEG-2's palette — entry 1 of
/// sub-palette `p` is `$p1`, so one pixel names the palette the tile was drawn with.
fn mmc5_fixture(program: &[u8]) -> TileFixture {
    let mut core = machine_with(&mmc5_image(program), "Mmc5RuntimeSwitch.nes");
    for _ in 0..3 {
        core.run_frame(0);
    }

    let mut fixture = TileFixture { core };
    fixture.fill_oam(0xFF);
    fixture.write_vram(0x3F00, &[BACKDROP]);
    for palette in 0..4u16 {
        fixture.write_vram(0x3F01 + (palette * 4), &[((palette as u8) << 4) | 0x01]);
    }
    fixture
}

/// A fixture whose CPU never leaves its wait loop: every register is the test's to write.
fn parked_fixture() -> TileFixture {
    mmc5_fixture(&arming_program())
}

/// Whether the cartridge is running the **full** variant, i.e. whether a write has armed one of
/// the two modes since the last power-on or reset.
fn is_extended(core: &mut NesCore) -> bool {
    matches!(
        core.cartridge_mut()
            .expect("the test ROM is loaded")
            .board(),
        Board::Mmc5Ext(_)
    )
}

/// Asserts the variant, with the write that was supposed to decide it in the message.
fn assert_variant(core: &mut NesCore, extended: bool, what: &str) {
    assert_eq!(
        extended,
        is_extended(core),
        "{what}: the cartridge should be running the {} MMC5",
        if extended { "full" } else { "light" }
    );
}

/// Puts the board in a state where a frame renders a plain background (MF-LEG-2's helper): CHR
/// mode 3 with the eight 1 KiB windows on banks 0 to 7, all four nametables on CIRAM page 0, the
/// whole nametable one tile with palette 0, background on.
fn plain_background(fixture: &mut TileFixture, tile: u8) {
    for window in 0..8u16 {
        fixture.core.write_memory(0x5120 + window, window as u8);
    }
    fixture.core.write_memory(0x5101, 0x03);
    fixture.core.write_memory(0x5105, 0x00);
    fixture.core.write_memory(0x5130, 0x00);
    fixture.fill_vram(0x2000, 960, tile);
    fixture.fill_vram(0x23C0, 64, 0x00);
    fixture.set_scroll(0, 0);
    fixture.set_control(0x00);
    fixture.set_mask(0x0A);
}

/// Fills `ExRAM` with `tile(row, column)` and an attribute table of `$55` — palette 1 in all four
/// quadrants, so a split tile is recognisable by its palette alone. `$5104` goes to **mode 2**
/// for the writes, the only mode in which the CPU may store a byte outside rendering (MR-5-2),
/// and then back to **mode 0**, which arms nothing.
fn fill_ex_ram(fixture: &mut TileFixture, tile: impl Fn(usize, usize) -> u8) {
    fixture.core.write_memory(0x5104, 0x02);
    for row in 0..30usize {
        for column in 0..32usize {
            let offset = (row * 32) + column;
            fixture
                .core
                .write_memory(0x5C00 + offset as u16, tile(row, column));
        }
    }
    for offset in 0x03C0..0x0400u16 {
        fixture.core.write_memory(0x5C00 + offset, 0x55);
    }
    fixture.core.write_memory(0x5104, 0x00);
}

/// The frame just run, as one row of NES colours per screen line.
fn rows(core: &NesCore) -> Vec<Vec<u8>> {
    let frame = core.pixel_frame_with_mask();
    (0..SCREEN_HEIGHT)
        .map(|y| {
            frame[y * SCREEN_WIDTH..(y + 1) * SCREEN_WIDTH]
                .iter()
                .map(|pixel| pixel & zax_nes::ppu::PIXEL_COLOR)
                .collect()
        })
        .collect()
}

// ------------------------------------------------------------------ the two arming writes

/// `$5104` = %01 is one of the two writes the owner's decision names: extended attribute mode
/// cannot be reached without it (nesdev, "MMC5", the `ExRAM` mode table), so it is the write that
/// brings the full board into being.
#[test]
fn mf_leg_3_the_light_variant_switches_on_ex_ram_mode_1() {
    let mut fixture = parked_fixture();
    assert_variant(&mut fixture.core, false, "a power-on");

    fixture.core.write_memory(0x5104, 0x01);
    assert_variant(&mut fixture.core, true, "$5104 = %01");
}

/// `$5200` bit 7 is the other: "enable the vertical split" (nesdev, "MMC5", `ESxW WWWW`), and
/// nothing of the split happens without it.
#[test]
fn mf_leg_3_the_light_variant_switches_on_the_splits_enable_bit() {
    let mut fixture = parked_fixture();
    fixture.core.write_memory(0x5200, 0x80);
    assert_variant(&mut fixture.core, true, "$5200 bit 7 set");
}

/// `$5201` and `$5202` arm it as well, and this is lot MF-LEG-3's one decision beyond the
/// brief's two writes.
///
/// The light board has nowhere to keep them — it is `04731c86`'s board, which has neither field —
/// so a game writing "bank, scroll, then enable", the natural order and the one a Koei title or
/// *Uchuu Keibitai SDF* would use, would lose both registers at the moment it enabled the split.
/// Arming on them costs nothing: a write to either is as rare as a write to `$5200`, and the
/// board they bring into being renders exactly as the light one does until a mode is on.
#[test]
fn mf_leg_3_the_light_variant_switches_on_the_splits_scroll_and_bank() {
    for (address, value) in [(0x5201u16, 0x5Au8), (0x5202, 0x03)] {
        let mut fixture = parked_fixture();
        fixture.core.write_memory(address, value);
        assert_variant(&mut fixture.core, true, &format!("${address:04X}"));
    }
}

/// The other side of the decision: a write that **cannot** arm either mode leaves the light
/// board, which is the whole point of the lot. `$5104` = 0, 2 and 3 are the three `ExRAM` modes
/// that are not extended attributes, and `$5200` with bit 7 clear says nothing until a later
/// write sets it — and that write carries the same other bits.
#[test]
fn mf_leg_3_the_light_variant_does_not_switch_on_the_other_writes() {
    let writes: &[(u16, u8, &str)] = &[
        (0x5104, 0x00, "$5104 = %00, the plain nametable mode"),
        (0x5104, 0x02, "$5104 = %10, ExRAM as CPU RAM"),
        (0x5104, 0x03, "$5104 = %11, ExRAM read-only"),
        (0x5200, 0x00, "$5200 = 0, the split disabled"),
        (0x5200, 0x08, "$5200 with bit 7 clear and a threshold"),
        (0x5200, 0x7F, "$5200 with every bit but 7"),
        (0x5100, 0x03, "$5100, the PRG mode"),
        (0x5101, 0x03, "$5101, the CHR mode"),
        (0x5105, 0xFF, "$5105, the nametable mapping"),
        (0x5120, 0x01, "$5120, a CHR register"),
        (0x5130, 0x03, "$5130, the global CHR bank bits"),
        (0x5203, 0x40, "$5203, the scanline target"),
        (0x5204, 0x80, "$5204, the IRQ enable"),
        (0x5C00, 0xFF, "an ExRAM byte"),
        (0x8000, 0xFF, "a PRG window"),
    ];
    for &(address, value, what) in writes {
        let mut fixture = parked_fixture();
        fixture.core.write_memory(address, value);
        assert_variant(&mut fixture.core, false, what);
    }
}

/// Every one of those writes, in one run and then a frame: the board still has not switched, so
/// a game that never arms a mode runs `04731c86`'s MMC5 for its whole life.
#[test]
fn mf_leg_3_a_game_that_arms_neither_mode_stays_on_the_light_variant() {
    let mut fixture = parked_fixture();
    plain_background(&mut fixture, OPAQUE_TILE);
    fill_ex_ram(&mut fixture, |_, _| OPAQUE_TILE);
    for mode in [0x00u8, 0x02, 0x03, 0x00] {
        fixture.core.write_memory(0x5104, mode);
    }
    fixture.core.write_memory(0x5200, 0x1F);
    for _ in 0..4 {
        fixture.core.run_frame(0);
    }

    assert_variant(&mut fixture.core, false, "four frames and no arming write");
}

/// It never switches back: the full board is correct with both modes off (the test below proves
/// it renders identically), so there is nothing to gain from switching back and a game that turns
/// a mode on and off every frame would otherwise pay for the switch twice a frame.
#[test]
fn mf_leg_3_the_full_variant_never_switches_back() {
    let mut fixture = parked_fixture();
    fixture.core.write_memory(0x5104, 0x01);
    assert_variant(&mut fixture.core, true, "$5104 = %01");

    fixture.core.write_memory(0x5104, 0x00);
    assert_variant(&mut fixture.core, true, "$5104 back to %00");
    fixture.core.write_memory(0x5200, 0x80);
    fixture.core.write_memory(0x5200, 0x00);
    assert_variant(&mut fixture.core, true, "$5200's enable bit cleared again");
    for _ in 0..3 {
        fixture.core.run_frame(0);
    }

    assert_variant(&mut fixture.core, true, "three frames with neither mode on");
}

/// Power-on and reset both start on the light variant (the brief's point 3): a reset puts every
/// other register back to its power-on value, and the variant is one more of them.
#[test]
fn mf_leg_3_a_reset_starts_on_the_light_variant() {
    let mut fixture = parked_fixture();
    fixture.core.write_memory(0x5104, 0x01);
    fixture.core.write_memory(0x5200, 0xD0);
    assert_variant(&mut fixture.core, true, "both modes armed");

    fixture.core.reset();
    assert_variant(&mut fixture.core, false, "a reset");

    // And it arms again, exactly as a game has to write every other register again.
    fixture.core.write_memory(0x5200, 0x80);
    assert_variant(&mut fixture.core, true, "an arming write after the reset");
}

// ------------------------------------------- the full variant with neither mode armed

/// **The equivalence test, and the one that guards the one body of `mmc5.rs` that is a copy.**
///
/// `Mmc5Ext::see_ppu_read` is the light board's own body with the two modes' per-tile state
/// latched in the middle of it — it has to be, the latching sitting between the fetch that raises
/// the in-frame flag and the repeat counter that consumes the last read address. So the two
/// copies can drift, and no comment can stop them. This test can: it runs the same twenty frames
/// through a board that has switched and one that has not, with **8x16 sprites and two different
/// CHR register sets**, so that the in-frame flag the scanline detection raises decides which set
/// the background is fetched from (MR-5-1) and any drift in the detection is a drift in the
/// picture.
///
/// It is also the brief's point 2 — "it never switches back: the full variant is correct with
/// both modes off" — measured rather than asserted.
#[test]
fn mf_leg_3_the_full_variant_renders_like_the_light_one_with_neither_mode() {
    let mut light = parked_fixture();
    let mut full = parked_fixture();

    for fixture in [&mut light, &mut full] {
        plain_background(fixture, OPAQUE_TILE);
        fill_ex_ram(fixture, |row, column| ((row * 32 + column) & 0xFF) as u8);
        // Set A on 1 KiB banks 8-15 (4 KiB banks 2 and 3), set B on 0-7: with 8x16 sprites the
        // background takes set B while the PPU is in frame and set A outside it, so the picture
        // says which one answered.
        for window in 0..8u16 {
            fixture
                .core
                .write_memory(0x5120 + window, (window + 8) as u8);
        }
        for window in 0..4u16 {
            fixture.core.write_memory(0x5128 + window, window as u8);
        }
        fixture.set_control(0x20); // 8x16 sprites.
    }

    // The full board, armed and then disarmed: every register is the light board's again and only
    // the variant differs.
    full.core.write_memory(0x5104, 0x01);
    full.core.write_memory(0x5200, 0x80);
    full.core.write_memory(0x5104, 0x00);
    full.core.write_memory(0x5200, 0x00);

    assert_variant(&mut light.core, false, "the light board");
    assert_variant(&mut full.core, true, "the armed-then-disarmed board");

    for frame in 0..20u8 {
        // A moving scroll, so that the tiles fetched are not the same ones every frame.
        light.set_scroll(frame * 3, frame);
        full.set_scroll(frame * 3, frame);
        light.core.run_frame(0);
        full.core.run_frame(0);

        assert_eq!(
            rows(&light.core),
            rows(&full.core),
            "frame {frame}: the full variant with neither mode armed must render exactly as the \
             light one — a drift between the two copies of see_ppu_read shows up here"
        );
        assert_eq!(
            light.core.read_memory(0x5204),
            full.core.read_memory(0x5204),
            "frame {frame}: $5204 — the in-frame flag and the scanline IRQ"
        );
        assert_eq!(
            light.core.cycle_count(),
            full.core.cycle_count(),
            "frame {frame}: the two boards must take the same number of CPU cycles"
        );
    }
}

// ----------------------------------------------------------------------- the save-states

/// **The block is the same length in both variants**, which is what lets a state written before
/// this lot load — packs' idle savestates included (the brief's point 5).
///
/// The split's three registers ride in the spare bits of the `i32` that carries `$5104`'s two
/// (`Mmc5Ext::packed_ex_ram_mode`), because `$5104` only ever holds 0 to 3 and bits 31-8 of that
/// word have been written as zero by every build there has ever been. Three bytes appended after
/// the sound block — MF-LEG-2's shape — would make the block three bytes longer and no existing
/// state would load.
#[test]
fn mf_leg_3_the_savestate_block_is_the_same_length_in_both_variants() {
    let mut light = parked_fixture();
    let mut full = parked_fixture();
    full.core.write_memory(0x5200, 0xD0);
    full.core.write_memory(0x5201, 0x5A);
    full.core.write_memory(0x5202, 0x03);

    assert_variant(&mut light.core, false, "no arming write");
    assert_variant(&mut full.core, true, "all three split registers written");

    let light_state = light.core.save_state(true).expect("a light state");
    let full_state = full.core.save_state(true).expect("a full state");
    assert_eq!(
        light_state.len(),
        full_state.len(),
        "the MMC5's savestate block must not change length with the variant"
    );
}

/// A state written by the light variant loads into the light variant and leaves it there: the
/// three registers come back as zero, which is the only value a board that has not switched can
/// have had (a write to any of them is what would have switched it).
#[test]
fn mf_leg_3_a_light_state_round_trips_and_stays_light() {
    let mut fixture = parked_fixture();
    plain_background(&mut fixture, OPAQUE_TILE);
    fixture.core.run_frame(0);
    let state = fixture.core.save_state(true).expect("a light state");

    let mut reloaded = parked_fixture();
    reloaded.core.try_load_state(&state, 0).expect("it loads");
    assert_variant(&mut reloaded.core, false, "a light state");

    fixture.core.run_frame(0);
    reloaded.core.run_frame(0);
    assert_eq!(
        rows(&fixture.core),
        rows(&reloaded.core),
        "the frame after a light round trip"
    );
}

/// A state written while a mode was armed **restores the full variant**, whatever variant the
/// core was running when it was loaded — here a freshly powered-on light one, which is what a
/// host does when it loads a state file at start-up.
#[test]
fn mf_leg_3_a_full_state_restores_the_full_variant_in_a_fresh_core() {
    let mut fixture = parked_fixture();
    plain_background(&mut fixture, OPAQUE_TILE);
    fill_ex_ram(&mut fixture, |row, column| {
        OPAQUE_TILE.wrapping_add((row + column) as u8)
    });
    fixture.core.write_memory(0x5200, ARMING_CONTROL);
    fixture.core.write_memory(0x5201, 0x08);
    fixture.core.write_memory(0x5202, 0x02);
    fixture.core.write_memory(0x5104, 0x01);
    fixture.core.run_frame(0);
    let state = fixture.core.save_state(true).expect("a full state");

    let mut reloaded = parked_fixture();
    assert_variant(&mut reloaded.core, false, "before the load");
    reloaded.core.try_load_state(&state, 0).expect("it loads");
    assert_variant(&mut reloaded.core, true, "a state with both modes armed");

    fixture.core.run_frame(0);
    reloaded.core.run_frame(0);
    assert_eq!(
        rows(&fixture.core),
        rows(&reloaded.core),
        "the frame after a full round trip: both modes, the split's three registers and the \
         per-tile state a line re-derives"
    );
}

/// The same, one register at a time: each of `$5200`, `$5201` and `$5202` survives the round trip
/// on its own, so the packing cannot have lost or crossed a field.
#[test]
fn mf_leg_3_each_split_register_survives_a_round_trip_on_its_own() {
    for (address, value) in [
        (0x5200u16, 0x80u8),
        (0x5200, 0xFF),
        (0x5201, 0x01),
        (0x5201, 0xFF),
        (0x5202, 0x01),
        (0x5202, 0xFF),
    ] {
        let mut fixture = parked_fixture();
        plain_background(&mut fixture, OPAQUE_TILE);
        fill_ex_ram(&mut fixture, |_, column| {
            OPAQUE_TILE.wrapping_add(column as u8)
        });
        // The split enabled in every case, so that the register under test reaches the picture.
        fixture.core.write_memory(0x5200, ARMING_CONTROL);
        fixture.core.write_memory(address, value);
        fixture.core.run_frame(0);
        let state = fixture.core.save_state(true).expect("a full state");

        let mut reloaded = parked_fixture();
        reloaded.core.try_load_state(&state, 0).expect("it loads");
        assert_variant(
            &mut reloaded.core,
            true,
            &format!("a state with ${address:04X} = ${value:02X}"),
        );

        fixture.core.run_frame(0);
        reloaded.core.run_frame(0);
        assert_eq!(
            rows(&fixture.core),
            rows(&reloaded.core),
            "${address:04X} = ${value:02X} must come back out of the packed word"
        );
    }
}

/// **The variant after a load is a function of the state alone**, and nothing else: a light
/// state loaded into a core that had switched comes back on the light variant.
///
/// That is not the write path's rule — a *write* never switches back — and it is stronger than
/// it. `NesCore::restore_state` builds a **new** `Cartridge` from the ROM and loads the block
/// into it, so the live board is not consulted at all; the variant is decided by
/// `Mmc5Ext::state_arms_substitution` reading the block. Rewind and run-ahead both go through
/// states and run-ahead loads one per frame (AMD-041), so this is what makes them
/// deterministic: the same state always gives the same board, whatever the core was running a
/// moment earlier.
#[test]
fn mf_leg_3_the_variant_after_a_load_comes_from_the_state_alone() {
    let mut light = parked_fixture();
    plain_background(&mut light, OPAQUE_TILE);
    light.core.run_frame(0);
    let state = light.core.save_state(true).expect("a light state");

    let mut full = parked_fixture();
    full.core.write_memory(0x5104, 0x01);
    assert_variant(&mut full.core, true, "armed");
    full.core.try_load_state(&state, 0).expect("it loads");
    assert_variant(&mut full.core, false, "a light state loaded over it");

    light.core.run_frame(0);
    full.core.run_frame(0);
    assert_eq!(
        rows(&light.core),
        rows(&full.core),
        "a light state loaded over the full variant must render as the light one"
    );

    // And the other way round, which is the same rule: the state decides.
    full.core.write_memory(0x5104, 0x01);
    let armed = full.core.save_state(true).expect("a full state");
    light.core.try_load_state(&armed, 0).expect("it loads");
    assert_variant(
        &mut light.core,
        true,
        "a full state loaded over the light one",
    );
}

/// **The other state path, and the one where "it never switches back" is the rule.**
///
/// `NesCore::capture_core_state` / `restore_core_state` are what rewind and run-ahead use
/// (REQ-LOOP-067): they load the block into **the cartridge already attached**, where a savestate
/// restore builds a new one. So here the live board does decide — a board that has switched stays
/// switched, whatever the block says, which is sound because the full variant renders a
/// both-modes-off state exactly as the light one does. The reverse is the promotion: a live light
/// board handed a block with a mode armed becomes the full one.
#[test]
fn mf_leg_3_a_core_state_restore_keeps_the_variant_and_promotes_a_light_one() {
    let mut fixture = parked_fixture();
    plain_background(&mut fixture, OPAQUE_TILE);
    fill_ex_ram(&mut fixture, |row, column| {
        ((row * 32 + column) & 0xFF) as u8
    });
    fixture.core.run_frame(0);

    // A block captured before anything is armed, then one with mode 1 on.
    let mut light_state = zax_nes::state::CoreState::with_capacity(0);
    fixture
        .core
        .capture_core_state(&mut light_state)
        .expect("a light block");
    fixture.core.write_memory(0x5104, 0x01);
    fixture.core.run_frame(0);
    let mut full_state = zax_nes::state::CoreState::with_capacity(0);
    fixture
        .core
        .capture_core_state(&mut full_state)
        .expect("a full block");
    assert_variant(&mut fixture.core, true, "mode 1 armed");

    // The light block over the full variant: it stays full, and renders what the light board
    // would — mode 1 is off in the block, so nothing is substituted.
    fixture
        .core
        .restore_core_state(&light_state)
        .expect("it restores");
    assert_variant(
        &mut fixture.core,
        true,
        "a light block over the full variant",
    );
    let after_light = {
        fixture.core.run_frame(0);
        rows(&fixture.core)
    };

    let mut light = parked_fixture();
    plain_background(&mut light, OPAQUE_TILE);
    fill_ex_ram(&mut light, |row, column| ((row * 32 + column) & 0xFF) as u8);
    light.core.run_frame(0);
    light.core.run_frame(0);
    assert_variant(&mut light.core, false, "the light reference");
    let light_rows = rows(&light.core);
    assert_eq!(
        light_rows, after_light,
        "the full variant restored from a both-modes-off block renders as the light board"
    );

    // And the promotion: a fresh light core handed the block with mode 1 on.
    let mut fresh = parked_fixture();
    plain_background(&mut fresh, OPAQUE_TILE);
    fill_ex_ram(&mut fresh, |row, column| ((row * 32 + column) & 0xFF) as u8);
    fresh.core.run_frame(0);
    assert_variant(&mut fresh.core, false, "before the restore");
    fresh
        .core
        .restore_core_state(&full_state)
        .expect("it restores");
    assert_variant(&mut fresh.core, true, "a block with mode 1 armed");
}

/// Run-ahead is a state save and a state load every frame (AMD-041): with a mode armed, twenty
/// frames of it must land exactly where twenty plain frames do.
#[test]
fn mf_leg_3_run_ahead_crosses_the_switch() {
    let mut plain = parked_fixture();
    let mut ahead = parked_fixture();
    for fixture in [&mut plain, &mut ahead] {
        plain_background(fixture, OPAQUE_TILE);
        fill_ex_ram(fixture, |row, column| ((row * 32 + column) & 0xFF) as u8);
        fixture.core.write_memory(0x5200, ARMING_CONTROL);
        fixture.core.write_memory(0x5201, 0x10);
        fixture.core.write_memory(0x5104, 0x01);
    }

    for _ in 0..20 {
        plain.core.run_frame(0);
        ahead.core.run_frame_ahead(0, 1);
    }

    assert_variant(&mut ahead.core, true, "after twenty run-ahead frames");
    assert_eq!(
        rows(&plain.core),
        rows(&ahead.core),
        "run-ahead must not move the two modes by a tile"
    );
}

// --------------------------------------------------------------------- arming mid-frame

/// **The brief's point 4: what the first line after a mid-frame switch shows.**
///
/// The light board does not keep the state the split's counters need — the column, the row, the
/// `ExRAM` index and the run of pattern fetches that finds the start of a line's fetch sequence —
/// so a board that switches *inside* a line starts them at zero. What that costs is bounded, and
/// the bound is the one the fields' own comments promise:
///
/// * `ex_attr_index` is re-derived by the **next nametable fetch**, i.e. within one tile;
/// * `chr_reads_since_nt` starts at 0, so no row start is recognised until the sixteen sprite
///   pattern fetches of dots 257-320 push it past four and the nametable fetch at dot 321 — the
///   first of the **next** line's sequence — reloads the row and puts the column back before 0;
/// * so the tiles of the arming line fetched after the write are drawn with a column counter
///   restarted at 0 and a row of 0, and **every line from the next one on is exact**.
///
/// This test pins all three, and it does not take the arming line on trust: it **finds** the
/// first line that shows anything of the split and then asserts that the frame is exactly the
/// never-armed picture above it, exactly the armed-before-the-frame picture below it, and neither
/// on that one line. Measured, the write of a program polling `$5203` = 60 lands inside the
/// visible dots of line **60** and the frame is
///
/// | lines | what they are |
/// | --- | --- |
/// | 0-59 | the never-armed picture, to the pixel |
/// | **60** | the arming line, and the only line that is neither picture |
/// | 61-239 | the armed-before-the-frame picture, to the pixel |
///
/// An MMC5 game that arms the split from a scanline IRQ handler writes in **`HBlank`** — dots 257
/// to 320, after the line's own fetches and before the dot-321 reload — and then even that line
/// is exact. This test takes the worst case instead, the write landing inside the visible dots,
/// by polling `$5204` with the IRQ line down.
#[test]
fn mf_leg_3_arming_mid_frame_costs_one_line_and_no_more() {
    let ex_ram = |row: usize, column: usize| OPAQUE_TILE.wrapping_add(((row * 3) + column) as u8);

    // Three cores, the same set-up: one never arms, one arms before the frame, one arms on line
    // ARMING_LINE from its own 6502 program.
    let mut never = parked_fixture();
    let mut before = parked_fixture();
    let mut during = parked_fixture();
    for fixture in [&mut never, &mut before, &mut during] {
        plain_background(fixture, OPAQUE_TILE);
        fill_ex_ram(fixture, ex_ram);
    }
    before.core.write_memory(0x5200, ARMING_CONTROL);
    // The mid-frame core's own program writes $5200; the flag lets it out of its wait loop.
    during.core.write_memory(0x0700, 0x01);

    never.core.run_frame(0);
    before.core.run_frame(0);
    during.core.run_frame(0);

    assert_variant(&mut never.core, false, "the core that never arms");
    assert_variant(
        &mut before.core,
        true,
        "the core that arms before the frame",
    );
    assert_variant(
        &mut during.core,
        true,
        "the core whose program armed the split mid-frame",
    );

    let never_rows = rows(&never.core);
    let before_rows = rows(&before.core);
    let during_rows = rows(&during.core);
    assert_ne!(
        never_rows, before_rows,
        "the split must change the picture, or this test proves nothing"
    );

    // The arming line is found, not assumed: it is the first line on which the split shows.
    let arming_line = (0..SCREEN_HEIGHT)
        .find(|&line| never_rows[line] != during_rows[line])
        .expect("the program must have armed the split somewhere in the frame");
    assert_eq!(
        ARMING_LINE, arming_line,
        "the write of a program polling $5203 = {ARMING_TARGET} lands on visible line \
         {ARMING_LINE}; if the scanline detection moves by a line this is the figure to re-take, \
         not the structure below"
    );

    for line in 0..arming_line {
        assert_eq!(
            never_rows[line], during_rows[line],
            "line {line} is above the arming line: nothing of the split may show yet"
        );
    }

    for line in (arming_line + 1)..SCREEN_HEIGHT {
        assert_eq!(
            before_rows[line], during_rows[line],
            "line {line} is below the arming line: the split's counters are re-derived by the \
             fetch sequence of the next line, so every line from there on must be exactly what \
             arming before the frame gives"
        );
    }

    assert_ne!(
        before_rows[arming_line], during_rows[arming_line],
        "the arming line itself is the one line that is neither picture, and the test says so \
         rather than hiding it: the column counter restarts at 0 and the row at 0 until the \
         reload at dot 321"
    );
}
