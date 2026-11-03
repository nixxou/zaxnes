//! RO-1 point 2: the set of settings the **desktop** options window shows may not change when the
//! window is reorganised.
//!
//! The golden `goldens/desktop-options-shown.txt` was computed from the rules of commit `0b0aa8c`
//! — the state of the window before RO-1 touched it — and is the contract the reorganisation has
//! to keep: the same keys, each shown exactly once, whatever tab or group they end up in. A key
//! that leaves the window has to leave this file in the same commit, with a reason.
//!
//! A line tagged `[wgpu]` is only shown in a build carrying that feature (`video.backend`, the
//! graphics API: see `options::is_shown`), so the comparison drops it elsewhere.

#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

mod common;
mod ui_harness;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use ui_harness::UiLanguageScope;
use zax_app::ui::handheld::advanced;
use zax_app::ui::keys;
use zax_app::ui::options::{self, OptionsModel};
use zax_app::ui::strings::{self, UiLanguage};
use zax_settings::catalogue::{self, SettingId};
use zax_settings::meta::SettingKind;

/// The tag of a line only a `wgpu` build shows.
const WGPU_TAG: &str = "[wgpu]";

fn golden_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../goldens/desktop-options-shown.txt")
}

/// The keys the golden file expects of this build.
fn expected() -> BTreeSet<String> {
    let text = std::fs::read_to_string(golden_path()).expect("the golden list of RO-1");
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| match line.split_once(char::is_whitespace) {
            Some((key, tag)) => {
                assert_eq!(WGPU_TAG, tag.trim(), "unknown tag on {key}");
                cfg!(feature = "wgpu").then(|| key.to_owned())
            }
            None => Some(line.to_owned()),
        })
        .collect()
}

/// The keys the window shows now, with the number of rows each one has.
fn shown() -> BTreeMap<String, usize> {
    let model = OptionsModel::new();
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for row in model.all_rows() {
        *counts.entry(row.descriptor.key().into_owned()).or_default() += 1;
    }
    counts
}

/// RO-1 point 2: exactly the settings of the golden list, no more and no fewer.
#[test]
fn ro_1_the_desktop_window_shows_exactly_the_settings_it_showed_before() {
    let shown = shown();
    let present: BTreeSet<String> = shown.keys().cloned().collect();
    let expected = expected();

    let gone: Vec<&String> = expected.difference(&present).collect();
    let added: Vec<&String> = present.difference(&expected).collect();
    assert_eq!(Vec::<&String>::new(), gone, "settings that disappeared");
    assert_eq!(Vec::<&String>::new(), added, "settings that appeared");
}

/// RO-1 point 2: each of them exactly once. Two rows for one setting would make the reset button
/// and the validation sound fire twice, and a compacted row (the overscan margins of point 6) must
/// not be a second row of the same key.
#[test]
fn ro_1_no_setting_is_shown_twice() {
    let counts = shown();
    let twice: Vec<(&String, &usize)> = counts.iter().filter(|(_, count)| **count > 1).collect();
    assert_eq!(Vec::<(&String, &usize)>::new(), twice);
}

/// RO-1 point 2: `EXCLUDED` is unchanged — the thirteen settings the window deliberately leaves
/// out stay out, and nothing new joins them (which is the other way a setting could vanish).
#[test]
fn ro_1_the_excluded_list_is_unchanged() {
    assert_eq!(
        [
            SettingId::UiLastBrowseDir,
            SettingId::UiWindowRect,
            SettingId::AudioBgMusic,
            SettingId::EmuHackOptions,
            SettingId::EmuBuiltinPatches,
            SettingId::EmuForcedPatch,
            SettingId::VideoWidescreen,
            SettingId::VideoWidescreenExtra,
            SettingId::VideoWidescreenAdvanced,
            SettingId::VideoOverscanTop,
            SettingId::VideoOverscanBottom,
            SettingId::VideoOverscanLeft,
            SettingId::VideoOverscanRight,
            SettingId::AchievementsUsername,
            SettingId::AchievementsToken,
            SettingId::VideoHalfRate3d,
            SettingId::InputTouchPad,
            SettingId::UiLibraryView,
            SettingId::VideoMsaa20,
        ],
        options::EXCLUDED
    );
}

/// RO-1: `LAYOUT` is the whole window, so every setting `is_shown` accepts has to be in it — and
/// in exactly one place. A setting added to the catalogue without a line in `LAYOUT` would
/// otherwise vanish silently; this is the test that sends its author to the table.
#[test]
fn ro_1_the_layout_table_places_every_shown_setting_exactly_once() {
    let placed = options::placed_settings();

    let missing: Vec<String> = catalogue::descriptors()
        .filter(|descriptor| options::is_shown(descriptor) && !placed.contains(&descriptor.id()))
        .map(|descriptor| descriptor.key().into_owned())
        .collect();
    assert_eq!(Vec::<String>::new(), missing, "settings with no row");

    for id in &placed {
        assert_eq!(
            1,
            placed.iter().filter(|other| *other == id).count(),
            "{} is placed twice",
            id.key()
        );
        // A line for a setting the window never shows would be a silent lie in the table. The
        // graphics API is the one exception: `is_shown` drops it off a `wgpu` build.
        assert!(
            options::is_shown(catalogue::descriptor(*id)) || *id == SettingId::VideoBackend,
            "{} is in LAYOUT but never shown",
            id.key()
        );
    }

    // Every group heading of the table has at least one row under it.
    for (tab, entries) in options::LAYOUT {
        for (index, entry) in entries.iter().enumerate() {
            if let options::LayoutEntry::Group(group) = entry {
                assert!(
                    entries
                        .get(index + 1)
                        .is_some_and(|next| !next.settings().is_empty()),
                    "{group:?} opens nothing on {tab:?}"
                );
            }
        }
    }
}

/// RO-1 point 6: the eight overscan margins are shown as **two** rows, one per region, and the
/// "nothing disappears" test above still sees all eight.
#[test]
fn ro_1_the_overscan_margins_are_two_rows_of_four_values() {
    use zax_app::ui::options::{CompactRow, LayoutEntry, TabEntry};

    // The layout places them as two compact entries and no longer as eight rows of their own.
    let entries: Vec<LayoutEntry> = options::LAYOUT
        .iter()
        .find(|(tab, _)| *tab == options::OptionsTab::NesPicture)
        .map(|(_, entries)| entries.to_vec())
        .expect("the NES picture tab");
    assert!(entries.contains(&LayoutEntry::Compact(CompactRow::OverscanNtsc)));
    assert!(entries.contains(&LayoutEntry::Compact(CompactRow::OverscanPal)));
    for id in CompactRow::OverscanNtsc
        .settings()
        .into_iter()
        .chain(CompactRow::OverscanPal.settings())
    {
        assert!(
            !entries.contains(&LayoutEntry::Row(id)),
            "{} still has a row of its own",
            id.key()
        );
        // And each one is still a setting of the window, in the Overscan group.
        assert_eq!(Some(options::OptionsTab::NesPicture), options::tab_of(id));
        assert_eq!(Some(options::OptionsGroup::Overscan), options::group_of(id));
    }

    // The built tab draws two compact entries, each holding its four cells in the order top,
    // bottom, left, right.
    let model = OptionsModel::new();
    let compact: Vec<(CompactRow, [options::OptionsRow; 4])> = model
        .entries(options::OptionsTab::NesPicture)
        .iter()
        .filter_map(|entry| match entry {
            TabEntry::Compact(compact, cells) => Some((*compact, *cells)),
            _ => None,
        })
        .collect();
    assert_eq!(2, compact.len());
    for (row, cells) in compact {
        let ids: Vec<SettingId> = cells.iter().map(|cell| cell.descriptor.id()).collect();
        assert_eq!(row.settings().to_vec(), ids);
        // Each of the four is a drop-down of the same 33 values it had as a row of its own, and
        // each carries its own tooltip.
        for cell in &cells {
            assert_eq!(
                33,
                cell.descriptor
                    .domain()
                    .options()
                    .expect("0 to 32 pixels")
                    .len()
            );
            assert!(options::tooltip_of(cell.descriptor.id()).len() > 80);
        }
    }

    // A caption for every cell, in both languages, and a title for both rows.
    let _language = UiLanguageScope::keep();
    for language in [UiLanguage::English, UiLanguage::French] {
        strings::set_language(language);
        for caption in options::COMPACT_CAPTIONS {
            assert!(strings::lookup(language, caption).is_some(), "{caption}");
        }
        assert_ne!("", CompactRow::OverscanNtsc.title());
        assert_ne!("", CompactRow::OverscanPal.title());
    }
    strings::set_language(UiLanguage::English);

    // The reset button of the tab still reaches all eight (they are rows of the tab).
    let rows: Vec<SettingId> = model
        .rows(options::OptionsTab::NesPicture)
        .iter()
        .map(|row| row.descriptor.id())
        .collect();
    for id in CompactRow::OverscanNtsc
        .settings()
        .into_iter()
        .chain(CompactRow::OverscanPal.settings())
    {
        assert!(rows.contains(&id), "{} is a row of the tab", id.key());
    }
}

/// RO-1 point 1: every reason a row is greyed out names a setting the player can reach, and the
/// greyed row keeps its value. Each case is read from the code path that consumes the setting —
/// the comments of `options::inert_reason` name them — so this test pins the trigger, not the
/// wording.
#[test]
fn ro_1_a_row_is_greyed_only_while_its_dependency_is_unmet() {
    use zax_settings::model::SettingsModel;
    use zax_settings::values::SettingValue;

    // A fresh model: shadows on, direct rendering, Classic, no curvature, automatic region off…
    let mut model = SettingsModel::new();

    // The one row that is greyed whatever anyone does: there is a single core (REQ-PERS-003).
    assert!(options::inert_reason(SettingId::EmuCore, &model).is_some());

    // Direct rendering greys the resolution scale and the automatic resolution (AMD-032 §3).
    assert!(options::inert_reason(SettingId::VideoResScale, &model).is_some());
    assert!(options::inert_reason(SettingId::VideoDynamicResolution, &model).is_some());
    model.set(SettingId::VideoRenderMethod, SettingValue::Enum(1));
    assert_eq!(
        None,
        options::inert_reason(SettingId::VideoDynamicResolution, &model)
    );
    assert_eq!(
        None,
        options::inert_reason(SettingId::VideoResScale, &model)
    );
    // …until the automatic resolution takes the choice over.
    model.set(SettingId::VideoDynamicResolution, SettingValue::Bool(true));
    assert!(options::inert_reason(SettingId::VideoResScale, &model).is_some());
    model.set(SettingId::VideoDynamicResolution, SettingValue::Bool(false));

    // The pixel shadows need the shadows.
    assert_eq!(
        None,
        options::inert_reason(SettingId::RenderStylePixelShadow, &model)
    );
    model.set(SettingId::RenderShadow, SettingValue::Bool(false));
    assert!(options::inert_reason(SettingId::RenderStylePixelShadow, &model).is_some());
    model.set(SettingId::RenderShadow, SettingValue::Bool(true));

    // The experimental Lua and the pipelined frames need the optimised Lua.
    for id in [SettingId::EmuLuaExperimental, SettingId::EmuPipeline] {
        assert!(options::inert_reason(id, &model).is_some(), "{id:?}");
    }
    model.set(SettingId::EmuLuaOptimized, SettingValue::Bool(true));
    for id in [SettingId::EmuLuaExperimental, SettingId::EmuPipeline] {
        assert_eq!(None, options::inert_reason(id, &model), "{id:?}");
    }

    // The custom background colour reaches neither the gradient nor the fantasy sky.
    for skybox in [-1, 0] {
        model.set(SettingId::SceneSkybox, SettingValue::Enum(skybox));
        assert_eq!(
            None,
            options::inert_reason(SettingId::SceneCustomBgColor, &model),
            "skybox {skybox}"
        );
    }
    for skybox in [1, 2] {
        model.set(SettingId::SceneSkybox, SettingValue::Enum(skybox));
        assert!(
            options::inert_reason(SettingId::SceneCustomBgColor, &model).is_some(),
            "skybox {skybox}"
        );
    }

    // The arc centre needs an arc; the 2D-mode switch needs some curvature.
    assert!(options::inert_reason(SettingId::SceneArcCenter, &model).is_some());
    assert!(options::inert_reason(SettingId::SceneBend2dMode, &model).is_some());
    model.set(SettingId::SceneArcH, SettingValue::Int(30));
    assert_eq!(
        None,
        options::inert_reason(SettingId::SceneArcCenter, &model)
    );
    assert_eq!(
        None,
        options::inert_reason(SettingId::SceneBend2dMode, &model)
    );
    model.set(SettingId::SceneArcH, SettingValue::Int(0));
    model.set(SettingId::SceneBendLr, SettingValue::Int(-30));
    assert_eq!(
        None,
        options::inert_reason(SettingId::SceneBend2dMode, &model),
        "either angle is enough"
    );

    // The region is overwritten by the detection, so the row says so rather than lying.
    assert_eq!(None, options::inert_reason(SettingId::EmuTvMode, &model));
    model.set(SettingId::EmuAutoTvMode, SettingValue::Bool(true));
    assert!(options::inert_reason(SettingId::EmuTvMode, &model).is_some());

    // The four RetroAchievements options need the switch.
    let achievements = [
        SettingId::AchievementsEncore,
        SettingId::AchievementsUnofficial,
        SettingId::AchievementsSound,
        SettingId::AchievementsChallenges,
    ];
    for id in achievements {
        assert!(options::inert_reason(id, &model).is_some(), "{id:?}");
    }
    model.set(SettingId::AchievementsEnabled, SettingValue::Bool(true));
    for id in achievements {
        assert_eq!(None, options::inert_reason(id, &model), "{id:?}");
    }
    // The switch itself is never greyed.
    assert_eq!(
        None,
        options::inert_reason(SettingId::AchievementsEnabled, &model)
    );

    // Greying is a presentation: the value of a greyed row is written and read back all the same.
    model.set(SettingId::EmuAutoTvMode, SettingValue::Bool(true));
    model.set(SettingId::EmuTvMode, SettingValue::Enum(1));
    assert_eq!(1, model.get(SettingId::EmuTvMode).as_int());
}

/// RO-1 point 3: **every** setting the desktop window shows has a detailed tooltip, in English and
/// in French — several sentences, not a label repeated.
///
/// The parity test of AMD-035 already forbids a key in one table and not the other; what this one
/// adds is that no shown row is left without a paragraph, and that the paragraph is the amendment's
/// sentence *plus* the detail where the amendment wrote one.
#[test]
fn ro_1_every_shown_setting_has_a_detailed_tooltip_in_both_languages() {
    let _language = UiLanguageScope::keep();
    let model = OptionsModel::new();

    for language in [UiLanguage::English, UiLanguage::French] {
        strings::set_language(language);
        for row in model.all_rows() {
            let id = row.descriptor.id();
            let key = keys::detail(&row.descriptor.key());
            assert!(
                strings::lookup(language, &key).is_some(),
                "{key} is missing from the {language:?} table"
            );

            let tooltip = options::tooltip_of(id);
            assert!(
                tooltip.len() > 80,
                "{language:?} {}: a tooltip of {} characters is not a few sentences — {tooltip:?}",
                row.descriptor.key(),
                tooltip.len()
            );
            // It says more than the label does.
            assert_ne!(tooltip.trim(), row.label(), "{}", row.descriptor.key());
            // And it never shows a raw key.
            assert!(!tooltip.contains(keys::DETAIL_PREFIX), "{tooltip}");
            assert!(!tooltip.contains("help."), "{tooltip}");

            // Where an amendment gave a sentence, the tooltip opens with it, word for word, and
            // the detail follows after a blank line rather than replacing it.
            if let Some(help) = options::help_of(id) {
                assert!(
                    tooltip.starts_with(&help),
                    "{}: the amendment's sentence is no longer first",
                    row.descriptor.key()
                );
                assert!(
                    tooltip.len() > help.len() + 80,
                    "{}: the amendment's sentence was not extended",
                    row.descriptor.key()
                );
            }
        }
    }
    strings::set_language(UiLanguage::English);
}

/// RO-1 point 3: and no `detail.*` key is left behind either — one that no longer names a shown
/// setting is a text nobody will ever read, and the sign of a row that was removed or renamed.
#[test]
fn ro_1_every_detail_text_belongs_to_a_shown_setting() {
    let shown: BTreeSet<String> = OptionsModel::new()
        .all_rows()
        .map(|row| keys::detail(&row.descriptor.key()))
        .collect();

    let orphans: Vec<&str> = strings::english_table()
        .iter()
        .map(|(key, _)| *key)
        .filter(|key| key.starts_with(keys::DETAIL_PREFIX) && !shown.contains(*key))
        // The graphics API has its text in every build; only a `wgpu` build shows the row.
        .filter(|key| *key != "detail.video.backend")
        .collect();
    assert_eq!(Vec::<&str>::new(), orphans);
}

/// RO-1 point 3: nothing of the tooltips reaches the console pages or Android. They are read
/// through `detail.*`, and the only caller is the desktop window.
#[test]
fn ro_1_the_detailed_tooltips_are_desktop_only() {
    let sources = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut callers: Vec<String> = Vec::new();
    for entry in walkdir::WalkDir::new(&sources)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "rs"))
    {
        let text = std::fs::read_to_string(entry.path()).expect("a source file");
        // Code, not prose: the two string tables name `tooltip_of` in the comment that explains
        // what their `detail.*` entries are for, and that is not a caller.
        let calls = text
            .lines()
            .map(str::trim_start)
            .filter(|line| !line.starts_with("//"))
            .any(|line| line.contains("tooltip_of") || line.contains("keys::detail("));
        if calls {
            callers.push(
                entry
                    .path()
                    .strip_prefix(&sources)
                    .expect("under src")
                    .display()
                    .to_string()
                    .replace('\\', "/"),
            );
        }
    }
    callers.sort();
    // `ui/options.rs` builds the text and `ui/zax_ui/options.rs` is the desktop window that shows
    // it. Nothing under `ui/handheld/`, and nothing in the Android shell.
    assert_eq!(vec!["ui/options.rs", "ui/zax_ui/options.rs"], callers);
}

/// RO-1 point 2: the console's "Advanced options" page shows exactly the rows it showed before,
/// in the same order. It used to be built by walking four tabs of the desktop window, so this is
/// the test that makes the desktop reorganisation invisible to it.
#[test]
fn ro_1_the_handheld_advanced_page_is_unchanged() {
    let text = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../goldens/handheld-advanced-rows.txt"),
    )
    .expect("the golden list of the console's advanced page");
    let expected: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();

    let shown: Vec<String> = advanced::ROW_SETTINGS
        .iter()
        .map(|id| id.key().into_owned())
        .collect();
    assert_eq!(expected, shown);

    // And every one of them is still a row the page can actually draw and step: the rule the
    // computed list applied before — a switch, a choice, or a number with bounds or named rungs —
    // and never the graphics API, which belongs to the desktop alone.
    let model = OptionsModel::new();
    for id in advanced::ROW_SETTINGS {
        assert_ne!(SettingId::VideoBackend, id);
        let row = model
            .all_rows()
            .find(|row| row.descriptor.id() == id)
            .unwrap_or_else(|| panic!("{} is not a row of the window", id.key()));
        assert!(!options::is_folder(row), "{}", id.key());
        let steppable = match row.descriptor.kind() {
            SettingKind::Bool | SettingKind::Enum => true,
            SettingKind::Int | SettingKind::Float => {
                row.maximum > row.minimum
                    || row
                        .descriptor
                        .domain()
                        .options()
                        .is_some_and(|options| options.len() > 1)
            }
            _ => false,
        };
        assert!(steppable, "{}", id.key());
    }
}

/// RO-1 point 2: the console's own pages — the main one, the Android one and the performance one —
/// are untouched. They are plain constants, so the check is their content, row for row.
#[test]
fn ro_1_the_handheld_option_pages_are_unchanged() {
    use zax_app::ui::handheld::options::{OptionRow, OPTIMIZATION_ROWS, ROWS, TOUCH_ROWS};

    assert_eq!(
        [
            OptionRow::Language,
            OptionRow::Volume,
            OptionRow::RenderScale,
            OptionRow::Shadows,
            OptionRow::Palette,
            OptionRow::Overscan,
            OptionRow::ShowFps,
            OptionRow::Camera,
            OptionRow::Region,
            OptionRow::Optimizations,
            OptionRow::Advanced,
            OptionRow::Controls,
            OptionRow::Achievements,
            OptionRow::Profiler,
        ],
        ROWS
    );
    assert_eq!(
        [
            OptionRow::Language,
            OptionRow::Volume,
            OptionRow::RenderScale,
            OptionRow::Shadows,
            OptionRow::Palette,
            OptionRow::Overscan,
            OptionRow::ShowFps,
            OptionRow::Camera,
            OptionRow::Region,
            OptionRow::Optimizations,
            OptionRow::Advanced,
            OptionRow::Controls,
            OptionRow::TouchPad,
            OptionRow::Achievements,
            OptionRow::Profiler,
        ],
        TOUCH_ROWS
    );
    assert_eq!(
        [
            OptionRow::FrameCap,
            OptionRow::HalfRate3d,
            OptionRow::CatchUp,
            OptionRow::CatchUpLast3d,
            OptionRow::Pause3dIn2d,
            OptionRow::Pipeline,
            OptionRow::LuaOptimized,
            OptionRow::LuaExperimental,
        ],
        OPTIMIZATION_ROWS
    );
}

/// The inventory of step 1, printed rather than asserted: run with `--nocapture` to read the
/// layout the window has now (tab, order, group). It is what the table of the progress file and
/// the old-tab column of `docs/options-layout-divergence.md` are built from.
#[test]
fn ro_1_print_the_layout() {
    let model = OptionsModel::new();
    for tab in options::TABS {
        println!("== {tab:?}");
        for (index, row) in model.rows(tab).iter().enumerate() {
            let id = row.descriptor.id();
            let group = if options::GEOMETRY.contains(&id) {
                " (Geometry)"
            } else if options::STYLE.contains(&id) {
                " (Style)"
            } else {
                ""
            };
            println!("  {index:3} {}{group}", row.descriptor.key());
        }
    }
    println!("== handheld advanced page");
    for id in advanced::ROW_SETTINGS {
        println!("  {}", id.key());
    }
}
