//! A game another application asks the console interface to play: an Android frontend (ES-DE,
//! Daijishou, a launcher) starts Zaxnes on a ROM (GitHub issue #1).
//!
//! The Android activity writes the request in a file of the user data folder, atomically (a
//! `.part` renamed), at start-up and whenever a frontend sends another game while Zaxnes runs;
//! the interface takes it from there. One channel, read by safe code, whatever the moment the
//! request arrives.
//!
//! The file, `launch/request`, holds `key=value` lines:
//! - `rom=<path>`: the ROM to play (the activity copies a `content://` one into the user data
//!   folder first, under its own name, so its saves stay with the game);
//! - `return=1`: quitting the game goes back to the frontend rather than to the list of games.

use std::path::{Path, PathBuf};

/// The folder of the user data that holds the request (and the ROMs copied for it).
pub const FOLDER: &str = "launch";
/// The request's file in [`FOLDER`].
pub const FILE: &str = "request";

/// The folder of the user data where what is to be imported waits (see
/// `zax_profiles::import::folder`): the Android activity copies a `.bundle` opened with or
/// shared to Zaxnes there, and one can be put there by hand.
pub const IMPORT_FOLDER: &str = zax_profiles::import::folder::FOLDER;

/// A game asked by another application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchRequest {
    /// The ROM to play.
    pub rom: PathBuf,
    /// Whether quitting the game goes back to the application that asked for it.
    pub return_on_quit: bool,
}

/// The request `text` holds, if it names a ROM.
#[must_use]
pub fn parse(text: &str) -> Option<LaunchRequest> {
    let mut rom = None;
    let mut return_on_quit = false;
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim() {
            "rom" if !value.trim().is_empty() => rom = Some(PathBuf::from(value.trim())),
            "return" => return_on_quit = value.trim() == "1",
            _ => {}
        }
    }
    rom.map(|rom| LaunchRequest {
        rom,
        return_on_quit,
    })
}

/// The request waiting in the user data folder `root`, taken (its file removed), if there is
/// one.
#[must_use]
pub fn take(root: &Path) -> Option<LaunchRequest> {
    let path = root.join(FOLDER).join(FILE);
    let text = std::fs::read_to_string(&path).ok()?;
    let _ = std::fs::remove_file(&path);
    parse(&text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_names_its_rom_and_whether_quitting_goes_back() {
        assert_eq!(
            parse("rom=/data/launch/Contra (USA).nes\nreturn=1\n"),
            Some(LaunchRequest {
                rom: PathBuf::from("/data/launch/Contra (USA).nes"),
                return_on_quit: true,
            })
        );
        assert_eq!(
            parse("rom=a.nes").map(|request| request.return_on_quit),
            Some(false)
        );
        assert_eq!(parse("return=1\n"), None);
        assert_eq!(parse("rom=\n"), None);
    }

    #[test]
    fn a_request_is_taken_once() {
        let root = std::env::temp_dir().join(format!("zaxnes-launch-{}", std::process::id()));
        let folder = root.join(FOLDER);
        std::fs::create_dir_all(&folder).unwrap_or_default();
        std::fs::write(folder.join(FILE), "rom=game.nes\n").unwrap_or_default();
        assert_eq!(
            take(&root).map(|request| request.rom),
            Some(PathBuf::from("game.nes"))
        );
        assert_eq!(take(&root), None);
        let _ = std::fs::remove_dir_all(&root);
    }
}
