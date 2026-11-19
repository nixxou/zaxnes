//! The hack pack files (`.zxpk`) the catalogue of [`crate::builtin_patches`] names by file name.
//!
//! They are in `patches/`, built into the binary (`build.rs`). A `zxpk` folder next to the
//! executable may hold more, or newer ones: both are read once, on first use, and a file in the
//! folder wins over the same name built in.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// The name of the packs folder next to the executable.
pub const FOLDER: &str = "zxpk";

/// The packs built into the binary: file name, bytes.
mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded_packs.rs"));
}

/// A folder set by the host before the packs are first read, in place of the one next to the
/// executable.
static FOLDER_OVERRIDE: Mutex<Option<PathBuf>> = Mutex::new(None);

/// The packs, read once.
static PACKS: OnceLock<Packs> = OnceLock::new();

/// What was found, for the log.
#[derive(Debug, Clone, Default)]
pub struct Report {
    /// Packs built into the binary.
    pub embedded: usize,
    /// The folder looked in, if one could be named.
    pub folder: Option<PathBuf>,
    /// Packs read from that folder.
    pub from_folder: usize,
}

struct Packs {
    bytes: HashMap<String, &'static [u8]>,
    report: Report,
}

/// Sets the packs folder, in place of `zxpk` next to the executable. It has no effect once the
/// packs have been read.
pub fn set_folder(folder: impl Into<PathBuf>) {
    if let Ok(mut slot) = FOLDER_OVERRIDE.lock() {
        *slot = Some(folder.into());
    }
}

/// The folder the packs are read from: the one set by [`set_folder`], else `ZAXNES_PACKS_DIR`,
/// else `zxpk` next to the executable.
fn folder() -> Option<PathBuf> {
    if let Some(folder) = FOLDER_OVERRIDE.lock().ok().and_then(|slot| slot.clone()) {
        return Some(folder);
    }
    if let Some(folder) = std::env::var_os("ZAXNES_PACKS_DIR").filter(|value| !value.is_empty()) {
        return Some(PathBuf::from(folder));
    }
    let executable = std::env::current_exe().ok()?;
    Some(executable.parent()?.join(FOLDER))
}

fn read_folder(folder: &Path, bytes: &mut HashMap<String, &'static [u8]>) -> usize {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return 0;
    };
    let mut count = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        let is_pack = path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("zxpk"));
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !is_pack || !path.is_file() {
            continue;
        }
        if let Ok(data) = std::fs::read(&path) {
            bytes.insert(name.to_owned(), Box::leak(data.into_boxed_slice()));
            count += 1;
        }
    }
    count
}

fn packs() -> &'static Packs {
    PACKS.get_or_init(|| {
        let mut bytes: HashMap<String, &'static [u8]> = embedded::PACKS
            .iter()
            .map(|(name, data)| ((*name).to_owned(), *data))
            .collect();
        let folder = folder();
        let from_folder = folder
            .as_deref()
            .map_or(0, |folder| read_folder(folder, &mut bytes));
        Packs {
            bytes,
            report: Report {
                embedded: embedded::PACKS.len(),
                folder,
                from_folder,
            },
        }
    })
}

/// The bytes of the pack file `name`, if it was built in or found in the folder.
#[must_use]
pub fn get(name: &str) -> Option<&'static [u8]> {
    packs().bytes.get(name).copied()
}

/// What was found (the packs are read if they were not yet).
#[must_use]
pub fn report() -> Report {
    packs().report.clone()
}
