//! The APU's part of a savestate: all five channels, the frame counter and the output path
//! (REQ-CORE-074, REQ-CORE-090, REQ-CORE-092).

// Test helpers: a failed setup panics.
#![allow(clippy::unwrap_used)]

mod common;

use common::{machine_with, minimal};
use zax_nes::NesCore;

fn sounding_core() -> NesCore {
    let mut core = machine_with(&minimal(), "Apu.nes");

    // All five channels busy, so that every piece of channel state is non-trivial.
    for (address, value) in [
        (0x4000, 0x7F), // Pulse 1: duty 1, halt, envelope period 15.
        (0x4001, 0x8B), // Sweep enabled, period 0, negate, shift 3.
        (0x4002, 0x9A),
        (0x4003, 0xC9),
        (0x4004, 0xB6), // Pulse 2: duty 2, halt, constant volume 6.
        (0x4005, 0x08),
        (0x4006, 0x47),
        (0x4007, 0xA1),
        (0x4008, 0x55), // Triangle: control clear, linear reload 85.
        (0x400A, 0x37),
        (0x400B, 0x92),
        (0x400C, 0x3A), // Noise: halt, envelope period 10.
        (0x400E, 0x86), // Short mode, period index 6.
        (0x400F, 0x70),
        (0x4010, 0x4B), // DMC: loop, rate 11.
        (0x4011, 0x39),
        (0x4012, 0x10),
        (0x4013, 0x05),
        (0x4015, 0x1F), // Everything enabled.
        (0x4017, 0x00), // Four-step, frame IRQ enabled.
    ] {
        core.write_memory(address, value);
    }

    for _ in 0..40_000 {
        core.bus_mut().tick();
    }

    // The samples produced while setting up are read off, so that a capture starts at the
    // same point in the stream whether or not a state was restored in between: restoring
    // drops what the host has not read yet (REQ-CORE-074).
    let mut one = [0f32; 1];
    while core.read_audio(&mut one, false).unwrap() != 0 {}

    core
}

fn capture(core: &mut NesCore, cycles: usize) -> Vec<f32> {
    let mut samples = Vec::new();
    let mut block = [0f32; 64];

    for _ in 0..cycles {
        core.bus_mut().tick();
        if core.apu().available_samples() >= block.len() {
            core.read_audio(&mut block, false).unwrap();
            samples.extend_from_slice(&block);
        }
    }

    samples
}

#[test]
fn req_core_090_the_apu_state_survives_a_save_and_restore() {
    let mut core = sounding_core();

    let status = core.apu().peek_status();
    let level = core.apu().mixed_level();
    let state = core.save_state(true).unwrap();

    let expected = capture(&mut core, 60_000);
    assert_ne!(expected, [] as [f32; 0]);
    assert!(expected.iter().any(|&sample| sample != 0.0));

    core.try_load_state(&state, 0).unwrap();

    assert_eq!(status, core.apu().peek_status());
    assert_eq!(level.to_bits(), core.apu().mixed_level().to_bits());

    let replayed = capture(&mut core, 60_000);

    assert_eq!(expected, replayed);
}

#[test]
fn req_core_092_a_restored_state_replays_the_same_audio_in_a_fresh_instance() {
    let mut source = sounding_core();
    let state = source.save_state(true).unwrap();
    let expected = capture(&mut source, 60_000);

    // A block is self-contained (REQ-CORE-090): a brand new instance that has never seen the
    // ROM has to produce the same samples.
    let mut restored = NesCore::new();
    let loaded = restored.try_load_state(&state, 0).unwrap();
    assert_eq!("Apu.nes", loaded.rom_name);

    assert_eq!(expected, capture(&mut restored, 60_000));
}

#[test]
fn req_core_074_restoring_a_state_drops_the_samples_waiting_to_be_read() {
    let mut core = sounding_core();
    let state = core.save_state(true).unwrap();

    for _ in 0..10 {
        core.run_frame(0);
    }

    assert!(core.apu().available_samples() > 512);

    core.try_load_state(&state, 0).unwrap();

    assert_eq!(0, core.apu().available_samples());
    assert_eq!(0, core.read_audio(&mut [0f32; 512], false).unwrap());

    core.run_frame(0);
    assert_eq!(512, core.read_audio(&mut [0f32; 512], false).unwrap());
}

#[test]
fn req_core_074_reset_drops_the_samples_and_restarts_from_silence() {
    let mut core = sounding_core();
    for _ in 0..10 {
        core.run_frame(0);
    }

    assert!(core.apu().available_samples() > 512);

    core.reset();

    assert_eq!(0, core.apu().available_samples());
    assert_eq!(0, core.read_audio(&mut [0f32; 512], false).unwrap());

    core.run_frame(0);
    let mut buffer = [0f32; 512];
    assert_eq!(512, core.read_audio(&mut buffer, false).unwrap());
    assert!(buffer.iter().all(|&sample| sample == 0.0));
}

#[test]
fn req_core_005_closing_the_rom_stops_the_audio() {
    let mut core = sounding_core();
    core.run_frame(0);

    core.close_rom();

    assert_eq!(0, core.apu().available_samples());

    for _ in 0..10 {
        assert_eq!(0, core.run_frame(0));
    }

    assert_eq!(0, core.apu().available_samples());
}

#[test]
fn req_arch_013_the_announced_state_size_covers_the_apu() {
    let core = sounding_core();

    assert!(core.save_state(true).unwrap().len() <= core.max_state_size());
}
