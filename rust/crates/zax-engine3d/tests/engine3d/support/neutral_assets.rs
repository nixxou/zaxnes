//! The built-in profiles (`profiles/*.bytes`), located through the `ZAXNES_PROFILES`
//! environment variable. The tests that use them are `#[ignore]`d.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use zax_profiles::format;
use zax_profiles::profile::GameProfile;

/// The folder that holds the `.bytes` profiles.
pub fn profiles_directory() -> PathBuf {
    let root = env::var_os("ZAXNES_PROFILES").unwrap_or_else(|| {
        panic!("ZAXNES_PROFILES is not set: it must name the folder that holds profiles/")
    });
    PathBuf::from(root).join("profiles")
}

/// The built-in profile files, ordered by file name.
pub fn profile_files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(profiles_directory())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "bytes")
        })
        .collect();
    files.sort();
    files
}

/// Loads one built-in profile by file name.
pub fn load(file_name: &str) -> GameProfile {
    load_path(&profiles_directory().join(file_name))
}

/// Loads one built-in profile from its full path.
pub fn load_path(path: &Path) -> GameProfile {
    let name = path.display().to_string();
    format::load(&fs::read(path).unwrap(), None, Some(&name))
        .unwrap_or_else(|error| panic!("{name}: {error}"))
        .profile
}
