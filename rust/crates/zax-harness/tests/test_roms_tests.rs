//! `test-roms` over a small, fixed subset of the public suites, as a regression guard.
//!
//! The ROMs are not in the repository, so this runs **only** when `ZAXNES_TEST_ROMS` points at
//! a copy of `assets-free/test-roms/`; without it every test here returns at once and the
//! suite stays green. With it, the three groups together take about eight seconds.
//!
//! What is asserted is the outcome each ROM has **today**, failures included, not just the
//! passes. A row that flips either way is a change worth knowing about:
//!
//! * `mmc3_test/6-MMC6` fails on purpose. It is the revision A half of a pair of MMC3 tests
//!   that contradict each other, and REQ-CORE-100 pins mapper 4 to revision B (AMD-061's
//!   answer to Q-114 restates it for REQ-CORE-114). `5-MMC3`, the revision B half, passes. If
//!   6 ever started passing, 5 would have stopped, and that would be a regression against the
//!   spec rather than a fix.
//! * every Holy Mapperel board the suite carries passes in the **default** build. MF-3 flipped
//!   eight of its rows: the two SNROM boards used to **fail** on `$A000` bit 4 not disabling the
//!   work RAM (step 1), and six were `unsupported` before their mappers were added — the two
//!   mapper-69 ones (step 3), the mapper-118 one (step 5), the two mapper-180 ones (step 6) and
//!   the mapper-78.3 one (step 7).
//! * those **two SNROM rows fail again** with `mmc1-snrom-header` on, which is why they are
//!   `SNROM_ROW` and not `Outcome::Pass`. See that constant and Q-176: the feature is off in
//!   `default` precisely because it reverts them.

// A test: a missing ROM or an unreadable directory should fail it outright.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use zax_harness::test_roms::{self, Outcome};
use zax_nes::cartridges::mappers::Mmc1;

/// What the suite's two **SNROM** rows do, which depends on how the build was compiled
/// (`mmc1-snrom-header`, off in `default`; Q-176).
///
/// `M1_P128K.nes` and `M1_P128K_CR8K.nes` announce themselves as "SNROM (MMC1)", report "8K PRG
/// RAM OK" and probe detailed digit `4000`, "$A000 bit 4 does not disable WRAM (SNROM), or
/// disables it (all but SNROM)" — and their NES 2.0 headers declare **no** PRG-RAM in byte 10. So
/// they pass under the three-way size test lot MF-3 step 1 built for them, and fail under
/// MF-LEG-1's header-declared rule, which cannot see the work-RAM chip the ROM is testing. The
/// row is written this way rather than pinned either way because the choice between the hardware
/// test and the header is R0's.
const SNROM_ROW: Outcome = if Mmc1::SNROM_NEEDS_A_DECLARED_HEADER {
    Outcome::Fail
} else {
    Outcome::Pass
};

/// The directory `ZAXNES_TEST_ROMS` names, or `None` when the variable is unset or does not
/// point at a directory that exists.
fn roms_dir() -> Option<PathBuf> {
    let value = std::env::var_os(test_roms::ROMS_DIR_VARIABLE)?;
    let dir = PathBuf::from(value);
    dir.is_dir().then_some(dir)
}

/// Runs the ROMs of `filter` and checks each against `expected`, keyed by relative path.
///
/// Every expected ROM must be there and no unexpected one may appear, so a ROM added to or
/// removed from the collection is caught rather than silently skipped.
fn check(filter: &str, expected: &[(&str, Outcome)]) {
    let Some(dir) = roms_dir() else {
        println!(
            "skipped: set {} to a copy of assets-free/test-roms/ to run this",
            test_roms::ROMS_DIR_VARIABLE
        );
        return;
    };

    let options = test_roms::Options::parse(&[
        dir.display().to_string(),
        dir.join("unused.json").display().to_string(),
        "--filter".to_owned(),
        filter.to_owned(),
        "--no-shots".to_owned(),
        "--quiet".to_owned(),
    ])
    .unwrap();
    let results = test_roms::collect(&options).unwrap();

    let mut seen: Vec<&str> = Vec::new();
    for (rom, want) in expected {
        let row = results
            .iter()
            .find(|row| row.rom == *rom)
            .unwrap_or_else(|| panic!("{filter}: {rom} was not run"));
        assert_eq!(
            *want,
            row.outcome,
            "{rom}: expected {}, got {} — {}",
            want.label(),
            row.outcome.label(),
            row.message
        );
        seen.push(rom);
    }

    let extra: Vec<&str> = results
        .iter()
        .map(|row| row.rom.as_str())
        .filter(|rom| !seen.contains(rom))
        .collect();
    assert!(
        extra.is_empty(),
        "{filter}: {extra:?} ran but are not in the expected list"
    );
}

#[test]
fn the_blargg_instruction_singles_all_pass() {
    const EXPECTED: [(&str, Outcome); 16] = [
        (
            "nes-test-roms/instr_test-v5/rom_singles/01-basics.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/instr_test-v5/rom_singles/02-implied.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/instr_test-v5/rom_singles/03-immediate.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/instr_test-v5/rom_singles/04-zero_page.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/instr_test-v5/rom_singles/05-zp_xy.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/instr_test-v5/rom_singles/06-absolute.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/instr_test-v5/rom_singles/07-abs_xy.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/instr_test-v5/rom_singles/08-ind_x.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/instr_test-v5/rom_singles/09-ind_y.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/instr_test-v5/rom_singles/10-branches.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/instr_test-v5/rom_singles/11-stack.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/instr_test-v5/rom_singles/12-jmp_jsr.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/instr_test-v5/rom_singles/13-rts.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/instr_test-v5/rom_singles/14-rti.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/instr_test-v5/rom_singles/15-brk.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/instr_test-v5/rom_singles/16-special.nes",
            Outcome::Pass,
        ),
    ];

    check("instr_test-v5/rom_singles", &EXPECTED);
}

#[test]
fn req_core_100_keeps_the_mmc3_at_revision_b() {
    // 1 to 5 pass; 6 is the revision A test and must keep failing while REQ-CORE-100 pins
    // mapper 4 to revision B.
    const EXPECTED: [(&str, Outcome); 6] = [
        ("nes-test-roms/mmc3_test/1-clocking.nes", Outcome::Pass),
        ("nes-test-roms/mmc3_test/2-details.nes", Outcome::Pass),
        ("nes-test-roms/mmc3_test/3-A12_clocking.nes", Outcome::Pass),
        (
            "nes-test-roms/mmc3_test/4-scanline_timing.nes",
            Outcome::Pass,
        ),
        ("nes-test-roms/mmc3_test/5-MMC3.nes", Outcome::Pass),
        ("nes-test-roms/mmc3_test/6-MMC6.nes", Outcome::Fail),
    ];

    check("mmc3_test/", &EXPECTED);
}

#[test]
fn holy_mapperel_detects_every_supported_board() {
    const EXPECTED: [(&str, Outcome); 40] = [
        ("holy-mapperel/testroms/M0_P32K_C8K_V.nes", Outcome::Pass),
        ("holy-mapperel/testroms/M0_P32K_CR32K_V.nes", Outcome::Pass),
        ("holy-mapperel/testroms/M0_P32K_CR8K_V.nes", Outcome::Pass),
        (
            "holy-mapperel/testroms/M10_P128K_C64K_S8K.nes",
            Outcome::Pass,
        ),
        (
            "holy-mapperel/testroms/M10_P128K_C64K_W8K.nes",
            Outcome::Pass,
        ),
        ("holy-mapperel/testroms/M118_P128K_C64K.nes", Outcome::Pass),
        ("holy-mapperel/testroms/M11_P64K_C64K_V.nes", Outcome::Pass),
        ("holy-mapperel/testroms/M11_P64K_CR32K_V.nes", Outcome::Pass),
        (
            "holy-mapperel/testroms/M180_P128K_CR8K_H.nes",
            Outcome::Pass,
        ),
        ("holy-mapperel/testroms/M180_P128K_H.nes", Outcome::Pass),
        ("holy-mapperel/testroms/M1_P128K.nes", SNROM_ROW),
        ("holy-mapperel/testroms/M1_P128K_C128K.nes", Outcome::Pass),
        (
            "holy-mapperel/testroms/M1_P128K_C128K_S8K.nes",
            Outcome::Pass,
        ),
        (
            "holy-mapperel/testroms/M1_P128K_C128K_W8K.nes",
            Outcome::Pass,
        ),
        ("holy-mapperel/testroms/M1_P128K_C32K.nes", Outcome::Pass),
        (
            "holy-mapperel/testroms/M1_P128K_C32K_S8K.nes",
            Outcome::Pass,
        ),
        (
            "holy-mapperel/testroms/M1_P128K_C32K_W8K.nes",
            Outcome::Pass,
        ),
        ("holy-mapperel/testroms/M1_P128K_CR8K.nes", SNROM_ROW),
        (
            "holy-mapperel/testroms/M1_P512K_CR8K_S32K.nes",
            Outcome::Pass,
        ),
        (
            "holy-mapperel/testroms/M1_P512K_CR8K_S8K.nes",
            Outcome::Pass,
        ),
        ("holy-mapperel/testroms/M1_P512K_S32K.nes", Outcome::Pass),
        ("holy-mapperel/testroms/M1_P512K_S8K.nes", Outcome::Pass),
        ("holy-mapperel/testroms/M28_P512K.nes", Outcome::Pass),
        ("holy-mapperel/testroms/M28_P512K_CR32K.nes", Outcome::Pass),
        ("holy-mapperel/testroms/M2_P128K_CR8K_V.nes", Outcome::Pass),
        ("holy-mapperel/testroms/M2_P128K_V.nes", Outcome::Pass),
        ("holy-mapperel/testroms/M34_P128K_CR8K_H.nes", Outcome::Pass),
        ("holy-mapperel/testroms/M34_P128K_H.nes", Outcome::Pass),
        ("holy-mapperel/testroms/M3_P32K_C32K_H.nes", Outcome::Pass),
        ("holy-mapperel/testroms/M4_P128K.nes", Outcome::Pass),
        ("holy-mapperel/testroms/M4_P128K_CR32K.nes", Outcome::Pass),
        ("holy-mapperel/testroms/M4_P128K_CR8K.nes", Outcome::Pass),
        ("holy-mapperel/testroms/M4_P256K_C256K.nes", Outcome::Pass),
        ("holy-mapperel/testroms/M66_P64K_C16K_V.nes", Outcome::Pass),
        (
            "holy-mapperel/testroms/M69_P128K_C64K_S8K.nes",
            Outcome::Pass,
        ),
        (
            "holy-mapperel/testroms/M69_P128K_C64K_W8K.nes",
            Outcome::Pass,
        ),
        ("holy-mapperel/testroms/M78.3_P128K_C64K.nes", Outcome::Pass),
        ("holy-mapperel/testroms/M7_P128K.nes", Outcome::Pass),
        ("holy-mapperel/testroms/M7_P128K_CR8K.nes", Outcome::Pass),
        ("holy-mapperel/testroms/M9_P128K_C64K.nes", Outcome::Pass),
    ];

    check("holy-mapperel", &EXPECTED);
}

/// The two OAM-against-DMC ROMs lot MF-4a turned green (REQ-CORE-113).
#[test]
fn req_core_113_an_oam_transfer_and_a_dmc_fetch_share_the_bus() {
    const EXPECTED: [(&str, Outcome); 2] = [
        (
            "nes-test-roms/sprdma_and_dmc_dma/sprdma_and_dmc_dma.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/sprdma_and_dmc_dma/sprdma_and_dmc_dma_512.nes",
            Outcome::Pass,
        ),
    ];

    check("sprdma_and_dmc_dma", &EXPECTED);
}

/// A DMC fetch that halts the CPU mid-read (REQ-CORE-113).
///
/// `dma_2007_read` and `double_2007_read` are `manual` because neither prints a verdict the
/// harness can read; they are listed so that a change to either shows up here. The first names
/// two acceptable outputs in its own source, of which this core prints the second; the second
/// is a PPU read-buffer row, not a DMA one.
#[test]
fn req_core_113_the_halt_cycle_repeats_the_read_it_interrupted() {
    const EXPECTED: [(&str, Outcome); 5] = [
        (
            "nes-test-roms/dmc_dma_during_read4/dma_2007_read.nes",
            Outcome::Manual,
        ),
        (
            "nes-test-roms/dmc_dma_during_read4/dma_2007_write.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/dmc_dma_during_read4/dma_4016_read.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/dmc_dma_during_read4/double_2007_read.nes",
            Outcome::Manual,
        ),
        (
            "nes-test-roms/dmc_dma_during_read4/read_write_2007.nes",
            Outcome::Pass,
        ),
    ];

    check("dmc_dma_during_read4", &EXPECTED);
}

/// The frame sequencer on a PAL console (REQ-CORE-015, REQ-CORE-111). The harness runs this
/// suite on a PAL console because its own readme says it was "tested on a PAL NES", though its
/// files carry NTSC headers. Seven of these ten were failing before lot MF-4a.
#[test]
fn req_core_015_the_frame_sequencer_passes_on_a_pal_console() {
    const EXPECTED: [(&str, Outcome); 10] = [
        ("nes-test-roms/pal_apu_tests/01.len_ctr.nes", Outcome::Pass),
        (
            "nes-test-roms/pal_apu_tests/02.len_table.nes",
            Outcome::Pass,
        ),
        ("nes-test-roms/pal_apu_tests/03.irq_flag.nes", Outcome::Pass),
        (
            "nes-test-roms/pal_apu_tests/04.clock_jitter.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/pal_apu_tests/05.len_timing_mode0.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/pal_apu_tests/06.len_timing_mode1.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/pal_apu_tests/07.irq_flag_timing.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/pal_apu_tests/08.irq_timing.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/pal_apu_tests/10.len_halt_timing.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/pal_apu_tests/11.len_reload_timing.nes",
            Outcome::Pass,
        ),
    ];

    check("pal_apu_tests", &EXPECTED);
}

/// The NTSC half of the same question: the PAL table and the NTSC one are separate, and a lot
/// that touches either must leave the other alone (REQ-CORE-111).
#[test]
fn req_core_111_the_ntsc_frame_sequencer_does_not_move_with_the_pal_one() {
    const EXPECTED: [(&str, Outcome); 11] = [
        (
            "nes-test-roms/blargg_apu_2005.07.30/01.len_ctr.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/blargg_apu_2005.07.30/02.len_table.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/blargg_apu_2005.07.30/03.irq_flag.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/blargg_apu_2005.07.30/04.clock_jitter.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/blargg_apu_2005.07.30/05.len_timing_mode0.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/blargg_apu_2005.07.30/06.len_timing_mode1.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/blargg_apu_2005.07.30/07.irq_flag_timing.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/blargg_apu_2005.07.30/08.irq_timing.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/blargg_apu_2005.07.30/09.reset_timing.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/blargg_apu_2005.07.30/10.len_halt_timing.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/blargg_apu_2005.07.30/11.len_reload_timing.nes",
            Outcome::Pass,
        ),
    ];

    check("blargg_apu_2005.07.30", &EXPECTED);
}

/// blargg's later APU suite, whose 7 and 8 are the DMC's own tests (REQ-CORE-113).
#[test]
fn req_core_113_the_apu_and_dmc_suite_passes() {
    const EXPECTED: [(&str, Outcome); 9] = [
        ("nes-test-roms/apu_test/apu_test.nes", Outcome::Pass),
        (
            "nes-test-roms/apu_test/rom_singles/1-len_ctr.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/apu_test/rom_singles/2-len_table.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/apu_test/rom_singles/3-irq_flag.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/apu_test/rom_singles/4-jitter.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/apu_test/rom_singles/5-len_timing.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/apu_test/rom_singles/6-irq_flag_timing.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/apu_test/rom_singles/7-dmc_basics.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/apu_test/rom_singles/8-dmc_rates.nes",
            Outcome::Pass,
        ),
    ];

    check("apu_test/", &EXPECTED);
}

/// `4017_timing` pins the convention every DMA alignment in this core is built on: the
/// sequencer reset lands three CPU cycles after a `$4017` write made during an APU cycle and
/// four otherwise (REQ-CORE-004, REQ-CORE-111).
#[test]
fn req_core_004_the_apu_comes_back_from_reset_where_it_should() {
    const EXPECTED: [(&str, Outcome); 6] = [
        ("nes-test-roms/apu_reset/4015_cleared.nes", Outcome::Pass),
        ("nes-test-roms/apu_reset/4017_timing.nes", Outcome::Pass),
        ("nes-test-roms/apu_reset/4017_written.nes", Outcome::Pass),
        (
            "nes-test-roms/apu_reset/irq_flag_cleared.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/apu_reset/len_ctrs_enabled.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/apu_reset/works_immediately.nes",
            Outcome::Pass,
        ),
    ];

    check("apu_reset", &EXPECTED);
}

/// Reads the per-test table of the one `AccuracyCoin` row, or `None` when the ROMs are not there.
fn accuracy_coin() -> Option<(usize, serde_json::Map<String, serde_json::Value>)> {
    let dir = roms_dir()?;
    let options = test_roms::Options::parse(&[
        dir.display().to_string(),
        dir.join("unused.json").display().to_string(),
        "--filter".to_owned(),
        "AccuracyCoin".to_owned(),
        "--no-shots".to_owned(),
        "--quiet".to_owned(),
    ])
    .unwrap();
    let results = test_roms::collect(&options).unwrap();
    let row = results.first().expect("the AccuracyCoin row");
    let detail = &row.detail;
    let passed = usize::try_from(detail["passed"].as_u64().unwrap()).unwrap();
    let tests = detail["tests"].as_object().unwrap().clone();
    Some((passed, tests))
}

/// `AccuracyCoin`'s own tally and the rows lot MF-4b turned green, which the campaign's single
/// `fail` row cannot show (REQ-CORE-110, REQ-CORE-111, REQ-CORE-112, REQ-CORE-113).
///
/// The score is asserted as a floor and the rows by name, so that a lot which fixes more does not
/// have to edit this test while one that breaks any of these does. The floor and the named list
/// both depend on the configuration: two PPU features are out of `default` and each is worth a
/// known number of rows, so a build cannot silently lose a row it ships nor be asked for one it
/// does not (Q-139).
#[test]
fn req_core_110_accuracy_coin_keeps_the_rows_mf_4b_to_mf_4e_turned_green() {
    let Some((passed, tests)) = accuracy_coin() else {
        println!(
            "skipped: set {} to a copy of assets-free/test-roms/ to run this",
            test_roms::ROMS_DIR_VARIABLE
        );
        return;
    };

    // Lot MF-4d-fix's end: MF-4c's 125 plus `Controller Strobing`, `DMC Reload Timing` and
    // `$2002 flag timing`. 116 of MF-4b's 124 were already there once the drive stopped
    // mistaking `Implied Dummy Reads`' backup of the zero page for the end of the run (see
    // `coin.rs`).
    //
    // **The floor is a sum of three terms, because two PPU features are out of `default`** and
    // each is worth a known number of rows (Q-125, Q-137, Q-139). 128 is the figure every build
    // has; `ppu-2007-render-read` adds one row and `ppu-exact-sprite-eval` three, so the four
    // combinations are 128 / 129 / 131 / 132 and each row is named below only in the configuration
    // that ships it.
    //
    // **`ppu-2007-render-read`, +1: `PPU Misc. / $2007 read w/ rendering`.** Lot MF-4d step 6
    // turned it green with a rule that is right on hardware, and lot MF-4d-fix backed it out
    // because as shipped it broke AMD-021 R2-005 criterion 2 — a P1 requirement — on Super Mario
    // Bros. 3's split line. Lot MF-4e holds **both**: the rule is back on `v`, and it no longer
    // reaches the captured row, which is where the collision actually was, so R2-005's own ignored
    // test (`amd_021_r2_005_smb3_emits_no_background_row_at_the_split_line`) passes beside it. Lot
    // MF-4e-gate then took it out of `default` — not for a figure, it measures +0.000 % to
    // +0.005 % on PR-1's eleven, but because R0's A/B of the 3D packs found it moving the picture
    // of Super Mario Bros. 3 and Zelda II, which the owner validates by eye first.
    //
    // **`ppu-exact-sprite-eval`, +3**: the exact sprite-evaluation pipeline of MF-4e step 3, out of
    // `default` because it measures +0.041 % to +0.133 % on PR-1's thirteen and above the tolerance
    // on all ten level-2 figures. `Address $2004 behavior`, `Arbitrary Sprite zero` and
    // `Misaligned OAM behavior`.
    let mut floor = 128;
    if cfg!(feature = "ppu-2007-render-read") {
        floor += 1;
    }
    if cfg!(feature = "ppu-exact-sprite-eval") {
        floor += 3;
    }
    assert!(
        passed >= floor,
        "AccuracyCoin scored {passed}/147, which is under this configuration's {floor}"
    );

    // Nothing may be left unrun or retired: the drive measures its progress by the result bytes
    // and a test that hangs would show up here rather than as a silent skip.
    assert_eq!(0, tests.values().filter(|v| *v == "not run").count());
    assert_eq!(
        0,
        tests
            .values()
            .filter(|v| v.as_str() == Some("skipped"))
            .count()
    );

    for name in [
        // Step 1: the second poll of a page-crossing branch does not clear the first's decision.
        "CPU Interrupts / Interrupt flag latency",
        // Step 3: `$4015` answers on the internal data bus and leaves the pins alone.
        "CPU Behavior / Open Bus",
        "CPU Behavior 2 / Internal Data Bus",
        // Step 4: a DMA on the dummy read drops the `SH*` stores' AND.
        "Unofficial SH* / $93 SHA Indirect,Y",
        "Unofficial SH* / $9F SHA Absolute,Y",
        "Unofficial SH* / $9B SHS Absolute,Y",
        "Unofficial SH* / $9C SHY Absolute,X",
        "Unofficial SH* / $9E SHX Absolute,Y",
        // Behind the drive's wall until step 3, and passing once they were reached.
        "CPU Behavior 2 / Implied Dummy Reads",
        "PPU Behavior / PPU Register Mirroring",
        "PPU Behavior / PPU Register Open Bus",
        "PPU Behavior / PPU Read Buffer",
        // MF-4c step 1: a palette read under greyscale is masked with `$30`.
        "PPU Behavior / Palette RAM Quirks",
        // MF-4d step 1: the shift registers reload on the put cycle the raised strobe line
        // covers, not on the `$4016` write.
        "APU Tests / Controller Strobing",
        // MF-4d step 2: the DMC timer reloads from the period a write on that very cycle left.
        "CPU Behavior 2 / DMC Reload Timing",
        // MF-4d step 5: a `$2002` read samples the sprite flags about 1.9 dots after bit 7.
        "Sprite Evaluation / $2002 flag timing",
    ] {
        let row = tests[name].as_str().unwrap();
        assert!(row.starts_with("pass"), "{name}: {row}");
    }

    // MF-4e step 2's one, named only in the configuration that ships it. A `$2007` access during
    // rendering triggers a coarse X and a Y increment at once (Q-125), and the borrowed increment
    // is kept out of the captured row so that AMD-021 R2-005 holds with it (Q-137). The row stood
    // in the list above from MF-4d step 6, came out when MF-4d-fix backed that half out, went back
    // in at MF-4e step 2, and is here behind its feature since MF-4e-gate (Q-139).
    if cfg!(feature = "ppu-2007-render-read") {
        let row = tests["PPU Misc. / $2007 read w/ rendering"]
            .as_str()
            .unwrap();
        assert!(row.starts_with("pass"), "$2007 read w/ rendering: {row}");
    }

    // MF-4e step 3's three, named only in the configuration that ships them.
    if cfg!(feature = "ppu-exact-sprite-eval") {
        for name in [
            // A `$2004` read during rendering answers from the OAM copy buffer, and from the
            // secondary OAM on the sprite-fetch dots.
            "PPU Misc. / Address $2004 behavior",
            // Sprite zero is whatever dot 66 looks at, not OAM index 0.
            "Sprite Evaluation / Arbitrary Sprite zero",
            // The range test on every even dot, and the conditional realignment after a slot.
            "Sprite Evaluation / Misaligned OAM behavior",
        ] {
            let row = tests[name].as_str().unwrap();
            assert!(row.starts_with("pass"), "{name}: {row}");
        }
    }
}

/// The CPU interrupt, timing and unofficial-instruction suites steps 1 to 4 must not move
/// (REQ-CORE-110, REQ-CORE-111).
#[test]
fn req_core_111_the_cpu_interrupt_and_timing_suites_pass() {
    const INTERRUPTS: [(&str, Outcome); 6] = [
        (
            "nes-test-roms/cpu_interrupts_v2/cpu_interrupts.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/cpu_interrupts_v2/rom_singles/1-cli_latency.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/cpu_interrupts_v2/rom_singles/2-nmi_and_brk.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/cpu_interrupts_v2/rom_singles/3-nmi_and_irq.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/cpu_interrupts_v2/rom_singles/4-irq_and_dma.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/cpu_interrupts_v2/rom_singles/5-branch_delays_irq.nes",
            Outcome::Pass,
        ),
    ];
    const BRANCHES: [(&str, Outcome); 3] = [
        (
            "nes-test-roms/branch_timing_tests/1.Branch_Basics.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/branch_timing_tests/2.Backward_Branch.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/branch_timing_tests/3.Forward_Branch.nes",
            Outcome::Pass,
        ),
    ];
    const TIMING: [(&str, Outcome); 3] = [
        ("nes-test-roms/instr_timing/instr_timing.nes", Outcome::Pass),
        (
            "nes-test-roms/instr_timing/rom_singles/1-instr_timing.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/instr_timing/rom_singles/2-branch_timing.nes",
            Outcome::Pass,
        ),
    ];
    check("cpu_interrupts_v2", &INTERRUPTS);
    check("branch_timing_tests", &BRANCHES);
    check("instr_timing", &TIMING);
}

/// The PPU's data-bus latch and its `$2007` read buffer, which lot MF-4b's step 2 moved
/// (REQ-CORE-112).
///
/// `ppu_open_bus` covers the latch and its decay; `ppu_read_buffer` covers the buffer, the
/// palette's immediate read and the nametable byte a palette read puts underneath it. Neither
/// reads `$2007` twice inside one instruction, which is why `double_2007_read` — pinned in
/// `req_core_113_the_halt_cycle_repeats_the_read_it_interrupted` above — is the row that moved.
#[test]
fn req_core_112_the_ppu_bus_latch_and_the_read_buffer_pass() {
    const EXPECTED: [(&str, Outcome); 1] =
        [("nes-test-roms/ppu_open_bus/ppu_open_bus.nes", Outcome::Pass)];
    const BUFFER: [(&str, Outcome); 1] = [(
        "nes-test-roms/ppu_read_buffer/test_ppu_read_buffer.nes",
        Outcome::Pass,
    )];
    check("ppu_open_bus", &EXPECTED);
    check("ppu_read_buffer", &BUFFER);
}

/// The dummy reads and dummy writes of the official and unofficial instructions, which step 4's
/// `SH*` change must not move (REQ-CORE-110).
#[test]
fn req_core_110_the_dummy_cycle_suites_pass() {
    const READS: [(&str, Outcome); 1] = [(
        "nes-test-roms/cpu_dummy_reads/cpu_dummy_reads.nes",
        Outcome::Pass,
    )];
    const WRITES: [(&str, Outcome); 2] = [
        (
            "nes-test-roms/cpu_dummy_writes/cpu_dummy_writes_oam.nes",
            Outcome::Pass,
        ),
        (
            "nes-test-roms/cpu_dummy_writes/cpu_dummy_writes_ppumem.nes",
            Outcome::Pass,
        ),
    ];
    check("cpu_dummy_reads", &READS);
    check("cpu_dummy_writes", &WRITES);
}

/// The MMC5 ROMs of the collection, which lot MF-4c's step 4 put on the board's own PPU bus
/// (finding MR-5-4, REQ-CORE-100).
///
/// All three are `manual` — they print a screen with no documented verdict — so what this group
/// guards is that they still **run**: a board that now reacts to every PPU read is a board that
/// can hang, loop or lose its IRQ, and any of those would turn one of these into a `timeout`.
/// The behaviour itself is pinned where it can be asserted rather than transcribed:
/// `mr1_review_tests::mr_5_4_*` for the set and the two clears,
/// `mapper_irq_tests::req_core_100_the_mmc5_scanline_irq_fires_on_the_line_5203_names` for the
/// line the IRQ lands on, and `mmc5test`'s picture — which lot MF-4c measured as the old picture
/// moved up exactly one scanline — in `docs/progress/MF-4c.md`, step 5.
#[test]
fn req_core_100_the_mmc5_rom_suites_keep_their_outcomes() {
    const EXRAM: [(&str, Outcome); 1] = [("nes-test-roms/exram/mmc5exram.nes", Outcome::Manual)];
    const MMC5: [(&str, Outcome); 2] = [
        ("nes-test-roms/mmc5test/mmc5test.nes", Outcome::Manual),
        ("nes-test-roms/mmc5test_v2/mmc5test.nes", Outcome::Manual),
    ];
    check("exram", &EXRAM);
    check("mmc5test", &MMC5);
}
