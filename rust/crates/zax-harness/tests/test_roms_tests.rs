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
//! * one Holy Mapperel board is `unsupported` because mapper 78.3 is not in the registry. Adding
//!   it should flip its row, and the test says so out loud.
//!
//! Rows MF-3 has already flipped: the two SNROM boards used to fail on `$A000` bit 4 not
//! disabling the work RAM (step 1), and five boards were `unsupported` before their mappers were
//! added — the two mapper-69 ones (step 3), the mapper-118 one (step 5) and the two mapper-180
//! ones (step 6).

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
        // Mapper 78 submapper 3 (IF-12) is not in the registry.
        (
            "holy-mapperel/testroms/M78.3_P128K_C64K.nes",
            Outcome::Unsupported,
        ),
        ("holy-mapperel/testroms/M7_P128K.nes", Outcome::Pass),
        ("holy-mapperel/testroms/M7_P128K_CR8K.nes", Outcome::Pass),
        ("holy-mapperel/testroms/M9_P128K_C64K.nes", Outcome::Pass),
    ];

    check("holy-mapperel", &EXPECTED);
}
