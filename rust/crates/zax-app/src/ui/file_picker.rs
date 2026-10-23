//! The choice of a file or folder outside the `ImGui` browser: what is asked of the platform's own
//! dialog and what comes back. The trait keeps the interface flow testable with a fake.

use super::file_browser::FileFilter;

/// What a dialog is asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickRequest {
    /// The title of the dialog.
    pub title: String,
    /// The file types offered; an empty extension list admits every file.
    pub filters: Vec<FileFilter>,
    /// The folder to start in, when it exists.
    pub start_directory: Option<String>,
    /// Whether the answer is a folder.
    pub folder: bool,
}

/// The state of a pick in progress.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickPoll {
    /// Nothing is being picked, or the dialog is still open.
    Pending,
    /// The user chose this path.
    Chosen(String),
    /// The user closed the dialog without choosing.
    Cancelled,
    /// The dialog could not be shown; the caller falls back to the `ImGui` browser.
    Failed,
}

/// A way to ask the user for a path that does not block the frame loop.
pub trait FilePicker {
    /// Starts a pick. Returns false when the picker cannot work here (no desktop portal, a
    /// handheld, the user's opt-out), in which case the caller uses the `ImGui` browser.
    fn start(&mut self, request: PickRequest) -> bool;

    /// Where the pick in progress stands; polled once per frame.
    fn poll(&mut self) -> PickPoll;

    /// Whether the dialog needs the window out of full screen while it is open (Windows: a
    /// full-screen OpenGL window can hide a dialog that is not its child).
    fn needs_windowed(&self) -> bool {
        false
    }
}
