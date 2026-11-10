use std::path::Path;
use std::path::PathBuf;

use time::macros::datetime;
use time::OffsetDateTime;
use zax_profiles::catalogue::{
    derived_id, import_messages, ProfileCatalogue, ProfileCatalogueError, ProfileFileSystem,
    ProfileImportOutcome, UserDataLayout, UserProfile, UserProfileFolder, DEMO_EDITION,
    FULL_EDITION,
};
use zax_profiles::format::{self, ProfileHeader};
use zax_profiles::layout::FieldPresence;
use zax_profiles::library::{
    GameLibrary, GameLibraryFile, GameLibraryRecord, GameLibraryStore, GameProfileSource,
    LibrarySortOption,
};
use zax_profiles::profile::GameProfile;

use super::common::fake_fs::FakeProfileFileSystem;
use super::common::neutral_assets;
use super::common::physical_fs::PhysicalProfileFileSystem;
use super::common::recording_log::RecordingProfileLog;

// Two roots of the in-memory file system of these tests (FakeProfileFileSystem): nothing is
// read from or written to a real disk, and no path of any machine appears here (lot PUB-1).
const ROOT: &str = "/fake/AppData/Local/Zaxnes";
const ASSETS: &str = "/fake/assets-neutral";

fn make_profile(id: Option<&str>, title: Option<&str>) -> Vec<u8> {
    let mut profile = GameProfile::create_empty();
    profile.id = id.map(str::to_owned);
    profile.title = title.map(str::to_owned);
    format::save(&profile, FieldPresence::All)
}

fn make_catalogue_json(entries: &[(&str, &str, &str)]) -> String {
    let full: Vec<String> = entries
        .iter()
        .map(|(id, title, file)| {
            format!(
                r#"{{
 "id": "{id}",
 "title": "{title}",
 "publisher": "",
 "description": "",
 "author": "",
 "file": "{file}"
}}"#
            )
        })
        .collect();

    format!(
        r#"{{
 "comment": "test",
 "editions": {{
  "full": [{}],
  "demo": []
 }}
}}"#,
        full.join(",")
    )
}

fn catalogue_of(files: &FakeProfileFileSystem, entries: &[(&str, &str, &str)]) -> ProfileCatalogue {
    let path = Path::new(ASSETS).join("profiles.json");
    files.add_text(&path, &make_catalogue_json(entries));
    ProfileCatalogue::load(files, &path, FULL_EDITION).unwrap()
}

fn titles(entries: &[&zax_profiles::library::GameLibraryEntry]) -> Vec<String> {
    entries
        .iter()
        .map(|entry| entry.display_title().to_owned())
        .collect()
}

/// The titles the enumeration of REQ-UI-210 and REQ-UI-211 keeps, in alphabetical order.
fn kept(library: &GameLibrary, search: &str, favorites_only: bool) -> Vec<String> {
    titles(&library.enumerate(LibrarySortOption::Alphabetical, search, favorites_only))
}

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_060_the_full_edition_of_the_catalogue_holds_the_101_built_in_games() {
    let catalogue = ProfileCatalogue::load(
        &PhysicalProfileFileSystem,
        &neutral_assets::catalogue_file(),
        FULL_EDITION,
    )
    .unwrap();

    assert_eq!(101, catalogue.entries().len());
    assert_eq!(101, catalogue.ids().len());

    let arkanoid = catalogue.find("1").unwrap();
    assert_eq!("Arkanoid", arkanoid.title);
    assert_eq!("profiles/Arkanoid (U).bytes", arkanoid.file);
    assert!(PhysicalProfileFileSystem.file_exists(&catalogue.profile_path(arkanoid)));
}

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_061_the_game_of_identifier_47_is_the_2d_mode_entry() {
    let catalogue = ProfileCatalogue::load(
        &PhysicalProfileFileSystem,
        &neutral_assets::catalogue_file(),
        FULL_EDITION,
    )
    .unwrap();

    let two_d = catalogue.find("47").unwrap();
    assert_eq!("2D Mode", two_d.title);
    assert_eq!("profiles/empty.bytes", two_d.file);
    assert!(two_d.is_2d_mode());
    assert!(std::ptr::eq(
        two_d,
        catalogue.two_dimensional_mode().unwrap()
    ));
}

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_060_every_catalogue_entry_points_at_a_profile_that_loads() {
    let catalogue = ProfileCatalogue::load(
        &PhysicalProfileFileSystem,
        &neutral_assets::catalogue_file(),
        FULL_EDITION,
    )
    .unwrap();

    for entry in catalogue.entries() {
        if let Err(error) = catalogue.load_profile(&PhysicalProfileFileSystem, entry, None) {
            panic!("{} {}: {error}", entry.id, entry.title);
        }
    }
}

/// DEC-03-Q04: only the full edition is used. The demo edition is still readable, and its
/// identifiers deliberately overlap those of the full edition.
#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_060_the_demo_edition_is_data_the_application_does_not_use() {
    let demo = ProfileCatalogue::load(
        &PhysicalProfileFileSystem,
        &neutral_assets::catalogue_file(),
        DEMO_EDITION,
    )
    .unwrap();

    assert_eq!(20, demo.entries().len());
    assert_ne!("Arkanoid", demo.find("1").unwrap().title);
}

#[test]
fn req_prof_060_an_unreadable_catalogue_is_reported_not_swallowed() {
    let files = FakeProfileFileSystem::new();
    files.add_text(Path::new("/assets/profiles.json"), "{{{");

    assert!(matches!(
        ProfileCatalogue::load(&files, Path::new("/assets/profiles.json"), FULL_EDITION),
        Err(ProfileCatalogueError::InvalidJson(_))
    ));
    assert!(matches!(
        ProfileCatalogue::load(&files, Path::new("/assets/missing.json"), FULL_EDITION),
        Err(ProfileCatalogueError::Missing(_))
    ));
    assert!(matches!(
        ProfileCatalogue::parse("{}", Path::new("/assets"), FULL_EDITION),
        Err(ProfileCatalogueError::NoEditions)
    ));
}

#[test]
fn req_prof_062_a_user_profile_without_a_title_is_named_after_its_file() {
    let files = FakeProfileFileSystem::new();
    let layout = UserDataLayout::with_default_names(Path::new(ROOT));
    files.add_file(
        &layout.profiles_directory().join("Foo.3dn"),
        &make_profile(Some("abc123def456"), None),
    );

    let log = RecordingProfileLog::new();
    let folder = UserProfileFolder::new(
        &files,
        layout.profiles_directory(),
        Some(&log),
        zax_profiles::catalogue::ProfileOrigin::User,
    );
    let profiles = folder.scan(&[]).unwrap();

    assert_eq!(1, profiles.len());
    assert_eq!("abc123def456", profiles[0].id);
    assert_eq!("Foo", profiles[0].title);
}

#[test]
fn req_prof_062_a_second_profile_with_the_same_identifier_is_ignored() {
    let files = FakeProfileFileSystem::new();
    let layout = UserDataLayout::with_default_names(Path::new(ROOT));
    files.add_file(
        &layout.profiles_directory().join("Aaa.3dn"),
        &make_profile(Some("dup"), Some("First")),
    );
    files.add_file(
        &layout.profiles_directory().join("Bbb.3dn"),
        &make_profile(Some("dup"), Some("Second")),
    );

    let log = RecordingProfileLog::new();
    let folder = user_folder(&files, layout.profiles_directory(), Some(&log));
    let profiles = folder.scan(&[]).unwrap();

    // The folder is walked in ordinal order of the file names, so the winner does not depend on
    // the file system (REQ-PROF-062, notes).
    assert_eq!(1, profiles.len());
    assert_eq!("First", profiles[0].title);
    assert!(log.any_contains("duplicated profile"));
}

fn user_folder<'a>(
    files: &'a FakeProfileFileSystem,
    directory: &Path,
    log: Option<&'a RecordingProfileLog>,
) -> UserProfileFolder<'a> {
    UserProfileFolder::new(
        files,
        directory,
        log.map(|log| log as &dyn zax_profiles::format::ProfileLog),
        zax_profiles::catalogue::ProfileOrigin::User,
    )
}

#[test]
fn req_prof_062_an_identifier_that_belongs_to_a_built_in_profile_is_ignored() {
    let files = FakeProfileFileSystem::new();
    let layout = UserDataLayout::with_default_names(Path::new(ROOT));
    files.add_file(
        &layout.profiles_directory().join("Clash.3dn"),
        &make_profile(Some("1"), Some("Mine")),
    );

    let log = RecordingProfileLog::new();
    let folder = user_folder(&files, layout.profiles_directory(), Some(&log));

    assert_eq!(
        folder.scan(&["1"]).unwrap(),
        [] as [zax_profiles::catalogue::UserProfile; 0]
    );
    assert!(log.any_contains("duplicated profile"));
}

#[test]
fn req_prof_062_an_unreadable_or_foreign_file_is_ignored_and_the_others_are_kept() {
    // AMD-037 changes the rule for the identifier-less file: it is kept, under a derived id.
    let files = FakeProfileFileSystem::new();
    let layout = UserDataLayout::with_default_names(Path::new(ROOT));
    let good = make_profile(Some("good"), Some("Good"));
    let directory = layout.profiles_directory();
    files.add_file(&directory.join("a-good.3dn"), &good);
    files.add_file(&directory.join("b-truncated.3dn"), &good[..good.len() / 2]);
    files.add_file(
        &directory.join("c-foreign.3dn"),
        &[0x0a, 0x03, 0x33, 0x64, 0x78],
    );
    files.add_file(
        &directory.join("d-noid.3dn"),
        &make_profile(None, Some("No identifier")),
    );

    let log = RecordingProfileLog::new();
    let folder = user_folder(&files, directory, Some(&log));
    let profiles = folder.scan(&[]).unwrap();

    assert_eq!(2, profiles.len());
    assert_eq!("good", profiles[0].id);
    assert_eq!("No identifier", profiles[1].title);
    assert!(log.any_contains(import_messages::NOT_A_3DN_FILE));
    assert!(log.any_contains("carries no identifier"));
}

#[test]
fn amd_037_the_derived_identifier_is_twelve_base64_characters_of_the_sha256() {
    // SHA-256("abc") starts with ba7816bf8f01cfea41...; its first 9 bytes in base64.
    assert_eq!("ungWv48Bz+pB", derived_id(b"abc"));
    assert_eq!(12, derived_id(&[]).len());
}

#[test]
fn amd_037_a_profile_without_identifier_gets_a_stable_derived_one_and_is_not_rewritten() {
    let files = FakeProfileFileSystem::new();
    let layout = UserDataLayout::with_default_names(Path::new(ROOT));
    let bytes = make_profile(None, Some("Sans id"));
    let path = layout.profiles_directory().join("Sans id.3dn");
    files.add_file(&path, &bytes);

    let log = RecordingProfileLog::new();
    let folder = user_folder(&files, layout.profiles_directory(), Some(&log));
    let first = folder.scan(&[]).unwrap();
    let again = user_folder(&files, layout.profiles_directory(), None)
        .scan(&[])
        .unwrap();

    assert_eq!(1, first.len());
    assert_eq!(derived_id(&bytes), first[0].id);
    assert_eq!(first, again);
    assert!(log.any_contains(&format!(
        "the profile carries no identifier; using \"{}\", derived from its content",
        first[0].id
    )));
    assert_eq!(bytes, files.read_all_bytes(&path).unwrap());
}

#[test]
fn amd_037_the_same_content_under_two_names_is_one_game() {
    let files = FakeProfileFileSystem::new();
    let layout = UserDataLayout::with_default_names(Path::new(ROOT));
    let bytes = make_profile(None, Some("Twin"));
    files.add_file(&layout.profiles_directory().join("A.3dn"), &bytes);
    files.add_file(&layout.profiles_directory().join("B.3dn"), &bytes);

    let folder = user_folder(&files, layout.profiles_directory(), None);
    let profiles = folder.scan(&[]).unwrap();

    assert_eq!(1, profiles.len());
    assert!(profiles[0].file_path.ends_with("A.3dn"));
}

#[test]
fn amd_037_a_copy_of_a_built_in_profile_adds_no_game_with_or_without_identifier() {
    let files = FakeProfileFileSystem::new();
    let layout = UserDataLayout::with_default_names(Path::new(ROOT));
    let without_id = make_profile(None, Some("Built-in"));
    let with_id = make_profile(Some("b1"), Some("Built-in too"));
    files.add_file(&layout.profiles_directory().join("A.3dn"), &without_id);
    files.add_file(&layout.profiles_directory().join("B.3dn"), &with_id);
    files.add_file(
        &layout.profiles_directory().join("C.3dn"),
        &make_profile(None, Some("Mine")),
    );

    let log = RecordingProfileLog::new();
    let folder = user_folder(&files, layout.profiles_directory(), Some(&log));
    let profiles = folder.scan_excluding(&[], &[without_id, with_id]).unwrap();

    assert_eq!(1, profiles.len());
    assert_eq!("Mine", profiles[0].title);
    assert!(log.any_contains("identical to a built-in profile"));
}

/// AMD-066 REQ-PROF-089, which replaces the refusal AMD-037 §2 kept: a chosen file with no `Id`
/// that is not one of the built-in profiles is accepted, copied byte for byte, and keyed by the
/// identifier derived from its content — the one the next reading of the folder gives it.
#[test]
fn req_prof_089_importing_a_profile_without_an_identifier_uses_its_derived_one() {
    let files = FakeProfileFileSystem::new();
    let layout = UserDataLayout::with_default_names(Path::new(ROOT));
    let source = Path::new("/downloads/Nameless.3dn");
    let bytes = make_profile(None, None);
    files.add_file(source, &bytes);

    let folder = user_folder(&files, layout.profiles_directory(), None);
    let result = folder.import(source, &[], &[], &[]).unwrap();

    assert_eq!(ProfileImportOutcome::Added, result.outcome);
    let imported = result.profile.expect("the imported profile");
    assert_eq!(derived_id(&bytes), imported.id);
    // « Titre. Inchangé : `Title`, sinon le nom du fichier sans extension » (AMD-037 §2).
    assert_eq!("Nameless", imported.title);

    // Copied byte for byte, and never rewritten.
    let written = layout.profiles_directory().join("Nameless.3dn");
    assert!(files.file_exists(&written));
    assert_eq!(bytes, files.read_all_bytes(&written).unwrap());

    // The identifier of the issue is the one the folder scan gives the copy.
    let scanned = folder.scan(&[]).unwrap();
    assert_eq!(1, scanned.len());
    assert_eq!(derived_id(&bytes), scanned[0].id);

    // « la bibliothèque a une entrée de plus » (AMD-066 §3, second criterion).
    let catalogue = catalogue_of(&files, &[]);
    let mut library = GameLibrary::build(&catalogue, &[], None, None);
    let before = library.entries().len();
    library.add_user_profile(&imported, None).unwrap();
    assert_eq!(before + 1, library.entries().len());
    assert!(library.find(&derived_id(&bytes)).is_some());
}

/// AMD-066 §3, third criterion: the same file imported again replaces its own entry, under the
/// same identifier; the library never gains a second one.
#[test]
fn req_prof_089_reimporting_the_same_identifier_less_file_replaces_its_entry() {
    let files = FakeProfileFileSystem::new();
    let layout = UserDataLayout::with_default_names(Path::new(ROOT));
    let source = Path::new("/downloads/Nameless.3dn");
    let bytes = make_profile(None, Some("Nameless"));
    files.add_file(source, &bytes);

    let folder = user_folder(&files, layout.profiles_directory(), None);
    folder.import(source, &[], &[], &[]).unwrap();
    let known = folder.scan(&[]).unwrap();
    let again = folder.import(source, &[], &[], &known).unwrap();

    assert_eq!(ProfileImportOutcome::Replaced, again.outcome);
    assert_eq!(derived_id(&bytes), again.profile.unwrap().id);
    assert_eq!(1, folder.scan(&[]).unwrap().len());
}

/// AMD-066 REQ-PROF-088: a chosen file whose **bytes** are those of a built-in profile is
/// « already built-in » and nothing is copied — with or without an `Id`. Neither of the two can
/// be recognised through `built_in_ids`, which holds the short catalogue identifiers.
#[test]
fn req_prof_088_importing_the_bytes_of_a_built_in_profile_imports_nothing() {
    let layout = UserDataLayout::with_default_names(Path::new(ROOT));
    for (name, bytes) in [
        ("Nameless.3dn", make_profile(None, Some("Built-in"))),
        (
            "Named.3dn",
            make_profile(Some("ABCDEFGHIJKL"), Some("Also")),
        ),
    ] {
        let files = FakeProfileFileSystem::new();
        let source = Path::new("/downloads").join(name);
        files.add_file(&source, &bytes);

        let folder = user_folder(&files, layout.profiles_directory(), None);
        let result = folder
            .import(&source, &[], std::slice::from_ref(&bytes), &[])
            .unwrap();

        assert_eq!(
            ProfileImportOutcome::AlreadyBuiltIn,
            result.outcome,
            "{name}"
        );
        assert!(result.profile.is_none(), "{name}");
        assert!(
            !files.file_exists(&layout.profiles_directory().join(name)),
            "{name}"
        );
    }
}

/// AMD-066 §3, first criterion, which is the reported case: `Contra.bytes` — one of the 45
/// built-in profiles of 2.0 that carry no `Id` (AMD-037 §1) — chosen as `Contra.3dn` against the
/// catalogue of `assets-neutral/` is « already built-in », and nothing lands in the user folder.
#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_prof_088_the_bytes_of_the_built_in_contra_are_already_in_the_catalogue() {
    let catalogue = ProfileCatalogue::load(
        &PhysicalProfileFileSystem,
        &neutral_assets::catalogue_file(),
        FULL_EDITION,
    )
    .unwrap();
    let contents = catalogue.built_in_contents(&PhysicalProfileFileSystem);
    let bytes = neutral_assets::read_profile("Contra.bytes");
    // The constat of AMD-066 §1: a sound 3dn of version "2" that carries no identifier.
    let header = format::load_header(&bytes, None, None).unwrap();
    assert!(header.has_acceptable_signature());
    assert!(!header.has_unsupported_version());
    assert!(!header.has_id(), "Contra.bytes carries an Id");

    let files = FakeProfileFileSystem::new();
    let layout = UserDataLayout::with_default_names(Path::new(ROOT));
    let source = Path::new("/downloads/Contra.3dn");
    files.add_file(source, &bytes);

    let folder = user_folder(&files, layout.profiles_directory(), None);
    let ids = catalogue.ids();
    let result = folder.import(source, &ids, &contents, &[]).unwrap();

    assert_eq!(ProfileImportOutcome::AlreadyBuiltIn, result.outcome);
    assert!(result.profile.is_none());
    assert_eq!(
        UserProfileFolder::list_profile_files(&files, layout.profiles_directory()).unwrap(),
        [] as [PathBuf; 0]
    );
}

/// AMD-066 REQ-PROF-090 and AMD-066 §3, fourth criterion: a truncated file and a malformed
/// protobuf keep the « obsolete or wrong profile format » text; a foreign signature keeps its
/// own. The absence of an identifier is no longer one of the causes.
#[test]
fn req_prof_090_an_unreadable_file_keeps_the_obsolete_format_message() {
    let layout = UserDataLayout::with_default_names(Path::new(ROOT));
    let whole = make_profile(Some("ABCDEFGHIJKL"), Some("Mine"));
    let truncated = whole[..whole.len() / 2].to_vec();
    for (name, bytes) in [
        ("Truncated.3dn", truncated),
        ("Malformed.3dn", vec![0xff, 0xff, 0xff, 0xff, 0xff]),
    ] {
        let files = FakeProfileFileSystem::new();
        let source = Path::new("/downloads").join(name);
        files.add_file(&source, &bytes);

        let folder = user_folder(&files, layout.profiles_directory(), None);
        let result = folder.import(&source, &[], &[], &[]).unwrap();

        assert_eq!(ProfileImportOutcome::Rejected, result.outcome, "{name}");
        assert_eq!(
            import_messages::obsolete_format(&source),
            result.message,
            "{name}"
        );
        assert!(
            !files.file_exists(&layout.profiles_directory().join(name)),
            "{name}"
        );
    }
}

#[test]
fn req_prof_063_importing_a_new_identifier_copies_the_file_and_adds_a_game() {
    let files = FakeProfileFileSystem::new();
    let layout = UserDataLayout::with_default_names(Path::new(ROOT));
    let source = Path::new("/downloads/Mine.3dn");
    files.add_file(source, &make_profile(Some("mine01"), Some("My Game")));

    let folder = user_folder(&files, layout.profiles_directory(), None);
    let result = folder.import(source, &["1", "47"], &[], &[]).unwrap();

    assert_eq!(ProfileImportOutcome::Added, result.outcome);
    assert_eq!("My Game", result.profile.unwrap().title);
    assert!(files.file_exists(&layout.profiles_directory().join("Mine.3dn")));

    // Importing the same file twice creates one game, not two.
    let known = folder.scan(&[]).unwrap();
    let again = folder.import(source, &["1", "47"], &[], &known).unwrap();
    assert_eq!(ProfileImportOutcome::Replaced, again.outcome);
    assert_eq!(1, folder.scan(&[]).unwrap().len());
}

#[test]
fn req_prof_063_importing_over_a_user_profile_replaces_its_file() {
    let files = FakeProfileFileSystem::new();
    let layout = UserDataLayout::with_default_names(Path::new(ROOT));
    let installed = layout.profiles_directory().join("Old.3dn");
    files.add_file(&installed, &make_profile(Some("same"), Some("Old title")));

    let source = Path::new("/downloads/New.3dn");
    files.add_file(source, &make_profile(Some("same"), Some("New title")));

    let folder = user_folder(&files, layout.profiles_directory(), None);
    let known = folder.scan(&[]).unwrap();
    let result = folder.import(source, &[], &[], &known).unwrap();

    assert_eq!(ProfileImportOutcome::Replaced, result.outcome);
    assert!(!files.file_exists(&installed));
    assert!(files.file_exists(&layout.profiles_directory().join("New.3dn")));
    let scanned = folder.scan(&[]).unwrap();
    assert_eq!(1, scanned.len());
    assert_eq!("New title", scanned[0].title);
}

#[test]
fn req_prof_063_importing_the_identifier_of_a_built_in_profile_does_nothing() {
    let files = FakeProfileFileSystem::new();
    let layout = UserDataLayout::with_default_names(Path::new(ROOT));
    let source = Path::new("/downloads/Clash.3dn");
    files.add_file(source, &make_profile(Some("1"), Some("Not Arkanoid")));

    let folder = user_folder(&files, layout.profiles_directory(), None);
    let result = folder.import(source, &["1"], &[], &[]).unwrap();

    assert_eq!(ProfileImportOutcome::AlreadyBuiltIn, result.outcome);
    assert_eq!(
        files
            .list_files(layout.profiles_directory(), UserProfileFolder::EXTENSION)
            .unwrap(),
        [] as [std::path::PathBuf; 0]
    );
}

#[test]
fn req_prof_063_importing_a_file_that_is_not_a_profile_is_refused() {
    let files = FakeProfileFileSystem::new();
    let layout = UserDataLayout::with_default_names(Path::new(ROOT));
    let source = Path::new("/downloads/Foreign.3dn");
    files.add_file(source, &[0x0a, 0x03, 0x33, 0x64, 0x78]);

    let folder = user_folder(&files, layout.profiles_directory(), None);
    let result = folder.import(source, &[], &[], &[]).unwrap();

    assert_eq!(ProfileImportOutcome::Rejected, result.outcome);
    assert_eq!(import_messages::NOT_A_3DN_FILE, result.message);
}

/// AMD-053 REQ-UI-212, second row of the table of causes, with the form AMD-056 §2 gives `<v>`:
/// a profile whose `Ver` reduces to "1" is refused, and the version shown is the sanitised one.
#[test]
fn req_ui_212_a_profile_of_an_unsupported_version_names_its_sanitised_version() {
    let files = FakeProfileFileSystem::new();
    let layout = UserDataLayout::with_default_names(Path::new(ROOT));
    let mut profile = GameProfile::create_empty();
    profile.id = Some("ABCDEFGHIJKL".to_owned());
    profile.title = Some("Old".to_owned());
    profile.version_text = Some("v1".to_owned());
    let source = Path::new("/downloads/Old.3dn");
    files.add_file(source, &format::save(&profile, FieldPresence::All));

    let folder = user_folder(&files, layout.profiles_directory(), None);
    let result = folder.import(source, &[], &[], &[]).unwrap();

    assert_eq!(ProfileImportOutcome::Rejected, result.outcome);
    assert_eq!("3dn format version 1  is not supported", result.message);
    assert!(!files.file_exists(&layout.profiles_directory().join("Old.3dn")));
}

/// AMD-056 REQ-UI-214: inspecting reads the file and says what an import would do without
/// writing anything; applying the plan is what writes.
#[test]
fn req_ui_214_inspecting_an_import_writes_nothing_and_applying_it_does() {
    let files = FakeProfileFileSystem::new();
    let layout = UserDataLayout::with_default_names(Path::new(ROOT));
    let source = Path::new("/downloads/Mine.3dn");
    files.add_file(source, &make_profile(Some("ABCDEFGHIJKL"), Some("Mine")));
    let destination = layout.profiles_directory().join("Mine.3dn");

    let folder = user_folder(&files, layout.profiles_directory(), None);
    let plan = folder.inspect(source, &[], &[], &[]).unwrap();

    assert_eq!(ProfileImportOutcome::Added, plan.outcome);
    assert!(plan.writes());
    assert_eq!("ABCDEFGHIJKL", plan.id);
    assert_eq!("Mine", plan.title);
    assert_eq!(destination, plan.destination);
    assert!(plan.replaced.is_none());
    assert!(!files.file_exists(&destination), "inspecting wrote a file");

    let result = folder.apply(&plan).unwrap();

    assert_eq!(ProfileImportOutcome::Added, result.outcome);
    assert!(files.file_exists(&destination));
}

/// AMD-056 REQ-UI-214: the plan of a replacement names the profile it would take over, which is
/// what the confirmation dialogue needs; a plan that is only inspected replaces nothing.
#[test]
fn req_ui_214_a_replacement_plan_names_the_profile_it_would_take_over() {
    let files = FakeProfileFileSystem::new();
    let layout = UserDataLayout::with_default_names(Path::new(ROOT));
    let existing = layout.profiles_directory().join("Theirs.3dn");
    files.add_file(
        &existing,
        &make_profile(Some("ABCDEFGHIJKL"), Some("Theirs")),
    );
    let source = Path::new("/downloads/Mine.3dn");
    files.add_file(source, &make_profile(Some("ABCDEFGHIJKL"), Some("Mine")));

    let folder = user_folder(&files, layout.profiles_directory(), None);
    let known = folder.scan(&[]).unwrap();
    let plan = folder.inspect(source, &[], &[], &known).unwrap();

    assert_eq!(ProfileImportOutcome::Replaced, plan.outcome);
    assert_eq!(
        Some(existing.clone()),
        plan.replaced.as_ref().map(|p| p.file_path.clone())
    );
    assert_eq!(
        Some("Theirs"),
        plan.replaced.as_ref().map(|p| p.title.as_str())
    );
    // Nothing happened yet: the old file is there and the new one is not.
    assert!(files.file_exists(&existing));
    assert!(!files.file_exists(&layout.profiles_directory().join("Mine.3dn")));

    folder.apply(&plan).unwrap();

    assert!(!files.file_exists(&existing));
    assert!(files.file_exists(&layout.profiles_directory().join("Mine.3dn")));
}

/// AMD-056 REQ-UI-214: the plan of a built-in identifier carries the title the « Already in
/// Zaxnes » dialogue names, and applying it writes nothing.
#[test]
fn req_ui_214_a_built_in_identifier_plan_carries_the_title_of_the_file() {
    let files = FakeProfileFileSystem::new();
    let layout = UserDataLayout::with_default_names(Path::new(ROOT));
    let source = Path::new("/downloads/x.3dn");
    files.add_file(source, &make_profile(Some("10"), Some("Foo")));

    let folder = user_folder(&files, layout.profiles_directory(), None);
    let plan = folder.inspect(source, &["10"], &[], &[]).unwrap();

    assert_eq!(ProfileImportOutcome::AlreadyBuiltIn, plan.outcome);
    assert!(!plan.writes());
    assert_eq!("Foo", plan.title);

    folder.apply(&plan).unwrap();

    assert_eq!(
        files
            .list_files(layout.profiles_directory(), UserProfileFolder::EXTENSION)
            .unwrap(),
        [] as [std::path::PathBuf; 0]
    );
}

#[test]
fn req_prof_064_the_library_lists_every_profile_and_remembers_the_chosen_rom() {
    let files = FakeProfileFileSystem::new();
    let catalogue = catalogue_of(
        &files,
        &[
            ("1", "Arkanoid", "profiles/ark.bytes"),
            ("47", "2D Mode", "profiles/empty.bytes"),
        ],
    );

    let mut library = GameLibrary::build(&catalogue, &[], None, None);

    assert_eq!(2, library.entries().len());
    assert!(!library.entries()[0].has_rom());
    assert_eq!(0, library.entries()[0].last_played);

    assert!(library.set_rom_path("1", "/roms/test.nes"));
    assert!(library.mark_played("1", 1_760_950_000));
    assert!(!library.set_rom_path("nope", "/roms/test.nes"));

    let arkanoid = library.find("1").unwrap();
    assert_eq!("/roms/test.nes", arkanoid.rom_path);
    assert_eq!(1_760_950_000, arkanoid.last_played);
    assert_eq!(GameProfileSource::BuiltIn, arkanoid.source());
    assert!(!arkanoid.can_delete());
}

#[test]
fn req_prof_061_the_2d_mode_entry_keeps_its_name_whatever_the_custom_title() {
    let files = FakeProfileFileSystem::new();
    let catalogue = catalogue_of(&files, &[("47", "2D Mode", "profiles/empty.bytes")]);

    let mut library = GameLibrary::build(&catalogue, &[], None, None);
    let two_d = library.find_mut("47").unwrap();
    two_d.title = "My 2D games".to_owned();

    assert!(two_d.is_2d_mode());
    assert_eq!("2D Mode", two_d.display_title());
    assert!(!two_d.can_delete());
}

#[test]
fn req_prof_065_only_a_game_from_a_user_profile_can_be_removed() {
    let files = FakeProfileFileSystem::new();
    let layout = UserDataLayout::with_default_names(Path::new(ROOT));
    let catalogue = catalogue_of(&files, &[("1", "Arkanoid", "profiles/ark.bytes")]);

    let user_file = layout.profiles_directory().join("Mine.3dn");
    files.add_file(&user_file, &make_profile(Some("mine01"), Some("Mine")));
    let folder = user_folder(&files, layout.profiles_directory(), None);
    let user_profiles = folder.scan(&catalogue.ids()).unwrap();

    let mut library = GameLibrary::build(&catalogue, &user_profiles, None, None);

    assert!(!library.remove("1"));
    assert!(library.remove("mine01"));
    assert_eq!(1, library.entries().len());

    folder.delete(&user_profiles[0]).unwrap();
    assert!(!files.file_exists(&user_file));
}

#[test]
fn req_pers_030_a_built_in_profile_wins_over_a_user_profile_of_the_same_identifier() {
    let files = FakeProfileFileSystem::new();
    let catalogue = catalogue_of(&files, &[("1", "Arkanoid", "profiles/ark.bytes")]);

    let log = RecordingProfileLog::new();
    let library = GameLibrary::build(
        &catalogue,
        &[UserProfile {
            id: "1".to_owned(),
            title: "Mine".to_owned(),
            file_path: PathBuf::from("/x/mine.3dn"),
            header: ProfileHeader::default(),
            origin: zax_profiles::catalogue::ProfileOrigin::User,
        }],
        None,
        Some(&log),
    );

    assert_eq!(1, library.entries().len());
    let entry = &library.entries()[0];
    assert_eq!("Arkanoid", entry.title);
    assert_eq!(GameProfileSource::BuiltIn, entry.source());
    assert!(log.any_contains("duplicated profile identifier"));
}

#[test]
fn req_pers_035_the_library_file_is_reconciled_with_the_available_profiles() {
    let files = FakeProfileFileSystem::new();
    let layout = UserDataLayout::with_default_names(Path::new(ROOT));
    let catalogue = catalogue_of(
        &files,
        &[
            ("1", "Arkanoid", "profiles/ark.bytes"),
            ("2", "Balloon", "profiles/bf.bytes"),
        ],
    );

    let store = GameLibraryStore::new(&files, None);
    let mut persisted = GameLibraryFile {
        games: vec![
            GameLibraryRecord {
                id: Some("1".to_owned()),
                title: Some("My Arkanoid".to_owned()),
                rom_path: Some("/a.nes".to_owned()),
                last_played: 7,
                cover: None,
                favorite: false,
            },
            GameLibraryRecord {
                id: Some("ghost".to_owned()),
                title: Some("Gone".to_owned()),
                rom_path: Some("/g.nes".to_owned()),
                ..GameLibraryRecord::default()
            },
        ],
        ..GameLibraryFile::default()
    };
    store.save(layout.library_file(), &mut persisted).unwrap();

    let log = RecordingProfileLog::new();
    let reloaded = store.load(layout.library_file(), OffsetDateTime::UNIX_EPOCH);
    let library = GameLibrary::build(&catalogue, &[], reloaded.as_ref(), Some(&log));

    assert_eq!(2, library.entries().len());
    assert_eq!("My Arkanoid", library.find("1").unwrap().title);
    assert_eq!("/a.nes", library.find("1").unwrap().rom_path);
    assert_eq!(7, library.find("1").unwrap().last_played);

    // A blank entry for the profile the file did not know, and the stale one is gone.
    assert_eq!("Balloon", library.find("2").unwrap().title);
    assert_eq!("", library.find("2").unwrap().rom_path);
    assert!(library.find("ghost").is_none());
    assert!(log.any_contains("matches no available profile"));
}

#[test]
fn req_prof_064_the_rom_association_survives_a_save_and_a_reload() {
    let files = FakeProfileFileSystem::new();
    let layout = UserDataLayout::with_default_names(Path::new(ROOT));
    let catalogue = catalogue_of(&files, &[("1", "Arkanoid", "profiles/ark.bytes")]);

    let store = GameLibraryStore::new(&files, None);
    let mut library = GameLibrary::build(&catalogue, &[], None, None);
    library.set_rom_path("1", "/roms/nestest.nes");
    library.mark_played("1", 1_760_950_000);
    store
        .save(layout.library_file(), &mut library.to_file())
        .unwrap();

    let loaded = store.load(layout.library_file(), OffsetDateTime::UNIX_EPOCH);
    let reloaded = GameLibrary::build(&catalogue, &[], loaded.as_ref(), None);

    assert_eq!("/roms/nestest.nes", reloaded.find("1").unwrap().rom_path);
    assert_eq!(1_760_950_000, reloaded.find("1").unwrap().last_played);
    assert_eq!(
        GameLibraryFile::CURRENT_SCHEMA_VERSION,
        store
            .load(layout.library_file(), OffsetDateTime::UNIX_EPOCH)
            .unwrap()
            .schema_version
    );
}

/// AMD-053 REQ-UI-210 and §3 point 11: a favourite mark survives a save and a reload, only its
/// own entry carries `"favorite": true`, and the file is written at schema 2.
#[test]
fn req_ui_210_a_favorite_mark_survives_a_save_and_a_reload() {
    let files = FakeProfileFileSystem::new();
    let layout = UserDataLayout::with_default_names(Path::new(ROOT));
    let catalogue = catalogue_of(
        &files,
        &[
            ("1", "Arkanoid", "profiles/ark.bytes"),
            ("2", "Balloon", "profiles/bf.bytes"),
        ],
    );

    let store = GameLibraryStore::new(&files, None);
    let mut library = GameLibrary::build(&catalogue, &[], None, None);
    library.set_favorite("1", true);
    store
        .save(layout.library_file(), &mut library.to_file())
        .unwrap();

    let text = files.read_all_text(layout.library_file()).unwrap();
    assert!(text.contains("\"schemaVersion\": 2"), "{text}");
    assert_eq!(1, text.matches("\"favorite\": true").count(), "{text}");

    let loaded = store.load(layout.library_file(), OffsetDateTime::UNIX_EPOCH);
    assert_eq!(None, store.migrated_from_version());
    let reloaded = GameLibrary::build(&catalogue, &[], loaded.as_ref(), None);

    assert!(reloaded.find("1").unwrap().favorite);
    assert!(!reloaded.find("2").unwrap().favorite);
}

/// AMD-053 §3 point 11 with REQ-PERS-024 and REQ-PERS-088: a schema-1 file is read with
/// `favorite` false everywhere, a `library.json.v1.bak` copy is kept, and the next write is at
/// schema 2.
#[test]
fn req_pers_024_a_schema_1_library_is_migrated_and_its_copy_is_kept() {
    let files = FakeProfileFileSystem::new();
    let layout = UserDataLayout::with_default_names(Path::new(ROOT));
    let catalogue = catalogue_of(&files, &[("1", "Arkanoid", "profiles/ark.bytes")]);
    let version_1 = r#"{
 "schemaVersion": 1,
 "notice": "Do not edit the id field.",
 "games": [
  { "id": "1", "title": "Arkanoid", "romPath": "/a.nes", "lastPlayed": 7 }
 ]
}"#;
    files.add_text(layout.library_file(), version_1);

    let store = GameLibraryStore::new(&files, None);
    let loaded = store.load(layout.library_file(), OffsetDateTime::UNIX_EPOCH);

    assert_eq!(Some(1), store.migrated_from_version());
    let backup = PathBuf::from(format!("{}.v1.bak", layout.library_file().display()));
    assert_eq!(version_1, files.read_all_text(&backup).unwrap());

    let library = GameLibrary::build(&catalogue, &[], loaded.as_ref(), None);
    assert!(!library.find("1").unwrap().favorite);
    assert_eq!("/a.nes", library.find("1").unwrap().rom_path);

    store
        .save(layout.library_file(), &mut library.to_file())
        .unwrap();
    assert_eq!(
        GameLibraryFile::CURRENT_SCHEMA_VERSION,
        store
            .load(layout.library_file(), OffsetDateTime::UNIX_EPOCH)
            .unwrap()
            .schema_version
    );
}

#[test]
fn req_pers_037_an_unreadable_library_is_set_aside_and_the_library_is_rebuilt() {
    let files = FakeProfileFileSystem::new();
    let layout = UserDataLayout::with_default_names(Path::new(ROOT));
    files.add_text(layout.library_file(), "{{{ not json");

    let log = RecordingProfileLog::new();
    let store = GameLibraryStore::new(&files, Some(&log));
    let loaded = store.load(layout.library_file(), datetime!(2026-10-20 18:30:05 UTC));

    assert!(loaded.is_none());
    assert!(files.file_exists(&PathBuf::from(format!(
        "{}.corrupt-20261020-183005",
        layout.library_file().display()
    ))));
    assert!(log.any_contains("unreadable library"));
}

#[test]
fn req_pers_025_the_rom_paths_of_an_original_rom_json_can_be_imported() {
    let files = FakeProfileFileSystem::new();
    let catalogue = catalogue_of(&files, &[("1", "Arkanoid", "profiles/ark.bytes")]);

    let rom_json = r#"
        {
          "Warning": "Don't touch id parameter,  you will regret it",
          "Items": [
            { "id": "1", "romPath": "D:\\roms\\arkanoid.nes", "title": "Arkanoid", "lastTime": 1760950000 },
            { "id": "999", "romPath": "D:\\roms\\gone.nes", "title": "Gone", "lastTime": 1 }
          ]
        }"#;
    let rom_file = Path::new(ROOT).join("rom.json");
    files.add_text(&rom_file, rom_json);

    let store = GameLibraryStore::new(&files, None);
    let records = store.load_rom_paths(&rom_file);
    let mut library = GameLibrary::build(&catalogue, &[], None, None);

    assert_eq!(2, records.len());
    assert_eq!(1, library.apply_rom_paths(&records));
    assert_eq!(r"D:\roms\arkanoid.nes", library.find("1").unwrap().rom_path);
    assert_eq!(1_760_950_000, library.find("1").unwrap().last_played);
}

#[test]
fn req_pers_033_the_library_sorts_by_title_or_by_last_played() {
    let files = FakeProfileFileSystem::new();
    let catalogue = catalogue_of(
        &files,
        &[
            ("1", "Zelda", "profiles/z.bytes"),
            ("2", "Arkanoid", "profiles/a.bytes"),
            ("3", "Metroid", "profiles/m.bytes"),
        ],
    );

    let mut library = GameLibrary::build(&catalogue, &[], None, None);
    library.mark_played("1", 300);
    library.mark_played("3", 500);

    assert_eq!(
        ["Arkanoid", "Metroid", "Zelda"],
        titles(&library.sorted(LibrarySortOption::Alphabetical)).as_slice()
    );
    assert_eq!(
        ["Metroid", "Zelda", "Arkanoid"],
        titles(&library.sorted(LibrarySortOption::Recent)).as_slice()
    );
}

/// AMD-053 §3 point 8, REQ-PERS-033 amended: « Favorites first » puts the favourites first, then
/// the others, each group by title.
#[test]
fn req_pers_033_favorites_first_orders_each_group_by_title() {
    let files = FakeProfileFileSystem::new();
    let catalogue = catalogue_of(
        &files,
        &[
            ("1", "Zelda", "profiles/z.bytes"),
            ("2", "Arkanoid", "profiles/a.bytes"),
            ("3", "Metroid", "profiles/m.bytes"),
            ("4", "Balloon", "profiles/b.bytes"),
        ],
    );

    let mut library = GameLibrary::build(&catalogue, &[], None, None);
    assert!(library.set_favorite("1", true));
    assert!(library.set_favorite("3", true));
    assert!(!library.set_favorite("nope", true));

    assert_eq!(
        ["Metroid", "Zelda", "Arkanoid", "Balloon"],
        titles(&library.sorted(LibrarySortOption::FavoritesFirst)).as_slice()
    );
    // The value of ui.sort_option: 0 A-Z, 1 Recent, 2 Favorites first; the slot lists ignore 2.
    assert_eq!(
        LibrarySortOption::FavoritesFirst,
        LibrarySortOption::of_setting(2)
    );
    assert_eq!(
        LibrarySortOption::Alphabetical,
        LibrarySortOption::of_setting_for_slots(2)
    );
}

/// AMD-053 REQ-UI-210: two favourites out of ten, the filter ticked gives exactly the two and
/// unticked gives the ten.
#[test]
fn req_ui_210_the_favorites_filter_keeps_exactly_the_favorites() {
    let files = FakeProfileFileSystem::new();
    let entries: Vec<(String, String, String)> = (0..10)
        .map(|index| {
            (
                index.to_string(),
                format!("Game {index}"),
                format!("profiles/g{index}.bytes"),
            )
        })
        .collect();
    let borrowed: Vec<(&str, &str, &str)> = entries
        .iter()
        .map(|(id, title, file)| (id.as_str(), title.as_str(), file.as_str()))
        .collect();
    let catalogue = catalogue_of(&files, &borrowed);

    let mut library = GameLibrary::build(&catalogue, &[], None, None);
    library.set_favorite("3", true);
    library.set_favorite("7", true);

    assert_eq!(
        ["Game 3", "Game 7"],
        titles(&library.enumerate(LibrarySortOption::Alphabetical, "", true)).as_slice()
    );
    assert_eq!(
        10,
        library
            .enumerate(LibrarySortOption::Alphabetical, "", false)
            .len()
    );

    // `favorite.toggle` flips the mark and reports the new value.
    assert_eq!(Some(false), library.toggle_favorite("3"));
    assert_eq!(Some(true), library.toggle_favorite("3"));
    assert_eq!(None, library.toggle_favorite("nope"));
}

/// AMD-053 REQ-UI-211: the search keeps the titles that contain the typed text, case and accents
/// ignored, and combines with the favourites filter — an entry is kept when it satisfies both.
#[test]
fn req_ui_211_the_search_ignores_case_and_accents_and_combines_with_the_filter() {
    let files = FakeProfileFileSystem::new();
    let catalogue = catalogue_of(
        &files,
        &[
            ("1", "Super Mario Bros.", "profiles/smb.bytes"),
            ("2", "MARIO Kart", "profiles/mk.bytes"),
            ("3", "Pokémon", "profiles/pk.bytes"),
            ("4", "Metroid", "profiles/mt.bytes"),
        ],
    );

    let mut library = GameLibrary::build(&catalogue, &[], None, None);

    assert_eq!(
        ["MARIO Kart", "Super Mario Bros."],
        kept(&library, "mario", false).as_slice()
    );
    assert_eq!(["Pokémon"], kept(&library, "pokemon", false).as_slice());
    assert_eq!(
        ["Super Mario Bros."],
        kept(&library, "bros", false).as_slice()
    );
    // A blank text excludes nothing.
    assert_eq!(4, kept(&library, "", false).len());
    assert_eq!(4, kept(&library, "   ", false).len());

    library.set_favorite("2", true);
    library.set_favorite("4", true);
    assert_eq!(["MARIO Kart"], kept(&library, "mario", true).as_slice());
}

#[test]
fn req_arch_033_the_user_data_layout_is_built_from_an_injected_root() {
    let files = FakeProfileFileSystem::new();
    let root = Path::new(ROOT);
    let layout = UserDataLayout::with_default_names(root);

    assert_eq!(root.join("profiles"), layout.profiles_directory());
    assert_eq!(root.join("library.json"), layout.library_file());
    assert_eq!(root.join("covers"), layout.covers_directory());

    layout.ensure_directories(&files).unwrap();
    assert!(files.directory_exists(layout.profiles_directory()));
    assert!(files.directory_exists(layout.covers_directory()));

    // Q-008: the original's folder name is available for whoever wants it.
    let original = UserDataLayout::new(root, "CustomProfiles");
    assert_eq!(root.join("CustomProfiles"), original.profiles_directory());
}

#[test]
#[ignore = "needs ZAXNES_PROFILES (the built-in profiles)"]
fn req_acc_010_the_whole_library_can_be_built_from_the_real_catalogue() {
    let catalogue = ProfileCatalogue::load(
        &PhysicalProfileFileSystem,
        &neutral_assets::catalogue_file(),
        FULL_EDITION,
    )
    .unwrap();

    let log = RecordingProfileLog::new();
    let library = GameLibrary::build(&catalogue, &[], None, Some(&log));

    assert_eq!(101, library.entries().len());
    assert!(log.warnings.borrow().is_empty());
    assert!(log.errors.borrow().is_empty());
    assert_eq!(
        1,
        library
            .entries()
            .iter()
            .filter(|entry| entry.is_2d_mode())
            .count()
    );
}
