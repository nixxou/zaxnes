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

/// Opens the options window on its RetroAchievements tab, where the account is (Rust-only, owner
/// request: RO-2 part 1 moved that entry out of a menu of its own and into Tools, and the Tools
/// menu is data, so the entry needs a command to name).
pub const ACHIEVEMENTS_ACCOUNT: &str = "ui.achievements_account";

/// Opens the shaders window (Rust-only, owner request): the slang preset of each display mode.
pub const SHOW_SHADERS: &str = "ui.show_shaders";

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
    /// The options window on its RetroAchievements tab, where the account is.
    AchievementsAccount,
    /// The shaders window.
    ShowShaders,
}

impl UiCommand {
    /// Every command of the interface.
    pub const ALL: [Self; 7] = [
        Self::ToggleDebugOverlay,
        Self::CloseWindows,
        Self::ToggleConsole,
        Self::SaveStates,
        Self::LoadStates,
        Self::AchievementsAccount,
        Self::ShowShaders,
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
            Self::AchievementsAccount => ACHIEVEMENTS_ACCOUNT,
            Self::ShowShaders => SHOW_SHADERS,
        }
    }

    /// The command with the stable name `name`.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|command| command.name() == name)
    }
}
