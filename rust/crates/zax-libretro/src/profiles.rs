//! The 3D profile of a game: the ones the core carries (the profiles made for Zaxnes that
//! `build.rs` finds; the core ships no game, `FlappyZaxNes` included, owner decision), then the
//! `.3dn` files of the frontend's system folder
//! (`<system>/Zaxnes/profiles`). A profile suits a ROM whose file name is its title, or starts
//! with it, as the application's ROM list matches them (`rom_list::matching_profiles`); a profile
//! with no title (those of a 3dSen 2.0 bundle) goes by its file name, as in the application.

use std::path::Path;

use zax_profiles::catalogue::profile_title;
use zax_profiles::format::{self, ProfileDocument};

/// The profiles made for Zaxnes (`rust/profiles`), as `build.rs` found them.
const BUNDLED: &[(&str, &[u8])] = include!(concat!(env!("OUT_DIR"), "/bundled-profiles.rs"));

/// The folder of the system folder the profiles are read from.
pub const FOLDER: &str = "Zaxnes/profiles";

/// The folder of the system folder where what is to be imported waits (owner request,
/// 2026-11-20): a 3dSen bundle, an archive holding one, or `.3dn` and `.3dh` profiles.
pub const IMPORT_FOLDER: &str = "Zaxnes/import";

/// Imports what waits in `system`'s import folder into its profile folder, each file then
/// marked with a `.done` beside it (as the application's import folder); what came of each,
/// for the log. Creates the folder and its README when they are missing.
#[must_use]
pub fn import_waiting(system: &Path) -> Vec<(bool, String)> {
    use zax_profiles::catalogue::PhysicalProfileFileSystem;
    use zax_profiles::import::folder::{self, WaitingKind};
    use zax_profiles::import::ThreeDSenImporter;

    let source = system.join(IMPORT_FOLDER);
    let destination = system.join(FOLDER);
    folder::prepare(&source);
    let mut results = Vec::new();
    for (path, kind) in folder::waiting(&source) {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let (ok, text) = match kind {
            WaitingKind::ThreeDSen => {
                let files = PhysicalProfileFileSystem;
                match ThreeDSenImporter::new(&files, &destination, None).import_from(&path) {
                    Ok(summary) => (summary.failed == 0, summary.message()),
                    Err(error) => (false, error.to_string()),
                }
            }
            // A profile is copied as it is, after a check that it decodes.
            WaitingKind::Profile => match std::fs::read(&path) {
                Ok(bytes) if format::load(&bytes, None, Some(&name)).is_ok() => {
                    let copied = std::fs::create_dir_all(&destination)
                        .and_then(|()| std::fs::write(destination.join(&name), &bytes));
                    match copied {
                        Ok(()) => (true, "profile copied".to_owned()),
                        Err(error) => (false, error.to_string()),
                    }
                }
                Ok(_) => (false, "not a usable profile".to_owned()),
                Err(error) => (false, error.to_string()),
            },
        };
        folder::mark_done(
            &path,
            &format!("{}: {text}", if ok { "ok" } else { "error" }),
        );
        results.push((ok, format!("{name}: {text}")));
    }
    results
}

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
    find_titled(rom_name, image, system).map(|(_, document)| document)
}

/// [`find`], with the title the profile goes by (its own, or its file name when it has none).
#[must_use]
pub fn find_titled(
    rom_name: &str,
    image: &[u8],
    system: Option<&Path>,
) -> Option<(String, ProfileDocument)> {
    let stem = Path::new(rom_name)
        .file_stem()
        .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned());
    let rom_name = stem.as_str();
    let mut best: Option<(u8, String, ProfileDocument)> = None;
    let mut consider = |bytes: &[u8], source: &str| {
        let Ok(document) = format::load(bytes, None, Some(source)) else {
            return;
        };
        let title = profile_title(document.profile.title.as_deref(), Path::new(source));
        let Some(rank) = rank(&title, rom_name) else {
            return;
        };
        if best.as_ref().is_none_or(|(kept, _, _)| rank < *kept) {
            best = Some((rank, title, document));
        }
    };
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
    let (_, title, mut document) = best?;
    if document.is_hash_profile() {
        document.resolve_from_rom(image);
    }
    Some((title, document))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_import_folder_copies_a_profile_and_marks_what_it_did() {
        let system =
            std::env::temp_dir().join(format!("zax-libretro-import-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&system);
        let folder = system.join(IMPORT_FOLDER);
        std::fs::create_dir_all(&folder).unwrap();
        let (_, profile) = BUNDLED[0];
        std::fs::write(folder.join("Mine.3dn"), profile).unwrap();
        std::fs::write(folder.join("broken.bundle"), b"not a bundle").unwrap();
        let results = import_waiting(&system);
        assert_eq!(2, results.len(), "{results:?}");
        assert_eq!(
            profile,
            std::fs::read(system.join(FOLDER).join("Mine.3dn")).unwrap()
        );
        assert!(results
            .iter()
            .any(|(ok, text)| !ok && text.starts_with("broken.bundle")));
        assert!(
            folder.join("Mine.3dn.done").is_file() && folder.join("broken.bundle.done").is_file()
        );
        assert!(folder.join("README.txt").is_file());
        // Done once.
        assert!(import_waiting(&system).is_empty());
        let _ = std::fs::remove_dir_all(&system);
    }

    #[test]
    fn a_title_suits_the_roms_that_carry_it() {
        assert_eq!(
            rank("Super Mario Bros. 3", "Super Mario Bros. 3 [USA][!!]"),
            Some(0)
        );
        assert_eq!(rank("Kirby", "Kirby's Adventure (USA)"), Some(1));
        assert_eq!(rank("Zelda II", "Metroid (USA)"), None);
    }

    /// Issue #2: the profiles of a 3dSen 2.0 bundle have no title; the file name stands for it.
    #[test]
    fn an_untitled_profile_goes_by_its_file_name() {
        let title = profile_title(None, Path::new("profiles/Adventures of Lolo (U).3dn"));
        assert_eq!(rank(&title, "Adventures of Lolo (USA)"), Some(0));
        let title = profile_title(Some(""), Path::new("Arkanoid (U).3dn"));
        assert_eq!(rank(&title, "Arkanoid (USA)"), Some(0));
        let title = profile_title(Some("Faxanadu"), Path::new("whatever.3dn"));
        assert_eq!(rank(&title, "Faxanadu (USA) (Rev 1)"), Some(0));
    }

    /// Owner decision: the core ships no game, so not `FlappyZaxNes`'s profile either.
    #[test]
    fn the_core_carries_no_profile_of_flappyzaxnes() {
        assert!(find("FlappyZaxNes.nes", &[], None).is_none());
    }
}
