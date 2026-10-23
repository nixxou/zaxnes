//! The 2A03 APU (port of `src/Zax.Nes/Apu`): five sound channels, the frame counter and its
//! IRQ, the $4015 status register, the mixer and the resampled output buffer
//! (REQ-CORE-070 to REQ-CORE-074, REQ-CORE-113).
//!
//! **Register writes land after the cycle.** The bus advances the whole machine by one
//! cycle and only then performs the register access, so a write always takes effect after
//! that cycle's frame-counter events. That ordering is what the hardware does and what
//! blargg's `10.len_halt_timing` and `11.len_reload_timing` measure.
//!
//! **Expansion audio** is not emulated: REQ-CORE-075 makes it optional and DEC-02-Q5
//! settles that it is not required.

mod band_limited_resampler;
mod dmc_channel;
mod envelope;
mod length_counter;
mod noise_channel;
mod pulse_channel;
mod triangle_channel;

use crate::state::{InvalidState, StateReader, StateWriter};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::RegionTiming;
use band_limited_resampler::BandLimitedResampler;
use dmc_channel::DmcChannel;
use noise_channel::NoiseChannel;
use pulse_channel::PulseChannel;
use triangle_channel::TriangleChannel;

pub use band_limited_resampler::HIGH_PASS_HZ;

/// A rejected audio setting or read (the C# throws `ArgumentException` for these).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AudioError {
    /// The sample rate is outside [`Apu2A03::MIN_SAMPLE_RATE`, `Apu2A03::MAX_SAMPLE_RATE`].
    SampleRateOutOfRange(i32),
    /// The speed factor is zero or negative.
    SpeedNotPositive(f64),
    /// The fine ratio adjustment exceeds [`Apu2A03::MAX_RATIO_ADJUSTMENT`].
    RatioAdjustmentOutOfRange(f64),
    /// A stereo buffer must hold an even number of values.
    OddStereoBuffer(usize),
}

impl std::fmt::Display for AudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SampleRateOutOfRange(rate) => write!(
                f,
                "sample rate {rate} is outside {}..={}",
                Apu2A03::MIN_SAMPLE_RATE,
                Apu2A03::MAX_SAMPLE_RATE
            ),
            Self::SpeedNotPositive(speed) => write!(f, "speed {speed} must be positive"),
            Self::RatioAdjustmentOutOfRange(value) => write!(
                f,
                "ratio adjustment {value} exceeds ±{}",
                Apu2A03::MAX_RATIO_ADJUSTMENT
            ),
            Self::OddStereoBuffer(len) => {
                write!(
                    f,
                    "a stereo buffer must hold an even number of values, not {len}"
                )
            }
        }
    }
}

impl std::error::Error for AudioError {}

/// Four-step frame sequencer, NTSC, in CPU cycles after the sequencer reset (nesdev, "APU
/// Frame Counter"). The last entry closes the sequence and restarts it.
const NTSC_FOUR_STEP: [i32; 6] = [7457, 14913, 22371, 29828, 29829, 29830];

/// Five-step frame sequencer, NTSC CPU cycles.
const NTSC_FIVE_STEP: [i32; 6] = [7457, 14913, 22371, 29829, 37281, 37282];

/// Four-step frame sequencer, PAL CPU cycles.
const PAL_FOUR_STEP: [i32; 6] = [8313, 16627, 24939, 33250, 33251, 33252];

/// Five-step frame sequencer, PAL CPU cycles.
const PAL_FIVE_STEP: [i32; 6] = [8313, 16627, 24939, 33253, 41565, 41566];

/// The output ring buffer, shared with [`AudioReader`]s on other threads.
#[derive(Debug, Clone)]
struct SampleRing {
    buffer: Vec<f32>,
    read_index: usize,
    write_index: usize,
    available: usize,
}

impl SampleRing {
    fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![0.0; capacity],
            read_index: 0,
            write_index: 0,
            available: 0,
        }
    }

    fn clear(&mut self) {
        self.read_index = 0;
        self.write_index = 0;
        self.available = 0;
    }

    fn enqueue(&mut self, sample: f32) {
        let capacity = self.buffer.len();
        if self.available == capacity {
            // On overflow the oldest samples are dropped (REQ-CORE-071,
            // SPEC-11 REQ-AUD-005).
            self.read_index = (self.read_index + 1) % capacity;
            self.available -= 1;
        }

        self.buffer[self.write_index] = sample;
        self.write_index = (self.write_index + 1) % capacity;
        self.available += 1;
    }

    fn read(&mut self, destination: &mut [f32], stereo: bool) -> Result<usize, AudioError> {
        if destination.is_empty() {
            return Ok(0);
        }

        if stereo && destination.len() % 2 != 0 {
            return Err(AudioError::OddStereoBuffer(destination.len()));
        }

        let needed = if stereo {
            destination.len() / 2
        } else {
            destination.len()
        };

        if self.available < needed {
            return Ok(0);
        }

        let capacity = self.buffer.len();
        for i in 0..needed {
            let sample = self.buffer[self.read_index];
            self.read_index = (self.read_index + 1) % capacity;

            if stereo {
                destination[2 * i] = sample;
                destination[(2 * i) + 1] = sample;
            } else {
                destination[i] = sample;
            }
        }

        self.available -= needed;
        Ok(destination.len())
    }
}

fn lock(ring: &Mutex<SampleRing>) -> MutexGuard<'_, SampleRing> {
    // The ring holds plain numbers: a panic elsewhere cannot leave it inconsistent.
    ring.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A handle on the APU's output buffer that another thread (the host's audio callback) can
/// read from while frames run (REQ-CORE-071).
#[derive(Debug, Clone)]
pub struct AudioReader {
    ring: Arc<Mutex<SampleRing>>,
}

impl AudioReader {
    /// Same as [`Apu2A03::read_samples`].
    ///
    /// # Errors
    /// [`AudioError::OddStereoBuffer`] when `stereo` and the length is odd.
    pub fn read_samples(&self, destination: &mut [f32], stereo: bool) -> Result<usize, AudioError> {
        lock(&self.ring).read(destination, stereo)
    }

    /// Mono samples waiting to be read.
    #[must_use]
    pub fn available_samples(&self) -> usize {
        lock(&self.ring).available
    }
}

/// The audio processing unit.
#[derive(Debug)]
#[allow(clippy::struct_excessive_bools)] // Mirrors the C# class: independent hardware flags.
pub struct Apu2A03 {
    pulse1: PulseChannel,
    pulse2: PulseChannel,
    triangle: TriangleChannel,
    noise: NoiseChannel,
    dmc: DmcChannel,
    resampler: BandLimitedResampler,
    ring: Arc<Mutex<SampleRing>>,

    timing: RegionTiming,
    sample_rate: i32,
    speed: f64,
    ratio_adjustment: f64,

    cycles: i64,
    apu_cycle: bool,
    frame_counter: i32,
    frame_step: usize,
    five_step_mode: bool,
    irq_inhibited: bool,
    frame_irq_flag: bool,
    frame_irq_pending: bool,
    pending_sequencer_reset: i32,
    readable_status: u8,
    level_key: i32,
    level: f64,
}

impl Clone for Apu2A03 {
    /// A deep copy: the clone gets its own output buffer.
    fn clone(&self) -> Self {
        let ring = lock(&self.ring).clone();
        Self {
            pulse1: self.pulse1.clone(),
            pulse2: self.pulse2.clone(),
            triangle: self.triangle.clone(),
            noise: self.noise.clone(),
            dmc: self.dmc.clone(),
            resampler: self.resampler.clone(),
            ring: Arc::new(Mutex::new(ring)),
            timing: self.timing,
            sample_rate: self.sample_rate,
            speed: self.speed,
            ratio_adjustment: self.ratio_adjustment,
            cycles: self.cycles,
            apu_cycle: self.apu_cycle,
            frame_counter: self.frame_counter,
            frame_step: self.frame_step,
            five_step_mode: self.five_step_mode,
            irq_inhibited: self.irq_inhibited,
            frame_irq_flag: self.frame_irq_flag,
            frame_irq_pending: self.frame_irq_pending,
            pending_sequencer_reset: self.pending_sequencer_reset,
            readable_status: self.readable_status,
            level_key: self.level_key,
            level: self.level,
        }
    }
}

impl Default for Apu2A03 {
    fn default() -> Self {
        Self::new()
    }
}

impl Apu2A03 {
    /// Lowest output sample rate the core accepts (REQ-CORE-070).
    pub const MIN_SAMPLE_RATE: i32 = 22_050;
    /// Highest output sample rate the core accepts (REQ-CORE-070).
    pub const MAX_SAMPLE_RATE: i32 = 96_000;
    /// Default output sample rate (REQ-CORE-070, SPEC-11 REQ-AUD-001).
    pub const DEFAULT_SAMPLE_RATE: i32 = 48_000;
    /// Minimum capacity of the output buffer, in mono samples (REQ-CORE-071).
    pub const MIN_BUFFER_SAMPLES: usize = 10_000;
    /// Largest fine adjustment of the resampling ratio a frontend may ask for (REQ-CORE-070).
    pub const MAX_RATIO_ADJUSTMENT: f64 = 0.005;
    /// Volume one step of a pulse channel contributes (REQ-CORE-073).
    pub const PULSE_STEP: f64 = 0.00752;
    /// Volume one step of the triangle contributes (REQ-CORE-073).
    pub const TRIANGLE_STEP: f64 = 0.00851;
    /// Volume one step of the noise contributes (REQ-CORE-073).
    pub const NOISE_STEP: f64 = 0.00494;
    /// Volume one step of the DMC contributes (REQ-CORE-073).
    pub const DMC_STEP: f64 = 0.00335;
    /// Output gain applied to the mix, 50 000 / 32 768 (REQ-CORE-073).
    pub const OUTPUT_GAIN: f64 = 1.526;

    /// Creates an APU with the default region and sample rate.
    #[must_use]
    pub fn new() -> Self {
        let mut apu = Self {
            pulse1: PulseChannel::new(true),
            pulse2: PulseChannel::new(false),
            triangle: TriangleChannel::default(),
            noise: NoiseChannel::default(),
            dmc: DmcChannel::default(),
            resampler: BandLimitedResampler::new(),
            ring: Arc::new(Mutex::new(SampleRing::new(Self::MIN_BUFFER_SAMPLES))),
            timing: RegionTiming::default(),
            sample_rate: Self::DEFAULT_SAMPLE_RATE,
            speed: 1.0,
            ratio_adjustment: 0.0,
            cycles: 0,
            apu_cycle: false,
            frame_counter: 0,
            frame_step: 0,
            five_step_mode: false,
            irq_inhibited: false,
            frame_irq_flag: false,
            frame_irq_pending: false,
            pending_sequencer_reset: 0,
            readable_status: 0,
            level_key: -1,
            level: 0.0,
        };
        apu.apply_timing();
        apu.recompute_ratio();
        apu
    }

    /// Whether the APU is pulling /IRQ low: frame counter IRQ or DMC IRQ.
    #[must_use]
    pub const fn irq_asserted(&self) -> bool {
        self.frame_irq_flag || self.dmc.irq_flag()
    }

    /// The mixed output level of the five channels on the cycle just stepped, in [-1, 1]
    /// (REQ-CORE-073), before resampling and before the DC blocker.
    #[must_use]
    pub const fn mixed_level(&self) -> f64 {
        self.level
    }

    /// Whether the DMC's memory reader wants a byte (nesdev, "DMA").
    #[must_use]
    pub const fn needs_dmc_dma(&self) -> bool {
        self.dmc.needs_sample_fetch()
    }

    /// Address the next DMC DMA reads.
    #[must_use]
    pub const fn dmc_dma_address(&self) -> u16 {
        self.dmc.fetch_address()
    }

    /// Hands the byte a DMC DMA fetched to the memory reader.
    pub fn supply_dmc_byte(&mut self, value: u8) {
        self.dmc.supply_byte(value);
    }

    /// Mono samples waiting to be read.
    #[must_use]
    pub fn available_samples(&self) -> usize {
        lock(&self.ring).available
    }

    /// Capacity of the output buffer, in mono samples.
    #[must_use]
    pub fn buffer_capacity(&self) -> usize {
        lock(&self.ring).buffer.len()
    }

    /// A handle another thread can read the output from (REQ-CORE-071).
    #[must_use]
    pub fn audio_reader(&self) -> AudioReader {
        AudioReader {
            ring: Arc::clone(&self.ring),
        }
    }

    /// Output sample rate in Hz (REQ-CORE-070).
    #[must_use]
    pub const fn sample_rate(&self) -> i32 {
        self.sample_rate
    }

    /// Sets the output sample rate (REQ-CORE-070).
    ///
    /// # Errors
    /// [`AudioError::SampleRateOutOfRange`] outside 22 050 to 96 000 Hz.
    #[allow(clippy::cast_sign_loss)] // Range-checked above.
    pub fn set_sample_rate(&mut self, value: i32) -> Result<(), AudioError> {
        if !(Self::MIN_SAMPLE_RATE..=Self::MAX_SAMPLE_RATE).contains(&value) {
            return Err(AudioError::SampleRateOutOfRange(value));
        }
        self.sample_rate = value;

        // At least the 10 000 samples REQ-CORE-071 demands, and otherwise 200 ms, which is
        // the upper end of the range SPEC-11 REQ-AUD-005 recommends.
        let capacity = Self::MIN_BUFFER_SAMPLES.max((value / 5) as usize);
        {
            let mut ring = lock(&self.ring);
            if capacity != ring.buffer.len() {
                *ring = SampleRing::new(capacity);
            }
        }

        self.recompute_ratio();
        Ok(())
    }

    /// Speed factor: only the audio pacing changes, never the emulation (REQ-CORE-072).
    #[must_use]
    pub const fn speed(&self) -> f64 {
        self.speed
    }

    /// Sets the speed factor (REQ-CORE-072).
    ///
    /// # Errors
    /// [`AudioError::SpeedNotPositive`] for zero or less.
    pub fn set_speed(&mut self, value: f64) -> Result<(), AudioError> {
        if value.is_nan() || value <= 0.0 {
            return Err(AudioError::SpeedNotPositive(value));
        }
        self.speed = value;
        self.recompute_ratio();
        Ok(())
    }

    /// Fine adjustment of the resampling ratio, at most ±0.5 % (REQ-CORE-070, SPEC-11
    /// REQ-AUD-005). It never alters the emulation.
    #[must_use]
    pub const fn ratio_adjustment(&self) -> f64 {
        self.ratio_adjustment
    }

    /// Sets the fine ratio adjustment.
    ///
    /// # Errors
    /// [`AudioError::RatioAdjustmentOutOfRange`] beyond ±0.5 %.
    pub fn set_ratio_adjustment(&mut self, value: f64) -> Result<(), AudioError> {
        if value.abs() > Self::MAX_RATIO_ADJUSTMENT {
            return Err(AudioError::RatioAdjustmentOutOfRange(value));
        }
        self.ratio_adjustment = value;
        self.recompute_ratio();
        Ok(())
    }

    /// Timings in force: they set the frame sequencer periods and select the noise and DMC
    /// period tables (REQ-CORE-015).
    #[must_use]
    pub const fn timing(&self) -> RegionTiming {
        self.timing
    }

    /// Changes the timings in force.
    pub fn set_timing(&mut self, timing: RegionTiming) {
        self.timing = timing;
        self.apply_timing();
        self.recompute_ratio();
    }

    /// Returns the machine to its power-on state, silent (REQ-CORE-004).
    ///
    /// The frame sequencer is left where a $4017 write of $00, ten CPU cycles before the
    /// first instruction, would have put it: the CPU's reset sequence takes seven cycles and
    /// the sequencer reset lands three cycles after the write, so starting the counter at
    /// zero reproduces the 9 to 12 cycle delay the hardware shows (blargg,
    /// `09.reset_timing`).
    pub fn reset(&mut self) {
        self.pulse1.reset();
        self.pulse2.reset();
        self.triangle.reset();
        self.noise.reset();
        self.dmc.reset();
        self.apply_timing();

        self.cycles = 0;
        self.apu_cycle = false;
        self.frame_counter = 0;
        self.frame_step = 0;
        self.five_step_mode = false;
        self.irq_inhibited = false;
        self.frame_irq_flag = false;
        self.frame_irq_pending = false;
        self.pending_sequencer_reset = 0;
        self.readable_status = 0;
        self.level_key = -1;
        self.level = 0.0;

        self.discard_samples();
    }

    /// Drops every pending sample and the resampler's history, so that the audio restarts
    /// from silence (REQ-CORE-074). Reset and closing the ROM do this.
    pub fn discard_samples(&mut self) {
        self.resampler.reset();
        self.drop_pending_samples();
    }

    /// Empties the queue of samples the host has not read yet, leaving the resampler's own
    /// history alone (REQ-CORE-074; Q-006). Restoring a savestate does this.
    pub fn drop_pending_samples(&mut self) {
        lock(&self.ring).clear();
    }

    /// Advances the APU by one CPU cycle.
    pub fn step_cpu_cycle(&mut self) {
        // The frame counter's "set the IRQ flag" signal reaches the flag at the end of the
        // cycle it is raised on, so the CPU can only see it from the next cycle (blargg's
        // 08.irq_timing).
        if self.frame_irq_pending {
            self.frame_irq_pending = false;
            self.frame_irq_flag = true;
        }

        // A CPU read of $4015 sees the register as it stood before this cycle's internal
        // clocking; the bus performs the access after stepping the machine, so the value a
        // read may return is latched here.
        self.readable_status = self.compose_status(0);

        self.cycles += 1;

        if self.pending_sequencer_reset > 0 {
            self.pending_sequencer_reset -= 1;
            if self.pending_sequencer_reset == 0 {
                self.frame_counter = 0;
                self.frame_step = 0;
            }
        }

        self.step_frame_counter();

        self.apu_cycle = !self.apu_cycle;
        if self.apu_cycle {
            // The pulse timers run at half the CPU clock (nesdev).
            self.pulse1.clock_timer();
            self.pulse2.clock_timer();
        }

        self.triangle.clock_timer();
        self.noise.clock_timer();
        self.dmc.clock_timer();

        let level = self.current_level();
        let ring = &self.ring;
        self.resampler
            .step_cycle(level, &mut |sample| lock(ring).enqueue(sample));
    }

    // ---------------------------------------------------------------- frame counter

    fn step_frame_counter(&mut self) {
        let sequence = self.sequence();
        self.frame_counter += 1;

        if self.frame_counter != sequence[self.frame_step] {
            return;
        }

        if self.five_step_mode {
            // Mode 1: quarter frames on steps 0 to 2, half frames on steps 1 and 4, and no
            // IRQ at all (nesdev).
            match self.frame_step {
                0 | 2 => self.clock_quarter_frame(),
                1 | 4 => {
                    self.clock_quarter_frame();
                    self.clock_half_frame();
                }
                _ => {}
            }
        } else {
            // Mode 0: the IRQ flag is set on the last three cycles of the sequence, and the
            // final quarter and half frames fall on the middle one (nesdev; blargg's
            // 07.irq_flag_timing measures all three).
            match self.frame_step {
                0 | 2 => self.clock_quarter_frame(),
                1 => {
                    self.clock_quarter_frame();
                    self.clock_half_frame();
                }
                4 => {
                    self.clock_quarter_frame();
                    self.clock_half_frame();
                    self.set_frame_irq();
                }
                _ => self.set_frame_irq(),
            }
        }

        self.frame_step += 1;
        if self.frame_step == sequence.len() {
            self.frame_step = 0;
            self.frame_counter = 0;
        }
    }

    const fn sequence(&self) -> &'static [i32; 6] {
        match (
            self.timing.uses_pal_noise_and_dmc_periods,
            self.five_step_mode,
        ) {
            (true, true) => &PAL_FIVE_STEP,
            (true, false) => &PAL_FOUR_STEP,
            (false, true) => &NTSC_FIVE_STEP,
            (false, false) => &NTSC_FOUR_STEP,
        }
    }

    fn set_frame_irq(&mut self) {
        if !self.irq_inhibited {
            self.frame_irq_pending = true;
        }
    }

    fn clock_quarter_frame(&mut self) {
        self.pulse1.envelope.clock();
        self.pulse2.envelope.clock();
        self.noise.envelope.clock();
        self.triangle.clock_linear_counter();
    }

    fn clock_half_frame(&mut self) {
        self.pulse1.length.clock(self.cycles);
        self.pulse2.length.clock(self.cycles);
        self.triangle.length.clock(self.cycles);
        self.noise.length.clock(self.cycles);

        self.pulse1.clock_sweep();
        self.pulse2.clock_sweep();
    }

    // ---------------------------------------------------------------- registers

    /// Reads $4015, the status register: length counters in bits 0 to 3, DMC activity in
    /// bit 4, open bus (`open_bus`) in bit 5, frame IRQ in bit 6 and DMC IRQ in bit 7.
    ///
    /// The value returned is the one latched at the start of the current CPU cycle (blargg's
    /// `04.clock_jitter`, `05.len_timing_mode0`, `07.irq_flag_timing`). The read clears the
    /// frame IRQ flag but leaves the DMC's alone (`apu_test` `7-dmc_basics`).
    pub fn read_status(&mut self, open_bus: u8) -> u8 {
        let status = self.readable_status | (open_bus & 0x20);
        self.frame_irq_flag = false;
        status
    }

    /// Reads $4015 with no side effect at all (REQ-CORE-081).
    #[must_use]
    pub const fn peek_status(&self) -> u8 {
        self.compose_status(0)
    }

    const fn compose_status(&self, open_bus: u8) -> u8 {
        let mut status = open_bus & 0x20;

        if self.pulse1.length.active() {
            status |= 0x01;
        }
        if self.pulse2.length.active() {
            status |= 0x02;
        }
        if self.triangle.length.active() {
            status |= 0x04;
        }
        if self.noise.length.active() {
            status |= 0x08;
        }
        if self.dmc.active() {
            status |= 0x10;
        }
        if self.frame_irq_flag {
            status |= 0x40;
        }
        if self.dmc.irq_flag() {
            status |= 0x80;
        }

        status
    }

    /// Writes an APU register, `address` in $4000-$4017.
    pub fn write(&mut self, address: u16, value: u8) {
        match address {
            0x4000..=0x4003 => self.pulse1.write(address - 0x4000, value, self.cycles),
            0x4004..=0x4007 => self.pulse2.write(address - 0x4004, value, self.cycles),
            0x4008..=0x400B => self.triangle.write(address - 0x4008, value, self.cycles),
            0x400C..=0x400F => self.noise.write(address - 0x400C, value, self.cycles),
            0x4010..=0x4013 => self.dmc.write(address - 0x4010, value),
            0x4015 => {
                self.pulse1.length.set_enabled((value & 0x01) != 0);
                self.pulse2.length.set_enabled((value & 0x02) != 0);
                self.triangle.length.set_enabled((value & 0x04) != 0);
                self.noise.length.set_enabled((value & 0x08) != 0);
                self.dmc.set_enabled((value & 0x10) != 0);
            }
            0x4017 => self.write_frame_counter(value),
            _ => {}
        }
    }

    fn write_frame_counter(&mut self, value: u8) {
        self.five_step_mode = (value & 0x80) != 0;
        self.irq_inhibited = (value & 0x40) != 0;

        if self.irq_inhibited {
            // The flag is cleared only by a $4015 read or by a $4017 write with bit 6 set
            // (blargg, "Misc").
            self.frame_irq_flag = false;
            self.frame_irq_pending = false;
        }

        // The divider and the sequencer are reset three CPU cycles after the write, four
        // when it lands between APU cycles (nesdev; blargg's 04.clock_jitter). The write is
        // handled after the cycle has been stepped, so the count starts from the next cycle.
        self.pending_sequencer_reset = if self.apu_cycle { 3 } else { 4 };

        if self.five_step_mode {
            // Mode 1 generates a quarter and a half frame signal straight away.
            self.clock_quarter_frame();
            self.clock_half_frame();
        }
    }

    // ---------------------------------------------------------------- output

    fn apply_timing(&mut self) {
        self.noise.set_timing(&self.timing);
        self.dmc.set_timing(&self.timing);
    }

    fn recompute_ratio(&mut self) {
        let timing = &self.timing;
        let cycles_per_frame = f64::from(RegionTiming::DOTS_PER_SCANLINE)
            * f64::from(timing.scanlines_per_frame)
            * f64::from(timing.dots_per_cycle_denominator)
            / f64::from(timing.dots_per_cycle_numerator);

        // REQ-CORE-070: a frame at speed 1 yields sampleRate / nominalFps samples (800 at
        // 48 kHz in NTSC, 960 in PAL; DEC-R2-T3). REQ-CORE-072 divides that by the speed
        // factor; the frontend's fine adjustment rides on top.
        let samples_per_frame = f64::from(self.sample_rate)
            / (f64::from(timing.nominal_frames_per_second) * self.speed);
        let samples_per_cycle =
            samples_per_frame * (1.0 + self.ratio_adjustment) / cycles_per_frame;

        self.resampler
            .set_ratio(samples_per_cycle, self.sample_rate);
    }

    /// The mixed output level of the five channels, clamped to [-1, 1] (REQ-CORE-073). Only
    /// recomputed when one of the channel outputs actually moves.
    fn current_level(&mut self) -> f64 {
        let pulse1 = self.pulse1.output();
        let pulse2 = self.pulse2.output();
        let triangle = self.triangle.output();
        let noise = self.noise.output();
        let dmc = self.dmc.output();

        let key = pulse1 | (pulse2 << 4) | (triangle << 8) | (noise << 12) | (dmc << 16);
        if key == self.level_key {
            return self.level;
        }

        self.level_key = key;

        // The linear approximation of nesdev's "APU Mixer", which REQ-CORE-073 prescribes.
        let mix = (Self::PULSE_STEP * f64::from(pulse1 + pulse2))
            + (Self::TRIANGLE_STEP * f64::from(triangle))
            + (Self::NOISE_STEP * f64::from(noise))
            + (Self::DMC_STEP * f64::from(dmc));

        self.level = (Self::OUTPUT_GAIN * mix).clamp(-1.0, 1.0);
        self.level
    }

    /// Fills `destination` with audio (REQ-CORE-071), entirely or not at all. With `stereo`
    /// the buffer is interleaved L, R with L = R, and half as many mono samples are
    /// consumed. Returns `destination.len()` when filled, 0 when there were not enough
    /// samples (nothing is consumed; the host pads with silence).
    ///
    /// # Errors
    /// [`AudioError::OddStereoBuffer`] when `stereo` and the length is odd.
    pub fn read_samples(&self, destination: &mut [f32], stereo: bool) -> Result<usize, AudioError> {
        lock(&self.ring).read(destination, stereo)
    }
}

impl Apu2A03 {
    /// Bytes the APU's part of a savestate occupies at most (REQ-ARCH-013).
    pub const MAX_STATE_SIZE: usize = 512 + BandLimitedResampler::STATE_SIZE;

    /// Appends the APU state to `writer` (REQ-CORE-090).
    pub fn save_state(&self, writer: &mut StateWriter) {
        self.pulse1.save_state(writer);
        self.pulse2.save_state(writer);
        self.triangle.save_state(writer);
        self.noise.save_state(writer);
        self.dmc.save_state(writer);

        writer.write_i64(self.cycles);
        writer.write_bool(self.apu_cycle);
        writer.write_i32(self.frame_counter);
        writer.write_usize(self.frame_step);
        writer.write_bool(self.five_step_mode);
        writer.write_bool(self.irq_inhibited);
        writer.write_bool(self.frame_irq_flag);
        writer.write_bool(self.frame_irq_pending);
        writer.write_i32(self.pending_sequencer_reset);
        writer.write_u8(self.readable_status);
        writer.write_i32(self.level_key);
        writer.write_f64(self.level);

        self.resampler.save_state(writer);
    }

    /// Restores the APU state from `reader` (REQ-CORE-091).
    ///
    /// # Errors
    /// [`InvalidState`] when the block is truncated or holds an impossible value.
    pub fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        self.pulse1.load_state(reader)?;
        self.pulse2.load_state(reader)?;
        self.triangle.load_state(reader)?;
        self.noise.load_state(reader)?;
        self.dmc.load_state(reader)?;

        self.cycles = reader.read_i64()?;
        self.apu_cycle = reader.read_bool()?;
        self.frame_counter = reader.read_i32()?;
        self.frame_step = reader.read_index(NTSC_FIVE_STEP.len())?;
        self.five_step_mode = reader.read_bool()?;
        self.irq_inhibited = reader.read_bool()?;
        self.frame_irq_flag = reader.read_bool()?;
        self.frame_irq_pending = reader.read_bool()?;
        self.pending_sequencer_reset = reader.read_i32()?;
        self.readable_status = reader.read_u8()?;
        self.level_key = reader.read_i32()?;
        self.level = reader.read_f64()?;

        self.resampler.load_state(reader)?;

        // Pending audio is dropped on restore (REQ-CORE-074); the resampler's own history was
        // restored above, so the sound picks up from the state that was saved (Q-006).
        self.drop_pending_samples();
        Ok(())
    }
}
