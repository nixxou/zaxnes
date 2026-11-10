//! The two timing profiles of the console chip (lot NESHAWK-PROFILE step 2).
//!
//! What this file checks is the **architecture**, not the NesHawk differences themselves: that the
//! core is generic over a profile, that both instantiations build and run, and that on a game the
//! two profiles cannot disagree about they produce the same machine to the cycle. The differences
//! are `neshawk_profile_tests.rs`.
//!
//! The last point is the durable one. Every item the NesHawk profile carries is about the DMC's
//! sample-fetch DMA, so a ROM that never starts a DPCM sample must run identically on both
//! profiles **for ever** — and that is the invariant which says a new item was put behind the
//! profile and not in front of it.
//!
//! No `REQ-` identifier: the two profiles have no requirement yet, and the wording proposed for
//! one is in `docs/progress/NESHAWK-PROFILE.md`.

mod common;

use zax_nes::{NesCore, NesCoreOf, NesHawkCompat, Standard, TimingProfile};

/// Runs `frames` frames of the minimal ROM on profile `P` and reports the CPU cycle count and a
/// checksum of the last picture.
fn run_minimal<P: TimingProfile>(frames: u32) -> (i64, u64)
where
    NesCoreOf<P>: ProfiledFrame,
{
    let mut core = NesCoreOf::<P>::new();
    assert!(
        core.load_rom(&common::minimal(), "Minimal.nes").success,
        "the minimal ROM loads on the {} profile",
        P::NAME
    );

    for _ in 0..frames {
        core.run_frame_for_test(0);
    }

    let mut picture = Vec::new();
    core.pixel_frame_indices_into(&mut picture);
    let checksum = picture.iter().enumerate().fold(0u64, |acc, (i, &byte)| {
        acc ^ ((u64::from(byte) + 1) << (i % 56))
    });
    (core.cycle_count(), checksum)
}

/// `run_frame` is declared once per profile rather than on the generic `impl` (see the note on
/// `NesCoreOf<Standard>`), so a generic test needs a name that reaches both.
trait ProfiledFrame {
    fn run_frame_for_test(&mut self, input_word: u16) -> i64;
}

impl ProfiledFrame for NesCoreOf<Standard> {
    fn run_frame_for_test(&mut self, input_word: u16) -> i64 {
        self.run_frame(input_word)
    }
}

impl ProfiledFrame for NesCoreOf<NesHawkCompat> {
    fn run_frame_for_test(&mut self, input_word: u16) -> i64 {
        self.run_frame(input_word)
    }
}

#[test]
fn the_two_profiles_are_named_and_numbered_apart() {
    assert_eq!(Standard::NAME, "standard");
    assert_eq!(NesHawkCompat::NAME, "neshawk");
    assert_eq!(Standard::STATE_ID, 0);
    assert_eq!(NesHawkCompat::STATE_ID, 1);
    assert_ne!(
        Standard::STATE_ID,
        NesHawkCompat::STATE_ID,
        "a savestate tells the two profiles apart by this byte"
    );
}

/// `NesCore` is the `Standard` instantiation and nothing else, which is what keeps every caller
/// outside this crate — and every savestate — unchanged.
#[test]
fn the_plain_core_name_is_the_standard_profile() {
    let plain: NesCore = NesCore::new();
    let spelled_out: NesCoreOf<Standard> = NesCoreOf::<Standard>::new();
    assert_eq!(
        std::mem::size_of_val(&plain),
        std::mem::size_of_val(&spelled_out),
        "`NesCore` and `NesCoreOf<Standard>` are one type"
    );
}

/// A zero-sized profile costs the machine no state: the two cores are the same size, and the
/// profile is in the code rather than in the struct.
#[test]
fn a_profile_costs_the_core_no_memory() {
    assert_eq!(std::mem::size_of::<Standard>(), 0);
    assert_eq!(std::mem::size_of::<NesHawkCompat>(), 0);
    assert_eq!(
        std::mem::size_of::<NesCoreOf<Standard>>(),
        std::mem::size_of::<NesCoreOf<NesHawkCompat>>(),
        "the profile is a type, so it occupies nothing in either core"
    );
}

/// The invariant that says every NesHawk item stays behind the profile: the minimal ROM writes no
/// `$4010`-`$4013` and no `$4015`, so its DMC never asks for a byte, and the two profiles have
/// nothing to disagree about. If this test ever fails, an item reached the common path.
#[test]
fn a_rom_that_plays_no_dpcm_runs_identically_on_both_profiles() {
    let standard = run_minimal::<Standard>(20);
    let neshawk = run_minimal::<NesHawkCompat>(20);

    assert_eq!(
        standard.0, neshawk.0,
        "20 frames of a ROM with no DPCM take the same CPU cycles on both profiles"
    );
    assert_eq!(standard.1, neshawk.1, "and leave the same picture behind");
    assert!(standard.0 > 0, "the frames actually ran");
}
