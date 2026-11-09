//! What a savestate can fail with, and what a successful restore reports.
//!
//! Moved here from `zax_nes::state` and `zax_nes::nes_core` by lot FCEUX-23 step 2 because
//! [`Engine::save_state_to`](crate::Engine::save_state_to) and
//! [`Engine::try_load_state`](crate::Engine::try_load_state) are on the trait, so both engines
//! have to return the same errors. The *format* of a block stays each engine's own — Zaxnes's
//! writer and reader (`zax_nes::state::StateWriter`/`StateReader`) are not here, and FCEUX's
//! `FCSX` blocks come out of its own library.

use std::fmt;

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
