//! [`Engine`] for FCEUX's core: [`FceuxEngine`], the handle the application holds.
//!
//! [`Fceux`] is the safe wrapper over the C API and stays exactly that. This file is the layer
//! above it: it holds the little state the trait needs and FCEUX does not keep — the ROM's name,
//! the region as the host asked for it, the frame-counter offset a restored savestate needs, the
//! frame's tile records copied out of the library's buffer, and a ring of audio samples — and it
//! wraps FCEUX's state block in a header that says which engine wrote it.
//!
//! There is no `unsafe` here: every FFI call is in [`crate::core`] behind [`Fceux`].

use std::any::Any;
use std::cell::RefCell;
use std::collections::VecDeque;

use zax_core_api::{
    state_magic, AudioError, Capabilities, ControllerPortType, Engine, EngineKind, FrameBatch,
    InputWord, InvalidPort, InvalidState, NesRegion, NoRomLoaded, PaletteSnapshot, RomLoadFailure,
    RomLoadResult, StateLoadFailure, StateLoaded, TileRecord, MAX_SAMPLE_RATE, MIN_SAMPLE_RATE,
    STANDARD_ROWS,
};

use crate::core::{Error, Fceux, RamInit, Region};

/// The sample rates FCEUX's own `FCEUI_Sound` accepts (`driver.h:152-154`), which is the whole
/// list: anything else is snapped to the nearest of these. See [`FceuxEngine::set_sample_rate`].
const FCEUX_RATES: [i32; 3] = [44_100, 48_000, 96_000];

/// The engine's savestate format, bumped if the header below changes.
const STATE_VERSION: i32 = 1;

/// What the header writes as the core's name, and what a block must carry to be accepted.
const CORE_NAME: &str = "FCEUX";

/// The rate the engine starts at, which is `zax_nes::apu::Apu2A03::DEFAULT_SAMPLE_RATE`
/// (REQ-CORE-070, SPEC-11 REQ-AUD-001) and one FCEUX supports as it is.
const DEFAULT_SAMPLE_RATE: i32 = 48_000;

/// FCEUX's core as an [`Engine`].
///
/// One per process, as [`Fceux`] is: [`FceuxEngine::new`] fails with [`Error::AlreadyLive`] while
/// another exists. `Send` but not `Sync`, inherited from [`Fceux`]'s own `PhantomData`.
#[derive(Debug)]
pub struct FceuxEngine {
    fceux: Fceux,
    /// The name the host loaded the ROM under. FCEUX reports a name of its own only through its
    /// log, so the handle keeps the host's.
    rom_name: Option<String>,
    /// The region as the *host* chose it. FCEUX has no `Multi`, which runs NTSC timing
    /// (REQ-CORE-015), so the host's choice is kept here and the core is given NTSC for it —
    /// otherwise a round trip through `region()` would silently turn `Multi` into `Ntsc`.
    region: NesRegion,
    /// Added to the core's own frame count. FCEUX's counter is the shim's, zeroed on every load,
    /// power and reset and **not** part of a savestate, so a restored state's frame number is
    /// carried in this.
    frame_offset: i64,
    /// The frame's tile records, copied out of the library's buffer once a frame. FCEUX's arrive
    /// through a C callback into memory it owns, so `frame_batch` cannot lend them directly.
    records: Vec<TileRecord>,
    /// The frame's palette (REQ-CORE-045).
    palette: PaletteSnapshot,
    /// Samples waiting for the host, oldest first, in the `-1.0..=1.0` scale `zax-nes` produces.
    ///
    /// Behind a `RefCell` because [`Engine::read_audio`] is `&self` — it is on `zax-nes` too,
    /// whose ring sits behind a `Mutex` for the same reason (its audio thread reads it while
    /// frames run). A cell rather than a mutex here because this handle is `!Sync` by
    /// construction, so there is no second thread to lock against.
    audio: RefCell<VecDeque<f32>>,
    /// One frame's samples as the core hands them over, reused so a frame allocates nothing.
    frame_audio: Vec<i16>,
    /// What [`Engine::sample_rate`] reports: the rate the core really runs at.
    sample_rate: i32,
    /// The core's own savestate block, reused so that a rollback saving every frame allocates
    /// nothing once it has grown. `&self` again, hence the cell.
    payload: RefCell<Vec<u8>>,
}

impl FceuxEngine {
    /// Creates the engine, with the tile capture **on** and the RAM fill set to **zero**.
    ///
    /// Both defaults are deliberate. The capture is what the 3D renderer reads, so an engine the
    /// application holds always has it (the harness's tools, which only want a picture, use
    /// [`Fceux`] directly and pay nothing). The zero fill is what `zax-nes` writes into
    /// uninitialised memory, and lot FCEUX-6 found FCEUX's own `00 00 00 00 FF FF FF FF` pattern
    /// alone worth +6 originals and +15 public ROMs of record differences — so for ordinary play
    /// the two engines start from the same machine. **`.fm2` playback must not use it**: a movie
    /// replays only against the fill its own header records, which is why
    /// [`FceuxEngine::set_ram_init`] exists (lot FCEUX-4).
    ///
    /// # Errors
    ///
    /// [`Error::AlreadyLive`] while another engine exists — there is one FCEUX per process.
    /// [`Error::InitFailed`] if the core refused to initialise, or refused either default.
    pub fn new() -> Result<Self, Error> {
        let mut fceux = Fceux::new()?;
        fceux.set_ram_init(RamInit::Zero)?;
        fceux.set_tile_capture(true);
        // The core starts silent (`FCEUI_Sound` has not been called), where `zax-nes` starts at
        // `Apu2A03::DEFAULT_SAMPLE_RATE`. A host switching engines must not have to notice, so
        // the engine starts at the same 48 kHz (REQ-CORE-070, SPEC-11 REQ-AUD-001).
        fceux.set_sample_rate(DEFAULT_SAMPLE_RATE)?;
        let sample_rate = fceux.sample_rate();

        Ok(Self {
            fceux,
            rom_name: None,
            region: NesRegion::Ntsc,
            frame_offset: 0,
            records: Vec::new(),
            palette: [0; 32],
            audio: RefCell::new(VecDeque::new()),
            frame_audio: Vec::new(),
            sample_rate,
            payload: RefCell::new(Vec::new()),
        })
    }

    /// What the next power-on writes into the uninitialised memory. Set it **before** loading.
    ///
    /// [`RamInit::Zero`] by default, which is `zax-nes`'s fill; a `.fm2` has to be played with
    /// the `RAMInitOption`/`RAMInitSeed` of its own header, or with FCEUX's default fill for a
    /// movie that records neither — **never** with zero (lot FCEUX-4).
    ///
    /// # Errors
    ///
    /// [`Error::BadArgument`] if the core refuses the value.
    pub fn set_ram_init(&mut self, init: RamInit) -> Result<(), Error> {
        self.fceux.set_ram_init(init)
    }

    /// The wrapper underneath, for what the trait does not carry (the emphasis plane, the log,
    /// the version string) and for lot FCEUX-4's movie calls.
    #[must_use]
    pub const fn fceux(&self) -> &Fceux {
        &self.fceux
    }

    /// The same, mutably.
    pub fn fceux_mut(&mut self) -> &mut Fceux {
        &mut self.fceux
    }

    /// The lines the core has printed since they were last taken, which a failed load's message
    /// already carries but which a host may want after any call.
    #[must_use]
    pub fn take_log(&self) -> Vec<String> {
        self.fceux.take_log()
    }

    /// Reads the frame the core just ran into [`Self::records`] and [`Self::palette`], and moves
    /// its samples into the ring.
    fn collect_frame(&mut self) {
        if let Some(capture) = self.fceux.take_tiles() {
            self.records.clear();
            self.records
                .extend(capture.records.iter().map(|tile| TileRecord {
                    plane0: tile.plane0,
                    plane1: tile.plane1,
                    x: tile.x,
                    y: tile.y,
                    index: tile.index,
                    mirror_h: tile.mirror_h,
                    mirror_v: tile.mirror_v,
                    hidden: tile.hidden,
                    palette: tile.palette,
                    background: tile.background,
                }));
            self.palette = capture.palette;
        }

        self.frame_audio.clear();
        if self.fceux.read_audio_into(&mut self.frame_audio).is_err() {
            return;
        }

        // The ring holds what `zax-nes`'s does: enough for a fifth of a second, never less than
        // its own floor, and the oldest sample goes when it is full (SPEC-11 REQ-AUD-005).
        let capacity = Self::ring_capacity(self.sample_rate);
        let Ok(mut audio) = self.audio.try_borrow_mut() else {
            return;
        };
        for &sample in &self.frame_audio {
            if audio.len() >= capacity {
                audio.pop_front();
            }
            // `i16::MIN / 32768` is exactly -1.0, so the whole range maps inside -1.0..=1.0.
            audio.push_back(f32::from(sample) / 32_768.0);
        }
    }

    /// Samples the ring holds at `rate`: `zax-nes`'s own rule, a fifth of a second but never
    /// below the APU's floor of 10 000 (REQ-CORE-071).
    fn ring_capacity(rate: i32) -> usize {
        /// The floor the capacity never goes below, which is the APU's own (REQ-CORE-071).
        const FLOOR: usize = 10_000;
        (usize::try_from(rate).unwrap_or(0) / 5).max(FLOOR)
    }

    /// The typed cause of a load FCEUX refused, read off the image's own header.
    ///
    /// FCEUX reports one failure for every reason, so the three causes REQ-CORE-011 distinguishes
    /// are decided here, from the header alone: no signature is a corrupt header, an image
    /// shorter than the header announces is truncated, and anything else FCEUX refuses is a
    /// mapper it does not have.
    fn load_failure(image: &[u8]) -> RomLoadFailure {
        let Some(header) = image.get(..16) else {
            return RomLoadFailure::CorruptHeader;
        };
        if &header[..4] != b"NES\x1a" || header[4] == 0 {
            return RomLoadFailure::CorruptHeader;
        }

        let trainer = if header[6] & 0x04 == 0 { 0 } else { 512 };
        let prg = usize::from(header[4]) * 16 * 1024;
        let chr = usize::from(header[5]) * 8 * 1024;
        if image.len() < 16 + trainer + prg + chr {
            return RomLoadFailure::TruncatedFile;
        }
        RomLoadFailure::UnsupportedMapper
    }

    /// The region FCEUX runs for `region`: `Multi` is NTSC timing (REQ-CORE-015), and FCEUX has
    /// no `Multi` of its own.
    const fn core_region(region: NesRegion) -> Region {
        match region {
            NesRegion::Ntsc | NesRegion::Multi => Region::Ntsc,
            NesRegion::Pal => Region::Pal,
            NesRegion::Dendy => Region::Dendy,
        }
    }

    /// The other way: what the core says it is running.
    const fn host_region(region: Region) -> NesRegion {
        match region {
            Region::Ntsc => NesRegion::Ntsc,
            Region::Pal => NesRegion::Pal,
            Region::Dendy => NesRegion::Dendy,
        }
    }

    /// Clears everything that belongs to one power-on: the frame counter, the frame just
    /// captured and the samples nobody read.
    ///
    /// The counter is restarted by *offset* rather than by trusting the core: FCEUX's shim zeroes
    /// its own counter on a load and a power cycle but **not** on the RESET line, where
    /// AMD-044 REQ-CORE-122 has the frame counter restart from 0 as `zax-nes` does. Computing the
    /// offset from whatever the core reports makes all four paths agree without the fork having
    /// to.
    fn forget_frame_state(&mut self) {
        self.frame_offset = -self.fceux.frame_count();
        self.records.clear();
        self.palette = [0; 32];
        if let Ok(mut audio) = self.audio.try_borrow_mut() {
            audio.clear();
        }
        let _ = self.fceux.take_tiles();
    }
}

// ------------------------------------------------------------------------------------------------
// The savestate header
// ------------------------------------------------------------------------------------------------
//
// FCEUX's own block is opaque (`FCSX`, written by `FCEUSS_SaveMS`). The header below wraps it so
// that a block is self-describing in the three ways the application needs, which §3 of
// `docs/review/FCEUX-ENGINE.md` asks for: it says **which engine** wrote it, in the same first
// four bytes a Zaxnes block uses for the same purpose (`zax_core_api::state_magic`), so a block of
// the other engine is refused with `ForeignCore` and the spec's own message rather than with
// "corrupt"; it carries the **ROM name** and the **frame count**, which `StateLoaded` reports and
// FCEUX does not keep; and it carries its own **length**, so a host may place its own data right
// after the block and `consumed` tells it where (REQ-CORE-091).
//
// The encoding is the flat little-endian one `zax_nes::state` uses — fixed-width integers, and a
// string or a blob as an `i32` count then its bytes — written by hand here rather than shared,
// because the two formats are each engine's own and only this header is common ground.

/// Appends a little-endian `i32` count and then `bytes`.
fn put_blob(out: &mut Vec<u8>, bytes: &[u8]) {
    let count = i32::try_from(bytes.len()).unwrap_or(0);
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(bytes);
}

/// A cursor over a block, which refuses to read past its end.
struct Reader<'a> {
    data: &'a [u8],
    at: usize,
    origin: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8], offset: usize) -> Result<Self, InvalidState> {
        if offset > data.len() {
            return Err(InvalidState::corrupt(format!(
                "Corrupt savestate: start position {offset} is outside the {}-byte array.",
                data.len()
            )));
        }
        Ok(Self {
            data,
            at: offset,
            origin: offset,
        })
    }

    fn raw(&mut self, count: usize) -> Result<&'a [u8], InvalidState> {
        let end = self.at.checked_add(count).ok_or_else(|| {
            InvalidState::corrupt("Corrupt savestate: a length overflows the block.")
        })?;
        let bytes = self.data.get(self.at..end).ok_or_else(|| {
            InvalidState::corrupt(format!(
                "Corrupt savestate: {count} bytes were expected at {}, only {} remain.",
                self.at,
                self.data.len().saturating_sub(self.at)
            ))
        })?;
        self.at = end;
        Ok(bytes)
    }

    fn u32(&mut self) -> Result<u32, InvalidState> {
        let bytes = self.raw(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn i32(&mut self) -> Result<i32, InvalidState> {
        let bytes = self.raw(4)?;
        Ok(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn u8(&mut self) -> Result<u8, InvalidState> {
        Ok(self.raw(1)?[0])
    }

    fn i64(&mut self) -> Result<i64, InvalidState> {
        let bytes = self.raw(8)?;
        let mut value = [0_u8; 8];
        value.copy_from_slice(bytes);
        Ok(i64::from_le_bytes(value))
    }

    fn blob(&mut self) -> Result<&'a [u8], InvalidState> {
        let count = self.i32()?;
        let count = usize::try_from(count).map_err(|_| {
            InvalidState::corrupt(format!("Corrupt savestate: negative count {count}."))
        })?;
        self.raw(count)
    }

    fn string(&mut self) -> Result<String, InvalidState> {
        Ok(String::from_utf8_lossy(self.blob()?).into_owned())
    }

    const fn consumed(&self) -> usize {
        self.at - self.origin
    }
}

impl Engine for FceuxEngine {
    // --------------------------------------------------------------- the game

    fn load_rom(&mut self, image: &[u8], name: &str) -> RomLoadResult {
        self.rom_name = None;
        self.forget_frame_state();

        if let Err(error) = self.fceux.load_rom(image, name) {
            self.fceux.close_rom();
            return RomLoadResult::fail(Self::load_failure(image), error.to_string());
        }

        self.rom_name = Some(name.to_owned());
        // FCEUX applies the image's own region on load, so what it reports now *is* the detected
        // one. REQ-CORE-013 has the host choose; it does so by calling `set_region` next, and
        // until it does the engine runs what the ROM asked for — which is what `zax-nes` also
        // ends up doing, by a different route.
        let detected = self
            .fceux
            .region()
            .map_or(NesRegion::Ntsc, Self::host_region);
        self.region = detected;

        let log = self.fceux.take_log();
        let note = (!log.is_empty()).then(|| log.join("; "));
        match note {
            Some(note) => RomLoadResult::ok_with_note(detected, note),
            None => RomLoadResult::ok(detected),
        }
    }

    fn close_rom(&mut self) {
        self.fceux.close_rom();
        self.rom_name = None;
        self.forget_frame_state();
    }

    fn is_running(&self) -> bool {
        self.fceux.has_rom()
    }

    fn rom_name(&self) -> Option<&str> {
        self.rom_name.as_deref()
    }

    fn power(&mut self) {
        if self.fceux.power().is_ok() {
            self.forget_frame_state();
        }
    }

    /// FCEUX's `zax_power` is the power cycle, which is what a hard reset is on `zax-nes` too
    /// (DEC-02-Q3); its `zax_reset` is the RESET line, which is [`Engine::soft_reset`].
    fn reset(&mut self) {
        self.power();
    }

    fn soft_reset(&mut self) {
        if self.fceux.reset().is_ok() {
            self.forget_frame_state();
        }
    }

    // ---------------------------------------------------------------- a frame

    fn run_frame(&mut self, input: InputWord) -> i64 {
        if !self.fceux.has_rom() {
            return 0;
        }
        if self.fceux.set_input_word(input).is_err() || self.fceux.run_frame().is_err() {
            return 0;
        }

        self.collect_frame();

        // FCEUX reports no cycle count. The frame's nominal CPU cycles for the region are the
        // honest answer: the application uses the figure for the audio ratio and the frame
        // profiler, never for emulation, and a game that steals cycles would make the exact
        // number a lie on either engine. NTSC 29 780.5 is reported as 29 781 and PAL 33 247.5 as
        // 33 248, the ceiling, which is what `zax-nes` averages to over two frames.
        match self.region {
            NesRegion::Pal | NesRegion::Dendy => 33_248,
            NesRegion::Ntsc | NesRegion::Multi => 29_781,
        }
    }

    fn frame_count(&self) -> i64 {
        self.fceux.frame_count().saturating_add(self.frame_offset)
    }

    fn set_frame_count(&mut self, value: i64) {
        self.frame_offset = value.saturating_sub(self.fceux.frame_count());
    }

    fn pixel_frame_indices_into(&self, out: &mut Vec<u8>) {
        if self.fceux.frame_indices_into(out).is_err() {
            // Before the first frame, or with no ROM: a black picture of the right size, which is
            // what `zax-nes` has in its own buffer at that point.
            out.clear();
            out.resize(crate::FRAME_LEN, 0);
        }
    }

    fn frame_batch(&self) -> FrameBatch<'_> {
        FrameBatch {
            records: &self.records,
            palette: self.palette,
            // Everything below is the widescreen extension or the margin decoders, which are
            // Zaxnes's bus; `Capabilities::FCEUX` says so and these stay at their standard
            // values. `mask_x`/`mask_y` are 0 because FCEUX's picture has no unreliable band of
            // its own to declare (REQ-CORE-046).
            margin_palette: None,
            mask_x: 0,
            mask_y: 0,
            margin: 0,
            visible_rows: STANDARD_ROWS,
            margin_y: 0,
        }
    }

    // ----------------------------------------------------------------- audio

    fn read_audio(&self, destination: &mut [f32], stereo: bool) -> Result<usize, AudioError> {
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
        let Ok(mut audio) = self.audio.try_borrow_mut() else {
            return Ok(0);
        };
        // Entirely or not at all, as REQ-CORE-071 has it on the other engine.
        if audio.len() < needed {
            return Ok(0);
        }

        for (index, sample) in audio.drain(..needed).enumerate() {
            if stereo {
                destination[2 * index] = sample;
                destination[(2 * index) + 1] = sample;
            } else {
                destination[index] = sample;
            }
        }
        Ok(destination.len())
    }

    fn sample_rate(&self) -> i32 {
        self.sample_rate
    }

    /// FCEUX's `FCEUI_Sound` takes only 44 100, 48 000, 96 000 and 0 (`driver.h:152-154`), where
    /// `zax-nes` resamples to anything from 22 050 to 96 000. A rate outside *that* range is
    /// refused exactly as `zax-nes` refuses it; a rate inside it that FCEUX cannot produce is
    /// **snapped to the nearest one it can**, and [`Engine::sample_rate`] then reports the rate
    /// the core really runs at rather than the one that was asked for — so a host that feeds its
    /// audio device from `sample_rate()` stays in tune. Silence (0) is passed through.
    fn set_sample_rate(&mut self, value: i32) -> Result<(), AudioError> {
        if value != 0 && !(MIN_SAMPLE_RATE..=MAX_SAMPLE_RATE).contains(&value) {
            return Err(AudioError::SampleRateOutOfRange(value));
        }

        let chosen = if value == 0 {
            0
        } else {
            FCEUX_RATES
                .into_iter()
                .min_by_key(|rate| (rate - value).abs())
                .unwrap_or(48_000)
        };

        if self.fceux.set_sample_rate(chosen).is_err() {
            return Err(AudioError::SampleRateOutOfRange(value));
        }
        self.sample_rate = self.fceux.sample_rate();
        if let Ok(mut audio) = self.audio.try_borrow_mut() {
            audio.clear();
        }
        Ok(())
    }

    // ---------------------------------------------------------------- memory

    fn read_memory(&self, address: u16) -> u8 {
        self.fceux.read_memory(address).unwrap_or(0)
    }

    fn write_memory(&mut self, address: u16, value: u8) {
        let _ = self.fceux.write_memory(address, value);
    }

    // ------------------------------------------------------------ savestates

    fn save_state_to(
        &self,
        include_rom_image: bool,
        buffer: &mut Vec<u8>,
    ) -> Result<(), NoRomLoaded> {
        if !self.fceux.has_rom() {
            return Err(NoRomLoaded);
        }

        // The core's own block, into the reused scratch so that `buffer` is untouched on an
        // error and a rollback saving every frame allocates nothing once it has grown.
        let Ok(mut payload) = self.payload.try_borrow_mut() else {
            return Err(NoRomLoaded);
        };
        if self.fceux.save_state_into(&mut payload).is_err() {
            return Err(NoRomLoaded);
        }

        buffer.clear();
        buffer.extend_from_slice(&state_magic(EngineKind::Fceux).to_le_bytes());
        buffer.extend_from_slice(&STATE_VERSION.to_le_bytes());
        put_blob(buffer, CORE_NAME.as_bytes());
        // `include_rom_image` is answered honestly as "no", whatever was asked: FCEUX loads a
        // ROM from memory and its state block holds the machine, not the cartridge, so a block of
        // this engine is never self-contained in `zax-nes`'s sense. A reader therefore knows not
        // to expect an image rather than finding an empty one where it expected bytes. The flag
        // and the blob are in the format so that a later lot can fill them without changing it;
        // `restore` already reads both.
        let _ = include_rom_image;
        buffer.push(0);
        put_blob(buffer, self.rom_name.as_deref().unwrap_or("").as_bytes());
        put_blob(buffer, &[]);
        buffer.push(self.region as u8);
        buffer.extend_from_slice(&self.frame_count().to_le_bytes());
        put_blob(buffer, &payload);
        Ok(())
    }

    fn max_state_size(&self) -> usize {
        // The core's measured bound, plus the header: two integers, three length prefixes, the
        // core name, the ROM name at four bytes a character, the region byte and the frame
        // counter, with room to spare.
        let core = self.fceux.state_size_hint().unwrap_or(0);
        let name = self.rom_name.as_deref().map_or(0, |name| name.len() * 4);
        core + name + 256
    }

    fn try_load_state(&mut self, data: &[u8], offset: usize) -> Result<StateLoaded, InvalidState> {
        let outcome = self.restore(data, offset);
        if outcome.is_err() {
            // As `zax-nes` does: a refused block leaves the engine stopped rather than half
            // restored (REQ-CORE-005), because FCEUX's own loader may have got part way.
            self.close_rom();
        }
        outcome
    }

    // ----------------------------------------------------------- battery RAM

    fn export_battery_ram(&self, out: &mut Vec<u8>) {
        if self.fceux.battery_ram_into(out).is_err() {
            out.clear();
        }
    }

    fn import_battery_ram(&mut self, data: &[u8]) -> bool {
        self.fceux.set_battery_ram(data).unwrap_or(false)
    }

    // --------------------------------------------------------------- machine

    fn region(&self) -> NesRegion {
        self.region
    }

    fn set_region(&mut self, region: NesRegion) {
        if self.fceux.set_region(Self::core_region(region)).is_ok() {
            self.region = region;
        }
    }

    /// FCEUX's two ports always hold a standard pad in this build: the fork's C API offers
    /// `zax_set_input` and nothing else, so there is no Zapper to report.
    fn port_type(&self, port: usize) -> Result<ControllerPortType, InvalidPort> {
        if port < zax_core_api::PORT_COUNT {
            Ok(ControllerPortType::Controller)
        } else {
            Err(InvalidPort(port))
        }
    }

    /// A pad is accepted on either port; a Zapper is refused, because this engine has none.
    fn set_port_type(&mut self, port: usize, kind: ControllerPortType) -> Result<(), InvalidPort> {
        if port >= zax_core_api::PORT_COUNT {
            return Err(InvalidPort(port));
        }
        match kind {
            ControllerPortType::Controller => Ok(()),
            ControllerPortType::Zapper => Err(InvalidPort(port)),
        }
    }

    // ---------------------------------------------------------- capabilities

    fn capabilities(&self) -> Capabilities {
        Capabilities::FCEUX
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl FceuxEngine {
    /// The body of [`Engine::try_load_state`], so that the "leave it stopped on a failure" rule
    /// is in one place.
    fn restore(&mut self, data: &[u8], offset: usize) -> Result<StateLoaded, InvalidState> {
        let mut reader = Reader::new(data, offset)?;

        let magic = reader.u32()?;
        if magic == state_magic(EngineKind::Zaxnes) {
            // The clear refusal §3 of the design note asks for: this is Zaxnes's own block, not a
            // corrupt one, and the message is the one REQ-CORE-120 fixes to the character.
            return Err(InvalidState {
                failure: StateLoadFailure::ForeignCore,
                message: zax_core_api::FOREIGN_CORE_MESSAGE.to_owned(),
            });
        }
        if magic != state_magic(EngineKind::Fceux) {
            return Err(InvalidState::corrupt(
                "Corrupt savestate: wrong magic number.",
            ));
        }

        let version = reader.i32()?;
        if version != STATE_VERSION {
            return Err(InvalidState::corrupt(format!(
                "Corrupt savestate: format version {version} is not supported (expected \
                 {STATE_VERSION})."
            )));
        }

        let core_name = reader.string()?;
        if core_name != CORE_NAME {
            return Err(InvalidState {
                failure: StateLoadFailure::ForeignCore,
                message: zax_core_api::FOREIGN_CORE_MESSAGE.to_owned(),
            });
        }

        let _has_rom_image = reader.u8()? != 0;
        let rom_name = reader.string()?;
        let _rom_image = reader.blob()?;
        let region_byte = reader.u8()?;
        let region = NesRegion::from_u8(region_byte).ok_or_else(|| {
            InvalidState::corrupt(format!("Corrupt savestate: unknown region {region_byte}."))
        })?;
        let frame_count = reader.i64()?;
        let payload = reader.blob()?;
        let consumed = reader.consumed();

        if !self.fceux.has_rom() {
            // FCEUX cannot load a state without the game: its block holds the machine, not the
            // cartridge. `zax-nes`'s self-contained block can, which is the one place the two
            // formats really differ, and the application's slot wrapper keeps the ROM anyway.
            return Err(InvalidState::corrupt(
                "This savestate needs its game loaded first: FCEUX's states do not carry the ROM.",
            ));
        }

        self.fceux
            .load_state(payload)
            .map_err(|error| InvalidState::corrupt(format!("Corrupt savestate: {error}")))?;

        self.records.clear();
        self.palette = [0; 32];
        if let Ok(mut audio) = self.audio.try_borrow_mut() {
            audio.clear();
        }
        let _ = self.fceux.take_tiles();
        self.region = region;
        let _ = self.fceux.set_region(Self::core_region(region));
        self.frame_offset = frame_count.saturating_sub(self.fceux.frame_count());
        if !rom_name.is_empty() {
            self.rom_name = Some(rom_name.clone());
        }

        Ok(StateLoaded { consumed, rom_name })
    }
}
