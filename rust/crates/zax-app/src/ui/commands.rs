//! The commands that belong to the interface itself, beside those of the host's command model
//! (SPEC-10 REQ-UI-205).

/// Shows or hides the debug overlay.
pub const TOGGLE_DEBUG_OVERLAY: &str = "ui.toggle_debug_overlay";

/// Closes every window of the interface.
pub const CLOSE_WINDOWS: &str = "ui.close_windows";

/// Switches the desktop between its menu bar and the console mode (Rust-only, owner request).
pub const TOGGLE_CONSOLE: &str = "ui.toggle_console";

/// Opens the save state screen of the console mode over the game (Rust-only, owner request).
pub const SAVE_STATES: &str = "ui.save_states";

/// Opens the load state screen of the console mode over the game (Rust-only, owner request).
pub const LOAD_STATES: &str = "ui.load_states";

/// A command the interface owns, beside the commands of the host's model (REQ-UI-205).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiCommand {
    /// Shows or hides the debug overlay.
    ToggleDebugOverlay,
    /// Closes every window of the interface.
    CloseWindows,
    /// Switches between the menu bar and the console mode.
    ToggleConsole,
    /// The save state screen of the console mode.
    SaveStates,
    /// The load state screen of the console mode.
    LoadStates,
}

impl UiCommand {
    /// Every command of the interface.
    pub const ALL: [Self; 5] = [
        Self::ToggleDebugOverlay,
        Self::CloseWindows,
        Self::ToggleConsole,
        Self::SaveStates,
        Self::LoadStates,
    ];

    /// The stable name of the command.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::ToggleDebugOverlay => TOGGLE_DEBUG_OVERLAY,
            Self::CloseWindows => CLOSE_WINDOWS,
            Self::ToggleConsole => TOGGLE_CONSOLE,
            Self::SaveStates => SAVE_STATES,
            Self::LoadStates => LOAD_STATES,
        }
    }

    /// The command with the stable name `name`.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|command| command.name() == name)
    }
}
