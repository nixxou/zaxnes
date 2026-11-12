//! The 3D profile of a game: the ones the core carries (`FlappyZaxNes`'s, and the profiles made
//! for Zaxnes that `build.rs` finds), then the `.3dn` files of the frontend's system folder
//! (`<system>/Zaxnes/profiles`). A profile suits a ROM whose file name is its title, or starts
//! with it, as the application's ROM list matches them (`rom_list::matching_profiles`).

use std::path::Path;

use zax_profiles::format::{self, ProfileDocument};

/// The profile of `FlappyZaxNes`, the game shipped with Zaxnes.
const FLAPPY: &[u8] = include_bytes!("../../zax-app/demo/flappyzaxnes.3dn");

/// The profiles made for Zaxnes (`zaxnes_3dn`), as `build.rs` found them.
const BUNDLED: &[(&str, &[u8])] = include!(concat!(env!("OUT_DIR"), "/bundled-profiles.rs"));

/// The folder of the system folder the profiles are read from.
pub const FOLDER: &str = "Zaxnes/profiles";

/// A name cut at its first tag and kept to its letters and digits, lower case: "Super Mario
/// Bros. 3 (USA)" and "`super_mario_bros_3`" meet at "supermariobros3".
#[must_use]
pub fn normalise(name: &str) -> String {
    let untagged = name.split(['(', '[']).next().unwrap_or_default();
    untagged
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// How well a profile titled `title` suits a ROM named `rom`: 0 the same name, 1 one starts
/// with the other, none otherwise.
fn rank(title: &str, rom: &str) -> Option<u8> {
    let (title, rom) = (normalise(title), normalise(rom));
    if title.is_empty() || rom.is_empty() {
        None
    } else if title == rom {
        Some(0)
    } else if rom.starts_with(&title) || title.starts_with(&rom) {
        Some(1)
    } else {
        None
    }
}

/// The profile that suits the ROM `rom_name` (its file name), from the core's own and those in
/// `system`'s profile folder, with its tiles resolved from `image` for a "3dh" one; the closest
/// title wins, the core's own on a tie.
#[must_use]
pub fn find(rom_name: &str, image: &[u8], system: Option<&Path>) -> Option<ProfileDocument> {
    let stem = Path::new(rom_name)
        .file_stem()
        .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned());
    let rom_name = stem.as_str();
    let mut best: Option<(u8, ProfileDocument)> = None;
    let mut consider = |bytes: &[u8], source: &str| {
        let Ok(document) = format::load(bytes, None, Some(source)) else {
            return;
        };
        let Some(rank) = document
            .profile
            .title
            .as_deref()
            .and_then(|title| rank(title, rom_name))
        else {
            return;
        };
        if best.as_ref().is_none_or(|(kept, _)| rank < *kept) {
            best = Some((rank, document));
        }
    };
    consider(FLAPPY, "flappyzaxnes.3dn");
    for (name, bytes) in BUNDLED {
        consider(bytes, name);
    }
    if let Some(folder) = system.map(|system| system.join(FOLDER)) {
        if let Ok(entries) = std::fs::read_dir(folder) {
            let mut files: Vec<_> = entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| {
                    path.extension()
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("3dn"))
                })
                .collect();
            files.sort();
            for path in files {
                if let Ok(bytes) = std::fs::read(&path) {
                    consider(&bytes, &path.to_string_lossy());
                }
            }
        }
    }
    let (_, mut document) = best?;
    if document.is_hash_profile() {
        document.resolve_from_rom(image);
    }
    Some(document)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_title_suits_the_roms_that_carry_it() {
        assert_eq!(
            rank("Super Mario Bros. 3", "Super Mario Bros. 3 [USA][!!]"),
            Some(0)
        );
        assert_eq!(rank("Kirby", "Kirby's Adventure (USA)"), Some(1));
        assert_eq!(rank("Zelda II", "Metroid (USA)"), None);
    }

    #[test]
    fn the_demo_finds_its_own_profile() {
        let document = find("FlappyZaxNes.nes", &[], None);
        assert!(
            document.is_some_and(|document| !document.profile.permanent_patterns.is_empty()
                || !document.profile.temporary_patterns.is_empty())
        );
    }
}
