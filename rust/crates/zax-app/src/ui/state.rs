//! What the interface shows: the windows that are open, whether the menu bar is visible and
//! whether a modal window holds the input (SPEC-10 REQ-UI-004, REQ-UI-200, Q-073).

/// Which windows are open and which view modes are on.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // one flag per window, as the C# state holds them
pub struct UiState {
    /// Full screen: the menu bar hides (REQ-UI-209).
    pub fullscreen: bool,
    /// Whether a game is loaded; the welcome screen shows otherwise.
    pub has_current_game: bool,
    /// The options window (REQ-UI-202).
    pub options: bool,
    /// The key configuration window (REQ-UI-203).
    pub key_config: bool,
    /// The about window (DEC-10-Q3).
    pub about: bool,
    /// The profile chooser (REQ-UI-201).
    pub profile_chooser: bool,
    /// The file browser (REQ-UI-201).
    pub file_browser: bool,
    /// A confirmation or message dialogue.
    pub dialog: bool,
    /// The platform's own file dialog is open, outside the frame.
    pub native_dialog: bool,
    /// The debug overlay (F3).
    pub debug_overlay: bool,
    /// The window that asks for a note on the frame on screen of the movie (Rust-only).
    pub movie_note: bool,
    /// The hacks of the game in play (Rust-only).
    pub hacks: bool,
}

impl UiState {
    /// The menu bar shows unless the window is in full screen.
    #[must_use]
    pub const fn menu_bar_visible(&self) -> bool {
        !self.fullscreen
    }

    /// The welcome screen shows while no game is loaded (Q-073).
    #[must_use]
    pub const fn welcome_visible(&self) -> bool {
        !self.has_current_game
    }

    /// The debug overlay shows when it is on and the window is not in full screen.
    #[must_use]
    pub const fn debug_overlay_visible(&self) -> bool {
        self.debug_overlay && !self.fullscreen
    }

    /// Whether a modal window holds the input: the file browser, the profile chooser, a
    /// dialogue (REQ-UI-004).
    #[must_use]
    pub const fn is_modal(&self) -> bool {
        self.file_browser
            || self.profile_chooser
            || self.dialog
            || self.native_dialog
            || self.movie_note
    }

    /// Whether any window is open.
    #[must_use]
    pub const fn any_window_open(&self) -> bool {
        self.options
            || self.key_config
            || self.about
            || self.profile_chooser
            || self.file_browser
            || self.dialog
            || self.movie_note
            || self.hacks
    }

    /// Closes every window (Escape).
    pub const fn close_all(&mut self) {
        self.options = false;
        self.key_config = false;
        self.about = false;
        self.profile_chooser = false;
        self.file_browser = false;
        self.dialog = false;
        self.movie_note = false;
        self.hacks = false;
    }
}
