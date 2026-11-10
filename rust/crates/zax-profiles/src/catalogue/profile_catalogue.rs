//! The catalogue of the built-in profiles (SPEC-03 REQ-PROF-060, REQ-PROF-061): `profiles.json`
//! with a `full` and a `demo` edition.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::ProfileFileSystem;
use crate::format::{
    self, ProfileDocument, ProfileHeader, ProfileLoadError, ProfileLoadErrorKind, ProfileLog,
    NULL_PROFILE_LOG,
};

/// Title of the "2D Mode" entry, which plays a ROM without conversion.
pub const TWO_DIMENSIONAL_MODE_TITLE: &str = "2D Mode";

/// Q-065: title of the entry that plays a ROM in 3D with the default geometry.
pub const DEFAULT_THREE_DIMENSIONAL_TITLE: &str = "3D par défaut (sans profil)";

/// The edition the application uses.
pub const FULL_EDITION: &str = "full";

/// The edition kept as data (DEC-03-Q04).
pub const DEMO_EDITION: &str = "demo";

/// One built-in game of the catalogue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogueEntry {
    /// Identifier.
    pub id: String,
    /// Title.
    pub title: String,
    /// Publisher.
    pub publisher: String,
    /// Description.
    pub description: String,
    /// Author.
    pub author: String,
    /// Profile file, relative to the asset root with `/` separators; empty for a generic entry.
    pub file: String,
}

impl CatalogueEntry {
    /// True for the "2D Mode" entry.
    #[must_use]
    pub fn is_2d_mode(&self) -> bool {
        self.title == TWO_DIMENSIONAL_MODE_TITLE
    }

    /// Q-065: true for the "3D by default" entry.
    #[must_use]
    pub fn is_default_three_dimensional(&self) -> bool {
        self.title == DEFAULT_THREE_DIMENSIONAL_TITLE
    }
}

/// The catalogue could not be read.
#[derive(Debug, thiserror::Error)]
pub enum ProfileCatalogueError {
    /// The catalogue file does not exist.
    #[error("the catalogue file {} is missing", .0.display())]
    Missing(PathBuf),
    /// The catalogue file could not be read.
    #[error("the catalogue file {} could not be read ({source})", .path.display())]
    Unreadable {
        /// The file.
        path: PathBuf,
        /// The cause.
        source: std::io::Error,
    },
    /// The text is not JSON.
    #[error("the catalogue is not valid JSON ({0})")]
    InvalidJson(#[from] serde_json::Error),
    /// The JSON has no `editions` object.
    #[error("the catalogue has no \"editions\" object")]
    NoEditions,
    /// The JSON has no such edition.
    #[error("the catalogue has no \"{0}\" edition")]
    NoEdition(String),
}

/// The built-in profile files as the start-up reads them (AMD-037, AMD-065 §2).
#[derive(Debug, Clone, Default)]
pub struct BuiltInProfiles {
    contents: Vec<Vec<u8>>,
    headers: HashMap<String, ProfileHeader>,
}

impl BuiltInProfiles {
    /// The bytes of every file that could be read, for AMD-037's byte-for-byte comparison.
    #[must_use]
    pub fn contents(&self) -> &[Vec<u8>] {
        &self.contents
    }

    /// The bytes, to keep for the whole session without copying them.
    #[must_use]
    pub fn into_contents(self) -> Vec<Vec<u8>> {
        self.contents
    }

    /// The identity fields the file of the catalogue entry `catalogue_id` carries; none when the
    /// entry has no file or its header could not be read.
    #[must_use]
    pub fn header(&self, catalogue_id: &str) -> Option<&ProfileHeader> {
        self.headers.get(catalogue_id)
    }
}

/// The built-in catalogue of one edition.
#[derive(Debug, Clone)]
pub struct ProfileCatalogue {
    asset_root: PathBuf,
    entries: Vec<CatalogueEntry>,
    by_id: HashMap<String, usize>,
}

impl ProfileCatalogue {
    /// A catalogue from its entries, with the profile files under `asset_root`.
    #[must_use]
    pub fn from_entries(asset_root: &Path, entries: Vec<CatalogueEntry>) -> Self {
        // A later entry of the same identifier replaces the earlier one in lookups.
        let by_id = entries
            .iter()
            .enumerate()
            .map(|(index, entry)| (entry.id.clone(), index))
            .collect();
        Self {
            asset_root: asset_root.to_path_buf(),
            entries,
            by_id,
        }
    }

    /// The folder the profile files are relative to.
    #[must_use]
    pub fn asset_root(&self) -> &Path {
        &self.asset_root
    }

    /// The entries, in catalogue order.
    #[must_use]
    pub fn entries(&self) -> &[CatalogueEntry] {
        &self.entries
    }

    /// The "2D Mode" entry, or none.
    #[must_use]
    pub fn two_dimensional_mode(&self) -> Option<&CatalogueEntry> {
        self.entries.iter().find(|entry| entry.is_2d_mode())
    }

    /// The identifiers of the catalogue.
    #[must_use]
    pub fn ids(&self) -> Vec<&str> {
        self.by_id.keys().map(String::as_str).collect()
    }

    /// The entry of `id`.
    #[must_use]
    pub fn find(&self, id: &str) -> Option<&CatalogueEntry> {
        self.by_id.get(id).map(|index| &self.entries[*index])
    }

    /// Reads the `edition` of the catalogue JSON `json`.
    ///
    /// # Errors
    ///
    /// The text is not JSON, or has no `editions` object or no such edition.
    pub fn parse(
        json: &str,
        asset_root: &Path,
        edition: &str,
    ) -> Result<Self, ProfileCatalogueError> {
        let document: Value = serde_json::from_str(json)?;
        let editions = document
            .get("editions")
            .ok_or(ProfileCatalogueError::NoEditions)?;
        let list = editions
            .get(edition)
            .and_then(Value::as_array)
            .ok_or_else(|| ProfileCatalogueError::NoEdition(edition.to_owned()))?;
        let entries = list
            .iter()
            .map(|item| CatalogueEntry {
                id: read_string(item, "id"),
                title: read_string(item, "title"),
                publisher: read_string(item, "publisher"),
                description: read_string(item, "description"),
                author: read_string(item, "author"),
                file: read_string(item, "file"),
            })
            .collect();
        Ok(Self::from_entries(asset_root, entries))
    }

    /// Reads the catalogue file `catalogue_path`; the profile files are relative to its folder.
    ///
    /// # Errors
    ///
    /// The file is missing or unreadable, or [`Self::parse`] refuses its content.
    pub fn load(
        files: &dyn ProfileFileSystem,
        catalogue_path: &Path,
        edition: &str,
    ) -> Result<Self, ProfileCatalogueError> {
        if !files.file_exists(catalogue_path) {
            return Err(ProfileCatalogueError::Missing(catalogue_path.to_path_buf()));
        }
        let root = catalogue_path.parent().unwrap_or(Path::new(""));
        let text = files.read_all_text(catalogue_path).map_err(|source| {
            ProfileCatalogueError::Unreadable {
                path: catalogue_path.to_path_buf(),
                source,
            }
        })?;
        Self::parse(&text, root, edition)
    }

    /// The path of the profile file of `entry`; empty for an entry without a file.
    #[must_use]
    pub fn profile_path(&self, entry: &CatalogueEntry) -> PathBuf {
        if entry.file.is_empty() {
            PathBuf::new()
        } else {
            self.asset_root.join(&entry.file)
        }
    }

    /// The bytes of the profile files of the entries, to recognise a user copy of a built-in
    /// profile (AMD-037). Entries without a file, and files that cannot be read, are left out.
    #[must_use]
    pub fn built_in_contents(&self, files: &dyn ProfileFileSystem) -> Vec<Vec<u8>> {
        self.read_built_in_profiles(files).contents
    }

    /// The profile files of the entries, read **once** for the two uses the start-up has for
    /// them: their bytes, which AMD-037 compares a user file against, and the identity fields
    /// each carries, which decide the same game and the generation (AMD-065 REQ-PROF-085,
    /// REQ-PROF-086).
    ///
    /// Entries without a file, and files that cannot be read, are left out of the bytes;
    /// a file whose header cannot be read keeps its bytes and gets no header, its name alone
    /// then deciding the game.
    #[must_use]
    pub fn read_built_in_profiles(&self, files: &dyn ProfileFileSystem) -> BuiltInProfiles {
        let mut read = BuiltInProfiles::default();
        for entry in self.entries.iter().filter(|entry| !entry.file.is_empty()) {
            let path = self.profile_path(entry);
            let Ok(bytes) = files.read_all_bytes(&path) else {
                continue;
            };
            if let Some(header) = format::load_header(&bytes, None, Some(&path.to_string_lossy())) {
                read.headers.insert(entry.id.clone(), header);
            }
            read.contents.push(bytes);
        }
        read
    }

    /// Loads the profile of `entry`.
    ///
    /// # Errors
    ///
    /// The profile file is missing, or [`format::load`] refuses it.
    pub fn load_profile(
        &self,
        files: &dyn ProfileFileSystem,
        entry: &CatalogueEntry,
        log: Option<&dyn ProfileLog>,
    ) -> Result<ProfileDocument, ProfileLoadError> {
        let path = self.profile_path(entry);
        if !files.file_exists(&path) {
            log.unwrap_or(&NULL_PROFILE_LOG).error(&format!(
                "{}: the built-in profile file is missing",
                path.display()
            ));
            return Err(ProfileLoadError {
                kind: ProfileLoadErrorKind::Malformed,
                message: "the profile file is missing".to_owned(),
            });
        }
        let bytes = files
            .read_all_bytes(&path)
            .map_err(|error| ProfileLoadError {
                kind: ProfileLoadErrorKind::Malformed,
                message: format!("the profile file could not be read ({error})"),
            })?;
        format::load(&bytes, log, Some(&path.to_string_lossy()))
    }
}

fn read_string(item: &Value, name: &str) -> String {
    item.get(name)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}
