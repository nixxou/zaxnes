//! The library file `library.json` (SPEC-09 REQ-PERS-035, REQ-PERS-037) and the import of the
//! original's `rom.json` (REQ-PERS-025).

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;

use crate::catalogue::ProfileFileSystem;
use crate::format::{ProfileLog, NULL_PROFILE_LOG};

/// One game of the library file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GameLibraryRecord {
    /// Profile identifier. Do not edit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The ROM chosen for this game.
    #[serde(rename = "romPath", skip_serializing_if = "Option::is_none")]
    pub rom_path: Option<String>,
    /// Last time played, in Unix seconds.
    #[serde(rename = "lastPlayed")]
    pub last_played: i64,
    /// Cover art file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,
}

/// The library file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GameLibraryFile {
    /// Schema version of the file.
    #[serde(rename = "schemaVersion")]
    pub schema_version: i32,
    /// A reminder to anyone editing the file by hand.
    pub notice: String,
    /// The games.
    pub games: Vec<GameLibraryRecord>,
}

impl GameLibraryFile {
    /// The schema version this program writes.
    pub const CURRENT_SCHEMA_VERSION: i32 = 1;

    /// The notice written in the file.
    pub const DEFAULT_NOTICE: &'static str = "Do not edit the id field.";
}

impl Default for GameLibraryFile {
    fn default() -> Self {
        Self {
            schema_version: Self::CURRENT_SCHEMA_VERSION,
            notice: Self::DEFAULT_NOTICE.to_owned(),
            games: Vec::new(),
        }
    }
}

/// One entry of the original's `rom.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RomPathRecord {
    /// Profile identifier.
    pub id: String,
    /// The ROM chosen for this game.
    pub rom_path: String,
    /// Title.
    pub title: String,
    /// Last time played, in Unix seconds.
    pub last_time: i64,
}

/// Reads and writes the library file through a [`ProfileFileSystem`].
pub struct GameLibraryStore<'a> {
    files: &'a dyn ProfileFileSystem,
    log: &'a dyn ProfileLog,
}

impl<'a> GameLibraryStore<'a> {
    /// A store over `files`.
    #[must_use]
    pub fn new(files: &'a dyn ProfileFileSystem, log: Option<&'a dyn ProfileLog>) -> Self {
        Self {
            files,
            log: log.unwrap_or(&NULL_PROFILE_LOG),
        }
    }

    /// Reads the library file; none when it does not exist or is unreadable. An unreadable file
    /// is renamed `<path>.corrupt-<yyyyMMdd-HHmmss>` (from `now`) and reported (REQ-PERS-037);
    /// a file written by a newer schema is read as far as it is known and left alone.
    #[must_use]
    pub fn load(&self, path: &Path, now: OffsetDateTime) -> Option<GameLibraryFile> {
        if !self.files.file_exists(path) {
            return None;
        }

        match self.read(path) {
            Ok(file) => {
                if file.schema_version > GameLibraryFile::CURRENT_SCHEMA_VERSION {
                    self.log.warning(&format!(
                        "{}: written by a newer version (schema {}); \
                         known values are used and the file is left alone",
                        path.display(),
                        file.schema_version
                    ));
                }
                Some(file)
            }
            Err(reason) => {
                let aside = corrupt_path(path, now);
                self.log.error(&format!(
                    "{}: unreadable library ({reason}); set aside as {}",
                    path.display(),
                    aside.display()
                ));
                if let Err(failure) = self.files.move_file(path, &aside, true) {
                    self.log.error(&format!(
                        "{}: could not be set aside ({failure})",
                        path.display()
                    ));
                }
                None
            }
        }
    }

    /// Writes the library file through a temporary file, at the current schema version (which
    /// `file` takes on).
    ///
    /// # Errors
    ///
    /// The file could not be written or moved into place.
    pub fn save(&self, path: &Path, file: &mut GameLibraryFile) -> io::Result<()> {
        file.schema_version = GameLibraryFile::CURRENT_SCHEMA_VERSION;
        let mut temporary = path.as_os_str().to_owned();
        temporary.push(".tmp");
        let temporary = PathBuf::from(temporary);
        let text = serde_json::to_string_pretty(file).map_err(io::Error::from)?;
        self.files.write_all_text(&temporary, &text)?;
        self.files.move_file(&temporary, path, true)
    }

    /// Reads the original's `rom.json`; empty, with a warning, when it is unreadable.
    #[must_use]
    pub fn load_rom_paths(&self, path: &Path) -> Vec<RomPathRecord> {
        if !self.files.file_exists(path) {
            return Vec::new();
        }
        let parsed = self
            .files
            .read_all_text(path)
            .map_err(|error| error.to_string())
            .and_then(|text| parse_rom_paths(&text).map_err(|error| error.to_string()));
        match parsed {
            Ok(records) => records,
            Err(reason) => {
                self.log.warning(&format!(
                    "{}: unreadable rom.json ({reason})",
                    path.display()
                ));
                Vec::new()
            }
        }
    }

    fn read(&self, path: &Path) -> Result<GameLibraryFile, String> {
        let text = self
            .files
            .read_all_text(path)
            .map_err(|error| error.to_string())?;
        serde_json::from_str(&text).map_err(|error| error.to_string())
    }
}

/// Reads the `Items` of a `rom.json` text; entries without an identifier are skipped.
///
/// # Errors
///
/// The text is not JSON.
pub fn parse_rom_paths(json: &str) -> Result<Vec<RomPathRecord>, serde_json::Error> {
    let document: Value = serde_json::from_str(json)?;
    let Some(items) = document.get("Items").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    Ok(items
        .iter()
        .filter_map(|item| {
            let id = read_string(item, "id");
            (!id.is_empty()).then(|| RomPathRecord {
                id,
                rom_path: read_string(item, "romPath"),
                title: read_string(item, "title"),
                last_time: item.get("lastTime").and_then(Value::as_i64).unwrap_or(0),
            })
        })
        .collect())
}

fn read_string(item: &Value, name: &str) -> String {
    item.get(name)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn corrupt_path(path: &Path, now: OffsetDateTime) -> PathBuf {
    let mut aside = path.as_os_str().to_owned();
    aside.push(format!(
        ".corrupt-{:04}{:02}{:02}-{:02}{:02}{:02}",
        now.year(),
        u8::from(now.month()),
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    ));
    PathBuf::from(aside)
}
