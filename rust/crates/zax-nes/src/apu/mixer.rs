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
//! **MMC5.** AMD-061 §6.4 abandons the reference's 8 000 and 31 and puts the board on the APU's
//! own curves, because that is what the public documentation says the hardware does: the two
//! square channels are "equivalent in volume to the corresponding APU channels" (nesdev, "MMC5
//! audio"), so they read [`PULSE_TABLE`] on their own summed index, and the PCM channel is
//! "equivalent in volume to the APU with equivalent input … but using the extra high bit of
//! input it can become twice as loud", i.e. the DMC's own curve `T` over a byte instead of seven
//! bits — [`MMC5_PCM_TABLE`]. No new constant and no fitted ratio: the documentation fixes one,
//! so §6.4's fallback to Mesen 2's is not needed.
//!
//! **Polarity.** The same page: "the polarity of all MMC5 channels is reversed compared to the
//! APU". AMD-061 §6.2 therefore makes the mix `S = A − M` with `M` a positive quantity, and
//! widens the clip to [`MIN_SCALE`]`..=`[`FULL_SCALE`]. See [`clip`] for why that costs a game
//! without expansion audio nothing.
//!
//! **Cost.** The three tables are `const`: they are built by the compiler from the formulas
//! above, so a mix is two array reads and an addition, with no initialisation check and no
//! magic numbers in the source. `mixer_tests` recomputes them at runtime and compares.

/// Largest value the mixer produces, `A` and `S` alike (REQ-CORE-073).
pub const FULL_SCALE: i32 = 32_767;

/// Smallest value `S` may take (REQ-CORE-073 as amended by AMD-061 §6.2). `A` never goes below
/// 0; only the subtracted `M` of an expansion audio can take the sum negative.
pub const MIN_SCALE: i32 = -32_768;

/// Divisor taking the integer output to the [-1, 1] scale of REQ-CORE-071.
pub const FLOAT_SCALE: f64 = 32_768.0;

/// Largest index of the pulse table: both pulses at volume 15.
pub const PULSE_INDEX_MAX: usize = 30;

/// Largest index of the triangle/noise/DMC table: 3x15 + 2x15 + 127.
pub const TND_INDEX_MAX: usize = 202;

/// Largest index of the MMC5's PCM table: the whole byte of $5011 (REQ-CORE-075 as amended by
/// AMD-061 §6.4, "the extra high bit of input").
pub const MMC5_PCM_INDEX_MAX: usize = 255;

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

/// The 256 entries `r(K · T(i))` the MMC5's PCM level reads (REQ-CORE-075 as amended by
/// AMD-061 §6.4). `T` and `K` are the APU's own, so the first 203 entries are exactly
/// [`TND_TABLE`] — the PCM channel *is* the DMC's curve — and the 53 above it are the eighth
/// bit of $5011 that the APU's seven-bit DMC has no room for.
pub const MMC5_PCM_TABLE: [i32; MMC5_PCM_INDEX_MAX + 1] =
    scaled(TND_NUMERATOR, TND_DENOMINATOR, NORMALISATION);

/// Number of levels one Sunsoft 5B channel's amplitude can take: the envelope generator's
/// "5-bit series of output levels".
pub const SUNSOFT_5B_LEVELS: usize = 32;

/// Amplitude ratio between two consecutive levels of [`SUNSOFT_5B_TABLE`]: `10^(1.5 / 20)`,
/// written out because `powf` is not a `const fn`.
///
/// nesdev, "Sunsoft 5B audio": the envelope's levels are "1.5db logarithmic steps", and since
/// "maximum level 31 corresponds to the 4-bit volume register level 15" a step of the *volume*
/// register is two of these, i.e. 3 dB. Mesen 2 builds its own table from the same figure
/// (`Sunsoft5bAudio.h:108-116`: `output *= 1.1885022274370184…` twice per volume step), so the
/// law — unlike the level, see [`SUNSOFT_5B_TABLE`] — is not in doubt.
/// `mixer_tests` recomputes the table from 1.5 dB with `powf` and compares.
const SUNSOFT_5B_RATIO: f64 = 1.188_502_227_437_018_4;

/// Amplitude of each of the 32 levels of one Sunsoft 5B channel, in the mixer's integer units
/// (`M` of REQ-CORE-073). The three channels are summed **linearly** into `M`, which is what the
/// chip does: "The three output channels are mixed together linearly" (nesdev, "Sunsoft 5B
/// audio").
///
/// **Q-118.** REQ-CORE-075 leaves this board's sound optional and gives it no level, and the page
/// says only that "It is very loud compared to other audio expansion carts" — no ratio, no
/// resistor values, nothing on polarity. The provisional choice is the anchor AMD-061 §6.4 used
/// for the MMC5, applied to the one quantitative sentence available: **level 31 of one channel is
/// one APU pulse at volume 15**, `PULSE_TABLE[15]`, because 31 is that channel's full converter
/// swing as volume 15 is a pulse's. Three channels therefore cap `M` at 3 × 4 890 = 14 670, 45 %
/// of full scale. Unlike the MMC5's two squares, which share one index of [`PULSE_TABLE`] because
/// they sit in the APU's own non-linear network, these three are added after the table: the 5B is
/// a separate chip whose output enters through the cartridge's audio pin. The ticket records the
/// two other readings; **one constant changes the level** if R0 wants another.
///
/// `M` is positive, so the APU **subtracts** it as it subtracts the MMC5's (AMD-061 §6.2). That
/// is also Q-118's provisional answer on polarity, and it is not free to reverse: [`clip`] only
/// implements the lower bound *because* `M` cannot be negative.
pub const SUNSOFT_5B_TABLE: [i32; SUNSOFT_5B_LEVELS] = sunsoft_5b_table();

/// Builds [`SUNSOFT_5B_TABLE`] downwards from its anchor, so that the top entry is exactly
/// `PULSE_TABLE[15]` whatever the rounding does below it.
const fn sunsoft_5b_table() -> [i32; SUNSOFT_5B_LEVELS] {
    let mut table = [0; SUNSOFT_5B_LEVELS];
    let anchor = NORMALISATION * pulse(15);
    let mut amplitude = anchor;
    let mut level = SUNSOFT_5B_LEVELS - 1;
    while level > 0 {
        table[level] = round_to_even(amplitude);
        amplitude /= SUNSOFT_5B_RATIO;
        level -= 1;
    }

    // Level 0 is silence. The converter's lowest step is not truly zero on the chip, but the
    // page's own mapping makes volume 0 the bottom of the scale and Mesen 2 does the same
    // (`_volumeLut[0] = 0`).
    table
}

/// Steps of the VRC6's DAC: "The final mix is a 6-bit DAC summing the two 4-bit pulse outputs
/// and the high 5 bits of the saw accumulator" (nesdev, "VRC6 audio") — 15 + 15 + 31, plus the
/// zero.
pub const VRC6_LEVELS: usize = 62;

/// `M` of REQ-CORE-073 for each step of the VRC6's DAC (REQ-CORE-075, optional).
///
/// **The page fixes both halves of this, which is what Q-118 said would settle such a level.**
/// Q-118 had to choose an anchor for the Sunsoft 5B because its page gives none — only "It is
/// very loud compared to other audio expansion carts" — and R0's answer kept the reading that a
/// channel at full converter swing equals an APU pulse at volume 15. The VRC6's page says the
/// same thing outright, and says the law too:
///
/// * the anchor — "At maximum volume, the pulse channels of the VRC6 are roughly equivalent to
///   the pulse channels of the 2A03 (except inverted)", so one pulse at `V = 15` is
///   [`PULSE_TABLE`]`[15]`;
/// * the law — "The DAC of the VRC6, unlike the 2A03, appears to be linear", and the mix is a
///   plain sum of the three channels, so step `i` is `i / 15` of that anchor.
///
/// `M` therefore caps at 61 × `PULSE_TABLE[15]` / 15 ≈ 19 886, about 61 % of full scale, which
/// is the right order for a chip the VRC6 pages call loud. Built from the unrounded anchor
/// rather than by multiplying the rounded `PULSE_TABLE[15]`, so the step is not quantised twice.
///
/// **Polarity: subtracted, and the page asks for it in so many words** — "(except inverted)".
/// For the 5B that was Q-118's provisional choice for want of a document; here the document
/// says it, and it is also the only polarity `clip` implements (AMD-061 §6.2), so the two agree
/// and nothing in the mixer had to move.
pub const VRC6_TABLE: [i32; VRC6_LEVELS] = vrc6_table();

/// Steps the Namco 163's summed output can take: one channel is `sample x volume` with both
/// four bits wide, so 0 to 225, and the sum of the enabled channels is divided by how many there
/// are — "it is often preferred to simply sum the channel outputs, and divide the output volume
/// by the number of active channels" (nesdev, "Namco 163 audio"). So the whole chip has the swing
/// of one channel.
pub const NAMCO163_LEVELS: usize = 15 * 15 + 1;

/// How much louder than an APU square the Namco 163 is, as an amplitude ratio: 10^(12/20).
///
/// Written out because `powf` is not a `const fn`, exactly as [`SUNSOFT_5B_RATIO`] is. The 12 dB
/// is the midpoint of the band the page gives NES 2.0 submapper 3, "N163 expansion sound:
/// 11.0-13.0 dB louder than NES APU" -- see [`NAMCO163_TABLE`] and Q-127.
pub const NAMCO163_GAIN: f64 = 3.981_071_705_534_972;

/// `M` of REQ-CORE-073 for each step of the Namco 163's output (REQ-CORE-075, optional).
///
/// **The law comes from the page and only the level needed a choice.** The page is explicit about
/// the shape of the mix -- sum the enabled channels, divide by how many are enabled, each channel
/// being a four-bit sample times a four-bit volume -- so the table is a straight line of 226
/// steps, as the VRC6's is of 62.
///
/// **Q-127, the level.** Unlike the VRC6's page, which hands over its anchor in a sentence, and
/// unlike the 5B's, which gives nothing at all, this page gives a *range per submapper*: 11.0 to
/// 13.0 dB louder than the NES APU for submapper 3, 16.0 to 17.0 for submapper 4 and 18.0 to 19.5
/// for submapper 5, with a per-title table from 11.2 dB to 19.5 dB. There is therefore no single
/// level, and an iNES 1.0 dump carries no submapper at all. The provisional choice is **one
/// table at 12.0 dB**, the midpoint of submapper 3's band:
///
/// * Q-118's rule still fixes what the dB is measured against -- one channel at its full
///   converter swing against one APU pulse at volume 15, `PULSE_TABLE[15]` -- so the top entry is
///   4 890 x [`NAMCO163_GAIN`] = about **19 467**, 60 % of full scale, which is within a few
///   hundred of the VRC6's own 19 886 and so needs nothing of [`clip`] that is not already there;
/// * submapper 3's band is the widest and the lowest, so an unlabelled dump placed in it is never
///   made louder than the documentation allows, where placing it in submapper 5's band would risk
///   the opposite on every image.
///
/// The ticket carries the two alternatives -- one table per submapper, or the per-title figures --
/// and **one constant changes the level** if R0 wants another.
///
/// **Polarity: subtracted, like every other board's `M`.** The chip's own converter is unipolar
/// (`sample x volume`, not Mesen's centred `(sample - 8) x volume`), which is what lets `M` stay
/// positive -- and [`clip`] only implements the lower bound *because* `M` cannot be negative
/// (AMD-061 §6.2).
pub const NAMCO163_TABLE: [i32; NAMCO163_LEVELS] = namco163_table();

/// Builds [`NAMCO163_TABLE`]: a straight line through `NAMCO163_GAIN` x `PULSE_TABLE[15]` at its
/// top step, from the unrounded anchor so that the step is not quantised twice.
const fn namco163_table() -> [i32; NAMCO163_LEVELS] {
    #[allow(clippy::cast_precision_loss)] // NAMCO163_LEVELS - 1 is 225.
    let step = NORMALISATION * pulse(15) * NAMCO163_GAIN / (NAMCO163_LEVELS - 1) as f64;
    let mut table = [0; NAMCO163_LEVELS];
    let mut level = 0;
    #[allow(clippy::cast_precision_loss)] // level is at most 225.
    while level < NAMCO163_LEVELS {
        table[level] = round_to_even(step * level as f64);
        level += 1;
    }

    table
}

/// Builds [`VRC6_TABLE`]: a straight line through `PULSE_TABLE[15]` at step 15.
const fn vrc6_table() -> [i32; VRC6_LEVELS] {
    let step = NORMALISATION * pulse(15) / 15.0;
    let mut table = [0; VRC6_LEVELS];
    let mut level = 0;
    #[allow(clippy::cast_precision_loss)] // level is at most 61.
    while level < VRC6_LEVELS {
        table[level] = round_to_even(step * level as f64);
        level += 1;
    }

    table
}

/// Amplitude level of a Sunsoft 5B channel whose volume register holds `volume`, for
/// [`SUNSOFT_5B_TABLE`].
///
/// nesdev: "maximum level 31 corresponds to the 4-bit volume register level 15", so the four-bit
/// volume lands on the odd levels, `2V + 1`. Volume 0 is silence rather than level 1 (Q-118).
#[must_use]
pub const fn sunsoft_5b_level(volume: u8) -> usize {
    if volume == 0 {
        0
    } else {
        2 * (volume & 0x0F) as usize + 1
    }
}

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

/// `S` of REQ-CORE-073 as AMD-061 §6.2 amends it — `S = A − M`, clipped to
/// [`MIN_SCALE`]`..=`[`FULL_SCALE`] — the value the test point of REQ-CORE-076 reports.
///
/// **Only the lower clip is tested, and that is not a shortcut.** `A` is a sum of two table
/// entries that cannot exceed 32 767 together, and `M` is a positive quantity by §6.2, so
/// `A − M ≤ FULL_SCALE` holds for every pair the mixer can produce: the upper arm of the clip
/// is unreachable, and emitting it would be dead code. What is left is one subtraction and one
/// clamp — the same three instructions as the `min(A + M, 32767)` this replaces, so a game
/// without expansion audio (where `M` is 0 and `S` is `A`) executes exactly what it did before.
#[must_use]
pub const fn clip(apu: i32, expansion: i32) -> i32 {
    let sum = apu - expansion;
    if sum < MIN_SCALE {
        MIN_SCALE
    } else {
        sum
    }
}

/// Takes an integer mixer output to the [-1, 1] scale of REQ-CORE-071, by `S / 32768`.
#[must_use]
pub fn to_float(output: i32) -> f64 {
    f64::from(output) / FLOAT_SCALE
}
