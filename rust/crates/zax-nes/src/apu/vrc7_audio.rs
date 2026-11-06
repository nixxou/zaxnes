//! The VRC7's FM sound: the OPLL subset Konami put in the chip (REQ-CORE-075, optional).
//!
//! **There is no reference implementation in reach, and that shapes the whole file.**
//! `third_party/mesen2`'s `Audio/Vrc7Audio.h` is a 91-line shell around `emu2413`, and
//! `Core/Shared/Utilities/` — where `emu2413` lives — is not in the copied tree
//! (`third_party/README.md`: only `Core/NES` and `Core/Shared/Audio` were taken). So every
//! number below comes from one of three public documents, and the four that none of them gives
//! are **Q-129**:
//!
//! * nesdev, "VRC7 audio": the register map, every operator field, the fifteen built-in patches
//!   as hex, the multiplier table, the feedback table in radians, the two waveforms, the
//!   frequency law, 49716 Hz, and the ratios the test register implies between the LFO rates;
//! * the YM2413 application manual: `RATE = 4 × R + Rks`, the TL and SL step sizes, the KSL law
//!   in dB per octave, the two LFO frequencies, and "rises exponentially during attack time and
//!   varies linearly everywhere else";
//! * the YM2413 reverse-engineering notes of 2015-03-20: the envelope's **0.375 dB over 128
//!   levels**, the shared per-sample envelope counter, and the `eg_shift`/`eg_select` law with
//!   the duration formula that pins the four advance patterns (see [`EG_PATTERNS`]).
//!
//! ## The unit everything is measured in
//!
//! One **1/256 of an octave of amplitude**, about 0.0235 dB. It is not an arbitrary choice: it
//! is the coarsest unit in which every quantity the documents give is a whole number —
//!
//! | documented quantity | units |
//! | --- | ---: |
//! | one envelope step, 0.375 dB | 16 |
//! | one total-level step, 0.75 dB | 32 |
//! | one sustain-level step, 3 dB | 128 |
//! | one carrier volume step, 3 dB | 128 |
//! | KSL at 1.5 / 3 / 6 dB per octave | 64 / 128 / 256 per octave |
//!
//! — and in which the exponential becomes [`EXP_TABLE`], 256 entries plus a shift, with no
//! division anywhere on the per-sample path. The 0.375 dB step is 0.37629 dB at 16 units, 0.3 %
//! high; over the envelope's whole 128 steps that is 48.2 dB where the notes measured 48.0.

use super::mixer;
use crate::state::{InvalidState, StateReader, StateWriter};

// ----------------------------------------------------------------------- the tables

/// Attenuation units to an octave of amplitude. See the module note.
const OCTAVE: i32 = 256;

/// One step of the envelope generator: "we measured 48dB/128 = 0.375dB" (the reverse-engineering
/// notes, against the datasheet's 0.325).
const EG_STEP: i32 = 16;

/// Envelope levels: "0.375 dB per step across 128 total levels (0 to -48 dB range)".
const EG_LEVELS: i32 = 128;

/// Attenuation at which the output is 0 whatever the fraction, so that the clamp below keeps the
/// shift of [`EXP_TABLE`] inside `i32` and a released channel is **exactly** silent rather than
/// one unit of DC. 15 octaves is 90.3 dB.
const MAX_ATTENUATION: i32 = 15 * OCTAVE;

/// Amplitude 1.0, in the fixed-point units the operators produce.
///
/// `1 << 14` and not `1 << 16`: six channels at full output then sum to 98 304, which stays
/// inside `i32` after the multiplication by [`mixer::PULSE_TABLE`]`[15]` that takes the sum to
/// the mixer's own units.
const UNIT: i32 = 1 << 14;

/// `2^-k/256` for `k` in `0..256`, in [`UNIT`]s: the exponential that turns an attenuation into
/// an amplitude. `amplitude = EXP_TABLE[a & 255] >> (a >> 8)`.
const EXP_TABLE: [i32; 256] = exp_table();

/// `-log2(sin(x)) × 256` over the first quarter of the sine, 256 entries, in attenuation units.
///
/// Sampled at the **middle** of each step, `(i + 0.5) × π / 512`, so that no entry sits on
/// `sin(0) = 0`: the largest is 2 138 units (50.2 dB), which an `i32` attenuation carries with
/// room to spare.
const LOG_SIN_TABLE: [i32; 256] = log_sin_table();

/// Steps of the phase accumulator to one cycle of the waveform, as a power of two.
///
/// From the page's own frequency law, "49716 Hz * freq / 2^(19 - octave)": a channel advances
/// `freq × 2^octave` steps of `1/2^19` cycle per sample, so the accumulator is 19 bits wide and
/// the increment is an integer with no rounding at all.
const PHASE_BITS: u32 = 19;

/// Steps of the sine table to one cycle: the top ten bits of the phase index it.
const SINE_BITS: u32 = 10;

/// Twice the frequency multiplier of each `MMMM` value: "$0-$7 produce 1/2, 1, 2, 3, 4, 5, 6, 7;
/// $8-$B produce 8, 9, 10, 10; $C-$F produce 12, 12, 15, 15" (nesdev, "VRC7 audio").
///
/// Doubled so that the half of value 0 is an integer, which is why the phase increment below is
/// shifted right by one.
const MULTIPLIER_X2: [u32; 16] = [1, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 20, 24, 24, 30, 30];

/// The four envelope advance patterns of `rate & 3`, one bit per tick of the shifted counter.
///
/// **Derived, not copied.** The reverse-engineering notes give the law — `eg_shift = 13 - rate/4`,
/// `eg_select = rate & 3`, "four lookup tables govern advancement frequency (4/8, 5/8, 6/8, or
/// 7/8 transitions per `eg_shift` period)" — and separately a duration formula that reproduces
/// the datasheet's own table:
///
/// ```text
/// cycles = (1 << (14 - rate/4)) * s[rate & 3],  s = {127, 102, 85, 73}
/// ```
///
/// A full 127-level decay at density `d`, with one tick every `2^eg_shift` samples, takes
/// `127 × 2^eg_shift / d` samples, i.e. `(1 << (14 - rate/4)) × 127 × (0.5 / d)`. So
/// `s[n] = 127 × 0.5 / d[n]`, and the four densities the notes name give **102, 85 and 73 to the
/// unit** (127 × 0.8, 127 × 2/3, 127 × 4/7). The count in each row is therefore fixed by the
/// documents; the *positions* are fixed by the notes' other measurement, "Rate 14: pattern of
/// 2048, 1024, 1024 samples" — rate 14 is `eg_shift = 10`, `eg_select = 2`, and the row below is
/// the only 6-in-8 arrangement whose gaps are 1, 1, 2 repeating.
const EG_PATTERNS: [[bool; 8]; 4] = [
    [false, true, false, true, false, true, false, true],
    [false, true, false, true, true, true, false, true],
    [false, true, true, true, false, true, true, true],
    [false, true, true, true, true, true, true, true],
];

/// Tremolo: one step of the 210-step triangle every 64 samples.
///
/// **Both numbers come from the page and the manual together.** The test register's bit 3
/// "update[s] tremolo and vibrato LFOs every sample instead of once every several samples
/// (Tremolo is 64x faster…)", so the divider is 64; the manual gives the rate as "3.7 Hz", and
/// `49716 / (64 × 210)` is **3.699 Hz**, which is what fixes the length at 210.
const TREMOLO_DIVIDER: i32 = 64;

/// Steps of the tremolo triangle. See [`TREMOLO_DIVIDER`].
const TREMOLO_STEPS: i32 = 210;

/// Vibrato: one step of the 8-step triangle every 1024 samples, from the same sentence
/// ("…vibrato is 1024x faster"). `49716 / (1024 × 8)` is **6.07 Hz** against the manual's
/// "6.4 Hz (@ øM = 3.6 MHz)", which is 6.36 Hz at the VRC7's own 3.58 MHz.
const VIBRATO_DIVIDER: i32 = 1024;

/// The vibrato LFO's eight steps, as a signed triangle. The phase increment is multiplied by
/// `1 + step/512`, so the peak is `2^(1/256)` in frequency: **6.77 cents**, where the OPL family
/// is documented at about 7 (Q-129 — the manual gives the rate and not the depth).
// Q-129
const VIBRATO_TABLE: [i32; 8] = [0, 1, 2, 1, 0, -1, -2, -1];

/// Release rate `$2X`'s sustain bit forces: "Using the sustain bit (S) in $2X overrides the
/// normal release value for the patch with a value of $5."
const SUSTAIN_RELEASE_RATE: u8 = 5;

/// The fifteen built-in patches, eight bytes each, exactly as the page tabulates them
/// (instrument 0 is the custom patch of `$00`-`$07` and is not here).
const INSTRUMENTS: [[u8; 8]; 15] = [
    [0x03, 0x21, 0x05, 0x06, 0xE8, 0x81, 0x42, 0x27], // 1 Buzzy Bell
    [0x13, 0x41, 0x14, 0x0D, 0xD8, 0xF6, 0x23, 0x12], // 2 Guitar
    [0x11, 0x11, 0x08, 0x08, 0xFA, 0xB2, 0x20, 0x12], // 3 Wurly
    [0x31, 0x61, 0x0C, 0x07, 0xA8, 0x64, 0x61, 0x27], // 4 Flute
    [0x32, 0x21, 0x1E, 0x06, 0xE1, 0x76, 0x01, 0x28], // 5 Clarinet
    [0x02, 0x01, 0x06, 0x00, 0xA3, 0xE2, 0xF4, 0xF4], // 6 Synth
    [0x21, 0x61, 0x1D, 0x07, 0x82, 0x81, 0x11, 0x07], // 7 Trumpet
    [0x23, 0x21, 0x22, 0x17, 0xA2, 0x72, 0x01, 0x17], // 8 Organ
    [0x35, 0x11, 0x25, 0x00, 0x40, 0x73, 0x72, 0x01], // 9 Bells
    [0xB5, 0x01, 0x0F, 0x0F, 0xA8, 0xA5, 0x51, 0x02], // A Vibes
    [0x17, 0xC1, 0x24, 0x07, 0xF8, 0xF8, 0x22, 0x12], // B Vibraphone
    [0x71, 0x23, 0x11, 0x06, 0x65, 0x74, 0x18, 0x16], // C Tutti
    [0x01, 0x02, 0xD3, 0x05, 0xC9, 0x95, 0x03, 0x02], // D Fretless
    [0x61, 0x63, 0x0C, 0x00, 0x94, 0xC0, 0x33, 0xF6], // E Synth Bass
    [0x21, 0x72, 0x0D, 0x00, 0xC1, 0xD5, 0x56, 0x06], // F Sweep
];

// ------------------------------------------------- the two tables, built at compile time

/// `e^x` for `x` in `[-1, 0]`, by its series. Sixteen terms take the remainder below `1e-17`
/// there, which is far inside the rounding of a table entry.
const fn exp_negative(x: f64) -> f64 {
    let mut term = 1.0;
    let mut sum = 1.0;
    let mut n: u32 = 1;
    while n <= 16 {
        term = term * x / n as f64;
        sum += term;
        n += 1;
    }

    sum
}

/// `sin(x)` for `x` in `[0, π/2]`, by its series.
const fn sine(x: f64) -> f64 {
    let mut term = x;
    let mut sum = x;
    let mut n: u32 = 1;
    while n <= 10 {
        term = -term * x * x / ((2 * n) as f64 * (2 * n + 1) as f64);
        sum += term;
        n += 1;
    }

    sum
}

/// `ln y` for `y` in `(0, 1]`, by `ln y = -2 atanh((1 - y) / (1 + y))` after taking out the
/// powers of two, so the series argument stays below 1/3 and converges fast.
const fn logarithm(y: f64) -> f64 {
    let mut mantissa = y;
    let mut octaves = 0.0;
    while mantissa < 0.5 {
        mantissa *= 2.0;
        octaves += 1.0;
    }

    let z = (mantissa - 1.0) / (mantissa + 1.0);
    let mut power = z;
    let mut sum = z;
    let mut n: u32 = 1;
    while n <= 40 {
        power = power * z * z;
        sum += power / (2 * n + 1) as f64;
        n += 1;
    }

    2.0 * sum - octaves * core::f64::consts::LN_2
}

/// Rounds to the nearest integer, halves away from zero. The two tables are built once at
/// compile time, so this is never on any path.
#[allow(clippy::cast_possible_truncation)] // Both tables are far inside i32.
const fn round(value: f64) -> i32 {
    if value < 0.0 {
        -((-value + 0.5) as i32)
    } else {
        (value + 0.5) as i32
    }
}

/// Builds [`EXP_TABLE`].
const fn exp_table() -> [i32; 256] {
    let mut table = [0; 256];
    let mut k: u32 = 0;
    while k < 256 {
        let exponent = -(k as f64) * core::f64::consts::LN_2 / OCTAVE as f64;
        table[k as usize] = round(UNIT as f64 * exp_negative(exponent));
        k += 1;
    }

    table
}

/// Builds [`LOG_SIN_TABLE`].
const fn log_sin_table() -> [i32; 256] {
    let mut table = [0; 256];
    let mut i: u32 = 0;
    while i < 256 {
        // (i + 0.5) / 256 of a quarter cycle, in radians: pi/2 x that.
        let angle = (i as f64 + 0.5) * core::f64::consts::FRAC_PI_2 / 256.0;
        let attenuation = -(OCTAVE as f64) * logarithm(sine(angle)) / core::f64::consts::LN_2;
        table[i as usize] = round(attenuation);
        i += 1;
    }

    table
}

// --------------------------------------------------------------------- the operators

/// Where an operator's envelope is. `Idle` is the fully-released state, kept apart so that a
/// quiet channel costs the sample loop nothing but a test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EgPhase {
    Attack,
    Decay,
    Sustain,
    Release,
    Idle,
}

impl EgPhase {
    const fn to_bits(self) -> u8 {
        match self {
            Self::Attack => 0,
            Self::Decay => 1,
            Self::Sustain => 2,
            Self::Release => 3,
            Self::Idle => 4,
        }
    }

    const fn from_bits(bits: u8) -> Option<Self> {
        match bits {
            0 => Some(Self::Attack),
            1 => Some(Self::Decay),
            2 => Some(Self::Sustain),
            3 => Some(Self::Release),
            4 => Some(Self::Idle),
            _ => None,
        }
    }
}

/// One operator's half of a patch, decoded once when the patch or the instrument changes so that
/// the per-sample path reads fields and never shifts a register apart.
///
/// The fields are the page's own, in its order: `$0X` is `TVSK MMMM`, `$02`/`$03` carry the key
/// level scaling and the output level, `$04`/`$05` are `AAAA DDDD` and `$06`/`$07` `SSSS RRRR`.
// Six independent bits of one patch, each named by the page; a bitfield would hide them.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OperatorPatch {
    /// `T`: "Tremolo applies amplitude modulation at a predefined rate."
    tremolo: bool,
    /// `V`: "Vibrato applies pitch modulation at a predefined rate."
    vibrato: bool,
    /// `S`, the envelope type: set means the Sustain portion of the envelope holds (see
    /// [`Operator::advance_envelope`]).
    sustained: bool,
    /// `K`: "Key rate scaling adjusts the ADSR envelope speed, faster for high frequencies,
    /// slower for low ones."
    key_rate_scaling: bool,
    /// Twice the frequency multiplier, from [`MULTIPLIER_X2`].
    multiplier_x2: u32,
    /// `KK`, 0 to 3: "0 = none, $3 = most", at 0, 1.5, 3 and 6 dB per octave.
    key_level_scaling: u8,
    /// The modulator's `OOOOOO` in attenuation units: "this value reduces the modulator volume
    /// in 0.75db increments". The carrier has none — `$3X`'s volume takes its place.
    total_level: i32,
    /// `W` or `Q`: "0 = sine, 1 = half-wave rectified sine (where sine values less than 0 are
    /// clipped to 0)".
    half_wave: bool,
    attack: u8,
    decay: u8,
    /// `SSSS`: "the attenuation after decay, in 3db increments. $0 = highest volume, $F = lowest."
    sustain_level: u8,
    release: u8,
}

/// One of the two operators of a channel: a phase accumulator and an envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Operator {
    /// [`PHASE_BITS`] wide, so one cycle is `1 << PHASE_BITS`.
    phase: u32,
    /// 0 (loudest) to [`EG_LEVELS`] − 1 (silent), in [`EG_STEP`]s.
    eg_level: i32,
    eg_phase: EgPhase,
}

impl Operator {
    const fn new() -> Self {
        Self {
            phase: 0,
            eg_level: EG_LEVELS - 1,
            eg_phase: EgPhase::Idle,
        }
    }

    /// "If trigger bit changes from 0 to 1 (key-on), a new note begins."
    const fn key_on(&mut self) {
        self.phase = 0;
        self.eg_phase = EgPhase::Attack;
    }

    /// "If it changes from 1 to 0 (key-off), it will begin the release portion."
    const fn key_off(&mut self) {
        if !matches!(self.eg_phase, EgPhase::Idle) {
            self.eg_phase = EgPhase::Release;
        }
    }

    /// `RATE = 4 × R + Rks` (the YM2413 manual), with `R` the four-bit register field of the
    /// envelope's current portion and `Rks` the key-scale offset.
    ///
    /// `Rks` is "based on octave and F-number MSB" and ranges 0 to 15, so it is
    /// `(block << 1) | (fnum >> 8)`; with `K` clear the manual's own table is that value
    /// quartered, which is the 0-to-3 range. "When R=0, RATE=0" — the envelope then holds.
    ///
    /// The sum is clamped at 63 because `4 × 15 + 15` overflows the manual's table, which stops
    /// there.
    const fn rate(&self, patch: &OperatorPatch, block: u8, fnum: u16, sustain: bool) -> u8 {
        let register = match self.eg_phase {
            EgPhase::Attack => patch.attack,
            EgPhase::Decay => patch.decay,
            // `$2X`'s sustain bit "overrides the normal release value for the patch with a value
            // of $5", and the Sustain portion of a percussive tone falls at the release rate.
            EgPhase::Sustain | EgPhase::Release => {
                if sustain {
                    SUSTAIN_RELEASE_RATE
                } else {
                    patch.release
                }
            }
            EgPhase::Idle => 0,
        };

        if register == 0 {
            return 0;
        }

        let scale = (block << 1) | ((fnum >> 8) as u8);
        let offset = if patch.key_rate_scaling {
            scale
        } else {
            scale >> 2
        };
        let rate = 4 * register + offset;
        if rate > 63 {
            63
        } else {
            rate
        }
    }

    /// How many envelope levels this sample's `rate` moves, given the shared `counter`.
    ///
    /// The reverse-engineering notes: rates 0-3 "never advance"; rates 60-63 "advance 2 levels
    /// per sample"; in between, "the counter is right-shifted by `eg_shift` bits; transitions
    /// occur only when shifted-out bits are zero", with `eg_shift = 13 - rate / 4` and the row
    /// of [`EG_PATTERNS`] chosen by `rate & 3`.
    ///
    /// `eg_shift` goes negative for rates 56-59, which the notes say are part of the irregular
    /// 52-to-59 band whose sixteen-entry tables they do not print. The law above is extrapolated
    /// there by doubling the advance per negative shift, which keeps it monotone up to the two
    /// levels per sample of rate 60 (**Q-129**).
    const fn envelope_advance(rate: u8, counter: u32) -> i32 {
        if rate < 4 {
            return 0;
        }

        if rate >= 60 {
            return 2;
        }

        let shift = 13 - (rate >> 2) as i32;
        let (shift, scale) = if shift < 0 {
            (0, 1 << -shift)
        } else {
            (shift, 1)
        };

        if counter & ((1 << shift) - 1) != 0 {
            return 0;
        }

        let tick = ((counter >> shift) & 7) as usize;
        if EG_PATTERNS[(rate & 3) as usize][tick] {
            scale
        } else {
            0
        }
    }

    /// One sample of the envelope.
    ///
    /// The manual: the envelope "rises exponentially during attack time and varies linearly
    /// everywhere else". Decay, Sustain and Release are therefore `+1` level per advance — a
    /// straight line in dB — and the Attack is the exponential approach
    /// `level -= (level >> 3) + 1`, which reaches 0 from the floor in 27 advances. **Q-129**: no
    /// allowed source gives the attack's per-step law, and the ratio this one produces between a
    /// 10-90 % attack and a 0-40 dB decay at the same `RATE` is 6.3 where the manual's Table
    /// III-7 implies about 12.
    ///
    /// The two transitions out of Decay are the page's own: "$06/$07 SSSS Sustain is the
    /// attenuation after decay", and the operator's `S` bit decides what happens at the bottom of
    /// it — set, "the Attack, Decay, and Sustain portions of the envelope are used" and the level
    /// holds; clear, it keeps falling at the release rate, which is the percussive tone.
    // Q-129
    fn advance_envelope(
        &mut self,
        patch: &OperatorPatch,
        block: u8,
        fnum: u16,
        sustain: bool,
        counter: u32,
    ) {
        let advance = Self::envelope_advance(self.rate(patch, block, fnum, sustain), counter);
        let mut remaining = advance;
        while remaining > 0 {
            remaining -= 1;
            match self.eg_phase {
                EgPhase::Attack => {
                    self.eg_level -= (self.eg_level >> 3) + 1;
                    if self.eg_level <= 0 {
                        self.eg_level = 0;
                        self.eg_phase = EgPhase::Decay;
                    }
                }
                EgPhase::Decay => {
                    self.eg_level += 1;
                    // The sustain level is four bits of 3 dB, which is eight envelope steps.
                    if self.eg_level >= i32::from(patch.sustain_level) * 8 {
                        self.eg_phase = EgPhase::Sustain;
                    }
                }
                EgPhase::Sustain => {
                    if patch.sustained {
                        break;
                    }

                    self.eg_level += 1;
                    if self.eg_level >= EG_LEVELS - 1 {
                        self.eg_level = EG_LEVELS - 1;
                        self.eg_phase = EgPhase::Idle;
                    }
                }
                EgPhase::Release => {
                    self.eg_level += 1;
                    if self.eg_level >= EG_LEVELS - 1 {
                        self.eg_level = EG_LEVELS - 1;
                        self.eg_phase = EgPhase::Idle;
                    }
                }
                EgPhase::Idle => break,
            }
        }
    }

    /// The operator's total attenuation this sample: its envelope, its own level, the key level
    /// scaling and the tremolo.
    fn attenuation(&self, patch: &OperatorPatch, block: u8, fnum: u16, tremolo: i32) -> i32 {
        // **The bottom level is silence, not 47.6 dB.** The envelope has 128 levels of 0.375 dB,
        // which runs out at 48 dB and not at nothing, so taking the last level literally would
        // leave a released note — and a note whose attack rate is $0, which the page calls a
        // "halt" — sounding faintly for ever. It is the converter that runs out of bits there,
        // and `req_core_075_a_vrc7_attack_of_zero_never_sounds` is what caught it.
        if self.eg_level >= EG_LEVELS - 1 {
            return MAX_ATTENUATION;
        }

        let mut total = self.eg_level * EG_STEP + patch.total_level + key_level(patch, block, fnum);
        if patch.tremolo {
            total += tremolo;
        }

        if total > MAX_ATTENUATION {
            MAX_ATTENUATION
        } else {
            total
        }
    }

    /// Advances the phase by one sample and returns the operator's signed output, `modulation`
    /// phase units into the waveform.
    fn run(
        &mut self,
        patch: &OperatorPatch,
        block: u8,
        fnum: u16,
        vibrato: i32,
        modulation: i32,
        attenuation: i32,
    ) -> i32 {
        // The page's law: "49716 Hz * freq / 2^(19 - octave)", times the operator's multiplier
        // and bent by the vibrato. `MULTIPLIER_X2` is doubled, hence the extra shift.
        let signed = i32::from(fnum);
        let bent = signed + ((signed * vibrato) >> 9);
        let increment = ((u32::try_from(bent).unwrap_or(0) * patch.multiplier_x2) << block) >> 1;
        self.phase = (self.phase + increment) & ((1 << PHASE_BITS) - 1);

        let index = (self.phase >> (PHASE_BITS - SINE_BITS)).wrapping_add_signed(modulation)
            & ((1 << SINE_BITS) - 1);
        waveform(index, patch.half_wave, attenuation)
    }
}

/// `-log2(sin)` of the waveform at `index`, exponentiated and signed: the operator's output.
///
/// The sine table holds a quarter cycle, so the second quarter reads it backwards and the lower
/// half is the upper half negated. "1 = half-wave rectified sine (where sine values less than 0
/// are clipped to 0)" is the lower half replaced by silence.
fn waveform(index: u32, half_wave: bool, attenuation: i32) -> i32 {
    let negative = index & (1 << (SINE_BITS - 1)) != 0;
    if negative && half_wave {
        return 0;
    }

    let quarter = (index >> (SINE_BITS - 2)) & 1;
    let step = (index & 0xFF) as usize;
    let log_sin = LOG_SIN_TABLE[if quarter == 0 { step } else { 255 - step }];

    // Both halves are in range by construction: `total` is below fifteen octaves, so the index
    // is a byte and the shift is at most fourteen.
    let total = log_sin + attenuation;
    let magnitude = if total >= MAX_ATTENUATION {
        0
    } else {
        EXP_TABLE[usize::try_from(total & 0xFF).unwrap_or(0)] >> (total >> 8)
    };

    if negative {
        -magnitude
    } else {
        magnitude
    }
}

/// Key level scaling, in attenuation units.
///
/// The manual gives the law and not the ROM: the attenuation "increases with pitch" at "0 dB/oct,
/// 1.5 dB/oct, 3 dB/oct, and 6 dB/oct", and its Table III-5 runs "from 0 dB (low F-numbers) to
/// approximately 21 dB (octave 7, highest F-numbers)" at the 3 dB setting. Neither the table nor
/// the ROM behind it is in an allowed source, so this is the **continuous** law through those two
/// anchors: `3 × log2(f × 2^block / 15)` dB, clamped at 0, which is 0 across the whole of octave
/// 0 and exactly 21 dB at the top of octave 7. **Q-129.**
///
/// Computed from `block` and the top four bits of the F-number, which is what the hardware's own
/// table is indexed by, through [`KEY_LEVEL_TABLE`].
// Q-129
fn key_level(patch: &OperatorPatch, block: u8, fnum: u16) -> i32 {
    if patch.key_level_scaling == 0 {
        return 0;
    }

    let row = (usize::from(block) & 7) * 16 + (usize::from(fnum >> 5) & 15);
    // 1.5, 3 and 6 dB per octave: the table holds the 1.5 dB row, and the other two double it.
    KEY_LEVEL_TABLE[row] << (patch.key_level_scaling - 1)
}

/// The 1.5 dB-per-octave key level scaling, by `block` then by the top four bits of the F-number.
const KEY_LEVEL_TABLE: [i32; 128] = key_level_table();

/// Builds [`KEY_LEVEL_TABLE`] from the law [`key_level`] documents: half of
/// `3 × log2(f × 2^block / 15)` octaves of attenuation, in units, clamped at 0.
const fn key_level_table() -> [i32; 128] {
    let mut table = [0; 128];
    let mut block: u32 = 0;
    while block < 8 {
        let mut f: u32 = 0;
        while f < 16 {
            // 3 dB per octave of frequency is half an octave of amplitude per octave of
            // frequency, so the 1.5 dB row is a quarter of it.
            if f > 0 {
                let octaves = block as f64 + logarithm(f as f64 / 15.0) / core::f64::consts::LN_2;
                if octaves > 0.0 {
                    table[(block * 16 + f) as usize] = round(octaves * OCTAVE as f64 / 4.0);
                }
            }

            f += 1;
        }

        block += 1;
    }

    table
}

// ---------------------------------------------------------------------- the channels

/// One of the VRC7's six FM channels: two operators, a patch, and the frequency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Vrc7Channel {
    /// `$1X` plus bit 0 of `$2X`: the nine-bit F-number.
    fnum: u16,
    /// `$2X` bits 1-3, the octave.
    block: u8,
    /// `$2X` bit 4, latched so that key-on and key-off are edges.
    triggered: bool,
    /// `$2X` bit 5.
    sustain: bool,
    /// `$3X` high nibble: 0 is the custom patch of `$00`-`$07`.
    instrument: u8,
    /// `$3X` low nibble, in attenuation units. "The volume value is inverted: $F is the lowest
    /// volume and $0 is the highest", "logarithmic in 3db increments", so it is the carrier's
    /// total level and four bits of 3 dB.
    volume: i32,
    modulator_patch: OperatorPatch,
    carrier_patch: OperatorPatch,
    /// `FFF`: "$0 through $7 = 0, pi/16, pi/8, pi/4, pi/2, pi, 2pi, 4pi".
    feedback: u8,
    modulator: Operator,
    carrier: Operator,
    /// The modulator's last two outputs, which the feedback path averages. **Q-129**: the two
    /// documents give the modulation index and say nothing about how many samples the feedback
    /// path carries; one sample alone makes high feedback settings ring.
    history: [i32; 2],
}

impl Vrc7Channel {
    const fn new() -> Self {
        Self {
            fnum: 0,
            block: 0,
            triggered: false,
            sustain: false,
            instrument: 0,
            volume: 0,
            modulator_patch: OperatorPatch::quiet(),
            carrier_patch: OperatorPatch::quiet(),
            feedback: 0,
            modulator: Operator::new(),
            carrier: Operator::new(),
            history: [0; 2],
        }
    }

    /// Decodes the eight patch bytes into the two [`OperatorPatch`]es. Called when `$3X` moves,
    /// and on every channel using the custom patch when one of `$00`-`$07` moves.
    fn load_patch(&mut self, bytes: [u8; 8]) {
        self.modulator_patch = OperatorPatch::decode(bytes, 0);
        self.carrier_patch = OperatorPatch::decode(bytes, 1);
        self.feedback = bytes[3] & 0x07;
    }

    /// One sample of this channel, in [`UNIT`]s: `UNIT` plus the carrier's signed amplitude
    /// while it sounds, and **0** while it does not.
    ///
    /// The offset is what keeps `M` positive, and the reason it belongs to the channel rather
    /// than to the chip is in [`mixer::vrc7_level`]: a chip with nothing playing must have an `M`
    /// of exactly 0.
    // Q-128, Q-129
    fn run(&mut self, counter: u32, tremolo: i32, vibrato: i32) -> i32 {
        if matches!(self.carrier.eg_phase, EgPhase::Idle) {
            // A released channel contributes nothing, and its phases are reset by the next
            // key-on, so there is nothing to keep moving.
            return 0;
        }

        let (block, fnum, sustain) = (self.block, self.fnum, self.sustain);
        self.modulator
            .advance_envelope(&self.modulator_patch, block, fnum, sustain, counter);
        self.carrier
            .advance_envelope(&self.carrier_patch, block, fnum, sustain, counter);

        // The feedback path, in phase units: a modulation index of `2^(F-6)` cycles per unit of
        // the modulator's own output, which is the page's radian table divided by 2 pi.
        let modulation = if self.feedback == 0 {
            0
        } else {
            ((self.history[0] + self.history[1]) / 2) << (self.feedback - 1)
        };

        let modulator_attenuation =
            self.modulator
                .attenuation(&self.modulator_patch, block, fnum, tremolo);
        let out = self.modulator.run(
            &self.modulator_patch,
            block,
            fnum,
            if self.modulator_patch.vibrato {
                vibrato
            } else {
                0
            },
            modulation >> (PHASE_BITS - SINE_BITS),
            modulator_attenuation,
        );
        self.history[1] = self.history[0];
        self.history[0] = out;

        // "The carrier produces the output tone, and the output of the modulator modulates its
        // frequency." The index at the modulator's full output is **2 pi** — one whole cycle of
        // the carrier's phase — which no allowed source gives numerically (**Q-129**); the
        // feedback table's own top entry of 4 pi is what puts it in that order.
        let carrier_attenuation =
            self.carrier
                .attenuation(&self.carrier_patch, block, fnum, tremolo)
                + self.volume;
        UNIT + self.carrier.run(
            &self.carrier_patch,
            block,
            fnum,
            if self.carrier_patch.vibrato {
                vibrato
            } else {
                0
            },
            (out * (1 << SINE_BITS)) / UNIT,
            carrier_attenuation.min(MAX_ATTENUATION),
        )
    }
}

impl OperatorPatch {
    /// The patch a channel has before a game writes one: every field zero, except that the
    /// multiplier of `MMMM = 0` is a **half** and not a zero, so the phase still advances.
    const fn quiet() -> Self {
        Self {
            tremolo: false,
            vibrato: false,
            sustained: false,
            key_rate_scaling: false,
            multiplier_x2: MULTIPLIER_X2[0],
            key_level_scaling: 0,
            total_level: 0,
            half_wave: false,
            attack: 0,
            decay: 0,
            sustain_level: 0,
            release: 0,
        }
    }

    /// Operator `which` (0 the modulator, 1 the carrier) of the eight patch bytes.
    fn decode(bytes: [u8; 8], which: usize) -> Self {
        let flags = bytes[which];
        let level = bytes[2 + which];
        Self {
            tremolo: flags & 0x80 != 0,
            vibrato: flags & 0x40 != 0,
            sustained: flags & 0x20 != 0,
            key_rate_scaling: flags & 0x10 != 0,
            multiplier_x2: MULTIPLIER_X2[usize::from(flags & 0x0F)],
            key_level_scaling: level >> 6,
            // `$02`'s six bits are the modulator's output level, 0.75 dB a step; `$03` has no
            // output level at all, the carrier's being `$3X`'s volume.
            total_level: if which == 0 {
                i32::from(level & 0x3F) * 32
            } else {
                0
            },
            // "$03 KK-Q WFFF": `Q` is the carrier's waveform and `W` the modulator's.
            half_wave: bytes[3] & (if which == 0 { 0x08 } else { 0x10 }) != 0,
            attack: bytes[4 + which] >> 4,
            decay: bytes[4 + which] & 0x0F,
            sustain_level: bytes[6 + which] >> 4,
            release: bytes[6 + which] & 0x0F,
        }
    }
}

// ------------------------------------------------------------------------- the chip

/// The VRC7's expansion audio: "The VRC7 has one DAC and output pin for audio, multiplexed for
/// all 6 channels" (REQ-CORE-075, optional; the level is **Q-128**).
///
/// Two CPU-visible ports: `$9010` latches the internal register number and `$9030` writes it.
/// "The VRCVII appears to have 26 internal registers" — `$00`-`$07` the custom patch, `$10`-`$15`
/// the frequency low bytes, `$20`-`$25` the control bytes and `$30`-`$35` the instrument and
/// volume — and "other register values are ignored, except $0F which is a test register", which
/// this build decodes as nothing at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Vrc7Audio {
    /// `$9010`: the internal register the next `$9030` write reaches.
    address: u8,
    /// `$00`-`$07`, the custom patch, exactly as written.
    custom: [u8; 8],
    channels: [Vrc7Channel; Self::CHANNELS],
    /// The envelope generator's shared counter: "a shared global counter (not per-operator)
    /// increments each sample across all 18 operators".
    eg_counter: u32,
    /// 0 to [`TREMOLO_STEPS`] − 1, one step every [`TREMOLO_DIVIDER`] samples.
    tremolo_step: i32,
    tremolo_divider: i32,
    /// 0 to 7, one step every [`VIBRATO_DIVIDER`] samples.
    vibrato_step: i32,
    vibrato_divider: i32,
    /// CPU cycles left before the next sample: "it takes 72 internal clock cycles to update all
    /// of its channels, which means each channel is updated with a frequency of 49716 Hz (or
    /// roughly 36 NES CPU clocks)".
    sample_timer: i32,
    /// `M` of REQ-CORE-073 as it stands, recomputed once per sample — so
    /// [`Vrc7Audio::output`], which the bus reads every CPU cycle, is one load. The same shape
    /// `Vrc6Audio` and `Mmc5Audio` have.
    level: i32,
}

impl Vrc7Audio {
    /// "Six channels exist at registers labeled X (channels 0-5)."
    const CHANNELS: usize = 6;

    /// CPU cycles between two samples: "roughly 36 NES CPU clocks". An integer divider and not
    /// Mesen's floating-point one, because REQ-CORE-092 asks for the same samples from the same
    /// state on every machine.
    const CYCLES_PER_SAMPLE: i32 = 36;

    /// A board's sound at power-on (REQ-CORE-004): every channel released, so silence whatever
    /// the registers say.
    pub fn new() -> Self {
        let mut chip = Self {
            address: 0,
            custom: [0; 8],
            channels: [Vrc7Channel::new(); Self::CHANNELS],
            eg_counter: 0,
            tremolo_step: 0,
            tremolo_divider: TREMOLO_DIVIDER,
            vibrato_step: 0,
            vibrato_divider: VIBRATO_DIVIDER,
            sample_timer: Self::CYCLES_PER_SAMPLE,
            level: 0,
        };
        chip.refresh_level();
        chip
    }

    /// Back to the power-on state (REQ-CORE-004).
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// `M` of REQ-CORE-073: the board's output in the mixer's integer units, a **positive**
    /// quantity the APU subtracts (AMD-061 §6.2).
    pub const fn output(&self) -> i32 {
        self.level
    }

    /// One CPU cycle.
    ///
    /// Three instructions and a call, which is the shape MF-5a's note on `Vrc6Audio` asks for:
    /// `Vrc7::on_cpu_cycle` is one arm of `Board::on_cpu_cycle` and the thirty-two arms of that
    /// dispatch share one prologue, so the body has to stay behind a boundary of its own. Unlike
    /// the VRC6's three oscillators, this chip **has** a cheap early-out — it runs at the board
    /// clock divided by 36 — so the divider inlines and the sample does not.
    #[inline]
    pub fn step_cpu_cycle(&mut self) {
        self.sample_timer -= 1;
        if self.sample_timer <= 0 {
            self.sample_timer = Self::CYCLES_PER_SAMPLE;
            self.run_sample();
        }
    }

    /// One sample of the whole chip, at 49716 Hz.
    #[inline(never)]
    fn run_sample(&mut self) {
        self.eg_counter = self.eg_counter.wrapping_add(1);

        self.tremolo_divider -= 1;
        if self.tremolo_divider == 0 {
            self.tremolo_divider = TREMOLO_DIVIDER;
            self.tremolo_step = (self.tremolo_step + 1) % TREMOLO_STEPS;
        }

        self.vibrato_divider -= 1;
        if self.vibrato_divider == 0 {
            self.vibrato_divider = VIBRATO_DIVIDER;
            self.vibrato_step = (self.vibrato_step + 1) & 7;
        }

        let tremolo = Self::tremolo_attenuation(self.tremolo_step);
        let vibrato = VIBRATO_TABLE[usize::try_from(self.vibrato_step).unwrap_or(0) & 7];
        let counter = self.eg_counter;
        let mut total = 0;
        for channel in &mut self.channels {
            total += channel.run(counter, tremolo, vibrato);
        }

        self.level = mixer::vrc7_level(total);
    }

    /// The tremolo's attenuation this step, in units.
    ///
    /// A triangle over [`TREMOLO_STEPS`], eight steps to each envelope level, so the peak is
    /// thirteen levels — **4.875 dB peak to peak**, which is the 4.8 dB the OPL family is
    /// documented at. The depth itself is **Q-129**: the manual gives the rate and not the depth,
    /// and this is the depth that falls out of the 210-step length [`TREMOLO_DIVIDER`] fixes.
    // Q-129
    const fn tremolo_attenuation(step: i32) -> i32 {
        let up = if step < TREMOLO_STEPS / 2 {
            step
        } else {
            TREMOLO_STEPS - 1 - step
        };
        (up / 8) * EG_STEP
    }

    /// `M` with nothing sounding, which is what power-on and a reset leave: exactly 0, because
    /// an idle channel carries no offset (see [`mixer::vrc7_level`]).
    fn refresh_level(&mut self) {
        self.level = mixer::vrc7_level(0);
    }

    /// A CPU write to `$9010` (the address port) or `$9030` (the data port).
    ///
    /// `silenced` is `$E000`'s `S` bit: "Writes to $9010 and $9030 are disregarded while this bit
    /// is set", which Mesen quotes on its own early return (`Vrc7Audio.h:74-78`).
    ///
    /// **Out of line, measured.** This is reached from the mapper-85 arm of
    /// `NesBus::write_internal`, a function every CPU write of every game passes through.
    /// Inlined, the eight-byte patch copies and the six-channel loop below took that function's
    /// stack frame from `subq $56, %rsp` to `subq $104, %rsp` — the same one instruction, so no
    /// instruction count moved, but 48 more bytes of stack touched on every write of every
    /// board, which is MF-3b rule 3's shape ("the arm that matters is the empty one"). A VRC7
    /// game writes these two ports a few hundred times a frame, so the `callq` is free where the
    /// frame is not.
    #[inline(never)]
    pub fn write(&mut self, address: u16, value: u8, silenced: bool) {
        if silenced {
            return;
        }

        match address & 0xF030 {
            0x9010 => self.address = value,
            0x9030 => self.write_register(value),
            _ => {}
        }
    }

    /// A write to the internal register `self.address`.
    fn write_register(&mut self, value: u8) {
        let register = self.address;
        match register {
            0x00..=0x07 => {
                self.custom[usize::from(register)] = value;
                // Every channel playing the custom patch hears the change at once, which is how
                // a VRC7 game sweeps a sound while it sounds.
                for index in 0..Self::CHANNELS {
                    if self.channels[index].instrument == 0 {
                        let custom = self.custom;
                        self.channels[index].load_patch(custom);
                    }
                }
            }
            0x10..=0x15 => {
                let channel = &mut self.channels[usize::from(register - 0x10)];
                channel.fnum = (channel.fnum & 0x100) | u16::from(value);
            }
            0x20..=0x25 => {
                let channel = &mut self.channels[usize::from(register - 0x20)];
                channel.fnum = (channel.fnum & 0x0FF) | (u16::from(value & 0x01) << 8);
                channel.block = (value >> 1) & 0x07;
                channel.sustain = value & 0x20 != 0;

                // "If trigger bit changes from 0 to 1 (key-on), a new note begins. If it changes
                // from 1 to 0 (key-off), it will begin the release portion of its envelope."
                let triggered = value & 0x10 != 0;
                if triggered && !channel.triggered {
                    channel.modulator.key_on();
                    channel.carrier.key_on();
                    channel.history = [0; 2];
                } else if !triggered && channel.triggered {
                    // "The modulator's sustain bit ($00 S) also disables the release section of
                    // its envelope … when the note is released the modulator will continue to
                    // sustain while the carrier releases. The carrier does not behave this way:
                    // its envelope always enters release when the note is released."
                    if !channel.modulator_patch.sustained {
                        channel.modulator.key_off();
                    }

                    channel.carrier.key_off();
                }

                channel.triggered = triggered;
            }
            0x30..=0x35 => {
                let index = usize::from(register - 0x30);
                let instrument = value >> 4;
                // Four bits of 3 dB, inverted: "$F is the lowest volume and $0 is the highest".
                self.channels[index].volume = i32::from(value & 0x0F) * 128;
                self.channels[index].instrument = instrument;
                let bytes = if instrument == 0 {
                    self.custom
                } else {
                    INSTRUMENTS[usize::from(instrument) - 1]
                };
                self.channels[index].load_patch(bytes);
            }
            // "$0F … is a test register", whose four bits all defeat part of the chip; no game
            // writes it and this build decodes it as nothing. Every other value "is ignored".
            _ => {}
        }
    }

    /// Appends the sound state to `writer` (REQ-CORE-090).
    pub fn save_state(&self, writer: &mut StateWriter) {
        writer.write_u8(self.address);
        writer.write_raw(&self.custom);
        writer.write_u32(self.eg_counter);
        writer.write_i32(self.tremolo_step);
        writer.write_i32(self.tremolo_divider);
        writer.write_i32(self.vibrato_step);
        writer.write_i32(self.vibrato_divider);
        writer.write_i32(self.sample_timer);
        // `level` is derived from the channels, but recomputing it here would need the whole
        // waveform evaluated without advancing it; one `i32` in the block is cheaper and makes
        // the restored `M` exactly the saved one rather than up to 36 CPU cycles stale. The same
        // trade `Namco163Audio::level`'s field doc records.
        writer.write_i32(self.level);
        for channel in &self.channels {
            writer.write_u16(channel.fnum);
            writer.write_u8(channel.block);
            writer.write_bool(channel.triggered);
            writer.write_bool(channel.sustain);
            writer.write_u8(channel.instrument);
            writer.write_i32(channel.volume);
            writer.write_i32(channel.history[0]);
            writer.write_i32(channel.history[1]);
            for operator in [&channel.modulator, &channel.carrier] {
                writer.write_u32(operator.phase);
                writer.write_i32(operator.eg_level);
                writer.write_u8(operator.eg_phase.to_bits());
            }
        }
    }

    /// Restores what [`Vrc7Audio::save_state`] wrote (REQ-CORE-091).
    ///
    /// The two patches of each channel are **not** in the block: they are decoded from `$3X` and
    /// the custom patch, both of which are, so `load_state` rebuilds them exactly as a write
    /// would. One fewer thing that can come back out of step with itself.
    ///
    /// # Errors
    /// [`InvalidState`] when the block is truncated or holds an impossible value.
    pub fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.address = reader.read_u8()?;
        reader.read_into(&mut self.custom)?;
        self.eg_counter = reader.read_u32()?;
        self.tremolo_step = reader.read_i32()?;
        self.tremolo_divider = reader.read_i32()?;
        self.vibrato_step = reader.read_i32()?;
        self.vibrato_divider = reader.read_i32()?;
        self.sample_timer = reader.read_i32()?;
        self.level = reader.read_i32()?;
        for index in 0..Self::CHANNELS {
            let channel = &mut self.channels[index];
            channel.fnum = reader.read_u16()?;
            channel.block = reader.read_u8()?;
            channel.triggered = reader.read_bool()?;
            channel.sustain = reader.read_bool()?;
            channel.instrument = reader.read_u8()?;
            channel.volume = reader.read_i32()?;
            channel.history[0] = reader.read_i32()?;
            channel.history[1] = reader.read_i32()?;
            for operator in 0..2 {
                let phase = reader.read_u32()?;
                let eg_level = reader.read_i32()?;
                let eg_phase = EgPhase::from_bits(reader.read_u8()?);
                let channel = &mut self.channels[index];
                let target = if operator == 0 {
                    &mut channel.modulator
                } else {
                    &mut channel.carrier
                };
                target.phase = phase & ((1 << PHASE_BITS) - 1);
                target.eg_level = eg_level.clamp(0, EG_LEVELS - 1);
                target.eg_phase = eg_phase.ok_or_else(|| {
                    InvalidState::corrupt("VRC7 audio: impossible envelope phase.")
                })?;
            }
        }

        if self.tremolo_step >= TREMOLO_STEPS
            || self.vibrato_step >= 8
            || self.tremolo_divider > TREMOLO_DIVIDER
            || self.vibrato_divider > VIBRATO_DIVIDER
            || self.sample_timer > Self::CYCLES_PER_SAMPLE
        {
            return Err(InvalidState::corrupt(
                "VRC7 audio: impossible LFO phase or sample divider.",
            ));
        }

        // The patches are derived, like `Mmc5Audio`'s cached level: rebuilt here rather than
        // serialised, from the instrument number and the custom patch the block does carry.
        for index in 0..Self::CHANNELS {
            let bytes = match self.channels[index].instrument {
                0 => self.custom,
                instrument if usize::from(instrument) <= INSTRUMENTS.len() => {
                    INSTRUMENTS[usize::from(instrument) - 1]
                }
                _ => {
                    return Err(InvalidState::corrupt(
                        "VRC7 audio: impossible instrument number.",
                    ))
                }
            };
            self.channels[index].load_patch(bytes);
        }

        Ok(())
    }
}
