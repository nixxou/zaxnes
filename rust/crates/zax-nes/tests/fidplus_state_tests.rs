//! Fidelite+ part A step 4: a state taken with PPU edges pending carries them
//! (`Ppu2C02::save_edge_state`), in a savestate file and in run-ahead's in-memory state.

#![allow(clippy::expect_used)]

mod common;

use common::{machine_with, minimal_rom};
use zax_nes::state::CoreState;
use zax_nes::NesCore;

/// Rendering off, then `$2007` written on every pass of a seven-cycle loop: a store edge (six
/// dots) is often still pending when a frame ends.
const PROGRAM: [u8; 16] = [
    0x78, 0xD8, // SEI, CLD
    0xA9, 0x00, 0x8D, 0x01, 0x20, // $2001 = 0
    0xA2, 0x00, // LDX #0
    0x8E, 0x07, 0x20, // loop: STX $2007
    0xE8, // INX
    0x4C, 0x09, 0x80, // JMP loop
];

fn machine() -> NesCore {
    let image = minimal_rom(0x00, Some(&PROGRAM), None);
    let mut core = machine_with(&image, "fidplus-state.nes");
    core.set_fidelity_plus(true);
    core.reset();
    core
}

fn run(core: &mut NesCore, frames: u32) -> (Vec<u8>, Vec<u8>) {
    for _ in 0..frames {
        core.run_frame(0);
    }
    (
        core.ppu().vram().to_vec(),
        core.save_state(false).expect("a ROM is loaded"),
    )
}

#[test]
fn fidplus_a4_states_with_pending_edges_restore_exactly() {
    let mut core = machine();
    let mut checked = 0;
    for _ in 0..60 {
        core.run_frame(0);
        if !core.ppu().edges_pending() {
            continue;
        }
        let state = core.save_state(true).expect("a ROM is loaded");
        let mut live_state = CoreState::default();
        core.capture_core_state(&mut live_state)
            .expect("a ROM is loaded");
        let live = run(&mut core, 2);

        let mut restored = machine();
        restored.try_load_state(&state, 0).expect("the state loads");
        assert!(run(&mut restored, 2) == live, "savestate file");

        // Run-ahead's in-memory state, restored on the same machine.
        core.restore_core_state(&live_state).expect("restores");
        assert!(run(&mut core, 2) == live, "in-memory state");
        checked += 1;
    }
    assert!(
        checked > 0,
        "no frame ended with an edge pending: the test proves nothing"
    );
}

/// A square wave that toggles its volume every frame: pulse 1 on, constant volume, a period that
/// keeps the profile filters busy.
const SOUND_PROGRAM: [u8; 27] = [
    0xA9, 0x01, 0x8D, 0x15, 0x40, // $4015 = 1
    0xA9, 0x40, 0x8D, 0x02, 0x40, // period low
    0xA9, 0x01, 0x8D, 0x03, 0x40, // period high, length
    0xA9, 0xBF, 0x8D, 0x00, 0x40, // loop ($8014): duty 2, constant volume 15
    0x49, 0x0F, // EOR #$0F
    0x4C, 0x14, 0x80, // JMP loop (a change every pass)
    0xEA, 0xEA,
];

fn sound_machine() -> NesCore {
    let image = minimal_rom(0x00, Some(&SOUND_PROGRAM), None);
    let mut core = machine_with(&image, "fidplus-d1b.nes");
    core.set_console_audio_profile(1);
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

/// Lot FIDPLUS-D1b (FIDELITY-LEADS E1-5): with a console audio profile, a savestate carries the
/// profile filters' memory, so the sound after a load is the sound the machine would have made.
#[test]
fn fidplus_d1b_a_savestate_keeps_the_profile_filters() {
    let mut original = sound_machine();
    for _ in 0..60 {
        original.run_frame(0);
    }
    drain(&original);
    let state = original.save_state(false).expect("a state");
    assert_eq!(
        i32::from_le_bytes([state[4], state[5], state[6], state[7]]),
        NesCore::EXTENDED_STATE_VERSION
    );

    let mut loaded = sound_machine();
    loaded.run_frame(0);
    loaded.try_load_state(&state, 0).expect("the state loads");
    drain(&loaded);

    for _ in 0..20 {
        original.run_frame(0);
        loaded.run_frame(0);
    }
    let (a, b) = (drain(&original), drain(&loaded));
    assert_ne!(a.len(), 0);
    assert_eq!(a.len(), b.len());
    let worst = a
        .iter()
        .zip(&b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0f32, f32::max);
    // Exactly the same samples (before D1b the filters restarted at rest: 6.7e-4 at worst, and
    // every ratio recomputation, a load's included, rebuilt them).
    assert!(worst <= 0.0, "largest difference {worst}");

    // Without a profile the block keeps the format earlier builds write.
    let mut plain = machine_with(&minimal_rom(0x00, Some(&SOUND_PROGRAM), None), "plain.nes");
    plain.run_frame(0);
    let state = plain.save_state(false).expect("a state");
    assert_eq!(
        i32::from_le_bytes([state[4], state[5], state[6], state[7]]),
        NesCore::STATE_VERSION
    );
}
