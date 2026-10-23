//! The 3dSen import through the interface: the folder picker, the counts, the errors
//! (`ThreeDSenUiTests.cs`).

#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

mod common;
mod ui_harness;

use ui_harness::bundle_support::synthetic_bundle::{self, SyntheticBundleOptions};
use ui_harness::UiHarness;
use zax_app::emulation::commands::{Command, CommandArgs};
use zax_app::emulation::notification_center::NotificationSeverity;
use zax_app::ui::file_browser::{self, FileBrowserModel, FileEntryKind};
use zax_app::ui::text;
use zax_profiles::format;
use zax_profiles::import::{expected_layout, UnityCompression, BUNDLE_EXTENSION, BUNDLE_PREFIX};
use zax_profiles::layout::FieldPresence;
use zax_profiles::library::GameProfileSource;
use zax_profiles::profile::GameProfile;

fn profile(id: &str, title: &str) -> Vec<u8> {
    let mut profile = GameProfile::create_empty();
    profile.id = Some(id.to_owned());
    profile.title = Some(title.to_owned());
    zax_profiles::export::normalize(&mut profile, None).expect("a valid profile");
    format::save(&profile, FieldPresence::All)
}

fn write_installation(harness: &UiHarness, assets: &[(&str, Vec<u8>)]) -> String {
    let install = harness.paths.root.join("3dSen");
    let directory = install
        .join("3dSen_Data")
        .join("StreamingAssets")
        .join("aa")
        .join("StandaloneWindows64");
    std::fs::create_dir_all(&directory).expect("the folder can be created");
    let bundle = synthetic_bundle::build(
        assets,
        &SyntheticBundleOptions {
            block_compression: UnityCompression::Lz4,
            block_size: 128 * 1024,
            info_at_end: true,
            ..SyntheticBundleOptions::default()
        },
    );
    std::fs::write(
        directory.join(format!("{BUNDLE_PREFIX}feedface{BUNDLE_EXTENSION}")),
        bundle,
    )
    .expect("the bundle can be written");
    install.to_string_lossy().into_owned()
}

fn profile_files(harness: &UiHarness) -> usize {
    std::fs::read_dir(&harness.paths.profiles_directory).map_or(0, |listing| {
        listing
            .flatten()
            .filter(|item| item.path().extension().is_some_and(|e| e == "3dn"))
            .count()
    })
}

// AMD-030 §2: "ouvre un sélecteur de dossier"
#[test]
fn the_menu_entry_opens_the_picker_in_folder_mode() {
    let mut harness = UiHarness::new(&["Arkanoid"]);

    harness.run(Command::ImportThreeDSen);

    assert!(harness.ui.state.file_browser);
    assert!(harness.ui.browser.allow_folders());
    assert_eq!(text::import_three_d_sen_title(), harness.ui.browser.title());
    assert_eq!(text::import_three_d_sen_hint(), harness.ui.browser.hint());

    // §3: the bundle can be named directly, so the dialog offers its extension.
    let labels = |filters: &[file_browser::FileFilter]| -> Vec<String> {
        filters.iter().map(|filter| filter.label.clone()).collect()
    };
    assert_eq!(
        labels(&file_browser::three_d_sen_filters()),
        labels(harness.ui.browser.filters())
    );
    assert_eq!(["bundle"], harness.ui.browser.extensions());
}

// AMD-030 §2: the dialog answers with the folder being shown.
#[test]
fn the_picker_answers_with_a_folder() {
    let harness = UiHarness::new(&[]);
    let root = harness.paths.root.to_string_lossy().into_owned();
    let mut browser = FileBrowserModel::new();
    browser.open(
        &text::import_three_d_sen_title(),
        file_browser::three_d_sen_filters(),
        Some(&root),
        true,
        None,
    );

    assert!(browser.can_confirm());
    assert!(browser.confirm());
    assert_eq!(Some(root.as_str()), browser.result());
}

// AMD-030 §2: a selected sub-folder is the answer too.
#[test]
fn a_selected_folder_is_the_answer() {
    let harness = UiHarness::new(&[]);
    let child = harness.paths.root.join("3dSen");
    std::fs::create_dir_all(&child).expect("the folder can be created");

    let root = harness.paths.root.to_string_lossy().into_owned();
    let mut browser = FileBrowserModel::new();
    browser.open(
        &text::import_three_d_sen_title(),
        file_browser::three_d_sen_filters(),
        Some(&root),
        true,
        None,
    );
    browser.selected = browser
        .entries()
        .iter()
        .find(|entry| entry.kind == FileEntryKind::Folder && entry.name == "3dSen")
        .cloned();

    assert!(browser.confirm());
    assert_eq!(Some(child.to_string_lossy().as_ref()), browser.result());
}

// REQ-UI-104: the ROM dialog still refuses to answer with a folder.
#[test]
fn the_rom_dialog_still_only_answers_with_a_file() {
    let harness = UiHarness::new(&[]);
    let root = harness.paths.root.to_string_lossy().into_owned();
    let mut browser = FileBrowserModel::new();
    browser.open(
        &text::open_rom_title(),
        file_browser::rom_filters(),
        Some(&root),
        false,
        None,
    );

    assert!(!browser.allow_folders());
    assert!(!browser.can_confirm());
    assert!(!browser.confirm());
    assert!(browser.result().is_none());
}

// AMD-030 §2: "un message donne le nombre de profils importés" + "apparaissent aussitôt"
#[test]
fn the_import_fills_the_library_and_reports_the_counts() {
    let mut harness = UiHarness::new(&["Arkanoid"]);
    let install = write_installation(
        &harness,
        &[
            ("Pinball", profile("PINBALL00001", "Pinball")),
            ("Tetris", profile("TETRIS000001", "Tetris")),
        ],
    );

    harness.run_with(Command::ImportThreeDSen, CommandArgs::of_text(&install));

    // AMD-036 §1: the notification is in the language of the interface, English by default.
    let shown = harness
        .host
        .notifications
        .current()
        .expect("a notification");
    assert_eq!(
        "3dSen: 2 profile(s) imported, 0 already present, 0 failed.",
        shown.text
    );
    assert_eq!(NotificationSeverity::Normal, shown.severity);

    // The profiles are in the user's folder, under their own name.
    assert!(harness
        .paths
        .profiles_directory
        .join("Pinball.3dn")
        .is_file());
    assert!(harness
        .paths
        .profiles_directory
        .join("Tetris.3dn")
        .is_file());

    // §5: the title and the identifier shown are the profile's own.
    let library = harness.host.games().library();
    let entry = library
        .find("PINBALL00001")
        .expect("the profile is in the library");
    assert_eq!("Pinball", entry.display_title());
    assert_eq!(GameProfileSource::User, entry.source());
    assert!(library.find("TETRIS000001").is_some());

    // The chooser of REQ-UI-201 is rebuilt, so the new profiles are pickable at once.
    assert!(harness
        .ui
        .chooser
        .all()
        .iter()
        .any(|row| row.title == "Pinball"));
}

// AMD-030 §2: a second import adds nothing.
#[test]
fn a_second_import_reports_the_profiles_as_already_present() {
    let mut harness = UiHarness::new(&["Arkanoid"]);
    let install = write_installation(&harness, &[("Pinball", profile("PINBALL00001", "Pinball"))]);

    harness.run_with(Command::ImportThreeDSen, CommandArgs::of_text(&install));
    harness.run_with(Command::ImportThreeDSen, CommandArgs::of_text(&install));

    assert_eq!(
        "3dSen: 0 profile(s) imported, 1 already present, 0 failed.",
        harness
            .host
            .notifications
            .current()
            .expect("a notification")
            .text
    );
    assert_eq!(1, profile_files(&harness));
}

// AMD-030 §3: "l'interface le dit clairement et rappelle le chemin attendu"
#[test]
fn an_installation_without_a_bundle_is_reported_as_an_error() {
    let mut harness = UiHarness::new(&["Arkanoid"]);
    let install = harness.paths.root.join("not-3dsen");
    std::fs::create_dir_all(&install).expect("the folder can be created");

    harness.run_with(
        Command::ImportThreeDSen,
        CommandArgs::of_text(&install.to_string_lossy()),
    );

    let shown = harness
        .host
        .notifications
        .current()
        .expect("a notification");
    assert_eq!(NotificationSeverity::Error, shown.severity);
    assert!(
        shown.text.contains("no 3dSen profile bundle was found"),
        "{}",
        shown.text
    );
    assert!(shown.text.contains(&expected_layout()), "{}", shown.text);
    assert_eq!(0, profile_files(&harness));
}

// AMD-030 §5: a corrupt bundle is a message, not a crash, and writes nothing.
#[test]
fn a_corrupt_bundle_is_reported_as_an_error() {
    let mut harness = UiHarness::new(&["Arkanoid"]);
    let install = write_installation(&harness, &[("Pinball", profile("PINBALL00001", "Pinball"))]);
    let bundle = walk(std::path::Path::new(&install)).expect("a bundle was written");
    let bytes = std::fs::read(&bundle).expect("the bundle can be read");
    std::fs::write(&bundle, &bytes[..64]).expect("the bundle can be cut");

    harness.run_with(Command::ImportThreeDSen, CommandArgs::of_text(&install));

    assert_eq!(
        NotificationSeverity::Error,
        harness
            .host
            .notifications
            .current()
            .expect("a notification")
            .severity
    );
    assert_eq!(0, profile_files(&harness));
}

fn walk(folder: &std::path::Path) -> Option<std::path::PathBuf> {
    for item in std::fs::read_dir(folder).ok()?.flatten() {
        let path = item.path();
        if path.is_dir() {
            if let Some(found) = walk(&path) {
                return Some(found);
            }
        } else if path.extension().is_some_and(|e| e == "bundle") {
            return Some(path);
        }
    }
    None
}
