//! The VRC7's FM sound: the OPLL subset of mapper 85 (REQ-CORE-075, which leaves expansion
//! audio optional, and REQ-CORE-073 for the mix it enters).
//!
//! **No mapper-85 image exists anywhere in `assets-free/test-roms`** — step 4 of
//! `docs/progress/MF-5b.md` records the sweep of all 330 headers — and, unlike every other
//! expansion chip in this build, **there is no reference implementation in reach either**:
//! Mesen's `Vrc7Audio.h` delegates to `emu2413`, which is in `Core/Shared/Utilities` and
//! therefore outside the copied tree. So every assertion below is a sentence of the nesdev page
//! "VRC7 audio", of the YM2413 application manual, or of the YM2413 reverse-engineering notes,
//! turned into an assertion — and the four quantities none of those three gives are **Q-129**,
//! with the level itself **Q-128**. *Lagrange Point* is what checks the whole.

// Register values, periods and levels are small literals.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::unwrap_used
)]

mod common;

use common::mapper_fixture::MapperRom;
use zax_nes::apu::mixer;
use zax_nes::NesCore;

/// A machine with a synthetic mapper-85 `VRC7a`, powered on.
fn core_85() -> NesCore {
    MapperRom::new(85).prg(16).chr(16).submapper(2).load()
}

/// `M` as the board reports it this instant.
fn level(core: &NesCore) -> i32 {
    let m = core.cartridge().unwrap().expansion_audio_output();
    assert!(m.is_some(), "a VRC7 always answers with an M");
    m.unwrap_or(0)
}

/// Writes `value` into the chip's internal register `register`, through the two ports.
///
/// "After writing to this register, the program must not write to $9030 (or $9010 again) for at
/// least 6 CPU clock cycles" — the eight ticks between the two writes are that wait, so the
/// tests drive the chip the way a game has to.
fn sound(core: &mut NesCore, register: u8, value: u8) {
    core.write_memory(0x9010, register);
    for _ in 0..8 {
        core.bus_mut().tick();
    }

    core.write_memory(0x9030, value);
    for _ in 0..8 {
        core.bus_mut().tick();
    }
}

/// Runs `cycles` CPU cycles and returns the extreme values `M` took, `(min, max)`.
fn swing(core: &mut NesCore, cycles: u32) -> (i32, i32) {
    let mut low = i32::MAX;
    let mut high = i32::MIN;
    for _ in 0..cycles {
        core.bus_mut().tick();
        let m = level(core);
        low = low.min(m);
        high = high.max(m);
    }

    (low, high)
}

/// `count` consecutive samples of `M`, one every 36 CPU cycles, which is the chip's own rate.
///
/// Two instruments at the same note and the same volume reach the **same** peak — the carrier's
/// level is the channel volume, not a patch byte — so a pair of extremes cannot tell them apart.
/// A sequence of samples can: it carries the waveform the modulator shapes.
fn samples(core: &mut NesCore, count: usize) -> Vec<i32> {
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        for _ in 0..36 {
            core.bus_mut().tick();
        }

        out.push(level(core));
    }

    out
}

/// How far from silence a run of `count` samples is, in total: the sum of the distances from the
/// DC offset. The tremolo only ever attenuates, so it lowers this where a pair of extremes over
/// a whole LFO period is unchanged.
fn energy(core: &mut NesCore, count: usize) -> i64 {
    samples(core, count)
        .iter()
        .map(|m| i64::from((m - mixer::VRC7_CHANNEL_DC).abs()))
        .sum()
}

/// Sets channel `channel` playing instrument `instrument` at volume 0 (the loudest), with the
/// F-number and octave given, and keys it on.
fn play(core: &mut NesCore, channel: u8, instrument: u8, fnum: u16, block: u8) {
    sound(core, 0x30 + channel, instrument << 4);
    sound(core, 0x10 + channel, (fnum & 0xFF) as u8);
    sound(
        core,
        0x20 + channel,
        0x10 | (block << 1) | ((fnum >> 8) as u8 & 1),
    );
}

// ------------------------------------------------------------------------- the level

#[test]
fn q_128_a_vrc7_channel_at_full_output_swings_like_an_apu_pulse_at_volume_15() {
    // Q-128: the page gives no level at all, so the anchor is Q-118's rule applied the one way
    // an FM channel allows — **one channel at full output has the peak-to-peak swing of one APU
    // pulse at volume 15**. The constants that express it, recomputed here from `PULSE_TABLE` so
    // that a change to either is caught by name.
    assert_eq!(6, mixer::VRC7_CHANNELS);
    assert_eq!(
        mixer::PULSE_TABLE[15] / 2,
        mixer::VRC7_CHANNEL_DC,
        "a sounding channel carries half an APU pulse's swing"
    );
    assert_eq!(2_445, mixer::VRC7_CHANNEL_DC);
    assert_eq!(29_340, mixer::VRC7_MAX);

    // **Nothing sounding is an `M` of exactly 0**, which is what makes a savestate round trip
    // byte-identical: REQ-CORE-074 clears the audio pipeline on a restore, so a standing offset
    // would leave the high-pass filter's own state behind.
    assert_eq!(0, mixer::vrc7_level(0));

    // One sounding channel: its trough, its own offset, and its peak.
    assert_eq!(mixer::VRC7_CHANNEL_DC, mixer::vrc7_level(mixer::VRC7_UNIT));
    assert_eq!(
        2 * mixer::VRC7_CHANNEL_DC,
        mixer::vrc7_level(2 * mixer::VRC7_UNIT)
    );

    // Six of them at the top of their swing is the whole span.
    assert_eq!(
        mixer::VRC7_MAX,
        mixer::vrc7_level(2 * mixer::VRC7_CHANNELS * mixer::VRC7_UNIT)
    );

    // AMD-061 §6.2 and Q-118: `M` is positive, so `clip` never meets its upper bound and the
    // lower one has 3 428 units to spare at the worst `M` this chip can produce.
    assert!(
        mixer::clip(0, mixer::VRC7_MAX) > mixer::MIN_SCALE,
        "A = 0 against the loudest M is still inside the clip"
    );
}

#[test]
fn req_core_075_a_vrc7_is_silent_until_a_channel_is_keyed_on() {
    // Power-on leaves every channel released, so `M` is the DC offset and nothing else, however
    // many cycles run. A volume without a key-on is still silence.
    let mut core = core_85();
    assert_eq!(0, level(&core), "nothing sounding is an M of exactly 0");

    sound(&mut core, 0x30, 0x10); // Instrument 1, volume 0: the loudest there is.
    sound(&mut core, 0x10, 0x80);
    let (low, high) = swing(&mut core, 4_000);
    assert_eq!((0, 0), (low, high), "no trigger bit, no sound");
}

#[test]
fn req_core_075_a_keyed_vrc7_channel_makes_a_tone_that_swings_both_ways() {
    // "If trigger bit changes from 0 to 1 (key-on), a new note begins." The carrier is a sine,
    // so `M` must go above **and** below the DC offset, and the envelope's attack must get the
    // amplitude up within a few thousand cycles.
    let mut core = core_85();
    play(&mut core, 0, 1, 0x180, 5);
    let (low, high) = swing(&mut core, 40_000);
    assert!(
        low < mixer::VRC7_CHANNEL_DC && high > mixer::VRC7_CHANNEL_DC,
        "a sine swings both ways: {low}..{high} about {}",
        mixer::VRC7_CHANNEL_DC
    );
    assert!(
        high - low > 500,
        "and audibly so: {} units peak to peak",
        high - low
    );
}

#[test]
fn req_core_075_a_vrc7_volume_of_f_is_the_quietest_and_0_the_loudest() {
    // "$3X IIII VVVV", and "the volume value is inverted: $F is the lowest volume and $0 is the
    // highest", "logarithmic in 3db increments" — so volume $F is 45 dB down, which is a swing
    // smaller by a factor of about 180.
    let mut core = core_85();
    play(&mut core, 0, 1, 0x180, 5);
    let (low, high) = swing(&mut core, 40_000);
    let loudest = high - low;

    let mut quiet = core_85();
    play(&mut quiet, 0, 1, 0x180, 5);
    sound(&mut quiet, 0x30, 0x1F); // The same instrument at volume $F.
    let (low, high) = swing(&mut quiet, 40_000);
    let quietest = high - low;

    assert!(
        quietest * 8 < loudest,
        "volume $F must be far below volume $0: {quietest} against {loudest}"
    );
}

// --------------------------------------------------------------- the envelope generator

#[test]
fn req_core_075_a_vrc7_note_releases_to_silence_when_the_trigger_clears() {
    // "If it changes from 1 to 0 (key-off), it will begin the release portion of its envelope",
    // and "the carrier … always enters release when the note is released". Instrument 1's
    // carrier release is $7, fast enough to finish inside the budget below.
    let mut core = core_85();
    play(&mut core, 0, 1, 0x180, 5);
    let (low, high) = swing(&mut core, 40_000);
    assert!(high - low > 500, "the note sounds first");

    // Clear the trigger, keeping the octave and the frequency.
    sound(&mut core, 0x20, (5 << 1) | 1);
    // The release runs first — instrument 1's carrier release of $7 at this note is `RATE = 30`,
    // which the reverse-engineering notes' own law makes about 11 000 samples for the whole
    // 128 levels — and only then is the silence measured. Measuring across the release would
    // only be measuring the note still fading.
    for _ in 0..1_000_000 {
        core.bus_mut().tick();
    }

    let (low, high) = swing(&mut core, 80_000);
    assert_eq!(
        (0, 0),
        (low, high),
        "the release reaches exact silence, not a floor of 47.6 dB"
    );
}

#[test]
fn req_core_075_a_vrc7_attack_of_zero_never_sounds() {
    // The manual: "When R=0, RATE=0", and the page: "Attack is the speed of the attack fade in
    // after key-on. $0 = halt". A custom patch whose carrier attack is $0 therefore stays at the
    // bottom of its envelope for ever, however long the note is held.
    let mut core = core_85();
    // The custom patch: $05 is the carrier's `AAAA DDDD`, so $0F is attack 0 and decay $F.
    for (register, value) in [(0x00, 0x01), (0x01, 0x01), (0x02, 0x00), (0x03, 0x00)] {
        sound(&mut core, register, value);
    }

    sound(&mut core, 0x04, 0xFF);
    sound(&mut core, 0x05, 0x0F);
    sound(&mut core, 0x06, 0x00);
    sound(&mut core, 0x07, 0x00);
    play(&mut core, 0, 0, 0x180, 5);

    let (low, high) = swing(&mut core, 200_000);
    assert_eq!(
        (mixer::VRC7_CHANNEL_DC, mixer::VRC7_CHANNEL_DC),
        (low, high),
        "attack $0 is a halt, so the envelope never leaves the floor (the channel is keyed \
             on, so it carries its own offset; what it must not do is move)"
    );
}

#[test]
fn req_core_075_a_faster_vrc7_attack_reaches_its_peak_sooner() {
    // "$1 = slowest, $F = fastest", through `RATE = 4 x R + Rks`. Measured as the number of CPU
    // cycles before the swing first exceeds a quarter of what the note eventually reaches.
    let time_to_sound = |attack: u8| -> u32 {
        let mut core = core_85();
        for (register, value) in [
            (0x00, 0x01),
            (0x01, 0x01),
            (0x02, 0x00),
            (0x03, 0x00),
            (0x04, 0xF0),
            (0x06, 0x00),
            (0x07, 0x00),
        ] {
            sound(&mut core, register, value);
        }

        // $05 is the carrier's attack and decay: decay $0 holds the peak once reached.
        sound(&mut core, 0x05, attack << 4);
        play(&mut core, 0, 0, 0x180, 5);

        for cycle in 0..2_000_000u32 {
            core.bus_mut().tick();
            if (level(&core) - mixer::VRC7_CHANNEL_DC).abs() > 400 {
                return cycle;
            }
        }

        u32::MAX
    };

    let fast = time_to_sound(0x0F);
    let slow = time_to_sound(0x05);
    assert!(
        fast < slow,
        "attack $F must beat attack $5: {fast} < {slow}"
    );
    assert!(slow < u32::MAX, "and attack $5 must sound at all");
}

#[test]
fn req_core_075_the_vrc7_sustain_bit_of_2x_forces_the_release_rate_to_5() {
    // "Using the sustain bit (S) in $2X overrides the normal release value for the patch with a
    // value of $5." Instrument 1's carrier release is $7, so setting `S` makes the release
    // **slower**, and the note is still sounding where it had gone silent without it.
    let release_time = |sustain: bool| -> u32 {
        let mut core = core_85();
        play(&mut core, 0, 1, 0x180, 5);
        swing(&mut core, 60_000);
        // Key off, with or without the sustain bit, keeping the frequency and the octave.
        let control = (5 << 1) | 1 | if sustain { 0x20 } else { 0 };
        sound(&mut core, 0x20, control);

        for cycle in 0..4_000_000u32 {
            core.bus_mut().tick();
            if level(&core) == 0 {
                // Silence can be crossed on the way through zero, so a run of it is what counts.
                let (low, high) = swing(&mut core, 2_000);
                if (low, high) == (0, 0) {
                    return cycle;
                }
            }
        }

        u32::MAX
    };

    let plain = release_time(false);
    let sustained = release_time(true);
    assert!(plain < u32::MAX, "release $7 must finish");
    assert!(
        sustained > plain,
        "rate $5 is slower than instrument 1's own $7: {sustained} against {plain}"
    );
}

// ------------------------------------------------------------------ the instruments

#[test]
fn req_core_075_the_fifteen_vrc7_instruments_are_the_pages_own_bytes() {
    // The page tabulates fifteen patches of eight bytes; instrument 0 is the custom one. Pinned
    // through behaviour rather than by reading the table back: each of the fifteen must sound,
    // and no two of them may produce the same waveform from the same note, which is what would
    // happen if the table were mis-transcribed into a repeated row.
    let mut voices = Vec::new();
    for instrument in 1..=15u8 {
        let mut core = core_85();
        play(&mut core, 0, instrument, 0x180, 5);
        // Long enough for the slowest attack of the fifteen to be under way.
        swing(&mut core, 200_000);
        let voice = samples(&mut core, 256);
        assert!(
            voice.iter().any(|m| *m != mixer::VRC7_CHANNEL_DC),
            "instrument {instrument:X} produces nothing at all"
        );
        voices.push((instrument, voice));
    }

    for (instrument, voice) in &voices {
        let twins = voices
            .iter()
            .filter(|(other, v)| other != instrument && v == voice)
            .count();
        assert_eq!(
            0, twins,
            "instrument {instrument:X} produces the same 256 samples as another instrument: a \
             row of the patch table is duplicated"
        );
    }
}

#[test]
fn req_core_075_instrument_0_is_the_custom_patch_and_follows_00_to_07_live() {
    // "$00-$07: Custom patch definition", and a channel on instrument 0 hears a change to those
    // registers at once — which is how a VRC7 game sweeps a timbre while it sounds.
    let mut core = core_85();
    // A custom patch with the carrier's output at nothing: multiplier 1 both operators, no
    // modulation, carrier attack $F, decay $0, so the note holds its peak.
    for (register, value) in [
        (0x00u8, 0x01u8),
        (0x01, 0x01),
        (0x02, 0x3F), // The modulator all the way down, so only the carrier is heard.
        (0x03, 0x00),
        (0x04, 0xF0),
        (0x05, 0xF0),
        (0x06, 0x00),
        (0x07, 0x00),
    ] {
        sound(&mut core, register, value);
    }

    play(&mut core, 0, 0, 0x180, 5);
    let (low, high) = swing(&mut core, 120_000);
    let before = high - low;
    assert!(before > 500, "the custom patch sounds: {before}");

    // Now move the carrier's own waveform to the half-rectified sine while the note holds:
    // `$03`'s `Q` bit. The negative half goes, so the swing below the offset must shrink.
    sound(&mut core, 0x03, 0x10);
    let (low, _) = swing(&mut core, 120_000);
    assert!(
        low > mixer::VRC7_CHANNEL_DC - 100,
        "a half-rectified carrier cannot go below the offset: {low}"
    );
}

// ------------------------------------------------------------------------ the LFOs

#[test]
fn req_core_075_the_vrc7_tremolo_runs_at_3_7_hz_and_the_vibrato_at_6_07() {
    // Both rates are fixed by one sentence of the page's test register — "(Tremolo is 64x
    // faster, and vibrato is 1024x faster)" when the LFOs are stepped every sample — together
    // with the manual's "3.7 Hz" and "6.4 Hz". 49716 / (64 x 210) is 3.699 Hz, which is what
    // fixes the tremolo's length at 210 steps; 49716 / (1024 x 8) is 6.07 Hz against the
    // manual's 6.36 at the VRC7's own 3.58 MHz (Q-129 for the depths, which no source gives).
    //
    // What is checked here is the arithmetic the code is built on, so that a change to any of
    // the three constants has to come and change this test too.
    let sample_rate: f64 = 1_789_773.0 / 36.0;
    assert!(
        (sample_rate / (64.0 * 210.0) - 3.7).abs() < 0.01,
        "the tremolo is 3.7 Hz"
    );
    assert!(
        (sample_rate / (1024.0 * 8.0) - 6.07).abs() < 0.01,
        "the vibrato is 6.07 Hz"
    );
}

#[test]
fn req_core_075_the_vrc7_tremolo_modulates_the_amplitude_and_the_vibrato_the_pitch() {
    // `$0X` bit 7 is `T`, "Tremolo applies amplitude modulation at a predefined rate", and bit 6
    // is `V`, "Vibrato applies pitch modulation". Both are checked against the same patch with
    // the bits clear: the tremolo must change the envelope's ceiling over a tremolo period, and
    // the vibrato must change the waveform the note produces.
    let run = |flags: u8| -> (i64, Vec<i32>) {
        let mut core = core_85();
        for (register, value) in [
            (0x00u8, flags | 0x01),
            (0x01, flags | 0x01),
            (0x02, 0x3F),
            (0x03, 0x00),
            (0x04, 0xF0),
            (0x05, 0xF0),
            (0x06, 0x00),
            (0x07, 0x00),
        ] {
            sound(&mut core, register, value);
        }

        play(&mut core, 0, 0, 0x180, 5);
        // Past the attack, then a whole tremolo period: 3.7 Hz is about 13 400 samples.
        swing(&mut core, 40_000);
        // **4 096 samples and not 256**: the vibrato LFO steps once every 1 024 samples, and its
        // first step bends a nine-bit F-number of 384 by `384 >> 9` = nothing at all. A window
        // shorter than a few steps compares two identical stretches and proves nothing, which is
        // what the first draft of this test did.
        let voice = samples(&mut core, 4_096);
        (energy(&mut core, 14_000), voice)
    };

    let (plain_energy, plain_voice) = run(0x00);
    let (tremolo_energy, _) = run(0x80);
    let (_, vibrato_voice) = run(0x40);

    // "Tremolo applies amplitude modulation": it only ever attenuates, so a whole LFO period of
    // it carries less than the same period without.
    assert!(
        tremolo_energy < plain_energy,
        "the tremolo must attenuate over its period: {tremolo_energy} against {plain_energy}"
    );

    // "Vibrato applies pitch modulation": the same note comes out as a different waveform.
    assert_ne!(
        plain_voice, vibrato_voice,
        "the vibrato must bend the pitch"
    );
}

// ------------------------------------------------------- the six channels, and $E000

#[test]
fn req_core_075_the_vrc7_sums_its_six_channels_into_one_dac() {
    // "The VRC7 has one DAC and output pin for audio, multiplexed for all 6 channels", so six
    // channels sounding together must swing further than one, and the seventh register of each
    // group ($16, $26, $36) is not a channel at all: "other register values are ignored".
    let mut one = core_85();
    play(&mut one, 0, 1, 0x180, 5);
    let (low, high) = swing(&mut one, 80_000);
    let single = high - low;

    let mut all = core_85();
    for channel in 0..6u8 {
        // Six different notes, so they cannot cancel by being in phase and opposite.
        play(&mut all, channel, 1, 0x100 + u16::from(channel) * 0x20, 5);
    }

    let (low, high) = swing(&mut all, 80_000);
    let six = high - low;
    assert!(
        six > single * 2,
        "six channels must be far louder than one: {six} against {single}"
    );

    // A seventh channel does not exist.
    let mut seventh = core_85();
    play(&mut seventh, 6, 1, 0x180, 5);
    let (low, high) = swing(&mut seventh, 80_000);
    assert_eq!(
        (0, 0),
        (low, high),
        "$16/$26/$36 are ignored, not a seventh channel"
    );
}

#[test]
fn req_core_075_e000_bit_6_silences_the_vrc7_and_disregards_its_writes() {
    // `$E000` = `RS.. ..MM`: "Silence expansion sound if set", and on the sound page "writes to
    // $9010 and $9030 are disregarded while this bit is set".
    let mut core = core_85();
    play(&mut core, 0, 1, 0x180, 5);
    let (low, high) = swing(&mut core, 80_000);
    assert!(high - low > 500, "the note sounds first");

    core.write_memory(0xE000, 0x40);
    let (low, high) = swing(&mut core, 80_000);
    assert_eq!((0, 0), (low, high), "silenced is an M of 0, offset and all");

    // A write while silenced reaches nothing: keying channel 1 on here must leave it unkeyed.
    play(&mut core, 1, 1, 0x200, 5);
    core.write_memory(0xE000, 0x00);
    let (low, high) = swing(&mut core, 80_000);
    assert!(
        high - low > 500,
        "channel 0 resumes where it was: {low}..{high}"
    );

    // And channel 1 really was not keyed: keying it now must change the sound.
    let before = (low, high);
    play(&mut core, 1, 1, 0x200, 5);
    assert_ne!(before, swing(&mut core, 80_000));
}

#[test]
fn req_core_075_a_vrc7_subtracts_its_m_from_the_apu_mix() {
    // AMD-061 §6.2: `S = A - M` with `M` positive, through the mixer's own test point.
    assert_eq!(
        mixer::PULSE_TABLE[15] - mixer::VRC7_MAX,
        mixer::clip(mixer::PULSE_TABLE[15], mixer::VRC7_MAX)
    );
    assert!(mixer::clip(0, mixer::VRC7_MAX) < 0);
}
