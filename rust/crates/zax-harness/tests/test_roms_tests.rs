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
//! * every Holy Mapperel board the suite carries now passes. MF-3 flipped eight of its rows: the
//!   two SNROM boards used to **fail** on `$A000` bit 4 not disabling the work RAM (step 1), and
//!   six were `unsupported` before their mappers were added — the two mapper-69 ones (step 3),
//!   the mapper-118 one (step 5), the two mapper-180 ones (step 6) and the mapper-78.3 one
//!   (step 7).

// A test: a missing ROM or an unreadable directory should fail it outright.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use zax_harness::test_roms::{self, Outcome};

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
        ("holy-mapperel/testroms/M1_P128K.nes", Outcome::Pass),
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
        ("holy-mapperel/testroms/M1_P128K_CR8K.nes", Outcome::Pass),
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
