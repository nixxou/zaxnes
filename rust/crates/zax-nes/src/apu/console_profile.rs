//! Per-console audio profiles: the analog path of a given NES or Famicom between the APU's mixer
//! and its audio connector, measured on real consoles (plan step 3 of lot FIDELITY).
//!
//! The figures are `BeesNES`'s (`Src/Audio/LSNAudioOptions.cpp:26-62`, L. Spiro, MIT licence; see
//! `THIRD-PARTY-NOTICES.md`): for each console, the corner of its low-pass, the corners of its
//! three high-passes, the relative gain of each channel, whether the output is inverted, and
//! whether its CPU is a "letterless" RP2A03 without the noise channel's short mode. `BeesNES` fitted
//! them to recordings of each console.
//!
//! What is kept from `BeesNES`, and what is not (see `docs/review/FIDELITY-LEADS.md`, E1):
//! * the channel gains enter the non-linear mix before the curves, as `BeesNES` does
//!   (`CApu2A0X::Tick`, `LSNApu2A0X.h:466-495`), with the exact formulas of nesdev ("APU Mixer")
//!   instead of the linear approximation of the TND table;
//! * the three high-passes are `BeesNES`'s one-pole RC, with its time constant `1 / f` (not
//!   `1 / 2πf`): the corners were fitted with that formula, so it is kept to the letter
//!   (`CHpfFilter::CreateHpf`, `LSNHpfFilter.h:36-48`; `CSampleBox::Init`, `LSNSampleBox.h:221-224`);
//! * the low-pass, a pole-zero filter `BeesNES` clocks 1.79 million times a second (`m_pfLpf`),
//!   and the output pole after it, run at the output rate as a filter fitted to their response
//!   where the band-limited signal is (`BandFilter`, lot FIDPLUS-D1d): within 0.001 dB up to 0.35
//!   of the output rate, the zeros and the output pole included, 14 samples of delay;
//! * the overall volume of each profile is not applied — it calibrates `BeesNES`'s own output level
//!   to each recording and would clip ours by up to 4.5 times — but its sign is.
//!
//! Nothing here runs per CPU cycle: the gains apply when the level changes, the filters once per
//! output sample, and a profile left at "none" keeps the output bit for bit as before.

use std::f64::consts::PI;

/// One console's analog audio path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConsoleAudioProfile {
    /// Name of the console, as `BeesNES` lists it.
    pub name: &'static str,
    /// Corner of the low-pass, Hz; 0 when that console has none.
    pub low_pass_hz: f64,
    /// Zero of the low-pass, Hz; 0 when it has none (`BeesNES`'s `fZero`).
    pub low_pass_zero_hz: f64,
    /// Corners of the three high-passes, Hz, in their order on the path.
    pub high_pass_hz: [f64; 3],
    /// Whether each high-pass is on.
    pub high_pass_on: [bool; 3],
    /// Gains of pulse 1, pulse 2, triangle, noise and DMC before the mix.
    pub gains: [f64; 5],
    /// Whether the output is inverted.
    pub inverted: bool,
    /// Whether the CPU is a letterless RP2A03, whose noise channel has no short mode.
    pub letterless: bool,
}

const fn profile(
    name: &'static str,
    low_pass: [f64; 2],
    high_pass_hz: [f64; 3],
    high_pass_on: [bool; 3],
    gains: [f64; 5],
    inverted: bool,
    letterless: bool,
) -> ConsoleAudioProfile {
    ConsoleAudioProfile {
        name,
        low_pass_hz: low_pass[0],
        low_pass_zero_hz: low_pass[1],
        high_pass_hz,
        high_pass_on,
        gains,
        inverted,
        letterless,
    }
}

const ALL: [bool; 3] = [true; 3];

/// The consoles `BeesNES` measured, in its order. Index 0 of the setting is "none"; index `n` is
/// `CONSOLE_AUDIO_PROFILES[n - 1]`.
#[allow(clippy::unreadable_literal, clippy::excessive_precision)] // Measured figures, verbatim.
pub const CONSOLE_AUDIO_PROFILES: [ConsoleAudioProfile; 14] = [
    profile(
        "NES Front-Loading N0260073",
        [18383.331849143484, 0.0],
        [128.203310801000, 36.579855150267, 36.579848133916],
        ALL,
        [1.0, 1.0, 0.933, 0.744, 0.938],
        true,
        true,
    ),
    profile(
        "NES Front-Loading N1151667",
        [16596.500112773516, 0.0],
        [91.670577848853171, 37.490016890980982, 7.9671720814703768],
        ALL,
        [1.0, 1.0, 0.924, 0.883, 0.943],
        true,
        false,
    ),
    profile(
        "NES Front-Loading N13721267",
        [17524.208150165094, 144384.174617951474],
        [97.341173341196, 61.825863515323, 6.059893000666],
        ALL,
        [1.0, 0.994, 0.975, 0.952, 0.966],
        true,
        false,
    ),
    profile(
        "NES Front-Loading N20914398, Sunsoft 5B",
        [0.0, 0.0],
        [224.20062289111536, 125.06316358912643, 51.287540390847823],
        ALL,
        [1.0, 1.0, 0.924, 0.906, 1.0],
        false,
        false,
    ),
    profile(
        "NES Front-Loading N32670105",
        [15851.963185973395, 0.0],
        [99.407819166312, 7.934473497640, 7.934471615916],
        ALL,
        [1.0, 0.975, 0.952, 0.933, 0.975],
        true,
        false,
    ),
    profile(
        "NES Front-Loading N34169630",
        [16879.431344679029, 145249.618967504910],
        [307.866180500731, 77.224662166419, 9.329261475967],
        ALL,
        [0.975, 0.975, 0.975, 0.910, 1.0],
        true,
        false,
    ),
    profile(
        "NES Front-Loading N8869725, NESRGB",
        [14482.132075089257, 0.0],
        [83.459640336709199, 55.985366776125886, 8.5714225234124299],
        ALL,
        [1.0, 1.0, 0.966, 0.924, 1.0],
        true,
        false,
    ),
    profile(
        "NES Front-Loading N8869725, NESRGB, stock RCA",
        [16003.975697663334, 0.0],
        [96.678039344736646, 36.696956603852406, 8.3012408410527119],
        ALL,
        [1.0, 1.0, 0.892, 0.855, 0.924],
        true,
        false,
    ),
    profile(
        "NES Top-Loading NN101255729, NESRGB",
        [14893.677244647428, 0.0],
        [80.308463849114986, 52.357757591613563, 7.9206051188443904],
        ALL,
        [1.0, 1.0, 0.731, 0.693, 0.744],
        true,
        false,
    ),
    profile(
        "Twin Famicom 441850, stock RCA",
        [3719.752299817048, 0.0],
        [206.591994496089, 24.535811985325, 24.535802584134],
        ALL,
        [0.906, 0.883, 1.0, 0.929, 1.0],
        true,
        false,
    ),
    profile(
        "Twin Famicom 475711, NESRGB, stock RCA",
        [6813.7368995, 0.0],
        [202.359654532700, 34.967573649161, 34.967565533357],
        ALL,
        [0.938, 0.938, 1.0, 0.869, 0.929],
        true,
        false,
    ),
    profile(
        "Twin Famicom 475711, NESRGB, SCART",
        [0.0, 0.0],
        [302.96765178343128, 30.586358931812008, 4.0903076481735425],
        ALL,
        [0.975, 1.0, 0.841, 0.827, 0.896],
        true,
        false,
    ),
    profile(
        "PAL NES NESE-001 (AUS) PM313006",
        [14912.561817246404, 188764.928831997269],
        [99.713059858228, 64.057801563356, 8.189826234573],
        ALL,
        [0.882, 0.859, 0.814, 0.771, 0.808],
        true,
        false,
    ),
    profile(
        "PAL NES NESE-001 (GBR) PM608755",
        [64000.0, 0.0],
        [94.986785805568559, 8.851913371856071, 8.8518991011365298],
        ALL,
        [1.0, 1.0, 0.957, 1.0, 1.0],
        true,
        false,
    ),
];

impl ConsoleAudioProfile {
    /// The mixer's output for the five channel levels (pulses, triangle and noise 0 to 15, DMC
    /// 0 to 127), each weighted by this console's gain, on the scale of `mixer::pulse` +
    /// `mixer::tnd` (nesdev, "APU Mixer": the exact non-linear formulas).
    #[must_use]
    pub fn mix(&self, levels: [i32; 5]) -> f64 {
        let [p1, p2, t, n, d] = levels.map(f64::from);
        let [g1, g2, gt, gn, gd] = self.gains;
        let pulses = p1.mul_add(g1, p2 * g2);
        let pulse = if pulses > 0.0 {
            95.88 / ((8128.0 / pulses) + 100.0)
        } else {
            0.0
        };
        let tnd_sum = (t * gt / 8227.0) + (n * gn / 12241.0) + (d * gd / 22638.0);
        let tnd = if tnd_sum > 0.0 {
            159.79 / ((1.0 / tnd_sum) + 100.0)
        } else {
            0.0
        };
        pulse + tnd
    }
}

/// Corner of `BeesNES`'s output pole, and of its low-pass when a console's is off or above the
/// output's Nyquist frequency: `output / 2 + 100` Hz (`LSNApu2A0X.h:725-737`) at the rate its
/// profiles were fitted at, 44 100 Hz (its default output, `LSNAudioOptions.h:96`; the
/// recordings, `README.txt:63`). A part of the fitted model, so fixed whatever our output rate.
pub(crate) const MODEL_POLE_HZ: f64 = 44_100.0 / 2.0 + 100.0;

/// Taps of the band filter's correction (lot FIDPLUS-D1d).
const BAND_TAPS: usize = 32;

/// Output samples the band filter delays the signal by: the room its correction needs to match
/// the analog response's phase (0.29 ms at 48 kHz, inside the profile option only).
pub(crate) const BAND_DELAY: usize = 14;

/// Frequencies the band filter is fitted on, from 0 to the output's Nyquist frequency.
const FIT_POINTS: usize = 800;

/// The magnitude of the resampler's band-limiting kernel on [`FIT_POINTS`] frequencies: the
/// fit's weights, since nothing the kernel removes reaches the band filter.
static KERNEL_RESPONSE: std::sync::OnceLock<Box<[f64]>> = std::sync::OnceLock::new();

/// The magnitude of the windowed sinc of `band_limited_resampler` at `FIT_POINTS` frequencies
/// (cycles per output sample, 0 to 0.5), by integration over its span.
#[allow(clippy::cast_precision_loss)] // Small counts.
fn kernel_response() -> Box<[f64]> {
    const STEPS: usize = 32 * 128;
    let dx = 32.0 / STEPS as f64;
    let impulse: Vec<f64> = (0..=STEPS)
        .map(|k| super::band_limited_resampler::kernel_impulse((k as f64).mul_add(dx, -16.0)))
        .collect();
    (0..FIT_POINTS)
        .map(|i| {
            let nu = 0.5 * i as f64 / (FIT_POINTS - 1) as f64;
            // The impulse is even: its response is the cosine transform, by the trapezoid rule.
            let sum: f64 = impulse
                .iter()
                .enumerate()
                .map(|(k, &value)| {
                    let x = (k as f64).mul_add(dx, -16.0);
                    let weight = if k == 0 || k == STEPS { 0.5 } else { 1.0 };
                    weight * value * (2.0 * PI * nu * x).cos()
                })
                .sum();
            (sum * dx).abs()
        })
        .collect()
}

/// A complex number, for the fit.
#[derive(Debug, Clone, Copy)]
struct Complex(f64, f64);

impl Complex {
    fn mul(self, other: Self) -> Self {
        Self(
            self.0.mul_add(other.0, -(self.1 * other.1)),
            self.0.mul_add(other.1, self.1 * other.0),
        )
    }
    fn div(self, other: Self) -> Self {
        let norm = other.0.mul_add(other.0, other.1 * other.1);
        Self(
            self.0.mul_add(other.0, self.1 * other.1) / norm,
            self.1.mul_add(other.0, -(self.0 * other.1)) / norm,
        )
    }
}

/// The analog response `BeesNES`'s model gives a console at `hz`: its low-pass (pole, and zero
/// when it has one), then the output pole. The model's filters run at 1.79 MHz, bilinear; the
/// analog form is the circuit they stand for, within 6e-4 of their frequency scale in band.
fn model_response(pole_hz: f64, zero_hz: f64, hz: f64) -> Complex {
    let one = Complex(1.0, 0.0);
    let mut response = one.div(Complex(1.0, hz / pole_hz));
    if zero_hz > 0.0 {
        response = response.mul(Complex(1.0, hz / zero_hz));
    }
    response.div(Complex(1.0, hz / MODEL_POLE_HZ))
}

/// Least squares by Householder QR: the `columns`-long `x` minimising `|a x - b|`, `a` stored
/// row after row.
fn least_squares(mut a: Vec<f64>, mut b: Vec<f64>, columns: usize) -> Vec<f64> {
    let rows = b.len();
    for column in 0..columns {
        let norm = (column..rows)
            .map(|row| a[row * columns + column].powi(2))
            .sum::<f64>()
            .sqrt();
        if norm == 0.0 {
            continue;
        }
        let pivot = a[column * columns + column];
        let alpha = if pivot > 0.0 { -norm } else { norm };
        let mut v: Vec<f64> = (column..rows).map(|row| a[row * columns + column]).collect();
        v[0] -= alpha;
        let v_norm = v.iter().map(|x| x * x).sum::<f64>();
        if v_norm == 0.0 {
            continue;
        }
        for target in column..columns {
            let dot: f64 = v
                .iter()
                .enumerate()
                .map(|(i, x)| x * a[(column + i) * columns + target])
                .sum();
            let scale = 2.0 * dot / v_norm;
            for (i, x) in v.iter().enumerate() {
                a[(column + i) * columns + target] -= scale * x;
            }
        }
        let dot: f64 = v.iter().enumerate().map(|(i, x)| x * b[column + i]).sum();
        let scale = 2.0 * dot / v_norm;
        for (i, x) in v.iter().enumerate() {
            b[column + i] -= scale * x;
        }
    }
    let mut x = vec![0.0; columns];
    for column in (0..columns).rev() {
        let mut value = b[column];
        for later in column + 1..columns {
            value -= a[column * columns + later] * x[later];
        }
        x[column] = value / a[column * columns + column];
    }
    x
}

/// `BeesNES`'s low-pass and output pole at the output rate (lot FIDPLUS-D1d, FIDELITY-LEADS
/// E1-1): two one-pole recursions at the model's poles (impulse invariant, the exact pole
/// positions), behind a [`BAND_TAPS`]-tap correction fitted by least squares, at setup, to the
/// model's analog response delayed by [`BAND_DELAY`] samples. The fit is weighted by the
/// resampler kernel's response, so it is exact where the signal is: within 0.001 dB and 0.1
/// degree up to 0.35 of the output rate for every profile (prototype, 44.1 and 48 kHz). The
/// zeros (144 to 189 kHz) and the output pole at 22.15 kHz, which a bilinear filter at the output
/// rate cannot place, are in it; before, our pre-warped low-pass alone left the band's top 1.6 to
/// 3.9 dB above the model.
#[derive(Debug, Clone, Copy)]
struct BandFilter {
    poles: [f64; 2],
    pole_output: [f64; 2],
    correction: [f64; BAND_TAPS],
    /// The last inputs, newest first.
    history: [f64; BAND_TAPS],
}

impl BandFilter {
    #[allow(clippy::cast_precision_loss)] // Small counts.
    fn new(profile: &ConsoleAudioProfile, sample_rate: f64) -> Self {
        let pole_hz = if profile.low_pass_hz > 0.0 {
            profile.low_pass_hz.min(MODEL_POLE_HZ)
        } else {
            MODEL_POLE_HZ
        };
        let poles = [
            (-2.0 * PI * pole_hz / sample_rate).exp(),
            (-2.0 * PI * MODEL_POLE_HZ / sample_rate).exp(),
        ];
        let weights = KERNEL_RESPONSE.get_or_init(kernel_response);
        let mut a = vec![0.0; 2 * FIT_POINTS * BAND_TAPS];
        let mut b = vec![0.0; 2 * FIT_POINTS];
        for (i, &kernel) in weights.iter().enumerate() {
            let nu = 0.5 * i as f64 / (FIT_POINTS - 1) as f64;
            let weight = kernel + 1e-9;
            let omega = 2.0 * PI * nu;
            let delay = Complex((omega * BAND_DELAY as f64).cos(), -(omega * BAND_DELAY as f64).sin());
            let target = model_response(pole_hz, profile.low_pass_zero_hz, nu * sample_rate)
                .mul(delay);
            let back = Complex(omega.cos(), -omega.sin());
            let mut recursion = Complex(1.0, 0.0);
            for pole in poles {
                recursion = recursion.div(Complex(1.0 - pole * back.0, -pole * back.1));
            }
            let mut basis = recursion;
            for tap in 0..BAND_TAPS {
                a[i * BAND_TAPS + tap] = basis.0 * weight;
                a[(FIT_POINTS + i) * BAND_TAPS + tap] = basis.1 * weight;
                basis = basis.mul(back);
            }
            b[i] = target.0 * weight;
            b[FIT_POINTS + i] = target.1 * weight;
        }
        let fitted = least_squares(a, b, BAND_TAPS);
        // Exactly unity at DC, as the model: a constant level keeps its value.
        let dc = fitted.iter().sum::<f64>() / ((1.0 - poles[0]) * (1.0 - poles[1]));
        let mut correction = [0.0; BAND_TAPS];
        for (slot, value) in correction.iter_mut().zip(&fitted) {
            *slot = value / dc;
        }
        Self {
            poles,
            pole_output: [0.0; 2],
            correction,
            history: [0.0; BAND_TAPS],
        }
    }

    fn process(&mut self, sample: f64) -> f64 {
        self.history.copy_within(0..BAND_TAPS - 1, 1);
        self.history[0] = sample;
        let mut value = self
            .correction
            .iter()
            .zip(&self.history)
            .fold(0.0, |sum, (c, x)| c.mul_add(*x, sum));
        for stage in 0..2 {
            value = self.poles[stage].mul_add(self.pole_output[stage], value);
            self.pole_output[stage] = value;
        }
        value
    }
}

/// The filters of a profile, run once per output sample after the resampler.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ProfileFilter {
    /// The low-pass and output pole (lot FIDPLUS-D1d).
    band: BandFilter,
    /// The expansion audio, delayed as much as the band filter delays the console's own mix, so
    /// that both stay aligned (newest first).
    expansion_delay: [f64; BAND_DELAY],
    /// One-pole RC high-passes, `a (y1 + x - x1)`, `BeesNES`'s formula.
    high_pass: [Option<f64>; 3],
    high_pass_input: [f64; 3],
    high_pass_output: [f64; 3],
    sign: f64,
}

impl ProfileFilter {
    /// A filter with no stage, to read a block into and drop.
    pub(crate) const fn at_rest() -> Self {
        Self {
            band: BandFilter {
                poles: [0.0; 2],
                pole_output: [0.0; 2],
                correction: [0.0; BAND_TAPS],
                history: [0.0; BAND_TAPS],
            },
            expansion_delay: [0.0; BAND_DELAY],
            high_pass: [None; 3],
            high_pass_input: [0.0; 3],
            high_pass_output: [0.0; 3],
            sign: 1.0,
        }
    }

    /// The filters of `profile` at `sample_rate` Hz, at rest.
    pub(crate) fn new(profile: &ConsoleAudioProfile, sample_rate: i32) -> Self {
        let rate = f64::from(sample_rate.max(1));
        let nyquist = rate * 0.5;
        let mut high_pass = [None; 3];
        for (slot, (&hz, &on)) in high_pass
            .iter_mut()
            .zip(profile.high_pass_hz.iter().zip(&profile.high_pass_on))
        {
            if on && hz > 0.0 && hz < nyquist {
                let delta = 1.0 / rate;
                let time_constant = 1.0 / hz;
                *slot = Some(time_constant / (time_constant + delta));
            }
        }
        Self {
            band: BandFilter::new(profile, rate),
            expansion_delay: [0.0; BAND_DELAY],
            high_pass,
            high_pass_input: [0.0; 3],
            high_pass_output: [0.0; 3],
            sign: if profile.inverted { -1.0 } else { 1.0 },
        }
    }

    /// Bytes [`Self::save_state`] writes.
    pub(crate) const STATE_SIZE: usize = 8 * (2 + BAND_TAPS + BAND_DELAY + 6);

    /// Appends the filters' memory (lots FIDPLUS-D1b and D1d, FIDELITY-LEADS E1-5): the band
    /// filter's recursions and inputs, the delayed expansion audio and the three high-passes'
    /// last input and output. The coefficients are the profile's and the sample rate's, settings
    /// rather than state.
    pub(crate) fn save_state(&self, writer: &mut crate::state::StateWriter) {
        for value in self.band.pole_output {
            writer.write_f64(value);
        }
        for value in self.band.history {
            writer.write_f64(value);
        }
        for value in self.expansion_delay {
            writer.write_f64(value);
        }
        for stage in 0..3 {
            writer.write_f64(self.high_pass_input[stage]);
            writer.write_f64(self.high_pass_output[stage]);
        }
    }

    /// Restores what [`Self::save_state`] wrote.
    ///
    /// # Errors
    /// [`crate::state::InvalidState`] when the block is truncated.
    pub(crate) fn load_state(
        &mut self,
        reader: &mut crate::state::StateReader,
    ) -> Result<(), crate::state::InvalidState> {
        for value in &mut self.band.pole_output {
            *value = reader.read_f64()?;
        }
        for value in &mut self.band.history {
            *value = reader.read_f64()?;
        }
        for value in &mut self.expansion_delay {
            *value = reader.read_f64()?;
        }
        for stage in 0..3 {
            self.high_pass_input[stage] = reader.read_f64()?;
            self.high_pass_output[stage] = reader.read_f64()?;
        }
        Ok(())
    }

    /// One output sample through the low-pass, the three high-passes and the sign.
    #[cfg(test)]
    pub(crate) fn process(&mut self, sample: f64) -> f64 {
        self.process_with_expansion(sample, 0.0)
    }

    /// One output sample: the console's own mix through the band filter and the first two
    /// high-passes, the cartridge's expansion audio added, then the third high-pass and the sign
    /// (`BeesNES`'s order: the mapper's `PostProcessAudioSample` between HPF1 and HPF2).
    pub(crate) fn process_with_expansion(&mut self, sample: f64, expansion: f64) -> f64 {
        let mut value = self.band.process(sample);
        let delayed_expansion = self.expansion_delay[BAND_DELAY - 1];
        self.expansion_delay.copy_within(0..BAND_DELAY - 1, 1);
        self.expansion_delay[0] = expansion;
        for stage in 0..3 {
            if stage == 2 {
                value += delayed_expansion;
            }
            if let Some(alpha) = self.high_pass[stage] {
                let output =
                    alpha * (self.high_pass_output[stage] + value - self.high_pass_input[stage]);
                self.high_pass_input[stage] = value;
                self.high_pass_output[stage] = output;
                value = output;
            }
        }
        value * self.sign
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_gains_match_the_documented_mix() {
        let unit = ConsoleAudioProfile {
            gains: [1.0; 5],
            ..CONSOLE_AUDIO_PROFILES[0]
        };
        // nesdev "APU Mixer": pulse_out = 95.88 / (8128 / 30 + 100) for both pulses at 15.
        let pulses = unit.mix([15, 15, 0, 0, 0]);
        assert!((pulses - (95.88 / ((8128.0 / 30.0) + 100.0))).abs() < 1e-12);
        assert!(unit.mix([0; 5]).abs() < 1e-15);
    }

    #[test]
    fn the_filters_block_dc_and_keep_the_sign() {
        let mut filter = ProfileFilter::new(&CONSOLE_AUDIO_PROFILES[1], 48_000);
        let mut last = 0.0;
        for _ in 0..48_000 {
            last = filter.process(0.5);
        }
        assert!(last.abs() < 1e-4, "a constant level dies out: {last}");
        let mut fresh = ProfileFilter::new(&CONSOLE_AUDIO_PROFILES[1], 48_000);
        assert!(fresh.process(0.5) < 0.0, "this console inverts its output");
    }

    /// Lot FIDPLUS-D1c (FIDELITY-LEADS E1-3): `BeesNES` runs HPF0 in its sample box, at the output
    /// rate right after the band-limiting sinc (`LSNSampleBox.h:641`), and its low-pass before
    /// the box; ours runs the low-pass first at the output rate, then HPF0. Both are linear and
    /// time-invariant, so the order does not change the signal: HPF0 then the low-pass gives the
    /// same samples, to rounding.
    #[test]
    fn fidplus_d1c_hpf0_order_against_the_low_pass_is_immaterial() {
        let profile = CONSOLE_AUDIO_PROFILES[0];
        let mut ours = ProfileFilter::new(&profile, 48_000);
        // A one-pole low-pass at the profile's corner (bilinear), as the chain had before D1d.
        let inverse_k = 1.0 / (PI * profile.low_pass_hz / 48_000.0).tan();
        let (b0, a1) = (1.0 / (1.0 + inverse_k), (1.0 - inverse_k) / (1.0 + inverse_k));
        let alpha = ours.high_pass[0].expect("HPF0 is on");
        let (mut ours_lp_in, mut ours_lp_out) = (0.0f64, 0.0f64);
        let (mut hp_in, mut hp_out, mut lp_in, mut lp_out) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
        let mut worst = 0.0f64;
        let mut state = 0x1234_5678u32;
        for step in 0..20_000u32 {
            // A square wave with a slow level drift and a little noise.
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            let square = if (step / 37) % 2 == 0 { 0.4 } else { 0.1 };
            let x = square + f64::from(step % 4000) * 1e-5 + f64::from(state % 100) * 1e-4;
            // Our order: low-pass, HPF0 (HPF1 and HPF2 left out of both sides).
            let lp = b0.mul_add(x + ours_lp_in, -(a1 * ours_lp_out));
            ours_lp_in = x;
            ours_lp_out = lp;
            let a = alpha * (ours.high_pass_output[0] + lp - ours.high_pass_input[0]);
            ours.high_pass_input[0] = lp;
            ours.high_pass_output[0] = a;
            // BeesNES's order: HPF0 after the low-pass's band-limiting, here simply first.
            let hp = alpha * (hp_out + x - hp_in);
            hp_in = x;
            hp_out = hp;
            let b = b0.mul_add(hp + lp_in, -(a1 * lp_out));
            lp_in = hp;
            lp_out = b;
            worst = worst.max((a - b).abs());
        }
        assert!(worst < 1e-12, "largest difference {worst}");
    }

    /// Lot FIDPLUS-D1d (FIDELITY-LEADS E1-1): the band filter's response against `BeesNES`'s
    /// model (low-pass with its zero, output pole at 22.15 kHz), delayed by `BAND_DELAY`, for every
    /// profile, where the resampler passes the signal; and exactly unity at DC.
    #[test]
    #[allow(clippy::cast_precision_loss)]
    fn fidplus_d1d_the_band_filter_matches_the_model_in_band() {
        let (mut worst_db, mut worst_degrees) = (0.0f64, 0.0f64);
        for rate in [44_100, 48_000] {
            for profile in &CONSOLE_AUDIO_PROFILES {
                let band = BandFilter::new(profile, f64::from(rate));
                let pole_hz = if profile.low_pass_hz > 0.0 {
                    profile.low_pass_hz.min(MODEL_POLE_HZ)
                } else {
                    MODEL_POLE_HZ
                };
                for i in 0..=140 {
                    let nu = 0.35 * f64::from(i) / 140.0;
                    let omega = 2.0 * PI * nu;
                    let mut sum = Complex(0.0, 0.0);
                    for (tap, c) in band.correction.iter().enumerate() {
                        let angle = omega * tap as f64;
                        sum = Complex(c.mul_add(angle.cos(), sum.0), c.mul_add(-angle.sin(), sum.1));
                    }
                    for pole in band.poles {
                        sum = sum.div(Complex(1.0 - pole * omega.cos(), pole * omega.sin()));
                    }
                    let delay = Complex(
                        (omega * BAND_DELAY as f64).cos(),
                        -(omega * BAND_DELAY as f64).sin(),
                    );
                    let model = model_response(pole_hz, profile.low_pass_zero_hz, nu * f64::from(rate))
                        .mul(delay);
                    let ratio = sum.div(model);
                    let magnitude = ratio.0.hypot(ratio.1);
                    worst_db = worst_db.max((20.0 * magnitude.log10()).abs());
                    worst_degrees = worst_degrees.max(ratio.1.atan2(ratio.0).to_degrees().abs());
                    if i == 0 {
                        assert!((sum.0 - 1.0).abs() < 1e-12 && sum.1.abs() < 1e-12, "DC gain");
                    }
                }
            }
        }
        assert!(worst_db < 0.002, "magnitude {worst_db} dB");
        assert!(worst_degrees < 0.2, "phase {worst_degrees} degrees");
    }
}
