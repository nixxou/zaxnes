//! The normative answers of AMD-002 to the L1a question tickets: typed failure causes
//! (Q-001) and the frame-length tolerance after a region change (Q-003).

// Test helpers: a failed setup panics.
#![allow(clippy::unwrap_used)]

mod common;

use common::{machine_with, minimal};
use zax_nes::rom::RomLoadFailure;
use zax_nes::state::StateLoadFailure;
use zax_nes::{NesCore, NesRegion, RegionTiming};

// ---------------------------------------------------------------- Q-001

#[test]
fn q001_a_corrupt_header_reports_the_typed_cause() {
    let mut core = NesCore::new();

    let too_short = core.load_rom(&[0; 16], "Zeros.nes");
    assert!(!too_short.success);
    assert_eq!(RomLoadFailure::CorruptHeader, too_short.failure);
    assert!(too_short.error.is_some());

    let empty = core.load_rom(&[], "Empty.nes");
    assert_eq!(RomLoadFailure::CorruptHeader, empty.failure);
}

#[test]
fn q001_a_truncated_image_reports_the_typed_cause() {
    let image = minimal();
    let mut core = NesCore::new();

    let result = core.load_rom(&image[..image.len() - 1], "Short.nes");

    assert!(!result.success);
    assert_eq!(RomLoadFailure::TruncatedFile, result.failure);
}

#[test]
fn q001_an_unsupported_mapper_reports_the_typed_cause_and_still_names_the_number() {
    // Mapper 200: high nibble of byte 6 is 8, high nibble of byte 7 is 12 -> 0xC8 = 200.
    let mut image = minimal();
    image[6] = (image[6] & 0x0F) | 0x80;
    image[7] = (image[7] & 0x0F) | 0xC0;

    let mut core = NesCore::new();
    let result = core.load_rom(&image, "Mapper200.nes");

    assert!(!result.success);
    assert_eq!(RomLoadFailure::UnsupportedMapper, result.failure);

    // REQ-CORE-011 only requires the number to appear in the message; the typed cause above
    // is what AMD-002 makes normative.
    assert!(result.error.unwrap().contains("200"));
}

#[test]
fn q001_a_successful_load_reports_no_failure() {
    let mut core = NesCore::new();
    let result = core.load_rom(&minimal(), "Minimal.nes");

    assert!(result.success);
    assert_eq!(RomLoadFailure::None, result.failure);
    assert!(result.error.is_none());
}

#[test]
fn q001_a_corrupt_savestate_reports_the_typed_cause() {
    let mut core = machine_with(&minimal(), "Minimal.nes");
    core.run_frame(0);

    let state = core.save_state(true).unwrap();
    let halved = &state[..state.len() / 2];

    let error = core.try_load_state(halved, 0).unwrap_err();

    assert_eq!(StateLoadFailure::CorruptState, error.failure);
    assert!(!error.message.is_empty());
    assert!(!core.is_running());
}

#[test]
fn q001_a_block_from_another_core_reports_the_foreign_core_cause_and_the_literal_message() {
    let mut core = machine_with(&minimal(), "Minimal.nes");
    core.run_frame(0);

    let mut state = core.save_state(true).unwrap();

    // The core name is a length-prefixed UTF-8 string right after the magic and the format
    // version; flipping one of its bytes makes the block foreign.
    state[12] ^= 0x20;

    let error = core.try_load_state(&state, 0).unwrap_err();

    assert_eq!(StateLoadFailure::ForeignCore, error.failure);

    // REQ-CORE-120 is one of the few literal strings of the spec.
    assert_eq!("Save state used a different core.", error.message);
}

#[test]
fn q001_a_successful_restore_reports_no_failure() {
    let mut core = machine_with(&minimal(), "Minimal.nes");
    core.run_frame(0);

    let state = core.save_state(true).unwrap();

    assert!(core.try_load_state(&state, 0).is_ok());
}

// ---------------------------------------------------------------- Q-003

#[test]
fn q003_the_first_frame_after_a_region_change_stays_under_two_nominal_frames() {
    for (from, to) in [
        (NesRegion::Ntsc, NesRegion::Dendy),
        (NesRegion::Dendy, NesRegion::Ntsc),
        (NesRegion::Ntsc, NesRegion::Pal),
        (NesRegion::Pal, NesRegion::Dendy),
    ] {
        let mut core = machine_with(&minimal(), "Minimal.nes");

        core.set_region(from);
        for _ in 0..5 {
            core.run_frame(0);
        }

        core.set_region(to);

        // AMD-002 (answer to Q-003): the transition frame may be of any length as long as it
        // does not exceed two nominal frames of the new region; the tolerance of REQ-CORE-015
        // applies from the frame after it.
        let transition = core.run_frame(0);
        let timing = RegionTiming::for_region(to);
        let nominal = i64::from(timing.cpu_clock_hz / timing.nominal_frames_per_second);

        assert!(
            (1..=2 * nominal).contains(&transition),
            "{from:?} -> {to:?}: {transition}"
        );

        let frames = 300;
        let total: i64 = (0..frames).map(|_| core.run_frame(0)).sum();

        let expected = match to {
            NesRegion::Pal => 33247.5,
            NesRegion::Dendy => 35464.0,
            _ => 29780.5,
        };

        #[allow(clippy::cast_precision_loss)] // Cycle counts are far below 2^52.
        let average = total as f64 / f64::from(frames);
        assert!(
            ((expected - 1.0)..=(expected + 1.0)).contains(&average),
            "{from:?} -> {to:?}: {average}"
        );
    }
}
