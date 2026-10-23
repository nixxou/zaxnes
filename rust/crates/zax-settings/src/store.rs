//! The abstract blob store of REQ-PERS-082: the model reads and writes named blobs and knows
//! nothing of the file system. The desktop host maps a name to a file of the user root; a libretro
//! frontend maps it to the folders the host provides (REQ-PERS-086); a test maps it to memory.

use std::collections::HashMap;

/// A store operation that did not complete.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StorageError {
    /// The store refuses writes: the program runs in the degraded mode of REQ-PERS-073.
    #[error("the store refuses writes")]
    ReadOnly,
    /// The host could not complete the operation on the named blob.
    #[error("the store could not complete the operation on '{0}'")]
    Failed(String),
}

/// The blob store the host supplies.
pub trait SettingsStorage {
    /// Whether writes can succeed. False puts the program in the degraded mode of REQ-PERS-073.
    fn is_writable(&self) -> bool;

    /// Whether a blob named `name` exists.
    fn exists(&self, name: &str) -> bool;

    /// The contents of `name`, or none when it does not exist or cannot be read. An unreadable
    /// blob must not fail the caller, which falls back to the defaults (REQ-PERS-022).
    fn read(&self, name: &str) -> Option<Vec<u8>>;

    /// Replaces `name` with `contents` atomically: a temporary blob beside it, then a replace
    /// (REQ-PERS-021).
    ///
    /// # Errors
    /// Returns [`StorageError`] when the store refuses the write or cannot complete it.
    fn write(&mut self, name: &str, contents: &[u8]) -> Result<(), StorageError>;

    /// Renames `name` to `target`, overwriting it. Used to set an unreadable blob aside as
    /// `*.corrupt-<timestamp>` (REQ-PERS-022).
    ///
    /// # Errors
    /// Returns [`StorageError`] when the store refuses or cannot complete the rename.
    fn rename(&mut self, name: &str, target: &str) -> Result<(), StorageError>;

    /// Copies `name` to `target`. Used to keep the `<name>.v<N>.bak` copy a migration leaves behind
    /// (REQ-PERS-024).
    ///
    /// # Errors
    /// Returns [`StorageError`] when the store refuses or cannot complete the copy.
    fn copy(&mut self, name: &str, target: &str) -> Result<(), StorageError>;
}

/// A store that holds its blobs in memory, for tests and for the degraded mode.
#[derive(Debug, Clone, Default)]
pub struct MemoryStorage {
    blobs: HashMap<String, Vec<u8>>,
    read_only: bool,
}

impl MemoryStorage {
    /// Creates an empty store that accepts writes.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an empty store that refuses writes (REQ-PERS-073).
    #[must_use]
    pub fn read_only() -> Self {
        Self {
            blobs: HashMap::new(),
            read_only: true,
        }
    }

    /// Puts `text` under `name`, bypassing the write refusal, for a test that prepares a file.
    pub fn put(&mut self, name: &str, text: &str) {
        self.blobs.insert(name.to_owned(), text.as_bytes().to_vec());
    }

    /// The contents of `name` as text, or none.
    #[must_use]
    pub fn text(&self, name: &str) -> Option<String> {
        self.blobs
            .get(name)
            .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
    }

    /// The names currently held.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.blobs.keys().map(String::as_str)
    }

    fn check_writable(&self) -> Result<(), StorageError> {
        if self.read_only {
            Err(StorageError::ReadOnly)
        } else {
            Ok(())
        }
    }
}

impl SettingsStorage for MemoryStorage {
    fn is_writable(&self) -> bool {
        !self.read_only
    }

    fn exists(&self, name: &str) -> bool {
        self.blobs.contains_key(name)
    }

    fn read(&self, name: &str) -> Option<Vec<u8>> {
        self.blobs.get(name).cloned()
    }

    fn write(&mut self, name: &str, contents: &[u8]) -> Result<(), StorageError> {
        self.check_writable()?;
        self.blobs.insert(name.to_owned(), contents.to_vec());
        Ok(())
    }

    fn rename(&mut self, name: &str, target: &str) -> Result<(), StorageError> {
        self.check_writable()?;
        let bytes = self
            .blobs
            .remove(name)
            .ok_or_else(|| StorageError::Failed(name.to_owned()))?;
        self.blobs.insert(target.to_owned(), bytes);
        Ok(())
    }

    fn copy(&mut self, name: &str, target: &str) -> Result<(), StorageError> {
        self.check_writable()?;
        let bytes = self
            .blobs
            .get(name)
            .cloned()
            .ok_or_else(|| StorageError::Failed(name.to_owned()))?;
        self.blobs.insert(target.to_owned(), bytes);
        Ok(())
    }
}
