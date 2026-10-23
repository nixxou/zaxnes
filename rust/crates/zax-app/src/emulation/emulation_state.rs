//! The three states the emulation is always in exactly one of (SPEC-07 REQ-LOOP-001).

/// The state of the emulation. There is no "idle" state distinct from `Stopped`, and the loading
/// indicator is orthogonal to the state (see `EmulationHost::is_loading`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmulationState {
    /// No game is running. This is the state the application starts in.
    Stopped,
    /// A game is loaded, its state is kept, no emulated frame runs.
    Paused,
    /// Emulated frames follow one another at the target rate.
    Running,
}
