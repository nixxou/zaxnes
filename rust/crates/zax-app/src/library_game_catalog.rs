//! The game catalogue over the profile library (SPEC-09 §3): the built-in entries, the profiles
//! of the user's folder and of the program's folder, and what `library.json` remembers.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use time::OffsetDateTime;
use zax_profiles::catalogue::{
    built_in, import_messages, ProfileFolders, ProfileImportOutcome,
    ProfileImportResult, ProfileOrigin, UserProfile, UserProfileFolder,
};
use zax_profiles::format::{self, ProfileLog};
use zax_profiles::import::{ThreeDSenImportError, ThreeDSenImportSummary, ThreeDSenImporter};
use zax_profiles::library::{GameLibrary, GameLibraryEntry, GameLibraryStore, GameProfileSource};
use zax_profiles::profile::GameProfile;

use crate::emulation::game_catalog::{GameCatalog, ResolvedGame};
use crate::neutral_assets;
use crate::persistence::game_art;
use crate::physical_file_system::PhysicalProfileFileSystem;
use crate::user_data_paths::UserDataPaths;

/// The text sink the catalogue logs to.
pub type LogSink = Box<dyn Fn(&str)>;

/// What a 3dSen import reports: the summary when it ran, and the message to show.
#[derive(Debug, Clone)]
pub struct ThreeDSenImportReport {
    /// The summary of the profiles written, when the import ran.
    pub summary: Option<ThreeDSenImportSummary>,
    /// The message to show the user.
    pub message: String,
}

struct ProfileLogAdapter<'a>(&'a dyn Fn(&str));

impl ProfileLog for ProfileLogAdapter<'_> {
    fn information(&self, message: &str) {
        (self.0)(message);
    }

    fn warning(&self, message: &str) {
        (self.0)(message);
    }

    fn error(&self, message: &str) {
        (self.0)(message);
    }
}

/// The library, its file, and the profiles read so far.
pub struct LibraryGameCatalog {
    library: GameLibrary,
    paths: UserDataPaths,
    log: LogSink,
    profiles: HashMap<String, Arc<GameProfile>>,
}

impl LibraryGameCatalog {
    /// Builds the library from the asset folder's catalogue (or the built-in one), the profile
    /// folders and `library.json`.
    pub fn open(asset_root: Option<&Path>, paths: &UserDataPaths, log: LogSink) -> Self {
        let catalogue = asset_root
            .and_then(|root| match neutral_assets::load_catalogue(root) {
                Ok(catalogue) => Some(catalogue),
                Err(error) => {
                    log(&format!("the profile catalogue could not be read: {error}"));
                    None
                }
            })
            // AMD-024 REQ-PUB-001 (Q-065): with no asset folder the catalogue is the one written
            // in code, "2D Mode" and "3D by default (no profile)", neither of which has a file.
            .unwrap_or_else(built_in::create);
        let adapter = ProfileLogAdapter(&*log);
        let files = PhysicalProfileFileSystem;
        let persisted = GameLibraryStore::new(&files, Some(&adapter))
            .load(&paths.library_file, OffsetDateTime::now_utc());
        // AMD-033 §1: the user folder and the read-only "profiles" folder next to the
        // executable, in every mode.
        let user_profiles = match profile_folders(&files, paths, &adapter).scan(&catalogue.ids()) {
            Ok(profiles) => profiles,
            Err(error) => {
                log(&format!("the user profile folder could not be read: {error}"));
                Vec::new()
            }
        };
        let library = GameLibrary::build(&catalogue, &user_profiles, persisted.as_ref(), Some(&adapter));
        Self::from_library(library, paths.clone(), log)
    }

    /// A catalogue over an already built library.
    #[must_use]
    pub fn from_library(library: GameLibrary, paths: UserDataPaths, log: LogSink) -> Self {
        Self {
            library,
            paths,
            log,
            profiles: HashMap::new(),
        }
    }

    /// The library.
    #[must_use]
    pub const fn library(&self) -> &GameLibrary {
        &self.library
    }

    /// The library, for editing.
    pub fn library_mut(&mut self) -> &mut GameLibrary {
        &mut self.library
    }

    /// Imports the `.3dn` file at `path` into the user folder and the library.
    pub fn import_profile(&mut self, path: &Path) -> ProfileImportResult {
        // Only the profiles of the user folder: the import replaces the file of the entry it
        // takes over, and nothing next to the executable is ever deleted (AMD-033 §1). A profile
        // of the program folder simply loses its entry to the imported one.
        let known: Vec<UserProfile> = self
            .library
            .entries()
            .iter()
            .filter(|entry| {
                entry.source() == GameProfileSource::User && entry.origin() == ProfileOrigin::User
            })
            .map(user_profile_of)
            .collect();
        let built_in: Vec<&str> = self
            .library
            .entries()
            .iter()
            .filter(|entry| entry.source() == GameProfileSource::BuiltIn)
            .map(GameLibraryEntry::id)
            .collect();
        let files = PhysicalProfileFileSystem;
        let adapter = ProfileLogAdapter(&*self.log);
        let folder = UserProfileFolder::new(
            &files,
            &self.paths.profiles_directory,
            Some(&adapter),
            ProfileOrigin::User,
        );
        let result = match folder.import(path, &built_in, &known) {
            Ok(result) => result,
            Err(_) => ProfileImportResult {
                outcome: ProfileImportOutcome::Rejected,
                profile: None,
                message: import_messages::obsolete_format(path),
            },
        };
        if let (ProfileImportOutcome::Added | ProfileImportOutcome::Replaced, Some(imported)) =
            (result.outcome, &result.profile)
        {
            self.library.add_user_profile(imported, Some(&adapter));
            // A replaced profile must be read again rather than served from the cache.
            self.profiles.remove(&imported.id);
            self.save();
        }
        result
    }

    /// Imports the profiles of a 3dSen installation or bundle into the user folder and the
    /// library.
    pub fn import_from_three_d_sen(&mut self, installation_or_bundle_path: &Path) -> ThreeDSenImportReport {
        if !self.paths.is_writable {
            // SPEC-09 REQ-PERS-073: with no writable root there is nowhere to put the profiles.
            return ThreeDSenImportReport {
                summary: None,
                message: self
                    .paths
                    .unwritable_message
                    .clone()
                    .unwrap_or_else(|| "the user folder does not accept writes.".to_owned()),
            };
        }
        let files = PhysicalProfileFileSystem;
        let importer = ThreeDSenImporter::new(&files, &self.paths.profiles_directory, None);
        let summary = match importer.import_from(installation_or_bundle_path) {
            Ok(summary) => summary,
            Err(error) => {
                let message = match &error {
                    ThreeDSenImportError::Format(format) => format.message().to_owned(),
                    ThreeDSenImportError::Io(io) => io.to_string(),
                };
                (self.log)(&format!("the 3dSen import failed: {message}"));
                return ThreeDSenImportReport {
                    summary: None,
                    message,
                };
            }
        };
        self.add_imported_profiles();
        let message = summary.message();
        ThreeDSenImportReport {
            summary: Some(summary),
            message,
        }
    }

    fn add_imported_profiles(&mut self) {
        let files = PhysicalProfileFileSystem;
        let adapter = ProfileLogAdapter(&*self.log);
        let profiles = match profile_folders(&files, &self.paths, &adapter).scan(&[]) {
            Ok(profiles) => profiles,
            Err(error) => {
                (self.log)(&format!("the user profile folder could not be read: {error}"));
                return;
            }
        };
        let mut added = false;
        for profile in &profiles {
            if self.library.add_user_profile(profile, Some(&adapter)).is_some() {
                self.profiles.remove(&profile.id);
                added = true;
            }
        }
        if added {
            self.save();
        }
    }

    /// Writes `library.json`, unless the root is read-only.
    pub fn save(&self) {
        if !self.paths.is_writable {
            return;
        }
        let files = PhysicalProfileFileSystem;
        let adapter = ProfileLogAdapter(&*self.log);
        let mut file = self.library.to_file();
        if let Err(error) = GameLibraryStore::new(&files, Some(&adapter)).save(&self.paths.library_file, &mut file) {
            (self.log)(&format!("library.json could not be written: {error}"));
        }
    }

    fn load_profile(&self, entry: &GameLibraryEntry) -> GameProfile {
        let path = entry.profile_path();
        if path.as_os_str().is_empty() || !path.is_file() {
            return GameProfile::create_empty();
        }
        match std::fs::read(path) {
            Ok(bytes) => match format::load(&bytes, None, Some(&path.to_string_lossy())) {
                Ok(document) => return document.profile,
                // REQ-PROF-033: a refused profile falls back to the empty profile rather than
                // stopping the launch.
                Err(error) => (self.log)(&format!(
                    "{}: {}; falling back to the empty profile.",
                    entry.display_title(),
                    error.message
                )),
            },
            Err(error) => (self.log)(&format!("{}: {error}", entry.display_title())),
        }
        GameProfile::create_empty()
    }
}

fn profile_folders<'a>(
    files: &'a PhysicalProfileFileSystem,
    paths: &UserDataPaths,
    log: &'a dyn ProfileLog,
) -> ProfileFolders<'a> {
    ProfileFolders::new(
        files,
        &paths.profiles_directory,
        Some(&paths.program_profiles_directory),
        Some(log),
    )
}

fn user_profile_of(entry: &GameLibraryEntry) -> UserProfile {
    UserProfile {
        id: entry.id().to_owned(),
        title: entry.title.clone(),
        file_path: entry.profile_path().to_path_buf(),
        header: format::ProfileHeader::default(),
        origin: ProfileOrigin::User,
    }
}

impl GameCatalog for LibraryGameCatalog {
    fn resolve(&mut self, game_id: &str) -> Option<ResolvedGame> {
        let entry = self.library.find(game_id)?;
        let profile = if let Some(profile) = self.profiles.get(game_id) {
            Arc::clone(profile)
        } else {
            let profile = Arc::new(self.load_profile(entry));
            self.profiles.insert(game_id.to_owned(), Arc::clone(&profile));
            profile
        };
        Some(ResolvedGame {
            id: entry.id().to_owned(),
            title: entry.display_title().to_owned(),
            profile,
            is_2d_mode: entry.is_2d_mode(),
            is_default_three_dimensional: entry.is_default_three_dimensional(),
        })
    }

    fn exists(&self, game_id: &str) -> bool {
        self.library.find(game_id).is_some()
    }

    fn rom_path(&self, game_id: &str) -> Option<String> {
        self.library
            .find(game_id)
            .filter(|entry| entry.has_rom())
            .map(|entry| entry.rom_path.clone())
    }

    fn game_for_rom(&self, rom_path: &str) -> Option<String> {
        let wanted = full_path(rom_path);
        self.library
            .entries()
            .iter()
            .find(|entry| entry.has_rom() && full_path(&entry.rom_path) == wanted)
            .map(|entry| entry.id().to_owned())
    }

    fn two_dimensional_mode_id(&self) -> Option<String> {
        self.library
            .entries()
            .iter()
            .find(|entry| entry.is_2d_mode())
            .map(|entry| entry.id().to_owned())
    }

    fn default_three_dimensional_id(&self) -> Option<String> {
        self.library
            .entries()
            .iter()
            .find(|entry| entry.is_default_three_dimensional())
            .map(|entry| entry.id().to_owned())
    }

    fn set_rom_path(&mut self, game_id: &str, rom_path: &str) {
        if self.library.set_rom_path(game_id, rom_path) {
            self.save();
        }
    }

    fn mark_played(&mut self, game_id: &str, unix_seconds: i64) {
        if self.library.mark_played(game_id, unix_seconds) {
            self.save();
        }
    }

    fn delete_game(&mut self, game_id: &str) -> bool {
        // A built-in profile and the "2D Mode" entry must not be deletable (REQ-UI-037), and
        // neither is a profile of the folder next to the executable: deleting a game deletes its
        // file, and that folder is read-only for Zaxnes (AMD-033 §1, Q-074).
        let Some(entry) = self.library.find(game_id).filter(|entry| entry.can_delete()) else {
            return false;
        };
        let title = entry.title.clone();
        let cover = entry.cover.clone();
        let path = entry.profile_path().to_path_buf();
        let id = entry.id().to_owned();
        if !path.as_os_str().is_empty() {
            let files = PhysicalProfileFileSystem;
            let folder = UserProfileFolder::new(&files, &self.paths.profiles_directory, None, ProfileOrigin::User);
            let profile = UserProfile {
                id: id.clone(),
                title: title.clone(),
                file_path: path,
                header: format::ProfileHeader::default(),
                origin: ProfileOrigin::User,
            };
            if let Err(error) = folder.delete(&profile) {
                (self.log)(&format!("the profile file of '{game_id}' could not be deleted: {error}"));
            }
        }
        game_art::delete_cover(&self.paths, &title, &id, cover.as_deref());
        if !self.library.remove(game_id) {
            return false;
        }
        self.profiles.remove(game_id);
        self.save();
        (self.log)(&format!("game '{game_id}' deleted with its profile file and its cover."));
        true
    }

    fn title(&self, game_id: &str) -> Option<String> {
        self.library.find(game_id).map(|entry| entry.display_title().to_owned())
    }

    fn can_delete(&self, game_id: &str) -> bool {
        self.library.find(game_id).is_some_and(GameLibraryEntry::can_delete)
    }

    fn set_cover(&mut self, game_id: &str, file_name: Option<&str>) -> bool {
        let Some(entry) = self.library.find_mut(game_id) else {
            return false;
        };
        entry.cover = file_name.map(str::to_owned);
        self.save();
        true
    }

    fn cover(&self, game_id: &str) -> Option<String> {
        self.library.find(game_id).and_then(|entry| entry.cover.clone())
    }
}

fn full_path(path: &str) -> String {
    std::path::absolute(path)
        .unwrap_or_else(|_| path.into())
        .to_string_lossy()
        .to_lowercase()
}
