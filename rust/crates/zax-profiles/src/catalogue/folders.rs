//! The two profile folders of AMD-033: the user's, and the optional one next to the executable.

use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};

use super::user_folder::{
    import_messages, same_path, sort_ordinal, ProfileOrigin, UserProfile, UserProfileFolder,
};
use super::ProfileFileSystem;
use crate::format::{ProfileLog, NULL_PROFILE_LOG};

/// The user folder and the program folder, scanned as one list (AMD-033 §2): the user's file
/// wins over a file of the same name, and over a profile of the same identifier, in the
/// program folder.
pub struct ProfileFolders<'a> {
    files: &'a dyn ProfileFileSystem,
    log: &'a dyn ProfileLog,
    user_directory: PathBuf,
    program_directory: Option<PathBuf>,
}

impl<'a> ProfileFolders<'a> {
    /// The folders; an absent, empty or identical `program_directory` leaves only the user
    /// folder.
    #[must_use]
    pub fn new(
        files: &'a dyn ProfileFileSystem,
        user_directory: &Path,
        program_directory: Option<&Path>,
        log: Option<&'a dyn ProfileLog>,
    ) -> Self {
        let program_directory = program_directory
            .filter(|directory| !directory.as_os_str().is_empty())
            .filter(|directory| !is_same_directory(user_directory, directory))
            .map(Path::to_path_buf);
        Self {
            files,
            log: log.unwrap_or(&NULL_PROFILE_LOG),
            user_directory: user_directory.to_path_buf(),
            program_directory,
        }
    }

    /// The user folder.
    #[must_use]
    pub fn user_directory(&self) -> &Path {
        &self.user_directory
    }

    /// The program folder, or none when there is none to scan.
    #[must_use]
    pub fn program_directory(&self) -> Option<&Path> {
        self.program_directory.as_deref()
    }

    /// Scans the user folder, then the program folder, and returns one list.
    ///
    /// # Errors
    ///
    /// The user folder exists but could not be listed. A program folder that cannot be listed
    /// is a warning, not an error.
    pub fn scan(&self, reserved_ids: &[&str]) -> io::Result<Vec<UserProfile>> {
        self.scan_excluding(reserved_ids, &[])
    }

    /// [`Self::scan`], where a file whose bytes are those of one of `built_in_contents` adds
    /// nothing (AMD-037).
    ///
    /// # Errors
    ///
    /// As [`Self::scan`].
    pub fn scan_excluding(
        &self,
        reserved_ids: &[&str],
        built_in_contents: &[Vec<u8>],
    ) -> io::Result<Vec<UserProfile>> {
        let user = UserProfileFolder::new(
            self.files,
            &self.user_directory,
            Some(self.log),
            ProfileOrigin::User,
        );
        let mut profiles = user.scan_excluding(reserved_ids, built_in_contents)?;

        let Some(program_directory) = &self.program_directory else {
            return Ok(profiles);
        };

        // The listing of the user folder, not the profiles it yielded: an unreadable or refused
        // file of the user folder still hides the one of the same name (AMD-033 §2). The scan is
        // flat, so the "relative path" of the rule is the file name.
        let user_names: Vec<String> = self
            .list_files(&self.user_directory)
            .iter()
            .map(|path| file_name(path).to_uppercase())
            .collect();

        let mut taken: HashSet<String> = reserved_ids.iter().map(|id| (*id).to_owned()).collect();
        let mut path_by_id: HashMap<String, PathBuf> = HashMap::new();
        for profile in &profiles {
            taken.insert(profile.id.clone());
            path_by_id.insert(profile.id.clone(), profile.file_path.clone());
        }

        let program = UserProfileFolder::new(
            self.files,
            program_directory,
            Some(self.log),
            ProfileOrigin::ProgramFolder,
        );
        let mut paths = self.list_files(program_directory);
        sort_ordinal(&mut paths);

        for path in paths {
            if user_names.contains(&file_name(&path).to_uppercase()) {
                // Same name: the user's file alone counts, and this one is not even read.
                continue;
            }
            let Some((profile, bytes)) = program.read(&path) else {
                continue;
            };
            if program.is_built_in_copy(&path, &bytes, built_in_contents) {
                continue;
            }
            if !taken.insert(profile.id.clone()) {
                // Byte-identical files are the same profile twice and appear once, with nothing
                // to report; two different files for one identifier are worth a line.
                if !self.have_same_content(&path, path_by_id.get(&profile.id)) {
                    self.log
                        .warning(&import_messages::duplicated_profile(&path, &profile.id));
                }
                continue;
            }
            path_by_id.insert(profile.id.clone(), path);
            profiles.push(profile);
        }
        Ok(profiles)
    }

    fn list_files(&self, directory: &Path) -> Vec<PathBuf> {
        match self
            .files
            .list_files(directory, UserProfileFolder::EXTENSION)
        {
            Ok(paths) => paths,
            Err(error) => {
                self.log.warning(&format!(
                    "{}: the profile folder could not be listed ({error})",
                    directory.display()
                ));
                Vec::new()
            }
        }
    }

    fn have_same_content(&self, left: &Path, right: Option<&PathBuf>) -> bool {
        let Some(right) = right else {
            // The identifier belongs to a built-in profile, whose file is not ours to compare.
            return false;
        };
        match (
            self.files.read_all_bytes(left),
            self.files.read_all_bytes(right),
        ) {
            (Ok(left), Ok(right)) => left == right,
            // An unreadable file is a different file; the duplicate is then reported.
            _ => false,
        }
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn is_same_directory(left: &Path, right: &Path) -> bool {
    let trim = |path: &Path| {
        path.to_string_lossy()
            .trim_end_matches(['/', '\\'])
            .to_owned()
    };
    same_path(Path::new(&trim(left)), Path::new(&trim(right)))
}
