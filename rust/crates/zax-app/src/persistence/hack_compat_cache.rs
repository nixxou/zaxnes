//! What the study of each ROM file found (Rust-only, owner request): whether a widescreen hack
//! fits it, by its CRC or by the signatures of a hack pack. A ROM is studied when its hacks
//! screen opens, when it starts, or when the console's list rests a second on it, never while
//! the ROMs are listed (that would read every file); the list shows what was found. Kept in `hack_compat.json` of the user data with the size and
//! the modification time of each file and a fingerprint of the built-in hacks, so that a changed
//! file or a new version of the hacks makes the ROM studied again.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use super::rom_crc_cache::Stamp;

/// The file of the cache, in the user data folder.
pub const FILE_NAME: &str = "hack_compat.json";

/// What the study of a ROM found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Certificate {
    /// A widescreen hack fits it: the built-in patch's name (none for a game the decoder alone
    /// knows), and whether every part of a hack pack applies.
    Fits { patch: Option<String>, whole: bool },
    /// No widescreen hack fits it.
    Unfit,
}

/// One file as the JSON holds it.
#[derive(Debug, Serialize, Deserialize)]
struct Record {
    path: String,
    size: u64,
    modified: u64,
    #[serde(default)]
    modified_nanos: u32,
    /// Whether a widescreen hack fits the ROM.
    fits: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    patch: Option<String>,
    #[serde(default)]
    whole: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Body {
    /// The fingerprint of the built-in hacks the ROMs were studied with, eight hexadecimal
    /// digits.
    #[serde(default)]
    hacks: String,
    entries: Vec<Record>,
}

/// The CRC-32 of every built-in patch (name, ROM CRC, bytes) in turn: another value means the
/// hacks changed and every ROM is studied again.
fn fingerprint() -> &'static str {
    static FINGERPRINT: OnceLock<String> = OnceLock::new();
    FINGERPRINT.get_or_init(|| {
        let mut bytes = Vec::new();
        for patch in &zax_nes::builtin_patches::PATCHES {
            bytes.extend_from_slice(patch.name.as_bytes());
            bytes.extend_from_slice(&patch.source_crc.to_le_bytes());
            bytes.extend_from_slice(patch.bps);
        }
        format!("{:08X}", zax_nes::builtin_patches::crc32(&bytes))
    })
}

/// What was found of each ROM studied.
#[derive(Debug, Default)]
pub struct HackCompatCache {
    /// Where it is kept; none when nothing is persisted.
    file: Option<PathBuf>,
    entries: HashMap<String, (Stamp, Certificate)>,
    loaded: bool,
    dirty: bool,
}

impl HackCompatCache {
    /// A cache kept in `file`, read when first asked.
    #[must_use]
    pub fn new(file: Option<PathBuf>) -> Self {
        Self {
            file,
            ..Self::default()
        }
    }

    /// What the study of the ROM in `path` found, while the file is as it was then; none when
    /// it was not studied as it is.
    pub fn certificate(&mut self, path: &Path) -> Option<Certificate> {
        self.load();
        let stamp = Stamp::of(path)?;
        let (kept, certificate) = self.entries.get(&*path.to_string_lossy())?;
        (*kept == stamp).then(|| certificate.clone())
    }

    /// Keeps what the study of the ROM in `path` found.
    pub fn certify(&mut self, path: &Path, certificate: Certificate) {
        self.load();
        let Some(stamp) = Stamp::of(path) else {
            return;
        };
        let key = path.to_string_lossy().into_owned();
        if self.entries.get(&key) != Some(&(stamp, certificate.clone())) {
            self.entries.insert(key, (stamp, certificate));
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
            .map(|(path, (stamp, certificate))| {
                let (fits, patch, whole) = match certificate {
                    Certificate::Fits { patch, whole } => (true, patch.clone(), *whole),
                    Certificate::Unfit => (false, None, false),
                };
                Record {
                    path: path.clone(),
                    size: stamp.size,
                    modified: stamp.seconds,
                    modified_nanos: stamp.nanos,
                    fits,
                    patch,
                    whole,
                }
            })
            .collect();
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        let text = serde_json::to_string_pretty(&Body {
            hacks: fingerprint().to_owned(),
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
            .filter(|body| body.hacks == fingerprint())
        else {
            return;
        };
        for record in body.entries {
            let stamp = Stamp {
                size: record.size,
                seconds: record.modified,
                nanos: record.modified_nanos,
            };
            let certificate = if record.fits {
                Certificate::Fits {
                    patch: record.patch,
                    whole: record.whole,
                }
            } else {
                Certificate::Unfit
            };
            self.entries.insert(record.path, (stamp, certificate));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_study_is_kept_until_the_file_changes() {
        let folder =
            std::env::temp_dir().join(format!("zaxnes-hack-compat-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let rom = folder.join("game.nes");
        std::fs::write(&rom, b"NES\x1a0123456789abDATA").unwrap();
        let file = folder.join(FILE_NAME);
        let mut cache = HackCompatCache::new(Some(file.clone()));
        assert_eq!(None, cache.certificate(&rom), "not studied yet");
        let fits = Certificate::Fits {
            patch: Some("Test".to_owned()),
            whole: false,
        };
        cache.certify(&rom, fits.clone());
        cache.save().unwrap();

        let mut again = HackCompatCache::new(Some(file));
        assert_eq!(Some(fits.clone()), again.certificate(&rom));
        again.certify(&rom, fits);
        assert!(!again.dirty, "nothing new was learned");

        std::fs::write(&rom, b"NES\x1a0123456789abOTHER!").unwrap();
        assert_eq!(None, again.certificate(&rom), "the file changed");
        again.certify(&rom, Certificate::Unfit);
        assert_eq!(Some(Certificate::Unfit), again.certificate(&rom));
        drop(std::fs::remove_dir_all(&folder));
    }
}
