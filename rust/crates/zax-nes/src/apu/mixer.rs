//! The non-linear mixer of REQ-CORE-073 (AMD-054): the two integer tables the five APU
//! channels are summed through, and the companion table the MMC5's two square channels use
//! (REQ-CORE-075).
//!
//! **The two functions.** nesdev's "APU Mixer" page gives the mixer as two real functions of a
//! channel index, which REQ-CORE-073 restates: `P(i) = 95.88 / (8128 / i + 100)` for the two
//! pulses summed, `i` from 0 to 30, and `T(i) = 163.67 / (24329 / i + 100)` for the triangle,
//! the noise and the DMC summed as `3t + 2n + d`, `i` from 0 to 202. Both are 0 at `i = 0`,
//! which the formulas do not define.
//!
//! **Why 95.88 and not 95.52.** The same page gives two sets of constants: the analytic forms
//! (95.88 for the pulses, 159.79 for the rest) and the lookup-table forms (95.52 and 163.67).
//! REQ-CORE-073 asks for 95.88 together with `163.67 / 24329`, which is the mix of the two the
//! reference uses; 95.52 would give 4876 / 8438 / 8371 instead of the 4890 / 8462 / 8363 the
//! requirement's criterion names. The choice is the requirement's, and its own note records it.
//!
//! **Scaling.** [`NORMALISATION`] is `K = 32767 / (P(30) + T(202))`, computed on the unrounded
//! values in double precision, so that the two terms at full scale add up to exactly 32767 —
//! 8462 + 24305, which holds after both roundings and is what stops `A` on its own from ever
//! overflowing. The two tables are the rounded products `r(K · P(i))` and `r(K · T(i))`, `r`
//! rounding to the nearest integer and to the even one on a tie, as REQ-CORE-073 prescribes.
//! One entry sits close enough to a tie for the rule to be worth pinning:
//! `K · T(196) = 23905.4997`.
//!
//! **MMC5.** [`MMC5_NORMALISATION`] is `Km = 8000 / P(30)`, so that the board's two square
//! channels together top out at 8000 rather than at the APU's 8462, and its PCM channel steps
//! by [`MMC5_PCM_STEP`] per unit. Both constants are the reference's, and REQ-CORE-075 records
//! that they put the PCM channel well below where the public documentation would (an accepted
//! divergence).
//!
//! **Cost.** The three tables are `const`: they are built by the compiler from the formulas
//! above, so a mix is two array reads and an addition, with no initialisation check and no
//! magic numbers in the source. `mixer_tests` recomputes them at runtime and compares.

/// Largest value the mixer produces, `A` and `S` alike (REQ-CORE-073).
pub const FULL_SCALE: i32 = 32_767;

/// Divisor taking the integer output to the [-1, 1] scale of REQ-CORE-071.
pub const FLOAT_SCALE: f64 = 32_768.0;

/// Largest index of the pulse table: both pulses at volume 15.
pub const PULSE_INDEX_MAX: usize = 30;

/// Largest index of the triangle/noise/DMC table: 3x15 + 2x15 + 127.
pub const TND_INDEX_MAX: usize = 202;

/// Value the MMC5's two square channels reach together at volume 15 (REQ-CORE-075).
pub const MMC5_FULL_SCALE: i32 = 8_000;

/// Mixer units one unit of the MMC5's PCM level contributes (REQ-CORE-075).
pub const MMC5_PCM_STEP: i32 = 31;

/// Numerator of `P`, nesdev's analytic form for the pulses.
const PULSE_NUMERATOR: f64 = 95.88;

/// Denominator constant of `P`.
const PULSE_DENOMINATOR: f64 = 8128.0;

/// Numerator of `T`, nesdev's lookup-table form.
const TND_NUMERATOR: f64 = 163.67;

/// Denominator constant of `T`.
const TND_DENOMINATOR: f64 = 24329.0;

/// `P(i)`, the real pulse function of REQ-CORE-073, 0 at `i = 0`.
#[must_use]
pub const fn pulse(i: usize) -> f64 {
    real(i, PULSE_NUMERATOR, PULSE_DENOMINATOR)
}

/// `T(i)`, the real triangle/noise/DMC function of REQ-CORE-073, 0 at `i = 0`.
#[must_use]
pub const fn tnd(i: usize) -> f64 {
    real(i, TND_NUMERATOR, TND_DENOMINATOR)
}

/// `numerator / (denominator / i + 100)`, and 0 at `i = 0`.
#[allow(clippy::cast_precision_loss)] // i is at most 202.
const fn real(i: usize, numerator: f64, denominator: f64) -> f64 {
    if i == 0 {
        0.0
    } else {
        numerator / ((denominator / i as f64) + 100.0)
    }
}

/// Rounds `value` as the `r` of REQ-CORE-073 does: to the nearest integer, and to the even one
/// when the two are equally near. Only ever called on values in `0..=32767`, where the
/// truncating cast is the floor.
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)] // Bounded by 32767.
pub const fn round_to_even(value: f64) -> i32 {
    let floor = value as i64;
    let fraction = value - floor as f64;
    let nearest = if fraction > 0.5 {
        floor + 1
    } else if fraction < 0.5 || floor % 2 == 0 {
        floor
    } else {
        floor + 1
    };
    nearest as i32
}

/// `K = 32767 / (P(30) + T(202))` (REQ-CORE-073).
pub const NORMALISATION: f64 = FULL_SCALE as f64 / (pulse(PULSE_INDEX_MAX) + tnd(TND_INDEX_MAX));

/// `Km = 8000 / P(30)` (REQ-CORE-075).
pub const MMC5_NORMALISATION: f64 = MMC5_FULL_SCALE as f64 / pulse(PULSE_INDEX_MAX);

/// Scales a real table by `factor` and rounds every entry.
const fn scaled<const N: usize>(numerator: f64, denominator: f64, factor: f64) -> [i32; N] {
    let mut table = [0; N];
    let mut i = 0;
    while i < N {
        table[i] = round_to_even(factor * real(i, numerator, denominator));
        i += 1;
    }
    table
}

/// The 31 entries `r(K · P(i))` of REQ-CORE-073.
pub const PULSE_TABLE: [i32; PULSE_INDEX_MAX + 1] =
    scaled(PULSE_NUMERATOR, PULSE_DENOMINATOR, NORMALISATION);

/// The 203 entries `r(K · T(i))` of REQ-CORE-073.
pub const TND_TABLE: [i32; TND_INDEX_MAX + 1] =
    scaled(TND_NUMERATOR, TND_DENOMINATOR, NORMALISATION);

/// The 31 entries `r(Km · P(i))` of REQ-CORE-075.
pub const MMC5_PULSE_TABLE: [i32; PULSE_INDEX_MAX + 1] =
    scaled(PULSE_NUMERATOR, PULSE_DENOMINATOR, MMC5_NORMALISATION);

/// `A` of REQ-CORE-073: the APU's own contribution, from the five channel outputs.
///
/// `pulse1` and `pulse2` are 0 to 15, `triangle` and `noise` 0 to 15, `dmc` 0 to 127.
#[must_use]
pub fn mix(pulse1: i32, pulse2: i32, triangle: i32, noise: i32, dmc: i32) -> i32 {
    mix_indexed(pulse1 + pulse2, (3 * triangle) + (2 * noise) + dmc)
}

/// `A` of REQ-CORE-073 from the two table indices already summed, which is what the APU itself
/// tracks. Out-of-range indices are clamped, which cannot happen with real channel outputs.
#[must_use]
pub fn mix_indexed(pulse_index: i32, tnd_index: i32) -> i32 {
    let pulse = usize::try_from(pulse_index)
        .unwrap_or(0)
        .min(PULSE_INDEX_MAX);
    let tnd = usize::try_from(tnd_index).unwrap_or(0).min(TND_INDEX_MAX);
    PULSE_TABLE[pulse] + TND_TABLE[tnd]
}

/// `S = min(A + M, 32767)` of REQ-CORE-073, the value the test point of REQ-CORE-076 reports.
#[must_use]
pub const fn clip(apu: i32, expansion: i32) -> i32 {
    let sum = apu + expansion;
    if sum > FULL_SCALE {
        FULL_SCALE
    } else {
        sum
    }
}

/// Takes an integer mixer output to the [-1, 1] scale of REQ-CORE-071, by `S / 32768`.
#[must_use]
pub fn to_float(output: i32) -> f64 {
    f64::from(output) / FLOAT_SCALE
}
