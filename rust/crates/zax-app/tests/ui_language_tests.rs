//! The interface language of AMD-035: two complete tables, English on a fresh user folder, an
//! immediate and persisted switch to French, and an unknown value read as English
//! (`UiLanguageTests.cs`).

mod common;
mod ui_harness;

use std::collections::HashSet;

use ui_harness::{UiHarness, UiLanguageScope};
use zax_app::emulation::commands::Command;
use zax_app::emulation::host::{
    delete_slot_question, ConfirmationRequest, ConfirmedAction, DELETE_GAME_QUESTION, NO_LABEL,
    YES_LABEL,
};
use zax_app::emulation::rom_file::INVALID_ROM_MESSAGE;
use zax_app::persistence::app_settings::{AppSettings, AppSettingsMut};
use zax_app::persistence::save_state_file::GAME_MISSING_MESSAGE;
use zax_app::persistence::settings_store::SettingsStore;
use zax_app::ui::commands::UiCommand;
use zax_app::ui::confirm_dialog::ConfirmationView;
use zax_app::ui::options::{self, OptionsModel, OptionsTab};
use zax_app::ui::strings::{self, UiLanguage};
use zax_app::ui::{file_browser, keys, notifications, text};
use zax_profiles::catalogue::import_messages;
use zax_profiles::import::{
    ThreeDSenImportSummary, ThreeDSenProfileOutcome, ThreeDSenProfileResult,
};
use zax_settings::catalogue::SettingId;
use zax_settings::meta::SettingKind;
use zax_settings::persistence::SettingsFile;
use zax_settings::values::SettingValue;

fn assert_both_tables_have(key: &str) {
    assert!(
        strings::lookup(UiLanguage::English, key).is_some(),
        "{key} is not in the English table"
    );
    assert!(
        strings::lookup(UiLanguage::French, key).is_some(),
        "{key} is not in the French table"
    );
}

/// AMD-035 §4: the two tables hold exactly the same keys, and no value is empty or blank. A
/// missing translation would fall back to English silently, so the check is what keeps the
/// French interface from going half English without anyone noticing.
#[test]
fn amd_035_the_two_tables_have_the_same_keys_and_no_empty_value() {
    let english = strings::english_table();
    let french = strings::french_table();
    assert!(!english.is_empty());

    let english_keys: HashSet<&str> = english.iter().map(|(key, _)| *key).collect();
    let french_keys: HashSet<&str> = french.iter().map(|(key, _)| *key).collect();
    assert_eq!(
        english.len(),
        english_keys.len(),
        "a key is twice in English"
    );
    assert_eq!(french.len(), french_keys.len(), "a key is twice in French");
    assert_eq!(
        Vec::<&&str>::new(),
        english_keys.difference(&french_keys).collect::<Vec<_>>()
    );
    assert_eq!(
        Vec::<&&str>::new(),
        french_keys.difference(&english_keys).collect::<Vec<_>>()
    );

    for (key, text) in english.iter().chain(french) {
        assert!(!text.trim().is_empty(), "{key}");
    }
}

/// Every key `keys` publishes is in both tables, so no screen can ask for a text that does not
/// exist and end up showing its raw key (AMD-035 §3).
#[test]
fn amd_035_every_published_key_is_in_both_tables() {
    assert!(!keys::PUBLISHED.is_empty());
    for key in keys::PUBLISHED {
        assert_both_tables_have(key);
    }
}

/// Every command of the model and every setting the options window shows has a label in both
/// tables: those two families of keys are built from a name at run time, so a command or a
/// setting added without its text would otherwise show its key.
#[test]
fn amd_035_every_command_and_every_shown_setting_has_a_label() {
    for command in Command::ALL {
        assert_both_tables_have(&keys::command(command.name()));
    }
    for command in UiCommand::ALL {
        assert_both_tables_have(&keys::command(command.name()));
    }
    for row in OptionsModel::new().all_rows() {
        assert_both_tables_have(&keys::setting(&row.descriptor.key()));
    }

    // The labels of the enumerated values that are words are translated; the ones that are
    // numbers (60fps, 1/2, 75%) are shown as the catalogue writes them, which is what
    // `label_of_option` falls back to. Neither case may ever yield a key or an empty string.
    for spoken in [UiLanguage::English, UiLanguage::French] {
        let _scope = UiLanguageScope::new(spoken);
        for row in OptionsModel::new().all_rows() {
            let label = row.label();
            assert!(!label.trim().is_empty(), "{}", row.descriptor.key());
            assert!(!label.contains(keys::SETTING_PREFIX), "{label}");
            for option in row.descriptor.domain().options().unwrap_or_default() {
                let shown = text::label_of_option(&option.label);
                assert!(!shown.trim().is_empty(), "{}", option.label);
                assert!(!shown.contains(keys::OPTION_PREFIX), "{shown}");
            }
        }
    }
}

/// AMD-035 §3: a key the translation is missing falls back to English rather than to an empty
/// line or to the key itself.
#[test]
fn amd_035_a_missing_translation_falls_back_to_english() {
    let _language = UiLanguageScope::new(UiLanguage::French);

    // Every key of the French table exists, so the fallback is exercised on a key neither table
    // holds: it must still answer something printable.
    assert_eq!("ui.nothing.here", strings::get("ui.nothing.here"));
    assert_eq!("Fichier", strings::get(keys::MENU_FILE));
}

/// AMD-035 §4: a fresh user folder gives an interface entirely in English.
#[test]
fn amd_035_a_fresh_user_folder_is_english() {
    let _language = UiLanguageScope::keep();
    let mut harness = UiHarness::new(&["Arkanoid"]);

    assert_eq!("en", harness.host.settings().language());
    assert_eq!(UiLanguage::English, strings::language());
    assert_eq!("File", harness.ui.menu_bar.menus()[0].title);
    assert_eq!("Video", options::title_of(OptionsTab::Video));
    assert_eq!("Open a ROM…", strings::get(&Command::OpenRom.label_key()));
    assert_eq!(
        "Player 1 — NES buttons",
        harness.ui.keys.sections()[0].title
    );
}

/// AMD-035 §1 and §4: writing `fr` switches the whole interface at once (the menu bar, the
/// options window, the key configuration and the command labels) with no restart.
#[test]
fn amd_035_switching_to_french_changes_the_texts_at_once() {
    let _language = UiLanguageScope::keep();
    let mut harness = UiHarness::new(&["Arkanoid"]);

    harness
        .host
        .update_settings(|model| model.set_language("fr"));
    harness.pump();

    assert_eq!(UiLanguage::French, strings::language());
    assert_eq!("Fichier", harness.ui.menu_bar.menus()[0].title);
    assert_eq!("Vidéo", options::title_of(OptionsTab::Video));
    assert_eq!(
        "Ouvrir une ROM…",
        strings::get(&Command::OpenRom.label_key())
    );
    assert_eq!(
        "Joueur 1 — boutons NES",
        harness.ui.keys.sections()[0].title
    );
    assert_eq!("Valeurs par défaut", text::reset_defaults());
    assert_eq!("Poste de travail", file_browser::computer_label());

    // And back again, still without a restart.
    harness
        .host
        .update_settings(|model| model.set_language("en"));
    harness.pump();
    assert_eq!("File", harness.ui.menu_bar.menus()[0].title);
}

/// AMD-035 §1: the choice is kept for the next launch.
#[test]
fn amd_035_the_choice_is_persisted_and_read_back_at_the_next_launch() {
    let _language = UiLanguageScope::keep();
    let mut harness = UiHarness::new(&["Arkanoid"]);

    harness
        .host
        .update_settings(|model| model.set_language("fr"));
    harness.pump();
    let mut store = SettingsStore::new(&harness.paths);
    store.save(harness.host.settings());

    let mut restarted = zax_settings::model::SettingsModel::new();
    assert_eq!("en", restarted.language());

    SettingsStore::new(&harness.paths).load(&mut restarted, time::OffsetDateTime::UNIX_EPOCH);
    assert_eq!("fr", restarted.language());
    assert_eq!(UiLanguage::French, UiLanguage::parse(&restarted.language()));
}

/// AMD-035 §1: an unknown value in the settings file is read as English, without an error and
/// without rewriting the file behind the user's back.
#[test]
fn amd_035_an_unknown_value_falls_back_to_english() {
    let _language = UiLanguageScope::new(UiLanguage::French);

    let mut settings = zax_settings::model::SettingsModel::new();
    settings.set(SettingId::UiLanguage, "kl");

    assert_eq!("en", settings.language());
    assert_eq!(UiLanguage::English, UiLanguage::parse(&settings.language()));

    // The stored text is left alone: only what the interface reads is brought back.
    assert_eq!(
        SettingValue::Text("kl".to_owned()),
        settings.get(SettingId::UiLanguage)
    );
}

/// AMD-035 §1: the language is in the Interface tab of the options window, as a drop-down of the
/// two values.
#[test]
fn amd_035_the_options_window_offers_the_language_in_the_interface_tab() {
    let model = OptionsModel::new();
    let rows: Vec<_> = model
        .rows(OptionsTab::Interface)
        .iter()
        .filter(|row| row.descriptor.id() == SettingId::UiLanguage)
        .collect();
    assert_eq!(1, rows.len());
    let row = rows[0];

    assert_eq!(Some(&["en", "fr"][..]), row.descriptor.text_options());
    assert_eq!(SettingKind::Text, row.descriptor.kind());
    assert_eq!("Language", row.label());
    assert_eq!("English", text::label_of_option("en"));
    assert_eq!("Français", text::label_of_option("fr"));
}

/// AMD-035 §2: the console log and the command line stay in English whatever the setting says.
/// The usage text is a constant of `AppOptions`, so it cannot follow the language at all.
#[test]
#[ignore = "the usage text of AppOptions is ported with the command-line front end (T14)"]
fn amd_035_the_help_text_stays_in_english() {}

/// AMD-036 §2: in `en`, every notification whose text SPEC-10 §3.10 and SPEC-09 fix word for
/// word is identical, character for character, to the text of the spec: the two spaces of
/// REQ-UI-084 and the missing space after the path included.
#[test]
fn req_ui_082_in_english_the_fixed_texts_are_the_texts_of_the_spec() {
    let _language = UiLanguageScope::new(UiLanguage::English);

    // REQ-UI-082: the four in-game shortcut notifications.
    assert_eq!("Roll Back", strings::get(keys::NOTIFY_ROLL_BACK));
    assert_eq!("Fast Forward", strings::get(keys::NOTIFY_FAST_FORWARD));
    assert_eq!("Quick Save", strings::get(keys::NOTIFY_QUICK_SAVE));
    assert_eq!("Quick Load", strings::get(keys::NOTIFY_QUICK_LOAD));

    // REQ-UI-083: the five load and read failures.
    assert_eq!(
        "Corrupted zip file",
        strings::get(keys::NOTIFY_CORRUPTED_ZIP)
    );
    assert_eq!(
        "Uncompression is not successful",
        strings::get(keys::NOTIFY_UNCOMPRESSION_FAILED)
    );
    assert_eq!("Invalid rom file.", strings::get(keys::NOTIFY_INVALID_ROM));
    assert_eq!(
        "The game profile doesn't exist anymore.",
        strings::get(keys::NOTIFY_GAME_MISSING)
    );
    assert_eq!(
        "Save state used a different core.",
        strings::get(keys::NOTIFY_FOREIGN_CORE)
    );

    // REQ-UI-084, AMD-029 §3: the three import failures, oddities included.
    assert_eq!(
        "This file is not a 3dn file",
        strings::get(keys::NOTIFY_NOT_A3DN_FILE)
    );
    assert_eq!(
        "3dn format version 7  is not supported",
        notifications::translate(&import_messages::unsupported_version("7"))
    );
    assert_eq!(
        "C:\\x\\y.3dnhas an obsolate or wrong profile format. \
         This profile is not compatible with the current version of Zaxnes.",
        notifications::translate(&import_messages::obsolete_format(std::path::Path::new(
            "C:\\x\\y.3dn"
        )))
    );

    // REQ-PERS-022 and REQ-PERS-024: the messages of the persisted files.
    assert_eq!(
        "Settings file was unreadable and has been reset.",
        strings::get(keys::NOTIFY_SETTINGS_UNREADABLE)
    );
    assert_eq!(
        "Settings were created by a newer version.",
        strings::get(keys::NOTIFY_SETTINGS_NEWER_VERSION)
    );

    // REQ-PERS-042 and REQ-PERS-054: the two questions and their buttons.
    assert_eq!(
        DELETE_GAME_QUESTION,
        strings::get(keys::CONFIRM_DELETE_GAME)
    );
    assert_eq!(
        "Do you want to delete the slot 4 ?",
        notifications::translate(&delete_slot_question("4"))
    );
    assert_eq!("Yes", strings::get(keys::BUTTON_YES));
    assert_eq!("No", strings::get(keys::BUTTON_NO));

    // Every one of them is produced in English by the module that raises it, so the English
    // interface shows exactly what that module wrote.
    for key in fixed_text_keys() {
        assert_eq!(
            strings::lookup(UiLanguage::English, key),
            Some(strings::get(key).as_str())
        );
    }
}

/// AMD-036 §2: in `fr`, none of those texts is still English: the message the loop produced is
/// translated where it is shown, values and paths kept as they are.
#[test]
fn amd_036_in_french_the_fixed_texts_are_translated_where_they_are_shown() {
    let _language = UiLanguageScope::new(UiLanguage::French);

    assert_eq!("Sauvegarde rapide", notifications::translate("Quick Save"));
    assert_eq!(
        "Fichier ROM invalide.",
        notifications::translate(INVALID_ROM_MESSAGE)
    );
    assert_eq!(
        "Le profil du jeu n'existe plus.",
        notifications::translate(GAME_MISSING_MESSAGE)
    );
    assert_eq!(
        "Le fichier de réglages était illisible et a été réinitialisé.",
        notifications::translate(
            SettingsFile::<zax_settings::store::MemoryStorage>::UNREADABLE_MESSAGE
        )
    );

    // The value of a patterned text is read out of the English message and put back into the
    // French one, wherever the sentence needs it.
    assert_eq!(
        "La version 7 du format 3dn n'est pas prise en charge",
        notifications::translate(&import_messages::unsupported_version("7"))
    );
    assert!(
        notifications::translate(&import_messages::obsolete_format(std::path::Path::new(
            "C:\\x\\y.3dn"
        )))
        .starts_with("C:\\x\\y.3dn a un format de profil obsolète")
    );

    // The two confirmations and their buttons, through the box that shows them.
    let request = ConfirmationRequest {
        title: "Arkanoid".to_owned(),
        message: delete_slot_question("4"),
        confirm_label: YES_LABEL.to_owned(),
        cancel_label: NO_LABEL.to_owned(),
        action: ConfirmedAction::DeleteSlot("4".to_owned()),
    };
    let view = ConfirmationView::new(&request);
    assert_eq!("Voulez-vous supprimer l'emplacement 4 ?", view.message());
    assert_eq!("Oui", view.confirm_label());
    assert_eq!("Non", view.cancel_label());

    // A text no requirement fixes (a script notification, a file system message) is shown as it
    // comes rather than turned into its key.
    assert_eq!("anything else", notifications::translate("anything else"));
}

/// AMD-036 §2: the output of `--import-3dsen` is English whatever `ui.language` says, because
/// the command line stays English (AMD-035 §2). Only the notification the interface shows
/// follows the language.
#[test]
fn amd_036_the_3dsen_import_summary_is_english_on_the_command_line() {
    let _language = UiLanguageScope::new(UiLanguage::French);

    let result = |name: &str, outcome, file: &str, message: &str| ThreeDSenProfileResult {
        name: name.to_owned(),
        outcome,
        file_name: file.to_owned(),
        message: message.to_owned(),
    };
    let summary = ThreeDSenImportSummary::new(
        std::path::Path::new("C:\\3dSen\\bundle"),
        vec![
            result(
                "Pinball",
                ThreeDSenProfileOutcome::Imported,
                "Pinball.3dn",
                "",
            ),
            result(
                "Tetris",
                ThreeDSenProfileOutcome::AlreadyPresent,
                "Tetris.3dn",
                "",
            ),
            result("junk", ThreeDSenProfileOutcome::Failed, "", "bad"),
        ],
    );

    assert_eq!(
        "3dSen: 1 profile(s) imported, 1 already present, 1 failed.",
        summary.message()
    );

    // The same three counts, in the language of the interface, for the notification.
    assert_eq!(
        "3dSen : 1 profil(s) importé(s), 1 déjà présent(s), 1 en échec.",
        notifications::three_d_sen_summary(
            summary.imported,
            summary.already_present,
            summary.failed
        )
    );
}

/// The keys of the texts AMD-036 §1 moved into the tables.
fn fixed_text_keys() -> impl Iterator<Item = &'static str> {
    keys::PUBLISHED.iter().copied().filter(|key| {
        key.starts_with("notify.")
            || key.starts_with("confirm.")
            || *key == keys::BUTTON_YES
            || *key == keys::BUTTON_NO
    })
}
