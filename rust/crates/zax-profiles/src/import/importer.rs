//! The import of the profiles of a 3dSen bundle into the user's profiles folder (AMD-030 §2,
//! §5): each profile is checked with the decoder of SPEC-03 and written unmodified under its
//! `m_Name`, never replacing a file and never leaving a partial one.

use std::io;
use std::path::{Path, PathBuf};

use super::installation::locate_bundle;
use super::profile_bundle::read_text_assets;
use super::text_asset::UnityTextAsset;
use super::UnityAssetFormatError;
use crate::catalogue::{ProfileFileSystem, UserProfileFolder};
use crate::format::{self, ProfileLog, NULL_PROFILE_LOG};

/// What became of one profile of the bundle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreeDSenProfileOutcome {
    /// Written as a new file.
    Imported,
    /// A file with the same bytes was already there.
    AlreadyPresent,
    /// Not a usable profile, or could not be written.
    Failed,
}

/// The result for one profile of the bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreeDSenProfileResult {
    /// The asset's name.
    pub name: String,
    /// What became of it.
    pub outcome: ThreeDSenProfileOutcome,
    /// The file it is in, empty on failure.
    pub file_name: String,
    /// The reason of a failure, empty otherwise.
    pub message: String,
}

/// The results of one import.
#[derive(Debug, Clone)]
pub struct ThreeDSenImportSummary {
    /// The bundle that was read.
    pub bundle_path: PathBuf,
    /// One result per profile of the bundle, in bundle order.
    pub results: Vec<ThreeDSenProfileResult>,
    /// Number of profiles written.
    pub imported: usize,
    /// Number of profiles already there.
    pub already_present: usize,
    /// Number of failures.
    pub failed: usize,
}

impl ThreeDSenImportSummary {
    /// A summary of `results`, with the counts.
    #[must_use]
    pub fn new(bundle_path: &Path, results: Vec<ThreeDSenProfileResult>) -> Self {
        let count = |outcome| {
            results
                .iter()
                .filter(|result| result.outcome == outcome)
                .count()
        };
        Self {
            bundle_path: bundle_path.to_path_buf(),
            imported: count(ThreeDSenProfileOutcome::Imported),
            already_present: count(ThreeDSenProfileOutcome::AlreadyPresent),
            failed: count(ThreeDSenProfileOutcome::Failed),
            results,
        }
    }

    /// True when nothing failed.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.failed == 0
    }

    /// The end-of-import message. AMD-036 §1: it is English, so the command line and the log
    /// always print English; the interface builds its own from the counts.
    #[must_use]
    pub fn message(&self) -> String {
        format!(
            "3dSen: {} profile(s) imported, {} already present, {} failed.",
            self.imported, self.already_present, self.failed
        )
    }
}

/// Why an import did not run.
#[derive(Debug, thiserror::Error)]
pub enum ThreeDSenImportError {
    /// No bundle was found, or the bundle is unreadable.
    #[error(transparent)]
    Format(#[from] UnityAssetFormatError),
    /// The bundle could not be read or the destination folder created.
    #[error("file error: {0}")]
    Io(#[from] io::Error),
}

/// Writes the profiles of a bundle to a destination folder.
pub struct ThreeDSenImporter<'a> {
    files: &'a dyn ProfileFileSystem,
    log: &'a dyn ProfileLog,
    destination: PathBuf,
}

impl<'a> ThreeDSenImporter<'a> {
    /// Suffix of the temporary file a profile is written to before it takes its name.
    pub const PARTIAL_SUFFIX: &'static str = ".part";

    /// Most names tried for one profile (`Name.3dn`, `Name (2).3dn`, ...).
    pub const MAXIMUM_NAME_ATTEMPTS: usize = 1000;

    /// An importer writing to `destination` through `files`.
    #[must_use]
    pub fn new(
        files: &'a dyn ProfileFileSystem,
        destination: &Path,
        log: Option<&'a dyn ProfileLog>,
    ) -> Self {
        Self {
            files,
            log: log.unwrap_or(&NULL_PROFILE_LOG),
            destination: destination.to_path_buf(),
        }
    }

    /// The destination folder.
    #[must_use]
    pub fn destination(&self) -> &Path {
        &self.destination
    }

    /// The file name of the profile named `asset_name`: what the file system forbids replaced
    /// by `_`, trailing spaces and dots dropped, `profil` for an empty result, then `.3dn`
    /// (AMD-030 §5).
    #[must_use]
    pub fn file_name_for(asset_name: &str) -> String {
        let replaced: String = asset_name
            .chars()
            .map(|c| if is_forbidden_in_file_name(c) { '_' } else { c })
            .collect();

        // Windows cannot create a name ending in a space or a dot at all; trimming them is the
        // same substitution rule carried to a character the list does not name.
        let trimmed = replaced.trim_end_matches([' ', '.']);
        let name = if trimmed.is_empty() {
            "profil"
        } else {
            trimmed
        };
        format!("{name}{}", UserProfileFolder::EXTENSION)
    }

    /// Finds the bundle under `installation_or_bundle_path`, reads it and imports its profiles.
    /// A bundle that cannot be read writes nothing (AMD-030 §4).
    ///
    /// # Errors
    ///
    /// No bundle is found, the bundle is unreadable, or the destination folder cannot be
    /// created.
    pub fn import_from(
        &self,
        installation_or_bundle_path: &Path,
    ) -> Result<ThreeDSenImportSummary, ThreeDSenImportError> {
        let bundle = locate_bundle(self.files, installation_or_bundle_path)?;
        let assets = read_text_assets(&self.files.read_all_bytes(&bundle)?)?;
        Ok(self.import(&bundle, &assets)?)
    }

    /// Imports `assets`, read from `bundle_path`.
    ///
    /// # Errors
    ///
    /// The destination folder cannot be created. A profile that cannot be written is a failed
    /// result, not an error.
    pub fn import(
        &self,
        bundle_path: &Path,
        assets: &[UnityTextAsset],
    ) -> io::Result<ThreeDSenImportSummary> {
        self.files.create_directory(&self.destination)?;

        let results = assets.iter().map(|asset| self.import_one(asset)).collect();
        let summary = ThreeDSenImportSummary::new(bundle_path, results);
        self.log
            .information(&format!("{}: {}", bundle_path.display(), summary.message()));
        Ok(summary)
    }

    fn import_one(&self, asset: &UnityTextAsset) -> ThreeDSenProfileResult {
        let failed = |message: String| {
            self.log.warning(&message);
            ThreeDSenProfileResult {
                name: asset.name.clone(),
                outcome: ThreeDSenProfileOutcome::Failed,
                file_name: String::new(),
                message,
            }
        };

        // AMD-030 §5: the payload is checked with the decoder of SPEC-03; a profile that does
        // not decode is a failure and the import carries on.
        if let Err(error) = format::load(&asset.script, None, Some(&asset.name)) {
            return failed(format!(
                "\"{}\" is not a usable profile: {}",
                asset.name, error.message
            ));
        }

        match self.write(
            &asset.name,
            &Self::file_name_for(&asset.name),
            &asset.script,
        ) {
            Ok(result) => result,
            Err(error) => failed(format!("\"{}\" could not be written: {error}", asset.name)),
        }
    }

    fn write(
        &self,
        asset_name: &str,
        file_name: &str,
        bytes: &[u8],
    ) -> io::Result<ThreeDSenProfileResult> {
        let result = |outcome, file_name: &str, message: String| ThreeDSenProfileResult {
            name: asset_name.to_owned(),
            outcome,
            file_name: file_name.to_owned(),
            message,
        };

        let mut candidate = file_name.to_owned();
        for attempt in 1..=Self::MAXIMUM_NAME_ATTEMPTS {
            let path = self.destination.join(&candidate);
            if !self.files.file_exists(&path) {
                self.write_whole(&path, bytes)?;
                return Ok(result(
                    ThreeDSenProfileOutcome::Imported,
                    &candidate,
                    String::new(),
                ));
            }

            if self.same_bytes(&path, bytes) {
                // AMD-030 §2: identical content is skipped, which is what makes a second import
                // of the same installation add nothing.
                return Ok(result(
                    ThreeDSenProfileOutcome::AlreadyPresent,
                    &candidate,
                    String::new(),
                ));
            }

            candidate = suffixed(file_name, attempt + 1);
        }

        let message = format!(
            "\"{asset_name}\" could not be imported: {} file names of the form \"{file_name}\" \
             are already taken by other profiles.",
            Self::MAXIMUM_NAME_ATTEMPTS
        );
        self.log.warning(&message);
        Ok(result(ThreeDSenProfileOutcome::Failed, "", message))
    }

    /// Writes through a temporary file and moves it into place without replacing anything, so
    /// that no partial file is ever left under a profile's name (AMD-030 §5).
    fn write_whole(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        let mut partial = path.as_os_str().to_owned();
        partial.push(Self::PARTIAL_SUFFIX);
        let partial = PathBuf::from(partial);

        let written = self
            .files
            .write_all_bytes(&partial, bytes)
            .and_then(|()| self.files.move_file(&partial, path, false));
        if written.is_err() {
            if let Err(error) = self.files.delete_file(&partial) {
                self.log.warning(&format!(
                    "{}: the temporary file could not be removed ({error}).",
                    partial.display()
                ));
            }
        }
        written
    }

    fn same_bytes(&self, path: &Path, bytes: &[u8]) -> bool {
        match self.files.read_all_bytes(path) {
            Ok(existing) => existing == bytes,
            Err(error) => {
                // An unreadable neighbour is treated as different, which sends the profile to
                // the next suffixed name rather than losing it.
                self.log.warning(&format!(
                    "{}: unreadable file, it is not compared ({error}).",
                    path.display()
                ));
                false
            }
        }
    }
}

/// `Name (n).3dn` for `Name.3dn`.
fn suffixed(file_name: &str, number: usize) -> String {
    let stem = file_name
        .strip_suffix(UserProfileFolder::EXTENSION)
        .unwrap_or(file_name);
    format!("{stem} ({number}){}", UserProfileFolder::EXTENSION)
}

/// The characters Windows forbids in a file name, which is the strictest list of the supported
/// platforms.
fn is_forbidden_in_file_name(c: char) -> bool {
    u32::from(c) < 32 || matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')
}
