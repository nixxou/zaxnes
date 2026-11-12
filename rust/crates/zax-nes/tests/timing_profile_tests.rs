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

use zax_core_api::Engine;
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

/// The invariant that says every NesHawk item stays behind the profile — in the form lot
/// NESHAWK-PROFILE-4 left it. The minimal ROM writes no `$4010`-`$4013` and no `$4015`, so its DMC
/// never asks for a byte and **every DMC item is silent on it**; the one difference such a ROM may
/// show is item 10's start-up frame, which is a PPU item and applies to every ROM whether it plays
/// a sample or not.
///
/// So the invariant is no longer "the same cycle count" but "the difference is **exactly** item
/// 10's, to the cycle". That is the stronger statement of the two: it still fails the moment a DMC
/// item reaches the common path, and it also fails if item 10's own cost ever stops being the one
/// thing a silent ROM can see.
///
/// Lot NESHAWK-PROFILE-5 step 3 restored this test. Item 10 (lot NESHAWK-PROFILE-4) had made it
/// fail — `595615` against `593227`, which is item 10's 7 165 dots and nothing else — and the
/// failure was not in that lot's suite figures.
#[test]
fn a_rom_that_plays_no_dpcm_differs_by_item_10_alone_on_the_two_profiles() {
    let standard = run_minimal::<Standard>(20);
    let neshawk = run_minimal::<NesHawkCompat>(20);

    // One start-up frame runs here — `power_on`'s — and a PPU dot is a third of a CPU cycle.
    let item_10 = NesHawkCompat::PPU_START_UP_DOTS_SKIPPED / 3;
    assert_eq!(
        0,
        Standard::PPU_START_UP_DOTS_SKIPPED,
        "the `Standard` profile runs no start-up frame, so the whole gap is the NesHawk one"
    );
    assert_eq!(
        item_10,
        standard.0 - neshawk.0,
        "20 frames of a ROM with no DPCM differ by item 10's start-up frame and by nothing else: \
         any other number means an item reached the common path"
    );
    assert!(standard.0 > 0 && neshawk.0 > 0, "the frames actually ran");
}

/// **Both profiles are an `Engine`** (step 4), from one generic `impl`, so the application and the
/// harness reach either through `Box<dyn Engine>` exactly as they reach the one engine today.
#[test]
fn each_profile_is_an_engine_of_its_own() {
    let engines: [Box<dyn Engine>; 2] = [
        Box::new(NesCoreOf::<Standard>::new()),
        Box::new(NesCoreOf::<NesHawkCompat>::new()),
    ];
    for engine in &engines {
        assert!(!engine.is_running(), "no ROM is loaded yet");
        assert_eq!(engine.frame_count(), 0);
    }

    // And the standard one is still reachable by the name every caller uses, which is what keeps
    // `EngineHandle::zaxnes()`'s downcast working.
    assert!(
        engines[0].as_any().downcast_ref::<NesCore>().is_some(),
        "`NesCore` is the standard engine"
    );
    assert!(
        engines[1].as_any().downcast_ref::<NesCore>().is_none(),
        "and the NesHawk engine is not it"
    );
}

/// A savestate of one profile is **refused** by the other, with the message REQ-CORE-120 fixes for
/// a block of a foreign core — because that is what it is: the two chips disagree about the DMC's
/// DMA by cycles, so a block of one restored into the other would drift from the next frame.
///
/// The refusal rides on the core name at the head of the block, so `NesCore::STATE_VERSION` did
/// not move and every block written before this lot still loads: the companion assertion below is
/// that a standard block is accepted by a standard core, which is the same path.
#[test]
fn a_savestate_of_one_profile_is_refused_by_the_other() {
    let mut standard = NesCoreOf::<Standard>::new();
    let mut neshawk = NesCoreOf::<NesHawkCompat>::new();
    assert!(standard.load_rom(&common::minimal(), "Minimal.nes").success);
    assert!(neshawk.load_rom(&common::minimal(), "Minimal.nes").success);

    let mut from_standard = Vec::new();
    let mut from_neshawk = Vec::new();
    Engine::save_state_to(&standard, false, &mut from_standard).expect("a ROM is loaded");
    Engine::save_state_to(&neshawk, false, &mut from_neshawk).expect("a ROM is loaded");

    // Each core reads its own block.
    assert!(standard.try_load_state(&from_standard, 0).is_ok());
    assert!(neshawk.try_load_state(&from_neshawk, 0).is_ok());

    // And refuses the other's, by name and with the spec's message.
    for (core_name, data) in [("standard", &from_neshawk), ("neshawk", &from_standard)] {
        let refused = if core_name == "standard" {
            standard.try_load_state(data, 0)
        } else {
            neshawk.try_load_state(data, 0)
        };
        let error = refused.expect_err("the other profile's block is refused");
        assert_eq!(
            error.message,
            NesCore::FOREIGN_CORE_MESSAGE,
            "{core_name}: the spec's own message, to the character"
        );
    }
}
