//! Savestates: self-containment, partial restore, failure modes and determinism
//! (REQ-CORE-090 to REQ-CORE-093, REQ-ARCH-012, REQ-ARCH-013).

// Test helpers: a failed setup panics.
#![allow(clippy::unwrap_used)]

mod common;

use common::{machine_with, minimal_rom, TestRandom};
use zax_nes::state::StateLoadFailure;
use zax_nes::{NesBus, NesCore, NesRegion};

/// A ROM whose loop stirs work RAM, so that state differences are visible.
const COUNTER_PROGRAM: [u8; 10] = [
    0xE6, 0x20, // INC $20
    0xA5, 0x20, // LDA $20
    0x8D, 0x00, 0x03, // STA $0300
    0x4C, 0x00, 0x80, // JMP $8000
];

fn loaded_core(name: &str) -> NesCore {
    machine_with(&minimal_rom(0x01, Some(&COUNTER_PROGRAM), None), name)
}

/// Work RAM plus a few machine registers, as a comparable blob.
fn snapshot(core: &NesCore) -> Vec<u8> {
    let mut blob = core.bus().ram().to_vec();
    let cpu = core.cpu();
    blob.extend_from_slice(&[
        cpu.a,
        cpu.x,
        cpu.y,
        cpu.s,
        cpu.p(),
        cpu.pc.to_le_bytes()[0],
        cpu.pc.to_le_bytes()[1],
        core.frame_count().to_le_bytes()[0],
    ]);
    assert_eq!(NesBus::RAM_SIZE + 8, blob.len());
    blob
}

fn run(core: &mut NesCore, frames: usize) {
    for _ in 0..frames {
        core.run_frame(0);
    }
}

#[test]
fn req_core_090_a_state_block_is_self_contained() {
    let mut core = loaded_core("Minimal.nes");
    run(&mut core, 30);

    let state = core.save_state(true).unwrap();
    let expected_ram = snapshot(&core);

    // Carry on in a reference run.
    let mut reference = NesCore::new();
    reference.try_load_state(&state, 0).unwrap();
    run(&mut reference, 20);
    let reference_ram = snapshot(&reference);

    // Now close the ROM and restore without reloading it: emulation must resume
    // identically (REQ-CORE-090).
    core.close_rom();
    assert!(!core.is_running());

    let loaded = core.try_load_state(&state, 0).unwrap();
    assert!(core.is_running());
    assert_eq!("Minimal.nes", loaded.rom_name);
    assert_eq!(expected_ram, snapshot(&core));

    run(&mut core, 20);
    assert_eq!(reference_ram, snapshot(&core));
}

#[test]
fn req_core_091_restoring_at_an_offset_reports_the_bytes_consumed() {
    let mut core = loaded_core("Minimal.nes");
    run(&mut core, 10);

    let state = core.save_state(true).unwrap();
    let mut framed = vec![0u8; 7];
    framed.extend_from_slice(&state);
    framed.extend_from_slice(&[0u8; 100]);

    let loaded = core.try_load_state(&framed, 7).unwrap();

    assert_eq!(state.len(), loaded.consumed);
    assert_eq!("Minimal.nes", loaded.rom_name);
}

#[test]
fn req_core_091_a_truncated_block_fails_and_leaves_the_core_stopped() {
    let mut core = loaded_core("Minimal.nes");
    core.run_frame(0);

    let state = core.save_state(true).unwrap();
    let halved = &state[..state.len() / 2];

    let error = core.try_load_state(halved, 0).unwrap_err();

    assert_eq!(StateLoadFailure::CorruptState, error.failure);
    assert!(!core.is_running());

    // And a frame on a stopped core still does nothing (REQ-CORE-005).
    assert_eq!(0, core.run_frame(0));
}

#[test]
fn req_core_091_a_block_with_the_wrong_magic_fails() {
    let mut core = loaded_core("Minimal.nes");
    let mut state = core.save_state(true).unwrap();
    state[0] ^= 0xFF;

    let error = core.try_load_state(&state, 0).unwrap_err();
    assert!(error.message.to_lowercase().contains("magic"), "{error}");
}

#[test]
fn req_core_120_a_block_from_another_core_is_refused_with_the_spec_message() {
    let mut core = loaded_core("Minimal.nes");
    let mut state = core.save_state(true).unwrap();

    // The core name is stored as a length-prefixed UTF-8 string right after the magic and
    // the version; corrupt its first character.
    state[4 + 4 + 4] = b'X';

    let error = core.try_load_state(&state, 0).unwrap_err();

    assert_eq!(StateLoadFailure::ForeignCore, error.failure);
    assert_eq!(NesCore::FOREIGN_CORE_MESSAGE, error.message);
    assert_eq!("Save state used a different core.", error.message);
}

#[test]
fn req_acc_041_random_bytes_never_crash_the_restore() {
    let mut random = TestRandom::new(99);
    let mut core = loaded_core("Minimal.nes");

    for _ in 0..300 {
        let mut junk = vec![0u8; random.below(512)];
        random.fill(&mut junk);
        // Random bytes are almost surely refused; the point is that nothing panics.
        if core.try_load_state(&junk, 0).is_err() {
            assert!(!core.is_running());
            core = loaded_core("Minimal.nes");
        }
    }
}

#[test]
fn req_core_091_an_offset_outside_the_array_fails_cleanly() {
    let mut core = loaded_core("Minimal.nes");

    let error = core.try_load_state(&[0u8; 10], 50).unwrap_err();
    assert_eq!(StateLoadFailure::CorruptState, error.failure);
}

fn run_and_hash(save_restore_at_frame: Option<usize>) -> u64 {
    let mut core = loaded_core("Minimal.nes");
    let mut random = TestRandom::new(20_261_020);
    let mut hash: u64 = 1_469_598_103_934_665_603;

    for frame in 0..1000 {
        core.run_frame(random.next_u16());

        if save_restore_at_frame == Some(frame) {
            let state = core.save_state(true).unwrap();
            core.try_load_state(&state, 0).unwrap();
        }

        for value in snapshot(&core) {
            hash = (hash ^ u64::from(value)).wrapping_mul(1_099_511_628_211);
        }
    }

    hash
}

#[test]
fn req_core_092_save_and_restore_at_frame_500_does_not_change_the_run() {
    // 1000 frames of fixed pseudo-random input, hashed; then the same run with a save and
    // restore at frame 500 (REQ-CORE-092).
    assert_eq!(run_and_hash(None), run_and_hash(Some(500)));
}

#[test]
fn req_core_092_the_same_inputs_give_the_same_frames_memory_and_audio() {
    // Two instances, same ROM, same input sequence: memory and samples must match exactly
    // (REQ-CORE-092, REQ-ARCH-012).
    let mut first = loaded_core("Minimal.nes");
    let mut second = loaded_core("Minimal.nes");
    let mut random = TestRandom::new(777);

    let mut first_audio = Vec::new();
    let mut second_audio = Vec::new();
    let mut buffer = [0f32; 512];

    for _ in 0..300 {
        let word = random.next_u16();
        assert_eq!(first.run_frame(word), second.run_frame(word));
        assert_eq!(snapshot(&first), snapshot(&second));

        while first.read_audio(&mut buffer, false).unwrap() != 0 {
            first_audio.extend_from_slice(&buffer);
        }
        while second.read_audio(&mut buffer, false).unwrap() != 0 {
            second_audio.extend_from_slice(&buffer);
        }
    }

    assert!(!first_audio.is_empty());
    assert_eq!(first_audio, second_audio);
}

#[test]
fn req_core_092_saving_and_restoring_immediately_changes_nothing() {
    let mut core = loaded_core("Minimal.nes");
    for i in 0..17 {
        core.run_frame(i);
    }

    let before = snapshot(&core);
    let state = core.save_state(true).unwrap();

    core.try_load_state(&state, 0).unwrap();

    assert_eq!(before, snapshot(&core));
    assert_eq!(state, core.save_state(true).unwrap());
}

#[test]
fn req_arch_013_a_savestate_round_trip_replays_the_same_frames() {
    let mut core = loaded_core("Minimal.nes");
    run(&mut core, 40);

    let state = core.save_state(true).unwrap();

    let mut first = Vec::new();
    for i in 0..25u16 {
        core.run_frame(i * 7);
        first.push(snapshot(&core));
    }

    core.try_load_state(&state, 0).unwrap();

    for (i, expected) in (0..25u16).zip(&first) {
        core.run_frame(i * 7);
        assert_eq!(*expected, snapshot(&core));
    }
}

#[test]
fn req_arch_013_the_announced_size_bounds_the_real_one() {
    let mut core = loaded_core("Minimal.nes");
    let announced = core.max_state_size();

    for i in 0..20 {
        core.run_frame(i);
        assert!(core.save_state(true).unwrap().len() <= announced);

        // The bound stays valid for the whole session.
        assert_eq!(announced, core.max_state_size());
    }
}

#[test]
fn req_core_093_a_lightweight_block_leaves_the_rom_image_out() {
    let mut core = loaded_core("Minimal.nes");
    core.run_frame(0);

    let full = core.save_state(true).unwrap();
    let light = core.save_state(false).unwrap();

    assert!(light.len() < full.len() - 32 * 1024);

    // It still restores, as long as the ROM is loaded.
    core.try_load_state(&light, 0).unwrap();
    assert!(core.is_running());
}

#[test]
fn req_core_093_a_lightweight_block_is_refused_when_no_rom_is_loaded() {
    let mut core = loaded_core("Minimal.nes");
    core.run_frame(0);
    let light = core.save_state(false).unwrap();

    let mut empty = NesCore::new();
    let error = empty.try_load_state(&light, 0).unwrap_err();
    assert!(
        error.message.to_lowercase().contains("no rom image"),
        "{error}"
    );
}

#[test]
fn req_core_090_saving_without_a_rom_is_refused() {
    let core = NesCore::new();
    assert!(core.save_state(true).is_err());
}

#[test]
fn req_core_014_the_stored_rom_name_comes_back_after_loading_another_rom() {
    let mut core = machine_with(&common::minimal(), "Test.nes");
    core.run_frame(0);

    let state = core.save_state(true).unwrap();

    assert!(
        core.load_rom(&minimal_rom(0x00, None, None), "Other.nes")
            .success
    );
    assert_eq!(Some("Other.nes"), core.rom_name());

    let loaded = core.try_load_state(&state, 0).unwrap();
    assert_eq!("Test.nes", loaded.rom_name);
    assert_eq!(Some("Test.nes"), core.rom_name());
}

#[test]
fn req_core_074_restoring_a_state_drops_the_pending_audio() {
    let mut core = loaded_core("Minimal.nes");
    core.run_frame(0);
    let state = core.save_state(true).unwrap();

    run(&mut core, 10);
    assert!(core.apu().available_samples() > 512);

    core.try_load_state(&state, 0).unwrap();

    assert_eq!(0, core.apu().available_samples());
    assert_eq!(0, core.read_audio(&mut [0f32; 512], false).unwrap());
}

#[test]
fn req_core_074_reset_drops_the_pending_audio() {
    let mut core = loaded_core("Minimal.nes");
    run(&mut core, 10);
    assert!(core.apu().available_samples() > 0);

    core.reset();
    assert_eq!(0, core.apu().available_samples());
}

#[test]
fn req_core_090_the_region_is_part_of_the_state() {
    let mut core = loaded_core("Minimal.nes");
    core.set_region(NesRegion::Pal);
    core.run_frame(0);

    let state = core.save_state(true).unwrap();

    core.set_region(NesRegion::Ntsc);
    core.try_load_state(&state, 0).unwrap();

    assert_eq!(NesRegion::Pal, core.region());
}
