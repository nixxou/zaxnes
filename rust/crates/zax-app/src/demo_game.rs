//! `FlappyZaxNes`, the game shipped with Zaxnes: a NES game written for it (after Flappy NES by
//! filipemd, GPL-3.0; its sources are in `flappy-src`) and its 3D profile, both inside the
//! executable. At start-up the ROM goes into the ROM folder and the profile into the user's
//! profiles, and the library ties the profile to the ROM, once per version of the pair: a
//! player who deletes them does not get them back until a release changes them.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::user_data_paths::UserDataPaths;

/// The file name of the ROM in the ROM folder.
pub const ROM_NAME: &str = "FlappyZaxNes.nes";
/// The file name of the profile in the user's profiles.
pub const PROFILE_NAME: &str = "flappyzaxnes.3dn";
/// The identifier the profile carries.
pub const PROFILE_ID: &str = "flappyzaxnes";
/// The file, in the user data, that records the version installed.
pub const MARKER_NAME: &str = "demo-game.txt";

const ROM: &[u8] = include_bytes!("../demo/FlappyZaxNes.nes");
const PROFILE: &[u8] = include_bytes!("../demo/flappyzaxnes.3dn");

/// The version of the pair: an FNV-1a hash of both files.
#[must_use]
pub fn version() -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in ROM.iter().chain(PROFILE) {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// Writes the ROM into `roms` and the profile into the user's profiles, unless this version is
/// already installed. Returns the path of the ROM when it was written now, for the library.
///
/// # Errors
///
/// A folder or a file that cannot be written.
pub fn install(paths: &UserDataPaths, roms: &Path) -> io::Result<Option<PathBuf>> {
    let marker = paths.root.join(MARKER_NAME);
    let version = version();
    if !paths.is_writable || fs::read_to_string(&marker).is_ok_and(|text| text.trim() == version) {
        return Ok(None);
    }
    fs::create_dir_all(roms)?;
    fs::create_dir_all(&paths.profiles_directory)?;
    let rom = roms.join(ROM_NAME);
    fs::write(&rom, ROM)?;
    fs::write(paths.profiles_directory.join(PROFILE_NAME), PROFILE)?;
    fs::write(&marker, format!("{version}\n"))?;
    Ok(Some(std::path::absolute(&rom).unwrap_or(rom)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pair goes in once per version: a second start writes nothing, and a deleted ROM
    /// stays deleted.
    #[test]
    fn the_demo_game_is_installed_once_per_version() {
        let root = std::env::temp_dir().join(format!("zax-demo-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let paths = UserDataPaths::resolve(Some(&root), None, None);
        let roms = root.join("roms");
        let rom = install(&paths, &roms).unwrap().unwrap();
        assert_eq!(fs::read(&rom).unwrap(), ROM);
        assert_eq!(
            fs::read(paths.profiles_directory.join(PROFILE_NAME)).unwrap(),
            PROFILE
        );
        fs::remove_file(&rom).unwrap();
        assert_eq!(install(&paths, &roms).unwrap(), None);
        assert!(!rom.exists());
        let _ = fs::remove_dir_all(&root);
    }
}
