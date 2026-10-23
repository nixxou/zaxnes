//! The commands that belong to the interface itself, beside those of the host's command model
//! (SPEC-10 REQ-UI-205).

/// Shows or hides the debug overlay.
pub const TOGGLE_DEBUG_OVERLAY: &str = "ui.toggle_debug_overlay";

/// Closes every window of the interface.
pub const CLOSE_WINDOWS: &str = "ui.close_windows";

/// A command the interface owns, beside the commands of the host's model (REQ-UI-205).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiCommand {
    /// Shows or hides the debug overlay.
    ToggleDebugOverlay,
    /// Closes every window of the interface.
    CloseWindows,
}

impl UiCommand {
    /// Every command of the interface.
    pub const ALL: [Self; 2] = [Self::ToggleDebugOverlay, Self::CloseWindows];

    /// The stable name of the command.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::ToggleDebugOverlay => TOGGLE_DEBUG_OVERLAY,
            Self::CloseWindows => CLOSE_WINDOWS,
        }
    }

    /// The command with the stable name `name`.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|command| command.name() == name)
    }
}
