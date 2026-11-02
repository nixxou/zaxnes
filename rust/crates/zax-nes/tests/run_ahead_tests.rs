//! The frame sequence of run-ahead inside the core (AMD-041 REQ-LOOP-065): the real frame and
//! the frames in advance, what each of them produces, and the restore that ends them.
//!
//! The acceptance criterion of REQ-LOOP-065 is the picture of the vector ROM, which is checked
//! here (the core owns the sequence) and again through the loop in `zax-app`.

// Test helpers: a failed setup panics.
#![allow(clippy::unwrap_used)]

mod common;

use common::run_ahead_rom as rom;
use common::{machine_with, minimal_rom};
use zax_nes::NesCore;

/// Button A of port 1, in the input word of REQ-CORE-022.
const BUTTON_A: u16 = 0x0001;

/// The vector ROM of REQ-LOOP-065, past its two-vblank warm-up and with the NMI running: the
/// counter then advances by one per emulated frame.
fn vector_core() -> NesCore {
    let mut core = machine_with(&rom::build(), "RunAhead.nes");
    // Four frames: the warm-up loop needs two vblanks, then the first NMI prints.
    for _ in 0..4 {
        core.run_frame(0);
    }
    core
}

fn counter(core: &NesCore) -> u32 {
    u32::from(core.read_memory(rom::COUNTER_ADDRESS))
}

/// REQ-LOOP-065, the acceptance criterion. Run-ahead = 2: just before the real frame k the
/// core's counter is k, button A is pressed on that frame, the displayed picture shows the
/// counter k + 2 and A pressed, and after step 5 the core's counter is **k + 1** — the one of the
/// real frame that has just run.
#[test]
fn req_loop_065_the_displayed_frame_is_two_frames_ahead_of_the_real_one() {
    let mut core = vector_core();
    let k = counter(&core);
    assert!(k >= 1, "the fixture never printed");

    core.run_frame_ahead(BUTTON_A, 2);

    assert_eq!(
        k + 2,
        rom::displayed(&core),
        "the displayed picture is not the counter k + 2"
    );
    assert!(
        rom::shows_button_a(&core),
        "the displayed picture does not show A pressed"
    );

    assert!(core.holds_a_real_frame());
    assert!(core.restore_real_frame());
    assert_eq!(
        k + 1,
        counter(&core),
        "after step 5 the core is not on the real frame"
    );
    assert!(!core.holds_a_real_frame());
}

/// The same criterion at the three values the setting offers, and at 0.
#[test]
fn req_loop_065_the_picture_is_n_frames_ahead_for_every_n() {
    for ahead in 0..=3_u8 {
        let mut core = vector_core();
        let k = counter(&core);

        core.run_frame_ahead(BUTTON_A, ahead);

        // The real frame prints the counter it had on entry (k), each frame in advance prints the
        // one after, so the displayed frame — the N-th in advance — prints k + N. At N = 0 the
        // single frame that runs is the real one and prints k.
        let shown = rom::displayed(&core);
        assert_eq!(
            k + u32::from(ahead),
            shown,
            "N = {ahead}: the picture shows {shown}, not {}",
            k + u32::from(ahead)
        );
        core.restore_real_frame();
        assert_eq!(
            k + 1,
            counter(&core),
            "N = {ahead}: the real state is not that of frame k + 1"
        );
    }
}

/// N = 0 is [`NesCore::run_frame`] and nothing else: the same frame, no state held, no restore
/// owed. This is the "N = 0 costs nothing" rule, stated as behaviour.
#[test]
fn req_loop_065_n_zero_is_the_ordinary_frame() {
    let mut plain = vector_core();
    let mut zero = vector_core();

    for frame in 0..30_u16 {
        let word = frame.wrapping_mul(0x0123);
        plain.run_frame(word);
        zero.run_frame_ahead(word, 0);

        assert!(
            !zero.holds_a_real_frame(),
            "frame {frame}: a state was held"
        );
        assert!(!zero.restore_real_frame());
        assert_eq!(plain.frame_count(), zero.frame_count());
        assert_eq!(plain.bus().ram(), zero.bus().ram());
        assert_eq!(plain.pixel_frame(), zero.pixel_frame());
    }
}

/// REQ-LOOP-065: one real frame of run-ahead advances the core's frame counter by exactly one,
/// whatever N says, because step 5 puts the real state back (REQ-LOOP-066).
#[test]
fn req_loop_066_the_core_counts_one_frame_per_real_frame() {
    for ahead in 0..=3_u8 {
        let mut core = vector_core();
        let start = core.frame_count();

        for _ in 0..20 {
            core.run_frame_ahead(0, ahead);
            core.restore_real_frame();
        }

        assert_eq!(
            start + 20,
            core.frame_count(),
            "N = {ahead}: the core counted the frames in advance"
        );
    }
}

/// REQ-LOOP-065 steps 1 and 3: the frames that are not displayed produce no sound. The displayed
/// one does, and the restore of step 5 leaves what it produced alone (REQ-CORE-121).
#[test]
fn req_loop_065_only_the_displayed_frame_is_heard() {
    // A square at full volume, so every frame would queue samples if it were heard.
    let program = [
        0xA9, 0x01, // LDA #$01
        0x8D, 0x15, 0x40, // STA $4015
        0xA9, 0xBF, // LDA #$BF
        0x8D, 0x00, 0x40, // STA $4000
        0xA9, 0x40, // LDA #$40
        0x8D, 0x02, 0x40, // STA $4002
        0xA9, 0x08, // LDA #$08
        0x8D, 0x03, 0x40, // STA $4003
        0x4C, 0x14, 0x80, // JMP $8014
    ];
    let mut core = machine_with(&minimal_rom(0x01, Some(&program), None), "Square.nes");
    for _ in 0..3 {
        core.run_frame(0);
    }

    // One ordinary frame, to learn what one frame of sound is worth here.
    let before = core.apu().available_samples();
    core.run_frame(0);
    let one_frame = core.apu().available_samples() - before;
    assert!(one_frame > 0, "the fixture produced no sound");

    // Four emulated frames (the real one, two in between, the displayed one) must queue the
    // samples of **one**.
    let before = core.apu().available_samples();
    core.run_frame_ahead(0, 3);
    let queued = core.apu().available_samples() - before;
    assert!(
        queued.abs_diff(one_frame) <= one_frame / 8,
        "four emulated frames queued {queued} samples, one frame is worth {one_frame}"
    );

    core.restore_real_frame();
    assert_eq!(
        before + queued,
        core.apu().available_samples(),
        "the restore dropped the sound of the displayed frame"
    );
    // And the switch is back on, so an ordinary frame is heard again.
    assert!(core.apu().output_enabled());
    core.run_frame(0);
    assert!(core.apu().available_samples() > before + queued);
}

/// REQ-LOOP-067: the host is handed exactly **one** frame per real frame, the displayed one. The
/// video callback of REQ-CORE-020 is what the conversion and the Lua frame hooks hang off, so it
/// must fire once and carry the displayed frame's records.
#[test]
fn req_loop_067_the_video_callback_fires_once_per_real_frame() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    let calls = Arc::new(AtomicUsize::new(0));
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut core = vector_core();
    {
        let calls = Arc::clone(&calls);
        let seen = Arc::clone(&seen);
        core.set_video_callback(Some(Box::new(move |batch| {
            calls.fetch_add(1, Ordering::Relaxed);
            // The first background record is the top-left cell (capture order, REQ-CORE-044),
            // and plane 0 of pattern n is eight bytes equal to n (SPEC-02 §5.1): its low byte is
            // the counter the picture shows.
            if let Some(record) = batch.records.iter().find(|record| record.background) {
                #[allow(clippy::cast_possible_truncation)] // The low byte, deliberately.
                seen.lock()
                    .unwrap()
                    .push(u32::from((record.plane0 & 0xFF) as u8));
            }
        })));
    }

    let k = counter(&core);
    for round in 0..5_u32 {
        calls.store(0, Ordering::Relaxed);
        core.run_frame_ahead(0, 3);
        assert_eq!(
            1,
            calls.load(Ordering::Relaxed),
            "round {round}: the hidden frames were handed over too"
        );
        core.restore_real_frame();
    }

    let indices = seen.lock().unwrap().clone();
    assert_eq!(5, indices.len());
    // Each displayed frame is three ahead of its real one, and the real frames are contiguous.
    #[allow(clippy::cast_possible_truncation)] // Small counters.
    for (round, index) in indices.iter().enumerate() {
        assert_eq!(
            k + 3 + round as u32,
            *index,
            "round {round}: the batch is not the displayed frame's"
        );
    }
}

/// REQ-LOOP-067: the core stays in the displayed frame's state until the restore, so the RAM a
/// script reads with `Script:ReadMem` during the frame hooks is the displayed frame's.
#[test]
fn req_loop_067_the_ram_read_before_the_restore_is_the_displayed_frame_s() {
    let mut core = vector_core();
    let k = counter(&core);

    core.run_frame_ahead(0, 2);
    // What a host reads between step 4 and step 5.
    assert_eq!(
        k + 2,
        u32::from(core.read_memory(rom::DISPLAYED_ADDRESS)),
        "the RAM is not the displayed frame's"
    );
    assert_eq!(k + 3, counter(&core), "the counter already moved on");

    core.restore_real_frame();
    assert_eq!(k + 1, counter(&core));
}

/// The sequence is faithful: the real states it leaves behind are the ones a run with no
/// run-ahead at all would have produced, frame for frame, picture included.
#[test]
fn req_arch_012_the_real_frames_are_those_of_a_run_without_run_ahead() {
    for ahead in 1..=3_u8 {
        let mut plain = vector_core();
        let mut ahead_core = vector_core();

        for frame in 0..60_u16 {
            let word = if frame % 3 == 0 { BUTTON_A } else { 0 };
            plain.run_frame(word);
            ahead_core.run_frame_ahead(word, ahead);
            ahead_core.restore_real_frame();

            assert_eq!(
                plain.frame_count(),
                ahead_core.frame_count(),
                "N = {ahead}, frame {frame}: the counters parted"
            );
            assert_eq!(
                plain.bus().ram(),
                ahead_core.bus().ram(),
                "N = {ahead}, frame {frame}: the work RAM parted"
            );
        }
    }
}

/// A host that forgets its restore cannot lose the real state: the next sequence puts it back
/// first, so the real frames stay contiguous.
#[test]
fn a_forgotten_restore_is_made_good_by_the_next_sequence() {
    let mut plain = vector_core();
    let mut sloppy = vector_core();

    for _ in 0..20 {
        plain.run_frame(0);
        sloppy.run_frame_ahead(0, 2);
        // No `restore_real_frame` here at all.
    }
    sloppy.restore_real_frame();

    assert_eq!(plain.frame_count(), sloppy.frame_count());
    assert_eq!(plain.bus().ram(), sloppy.bus().ram());
}

/// REQ-LOOP-066: a reset, a soft reset, a state load and a closed ROM all drop the state of a
/// real frame instead of putting it back — the machine it belonged to is gone.
#[test]
fn req_loop_066_a_reset_or_a_state_load_drops_the_frame_it_held() {
    let mut core = vector_core();
    core.run_frame_ahead(0, 2);
    assert!(core.holds_a_real_frame());
    core.reset();
    assert!(!core.holds_a_real_frame());
    assert_eq!(0, core.frame_count());

    for _ in 0..4 {
        core.run_frame(0);
    }
    core.run_frame_ahead(0, 2);
    assert!(core.holds_a_real_frame());
    core.soft_reset();
    assert!(!core.holds_a_real_frame());

    for _ in 0..4 {
        core.run_frame(0);
    }
    let block = core.save_state(true).unwrap();
    core.run_frame_ahead(0, 3);
    assert!(core.holds_a_real_frame());
    core.try_load_state(&block, 0).unwrap();
    assert!(!core.holds_a_real_frame());
    // And the frame after the load is the loaded state's own, not the dropped one.
    let loaded = core.frame_count();
    core.run_frame_ahead(0, 3);
    core.restore_real_frame();
    assert_eq!(loaded + 1, core.frame_count());

    core.run_frame_ahead(0, 1);
    assert!(core.holds_a_real_frame());
    core.close_rom();
    assert!(!core.holds_a_real_frame());
    assert!(core.apu().output_enabled());
}

/// A value above the setting's domain cannot make the core run more frames than REQ-LOOP-064
/// allows, whatever a caller passes.
#[test]
fn req_loop_064_the_core_never_runs_more_than_three_frames_ahead() {
    let mut core = vector_core();
    let k = counter(&core);
    core.run_frame_ahead(0, 200);
    assert_eq!(
        k + u32::from(NesCore::MAX_RUN_AHEAD),
        rom::displayed(&core),
        "a caller's out-of-range value was not clamped"
    );
    core.restore_real_frame();
    assert_eq!(k + 1, counter(&core));
}

/// With no ROM loaded the sequence does nothing at all, like [`NesCore::run_frame`]
/// (REQ-CORE-005).
#[test]
fn a_stopped_core_runs_no_frame_ahead() {
    let mut core = NesCore::new();
    assert_eq!(0, core.run_frame_ahead(BUTTON_A, 3));
    assert!(!core.holds_a_real_frame());
    assert_eq!(0, core.frame_count());
}
