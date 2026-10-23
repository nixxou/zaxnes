use std::path::{Path, PathBuf};

use zax_profiles::catalogue::{
    import_messages, CatalogueEntry, ProfileCatalogue, ProfileFileSystem, ProfileFolders,
    ProfileOrigin, UserProfileFolder,
};
use zax_profiles::format::{self, ProfileLog};
use zax_profiles::layout::FieldPresence;
use zax_profiles::library::{GameLibrary, GameProfileSource};
use zax_profiles::profile::GameProfile;

use super::common::fake_fs::FakeProfileFileSystem;
use super::common::recording_log::RecordingProfileLog;

// In-memory paths: nothing is read from or written to a real disk, and no path of any machine
// appears here (lot PUB-1).
const USER_FOLDER: &str = "/fake/AppData/Local/Zaxnes/profiles";
const PROGRAM_FOLDER: &str = "/fake/Program Files/Zaxnes/profiles";

fn make_profile(id: Option<&str>, title: Option<&str>) -> Vec<u8> {
    let mut profile = GameProfile::create_empty();
    profile.id = id.map(str::to_owned);
    profile.title = title.map(str::to_owned);
    format::save(&profile, FieldPresence::All)
}

fn folders<'a>(
    files: &'a FakeProfileFileSystem,
    program: Option<&str>,
    log: Option<&'a RecordingProfileLog>,
) -> ProfileFolders<'a> {
    ProfileFolders::new(
        files,
        Path::new(USER_FOLDER),
        program.map(Path::new),
        log.map(|log| log as &dyn ProfileLog),
    )
}

fn user(name: &str) -> PathBuf {
    Path::new(USER_FOLDER).join(name)
}

fn program(name: &str) -> PathBuf {
    Path::new(PROGRAM_FOLDER).join(name)
}

#[test]
fn amd_033_both_folders_are_scanned_and_the_origin_is_kept() {
    let files = FakeProfileFileSystem::new();
    files.add_file(&user("Mine.3dn"), &make_profile(Some("mine"), Some("Mine")));
    files.add_file(
        &program("Shipped.3dn"),
        &make_profile(Some("shipped"), Some("Shipped")),
    );

    let log = RecordingProfileLog::new();
    let profiles = folders(&files, Some(PROGRAM_FOLDER), Some(&log))
        .scan(&[])
        .unwrap();

    assert_eq!(2, profiles.len());
    let mine = profiles
        .iter()
        .find(|profile| profile.id == "mine")
        .unwrap();
    let shipped = profiles
        .iter()
        .find(|profile| profile.id == "shipped")
        .unwrap();
    assert_eq!(ProfileOrigin::User, mine.origin);
    assert_eq!(ProfileOrigin::ProgramFolder, shipped.origin);
    assert_eq!(program("Shipped.3dn"), shipped.file_path);
    assert!(log.warnings.borrow().is_empty());
}

#[test]
fn amd_033_a_file_of_the_same_name_hides_the_one_of_the_program_folder() {
    let files = FakeProfileFileSystem::new();
    files.add_file(&user("Game.3dn"), &make_profile(Some("mine"), Some("Mine")));

    // Not a profile at all: were it read, it would be reported as refused.
    let hidden = program("GAME.3dn");
    files.add_file(&hidden, &[0x0a, 0x03, 0x33, 0x64, 0x78]);

    let log = RecordingProfileLog::new();
    let profiles = folders(&files, Some(PROGRAM_FOLDER), Some(&log))
        .scan(&[])
        .unwrap();

    assert_eq!(1, profiles.len());
    assert_eq!("mine", profiles[0].id);
    assert_eq!(ProfileOrigin::User, profiles[0].origin);
    assert!(!files.reads().contains(&hidden));
    assert!(log.warnings.borrow().is_empty());
}

#[test]
fn amd_033_two_byte_identical_files_appear_once_and_silently() {
    let bytes = make_profile(Some("twice"), Some("Twice"));
    let files = FakeProfileFileSystem::new();
    files.add_file(&user("Here.3dn"), &bytes);
    files.add_file(&program("There.3dn"), &bytes);

    let log = RecordingProfileLog::new();
    let profiles = folders(&files, Some(PROGRAM_FOLDER), Some(&log))
        .scan(&[])
        .unwrap();

    assert_eq!(1, profiles.len());
    assert_eq!(user("Here.3dn"), profiles[0].file_path);
    assert_eq!(ProfileOrigin::User, profiles[0].origin);
    assert!(log.warnings.borrow().is_empty());
}

#[test]
fn amd_033_the_user_folder_wins_the_same_identifier_and_the_other_is_logged_once() {
    let files = FakeProfileFileSystem::new();
    files.add_file(&user("Mine.3dn"), &make_profile(Some("same"), Some("Mine")));
    let loser = program("Theirs.3dn");
    files.add_file(&loser, &make_profile(Some("same"), Some("Theirs")));

    let log = RecordingProfileLog::new();
    let profiles = folders(&files, Some(PROGRAM_FOLDER), Some(&log))
        .scan(&[])
        .unwrap();

    assert_eq!(1, profiles.len());
    assert_eq!("Mine", profiles[0].title);
    assert_eq!(ProfileOrigin::User, profiles[0].origin);
    assert_eq!(1, log.warnings.borrow().len());
    assert!(log
        .warnings
        .borrow()
        .contains(&import_messages::duplicated_profile(&loser, "same")));
}

#[test]
fn amd_033_the_reading_and_rejection_rules_apply_to_the_second_folder() {
    let files = FakeProfileFileSystem::new();
    files.add_file(
        &program("a-builtin.3dn"),
        &make_profile(Some("1"), Some("Not Arkanoid")),
    );
    files.add_file(
        &program("b-noid.3dn"),
        &make_profile(None, Some("No identifier")),
    );
    files.add_file(&program("c-foreign.3dn"), &[0x0a, 0x03, 0x33, 0x64, 0x78]);
    files.add_file(
        &program("d-good.3dn"),
        &make_profile(Some("good"), Some("Good")),
    );

    let log = RecordingProfileLog::new();
    let profiles = folders(&files, Some(PROGRAM_FOLDER), Some(&log))
        .scan(&["1"])
        .unwrap();

    assert_eq!(1, profiles.len());
    assert_eq!("good", profiles[0].id);
    assert_eq!(ProfileOrigin::ProgramFolder, profiles[0].origin);
    assert!(log.any_contains("duplicated profile"));
    assert!(log.any_contains("carries no identifier"));
    assert!(log.any_contains(import_messages::NOT_A_3DN_FILE));
}

#[test]
fn amd_033_a_missing_or_unreadable_folder_is_not_an_error() {
    let files = FakeProfileFileSystem::new();
    files.add_file(&user("Mine.3dn"), &make_profile(Some("mine"), Some("Mine")));

    assert_eq!(
        1,
        folders(&files, Some(PROGRAM_FOLDER), None)
            .scan(&[])
            .unwrap()
            .len()
    );
    assert_eq!(1, folders(&files, None, None).scan(&[]).unwrap().len());
    assert_eq!(1, folders(&files, Some(""), None).scan(&[]).unwrap().len());

    files.add_file(
        &program("Shipped.3dn"),
        &make_profile(Some("shipped"), None),
    );
    files.make_unreadable(Path::new(PROGRAM_FOLDER));

    let log = RecordingProfileLog::new();
    let profiles = folders(&files, Some(PROGRAM_FOLDER), Some(&log))
        .scan(&[])
        .unwrap();
    assert_eq!(1, profiles.len());
    assert_eq!("mine", profiles[0].id);
    assert!(log.any_contains("could not be listed"));
}

#[test]
fn amd_033_the_same_folder_under_both_names_is_scanned_once() {
    let files = FakeProfileFileSystem::new();
    files.add_file(
        &program("Mine.3dn"),
        &make_profile(Some("mine"), Some("Mine")),
    );

    let log = RecordingProfileLog::new();
    let same = ProfileFolders::new(
        &files,
        Path::new(PROGRAM_FOLDER),
        Some(Path::new(&format!("{PROGRAM_FOLDER}/"))),
        Some(&log),
    );

    assert!(same.program_directory().is_none());
    let profiles = same.scan(&[]).unwrap();
    assert_eq!(1, profiles.len());
    assert_eq!(ProfileOrigin::User, profiles[0].origin);
    assert!(log.warnings.borrow().is_empty());
}

// Q-074
#[test]
fn amd_033_the_library_carries_the_origin_and_protects_the_program_folder() {
    let files = FakeProfileFileSystem::new();
    files.add_file(&user("Mine.3dn"), &make_profile(Some("mine"), Some("Mine")));
    files.add_file(
        &program("Shipped.3dn"),
        &make_profile(Some("shipped"), Some("Shipped")),
    );

    let catalogue = ProfileCatalogue::from_entries(
        Path::new("/fake/assets"),
        vec![CatalogueEntry {
            id: "47".to_owned(),
            title: "2D Mode".to_owned(),
            publisher: String::new(),
            description: String::new(),
            author: String::new(),
            file: "profiles/empty.bytes".to_owned(),
        }],
    );
    let scanned = folders(&files, Some(PROGRAM_FOLDER), None)
        .scan(&catalogue.ids())
        .unwrap();
    let mut library = GameLibrary::build(&catalogue, &scanned, None, None);

    let shipped = library.find("shipped").unwrap();
    assert_eq!(GameProfileSource::User, shipped.source());
    assert_eq!(ProfileOrigin::ProgramFolder, shipped.origin());
    assert!(shipped.is_from_program_folder());
    assert!(!shipped.can_delete());
    assert!(!library.remove("shipped"));

    let mine = library.find("mine").unwrap();
    assert!(!mine.is_from_program_folder());
    assert!(mine.can_delete());
    assert!(library.remove("mine"));
}

#[test]
fn amd_033_an_import_writes_to_the_user_folder_and_takes_over_the_entry() {
    let files = FakeProfileFileSystem::new();
    let shipped = program("Shipped.3dn");
    files.add_file(&shipped, &make_profile(Some("same"), Some("Shipped")));
    let source = Path::new("/fake/downloads/Better.3dn");
    files.add_file(source, &make_profile(Some("same"), Some("Better")));

    let profile_folders = folders(&files, Some(PROGRAM_FOLDER), None);
    let catalogue = ProfileCatalogue::from_entries(Path::new("/fake/assets"), Vec::new());
    let mut library =
        GameLibrary::build(&catalogue, &profile_folders.scan(&[]).unwrap(), None, None);
    assert_eq!(
        ProfileOrigin::ProgramFolder,
        library.find("same").unwrap().origin()
    );

    // The import only ever knows the profiles of the user folder, so it has nothing to delete;
    // the file next to the executable stays where it is.
    let user_folder =
        UserProfileFolder::new(&files, Path::new(USER_FOLDER), None, ProfileOrigin::User);
    let result = user_folder.import(source, &[], &[]).unwrap();

    assert_eq!(
        zax_profiles::catalogue::ProfileImportOutcome::Added,
        result.outcome
    );
    assert!(files.file_exists(&user("Better.3dn")));
    assert!(files.file_exists(&shipped));

    library
        .add_user_profile(result.profile.as_ref().unwrap(), None)
        .unwrap();
    let entry = library.find("same").unwrap();
    assert_eq!(user("Better.3dn"), entry.profile_path());
    assert_eq!(ProfileOrigin::User, entry.origin());
    assert!(entry.can_delete());

    // And the next scan keeps the user's file, the program one now being a duplicate.
    let kept = profile_folders.scan(&[]).unwrap();
    assert_eq!(1, kept.len());
    assert_eq!(ProfileOrigin::User, kept[0].origin);
}
