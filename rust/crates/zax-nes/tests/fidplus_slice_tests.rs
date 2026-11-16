//! Lot FIDPLUS-B step 2 (`docs/design/REALTIME-PACING.md`): a frame run in slices is the same
//! frame, and the pad's state given in the middle of a frame is read at the game's next strobe.

#![allow(clippy::expect_used)]

mod common;

use common::{machine_with, minimal_rom};
use zax_nes::NesCore;

/// Strobes the pad and stores its first bit at `$00`, then counts frames at `$01` in the NMI,
/// forever: the pad is read many times a frame.
const PROGRAM: [u8; 32] = [
    0x78, 0xD8, // SEI, CLD
    0xA9, 0x80, 0x8D, 0x00, 0x20, // $2000 = $80 (NMI on)
    0xA9, 0x01, 0x8D, 0x16, 0x40, // loop: $4016 = 1
    0xA9, 0x00, 0x8D, 0x16, 0x40, // $4016 = 0
    0xAD, 0x16, 0x40, // LDA $4016
    0x29, 0x01, // AND #1
    0x85, 0x00, // STA $00
    0x4C, 0x07, 0x80, // JMP loop
    0xE6, 0x01, // NMI ($801B): INC $01
    0x40, // RTI
    0x00, 0x00,
];

fn machine() -> NesCore {
    let image = minimal_rom(0x00, Some(&PROGRAM), Some(0x801B));
    let mut core = machine_with(&image, "slices.nes");
    core.reset();
    core
}

#[test]
fn fidplus_b2_slices_of_any_size_make_the_same_frames() {
    for slice in [1_i64, 7, 113, 1000, 29_781, 100_000] {
        let mut whole = machine();
        let mut sliced = machine();
        for frame in 0..30u16 {
            let input = u16::from(frame % 3 == 0);
            let cycles = whole.run_frame(input);
            let mut until = sliced.cycle_count();
            let got = loop {
                until += slice;
                if let Some(cycles) = sliced.run_frame_slice(input, until) {
                    break cycles;
                }
            };
            assert_eq!(cycles, got, "slice {slice}, frame {frame}: cycles");
            assert!(
                whole.save_state(false).expect("loaded")
                    == sliced.save_state(false).expect("loaded"),
                "slice {slice}, frame {frame}: state"
            );
        }
    }
}

#[test]
fn fidplus_b2_a_button_pressed_mid_frame_is_read_at_the_next_strobe() {
    let mut core = machine();
    for _ in 0..3 {
        core.run_frame(0);
    }
    assert_eq!(0, core.read_memory(0x0000));
    // Half a frame with nothing pressed, then A: the game reads it before the frame ends.
    let start = core.cycle_count();
    assert_eq!(None, core.run_frame_slice(0, start + 15_000));
    assert_eq!(0, core.read_memory(0x0000), "nothing pressed yet");
    assert_eq!(None, core.run_frame_slice(0x01, start + 15_500));
    assert_eq!(
        1,
        core.read_memory(0x0000),
        "A read inside the frame it was pressed in"
    );
    assert!(core.frame_in_progress());
    while core
        .run_frame_slice(0x01, core.cycle_count() + 1000)
        .is_none()
    {}
    assert!(!core.frame_in_progress());
}
