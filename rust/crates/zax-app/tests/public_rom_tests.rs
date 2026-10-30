//! Port of `PublicRomTests.cs`: savestate round-trip determinism on a public test ROM, which is
//! the automatable part of SPEC-12 REQ-ACC-040 together with REQ-ARCH-012 and REQ-ARCH-013.
//!
//! The public ROMs live in `rust/test-roms/` and are never committed (its README says where to
//! fetch them); a test whose ROM is absent reports that and returns. The synthetic-ROM version of
//! these checks lives in `save_state_tests.rs` and always runs.
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

mod common;

use std::path::{Path, PathBuf};

use common::LoopHarness;
use zax_app::emulation::commands::{Command, CommandArgs};
use zax_app::emulation::emulation_state::EmulationState;
use zax_app::input::InputAction;

const NESTEST: &str = "other/nestest.nes";

/// The full path of a ROM under `rust/test-roms`, or `None` after saying what is missing.
fn public_rom(relative: &str) -> Option<PathBuf> {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test-roms");
    let candidate = folder.join(relative);
    if candidate.is_file() {
        return Some(candidate);
    }
    eprintln!(
        "SKIPPED: public test ROM not available: {relative}. See {} for its source and licence.",
        folder.join("README.md").display()
    );
    None
}

fn play_and_capture(harness: &mut LoopHarness) -> Vec<u8> {
    // A deliberate input sequence, so the replay is not a trivially identical idle run.
    let sequence = [
        InputAction::NONE,
        InputAction::START,
        InputAction::NONE,
        InputAction::A,
        InputAction::RIGHT,
        InputAction::A | InputAction::RIGHT,
    ];
    harness.run_exact_frames(180, &sequence);
    harness
        .host
        .session()
        .unwrap()
        .core()
        .save_state(false)
        .unwrap()
}

/// REQ-ARCH-012 and REQ-LOOP-060: saving at frame N on a public test ROM, running on, reloading
/// and replaying the same input sequence reproduces the core state byte for byte.
#[test]
fn req_arch_012_a_public_rom_replays_identically_after_a_load() {
    let Some(rom) = public_rom(NESTEST) else {
        return;
    };
    let mut harness = LoopHarness::create();
    harness
        .catalog()
        .add("nestest", Some(rom.to_str().unwrap()));

    harness.run_with(Command::LaunchGame, CommandArgs::of_text("nestest"));
    assert_eq!(harness.host.state(), EmulationState::Running);

    harness.run_frames(120);
    harness.run_with(Command::SaveSlot, CommandArgs::of_number(1));

    let first_run = play_and_capture(&mut harness);

    harness.run_with(Command::LoadSlot, CommandArgs::of_number(1));
    assert_eq!(harness.host.state(), EmulationState::Running);

    let second_run = play_and_capture(&mut harness);

    assert_eq!(first_run, second_run);
}

/// REQ-ARCH-013 and REQ-PERS-056: on a real ROM the file stays under the announced bound, and two
/// saves taken without an emulated frame in between are identical.
#[test]
fn req_arch_013_a_public_rom_stays_under_the_announced_bound() {
    let Some(rom) = public_rom(NESTEST) else {
        return;
    };
    let mut harness = LoopHarness::create();
    harness
        .catalog()
        .add("nestest", Some(rom.to_str().unwrap()));

    harness.run_with(Command::LaunchGame, CommandArgs::of_text("nestest"));
    harness.run_frames(60);

    let bound = harness.host.max_save_state_size();
    harness.run_with(Command::SaveSlot, CommandArgs::of_number(2));

    let written = std::fs::metadata(harness.host.slot_state_file("2"))
        .unwrap()
        .len();
    assert!(
        written <= bound as u64,
        "{written} bytes written, bound announced {bound}"
    );

    harness.run(Command::Pause);
    let first = harness.host.capture_save_state("2").unwrap();
    let second = harness.host.capture_save_state("2").unwrap();
    assert_eq!(first.core, second.core);
    assert_eq!(first.conversion, second.conversion);
}
