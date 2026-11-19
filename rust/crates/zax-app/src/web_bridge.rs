//! The web build's exchanges with its page (`rust/web/index.html`), through files of Emscripten's
//! in-memory file system: no foreign call, so the application stays free of `unsafe`.
//!
//! - The application asks the page for its ROM dialogue by creating [`OPEN_REQUEST`]; the page
//!   polls for it, removes it and shows the dialogue.
//! - The page answers with the path of the ROM it chose (a prepackaged one, or a file of the
//!   player's machine it copied into the ROM folder) in [`OPEN_ANSWER`], which the application
//!   reads once a frame.
//!
//! On every other platform the functions do nothing.

/// The folder of the exchange files.
pub const FOLDER: &str = "/zaxnes-web";
/// Created by the application: show the ROM dialogue.
pub const OPEN_REQUEST: &str = "/zaxnes-web/open-request";
/// Written by the page: the ROM to open, as a path of the file system.
pub const OPEN_ANSWER: &str = "/zaxnes-web/open-answer";

/// Written by the application: `playing` while a game is on the screen, `idle` otherwise.
pub const STATE: &str = "/zaxnes-web/state";

/// Whether this build talks to a page.
#[must_use]
pub const fn active() -> bool {
    cfg!(target_os = "emscripten")
}

/// Asks the page for its ROM dialogue.
pub fn request_open_dialog() {
    if active() {
        let _ = std::fs::create_dir_all(FOLDER);
        let _ = std::fs::write(OPEN_REQUEST, b"rom");
    }
}

/// Tells the page whether a game is on the screen; writes only when that changes.
pub fn report_playing(playing: bool) {
    use std::sync::atomic::{AtomicU8, Ordering};
    // 0 never reported, 1 idle, 2 playing.
    static LAST: AtomicU8 = AtomicU8::new(0);
    if !active() {
        return;
    }
    let now = if playing { 2 } else { 1 };
    if LAST.swap(now, Ordering::Relaxed) != now {
        let _ = std::fs::create_dir_all(FOLDER);
        let _ = std::fs::write(STATE, if playing { "playing" } else { "idle" });
    }
}

/// The ROM the page chose since the last call, if any.
#[must_use]
pub fn take_open_answer() -> Option<String> {
    if !active() {
        return None;
    }
    let text = std::fs::read_to_string(OPEN_ANSWER).ok()?;
    let _ = std::fs::remove_file(OPEN_ANSWER);
    let path = text.trim();
    (!path.is_empty()).then(|| path.to_owned())
}
