//! The per-ROM choices beside the library: the profile the user picked for a ROM file and the ten
//! most recent ROMs (AMD-010 REQ-UI-200, REQ-UI-201, REQ-UI-207), and the options of a game
//! (Rust-only, owner request: the few settings the user set for that ROM, only those that differ
//! from the preferences; REQ-PERS-031 knew none). Its key is a path, not a profile identifier, so
//! it lives beside `library.json` (Q-036).

use serde_json::{Map, Value};
use time::OffsetDateTime;

use crate::catalogue::{self, SettingId};
use crate::json;
use crate::persistence::{self, SaveError, SettingsLoadResult, SettingsLoadStatus};
use crate::store::{SettingsStorage, StorageError};
use crate::values::SettingValue;

/// The options of one game: the settings it holds a value of its own for.
pub type GameOptions = Vec<(SettingId, SettingValue)>;

/// The per-ROM choices of the user root, over a blob store.
pub struct GameConfigFile<S> {
    storage: S,
    recent: Vec<String>,
    rom_profiles: Vec<(String, String)>,
    rom_options: Vec<(String, GameOptions)>,
    read_only: bool,
    dirty: bool,
}

impl<S: SettingsStorage> GameConfigFile<S> {
    /// The blob name, beside `library.json` in the user root.
    pub const DEFAULT_NAME: &'static str = "games.json";
    /// The schema version this build writes.
    pub const CURRENT_SCHEMA_VERSION: i64 = 1;
    /// How many recent ROMs the File menu lists (AMD-010 REQ-UI-200).
    pub const RECENT_ROM_LIMIT: usize = 10;
    /// Notification shown when the file could not be read.
    pub const UNREADABLE_MESSAGE: &'static str = "Game list was unreadable and has been reset.";
    /// Notification shown for a file from a newer version.
    pub const NEWER_VERSION_MESSAGE: &'static str = "Game list was created by a newer version.";

    /// Creates the file over `storage`; nothing is read until [`load`](Self::load).
    #[must_use]
    pub fn new(storage: S) -> Self {
        Self {
            storage,
            recent: Vec::new(),
            rom_profiles: Vec::new(),
            rom_options: Vec::new(),
            read_only: false,
            dirty: false,
        }
    }

    /// The blob store this file reads and writes.
    #[must_use]
    pub fn storage(&self) -> &S {
        &self.storage
    }

    /// The recent ROM paths, most recent first, at most [`RECENT_ROM_LIMIT`](Self::RECENT_ROM_LIMIT).
    #[must_use]
    pub fn recent_roms(&self) -> &[String] {
        &self.recent
    }

    /// Whether a change has not been written yet.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Records that `rom_path` was just started: it moves to the head of the recent list, which keeps
    /// at most ten entries.
    pub fn note_rom_opened(&mut self, rom_path: &str) {
        if rom_path.trim().is_empty() {
            return;
        }
        self.recent.retain(|path| !same_path(path, rom_path));
        self.recent.insert(0, rom_path.to_owned());
        self.recent.truncate(Self::RECENT_ROM_LIMIT);
        self.dirty = true;
    }

    /// Forgets `rom_path`, for a file that has disappeared. Reports whether anything was forgotten.
    pub fn forget_rom(&mut self, rom_path: &str) -> bool {
        if rom_path.trim().is_empty() {
            return false;
        }
        let recent_before = self.recent.len();
        self.recent.retain(|path| !same_path(path, rom_path));
        let removed = self.recent.len() != recent_before;
        let profiles_before = self.rom_profiles.len();
        self.rom_profiles
            .retain(|(path, _)| !same_path(path, rom_path));
        let options_before = self.rom_options.len();
        self.rom_options
            .retain(|(path, _)| !same_path(path, rom_path));
        let dropped =
            self.rom_profiles.len() != profiles_before || self.rom_options.len() != options_before;
        self.dirty |= removed || dropped;
        removed || dropped
    }

    /// The profile identifier the user chose for `rom_path`, or none when this ROM has never been
    /// started (AMD-010 REQ-UI-201 step 1).
    #[must_use]
    pub fn profile_for(&self, rom_path: &str) -> Option<&str> {
        self.rom_profiles
            .iter()
            .find(|(path, _)| same_path(path, rom_path))
            .map(|(_, id)| id.as_str())
    }

    /// Remembers the profile chosen for a ROM file (AMD-010 REQ-UI-201).
    pub fn set_profile_for(&mut self, rom_path: &str, profile_id: &str) {
        if rom_path.trim().is_empty() || profile_id.trim().is_empty() {
            return;
        }
        if self.profile_for(rom_path) == Some(profile_id) {
            return;
        }
        match self
            .rom_profiles
            .iter_mut()
            .find(|(path, _)| same_path(path, rom_path))
        {
            Some((_, id)) => profile_id.clone_into(id),
            None => self
                .rom_profiles
                .push((rom_path.to_owned(), profile_id.to_owned())),
        }
        self.dirty = true;
    }

    /// The options of the game in `rom_path` (Rust-only): empty when it has none.
    #[must_use]
    pub fn options_for(&self, rom_path: &str) -> &[(SettingId, SettingValue)] {
        self.rom_options
            .iter()
            .find(|(path, _)| same_path(path, rom_path))
            .map_or(&[], |(_, options)| options.as_slice())
    }

    /// Remembers the options of the game in `rom_path`; none forgets them.
    pub fn set_options_for(&mut self, rom_path: &str, options: GameOptions) {
        if rom_path.trim().is_empty() || self.options_for(rom_path) == options.as_slice() {
            return;
        }
        self.rom_options
            .retain(|(path, _)| !same_path(path, rom_path));
        if !options.is_empty() {
            self.rom_options.push((rom_path.to_owned(), options));
        }
        self.dirty = true;
    }

    /// Reads the file; an absent file leaves empty lists (REQ-PERS-075).
    pub fn load(&mut self, now: OffsetDateTime) -> SettingsLoadResult {
        self.recent.clear();
        self.rom_profiles.clear();
        self.rom_options.clear();
        self.read_only = false;
        self.dirty = false;
        let name = Self::DEFAULT_NAME;
        if !self.storage.exists(name) {
            return SettingsLoadResult {
                status: SettingsLoadStatus::Absent,
                message: None,
                version: Self::CURRENT_SCHEMA_VERSION,
            };
        }
        let Some(root) = self
            .storage
            .read(name)
            .as_deref()
            .and_then(persistence::parse_object)
        else {
            // The set-aside is best effort, as in the C#: the empty lists are in force either way.
            let _ = self
                .storage
                .rename(name, &persistence::corrupt_name(name, now));
            self.dirty = true;
            return SettingsLoadResult {
                status: SettingsLoadStatus::Unreadable,
                message: Some(Self::UNREADABLE_MESSAGE),
                version: 0,
            };
        };
        let version = persistence::schema_version(&root, Self::CURRENT_SCHEMA_VERSION);
        self.read_recent(&root);
        self.read_profiles(&root);
        self.read_options(&root);
        if version > Self::CURRENT_SCHEMA_VERSION {
            self.read_only = true;
            return SettingsLoadResult {
                status: SettingsLoadStatus::NewerVersion,
                message: Some(Self::NEWER_VERSION_MESSAGE),
                version,
            };
        }
        if version < Self::CURRENT_SCHEMA_VERSION {
            // The copy is best effort, as in the C#: a failed copy must not block the migration.
            let _ = self
                .storage
                .copy(name, &persistence::backup_name(name, version));
            self.dirty = true;
            return SettingsLoadResult {
                status: SettingsLoadStatus::Migrated,
                message: None,
                version,
            };
        }
        SettingsLoadResult {
            status: SettingsLoadStatus::Loaded,
            message: None,
            version,
        }
    }

    /// Writes the file atomically.
    ///
    /// # Errors
    /// Returns [`SaveError::ReadOnlySession`] for a file from a newer version, [`SaveError::Storage`]
    /// when the store refuses or fails the write, and [`SaveError::Encode`] when the content cannot be
    /// encoded.
    pub fn save(&mut self) -> Result<(), SaveError> {
        if self.read_only {
            return Err(SaveError::ReadOnlySession);
        }
        if !self.storage.is_writable() {
            return Err(StorageError::ReadOnly.into());
        }
        let recent: Vec<Value> = self
            .recent
            .iter()
            .map(|path| Value::String(path.clone()))
            .collect();
        let associations: Map<String, Value> = self
            .rom_profiles
            .iter()
            .map(|(path, id)| (path.clone(), Value::String(id.clone())))
            .collect();
        let options: Map<String, Value> = self
            .rom_options
            .iter()
            .map(|(path, options)| {
                let values: Map<String, Value> = options
                    .iter()
                    .filter_map(|(id, value)| {
                        json::to_json(catalogue::descriptor(*id), value)
                            .map(|node| (id.key().into_owned(), node))
                    })
                    .collect();
                (path.clone(), Value::Object(values))
            })
            .collect();
        let root = serde_json::json!({
            "schemaVersion": Self::CURRENT_SCHEMA_VERSION,
            "recentRoms": recent,
            "romProfiles": associations,
            "romOptions": options,
        });
        let bytes = persistence::encode(&root)?;
        self.storage.write(Self::DEFAULT_NAME, &bytes)?;
        self.dirty = false;
        Ok(())
    }

    fn read_recent(&mut self, root: &Map<String, Value>) {
        let Some(entries) = root.get("recentRoms").and_then(Value::as_array) else {
            return;
        };
        for path in entries
            .iter()
            .filter_map(|entry| entry.as_str().filter(|path| !path.is_empty()))
        {
            if self.recent.len() < Self::RECENT_ROM_LIMIT
                && !self.recent.iter().any(|known| same_path(known, path))
            {
                self.recent.push(path.to_owned());
            }
        }
    }

    fn read_profiles(&mut self, root: &Map<String, Value>) {
        let Some(associations) = root.get("romProfiles").and_then(Value::as_object) else {
            return;
        };
        for (path, id) in associations {
            if let Some(id) = id.as_str().filter(|id| !id.is_empty()) {
                match self
                    .rom_profiles
                    .iter_mut()
                    .find(|(known, _)| same_path(known, path))
                {
                    Some((_, value)) => id.clone_into(value),
                    None => self.rom_profiles.push((path.clone(), id.to_owned())),
                }
            }
        }
    }
}

impl<S> GameConfigFile<S> {
    /// Reads `romOptions`: an unknown key or a value of the wrong type is skipped, alone
    /// (REQ-PERS-022).
    fn read_options(&mut self, root: &Map<String, Value>) {
        let Some(games) = root.get("romOptions").and_then(Value::as_object) else {
            return;
        };
        for (path, values) in games {
            let Some(values) = values.as_object() else {
                continue;
            };
            let options: GameOptions = values
                .iter()
                .filter_map(|(key, node)| {
                    let id = SettingId::from_key(key)?;
                    json::from_json(catalogue::descriptor(id), node).map(|value| (id, value))
                })
                .collect();
            if !options.is_empty() && path.trim() != "" {
                self.rom_options
                    .retain(|(known, _)| !same_path(known, path));
                self.rom_options.push((path.clone(), options));
            }
        }
    }
}

/// Whether two ROM paths are the same file as the C# `OrdinalIgnoreCase` compares them.
fn same_path(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}
