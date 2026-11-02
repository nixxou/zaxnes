//! The mixer and the resampled output (REQ-CORE-073, REQ-CORE-070, REQ-CORE-004).

// Helpers unwrap like the tests do; casts are on small test values.
#![allow(
    clippy::unwrap_used,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::float_cmp
)]

mod common;

use std::collections::HashSet;
use std::f64::consts::PI;

use common::minimal;
use zax_nes::apu::mixer;
use zax_nes::{NesCore, RegionTiming};

fn core_at(sample_rate: i32) -> NesCore {
    let mut core = NesCore::new();
    assert!(core.load_rom(&minimal(), "Audio.nes").success);
    core.set_sample_rate(sample_rate).unwrap();
    core
}

fn core() -> NesCore {
    core_at(48_000)
}

fn start_pulse1(core: &mut NesCore, period: i32) {
    core.write_memory(0x4015, 0x01);
    core.write_memory(0x4000, 0xBF); // Duty 2 (50 %), length halted, constant volume 15.
    core.write_memory(0x4001, 0x08); // Negate with shift 0: the target never exceeds $7FF.
    core.write_memory(0x4002, period as u8);
    core.write_memory(0x4003, 0xF8 | ((period >> 8) & 0x07) as u8);
    core.write_memory(0x4017, 0x40); // Four-step, IRQ inhibited: nothing else perturbs it.
}

fn pulse_frequency(core: &NesCore, period: i32) -> f64 {
    let timing = RegionTiming::for_region(core.region());
    let cycles_per_frame = f64::from(RegionTiming::DOTS_PER_SCANLINE)
        * f64::from(timing.scanlines_per_frame)
        * f64::from(timing.dots_per_cycle_denominator)
        / f64::from(timing.dots_per_cycle_numerator);

    f64::from(timing.nominal_frames_per_second) * cycles_per_frame / (16.0 * f64::from(period + 1))
}

fn capture(core: &mut NesCore, count: usize, skip: usize) -> Vec<f32> {
    let mut samples = Vec::with_capacity(count + skip);
    let mut block = [0.0f32; 64];

    while samples.len() < count + skip {
        core.bus_mut().tick();
        if core.apu().available_samples() >= block.len() {
            core.read_audio(&mut block, false).unwrap();
            samples.extend_from_slice(&block);
        }
    }

    samples.into_iter().skip(skip).take(count).collect()
}

/// The highest `A` of REQ-CORE-073 seen over `cycles` CPU cycles, at the test point of
/// REQ-CORE-076.
fn peak_mixer_over(core: &mut NesCore, cycles: usize) -> i32 {
    let mut peak = 0;
    for _ in 0..cycles {
        core.bus_mut().tick();
        peak = peak.max(core.apu().mixer_apu());
    }
    peak
}

// ---------------------------------------------------------------- REQ-CORE-073

/// REQ-CORE-073 (AMD-054) — the four figures the requirement's first criterion names, read at
/// the test point of REQ-CORE-076: a lone pulse at volume 15 gives 4 890, the two pulses
/// 8 462 — a ratio of 1.73 rather than the 2 a linear law would give — the triangle alone
/// 8 363, and every channel at its maximum exactly 32 767.
#[test]
fn req_core_073_the_mixer_produces_the_documented_integers() {
    // A lone pulse at volume 15, 50 % duty.
    let mut one = core();
    start_pulse1(&mut one, 100);
    assert_eq!(4890, peak_mixer_over(&mut one, 4000));

    // Both pulses on the same period, in phase.
    let mut two = core();
    start_pulse1(&mut two, 100);
    two.write_memory(0x4015, 0x03);
    two.write_memory(0x4004, 0xBF);
    two.write_memory(0x4005, 0x08);
    two.write_memory(0x4006, 100);
    two.write_memory(0x4007, 0xF8);
    assert_eq!(8462, peak_mixer_over(&mut two, 4000));

    // The ratio the note of the requirement contrasts with the linear law's 2.
    assert!((8462.0_f64 / 4890.0 - 1.73).abs() < 0.005);

    // The triangle alone: its index runs 45 down to 0, so 8 363 is the top of the staircase
    // rather than a volume setting — the channel has none.
    let mut triangle = core();
    triangle.write_memory(0x4015, 0x04);
    triangle.write_memory(0x4008, 0x7F);
    triangle.write_memory(0x400A, 50);
    triangle.write_memory(0x400B, 0xF8);
    triangle.write_memory(0x4017, 0x80);
    assert_eq!(8363, peak_mixer_over(&mut triangle, 40 * 51));
    assert_eq!(8363, mixer::TND_TABLE[45]);

    // Every channel at its maximum: index 30 on one table, 3x15 + 2x15 + 127 = 202 on the
    // other. Both roundings land so that the two terms add up to full scale exactly.
    assert_eq!(mixer::FULL_SCALE, mixer::mix(15, 15, 15, 15, 127));
    assert_eq!(8462, mixer::PULSE_TABLE[30]);
    assert_eq!(24_305, mixer::TND_TABLE[202]);
}

/// REQ-CORE-073 (AMD-054) — the two tables are built from the real functions and scaled by `K`,
/// which is computed on the unrounded values; the rounding goes to the nearest integer and to
/// the even one on a tie, which the entry `K · T(196) = 23 905.4997` is near enough to be worth
/// pinning. The numerator is 95.88, the analytic one, and not the 95.52 of the same page's
/// lookup table, which would give 4 876 / 8 438.
#[test]
fn req_core_073_the_tables_follow_the_documented_formulas() {
    assert_eq!(31, mixer::PULSE_TABLE.len());
    assert_eq!(203, mixer::TND_TABLE.len());
    assert_eq!(0, mixer::PULSE_TABLE[0]);
    assert_eq!(0, mixer::TND_TABLE[0]);

    for i in 1..=30 {
        let expected = 95.88 / ((8128.0 / i as f64) + 100.0);
        assert!((mixer::pulse(i) - expected).abs() < 1e-12);
        assert_eq!(
            mixer::round_to_even(mixer::NORMALISATION * expected),
            mixer::PULSE_TABLE[i]
        );
    }
    for i in 1..=202 {
        let expected = 163.67 / ((24329.0 / i as f64) + 100.0);
        assert!((mixer::tnd(i) - expected).abs() < 1e-12);
        assert_eq!(
            mixer::round_to_even(mixer::NORMALISATION * expected),
            mixer::TND_TABLE[i]
        );
    }

    let k = 32_767.0 / (mixer::pulse(30) + mixer::tnd(202));
    assert!((mixer::NORMALISATION - k).abs() < 1e-9);
    assert!((mixer::NORMALISATION - 32_735.877_6).abs() < 1e-3);

    // Round half to even, as the requirement asks.
    assert_eq!(2, mixer::round_to_even(2.5));
    assert_eq!(2, mixer::round_to_even(1.5));
    assert_eq!(23_905, mixer::TND_TABLE[196]);
    let near_tie = mixer::NORMALISATION * mixer::tnd(196);
    assert!((23_905.499..23_905.5).contains(&near_tie), "{near_tie}");

    // The lookup-table numerator the requirement's note sets aside, with the K its own table
    // would give: 4 876 for a lone pulse instead of 4 890.
    let lookup_pulse = |i: usize| 95.52 / ((8128.0 / i as f64) + 100.0);
    let lookup_k = 32_767.0 / (lookup_pulse(30) + mixer::tnd(202));
    assert_eq!(4876, mixer::round_to_even(lookup_k * lookup_pulse(15)));
    assert_eq!(8438, mixer::round_to_even(lookup_k * lookup_pulse(30)));
}

/// REQ-CORE-073 (AMD-054) — the chain from the mixer's integer to the float the host reads:
/// `S = min(A + M, 32 767)`, then `S / 32 768`. Without expansion audio `M = 0` and `S = A`.
#[test]
fn req_core_073_the_integer_output_becomes_the_float_of_req_core_071() {
    let mut core = core();
    start_pulse1(&mut core, 100);

    for _ in 0..4000 {
        core.bus_mut().tick();
        let apu = core.apu();
        assert_eq!(0, apu.mixer_expansion());
        assert_eq!(apu.mixer_apu(), apu.mixer_output());
        assert!((f64::from(apu.mixer_output()) / 32_768.0 - apu.mixed_level()).abs() < 1e-12);
    }

    // The clipping of REQ-CORE-073 is a minimum, not a wrap; `A` on its own never reaches it,
    // the two roundings being chosen so that full scale is exactly 32 767.
    assert_eq!(mixer::FULL_SCALE, mixer::clip(mixer::FULL_SCALE, 5000));
    assert_eq!(mixer::FULL_SCALE, mixer::clip(30_000, 5000));
    assert_eq!(30_000, mixer::clip(30_000, 0));
}

/// REQ-CORE-076 — silence reads `A = M = S = 0` at the test point.
#[test]
fn req_core_076_silence_reads_zero_at_the_test_point() {
    let mut core = core();
    for _ in 0..200 {
        core.bus_mut().tick();
        let apu = core.apu();
        assert_eq!(0, apu.mixer_apu());
        assert_eq!(0, apu.mixer_expansion());
        assert_eq!(0, apu.mixer_output());
        assert_eq!(0.0, apu.mixed_level());
    }
}

fn median(mut values: Vec<f32>) -> f64 {
    values.sort_by(f32::total_cmp);
    f64::from(values[values.len() / 2])
}

#[test]
fn req_core_073_a_lone_pulse_at_volume_fifteen_swings_by_zero_point_one_four_nine() {
    let mut core = core();
    start_pulse1(&mut core, 111); // About 1 kHz: 24 output samples per half period.

    // The first samples carry the DC blocker's start-up transient.
    let samples = capture(&mut core, 8192, 8192);

    // The median of the samples on each side of the mean is the settled plateau, immune to
    // the ringing of a band-limited edge.
    let mean = samples.iter().map(|&s| f64::from(s)).sum::<f64>() / samples.len() as f64;
    let high = median(
        samples
            .iter()
            .copied()
            .filter(|&s| f64::from(s) > mean)
            .collect(),
    );
    let low = median(
        samples
            .iter()
            .copied()
            .filter(|&s| f64::from(s) < mean)
            .collect(),
    );

    // AMD-054: 0.149 ± 0.002 at the output of REQ-CORE-071, where the linear law of the
    // previous version of the requirement gave 0.172 ± 0.02 (the level drops by about 13 %).
    assert!((high - low - 0.149).abs() <= 0.002, "swing {}", high - low);
}

#[test]
fn req_core_073_a_constant_level_decays_to_zero_in_less_than_two_hundred_milliseconds() {
    let mut core = core();

    // A DMC level written through $4011 is a pure step: every channel then sits still.
    core.write_memory(0x4011, 100);

    let samples = capture(&mut core, 48_000 / 5, 0); // 200 ms at 48 kHz.

    assert!(
        samples[32].abs() > 0.1,
        "the step should be audible to begin with"
    );
    let last = samples[samples.len() - 1];
    assert!((-0.001..=0.001).contains(&last), "{last}");
}

#[test]
fn req_core_004_a_silent_apu_produces_digital_silence() {
    let mut core = core();

    let samples = capture(&mut core, 2048, 0);

    assert!(samples.iter().all(|&s| s == 0.0));
}

// ---------------------------------------------------------------- REQ-CORE-070

#[test]
fn req_core_070_resampling_a_pulse_leaves_no_aliasing_product_above_minus_sixty_dbfs() {
    // Pulse periods whose fundamental sits between 4 and 11 kHz, so that every odd harmonic
    // lands above Nyquist and would fold back if the resampling were not band-limited.
    const SAMPLE_RATE: i32 = 48_000;
    const SIZE: usize = 8192;

    for period in [10, 13, 17, 22, 27] {
        let mut core = core_at(SAMPLE_RATE);
        start_pulse1(&mut core, period);

        let samples = capture(&mut core, SIZE, 8192);
        let spectrum = hann_spectrum(&samples);

        let bin_hz = f64::from(SAMPLE_RATE) / SIZE as f64;
        let fundamental = pulse_frequency(&core, period);

        // Bins within eight of a harmonic below Nyquist hold signal, not aliasing.
        let mut signal_bins = HashSet::new();
        let mut harmonic = fundamental;
        while harmonic < f64::from(SAMPLE_RATE) / 2.0 {
            let centre = (harmonic / bin_hz).round_ties_even() as i64;
            for offset in -8..=8 {
                signal_bins.insert(centre + offset);
            }
            harmonic += 2.0 * fundamental;
        }

        // The lowest bins hold the DC blocker's own residue.
        for bin in 0..=8 {
            signal_bins.insert(bin);
        }

        let mut worst_bin = -1i64;
        let mut worst = 0.0f64;
        for (bin, &value) in spectrum.iter().enumerate() {
            if !signal_bins.contains(&(bin as i64)) && value > worst {
                worst = value;
                worst_bin = bin as i64;
            }
        }

        let worst_db = 20.0 * worst.max(1e-12).log10();

        assert!(
            worst_db <= -60.0,
            "period {period} ({fundamental:.0} Hz): aliasing product of {worst_db:.1} dBFS at {:.0} Hz",
            worst_bin as f64 * bin_hz
        );
    }
}

#[test]
fn req_core_070_the_passband_carries_the_fundamental_at_its_full_level() {
    // A 50 % square of 0.149 peak to peak (AMD-054; 0.172 under the linear law of the
    // previous version of REQ-CORE-073) carries 2h/pi at its fundamental; the passband runs
    // flat to roughly 0.31 of the output rate.
    for period in [111, 27, 10] {
        let mut core = core();
        start_pulse1(&mut core, period);

        let samples = capture(&mut core, 8192, 8192);

        let amplitude = amplitude_at(&samples, pulse_frequency(&core, period), 48_000);
        let expected = 2.0 * 0.149 / PI;

        assert!(
            amplitude >= expected * 0.95 && amplitude <= expected * 1.05,
            "period {period}: {amplitude} vs {expected}"
        );
    }
}

#[derive(Debug, Clone, Copy)]
struct Complex {
    re: f64,
    im: f64,
}

impl Complex {
    const fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    fn polar(angle: f64) -> Self {
        Self::new(angle.cos(), angle.sin())
    }

    fn mul(self, o: Self) -> Self {
        Self::new(
            self.re * o.re - self.im * o.im,
            self.re * o.im + self.im * o.re,
        )
    }

    fn add(self, o: Self) -> Self {
        Self::new(self.re + o.re, self.im + o.im)
    }

    fn sub(self, o: Self) -> Self {
        Self::new(self.re - o.re, self.im - o.im)
    }

    fn magnitude(self) -> f64 {
        self.re.hypot(self.im)
    }
}

fn hann(i: usize, size: usize) -> f64 {
    0.5 - (0.5 * (2.0 * PI * i as f64 / size as f64).cos())
}

fn amplitude_at(samples: &[f32], frequency: f64, sample_rate: i32) -> f64 {
    let size = samples.len();
    let mut sum = Complex::new(0.0, 0.0);

    for (i, &sample) in samples.iter().enumerate() {
        let angle = -2.0 * PI * frequency * i as f64 / f64::from(sample_rate);
        let weight = f64::from(sample) * hann(i, size);
        let rotated = Complex::polar(angle);
        sum = sum.add(Complex::new(weight * rotated.re, weight * rotated.im));
    }

    4.0 * sum.magnitude() / size as f64
}

fn hann_spectrum(samples: &[f32]) -> Vec<f64> {
    let size = samples.len();
    let mut buffer: Vec<Complex> = samples
        .iter()
        .enumerate()
        .map(|(i, &s)| Complex::new(f64::from(s) * hann(i, size), 0.0))
        .collect();

    fft(&mut buffer);

    // A Hann window halves the coherent gain, and a real signal's energy is split between
    // the positive and the negative frequency.
    (0..size / 2)
        .map(|bin| 4.0 * buffer[bin].magnitude() / size as f64)
        .collect()
}

fn fft(buffer: &mut [Complex]) {
    let size = buffer.len();

    let mut j = 0;
    for i in 1..size {
        let mut bit = size >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            buffer.swap(i, j);
        }
    }

    let mut length = 2;
    while length <= size {
        let step = Complex::polar(-2.0 * PI / length as f64);
        for start in (0..size).step_by(length) {
            let mut twiddle = Complex::new(1.0, 0.0);
            for k in 0..length / 2 {
                let even = buffer[start + k];
                let odd = buffer[start + k + length / 2].mul(twiddle);
                buffer[start + k] = even.add(odd);
                buffer[start + k + length / 2] = even.sub(odd);
                twiddle = twiddle.mul(step);
            }
        }
        length <<= 1;
    }
}
