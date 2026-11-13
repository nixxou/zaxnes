//! The 2A03 APU (port of `src/Zax.Nes/Apu`): five sound channels, the frame counter and its
//! IRQ, the $4015 status register, the mixer and the resampled output buffer
//! (REQ-CORE-070 to REQ-CORE-074, REQ-CORE-113).
//!
//! **Register writes land after the cycle.** The bus advances the whole machine by one
//! cycle and only then performs the register access, so a write always takes effect after
//! that cycle's frame-counter events. That ordering is what the hardware does and what
//! blargg's `10.len_halt_timing` and `11.len_reload_timing` measure.
//!
//! **Expansion audio.** REQ-CORE-075 as amended by AMD-054 requires the MMC5's own sound and
//! leaves the other boards (VRC6, VRC7, Namco 163, Sunsoft 5B, FDS) optional. Two boards supply
//! one: the MMC5, and from MF-3 the **Sunsoft 5B** (the YM2149F of mapper 69, whose level Q-118
//! settles provisionally). The output of either is the `M` of REQ-CORE-073, pushed in by the bus
//! ([`Apu2A03::set_expansion_level`]) and subtracted from the APU's own `A` before the sum is
//! clipped (AMD-061 §6.2).

mod band_limited_resampler;
pub mod console_profile;
mod dmc_channel;
mod envelope;
mod length_counter;
pub mod mixer;
mod mmc5_audio;
mod mmc5_pulse_channel;
mod namco163_audio;
mod noise_channel;
mod pulse_channel;
mod sunsoft5b_audio;
mod triangle_channel;
mod vrc6_audio;
mod vrc7_audio;

use crate::state::{InvalidState, StateReader, StateWriter};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::RegionTiming;
use band_limited_resampler::BandLimitedResampler;
use dmc_channel::DmcChannel;
use noise_channel::NoiseChannel;
use pulse_channel::PulseChannel;
use triangle_channel::TriangleChannel;

pub use band_limited_resampler::HIGH_PASS_HZ;
pub(crate) use mmc5_audio::Mmc5Audio;
pub(crate) use namco163_audio::Namco163Audio;
pub(crate) use sunsoft5b_audio::Sunsoft5bAudio;
pub(crate) use vrc6_audio::Vrc6Audio;
pub(crate) use vrc7_audio::Vrc7Audio;

// A rejected audio setting or read is on the `Engine` trait (`read_audio`), so it lives in
// `zax-core-api` since lot FCEUX-23 step 2, with the three bounds its messages print; re-exported
// here at the path it was at, and `Apu2A03`'s three associated constants are defined as those
// bounds, so every message is unchanged to the character.
pub use zax_core_api::AudioError;

/// Four-step frame sequencer, NTSC, in CPU cycles after the sequencer reset (nesdev, "APU
/// Frame Counter"). The last entry closes the sequence and restarts it.
const NTSC_FOUR_STEP: [i32; 6] = [7457, 14913, 22371, 29828, 29829, 29830];

/// Five-step frame sequencer, NTSC CPU cycles.
const NTSC_FIVE_STEP: [i32; 6] = [7457, 14913, 22371, 29829, 37281, 37282];

/// Four-step frame sequencer, PAL CPU cycles.
///
/// The nesdev page gives the 2A07's steps in APU cycles — 4156.5, 8313.5, 12469.5, then
/// 16626, 16626.5 and 16627 for the three on which the IRQ flag is held — and a CPU cycle is
/// half an APU cycle, so the sequence closes at 33252, 33253 and 33254. It used to close at
/// 33250, two cycles early, which is the whole of `pal_apu_tests` 04 to 08, 10 and 11: every
/// one of them measures something against the end of the mode 0 sequence, and 06 does so too
/// because it synchronises on the mode 0 frame IRQ before switching to mode 1.
const PAL_FOUR_STEP: [i32; 6] = [8313, 16627, 24939, 33252, 33253, 33254];

/// Five-step frame sequencer, PAL CPU cycles.
const PAL_FIVE_STEP: [i32; 6] = [8313, 16627, 24939, 33253, 41565, 41566];

/// A value [`Apu2A03::level_key`] can never hold for real channel outputs — the five fields it
/// packs are all non-negative — so storing it means "the mixed level has to be recomputed".
const NO_LEVEL_KEY: i32 = -1;

/// The output ring buffer, shared with [`AudioReader`]s on other threads.
#[derive(Debug, Clone)]
struct SampleRing {
    buffer: Vec<f32>,
    read_index: usize,
    write_index: usize,
    available: usize,
    /// Whether the samples reaching [`SampleRing::enqueue`] are dropped instead of queued
    /// ([`Apu2A03::set_output_enabled`]): a frame of run-ahead that is not the displayed one
    /// (AMD-041 REQ-LOOP-065, steps 1 and 3).
    ///
    /// The test lives here, behind the mutex the enqueue takes anyway, because the enqueue runs
    /// once per **output sample** — about 735 per emulated frame at 44.1 kHz — while the closure
    /// `|sample| lock(ring).enqueue(sample)` that calls it is built once per **CPU cycle**.
    /// Carrying the flag in that closure instead would widen the hottest loop of the APU for
    /// nothing.
    muted: bool,
}

impl SampleRing {
    fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![0.0; capacity],
            read_index: 0,
            write_index: 0,
            available: 0,
            muted: false,
        }
    }

    fn clear(&mut self) {
        self.read_index = 0;
        self.write_index = 0;
        self.available = 0;
    }

    fn enqueue(&mut self, sample: f32) {
        if self.muted {
            // The resampler has already advanced, so the machine is in the very same state
            // either way: only the queueing is skipped (REQ-LOOP-065).
            return;
        }

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
    /// The console whose analog audio path is imitated (`console_profile`); `None`, the default,
    /// keeps the output as it always was.
    console_profile: Option<&'static console_profile::ConsoleAudioProfile>,
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
    /// Bit 6 of `$4015`, which is the frame IRQ flag as a **read** of the register sees it.
    ///
    /// Kept apart from [`Self::frame_irq_flag`], the IRQ line, because a `$4015` read does not
    /// clear the two at the same instant: the line goes low on the read itself, and the flag a
    /// *later* read sees only clears at the start of the next **get** cycle
    /// ([`Self::frame_irq_clear_at`]). See [`Self::read_status`].
    frame_irq_status: bool,
    /// The cycle from which a `$4015` read's clear of [`Self::frame_irq_status`] counts, 0 while
    /// no read is waiting for its get cycle. Consumed by [`Self::read_status`] and looked at by
    /// [`Self::peek_status`]; nothing per cycle reads it, which is what makes the rule free.
    frame_irq_clear_at: i64,
    frame_irq_pending: bool,
    pending_sequencer_reset: i32,
    readable_status: u8,
    /// The five channel outputs the cached [`Self::level`] was mixed from, packed one per
    /// nibble, or [`NO_LEVEL_KEY`] when the level has to be recomputed whatever they read.
    /// Comparing it is the whole of the mixer's cache test, which is why a move of `M`
    /// invalidates it rather than adding a second comparison (see [`Self::current_level`]).
    level_key: i32,
    level: f64,
    /// Whether something the mixed level depends on may have changed since it was computed
    /// (a register write, a frame-counter clock, a reset or a loaded state); a timer that moves
    /// a channel says so itself. The level is only recomputed then: it depends on nothing else.
    level_dirty: bool,
    /// `M` of REQ-CORE-073 (AMD-054): the cartridge's expansion audio, 0 for every board but
    /// the MMC5. The bus pushes it in before the cycles it belongs to are run; it is not part
    /// of the savestate, the board owning its own state and the bus pushing it again at once.
    expansion_level: i32,
    /// The expansion level the cached [`Self::level`] was computed with: the `M` of the cycle
    /// just mixed, which is the test point [`Self::mixer_expansion`] reports.
    level_expansion: i32,
    /// `A` of REQ-CORE-073 on the cycle just mixed (the test point of REQ-CORE-076).
    mixer_apu: i32,
    /// `S` of REQ-CORE-073 on the cycle just mixed (the test point of REQ-CORE-076).
    mixer_output: i32,
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
            console_profile: self.console_profile,
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
            frame_irq_status: self.frame_irq_status,
            frame_irq_clear_at: self.frame_irq_clear_at,
            frame_irq_pending: self.frame_irq_pending,
            pending_sequencer_reset: self.pending_sequencer_reset,
            readable_status: self.readable_status,
            level_key: self.level_key,
            level: self.level,
            level_dirty: self.level_dirty,
            expansion_level: self.expansion_level,
            level_expansion: self.level_expansion,
            mixer_apu: self.mixer_apu,
            mixer_output: self.mixer_output,
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
    pub const MIN_SAMPLE_RATE: i32 = zax_core_api::MIN_SAMPLE_RATE;
    /// Highest output sample rate the core accepts (REQ-CORE-070).
    pub const MAX_SAMPLE_RATE: i32 = zax_core_api::MAX_SAMPLE_RATE;
    /// Default output sample rate (REQ-CORE-070, SPEC-11 REQ-AUD-001).
    pub const DEFAULT_SAMPLE_RATE: i32 = 48_000;
    /// Minimum capacity of the output buffer, in mono samples (REQ-CORE-071).
    pub const MIN_BUFFER_SAMPLES: usize = 10_000;
    /// Largest fine adjustment of the resampling ratio a frontend may ask for (REQ-CORE-070).
    pub const MAX_RATIO_ADJUSTMENT: f64 = zax_core_api::MAX_RATIO_ADJUSTMENT;
    // The linear steps and the 1.526 gain of REQ-CORE-073 before AMD-054 are gone: the mixing
    // is the non-linear form of `mixer`.

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
            console_profile: None,
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
            frame_irq_status: false,
            frame_irq_clear_at: 0,
            frame_irq_pending: false,
            pending_sequencer_reset: 0,
            readable_status: 0,
            level_key: NO_LEVEL_KEY,
            level: 0.0,
            level_dirty: true,
            expansion_level: 0,
            level_expansion: 0,
            mixer_apu: 0,
            mixer_output: 0,
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

    /// The mixed output level on the cycle just stepped, in [-1, 1] (REQ-CORE-073), before
    /// resampling and before the DC blocker: [`Self::mixer_output`] divided by 32 768.
    #[must_use]
    pub const fn mixed_level(&self) -> f64 {
        self.level
    }

    /// `A` of REQ-CORE-073 on the cycle just stepped: the five APU channels mixed through the
    /// two non-linear tables, 0 to 32 767. Part of the test point of REQ-CORE-076.
    #[must_use]
    pub const fn mixer_apu(&self) -> i32 {
        self.mixer_apu
    }

    /// `M` of REQ-CORE-073 on the cycle just stepped: the expansion audio's output, 0 without
    /// any (REQ-CORE-075). Part of the test point of REQ-CORE-076.
    #[must_use]
    pub const fn mixer_expansion(&self) -> i32 {
        self.level_expansion
    }

    /// `S = A − M` of REQ-CORE-073, clipped to `-32 768..=32 767` (AMD-061 §6.2), on the cycle
    /// just stepped, before the conversion to floating point and before the DC blocker. Part of
    /// the test point of REQ-CORE-076.
    #[must_use]
    pub const fn mixer_output(&self) -> i32 {
        self.mixer_output
    }

    /// Hands the APU the cartridge's expansion audio output, the `M` of REQ-CORE-073
    /// (REQ-CORE-075). Called by the bus for a board that has one, before the cycles that `M`
    /// belongs to are run; a board without expansion audio never calls it and the level stays
    /// 0, which is what makes the mixing cost nothing for every other cartridge.
    ///
    /// A move of `M` invalidates the mixer's cache key rather than joining the comparison the
    /// cache makes: this runs only for the MMC5 and only when the value actually changes, while
    /// the comparison is on a per-cycle path (see [`Self::current_level`]).
    pub fn set_expansion_level(&mut self, level: i32) {
        if level != self.expansion_level {
            self.expansion_level = level;
            self.level_dirty = true;
            self.level_key = NO_LEVEL_KEY;
        }
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

    /// CPU cycles from the APU's own position until the DMC's memory reader could next ask for a
    /// byte, or `None` while no sample is playing (`DmcChannel::cycles_until_fetch`).
    ///
    /// The position is the APU's, not the bus's: a caller whose APU is lagging has to take its
    /// lag off the answer, because on a quiet cycle the DMC's timer has not been counted down
    /// yet (see `NesBus::oam_fast_pairs`).
    #[must_use]
    pub const fn cycles_until_dmc_fetch(&self) -> Option<u32> {
        self.dmc.cycles_until_fetch()
    }

    /// The half of the APU cycle the next CPU cycle is: the parity the `$4017` reset delay and the
    /// DMC's load delay read.
    #[must_use]
    pub const fn apu_cycle(&self) -> bool {
        self.apu_cycle
    }

    /// Hands the byte a DMC DMA fetched to the memory reader.
    ///
    /// `Some` when the hardware would ask for one more fetch and take it back (see
    /// `DmcChannel::supply_byte`).
    pub fn supply_dmc_byte(&mut self, value: u8) -> Option<u32> {
        self.dmc.supply_byte(value)
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
                let muted = ring.muted;
                *ring = SampleRing::new(capacity);
                ring.muted = muted;
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
        self.level_dirty = true;
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
        self.frame_irq_status = false;
        self.frame_irq_clear_at = 0;
        self.frame_irq_pending = false;
        self.pending_sequencer_reset = 0;
        self.readable_status = 0;
        self.level_key = NO_LEVEL_KEY;
        self.level = 0.0;
        self.level_dirty = true;
        // The board is reset with the rest of the machine, so its `M` goes back to 0 too; the
        // bus pushes the real value again before the next cycle runs.
        self.expansion_level = 0;
        self.level_expansion = 0;
        self.mixer_apu = 0;
        self.mixer_output = 0;

        self.discard_samples();
    }

    /// The documented *after reset* state of the APU (AMD-044 REQ-CORE-122): the five channels
    /// are silenced as a `$4015 = 0` write would, the frame IRQ flag goes away, the triangle
    /// phase goes back to 0 and the DMC output is `AND`ed with 1. The `$4017` mode — five-step
    /// and IRQ inhibit — is **unchanged** (nesdev, "CPU power up state").
    ///
    /// The frame sequencer is put back where [`Self::reset`] leaves it, which is the 2A03G
    /// behaviour and keeps the two resets consistent; the letterless revision leaves it
    /// running. The choice is recorded in `docs/spec/changes/Q-078.md`. The pending samples and
    /// the resampler's history are **kept**: a RESET press does not interrupt the session, the
    /// channels simply fall silent (SPEC-11 REQ-AUD-012).
    // Q-078
    pub fn soft_reset(&mut self) {
        // $4015 = 0.
        self.pulse1.length.set_enabled(false);
        self.pulse2.length.set_enabled(false);
        self.triangle.length.set_enabled(false);
        self.noise.length.set_enabled(false);
        // A disable queues no load, so the returned delay is always 0.
        self.dmc.set_enabled(false, self.apu_cycle);

        self.triangle.reset_phase();
        self.dmc.soft_reset();

        self.frame_irq_flag = false;
        self.frame_irq_status = false;
        self.frame_irq_clear_at = 0;
        self.frame_irq_pending = false;
        self.frame_counter = 0;
        self.frame_step = 0;
        self.pending_sequencer_reset = 0;

        // Every channel output may have moved, and the status byte a read would see with it.
        // The level is recomputed on the next cycle, test point included.
        self.level_dirty = true;
        self.level_key = NO_LEVEL_KEY;
        self.readable_status = self.compose_status(0);
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

    /// Whether the samples the resampler produces reach the output queue; true by default
    /// (AMD-041 REQ-LOOP-065).
    ///
    /// A frame of run-ahead that is neither the displayed one nor heard clears it.
    /// [`NesCore::run_frame_ahead`] owns it and puts it back before the frame that *is* heard, so
    /// a host that never asks for run-ahead never sees it move. The resampler keeps running
    /// either way — its filter history and its fractional position are part of the machine
    /// (REQ-CORE-090) and must advance exactly as they would otherwise — so only the queueing is
    /// skipped and nothing the emulation reads back depends on it.
    ///
    /// [`NesCore::run_frame_ahead`]: crate::NesCore::run_frame_ahead
    pub fn set_output_enabled(&mut self, enabled: bool) {
        lock(&self.ring).muted = !enabled;
    }

    /// Whether [`Apu2A03::set_output_enabled`] is on.
    #[must_use]
    pub fn output_enabled(&self) -> bool {
        !lock(&self.ring).muted
    }

    /// Advances the APU by one CPU cycle.
    pub fn step_cpu_cycle(&mut self) {
        // The frame counter's "set the IRQ flag" signal reaches the flag at the end of the
        // cycle it is raised on, so the CPU can only see it from the next cycle (blargg's
        // 08.irq_timing).
        if self.frame_irq_pending {
            self.frame_irq_pending = false;
            // With the interrupt inhibited the flag still shows in `$4015` (see
            // `set_frame_irq`); only the line stays up.
            self.frame_irq_flag = !self.irq_inhibited;
            // The flag being raised again overrides a clear a `$4015` read had armed and whose
            // get cycle has not come: two stores in a branch three cycles of a frame take, and
            // nothing outside it.
            self.frame_irq_status = true;
            self.frame_irq_clear_at = 0;
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

        let mut changed = self.step_frame_counter();

        self.apu_cycle = !self.apu_cycle;
        if self.apu_cycle {
            // The pulse timers run at half the CPU clock (nesdev).
            changed |= self.pulse1.clock_timer();
            changed |= self.pulse2.clock_timer();
        }

        changed |= self.triangle.clock_timer();
        changed |= self.noise.clock_timer();
        changed |= self.dmc.clock_timer();

        if changed || self.level_dirty {
            self.level_dirty = false;
            self.current_level();
        }
        let level = self.level;
        let ring = &self.ring;
        self.resampler
            .step_cycle(level, &mut |sample| lock(ring).enqueue(sample));
    }

    /// CPU cycles from now that would only count the timers and the frame counter down: no
    /// channel moves on them, no frame-counter step falls on them, so the output, the IRQ
    /// line, $4015 and the DMC's fetch request all stay as they are. 0 while a step of the
    /// level, a pending IRQ flag or a sequencer reset is due.
    #[must_use]
    #[allow(clippy::float_cmp, clippy::cast_sign_loss)] // Exact level; positive counts.
    pub fn quiet_cycles(&self) -> u32 {
        if self.frame_irq_pending || self.level_dirty || self.resampler.level() != self.level {
            return 0;
        }

        let mut quiet = u32::MAX;
        if self.pending_sequencer_reset > 0 {
            quiet = (self.pending_sequencer_reset - 1) as u32;
        }
        let next_step = self.sequence()[self.frame_step];
        if next_step > self.frame_counter {
            quiet = quiet.min((next_step - self.frame_counter - 1) as u32);
        }

        // A pulse timer is clocked on the cycles that turn `apu_cycle` on: the next one is
        // the coming cycle when it is off.
        let parity = u32::from(self.apu_cycle);
        for pulse in [&self.pulse1, &self.pulse2] {
            quiet = quiet.min(
                pulse
                    .quiet_clocks()
                    .saturating_mul(2)
                    .saturating_add(parity),
            );
        }

        quiet
            .min(self.triangle.quiet_clocks())
            .min(self.noise.quiet_clocks())
            .min(self.dmc.quiet_clocks())
    }

    /// Runs `count` CPU cycles, as many calls of [`Self::step_cpu_cycle`] would: the quiet
    /// stretches are skipped in one go, and the last cycle is always stepped, so $4015 holds
    /// the value latched at its start.
    pub fn run_cpu_cycles(&mut self, mut count: u32) {
        while count > 0 {
            let skip = if count > 1 {
                self.quiet_cycles().min(count - 1)
            } else {
                0
            };
            if skip > 0 {
                self.skip_quiet_cycles(skip);
                count -= skip;
            }
            self.step_cpu_cycle();
            count -= 1;
        }
    }

    /// `cycles` quiet cycles (at most [`Self::quiet_cycles`]) in one go.
    #[allow(clippy::cast_possible_wrap)] // At most a frame's cycles.
    fn skip_quiet_cycles(&mut self, cycles: u32) {
        // Nothing the status depends on moves, so every skipped cycle latches this value.
        self.readable_status = self.compose_status(0);
        self.cycles += i64::from(cycles);
        self.frame_counter += cycles as i32;
        if self.pending_sequencer_reset > 0 {
            // Still above 0: the reset itself is not quiet.
            self.pending_sequencer_reset -= cycles as i32;
        }

        let pulse_clocks = if self.apu_cycle {
            cycles / 2
        } else {
            cycles.div_ceil(2)
        };
        self.apu_cycle ^= (cycles & 1) != 0;
        self.pulse1.skip_quiet_clocks(pulse_clocks);
        self.pulse2.skip_quiet_clocks(pulse_clocks);
        self.triangle.skip_quiet_clocks(cycles);
        self.noise.skip_quiet_clocks(cycles);
        self.dmc.skip_quiet_clocks(cycles);

        let ring = &self.ring;
        self.resampler
            .skip_cycles(cycles, &mut |sample| lock(ring).enqueue(sample));
    }

    // ---------------------------------------------------------------- frame counter

    /// Moves the frame counter on; `true` when a step fell on this cycle (a quarter or half
    /// frame, or an IRQ step).
    fn step_frame_counter(&mut self) -> bool {
        let sequence = self.sequence();
        self.frame_counter += 1;

        if self.frame_counter != sequence[self.frame_step] {
            return false;
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
                3 => self.set_frame_irq(),
                _ => {
                    if self.irq_inhibited {
                        // The third cycle sets bit 6 "according to the Suppress Frame Counter
                        // Interrupts flag": inhibited, the bit the two cycles before raised goes
                        // back down, seen from the next cycle like everything the sequencer does
                        // (`AccuracyCoin`, `Frame Counter IRQ` L).
                        if self.frame_irq_status {
                            self.frame_irq_clear_at = self.cycles + 1;
                        }
                    } else {
                        self.set_frame_irq();
                    }
                }
            }
        }

        self.frame_step += 1;
        if self.frame_step == sequence.len() {
            self.frame_step = 0;
            self.frame_counter = 0;
        }

        true
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

    /// Raises bit 6 of `$4015` for the next cycle, and the IRQ line with it unless the interrupt
    /// is inhibited.
    ///
    /// **Inhibited, the bit is still raised**, on the first two of the sequence's last three
    /// cycles: "29828 cycles after frame counter reset: $4015.6 is set to 1. 29829: $4015.6 is
    /// set to 1 (the IRQ level detector is only pulled low if the Suppress Frame Counter
    /// Interrupts flag is false). 29830: $4015.6 is set according to the flag" (`AccuracyCoin`,
    /// `Frame Counter IRQ` I to M). `step_cpu_cycle` keeps the line up when it applies this.
    fn set_frame_irq(&mut self) {
        self.frame_irq_pending = true;
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
    ///
    /// **The clear of bit 6 waits for a get cycle.** `AccuracyCoin`'s `Frame Counter IRQ` 7 says
    /// it in one line — "The IRQ flag should not be cleared when the APU transitions from a
    /// 'get' cycle to a 'put' cycle" — and its 6 says the other half, that it *is* cleared on the
    /// put→get transition. The ROM's own commentary spells the two cases out: a `$1F` (`SLO
    /// Absolute,X`) over `$4015` reads the register twice, one CPU cycle apart, and
    ///
    /// * read on a **put** cycle, read again on the **get** after it → the second read sees bit 6
    ///   **clear** (its test 6);
    /// * read on a **get** cycle, read again on the **put** after it → the second read still sees
    ///   bit 6 **set**, and the clear lands on the get cycle after *that* (its test 7).
    ///
    /// So the clear takes effect at the start of the first get cycle strictly after the read,
    /// which in this core's numbering — where a get cycle is an **even** one, the convention
    /// [`NesBus::run_dma`](crate::NesBus) already uses — is `(cycle + 2) & !1`.
    ///
    /// It is applied **lazily**, when a read or a peek next asks, and not from
    /// [`Self::step_cpu_cycle`]: only a read of `$4015` can see bit 6 at all, so a per-cycle test
    /// would buy nothing and would land on the one path the APU walks every cycle. The IRQ
    /// **line** ([`Self::frame_irq_flag`]) is a different matter and goes low here, at once: see
    /// `Q-122` for what pins that half and what does not.
    // Q-122
    // Portions adapted from Mesen 2 (c) Sour, GPL-3.0
    pub fn read_status(&mut self, open_bus: u8) -> u8 {
        let mut status = self.readable_status | (open_bus & 0x20);

        if self.frame_irq_status {
            if self.frame_irq_clear_at != 0 && self.cycles >= self.frame_irq_clear_at {
                // The get cycle an earlier read armed has come. `readable_status` was latched at
                // the start of this cycle, before the clear, and this read is the first thing
                // able to tell the difference.
                self.frame_irq_status = false;
                self.frame_irq_clear_at = 0;
                status &= !0x40;
            } else {
                // This read arms the clear for the first get cycle strictly after it. A read
                // that re-arms what an earlier one armed computes the same cycle, so the two
                // reads of `Frame Counter IRQ` 7 agree.
                self.frame_irq_clear_at = (self.cycles + 2) & !1;
            }
        }

        self.frame_irq_flag = false;
        status
    }

    /// Reads $4015 with no side effect at all (REQ-CORE-081).
    #[must_use]
    pub const fn peek_status(&self) -> u8 {
        let status = self.compose_status(0);
        if self.frame_irq_clear_at != 0 && self.cycles >= self.frame_irq_clear_at {
            // A clear an earlier read armed is due; a read would see bit 6 gone. Looked at and
            // not applied: a peek may not move the machine (REQ-CORE-081).
            return status & !0x40;
        }
        status
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
        // Bit 6 is `frame_irq_status` and not the IRQ line: the two differ for the cycle or two
        // between a `$4015` read and the get cycle its clear waits for (see
        // [`Self::read_status`]).
        if self.frame_irq_status {
            status |= 0x40;
        }
        if self.dmc.irq_flag() {
            status |= 0x80;
        }

        status
    }

    /// Writes an APU register, `address` in $4000-$4017.
    ///
    /// Returns the DMC **load** fetch's start delay in CPU cycles, 0 for every write but a
    /// `$4015` that starts a sample: the delay belongs to the bus, which owns the DMA cycles and
    /// can hold it as one absolute cycle instead of a counter the per-cycle APU code would have
    /// to look at (see [`DmcChannel::set_enabled`]).
    pub fn write(&mut self, address: u16, value: u8) -> u32 {
        self.level_dirty = true;
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
                return self.dmc.set_enabled((value & 0x10) != 0, self.apu_cycle);
            }
            0x4017 => self.write_frame_counter(value),
            _ => {}
        }
        0
    }

    fn write_frame_counter(&mut self, value: u8) {
        self.five_step_mode = (value & 0x80) != 0;
        self.irq_inhibited = (value & 0x40) != 0;

        if self.irq_inhibited {
            // The flag is cleared only by a $4015 read or by a $4017 write with bit 6 set
            // (blargg, "Misc"). This clear has no get-cycle delay: `AccuracyCoin`'s
            // `Frame Counter IRQ` 9 reads `$4015` on the very next instruction and requires bit 6
            // to be gone already.
            self.frame_irq_flag = false;
            self.frame_irq_status = false;
            self.frame_irq_clear_at = 0;
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
        self.resampler
            .set_profile(self.console_profile, self.sample_rate);
    }

    /// Imitates the analog audio path of one measured console (`console_profile`): `index` 0 for
    /// none, the output as it always was; `n` for `CONSOLE_AUDIO_PROFILES[n - 1]`. Out-of-range
    /// indices mean none. Nothing changes per CPU cycle: the channel gains apply when the level
    /// moves and the filters once per output sample.
    pub fn set_console_profile(&mut self, index: usize) {
        self.console_profile = index
            .checked_sub(1)
            .and_then(|slot| console_profile::CONSOLE_AUDIO_PROFILES.get(slot));
        self.noise
            .set_letterless(self.console_profile.is_some_and(|profile| profile.letterless));
        self.resampler
            .set_profile(self.console_profile, self.sample_rate);
        self.level_key = -1;
        self.level_dirty = true;
    }

    /// The mixed output level, in [-1, 1] (REQ-CORE-073): the non-linear mix `A` of the five
    /// channels plus the expansion audio's `M`, clipped to 32 767 and divided by 32 768.
    ///
    /// The channel outputs are small integers, so the mix is only recomputed when one of them
    /// or the expansion output actually moves; most CPU cycles change nothing. Where the linear
    /// form of REQ-CORE-073 before AMD-054 did four multiply-adds and a clamp, this does two
    /// array reads and an addition, so the non-linear mixing costs strictly less than what it
    /// replaces and the lazy mixing stays exactly as it was.
    ///
    /// The DC blocker REQ-CORE-073 puts after this conversion runs on the output samples rather
    /// than on this signal, inside [`BandLimitedResampler`]; the two orderings are equivalent
    /// for a linear filter, and this one keeps the resampler's input piecewise constant, which
    /// is what makes its step synthesis cheap. See `docs/spec/changes/Q-091.md`.
    // Q-091
    //
    // It has exactly **one** caller, the per-cycle step, which is what lets the inliner put it
    // there. A second call site is enough to make it leave the function out of line, and the
    // call plus its prologue would then cost every one of the ~15 000 cycles per frame that
    // reach it; `load_state` therefore writes the same four lines out rather than calling here.
    fn current_level(&mut self) -> f64 {
        let pulse1 = self.pulse1.output();
        let pulse2 = self.pulse2.output();
        let triangle = self.triangle.output();
        let noise = self.noise.output();
        let dmc = self.dmc.output();

        // One comparison, as before AMD-054: a move of `M` invalidates `level_key` itself (see
        // [`Self::set_expansion_level`]) instead of being compared here. This is a per-cycle
        // path — `nestest` reaches it about 15 000 times per emulated frame — so the second
        // comparison RU-3a added cost about 1 % of the APU's instructions for every board.
        let key = pulse1 | (pulse2 << 4) | (triangle << 8) | (noise << 12) | (dmc << 16);
        if key == self.level_key {
            return self.level;
        }

        self.level_key = key;
        self.level_expansion = self.expansion_level;
        self.mixer_apu = match self.console_profile {
            None => mixer::mix(pulse1, pulse2, triangle, noise, dmc),
            Some(profile) => profile_mix(profile, [pulse1, pulse2, triangle, noise, dmc]),
        };
        self.mixer_output = mixer::clip(self.mixer_apu, self.expansion_level);
        self.level = mixer::to_float(self.mixer_output);
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
        self.load_state_inner(reader, false)
    }

    /// [`Apu2A03::load_state`] leaving the samples the host has not read yet where they are
    /// (AMD-041 REQ-CORE-121).
    ///
    /// Step 5 of REQ-LOOP-065 undoes the frames in advance, not the sound the displayed frame
    /// has just produced: that future *does* happen — it is the one the player is about to hear —
    /// so the queue survives, where REQ-CORE-074 empties it for a savestate or a rollback. The
    /// resampler's own history is restored either way, so the machine is in the very same state.
    ///
    /// # Errors
    /// [`InvalidState`] when the block is truncated or holds an impossible value.
    pub fn load_state_keeping_samples(
        &mut self,
        reader: &mut StateReader,
    ) -> Result<(), InvalidState> {
        self.load_state_inner(reader, true)
    }

    fn load_state_inner(
        &mut self,
        reader: &mut StateReader,
        keep_pending_samples: bool,
    ) -> Result<(), InvalidState> {
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
        // Not in the block, and nothing observable is lost by deriving them (REQ-CORE-091, the
        // format is unchanged). The flag a read sees and the IRQ line differ only between a
        // `$4015` read and the get cycle its clear waits for — one or two cycles — and the
        // nearest a *second* `$4015` read can be to the first across a save point is three
        // cycles, a state being written between instructions and the first cycle of an
        // instruction being its opcode fetch. So the window can never still be open when
        // anything looks at it.
        self.frame_irq_status = self.frame_irq_flag;
        self.frame_irq_clear_at = 0;
        self.frame_irq_pending = reader.read_bool()?;
        self.pending_sequencer_reset = reader.read_i32()?;
        self.readable_status = reader.read_u8()?;
        self.level_key = reader.read_i32()?;
        self.level = reader.read_f64()?;
        self.level_dirty = true;
        // `M` and the two integers of the test point are not in the block: the board restores
        // its own sound state and the bus pushes `M` again before the next cycle. Keeping them
        // out is what lets a block written before AMD-054 still load (REQ-CORE-091): the layout
        // is unchanged.
        self.expansion_level = 0;
        self.level_expansion = 0;
        // The three derived fields are rebuilt here and now, from the channel outputs just read
        // back, so that the test point of REQ-CORE-076 agrees with the restored level instead of
        // reading 0 until a channel next moves. `level_key` is left exactly as the block gave
        // it, so saving again writes the same bytes (REQ-CORE-092); whether it matches the key
        // these outputs pack or not, the next cycle is right either way — it either keeps the
        // level computed here or recomputes the same one.
        //
        // The mix is written out rather than calling `current_level`, which must keep its single
        // call site to stay inlined in the per-cycle step (see the note there).
        let levels = [
            self.pulse1.output(),
            self.pulse2.output(),
            self.triangle.output(),
            self.noise.output(),
            self.dmc.output(),
        ];
        self.mixer_apu = match self.console_profile {
            None => mixer::mix(levels[0], levels[1], levels[2], levels[3], levels[4]),
            Some(profile) => profile_mix(profile, levels),
        };
        self.mixer_output = mixer::clip(self.mixer_apu, 0);
        self.level = mixer::to_float(self.mixer_output);

        self.resampler.load_state(reader)?;

        // Pending audio is dropped on restore (REQ-CORE-074); the resampler's own history was
        // restored above, so the sound picks up from the state that was saved (Q-006). The
        // in-memory restore of REQ-CORE-121 is the one exception.
        if !keep_pending_samples {
            self.drop_pending_samples();
        }
        Ok(())
    }
}

/// `A` of REQ-CORE-073 for a console profile: its weighted exact mix, on the tables' scale.
#[cold]
#[inline(never)]
#[allow(clippy::cast_possible_truncation)] // At most FULL_SCALE.
fn profile_mix(profile: &console_profile::ConsoleAudioProfile, levels: [i32; 5]) -> i32 {
    (mixer::NORMALISATION * profile.mix(levels)).round() as i32
}
