//! The [`Engine`] trait: one emulated NES machine, seen from the application.
//!
//! Shaped by §2 of `docs/review/FCEUX-ENGINE.md`, which split what the application asks of a core
//! into *what both engines can do* — this trait — and *what is Zaxnes's machine rather than an
//! emulator's* — the widescreen extension, the hack packs, the sprite limit, the exact run-ahead,
//! the bus and the PPU themselves. The second list is **not** a gap to be filled later: those are
//! patches applied to Zaxnes's bus, and porting them to FCEUX would mean reimplementing that bus
//! inside the fork. They are reached by a checked downcast ([`Engine::as_any`]) or by a
//! Zaxnes-only handle, and [`Capabilities`] is what the user interface reads to disable them.

use std::any::Any;

use crate::{
    AudioError, ControllerPortType, FrameBatch, InputWord, InvalidPort, InvalidState, NesRegion,
    NoRomLoaded, RomLoadResult, StateLoaded,
};

/// Which engine a handle, a savestate or a per-game option names.
///
/// Numeric values are persisted (the per-game option of step 5 and the engine stamp of a
/// savestate), so they must not change: Zaxnes = 0, FCEUX = 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum EngineKind {
    /// Zaxnes's own core, `zax-nes`. The default everywhere, and the only engine in a build
    /// without the `fceux-core` feature.
    #[default]
    Zaxnes = 0,
    /// FCEUX's core through `zax-fceux`.
    Fceux = 1,
}

impl EngineKind {
    /// The kind of numeric value `value`, `None` otherwise.
    #[must_use]
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Zaxnes),
            1 => Some(Self::Fceux),
            _ => None,
        }
    }

    /// The identifier the per-game option and the savestate stamp store.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Zaxnes => "zaxnes",
            Self::Fceux => "fceux",
        }
    }

    /// The kind `text` names, case-insensitively; `None` for anything else.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_lowercase().as_str() {
            "zaxnes" => Some(Self::Zaxnes),
            "fceux" => Some(Self::Fceux),
            _ => None,
        }
    }

    /// The name to show a user.
    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Zaxnes => "Zaxnes",
            Self::Fceux => "FCEUX",
        }
    }
}

impl std::fmt::Display for EngineKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.display_name())
    }
}

/// What an engine can do beyond the trait, for the user interface to enable or disable rather
/// than hide (§4 of the design note: "a hidden control reads as a bug").
///
/// Every flag is `true` on Zaxnes and `false` on FCEUX as this lot leaves it; each says what the
/// *engine* supports, not whether the user has switched it on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
// Seven independent yes-or-no answers about one engine, which is what the interface asks it. A
// state machine would say they vary together and they do not: lot FCEUX-5 raises `run_ahead`
// alone, lot FCEUX-7 may raise none of them, and `Capability` below already gives a caller the
// one name it needs without touching the rest.
#[allow(clippy::struct_excessive_bools)]
pub struct Capabilities {
    /// The engine this is. A field rather than a method so that a `Capabilities` copied into
    /// the interface's own state still says which engine it describes.
    pub kind: EngineKind,
    /// The widescreen extension: `wide_picture_into`, the margin the host offers, and the
    /// `margin`/`margin_y`/`visible_rows` fields of a [`FrameBatch`] carrying anything but their
    /// standard values. Zaxnes's bus, not an emulator feature.
    pub widescreen: bool,
    /// The built-in hack packs and their option bits (`$500A`), and the testing-aid packs.
    /// Zaxnes's bus again.
    pub hack_packs: bool,
    /// Lifting or smoothing the 8-sprites-per-line limit of the picture.
    pub sprite_limit: bool,
    /// Run-ahead. Exact on Zaxnes (`run_frame_ahead` + `restore_real_frame`); on FCEUX it would
    /// be save → run N → take the frame → load every frame, which lot FCEUX-5 is for, so it is
    /// **off** here rather than slow and silent.
    pub run_ahead: bool,
    /// Rollback, which every engine can do — but only Zaxnes has the cheap in-memory
    /// `capture_core_state`/`restore_core_state` path; see [`Self::fast_core_snapshot`].
    pub rollback: bool,
    /// The in-memory core snapshot of AMD-041 REQ-CORE-121: a reused buffer that carries neither
    /// magic number, format version, core name, ROM name nor ROM image, and leaves out the half
    /// of the PPU state that *is* the frame just run. FCEUX has only its serialised `FCSX`
    /// block, so rollback there pays a full save and load per slot.
    pub fast_core_snapshot: bool,
    /// Zaxnes's `cartridge()`, `bus()` and `ppu()`, which the movie timeline, the achievements
    /// and the Lua host reach for.
    pub machine_internals: bool,
}

impl Capabilities {
    /// Everything Zaxnes's core offers.
    pub const ZAXNES: Self = Self {
        kind: EngineKind::Zaxnes,
        widescreen: true,
        hack_packs: true,
        sprite_limit: true,
        run_ahead: true,
        rollback: true,
        fast_core_snapshot: true,
        machine_internals: true,
    };

    /// What FCEUX offers as lot FCEUX-23 leaves it: the trait, and rollback over its serialised
    /// savestates. Lots FCEUX-5 (run-ahead) and FCEUX-7/8 may raise some of the rest.
    pub const FCEUX: Self = Self {
        kind: EngineKind::Fceux,
        widescreen: false,
        hack_packs: false,
        sprite_limit: false,
        run_ahead: false,
        rollback: true,
        fast_core_snapshot: false,
        machine_internals: false,
    };

    /// Why a control is disabled, for the interface to show beside it; `None` when `capability`
    /// is supported. `capability` names the field, as [`Capability`] does.
    #[must_use]
    pub const fn refusal(self, capability: Capability) -> Option<&'static str> {
        if self.has(capability) {
            return None;
        }
        Some(match (self.kind, capability) {
            (_, Capability::Widescreen) => {
                "Widescreen is part of Zaxnes's own machine, not of an emulator; FCEUX cannot \
                 provide it. Choose the Zaxnes engine for this game to use it."
            }
            (_, Capability::HackPacks) => {
                "The hack packs patch Zaxnes's bus as the game runs; FCEUX cannot apply them. \
                 Choose the Zaxnes engine for this game to use them."
            }
            (_, Capability::SpriteLimit) => {
                "Changing the 8-sprites-per-line limit needs Zaxnes's PPU. Choose the Zaxnes \
                 engine for this game to use it."
            }
            (_, Capability::RunAhead) => {
                "Run-ahead on FCEUX would save and restore a state every frame; it is not \
                 implemented yet (lot FCEUX-5). Choose the Zaxnes engine for this game to use it."
            }
            (_, Capability::Rollback) => "This engine cannot roll back.",
            (_, Capability::FastCoreSnapshot) => {
                "This engine has no in-memory snapshot; rollback uses its serialised savestates."
            }
            (_, Capability::MachineInternals) => {
                "This engine does not expose the cartridge, the bus or the PPU."
            }
        })
    }

    /// Whether `capability` is supported.
    #[must_use]
    pub const fn has(self, capability: Capability) -> bool {
        match capability {
            Capability::Widescreen => self.widescreen,
            Capability::HackPacks => self.hack_packs,
            Capability::SpriteLimit => self.sprite_limit,
            Capability::RunAhead => self.run_ahead,
            Capability::Rollback => self.rollback,
            Capability::FastCoreSnapshot => self.fast_core_snapshot,
            Capability::MachineInternals => self.machine_internals,
        }
    }
}

/// One field of [`Capabilities`], so that a control can ask about itself by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Capability {
    /// [`Capabilities::widescreen`].
    Widescreen,
    /// [`Capabilities::hack_packs`].
    HackPacks,
    /// [`Capabilities::sprite_limit`].
    SpriteLimit,
    /// [`Capabilities::run_ahead`].
    RunAhead,
    /// [`Capabilities::rollback`].
    Rollback,
    /// [`Capabilities::fast_core_snapshot`].
    FastCoreSnapshot,
    /// [`Capabilities::machine_internals`].
    MachineInternals,
}

/// One emulated NES machine.
///
/// # `Send` but **not** `Sync`
///
/// FCEUX keeps its whole machine in file-scope globals, so there is exactly one per process. A
/// handle may *move* to another thread — which is what the application's emulation thread does
/// with it — but two threads must never be inside the library at once. `zax-fceux`'s handle
/// enforces that with a `PhantomData<Cell<()>>`, and this trait must not undo it by adding
/// `Sync`. `NesCore` is `Sync`-able in principle and loses nothing by being handled this way.
///
/// # Nothing advances by itself
///
/// As REQ-ARCH-010 requires of `zax-nes`, an engine owns no clock and no thread: nothing happens
/// unless the host calls [`Engine::run_frame`].
pub trait Engine: Send {
    // --------------------------------------------------------------- the game

    /// Loads a ROM image and powers the machine on (REQ-CORE-010). On failure the engine is left
    /// stopped (REQ-CORE-011); the detected region is returned, **not applied** (REQ-CORE-013).
    fn load_rom(&mut self, image: &[u8], name: &str) -> RomLoadResult;

    /// Unloads the ROM and leaves the engine stopped (REQ-CORE-005).
    fn close_rom(&mut self);

    /// Whether a ROM is loaded.
    fn is_running(&self) -> bool;

    /// The name the ROM was loaded under, `None` when none is loaded.
    fn rom_name(&self) -> Option<&str>;

    /// Powers the machine on again from the ROM already loaded, as a fresh load would
    /// (REQ-CORE-010). Switching engines for a game goes through a load, never through this.
    fn power(&mut self);

    /// A hard reset: the power cycle of REQ-CORE-014.
    fn reset(&mut self);

    /// A soft reset: the RESET line, which keeps the RAM (REQ-CORE-014).
    fn soft_reset(&mut self);

    // ---------------------------------------------------------------- a frame

    /// Runs one frame with `input` (REQ-CORE-020, REQ-CORE-021). Returns the CPU cycles the frame
    /// took, 0 when no ROM is loaded (REQ-CORE-005).
    fn run_frame(&mut self, input: InputWord) -> i64;

    /// Frames run since the last power-on (REQ-CORE-032).
    fn frame_count(&self) -> i64;

    /// Sets the frame counter, which a restored savestate and the movie host need.
    fn set_frame_count(&mut self, value: i64);

    /// The 2D picture of the frame just run, as one NES colour index (and its emphasis and
    /// opacity bits) per pixel, written into `out` whose earlier content is replaced.
    fn pixel_frame_indices_into(&self, out: &mut Vec<u8>);

    /// The frame's tile records and palette (REQ-CORE-030, REQ-CORE-045).
    ///
    /// The records are **borrowed**: a host that keeps them must copy them. FCEUX's arrive
    /// through a C callback into a buffer the library owns, so its implementation copies them
    /// into its own `Vec` once a frame and lends that.
    fn frame_batch(&self) -> FrameBatch<'_>;

    // ----------------------------------------------------------------- audio

    /// Reads up to `destination.len()` samples of the frames run since the last read, and returns
    /// how many were written (REQ-CORE-071).
    ///
    /// # Errors
    ///
    /// [`AudioError::OddStereoBuffer`] when `stereo` is set and the buffer holds an odd number of
    /// values.
    fn read_audio(&self, destination: &mut [f32], stereo: bool) -> Result<usize, AudioError>;

    /// Sets the output sample rate in Hz (REQ-CORE-070).
    ///
    /// An engine that cannot produce the rate asked for may run the nearest one it can, which is
    /// why [`Engine::sample_rate`] exists: a host that feeds an audio device must read the rate
    /// back rather than assume the one it set. FCEUX's core takes only 44 100, 48 000 and 96 000.
    ///
    /// # Errors
    ///
    /// [`AudioError::SampleRateOutOfRange`] outside
    /// [`MIN_SAMPLE_RATE`](crate::MIN_SAMPLE_RATE)..=[`MAX_SAMPLE_RATE`](crate::MAX_SAMPLE_RATE).
    fn set_sample_rate(&mut self, value: i32) -> Result<(), AudioError>;

    /// The rate the engine is really running at, which may not be the one
    /// [`Engine::set_sample_rate`] was given.
    fn sample_rate(&self) -> i32;

    // ---------------------------------------------------------------- memory

    /// Reads one byte of the CPU address space, as the CPU would but without side effects where
    /// the engine can avoid them (REQ-CORE-080).
    fn read_memory(&self, address: u16) -> u8;

    /// Writes one byte of the CPU address space (REQ-CORE-081).
    fn write_memory(&mut self, address: u16, value: u8);

    // ------------------------------------------------------------ savestates

    /// Writes a savestate into `buffer`, whose earlier content is replaced and whose allocation
    /// is kept, so a rollback history can recycle one buffer per slot.
    ///
    /// `include_rom_image` asks for a self-contained block (REQ-CORE-090). An engine that cannot
    /// leave the image out writes it either way, which is why
    /// [`Capabilities::fast_core_snapshot`] exists: it is the flag that decides whether rollback
    /// pays for this at all.
    ///
    /// # Errors
    ///
    /// [`NoRomLoaded`] when no ROM is loaded; `buffer` is then untouched.
    fn save_state_to(
        &self,
        include_rom_image: bool,
        buffer: &mut Vec<u8>,
    ) -> Result<(), NoRomLoaded>;

    /// Upper bound on the size of a block for the ROM loaded, valid for the whole session
    /// (REQ-ARCH-013).
    fn max_state_size(&self) -> usize;

    /// Restores the block that starts at `offset` in `data`, which may hold other data around it
    /// (REQ-CORE-091).
    ///
    /// # Errors
    ///
    /// [`InvalidState`] for an invalid or truncated block, or one written by another core
    /// ([`StateLoadFailure::ForeignCore`](crate::StateLoadFailure::ForeignCore)); the engine is
    /// then left stopped (REQ-CORE-005). A block written by the *other engine* is refused the
    /// same way, which step 5 of the lot relies on.
    fn try_load_state(&mut self, data: &[u8], offset: usize) -> Result<StateLoaded, InvalidState>;

    // ----------------------------------------------------------- battery RAM

    /// The battery-backed PRG-RAM to persist, copied into `out` whose earlier content is
    /// replaced; `out` is left empty when the cartridge has none (REQ-CORE-095).
    ///
    /// A copy rather than the borrow `NesCore::export_battery_ram` lends, because FCEUX's bytes
    /// have to be fetched across the C API and so cannot be lent out of a `&self`.
    fn export_battery_ram(&self, out: &mut Vec<u8>);

    /// Restores battery-backed PRG-RAM; `false` when the cartridge has none or the size is
    /// wrong (REQ-CORE-095).
    fn import_battery_ram(&mut self, data: &[u8]) -> bool;

    // --------------------------------------------------------------- machine

    /// The region the machine is running (REQ-CORE-015).
    fn region(&self) -> NesRegion;

    /// Chooses the region; takes effect on the next frame, without a reset (REQ-CORE-016).
    fn set_region(&mut self, region: NesRegion);

    /// What is plugged into `port` (0 or 1).
    ///
    /// # Errors
    ///
    /// [`InvalidPort`] for any other port number (REQ-CORE-024).
    fn port_type(&self, port: usize) -> Result<ControllerPortType, InvalidPort>;

    /// Plugs `kind` into `port` (0 or 1).
    ///
    /// # Errors
    ///
    /// [`InvalidPort`] for any other port number, or [`InvalidPort`] when the engine cannot
    /// offer that device (REQ-CORE-024).
    fn set_port_type(&mut self, port: usize, kind: ControllerPortType) -> Result<(), InvalidPort>;

    // ---------------------------------------------------------- capabilities

    /// What this engine can do beyond the trait, and which engine it is.
    fn capabilities(&self) -> Capabilities;

    /// The engine this is, which is [`Capabilities::kind`]; here too because a caller that only
    /// wants the marker should not have to build the whole struct.
    fn kind(&self) -> EngineKind {
        self.capabilities().kind
    }

    /// For the checked downcast to a concrete engine, which is how the Zaxnes-only features are
    /// reached — safely, with no `unsafe` and no panic on the wrong engine.
    ///
    /// Implementations are one line, `self`. It is on the trait rather than a supertrait bound
    /// (`Engine: Any`) because `Any` requires `'static`, which would be a needless promise, and
    /// because `&dyn Engine` cannot be upcast to `&dyn Any` on the pinned toolchain.
    fn as_any(&self) -> &dyn Any;

    /// The same, mutably: Zaxnes's run-ahead, its hack options and its widescreen margin all need
    /// `&mut`.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
