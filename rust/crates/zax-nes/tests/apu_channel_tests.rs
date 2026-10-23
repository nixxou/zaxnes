//! The five APU channels, read through the mixer (REQ-CORE-015, REQ-CORE-073,
//! REQ-CORE-113, REQ-CORE-004).

// Test periods and bytes are small: the narrowing casts cannot lose anything.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::float_cmp
)]

mod common;

use std::collections::HashSet;

use common::{machine_with, minimal};
use zax_nes::{Machine, NesBus, NesRegion};

/// A pulse at volume 15 through the linear mixer.
const PULSE_FULL: f64 = 1.526 * 0.00752 * 15.0;

/// $4001 value that keeps the sweep from muting: negate with shift 0.
const SWEEP_NEVER_MUTES: u8 = 0x08;

fn core_in(region: NesRegion) -> Machine {
    let mut machine = Machine::new();
    machine.set_region(region);
    assert!(machine.load_rom(&minimal(), "Apu.nes").success);
    machine.set_region(region);
    machine
}

fn core() -> Machine {
    machine_with(&minimal(), "Apu.nes")
}

fn tick(core: &mut Machine, cycles: usize) {
    for _ in 0..cycles {
        core.bus_mut().tick();
    }
}

fn level(core: &Machine) -> f64 {
    core.apu().mixed_level()
}

fn peak(core: &mut Machine, cycles: usize) -> f64 {
    let mut peak = f64::NEG_INFINITY;
    for _ in 0..cycles {
        core.bus_mut().tick();
        peak = peak.max(level(core));
    }
    peak
}

fn high_fraction(core: &mut Machine, cycles: usize) -> f64 {
    let mut high = 0;
    for _ in 0..cycles {
        core.bus_mut().tick();
        if level(core) > 0.0 {
            high += 1;
        }
    }
    f64::from(high) / cycles as f64
}

fn transitions(core: &mut Machine, cycles: usize) -> i32 {
    let mut previous = level(core);
    let mut changes = 0;
    for _ in 0..cycles {
        core.bus_mut().tick();
        if level(core) != previous {
            changes += 1;
            previous = level(core);
        }
    }
    changes
}

fn start_pulse1(core: &mut Machine, duty: u8, period: i32, sweep: u8) {
    core.write_memory(0x4015, 0x01);
    core.write_memory(0x4000, (duty << 6) | 0x10 | 0x0F);
    core.write_memory(0x4001, sweep);
    core.write_memory(0x4002, period as u8);
    core.write_memory(0x4003, 0xF8 | ((period >> 8) & 0x07) as u8); // Length index 31 → 30.
}

fn assert_near(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1e-9,
        "{actual} is not within 1e-9 of {expected}"
    );
}

// ---------------------------------------------------------------- pulse

#[test]
fn req_core_073_each_duty_sequence_holds_its_documented_fraction_high() {
    // The four duty sequences of nesdev: 1, 2, 4 and 6 of the eight steps are high.
    for (duty, expected) in [
        (0, 1.0 / 8.0),
        (1, 2.0 / 8.0),
        (2, 4.0 / 8.0),
        (3, 6.0 / 8.0),
    ] {
        let mut core = core();
        start_pulse1(&mut core, duty, 100, SWEEP_NEVER_MUTES);

        // Twenty full sequences: 8 steps of (period + 1) APU cycles, two CPU cycles each.
        let fraction = high_fraction(&mut core, 20 * 8 * 2 * 101);

        assert!(
            (fraction - expected).abs() <= 0.01,
            "duty {duty}: {fraction}"
        );
    }
}

#[test]
fn req_core_073_a_pulse_at_volume_fifteen_swings_between_zero_and_its_step_value() {
    let mut core = core();
    start_pulse1(&mut core, 2, 100, SWEEP_NEVER_MUTES);

    let mut levels = HashSet::new();
    for _ in 0..4000 {
        core.bus_mut().tick();
        levels.insert(level(&core).to_bits());
    }

    assert_eq!(2, levels.len());
    assert!(levels.contains(&0.0f64.to_bits()));
    assert!(levels
        .iter()
        .any(|&bits| (f64::from_bits(bits) - PULSE_FULL).abs() < 1e-9));
}

#[test]
fn req_core_073_the_envelope_decays_one_step_per_quarter_frame() {
    // nesdev, "APU Envelope": the start flag reloads the decay to 15, then each quarter
    // frame steps it down by one once the divider (period 0 here) expires.
    for (quarter_frames, expected_volume) in [(1, 15), (2, 14), (6, 10), (16, 0)] {
        let mut core = core();

        core.write_memory(0x4015, 0x01);
        core.write_memory(0x4000, 0x80); // Duty 2, envelope mode, period 0, no loop.
        core.write_memory(0x4001, SWEEP_NEVER_MUTES);
        core.write_memory(0x4002, 10);
        core.write_memory(0x4003, 0xF8); // Sets the envelope's start flag.

        // Writing $80 to $4017 generates a quarter and a half frame straight away (nesdev).
        for _ in 0..quarter_frames {
            core.write_memory(0x4017, 0x80);
            tick(&mut core, 2);
        }

        let peak = peak(&mut core, 200);

        assert_near(peak, f64::from(expected_volume) * PULSE_FULL / 15.0);
    }
}

#[test]
fn req_core_073_a_looping_envelope_reloads_to_fifteen_instead_of_staying_at_zero() {
    let mut core = core();

    core.write_memory(0x4015, 0x01);
    core.write_memory(0x4000, 0xA0); // Duty 2, envelope mode, loop set, period 0.
    core.write_memory(0x4001, SWEEP_NEVER_MUTES);
    core.write_memory(0x4002, 10);
    core.write_memory(0x4003, 0xF8);

    for _ in 0..17 {
        core.write_memory(0x4017, 0x80);
        tick(&mut core, 2);
    }

    // Sixteen quarter frames take the decay to 0; the seventeenth wraps it back to 15.
    assert_near(peak(&mut core, 200), PULSE_FULL);
}

#[test]
fn req_core_073_the_sweep_unit_mutes_a_period_out_of_range() {
    // nesdev, "APU Sweep": a period below 8, or a target period above $7FF, mutes the
    // channel, and the target is recomputed whether the sweep is enabled or not.
    for (period, sweep, audible) in [
        (7, SWEEP_NEVER_MUTES, false),
        (8, SWEEP_NEVER_MUTES, true),
        (0x7FF, SWEEP_NEVER_MUTES, true),
        (0x400, 0x00, false), // Shift 0, no negate: target = 2 x period > $7FF.
        (0x100, 0x00, true),
    ] {
        let mut core = core();
        start_pulse1(&mut core, 2, period, sweep);

        let peak = peak(&mut core, 8 * 2 * (period as usize + 1));

        assert_eq!(audible, peak > 0.0, "period {period:#x}, sweep {sweep:#x}");
    }
}

#[test]
fn req_core_073_a_negative_sweep_halves_the_period_and_doubles_the_frequency() {
    const WINDOW: usize = 8 * 2 * 0x101;
    let mut core = core();
    start_pulse1(&mut core, 2, 0x100, 0x89); // Enabled, period 0, negate, shift 1.

    let before = transitions(&mut core, WINDOW);

    // One half frame loads the target period, $100 − $80 − 1 = $7F (nesdev: pulse 1 negates
    // with a one's complement).
    core.write_memory(0x4017, 0x80);
    tick(&mut core, 2);

    let after = transitions(&mut core, WINDOW);

    assert!(
        (2 * before - 4..=2 * before + 4).contains(&after),
        "{before} → {after}"
    );
}

// ---------------------------------------------------------------- triangle

#[test]
fn req_core_073_the_triangle_walks_its_staircase_in_steps_of_one() {
    let mut core = core();
    let step = 1.526 * 0.00851;

    core.write_memory(0x4015, 0x04);
    core.write_memory(0x4008, 0x7F); // Control clear, linear counter reload 127.
    core.write_memory(0x400A, 50);
    core.write_memory(0x400B, 0xF8); // Length index 31, linear reload flag.
    core.write_memory(0x4017, 0x80); // Quarter frame: load the linear counter.

    let mut levels: Vec<f64> = Vec::new();
    for _ in 0..40 * 51 {
        core.bus_mut().tick();
        levels.push(level(&core));
    }
    levels.sort_by(f64::total_cmp);
    levels.dedup();

    // The staircase visits all sixteen output values, one step apart.
    assert_eq!(16, levels.len());
    for pair in levels.windows(2) {
        assert_near(pair[1] - pair[0], step);
    }
}

#[test]
fn req_core_073_the_triangle_holds_its_level_while_the_linear_counter_is_zero() {
    let mut core = core();

    core.write_memory(0x4015, 0x04);
    core.write_memory(0x4008, 0x00); // Control clear, reload value 0: the counter stays 0.
    core.write_memory(0x400A, 50);
    core.write_memory(0x400B, 0xF8);
    core.write_memory(0x4017, 0x80);

    assert_eq!(0, transitions(&mut core, 20 * 51));
}

#[test]
fn req_core_073_the_triangle_is_held_when_its_period_is_ultrasonic() {
    let mut core = core();

    core.write_memory(0x4015, 0x04);
    core.write_memory(0x4008, 0x7F);
    core.write_memory(0x400A, 1); // Period 1: below the audible range.
    core.write_memory(0x400B, 0xF8);
    core.write_memory(0x4017, 0x80);

    assert_eq!(0, transitions(&mut core, 4000));
}

// ---------------------------------------------------------------- noise

/// One sample per shift at the shortest period (4 CPU cycles).
fn noise_pattern(core: &mut Machine, shifts: usize) -> Vec<bool> {
    let mut pattern = vec![false; shifts];
    core.bus_mut().tick();
    pattern[0] = level(core) > 0.0;

    for slot in pattern.iter_mut().skip(1) {
        tick(core, 4);
        *slot = level(core) > 0.0;
    }
    pattern
}

fn start_noise(core: &mut Machine, period: u8) {
    core.write_memory(0x4015, 0x08);
    core.write_memory(0x400C, 0x1F); // Constant volume 15, no halt.
    core.write_memory(0x400E, period);
    core.write_memory(0x400F, 0xF8); // Length index 31 → 30.
}

#[test]
fn req_core_073_the_noise_register_repeats_every_ninety_three_shifts_in_short_mode() {
    let mut core = core();
    start_noise(&mut core, 0x80); // Mode flag set: feedback from bit 6.

    let pattern = noise_pattern(&mut core, 300);

    for i in 93..pattern.len() {
        assert_eq!(pattern[i - 93], pattern[i], "shift {i}");
    }
}

#[test]
fn req_core_073_the_noise_register_does_not_repeat_within_ninety_three_shifts_in_long_mode() {
    let mut core = core();
    start_noise(&mut core, 0x00); // Feedback from bit 1: a period of 32 767 shifts.

    let pattern = noise_pattern(&mut core, 300);

    assert!((93..pattern.len()).any(|i| pattern[i - 93] != pattern[i]));
}

#[test]
fn req_core_015_the_noise_period_table_follows_the_region() {
    // Noise period tables of REQ-CORE-015: the PAL table in PAL, the NTSC one in NTSC,
    // MULTI and DENDY. The shortest gap between two output changes is exactly one period.
    for (region, period_index, cpu_cycles_per_shift) in [
        (NesRegion::Ntsc, 0, 4),
        (NesRegion::Pal, 0, 4),
        (NesRegion::Ntsc, 2, 16),
        (NesRegion::Pal, 2, 14),
        (NesRegion::Dendy, 2, 16),
        (NesRegion::Multi, 2, 16),
        (NesRegion::Ntsc, 4, 64),
        (NesRegion::Pal, 4, 60),
    ] {
        let mut core = core_in(region);
        start_noise(&mut core, period_index);

        // The first gap starts from whatever phase the power-on left the timer in.
        cycles_to_next_transition(&mut core);

        let mut shortest = i32::MAX;
        for _ in 0..80 {
            shortest = shortest.min(cycles_to_next_transition(&mut core));
        }

        assert_eq!(
            cpu_cycles_per_shift, shortest,
            "{region:?}, index {period_index}"
        );
    }
}

fn cycles_to_next_transition(core: &mut Machine) -> i32 {
    let start = level(core);
    for cycles in 1..=400_000 {
        core.bus_mut().tick();
        if level(core) != start {
            return cycles;
        }
    }
    -1
}

// ---------------------------------------------------------------- DMC

#[test]
fn req_core_073_a_write_to_4011_loads_the_dmc_output_level_directly() {
    let mut core = core();

    core.write_memory(0x4011, 64);
    core.bus_mut().tick();

    assert_near(level(&core), 1.526 * 0.00335 * 64.0);

    core.write_memory(0x4011, 0xFF); // Only the low seven bits are kept.
    core.bus_mut().tick();

    assert_near(level(&core), 1.526 * 0.00335 * 127.0);
}

fn start_dmc(core: &mut Machine, rate: u8, length: u8, irq: bool, looping: bool) {
    core.write_memory(
        0x4010,
        (if irq { 0x80 } else { 0 }) | (if looping { 0x40 } else { 0 }) | rate,
    );
    core.write_memory(0x4012, 0x00); // Sample at $C000.
    core.write_memory(0x4013, length);
    core.write_memory(0x4015, 0x10);
}

#[test]
fn req_core_113_a_dmc_sample_fetch_steals_four_cpu_cycles() {
    const TICKS: i64 = 4000;
    let mut core = core();
    start_dmc(&mut core, 15, 2, false, false);

    let before = core.cycle_count();
    tick(&mut core, TICKS as usize);

    let stolen = core.cycle_count() - before - TICKS;

    assert!(core.bus().dmc_dma_count() > 0);
    assert_eq!(
        i64::from(NesBus::DMC_DMA_CYCLES) * core.bus().dmc_dma_count(),
        stolen
    );
}

#[test]
fn req_core_015_the_dmc_rate_table_follows_the_region() {
    // DMC rate tables of REQ-CORE-015: rate 15 is 54 CPU cycles per bit in NTSC and 50 in
    // PAL, the NTSC table serving MULTI and DENDY too. Lengthening a sample by 32 bytes
    // costs exactly 256 more bits.
    for (region, cycles_per_bit) in [
        (NesRegion::Ntsc, 54),
        (NesRegion::Pal, 50),
        (NesRegion::Dendy, 54),
        (NesRegion::Multi, 54),
    ] {
        let short_sample = cycles_to_dmc_irq(region, 1); // 17 bytes.
        let long_sample = cycles_to_dmc_irq(region, 3); // 49 bytes.

        assert_eq!(
            32 * 8 * cycles_per_bit,
            long_sample - short_sample,
            "{region:?}"
        );
    }
}

fn cycles_to_dmc_irq(region: NesRegion, length: u8) -> i64 {
    let mut core = core_in(region);
    start_dmc(&mut core, 15, length, true, false);

    let before = core.cycle_count();
    while !core.apu().irq_asserted() && core.cycle_count() - before < 200_000 {
        core.bus_mut().tick();
    }

    assert!(core.apu().irq_asserted());

    // The channel keeps clocking while a fetch halts the CPU, so the stolen cycles count.
    core.cycle_count() - before
}

#[test]
fn req_core_113_4015_reports_the_dmc_and_its_irq() {
    let mut core = core();
    start_dmc(&mut core, 15, 1, true, false);
    tick(&mut core, 10);

    assert_eq!(0x10, core.apu().peek_status() & 0x10); // Bit 4: a sample is playing.

    while !core.apu().irq_asserted() {
        core.bus_mut().tick();
    }

    assert_eq!(0x80, core.apu().peek_status() & 0x80); // Bit 7: the DMC IRQ.
    assert_eq!(0x00, core.apu().peek_status() & 0x10); // The sample has ended.

    // A $4015 write clears the DMC IRQ flag; a read does not (apu_test 7-dmc_basics).
    core.bus_mut().apu_mut().read_status(0);
    assert!(core.apu().irq_asserted());

    core.write_memory(0x4015, 0x00);
    assert!(!core.apu().irq_asserted());
}

#[test]
fn req_core_113_a_looping_dmc_sample_never_ends_and_never_raises_its_irq() {
    let mut core = core();
    start_dmc(&mut core, 15, 0, true, true);

    tick(&mut core, 20_000);

    assert!(!core.apu().irq_asserted());
    assert_eq!(0x10, core.apu().peek_status() & 0x10);

    // Clearing bit 4 of $4015 stops it.
    core.write_memory(0x4015, 0x00);
    tick(&mut core, 10);
    assert_eq!(0x00, core.apu().peek_status() & 0x10);
}

#[test]
fn req_core_113_clearing_the_dmc_irq_enable_bit_clears_a_pending_irq() {
    let mut core = core();
    start_dmc(&mut core, 15, 0, true, false);

    while !core.apu().irq_asserted() {
        core.bus_mut().tick();
    }

    core.write_memory(0x4010, 0x0F); // IRQ enable cleared.
    assert!(!core.apu().irq_asserted());
}

// ---------------------------------------------------------------- reset

#[test]
fn req_core_004_reset_silences_every_channel() {
    let mut core = core();
    start_pulse1(&mut core, 2, 100, SWEEP_NEVER_MUTES);
    core.write_memory(0x4011, 100);
    start_noise(&mut core, 0x04);
    tick(&mut core, 1000);

    assert!(peak(&mut core, 1000) > 0.0);

    core.reset();
    core.bus_mut().tick();

    assert_eq!(0.0, level(&core));
    assert_eq!(0x00, core.apu().peek_status());
    assert_eq!(0, core.apu().available_samples());
}
