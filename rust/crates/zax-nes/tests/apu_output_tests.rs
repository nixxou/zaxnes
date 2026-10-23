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
use zax_nes::{Machine, RegionTiming};

fn core_at(sample_rate: i32) -> Machine {
    let mut core = Machine::new();
    assert!(core.load_rom(&minimal(), "Audio.nes").success);
    core.set_sample_rate(sample_rate).unwrap();
    core
}

fn core() -> Machine {
    core_at(48_000)
}

fn level(core: &Machine) -> f64 {
    core.apu().mixed_level()
}

fn start_pulse1(core: &mut Machine, period: i32) {
    core.write_memory(0x4015, 0x01);
    core.write_memory(0x4000, 0xBF); // Duty 2 (50 %), length halted, constant volume 15.
    core.write_memory(0x4001, 0x08); // Negate with shift 0: the target never exceeds $7FF.
    core.write_memory(0x4002, period as u8);
    core.write_memory(0x4003, 0xF8 | ((period >> 8) & 0x07) as u8);
    core.write_memory(0x4017, 0x40); // Four-step, IRQ inhibited: nothing else perturbs it.
}

fn pulse_frequency(core: &Machine, period: i32) -> f64 {
    let timing = RegionTiming::for_region(core.region());
    let cycles_per_frame = f64::from(RegionTiming::DOTS_PER_SCANLINE)
        * f64::from(timing.scanlines_per_frame)
        * f64::from(timing.dots_per_cycle_denominator)
        / f64::from(timing.dots_per_cycle_numerator);

    f64::from(timing.nominal_frames_per_second) * cycles_per_frame / (16.0 * f64::from(period + 1))
}

fn capture(core: &mut Machine, count: usize, skip: usize) -> Vec<f32> {
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

fn assert_near(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1e-9,
        "{actual} is not within 1e-9 of {expected}"
    );
}

fn peak_over(core: &mut Machine, cycles: usize) -> f64 {
    let mut peak = f64::NEG_INFINITY;
    for _ in 0..cycles {
        core.bus_mut().tick();
        peak = peak.max(level(core));
    }
    peak
}

// ---------------------------------------------------------------- REQ-CORE-073

#[test]
fn req_core_073_each_channel_contributes_its_documented_step() {
    // The linear approximation of nesdev's "APU Mixer", scaled by 50 000 / 32 768. One
    // channel at a time, so each step value is read on its own.
    for (control, value, enable, expected_mix) in [
        (0x4000u16, 0x9Fu8, 0x01u8, 15.0 * 0.00752), // Pulse 1 at volume 15.
        (0x4004, 0x9F, 0x02, 15.0 * 0.00752),        // Pulse 2 at volume 15.
        (0x400C, 0x1F, 0x08, 15.0 * 0.00494),        // Noise at volume 15.
    ] {
        let mut core = core();

        core.write_memory(0x4015, enable);
        core.write_memory(control, value);
        core.write_memory(control + 1, 0x08);
        core.write_memory(control + 2, 100);
        core.write_memory(control + 3, 0xF8);

        assert_near(peak_over(&mut core, 4000), 1.526 * expected_mix);
    }
}

#[test]
fn req_core_073_the_triangle_and_the_dmc_contribute_their_documented_steps() {
    let mut core = core();

    // Triangle on its own, at the top of the staircase.
    core.write_memory(0x4015, 0x04);
    core.write_memory(0x4008, 0x7F);
    core.write_memory(0x400A, 50);
    core.write_memory(0x400B, 0xF8);
    core.write_memory(0x4017, 0x80);

    assert_near(peak_over(&mut core, 40 * 51), 1.526 * 15.0 * 0.00851);

    // DMC on its own, level loaded straight through $4011.
    let mut dmc = core_at(48_000);
    dmc.write_memory(0x4011, 127);
    dmc.bus_mut().tick();

    assert_near(level(&dmc), 1.526 * 127.0 * 0.00335);
}

#[test]
fn req_core_073_the_mix_is_the_sum_of_the_channels_and_is_clamped_to_one() {
    let mut core = core();

    // Both pulses at volume 15 plus a DMC level of 127: 0.2256 + 0.42545, under the clamp.
    core.write_memory(0x4015, 0x03);
    core.write_memory(0x4000, 0x9F);
    core.write_memory(0x4001, 0x08);
    core.write_memory(0x4002, 100);
    core.write_memory(0x4003, 0xF8);
    core.write_memory(0x4004, 0x9F);
    core.write_memory(0x4005, 0x08);
    core.write_memory(0x4006, 100);
    core.write_memory(0x4007, 0xF8);
    core.write_memory(0x4011, 127);

    let expected = 1.526 * ((2.0 * 15.0 * 0.00752) + (127.0 * 0.00335));
    assert_near(peak_over(&mut core, 4000), expected);
    assert!(expected < 1.0);

    // Adding the triangle at the top of its staircase pushes the sum past full scale, where
    // REQ-CORE-073 asks for a clamp rather than a wrap.
    core.write_memory(0x4015, 0x07);
    core.write_memory(0x4008, 0x7F);
    core.write_memory(0x400A, 50);
    core.write_memory(0x400B, 0xF8);
    core.write_memory(0x4017, 0x80);

    assert_eq!(1.0, peak_over(&mut core, 40 * 51));
}

fn median(mut values: Vec<f32>) -> f64 {
    values.sort_by(f32::total_cmp);
    f64::from(values[values.len() / 2])
}

#[test]
fn req_core_073_a_lone_pulse_at_volume_fifteen_swings_by_zero_point_one_seven_two() {
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

    assert!((high - low - 0.172).abs() <= 0.02, "swing {}", high - low);
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
    // A 50 % square of 0.172 peak to peak carries 2h/pi at its fundamental; the passband
    // runs flat to roughly 0.31 of the output rate.
    for period in [111, 27, 10] {
        let mut core = core();
        start_pulse1(&mut core, period);

        let samples = capture(&mut core, 8192, 8192);

        let amplitude = amplitude_at(&samples, pulse_frequency(&core, period), 48_000);
        let expected = 2.0 * 0.172 / PI;

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
