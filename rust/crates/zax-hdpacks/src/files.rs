//! Where a pack's files come from: a folder on disk, or files already read from an archive.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The files of a pack, by their path relative to the folder of its `hires.txt`.
pub trait PackFiles {
    /// The bytes of `name` (forward slashes), `None` when there is no such file.
    fn read(&self, name: &str) -> Option<Vec<u8>>;
}

/// A pack in a folder.
#[derive(Debug, Clone)]
pub struct FolderFiles(pub PathBuf);

impl PackFiles for FolderFiles {
    fn read(&self, name: &str) -> Option<Vec<u8>> {
        std::fs::read(self.0.join(name)).ok()
    }
}

/// A pack read from an archive: its files by path, the folder of its `hires.txt` taken as root.
#[derive(Debug, Clone, Default)]
pub struct MemoryFiles {
    files: HashMap<String, Vec<u8>>,
}

/// A path compared without regard to case or slash direction (packs are made on Windows).
fn key(name: &str) -> String {
    name.replace('\\', "/")
        .trim_start_matches("./")
        .to_lowercase()
}

impl MemoryFiles {
    /// The pack among `entries` (archive paths and bytes): the files under the folder of the first
    /// `hires.txt`, the shallowest one. `None` when there is no `hires.txt`.
    #[must_use]
    pub fn from_archive(entries: Vec<(String, Vec<u8>)>) -> Option<Self> {
        let root = entries
            .iter()
            .map(|(name, _)| key(name))
            .filter(|name| name == "hires.txt" || name.ends_with("/hires.txt"))
            .min_by_key(|name| (name.matches('/').count(), name.clone()))?;
        let prefix = root.trim_end_matches("hires.txt").to_owned();
        let files = entries
            .into_iter()
            .filter_map(|(name, bytes)| {
                let name = key(&name);
                name.strip_prefix(&prefix)
                    .map(|rest| (rest.to_owned(), bytes))
            })
            .collect();
        Some(Self { files })
    }

    /// True when `name` is the name of a pack's text file in an archive listing.
    #[must_use]
    pub fn is_pack_text(name: &str) -> bool {
        let name = key(name);
        name == "hires.txt" || name.ends_with("/hires.txt")
    }
}

impl PackFiles for MemoryFiles {
    fn read(&self, name: &str) -> Option<Vec<u8>> {
        self.files.get(&key(name)).cloned()
    }
}

/// True when `folder` holds a pack.
#[must_use]
pub fn is_pack_folder(folder: &Path) -> bool {
    folder.join("hires.txt").is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_archive_pack_is_rooted_at_its_hires_txt() {
        let files = MemoryFiles::from_archive(vec![
            ("Game.nes".into(), b"rom".to_vec()),
            ("pack\\HIRES.txt".into(), b"<ver>105".to_vec()),
            ("pack/tiles.png".into(), b"png".to_vec()),
            ("deeper/x/hires.txt".into(), b"other".to_vec()),
        ])
        .unwrap_or_default();
        assert_eq!(Some(b"<ver>105".to_vec()), files.read("hires.txt"));
        assert_eq!(Some(b"png".to_vec()), files.read("Tiles.png"));
        assert_eq!(None, files.read("Game.nes"));
        assert!(MemoryFiles::from_archive(vec![("a.png".into(), vec![])]).is_none());
    }
}
