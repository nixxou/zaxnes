//! The persistent JSON files: `settings.json` (SPEC-09 §4.5.1, REQ-PERS-020 to REQ-PERS-024). The
//! shared rules of the files of the user root live here too: the schema version, the set-aside of an
//! unreadable file and the encoding.

use serde_json::{Map, Value};
use time::OffsetDateTime;

use crate::catalogue::{self, SettingId};
use crate::json;
use crate::meta::SettingScope;
use crate::model::{SettingLayer, SettingsModel};
use crate::store::{SettingsStorage, StorageError};

/// What happened while a persistent file was being read (REQ-PERS-022, REQ-PERS-024).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsLoadStatus {
    /// No file: every setting holds its default (REQ-PERS-002).
    Absent,
    /// Read and applied.
    Loaded,
    /// Read, migrated from an earlier schema version and ready to be written again.
    Migrated,
    /// Unreadable: the defaults are in force, the file has been set aside and a notification is owed.
    Unreadable,
    /// Written by a newer version: the known keys are applied and the file is left untouched for the
    /// rest of the session.
    NewerVersion,
}

/// The outcome of a read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsLoadResult {
    /// What happened.
    pub status: SettingsLoadStatus,
    /// The non-blocking notification the host owes the user, or none.
    pub message: Option<&'static str>,
    /// The schema version the file carried.
    pub version: i64,
}

impl SettingsLoadResult {
    /// Whether the file must be left alone for the rest of the session (REQ-PERS-024).
    #[must_use]
    pub fn is_read_only_session(&self) -> bool {
        self.status == SettingsLoadStatus::NewerVersion
    }
}

/// One step of the migration chain of REQ-PERS-024: it rewrites the `values` object of a file at
/// version [`from_version`](Self::from_version) into the shape of the next version.
pub trait SettingsMigration {
    /// The version this step upgrades from; it produces `source_version() + 1`.
    fn source_version(&self) -> i64;

    /// Rewrites `values` in place.
    fn apply(&self, values: &mut Map<String, Value>);
}

/// Why a file could not be written.
#[derive(Debug, thiserror::Error)]
pub enum SaveError {
    /// The file came from a newer version and is left untouched for the rest of the session.
    #[error("the file was created by a newer version and is read-only in this session")]
    ReadOnlySession,
    /// The store refused the write or failed it.
    #[error(transparent)]
    Storage(#[from] StorageError),
    /// The content could not be encoded as JSON.
    #[error(transparent)]
    Encode(#[from] serde_json::Error),
}

/// Reads and writes `settings.json` over a blob store: the user preferences indexed by stable key,
/// a `schemaVersion`, and every key this version does not know, kept as it is.
///
/// Only scope U is persisted, `ui.fullscreen` among them. Unknown keys survive a rewrite
/// (REQ-PERS-023). A retired key (AMD-031 §3) is read without error and dropped. A file that cannot be
/// parsed takes the model back to the defaults, is renamed `settings.json.corrupt-<timestamp>` and asks
/// for the notification of REQ-PERS-022. A single value of the wrong type or outside its domain is
/// replaced on its own, silently (REQ-PERS-003).
pub struct SettingsFile<S> {
    storage: S,
    migrations: Vec<Box<dyn SettingsMigration>>,
    unknown: Map<String, Value>,
    read_only: bool,
}

impl<S: SettingsStorage> SettingsFile<S> {
    /// The blob name of the settings file (REQ-PERS-072).
    pub const DEFAULT_NAME: &'static str = "settings.json";
    /// The schema version this build writes.
    pub const CURRENT_SCHEMA_VERSION: i64 = 1;
    /// Notification of REQ-PERS-022, word for word.
    pub const UNREADABLE_MESSAGE: &'static str = "Settings file was unreadable and has been reset.";
    /// Notification of REQ-PERS-024, word for word.
    pub const NEWER_VERSION_MESSAGE: &'static str = "Settings were created by a newer version.";

    /// Creates a settings file over `storage`, with no migration step.
    #[must_use]
    pub fn new(storage: S) -> Self {
        Self {
            storage,
            migrations: Vec::new(),
            unknown: Map::new(),
            read_only: false,
        }
    }

    /// Sets the migration chain, in any order; the steps are applied by their starting version.
    #[must_use]
    pub fn with_migrations(
        mut self,
        migrations: impl IntoIterator<Item = Box<dyn SettingsMigration>>,
    ) -> Self {
        self.migrations = migrations.into_iter().collect();
        self.migrations
            .sort_by_key(|migration| migration.source_version());
        self
    }

    /// The blob store this file reads and writes.
    #[must_use]
    pub fn storage(&self) -> &S {
        &self.storage
    }

    /// Reads the file into `model`: every U setting it holds is applied through
    /// [`SettingsModel::set_layer`], so that the validation of REQ-PERS-003 and the notification of
    /// REQ-PERS-009 apply exactly as they do to a change from the UI. Reading again applies only the
    /// U settings of the file (REQ-PERS-015).
    pub fn load(&mut self, model: &mut SettingsModel, now: OffsetDateTime) -> SettingsLoadResult {
        self.unknown.clear();
        self.read_only = false;
        let name = Self::DEFAULT_NAME;
        if !self.storage.exists(name) {
            return SettingsLoadResult {
                status: SettingsLoadStatus::Absent,
                message: None,
                version: Self::CURRENT_SCHEMA_VERSION,
            };
        }
        let Some(root) = self.storage.read(name).as_deref().and_then(parse_object) else {
            // The set-aside is best effort, as in the C#: the defaults are in force either way.
            let _ = self.storage.rename(name, &corrupt_name(name, now));
            return SettingsLoadResult {
                status: SettingsLoadStatus::Unreadable,
                message: Some(Self::UNREADABLE_MESSAGE),
                version: 0,
            };
        };
        let version = schema_version(&root, Self::CURRENT_SCHEMA_VERSION);
        let mut values = root
            .get("values")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_else(Map::new);
        let (status, message) = match version.cmp(&Self::CURRENT_SCHEMA_VERSION) {
            std::cmp::Ordering::Greater => {
                self.read_only = true;
                (
                    SettingsLoadStatus::NewerVersion,
                    Some(Self::NEWER_VERSION_MESSAGE),
                )
            }
            std::cmp::Ordering::Less => {
                // Keep a copy at the old version before the first migrated rewrite (REQ-PERS-024). The copy
                // is best effort, as in the C#: a failed copy must not block the migration.
                let _ = self.storage.copy(name, &backup_name(name, version));
                self.migrate(&mut values, version);
                (SettingsLoadStatus::Migrated, None)
            }
            std::cmp::Ordering::Equal => (SettingsLoadStatus::Loaded, None),
        };
        self.apply(model, &values);
        SettingsLoadResult {
            status,
            message,
            version,
        }
    }

    /// Writes the file atomically (REQ-PERS-021), with the user preferences of `model`.
    ///
    /// # Errors
    /// Returns [`SaveError::ReadOnlySession`] for a file from a newer version, [`SaveError::Storage`]
    /// when the store refuses or fails the write (REQ-PERS-073), and [`SaveError::Encode`] when the
    /// content cannot be encoded.
    pub fn save(&mut self, model: &SettingsModel) -> Result<(), SaveError> {
        if self.read_only {
            return Err(SaveError::ReadOnlySession);
        }
        if !self.storage.is_writable() {
            return Err(StorageError::ReadOnly.into());
        }
        // The known keys are rebuilt from the model, so a preference back at its default is absent
        // from the file and the next default change reaches a user who never touched the setting.
        let mut values = self.unknown.clone();
        for (id, value) in model.user_preferences() {
            if let Some(node) = json::to_json(catalogue::descriptor(id), value) {
                values.insert(id.key().into_owned(), node);
            }
        }
        let root = serde_json::json!({
            "schemaVersion": Self::CURRENT_SCHEMA_VERSION,
            "values": values,
        });
        let bytes = encode(&root)?;
        self.storage.write(Self::DEFAULT_NAME, &bytes)?;
        Ok(())
    }

    fn migrate(&self, values: &mut Map<String, Value>, from_version: i64) {
        let mut version = from_version;
        for migration in &self.migrations {
            if migration.source_version() != version {
                continue;
            }
            migration.apply(values);
            version += 1;
        }
    }

    fn apply(&mut self, model: &mut SettingsModel, values: &Map<String, Value>) {
        for (key, node) in values {
            if catalogue::is_retired_key(key) {
                // AMD-031 §3: a value stored for a retired key is ignored without error and is not kept,
                // so the next save no longer writes it. It is the one exception to REQ-PERS-023.
                continue;
            }
            let Some(id) = SettingId::from_key(key) else {
                // An unknown key is kept as it is and written back untouched (REQ-PERS-023).
                self.unknown.insert(key.clone(), node.clone());
                continue;
            };
            let descriptor = catalogue::descriptor(id);
            if descriptor.scope() != SettingScope::User {
                // A P or S setting in the file is ignored, not an error (REQ-PERS-020).
                continue;
            }
            // A value of the wrong type keeps the default, silently (REQ-PERS-022).
            if let Some(value) = json::from_json(descriptor, node) {
                model.set_layer(id, value, SettingLayer::User);
            }
        }
    }
}

/// The name an unreadable blob is set aside under (REQ-PERS-022).
pub(crate) fn corrupt_name(name: &str, now: OffsetDateTime) -> String {
    format!(
        "{name}.corrupt-{:04}{:02}{:02}-{:02}{:02}{:02}",
        now.year(),
        u8::from(now.month()),
        now.day(),
        now.hour(),
        now.minute(),
        now.second(),
    )
}

/// The name of the copy a migration leaves of a file at `version` (REQ-PERS-024).
pub(crate) fn backup_name(name: &str, version: i64) -> String {
    format!("{name}.v{version}.bak")
}

/// The object at the root of `bytes`, or none when the bytes are not JSON or not an object.
pub(crate) fn parse_object(bytes: &[u8]) -> Option<Map<String, Value>> {
    match serde_json::from_slice::<Value>(bytes) {
        Ok(Value::Object(root)) => Some(root),
        _ => None,
    }
}

/// The schema version of a file, or `current` when the file does not give one.
pub(crate) fn schema_version(root: &Map<String, Value>, current: i64) -> i64 {
    root.get("schemaVersion")
        .and_then(Value::as_i64)
        .unwrap_or(current)
}

/// The bytes of `root` as the file holds them: indented UTF-8 without a byte-order mark (§4.5).
pub(crate) fn encode(root: &Value) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec_pretty(root)
}
