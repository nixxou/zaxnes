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
//! * the low-pass runs at the output rate, not at the CPU rate as in `BeesNES` (`m_pfLpf`, a
//!   pole-zero filter clocked 1.79 million times a second): its pole is pre-warped to the output
//!   rate and its zero, above every output rate's Nyquist frequency, is dropped. Approximation;
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
    low_pass_hz: f64,
    high_pass_hz: [f64; 3],
    high_pass_on: [bool; 3],
    gains: [f64; 5],
    inverted: bool,
    letterless: bool,
) -> ConsoleAudioProfile {
    ConsoleAudioProfile {
        name,
        low_pass_hz,
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
    profile("NES Front-Loading N0260073", 18383.331849143484, [128.203310801000, 36.579855150267, 36.579848133916], ALL, [1.0, 1.0, 0.933, 0.744, 0.938], true, true),
    profile("NES Front-Loading N1151667", 16596.500112773516, [91.670577848853171, 37.490016890980982, 7.9671720814703768], ALL, [1.0, 1.0, 0.924, 0.883, 0.943], true, false),
    profile("NES Front-Loading N13721267", 17524.208150165094, [97.341173341196, 61.825863515323, 6.059893000666], ALL, [1.0, 0.994, 0.975, 0.952, 0.966], true, false),
    profile("NES Front-Loading N20914398, Sunsoft 5B", 0.0, [224.20062289111536, 125.06316358912643, 51.287540390847823], ALL, [1.0, 1.0, 0.924, 0.906, 1.0], false, false),
    profile("NES Front-Loading N32670105", 15851.963185973395, [99.407819166312, 7.934473497640, 7.934471615916], ALL, [1.0, 0.975, 0.952, 0.933, 0.975], true, false),
    profile("NES Front-Loading N34169630", 16879.431344679029, [307.866180500731, 77.224662166419, 9.329261475967], ALL, [0.975, 0.975, 0.975, 0.910, 1.0], true, false),
    profile("NES Front-Loading N8869725, NESRGB", 14482.132075089257, [83.459640336709199, 55.985366776125886, 8.5714225234124299], ALL, [1.0, 1.0, 0.966, 0.924, 1.0], true, false),
    profile("NES Front-Loading N8869725, NESRGB, stock RCA", 16003.975697663334, [96.678039344736646, 36.696956603852406, 8.3012408410527119], ALL, [1.0, 1.0, 0.892, 0.855, 0.924], true, false),
    profile("NES Top-Loading NN101255729, NESRGB", 14893.677244647428, [80.308463849114986, 52.357757591613563, 7.9206051188443904], ALL, [1.0, 1.0, 0.731, 0.693, 0.744], true, false),
    profile("Twin Famicom 441850, stock RCA", 3719.752299817048, [206.591994496089, 24.535811985325, 24.535802584134], ALL, [0.906, 0.883, 1.0, 0.929, 1.0], true, false),
    profile("Twin Famicom 475711, NESRGB, stock RCA", 6813.7368995, [202.359654532700, 34.967573649161, 34.967565533357], ALL, [0.938, 0.938, 1.0, 0.869, 0.929], true, false),
    profile("Twin Famicom 475711, NESRGB, SCART", 0.0, [302.96765178343128, 30.586358931812008, 4.0903076481735425], ALL, [0.975, 1.0, 0.841, 0.827, 0.896], true, false),
    profile("PAL NES NESE-001 (AUS) PM313006", 14912.561817246404, [99.713059858228, 64.057801563356, 8.189826234573], ALL, [0.882, 0.859, 0.814, 0.771, 0.808], true, false),
    profile("PAL NES NESE-001 (GBR) PM608755", 64000.0, [94.986785805568559, 8.851913371856071, 8.8518991011365298], ALL, [1.0, 1.0, 0.957, 1.0, 1.0], true, false),
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

/// The filters of a profile, run once per output sample after the resampler.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ProfileFilter {
    /// Pole-only low-pass, bilinear with pre-warping: `b0 (x + x1) - a1 y1`.
    low_pass: Option<(f64, f64)>,
    low_pass_input: f64,
    low_pass_output: f64,
    /// One-pole RC high-passes, `a (y1 + x - x1)`, `BeesNES`'s formula.
    high_pass: [Option<f64>; 3],
    high_pass_input: [f64; 3],
    high_pass_output: [f64; 3],
    sign: f64,
}

impl ProfileFilter {
    /// The filters of `profile` at `sample_rate` Hz, at rest.
    pub(crate) fn new(profile: &ConsoleAudioProfile, sample_rate: i32) -> Self {
        let rate = f64::from(sample_rate.max(1));
        let nyquist = rate * 0.5;
        let low_pass = (profile.low_pass_hz > 0.0 && profile.low_pass_hz < nyquist).then(|| {
            // BeesNES's `CPoleZeroFilter::CreatePoleZero` with no zero, at the output rate.
            let inverse_k = 1.0 / (PI * profile.low_pass_hz / rate).tan();
            let a0 = 1.0 + inverse_k;
            (1.0 / a0, (1.0 - inverse_k) / a0)
        });
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
            low_pass,
            low_pass_input: 0.0,
            low_pass_output: 0.0,
            high_pass,
            high_pass_input: [0.0; 3],
            high_pass_output: [0.0; 3],
            sign: if profile.inverted { -1.0 } else { 1.0 },
        }
    }

    /// One output sample through the low-pass, the three high-passes and the sign.
    pub(crate) fn process(&mut self, sample: f64) -> f64 {
        let mut value = sample;
        if let Some((b0, a1)) = self.low_pass {
            let output = b0.mul_add(value + self.low_pass_input, -(a1 * self.low_pass_output));
            self.low_pass_input = value;
            self.low_pass_output = output;
            value = output;
        }
        for stage in 0..3 {
            if let Some(alpha) = self.high_pass[stage] {
                let output = alpha * (self.high_pass_output[stage] + value - self.high_pass_input[stage]);
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
}
