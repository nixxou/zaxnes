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
