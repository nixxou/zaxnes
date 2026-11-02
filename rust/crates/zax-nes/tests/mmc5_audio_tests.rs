//! The MMC5's expansion audio (REQ-CORE-075, AMD-054): two square channels, the PCM channel,
//! $5010 and $5015, and the way their output `M` joins the APU's `A` in the mixer
//! (REQ-CORE-073), read at the test point of REQ-CORE-076.

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
use zax_nes::cpu::CpuBus;
use zax_nes::{NesCore, RegionTiming};

/// A machine with a synthetic MMC5 cartridge, powered on.
fn core() -> NesCore {
    let image = MapperRom::new(5).image();
    let mut core = NesCore::new();
    let load = core.load_rom(&image, "Mmc5Audio.nes");
    assert!(load.success, "{:?}", load.error);
    core
}

/// Sets square `channel` (0 or 1) up, without touching $5015: the enable register is written on
/// its own because clearing a bit puts that channel's length counter back to 0, so the two
/// channels cannot be enabled one after the other.
///
/// `control` is the first register; 0xBF is duty 2 (50 %), length halted, constant volume 15.
fn configure_square(core: &mut NesCore, channel: u16, period: i32, control: u8) {
    let base = 0x5000 + (4 * channel);
    core.write_memory(base, control);
    core.write_memory(base + 2, period as u8);
    core.write_memory(base + 3, 0xF8 | ((period >> 8) & 0x07) as u8);
}

/// Enables and starts square `channel` alone.
fn start_square(core: &mut NesCore, channel: u16, period: i32) {
    core.write_memory(0x5015, 0x01 << channel);
    configure_square(core, channel, period, 0xBF);
}

/// Enables and starts both squares on the same period, in phase: a write to $5003 or $5007
/// restarts the duty sequencer, and no cycle passes between the two.
fn start_both_squares(core: &mut NesCore, period: i32) {
    core.write_memory(0x5015, 0x03);
    configure_square(core, 0, period, 0xBF);
    configure_square(core, 1, period, 0xBF);
}

/// Largest `M` the test point reports over `cycles` cycles.
fn peak_expansion(core: &mut NesCore, cycles: usize) -> i32 {
    let mut peak = 0;
    for _ in 0..cycles {
        core.bus_mut().tick();
        peak = peak.max(core.apu().mixer_expansion());
    }
    peak
}

/// A CPU read of `address`, with the side effects a read has (REQ-CORE-080).
fn read(core: &mut NesCore, address: u16) -> u8 {
    core.bus_mut().read(address)
}

// ---------------------------------------------------------------- REQ-CORE-075, levels

/// REQ-CORE-075 (AMD-054), first criterion: one square channel at volume 15 gives `M` = 4 623,
/// the two of them 8 000, and a PCM level of 128 gives 3 968.
#[test]
fn req_core_075_the_board_produces_the_documented_integers() {
    let mut one = core();
    start_square(&mut one, 0, 100);
    assert_eq!(4623, peak_expansion(&mut one, 4000));

    let mut other = core();
    start_square(&mut other, 1, 100);
    assert_eq!(4623, peak_expansion(&mut other, 4000));

    // Both squares on the same period and started in phase.
    let mut both = core();
    start_both_squares(&mut both, 100);
    assert_eq!(8000, peak_expansion(&mut both, 4000));

    let mut pcm = core();
    pcm.write_memory(0x5011, 128);
    pcm.bus_mut().tick();
    assert_eq!(3968, pcm.apu().mixer_expansion());
    assert_eq!(31 * 128, pcm.apu().mixer_expansion());
}

/// REQ-CORE-075 (AMD-054) — `Km = 8000 / P(30)`, so the table the board's squares use is the
/// pulse function of REQ-CORE-073 scaled to top out at 8 000 instead of at the APU's 8 462, and
/// the PCM channel steps by 31 per unit.
#[test]
fn req_core_075_the_board_table_is_the_pulse_function_scaled_to_eight_thousand() {
    assert_eq!(31, mixer::MMC5_PULSE_TABLE.len());
    assert_eq!(0, mixer::MMC5_PULSE_TABLE[0]);
    assert_eq!(4623, mixer::MMC5_PULSE_TABLE[15]);
    assert_eq!(mixer::MMC5_FULL_SCALE, mixer::MMC5_PULSE_TABLE[30]);

    let km = 8000.0 / mixer::pulse(30);
    assert!((mixer::MMC5_NORMALISATION - km).abs() < 1e-6);
    assert!((mixer::MMC5_NORMALISATION - 30_949.798).abs() < 1e-3);

    for i in 0..=30 {
        assert_eq!(
            mixer::round_to_even(mixer::MMC5_NORMALISATION * mixer::pulse(i)),
            mixer::MMC5_PULSE_TABLE[i]
        );
    }

    assert_eq!(31, mixer::MMC5_PCM_STEP);
}

/// REQ-CORE-073 (AMD-054) — `M` is added to `A` and the sum is clipped to 32 767, never
/// wrapped: the board's full scale on top of the APU's would reach 48 672.
#[test]
fn req_core_073_the_board_output_is_added_to_the_apu_and_the_sum_is_clipped() {
    let mut core = core();

    // A DMC level written through $4011 is a pure step, so `A` sits still and the sum is easy
    // to read: A = T(100), M = 31 x 100.
    core.write_memory(0x4011, 100);
    core.write_memory(0x5011, 100);
    core.bus_mut().tick();

    assert_eq!(mixer::TND_TABLE[100], core.apu().mixer_apu());
    assert_eq!(3100, core.apu().mixer_expansion());
    assert_eq!(
        core.apu().mixer_apu() + 3100,
        core.apu().mixer_output(),
        "S = A + M below full scale"
    );
    assert!(
        (f64::from(core.apu().mixer_output()) / 32_768.0 - core.apu().mixed_level()).abs() < 1e-12
    );

    // Both pulses at volume 15 on top of a DMC level of 127 take `A` to 8 462 + 18 376, and a
    // PCM level of 255 takes `M` to 7 905: the sum is past full scale, where REQ-CORE-073 asks
    // for a clip rather than a wrap.
    core.write_memory(0x4015, 0x03);
    core.write_memory(0x4000, 0xBF);
    core.write_memory(0x4001, 0x08); // Negate with a shift of 0: the sweep never mutes.
    core.write_memory(0x4002, 100);
    core.write_memory(0x4003, 0xF8);
    core.write_memory(0x4004, 0xBF);
    core.write_memory(0x4005, 0x08);
    core.write_memory(0x4006, 100);
    core.write_memory(0x4007, 0xF8);
    core.write_memory(0x4011, 127);
    core.write_memory(0x5011, 255);

    let mut peak_apu = 0;
    let mut peak_output = 0;
    let mut peak_level = 0.0;
    for _ in 0..4000 {
        core.bus_mut().tick();
        if core.apu().mixer_apu() > peak_apu {
            peak_apu = core.apu().mixer_apu();
            peak_output = core.apu().mixer_output();
            peak_level = core.apu().mixed_level();
        }
    }

    assert_eq!(
        mixer::PULSE_TABLE[30] + mixer::TND_TABLE[127],
        peak_apu,
        "A at the peak"
    );
    assert_eq!(31 * 255, core.apu().mixer_expansion());
    assert!(peak_apu + (31 * 255) > mixer::FULL_SCALE);
    assert_eq!(mixer::FULL_SCALE, peak_output);
    assert!((0.999_96..=1.0).contains(&peak_level), "{peak_level}");
}

// ---------------------------------------------------------------- REQ-CORE-075, registers

/// REQ-CORE-075 (AMD-054), first criterion, second half: a write of 0 to $5011 leaves the level
/// alone and raises the PCM IRQ flag; a read of $5010 then returns $80 and the next one $00.
/// The flag never pulls the CPU's IRQ line (an accepted divergence).
#[test]
fn req_core_075_a_write_of_zero_to_5011_keeps_the_level_and_raises_the_pcm_irq() {
    let mut core = core();

    core.write_memory(0x5010, 0x80); // PCM IRQ enabled, write mode.
    core.write_memory(0x5011, 200);
    core.bus_mut().tick();
    assert_eq!(31 * 200, core.apu().mixer_expansion());
    assert_eq!(0x00, read(&mut core, 0x5010));

    core.write_memory(0x5011, 0x00);
    core.bus_mut().tick();

    // The level is unchanged, and the flag is up.
    assert_eq!(31 * 200, core.apu().mixer_expansion());
    assert_eq!(0x80, read(&mut core, 0x5010));

    // Reading it acknowledged it.
    assert_eq!(0x00, read(&mut core, 0x5010));

    // And it is not the CPU's IRQ line.
    assert!(!CpuBus::irq_asserted(core.bus()));
}

/// REQ-CORE-081 — the host and Lua accessors read with no side effect, so peeking at $5010 must
/// not acknowledge the PCM IRQ flag.
#[test]
fn req_core_081_peeking_at_5010_does_not_acknowledge_the_pcm_irq() {
    let mut core = core();

    core.write_memory(0x5011, 0x00);
    assert_eq!(0x80, core.read_memory(0x5010));
    assert_eq!(0x80, core.read_memory(0x5010));
    assert_eq!(0x80, read(&mut core, 0x5010));
    assert_eq!(0x00, core.read_memory(0x5010));
}

/// REQ-CORE-075 (AMD-054) — $5015: bits 0 and 1 enable the two squares, clearing one puts its
/// length counter to 0, and a read reports the two counters.
#[test]
fn req_core_075_5015_enables_the_squares_and_reports_their_length_counters() {
    let mut core = core();

    assert_eq!(0x00, read(&mut core, 0x5015));

    start_both_squares(&mut core, 100);
    core.bus_mut().tick();

    assert_eq!(0x03, read(&mut core, 0x5015));
    assert!(peak_expansion(&mut core, 4000) > 0);

    // Clearing bit 0 silences that channel at once: its length counter goes to 0.
    core.write_memory(0x5015, 0x02);
    core.bus_mut().tick();

    assert_eq!(0x02, read(&mut core, 0x5015));
    assert_eq!(4623, peak_expansion(&mut core, 4000));

    core.write_memory(0x5015, 0x00);
    core.bus_mut().tick();

    assert_eq!(0x00, read(&mut core, 0x5015));
    assert_eq!(0, peak_expansion(&mut core, 4000));
}

/// REQ-CORE-075 (AMD-054) — the squares take the duty, the timer and the muting of periods
/// below 8 from the APU's pulses, and have **no sweep unit**: writing the register an APU pulse
/// would read as a sweep setting changes nothing, and in particular a period above $3FF — which
/// an APU pulse with a zeroed sweep register mutes — stays audible.
#[test]
fn req_core_075_the_squares_have_no_sweep_unit() {
    for (period, audible) in [
        (7, false), // Below 8: muted, as on an APU pulse.
        (8, true),
        (0x400, true), // An APU pulse with $4001 = 0 would mute this one.
        (0x7FF, true),
    ] {
        let mut core = core();
        start_square(&mut core, 0, period);
        core.write_memory(0x5001, 0x00); // The APU's sweep register: ignored by the board.

        let cycles = 8 * 2 * (period as usize + 1);
        assert_eq!(
            audible,
            peak_expansion(&mut core, cycles) > 0,
            "period {period}"
        );
    }
}

/// REQ-CORE-075 (AMD-054) — the squares use the four duty sequences of an APU pulse, with the
/// same high fractions.
#[test]
fn req_core_075_the_squares_use_the_duty_sequences_of_an_apu_pulse() {
    for (duty, expected) in [
        (0u8, 1.0 / 8.0),
        (1, 2.0 / 8.0),
        (2, 4.0 / 8.0),
        (3, 6.0 / 8.0),
    ] {
        duty_fraction(duty, expected);
    }
}

/// Twenty full sequences: 8 steps of (period + 1) APU cycles, two CPU cycles each.
const DUTY_CYCLES: usize = 20 * 8 * 2 * 101;

fn duty_fraction(duty: u8, expected: f64) {
    let mut core = core();
    start_square(&mut core, 0, 100);
    core.write_memory(0x5000, (duty << 6) | 0x3F); // Length halted, constant volume 15.

    let mut high = 0i32;
    for _ in 0..DUTY_CYCLES {
        core.bus_mut().tick();
        if core.apu().mixer_expansion() > 0 {
            high += 1;
        }
    }

    let fraction = f64::from(high) / DUTY_CYCLES as f64;
    assert!(
        (fraction - expected).abs() <= 0.01,
        "duty {duty}: {fraction} vs {expected}"
    );
}

// ---------------------------------------------------------------- REQ-CORE-075, cadence

/// Shortest timer period that is not muted (REQ-CORE-075 keeps the APU's cut below 8), so that
/// one turn of the duty sequencer is as short as possible and the cadence tests can read the
/// envelope's volume often.
const SHORT_PERIOD: i32 = 8;

/// CPU cycles one turn of the duty sequencer takes at [`SHORT_PERIOD`]: eight steps of nine APU
/// cycles, each of two CPU cycles.
const SEQUENCE_CYCLES: i64 = 8 * (SHORT_PERIOD as i64 + 1) * 2;

/// Runs one turn of the duty sequencer and returns the volume the first square was driven at
/// over it: the channel's output is only non-zero for part of a turn, so the volume has to be
/// read from the peak rather than from any one cycle.
fn step_one_sequence(core: &mut NesCore) -> i32 {
    let peak = peak_expansion(core, SEQUENCE_CYCLES as usize);
    for volume in 0..=15i32 {
        if mixer::MMC5_PULSE_TABLE[volume as usize] == peak {
            return volume;
        }
    }
    -1
}

/// Seconds `cycles` CPU cycles last, on the current region.
fn seconds(core: &NesCore, cycles: i64) -> f64 {
    let timing = RegionTiming::for_region(core.region());
    let cycles_per_frame = f64::from(RegionTiming::DOTS_PER_SCANLINE)
        * f64::from(timing.scanlines_per_frame)
        * f64::from(timing.dots_per_cycle_denominator)
        / f64::from(timing.dots_per_cycle_numerator);

    cycles as f64 / (f64::from(timing.nominal_frames_per_second) * cycles_per_frame)
}

/// REQ-CORE-075 (AMD-054), second criterion: a square with a decaying envelope (start volume
/// 15, envelope period 0) goes from 15 to 0 in 15 x 1/240 s ± 5 %, whatever `$4017` holds — the
/// board has no frame sequencer, so its cadence is fixed.
#[test]
fn req_core_075_the_envelope_runs_at_a_fixed_cadence_whatever_4017_holds() {
    for frame_counter in [0x00, 0x80, 0xC0] {
        envelope_cadence(frame_counter);
    }
}

fn envelope_cadence(frame_counter: u8) {
    let mut core = core();

    core.write_memory(0x4017, frame_counter);

    core.write_memory(0x5015, 0x01);
    core.write_memory(0x5000, 0x80); // Duty 2, envelope mode, period 0, no loop.
    core.write_memory(0x5002, SHORT_PERIOD as u8);
    core.write_memory(0x5003, 0x08); // Length index 1 (254 steps at 120 Hz), envelope restart.

    // Wait for the first envelope clock, which loads the decay with 15, then time the fifteen
    // clocks that take it back to 0.
    let mut cycles = 0;
    let mut start = -1;
    let mut end = -1;
    while cycles < 400_000 && end < 0 {
        let volume = step_one_sequence(&mut core);
        cycles += SEQUENCE_CYCLES;

        if start < 0 {
            if volume == 15 {
                start = cycles;
            }
        } else if volume == 0 {
            end = cycles;
        }
    }

    assert!(start >= 0, "the envelope never reached 15");
    assert!(end > start, "the envelope never reached 0");

    let measured = seconds(&core, end - start);
    let expected = 15.0 / 240.0;
    assert!(
        (expected * 0.95..=expected * 1.05).contains(&measured),
        "{measured} s vs {expected} s"
    );
}

/// REQ-CORE-075 (AMD-054) — the length counter runs at 120 Hz, one envelope clock in two: a
/// counter loaded with 10 steps therefore silences the channel after 10 / 120 s.
#[test]
fn req_core_075_the_length_counter_runs_at_half_the_envelope_cadence() {
    let mut core = core();

    core.write_memory(0x5015, 0x01);
    core.write_memory(0x5000, 0x9F); // Duty 2, constant volume 15, length running.
    core.write_memory(0x5002, SHORT_PERIOD as u8);
    core.write_memory(0x5003, 0x00); // Length index 0: 10 steps.

    let mut cycles = 0;
    let mut silent = -1;
    while cycles < 400_000 && silent < 0 {
        let volume = step_one_sequence(&mut core);
        cycles += SEQUENCE_CYCLES;
        if volume == 0 {
            silent = cycles;
        }
    }

    assert!(silent > 0, "the length counter never ran out");
    assert_eq!(0x00, read(&mut core, 0x5015));

    let measured = seconds(&core, silent);
    let expected = 10.0 / 120.0;
    assert!(
        (expected * 0.95..=expected * 1.05).contains(&measured),
        "{measured} s vs {expected} s"
    );
}

/// REQ-CORE-090, REQ-CORE-091 — the board's sound is part of the savestate: restoring one taken
/// mid-tone brings the level, the envelope and the PCM level back.
#[test]
fn req_core_090_the_board_sound_survives_a_savestate_round_trip() {
    let mut core = core();
    start_square(&mut core, 0, 100);
    core.write_memory(0x5011, 77);

    for _ in 0..500 {
        core.bus_mut().tick();
    }

    let state = core.save_state(true).unwrap();
    let mut expected = [0; 256];
    for slot in &mut expected {
        core.bus_mut().tick();
        *slot = core.apu().mixer_expansion();
    }

    core.try_load_state(&state, 0).unwrap();
    for slot in expected {
        core.bus_mut().tick();
        assert_eq!(slot, core.apu().mixer_expansion());
    }
}

/// REQ-CORE-091 — a block written before AMD-054 carries no sound state, and loads with the
/// board silent rather than being refused. The version of such a block is simulated by patching
/// the format version in the header of a fresh one; the sound block it still carries is then
/// left unread, as the trailing bytes a host may place after the core block are.
#[test]
fn req_core_091_a_block_written_before_the_board_sound_still_loads() {
    let mut core = core();
    start_square(&mut core, 0, 100);
    core.write_memory(0x5011, 77);
    for _ in 0..500 {
        core.bus_mut().tick();
    }

    let mut state = core.save_state(true).unwrap();
    // The format version sits right after the four magic bytes.
    state[4..8].copy_from_slice(&NesCore::STATE_VERSION_4.to_le_bytes());

    core.try_load_state(&state, 0).unwrap();
    core.bus_mut().tick();
    assert_eq!(
        0,
        core.apu().mixer_expansion(),
        "the board is silent after an older block"
    );
}

/// REQ-CORE-004 — power-on leaves the board silent, and the APU reads 0 at the test point of
/// REQ-CORE-076.
#[test]
fn req_core_004_the_board_powers_on_silent() {
    let mut core = core();
    start_square(&mut core, 0, 100);
    core.write_memory(0x5011, 200);
    core.bus_mut().tick();
    assert!(core.apu().mixer_expansion() > 0);

    core.reset();
    core.bus_mut().tick();

    assert_eq!(0, core.apu().mixer_expansion());
    assert_eq!(0, core.apu().mixer_output());
    assert_eq!(0, peak_expansion(&mut core, 4000));
}

/// REQ-CORE-075 — a cartridge that is not an MMC5 has no expansion audio at all, so `M` stays
/// 0 and the APU's own mixing is the whole output. This is what makes the board's sound cost
/// nothing for every other game.
#[test]
fn req_core_075_a_board_without_expansion_audio_leaves_m_at_zero() {
    for mapper in [0, 1, 4, 7] {
        let image = MapperRom::new(mapper).image();
        let mut core = NesCore::new();
        assert!(core.load_rom(&image, "Other.nes").success);
        assert_eq!(
            None,
            core.cartridge().unwrap().expansion_audio_output(),
            "mapper {mapper}"
        );

        // Writes in the MMC5's register window reach no sound unit.
        core.write_memory(0x5011, 200);
        core.write_memory(0x5015, 0x03);
        core.write_memory(0x5000, 0xBF);
        assert_eq!(0, peak_expansion(&mut core, 2000), "mapper {mapper}");
    }
}
