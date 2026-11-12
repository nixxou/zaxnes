//! [`Engine`] for the NesHawk port: [`NesHawkEngine`], the handle the application holds.
//!
//! [`Nes`] is the port of the machine and stays exactly that — it knows nothing of this trait.
//! This file is the layer above it: the little state the trait needs and `NES.Core.cs` does not
//! keep (the ROM's name and image, the host's region, the frame-counter offset a restored
//! savestate needs, the frame's tile records, a ring of audio samples), and the block header that
//! says which engine wrote a savestate.
//!
//! # The three places this engine is not the Zaxnes engine
//!
//! 1. **No widescreen, no hack packs, no machine internals** ([`Capabilities::NESHAWK`]). Those
//!    are patches to *Zaxnes's* bus, not features of an emulator; §2 of
//!    `docs/review/FCEUX-ENGINE.md` settled that for FCEUX and the same holds here.
//! 2. **Audio is resampled here, not by a `blip_buf`.** See [`NesHawkEngine::read_audio`].
//! 3. **Board detection wants BizHawk's `gamedb`.** Without it a cartridge whose iNES header
//!    disagrees with NesCartDB gets a *different board* — which for an MMC3 cartridge means a
//!    different IRQ edge. [`NesHawkEngine::set_gamedb_dir`] and `ZAXNES_BIZHAWK_GAMEDB` are how
//!    the host supplies it, and the load logs a line when it is missing rather than pretending.

use std::any::Any;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use zax_core_api::{
    state_core_name, state_magic, AudioError, Capabilities, ControllerPortType, Engine, EngineKind,
    FrameBatch, InputWord, InvalidPort, InvalidState, NesRegion, NoRomLoaded, PaletteSnapshot,
    RomLoadFailure, RomLoadResult, StateLoadFailure, StateLoaded, TileRecord, MAX_SAMPLE_RATE,
    MIN_SAMPLE_RATE, STANDARD_ROWS,
};

use crate::controllers::PortDevice;
use crate::gamedb::GameDb;
use crate::nes::{DisplayType, LoadError, Nes};
use crate::state::{StateReader, StateWriter};

/// The engine's savestate format version, bumped if the header or the field order changes.
const STATE_VERSION: i32 = 1;

/// What the header writes as the core's name, and what a block must carry to be accepted.
const CORE_NAME: &str = state_core_name(EngineKind::NesHawk);

/// The format version of this engine's own block, which a savestate slot records beside the name.
pub const BLOCK_VERSION: i32 = STATE_VERSION;

/// The rate the engine starts at: `zax_nes::apu::Apu2A03::DEFAULT_SAMPLE_RATE`, so that a host
/// switching engines does not have to notice (REQ-CORE-070, SPEC-11 REQ-AUD-001).
const DEFAULT_SAMPLE_RATE: i32 = 48_000;

/// Pixels in a frame: `256 * 240`.
const FRAME_LEN: usize = 256 * 240;

/// `ZXNH`, then the version, the flags, the board kind, the frame count, the region, the rom
/// name and the payload length — the fixed part of the header, before the two length-prefixed
/// strings.
const HEADER_FIXED: usize = 4 + 4 + 1 + 1 + 8 + 1 + 4;

/// Bit 0 of the header's flags byte: the block carries the frame buffer (a state the user saved,
/// not a rollback slot). Bit 1: the block carries the ROM image.
const FLAG_FULL: u8 = 1 << 0;
const FLAG_ROM_IMAGE: u8 = 1 << 1;

/// The NesHawk port as an [`Engine`].
///
/// `Send` and deliberately not `Sync`, as the trait requires: the machine moves to the
/// application's emulation thread, and nothing reads it from a second thread. Unlike FCEUX there
/// may be **any number** of these — the port keeps no process-global state at all, which is one
/// of the things safe Rust bought.
pub struct NesHawkEngine {
    /// The machine. `None` when no ROM is loaded: the port has no "empty NES", because
    /// `NES.HardReset` builds the board from the cartridge.
    nes: Option<Nes>,
    /// The image the machine was loaded from, which a hard reset and a self-contained savestate
    /// both need.
    image: Vec<u8>,
    /// BizHawk's game databases, loaded once per engine rather than per ROM.
    db: GameDb,
    /// Where [`Self::db`] came from, `None` when detection is on the iNES header alone.
    gamedb_dir: Option<PathBuf>,
    /// The name the host loaded the ROM under.
    rom_name: Option<String>,
    /// The region as the *host* chose it, which is not always what the machine runs: NesHawk has
    /// no `Multi`, so that choice runs NTSC timing (REQ-CORE-015) and is remembered here, or a
    /// round trip through [`Engine::region`] would silently turn `Multi` into `Ntsc`.
    region: NesRegion,
    /// What is plugged into each port, which the deck is rebuilt from on a load.
    ports: [ControllerPortType; 2],
    /// Added to the machine's own frame counter, which a restored savestate sets.
    frame_offset: i64,
    /// Whether the 3D tile capture runs. Kept here as well as on the machine because a load or
    /// a power-on builds a new machine, whose capture is off by default.
    capture_tiles: bool,
    /// `NESSyncSettings.InitialWRamStatePattern`, kept here for the same reason: `HardReset`
    /// reads it, and a load builds a new machine. Empty means NesHawk's own default, which is
    /// FCEUX's `00 00 00 00 FF FF FF FF`.
    wram_pattern: Vec<u8>,
    /// The frame's tile records (REQ-CORE-030), copied off the machine once a frame.
    records: Vec<TileRecord>,
    /// The frame's palette (REQ-CORE-045).
    palette: PaletteSnapshot,
    /// Samples waiting for the host, oldest first, in the `-1.0..=1.0` scale `zax-nes` produces.
    ///
    /// Behind a `RefCell` because [`Engine::read_audio`] is `&self`, as it is on both other
    /// engines. A cell rather than a mutex because this handle is `!Sync`.
    audio: RefCell<VecDeque<f32>>,
    /// What [`Engine::sample_rate`] reports.
    sample_rate: i32,
    /// The resampler's state, carried across frames: the APU's current output level, the
    /// `sampleclock` it was last integrated to, and how much of the output sample being built is
    /// filled. See [`Self::read_audio`].
    audio_level: i32,
    audio_last_clock: u32,
    audio_accumulated: f64,
    audio_filled: f64,
}

impl std::fmt::Debug for NesHawkEngine {
    /// Written out because `Nes` cannot derive `Debug`: a `Box<dyn Board>` has no `Debug`, and
    /// adding it as a supertrait would be a bound the port does not need. What a reader of a log
    /// wants is the identity of the machine, not its 84 PPU fields.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NesHawkEngine")
            .field("rom_name", &self.rom_name)
            .field("board", &self.board_name())
            .field("running", &self.is_running())
            .field("region", &self.region)
            .field("frame", &self.frame_count())
            .field("gamedb", &self.gamedb_dir)
            .field("tile_capture", &self.capture_tiles)
            .field("sample_rate", &self.sample_rate)
            .finish_non_exhaustive()
    }
}

impl Default for NesHawkEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl NesHawkEngine {
    /// A fresh engine with no ROM, reading `ZAXNES_BIZHAWK_GAMEDB` for BizHawk's game databases.
    #[must_use]
    pub fn new() -> Self {
        let dir = std::env::var_os("ZAXNES_BIZHAWK_GAMEDB").map(PathBuf::from);
        let mut engine = Self {
            nes: None,
            image: Vec::new(),
            db: GameDb::empty(),
            gamedb_dir: None,
            rom_name: None,
            region: NesRegion::Ntsc,
            ports: [ControllerPortType::Controller; 2],
            frame_offset: 0,
            capture_tiles: false,
            wram_pattern: Vec::new(),
            records: Vec::new(),
            palette: [0; 32],
            audio: RefCell::new(VecDeque::new()),
            sample_rate: DEFAULT_SAMPLE_RATE,
            audio_level: 0,
            audio_last_clock: 0,
            audio_accumulated: 0.0,
            audio_filled: 0.0,
        };
        if let Some(dir) = dir {
            engine.set_gamedb_dir(&dir);
        }
        engine
    }

    /// Points board detection at BizHawk's `gamedb` directory (`NesCarts.xml`, `gamedb_nes.txt`),
    /// which is what makes this engine pick the board BizHawk picks.
    ///
    /// Takes effect on the next load. Without it detection falls back to the iNES header, and
    /// `crate::gamedb` explains why that is a *different machine* for some cartridges —
    /// *Faxanadu* is `NES-SGROM` with no WRAM where its header implies 8 KiB, and an MMC3 cart
    /// names its revision chip there, which decides the IRQ edge.
    pub fn set_gamedb_dir(&mut self, dir: &Path) {
        self.db = GameDb::load_from_dir(dir);
        self.gamedb_dir = Some(dir.to_path_buf());
    }

    /// Whether board detection has BizHawk's databases.
    #[must_use]
    pub const fn has_gamedb(&self) -> bool {
        self.gamedb_dir.is_some()
    }

    /// What the load decided and from where (`NES.RomDetails`), `None` with no ROM loaded.
    #[must_use]
    pub fn load_report(&self) -> Option<&crate::nes::LoadReport> {
        self.nes.as_ref()?.load_report.as_ref()
    }

    /// The board class the loaded cartridge is running, `None` with no ROM loaded.
    #[must_use]
    pub fn board_name(&self) -> Option<&'static str> {
        Some(self.load_report()?.board_name)
    }

    /// Whether the loaded cartridge runs on a board of **Zaxnes's** rather than a ported NesHawk
    /// board, in which case its timing is not NesHawk-exact (the owner's decision of
    /// 2026-11-12). The load report says so in words too.
    #[must_use]
    pub fn board_is_from_zaxnes(&self) -> bool {
        self.load_report()
            .is_some_and(|report| report.board_from_zaxnes)
    }

    /// The machine, for a host that wants NesHawk's own state (the harness's ledger does).
    #[must_use]
    pub const fn nes(&self) -> Option<&Nes> {
        self.nes.as_ref()
    }

    /// The machine, mutably.
    pub const fn nes_mut(&mut self) -> Option<&mut Nes> {
        self.nes.as_mut()
    }

    /// `PortDevice` for a port the host described, which is `NESControllers.cs`'s own list cut
    /// down to what is ported.
    const fn device(kind: ControllerPortType) -> PortDevice {
        match kind {
            ControllerPortType::Empty => PortDevice::Unplugged,
            // NesHawk's Zapper is not ported, so a port that asks for one gets a pad. The
            // substitution is visible: `port_type` goes on reporting what the host asked for.
            _ => PortDevice::ControllerNes,
        }
    }

    /// `DisplayType` for a region the host chose. NesHawk has no `Multi`, which runs NTSC
    /// (REQ-CORE-015).
    const fn display_type(region: NesRegion) -> DisplayType {
        match region {
            NesRegion::Pal => DisplayType::Pal,
            NesRegion::Dendy => DisplayType::Dendy,
            _ => DisplayType::Ntsc,
        }
    }

    /// The region a `DisplayType` is.
    const fn region_of(display_type: DisplayType) -> NesRegion {
        match display_type {
            DisplayType::Pal => NesRegion::Pal,
            DisplayType::Dendy => NesRegion::Dendy,
            DisplayType::Ntsc => NesRegion::Ntsc,
        }
    }

    /// Builds the machine from [`Self::image`] and powers it on, which is what both a load and a
    /// hard reset do — `NES.HardReset` rebuilds the board from the cartridge, so there is no
    /// shortcut that keeps it.
    fn build(&mut self) -> Result<Nes, LoadError> {
        let ports = [Self::device(self.ports[0]), Self::device(self.ports[1])];
        let mut nes = Nes::load(&self.image, &self.db, ports[0], ports[1])?;
        // `_display_type` and `InitialWRamStatePattern` are both read by `HardReset`, so they are
        // set and the machine powered on again rather than patched over afterwards.
        nes.set_display_type(Self::display_type(self.region));
        nes.initial_wram_state_pattern
            .clone_from(&self.wram_pattern);
        nes.hard_reset();
        nes.set_tile_capture(self.capture_tiles);
        Ok(nes)
    }

    /// Takes the frame's palette and tile records off the machine.
    fn capture_frame(&mut self) {
        let Some(nes) = self.nes.as_ref() else {
            return;
        };
        self.palette = nes.ppu.palram;
        self.records.clear();
        self.records.extend_from_slice(nes.tile_records());
    }

    /// Whether the 3D tile capture is running. Off until a host asks for it, because a host that
    /// only wants a picture (the harness's tools, the test-ROM readers) should pay nothing for
    /// records it will not read.
    #[must_use]
    pub fn tile_capture(&self) -> bool {
        self.nes.as_ref().is_some_and(Nes::tile_capture_enabled)
    }

    /// What the power-on writes into the 2 KiB of work RAM, `NESSyncSettings`'s
    /// `InitialWRamStatePattern` repeated to fill it.
    ///
    /// Empty — the default — is NesHawk's own: FCEUX's `00 00 00 00 FF FF FF FF`, which
    /// `HardReset` writes when the setting names no pattern. `zax-nes` fills **zeros**, so two
    /// engines compared without this agree about nothing a game reads out of uninitialised
    /// memory; lot FCEUX-6 measured that difference alone as worth six originals' worth of
    /// record differences. Takes effect at the next load or power-on, because `HardReset` is
    /// what reads it.
    pub fn set_initial_wram_pattern(&mut self, pattern: Vec<u8>) {
        self.wram_pattern = pattern;
    }

    /// Turns the 3D tile capture on or off. The application's engine has it **on** — the records
    /// are what the 3D renderer reads — and it survives a load, a power-on and a savestate.
    pub fn set_tile_capture(&mut self, on: bool) {
        self.capture_tiles = on;
        if let Some(nes) = self.nes.as_mut() {
            nes.set_tile_capture(on);
        }
    }

    /// Forgets the resampler's half-built sample, which a load, a power-on or a restored state
    /// must do: its level and its clock belong to the machine that is going away.
    fn reset_audio(&mut self) {
        self.audio.borrow_mut().clear();
        self.audio_level = 0;
        self.audio_last_clock = self.nes.as_ref().map_or(0, |nes| nes.apu.sampleclock);
        self.audio_accumulated = 0.0;
        self.audio_filled = 0.0;
    }

    /// Turns the frame's `blip.AddDelta` stream into samples at [`Self::sample_rate`].
    ///
    /// See [`Engine::read_audio`]'s note on why this is not `blip_buf`.
    fn resample_frame(&mut self) {
        let Some(nes) = self.nes.as_mut() else {
            return;
        };
        let now = nes.apu.sampleclock;
        let cycles_per_sample = f64::from(nes.cpuclockrate) / f64::from(self.sample_rate);
        // The deltas are borrowed out so that `advance_audio` can hold `&mut self`; the machine
        // gets its (cleared) vector back, so a frame still allocates nothing after the first.
        let deltas = std::mem::take(&mut nes.audio_deltas);
        for &(clock, delta) in &deltas {
            self.advance_audio(clock, cycles_per_sample);
            self.audio_level = self.audio_level.saturating_add(delta);
        }
        // Then on to the end of the frame, whether or not there were any deltas — a silent game
        // still owes the host its samples, or the audio device starves.
        self.advance_audio(now, cycles_per_sample);
        if let Some(nes) = self.nes.as_mut() {
            nes.audio_deltas = deltas;
            nes.audio_deltas.clear();
        }
        self.trim_audio();
    }

    /// Integrates the output level forward to `sampleclock` `to`, emitting a sample every
    /// `cycles_per_sample` cycles.
    ///
    /// The deltas are a derivative: the level between two of them is their running sum, so each
    /// output sample is the **mean level over its window** — a box filter, which has a zero at
    /// every multiple of the output rate, so the worst aliasing is attenuated rather than
    /// mirrored back as point sampling would mirror it.
    ///
    /// The step is `to.wrapping_sub(last)`, never a difference of absolute positions, because
    /// `APU.sampleclock` is a `uint` that the port never resets: it wraps after about forty
    /// minutes of play. Wrapping arithmetic on the step makes that moment a non-event instead of
    /// a negative time span and four billion samples of silence.
    fn advance_audio(&mut self, to: u32, cycles_per_sample: f64) {
        let mut remaining = f64::from(to.wrapping_sub(self.audio_last_clock));
        self.audio_last_clock = to;
        let level = f64::from(self.audio_level);
        let mut ring = self.audio.borrow_mut();
        while self.audio_filled + remaining >= cycles_per_sample {
            let take = cycles_per_sample - self.audio_filled;
            self.audio_accumulated += level * take;
            ring.push_back(Self::scale(self.audio_accumulated / cycles_per_sample));
            remaining -= take;
            self.audio_accumulated = 0.0;
            self.audio_filled = 0.0;
        }
        self.audio_accumulated += level * remaining;
        self.audio_filled += remaining;
    }

    /// One sample of the mixer's own scale as the `-1.0..=1.0` the trait promises.
    ///
    /// `APU.cs`'s mixer produces `20000.0 * output * (1 + m_vol / 5)`, so the full-scale
    /// magnitude at the default volume is about 20 000 — the same order as the `i16` the C# feeds
    /// its `blip`. Dividing by `i16::MAX` is therefore the same normalisation `zax-nes` applies
    /// to its own mixer, and the clamp is what stops a stuck DMC level from going out of range.
    fn scale(level: f64) -> f32 {
        (level as f32 / f32::from(i16::MAX)).clamp(-1.0, 1.0)
    }

    /// Caps the ring at a quarter-second, so that a host which never reads audio cannot grow it
    /// without bound. The oldest samples go, which is what a late reader wants anyway.
    fn trim_audio(&self) {
        let cap = (self.sample_rate as usize) / 4;
        let mut ring = self.audio.borrow_mut();
        while ring.len() > cap {
            ring.pop_front();
        }
    }

    /// Writes the header of a savestate block into `out` and returns where the payload length
    /// was left to be patched.
    fn write_header(&self, out: &mut Vec<u8>, full: bool, include_rom_image: bool) -> usize {
        let nes = self.nes.as_ref();
        out.extend_from_slice(&state_magic(EngineKind::NesHawk).to_le_bytes());
        out.extend_from_slice(&STATE_VERSION.to_le_bytes());
        let mut flags = 0u8;
        if full {
            flags |= FLAG_FULL;
        }
        if include_rom_image {
            flags |= FLAG_ROM_IMAGE;
        }
        out.push(flags);
        // The board class, so that a block cannot be restored onto a different mapper even if
        // the ROM name matches.
        out.push(nes.map_or(0xFF, |nes| board_kind_byte(nes.board_kind())));
        out.extend_from_slice(&self.frame_count().to_le_bytes());
        out.push(region_byte(self.region));
        let name = self.rom_name.clone().unwrap_or_default();
        let length = out.len();
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(name.len() as u32).to_le_bytes());
        out.extend_from_slice(name.as_bytes());
        if include_rom_image {
            out.extend_from_slice(&(self.image.len() as u32).to_le_bytes());
            out.extend_from_slice(&self.image);
        }
        length
    }
}

/// The byte a block records a board class as. Goes through the port's own `SyncState` so that the
/// two cannot disagree about the numbering.
fn board_kind_byte(kind: crate::boards::BoardKind) -> u8 {
    let mut writer = StateWriter::new(Vec::new(), false);
    crate::state::SyncState::write_state(&kind, &mut writer);
    writer.finish().first().copied().unwrap_or(0xFF)
}

/// The byte a block records the host's region choice as. Not `NesRegion`'s `as u8`, because this
/// is a persisted format and must not follow a type of another crate.
const fn region_byte(region: NesRegion) -> u8 {
    match region {
        NesRegion::Ntsc => 0,
        NesRegion::Pal => 1,
        NesRegion::Dendy => 2,
        NesRegion::Multi => 3,
    }
}

/// The region a block's byte names, `None` for anything else.
const fn region_of_byte(value: u8) -> Option<NesRegion> {
    match value {
        0 => Some(NesRegion::Ntsc),
        1 => Some(NesRegion::Pal),
        2 => Some(NesRegion::Dendy),
        3 => Some(NesRegion::Multi),
        _ => None,
    }
}

impl Engine for NesHawkEngine {
    // --------------------------------------------------------------- the game

    fn load_rom(&mut self, image: &[u8], name: &str) -> RomLoadResult {
        self.image = image.to_vec();
        self.rom_name = Some(name.to_owned());
        self.frame_offset = 0;
        self.records.clear();
        self.palette = [0; 32];
        self.audio.borrow_mut().clear();
        self.reset_audio();

        // The region the *cartridge* is, which the load detects (`NES.DetectRegion`) and which
        // REQ-CORE-013 says to report rather than apply. `build` runs the host's choice, which
        // for a fresh load is whatever the host set last — the host then calls `set_region` with
        // what it decided, exactly as it does for Zaxnes.
        match self.build() {
            Ok(nes) => {
                let detected = Self::region_of(nes.display_type());
                let note = (!self.has_gamedb()).then(|| {
                    format!(
                        "NesHawk: BizHawk's gamedb was not found, so the board for {name} was \
                         chosen from the iNES header alone. For some cartridges that is a \
                         different board, and so different timing, from the one BizHawk picks; \
                         set ZAXNES_BIZHAWK_GAMEDB to BizHawk's gamedb directory."
                    )
                });
                let from_zaxnes = nes
                    .load_report
                    .as_ref()
                    .is_some_and(|report| report.board_from_zaxnes);
                let board = nes.load_report.as_ref().map(|report| report.board_name);
                self.nes = Some(nes);
                self.capture_frame();
                let note = match (note, from_zaxnes, board) {
                    (note, false, _) => note,
                    (note, true, board) => {
                        let board = board.unwrap_or("?");
                        let extra = format!(
                            "NesHawk: {name} has no ported NesHawk board, so it runs on Zaxnes's \
                             own {board} — board from Zaxnes, not NesHawk-exact."
                        );
                        Some(match note {
                            Some(note) => format!("{note}\n{extra}"),
                            None => extra,
                        })
                    }
                };
                match note {
                    Some(note) => RomLoadResult::ok_with_note(detected, note),
                    None => RomLoadResult::ok(detected),
                }
            }
            Err(error) => {
                self.nes = None;
                self.image.clear();
                self.rom_name = None;
                let failure = match &error {
                    LoadError::TooSmall | LoadError::MalformedHeader => {
                        RomLoadFailure::CorruptHeader
                    }
                    LoadError::NoHeaderAndNoDatabaseEntry | LoadError::NoBoardFor(_) => {
                        RomLoadFailure::UnsupportedMapper
                    }
                    LoadError::UnsupportedFormat(_) => RomLoadFailure::CorruptHeader,
                };
                RomLoadResult::fail(failure, error.to_string())
            }
        }
    }

    fn close_rom(&mut self) {
        self.nes = None;
        self.image.clear();
        self.rom_name = None;
        self.frame_offset = 0;
        self.records.clear();
        self.palette = [0; 32];
        self.audio.borrow_mut().clear();
        self.reset_audio();
    }

    fn is_running(&self) -> bool {
        self.nes.is_some()
    }

    fn rom_name(&self) -> Option<&str> {
        self.rom_name.as_deref()
    }

    fn power(&mut self) {
        if self.nes.is_none() {
            return;
        }
        if let Ok(nes) = self.build() {
            self.nes = Some(nes);
            self.frame_offset = 0;
            self.audio.borrow_mut().clear();
            self.reset_audio();
            self.capture_frame();
        }
    }

    fn reset(&mut self) {
        self.power();
    }

    fn soft_reset(&mut self) {
        if let Some(nes) = self.nes.as_mut() {
            // `NES.SoftReset` is the RESET line, which keeps the RAM. NesHawk applies it through
            // the four components' own `NESSoftReset`, which is what this calls.
            nes.soft_reset();
        }
    }

    // ---------------------------------------------------------------- a frame

    fn run_frame(&mut self, input: InputWord) -> i64 {
        let Some(nes) = self.nes.as_mut() else {
            return 0;
        };
        let before = nes.total_executed_cycles;
        nes.controller_deck.pad = [(input & 0xFF) as u8, (input >> 8) as u8];
        nes.frame_advance(false, false);
        let cycles = nes.total_executed_cycles - before;
        self.capture_frame();
        self.resample_frame();
        cycles
    }

    fn frame_count(&self) -> i64 {
        self.frame_offset + self.nes.as_ref().map_or(0, |nes| i64::from(nes.frame))
    }

    fn set_frame_count(&mut self, value: i64) {
        let machine = self.nes.as_ref().map_or(0, |nes| i64::from(nes.frame));
        self.frame_offset = value - machine;
    }

    fn pixel_frame_indices_into(&self, out: &mut Vec<u8>) {
        out.clear();
        let Some(nes) = self.nes.as_ref() else {
            // Before the first frame, or with no ROM: a black picture of the right size, which is
            // what `zax-nes` has in its own buffer at that point.
            out.resize(FRAME_LEN, 0);
            return;
        };
        out.reserve(FRAME_LEN);
        // `xbuf` holds `pixel | intensity << 6`, with bit 15 marking a pixel the backdrop shows
        // through; the trait's picture is one NES colour index a pixel, which is the low six —
        // the same `& PIXEL_COLOR` `zax-nes` applies to its own buffer.
        out.extend(nes.ppu.xbuf.iter().map(|&p| (p & 0x3F) as u8));
    }

    fn frame_batch(&self) -> FrameBatch<'_> {
        FrameBatch {
            records: &self.records,
            palette: self.palette,
            margin_palette: None,
            // The unreliable-band widths of REQ-CORE-046 are the margin decoder's, which is
            // Zaxnes's bus; a NesHawk frame is a standard frame with no margin.
            mask_x: 0,
            mask_y: 0,
            margin: 0,
            visible_rows: STANDARD_ROWS,
            margin_y: 0,
        }
    }

    // ----------------------------------------------------------------- audio

    /// # The one place this engine synthesises rather than ports
    ///
    /// NesHawk's `APU` calls `blip.AddDelta(sampleclock, delta)`, and the port records every one
    /// of those calls (`Nes::audio_deltas`) at the cycle the C# makes it. What BizHawk does with
    /// them afterwards is **not C# at all**: `BlipBuffer.cs` is a `DllImport` shim over Blargg's
    /// native `blip_buf`, so there is no reference source in the clone to port — the band-limited
    /// synthesis is in a C library this lot may not fetch.
    ///
    /// So the deltas are integrated back to a level and box-filtered down to the output rate
    /// here. The *timing* of every delta is NesHawk's to the cycle, which is what the port is
    /// verified on (work RAM, the cycle ledgers, the test-ROM verdicts); the reconstruction
    /// filter differs from `blip_buf`'s windowed-sinc, so the waveform is this engine's own and
    /// not bit-identical to BizHawk's `.wav`. `docs/review/NESHAWK-REWRITE.md` §7 states it as a
    /// gap rather than leaving it to be discovered.
    fn read_audio(&self, destination: &mut [f32], stereo: bool) -> Result<usize, AudioError> {
        if stereo && destination.len() % 2 != 0 {
            return Err(AudioError::OddStereoBuffer(destination.len()));
        }
        let mut ring = self.audio.borrow_mut();
        let mut written = 0;
        if stereo {
            for pair in destination.chunks_exact_mut(2) {
                let Some(sample) = ring.pop_front() else {
                    break;
                };
                pair[0] = sample;
                pair[1] = sample;
                written += 2;
            }
        } else {
            for slot in destination.iter_mut() {
                let Some(sample) = ring.pop_front() else {
                    break;
                };
                *slot = sample;
                written += 1;
            }
        }
        Ok(written)
    }

    fn set_sample_rate(&mut self, value: i32) -> Result<(), AudioError> {
        if !(MIN_SAMPLE_RATE..=MAX_SAMPLE_RATE).contains(&value) {
            return Err(AudioError::SampleRateOutOfRange(value));
        }
        self.sample_rate = value;
        // The half-built sample belongs to the old window; keeping it would put one sample out of
        // place at the join, so it is dropped rather than rescaled.
        self.audio_accumulated = 0.0;
        self.audio_filled = 0.0;
        Ok(())
    }

    fn sample_rate(&self) -> i32 {
        self.sample_rate
    }

    // ---------------------------------------------------------------- memory

    fn read_memory(&self, address: u16) -> u8 {
        // `NES.PeekMemory`, not `ReadMemory`: REQ-CORE-080 asks for no side effects where the
        // engine can avoid them, and NesHawk's own debugger reads through the peek path.
        self.nes.as_ref().map_or(0, |nes| nes.peek_memory(address))
    }

    fn write_memory(&mut self, address: u16, value: u8) {
        if let Some(nes) = self.nes.as_mut() {
            nes.write_memory(address, value);
        }
    }

    // ------------------------------------------------------------ savestates

    fn save_state_to(
        &self,
        include_rom_image: bool,
        buffer: &mut Vec<u8>,
    ) -> Result<(), NoRomLoaded> {
        let Some(nes) = self.nes.as_ref() else {
            return Err(NoRomLoaded);
        };
        let mut out = std::mem::take(buffer);
        out.clear();
        // A state the host asks to be self-contained is one the user saved, so it carries the
        // frame buffer too: the picture comes back at once instead of on the next frame. A
        // rollback slot asks for neither, and REQ-CORE-121's own snapshot leaves out exactly
        // that "half of the PPU state that *is* the frame just run".
        let full = include_rom_image;
        let length_at = self.write_header(&mut out, full, include_rom_image);
        let header_len = out.len();
        let mut writer = StateWriter::new(out, full);
        nes.write_state(&mut writer);
        let payload = (writer.len() - header_len) as u32;
        writer.patch_u32(length_at, payload);
        *buffer = writer.finish();
        Ok(())
    }

    fn max_state_size(&self) -> usize {
        // The machine's own fields, generously: 2 KiB of work RAM, 2 KiB of CIRAM, the PPU's
        // tables, the board's WRAM and VRAM, the frame buffer, and the ROM image.
        let board = self.nes.as_ref().map_or(0, |nes| {
            nes.board.base().wram.len() + nes.board.base().vram.len()
        });
        HEADER_FIXED
            + 4
            + self.rom_name.as_ref().map_or(0, String::len)
            + 4
            + self.image.len()
            + 0x1000
            + 0x2000
            + board
            + FRAME_LEN * 2
            // the CPU, the APU, the PPU's scalars and the board's registers, with room to spare
            + 0x1000
    }

    fn try_load_state(&mut self, data: &[u8], offset: usize) -> Result<StateLoaded, InvalidState> {
        if self.nes.is_none() {
            return Err(InvalidState::corrupt(
                "No ROM is loaded, so there is no machine to restore a state onto.",
            ));
        }
        let block = data.get(offset..).ok_or_else(|| {
            InvalidState::corrupt("The savestate starts past the end of the data.")
        })?;

        // The first four bytes name the engine, which is what makes a block of another engine a
        // clear refusal rather than "corrupt" (REQ-CORE-120).
        let head = block
            .get(..4)
            .ok_or_else(|| InvalidState::corrupt("The savestate is shorter than its header."))?;
        let magic = u32::from_le_bytes([head[0], head[1], head[2], head[3]]);
        if magic != state_magic(EngineKind::NesHawk) {
            return Err(InvalidState {
                failure: StateLoadFailure::ForeignCore,
                message: format!(
                    "{} This one was written by {}, not by {CORE_NAME}.",
                    zax_core_api::FOREIGN_CORE_MESSAGE,
                    zax_core_api::engine_of_state(block, 0)
                        .map_or("an unknown core", EngineKind::display_name),
                ),
            });
        }

        let mut at = 4;
        let mut take = |n: usize| -> Result<&[u8], InvalidState> {
            let slice = block
                .get(at..at + n)
                .ok_or_else(|| InvalidState::corrupt("The savestate is truncated."))?;
            at += n;
            Ok(slice)
        };

        let version = i32::from_le_bytes(take(4)?.try_into().unwrap_or([0; 4]));
        if version != STATE_VERSION {
            return Err(InvalidState::corrupt(format!(
                "The savestate is version {version}; this build of {CORE_NAME} writes version \
                 {STATE_VERSION}."
            )));
        }
        let flags = take(1)?[0];
        let full = flags & FLAG_FULL != 0;
        let board = take(1)?[0];
        let frame_count = i64::from_le_bytes(take(8)?.try_into().unwrap_or([0; 8]));
        let region = region_of_byte(take(1)?[0]).ok_or_else(|| {
            InvalidState::corrupt("The savestate names a region this build does not know.")
        })?;
        let payload_len = u32::from_le_bytes(take(4)?.try_into().unwrap_or([0; 4])) as usize;
        let name_len = u32::from_le_bytes(take(4)?.try_into().unwrap_or([0; 4])) as usize;
        let rom_name = String::from_utf8(take(name_len)?.to_vec())
            .map_err(|_| InvalidState::corrupt("The savestate's ROM name is not valid UTF-8."))?;
        let image = if flags & FLAG_ROM_IMAGE != 0 {
            let len = u32::from_le_bytes(take(4)?.try_into().unwrap_or([0; 4])) as usize;
            Some(take(len)?.to_vec())
        } else {
            None
        };

        // The board class the block was taken on. A block restored onto a different mapper would
        // read the right number of bytes into the wrong registers, which is a wrong machine
        // rather than an error — so it is refused here, before anything is touched.
        let running = self
            .nes
            .as_ref()
            .map_or(0xFF, |nes| board_kind_byte(nes.board_kind()));
        if board != running {
            return Err(InvalidState::corrupt(format!(
                "The savestate was taken on board class {board}, and this machine is running \
                 class {running}: it belongs to another cartridge."
            )));
        }

        let payload = block
            .get(at..at + payload_len)
            .ok_or_else(|| InvalidState::corrupt("The savestate's payload is truncated."))?;

        // Restored onto a copy, so that a block that turns out to be truncated half-way cannot
        // leave the live machine in a state that is neither the old one nor the new one.
        let mut restored = self.nes.clone().ok_or_else(|| {
            InvalidState::corrupt("No ROM is loaded, so there is no machine to restore onto.")
        })?;
        let mut reader = StateReader::new(payload, full);
        restored.read_state(&mut reader);
        if reader.failed() {
            return Err(InvalidState::corrupt(
                "The savestate is truncated or corrupt: it ran out part-way through the machine.",
            ));
        }
        if reader.position() != payload_len {
            return Err(InvalidState::corrupt(format!(
                "The savestate's payload is {payload_len} bytes and the machine read \
                 {}: it was written by a different build.",
                reader.position()
            )));
        }

        if let Some(image) = image {
            self.image = image;
        }
        self.nes = Some(restored);
        self.rom_name = Some(rom_name.clone());
        self.region = region;
        self.set_frame_count(frame_count);
        self.audio.borrow_mut().clear();
        self.reset_audio();
        self.capture_frame();

        Ok(StateLoaded {
            consumed: at + payload_len,
            rom_name,
        })
    }

    // ----------------------------------------------------------- battery RAM

    fn export_battery_ram(&self, out: &mut Vec<u8>) {
        out.clear();
        if let Some(nes) = self.nes.as_ref() {
            nes.board.save_ram_into(out);
        }
    }

    fn import_battery_ram(&mut self, data: &[u8]) -> bool {
        let Some(nes) = self.nes.as_mut() else {
            return false;
        };
        let Some(ram) = nes.board.save_ram_mut() else {
            return false;
        };
        if ram.len() != data.len() {
            return false;
        }
        ram.copy_from_slice(data);
        true
    }

    // --------------------------------------------------------------- machine

    fn region(&self) -> NesRegion {
        self.region
    }

    /// # One deviation from REQ-CORE-016, and why it is NesHawk's own
    ///
    /// The requirement is that a region change take effect on the next frame **without** a
    /// reset, which `zax-nes` can do because its PPU and APU read their region constants as they
    /// run. NesHawk cannot: `_display_type` is read by `HardReset`, which picks the APU's
    /// tables, the PPU's region and `cpu_sequence` once — in BizHawk, changing a NES core's
    /// region needs a core reboot, and this is that same fact.
    ///
    /// So a change that really changes the timing powers the machine on again, and a change that
    /// does not (`Ntsc` to `Multi`, which both run NTSC) costs nothing. The host's own sequence
    /// is `load_rom` then `set_region`, where there is no state to lose; a change mid-game is a
    /// power cycle, as it is in BizHawk.
    fn set_region(&mut self, region: NesRegion) {
        self.region = region;
        let wanted = Self::display_type(region);
        if self
            .nes
            .as_ref()
            .is_some_and(|nes| nes.display_type() == wanted)
        {
            return;
        }
        self.power();
    }

    fn port_type(&self, port: usize) -> Result<ControllerPortType, InvalidPort> {
        self.ports.get(port).copied().ok_or(InvalidPort(port))
    }

    fn set_port_type(&mut self, port: usize, kind: ControllerPortType) -> Result<(), InvalidPort> {
        if port >= self.ports.len() {
            return Err(InvalidPort(port));
        }
        self.ports[port] = kind;
        if let Some(nes) = self.nes.as_mut() {
            nes.controller_deck.set_device(port, Self::device(kind));
        }
        Ok(())
    }

    // ---------------------------------------------------------- capabilities

    fn capabilities(&self) -> Capabilities {
        Capabilities::NESHAWK
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
