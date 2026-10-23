//! The fast-forward model of SPEC-07 REQ-LOOP-016: every request arms 30 emulated frames at
//! `k = 1.5`, and a request received while one is already running has no effect.
//!
//! The hotkey emits one request per rendered frame while it is held (SPEC-08 REQ-INP-103), so
//! holding it re-arms the burst as soon as the previous one ends: the speed is x1.5 almost
//! continuously. On release, 0 to 30 emulated frames remain, then `k` goes back to the speed
//! setting.

/// Emulated frames one request arms (REQ-LOOP-016).
pub const BURST_FRAMES: u32 = 30;

/// The burst in progress.
#[derive(Debug, Clone, Copy, Default)]
pub struct FastForwardState {
    remaining: u32,
}

impl FastForwardState {
    /// Creates the state with no burst running.
    #[must_use]
    pub const fn new() -> Self {
        Self { remaining: 0 }
    }

    /// Whether a burst is running, that is whether `k` is forced to 1.5.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.remaining > 0
    }

    /// Emulated frames left in the current burst, 0 when none runs.
    #[must_use]
    pub const fn remaining(&self) -> u32 {
        self.remaining
    }

    /// One request. Arms a burst when none is running and returns true; does nothing otherwise.
    pub const fn request(&mut self) -> bool {
        if self.remaining > 0 {
            return false;
        }
        self.remaining = BURST_FRAMES;
        true
    }

    /// Accounts for one emulated frame that has just run.
    pub const fn on_frame_run(&mut self) {
        self.remaining = self.remaining.saturating_sub(1);
    }

    /// Drops the burst: what stopping a game or loading a state does.
    pub const fn clear(&mut self) {
        self.remaining = 0;
    }
}
