//! The slang shader pack (Rust-only, owner request): a part of libretro/slang-shaders inside the
//! executable (feature `shaders`, zipped by build.rs from `shaders/pack-files.txt`), written to
//! the user folder at start-up once per version of the pack. The presets the settings name by a
//! relative path are read from there.

use std::path::{Path, PathBuf};

/// The folder of the pack in the user folder `user_root`.
#[must_use]
pub fn folder(user_root: &Path) -> PathBuf {
    user_root.join("shaders").join("slang")
}

/// `preset` as a file: a relative path is one of the pack. A combination ("a.slangp|b.slangp",
/// the presets one after the other) gives each of its presets resolved, joined by the same bar.
#[must_use]
pub fn resolve(user_root: &Path, preset: &str) -> PathBuf {
    let one = |part: &str| {
        let path = Path::new(part);
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            folder(user_root).join(path)
        }
    };
    if !preset.contains(COMBINATION_SEPARATOR) {
        return one(preset);
    }
    let parts: Vec<String> = preset
        .split(COMBINATION_SEPARATOR)
        .map(|part| one(part).to_string_lossy().into_owned())
        .collect();
    PathBuf::from(parts.join(&COMBINATION_SEPARATOR.to_string()))
}

/// What separates the presets of a combination (`zax_platform::slang_chain`, which reads it).
pub const COMBINATION_SEPARATOR: char = '|';

/// The names the pack inside the executable holds, one per line (empty without the pack).
#[cfg(feature = "shaders")]
const PACKED: &str = include_str!(concat!(env!("OUT_DIR"), "/slang-pack-files.txt"));

/// Whether every preset of `preset` (one, or the parts of a combination) is in the pack the
/// executable carries: the Android package carries the light one (owner decision), and the
/// catalogue shows only what it can draw. A build without a pack keeps every preset, its
/// settings being the ones a desktop shares.
#[must_use]
pub fn contains(preset: &str) -> bool {
    #[cfg(feature = "shaders")]
    {
        if PACKED.is_empty() {
            return true;
        }
        parts(preset)
            .into_iter()
            .all(|part| PACKED.lines().any(|line| line == part))
    }
    #[cfg(not(feature = "shaders"))]
    {
        let _ = preset;
        true
    }
}

/// The presets of a stored preset, one for a plain preset, several for a combination.
#[must_use]
pub fn parts(preset: &str) -> Vec<&str> {
    preset.split(COMBINATION_SEPARATOR).collect()
}

/// The file, in the folder of the pack, that records the version written.
#[cfg(feature = "shaders")]
const MARKER_NAME: &str = ".zaxnes-pack";

#[cfg(feature = "shaders")]
const PACK: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/slang-pack.zip"));

/// The version of the pack inside the executable: `none` when it was built without the
/// slang-shaders collection.
#[must_use]
pub fn version() -> &'static str {
    #[cfg(feature = "shaders")]
    {
        env!("ZAXNES_SHADER_PACK_VERSION")
    }
    #[cfg(not(feature = "shaders"))]
    {
        "none"
    }
}

/// Whether the executable carries a pack.
#[must_use]
pub fn carried() -> bool {
    version() != "none"
}

/// Writes the pack into the user folder unless this version is already there: the folder of
/// the pack is replaced as a whole (it is Zaxnes's own). Returns whether it was written now.
///
/// # Errors
///
/// A folder or a file that cannot be written, or a damaged pack.
#[cfg(feature = "shaders")]
pub fn install(user_root: &Path) -> std::io::Result<bool> {
    use std::fs;
    let folder = folder(user_root);
    let marker = folder.join(MARKER_NAME);
    let version = version();
    if !carried() || fs::read_to_string(&marker).is_ok_and(|text| text.trim() == version) {
        return Ok(false);
    }
    if folder.is_dir() {
        fs::remove_dir_all(&folder)?;
    }
    fs::create_dir_all(&folder)?;
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(PACK))
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    // The names are checked by the archive: none leaves the folder.
    archive
        .extract(&folder)
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    fs::write(&marker, format!("{version}\n"))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_presets_are_the_packs() {
        let root = Path::new("user");
        assert_eq!(
            resolve(root, "crt/crt-easymode.slangp"),
            Path::new("user/shaders/slang/crt/crt-easymode.slangp")
        );
        let absolute = std::env::temp_dir().join("x.slangp");
        assert_eq!(resolve(root, &absolute.display().to_string()), absolute);
    }
}
