//! The user profiles folder and the import of `.3dn` files into it (SPEC-03 REQ-PROF-062,
//! REQ-PROF-063).

use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};

use super::ProfileFileSystem;
use crate::format::{self, ProfileHeader, ProfileLog, NULL_PROFILE_LOG};
use crate::text;

/// Where a user profile was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileOrigin {
    /// The user's profiles folder.
    User,
    /// The profiles folder next to the executable (AMD-033).
    ProgramFolder,
}

/// A profile file of a profiles folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserProfile {
    /// Identifier.
    pub id: String,
    /// Title; the file name without extension when the profile has none.
    pub title: String,
    /// The file.
    pub file_path: PathBuf,
    /// The identity fields of the file.
    pub header: ProfileHeader,
    /// The folder it was found in.
    pub origin: ProfileOrigin,
}

/// What an import did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileImportOutcome {
    /// A new profile was added.
    Added,
    /// An existing profile of the same identifier was replaced.
    Replaced,
    /// The identifier belongs to a built-in profile; nothing was imported.
    AlreadyBuiltIn,
    /// The file was refused.
    Rejected,
}

/// The result of an import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileImportResult {
    /// What happened.
    pub outcome: ProfileImportOutcome,
    /// The imported profile, for [`ProfileImportOutcome::Added`] and
    /// [`ProfileImportOutcome::Replaced`].
    pub profile: Option<UserProfile>,
    /// The reason of a refusal, empty otherwise.
    pub message: String,
}

/// The messages of a refused or ignored profile file. The wording, misspelling included, is the
/// original's (REQ-PROF-063).
pub mod import_messages {
    use std::path::Path;

    /// The file is not a `3dn` file.
    pub const NOT_A_3DN_FILE: &str = "This file is not a 3dn file";

    /// The format version is not supported.
    #[must_use]
    pub fn unsupported_version(version: &str) -> String {
        format!("3dn format version {version}  is not supported")
    }

    /// The file is unreadable or has no identifier.
    #[must_use]
    pub fn obsolete_format(path: &Path) -> String {
        format!(
            "{}has an obsolate or wrong profile format. \
             This profile is not compatible with the current version of Zaxnes.",
            path.display()
        )
    }

    /// The identifier is already taken.
    #[must_use]
    pub fn duplicated_profile(path: &Path, id: &str) -> String {
        format!(
            "{}: duplicated profile, the identifier \"{id}\" is already in the library",
            path.display()
        )
    }
}

/// A folder of `.3dn` profiles.
pub struct UserProfileFolder<'a> {
    files: &'a dyn ProfileFileSystem,
    log: &'a dyn ProfileLog,
    directory: PathBuf,
    origin: ProfileOrigin,
}

impl<'a> UserProfileFolder<'a> {
    /// File extension of a profile.
    pub const EXTENSION: &'static str = ".3dn";

    /// The folder `directory`, read through `files`; `origin` is stamped on what it yields.
    #[must_use]
    pub fn new(
        files: &'a dyn ProfileFileSystem,
        directory: &Path,
        log: Option<&'a dyn ProfileLog>,
        origin: ProfileOrigin,
    ) -> Self {
        Self {
            files,
            log: log.unwrap_or(&NULL_PROFILE_LOG),
            directory: directory.to_path_buf(),
            origin,
        }
    }

    /// The folder.
    #[must_use]
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// The origin stamped on the profiles of this folder.
    #[must_use]
    pub fn origin(&self) -> ProfileOrigin {
        self.origin
    }

    /// Reads the folder in the ordinal order of the file names, so that the winner of a
    /// duplicated identifier does not depend on the file system (REQ-PROF-062). Identifiers in
    /// `reserved_ids` (the built-in profiles) are taken; unreadable, foreign and identifier-less
    /// files are skipped with a warning.
    ///
    /// # Errors
    ///
    /// The folder exists but could not be listed.
    pub fn scan(&self, reserved_ids: &[&str]) -> io::Result<Vec<UserProfile>> {
        let mut paths = self.files.list_files(&self.directory, Self::EXTENSION)?;
        sort_ordinal(&mut paths);

        let mut taken: HashSet<String> = reserved_ids.iter().map(|id| (*id).to_owned()).collect();
        let mut profiles = Vec::new();
        for path in paths {
            let Some(profile) = self.try_read(&path) else {
                continue;
            };
            if !taken.insert(profile.id.clone()) {
                self.log
                    .warning(&import_messages::duplicated_profile(&path, &profile.id));
                continue;
            }
            profiles.push(profile);
        }
        Ok(profiles)
    }

    /// Imports `source_path` into this folder (REQ-PROF-063): the file is copied under its own
    /// name, replacing the profile of the same identifier in `user_profiles` if there is one. A
    /// file that is not an acceptable profile is a [`ProfileImportOutcome::Rejected`] result,
    /// not an error.
    ///
    /// # Errors
    ///
    /// The source could not be read, or the old file could not be deleted or the new one
    /// copied.
    pub fn import(
        &self,
        source_path: &Path,
        built_in_ids: &[&str],
        user_profiles: &[UserProfile],
    ) -> io::Result<ProfileImportResult> {
        if !self.files.file_exists(source_path) {
            return Ok(self.reject(import_messages::obsolete_format(source_path)));
        }

        let header = format::load_header(
            &self.files.read_all_bytes(source_path)?,
            Some(self.log),
            Some(&source_path.to_string_lossy()),
        );
        let Some(header) = header else {
            return Ok(self.reject(import_messages::obsolete_format(source_path)));
        };
        if !header.has_acceptable_signature() {
            return Ok(self.reject(import_messages::NOT_A_3DN_FILE.to_owned()));
        }
        // An absent or empty identifier is refused: there would be no way to key the library
        // entry, the slots or the cover art on it (REQ-PROF-063).
        if !header.has_id() {
            return Ok(self.reject(import_messages::obsolete_format(source_path)));
        }

        let id = header.id.clone().unwrap_or_default();
        if built_in_ids.contains(&id.as_str()) {
            self.log.information(&format!(
                "{}: the identifier \"{id}\" belongs to a built-in profile; nothing imported",
                source_path.display()
            ));
            return Ok(ProfileImportResult {
                outcome: ProfileImportOutcome::AlreadyBuiltIn,
                profile: None,
                message: String::new(),
            });
        }

        let destination = self
            .directory
            .join(source_path.file_name().unwrap_or_default());
        let replaced = user_profiles.iter().find(|profile| profile.id == id);
        if let Some(replaced) = replaced {
            if !same_path(&replaced.file_path, &destination) {
                // Same identifier under another file name: the old file goes away.
                self.files.delete_file(&replaced.file_path)?;
            }
        }

        self.files.create_directory(&self.directory)?;
        if !same_path(source_path, &destination) {
            self.files.copy_file(source_path, &destination, true)?;
        }

        let imported = UserProfile {
            title: title_of(&header, &destination),
            id,
            file_path: destination,
            header,
            origin: ProfileOrigin::User,
        };
        Ok(ProfileImportResult {
            outcome: if replaced.is_none() {
                ProfileImportOutcome::Added
            } else {
                ProfileImportOutcome::Replaced
            },
            profile: Some(imported),
            message: String::new(),
        })
    }

    /// Deletes the file of `profile`.
    ///
    /// # Errors
    ///
    /// The file exists but could not be deleted.
    pub fn delete(&self, profile: &UserProfile) -> io::Result<()> {
        self.files.delete_file(&profile.file_path)
    }

    /// Reads the header of the profile file `path`; none, with a warning, when the file is
    /// unreadable, foreign or has no identifier.
    #[must_use]
    pub fn try_read(&self, path: &Path) -> Option<UserProfile> {
        let bytes = match self.files.read_all_bytes(path) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.log
                    .warning(&format!("{}: unreadable file ({error})", path.display()));
                return None;
            }
        };

        let header = format::load_header(&bytes, Some(self.log), Some(&path.to_string_lossy()))?;
        if !header.has_acceptable_signature() {
            self.log.warning(&format!(
                "{}: ignored, {}",
                path.display(),
                import_messages::NOT_A_3DN_FILE
            ));
            return None;
        }
        if !header.has_id() {
            self.log.warning(&format!(
                "{}: ignored, the profile carries no identifier",
                path.display()
            ));
            return None;
        }

        Some(UserProfile {
            id: header.id.clone().unwrap_or_default(),
            title: title_of(&header, path),
            file_path: path.to_path_buf(),
            header,
            origin: self.origin,
        })
    }

    fn reject(&self, message: String) -> ProfileImportResult {
        self.log.warning(&message);
        ProfileImportResult {
            outcome: ProfileImportOutcome::Rejected,
            profile: None,
            message,
        }
    }
}

fn title_of(header: &ProfileHeader, path: &Path) -> String {
    match header.title.as_deref() {
        Some(title) if !title.is_empty() => title.to_owned(),
        _ => path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default(),
    }
}

/// Sorts paths by the ordinal order of their text.
pub(crate) fn sort_ordinal(paths: &mut [PathBuf]) {
    paths.sort_by(|left, right| text::ordinal(&left.to_string_lossy(), &right.to_string_lossy()));
}

/// True when two paths are the same text, ignoring case.
pub(crate) fn same_path(left: &Path, right: &Path) -> bool {
    text::ordinal_ignore_case(&left.to_string_lossy(), &right.to_string_lossy()).is_eq()
}
