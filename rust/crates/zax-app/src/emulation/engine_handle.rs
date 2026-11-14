//! [`EngineHandle`]: the application's one way of holding an emulation engine.
//!
//! The application used to hold `Box<NesCore>`. Since lot FCEUX-23 step 4 it holds this, which
//! wraps a `Box<dyn Engine>` and adds, in **one place**, every Zaxnes-only operation the
//! application makes — each one asking [`Capabilities`] first and giving a stated answer when the
//! engine does not have it.
//!
//! # Why a façade and not scattered downcasts
//!
//! §2 of `docs/review/FCEUX-ENGINE.md` splits what the application asks of a core in two: what
//! both engines can do, which is the [`Engine`] trait, and what is *Zaxnes's machine* rather than
//! an emulator's — the widescreen extension, the hack packs, the sprite limit, the exact
//! run-ahead, the bus and the PPU. The second list is reached here through
//! [`EngineHandle::zaxnes`] / [`EngineHandle::zaxnes_mut`], a checked `Any` downcast: safe Rust,
//! no `unsafe`, and no panic on the other engine. Putting all of them in this file means a reader
//! can see what FCEUX does instead of each one without reading the whole application, and means
//! the call sites did not have to change: the trait's methods arrive through [`Deref`], the
//! Zaxnes-only ones are inherent methods here, and an inherent method wins over a `Deref` one.
//!
//! # The default build
//!
//! With the `fceux-core` feature off, `Box<dyn Engine>` can only ever hold a `NesCore`: the other
//! implementation is not compiled. Every `zaxnes()` below then succeeds, every fallback is dead
//! at run time, and the application behaves exactly as it did — which golden-core, the render
//! comparison and `bench-core` are measured against in `docs/progress/FCEUX-23.md` step 4.

use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};

use zax_core_api::{Capabilities, Capability, Engine, EngineKind, InputWord, NoRomLoaded};
use zax_nes::apu::AudioReader;
use zax_nes::builtin_patches::{BuiltinPatch, HackOption};
use zax_nes::cartridges::Cartridge;
use zax_nes::ppu::{Ppu2C02, SpriteLimit};
use zax_nes::state::{CoreState, InvalidState};
use zax_nes::{NesBus, NesCore, RegionTiming, ZapperPosition, ZapperPositionSource};

/// The rows of a standard NES picture, which is what the widescreen-less picture has.
const STANDARD_ROWS: usize = 240;

/// The version of FCEUX's own savestate block. Named here rather than reached through the crate
/// so that the figure is one line to find in a build without the engine.
#[cfg(feature = "fceux-core")]
const FCEUX_BLOCK_VERSION: i32 = zax_fceux::BLOCK_VERSION;
#[cfg(not(feature = "fceux-core"))]
const FCEUX_BLOCK_VERSION: i32 = 0;

/// One emulation engine, as the application holds it.
pub struct EngineHandle {
    engine: Box<dyn Engine>,
    capabilities: Capabilities,
    /// The image the host handed to [`EngineHandle::load_rom`], kept so that an engine which
    /// does not keep one — FCEUX — can still answer [`EngineHandle::original_image`], which the
    /// achievements' ROM hash and the save-state header need.
    loaded_image: Option<Vec<u8>>,
    /// CPU cycles the frames run have taken, accumulated from [`Engine::run_frame`]'s return so
    /// that an engine without a cycle counter still answers [`EngineHandle::cycle_count`].
    cycles: i64,
    /// Where [`EngineHandle::pump_audio`] puts the frame's samples for the audio thread to read,
    /// on an engine that has no ring of its own to share. `None` on Zaxnes, whose APU ring *is*
    /// shared and is used unchanged.
    shared_audio: Option<Arc<Mutex<SharedRing>>>,
}

/// A ring of samples the main thread fills a frame at a time and the audio thread drains, for an
/// engine whose own samples are behind a `!Sync` handle.
///
/// The rules are `zax-nes`'s (SPEC-11 REQ-AUD-005, REQ-CORE-071): the oldest sample goes when it
/// is full, and a read takes what it can rather than all or nothing — `ConsoleSamples::read` is
/// documented to return a partial read, which is what lets the stream hold and fade instead of
/// emitting a block of zeros.
#[derive(Debug, Default)]
pub struct SharedRing {
    samples: std::collections::VecDeque<f32>,
}

impl SharedRing {
    /// A fifth of a second at 96 kHz, the ceiling of what one engine can hand over before the
    /// audio thread has read any of it.
    const CAPACITY: usize = 19_200;

    fn write(ring: &Arc<Mutex<Self>>, samples: &[f32]) {
        let Ok(mut ring) = ring.lock() else {
            return;
        };
        for &sample in samples {
            if ring.samples.len() >= Self::CAPACITY {
                ring.samples.pop_front();
            }
            ring.samples.push_back(sample);
        }
    }

    fn read_into(ring: &Arc<Mutex<Self>>, out: &mut [f32]) -> usize {
        let Ok(mut ring) = ring.lock() else {
            return 0;
        };
        let count = ring.samples.len().min(out.len());
        for (slot, sample) in out.iter_mut().zip(ring.samples.drain(..count)) {
            *slot = sample;
        }
        count
    }

    fn available(ring: &Arc<Mutex<Self>>) -> usize {
        ring.lock().map_or(0, |ring| ring.samples.len())
    }
}

/// [`crate::audio::ConsoleSamples`] over a [`SharedRing`].
struct SharedRingSamples(Arc<Mutex<SharedRing>>);

impl crate::audio::ConsoleSamples for SharedRingSamples {
    fn available(&self) -> usize {
        SharedRing::available(&self.0)
    }

    fn read(&mut self, out: &mut [f32]) -> usize {
        SharedRing::read_into(&self.0, out)
    }
}

impl std::fmt::Debug for EngineHandle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The engine itself is not `Debug` and the kept image is a megabyte of ROM, so this
        // prints what identifies the handle: which engine, and whether it has a game.
        formatter
            .debug_struct("EngineHandle")
            .field("engine", &self.capabilities.kind)
            .field("running", &self.engine.is_running())
            .field("cycles", &self.cycles)
            .field("image", &self.loaded_image.as_ref().map(Vec::len))
            .field("shared_audio", &self.shared_audio.is_some())
            .finish()
    }
}

impl EngineHandle {
    /// A handle over `engine`.
    #[must_use]
    pub fn new(engine: Box<dyn Engine>) -> Self {
        let capabilities = engine.capabilities();
        // Zaxnes's APU ring is already shared with the audio thread and is used unchanged; an
        // engine that keeps its samples behind a `!Sync` handle gets one of ours.
        let shared_audio = (capabilities.kind != EngineKind::Zaxnes)
            .then(|| Arc::new(Mutex::new(SharedRing::default())));
        Self {
            engine,
            capabilities,
            loaded_image: None,
            cycles: 0,
            shared_audio,
        }
    }

    /// A handle over a fresh Zaxnes core, which is what every caller that does not choose an
    /// engine gets — and the only engine a build without `fceux-core` has.
    #[must_use]
    pub fn zaxnes_core() -> Self {
        Self::new(Box::new(NesCore::new()))
    }

    /// A handle over a fresh engine of `kind` (lot FCEUX-23 step 5).
    ///
    /// Falls back to Zaxnes's core, with a line in the log saying why, when the engine asked for
    /// cannot be made: a build without the `fceux-core` feature has only one engine, and FCEUX
    /// keeps one machine per process so a second handle is refused while the first lives. The
    /// fallback is never silent, and it is a fallback rather than a failure because a game the
    /// user asked for must still start.
    #[must_use]
    pub fn of_kind(kind: EngineKind) -> Self {
        match kind {
            EngineKind::Zaxnes => Self::zaxnes_core(),
            EngineKind::Fceux => Self::fceux_core(),
            EngineKind::NesHawk => Self::neshawk_core(),
        }
    }

    /// A handle over `zax-neshawk`, the Rust port of BizHawk's NesHawk core (lot
    /// NESHAWK-REWRITE).
    ///
    /// No feature gate and no fallback: the crate is safe Rust in every build, keeps no
    /// process-global state, and so cannot refuse to be created the way FCEUX can.
    ///
    /// The tile capture is turned **on**, because the records are what the 3D renderer reads —
    /// the harness's tools, which only want a picture, use `zax_neshawk::Nes` directly and pay
    /// nothing for it.
    fn neshawk_core() -> Self {
        let mut engine = zax_neshawk::NesHawkEngine::new();
        engine.set_tile_capture(true);
        if !engine.has_gamedb() {
            crate::app_log::line(
                "Zaxnes: the NesHawk engine has no copy of BizHawk's gamedb (set                  ZAXNES_BIZHAWK_GAMEDB), so it picks a cartridge's board from the iNES header                  alone. For some cartridges that is a different board, and so different timing,                  from the one BizHawk picks.",
            );
        }
        Self::new(Box::new(engine))
    }

    #[cfg(feature = "fceux-core")]
    fn fceux_core() -> Self {
        match zax_fceux::FceuxEngine::new() {
            Ok(engine) => Self::new(Box::new(engine)),
            Err(error) => {
                crate::app_log::error(&format!(
                    "Zaxnes: the FCEUX engine could not be created ({error}); this game plays on \
                     Zaxnes's own core."
                ));
                Self::zaxnes_core()
            }
        }
    }

    #[cfg(not(feature = "fceux-core"))]
    fn fceux_core() -> Self {
        crate::app_log::line(
            "Zaxnes: this build has no FCEUX engine; the game plays on Zaxnes's own core.",
        );
        Self::zaxnes_core()
    }

    /// The samples of this engine, for the audio thread's console slot — `None` on an engine
    /// whose samples nothing can read from another thread.
    ///
    /// On Zaxnes this is the APU's own ring behind [`zax_nes::apu::AudioReader`], exactly the
    /// path of before this lot. On FCEUX the samples are in the handle, which is `!Sync`, so
    /// they are pumped into a shared ring once a frame by [`EngineHandle::pump_audio`] and this
    /// reads that.
    #[must_use]
    pub fn console_samples(&self) -> Option<Box<dyn crate::audio::ConsoleSamples>> {
        if let Some(reader) = self.audio_reader() {
            return Some(Box::new(crate::audio::ApuSamples::new(reader)));
        }
        self.shared_audio
            .clone()
            .map(|ring| Box::new(SharedRingSamples(ring)) as Box<dyn crate::audio::ConsoleSamples>)
    }

    /// The same samples as a plain closure, for the classic output's mixer source.
    #[must_use]
    pub fn audio_source(&self) -> Option<crate::audio::audio_mixer::AudioSampleSource> {
        if let Some(reader) = self.audio_reader() {
            return Some(Box::new(move |destination| {
                // A mono read cannot be refused; an empty buffer reads as silence.
                reader.read_samples(destination, false).unwrap_or(0)
            }));
        }
        let ring = self.shared_audio.clone()?;
        Some(Box::new(move |destination| {
            SharedRing::read_into(&ring, destination)
        }))
    }

    /// Moves the frame's samples out of the engine into the shared ring, for an engine that keeps
    /// them where another thread cannot read them. A no-op on Zaxnes, whose ring *is* shared.
    ///
    /// Called by [`EngineHandle::run_frame`] and [`EngineHandle::run_frame_ahead`] rather than by
    /// the session, so that it cannot be missed on one of the session's several frame paths. On
    /// Zaxnes it is one `Option::is_none` and a return.
    fn pump_audio(&mut self) {
        let Some(ring) = self.shared_audio.clone() else {
            return;
        };
        // One frame is about 800 samples at 48 kHz; the engine's own read is all-or-nothing, so
        // the chunk is asked for in halves down to a sample while anything is left.
        let mut chunk = [0.0_f32; 1024];
        loop {
            let mut taken = 0;
            let mut wanted = chunk.len();
            while wanted > 0 {
                match self.engine.read_audio(&mut chunk[..wanted], false) {
                    Ok(0) => wanted /= 2,
                    Ok(count) => {
                        taken = count;
                        break;
                    }
                    Err(_) => return,
                }
            }
            if taken == 0 {
                return;
            }
            SharedRing::write(&ring, &chunk[..taken]);
        }
    }

    /// The engine behind the handle, for a caller that wants the trait and nothing else.
    #[must_use]
    pub fn engine(&self) -> &dyn Engine {
        self.engine.as_ref()
    }

    /// The same, mutably.
    pub fn engine_mut(&mut self) -> &mut dyn Engine {
        self.engine.as_mut()
    }

    /// Which engine this is, for the marker the interface shows and for a savestate's stamp.
    #[must_use]
    pub const fn kind(&self) -> EngineKind {
        self.capabilities.kind
    }

    /// What this engine can do beyond the trait.
    #[must_use]
    pub const fn capabilities(&self) -> Capabilities {
        self.capabilities
    }

    /// Whether `capability` is supported, which is the check every method below makes.
    #[must_use]
    pub const fn has(&self, capability: Capability) -> bool {
        self.capabilities.has(capability)
    }

    /// Why a control is disabled on this engine, or `None` when it is not.
    #[must_use]
    pub const fn refusal(&self, capability: Capability) -> Option<&'static str> {
        self.capabilities.refusal(capability)
    }

    // ------------------------------------------------------------- the downcast

    /// Zaxnes's own core, or `None` on another engine.
    #[must_use]
    pub fn zaxnes(&self) -> Option<&NesCore> {
        self.engine.as_any().downcast_ref::<NesCore>()
    }

    /// The same, mutably.
    pub fn zaxnes_mut(&mut self) -> Option<&mut NesCore> {
        self.engine.as_any_mut().downcast_mut::<NesCore>()
    }

    // ------------------------------------------------------------------ the game

    /// Loads a ROM and keeps the image, so that [`EngineHandle::original_image`] can answer on
    /// an engine that does not keep one.
    pub fn load_rom(&mut self, image: &[u8], name: &str) -> zax_core_api::RomLoadResult {
        self.cycles = 0;
        let result = self.engine.load_rom(image, name);
        self.loaded_image = result.success.then(|| image.to_vec());
        result
    }

    /// The image the host handed over, as the achievements' ROM hash and the save-state header
    /// want it: on Zaxnes the core's own answer (the unpatched original when a built-in patch
    /// changed the ROM in memory), on another engine the image this handle kept.
    #[must_use]
    pub fn original_image(&self) -> Option<&[u8]> {
        match self.zaxnes() {
            Some(core) => core.original_image(),
            None => self.loaded_image.as_deref(),
        }
    }

    /// The cartridge, `None` on an engine that does not expose one.
    #[must_use]
    pub fn cartridge(&self) -> Option<&Cartridge> {
        self.zaxnes().and_then(NesCore::cartridge)
    }

    /// The bus, `None` on an engine that does not expose one.
    #[must_use]
    pub fn bus(&self) -> Option<&NesBus> {
        self.zaxnes().map(NesCore::bus)
    }

    /// The same, mutably.
    pub fn bus_mut(&mut self) -> Option<&mut NesBus> {
        self.zaxnes_mut().map(NesCore::bus_mut)
    }

    /// The PPU, `None` on an engine that does not expose one.
    #[must_use]
    pub fn ppu(&self) -> Option<&Ppu2C02> {
        self.zaxnes().map(NesCore::ppu)
    }

    /// Timings of the region in force (REQ-CORE-015). Derived from the region, so both engines
    /// answer it.
    #[must_use]
    pub fn timing(&self) -> RegionTiming {
        RegionTiming::for_region(self.engine.region())
    }

    /// CPU cycles since the last power-on. Zaxnes counts them; another engine's are the sum of
    /// what its frames reported, which is what the audio ratio and the frame profiler use them
    /// for.
    #[must_use]
    ///
    /// **Not comparable across a state restore on an engine without a counter of its own** (lot
    /// FCEUX-48 step 4): the sum is of the frames this *session* has run, so a movie seek that
    /// replays two hundred frames a second time leaves it two hundred frames higher than an
    /// uninterrupted run at the same frame. What it is used for — the audio ratio and the frame
    /// profiler — wants exactly that; comparing two paths to the same frame does not.
    pub fn cycle_count(&self) -> i64 {
        match self.zaxnes() {
            Some(core) => core.cycle_count(),
            None => self.cycles,
        }
    }

    /// The work RAM, `$0000-$07FF`, copied into `out` — the snapshot the scripts read from while
    /// the emulation thread has the engine. Read through the engine on a core that does not lend
    /// its RAM.
    pub fn work_ram_into(&self, out: &mut [u8]) {
        match self.bus() {
            Some(bus) => out.copy_from_slice(bus.ram()),
            None => {
                for (address, byte) in out.iter_mut().enumerate() {
                    *byte = u16::try_from(address)
                        .map_or(0, |address| self.engine.read_memory(address));
                }
            }
        }
    }

    // ------------------------------------------------------------------ a frame

    /// One frame (REQ-CORE-020). An inherent method rather than the trait's through [`Deref`],
    /// so that the cycle total [`EngineHandle::cycle_count`] falls back on is kept whichever way
    /// a frame was run.
    pub fn run_frame(&mut self, word: InputWord) -> i64 {
        let cycles = self.engine.run_frame(word);
        self.cycles = self.cycles.saturating_add(cycles);
        self.pump_audio();
        cycles
    }

    /// One frame with `ahead` frames of run-ahead (AMD-041 REQ-LOOP-065), which is exact on
    /// Zaxnes. An engine without run-ahead runs the plain frame: lot FCEUX-5 is what would make
    /// it save, run N, take the frame and load, and until then the interface shows the control
    /// disabled with that reason rather than running it slowly and silently.
    pub fn run_frame_ahead(&mut self, word: InputWord, ahead: u8) -> i64 {
        let cycles = match self.zaxnes_mut() {
            Some(core) => core.run_frame_ahead(word, ahead),
            None => self.engine.run_frame(word),
        };
        self.cycles = self.cycles.saturating_add(cycles);
        self.pump_audio();
        cycles
    }

    /// Whether the engine still holds the real frame a run-ahead computed past; `false` on an
    /// engine without run-ahead, which never does.
    #[must_use]
    pub fn holds_a_real_frame(&self) -> bool {
        self.zaxnes().is_some_and(NesCore::holds_a_real_frame)
    }

    /// Puts the real frame back after a run-ahead; `false` when there was none.
    pub fn restore_real_frame(&mut self) -> bool {
        self.zaxnes_mut().is_some_and(NesCore::restore_real_frame)
    }

    /// The frame's picture widened by its widescreen margins, into `out`; the number of rows.
    /// An engine without the widescreen extension gives the plain 256 x 240 picture, which is
    /// what its frames are.
    ///
    /// The 2D mode draws only the pixels whose opacity bit is set (REQ-CORE-049, REQ-SCN-076),
    /// and only Zaxnes's PPU keeps that bit. Another engine's picture is plain colour indices, so
    /// every pixel is marked opaque: the whole picture is drawn, its backdrop colour included,
    /// rather than nothing at all (owner report, 2026-11-12: FCEUX and NesHawk showed only the
    /// background colour in 2D).
    pub fn wide_picture_into(&self, out: &mut Vec<u8>) -> usize {
        if let Some(core) = self.zaxnes() {
            return core.wide_picture_into(out);
        }
        self.engine.pixel_frame_indices_into(out);
        for pixel in out.iter_mut() {
            *pixel = (*pixel & zax_nes::ppu::PIXEL_COLOR) | zax_nes::ppu::OPAQUE_PIXEL;
        }
        STANDARD_ROWS
    }

    /// The frame's experimental smooth-scrolling shift; `None` on an engine that has none.
    #[must_use]
    pub fn picture_shift(&self) -> Option<(f32, i32, i32)> {
        self.zaxnes().and_then(NesCore::picture_shift)
    }

    /// The frame's camera motion to the next one; `None` on an engine that has none.
    #[must_use]
    pub fn picture_motion(&self) -> Option<f32> {
        self.zaxnes().and_then(NesCore::picture_motion)
    }

    /// The camera position the frame's own scroll implies; `None` on an engine that has none.
    #[must_use]
    pub fn picture_camera(&self) -> Option<f64> {
        self.zaxnes().and_then(NesCore::picture_camera)
    }

    /// Moves the sprites that travel with the camera out of `picture` into `overlay`. An engine
    /// without the split leaves the picture alone and clears the overlay, which is what "no
    /// sprite was moved out" means to the caller.
    pub fn split_steady_sprites(&self, picture: &mut [u8], overlay: &mut Vec<u8>) {
        match self.zaxnes() {
            Some(core) => core.split_steady_sprites(picture, overlay),
            None => overlay.clear(),
        }
    }

    /// Whether the frame's margin decoder asks the top and bottom 8 lines hidden; `false` on an
    /// engine without the decoders.
    #[must_use]
    pub fn hides_overscan(&self) -> bool {
        self.zaxnes().is_some_and(NesCore::hides_overscan)
    }

    /// The picture with its opacity mask, which the movie timeline hashes; `None` on an engine
    /// that does not keep one.
    #[must_use]
    pub fn pixel_frame_with_mask(&self) -> Option<&[u8]> {
        self.zaxnes().map(NesCore::pixel_frame_with_mask)
    }

    // -------------------------------------------------------------------- audio

    /// The APU, `None` on an engine that does not expose one.
    #[must_use]
    pub fn apu(&self) -> Option<&zax_nes::apu::Apu2A03> {
        self.zaxnes().map(NesCore::apu)
    }

    /// Throws away the samples of frames already run, so that none of a previous game or of a
    /// seek's frames is ever replayed (REQ-AUD-010). A no-op on an engine whose samples the host
    /// has already taken frame by frame.
    pub fn drop_pending_samples(&mut self) {
        if let Some(core) = self.zaxnes_mut() {
            core.bus_mut().apu_mut().drop_pending_samples();
        }
        if let Some(ring) = &self.shared_audio {
            if let Ok(mut ring) = ring.lock() {
                ring.samples.clear();
            }
        }
    }

    /// Silences or unsilences the output while frames run that the player must not hear — a
    /// movie seek. Likewise a no-op where the host takes the samples itself.
    pub fn set_audio_output_enabled(&mut self, on: bool) {
        if let Some(core) = self.zaxnes_mut() {
            core.bus_mut().apu_mut().set_output_enabled(on);
        }
    }

    /// Mono samples waiting to be read (REQ-CORE-071), which the audio clock steers on.
    #[must_use]
    pub fn audio_available(&self) -> usize {
        if let Some(apu) = self.apu() {
            return apu.available_samples();
        }
        self.shared_audio.as_ref().map_or(0, SharedRing::available)
    }

    /// The speed factor in force (REQ-CORE-072); 1.0 on an engine without one.
    #[must_use]
    pub fn speed(&self) -> f64 {
        self.zaxnes().map_or(1.0, NesCore::speed)
    }

    /// The fine resampling-ratio adjustment in force; 0.0 on an engine without a resampler.
    #[must_use]
    pub fn audio_ratio_adjustment(&self) -> f64 {
        self.zaxnes().map_or(0.0, NesCore::audio_ratio_adjustment)
    }

    /// A handle the host's audio thread can read samples from (REQ-CORE-071); `None` on an engine
    /// whose samples are not in a shared ring.
    #[must_use]
    pub fn audio_reader(&self) -> Option<AudioReader> {
        self.zaxnes().map(NesCore::audio_reader)
    }

    /// The speed factor, which only retimes the audio and never the emulation (REQ-CORE-072).
    ///
    /// # Errors
    ///
    /// As `NesCore::set_speed`. An engine with no speed control accepts the call and ignores it:
    /// it produces one frame's samples per frame whatever the factor says.
    pub fn set_speed(&mut self, value: f64) -> Result<(), zax_core_api::AudioError> {
        match self.zaxnes_mut() {
            Some(core) => core.set_speed(value),
            None => Ok(()),
        }
    }

    /// The fine resampling-ratio adjustment the audio clock asks for.
    ///
    /// # Errors
    ///
    /// As `NesCore::set_audio_ratio_adjustment`; ignored on an engine without a resampler of its
    /// own, for the reason above.
    pub fn set_audio_ratio_adjustment(
        &mut self,
        value: f64,
    ) -> Result<(), zax_core_api::AudioError> {
        match self.zaxnes_mut() {
            Some(core) => core.set_audio_ratio_adjustment(value),
            None => Ok(()),
        }
    }

    // -------------------------------------------------------------- savestates

    /// The engine's name, which a savestate slot records so that it can refuse a state written by
    /// the other engine (lot FCEUX-23 step 5). On Zaxnes it is `NesCore::CORE_NAME`, the string
    /// its slots have always carried.
    #[must_use]
    pub const fn state_core_id(&self) -> &'static str {
        zax_core_api::state_core_name(self.capabilities.kind)
    }

    /// The version of the engine's own block format, recorded beside the name.
    #[must_use]
    pub const fn state_version(&self) -> i32 {
        match self.capabilities.kind {
            EngineKind::Zaxnes => NesCore::STATE_VERSION,
            EngineKind::Fceux => FCEUX_BLOCK_VERSION,
            EngineKind::NesHawk => zax_neshawk::engine::BLOCK_VERSION,
        }
    }

    /// A savestate as a new vector, which is what every caller outside the rollback history
    /// wants. Both engines answer it, through [`Engine::save_state_to`].
    ///
    /// # Errors
    ///
    /// [`NoRomLoaded`] when no ROM is loaded.
    pub fn save_state(&self, include_rom_image: bool) -> Result<Vec<u8>, NoRomLoaded> {
        let mut block = Vec::new();
        self.engine.save_state_to(include_rom_image, &mut block)?;
        Ok(block)
    }

    /// A savestate taken at a CPU instruction boundary where the machine is idle, with whether it
    /// found one. An engine without the idle search answers its ordinary state and `false`.
    ///
    /// # Errors
    ///
    /// [`NoRomLoaded`] when no ROM is loaded.
    pub fn save_state_at_idle(
        &mut self,
        include_rom_image: bool,
    ) -> Result<(Vec<u8>, bool), NoRomLoaded> {
        match self.zaxnes_mut() {
            Some(core) => core.save_state_at_idle(include_rom_image),
            None => Ok((self.save_state(include_rom_image)?, false)),
        }
    }

    /// The in-memory snapshot of AMD-041 REQ-CORE-121 — the cheap path rollback uses on Zaxnes.
    /// `false` on an engine without it, and [`Capability::FastCoreSnapshot`] says which.
    ///
    /// # Errors
    ///
    /// [`NoRomLoaded`] when no ROM is loaded.
    pub fn capture_core_state(&self, state: &mut CoreState) -> Result<bool, NoRomLoaded> {
        match self.zaxnes() {
            Some(core) => core.capture_core_state(state).map(|()| true),
            None => Ok(false),
        }
    }

    /// Reads a snapshot back; `false` on an engine without the fast path.
    ///
    /// # Errors
    ///
    /// [`InvalidState`] when the buffer does not fit the machine.
    pub fn restore_core_state(&mut self, state: &CoreState) -> Result<bool, InvalidState> {
        match self.zaxnes_mut() {
            Some(core) => core.restore_core_state(state).map(|()| true),
            None => Ok(false),
        }
    }

    /// Upper bound on a snapshot's size; 0 on an engine without the fast path.
    #[must_use]
    pub fn max_core_state_size(&self) -> usize {
        self.zaxnes().map_or(0, NesCore::max_core_state_size)
    }

    /// Which built-in components a savestate of this machine carries, for the slot's header;
    /// empty on an engine with no built-in patches.
    #[must_use]
    pub fn applied_components(&self) -> Vec<&'static str> {
        self.zaxnes()
            .map(NesCore::applied_components)
            .unwrap_or_default()
    }

    // --------------------------------------------------------- flash and battery

    /// The battery-backed PRG-RAM to persist (REQ-CORE-095).
    ///
    /// Borrowed on Zaxnes, which lends it — so the default build copies nothing, as it did
    /// before this lot; owned on an engine whose bytes come across a C API.
    #[must_use]
    pub fn battery_ram(&self) -> std::borrow::Cow<'_, [u8]> {
        if let Some(core) = self.zaxnes() {
            return std::borrow::Cow::Borrowed(core.export_battery_ram());
        }
        let mut bytes = Vec::new();
        self.engine.export_battery_ram(&mut bytes);
        std::borrow::Cow::Owned(bytes)
    }

    /// A flash chip's changed sectors, for a host that persists them; empty on an engine without
    /// a flash cartridge of its own.
    #[must_use]
    pub fn export_flash_save(&self) -> Vec<u8> {
        self.zaxnes()
            .map(NesCore::export_flash_save)
            .unwrap_or_default()
    }

    /// Restores them; `false` on an engine without them.
    pub fn import_flash_save(&mut self, data: &[u8]) -> bool {
        match self.zaxnes_mut() {
            Some(core) => core.import_flash_save(data),
            None => false,
        }
    }

    // ------------------------------------------------- the widescreen extension

    /// The widescreen margin the host offers, in NES pixels a side; ignored on an engine without
    /// the extension, whose frames stay 256 wide.
    pub fn set_widescreen_margin(&mut self, pixels: i32) {
        if let Some(core) = self.zaxnes_mut() {
            core.set_widescreen_margin(pixels);
        }
    }

    /// The same above and below.
    pub fn set_widescreen_vertical_margin(&mut self, pixels: i32) {
        if let Some(core) = self.zaxnes_mut() {
            core.set_widescreen_vertical_margin(pixels);
        }
    }

    /// The margin in force; 0 on an engine without the extension.
    #[must_use]
    pub fn widescreen_margin(&self) -> i32 {
        self.zaxnes().map_or(0, NesCore::widescreen_margin)
    }

    /// The smooth scrolling of the run-ahead (`emu.smooth_scroll_ahead`); ignored without it.
    pub fn set_smooth_scroll_ahead(&mut self, on: bool) {
        if let Some(core) = self.zaxnes_mut() {
            core.set_smooth_scroll_ahead(on);
        }
    }

    /// The 8-sprites-per-line limit of the picture; ignored on an engine whose PPU is not ours.
    pub fn set_sprite_limit(&mut self, limit: SpriteLimit) {
        if let Some(core) = self.zaxnes_mut() {
            core.set_sprite_limit(limit);
        }
    }

    // ---------------------------------------------------------- the hack packs

    /// Whether the built-in patches and the advanced ones are on for the next load; both off on
    /// an engine they cannot patch.
    #[must_use]
    pub fn patch_switches(&self) -> (bool, bool) {
        self.zaxnes()
            .map_or((false, false), NesCore::patch_switches)
    }

    /// Whether the savestate block at `offset` of `data` fits the built-in patches as they are
    /// set; `None` on an engine without them or for a block it cannot read.
    #[must_use]
    pub fn state_fits_builtin_patches(&self, data: &[u8], offset: usize) -> Option<bool> {
        self.zaxnes()?.state_fits_builtin_patches(data, offset)
    }

    /// The cartridge's CHR-RAM, which a "3dh" profile's tiles are looked for in (AMD-062
    /// REQ-PROF-080); empty on an engine that does not lend it.
    #[must_use]
    pub fn chr_ram(&self) -> &[u8] {
        self.zaxnes().map_or(&[], NesCore::chr_ram)
    }

    /// Whether the built-in patches apply at all; ignored on an engine they cannot patch.
    pub fn set_builtin_patches(&mut self, enabled: bool) {
        if let Some(core) = self.zaxnes_mut() {
            core.set_builtin_patches(enabled);
        }
    }

    /// Whether the cartridge database corrects iNES 1.0 headers at the next load (lot CARTDB);
    /// ignored on another engine, which reads headers its own way.
    pub fn set_cartridge_db(&mut self, enabled: bool) {
        if let Some(core) = self.zaxnes_mut() {
            core.set_cartridge_db(enabled);
        }
    }

    /// Whether the PPU corrupts a row of OAM when the rendering is switched off inside a line
    /// (lot FIDELITY); likewise.
    pub fn set_oam_corruption(&mut self, enabled: bool) {
        if let Some(core) = self.zaxnes_mut() {
            core.set_oam_corruption(enabled);
        }
    }

    /// Whether a `$2007` read or a `$2006` write that collides with a fetch of the PPU disturbs
    /// it (lot FIDELITY); likewise.
    pub fn set_bus_conflicts(&mut self, enabled: bool) {
        if let Some(core) = self.zaxnes_mut() {
            core.set_bus_conflicts(enabled);
        }
    }

    /// Whether the 2C02G/H's OAMADDR copy when rendering starts is emulated (lot FIDELITY);
    /// likewise.
    pub fn set_oamaddr_bug(&mut self, enabled: bool) {
        if let Some(core) = self.zaxnes_mut() {
            core.set_oamaddr_bug(enabled);
        }
    }

    /// Whether OAM decays with the rendering off (lot FIDELITY); likewise.
    pub fn set_oam_decay(&mut self, enabled: bool) {
        if let Some(core) = self.zaxnes_mut() {
            core.set_oam_decay(enabled);
        }
    }

    /// `emu.ppu_2006_delay` (study access-phase); Zaxnes only.
    pub fn set_address_load_delay(&mut self, enabled: bool) {
        if let Some(core) = self.zaxnes_mut() {
            core.set_address_load_delay(enabled);
        }
    }

    /// `emu.ppu_2006_glitch` (study access-phase); Zaxnes only.
    pub fn set_glitch_2006(&mut self, enabled: bool) {
        if let Some(core) = self.zaxnes_mut() {
            core.set_glitch_2006(enabled);
        }
    }

    /// `emu.ppu_2000_glitch` (study access-phase); Zaxnes only.
    pub fn set_glitch_2000(&mut self, enabled: bool) {
        if let Some(core) = self.zaxnes_mut() {
            core.set_glitch_2000(enabled);
        }
    }

    /// `emu.palette_corruption` (lot FIDELITY step 32); Zaxnes only.
    pub fn set_palette_corruption(&mut self, enabled: bool) {
        if let Some(core) = self.zaxnes_mut() {
            core.set_palette_corruption(enabled);
        }
    }

    /// `emu.cpu_ppu_alignment` (lot FIDELITY step 28); Zaxnes only.
    pub fn set_cpu_ppu_alignment(&mut self, alignment: u8) {
        if let Some(core) = self.zaxnes_mut() {
            core.set_cpu_ppu_alignment(alignment);
        }
    }

    /// Whether a power-on puts the PPU at the top of the picture (lot FIDELITY); likewise.
    pub fn set_power_on_top(&mut self, enabled: bool) {
        if let Some(core) = self.zaxnes_mut() {
            core.set_power_on_top(enabled);
        }
    }

    /// The console audio profile of `audio.console_profile` (lot FIDELITY); Zaxnes only.
    pub fn set_console_audio_profile(&mut self, index: usize) {
        if let Some(core) = self.zaxnes_mut() {
            core.set_console_audio_profile(index);
        }
    }

    /// What the work RAM holds at the next power-on (lot CARTDB); likewise.
    pub fn set_power_on_ram(&mut self, pattern: zax_nes::PowerOnRam) {
        if let Some(core) = self.zaxnes_mut() {
            core.set_power_on_ram(pattern);
        }
    }

    /// Whether the advanced widescreen patches apply too; likewise.
    pub fn set_advanced_patches(&mut self, enabled: bool) {
        if let Some(core) = self.zaxnes_mut() {
            core.set_advanced_patches(enabled);
        }
    }

    /// The patch to force on a ROM none is made for; likewise.
    pub fn set_forced_patch(&mut self, patch: Option<&'static BuiltinPatch>) {
        if let Some(core) = self.zaxnes_mut() {
            core.set_forced_patch(patch);
        }
    }

    /// The built-in patch applied to the ROM loaded; `None` on an engine without them.
    #[must_use]
    pub fn builtin_patch(&self) -> Option<&'static str> {
        self.zaxnes().and_then(NesCore::builtin_patch)
    }

    /// The pack whose options the game offers; `None` on an engine without them.
    #[must_use]
    pub fn options_patch(&self) -> Option<&'static str> {
        self.zaxnes().and_then(NesCore::options_patch)
    }

    /// Those options; empty on an engine without them.
    #[must_use]
    pub fn hack_options(&self) -> Vec<HackOption> {
        self.zaxnes().map(NesCore::hack_options).unwrap_or_default()
    }

    /// The option bits the pack reads at `$500A`; ignored without a pack.
    pub fn set_hack_options(&mut self, options: u16) {
        if let Some(core) = self.zaxnes_mut() {
            core.set_hack_options(options);
        }
    }

    // ------------------------------------------------------------- the Zapper

    /// Where the Zapper is aimed, asked once a frame; ignored on an engine without one, whose
    /// ports only ever hold a pad.
    pub fn set_zapper_position_source(&mut self, source: Option<ZapperPositionSource>) {
        if let Some(core) = self.zaxnes_mut() {
            core.set_zapper_position_source(source);
        }
    }

    /// The aim of the gun on `port`; off screen on an engine without one.
    #[must_use]
    pub fn zapper_position(&self, port: usize) -> ZapperPosition {
        self.bus()
            .and_then(|bus| bus.input().zapper_position(port).ok())
            .unwrap_or(ZapperPosition::OFF_SCREEN)
    }

    /// The byte the scripts read for `port`'s buttons; 0 on an engine that does not lend its
    /// ports.
    #[must_use]
    pub fn port_byte(&self, port: usize) -> u8 {
        self.bus().map_or(0, |bus| bus.input().port_byte(port))
    }
}

/// The trait's own methods, so that every call site that only needs them reads as it did.
impl Deref for EngineHandle {
    type Target = dyn Engine;

    fn deref(&self) -> &Self::Target {
        self.engine.as_ref()
    }
}

impl DerefMut for EngineHandle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.engine.as_mut()
    }
}

// ------------------------------------------------------------------------------------------------
// Movies the engine plays itself (lot FCEUX-48 step 4)
// ------------------------------------------------------------------------------------------------

/// What a `.fm2` header says, as much of it as the application needs.
///
/// A record of plain values rather than the engine's own type, for the same reason the rest of
/// this file exists: no FCEUX type may appear in a build without `fceux-core`, and the
/// application must read the same shape whichever engine answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnMovieHeader {
    /// Input records, which is the movie's length in frames.
    pub frames: u32,
    /// The region the movie was recorded at, which playback puts in force.
    pub region: zax_core_api::NesRegion,
    /// The movie starts from a savestate it carries rather than from power-on.
    pub from_savestate: bool,
    /// The header's re-record count, for a host that shows it.
    pub rerecord_count: i32,
    /// Whether the movie's stated ROM checksum is the loaded ROM's; `None` when it states none.
    pub rom_matches: Option<bool>,
}

/// The movie operations an engine may have of its own.
///
/// **Two whole definitions, one per build**, rather than one body with a `cfg` block inside it:
/// a build without `fceux-core` must name no FCEUX type at all, and a `cfg` inside a function
/// leaves an early `return` that reads as dead code in the other build. It is the idiom
/// `EngineHandle::fceux_core` above already uses.
#[cfg(feature = "fceux-core")]
impl EngineHandle {
    /// Whether this engine plays `.fm2` movies **itself**, through a movie mode of its own.
    ///
    /// True on FCEUX and false on Zaxnes, and the difference is not a gap: a movie only replays
    /// on the engine that recorded it, and FCEUX's movie mode is the only way to honour a
    /// movie's `RAMInitSeed` at all (§5 of `docs/review/FCEUX-ENGINE.md`). Zaxnes plays an
    /// `.fm2` through its own clean-room importer instead, which is a different and older path
    /// the application keeps unchanged.
    #[must_use]
    pub fn plays_its_own_movies(&self) -> bool {
        self.own_movie_engine().is_some()
    }

    /// Reads a `.fm2` header **without touching the machine**; `None` on an engine that plays no
    /// movie of its own, `Some(Err)` with the reason when the movie cannot be played.
    ///
    /// The error text is the engine's own message, which names the one thing the movie needs and
    /// the build has not, so a host can say which rather than "unsupported".
    #[must_use]
    pub fn inspect_own_movie(&self, movie: &[u8]) -> Option<Result<OwnMovieHeader, String>> {
        let engine = self.own_movie_engine()?;
        Some(match engine.inspect_movie(movie) {
            Ok(header) => own_header(&header).ok_or_else(|| {
                header
                    .refusal()
                    .map_or_else(|| "the movie was refused".to_owned(), |why| why.to_string())
            }),
            Err(error) => Err(error.to_string()),
        })
    }

    /// Plays a `.fm2` through the engine's own movie mode, from the movie's own beginning: the
    /// machine is re-powered with the region and the RAM fill the header asks for, so the frame
    /// counter starts again at 0.
    ///
    /// `None` on an engine that plays no movie of its own; `Some(Err)` with the reason on a
    /// refusal, in which case **the machine is left exactly as it was**.
    pub fn play_own_movie(&mut self, movie: &[u8]) -> Option<Result<OwnMovieHeader, String>> {
        self.cycles = 0;
        let engine = self.own_movie_engine_mut()?;
        Some(match engine.play_movie(movie) {
            Ok(header) => own_header(&header).ok_or_else(|| "the movie was refused".to_owned()),
            Err(error) => Err(error.to_string()),
        })
    }

    /// Stops the engine's own movie, if it has one; the machine goes on from where it is.
    pub fn stop_own_movie(&mut self) {
        if let Some(engine) = self.own_movie_engine_mut() {
            let _ = engine.stop_movie();
        }
    }

    /// The movie frame the next [`zax_core_api::Engine::run_frame`] will play and the movie's
    /// length, in **that** order — the host's own convention, which is the reverse of the
    /// engine's. `None` with no movie of the engine's own loaded.
    ///
    /// The frame counts **past** the length while the movie stays loaded, so "over" is
    /// `frame >= length` (`zax_fceux::Fceux::movie_position` says why).
    #[must_use]
    pub fn own_movie_position(&self) -> Option<(u32, u32)> {
        let (length, frame) = self.own_movie_engine()?.movie_position()?;
        Some((frame, length))
    }

    /// Judges a `.fm2` against `image` **without disturbing anything**, on a throw-away engine of
    /// the kind that plays its own movies: the ROM is loaded into it, the header read and judged,
    /// and the engine dropped before this returns.
    ///
    /// That is what lets a host refuse a movie *before* it switches a player's game to another
    /// engine — which re-powers the machine, so a refusal decided afterwards would have cost the
    /// game. `Err` carries the engine's own message, which names the one thing that is wrong.
    ///
    /// `None` only in a build with no such engine. `Some(Err(…))` when an engine of that kind is
    /// already alive, in which case the caller holds it and should ask it with
    /// [`EngineHandle::inspect_own_movie`] instead: there is nothing left to lose by then.
    #[must_use]
    pub fn judge_own_movie(
        image: &[u8],
        name: &str,
        movie: &[u8],
    ) -> Option<Result<OwnMovieHeader, String>> {
        Some(match zax_fceux::judge_movie(image, name, movie) {
            Ok(header) => own_header(&header).ok_or_else(|| "the movie was refused".to_owned()),
            Err(error) => Err(error.to_string()),
        })
    }

    /// Whether the message of [`EngineHandle::judge_own_movie`] is about the **ROM not loading on
    /// that engine** rather than about the movie.
    ///
    /// The two call for different answers: a movie this engine cannot play is refused, while a
    /// *game* it cannot load is a reason to fall back to the other engine's own reader. The
    /// distinction is made here so that the host does not have to know the engine's error type.
    #[must_use]
    pub fn judgement_is_about_the_rom(message: &str) -> bool {
        message.starts_with("FCEUX refused the ROM image")
    }

    /// FCEUX's engine, or `None` on another one. The mirror of [`EngineHandle::zaxnes`], and
    /// only for the handful of operations FCEUX has and Zaxnes has not.
    fn own_movie_engine(&self) -> Option<&zax_fceux::FceuxEngine> {
        self.engine
            .as_any()
            .downcast_ref::<zax_fceux::FceuxEngine>()
    }

    /// The same, mutably.
    fn own_movie_engine_mut(&mut self) -> Option<&mut zax_fceux::FceuxEngine> {
        self.engine
            .as_any_mut()
            .downcast_mut::<zax_fceux::FceuxEngine>()
    }
}

/// The same five, in a build whose only engine is Zaxnes's own core: there is no engine with a
/// movie mode of its own, so every one of them is a constant. The application's `.fm2` path then
/// never leaves Zaxnes's clean-room importer, which is the behaviour of before this lot.
#[cfg(not(feature = "fceux-core"))]
impl EngineHandle {
    /// Always false: this build has one engine and it plays no movie of its own.
    #[must_use]
    pub const fn plays_its_own_movies(&self) -> bool {
        false
    }

    /// Always `None`.
    #[must_use]
    pub const fn inspect_own_movie(&self, _movie: &[u8]) -> Option<Result<OwnMovieHeader, String>> {
        None
    }

    /// Always `None`.
    #[must_use]
    pub const fn play_own_movie(
        &mut self,
        _movie: &[u8],
    ) -> Option<Result<OwnMovieHeader, String>> {
        None
    }

    /// Nothing to stop.
    pub const fn stop_own_movie(&mut self) {}

    /// Always `None`.
    #[must_use]
    pub const fn own_movie_position(&self) -> Option<(u32, u32)> {
        None
    }

    /// Always `None`: this build has no engine that plays its own movies to judge one with.
    #[must_use]
    pub const fn judge_own_movie(
        _image: &[u8],
        _name: &str,
        _movie: &[u8],
    ) -> Option<Result<OwnMovieHeader, String>> {
        None
    }

    /// Never, there being no such judgement to classify.
    #[must_use]
    pub const fn judgement_is_about_the_rom(_message: &str) -> bool {
        false
    }
}

/// The engine's header as the application reads it, or `None` when the header is one the engine
/// would refuse — which `play_own_movie` has already checked, and `inspect_own_movie` turns into
/// the named reason.
#[cfg(feature = "fceux-core")]
fn own_header(header: &zax_fceux::MovieHeader) -> Option<OwnMovieHeader> {
    if header.refusal().is_some() {
        return None;
    }
    Some(OwnMovieHeader {
        frames: header.frames,
        region: match header.region {
            zax_fceux::Region::Pal => zax_core_api::NesRegion::Pal,
            zax_fceux::Region::Dendy => zax_core_api::NesRegion::Dendy,
            zax_fceux::Region::Ntsc => zax_core_api::NesRegion::Ntsc,
        },
        from_savestate: header.from_savestate,
        rerecord_count: header.rerecord_count,
        rom_matches: header.rom_matches,
    })
}
