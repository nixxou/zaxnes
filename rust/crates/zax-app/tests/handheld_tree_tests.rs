//! RO-3: the console option pages as the owner's mockup lays them out — a main page, an
//! Optimisations page, the overscan screen and an *Advanced options* hub of six pages, two of
//! which have a page of their own.
//!
//! The lot moves 56 flat rows into that tree and moves nothing else, so the tests that matter are
//! the two the brief makes mandatory: **everything the console offered at `336dd8f` is still
//! reachable**, and **no row is reachable twice**. Both are walks of the tree from the main page,
//! so a page the lot forgot to hang anywhere fails them.

use std::collections::BTreeMap;

use zax_app::ui::handheld::game_options::{self, GameRow};
use zax_app::ui::handheld::options::{OptionRow, OptionsPage, ROWS, TOUCH_ROWS};
use zax_app::ui::handheld::{advanced, overscan};
use zax_settings::catalogue::SettingId;

/// A stable name for a row, for the golden and for the failure messages: a setting row is its
/// settings-file key, every other row a name of its own that no translation can move.
fn row_key(row: OptionRow) -> String {
    match row {
        OptionRow::Setting(id) | OptionRow::Choice(id) => id.key().into_owned(),
        OptionRow::Group(key) => format!("[{key}]"),
        OptionRow::Page(page) => format!("page.{}", page_name(page)),
        OptionRow::Language => "row.language".to_owned(),
        OptionRow::Volume => "row.volume".to_owned(),
        OptionRow::RenderScale => "row.render_scale".to_owned(),
        OptionRow::Shadows => "row.shadows".to_owned(),
        OptionRow::Palette => "row.palette".to_owned(),
        OptionRow::Overscan => "page.overscan".to_owned(),
        OptionRow::FrameCap => "row.frame_cap".to_owned(),
        OptionRow::ShowFps => "row.show_fps".to_owned(),
        OptionRow::Profiler => "row.profiler".to_owned(),
        OptionRow::Camera => "row.camera".to_owned(),
        OptionRow::CatchUp => "row.catch_up".to_owned(),
        OptionRow::LuaOptimized => "row.lua_optimized".to_owned(),
        OptionRow::LuaExperimental => "row.lua_experimental".to_owned(),
        OptionRow::Pipeline => "row.pipeline".to_owned(),
        OptionRow::Pause3dIn2d => "row.pause_3d_in_2d".to_owned(),
        OptionRow::CatchUpLast3d => "row.catch_up_last_3d".to_owned(),
        OptionRow::HalfRate3d => "row.half_rate_3d".to_owned(),
        OptionRow::Region => "row.region".to_owned(),
        OptionRow::Controls => "page.controls".to_owned(),
        OptionRow::Achievements => "page.achievements".to_owned(),
        OptionRow::TouchPad => "row.touch_pad".to_owned(),
        OptionRow::Shader(slot) => format!("row.shader.{}", slot.ids().mode.key()),
        OptionRow::ShaderBackend => "video.backend".to_owned(),
        OptionRow::ShaderPreFilter(slot) => {
            format!("row.shader_pre_filter.{}", slot.ids().preset.key())
        }
    }
}

fn page_name(page: OptionsPage) -> &'static str {
    match page {
        OptionsPage::Optimizations => "optimizations",
        OptionsPage::Advanced => "advanced",
        OptionsPage::Display => "display",
        OptionsPage::Rendering3d => "rendering_3d",
        OptionsPage::EnhancedEffects => "enhanced_effects",
        OptionsPage::Geometry => "geometry",
        OptionsPage::LightShadows => "light_shadows",
        OptionsPage::NesPicture => "nes_picture",
        OptionsPage::Camera => "camera",
        OptionsPage::Emulation => "emulation",
        OptionsPage::Audio => "audio",
        OptionsPage::Shaders => "shaders",
        OptionsPage::Touch => "touch",
    }
}

/// Which tree is walked: the console's global options, or a game's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tree {
    Global,
    Game,
}

impl Tree {
    fn rows_of(self, page: OptionsPage) -> &'static [OptionRow] {
        match self {
            Self::Global => page.rows(),
            Self::Game => page.game_rows(),
        }
    }

    /// The rows of the first page, the game's specials dropped (they open no page and hold no
    /// setting of the options).
    fn main(self) -> Vec<OptionRow> {
        match self {
            Self::Global => ROWS.to_vec(),
            Self::Game => game_options::GAME_MAIN
                .iter()
                .filter_map(|row| match row {
                    GameRow::Option(row) => Some(*row),
                    GameRow::ResetAll | GameRow::Widescreen | GameRow::Hacks => None,
                })
                .collect(),
        }
    }
}

/// Every row of the tree, with the path of page names that leads to it. Walks pages depth first,
/// in the order each page lists them, so the result is the order the mockup reads in.
fn walk(tree: Tree) -> Vec<(Vec<&'static str>, OptionRow)> {
    let mut out = Vec::new();
    descend(tree, &mut vec!["main"], &tree.main(), &mut out);
    out
}

fn descend(
    tree: Tree,
    path: &mut Vec<&'static str>,
    rows: &[OptionRow],
    out: &mut Vec<(Vec<&'static str>, OptionRow)>,
) {
    for row in rows {
        out.push((path.clone(), *row));
        if let OptionRow::Page(page) = row {
            let name = page_name(*page);
            assert!(
                !path.contains(&name),
                "the page {name} is reachable from itself: {path:?}"
            );
            path.push(name);
            let rows = tree.rows_of(*page);
            assert!(!rows.is_empty(), "the page {name} of {tree:?} has no row");
            descend(tree, path, rows, out);
            path.pop();
        }
    }
}

/// The settings a row of the console's **global** tree offers. A plain row is named by hand: the
/// per-game `row_settings` answers nothing for the rows a game cannot have (the language, the
/// camera) and this walk has to see those too.
fn settings_of(row: OptionRow) -> Vec<SettingId> {
    match row {
        OptionRow::Setting(id) => vec![id],
        // The camera view asks the application for a preset, the controls open their own screen
        // and a title or a page row holds nothing: none of them is a setting of the model.
        OptionRow::Group(_) | OptionRow::Page(_) | OptionRow::Controls | OptionRow::Camera => {
            Vec::new()
        }
        OptionRow::Overscan => overscan::GLOBAL_ROWS.to_vec(),
        OptionRow::Language => vec![SettingId::UiLanguage],
        other => game_options::row_settings(other).to_vec(),
    }
}

/// The six settings that two rows each offer, and why.
///
/// The brief's rule reads "no setting other than `video.res_scale` is reachable from two places",
/// but it is a rule about **rows**: no row appears twice in the tree. What does appear twice is a
/// setting a *composite* row moves together with others, where the mockup also asks for the
/// `Setting` row of that very setting on a page. There are three such rows, every one of them a
/// row the mockup keeps where it was:
///
/// | Composite row | Its settings | The `Setting` rows of the same settings |
/// |---|---|---|
/// | Render scale (main page) | `video.res_scale`, `video.dynamic_resolution`, `video.render_method` | Display › Resolution, all three |
/// | Region (main page) | `emu.auto_tv_mode`, `emu.tv_mode` | Emulation, both |
/// | Frame cap (Optimisations) | `video.vsync` | Display |
///
/// The brief's own *Display* and *Emulation* pages create the second and the third of these, so
/// the one-sentence rule cannot be read literally; this is the set it means. Recorded for R1 in
/// Q-107 §6.
const TWO_PLACES: [SettingId; 6] = [
    SettingId::VideoResScale,
    SettingId::VideoDynamicResolution,
    SettingId::VideoRenderMethod,
    SettingId::EmuAutoTvMode,
    SettingId::EmuTvMode,
    SettingId::VideoVSync,
];

/// Mandatory coverage test, first half: every row the console offered at `336dd8f` is still
/// reachable from the main page — the main page's own, the Android one's, the Optimisations page's,
/// the 56 of the flat *Advanced options* page and the overscan screen's.
#[test]
fn ro_3_every_row_the_console_offered_is_still_reachable() {
    let reachable: Vec<String> = walk(Tree::Global)
        .into_iter()
        .map(|(_, row)| row_key(row))
        .collect();
    let mut missing = Vec::new();
    for row in rows_of_336dd8f() {
        let key = row_key(row);
        if !reachable.contains(&key) {
            missing.push(key);
        }
    }
    assert!(missing.is_empty(), "no longer reachable: {missing:?}");

    // The Android main page is the main page, the orientation and the touch screen's page, which
    // holds the touch-pad row (owner request: the pad's size, vibrations and extra buttons).
    let touch = OptionRow::Page(OptionsPage::Touch);
    let orientation = OptionRow::Choice(SettingId::UiOrientation);
    assert_eq!(
        TOUCH_ROWS.len(),
        ROWS.len() + 2,
        "the Android page is the main page, the orientation and the touch screen's page"
    );
    assert!(TOUCH_ROWS.contains(&touch) && TOUCH_ROWS.contains(&orientation));
    assert!(!ROWS.contains(&touch));
    assert!(OptionsPage::Touch.rows().contains(&OptionRow::TouchPad));
    assert!(!OptionsPage::Touch.rows().contains(&orientation));
    let without: Vec<OptionRow> = TOUCH_ROWS
        .into_iter()
        .filter(|row| *row != touch && *row != orientation)
        .collect();
    assert_eq!(ROWS.to_vec(), without);

    // Every setting the console offered is still offered, which is the same claim read of the
    // settings rather than of the rows — the overscan's nine included.
    let offered: Vec<SettingId> = walk(Tree::Global)
        .into_iter()
        .flat_map(|(_, row)| settings_of(row))
        .collect();
    for id in advanced::ROW_SETTINGS {
        assert!(offered.contains(&id), "{} is not offered", id.key());
    }
    for id in overscan::GLOBAL_ROWS {
        assert!(offered.contains(&id), "{} is not offered", id.key());
    }
}

/// The rows of the console's main page, Android page and Optimisations page at `336dd8f`, written
/// out so that the test does not depend on the lists this lot rewrites. The 56 of the flat
/// *Advanced options* page are not here: several of their settings are offered by a row of the
/// interface's own rather than by a `Setting` row (the palette, the shadows, the catch-up, the
/// frame cap, the overscan screen), so they are checked setting by setting instead.
fn rows_of_336dd8f() -> Vec<OptionRow> {
    vec![
        OptionRow::Language,
        OptionRow::Volume,
        OptionRow::RenderScale,
        OptionRow::Shadows,
        OptionRow::Palette,
        OptionRow::Overscan,
        OptionRow::ShowFps,
        OptionRow::Camera,
        OptionRow::Region,
        OptionRow::Controls,
        OptionRow::Achievements,
        OptionRow::Profiler,
        // the Optimizations page of `336dd8f`
        OptionRow::FrameCap,
        OptionRow::HalfRate3d,
        OptionRow::CatchUp,
        OptionRow::CatchUpLast3d,
        OptionRow::Pause3dIn2d,
        OptionRow::Pipeline,
        OptionRow::LuaOptimized,
        OptionRow::LuaExperimental,
    ]
}

/// Mandatory coverage test, second half: no row is reachable from two places, and the only
/// settings two rows each offer are the five of [`TWO_PLACES`].
#[test]
fn ro_3_no_row_is_reachable_from_two_places() {
    let mut places: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (path, row) in walk(Tree::Global) {
        // A group title is a label, not a row: the same title may head a group on two pages.
        if matches!(row, OptionRow::Group(_)) {
            continue;
        }
        places
            .entry(row_key(row))
            .or_default()
            .push(path.join(" › "));
    }
    let twice: Vec<(&String, &Vec<String>)> =
        places.iter().filter(|(_, paths)| paths.len() > 1).collect();
    assert!(twice.is_empty(), "reachable twice: {twice:?}");

    let mut per_setting: BTreeMap<SettingId, Vec<String>> = BTreeMap::new();
    for (_, row) in walk(Tree::Global) {
        for id in settings_of(row) {
            per_setting.entry(id).or_default().push(row_key(row));
        }
    }
    let mut shared: Vec<SettingId> = per_setting
        .iter()
        .filter(|(_, rows)| rows.len() > 1)
        .map(|(id, _)| *id)
        .collect();
    shared.sort_by_key(|id| id.key().into_owned());
    let mut expected = TWO_PLACES.to_vec();
    expected.sort_by_key(|id| id.key().into_owned());
    assert_eq!(
        expected
            .iter()
            .map(|id| id.key().into_owned())
            .collect::<Vec<_>>(),
        shared
            .iter()
            .map(|id| id.key().into_owned())
            .collect::<Vec<_>>(),
        "the settings two rows each offer"
    );
}

/// The same two claims for the **game** tree: the 30 rows a game could have at `336dd8f` are all
/// still reachable, each from one place.
#[test]
fn ro_3_the_game_tree_keeps_every_row_a_game_could_have() {
    let reachable: Vec<String> = walk(Tree::Game)
        .into_iter()
        .map(|(_, row)| row_key(row))
        .collect();
    let mut missing = Vec::new();
    for row in game_rows_of_336dd8f() {
        let key = row_key(row);
        if !reachable.contains(&key) {
            missing.push(key);
        }
    }
    assert!(missing.is_empty(), "no longer reachable: {missing:?}");

    let mut places: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (path, row) in walk(Tree::Game) {
        if matches!(row, OptionRow::Group(_)) {
            continue;
        }
        places
            .entry(row_key(row))
            .or_default()
            .push(path.join(" › "));
    }
    let twice: Vec<(&String, &Vec<String>)> =
        places.iter().filter(|(_, paths)| paths.len() > 1).collect();
    assert!(twice.is_empty(), "reachable twice: {twice:?}");

    // Every row of the game tree is a row a game can really have an option for.
    for (path, row) in walk(Tree::Game) {
        if matches!(row, OptionRow::Group(_) | OptionRow::Page(_)) {
            continue;
        }
        assert!(
            !game_options::row_settings(row).is_empty(),
            "{} at {path:?} names no setting",
            row_key(row)
        );
    }
}

/// The 30 rows of `GAME_ROWS` and `GAME_ADVANCED_ROWS` at `336dd8f`.
fn game_rows_of_336dd8f() -> Vec<OptionRow> {
    vec![
        OptionRow::Volume,
        OptionRow::RenderScale,
        OptionRow::Shadows,
        OptionRow::Palette,
        OptionRow::Overscan,
        OptionRow::FrameCap,
        OptionRow::ShowFps,
        OptionRow::Profiler,
        OptionRow::CatchUp,
        OptionRow::CatchUpLast3d,
        OptionRow::HalfRate3d,
        OptionRow::LuaOptimized,
        OptionRow::LuaExperimental,
        OptionRow::Pipeline,
        OptionRow::Pause3dIn2d,
        OptionRow::Region,
        OptionRow::Setting(SettingId::EmuRunAhead),
        OptionRow::Setting(SettingId::VideoAntiAlias),
        OptionRow::Setting(SettingId::VideoMaxQueuedFrames),
        OptionRow::Setting(SettingId::RenderVisualStyle),
        OptionRow::Setting(SettingId::RenderStyleVoxelGrid),
        OptionRow::Setting(SettingId::RenderStylePixelShadow),
        OptionRow::Setting(SettingId::RenderStyleAo),
        OptionRow::Setting(SettingId::RenderStyleReflections),
        OptionRow::Setting(SettingId::RenderBackdropLighting),
        OptionRow::Setting(SettingId::SceneBendLr),
        OptionRow::Setting(SettingId::SceneBendTb),
        OptionRow::Setting(SettingId::SceneArcH),
        OptionRow::Setting(SettingId::SceneArcCenter),
        OptionRow::Setting(SettingId::SceneBend2dMode),
    ]
}

/// Every `Setting` row of either tree names a setting of [`advanced::ROW_SETTINGS`], which is what
/// gives it its widget bounds: a page that showed a setting outside that list would have no step
/// to move it by.
#[test]
fn ro_3_every_setting_row_of_the_tree_is_a_known_console_setting() {
    for tree in [Tree::Global, Tree::Game] {
        for (path, row) in walk(tree) {
            if let OptionRow::Setting(id) = row {
                assert!(
                    advanced::ROW_SETTINGS.contains(&id),
                    "{} at {path:?} of {tree:?} is not a console setting",
                    id.key()
                );
            }
        }
    }
    // And the list holds nothing the console no longer shows: every one of the 56 is offered by
    // the global tree, as a `Setting` row or through a row of the interface's own.
    let offered: Vec<SettingId> = walk(Tree::Global)
        .into_iter()
        .flat_map(|(_, row)| settings_of(row))
        .collect();
    for id in advanced::ROW_SETTINGS {
        assert!(
            offered.contains(&id),
            "{} is on no page of the tree",
            id.key()
        );
    }
}

/// The golden of the whole tree, page › group › row, for R0 to check against the mockup. It
/// replaces `goldens/handheld-advanced-rows.txt`, which pinned the one flat page this lot splits.
#[test]
fn ro_3_the_option_tree_matches_its_golden() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../goldens/handheld-option-tree.txt");
    let golden = std::fs::read_to_string(&path).expect("the golden of the option tree");
    let drawn = tree_text();
    let expected: Vec<&str> = golden
        .lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .collect();
    let drawn_lines: Vec<&str> = drawn.lines().collect();
    assert_eq!(
        expected.join("\n"),
        drawn_lines.join("\n"),
        "the option tree moved; regenerate goldens/handheld-option-tree.txt"
    );
}

/// Writes the golden from the tree as it is. Ignored: it is the regeneration aid for a deliberate
/// change of the layout, and R0 checks the result against the owner's mockup.
/// `cargo test -p zax-app --test handheld_tree_tests -- --ignored write_the_option_tree_golden`
#[test]
#[ignore = "regenerates goldens/handheld-option-tree.txt"]
fn ro_3_write_the_option_tree_golden() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../goldens/handheld-option-tree.txt");
    let header = "\
# The console option tree, page by page, as of RO-3 (the owner's mockup).
#
# Two spaces per level; the rows under a group title are indented once more under it, and the
# rows of the overscan screen under the row that opens it. A `page.*` line is a row that climbs
# to a page, a `[group.*]` line a non-selectable title, a `row.*` line a row of the interface's
# own and anything else the key of a setting of the settings file.
#
# It replaces `handheld-advanced-rows.txt`, which pinned the one flat page RO-3 split.
# Regenerate with the ignored test `ro_3_write_the_option_tree_golden`.
# Checked by `crates/zax-app/tests/handheld_tree_tests.rs`.

";
    std::fs::write(&path, format!("{header}{}", tree_text())).expect("writing the golden");
    println!("wrote {}", path.display());
}

/// The tree as the golden writes it: one line per row, two spaces per level, the rows that follow
/// a group title indented once more under it, and the rows of the overscan screen under the row
/// that opens it.
fn tree_text() -> String {
    let mut out = String::from("options\n");
    emit(Tree::Global, &ROWS, 1, &mut out);
    out.push_str("game options\n");
    for row in game_options::GAME_MAIN {
        match row {
            GameRow::ResetAll => line(&mut out, 1, "row.reset_all"),
            GameRow::Widescreen => line(&mut out, 1, "row.widescreen"),
            GameRow::Hacks => line(&mut out, 1, "page.hacks"),
            GameRow::Option(row) => emit(Tree::Game, &[row], 1, &mut out),
        }
    }
    out
}

fn line(out: &mut String, depth: usize, text: &str) {
    for _ in 0..depth {
        out.push_str("  ");
    }
    out.push_str(text);
    out.push('\n');
}

fn emit(tree: Tree, rows: &[OptionRow], depth: usize, out: &mut String) {
    let mut in_group = false;
    for row in rows {
        let group = matches!(row, OptionRow::Group(_));
        in_group |= group;
        line(out, depth + usize::from(in_group && !group), &row_key(*row));
        match row {
            OptionRow::Page(page) => emit(tree, tree.rows_of(*page), depth + 1, out),
            OptionRow::Overscan => {
                let sides: &[SettingId] = match tree {
                    Tree::Global => &overscan::GLOBAL_ROWS,
                    Tree::Game => &overscan::GAME_ROWS,
                };
                for id in sides {
                    line(out, depth + 1, &id.key());
                }
            }
            _ => {}
        }
    }
}
