//! The import dialogues of AMD-056 REQ-UI-214: the four rows of its table, and the five
//! acceptance criteria.
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

mod common;
mod ui_harness;

use std::path::{Path, PathBuf};

use ui_harness::{UiHarness, UiLanguageScope};
use zax_app::ui::profile_import::ProfileImportDialogKind;
use zax_app::ui::strings::UiLanguage;
use zax_profiles::format;
use zax_profiles::layout::FieldPresence;
use zax_profiles::profile::GameProfile;

/// A profile SPEC-03 accepts, with the identifier, the title and the version text given.
fn profile_bytes(id: Option<&str>, title: Option<&str>, version: Option<&str>) -> Vec<u8> {
    let mut profile = GameProfile::create_empty();
    profile.id = id.map(str::to_owned);
    profile.title = title.map(str::to_owned);
    profile.version_text = version.map(str::to_owned);
    let bytes = format::save(&profile, FieldPresence::All);
    // The decoder of SPEC-03 is what the import reads with, so a fixture of a supported version
    // must pass it; one of version "1" is refused on purpose (REQ-PROF-021).
    assert_eq!(version.is_none(), format::load(&bytes, None, None).is_ok());
    bytes
}

/// Writes `bytes` under `name` in the user folder and returns its path.
fn write(harness: &UiHarness, name: &str, bytes: &[u8]) -> PathBuf {
    let path = harness.paths.root.join(name);
    std::fs::write(&path, bytes).expect("the file can be written");
    path
}

/// Chooses `path` in the import dialog of the interface, as the file browser would.
fn choose(harness: &mut UiHarness, path: &Path) {
    let text = path.to_string_lossy().into_owned();
    harness
        .ui
        .choose_profile_to_import(&mut harness.host, &text);
}

/// How many `.3dn` files the user's profile folder holds.
fn profile_files(harness: &UiHarness) -> usize {
    std::fs::read_dir(&harness.paths.profiles_directory).map_or(0, |entries| {
        entries
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("3dn"))
            })
            .count()
    })
}

/// REQ-UI-214, first criterion: a text file called `junk.3dn` gives `Cannot import this file`,
/// whose body is the file name, a colon and the third cause of REQ-UI-084 — which carries the
/// full path itself — and the library is unchanged.
#[test]
fn req_ui_214_a_file_that_is_not_a_profile_cannot_be_imported() {
    let _language = UiLanguageScope::new(UiLanguage::English);
    let mut harness = UiHarness::new(&["Arkanoid"]);
    let before = harness.host.games().library().entries().len();
    let junk = write(&harness, "junk.3dn", b"this is plain text, not a profile");

    choose(&mut harness, &junk);

    let dialog = harness.ui.import_dialog().expect("a dialogue").clone();
    assert_eq!(ProfileImportDialogKind::Refused, dialog.kind);
    assert_eq!("Cannot import this file", dialog.title);
    assert_eq!(
        format!(
            "junk.3dn: {} has an obsolete or wrong profile format. \
             This profile is not compatible with the current version of Zaxnes.",
            junk.display()
        ),
        dialog.body
    );
    assert_eq!("OK", dialog.primary_label);
    assert_eq!(None, dialog.cancel_label);
    assert!(!dialog.destructive);
    assert_eq!(before, harness.host.games().library().entries().len());
    assert_eq!(0, profile_files(&harness));
}

/// REQ-UI-214, second criterion: a profile whose `Ver` is `"v1"` gives a body ending in
/// `3dn format version 1  is not supported` — the sanitised version, two spaces (AMD-056 §2).
#[test]
fn req_ui_214_an_unsupported_version_names_its_sanitised_version() {
    let _language = UiLanguageScope::new(UiLanguage::English);
    let mut harness = UiHarness::new(&["Arkanoid"]);
    let old = write(
        &harness,
        "old.3dn",
        &profile_bytes(Some("ABCDEFGHIJKL"), Some("Old"), Some("v1")),
    );

    choose(&mut harness, &old);

    let dialog = harness.ui.import_dialog().expect("a dialogue").clone();
    assert_eq!(ProfileImportDialogKind::Refused, dialog.kind);
    assert_eq!("Cannot import this file", dialog.title);
    assert_eq!(
        "old.3dn: 3dn format version 1  is not supported",
        dialog.body
    );
    assert_eq!(0, profile_files(&harness));
}

/// REQ-UI-214, third criterion: a file carrying the identifier of a built-in profile whose own
/// `Title` is « Foo » gives `Already in Zaxnes`, the body naming the title and the file, and
/// nothing is copied.
#[test]
fn req_ui_214_a_built_in_identifier_is_already_in_zaxnes() {
    let _language = UiLanguageScope::new(UiLanguage::English);
    let mut harness = UiHarness::new(&["Arkanoid"]);
    let before = harness.host.games().library().entries().len();
    // "g0" is the identifier of the first built-in entry of the harness's catalogue.
    let file = write(
        &harness,
        "x.3dn",
        &profile_bytes(Some("g0"), Some("Foo"), None),
    );

    choose(&mut harness, &file);

    let dialog = harness.ui.import_dialog().expect("a dialogue").clone();
    assert_eq!(ProfileImportDialogKind::AlreadyBuiltIn, dialog.kind);
    assert_eq!("Already in Zaxnes", dialog.title);
    assert_eq!(
        "Foo is a built-in profile, so x.3dn was not imported.",
        dialog.body
    );
    assert_eq!("OK", dialog.primary_label);
    assert_eq!(None, dialog.cancel_label);
    assert_eq!(before, harness.host.games().library().entries().len());
    assert_eq!(0, profile_files(&harness));
}

/// REQ-UI-214, fourth criterion: a new version of an existing user profile asks
/// `Replace <nom>?`; `Cancel` leaves the file and the entry intact, `Replace` replaces the file.
#[test]
fn req_ui_214_a_new_version_of_a_user_profile_asks_before_it_replaces() {
    let _language = UiLanguageScope::new(UiLanguage::English);
    let mut harness = UiHarness::new(&["Arkanoid"]);
    // First import: a new identifier, which goes through with no dialogue at all.
    let first = write(
        &harness,
        "Theirs.3dn",
        &profile_bytes(Some("ABCDEFGHIJKL"), Some("Theirs"), None),
    );
    choose(&mut harness, &first);
    assert_eq!(None, harness.ui.import_dialog());
    assert_eq!(1, profile_files(&harness));
    let imported = harness.paths.profiles_directory.join("Theirs.3dn");
    let as_imported = std::fs::read(&imported).unwrap();

    // Second import: the same identifier under another file name and another title.
    let second = write(
        &harness,
        "Mine.3dn",
        &profile_bytes(Some("ABCDEFGHIJKL"), Some("Mine"), None),
    );
    choose(&mut harness, &second);

    let dialog = harness.ui.import_dialog().expect("a dialogue").clone();
    assert_eq!(ProfileImportDialogKind::ConfirmReplacement, dialog.kind);
    // « <nom> » is the displayed title of the existing entry.
    assert_eq!("Replace Theirs?", dialog.title);
    assert_eq!(
        "The profile will be replaced with the one in Mine.3dn. \
         The ROM, cover and save states will be kept.",
        dialog.body
    );
    assert_eq!("Replace", dialog.primary_label);
    assert_eq!(Some("Cancel".to_owned()), dialog.cancel_label);
    assert!(dialog.destructive);

    // Cancel: file and entry intact.
    harness.ui.cancel_import_dialog();
    assert_eq!(None, harness.ui.import_dialog());
    assert_eq!(1, profile_files(&harness));
    assert!(imported.is_file());
    assert_eq!(as_imported, std::fs::read(&imported).unwrap());

    // Replace: the old file goes and the new one takes the entry.
    choose(&mut harness, &second);
    assert!(harness.ui.import_dialog().is_some());
    harness.ui.confirm_import_dialog(&mut harness.host);

    assert_eq!(None, harness.ui.import_dialog());
    assert!(!imported.is_file());
    let replacement = harness.paths.profiles_directory.join("Mine.3dn");
    assert!(replacement.is_file());
    assert_eq!(1, profile_files(&harness));
    let entry = harness
        .host
        .games()
        .library()
        .find("ABCDEFGHIJKL")
        .expect("the entry survived");
    assert_eq!(replacement, entry.profile_path());
}

/// REQ-UI-214, fifth criterion: a profile of an unknown identifier shows neither notification nor
/// dialogue, and the library gains one entry — the selected one of the chooser.
#[test]
fn req_ui_214_an_unknown_identifier_shows_nothing_and_is_selected() {
    let _language = UiLanguageScope::new(UiLanguage::English);
    let mut harness = UiHarness::new(&["Arkanoid"]);
    let before = harness.host.games().library().entries().len();
    let file = write(
        &harness,
        "New.3dn",
        &profile_bytes(Some("ABCDEFGHIJKL"), Some("New"), None),
    );

    choose(&mut harness, &file);

    assert_eq!(None, harness.ui.import_dialog());
    assert!(!harness.ui.state.dialog);
    assert!(harness.host.notifications.current().is_none());
    assert_eq!(before + 1, harness.host.games().library().entries().len());
    // « l'entrée importée DEVRAIT devenir l'entrée sélectionnée de la boîte de sélection ».
    assert_eq!(
        Some("ABCDEFGHIJKL".to_owned()),
        harness.ui.chooser.selected_id
    );
}

/// AMD-066 §3, last criterion: the reported case. `Contra.bytes`, one of the 45 built-in
/// profiles of 2.0 that carry no `Id`, chosen as `Contra.3dn` against the catalogue of
/// `assets-neutral/` shows « already built-in » — the dialogue of row 2 of REQ-UI-214 — in
/// English and in French, and nothing is copied. Before AMD-066 it was refused as an obsolete
/// format (REQ-PROF-090).
#[test]
#[ignore = "needs ZAXNES_PROFILES (the folder that holds profiles/ and profiles.json)"]
fn req_prof_088_the_built_in_contra_is_already_in_zaxnes_in_both_languages() {
    let assets = PathBuf::from(
        std::env::var_os("ZAXNES_PROFILES").expect("ZAXNES_PROFILES names the asset folder"),
    );
    let bytes = std::fs::read(assets.join("profiles").join("Contra.bytes")).expect("Contra.bytes");
    // The constat of AMD-066 §1: the file carries no identifier.
    let header = format::load_header(&bytes, None, None).expect("a readable profile");
    assert!(!header.has_id(), "Contra.bytes carries an Id");
    let shown = header
        .title
        .clone()
        .unwrap_or_else(|| "Contra.3dn".to_owned());

    for (language, title, body) in [
        (
            UiLanguage::English,
            "Already in Zaxnes",
            format!("{shown} is a built-in profile, so Contra.3dn was not imported."),
        ),
        (
            UiLanguage::French,
            "Déjà dans Zaxnes",
            format!("{shown} est un profil embarqué ; Contra.3dn n'a pas été importé."),
        ),
    ] {
        let mut harness = UiHarness::with_asset_root(&assets);
        let _language = UiLanguageScope::new(language);
        let before = harness.host.games().library().entries().len();
        // The catalogue of `assets-neutral/` really was read: without it there would be no
        // built-in bytes to compare against and the file would simply be added.
        assert_eq!(
            101, before,
            "the catalogue of the asset folder was not read"
        );
        let chosen = write(&harness, "Contra.3dn", &bytes);

        choose(&mut harness, &chosen);

        let dialog = harness.ui.import_dialog().expect("a dialogue").clone();
        assert_eq!(ProfileImportDialogKind::AlreadyBuiltIn, dialog.kind);
        assert_eq!(title, dialog.title);
        assert_eq!(body, dialog.body);
        assert_eq!(None, dialog.cancel_label);
        // Nothing was copied, and the library did not grow.
        assert_eq!(0, profile_files(&harness));
        assert_eq!(before, harness.host.games().library().entries().len());
    }
}

/// AMD-035 and AMD-036: the three dialogues follow the interface language.
#[test]
fn amd_036_the_import_dialogues_follow_the_interface_language() {
    let mut harness = UiHarness::new(&["Arkanoid"]);
    // The host reads the language setting when it is built, so the switch comes after it.
    let _language = UiLanguageScope::new(UiLanguage::French);

    let junk = write(&harness, "junk.3dn", b"not a profile");
    choose(&mut harness, &junk);
    let refused = harness.ui.import_dialog().expect("a dialogue").clone();
    assert_eq!("Impossible d'importer ce fichier", refused.title);
    assert!(
        refused.body.contains("a un format de profil obsolète"),
        "{}",
        refused.body
    );
    assert_eq!("OK", refused.primary_label);
    harness.ui.cancel_import_dialog();

    let built_in = write(
        &harness,
        "x.3dn",
        &profile_bytes(Some("g0"), Some("Foo"), None),
    );
    choose(&mut harness, &built_in);
    let already = harness.ui.import_dialog().expect("a dialogue").clone();
    assert_eq!("Déjà dans Zaxnes", already.title);
    assert_eq!(
        "Foo est un profil embarqué ; x.3dn n'a pas été importé.",
        already.body
    );
    harness.ui.cancel_import_dialog();

    let first = write(
        &harness,
        "Theirs.3dn",
        &profile_bytes(Some("ABCDEFGHIJKL"), Some("Theirs"), None),
    );
    choose(&mut harness, &first);
    let second = write(
        &harness,
        "Mine.3dn",
        &profile_bytes(Some("ABCDEFGHIJKL"), Some("Mine"), None),
    );
    choose(&mut harness, &second);
    let replace = harness.ui.import_dialog().expect("a dialogue").clone();
    assert_eq!("Remplacer Theirs ?", replace.title);
    assert_eq!(
        "Le profil sera remplacé par celui de Mine.3dn. \
         La ROM, la jaquette et les sauvegardes d'état seront conservées.",
        replace.body
    );
    assert_eq!("Remplacer", replace.primary_label);
    assert_eq!(Some("Annuler".to_owned()), replace.cancel_label);
}
