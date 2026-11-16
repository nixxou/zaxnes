//! Lot FIDPLUS-D1e: with a console audio profile, the expansion audio's own band-limited signal
//! (`BandLimitedResampler::expansion_ring`) and the profile filters survive a savestate load and
//! run-ahead's in-memory restore, so the sound goes on as the machine would have made it.

#![allow(clippy::expect_used)]

mod common;

use common::mapper_fixture::MapperRom;
use zax_nes::state::CoreState;
use zax_nes::NesCore;

/// A mapper-69 cartridge (Sunsoft 5B) playing a square on channel A, with profile 1.
fn machine() -> NesCore {
    let image = MapperRom::new(69).image();
    let mut core = NesCore::new();
    let load = core.load_rom(&image, "fidplus-d1e.nes");
    assert!(load.success, "{:?}", load.error);
    core.set_console_audio_profile(1);
    for (register, value) in [(0x00u8, 0x40u8), (0x01, 0x01), (0x07, 0x3E), (0x08, 0x0F)] {
        core.write_memory(0xC000, register);
        core.write_memory(0xE000, value);
    }
    core
}

fn drain(core: &NesCore) -> Vec<f32> {
    let mut all = Vec::new();
    let mut buffer = [0.0f32; 4096];
    loop {
        let read = core.read_audio(&mut buffer, false).expect("mono read");
        if read == 0 {
            return all;
        }
        all.extend_from_slice(&buffer[..read]);
    }
}

fn worst(a: &[f32], b: &[f32]) -> f32 {
    assert_ne!(a.len(), 0);
    assert_eq!(a.len(), b.len());
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0f32, f32::max)
}

fn run(core: &mut NesCore, frames: u32) -> Vec<f32> {
    for _ in 0..frames {
        core.run_frame(0);
    }
    drain(core)
}

#[test]
fn fidplus_d1e_a_savestate_keeps_the_expansion_signal() {
    let mut original = machine();
    run(&mut original, 30);
    let state = original.save_state(false).expect("a state");

    let mut loaded = machine();
    run(&mut loaded, 3);
    loaded.try_load_state(&state, 0).expect("the state loads");
    drain(&loaded);

    let difference = worst(&run(&mut original, 20), &run(&mut loaded, 20));
    assert!(difference <= 0.0, "largest difference {difference}");
}

#[test]
fn fidplus_d1e_run_ahead_restore_keeps_the_profile_chain() {
    let mut twin = machine();
    let mut ahead = machine();
    run(&mut twin, 30);
    run(&mut ahead, 30);

    let mut state = CoreState::default();
    ahead.capture_core_state(&mut state).expect("a ROM is loaded");
    // Frames in advance whose sound nobody hears (drained and dropped), then back.
    let _ = run(&mut ahead, 3);
    ahead.restore_core_state(&state).expect("restores");

    let difference = worst(&run(&mut twin, 20), &run(&mut ahead, 20));
    assert!(difference <= 0.0, "largest difference {difference}");
}
