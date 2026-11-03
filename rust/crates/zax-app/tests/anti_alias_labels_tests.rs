//! RO-5 part 1 (owner request): the anti-aliasing levels are **named** after what the renderer
//! really does — MSAA 4× and MSAA 8× — everywhere the user reads them, while everything a machine
//! reads keeps the names REQ-PERS-017 gave them.
//!
//! `video.anti_alias` is multisampling, as AMD-032 §2 decided: FFAA is 4 samples per pixel and
//! SMAA is 8 (`zax_render::geometry::anti_alias_samples`). The two names are the specification's,
//! not a description of the picture, so the interface now says the samples and the tooltip says
//! who calls them what.
//!
//! What may **not** move, and is pinned here: the stored values 0/1/2, the labels the settings
//! file holds, the libretro value strings (which `RetroArch` persists), the profile field of the
//! 3dSen import and the Lua surface. All of it is checked on the models and the tables, so it runs
//! without a window.
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

use zax_app::ui::handheld::advanced;
use zax_app::ui::options::{self, OptionsModel};
use zax_app::ui::strings::{self, UiLanguage};
use zax_app::ui::text;
use zax_settings::catalogue::{self, SettingId};
use zax_settings::core_options;
use zax_settings::model::SettingsModel;
use zax_settings::persistence::SettingsFile;
use zax_settings::store::MemoryStorage;
use zax_settings::values::SettingValue;

/// The three stored values, with the label the machine keeps and the text the user now reads in
/// each language.
const LEVELS: [(i32, &str, &str, &str); 3] = [
    (0, "None", "None", "Aucun"),
    (1, "FFAA", "MSAA 4×", "MSAA 4×"),
    (2, "SMAA", "MSAA 8×", "MSAA 8×"),
];

fn now() -> time::OffsetDateTime {
    time::OffsetDateTime::UNIX_EPOCH
}

/// REQ-PERS-017 as RO-5 part 1 presents it: every interface that shows the setting shows the
/// multisampling level, in both languages.
///
/// The four places the brief lists — the desktop options window, the console's global pages, the
/// console's game pages and the desktop *Game options* menu — all read the value through
/// `text::label_of_option`, so the two string-table entries cover all four. The two ends of that
/// path are checked here: the drop-down the window builds (`draw_enum_labelled`'s label list) and
/// the console row's text (`advanced::value_text`, which the game pages and the desktop menu go
/// through as well, by way of `handheld::options::value_text`).
#[test]
fn ro_5_the_three_levels_read_as_the_multisampling_they_are() {
    let descriptor = catalogue::descriptor(SettingId::VideoAntiAlias);
    let options = descriptor.domain().options().expect("three levels");

    for (language, pick) in [
        (UiLanguage::English, 1_usize),
        (UiLanguage::French, 2_usize),
    ] {
        strings::set_language(language);

        // The window's drop-down: one entry per rung, in order.
        let shown: Vec<String> = options
            .iter()
            .map(|option| text::label_of_option(&option.label))
            .collect();
        let wanted: Vec<String> = LEVELS
            .iter()
            .map(|level| {
                if pick == 1 {
                    level.2.to_owned()
                } else {
                    level.3.to_owned()
                }
            })
            .collect();
        assert_eq!(wanted, shown, "{language:?}");

        // The console row, and with it the game pages and the desktop menu.
        for (value, _, english, french) in LEVELS {
            let mut model = SettingsModel::new();
            model.set(SettingId::VideoAntiAlias, SettingValue::Int(value));
            let expected = if pick == 1 { english } else { french };
            assert_eq!(
                expected,
                advanced::value_text(SettingId::VideoAntiAlias, &model),
                "{language:?} at {value}"
            );
        }
    }
    strings::set_language(UiLanguage::English);
}

/// The setting is still a row of the window and of the console, on the same page as before: part 1
/// is a change of **text** and nothing else.
#[test]
fn ro_5_the_row_did_not_move() {
    assert!(OptionsModel::new()
        .all_rows()
        .any(|row| row.descriptor.id() == SettingId::VideoAntiAlias));
    assert!(advanced::ROW_SETTINGS.contains(&SettingId::VideoAntiAlias));
    assert!(!options::EXCLUDED.contains(&SettingId::VideoAntiAlias));
}

/// What a machine reads is untouched: the three stored values, the **domain** labels — which are
/// what the settings file holds, `video.anti_alias` being a `SettingKind::Enum` — and the profile
/// field of the 3dSen import (field 32).
#[test]
fn ro_5_the_stored_values_the_domain_labels_and_field_32_are_untouched() {
    let descriptor = catalogue::descriptor(SettingId::VideoAntiAlias);
    let options = descriptor.domain().options().expect("three levels");
    assert_eq!(3, options.len());
    for (index, (value, label, _, _)) in LEVELS.into_iter().enumerate() {
        assert_eq!(f64::from(value), options[index].value, "{label}");
        assert_eq!(label, options[index].label, "rung {index}");
    }
    assert_eq!(1, descriptor.default_value().as_int());
    assert_eq!(Some("32"), descriptor.profile_field());
    assert_eq!(Some("AntiAlias"), descriptor.lua_path());
}

/// The settings file: writing SMAA writes the label `"SMAA"`, and a file written by any earlier
/// Zaxnes — which holds those same labels — reads back as the same number.
#[test]
fn ro_5_the_settings_file_still_holds_the_specification_names() {
    let mut model = SettingsModel::new();
    model.set(SettingId::VideoAntiAlias, SettingValue::Int(2));
    let mut file = SettingsFile::new(MemoryStorage::new());
    file.save(&model).expect("the store accepts the write");
    let written = file
        .storage()
        .text(SettingsFile::<MemoryStorage>::DEFAULT_NAME)
        .expect("the file was written");
    assert!(written.contains("\"SMAA\""), "{written}");
    assert!(!written.contains("MSAA"), "{written}");

    for (value, label) in [(0, "None"), (1, "FFAA"), (2, "SMAA")] {
        let mut storage = MemoryStorage::new();
        storage.put(
            SettingsFile::<MemoryStorage>::DEFAULT_NAME,
            &format!("{{\"version\":1,\"values\":{{\"video.anti_alias\":\"{label}\"}}}}"),
        );
        let mut read = SettingsModel::new();
        SettingsFile::new(storage).load(&mut read, now());
        assert_eq!(
            value,
            read.get(SettingId::VideoAntiAlias).as_int(),
            "a file holding {label}"
        );
    }
}

/// The libretro option keeps its **value strings**: `RetroArch` persists them, and
/// `core_options::CoreOption` has one string per value, which is at once what it stores and what
/// it displays — there is no display-only field to change. So nothing changed for libretro, and
/// that is the half of the brief's alternative this lot took (Q-107 §7).
#[test]
fn ro_5_the_libretro_values_are_unchanged() {
    let option = core_options::find("zaxnes_video_anti_alias").expect("the anti-aliasing option");
    assert_eq!(["None", "FFAA", "SMAA"], option.values.as_slice());
    assert_eq!("FFAA", option.default_value);

    // And a value RetroArch persisted still applies, by that same string.
    let mut model = SettingsModel::new();
    assert!(core_options::try_apply(
        &mut model,
        "zaxnes_video_anti_alias",
        "SMAA"
    ));
    assert_eq!(2, model.get(SettingId::VideoAntiAlias).as_int());
}

/// The tooltip explains the divergence rather than hiding it: it gives the sample counts, says
/// that 3dSen calls the levels FFAA and SMAA, and says that those are the names the settings file
/// and the libretro option hold — so a user who opens either one is not surprised.
#[test]
fn ro_5_the_tooltip_names_both_the_3dsen_names_and_the_msaa_levels() {
    for language in [UiLanguage::English, UiLanguage::French] {
        strings::set_language(language);
        let tooltip = options::tooltip_of(SettingId::VideoAntiAlias);
        for fragment in ["FFAA", "SMAA", "MSAA 4×", "MSAA 8×"] {
            assert!(
                tooltip.contains(fragment),
                "{language:?}: {tooltip:?} should mention {fragment:?}"
            );
        }
        // And it no longer claims the two names describe what the user gets.
        assert!(
            !tooltip.contains("the ones the specification gives")
                && !tooltip.contains("sont ceux que la sp"),
            "{language:?}: {tooltip:?}"
        );
    }
    strings::set_language(UiLanguage::English);
}
