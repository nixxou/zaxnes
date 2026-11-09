//! What a savestate can fail with, and what a successful restore reports.
//!
//! Moved here from `zax_nes::state` and `zax_nes::nes_core` by lot FCEUX-23 step 2 because
//! [`Engine::save_state_to`](crate::Engine::save_state_to) and
//! [`Engine::try_load_state`](crate::Engine::try_load_state) are on the trait, so both engines
//! have to return the same errors. The *format* of a block stays each engine's own — Zaxnes's
//! writer and reader (`zax_nes::state::StateWriter`/`StateReader`) are not here, and FCEUX's
//! `FCSX` blocks come out of its own library.

use std::fmt;

use crate::EngineKind;

/// The message REQ-CORE-120 fixes to the character for a state written by another core — one of
/// the few literal strings of the spec (AMD-002, answer to Q-001). Both engines return it, so it
/// lives here; `zax_nes::NesCore::FOREIGN_CORE_MESSAGE` is defined as this.
pub const FOREIGN_CORE_MESSAGE: &str = "Save state used a different core.";

/// The first four bytes of a savestate block name the engine that wrote it.
///
/// Zaxnes's value is the one its own blocks have always carried, so every state written before
/// this lot still identifies itself; FCEUX's is `ZXFX` and was new with lot FCEUX-23. A block of
/// one engine handed to the other is refused with
/// [`StateLoadFailure::ForeignCore`] and [`FOREIGN_CORE_MESSAGE`] — which is what makes the
/// refusal a clear message rather than "corrupt".
#[must_use]
pub const fn state_magic(engine: EngineKind) -> u32 {
    match engine {
        // `SEN0` little-endian: Zaxnes's blocks have carried it since the C# original.
        EngineKind::Zaxnes => 0x304E_4553,
        // `ZXFX` little-endian.
        EngineKind::Fceux => 0x5846_585A,
    }
}

/// Which engine wrote the block that starts at `offset` in `data`, from its first four bytes;
/// `None` when they name no engine this build knows, or the block is shorter than four bytes.
#[must_use]
pub fn engine_of_state(data: &[u8], offset: usize) -> Option<EngineKind> {
    let head = data.get(offset..offset.checked_add(4)?)?;
    let magic = u32::from_le_bytes([head[0], head[1], head[2], head[3]]);
    [EngineKind::Zaxnes, EngineKind::Fceux]
        .into_iter()
        .find(|&engine| state_magic(engine) == magic)
}

/// Typed cause of a failed "restore state" (REQ-CORE-091, AREQ-CORE-120; AMD-002 answer to
/// Q-001). The host picks the text it shows from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateLoadFailure {
    /// The block is invalid or truncated ("savestate corrompu").
    CorruptState,
    /// The block was written by another core, which REQ-CORE-120 refuses with the literal
    /// message `Save state used a different core.`
    ForeignCore,
}

/// A savestate block that is invalid or truncated, or written by another core.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidState {
    /// Typed cause of the failure.
    pub failure: StateLoadFailure,
    /// Technical message naming the cause.
    pub message: String,
}

impl InvalidState {
    /// A corrupt or truncated block, described by `message`.
    #[must_use]
    pub fn corrupt(message: impl Into<String>) -> Self {
        Self {
            failure: StateLoadFailure::CorruptState,
            message: message.into(),
        }
    }
}

impl fmt::Display for InvalidState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for InvalidState {}

/// What a successful "restore state" reports (REQ-CORE-091).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateLoaded {
    /// Bytes the block occupied, so the host can read on.
    pub consumed: usize,
    /// The ROM name stored in the block.
    pub rom_name: String,
}

/// Why a savestate could not be written: no ROM is loaded, so there is no state to save.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoRomLoaded;

impl fmt::Display for NoRomLoaded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("No ROM is loaded: there is no state to save.")
    }
}

impl std::error::Error for NoRomLoaded {}
