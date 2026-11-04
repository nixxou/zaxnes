//! The MMC5's expansion audio (REQ-CORE-075, AMD-054 as amended by **AMD-061 §6**): two square
//! channels, the PCM channel, $5010 and $5015, and the way their output `M` is **subtracted**
//! from the APU's `A` in the mixer (REQ-CORE-073), read at the test point of REQ-CORE-076.
//!
//! The numbered criteria are §6.5's, which replace AMD-054's 4 623 / 8 000 / 3 968.

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

/// AMD-061 §6.5, first two criteria: a single MMC5 square at volume 15 contributes exactly what
/// a single APU pulse at volume 15 does, and the two of them exactly what two APU pulses do —
/// "the sound output of the square channels are equivalent in volume to the corresponding APU
/// channels" (nesdev, "MMC5 audio"). `M` is reported as a positive quantity; §6.2's reversed
/// polarity is what the mixer then does with it, and
/// `amd_061_the_board_output_is_subtracted_from_the_apu` pins that.
#[test]
fn amd_061_the_squares_contribute_the_opposite_of_the_apu_pulses() {
    let mut one = core();
    start_square(&mut one, 0, 100);
    assert_eq!(mixer::PULSE_TABLE[15], peak_expansion(&mut one, 4000));

    let mut other = core();
    start_square(&mut other, 1, 100);
    assert_eq!(mixer::PULSE_TABLE[15], peak_expansion(&mut other, 4000));

    // Both squares on the same period and started in phase: one table index of 30, not two of
    // 15, which is the whole point of a non-linear mixer.
    let mut both = core();
    start_both_squares(&mut both, 100);
    assert_eq!(mixer::PULSE_TABLE[30], peak_expansion(&mut both, 4000));

    // And those are the APU's own numbers, from REQ-CORE-073's criterion.
    assert_eq!(4890, mixer::PULSE_TABLE[15]);
    assert_eq!(8462, mixer::PULSE_TABLE[30]);
}

/// AMD-061 §6.4 — the PCM channel is "equivalent in volume to the APU with equivalent input …
/// but using the extra high bit of input it can become twice as loud", so it reads the APU's own
/// triangle/noise/DMC curve over a whole byte: the first 203 entries of its table *are*
/// `TND_TABLE`, and the 53 above them are the eighth bit the DMC has no room for. The
/// reference's step of 31 per unit is gone.
#[test]
fn amd_061_the_pcm_channel_follows_the_dmc_curve_over_a_full_byte() {
    assert_eq!(256, mixer::MMC5_PCM_TABLE.len());
    assert_eq!(0, mixer::MMC5_PCM_TABLE[0]);

    // The DMC's range, byte for byte.
    for i in 0..mixer::TND_TABLE.len() {
        assert_eq!(mixer::TND_TABLE[i], mixer::MMC5_PCM_TABLE[i], "index {i}");
    }

    // The whole table is `r(K · T(i))` on the APU's own `K`, the eighth bit included.
    for i in 0..mixer::MMC5_PCM_TABLE.len() {
        assert_eq!(
            mixer::round_to_even(mixer::NORMALISATION * mixer::tnd(i)),
            mixer::MMC5_PCM_TABLE[i],
            "index {i}"
        );
    }

    // Monotonic and strictly above the DMC's full scale at the top.
    assert!(mixer::MMC5_PCM_TABLE[255] > mixer::TND_TABLE[127]);

    // And a level written to $5011 is read back at that entry.
    let mut pcm = core();
    pcm.write_memory(0x5011, 128);
    pcm.bus_mut().tick();
    assert_eq!(mixer::MMC5_PCM_TABLE[128], pcm.apu().mixer_expansion());
}

/// AMD-061 §6.5, third criterion: "une voie carrée MMC5 de période 4 produit un signal non nul".
/// §6.1 takes the cut below period 8 off the board's squares — "frequency values less than 8 do
/// not silence the MMC5 pulse channels; they can output ultrasonic frequencies" — and leaves it
/// on the APU's pulses.
#[test]
fn amd_061_a_square_below_period_eight_is_not_silenced() {
    for period in [0, 1, 4, 7] {
        let mut core = core();
        start_square(&mut core, 0, period);
        assert!(
            peak_expansion(&mut core, 400) > 0,
            "period {period} must still be audible"
        );
    }

    // The APU's own pulse keeps its cut, which is the other half of §6.1.
    let mut apu = core();
    apu.write_memory(0x4015, 0x01);
    apu.write_memory(0x4000, 0xBF);
    apu.write_memory(0x4001, 0x08);
    apu.write_memory(0x4002, 4);
    apu.write_memory(0x4003, 0xF8);
    let mut peak = 0;
    for _ in 0..400 {
        apu.bus_mut().tick();
        peak = peak.max(apu.apu().mixer_apu());
    }
    assert_eq!(0, peak, "an APU pulse below period 8 is still muted");
}

/// AMD-061 §6.2 — `M` is **subtracted** from `A`, and `S` is clipped to `-32 768..=32 767`
/// instead of `0..=32 767`: "the polarity of all MMC5 channels is reversed compared to the APU"
/// (nesdev, "MMC5 audio").
#[test]
fn amd_061_the_board_output_is_subtracted_from_the_apu() {
    let mut core = core();

    // A DMC level written through $4011 is a pure step, so `A` sits still and the difference is
    // easy to read: A = T(100), M = T(100) as well, so the two cancel exactly. That the same
    // index gives the same magnitude on both sides is §6.4 restated.
    core.write_memory(0x4011, 100);
    core.write_memory(0x5011, 100);
    core.bus_mut().tick();

    assert_eq!(mixer::TND_TABLE[100], core.apu().mixer_apu());
    assert_eq!(mixer::MMC5_PCM_TABLE[100], core.apu().mixer_expansion());
    assert_eq!(0, core.apu().mixer_output(), "S = A - M, and here A = M");

    // With the APU silent, `S` goes negative, which AMD-054's clip to 0 could not express.
    core.write_memory(0x4011, 0);
    core.bus_mut().tick();
    assert_eq!(0, core.apu().mixer_apu());
    assert_eq!(-mixer::MMC5_PCM_TABLE[100], core.apu().mixer_output());
    assert!(
        (f64::from(core.apu().mixer_output()) / 32_768.0 - core.apu().mixed_level()).abs() < 1e-12
    );

    // The clip is a floor, not a wrap: the board's full scale under a silent APU is past
    // -32 768, where §6.2 asks for the floor.
    core.write_memory(0x5011, 255);
    start_both_squares(&mut core, 100);
    let mut trough = 0;
    let mut peak_expansion = 0;
    for _ in 0..4000 {
        core.bus_mut().tick();
        trough = trough.min(core.apu().mixer_output());
        peak_expansion = peak_expansion.max(core.apu().mixer_expansion());
    }

    assert_eq!(
        mixer::PULSE_TABLE[30] + mixer::MMC5_PCM_TABLE[255],
        peak_expansion,
        "M at the peak"
    );
    assert!(peak_expansion > -mixer::MIN_SCALE, "{peak_expansion}");
    assert_eq!(mixer::MIN_SCALE, trough);
}

/// AMD-061 §6.2 — a game **without** expansion audio is untouched by the new polarity: `M` is 0,
/// so `S` is `A` exactly, and the clip never bites because `A` cannot leave `0..=32 767`.
#[test]
fn amd_061_the_mix_of_a_game_without_expansion_audio_is_unchanged() {
    assert_eq!(0, mixer::clip(0, 0));
    assert_eq!(1234, mixer::clip(1234, 0));
    assert_eq!(mixer::FULL_SCALE, mixer::clip(mixer::FULL_SCALE, 0));
    assert_eq!(
        mixer::FULL_SCALE,
        mixer::PULSE_TABLE[30] + mixer::TND_TABLE[202],
        "A at full scale is exactly 32 767, so the upper clip is unreachable"
    );
}

// ---------------------------------------------------------------- REQ-CORE-075, registers

/// REQ-CORE-075 (AMD-054) — a write of 0 to $5011 leaves the level alone and raises the PCM IRQ
/// flag; a read of $5010 then returns $80 and the next one $00. **AMD-061 §6.3**: while bit 7 of
/// $5010 allows it, that flag now pulls the CPU's IRQ line, and the read that acknowledges it
/// releases it.
#[test]
fn amd_061_the_pcm_irq_flag_pulls_the_cpu_irq_line() {
    let mut core = core();

    core.write_memory(0x5010, 0x80); // PCM IRQ enabled, write mode.
    core.write_memory(0x5011, 200);
    core.bus_mut().tick();
    assert_eq!(mixer::MMC5_PCM_TABLE[200], core.apu().mixer_expansion());
    assert_eq!(0x00, read(&mut core, 0x5010));
    assert!(!CpuBus::irq_asserted(core.bus()), "no flag, no IRQ");

    core.write_memory(0x5011, 0x00);
    core.bus_mut().tick();

    // The level is unchanged, the flag is up, and the line is down.
    assert_eq!(mixer::MMC5_PCM_TABLE[200], core.apu().mixer_expansion());
    assert!(CpuBus::irq_asserted(core.bus()), "the flag drives /IRQ");
    assert_eq!(0x80, read(&mut core, 0x5010));

    // Reading it acknowledged it, which also released the line.
    assert_eq!(0x00, read(&mut core, 0x5010));
    assert!(!CpuBus::irq_asserted(core.bus()));
}

/// AMD-061 §6.3 — `Cart /IRQ line = NOT(irqTrip AND irqEnable)` (nesdev's own pseudocode): with
/// bit 7 of $5010 clear the flag still latches and still reads back, but it drives nothing.
#[test]
fn amd_061_the_pcm_irq_needs_bit_7_of_5010_to_reach_the_cpu() {
    let mut core = core();

    core.write_memory(0x5010, 0x00); // Write mode, IRQ *not* enabled.
    core.write_memory(0x5011, 0x00);
    core.bus_mut().tick();

    assert!(!CpuBus::irq_asserted(core.bus()), "disabled, so no IRQ");
    assert_eq!(0x80, read(&mut core, 0x5010), "but the flag is there");
    assert_eq!(0x00, read(&mut core, 0x5010));
}

/// AMD-061 §6.3, the read mode of $5010: with bit 0 set, "une lecture CPU dans $8000-$BFFF place
/// l'octet lu dans le niveau PCM ; une lecture de valeur 0 lève le drapeau d'IRQ PCM". A read
/// outside that window does neither, and the byte the read returns is unchanged.
#[test]
fn amd_061_read_mode_takes_the_pcm_level_from_a_read_of_8000_bfff() {
    let mut core = core();

    // The image numbers each 8 KiB PRG bank with its own index, so banking a known ROM bank
    // into each window makes the byte a read returns known too. Bit 7 of $511x selects ROM
    // over RAM, and the board powers on in PRG mode 3, four 8 KiB windows.
    core.write_memory(0x5114, 0x80 | 3); // $8000-$9FFF: bank 3.
    core.write_memory(0x5115, 0x80 | 5); // $A000-$BFFF: bank 5.
    core.write_memory(0x5116, 0x80 | 6); // $C000-$DFFF: bank 6, outside the window.
    assert_eq!(3, core.read_memory(0x8000));
    assert_eq!(5, core.read_memory(0xA000));
    assert_eq!(6, core.read_memory(0xC000));

    // Out of read mode the read changes nothing.
    core.write_memory(0x5010, 0x00);
    assert_eq!(3, read(&mut core, 0x8000));
    core.bus_mut().tick();
    assert_eq!(0, core.apu().mixer_expansion());

    // In read mode it becomes the level, and the read still returns the same byte.
    core.write_memory(0x5010, 0x01);
    assert_eq!(3, read(&mut core, 0x8000));
    core.bus_mut().tick();
    assert_eq!(mixer::MMC5_PCM_TABLE[3], core.apu().mixer_expansion());

    // The whole of $8000-$BFFF feeds it, not just the first window.
    assert_eq!(5, read(&mut core, 0xA000));
    core.bus_mut().tick();
    assert_eq!(mixer::MMC5_PCM_TABLE[5], core.apu().mixer_expansion());

    // $C000-$FFFF is outside the window: the level stays where the last $8000-$BFFF read put it.
    assert_eq!(6, read(&mut core, 0xC000));
    core.bus_mut().tick();
    assert_eq!(mixer::MMC5_PCM_TABLE[5], core.apu().mixer_expansion());

    // And a write to $5011 is not how a level is set in read mode on real hardware, but
    // AMD-054's clause on it is unamended, so it still takes: this pins what we do, not what
    // the board does (see Q-115).
    core.write_memory(0x5011, 40);
    core.bus_mut().tick();
    assert_eq!(mixer::MMC5_PCM_TABLE[40], core.apu().mixer_expansion());
}

/// AMD-061 §6.3 — in read mode, a byte of 0 read from $8000-$BFFF raises the PCM IRQ flag and
/// leaves the level alone, exactly as a write of 0 does in write mode.
#[test]
fn amd_061_read_mode_raises_the_pcm_irq_on_a_byte_of_zero() {
    let mut core = core();

    // Bank 3 at $8000 to park a non-zero level, then bank 0 — whose every byte is 0, the image
    // numbering each bank with its index — to read the byte that trips the flag.
    core.write_memory(0x5010, 0x81); // Read mode, PCM IRQ enabled.
    core.write_memory(0x5114, 0x80 | 3);
    assert_eq!(3, read(&mut core, 0x8000));
    core.bus_mut().tick();
    assert_eq!(mixer::MMC5_PCM_TABLE[3], core.apu().mixer_expansion());
    assert!(!CpuBus::irq_asserted(core.bus()));

    core.write_memory(0x5114, 0x80);
    assert_eq!(0, read(&mut core, 0x8000));
    core.bus_mut().tick();

    // The level is unchanged and the flag is up, exactly as for a write of 0 in write mode.
    assert_eq!(mixer::MMC5_PCM_TABLE[3], core.apu().mixer_expansion());
    assert!(
        CpuBus::irq_asserted(core.bus()),
        "a byte of 0 trips the IRQ"
    );
    assert_eq!(0x80, read(&mut core, 0x5010));
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
    assert_eq!(mixer::PULSE_TABLE[15], peak_expansion(&mut core, 4000));

    core.write_memory(0x5015, 0x00);
    core.bus_mut().tick();

    assert_eq!(0x00, read(&mut core, 0x5015));
    assert_eq!(0, peak_expansion(&mut core, 4000));
}

/// REQ-CORE-075 (AMD-054) — the squares take the duty and the timer from the APU's pulses, and
/// have **no sweep unit**: writing the register an APU pulse would read as a sweep setting
/// changes nothing, and in particular a period above $3FF — which an APU pulse with a zeroed
/// sweep register mutes — stays audible. Since AMD-061 §6.1 a period *below* 8 is audible too,
/// which `amd_061_a_square_below_period_eight_is_not_silenced` is about.
#[test]
fn req_core_075_the_squares_have_no_sweep_unit() {
    for period in [7, 8, 0x400, 0x7FF] {
        let mut core = core();
        start_square(&mut core, 0, period);
        core.write_memory(0x5001, 0x00); // The APU's sweep register: ignored by the board.

        let cycles = 8 * 2 * (period as usize + 1);
        assert!(
            peak_expansion(&mut core, cycles) > 0,
            "period {period} must be audible"
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

/// A short timer period, so that one turn of the duty sequencer is quick and the cadence tests
/// can read the envelope's volume often. AMD-061 §6.1 would allow a shorter one — the board has
/// no cut below 8 any more — but 8 keeps these two tests comparable with their AMD-054 versions.
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
        if mixer::PULSE_TABLE[volume as usize] == peak {
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
    core.write_memory(0x5003, 0x08); // Length index 1 (254 steps at 240 Hz), envelope restart.

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

/// AMD-061 §6.5, fifth criterion: the length counter runs at **240 Hz**, the envelope's own
/// cadence, so a counter loaded with 10 steps silences the channel after 10 / 240 s whatever
/// `$4017` holds. §6.3 withdraws AMD-054's 120 Hz divergence: "Length counter operates twice as
/// fast as the APU length counter", both "fixed to a 240Hz update rate" (nesdev, "MMC5 audio").
#[test]
fn amd_061_the_length_counter_runs_at_240_hz_whatever_4017_holds() {
    for frame_counter in [0x00, 0x80, 0xC0] {
        length_cadence(frame_counter);
    }
}

fn length_cadence(frame_counter: u8) {
    let mut core = core();

    core.write_memory(0x4017, frame_counter);

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
    let expected = 10.0 / 240.0;
    assert!(
        (expected * 0.95..=expected * 1.05).contains(&measured),
        "$4017 = {frame_counter:#04X}: {measured} s vs {expected} s"
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
