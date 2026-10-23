//! Port of `ProgramProfilesTests.cs` (AMD-033): the `profiles` folder next to the executable is
//! scanned in every mode, is only ever read, and its profiles are merged with the user's by the
//! rules of section 2.
//!
//! The chooser-label test is not ported here: the chooser model is part of the interface (T13).
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

mod common;

use std::fs;
use std::path::Path;

use common::TemporaryRoot;
use rstest::rstest;
use zax_app::emulation::game_catalog::GameCatalog;
use zax_app::library_game_catalog::LibraryGameCatalog;
use zax_app::user_data_paths::{UserDataPaths, UserDataRootKind, PORTABLE_MARKER_NAME};
use zax_profiles::catalogue::{ProfileImportOutcome, ProfileOrigin};
use zax_profiles::layout::FieldPresence;
use zax_profiles::library::GameProfileSource;
use zax_profiles::profile::GameProfile;
use zax_profiles::{export, format};

fn write_profile(path: &Path, id: &str, title: &str) {
    let mut profile = GameProfile::create_empty();
    profile.id = Some(id.to_owned());
    profile.title = Some(title.to_owned());
    export::normalize(&mut profile, None).unwrap();

    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, format::save(&profile, FieldPresence::All)).unwrap();
}

fn open(paths: &UserDataPaths) -> LibraryGameCatalog {
    LibraryGameCatalog::open(None, paths, Box::new(|_| {}))
}

fn file_count(folder: &Path) -> usize {
    fs::read_dir(folder).unwrap().count()
}

/// AMD-033 section 1: the second folder is `profiles` beside the executable in the three modes —
/// default root, portable and `--userdata` — and a profile dropped in it shows up in the library
/// in each of them.
#[rstest]
#[case(UserDataRootKind::LocalAppData)]
#[case(UserDataRootKind::Portable)]
#[case(UserDataRootKind::Explicit)]
fn amd_033_the_folder_beside_the_executable_is_scanned_in_every_mode(
    #[case] mode: UserDataRootKind,
) {
    let root = TemporaryRoot::new("zax-amd033");
    let executable = root.path().join("bin");
    let program_profiles = executable.join("profiles");
    fs::create_dir_all(&program_profiles).unwrap();
    write_profile(&program_profiles.join("Shipped.3dn"), "shipped", "Shipped");

    if mode == UserDataRootKind::Portable {
        fs::write(executable.join(PORTABLE_MARKER_NAME), "").unwrap();
    }

    let explicit = root.path().join("elsewhere");
    let paths = UserDataPaths::resolve(
        (mode == UserDataRootKind::Explicit).then_some(explicit.as_path()),
        Some(&executable),
        Some(&root.path().join("local")),
    );

    assert_eq!(paths.kind, mode);
    assert_eq!(paths.program_profiles_directory, program_profiles);
    assert_ne!(paths.profiles_directory, paths.program_profiles_directory);

    let mut catalog = open(&paths);
    let entry = catalog.library().find("shipped").unwrap();

    assert_eq!(entry.display_title(), "Shipped");
    assert_eq!(entry.source(), GameProfileSource::User);
    assert_eq!(entry.origin(), ProfileOrigin::ProgramFolder);
    assert_eq!(catalog.resolve("shipped").unwrap().id, "shipped");

    // The folder of the executable is read-only: the start-up writes nothing into it.
    assert_eq!(file_count(&program_profiles), 1);
}

/// AMD-033 section 1: a missing folder is neither an error nor created, and an unreadable one
/// lets the program start all the same.
#[test]
fn amd_033_a_missing_folder_is_not_created_and_is_not_an_error() {
    let root = TemporaryRoot::new("zax-amd033");
    let executable = root.path().join("bin");
    fs::create_dir_all(&executable).unwrap();

    let local = root.path().join("local");
    let paths = UserDataPaths::resolve(
        Some(&root.path().join("userdata")),
        Some(&executable),
        Some(&local),
    );

    assert!(paths.is_writable);
    assert!(!paths.program_profiles_directory.exists());

    let catalog = open(&paths);
    assert_eq!(catalog.library().entries().len(), 2);
    assert!(!paths.program_profiles_directory.exists());

    // A file where the folder is expected makes it unreadable as a folder; the start-up carries
    // on with the user profiles alone.
    let other = root.path().join("bin2");
    fs::create_dir_all(&other).unwrap();
    fs::write(other.join("profiles"), "not a folder").unwrap();
    let blocked = UserDataPaths::resolve(
        Some(&root.path().join("userdata2")),
        Some(&other),
        Some(&local),
    );

    assert_eq!(open(&blocked).library().entries().len(), 2);
}

/// AMD-033 section 2: a file of the same name in the user folder hides the one beside the
/// executable, and the merge rules apply to the real folders as well.
#[test]
fn amd_033_the_user_folder_hides_the_file_of_the_same_name() {
    let root = TemporaryRoot::new("zax-amd033");
    let executable = root.path().join("bin");
    let program_profiles = executable.join("profiles");
    fs::create_dir_all(&program_profiles).unwrap();

    let paths = UserDataPaths::resolve(
        Some(&root.path().join("userdata")),
        Some(&executable),
        Some(&root.path().join("local")),
    );

    write_profile(&paths.profiles_directory.join("Game.3dn"), "both", "Mine");
    write_profile(&program_profiles.join("GAME.3dn"), "other", "Theirs");
    write_profile(&program_profiles.join("Only.3dn"), "only", "Only there");

    let catalog = open(&paths);
    let library = catalog.library();

    assert_eq!(library.find("both").unwrap().display_title(), "Mine");
    assert!(library.find("other").is_none());
    assert_eq!(
        library.find("only").unwrap().origin(),
        ProfileOrigin::ProgramFolder
    );
}

/// AMD-033 section 1: an import writes into the user folder only, takes over the library entry of
/// the profile of the same identifier found beside the executable, and leaves that file where it
/// is. Q-074: a game of the program folder cannot be deleted either.
#[test]
fn amd_033_an_import_writes_to_the_user_folder_and_never_touches_the_other() {
    let root = TemporaryRoot::new("zax-amd033");
    let executable = root.path().join("bin");
    let program_profiles = executable.join("profiles");
    fs::create_dir_all(&program_profiles).unwrap();
    let shipped = program_profiles.join("Shipped.3dn");
    write_profile(&shipped, "same", "Shipped");

    let paths = UserDataPaths::resolve(
        Some(&root.path().join("userdata")),
        Some(&executable),
        Some(&root.path().join("local")),
    );
    let mut catalog = open(&paths);

    let from_program = catalog.library().find("same").unwrap();
    assert!(from_program.is_from_program_folder());
    assert!(!from_program.can_delete());
    assert!(!catalog.delete_game("same"));
    assert!(shipped.is_file());

    let source = root.path().join("Better.3dn");
    write_profile(&source, "same", "Better");
    let result = catalog.import_profile(&source);

    assert_eq!(result.outcome, ProfileImportOutcome::Added);
    let imported = paths.profiles_directory.join("Better.3dn");
    assert!(imported.is_file());
    assert!(shipped.is_file());
    assert_eq!(file_count(&program_profiles), 1);

    let entry = catalog.library().find("same").unwrap();
    assert_eq!(entry.profile_path(), imported);
    assert_eq!(entry.origin(), ProfileOrigin::User);
    assert!(entry.can_delete());
}
