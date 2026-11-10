//! The user profiles folder and the import of `.3dn` files into it (SPEC-03 REQ-PROF-062,
//! REQ-PROF-063).

use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

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

/// What importing a chosen file would do, read without writing anything (AMD-056 REQ-UI-214):
/// the dialogue the interface puts up is chosen from this, and only then is [`UserProfileFolder::apply`]
/// called.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileImportPlan {
    /// What applying the plan would do.
    pub outcome: ProfileImportOutcome,
    /// The file the user chose.
    pub source_path: PathBuf,
    /// The reason of a refusal, empty otherwise.
    pub message: String,
    /// The identifier the file carries, empty for a refusal.
    pub id: String,
    /// The title of the file: its `Title`, else its file name without extension.
    pub title: String,
    /// The identity fields read from the file.
    pub header: ProfileHeader,
    /// The user profile the identifier would take over, and whose displayed name the
    /// confirmation of REQ-UI-214 names.
    pub replaced: Option<UserProfile>,
    /// Where the file would be copied, empty when nothing would be.
    pub destination: PathBuf,
}

impl ProfileImportPlan {
    /// Whether [`UserProfileFolder::apply`] would write anything.
    #[must_use]
    pub fn writes(&self) -> bool {
        matches!(
            self.outcome,
            ProfileImportOutcome::Added | ProfileImportOutcome::Replaced
        )
    }
}

/// The texts the original shows when an import fails. SPEC-10 REQ-UI-084 is the only normative
/// source for them, and one oddity survives their rewording by AMD-056 §2: the two spaces before
/// "is" in the version message.
///
/// AMD-056 §1 reads two corrections out of the reference program 2.0 that AMD-053 had missed —
/// the version message carries the **sanitised** version and not the raw string of the file, and
/// the format message has a space after the path and spells « obsolete ». The « obsolate » of
/// the earlier reading was a misreading, not an oddity of the original.
pub mod import_messages {
    use std::path::Path;

    /// The file is not a `3dn` file.
    pub const NOT_A_3DN_FILE: &str = "This file is not a 3dn file";

    /// The format version is not supported. The version is sanitised as REQ-PROF-021 sanitises
    /// it — digits, `.` and `$` only — so a file writing `"v1"` reads « 3dn format version 1 »
    /// (AMD-056 §2, which supersedes the answer of Q-089 on the form of `<v>`). The sanitising
    /// happens here rather than at the call sites, so that every text carrying a version — the
    /// import of REQ-UI-212 and the load of REQ-PERS-040 — spells it the same way. The two
    /// spaces before "is" are the original's and stay.
    #[must_use]
    pub fn unsupported_version(version: &str) -> String {
        format!(
            "3dn format version {}  is not supported",
            crate::profile::version_rules::sanitize(Some(version))
        )
    }

    /// The file is unreadable or has no identifier. AMD-029 §3, closing Q-068: the last words
    /// name the product, « Zaxnes ». AMD-056 §2 adds the space after the path and the correct
    /// spelling of « obsolete ».
    #[must_use]
    pub fn obsolete_format(path: &Path) -> String {
        format!(
            "{} has an obsolete or wrong profile format. \
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
    /// File extension of a profile, and the one a file imported into the folder is written with.
    pub const EXTENSION: &'static str = ".3dn";

    /// The extension 3dSen 2.1 gives a "3dh" profile (AMD-062 REQ-PROF-062).
    pub const HASH_EXTENSION: &'static str = ".3dh";

    /// The extensions a profiles folder is scanned for (REQ-PROF-062 as amended by AMD-062).
    ///
    /// Which format a file holds is decided by its **content**, never by its name
    /// (REQ-PROF-070): a "3dh" profile named `.3dn` — which is what the import from a 3dSen
    /// installation writes — is read just the same.
    pub const EXTENSIONS: [&'static str; 2] = [Self::EXTENSION, Self::HASH_EXTENSION];

    /// The profile files of `directory`, both extensions, in the order the file system gives
    /// them.
    ///
    /// # Errors
    ///
    /// The folder exists but could not be listed.
    pub fn list_profile_files(
        files: &dyn ProfileFileSystem,
        directory: &Path,
    ) -> io::Result<Vec<PathBuf>> {
        let mut paths = Vec::new();
        for extension in Self::EXTENSIONS {
            paths.extend(files.list_files(directory, extension)?);
        }

        Ok(paths)
    }

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
    /// Both extensions are read (AMD-062 REQ-PROF-062), and a 3dh profile's `Id` is the one of
    /// its envelope, AMD-037 applying to it as to a 3dn's (REQ-PROF-073).
    ///
    /// # Errors
    ///
    /// The folder exists but could not be listed.
    pub fn scan(&self, reserved_ids: &[&str]) -> io::Result<Vec<UserProfile>> {
        self.scan_excluding(reserved_ids, &[])
    }

    /// [`Self::scan`], also skipping, with a single log line, the files whose bytes are those of
    /// one of `built_in_contents` (AMD-037): a copy of a built-in profile adds no game.
    ///
    /// # Errors
    ///
    /// The folder exists but could not be listed.
    pub fn scan_excluding(
        &self,
        reserved_ids: &[&str],
        built_in_contents: &[Vec<u8>],
    ) -> io::Result<Vec<UserProfile>> {
        let mut paths = Self::list_profile_files(self.files, &self.directory)?;
        sort_ordinal(&mut paths);

        let mut taken: HashSet<String> = reserved_ids.iter().map(|id| (*id).to_owned()).collect();
        let mut profiles = Vec::new();
        for path in paths {
            let Some((profile, bytes)) = self.read(&path) else {
                continue;
            };
            if self.is_built_in_copy(&path, &bytes, built_in_contents) {
                continue;
            }
            if !taken.insert(profile.id.clone()) {
                self.log
                    .warning(&import_messages::duplicated_profile(&path, &profile.id));
                continue;
            }
            profiles.push(profile);
        }
        Ok(profiles)
    }

    /// Imports `source_path` into this folder (REQ-PROF-063): the reading of [`Self::inspect`]
    /// followed at once by the writing of [`Self::apply`]. A file that is not an acceptable
    /// profile is a [`ProfileImportOutcome::Rejected`] result, not an error.
    ///
    /// This is the **non-interactive** import: it replaces an existing user profile without
    /// asking. The desktop interface no longer takes this path — AMD-056 REQ-UI-214 has it
    /// inspect, put up a dialogue, and apply only once the dialogue says so — but everything
    /// with nowhere to show a dialogue still does.
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
        let plan = self.inspect(source_path, built_in_ids, user_profiles)?;
        self.apply(&plan)
    }

    /// Reads `source_path` and says what importing it would do, **without writing anything**
    /// (AMD-056 REQ-UI-214).
    ///
    /// AMD-053 REQ-UI-212: the file is read as REQ-PROF-062 reads one — the header alone, no
    /// pattern and no script — and the message of a refusal is chosen by its **cause**. The four
    /// rows of the table are, in the order they can be told apart: an unreadable file or a
    /// malformed protobuf, a signature that is not `"3dn"`, an unsupported version, and a missing
    /// identifier.
    ///
    /// # Errors
    ///
    /// The source exists but could not be read.
    pub fn inspect(
        &self,
        source_path: &Path,
        built_in_ids: &[&str],
        user_profiles: &[UserProfile],
    ) -> io::Result<ProfileImportPlan> {
        if !self.files.file_exists(source_path) {
            return Ok(refusal(
                source_path,
                import_messages::obsolete_format(source_path),
            ));
        }

        let header = format::load_header(
            &self.files.read_all_bytes(source_path)?,
            Some(self.log),
            Some(&source_path.to_string_lossy()),
        );
        // Row 3 of the table: a malformed protobuf, or a file that could not be read at all.
        let Some(header) = header else {
            return Ok(refusal(
                source_path,
                import_messages::obsolete_format(source_path),
            ));
        };
        // Row 1: `Type` is present and is not "3dn" (REQ-PROF-033).
        if !header.has_acceptable_signature() {
            return Ok(refusal(
                source_path,
                import_messages::NOT_A_3DN_FILE.to_owned(),
            ));
        }
        // Row 2: `Ver` reduces to "1" (REQ-PROF-021).
        if header.has_unsupported_version() {
            // AMD-062 REQ-PROF-063: the causes of a refused "3dh" are those of REQ-PROF-072 —
            // here an envelope version outside "1", "2" and "5" — and the cause **shown** is the
            // third text of REQ-UI-084, as for a 3dn that cannot be read at all. The 3dn version
            // message would name a format version this file does not have.
            if header.is_hash_envelope() {
                return Ok(refusal(
                    source_path,
                    import_messages::obsolete_format(source_path),
                ));
            }

            let message =
                import_messages::unsupported_version(header.version.as_deref().unwrap_or_default());
            return Ok(refusal(source_path, message));
        }
        // Row 4: an absent or empty identifier is refused — there would be no way to key the
        // library entry, the slots or the cover art on it (REQ-PROF-063, REQ-PERS-041 point 1).
        if !header.has_id() {
            return Ok(refusal(
                source_path,
                import_messages::obsolete_format(source_path),
            ));
        }

        let id = header.id.clone().unwrap_or_default();
        if built_in_ids.contains(&id.as_str()) {
            return Ok(ProfileImportPlan {
                outcome: ProfileImportOutcome::AlreadyBuiltIn,
                source_path: source_path.to_path_buf(),
                message: String::new(),
                title: title_of(&header, source_path),
                id,
                header,
                replaced: None,
                destination: PathBuf::new(),
            });
        }

        let destination = self
            .directory
            .join(source_path.file_name().unwrap_or_default());
        let replaced = user_profiles
            .iter()
            .find(|profile| profile.id == id)
            .cloned();
        Ok(ProfileImportPlan {
            outcome: if replaced.is_none() {
                ProfileImportOutcome::Added
            } else {
                ProfileImportOutcome::Replaced
            },
            source_path: source_path.to_path_buf(),
            message: String::new(),
            title: title_of(&header, &destination),
            id,
            header,
            replaced,
            destination,
        })
    }

    /// Carries out what [`Self::inspect`] planned: the old file of a taken-over identifier goes,
    /// and the chosen file is copied into the folder. A plan that writes nothing — a refusal, or
    /// a built-in identifier — is only logged.
    ///
    /// REQ-PERS-041 keeps the fact logged in every case, which is why the two outcomes that do
    /// nothing are logged here rather than in [`Self::inspect`]: inspecting is what the interface
    /// does to choose a dialogue, and it may do it without importing anything.
    ///
    /// # Errors
    ///
    /// The old file could not be deleted or the new one copied.
    pub fn apply(&self, plan: &ProfileImportPlan) -> io::Result<ProfileImportResult> {
        match plan.outcome {
            ProfileImportOutcome::Rejected => {
                self.log.warning(&plan.message);
                return Ok(ProfileImportResult {
                    outcome: ProfileImportOutcome::Rejected,
                    profile: None,
                    message: plan.message.clone(),
                });
            }
            ProfileImportOutcome::AlreadyBuiltIn => {
                self.log.information(&format!(
                    "{}: the identifier \"{}\" belongs to a built-in profile; nothing imported",
                    plan.source_path.display(),
                    plan.id
                ));
                return Ok(ProfileImportResult {
                    outcome: ProfileImportOutcome::AlreadyBuiltIn,
                    profile: None,
                    message: String::new(),
                });
            }
            ProfileImportOutcome::Added | ProfileImportOutcome::Replaced => {}
        }

        if let Some(replaced) = &plan.replaced {
            if !same_path(&replaced.file_path, &plan.destination) {
                // Same identifier under another file name: the old file goes away.
                self.files.delete_file(&replaced.file_path)?;
            }
        }

        self.files.create_directory(&self.directory)?;
        if !same_path(&plan.source_path, &plan.destination) {
            self.files
                .copy_file(&plan.source_path, &plan.destination, true)?;
        }

        Ok(ProfileImportResult {
            outcome: plan.outcome,
            profile: Some(UserProfile {
                id: plan.id.clone(),
                title: plan.title.clone(),
                file_path: plan.destination.clone(),
                header: plan.header.clone(),
                origin: ProfileOrigin::User,
            }),
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
    /// unreadable or foreign. A profile with no identifier gets the one derived from its content
    /// (AMD-037); the file is never rewritten.
    #[must_use]
    pub fn try_read(&self, path: &Path) -> Option<UserProfile> {
        self.read(path).map(|(profile, _)| profile)
    }

    /// Like [`Self::try_read`], with the bytes of the file.
    pub(crate) fn read(&self, path: &Path) -> Option<(UserProfile, Vec<u8>)> {
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
        let id = if header.has_id() {
            header.id.clone().unwrap_or_default()
        } else {
            let id = derived_id(&bytes);
            self.log.information(&format!(
                "{}: the profile carries no identifier; using \"{id}\", derived from its content",
                path.display()
            ));
            id
        };

        let profile = UserProfile {
            id,
            title: title_of(&header, path),
            file_path: path.to_path_buf(),
            header,
            origin: self.origin,
        };
        Some((profile, bytes))
    }

    /// True, with a log line, when `bytes` are those of a built-in profile.
    pub(crate) fn is_built_in_copy(
        &self,
        path: &Path,
        bytes: &[u8],
        built_in_contents: &[Vec<u8>],
    ) -> bool {
        let copy = built_in_contents.iter().any(|content| content == bytes);
        if copy {
            self.log.information(&format!(
                "{}: identical to a built-in profile; nothing added",
                path.display()
            ));
        }
        copy
    }
}

/// A plan that writes nothing and carries the reason (AMD-053 REQ-UI-212). It is not logged here:
/// [`UserProfileFolder::apply`] logs it, so that an inspection the interface makes to choose a
/// dialogue does not log a refusal twice.
fn refusal(source_path: &Path, message: String) -> ProfileImportPlan {
    ProfileImportPlan {
        outcome: ProfileImportOutcome::Rejected,
        source_path: source_path.to_path_buf(),
        message,
        id: String::new(),
        title: String::new(),
        header: ProfileHeader::default(),
        replaced: None,
        destination: PathBuf::new(),
    }
}

/// The identifier of a profile that carries none (AMD-037): the first 9 bytes of the SHA-256 of
/// the file, in standard base64 without padding, 12 characters.
#[must_use]
pub fn derived_id(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let digest = Sha256::digest(bytes);
    let mut id = String::with_capacity(12);
    for group in digest[..9].chunks_exact(3) {
        let bits = (u32::from(group[0]) << 16) | (u32::from(group[1]) << 8) | u32::from(group[2]);
        for shift in [18, 12, 6, 0] {
            id.push(char::from(ALPHABET[((bits >> shift) & 63) as usize]));
        }
    }
    id
}

fn title_of(header: &ProfileHeader, path: &Path) -> String {
    match header.title.as_deref() {
        Some(title) if !title.is_empty() => shown_title(title, path),
        _ => path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default(),
    }
}

/// The titles of the profiles many others were made from, kept by mistake in those (Rust-only,
/// owner request).
const TEMPLATE_TITLES: [&str; 2] = ["Tetris", "Tennis"];

/// The title shown for a profile titled `title` in the file `path`: the file name without its
/// extension when the title is one of a template ("Tetris", "Tennis") the file name does not
/// say, else `title`.
#[must_use]
pub fn shown_title(title: &str, path: &Path) -> String {
    let stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let template = TEMPLATE_TITLES
        .iter()
        .any(|template| title.eq_ignore_ascii_case(template));
    let says_template = TEMPLATE_TITLES
        .iter()
        .any(|template| stem.contains(&template.to_lowercase()));
    match path.file_stem() {
        Some(file) if template && !says_template => file.to_string_lossy().into_owned(),
        _ => title.to_owned(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_template_title_the_file_name_does_not_say_gives_the_file_name() {
        let title = |title, file| shown_title(title, Path::new(file));
        assert_eq!(
            "Final_Fantasy_USA",
            title("Tetris", "p/Final_Fantasy_USA.3dn")
        );
        assert_eq!("TwinBee", title("tennis", "p/TwinBee.3dn"));
        assert_eq!("Tetris", title("Tetris", "p/Tetris-Nintendo.3dn"));
        assert_eq!("Tennis", title("Tennis", "p/Tennis (JU).3dn"));
        assert_eq!("Zelda II", title("Zelda II", "p/z2.3dn"));
    }
}
