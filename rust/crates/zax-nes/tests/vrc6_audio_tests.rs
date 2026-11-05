//! The VRC6's expansion audio (REQ-CORE-075, which leaves it optional): the two pulse channels,
//! the sawtooth, and the frequency scaling and halt register `$9003`.
//!
//! Read from the public nesdev page "VRC6 audio". Unlike the Sunsoft 5B's, that page fixes the
//! **level** as well as the behaviour — "At maximum volume, the pulse channels of the VRC6 are
//! roughly equivalent to the pulse channels of the 2A03 (except inverted)" and "The DAC of the
//! VRC6, unlike the 2A03, appears to be linear" — so Q-118's question does not reopen here;
//! `req_core_075_the_vrc6_dac_is_linear_through_an_apu_pulse` is that sentence as an assertion.
//! Mesen 2's `Vrc6Pulse.h` / `Vrc6Saw.h` / `Vrc6Audio.h` transcribe the same page and agree.

// Register values and periods are small literals.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::unwrap_used
)]

mod common;

use common::mapper_fixture::MapperRom;
use zax_nes::apu::mixer;
use zax_nes::NesCore;

/// A machine with a synthetic mapper-24 cartridge, powered on.
fn core() -> NesCore {
    let image = MapperRom::new(24).image();
    let mut core = NesCore::new();
    let load = core.load_rom(&image, "Vrc6.nes");
    assert!(load.success, "{:?}", load.error);
    core
}

/// `M` as the test point of REQ-CORE-076 reports it.
fn expansion(core: &NesCore) -> i32 {
    core.apu().mixer_expansion()
}

/// Runs `cycles` CPU cycles and collects every distinct `M` seen, in order of first appearance.
fn levels_over(core: &mut NesCore, cycles: usize) -> Vec<i32> {
    let mut seen = Vec::new();
    for _ in 0..cycles {
        core.bus_mut().tick();
        let level = expansion(core);
        if seen.last() != Some(&level) {
            seen.push(level);
        }
    }
    seen
}

/// The highest `M` over `cycles` CPU cycles.
fn peak(core: &mut NesCore, cycles: usize) -> i32 {
    let mut high = i32::MIN;
    for _ in 0..cycles {
        core.bus_mut().tick();
        high = high.max(expansion(core));
    }
    high
}

/// Sets up pulse channel `base` ($9000 or $A000): volume, duty, and the 12-bit period with `E`.
fn pulse(core: &mut NesCore, base: u16, control: u8, period: u16, enabled: bool) {
    core.write_memory(base, control);
    core.write_memory(base + 1, (period & 0xFF) as u8);
    let high = ((period >> 8) & 0x0F) as u8 | if enabled { 0x80 } else { 0 };
    core.write_memory(base + 2, high);
}

/// Sets up the sawtooth: the 6-bit accumulator rate, and the period with `E`.
fn saw(core: &mut NesCore, rate: u8, period: u16, enabled: bool) {
    core.write_memory(0xB000, rate & 0x3F);
    core.write_memory(0xB001, (period & 0xFF) as u8);
    let high = ((period >> 8) & 0x0F) as u8 | if enabled { 0x80 } else { 0 };
    core.write_memory(0xB002, high);
}

// ---------------------------------------------------------------- the level

#[test]
fn req_core_075_the_vrc6_dac_is_linear_through_an_apu_pulse() {
    // "The final mix is a 6-bit DAC summing the two 4-bit pulse outputs and the high 5 bits of
    // the saw accumulator": 15 + 15 + 31 and the zero.
    assert_eq!(62, mixer::VRC6_TABLE.len());
    assert_eq!(0, mixer::VRC6_TABLE[0]);

    // "At maximum volume, the pulse channels of the VRC6 are roughly equivalent to the pulse
    // channels of the 2A03": step 15 is one APU pulse at volume 15.
    assert_eq!(mixer::PULSE_TABLE[15], mixer::VRC6_TABLE[15]);
    assert_eq!(4890, mixer::VRC6_TABLE[15]);

    // "The DAC of the VRC6, unlike the 2A03, appears to be linear": every step is the same
    // height, recomputed here from the unrounded anchor.
    let step = mixer::NORMALISATION * mixer::pulse(15) / 15.0;
    for (level, &entry) in mixer::VRC6_TABLE.iter().enumerate() {
        assert_eq!(
            mixer::round_to_even(step * level as f64),
            entry,
            "level {level}"
        );
    }

    // And the whole chip at full tilt is 61 steps, which is what the mixer may subtract.
    assert_eq!(19_886, mixer::VRC6_TABLE[61]);
}

#[test]
fn req_core_075_a_vrc6_is_silent_until_a_channel_is_enabled() {
    let mut core = core();
    // Power-on: every `E` bit clear, so silence whatever the volumes say (REQ-CORE-004).
    assert_eq!(0, peak(&mut core, 2_000));

    // A volume with no `E` is still silence.
    pulse(&mut core, 0x9000, 0x8F, 4, false);
    assert_eq!(0, peak(&mut core, 2_000));
}

// ---------------------------------------------------------------- the pulse channels

#[test]
fn req_core_075_a_vrc6_pulse_in_mode_m_outputs_its_volume_flat() {
    let mut core = core();
    // "When the mode bit M is true, the channel ignores the duty cycle generator and outputs
    // the current volume regardless." So one pulse at volume 15 is exactly step 15.
    pulse(&mut core, 0x9000, 0x8F, 0x010, true);
    assert_eq!(vec![mixer::VRC6_TABLE[15]], levels_over(&mut core, 4_000));

    // Volume 7 is step 7, and the DAC being linear that is about half.
    pulse(&mut core, 0x9000, 0x87, 0x010, true);
    assert_eq!(vec![mixer::VRC6_TABLE[7]], levels_over(&mut core, 4_000));
}

#[test]
fn req_core_075_a_vrc6_pulse_duty_is_d_plus_one_sixteenths() {
    // "The duty cycle generator operates in 16 steps: When the current step is less than or
    // equal to the given duty cycle D, the channel volume V is output, otherwise 0 is output."
    // So D = 0 is one step in sixteen high (6.25 %) and D = 7 is eight (50 %).
    for (duty, high_steps) in [(0u8, 1u32), (3, 4), (7, 8)] {
        let mut core = core();
        // Period 0: the divider reloads with 1, so the generator steps every CPU cycle and one
        // whole 16-step cycle is 16 cycles.
        pulse(&mut core, 0x9000, (duty << 4) | 0x0F, 0, true);
        let mut high = 0u32;
        let cycles = 16u32 * 64;
        for _ in 0..cycles {
            core.bus_mut().tick();
            if expansion(&core) != 0 {
                high += 1;
            }
        }

        // Within one step of the exact ratio: where the sample lands inside the first 16-cycle
        // period depends on the writes above, and the test is the duty and not the phase.
        let expected = high_steps * (cycles / 16);
        assert!(
            high.abs_diff(expected) <= cycles / 16,
            "duty {duty}: {high} high of {cycles}, expected about {expected}"
        );
    }
}

#[test]
fn req_core_075_the_two_vrc6_pulses_add_into_the_same_dac() {
    let mut core = core();
    // Both flat at volume 15: 15 + 15 = step 30, and the DAC being linear that is twice one.
    pulse(&mut core, 0x9000, 0x8F, 0x010, true);
    pulse(&mut core, 0xA000, 0x8F, 0x010, true);
    assert_eq!(vec![mixer::VRC6_TABLE[30]], levels_over(&mut core, 4_000));
    assert_eq!(2 * mixer::VRC6_TABLE[15], mixer::VRC6_TABLE[30]);

    // $A000-$A002 really is the second channel and not a mirror of the first: silencing one
    // leaves the other.
    pulse(&mut core, 0x9000, 0x8F, 0x010, false);
    assert_eq!(vec![mixer::VRC6_TABLE[15]], levels_over(&mut core, 4_000));
}

#[test]
fn req_core_075_clearing_e_forces_a_vrc6_pulse_to_zero_and_resets_its_duty() {
    let mut core = core();
    // D = 0, so only step 0 of sixteen is high.
    pulse(&mut core, 0x9000, 0x0F, 0, true);
    assert!(peak(&mut core, 64) > 0);

    // "When the channel is disabled by clearing the E bit, output is forced to 0, and the duty
    // cycle is immediately reset and halted."
    core.write_memory(0x9002, 0x00);
    assert_eq!(0, peak(&mut core, 2_000));

    // "it will resume from the beginning when E is once again set": step 0 is the high one for
    // D = 0, so the channel speaks on the first divider step after the enable.
    core.write_memory(0x9002, 0x80);
    assert!(peak(&mut core, 32) > 0);
}

// ---------------------------------------------------------------- the sawtooth

#[test]
fn req_core_075_the_vrc6_saw_accumulates_six_times_then_resets() {
    let mut core = core();
    // Rate 8, period 0: the divider steps every CPU cycle, so the fourteen phase steps are
    // fourteen cycles. The accumulator adds on the six even steps and resets on step 0, and the
    // output is its high five bits: 8, 16, 24, 32, 40, 48 >> 3 = 1, 2, 3, 4, 5, 6, then 0.
    saw(&mut core, 8, 0, true);
    let seen = levels_over(&mut core, 14 * 8);
    let expected: Vec<i32> = (0..=6).map(|step| mixer::VRC6_TABLE[step]).collect();
    for level in &expected {
        assert!(seen.contains(level), "missing {level} in {seen:?}");
    }

    // Six additions and no seventh: 7 is never reached.
    assert!(
        !seen.contains(&mixer::VRC6_TABLE[7]),
        "a seventh addition happened: {seen:?}"
    );
}

#[test]
fn req_core_075_a_vrc6_saw_rate_above_42_wraps_the_accumulator() {
    let mut core = core();
    // "If A is more than 42 (floor(255 / 6)), the accumulator will wrap, resulting in distorted
    // sound." Rate 43 x 6 = 258, which an 8-bit accumulator cannot hold.
    saw(&mut core, 43, 0, true);
    let seen = levels_over(&mut core, 14 * 8);
    // 43, 86, 129, 172, 215, 258 & $FF = 2 -> high five bits 5, 10, 16, 21, 26, 0.
    for step in [5usize, 10, 16, 21, 26] {
        assert!(
            seen.contains(&mixer::VRC6_TABLE[step]),
            "missing step {step} in {seen:?}"
        );
    }

    // The wrap takes the sixth addition back to the bottom rather than up to 32.
    assert!(
        !seen.contains(&mixer::VRC6_TABLE[31]),
        "the accumulator did not wrap: {seen:?}"
    );
}

#[test]
fn req_core_075_clearing_e_zeroes_the_vrc6_saw_accumulator() {
    let mut core = core();
    saw(&mut core, 20, 0, true);
    assert!(peak(&mut core, 14 * 4) > 0);

    // "If E is clear, the accumulator is forced to zero until E is again set."
    core.write_memory(0xB002, 0x00);
    assert_eq!(0, peak(&mut core, 2_000));
}

#[test]
fn req_core_075_the_vrc6_saw_divides_the_clock_by_14_and_the_pulses_by_16() {
    // `f = CPU / (14 * (t + 1))` for the saw against `CPU / (16 * (t + 1))` for a pulse: the
    // same period register gives the saw a *longer* phase in divider steps but a shorter one in
    // CPU cycles. Measured as the number of cycles one whole phase takes.
    let period = 3u16;
    let divider = usize::from(period) + 1;

    let mut saw_core = core();
    saw(&mut saw_core, 8, period, true);
    // One saw phase is 14 divider steps; count the cycles between two resets to zero.
    let mut cycles_to_zero = 0;
    for cycle in 1..=(14 * divider * 4) {
        saw_core.bus_mut().tick();
        if expansion(&saw_core) == 0 && cycle > 14 * divider {
            cycles_to_zero = cycle;
            break;
        }
    }
    assert!(cycles_to_zero > 0, "the saw never reset");

    // A pulse with the same period cycles in 16 divider steps, so a full duty cycle is longer.
    let mut pulse_core = core();
    pulse(&mut pulse_core, 0x9000, 0x0F, period, true);
    let mut high_cycles = 0usize;
    for _ in 0..(16 * divider) {
        pulse_core.bus_mut().tick();
        if expansion(&pulse_core) != 0 {
            high_cycles += 1;
        }
    }
    // D = 0 means one of sixteen steps, i.e. `divider` cycles out of 16 x divider.
    assert!(
        high_cycles > 0 && high_cycles <= 2 * divider,
        "a 1/16 duty gave {high_cycles} of {} cycles",
        16 * divider
    );
}

// ---------------------------------------------------------------- $9003

#[test]
fn req_core_075_9003_bit_0_halts_every_vrc6_oscillator_in_place() {
    let mut core = core();
    // D = 0 and period 0: the channel is high on one cycle in sixteen.
    pulse(&mut core, 0x9000, 0x0F, 0, true);

    // Find a cycle on which it is high, then halt there: "halts all oscillators, stopping them
    // in their current state", so the level must not move again.
    let mut halted_at = 0;
    for _ in 0..64 {
        core.bus_mut().tick();
        if expansion(&core) != 0 {
            core.write_memory(0x9003, 0x01);
            halted_at = expansion(&core);
            break;
        }
    }
    assert!(halted_at > 0, "never caught the channel high");
    assert_eq!(vec![halted_at], levels_over(&mut core, 4_000));

    // Clearing the halt lets it go again, from where it stopped.
    core.write_memory(0x9003, 0x00);
    assert!(levels_over(&mut core, 4_000).len() > 1);
}

#[test]
fn req_core_075_9003_scales_every_frequency_by_16_or_256() {
    // "16x frequency (bit 1)" is a four-octave rise and "256x frequency (bit 2)" an eight-octave
    // one, which is the period shifted right by 4 and by 8. Measured as the number of duty
    // cycles a channel completes in a fixed window.
    let period = 0x100u16;
    let count_edges = |scale: u8| -> usize {
        let mut core = core();
        pulse(&mut core, 0x9000, 0x0F, period, true);
        core.write_memory(0x9003, scale);
        let mut edges = 0;
        let mut was_high = expansion(&core) != 0;
        for _ in 0..40_000 {
            core.bus_mut().tick();
            let high = expansion(&core) != 0;
            if high && !was_high {
                edges += 1;
            }
            was_high = high;
        }
        edges
    };

    let plain = count_edges(0x00);
    let times16 = count_edges(0x02);
    let times256 = count_edges(0x04);

    // $100 >> 4 = $10 and $100 >> 8 = $1, so the divider goes 257 -> 17 -> 2 and the rates rise
    // by about 15x then another 8.5x. Checked as ratios rather than absolute counts.
    assert!(plain > 0, "no edge at all");
    assert!(
        times16 > plain * 8,
        "16x gave {times16} against {plain} edges"
    );
    assert!(
        times256 > times16 * 4,
        "256x gave {times256} against {times16} edges"
    );

    // "The 256x flag overrides the 16x flag."
    assert_eq!(times256, count_edges(0x06));

    // "The halt flag overrides the other flags."
    let mut core = core();
    pulse(&mut core, 0x9000, 0x8F, period, true);
    core.write_memory(0x9003, 0x07);
    assert_eq!(1, levels_over(&mut core, 40_000).len());
}

// ---------------------------------------------------------------- the whole chip

#[test]
fn req_core_075_the_vrc6_sums_its_three_channels_into_one_m() {
    let mut core = core();
    // Both pulses flat at 15 and the saw held at its top step: 15 + 15 + 31 is the DAC's
    // ceiling, and `M` is positive because the APU subtracts it (AMD-061 §6.2).
    pulse(&mut core, 0x9000, 0x8F, 0x200, true);
    pulse(&mut core, 0xA000, 0x8F, 0x200, true);
    // Rate 42 is the largest that does not wrap: 42 x 6 = 252, high five bits 31.
    saw(&mut core, 42, 0, true);
    let high = peak(&mut core, 4_000);
    assert_eq!(mixer::VRC6_TABLE[61], high);
    assert!(high > 0, "M must be positive for the APU to subtract it");
}

#[test]
fn req_core_075_a_vrc6_subtracts_its_m_from_the_apu_mix() {
    let mut core = core();
    // The whole chip quiet: `S` is `A` alone.
    let quiet = {
        core.bus_mut().tick();
        core.apu().mixer_output()
    };

    // One pulse flat at 15, and `S` drops by exactly that step — REQ-CORE-073's `S = A - M` as
    // AMD-061 §6.2 amends it, and the page's "(except inverted)".
    pulse(&mut core, 0x9000, 0x8F, 0x200, true);
    core.bus_mut().tick();
    core.bus_mut().tick();
    assert_eq!(quiet - mixer::VRC6_TABLE[15], core.apu().mixer_output());
}
