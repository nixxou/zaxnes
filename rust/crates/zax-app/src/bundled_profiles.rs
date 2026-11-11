//! The 3D profiles made for Zaxnes (the `zaxnes_3dn` repository: *Faxanadu*, *Metal Force*...),
//! inside the executable of a release build (owner request). Their files are not in these
//! sources: `build.rs` includes those of the copy it finds, and a build without one carries none.
//! At start-up they go into the user's profiles, once per version of the set, as `FlappyZaxNes`
//! does: a player who deletes one does not get it back until a release changes the set.

use std::fs;
use std::io;

use crate::user_data_paths::UserDataPaths;

/// The file, in the user data, that records the version installed.
pub const MARKER_NAME: &str = "bundled-profiles.txt";

/// Each profile's file name and bytes.
pub const PROFILES: &[(&str, &[u8])] = include!(concat!(env!("OUT_DIR"), "/bundled-profiles.rs"));

/// The version of the set: an FNV-1a hash of the names and the files.
#[must_use]
pub fn version() -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for (name, bytes) in PROFILES {
        for byte in name.as_bytes().iter().chain(*bytes) {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    }
    format!("{hash:016x}")
}

/// Writes the profiles into the user's profiles, unless this version of the set is already
/// installed; gives the names written now.
///
/// # Errors
///
/// A folder or a file that cannot be written.
pub fn install(paths: &UserDataPaths) -> io::Result<Vec<&'static str>> {
    let marker = paths.root.join(MARKER_NAME);
    let version = version();
    if PROFILES.is_empty()
        || !paths.is_writable
        || fs::read_to_string(&marker).is_ok_and(|text| text.trim() == version)
    {
        return Ok(Vec::new());
    }
    fs::create_dir_all(&paths.profiles_directory)?;
    for (name, bytes) in PROFILES {
        fs::write(paths.profiles_directory.join(name), bytes)?;
    }
    fs::write(&marker, format!("{version}\n"))?;
    Ok(PROFILES.iter().map(|(name, _)| *name).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The set goes in once per version: a second start writes nothing, and a deleted profile
    /// stays deleted.
    #[test]
    fn the_bundled_profiles_are_installed_once_per_version() {
        let root = std::env::temp_dir().join(format!("zax-bundled-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let paths = UserDataPaths::resolve(Some(&root), None, None);
        let written = install(&paths).unwrap();
        assert_eq!(written.len(), PROFILES.len());
        for (name, bytes) in PROFILES {
            assert_eq!(
                fs::read(paths.profiles_directory.join(name)).unwrap(),
                *bytes
            );
        }
        if let Some((name, _)) = PROFILES.first() {
            fs::remove_file(paths.profiles_directory.join(name)).unwrap();
            assert_eq!(install(&paths).unwrap(), Vec::<&str>::new());
            assert!(!paths.profiles_directory.join(name).exists());
        }
        let _ = fs::remove_dir_all(&root);
    }
}
