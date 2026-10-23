use crate::state::{InvalidState, StateReader, StateWriter};
use std::f64::consts::PI;
use std::sync::OnceLock;

/// Cutoff of the DC-blocking high-pass, in Hz (REQ-CORE-073: 10 to 40 Hz).
pub const HIGH_PASS_HZ: f64 = 20.0;

/// Output samples the kernel spans.
const TAPS: usize = 32;

/// Taps to the left of the step's position.
const LEADING_TAPS: i64 = 15;

/// Sub-sample offsets the kernel is tabulated at.
const PHASES: usize = 128;

/// Bits of `fraction` that select the offset.
const PHASE_BITS: u32 = 7;

/// Cutoff frequency, as a fraction of the output rate.
const CUTOFF: f64 = 0.40;

/// Length of the accumulator ring; a power of two above [`TAPS`].
const RING_SIZE: usize = 64;

/// [`RING_SIZE`] - 1, as the type positions are counted in.
const RING_MASK: i64 = 63;

/// Safety cap on the ratio: half an output sample per source cycle. Reaching it would need
/// a speed factor below 0.06 (REQ-CORE-072), and the fixed-point step has to stay inside
/// 32 bits.
const MAX_RATIO: f64 = 0.5;

/// Integration sub-steps per kernel offset, for tabulating the step response.
const INTEGRATION_SUB_STEPS: usize = 8;

/// Kernel rows, `PHASES + 1` of them so that interpolation towards the next offset never
/// runs off the end.
static KERNEL: OnceLock<Box<[f64]>> = OnceLock::new();

/// Turns the APU's output, which changes at CPU-cycle resolution, into samples at the
/// host's output rate, without aliasing (REQ-CORE-070).
///
/// **Band-limited step synthesis.** The APU's output is piecewise constant, so for every
/// step (time, amplitude change) a band-limited impulse is added into an output-rate
/// accumulator at the step's exact fractional position, and the signal is read back as the
/// running sum of that accumulator: the running sum of a band-limited impulse is a
/// band-limited step. The cost is proportional to the number of steps, not to the source
/// rate.
///
/// **Kernel.** [`TAPS`] output samples of a Blackman-windowed sinc at [`PHASES`] sub-sample
/// offsets, linearly interpolated between neighbouring offsets, each row normalised to sum
/// to exactly 1. Cutoff 0.4 of the output rate.
///
/// Every arithmetic step is a fixed sequence of IEEE-754 double operations on integer
/// inputs, in the order of the C# reference, so two runs from the same state produce
/// bit-identical samples (REQ-CORE-092).
#[derive(Debug, Clone)]
pub(crate) struct BandLimitedResampler {
    kernel: &'static [f64],
    ring: [f64; RING_SIZE],
    /// Fractional part of the output position, as a 0.32 fixed-point value.
    fraction: u32,
    /// Output sample the source is currently at. Starts past the kernel's lead-in.
    position: i64,
    /// Next output sample to emit.
    emitted: i64,
    /// Step added to `fraction` per source cycle: output samples per CPU cycle.
    step: u32,
    /// Running sum of `ring`: the signal itself.
    signal: f64,
    /// Last level a step was emitted for.
    level: f64,
    high_pass_coefficient: f64,
    high_pass_previous: f64,
    high_pass_output: f64,
}

impl BandLimitedResampler {
    /// Creates a resampler at silence, with no ratio set.
    pub fn new() -> Self {
        Self {
            kernel: KERNEL.get_or_init(build_kernel),
            ring: [0.0; RING_SIZE],
            fraction: 0,
            position: LEADING_TAPS + 1,
            emitted: 1,
            step: 0,
            signal: 0.0,
            level: 0.0,
            high_pass_coefficient: 0.9974,
            high_pass_previous: 0.0,
            high_pass_output: 0.0,
        }
    }

    /// Sets the resampling ratio: output samples per source (CPU) cycle, and the output
    /// rate in Hz for the high-pass coefficient.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // In [0, 2^31].
    pub fn set_ratio(&mut self, samples_per_source_cycle: f64, sample_rate: i32) {
        // C# Math.Round rounds half to even.
        self.step =
            (samples_per_source_cycle.min(MAX_RATIO) * 4_294_967_296.0).round_ties_even() as u32;
        self.high_pass_coefficient = (-2.0 * PI * HIGH_PASS_HZ / f64::from(sample_rate)).exp();
    }

    /// Advances the source clock by one CPU cycle, having been handed the output level for
    /// that cycle; every finished sample goes to `sink`.
    #[allow(clippy::float_cmp)] // Exact comparison, as in the C#: any change is a step.
    pub fn step_cycle(&mut self, level: f64, sink: &mut impl FnMut(f32)) {
        if level != self.level {
            self.add_step(level - self.level);
            self.level = level;
        }

        let (fraction, carry) = self.fraction.overflowing_add(self.step);
        self.fraction = fraction;

        if carry {
            self.position += 1;
            self.drain(sink);
        }
    }

    /// Back to silence: no pending step, no pending sample (REQ-CORE-074).
    pub fn reset(&mut self) {
        self.ring = [0.0; RING_SIZE];
        self.fraction = 0;
        self.position = LEADING_TAPS + 1;
        self.emitted = 1;
        self.signal = 0.0;
        self.level = 0.0;
        self.high_pass_previous = 0.0;
        self.high_pass_output = 0.0;
    }

    /// Adds a band-limited step of `delta` at the current fractional position.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // Masked ring index.
    fn add_step(&mut self, delta: f64) {
        let phase = (self.fraction >> (32 - PHASE_BITS)) as usize;
        let interpolation = f64::from((self.fraction >> (32 - PHASE_BITS - 16)) & 0xFFFF) / 65536.0;

        let low = phase * TAPS;
        let high = low + TAPS;
        let start = ((self.position - LEADING_TAPS) & (RING_MASK)) as usize;

        for tap in 0..TAPS {
            let weight = self.kernel[low + tap]
                + ((self.kernel[high + tap] - self.kernel[low + tap]) * interpolation);
            self.ring[(start + tap) & (RING_SIZE - 1)] += weight * delta;
        }
    }

    /// Emits every output sample no future step can still reach. A step at source position
    /// `p` writes to output samples `p - 15` to `p + 16`, and the position only moves
    /// forward, so everything below `position - 15` is final.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // Masked ring index.
    fn drain(&mut self, sink: &mut impl FnMut(f32)) {
        while self.emitted < self.position - LEADING_TAPS {
            let slot = (self.emitted & (RING_MASK)) as usize;
            self.signal += self.ring[slot];
            self.ring[slot] = 0.0;
            self.emitted += 1;

            // One-pole DC blocker (REQ-CORE-073). The level the mixer produces is
            // non-negative, so without it every sample would sit on a large offset.
            self.high_pass_output = self.signal - self.high_pass_previous
                + (self.high_pass_coefficient * self.high_pass_output);
            self.high_pass_previous = self.signal;

            sink(self.high_pass_output.clamp(-1.0, 1.0) as f32);
        }
    }
}

/// Builds the kernel rows as successive differences of the band-limited step response.
///
/// Tabulating `S(x) - S(x - 1)`, where `S` is the integral of the windowed sinc, rather
/// than the sinc itself, cancels the treble boost the running-sum read-out would otherwise
/// add, and makes each row sum to 1 on its own.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)] // Small table indices and counts, all exact in every type involved.
fn build_kernel() -> Box<[f64]> {
    // S tabulated from -17 to +17 output samples, at the kernel's offset resolution.
    const SPAN: usize = 2 * (TAPS + 1);
    const ORIGIN: i32 = ((TAPS + 1) * PHASES) as i32;
    let mut step = vec![0.0f64; (SPAN * PHASES) + 1];

    let mut accumulated = 0.0f64;
    let dx = 1.0 / (PHASES * INTEGRATION_SUB_STEPS) as f64;

    for (i, slot) in step.iter_mut().enumerate().skip(1) {
        // Trapezoidal integration of the windowed sinc over one offset's worth of x.
        let from = f64::from(i as i32 - 1 - ORIGIN) / PHASES as f64;
        for sub in 0..INTEGRATION_SUB_STEPS {
            let a = from + (sub as f64 * dx);
            accumulated += (impulse(a) + impulse(a + dx)) * 0.5 * dx;
        }

        *slot = accumulated;
    }

    let mut kernel = vec![0.0f64; (PHASES + 1) * TAPS];

    for phase in 0..=PHASES {
        let row = phase * TAPS;
        let mut sum = 0.0;

        for tap in 0..TAPS {
            // x = tap - LEADING_TAPS - phase / PHASES, as an exact index into the table.
            let index = (((tap as i32 - LEADING_TAPS as i32) * PHASES as i32) - phase as i32
                + ORIGIN) as usize;
            let value = step[index] - step[index - PHASES];

            kernel[row + tap] = value;
            sum += value;
        }

        for tap in 0..TAPS {
            kernel[row + tap] /= sum;
        }
    }

    kernel.into_boxed_slice()
}

/// The windowed sinc the step response integrates, zero outside the window.
#[allow(clippy::cast_precision_loss)] // TAPS is 32.
fn impulse(x: f64) -> f64 {
    if x.abs() >= TAPS as f64 / 2.0 {
        0.0
    } else {
        2.0 * CUTOFF * sinc(2.0 * CUTOFF * x) * blackman(x / TAPS as f64)
    }
}

#[allow(clippy::float_cmp)] // The removable singularity is exactly at 0.
fn sinc(x: f64) -> f64 {
    if x == 0.0 {
        1.0
    } else {
        (PI * x).sin() / (PI * x)
    }
}

/// Blackman window, `x` running over [-0.5, 0.5].
fn blackman(x: f64) -> f64 {
    let t = x + 0.5;
    0.42 - (0.5 * (2.0 * PI * t).cos()) + (0.08 * (4.0 * PI * t).cos())
}

impl BandLimitedResampler {
    /// Appends the state to `writer` (REQ-CORE-090).
    pub fn save_state(&self, writer: &mut StateWriter) {
        for value in self.ring {
            writer.write_f64(value);
        }

        writer.write_u32(self.fraction);
        writer.write_i64(self.position);
        writer.write_i64(self.emitted);
        writer.write_f64(self.signal);
        writer.write_f64(self.level);
        writer.write_f64(self.high_pass_previous);
        writer.write_f64(self.high_pass_output);
    }

    /// Restores the state from `reader` (REQ-CORE-091).
    ///
    /// # Errors
    /// [`InvalidState`] when the block is truncated or holds an impossible value.
    pub fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        for value in &mut self.ring {
            *value = reader.read_f64()?;
        }

        self.fraction = reader.read_u32()?;
        self.position = reader.read_i64()?;
        self.emitted = reader.read_i64()?;
        self.signal = reader.read_f64()?;
        self.level = reader.read_f64()?;
        self.high_pass_previous = reader.read_f64()?;
        self.high_pass_output = reader.read_f64()?;
        Ok(())
    }
}

impl BandLimitedResampler {
    /// Bytes a saved resampler state occupies, for the APU's `MAX_STATE_SIZE`.
    pub const STATE_SIZE: usize = RING_SIZE * 8 + 4 + 8 + 8 + 4 * 8;
}
