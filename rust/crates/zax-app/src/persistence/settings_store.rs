//! The settings file `settings.json` and the game list `games.json` in the user data folder
//! (SPEC-09 REQ-PERS-020 to REQ-PERS-024), written two seconds after the last change.

use std::fs;
use std::path::PathBuf;

use time::OffsetDateTime;
use zax_settings::catalogue::{self, SettingId};
use zax_settings::games::GameConfigFile;
use zax_settings::model::SettingsModel;
use zax_settings::persistence::{SettingsFile, SettingsLoadStatus};
use zax_settings::store::{SettingsStorage, StorageError};

use crate::user_data_paths::{write_atomic, UserDataPaths};

/// Notification text of an unreadable settings file.
pub const UNREADABLE_MESSAGE: &str = SettingsFile::<UserDataSettingsStorage>::UNREADABLE_MESSAGE;
/// Notification text of a settings file written by a newer version.
pub const NEWER_VERSION_MESSAGE: &str =
    SettingsFile::<UserDataSettingsStorage>::NEWER_VERSION_MESSAGE;
/// Schema version of the settings file.
pub const SCHEMA_VERSION: i64 = SettingsFile::<UserDataSettingsStorage>::CURRENT_SCHEMA_VERSION;
/// How long after a change the file is written, in real seconds.
pub const WRITE_DELAY_SECONDS: f64 = 2.0;

/// The blob store of the settings files: a name is a file of the user root.
#[derive(Debug, Clone)]
pub struct UserDataSettingsStorage {
    root: PathBuf,
    writable: bool,
}

impl UserDataSettingsStorage {
    /// The store over the root of `paths`.
    #[must_use]
    pub fn new(paths: &UserDataPaths) -> Self {
        Self {
            root: paths.root.clone(),
            writable: paths.is_writable,
        }
    }

    fn path_of(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }
}

impl SettingsStorage for UserDataSettingsStorage {
    fn is_writable(&self) -> bool {
        self.writable
    }

    fn exists(&self, name: &str) -> bool {
        self.path_of(name).is_file()
    }

    fn read(&self, name: &str) -> Option<Vec<u8>> {
        fs::read(self.path_of(name)).ok()
    }

    fn write(&mut self, name: &str, contents: &[u8]) -> Result<(), StorageError> {
        if !self.writable {
            return Err(StorageError::ReadOnly);
        }
        write_atomic(&self.path_of(name), contents)
            .map_err(|_| StorageError::Failed(name.to_owned()))
    }

    fn rename(&mut self, name: &str, target: &str) -> Result<(), StorageError> {
        let path = self.path_of(name);
        if !path.is_file() {
            return Err(StorageError::Failed(name.to_owned()));
        }
        fs::rename(path, self.path_of(target)).map_err(|_| StorageError::Failed(name.to_owned()))
    }

    fn copy(&mut self, name: &str, target: &str) -> Result<(), StorageError> {
        let path = self.path_of(name);
        if !path.is_file() {
            return Err(StorageError::Failed(name.to_owned()));
        }
        fs::copy(path, self.path_of(target))
            .map(|_| ())
            .map_err(|_| StorageError::Failed(name.to_owned()))
    }
}

/// The settings file and the game list, with the delayed write.
pub struct SettingsStore {
    file: SettingsFile<UserDataSettingsStorage>,
    games: GameConfigFile<UserDataSettingsStorage>,
    messages: Vec<String>,
    is_read_only_session: bool,
    due_at: Option<f64>,
    pending: bool,
}

impl SettingsStore {
    /// Creates the store over the user root of `paths`.
    #[must_use]
    pub fn new(paths: &UserDataPaths) -> Self {
        Self {
            file: SettingsFile::new(UserDataSettingsStorage::new(paths)),
            games: GameConfigFile::new(UserDataSettingsStorage::new(paths)),
            messages: Vec::new(),
            is_read_only_session: false,
            due_at: None,
            pending: false,
        }
    }

    /// The first notification the last load owes the user.
    #[must_use]
    pub fn message(&self) -> Option<&str> {
        self.messages.first().map(String::as_str)
    }

    /// Every notification the last load owes the user.
    #[must_use]
    pub fn messages(&self) -> &[String] {
        &self.messages
    }

    /// Whether the settings file came from a newer version and is left alone.
    #[must_use]
    pub const fn is_read_only_session(&self) -> bool {
        self.is_read_only_session
    }

    /// The recent ROMs and the profile chosen per ROM.
    #[must_use]
    pub const fn games(&self) -> &GameConfigFile<UserDataSettingsStorage> {
        &self.games
    }

    /// The same, for writing.
    pub fn games_mut(&mut self) -> &mut GameConfigFile<UserDataSettingsStorage> {
        &mut self.games
    }

    /// Reads both files into `model` (the settings) and the game list.
    pub fn load(&mut self, model: &mut SettingsModel, now: OffsetDateTime) {
        self.messages.clear();
        let result = self.file.load(model, now);
        self.is_read_only_session = result.is_read_only_session();
        if let Some(message) = result.message {
            self.messages.push(message.to_owned());
        }
        if let Some(message) = self.games.load(now).message {
            self.messages.push(message.to_owned());
        }
        if result.status == SettingsLoadStatus::Migrated {
            // A migrated file is written again at the current version straight away
            // (REQ-PERS-024).
            self.write_settings(model);
        }
    }

    /// Writes both files now. A file that cannot be written (read-only root, newer version)
    /// keeps the previous content.
    pub fn save(&mut self, model: &SettingsModel) {
        self.due_at = None;
        self.pending = false;
        self.write_settings(model);
        if self.games.save().is_err() {
            // Reported nowhere: the read-only root already raised its notification (REQ-PERS-073).
        }
    }

    fn write_settings(&mut self, model: &SettingsModel) {
        if self.file.save(model).is_err() {
            // As above: the refusal of a read-only root or a newer file is expected.
        }
    }

    /// Notes that `id` changed. A change a script made, or one the command line imposes, is
    /// not a preference and schedules nothing (REQ-PERS-010, Q-066).
    pub fn on_setting_changed(&mut self, model: &SettingsModel, id: SettingId) {
        if !catalogue::descriptor(id).is_persisted() {
            return;
        }
        if model.has_session_value(id) || model.has_command_line_value(id) {
            return;
        }
        self.pending = true;
    }

    /// Writes the files once the delay since the first change has passed. `elapsed` is the
    /// real time since the program started, in seconds.
    pub fn tick(&mut self, model: &SettingsModel, elapsed: f64) {
        if self.pending && self.due_at.is_none() {
            self.due_at = Some(elapsed + WRITE_DELAY_SECONDS);
        }
        if self.due_at.is_some_and(|due| elapsed >= due) {
            self.save(model);
        }
    }

    /// Schedules a write at `elapsed` plus the delay, unless one is already scheduled.
    pub fn schedule_write(&mut self, elapsed: f64) {
        self.due_at.get_or_insert(elapsed + WRITE_DELAY_SECONDS);
    }

    /// Whether a write is waiting.
    #[must_use]
    pub const fn has_pending_write(&self) -> bool {
        self.pending || self.due_at.is_some()
    }
}
