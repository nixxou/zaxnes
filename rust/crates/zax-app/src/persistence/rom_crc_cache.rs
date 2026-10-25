//! The CRC-32 of the data of the ROM files (Rust-only, owner request), which tells which games a
//! hack is made for. Kept in `rom_crc.json` of the user data with the size and the modification
//! time of each file, so that a file is read again only when it changed.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

/// The file of the cache, in the user data folder.
pub const FILE_NAME: &str = "rom_crc.json";

/// The version of what the CRC is taken over: 2 is the data within the sizes the header gives
/// (no trainer, no overdump). A cache of another version is read again from the files.
const VERSION: u32 = 2;

/// What tells a file changed: its size and its modification time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Stamp {
    size: u64,
    seconds: u64,
    nanos: u32,
}

impl Stamp {
    fn of(path: &Path) -> Option<Self> {
        let metadata = std::fs::metadata(path).ok()?;
        let modified = metadata.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
        Some(Self {
            size: metadata.len(),
            seconds: modified.as_secs(),
            nanos: modified.subsec_nanos(),
        })
    }
}

/// One file as the JSON holds it.
#[derive(Debug, Serialize, Deserialize)]
struct Record {
    path: String,
    size: u64,
    modified: u64,
    #[serde(default)]
    modified_nanos: u32,
    /// Eight hexadecimal digits.
    crc: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Body {
    #[serde(default)]
    version: u32,
    entries: Vec<Record>,
}

/// The CRC of each ROM file seen, read from its file on first use.
#[derive(Debug, Default)]
pub struct RomCrcCache {
    /// Where it is kept; none when nothing is persisted.
    file: Option<PathBuf>,
    entries: HashMap<String, (Stamp, u32)>,
    loaded: bool,
    dirty: bool,
}

impl RomCrcCache {
    /// A cache kept in `file`, read when first asked.
    #[must_use]
    pub fn new(file: Option<PathBuf>) -> Self {
        Self {
            file,
            ..Self::default()
        }
    }

    /// The CRC-32 of the data of the ROM in `path` (`builtin_patches::data_crc`): the one kept while
    /// the file has the same size and modification time, or read from the file. None when the
    /// file cannot be read.
    pub fn crc_of(&mut self, path: &Path) -> Option<u32> {
        self.load();
        let stamp = Stamp::of(path)?;
        let key = path.to_string_lossy().into_owned();
        if let Some((kept, crc)) = self.entries.get(&key) {
            if *kept == stamp {
                return Some(*crc);
            }
        }
        let image = std::fs::read(path).ok()?;
        let crc = zax_nes::builtin_patches::data_crc(&image);
        self.entries.insert(key, (stamp, crc));
        self.dirty = true;
        Some(crc)
    }

    /// Writes the cache when it learned something since it was read or last written.
    ///
    /// # Errors
    /// The file could not be written.
    pub fn save(&mut self) -> std::io::Result<()> {
        let Some(file) = self.file.as_ref().filter(|_| self.dirty) else {
            return Ok(());
        };
        let mut entries: Vec<Record> = self
            .entries
            .iter()
            .map(|(path, (stamp, crc))| Record {
                path: path.clone(),
                size: stamp.size,
                modified: stamp.seconds,
                modified_nanos: stamp.nanos,
                crc: format!("{crc:08X}"),
            })
            .collect();
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        let text = serde_json::to_string_pretty(&Body {
            version: VERSION,
            entries,
        })
        .map_err(std::io::Error::other)?;
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(file, text)?;
        self.dirty = false;
        Ok(())
    }

    fn load(&mut self) {
        if std::mem::replace(&mut self.loaded, true) {
            return;
        }
        let Some(body) = self
            .file
            .as_ref()
            .and_then(|file| std::fs::read_to_string(file).ok())
            .and_then(|text| serde_json::from_str::<Body>(&text).ok())
            .filter(|body| body.version == VERSION)
        else {
            return;
        };
        for record in body.entries {
            let Ok(crc) = u32::from_str_radix(&record.crc, 16) else {
                continue;
            };
            let stamp = Stamp {
                size: record.size,
                seconds: record.modified,
                nanos: record.modified_nanos,
            };
            self.entries.insert(record.path, (stamp, crc));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_is_read_once_and_again_when_it_changes() {
        let folder = std::env::temp_dir().join(format!("zaxnes-rom-crc-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let rom = folder.join("game.nes");
        std::fs::write(&rom, b"NES\x1a0123456789abDATA").unwrap();
        let file = folder.join(FILE_NAME);
        let mut cache = RomCrcCache::new(Some(file.clone()));
        let crc = cache.crc_of(&rom).unwrap();
        assert_eq!(zax_nes::builtin_patches::crc32(b"DATA"), crc);
        cache.save().unwrap();

        // A new cache gives the kept value; a changed file is read again.
        let mut again = RomCrcCache::new(Some(file));
        again.load();
        assert_eq!(1, again.entries.len());
        assert_eq!(Some(crc), again.crc_of(&rom));
        assert!(!again.dirty, "nothing new was learned");

        std::fs::write(&rom, b"NES\x1a0123456789abOTHER!").unwrap();
        let other = again.crc_of(&rom).unwrap();
        assert_eq!(zax_nes::builtin_patches::crc32(b"OTHER!"), other);
        assert!(again.dirty);
        assert_eq!(None, again.crc_of(&folder.join("missing.nes")));
        drop(std::fs::remove_dir_all(&folder));
    }
}
