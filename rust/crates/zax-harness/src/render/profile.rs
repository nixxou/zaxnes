//! The profile `render --profile` names: a file, or a title of the catalogue
//! `--assets/profiles.json` (port of `RenderExport.LoadProfile` and `LocateAssets`).

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use zax_profiles::catalogue::{ProfileCatalogue, ProfileFileSystem, FULL_EDITION};
use zax_profiles::format;
use zax_profiles::profile::GameProfile;

use crate::HarnessError;

/// Name of the folder a build ships beside the program, searched upwards from the executable
/// when `--assets` is not given.
const ASSETS_FOLDER: &str = "assets-neutral";
const CATALOGUE_FILE: &str = "profiles.json";

/// The disk as the catalogue loader sees it. The harness only reads.
struct ReadOnlyFiles;

impl ReadOnlyFiles {
    fn refuse() -> io::Error {
        io::Error::new(io::ErrorKind::PermissionDenied, "the harness only reads")
    }
}

impl ProfileFileSystem for ReadOnlyFiles {
    fn file_exists(&self, path: &Path) -> bool {
        path.is_file()
    }

    fn directory_exists(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn create_directory(&self, _path: &Path) -> io::Result<()> {
        Err(Self::refuse())
    }

    fn list_files(&self, directory: &Path, extension: &str) -> io::Result<Vec<PathBuf>> {
        let wanted = extension.to_lowercase();
        let mut files = Vec::new();
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_file()
                && path.to_string_lossy().to_lowercase().ends_with(&wanted)
            {
                files.push(path);
            }
        }
        Ok(files)
    }

    fn list_directories(&self, directory: &Path) -> io::Result<Vec<PathBuf>> {
        let mut folders = Vec::new();
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                folders.push(entry.path());
            }
        }
        Ok(folders)
    }

    fn read_all_bytes(&self, path: &Path) -> io::Result<Vec<u8>> {
        fs::read(path)
    }

    fn read_all_text(&self, path: &Path) -> io::Result<String> {
        fs::read_to_string(path)
    }

    fn write_all_bytes(&self, _path: &Path, _contents: &[u8]) -> io::Result<()> {
        Err(Self::refuse())
    }

    fn write_all_text(&self, _path: &Path, _contents: &str) -> io::Result<()> {
        Err(Self::refuse())
    }

    fn delete_file(&self, _path: &Path) -> io::Result<()> {
        Err(Self::refuse())
    }

    fn copy_file(&self, _source: &Path, _destination: &Path, _overwrite: bool) -> io::Result<()> {
        Err(Self::refuse())
    }

    fn move_file(&self, _source: &Path, _destination: &Path, _overwrite: bool) -> io::Result<()> {
        Err(Self::refuse())
    }
}

/// Loads the profile called `name`: the empty profile when there is none, the file when `name`
/// is a path, otherwise the entry of the catalogue whose title is `name` (case ignored).
///
/// # Errors
///
/// [`HarnessError::Profile`] when the file or the entry is refused, the catalogue cannot be
/// found or read, or no entry has that title.
pub fn load(name: Option<&str>, assets: Option<&Path>) -> Result<GameProfile, HarnessError> {
    let Some(name) = name.filter(|name| !name.trim().is_empty()) else {
        return Ok(GameProfile::create_empty());
    };

    if Path::new(name).is_file() {
        return Ok(format::load(&fs::read(name)?, None, Some(name))?.profile);
    }

    let root = locate_assets(assets).ok_or_else(|| {
        HarnessError::Profile(
            "assets-neutral was not found; pass --assets <dir> or a profile file path.".to_owned(),
        )
    })?;
    let catalogue =
        ProfileCatalogue::load(&ReadOnlyFiles, &root.join(CATALOGUE_FILE), FULL_EDITION)
            .map_err(|error| HarnessError::Profile(error.to_string()))?;
    let entry = catalogue
        .entries()
        .iter()
        .find(|entry| entry.title.eq_ignore_ascii_case(name))
        .ok_or_else(|| HarnessError::Profile(format!("no built-in profile is titled '{name}'.")))?;
    // The built-in entries without a file ("2D Mode") are the empty profile.
    if entry.file.is_empty() {
        return Ok(GameProfile::create_empty());
    }

    Ok(catalogue.load_profile(&ReadOnlyFiles, entry, None)?.profile)
}

/// The folder `--assets` names, or, without the option, the first `assets-neutral` holding a
/// catalogue found going up from the executable.
fn locate_assets(hint: Option<&Path>) -> Option<PathBuf> {
    if let Some(hint) = hint.filter(|hint| !hint.as_os_str().is_empty()) {
        return hint.is_dir().then(|| hint.to_path_buf());
    }

    let executable = env::current_exe().ok()?;
    executable
        .ancestors()
        .skip(1)
        .map(|directory| directory.join(ASSETS_FOLDER))
        .find(|candidate| candidate.join(CATALOGUE_FILE).is_file())
}
