//! A 2.1 profile replaces the 2.0-or-older profile of the same game (AMD-065 REQ-PROF-085 to
//! REQ-PROF-087).

use std::path::{Path, PathBuf};

use zax_profiles::catalogue::{
    built_in, normalised_name, GameIdentity, ProfileCatalogue, ProfileFileSystem,
    ProfileGeneration, ProfileOrigin, UserProfile, UserProfileFolder, FULL_EDITION,
};
use zax_profiles::format::{self, ProfileLog};
use zax_profiles::layout::FieldPresence;
use zax_profiles::library::{GameLibrary, GameLibraryEntry, GameProfileSource, RomPathRecord};
use zax_profiles::profile::GameProfile;

use super::common::fake_fs::FakeProfileFileSystem;
use super::common::neutral_assets;
use super::common::physical_fs::PhysicalProfileFileSystem;
use super::common::recording_log::RecordingProfileLog;
use super::common::synthetic_hash_profile::EnvelopeSpec;

/// Roots of the in-memory file system of these tests: nothing is read from or written to a real
/// disk, and no path of any machine appears here (lot PUB-1).
const ASSETS: &str = "/fake/assets-neutral";
const USER: &str = "/fake/AppData/Local/Zaxnes/profiles";

fn user_directory() -> PathBuf {
    PathBuf::from(USER)
}

fn folder<'a>(
    files: &'a FakeProfileFileSystem,
    log: Option<&'a RecordingProfileLog>,
) -> UserProfileFolder<'a> {
    UserProfileFolder::new(
        files,
        &PathBuf::from(USER),
        log.map(|log| log as &dyn ProfileLog),
        ProfileOrigin::User,
    )
}

/// A 3dn profile (generation earlier than 2.1), with or without an `Id`.
fn plain_profile(id: Option<&str>, title: Option<&str>) -> Vec<u8> {
    let mut profile = GameProfile::create_empty();
    profile.id = id.map(str::to_owned);
    profile.title = title.map(str::to_owned);
    format::save(&profile, FieldPresence::All)
}

/// A 3dh profile (generation 2.1): only its envelope matters here, which is what the folder scan
/// and the library read (REQ-PROF-073).
fn hash_profile(id: &str, title: Option<&str>) -> Vec<u8> {
    EnvelopeSpec {
        id: Some(id.to_owned()),
        title: title.map(str::to_owned),
        ..EnvelopeSpec::default()
    }
    .encode()
}

fn catalogue_json(entries: &[(&str, &str, &str)]) -> String {
    let full: Vec<String> = entries
        .iter()
        .map(|(id, title, file)| {
            format!(
                r#"{{"id":"{id}","title":"{title}","publisher":"","description":"","author":"","file":"{file}"}}"#
            )
        })
        .collect();
    format!(
        r#"{{"editions":{{"full":[{}],"demo":[]}}}}"#,
        full.join(",")
    )
}

/// A catalogue of the given entries, whose profile files hold `bytes`.
fn catalogue_of(
    files: &FakeProfileFileSystem,
    entries: &[(&str, &str, &str, Vec<u8>)],
) -> ProfileCatalogue {
    let described: Vec<(&str, &str, &str)> = entries
        .iter()
        .map(|(id, title, file, _)| (*id, *title, *file))
        .collect();
    let path = Path::new(ASSETS).join("profiles.json");
    files.add_text(&path, &catalogue_json(&described));
    for (_, _, file, bytes) in entries {
        files.add_file(&Path::new(ASSETS).join(file), bytes);
    }
    ProfileCatalogue::load(files, &path, FULL_EDITION).unwrap()
}

/// The library built as the application builds it (AMD-065 REQ-PROF-086 point 1 needs the
/// built-in files to have been read).
fn library_of(
    files: &FakeProfileFileSystem,
    catalogue: &ProfileCatalogue,
    log: Option<&RecordingProfileLog>,
) -> GameLibrary {
    let built_in = catalogue.read_built_in_profiles(files);
    let profiles = folder(files, log)
        .scan_excluding(&catalogue.ids(), built_in.contents())
        .unwrap();
    GameLibrary::build_with_built_in_profiles(
        catalogue,
        &built_in,
        &profiles,
        None,
        log.map(|log| log as &dyn ProfileLog),
    )
}

fn ids(library: &GameLibrary) -> Vec<&str> {
    library.entries().iter().map(GameLibraryEntry::id).collect()
}

// --- REQ-PROF-085 ------------------------------------------------------------------------------

/// REQ-PROF-085: the generation is read at the content. The import from an installation writes a
/// 3dh under the name `.3dn` (AMD-062 §8), and it must still replace the 3dn of the same game.
#[test]
fn req_prof_085_a_2_1_profile_named_3dn_still_replaces_the_profile_of_the_same_game() {
    let files = FakeProfileFileSystem::new();
    let catalogue = catalogue_of(
        &files,
        &[(
            "20",
            "Hogan's Alley",
            "profiles/Hogan_s Alley.bytes",
            plain_profile(None, None),
        )],
    );
    files.add_file(
        &user_directory().join("Hogan's Alley.3dn"),
        &hash_profile("3348a6f46bb7", None),
    );

    let library = library_of(&files, &catalogue, None);

    assert_eq!(vec!["20"], ids(&library));
    assert_eq!(
        ProfileGeneration::TwoPointOne,
        library.find("20").unwrap().generation()
    );
    assert!(library
        .find("20")
        .unwrap()
        .profile_path()
        .ends_with("Hogan's Alley.3dn"));
}

/// REQ-PROF-085, the other way round: a 3dn named `.3dh` is of an earlier generation and replaces
/// nothing.
#[test]
fn req_prof_085_a_3dn_named_3dh_is_not_of_generation_2_1() {
    let files = FakeProfileFileSystem::new();
    let catalogue = catalogue_of(
        &files,
        &[(
            "1",
            "Arkanoid",
            "profiles/Arkanoid (U).bytes",
            plain_profile(None, None),
        )],
    );
    files.add_file(
        &user_directory().join("Arkanoid (U).3dh"),
        &plain_profile(Some("ark-user-001"), Some("Arkanoid mine")),
    );

    let library = library_of(&files, &catalogue, None);

    // Two entries, as AMD-039 §5 already had them: nothing of generation 2.1 is involved.
    assert_eq!(vec!["1", "ark-user-001"], ids(&library));
}

// --- REQ-PROF-086 ------------------------------------------------------------------------------

/// REQ-PROF-086 point 1: the same `Id` read in the file is the same game, whatever the file name.
/// This is the case of the 56 built-in profiles that carry an `Id`.
#[test]
fn req_prof_086_the_same_file_id_pairs_a_2_1_profile_with_the_built_in() {
    let files = FakeProfileFileSystem::new();
    let catalogue = catalogue_of(
        &files,
        &[(
            "37",
            "Super Mario Bros",
            "profiles/Super Mario Bros.bytes",
            plain_profile(Some("Jhj6E7iOysxT"), None),
        )],
    );
    files.add_file(
        &user_directory().join("smb-renamed.3dn"),
        &hash_profile("Jhj6E7iOysxT", Some("Super Mario Bros. 2.1")),
    );

    let built_in = catalogue.read_built_in_profiles(&files);
    let profiles = folder(&files, None)
        .scan_excluding(&catalogue.ids(), built_in.contents())
        .unwrap();
    assert_eq!(1, profiles.len());

    // The `Id` of a built-in profile is in its **file**, not in the catalogue: without having
    // read the files, only the name can pair the two, and here the names differ.
    let without = GameLibrary::build(&catalogue, &profiles, None, None);
    assert_eq!(vec!["37", "Jhj6E7iOysxT"], ids(&without));

    let with =
        GameLibrary::build_with_built_in_profiles(&catalogue, &built_in, &profiles, None, None);
    assert_eq!(vec!["37"], ids(&with));
    assert_eq!("Super Mario Bros. 2.1", with.find("37").unwrap().title);
}

/// REQ-PROF-086 point 2: the profile name alone, the apostrophe counting as `_`. This is the
/// case of the 45 built-in profiles without an `Id`, and the one AMD-065 §3 names.
#[test]
fn req_prof_086_the_profile_name_pairs_hogans_alley_across_the_apostrophe() {
    let files = FakeProfileFileSystem::new();
    let catalogue = catalogue_of(
        &files,
        &[(
            "20",
            "Hogan's Alley",
            "profiles/Hogan_s Alley.bytes",
            plain_profile(None, None),
        )],
    );
    files.add_file(
        &user_directory().join("Hogan's Alley.3dh"),
        &hash_profile("3348a6f46bb7", None),
    );

    let library = library_of(&files, &catalogue, None);

    assert_eq!(vec!["20"], ids(&library));
    // Both halves of the rule, on the names of the two installations.
    assert_eq!(
        normalised_name(Path::new("profiles/Hogan_s Alley.bytes")),
        normalised_name(Path::new("u/HOGAN'S ALLEY.3dh"))
    );
}

/// REQ-PROF-086 point 1 excludes a derived identifier (AMD-037): the two formats of one game have
/// different bytes, so their derived identifiers differ and must never be read as an `Id`.
#[test]
fn req_prof_086_a_derived_identifier_is_not_a_file_id() {
    let files = FakeProfileFileSystem::new();
    let bytes = plain_profile(None, Some("Nameless"));
    files.add_file(&user_directory().join("Nameless.3dn"), &bytes);

    let profiles = folder(&files, None).scan(&[]).unwrap();
    assert_eq!(1, profiles.len());
    assert!(!profiles[0].id.is_empty(), "it got a derived identifier");
    assert_eq!("", GameIdentity::of_user(&profiles[0]).file_id());
}

/// REQ-PROF-086: the generic entries of the catalogue (Q-065) have neither an `Id` nor a file and
/// designate no game but themselves — a 2.1 profile must not swallow « 2D Mode ».
#[test]
fn req_prof_086_the_generic_catalogue_entries_designate_no_game() {
    let files = FakeProfileFileSystem::new();
    let catalogue = built_in::create();
    files.add_file(
        &user_directory().join("Mine.3dn"),
        &hash_profile("mine-2-1-0001", Some("Mine")),
    );

    let built_in = catalogue.read_built_in_profiles(&files);
    assert_eq!(0, built_in.contents().len(), "no built-in profile file");
    let profiles = folder(&files, None)
        .scan_excluding(&catalogue.ids(), built_in.contents())
        .unwrap();
    let library =
        GameLibrary::build_with_built_in_profiles(&catalogue, &built_in, &profiles, None, None);

    assert_eq!(
        vec!["47", "3d", "mine-2-1-0001"],
        ids(&library),
        "the two generic entries and the 2.1 profile"
    );
}

// --- REQ-PROF-087 ------------------------------------------------------------------------------

/// REQ-PROF-087: the entry keeps the identifier of the entry it replaced, so that `rom.json`
/// (REQ-PROF-064), the saves and the cover stay attached to the game.
#[test]
fn req_prof_087_the_entry_keeps_the_replaced_identifier_so_rom_json_stays_attached() {
    let files = FakeProfileFileSystem::new();
    let catalogue = catalogue_of(
        &files,
        &[(
            "1",
            "Arkanoid",
            "profiles/Arkanoid (U).bytes",
            plain_profile(None, None),
        )],
    );
    files.add_file(
        &user_directory().join("Arkanoid (U).3dn"),
        &hash_profile("e13ace8d5b3c", None),
    );

    let mut library = library_of(&files, &catalogue, None);

    assert_eq!(vec!["1"], ids(&library));
    let applied = library.apply_rom_paths(&[RomPathRecord {
        id: "1".to_owned(),
        rom_path: "/roms/arkanoid.nes".to_owned(),
        title: String::new(),
        last_time: 1_760_950_000,
    }]);
    assert_eq!(1, applied);
    let entry = library.find("1").unwrap();
    assert_eq!("/roms/arkanoid.nes", entry.rom_path);
    assert_eq!(1_760_950_000, entry.last_played);
    // And the file the library persists is keyed on the same identifier.
    assert_eq!(
        vec![Some("1".to_owned())],
        library
            .to_file()
            .games
            .iter()
            .map(|game| game.id.clone())
            .collect::<Vec<Option<String>>>()
    );
}

/// REQ-PROF-087: the title shown is the one of the 2.1 profile — its envelope `Title`
/// (REQ-PROF-073), else its file name — not the catalogue's.
#[test]
fn req_prof_087_the_title_comes_from_the_2_1_profile() {
    let files = FakeProfileFileSystem::new();
    let catalogue = catalogue_of(
        &files,
        &[
            (
                "1",
                "Arkanoid",
                "profiles/Arkanoid (U).bytes",
                plain_profile(None, None),
            ),
            (
                "2",
                "Balloon Fight",
                "profiles/BalloonFight.bytes",
                plain_profile(None, None),
            ),
        ],
    );
    // One carries a `Title`, the other none: its file name answers for it (REQ-PROF-062).
    files.add_file(
        &user_directory().join("Arkanoid (U).3dn"),
        &hash_profile("e13ace8d5b3c", Some("Arkanoid (2.1)")),
    );
    files.add_file(
        &user_directory().join("BalloonFight.3dn"),
        &hash_profile("balloon-2-1-01", None),
    );

    let library = library_of(&files, &catalogue, None);

    assert_eq!(vec!["1", "2"], ids(&library));
    assert_eq!("Arkanoid (2.1)", library.find("1").unwrap().title);
    assert_eq!("BalloonFight", library.find("2").unwrap().title);
}

/// AMD-065 §3, third criterion: a user 3dn and a user 3dh of the same game, without any built-in
/// profile — one entry, the 3dh, and the 3dn still on the disk byte for byte.
#[test]
fn req_prof_087_a_user_3dn_and_a_user_3dh_of_one_game_make_a_single_entry() {
    let files = FakeProfileFileSystem::new();
    let catalogue = catalogue_of(&files, &[]);
    let plain = plain_profile(Some("mine-3dn-0001"), Some("Mine"));
    let plain_path = user_directory().join("Mine.3dn");
    files.add_file(&plain_path, &plain);
    files.add_file(
        &user_directory().join("Mine.3dh"),
        &hash_profile("mine-3dh-0001", Some("Mine 2.1")),
    );

    let log = RecordingProfileLog::new();
    let library = library_of(&files, &catalogue, Some(&log));

    assert_eq!(1, library.entries().len());
    let entry = &library.entries()[0];
    assert_eq!("Mine 2.1", entry.title);
    assert_eq!(ProfileGeneration::TwoPointOne, entry.generation());
    assert!(entry.profile_path().ends_with("Mine.3dh"));
    assert!(log.any_contains("are the same game"));
    assert!(log.any_contains("its file stays on the disk"));

    // Nothing was deleted or rewritten (AMD-030 §5, AMD-037 « Aucune réécriture »).
    assert!(files.file_exists(&plain_path));
    assert_eq!(plain, files.read_all_bytes(&plain_path).unwrap());
}

/// REQ-PROF-087 whichever order the two files are read in: the 3dh wins even when the 3dn is read
/// first, and then the entry keeps the identifier the 3dn had given it.
#[test]
fn req_prof_087_the_3dh_wins_whichever_file_is_read_first() {
    let files = FakeProfileFileSystem::new();
    let catalogue = catalogue_of(&files, &[]);
    // "A-plain.3dn" sorts before "B-hash.3dn": the earlier generation is read first.
    files.add_file(
        &user_directory().join("A-plain.3dn"),
        &plain_profile(Some("same-id-0001"), Some("First")),
    );
    files.add_file(
        &user_directory().join("B-hash.3dn"),
        &hash_profile("same-id-0001", Some("Second")),
    );

    // The two files carry the same `Id`, which is the case of 56 of the 101 games: the folder
    // scan is what has to prefer the 2.1 one, since it yields one profile per identifier.
    let log = RecordingProfileLog::new();
    let scanned = folder(&files, Some(&log)).scan(&[]).unwrap();
    assert_eq!(1, scanned.len());
    assert!(scanned[0].file_path.ends_with("B-hash.3dn"));
    assert!(log.any_contains("of generation 2.1; it is the one shown"));
    assert!(!log.any_contains("duplicated profile"));

    let library = library_of(&files, &catalogue, None);

    assert_eq!(1, library.entries().len());
    let entry = &library.entries()[0];
    assert_eq!("same-id-0001", entry.id());
    assert_eq!("Second", entry.title);
    assert!(entry.profile_path().ends_with("B-hash.3dn"));
}

/// AMD-065 §3, fourth criterion, and AMD-065 §2 last paragraph: between two profiles of an
/// earlier generation nothing changes. The four profiles of AMD-039 §5 — a 2.0 whose Lua scripts
/// differ from the built-in of the same name — keep their own entry.
#[test]
fn req_prof_087_between_two_earlier_generations_nothing_changes() {
    let files = FakeProfileFileSystem::new();
    let catalogue = catalogue_of(
        &files,
        &[(
            "37",
            "Super Mario Bros",
            "profiles/Super Mario Bros.bytes",
            plain_profile(Some("Jhj6E7iOysxT"), None),
        )],
    );
    // AMD-039 §5: the same game, the same name, another content — and no `Id` clash, since the
    // library identifier of the built-in is the catalogue's.
    files.add_file(
        &user_directory().join("Super Mario Bros.3dn"),
        &plain_profile(Some("smb-2-0-00001"), Some("Super Mario Bros")),
    );

    let library = library_of(&files, &catalogue, None);

    assert_eq!(vec!["37", "smb-2-0-00001"], ids(&library));
    assert_eq!(
        GameProfileSource::BuiltIn,
        library.find("37").unwrap().source()
    );
}

/// REQ-PROF-087 on a profile imported while the application runs: the entry is taken over at
/// once, without a restart. This is the path of AMD-056 REQ-UI-214 and of the import of a whole
/// installation.
#[test]
fn req_prof_087_a_2_1_profile_imported_at_run_time_takes_the_entry_over() {
    let files = FakeProfileFileSystem::new();
    let catalogue = catalogue_of(
        &files,
        &[(
            "20",
            "Hogan's Alley",
            "profiles/Hogan_s Alley.bytes",
            plain_profile(None, None),
        )],
    );
    let mut library = library_of(&files, &catalogue, None);
    assert_eq!(GameProfileSource::BuiltIn, library.entries()[0].source());

    let imported = UserProfile {
        id: "3348a6f46bb7".to_owned(),
        title: "Hogan's Alley".to_owned(),
        file_path: user_directory().join("Hogan's Alley.3dn"),
        header: format::load_header(&hash_profile("3348a6f46bb7", None), None, None).unwrap(),
        origin: ProfileOrigin::User,
    };
    let log = RecordingProfileLog::new();
    let added = library.add_user_profile(&imported, Some(&log)).unwrap();

    assert_eq!("20", added.id(), "the entry keeps the catalogue identifier");
    assert_eq!(GameProfileSource::User, added.source());
    assert!(added.profile_path().ends_with("Hogan's Alley.3dn"));
    assert_eq!(1, library.entries().len());
    assert!(library.find("3348a6f46bb7").is_none());
}

/// REQ-PROF-087 the other way round: a 3dn imported while the application runs adds nothing to a
/// game a 2.1 profile already holds, whether the two agree on their `Id` or on their name alone.
#[test]
fn req_prof_087_a_3dn_imported_at_run_time_does_not_take_a_2_1_entry_over() {
    let files = FakeProfileFileSystem::new();
    let catalogue = catalogue_of(&files, &[]);
    files.add_file(
        &user_directory().join("Mine.3dh"),
        &hash_profile("mine-3dh-0001", Some("Mine 2.1")),
    );
    let mut library = library_of(&files, &catalogue, None);
    assert_eq!(1, library.entries().len());

    let log = RecordingProfileLog::new();
    // Same name, another identifier: condition 2.
    let by_name = UserProfile {
        id: "mine-3dn-0001".to_owned(),
        title: "Mine".to_owned(),
        file_path: user_directory().join("Mine.3dn"),
        header: format::load_header(
            &plain_profile(Some("mine-3dn-0001"), Some("Mine")),
            None,
            None,
        )
        .unwrap(),
        origin: ProfileOrigin::User,
    };
    assert!(library.add_user_profile(&by_name, Some(&log)).is_none());

    // Same identifier: the 56 games whose two formats agree on it.
    let by_id = UserProfile {
        id: "mine-3dh-0001".to_owned(),
        title: "Elsewhere".to_owned(),
        file_path: user_directory().join("Elsewhere.3dn"),
        header: format::load_header(
            &plain_profile(Some("mine-3dh-0001"), Some("Elsewhere")),
            None,
            None,
        )
        .unwrap(),
        origin: ProfileOrigin::User,
    };
    assert!(library.add_user_profile(&by_id, Some(&log)).is_none());

    assert_eq!(1, library.entries().len());
    assert_eq!("Mine 2.1", library.entries()[0].title);
    assert_eq!(2, log.informations.borrow().len());
}

/// REQ-PROF-087: a profile paired with the built-in by its `Id` leaves the entry reachable under
/// the built-in's name too, so that a third profile of the game — the 3dn of the 2.0 folder —
/// still finds it instead of adding an entry.
#[test]
fn req_prof_087_a_replaced_entry_is_still_found_by_the_name_it_was_known_under() {
    let files = FakeProfileFileSystem::new();
    let catalogue = catalogue_of(
        &files,
        &[(
            "37",
            "Super Mario Bros",
            "profiles/Super Mario Bros.bytes",
            plain_profile(Some("Jhj6E7iOysxT"), None),
        )],
    );
    // Read first (ordinal order): the 2.1 profile, under a name of its own.
    files.add_file(
        &user_directory().join("A-smb-2-1.3dn"),
        &hash_profile("Jhj6E7iOysxT", Some("SMB 2.1")),
    );
    // Then a 2.0 profile of the same game, recognised by the built-in's name alone.
    files.add_file(
        &user_directory().join("Super Mario Bros.3dn"),
        &plain_profile(Some("smb-2-0-00001"), Some("Super Mario Bros")),
    );

    let library = library_of(&files, &catalogue, None);

    assert_eq!(vec!["37"], ids(&library));
    assert_eq!("SMB 2.1", library.find("37").unwrap().title);
}

// --- AMD-065 §3, on the extracted assets --------------------------------------------------------

/// The user folder of a fake file system holding the 2.1 files of `assets-neutral/profiles-2.1/`,
/// named `.3dn` as the import from an installation names them (AMD-062 §8).
fn add_hash_profiles(files: &FakeProfileFileSystem, names: &[&str]) {
    for name in names {
        let stem = name.trim_end_matches(".bytes");
        files.add_file(
            &user_directory().join(format!("{stem}.3dn")),
            &neutral_assets::read_hash_profile(name),
        );
    }
}

/// The full catalogue of `assets-neutral/`, read from the real folder.
fn neutral_catalogue() -> ProfileCatalogue {
    ProfileCatalogue::load(
        &PhysicalProfileFileSystem,
        &neutral_assets::catalogue_file(),
        FULL_EDITION,
    )
    .unwrap()
}

/// AMD-065 §3, first and second criteria: the built-in catalogue of `assets-neutral/` plus a user
/// folder holding three 2.1 profiles — one paired by its `Id` (condition 1), two by their name
/// (condition 2), one of those across the apostrophe. The library has exactly as many entries as
/// the catalogue alone, and the three entries keep the catalogue's identifier and load the 3dh.
#[test]
#[ignore = "needs the built-in profiles: set ZAXNES_PROFILES to the folder that holds profiles/, profiles-2.1/ and profiles.json"]
fn amd_065_three_2_1_profiles_replace_their_built_in_and_leave_the_catalogue_size() {
    let catalogue = neutral_catalogue();
    let files = FakeProfileFileSystem::new();
    add_hash_profiles(
        &files,
        &[
            "Super Mario Bros.bytes",
            "Arkanoid (U).bytes",
            "Hogan's Alley.bytes",
        ],
    );

    let built_in = catalogue.read_built_in_profiles(&PhysicalProfileFileSystem);
    let profiles = folder(&files, None)
        .scan_excluding(&catalogue.ids(), built_in.contents())
        .unwrap();
    assert_eq!(3, profiles.len(), "nothing ignored by the scan");

    let library =
        GameLibrary::build_with_built_in_profiles(&catalogue, &built_in, &profiles, None, None);

    assert_eq!(
        catalogue.entries().len(),
        library.entries().len(),
        "as many entries as the catalogue alone"
    );
    for name in ["Super Mario Bros", "Arkanoid (U)", "Hogan's Alley"] {
        // The built-in profile of the game, found by the file the catalogue names: for
        // `Hogan's Alley` that file is `Hogan_s Alley.bytes` (REQ-PROF-086 point 2).
        let wanted = normalised_name(Path::new(name));
        let expected = catalogue
            .entries()
            .iter()
            .find(|entry| normalised_name(Path::new(&entry.file)) == wanted)
            .unwrap_or_else(|| panic!("the catalogue has a profile file for {name}"));
        let entry = library.find(&expected.id).unwrap();
        assert_eq!(
            ProfileGeneration::TwoPointOne,
            entry.generation(),
            "{name} is shown from its 2.1 profile"
        );
        assert!(
            entry.profile_path().ends_with(format!("{name}.3dn")),
            "{name}: {}",
            entry.profile_path().display()
        );
        // And the file it loads really is a 3dh that the loader accepts (REQ-PROF-072).
        let bytes = files.read_all_bytes(entry.profile_path()).unwrap();
        assert_eq!(
            ProfileGeneration::TwoPointOne,
            ProfileGeneration::of_bytes(&bytes)
        );
        format::load(&bytes, None, Some(name)).unwrap();
    }
}

/// AMD-065 §3, the 2.1 installation criterion, without needing the installation: the 101 files of
/// `assets-neutral/profiles-2.1/` are what its import writes. Every one of them pairs with a
/// built-in profile — 56 by their `Id`, 45 by their name — so the library is exactly the
/// catalogue, every entry shown from its 2.1 profile, and no file is ignored.
#[test]
#[ignore = "needs the built-in profiles: set ZAXNES_PROFILES to the folder that holds profiles/, profiles-2.1/ and profiles.json"]
fn amd_065_the_101_2_1_profiles_leave_exactly_the_catalogue_entries() {
    let catalogue = neutral_catalogue();
    let files = FakeProfileFileSystem::new();
    let names: Vec<String> = neutral_assets::hash_profile_files()
        .iter()
        .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(101, names.len());
    add_hash_profiles(
        &files,
        &names.iter().map(String::as_str).collect::<Vec<&str>>(),
    );

    let log = RecordingProfileLog::new();
    let built_in = catalogue.read_built_in_profiles(&PhysicalProfileFileSystem);
    let profiles = folder(&files, Some(&log))
        .scan_excluding(&catalogue.ids(), built_in.contents())
        .unwrap();

    // Every envelope of the 2.1 folder carries an `Id`: nothing is derived, nothing is ignored.
    assert_eq!(101, profiles.len());
    assert!(!log.any_contains("derived from its content"));
    assert!(!log.any_contains("ignored"));

    let library =
        GameLibrary::build_with_built_in_profiles(&catalogue, &built_in, &profiles, None, None);

    assert_eq!(catalogue.entries().len(), library.entries().len());
    assert_eq!(101, library.entries().len());
    for entry in library.entries() {
        assert_eq!(
            ProfileGeneration::TwoPointOne,
            entry.generation(),
            "{}: {}",
            entry.id(),
            entry.display_title()
        );
        assert_eq!(GameProfileSource::User, entry.source());
    }
    // The identifiers are the catalogue's, so nothing of `rom.json` moves.
    let mut kept: Vec<&str> = ids(&library);
    let mut expected: Vec<&str> = catalogue
        .entries()
        .iter()
        .map(|entry| entry.id.as_str())
        .collect();
    kept.sort_unstable();
    expected.sort_unstable();
    assert_eq!(expected, kept);
}
