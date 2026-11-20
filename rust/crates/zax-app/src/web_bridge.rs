//! The web build's exchanges with its page (`rust/web/index.html`), through files of Emscripten's
//! in-memory file system: no foreign call, so the application stays free of `unsafe`.
//!
//! - The application asks the page for one of its dialogues by writing its name
//!   ([`WebDialog::name`]) in [`OPEN_REQUEST`]; the page polls for it, removes it and shows it.
//! - The page answers with the path of the ROM it chose (a prepackaged one, or a file of the
//!   player's machine it copied into the ROM folder) in [`OPEN_ANSWER`], which the application
//!   reads once a frame; a second line `profile=<id or title>` names the 3D profile to play it
//!   with (`?profile=` of the page's address).
//! - The page puts the 3D profiles (`.3dn`) and 3dSen data (`.bundle`) the player imports in the
//!   user data's import folder; the application imports them and tells how it went in
//!   [`IMPORT_RESULT`], which the page shows in its dialogue.
//!
//! On every other platform the functions do nothing.

/// The folder of the exchange files.
pub const FOLDER: &str = "/zaxnes-web";
/// Written by the application: the dialogue to show ([`WebDialog::name`]).
pub const OPEN_REQUEST: &str = "/zaxnes-web/open-request";
/// Written by the page: the ROM to open, as a path of the file system.
pub const OPEN_ANSWER: &str = "/zaxnes-web/open-answer";
/// Written by the application: one line per imported file, `ok|<profile id>|<count>|<text>` or
/// `error||0|<text>` (the identifier empty for a bundle, which brings many; the count, how many
/// profiles it brought).
pub const IMPORT_RESULT: &str = "/zaxnes-web/import-result";

/// Written by the application: `playing` while a game is on the screen, `idle` otherwise.
pub const STATE: &str = "/zaxnes-web/state";

/// A dialogue of the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebDialog {
    /// Open (or import) a ROM.
    Rom,
    /// Import 3D profiles and 3dSen data.
    Profile,
    /// What the browser keeps, and its management.
    Storage,
}

impl WebDialog {
    /// Its name in [`OPEN_REQUEST`].
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Rom => "rom",
            Self::Profile => "profile",
            Self::Storage => "storage",
        }
    }
}

/// What the page chose to play.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenAnswer {
    /// The ROM, as a path of the file system.
    pub path: String,
    /// The 3D profile asked for it, by identifier or title, if any.
    pub profile: Option<String>,
}

impl OpenAnswer {
    /// The answer the text of [`OPEN_ANSWER`] holds: the path on its first line.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let mut lines = text.lines();
        let path = lines.next()?.trim();
        if path.is_empty() {
            return None;
        }
        let profile = lines
            .filter_map(|line| line.trim().strip_prefix("profile="))
            .map(str::trim)
            .find(|profile| !profile.is_empty())
            .map(str::to_owned);
        Some(Self {
            path: path.to_owned(),
            profile,
        })
    }
}

/// Whether this build talks to a page.
#[must_use]
pub const fn active() -> bool {
    cfg!(target_os = "emscripten")
}

/// Asks the page for its ROM dialogue.
pub fn request_open_dialog() {
    request_dialog(WebDialog::Rom);
}

/// Asks the page for `dialog`.
pub fn request_dialog(dialog: WebDialog) {
    if active() {
        let _ = std::fs::create_dir_all(FOLDER);
        let _ = std::fs::write(OPEN_REQUEST, dialog.name());
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

/// How the import of one file went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportResult {
    /// Whether it went through.
    pub ok: bool,
    /// The identifier of the profile it became, for a single profile.
    pub id: Option<String>,
    /// How many profiles it brought (the page's progress window says it).
    pub imported: usize,
    /// What to tell the player.
    pub text: String,
}

/// Tells the page how the import of its files went.
pub fn report_import(results: &[ImportResult]) {
    if !active() || results.is_empty() {
        return;
    }
    let text: String = results
        .iter()
        .map(|result| {
            let flat = result.text.replace(['\n', '\r'], " ");
            let id = result.id.as_deref().unwrap_or("").replace(['|', '\n', '\r'], " ");
            format!(
                "{}|{id}|{}|{flat}\n",
                if result.ok { "ok" } else { "error" },
                result.imported
            )
        })
        .collect();
    let _ = std::fs::create_dir_all(FOLDER);
    let _ = std::fs::write(IMPORT_RESULT, text);
}

/// What the page chose since the last call, if anything.
#[must_use]
pub fn take_open_answer() -> Option<OpenAnswer> {
    if !active() {
        return None;
    }
    let text = std::fs::read_to_string(OPEN_ANSWER).ok()?;
    let _ = std::fs::remove_file(OPEN_ANSWER);
    OpenAnswer::parse(&text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_answer_is_a_path_and_maybe_a_profile() {
        assert_eq!(None, OpenAnswer::parse("\n"));
        assert_eq!(
            Some(OpenAnswer {
                path: "/roms/a.nes".to_owned(),
                profile: None
            }),
            OpenAnswer::parse("/roms/a.nes")
        );
        assert_eq!(
            Some("Super Mario Bros.".to_owned()),
            OpenAnswer::parse("/roms/a.nes\nprofile= Super Mario Bros. \n")
                .and_then(|answer| answer.profile)
        );
    }
}
