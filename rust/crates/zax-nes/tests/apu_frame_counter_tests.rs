//! The frame counter, its IRQ and the $4015 status (REQ-CORE-111, REQ-CORE-081,
//! REQ-CORE-004, REQ-CORE-070).

#![allow(clippy::float_cmp)] // Exact figures.

mod common;

use common::{machine_with, minimal};
use zax_nes::{NesCore, NesRegion, RegionTiming};

const NTSC_FOUR_STEP_PERIOD: usize = 29830;

fn loaded_core() -> NesCore {
    machine_with(&minimal(), "Apu.nes")
}

fn tick(core: &mut NesCore, cycles: usize) {
    for _ in 0..cycles {
        core.bus_mut().tick();
    }
}

#[test]
fn req_core_111_the_four_step_sequence_raises_the_frame_irq() {
    let mut core = loaded_core();

    // Four-step mode, IRQ enabled.
    core.write_memory(0x4017, 0x00);
    assert!(!core.apu().irq_asserted());

    tick(&mut core, NTSC_FOUR_STEP_PERIOD + 8);

    assert!(core.apu().irq_asserted());

    // Reading $4015 clears the flag.
    assert_eq!(0x40, core.bus_mut().apu_mut().read_status(0) & 0x40);
    assert!(!core.apu().irq_asserted());
}

#[test]
fn req_core_111_the_inhibit_bit_suppresses_the_frame_irq() {
    let mut core = loaded_core();

    core.write_memory(0x4017, 0x40); // Four-step mode, IRQ inhibited.
    tick(&mut core, 2 * NTSC_FOUR_STEP_PERIOD);

    assert!(!core.apu().irq_asserted());
}

#[test]
fn req_core_111_the_five_step_sequence_never_raises_the_frame_irq() {
    let mut core = loaded_core();

    core.write_memory(0x4017, 0x80); // Five-step mode.
    tick(&mut core, 2 * 37282);

    assert!(!core.apu().irq_asserted());
}

#[test]
fn req_core_111_setting_the_inhibit_bit_clears_a_pending_frame_irq() {
    let mut core = loaded_core();
    core.write_memory(0x4017, 0x00);
    tick(&mut core, NTSC_FOUR_STEP_PERIOD + 8);

    assert!(core.apu().irq_asserted());

    core.write_memory(0x4017, 0x40);
    assert!(!core.apu().irq_asserted());
}

#[test]
fn req_core_081_peeking_4015_does_not_clear_the_frame_irq() {
    let mut core = loaded_core();
    core.write_memory(0x4017, 0x00);
    tick(&mut core, NTSC_FOUR_STEP_PERIOD + 8);

    assert_eq!(0x40, core.apu().peek_status() & 0x40);
    assert!(core.apu().irq_asserted());

    // Only a real CPU read clears it.
    core.bus_mut().apu_mut().read_status(0);
    assert!(!core.apu().irq_asserted());
}

#[test]
fn req_core_081_4015_reports_the_channels_whose_length_counter_is_running() {
    let mut core = loaded_core();

    // Enable pulse 1 and the triangle, then load their length counters.
    core.write_memory(0x4015, 0x05);
    core.write_memory(0x4003, 0xF8); // Pulse 1 length index 31 → 30.
    core.write_memory(0x400B, 0xF8); // Triangle.

    assert_eq!(0x05, core.apu().peek_status() & 0x0F);

    // A disabled channel's counter is cleared at once.
    core.write_memory(0x4015, 0x04);
    assert_eq!(0x04, core.apu().peek_status() & 0x0F);
}

#[test]
fn req_core_081_a_length_counter_is_not_loaded_while_its_channel_is_disabled() {
    let mut core = loaded_core();

    core.write_memory(0x4015, 0x00);
    core.write_memory(0x4003, 0xF8);

    assert_eq!(0x00, core.apu().peek_status() & 0x0F);
}

#[test]
fn req_core_111_half_frames_count_the_length_counters_down() {
    let mut core = loaded_core();

    core.write_memory(0x4015, 0x01); // Pulse 1 on.
    core.write_memory(0x4000, 0x00); // Length counter not halted.
    core.write_memory(0x4003, 0x08); // Length index 1 → 254.
    core.write_memory(0x4017, 0x00); // Four-step mode, sequencer restarted.

    assert_eq!(0x01, core.apu().peek_status() & 0x01);

    // 254 half frames, two per sequence period, plus a margin.
    tick(&mut core, 128 * NTSC_FOUR_STEP_PERIOD);

    assert_eq!(0x00, core.apu().peek_status() & 0x01);
}

#[test]
fn req_core_111_a_halted_length_counter_never_reaches_zero() {
    let mut core = loaded_core();

    core.write_memory(0x4015, 0x01);
    core.write_memory(0x4000, 0x20); // Halt flag set.
    core.write_memory(0x4003, 0x08);
    core.write_memory(0x4017, 0x00);

    tick(&mut core, 300 * NTSC_FOUR_STEP_PERIOD);

    assert_eq!(0x01, core.apu().peek_status() & 0x01);
}

#[test]
fn req_core_004_reset_silences_the_apu_and_clears_its_irq() {
    let mut core = loaded_core();
    core.write_memory(0x4015, 0x0F);
    core.write_memory(0x4003, 0xF8);
    core.write_memory(0x4017, 0x00);
    tick(&mut core, NTSC_FOUR_STEP_PERIOD + 8);

    assert!(core.apu().irq_asserted());

    core.reset();

    assert!(!core.apu().irq_asserted());
    assert_eq!(0x00, core.apu().peek_status());
    assert_eq!(0, core.apu().available_samples());
}

#[test]
fn req_core_070_one_frame_produces_the_documented_number_of_samples() {
    // Cross-check of the per-frame figure the resampler is calibrated on.
    assert_eq!(
        800.0,
        RegionTiming::for_region(NesRegion::Ntsc).samples_per_frame(48_000)
    );
    assert_eq!(
        960.0,
        RegionTiming::for_region(NesRegion::Pal).samples_per_frame(48_000)
    );
    assert_eq!(
        960.0,
        RegionTiming::for_region(NesRegion::Dendy).samples_per_frame(48_000)
    );
    assert_eq!(
        800.0,
        RegionTiming::for_region(NesRegion::Multi).samples_per_frame(48_000)
    );
}
