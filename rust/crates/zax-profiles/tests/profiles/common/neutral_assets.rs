//! The extracted profile assets (`profiles/*.bytes` and `profiles.json`), located through the
//! `ZAXNES_PROFILES` environment variable. The tests that use them are `#[ignore]`d.

use std::env;
use std::fs;
use std::path::PathBuf;

use zax_profiles::format::{self, ProfileDocument};

/// The reason shown by the `#[ignore]` of every test that reads the built-in profiles.
pub const IGNORE_REASON: &str = "needs the built-in profiles: set ZAXNES_PROFILES to the folder that holds profiles/ and profiles.json";

pub fn root() -> PathBuf {
    let root = env::var_os("ZAXNES_PROFILES")
        .unwrap_or_else(|| panic!("ZAXNES_PROFILES is not set. {IGNORE_REASON}"));
    PathBuf::from(root)
}

pub fn profiles_directory() -> PathBuf {
    root().join("profiles")
}

pub fn catalogue_file() -> PathBuf {
    root().join("profiles.json")
}

/// The `.bytes` files of the profiles folder, in ordinal order.
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

/// The folder of the 101 "3dh" profiles of 3dSen 2.1 (AMD-062 §1).
pub fn hash_profiles_directory() -> PathBuf {
    root().join("profiles-2.1")
}

/// The `.bytes` files of the 2.1 folder, in ordinal order.
pub fn hash_profile_files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(hash_profiles_directory())
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

pub fn read_profile(file_name: &str) -> Vec<u8> {
    fs::read(profiles_directory().join(file_name)).unwrap()
}

pub fn read_hash_profile(file_name: &str) -> Vec<u8> {
    fs::read(hash_profiles_directory().join(file_name)).unwrap()
}

/// The name of the 1.x counterpart of a 2.1 file: the same name, except that two of them keep
/// the apostrophe the `profiles/` folder writes `_` (AMD-062 §1).
pub fn counterpart_name(file_name: &str) -> String {
    file_name.replace('\'', "_")
}

pub fn load_profile(file_name: &str) -> ProfileDocument {
    format::load(&read_profile(file_name), None, Some(file_name))
        .unwrap_or_else(|error| panic!("{file_name}: {error}"))
}
