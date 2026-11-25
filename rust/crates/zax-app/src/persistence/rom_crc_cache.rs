//! The hashes of the ROM files (Rust-only, owner request), taken once per file: the CRC-32 of the
//! data, which tells which games a hack is made for and is the key of the game databases, the MD5
//! of the same bytes (RetroAchievements' hash of a NES game) and the CRC-32 of the whole file (what
//! a `.7z` or `.zip` lists for it). Kept in `rom_crc.json` of the user data with the size and the
//! modification time of each file, so that a file is read again only when it changed. A game in an
//! archive is hashed as the file the archive holds.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

/// The file of the cache, in the user data folder.
pub const FILE_NAME: &str = "rom_crc.json";

/// The version of what is kept: 2 is the data CRC within the sizes the header gives (no
/// trainer, no overdump), 3 adds the MD5 of the same data and the CRC of the whole file, and reads
/// a game in an archive. A cache of another version is read again from the files.
const VERSION: u32 = 3;

/// The hashes of one ROM image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RomHashes {
    /// `builtin_patches::data_crc`: PRG and CHR at the header's sizes.
    pub data_crc: u32,
    /// The MD5 of the same bytes (`builtin_patches::data_md5`).
    pub data_md5: [u8; 16],
    /// The CRC-32 of the whole image, header included.
    pub file_crc32: u32,
}

impl RomHashes {
    /// The hashes of `image`, in one reading of it.
    #[must_use]
    pub fn of(image: &[u8]) -> Self {
        Self {
            data_crc: zax_nes::builtin_patches::data_crc(image),
            data_md5: zax_nes::builtin_patches::data_md5(image),
            file_crc32: zax_nes::builtin_patches::crc32(image),
        }
    }

    /// The MD5 in lowercase hexadecimal, as RetroAchievements writes it.
    #[must_use]
    pub fn data_md5_hex(&self) -> String {
        use std::fmt::Write as _;
        self.data_md5.iter().fold(String::new(), |mut text, byte| {
            let _ = write!(text, "{byte:02x}");
            text
        })
    }
}

/// What tells a file changed: its size and its modification time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Stamp {
    pub(crate) size: u64,
    pub(crate) seconds: u64,
    pub(crate) nanos: u32,
}

impl Stamp {
    pub(crate) fn of(path: &Path) -> Option<Self> {
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
    /// 32 hexadecimal digits.
    md5: String,
    /// Eight hexadecimal digits.
    file_crc: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Body {
    #[serde(default)]
    version: u32,
    entries: Vec<Record>,
}

/// The hashes of each ROM file seen, read from its file on first use.
#[derive(Debug, Default)]
pub struct RomCrcCache {
    /// Where it is kept; none when nothing is persisted.
    file: Option<PathBuf>,
    entries: HashMap<String, (Stamp, RomHashes)>,
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
        self.hashes_of(path).map(|hashes| hashes.data_crc)
    }

    /// The hashes of the ROM in `path`, bare or in an archive: the ones kept while the file has the
    /// same size and modification time, or read from the file. None when it cannot be read.
    pub fn hashes_of(&mut self, path: &Path) -> Option<RomHashes> {
        if let Some(hashes) = self.known_hashes(path) {
            return Some(hashes);
        }
        let stamp = Stamp::of(path)?;
        let rom = crate::emulation::rom_file::read(path).ok()?;
        let hashes = RomHashes::of(&rom.image);
        self.insert(path, stamp, hashes);
        Some(hashes)
    }

    /// The data CRC kept for the ROM in `path` while the file is as it was then, without reading
    /// the file: none when it was not seen as it is.
    pub fn known(&mut self, path: &Path) -> Option<u32> {
        self.known_hashes(path).map(|hashes| hashes.data_crc)
    }

    /// The hashes kept for the ROM in `path` while the file is as it was then, without reading the
    /// file: none when it was not seen as it is.
    pub fn known_hashes(&mut self, path: &Path) -> Option<RomHashes> {
        self.load();
        let stamp = Stamp::of(path)?;
        let (kept, hashes) = self.entries.get(&*path.to_string_lossy())?;
        (*kept == stamp).then_some(*hashes)
    }

    /// The hashes of the ROM in `path`, whose image is `image` (read elsewhere): the ones kept
    /// while the file is as it was, else taken from `image` and kept.
    pub fn hashes_for(&mut self, path: &Path, image: &[u8]) -> RomHashes {
        if let Some(hashes) = self.known_hashes(path) {
            return hashes;
        }
        let hashes = RomHashes::of(image);
        if let Some(stamp) = Stamp::of(path) {
            self.insert(path, stamp, hashes);
        }
        hashes
    }

    /// Keeps `hashes`, read from the file in `path` when it had `stamp` (a file read on another
    /// thread, `rom_hasher`).
    pub(crate) fn store(&mut self, path: &Path, stamp: Stamp, hashes: RomHashes) {
        self.load();
        self.insert(path, stamp, hashes);
    }

    fn insert(&mut self, path: &Path, stamp: Stamp, hashes: RomHashes) {
        let key = path.to_string_lossy().into_owned();
        if self.entries.get(&key) != Some(&(stamp, hashes)) {
            self.entries.insert(key, (stamp, hashes));
            self.dirty = true;
        }
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
            .map(|(path, (stamp, hashes))| Record {
                path: path.clone(),
                size: stamp.size,
                modified: stamp.seconds,
                modified_nanos: stamp.nanos,
                crc: format!("{:08X}", hashes.data_crc),
                md5: hashes.data_md5_hex(),
                file_crc: format!("{:08X}", hashes.file_crc32),
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
            let Some(hashes) = parse_hashes(&record) else {
                continue;
            };
            let stamp = Stamp {
                size: record.size,
                seconds: record.modified,
                nanos: record.modified_nanos,
            };
            self.entries.insert(record.path, (stamp, hashes));
        }
    }
}

/// The hashes of `record`; none for a malformed one.
fn parse_hashes(record: &Record) -> Option<RomHashes> {
    let mut data_md5 = [0u8; 16];
    if record.md5.len() != 32 {
        return None;
    }
    for (n, byte) in data_md5.iter_mut().enumerate() {
        *byte = u8::from_str_radix(record.md5.get(n * 2..n * 2 + 2)?, 16).ok()?;
    }
    Some(RomHashes {
        data_crc: u32::from_str_radix(&record.crc, 16).ok()?,
        data_md5,
        file_crc32: u32::from_str_radix(&record.file_crc, 16).ok()?,
    })
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
        let hashes = cache.known_hashes(&rom).unwrap();
        assert_eq!(zax_nes::movie::fm2::md5(b"DATA"), hashes.data_md5);
        assert_eq!(
            zax_nes::builtin_patches::crc32(b"NES\x1a0123456789abDATA"),
            hashes.file_crc32
        );
        cache.save().unwrap();

        // A new cache gives the kept value; a changed file is read again.
        let mut again = RomCrcCache::new(Some(file));
        again.load();
        assert_eq!(1, again.entries.len());
        assert_eq!(Some(crc), again.crc_of(&rom));
        assert_eq!(Some(hashes), again.known_hashes(&rom));
        assert!(!again.dirty, "nothing new was learned");

        std::fs::write(&rom, b"NES\x1a0123456789abOTHER!").unwrap();
        let other = again.crc_of(&rom).unwrap();
        assert_eq!(zax_nes::builtin_patches::crc32(b"OTHER!"), other);
        assert!(again.dirty);
        assert_eq!(None, again.crc_of(&folder.join("missing.nes")));
        drop(std::fs::remove_dir_all(&folder));
    }
}
