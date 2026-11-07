//! The pull side of the audio output: what SDL's audio thread runs (Q-153).
//!
//! Two independent signals meet here.
//!
//! * **The console.** Read straight out of the APU's sample ring, on the audio thread, through
//!   [`ConsoleSamples`]. Nothing of the main thread stands between the core and the driver, so a
//!   rendered frame of any length cannot starve the device. The ring is *latency controlled*:
//!   playback only starts once it holds the configured latency ([`AudioControl::target_frames`]),
//!   and the host's rate control steers the core's resampling ratio to keep it there.
//! * **The program** — the menu music and the interface sounds. Those need file reads, a track
//!   chooser and the fade state machine of SPEC-11 §3.3, none of which belongs on a real-time
//!   thread, so the mixer still produces them on the main thread, a few tens of milliseconds
//!   ahead, into the stereo ring of [`ProgramWriter`]. The audio thread only copies.
//!
//! **No silence is ever inserted while data is merely late.** When a signal runs out mid-block
//! its last sample is held and ramped to zero over [`RAMP_MS`]; when samples come back it is
//! ramped in again over the same time. A block therefore never contains a discontinuity, which
//! is what the all-or-nothing rule of REQ-AUD-003 was after — see Q-153 §1, which records the
//! divergence from that requirement's letter (it mandates a whole block of zeros, and that block
//! is exactly the pop the owner hears).
//!
//! The console gain of REQ-AUD-020 is published by the main thread as a plain number and
//! followed here **per sample**, so a fade is no longer a staircase of block-sized steps.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use zax_nes::apu::AudioReader;
use zax_platform::audio::{AudioRender, CHANNELS};

/// Default target latency of the ring, in milliseconds (`audio.latency_ms`, Q-153 §2).
pub const DEFAULT_LATENCY_MS: f32 = 60.0;
/// Least target latency the setting accepts.
pub const MIN_LATENCY_MS: f32 = 20.0;
/// Greatest target latency the setting accepts; what the core's ring can really hold may be
/// smaller (Q-153 §3).
pub const MAX_LATENCY_MS: f32 = 200.0;
/// How long the hold-and-fade of a drought lasts, and the ramp back in (Q-153 §1).
pub const RAMP_MS: f32 = 6.0;
/// How long the console gain takes to reach a new published value, at most.
pub const GAIN_SLEW_MS: f32 = 8.0;
/// How far ahead the mixer keeps the menu music and the interface sounds.
///
/// It only has to cover the jitter of the main thread's top-ups, and a shortfall is a smooth
/// dip of a signal at gain 0,02 rather than a hole in the game's sound, so it stays short: an
/// interface sound is then heard about 48 ms after its event at worst, which is what the four
/// queued blocks of v0.4.2 already cost (Q-153 §5).
pub const PROGRAM_AHEAD_MS: f32 = 48.0;

/// Mono samples the emulated console produced.
///
/// Implemented over the APU's ring by [`ApuSamples`]; a test installs its own.
pub trait ConsoleSamples: Send {
    /// Samples waiting to be read.
    fn available(&self) -> usize;
    /// Reads **at most** `out.len()` samples and returns how many; a partial read is expected
    /// and is what lets the stream hold-and-fade instead of emitting a block of zeros.
    fn read(&mut self, out: &mut [f32]) -> usize;
}

/// [`ConsoleSamples`] over the APU's output ring (REQ-CORE-071).
pub struct ApuSamples {
    reader: AudioReader,
}

impl ApuSamples {
    /// Reads from `reader`.
    #[must_use]
    pub const fn new(reader: AudioReader) -> Self {
        Self { reader }
    }
}

impl ConsoleSamples for ApuSamples {
    fn available(&self) -> usize {
        self.reader.available_samples()
    }

    fn read(&mut self, out: &mut [f32]) -> usize {
        // The ring's own read is all-or-nothing, so it is asked for exactly what it holds. Only
        // the producer grows `available`, so the count cannot have fallen in between.
        let wanted = self.reader.available_samples().min(out.len());
        if wanted == 0 {
            return 0;
        }
        self.reader
            .read_samples(&mut out[..wanted], false)
            .unwrap_or(0)
    }
}

/// The slot the main thread puts the console's reader in when a game is loaded.
pub type ConsoleSlot = Arc<Mutex<Option<Box<dyn ConsoleSamples>>>>;

/// What the main thread keeps of an open stream: everything it needs to steer the audio thread
/// without ever waiting on it.
#[derive(Clone)]
pub struct AudioLink {
    /// The shared control and statistics block.
    pub control: Arc<AudioControl>,
    /// Where the console's reader goes when a game is loaded.
    pub console: ConsoleSlot,
    /// The rate the device was opened at.
    pub sample_rate: u32,
}

impl std::fmt::Debug for AudioLink {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AudioLink")
            .field("control", &self.control)
            .field("sample_rate", &self.sample_rate)
            .finish_non_exhaustive()
    }
}

impl AudioLink {
    /// A link to a stream nothing pulls from: what the host runs with when no device opened,
    /// and what a test drives.
    #[must_use]
    pub fn detached(sample_rate: u32, target_frames: usize) -> Self {
        Self {
            control: Arc::new(AudioControl::new(target_frames)),
            console: Arc::new(Mutex::new(None)),
            sample_rate: sample_rate.max(1),
        }
    }

    /// Hands the console's samples to the audio thread, or takes them away.
    pub fn set_console(&self, source: Option<Box<dyn ConsoleSamples>>) {
        *guard(&self.console) = source;
    }

    /// The target latency, in milliseconds.
    #[must_use]
    pub fn target_ms(&self) -> f32 {
        #[allow(clippy::cast_precision_loss)] // a target is a few thousand frames
        let frames = self.control.target_frames() as f32;
        ms_of_frames(self.sample_rate, frames)
    }

    /// Sets the target latency from a value in milliseconds.
    pub fn set_target_ms(&self, milliseconds: f32) {
        self.control
            .set_target_frames(frames_of_ms(self.sample_rate, milliseconds));
    }

    /// The measured latency of the ring, in milliseconds, averaged over about a second.
    #[must_use]
    pub fn measured_ms(&self) -> f32 {
        ms_of_frames(self.sample_rate, self.control.average_fill())
    }
}

fn guard<T>(value: &Mutex<T>) -> MutexGuard<'_, T> {
    // These hold plain numbers and buffers: a panic elsewhere cannot leave one inconsistent.
    value.lock().unwrap_or_else(PoisonError::into_inner)
}

fn bits(value: f32) -> u32 {
    value.to_bits()
}

/// What the main thread and the audio thread tell each other, without a lock.
#[derive(Debug)]
pub struct AudioControl {
    console_gain: AtomicU32,
    console_active: AtomicBool,
    target_frames: AtomicU32,
    resync: AtomicU32,
    playing: AtomicBool,
    underruns: AtomicU64,
    rendered_frames: AtomicU64,
    fill_average: AtomicU32,
    fill_minimum: AtomicU32,
    fill_maximum: AtomicU32,
    window_minimum: AtomicU32,
    window_maximum: AtomicU32,
}

impl AudioControl {
    /// A control block with no console, silent, waiting for `target_frames` of prefill.
    #[must_use]
    pub fn new(target_frames: usize) -> Self {
        let control = Self {
            console_gain: AtomicU32::new(bits(0.0)),
            console_active: AtomicBool::new(false),
            target_frames: AtomicU32::new(0),
            resync: AtomicU32::new(0),
            playing: AtomicBool::new(false),
            underruns: AtomicU64::new(0),
            rendered_frames: AtomicU64::new(0),
            fill_average: AtomicU32::new(bits(0.0)),
            fill_minimum: AtomicU32::new(u32::MAX),
            fill_maximum: AtomicU32::new(0),
            window_minimum: AtomicU32::new(u32::MAX),
            window_maximum: AtomicU32::new(0),
        };
        control.set_target_frames(target_frames);
        control
    }

    /// Sets the gain the console signal is played at (REQ-AUD-020, REQ-AUD-006).
    pub fn set_console_gain(&self, value: f32) {
        self.console_gain.store(bits(value), Ordering::Relaxed);
    }

    /// The gain the console signal is played at.
    #[must_use]
    pub fn console_gain(&self) -> f32 {
        f32::from_bits(self.console_gain.load(Ordering::Relaxed))
    }

    /// Says whether the console is expected to produce samples — that is, whether a game is
    /// running. While it is not, a dry ring is the silence of REQ-AUD-010 and not an underrun.
    pub fn set_console_active(&self, value: bool) {
        self.console_active.store(value, Ordering::Relaxed);
    }

    /// Whether the console is expected to produce samples.
    #[must_use]
    pub fn console_active(&self) -> bool {
        self.console_active.load(Ordering::Relaxed)
    }

    /// Sets the latency the ring is held at, in frames.
    pub fn set_target_frames(&self, frames: usize) {
        self.target_frames
            .store(u32::try_from(frames).unwrap_or(u32::MAX), Ordering::Relaxed);
    }

    /// The latency the ring is held at, in frames.
    #[must_use]
    pub fn target_frames(&self) -> usize {
        self.target_frames.load(Ordering::Relaxed) as usize
    }

    /// Asks the stream to resynchronise: it stops reading until the ring holds the prefill
    /// again, and ramps the signal in rather than cutting it. The caller empties the ring
    /// first, so that no stale fragment of the previous moment is replayed (REQ-AUD-010).
    pub fn request_resync(&self) {
        self.resync.fetch_add(1, Ordering::Release);
    }

    /// The resynchronisation counter the stream compares against its own.
    #[must_use]
    pub fn resync_requests(&self) -> u32 {
        self.resync.load(Ordering::Acquire)
    }

    /// Whether the stream is past its prefill and playing the console.
    #[must_use]
    pub fn playing(&self) -> bool {
        self.playing.load(Ordering::Relaxed)
    }

    /// Droughts of the console signal while a game was running: the pops this lot is about.
    #[must_use]
    pub fn underruns(&self) -> u64 {
        self.underruns.load(Ordering::Relaxed)
    }

    /// Frames handed to the device since it was opened.
    #[must_use]
    pub fn rendered_frames(&self) -> u64 {
        self.rendered_frames.load(Ordering::Relaxed)
    }

    /// The ring fill the audio thread measured, in frames, averaged over about a second.
    #[must_use]
    pub fn average_fill(&self) -> f32 {
        f32::from_bits(self.fill_average.load(Ordering::Relaxed))
    }

    /// The least and the most the ring held over the last **complete** window, in frames.
    /// `None` until one window has closed.
    ///
    /// The window is double-buffered on purpose: the overlay is drawn every frame, and reading
    /// the window that is still filling would show a minimum and a maximum a few milliseconds
    /// old and all but equal.
    #[must_use]
    pub fn extreme_fill(&self) -> Option<(usize, usize)> {
        let minimum = self.window_minimum.load(Ordering::Relaxed);
        if minimum == u32::MAX {
            return None;
        }
        let maximum = self.window_maximum.load(Ordering::Relaxed);
        Some((minimum as usize, maximum as usize))
    }

    /// Ends the window [`extreme_fill`](Self::extreme_fill) reports and starts another. The host
    /// calls it once a second.
    pub fn close_window(&self) {
        let minimum = self.fill_minimum.swap(u32::MAX, Ordering::Relaxed);
        let maximum = self.fill_maximum.swap(0, Ordering::Relaxed);
        if minimum != u32::MAX {
            self.window_minimum.store(minimum, Ordering::Relaxed);
            self.window_maximum.store(maximum, Ordering::Relaxed);
        }
    }

    fn record_fill(&self, fill: usize, weight: f32) {
        #[allow(clippy::cast_precision_loss)] // a fill is a few thousand frames
        let value = fill as f32;
        let average = self.average_fill();
        let updated = average + ((value - average) * weight);
        self.fill_average.store(bits(updated), Ordering::Relaxed);
        let frames = u32::try_from(fill).unwrap_or(u32::MAX);
        self.fill_minimum.fetch_min(frames, Ordering::Relaxed);
        self.fill_maximum.fetch_max(frames, Ordering::Relaxed);
    }
}

// --------------------------------------------------------------------------- the program ring

/// A ring of interleaved stereo frames, written by the main thread and read by the audio thread.
#[derive(Debug)]
struct StereoRing {
    buffer: Vec<f32>,
    read: usize,
    write: usize,
    available: usize,
}

impl StereoRing {
    fn new(frames: usize) -> Self {
        Self {
            buffer: vec![0.0; frames.max(1) * CHANNELS],
            read: 0,
            write: 0,
            available: 0,
        }
    }

    fn capacity(&self) -> usize {
        self.buffer.len()
    }

    fn clear(&mut self) {
        self.read = 0;
        self.write = 0;
        self.available = 0;
    }

    /// Appends `values`, dropping the oldest on overflow as the APU's ring does
    /// (REQ-AUD-005). The writer keeps to what fits, so that only happens after a clear.
    fn push(&mut self, values: &[f32]) {
        let capacity = self.capacity();
        for &value in values {
            if self.available == capacity {
                self.read = (self.read + 1) % capacity;
                self.available -= 1;
            }
            self.buffer[self.write] = value;
            self.write = (self.write + 1) % capacity;
            self.available += 1;
        }
    }

    /// Reads as much of `out` as it can and returns how many **frames** that was.
    fn pull(&mut self, out: &mut [f32]) -> usize {
        let capacity = self.capacity();
        let values = out.len().min(self.available) / CHANNELS * CHANNELS;
        for slot in out.iter_mut().take(values) {
            *slot = self.buffer[self.read];
            self.read = (self.read + 1) % capacity;
        }
        self.available -= values;
        values / CHANNELS
    }
}

/// The main thread's end of the program ring: the mixer tops it up once per rendered frame.
#[derive(Debug, Clone)]
pub struct ProgramWriter {
    ring: Arc<Mutex<StereoRing>>,
    target_frames: usize,
}

impl ProgramWriter {
    /// Frames the ring is short of its target; what the mixer should produce now.
    #[must_use]
    pub fn missing_frames(&self) -> usize {
        let ring = guard(&self.ring);
        self.target_frames.saturating_sub(ring.available / CHANNELS)
    }

    /// Appends interleaved stereo frames.
    pub fn push(&self, frames: &[f32]) {
        guard(&self.ring).push(frames);
    }

    /// Drops everything waiting: what a volume change big enough to matter, or a language of
    /// the interface sounds changing, would need. Nothing calls it yet; the music tolerates its
    /// own latency.
    pub fn clear(&self) {
        guard(&self.ring).clear();
    }
}

/// The audio thread's end of the program ring.
#[derive(Debug)]
struct ProgramReader {
    ring: Arc<Mutex<StereoRing>>,
}

impl ProgramReader {
    fn pull(&self, out: &mut [f32]) -> usize {
        guard(&self.ring).pull(out)
    }
}

// --------------------------------------------------------------------------- the stream

/// A signal that is held and ramped rather than cut (Q-153 §1).
#[derive(Debug, Clone, Copy)]
struct Envelope {
    level: f32,
    step: f32,
}

impl Envelope {
    const fn new(step: f32) -> Self {
        Self { level: 0.0, step }
    }

    /// Moves one sample towards `target` and returns the level to apply.
    fn advance(&mut self, target: f32) -> f32 {
        if self.level < target {
            self.level = (self.level + self.step).min(target);
        } else if self.level > target {
            self.level = (self.level - self.step).max(target);
        }
        self.level
    }
}

/// What SDL's audio thread runs.
pub struct AudioStream {
    control: Arc<AudioControl>,
    console: ConsoleSlot,
    program: ProgramReader,
    mono: Vec<f32>,
    stereo: Vec<f32>,
    /// Weight of one block in the one-second average of the ring fill.
    fill_weight: f32,
    gain: Envelope,
    console_envelope: Envelope,
    console_hold: f32,
    program_envelope: Envelope,
    program_hold: [f32; CHANNELS],
    /// Frames the ring must hold before the console is read again; 0 while playing.
    prefill: usize,
    seen_resync: u32,
}

impl AudioStream {
    /// Creates the stream the device will pull from, and the handles the main thread keeps.
    ///
    /// `sample_rate` is the device's; `block_frames` the largest block it will ask for, which
    /// sizes the scratch buffers once and for all.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // device rates and block sizes are small integers
    pub fn open(
        sample_rate: u32,
        block_frames: usize,
        target_frames: usize,
    ) -> (Self, Arc<AudioControl>, ConsoleSlot, ProgramWriter) {
        let rate = sample_rate.max(1);
        let rate_f = rate as f32;
        let program_frames = frames_of_ms(rate, PROGRAM_AHEAD_MS);
        let control = Arc::new(AudioControl::new(target_frames));
        let console: ConsoleSlot = Arc::new(Mutex::new(None));
        // Twice what the mixer keeps ahead, so a top-up always fits whole blocks.
        let ring = Arc::new(Mutex::new(StereoRing::new(
            program_frames * 2 + block_frames,
        )));
        let writer = ProgramWriter {
            ring: Arc::clone(&ring),
            target_frames: program_frames,
        };
        let scratch = block_frames.max(1);
        let stream = Self {
            control: Arc::clone(&control),
            console: Arc::clone(&console),
            program: ProgramReader { ring },
            mono: vec![0.0; scratch],
            stereo: vec![0.0; scratch * CHANNELS],
            fill_weight: (scratch as f32 / rate_f).clamp(0.0, 1.0),
            gain: Envelope::new(1.0 / (rate_f * GAIN_SLEW_MS / 1000.0).max(1.0)),
            console_envelope: Envelope::new(1.0 / (rate_f * RAMP_MS / 1000.0).max(1.0)),
            console_hold: 0.0,
            program_envelope: Envelope::new(1.0 / (rate_f * RAMP_MS / 1000.0).max(1.0)),
            program_hold: [0.0; CHANNELS],
            prefill: target_frames.max(scratch),
            seen_resync: 0,
        };
        (stream, control, console, writer)
    }

    /// Fills `out` with interleaved stereo frames. This is the callback's whole job.
    pub fn render_block(&mut self, out: &mut [f32]) {
        let frames = out.len() / CHANNELS;
        if frames == 0 {
            return;
        }
        self.grow(frames);
        self.observe_resync();
        self.render_console(frames);
        self.render_program(frames);
        for (frame, pair) in out.chunks_exact_mut(CHANNELS).take(frames).enumerate() {
            let console = self.mono[frame];
            // The hard clamp of REQ-AUD-006 and SPEC-11 §4 is kept: see the note on headroom
            // in docs/progress/AUDIO-1.md.
            pair[0] = (console + self.stereo[frame * CHANNELS]).clamp(-1.0, 1.0);
            pair[1] = (console + self.stereo[(frame * CHANNELS) + 1]).clamp(-1.0, 1.0);
        }
        self.control
            .rendered_frames
            .fetch_add(frames as u64, Ordering::Relaxed);
    }

    /// A driver that asks for more than the block it negotiated: grow once rather than clip the
    /// request. Allocating on the audio thread is unwelcome but a dropped block is worse, and
    /// it cannot happen twice for the same size.
    fn grow(&mut self, frames: usize) {
        if self.mono.len() < frames {
            self.mono.resize(frames, 0.0);
            self.stereo.resize(frames * CHANNELS, 0.0);
        }
    }

    fn observe_resync(&mut self) {
        let requests = self.control.resync_requests();
        if requests == self.seen_resync {
            return;
        }
        self.seen_resync = requests;
        // The ring was emptied by the caller; wait for it to hold the prefill again. Mid-stream
        // that is half the latency, so that a rollback stays inside the 50 ms REQ-AUD-012 allows
        // at the default latency (Q-153 §4).
        self.prefill = self.resume_prefill();
        self.control.playing.store(false, Ordering::Relaxed);
    }

    /// The fill the ring must reach before the console is read again. Never less than one block,
    /// or the gate would open on too little and under-run on the very next read.
    ///
    /// Half the target only when something is actually being heard — a rollback, a slot loaded
    /// mid-game, a drought — so that the hole stays inside the 50 ms REQ-AUD-012 allows. While
    /// the output is already silent (a game starting, a resume from a pause) the wait costs
    /// nothing audible and the whole target is taken, which is what gives the game its full
    /// margin from its first note (Q-153 §4).
    fn resume_prefill(&self) -> usize {
        let target = self.control.target_frames();
        let audible = self.console_envelope.level > 0.0;
        let wanted = if audible { target / 2 } else { target };
        wanted.max(self.mono.len())
    }

    /// Reads the console into `self.mono`, held and ramped where the ring ran out.
    fn render_console(&mut self, frames: usize) {
        let active = self.control.console_active();
        let target_gain = self.control.console_gain();
        let mut available = 0;
        let mut read = 0;
        let was_gated = self.prefill > 0;
        {
            let mut slot = guard(&self.console);
            if let Some(source) = slot.as_mut() {
                available = source.available();
                if self.prefill > 0 && available >= self.prefill {
                    self.prefill = 0;
                    self.control.playing.store(true, Ordering::Relaxed);
                }
                if self.prefill == 0 && active {
                    read = source.read(&mut self.mono[..frames]);
                }
            }
        }
        if active && self.prefill == 0 && read < frames {
            // A real drought while a game was running: one event, and a wait for the ring to
            // fill again rather than a scramble that would underrun every block.
            self.control.underruns.fetch_add(1, Ordering::Relaxed);
            self.prefill = self.resume_prefill();
            self.control.playing.store(false, Ordering::Relaxed);
        }
        for frame in 0..frames {
            let fed = frame < read;
            if fed {
                self.console_hold = self.mono[frame];
            }
            let wanted = if fed && active { 1.0 } else { 0.0 };
            let envelope = self.console_envelope.advance(wanted);
            let gain = self.gain.advance(target_gain);
            self.mono[frame] = self.console_hold * envelope * gain;
        }
        if active {
            // `available` is what was waiting when the device came for its block — the latency
            // in the plain sense, and the number the user asked for with `audio.latency_ms`.
            // Measuring what is *left* after the read would read a whole block low, and the
            // controller would then hold the ring a block fuller than it was told to.
            //
            // While the gate is shut the average is *set*, not blended: when the gate opens it
            // must already read the real fill, or the rate controller would see a latency a
            // second out of date and take the start of a game for a drift worth resynchronising.
            let weight = if was_gated { 1.0 } else { self.fill_weight };
            self.control.record_fill(available, weight);
        }
    }

    /// Reads the menu music and the interface sounds into `self.stereo`, held and ramped the
    /// same way. A shortfall here is not counted: it is a dip of the programme, not a hole in
    /// the game's sound.
    fn render_program(&mut self, frames: usize) {
        let read = self.program.pull(&mut self.stereo[..frames * CHANNELS]);
        for frame in 0..frames {
            let fed = frame < read;
            if fed {
                self.program_hold[0] = self.stereo[frame * CHANNELS];
                self.program_hold[1] = self.stereo[(frame * CHANNELS) + 1];
            }
            let envelope = self.program_envelope.advance(if fed { 1.0 } else { 0.0 });
            self.stereo[frame * CHANNELS] = self.program_hold[0] * envelope;
            self.stereo[(frame * CHANNELS) + 1] = self.program_hold[1] * envelope;
        }
    }
}

impl AudioRender for AudioStream {
    fn render(&mut self, stereo: &mut [f32]) {
        self.render_block(stereo);
    }
}

/// Room the rate controller needs above the target, in milliseconds: its resynchronisation
/// threshold plus one emulated frame of production.
pub const CONTROL_HEADROOM_MS: f32 = 70.0;

/// The capacity the core gives its output ring at `sample_rate`: at least 10 000 mono samples,
/// otherwise 200 ms (SPEC-02 REQ-CORE-071, `Apu2A03::set_sample_rate`).
///
/// Mirrored here because the target latency has to be decided when the device opens, before any
/// core exists. It is a read of the core's rule, not a change to it.
#[must_use]
pub fn core_ring_samples(sample_rate: u32) -> usize {
    zax_nes::apu::Apu2A03::MIN_BUFFER_SAMPLES.max(sample_rate as usize / 5)
}

/// The target latency the core's ring can really hold, in milliseconds (Q-153 §3).
///
/// The ring is `max(10 000, rate / 5)` mono samples — 208 ms at 48 kHz — and the work order
/// forbids touching the core, so a configured latency above what the ring can hold with room
/// for the controller to work is clamped. At 48 kHz the effective maximum is about 138 ms; a
/// device opened at 96 kHz can use the whole 20–200 ms range.
#[must_use]
pub fn usable_latency_ms(sample_rate: u32, capacity_samples: usize, wanted_ms: f32) -> f32 {
    #[allow(clippy::cast_precision_loss)] // a capacity is a few tens of thousands of samples
    let capacity_ms = ms_of_frames(sample_rate, capacity_samples as f32);
    let ceiling = (capacity_ms - CONTROL_HEADROOM_MS).max(MIN_LATENCY_MS);
    wanted_ms.clamp(MIN_LATENCY_MS, MAX_LATENCY_MS).min(ceiling)
}

/// `milliseconds` of audio at `sample_rate`, in frames, at least one.
#[must_use]
pub fn frames_of_ms(sample_rate: u32, milliseconds: f32) -> usize {
    #[allow(clippy::cast_precision_loss)] // device rates are a few tens of thousands
    let rate = sample_rate.max(1) as f32;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // bounded and positive
    let frames = (rate * milliseconds.max(0.0) / 1000.0) as usize;
    frames.max(1)
}

/// How many milliseconds `frames` last at `sample_rate`.
#[must_use]
pub fn ms_of_frames(sample_rate: u32, frames: f32) -> f32 {
    #[allow(clippy::cast_precision_loss)] // device rates are a few tens of thousands
    let rate = sample_rate.max(1) as f32;
    frames * 1000.0 / rate
}
