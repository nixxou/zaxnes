//! The Sunsoft 5B's expansion audio (REQ-CORE-075, which leaves it optional; **Q-118** for its
//! level and its polarity): the three square channels of the YM2149F mapper 69 carries, their
//! logarithmic volume, the envelope generator and the noise generator.
//!
//! Everything here is read from the public nesdev page "Sunsoft 5B audio". Mesen 2 has neither
//! the envelope nor the noise, so there was nothing to adapt beyond the volume law, which the two
//! sources agree on.

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

/// A machine with a synthetic mapper-69 cartridge, powered on.
fn core() -> NesCore {
    let image = MapperRom::new(69).image();
    let mut core = NesCore::new();
    let load = core.load_rom(&image, "Sunsoft5b.nes");
    assert!(load.success, "{:?}", load.error);
    core
}

/// Writes one of the chip's sixteen registers: `$C000-$DFFF` is "Audio Register Select" and
/// `$E000-$FFFF` is "Audio Register Write".
fn write(core: &mut NesCore, register: u8, value: u8) {
    core.write_memory(0xC000, register);
    core.write_memory(0xE000, value);
}

/// Largest and smallest `M` the test point of REQ-CORE-076 reports over `cycles` CPU cycles.
fn expansion_range(core: &mut NesCore, cycles: usize) -> (i32, i32) {
    let mut low = i32::MAX;
    let mut high = i32::MIN;
    for _ in 0..cycles {
        core.bus_mut().tick();
        let level = core.apu().mixer_expansion();
        low = low.min(level);
        high = high.max(level);
    }
    (low, high)
}

fn peak_expansion(core: &mut NesCore, cycles: usize) -> i32 {
    expansion_range(core, cycles).1
}

/// Sets channel `channel` up as a plain square at `volume`: its 12-bit period, its volume with
/// the envelope bit clear, and `$07` enabling the tone and disabling the noise on all three.
fn plain_square(core: &mut NesCore, channel: u8, period: u16, volume: u8) {
    write(core, channel * 2, (period & 0xFF) as u8);
    write(core, channel * 2 + 1, (period >> 8) as u8);
    // `--CB Acba`: noise disabled on C/B/A (bits 3-5 set), tone enabled only on `channel`.
    write(core, 0x07, 0x38 | (0x07 & !(1 << channel)));
    write(core, 0x08 + channel, volume & 0x0F);
}

// ---------------------------------------------------------------- the volume law, Q-118

/// The table is the page's law: thirty-two levels "1.5db" apart, "maximum level 31 corresponds to
/// the 4-bit volume register level 15". Recomputed here with `powf` from 1.5 dB, which the
/// `const fn` in the mixer cannot use.
#[test]
fn req_core_075_the_5b_levels_are_1_5_db_apart() {
    assert_eq!(32, mixer::SUNSOFT_5B_TABLE.len());
    assert_eq!(0, mixer::SUNSOFT_5B_TABLE[0]);

    // Q-118's anchor: level 31 of one channel is one APU pulse at volume 15.
    assert_eq!(mixer::PULSE_TABLE[15], mixer::SUNSOFT_5B_TABLE[31]);
    assert_eq!(4890, mixer::SUNSOFT_5B_TABLE[31]);

    let anchor = mixer::NORMALISATION * mixer::pulse(15);
    for level in 1..32usize {
        let steps = 31 - level;
        let expected = mixer::round_to_even(anchor / 10f64.powf(1.5 * steps as f64 / 20.0));
        assert_eq!(expected, mixer::SUNSOFT_5B_TABLE[level], "level {level}");
    }

    // A step of the volume register is two levels, so 3 dB: every pair of neighbours is a
    // factor of 10^(3/20), within what rounding to an integer can do to the small entries.
    let step = 10f64.powf(3.0 / 20.0);
    for volume in 1..15u8 {
        let low = f64::from(mixer::SUNSOFT_5B_TABLE[mixer::sunsoft_5b_level(volume)]);
        let high = f64::from(mixer::SUNSOFT_5B_TABLE[mixer::sunsoft_5b_level(volume + 1)]);
        let ratio = high / low;
        assert!(
            (ratio - step).abs() / step < 0.02,
            "volume {volume}: {ratio}, expected {step}"
        );
    }
}

/// "maximum level 31 corresponds to the 4-bit volume register level 15", so the four-bit volume
/// lands on the odd levels; volume 0 is silence (Q-118).
#[test]
fn req_core_075_the_volume_register_lands_on_the_odd_levels() {
    assert_eq!(0, mixer::sunsoft_5b_level(0));
    for volume in 1..=15u8 {
        assert_eq!(2 * usize::from(volume) + 1, mixer::sunsoft_5b_level(volume));
    }
}

// ---------------------------------------------------------------- the three squares

/// One square at volume 15 reaches exactly `SUNSOFT_5B_TABLE[31]`, and the channel's output is a
/// square: `M` spends half its time at 0.
#[test]
fn req_core_075_one_5b_square_swings_between_zero_and_its_level() {
    let mut core = core();
    plain_square(&mut core, 0, 100, 15);

    let (low, high) = expansion_range(&mut core, 20_000);
    assert_eq!(0, low);
    assert_eq!(mixer::SUNSOFT_5B_TABLE[31], high);
}

/// "The three output channels are mixed together linearly" — so three squares in phase at
/// volume 15 give three times one, not one table entry at a higher index the way the MMC5's two
/// squares do (Q-118).
#[test]
fn req_core_075_the_three_5b_squares_add_linearly() {
    let mut core = core();
    // The same period on all three, written one after another, and `$07` enabling all three
    // tones at once.
    for channel in 0..3u8 {
        write(&mut core, channel * 2, 100);
        write(&mut core, channel * 2 + 1, 0);
        write(&mut core, 0x08 + channel, 0x0F);
    }

    write(&mut core, 0x07, 0x38);

    assert_eq!(
        3 * mixer::SUNSOFT_5B_TABLE[31],
        peak_expansion(&mut core, 20_000)
    );
    assert_eq!(14_670, 3 * mixer::SUNSOFT_5B_TABLE[31]);
}

/// The 12-bit period is "Clock / (32 × Period)": the square's full cycle is 32 × period CPU
/// cycles, so doubling the period doubles the time `M` spends on each half.
#[test]
fn req_core_075_the_5b_square_period_is_thirty_two_cycles_per_unit() {
    for period in [4u16, 8, 16] {
        let mut core = core();
        plain_square(&mut core, 0, period, 15);
        // Let the counters settle, then measure one full cycle by counting the cycles the
        // output spends high over a long window.
        let window = 32 * usize::from(period) * 20;
        let mut high = 0i32;
        for _ in 0..window {
            core.bus_mut().tick();
            if core.apu().mixer_expansion() != 0 {
                high += 1;
            }
        }

        // A 50 % duty, within the granularity of the chip's divide-by-16 prescaler.
        let ratio = f64::from(high) / window as f64;
        assert!((0.40..0.60).contains(&ratio), "period {period}: {ratio}");
    }
}

/// `$07` is `--CB Acba`: "A bit of 0 enables the noise/tone on the specified channel, and a bit
/// of 1 disables it. If both bits are 1, the channel outputs a constant signal at the specified
/// volume."
#[test]
fn req_core_075_disabling_both_generators_gives_a_constant_level() {
    let mut core = core();
    write(&mut core, 0x00, 100);
    write(&mut core, 0x08, 0x0F);
    // Channel A: tone and noise both disabled, B and C disabled too.
    write(&mut core, 0x07, 0x3F);

    let (low, high) = expansion_range(&mut core, 20_000);
    assert_eq!(mixer::SUNSOFT_5B_TABLE[31], low);
    assert_eq!(mixer::SUNSOFT_5B_TABLE[31], high);
}

/// Volume 0 is the only way a channel is truly silent: with both generators disabled it outputs
/// "a constant signal at the specified volume", and level 0 of the table is 0 (Q-118). This is
/// also the power-on state of all three channels, every register being 0.
#[test]
fn req_core_075_a_5b_channel_at_volume_zero_contributes_nothing() {
    // A board that never writes $C000-$FFFF at all — which is what a 5A, the same chip without
    // the sound, does — is silent out of a power-on.
    let mut untouched = core();
    assert_eq!((0, 0), expansion_range(&mut untouched, 20_000));

    let mut written = core();
    write(&mut written, 0x00, 100);
    write(&mut written, 0x07, 0x3F); // Both generators disabled on all three: constant.
    write(&mut written, 0x08, 0x00);
    write(&mut written, 0x09, 0x00);
    write(&mut written, 0x0A, 0x00);
    assert_eq!((0, 0), expansion_range(&mut written, 20_000));
}

// ---------------------------------------------------------------- the noise generator

/// The noise is "a 1-bit random output" from "a 17-bit linear feedback shift register with taps
/// at bits 16 and 13", gating the channel like the tone does: a channel on noise alone swings
/// between 0 and its level, and not on the square's own period.
#[test]
fn req_core_075_the_5b_noise_gates_a_channel() {
    let mut core = core();
    write(&mut core, 0x06, 0x01); // Shortest noise period.
    write(&mut core, 0x08, 0x0F);
    // Noise enabled on A only (bit 3 clear), tone disabled on all three.
    write(&mut core, 0x07, 0x07 | 0x30);

    let (low, high) = expansion_range(&mut core, 40_000);
    assert_eq!(0, low);
    assert_eq!(mixer::SUNSOFT_5B_TABLE[31], high);

    // It is not periodic at the square's rate: over a window the output is neither always on
    // nor a clean half, because the shift register's run lengths vary.
    let mut transitions = 0;
    let mut previous = core.apu().mixer_expansion();
    for _ in 0..40_000 {
        core.bus_mut().tick();
        let level = core.apu().mixer_expansion();
        if level != previous {
            transitions += 1;
            previous = level;
        }
    }

    assert!(transitions > 10, "the noise never changed: {transitions}");
}

// ---------------------------------------------------------------- the envelope generator

/// `$08`-`$0A` bit 4 hands a channel to the envelope generator, whose level is the five-bit one
/// directly: with a rising shape the channel climbs from `SUNSOFT_5B_TABLE[0]` to
/// `SUNSOFT_5B_TABLE[31]` instead of sitting at one volume.
#[test]
fn req_core_075_the_5b_envelope_drives_a_channels_level() {
    let mut core = core();
    write(&mut core, 0x08, 0x10); // Envelope enable, volume bits ignored.
                                  // Tone and noise both disabled on A, so the channel is a constant the envelope shapes.
    write(&mut core, 0x07, 0x3F);
    write(&mut core, 0x0B, 0x01); // Envelope period 1: one step every 16 CPU cycles.
    write(&mut core, 0x0C, 0x00);
    // `---- CAaH` = continue + attack + hold: one rising ramp, then hold at the top.
    write(&mut core, 0x0D, 0x0D);

    let (low, high) = expansion_range(&mut core, 32 * 16 + 64);
    assert_eq!(0, low, "the ramp starts at level 0");
    assert_eq!(
        mixer::SUNSOFT_5B_TABLE[31],
        high,
        "and reaches the top level"
    );

    // And then holds there: no further change.
    let (held_low, held_high) = expansion_range(&mut core, 20_000);
    assert_eq!(mixer::SUNSOFT_5B_TABLE[31], held_low);
    assert_eq!(mixer::SUNSOFT_5B_TABLE[31], held_high);
}

/// "Continue determines if oscillation continues": with `C` clear the envelope runs one ramp and
/// then stays at 0 for ever, whichever way the ramp went.
#[test]
fn req_core_075_a_5b_envelope_without_continue_decays_to_silence() {
    for shape in [0x00u8, 0x04] {
        let mut core = core();
        write(&mut core, 0x08, 0x10);
        write(&mut core, 0x07, 0x3F);
        write(&mut core, 0x0B, 0x01);
        write(&mut core, 0x0C, 0x00);
        write(&mut core, 0x0D, shape);

        // Long enough for 32 steps of 16 cycles each, several times over.
        expansion_range(&mut core, 32 * 16 * 4);
        assert_eq!(
            (0, 0),
            expansion_range(&mut core, 20_000),
            "shape {shape:#04X} must end at silence"
        );
    }
}

/// "Alternate enables up/down alternation": with `C` set, `a` set and `H` clear the envelope
/// keeps going, so it comes back down after each rise and the level keeps moving for ever.
#[test]
fn req_core_075_a_5b_alternating_envelope_never_settles() {
    let mut core = core();
    write(&mut core, 0x08, 0x10);
    write(&mut core, 0x07, 0x3F);
    write(&mut core, 0x0B, 0x01);
    write(&mut core, 0x0C, 0x00);
    write(&mut core, 0x0D, 0x0E); // Continue + attack + alternate: /\/\.

    // After many full cycles it is still sweeping the whole range.
    expansion_range(&mut core, 32 * 16 * 10);
    let (low, high) = expansion_range(&mut core, 32 * 16 * 2);
    assert_eq!(0, low);
    assert_eq!(mixer::SUNSOFT_5B_TABLE[31], high);
}

/// A write to `$0D` is an "envelope reset": it restarts the ramp at the end the attack bit names
/// and lets a held envelope go again.
#[test]
fn req_core_075_writing_the_5b_shape_register_restarts_the_envelope() {
    let mut core = core();
    write(&mut core, 0x08, 0x10);
    write(&mut core, 0x07, 0x3F);
    write(&mut core, 0x0B, 0x01);
    write(&mut core, 0x0C, 0x00);
    write(&mut core, 0x0D, 0x0D); // Attack + hold: up, then held at the top.
    expansion_range(&mut core, 32 * 16 * 2);
    assert_eq!(mixer::SUNSOFT_5B_TABLE[31], peak_expansion(&mut core, 64));

    // The same shape again: the ramp starts over, so the level drops to the bottom of the range
    // and climbs back to the top. Checked as "low again" rather than "exactly 0" because the
    // chip's divide-by-16 prescaler is wherever the register write left it, so the first step
    // may already have happened.
    write(&mut core, 0x0D, 0x0D);
    let restarted = peak_expansion(&mut core, 1);
    assert!(
        restarted < mixer::SUNSOFT_5B_TABLE[8],
        "the envelope did not restart: {restarted}"
    );
    assert_eq!(
        mixer::SUNSOFT_5B_TABLE[31],
        peak_expansion(&mut core, 32 * 16 + 64)
    );
}

// ---------------------------------------------------------------- the mix, and the other boards

/// `M` is positive and the APU **subtracts** it, as it subtracts the MMC5's (AMD-061 §6.2,
/// Q-118): the mixed output of a silent APU with the 5B playing is negative.
#[test]
fn q_118_the_5b_output_is_subtracted_from_the_apu() {
    let mut core = core();
    write(&mut core, 0x00, 100);
    write(&mut core, 0x08, 0x0F);
    write(&mut core, 0x07, 0x3F); // A constant level, both generators disabled.

    core.bus_mut().tick();
    let expansion = core.apu().mixer_expansion();
    assert_eq!(mixer::SUNSOFT_5B_TABLE[31], expansion);
    assert_eq!(mixer::clip(0, expansion), core.apu().mixer_output());
    assert!(core.apu().mixer_output() < 0);
}

/// A board with no sound of its own still reports nothing, so the mix path is untouched for it
/// (REQ-CORE-075, and the lot's rule that games without expansion audio pay nothing).
#[test]
fn req_core_075_a_board_without_expansion_audio_reports_none() {
    for mapper in [0usize, 4, 69, 5] {
        let image = MapperRom::new(mapper).image();
        let mut core = NesCore::new();
        assert!(core.load_rom(&image, "Board.nes").success);
        let has_audio = core.cartridge().unwrap().has_expansion_audio();
        assert_eq!(
            matches!(mapper, 5 | 69),
            has_audio,
            "mapper {mapper} expansion audio"
        );
    }
}

/// The chip has sixteen registers and a select above `$0F` names none of them.
#[test]
fn req_core_075_a_5b_register_select_above_fifteen_writes_nothing() {
    let mut core = core();
    write(&mut core, 0x00, 100);
    write(&mut core, 0x07, 0x3F);
    write(&mut core, 0x08, 0x0F);
    core.bus_mut().tick();
    let before = core.apu().mixer_expansion();

    // $18 would be register 8 if only the low bits were decoded.
    write(&mut core, 0x18, 0x00);
    core.bus_mut().tick();
    assert_eq!(before, core.apu().mixer_expansion());
}
