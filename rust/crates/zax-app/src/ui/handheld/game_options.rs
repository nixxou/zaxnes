//! The options of one game as data (Rust-only, owner request): the rows of the options that make
//! sense for a single ROM, each either the global value ("Use Global") or one of its own, and a
//! first row that puts every one back to the global value. Only the settings that differ from
//! the global ones are kept.

use std::path::PathBuf;

use zax_settings::catalogue::SettingId;
use zax_settings::games::GameOptions;
use zax_settings::model::SettingsModel;

use super::options::{self, OptionRow};
use super::overlay::CameraPreset;
use super::pad::PadButton;
use crate::ui::{keys, strings};

/// The rows a game can have a value of its own for: the options, without the language, the
/// camera and the bindings, which belong to the player rather than to the game.
///
/// These are the economical ones — the main page of the console, the first level of the desktop
/// menu. The ones RO-2 added are in [`GAME_ADVANCED_ROWS`], apart, because every one of them is
/// a row of the console's *global* advanced page (`super::advanced::ROW_SETTINGS`) and the brief
/// asks each interface to keep its own habit about those.
pub const GAME_ROWS: [OptionRow; 16] = [
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
];

/// The rows RO-2 added that belong to the picture as it reaches the screen: the run-ahead, whose
/// right value depends on each game's own lag (Q-103 still holds it at 0 for a profile whose
/// scripts change the core), the anti-aliasing and the depth of the GPU frame queue. The desktop
/// menu draws these three at the level of [`GAME_ROWS`].
pub const GAME_DISPLAY_ROWS: [OptionRow; 3] = [
    OptionRow::Setting(SettingId::EmuRunAhead),
    OptionRow::Setting(SettingId::VideoAntiAlias),
    OptionRow::Setting(SettingId::VideoMaxQueuedFrames),
];

/// The rows RO-2 added that decide what the 3D scene looks like: the visual style, the four
/// Enhanced effects of AMD-055 REQ-SCN-079 and the backdrop lighting of AMD-049 REQ-SCN-075.
/// The desktop menu draws them in a *3D rendering* submenu.
///
/// The graded sub-parameters of the effects (`ao_*`, `ssr_*`) stay **global** on purpose: they
/// tune an effect rather than decide whether it runs, and they would take the per-game rows from
/// 14 to 22. Adding one later is one line here.
pub const GAME_3D_ROWS: [OptionRow; 6] = [
    OptionRow::Setting(SettingId::RenderVisualStyle),
    OptionRow::Setting(SettingId::RenderStyleVoxelGrid),
    OptionRow::Setting(SettingId::RenderStylePixelShadow),
    OptionRow::Setting(SettingId::RenderStyleAo),
    OptionRow::Setting(SettingId::RenderStyleReflections),
    OptionRow::Setting(SettingId::RenderBackdropLighting),
];

/// The curvature of AMD-043 REQ-SCN-066: the three angles, the arc centre and the 2D mode, which
/// the desktop menu puts in a *Geometry* submenu of the *3D rendering* one — the same name the
/// options window gives that very group.
pub const GAME_CURVATURE_ROWS: [OptionRow; 5] = [
    OptionRow::Setting(SettingId::SceneBendLr),
    OptionRow::Setting(SettingId::SceneBendTb),
    OptionRow::Setting(SettingId::SceneArcH),
    OptionRow::Setting(SettingId::SceneArcCenter),
    OptionRow::Setting(SettingId::SceneBend2dMode),
];

/// The fourteen rows RO-2 added, in the order the console's advanced page of a game lists them:
/// [`GAME_DISPLAY_ROWS`], then [`GAME_3D_ROWS`], then [`GAME_CURVATURE_ROWS`]. The desktop menu
/// draws the same fourteen, grouped; a test holds the two to the same list.
pub const GAME_ADVANCED_ROWS: [OptionRow; 14] = [
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
];

/// Every row a game can have a value of its own for, the economical ones first.
pub fn all_game_rows() -> impl Iterator<Item = OptionRow> {
    GAME_ROWS.into_iter().chain(GAME_ADVANCED_ROWS)
}

/// The settings a row of [`GAME_ROWS`] or [`GAME_ADVANCED_ROWS`] changes.
#[must_use]
pub const fn row_settings(row: OptionRow) -> &'static [SettingId] {
    match row {
        OptionRow::Volume => &[SettingId::AudioVolume],
        OptionRow::RenderScale => &[
            SettingId::VideoResScale,
            SettingId::VideoDynamicResolution,
            SettingId::VideoRenderMethod,
        ],
        OptionRow::Shadows => &[SettingId::RenderShadow],
        OptionRow::Palette => &[SettingId::VideoNesPalette],
        OptionRow::Overscan => &super::overscan::GAME_ROWS,
        OptionRow::FrameCap => &[SettingId::VideoVSync],
        OptionRow::ShowFps => &[SettingId::UiShowFps],
        OptionRow::Profiler => &[SettingId::UiProfiler],
        OptionRow::CatchUp => &[SettingId::EmuCatchUp],
        OptionRow::LuaOptimized => &[SettingId::EmuLuaOptimized],
        OptionRow::LuaExperimental => &[SettingId::EmuLuaExperimental],
        OptionRow::Pipeline => &[SettingId::EmuPipeline],
        OptionRow::Pause3dIn2d => &[SettingId::EmuPause3dIn2d],
        OptionRow::CatchUpLast3d => &[SettingId::EmuCatchUpLast3d],
        OptionRow::HalfRate3d => &[SettingId::VideoHalfRate3d],
        OptionRow::Region => &[SettingId::EmuAutoTvMode, SettingId::EmuTvMode],
        OptionRow::TouchPad => &[SettingId::InputTouchPad],
        // The fourteen of RO-2, each its own one setting. A `Setting` row that is **not** one of
        // them — the fifty-six of the console's global advanced page go through the same
        // variant — names nothing here, so it can never become an option of a game.
        OptionRow::Setting(id) => one_setting(id),
        OptionRow::Language
        | OptionRow::Advanced
        | OptionRow::Camera
        | OptionRow::Controls
        | OptionRow::Achievements
        | OptionRow::Optimizations => &[],
    }
}

/// The one setting of a [`GAME_ADVANCED_ROWS`] row, as a `'static` slice of one; nothing for
/// any other setting.
const fn one_setting(id: SettingId) -> &'static [SettingId] {
    match id {
        SettingId::EmuRunAhead => &[SettingId::EmuRunAhead],
        SettingId::VideoAntiAlias => &[SettingId::VideoAntiAlias],
        SettingId::VideoMaxQueuedFrames => &[SettingId::VideoMaxQueuedFrames],
        SettingId::RenderVisualStyle => &[SettingId::RenderVisualStyle],
        SettingId::RenderStyleVoxelGrid => &[SettingId::RenderStyleVoxelGrid],
        SettingId::RenderStylePixelShadow => &[SettingId::RenderStylePixelShadow],
        SettingId::RenderStyleAo => &[SettingId::RenderStyleAo],
        SettingId::RenderStyleReflections => &[SettingId::RenderStyleReflections],
        SettingId::RenderBackdropLighting => &[SettingId::RenderBackdropLighting],
        SettingId::SceneBendLr => &[SettingId::SceneBendLr],
        SettingId::SceneBendTb => &[SettingId::SceneBendTb],
        SettingId::SceneArcH => &[SettingId::SceneArcH],
        SettingId::SceneArcCenter => &[SettingId::SceneArcCenter],
        SettingId::SceneBend2dMode => &[SettingId::SceneBend2dMode],
        _ => &[],
    }
}

/// Whether `id` is a setting of a row of [`GAME_ROWS`] or [`GAME_ADVANCED_ROWS`]; the game's
/// other options (its hacks) belong to other screens.
#[must_use]
pub fn is_game_row_setting(id: SettingId) -> bool {
    all_game_rows().any(|row| row_settings(row).contains(&id))
}

/// What the screen asks of the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameOptionsCommand {
    /// The options changed: write them down.
    Save,
    /// Leave the screen.
    Close,
    /// Open the hacks of the game (its last row, Rust-only).
    OpenHacks,
    /// Open the settings of the widescreen of the game.
    OpenWidescreen,
    /// A went down on the widescreen row: released, it turns it on with everything or off;
    /// held, its settings show.
    Widescreen,
    /// Open the game's overscan (Rust-only).
    OpenOverscan,
}

/// The options of one game and the selection; row 0 is "reset everything".
#[derive(Debug, Clone)]
pub struct GameOptionsModel {
    rom: PathBuf,
    name: String,
    selected: usize,
    /// On the advanced page of the game's options: the row of the main page to come back to.
    /// `selected` then indexes [`GAME_ADVANCED_ROWS`] instead of the main page.
    page: Option<usize>,
    /// The global values: the preferences, without the scripts' or a game's.
    global: SettingsModel,
    /// The global values with the game's own over them.
    edited: SettingsModel,
    /// The game's options of other screens (its hacks), kept as they are.
    kept: GameOptions,
}

impl GameOptionsModel {
    /// The options of the game in `rom`, named `name`, over the global values of `settings`.
    #[must_use]
    pub fn new(rom: PathBuf, name: String, settings: &SettingsModel, options: GameOptions) -> Self {
        let mut global = settings.clone();
        global.clear_session_layer();
        global.clear_game_values();
        global.drain_changes().for_each(drop);
        let (options, kept): (GameOptions, GameOptions) = options
            .into_iter()
            .partition(|(id, _)| is_game_row_setting(*id));
        let mut model = Self {
            rom,
            name,
            selected: 0,
            page: None,
            edited: global.clone(),
            global,
            kept,
        };
        model.edited.set_game_values(options);
        model.normalise();
        model
    }

    /// The ROM file.
    #[must_use]
    pub const fn rom(&self) -> &PathBuf {
        &self.rom
    }

    /// The name of the game.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The rows of the main screen: "reset everything", the rows of [`GAME_ROWS`], the advanced
    /// page, then the widescreen and the hacks (owner request: apart).
    pub const ROW_COUNT: usize = GAME_ROWS.len() + 4;

    /// The row that opens the advanced options of the game ([`GAME_ADVANCED_ROWS`]).
    pub const ADVANCED_ROW: usize = GAME_ROWS.len() + 1;

    /// The row of the widescreen of the game.
    pub const WIDESCREEN_ROW: usize = GAME_ROWS.len() + 2;

    /// The row that opens the hacks of the game, the last one.
    pub const HACKS_ROW: usize = GAME_ROWS.len() + 3;

    /// The index of the selected row: on the main screen 0 is "reset everything", on the advanced
    /// page 0 is its first row.
    #[must_use]
    pub const fn selected_index(&self) -> usize {
        self.selected
    }

    /// Whether the advanced page of the game's options is shown.
    #[must_use]
    pub const fn in_advanced(&self) -> bool {
        self.page.is_some()
    }

    /// The rows of the screen shown: the advanced page's, or none for the main screen, whose
    /// rows are not a plain list (it has "reset everything", the widescreen and the hacks too).
    #[must_use]
    pub const fn page_rows(&self) -> Option<&'static [OptionRow]> {
        if self.page.is_some() {
            Some(&GAME_ADVANCED_ROWS)
        } else {
            None
        }
    }

    /// How many rows the screen shown has.
    #[must_use]
    pub const fn row_count(&self) -> usize {
        if self.page.is_some() {
            GAME_ADVANCED_ROWS.len()
        } else {
            Self::ROW_COUNT
        }
    }

    /// Selects the row `index` of the main screen (back from the hacks, on their row), leaving
    /// the advanced page if it was shown.
    pub const fn select(&mut self, index: usize) {
        if index < Self::ROW_COUNT {
            self.page = None;
            self.selected = index;
        }
    }

    /// The options to keep: only the settings that differ from the global values, and the
    /// game's options of other screens as they were.
    #[must_use]
    pub fn options(&self) -> GameOptions {
        self.edited
            .game_values()
            .iter()
            .map(|(id, value)| (*id, value.clone()))
            .chain(self.kept.iter().cloned())
            .collect()
    }

    /// Whether the game has any option of its own.
    #[must_use]
    pub fn has_options(&self) -> bool {
        !self.edited.game_values().is_empty()
    }

    /// Whether `row` has a value of its own for this game.
    #[must_use]
    pub fn is_own(&self, row: OptionRow) -> bool {
        row_settings(row)
            .iter()
            .any(|id| self.edited.has_game_value(*id))
    }

    /// The value of `row` as the screen shows it: the game's own, or "Use Global : <value>".
    #[must_use]
    pub fn value_text(&self, row: OptionRow) -> String {
        if self.is_own(row) {
            options::value_text(row, &self.edited, CameraPreset::Orbit)
        } else {
            let global = options::value_text(row, &self.global, CameraPreset::Orbit);
            strings::format(&strings::get(keys::HANDHELD_USE_GLOBAL), &[&global])
        }
    }

    /// The values `row` can take, as a menu lists them (the console steps through them instead).
    #[must_use]
    pub fn choices(&self, row: OptionRow) -> Vec<String> {
        self.walk(row).into_iter().map(|(text, _)| text).collect()
    }

    /// Gives `row` the value `index` of [`Self::choices`]; the global one is not kept as the
    /// game's own.
    pub fn choose(&mut self, row: OptionRow, index: usize) {
        if let Some((_, chosen)) = self.walk(row).into_iter().nth(index) {
            self.edited = chosen;
            self.normalise();
        }
    }

    /// Gives `row` the global value again.
    pub fn use_global(&mut self, row: OptionRow) {
        let own: GameOptions = self
            .options()
            .into_iter()
            .filter(|(id, _)| is_game_row_setting(*id))
            .filter(|(id, _)| !row_settings(row).contains(id))
            .collect();
        self.edited = self.global.clone();
        self.edited.set_game_values(own);
        self.edited.drain_changes().for_each(drop);
    }

    /// Every value of `row`, each with the settings that hold it: from the first value (the
    /// stepping down stops there, or comes round) up to the last.
    fn walk(&self, row: OptionRow) -> Vec<(String, SettingsModel)> {
        // The longest row is a curvature angle: 23 rungs since RO-1 point 7, so 24 was exactly
        // the length of the list and a rung more would have been dropped in silence.
        const LIMIT: usize = 48;
        let text =
            |settings: &SettingsModel| options::value_text(row, settings, CameraPreset::Orbit);
        let mut current = self.edited.clone();
        let mut seen = Vec::new();
        for _ in 0..LIMIT {
            let shown = text(&current);
            if seen.contains(&shown) {
                break;
            }
            seen.push(shown);
            let mut previous = current.clone();
            options::step_row(row, &mut previous, -1);
            if text(&previous) == text(&current) {
                break;
            }
            current = previous;
        }
        let mut values: Vec<(String, SettingsModel)> = Vec::new();
        for _ in 0..LIMIT {
            let shown = text(&current);
            if values.iter().any(|(known, _)| *known == shown) {
                break;
            }
            values.push((shown, current.clone()));
            options::step_row(row, &mut current, 1);
        }
        values
    }

    /// One press of a button.
    pub fn handle(&mut self, button: PadButton) -> Option<GameOptionsCommand> {
        let count = self.row_count();
        match button {
            PadButton::Up => self.selected = (self.selected + count - 1) % count,
            PadButton::Down => self.selected = (self.selected + 1) % count,
            // The advanced page: every row is a value row, B comes back to the row that opened
            // it, and X still puts every option of the game back.
            PadButton::X if self.page.is_some() => return self.reset(),
            PadButton::B | PadButton::Menu if self.page.is_some() => {
                self.selected = self.page.take().unwrap_or(Self::ADVANCED_ROW);
            }
            PadButton::Left if self.page.is_some() => return self.step(-1),
            PadButton::Right | PadButton::A if self.page.is_some() => return self.step(1),
            PadButton::A | PadButton::Right if self.selected == Self::ADVANCED_ROW => {
                self.page = Some(self.selected);
                self.selected = 0;
            }
            PadButton::A | PadButton::Right if self.selected == Self::HACKS_ROW => {
                return Some(GameOptionsCommand::OpenHacks);
            }
            PadButton::A if self.selected == Self::WIDESCREEN_ROW => {
                return Some(GameOptionsCommand::Widescreen);
            }
            PadButton::Right if self.selected == Self::WIDESCREEN_ROW => {
                return Some(GameOptionsCommand::OpenWidescreen);
            }
            PadButton::A | PadButton::Left | PadButton::Right
                if self.selected_row() == Some(OptionRow::Overscan) =>
            {
                return Some(GameOptionsCommand::OpenOverscan);
            }
            PadButton::Left => return self.step(-1),
            PadButton::A if self.selected == 0 => return self.reset(),
            PadButton::Right | PadButton::A => return self.step(1),
            PadButton::X => return self.reset(),
            PadButton::B | PadButton::Menu => return Some(GameOptionsCommand::Close),
            _ => {}
        }
        None
    }

    /// The row selected: one of [`GAME_ADVANCED_ROWS`] on the advanced page, one of
    /// [`GAME_ROWS`] on the main screen, none on its first and last rows.
    #[must_use]
    pub fn selected_row(&self) -> Option<OptionRow> {
        match self.page_rows() {
            Some(rows) => rows.get(self.selected).copied(),
            None => GAME_ROWS.get(self.selected.checked_sub(1)?).copied(),
        }
    }

    /// The global values with the game's own over them.
    #[must_use]
    pub const fn edited(&self) -> &SettingsModel {
        &self.edited
    }

    /// The global values alone: the preferences, without a script's or this game's.
    #[must_use]
    pub const fn global(&self) -> &SettingsModel {
        &self.global
    }

    /// Steps the game's own `id` (a setting of a row's own screen, the overscan's) by `step`.
    pub fn step_setting(&mut self, id: SettingId, step: i32) {
        super::overscan::step(id, &mut self.edited, step);
        self.normalise();
    }

    /// Gives the game's own `id` (a setting of a row's own screen, the overscan's) the value
    /// `value`; the global value is not kept as the game's own.
    pub fn set_setting(&mut self, id: SettingId, value: i32) {
        self.edited
            .set(id, zax_settings::values::SettingValue::Enum(value));
        self.normalise();
    }

    /// Every row back to the global value; the game's options of other screens stay.
    pub fn reset_all(&mut self) {
        self.edited.clear_game_values();
    }

    /// Every row back to the global value.
    fn reset(&mut self) -> Option<GameOptionsCommand> {
        if !self.has_options() {
            return None;
        }
        self.edited.clear_game_values();
        Some(GameOptionsCommand::Save)
    }

    fn step(&mut self, step: i32) -> Option<GameOptionsCommand> {
        let row = self.selected_row()?;
        // A write on a setting the game has a value for changes that value; on another, it
        // changes the global value of this copy, which `normalise` turns into the game's own.
        options::step_row(row, &mut self.edited, step);
        self.normalise();
        Some(GameOptionsCommand::Save)
    }

    /// Keeps as the game's own only the rows that differ from the global values, and in them
    /// only the settings that differ.
    fn normalise(&mut self) {
        let mut own = GameOptions::new();
        for row in all_game_rows() {
            let shown = options::value_text(row, &self.edited, CameraPreset::Orbit);
            if shown == options::value_text(row, &self.global, CameraPreset::Orbit) {
                continue;
            }
            for id in row_settings(row) {
                let value = self.edited.get(*id);
                if value != self.global.get(*id) {
                    own.push((*id, value));
                }
            }
        }
        self.edited = self.global.clone();
        self.edited.set_game_values(own);
        self.edited.drain_changes().for_each(drop);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)] // test code
mod tests {
    use zax_settings::values::SettingValue;

    use super::*;

    fn model(options: GameOptions) -> GameOptionsModel {
        let mut settings = SettingsModel::new();
        settings.set(SettingId::RenderShadow, true);
        GameOptionsModel::new(
            PathBuf::from("zelda.nes"),
            "zelda".to_owned(),
            &settings,
            options,
        )
    }

    fn go_to(model: &mut GameOptionsModel, row: OptionRow) {
        while model.selected_index() == 0 || GAME_ROWS[model.selected_index() - 1] != row {
            model.handle(PadButton::Down);
        }
    }

    #[test]
    fn every_row_starts_on_the_global_value() {
        let model = model(GameOptions::new());
        assert!(!model.has_options());
        assert!(GAME_ROWS.iter().all(|row| !model.is_own(*row)));
        assert!(model.value_text(OptionRow::Shadows).contains(':'));
    }

    #[test]
    fn a_change_keeps_only_what_differs_and_coming_back_is_global_again() {
        let mut model = model(GameOptions::new());
        go_to(&mut model, OptionRow::Shadows);
        assert_eq!(Some(GameOptionsCommand::Save), model.handle(PadButton::A));
        assert_eq!(
            model.options(),
            [(SettingId::RenderShadow, SettingValue::Bool(false))]
        );
        assert!(model.is_own(OptionRow::Shadows));
        model.handle(PadButton::A);
        assert_eq!(0, model.options().len(), "the global value again");

        // The region row moves two settings; back on the global choice, neither is kept.
        go_to(&mut model, OptionRow::Region);
        model.handle(PadButton::Left);
        assert!(model.is_own(OptionRow::Region));
        model.handle(PadButton::Right);
        assert!(!model.is_own(OptionRow::Region), "{:?}", model.options());
    }

    #[test]
    fn a_game_keeps_an_overscan_side_of_its_own_until_it_is_global_again() {
        let mut model = model(GameOptions::new());
        model.set_setting(SettingId::VideoOverscanTop, 8);
        assert!(model.is_own(OptionRow::Overscan));
        assert_eq!(
            model.options(),
            [(SettingId::VideoOverscanTop, SettingValue::Enum(8))]
        );
        model.set_setting(SettingId::VideoOverscanTop, -1);
        assert!(!model.is_own(OptionRow::Overscan), "{:?}", model.options());
    }

    #[test]
    fn a_game_keeps_a_palette_of_its_own() {
        let mut model = model(GameOptions::new());
        go_to(&mut model, OptionRow::Palette);
        // Every palette; the global one is Mesen's.
        let choices = model.choices(OptionRow::Palette);
        assert_eq!(
            choices.len(),
            zax_engine3d::palette::NesPalette::LABELS.len()
        );
        let zaxnes = choices
            .iter()
            .position(|choice| choice == "Zaxnes")
            .unwrap();
        model.choose(OptionRow::Palette, zaxnes);
        assert_eq!(
            model.options(),
            [(SettingId::VideoNesPalette, SettingValue::Enum(0))]
        );
        assert_eq!(model.value_text(OptionRow::Palette), "Zaxnes");
        model.use_global(OptionRow::Palette);
        assert!(!model.is_own(OptionRow::Palette));
    }

    #[test]
    fn the_first_row_and_x_put_everything_back() {
        let mut model = model(vec![
            (SettingId::RenderShadow, SettingValue::Bool(false)),
            (SettingId::EmuCatchUp, SettingValue::Enum(4)),
        ]);
        assert_eq!(2, model.options().len());
        assert_eq!(0, model.selected_index());
        assert_eq!(Some(GameOptionsCommand::Save), model.handle(PadButton::A));
        assert_eq!(0, model.options().len());
        assert_eq!(None, model.handle(PadButton::X), "nothing left to reset");
        assert_eq!(Some(GameOptionsCommand::Close), model.handle(PadButton::B));
    }

    #[test]
    fn a_menu_lists_every_value_of_a_row_and_chooses_one() {
        let mut model = model(GameOptions::new());
        let catch_ups = model.choices(OptionRow::CatchUp);
        assert_eq!(6, catch_ups.len(), "{catch_ups:?}");
        let volumes = model.choices(OptionRow::Volume);
        assert_eq!(11, volumes.len(), "{volumes:?}");
        assert_eq!("0 %", volumes[0]);
        let index = catch_ups.iter().position(|text| text == "4").unwrap();
        model.choose(OptionRow::CatchUp, index);
        assert_eq!(
            model.options(),
            [(SettingId::EmuCatchUp, SettingValue::Enum(4))]
        );
        model.use_global(OptionRow::CatchUp);
        assert_eq!(0, model.options().len());
    }

    #[test]
    fn an_option_equal_to_the_global_value_is_dropped() {
        let model = model(vec![(SettingId::RenderShadow, SettingValue::Bool(true))]);
        assert_eq!(0, model.options().len());
    }

    /// RO-2 part 2: the fourteen rows the lot added are the three groups the desktop menu draws,
    /// concatenated in that order; each names exactly one setting, and no setting is a row twice.
    #[test]
    fn ro_2_the_desktop_groups_are_the_advanced_rows_and_nothing_else() {
        let groups: Vec<OptionRow> = GAME_DISPLAY_ROWS
            .into_iter()
            .chain(GAME_3D_ROWS)
            .chain(GAME_CURVATURE_ROWS)
            .collect();
        assert_eq!(GAME_ADVANCED_ROWS.to_vec(), groups);

        let mut seen: Vec<SettingId> = Vec::new();
        for row in GAME_ADVANCED_ROWS {
            let settings = row_settings(row);
            assert_eq!(1, settings.len(), "{row:?}");
            assert!(!seen.contains(&settings[0]), "{row:?} twice");
            seen.push(settings[0]);
            assert!(is_game_row_setting(settings[0]));
            // Every one of them is a row of the console's **global** advanced page, which is what
            // settles that the game's copy belongs on a page of its own too.
            assert!(
                super::super::advanced::ROW_SETTINGS.contains(&settings[0]),
                "{row:?}"
            );
        }
        // The fourteen do not overlap the sixteen economical rows.
        for row in GAME_ROWS {
            for id in row_settings(row) {
                assert!(!seen.contains(id), "{row:?} is also an advanced row");
            }
        }
        // A `Setting` row of the console's global page that this lot did not add names nothing,
        // so it can never become an option of a game.
        assert_eq!(
            row_settings(OptionRow::Setting(SettingId::LightIntensity)),
            []
        );
        assert!(!is_game_row_setting(SettingId::LightIntensity));
    }

    /// Each of the fourteen stores a value of its own, "Use Global" takes it back, and "Put
    /// everything back to global" clears the lot.
    #[test]
    fn ro_2_every_new_row_stores_its_own_value_and_goes_back_to_global() {
        let mut model = model(GameOptions::new());
        for row in GAME_ADVANCED_ROWS {
            assert!(!model.is_own(row), "{row:?} starts on the global value");
            assert!(
                model.value_text(row).contains(':'),
                "{row:?} shows \"Use Global : <value>\""
            );
            // The first value that differs from the global one, whichever direction gets there.
            let choices = model.choices(row);
            assert!(choices.len() > 1, "{row:?} has {} choices", choices.len());
            let global = options::value_text(row, model.global(), CameraPreset::Orbit);
            let other = choices
                .iter()
                .position(|text| *text != global)
                .expect("a value other than the global one");
            model.choose(row, other);
            assert!(model.is_own(row), "{row:?} keeps its own value");
            assert_eq!(1, model.options().len(), "{row:?}: {:?}", model.options());
            assert_eq!(row_settings(row)[0], model.options()[0].0);
            model.use_global(row);
            assert!(!model.is_own(row), "{row:?}: {:?}", model.options());
            assert_eq!(0, model.options().len());
        }

        // Several at once, then "Put everything back to global".
        let mut kept = 0;
        for row in GAME_ADVANCED_ROWS {
            let global = options::value_text(row, model.global(), CameraPreset::Orbit);
            let choices = model.choices(row);
            let other = choices
                .iter()
                .position(|text| *text != global)
                .expect("a value other than the global one");
            model.choose(row, other);
            kept += 1;
            assert_eq!(kept, model.options().len(), "{row:?}");
        }
        assert_eq!(GAME_ADVANCED_ROWS.len(), model.options().len());
        model.reset_all();
        assert_eq!(0, model.options().len());
        assert!(!model.has_options());
    }

    /// The advanced page of the game's options: the row opens it, its rows step, B comes back to
    /// the row that opened it, and X still puts everything back from inside it.
    #[test]
    fn ro_2_the_advanced_page_of_a_game_opens_steps_and_goes_back() {
        let mut model = model(GameOptions::new());
        model.select(GameOptionsModel::ADVANCED_ROW);
        assert!(!model.in_advanced());
        assert_eq!(None, model.handle(PadButton::A));
        assert!(model.in_advanced());
        assert_eq!(0, model.selected_index());
        assert_eq!(GAME_ADVANCED_ROWS.len(), model.row_count());
        assert_eq!(Some(GAME_ADVANCED_ROWS[0]), model.selected_row());

        // Down walks the page and wraps round it, not round the main screen.
        for _ in 0..GAME_ADVANCED_ROWS.len() {
            model.handle(PadButton::Down);
        }
        assert_eq!(0, model.selected_index());

        // A steps the selected row, and the change is the game's own.
        assert_eq!(Some(GameOptionsCommand::Save), model.handle(PadButton::A));
        assert!(model.is_own(GAME_ADVANCED_ROWS[0]));
        assert_eq!(1, model.options().len());

        // X still puts everything back from inside the page, without leaving it.
        assert_eq!(Some(GameOptionsCommand::Save), model.handle(PadButton::X));
        assert_eq!(0, model.options().len());
        assert!(model.in_advanced());

        // B comes back to the advanced row of the main screen.
        model.handle(PadButton::Down);
        assert_eq!(None, model.handle(PadButton::B));
        assert!(!model.in_advanced());
        assert_eq!(GameOptionsModel::ADVANCED_ROW, model.selected_index());
        assert_eq!(GameOptionsModel::ROW_COUNT, model.row_count());
        // And B again leaves the screen, as it always did.
        assert_eq!(Some(GameOptionsCommand::Close), model.handle(PadButton::B));
    }

    /// The curvature rows offer the 23 rungs of RO-1 point 7, not the 13 of AMD-043 — the menu
    /// and the console page of a game read the setting's own domain, like every other row.
    #[test]
    fn ro_2_a_curvature_row_of_a_game_offers_the_finer_grid() {
        let model = model(GameOptions::new());
        let row = OptionRow::Setting(SettingId::SceneBendLr);
        let choices = model.choices(row);
        assert_eq!(23, choices.len(), "{choices:?}");
        for degrees in ["-180", "-30", "-25", "-5", "0", "5", "25", "30", "180"] {
            assert!(choices.iter().any(|text| text == degrees), "{degrees}");
        }
    }

    /// The main screen kept its sixteen rows and its order; only the advanced row is new, and it
    /// sits before the widescreen and the hacks, which the owner keeps last.
    #[test]
    fn ro_2_the_main_screen_kept_its_rows() {
        assert_eq!(16, GAME_ROWS.len());
        assert_eq!(OptionRow::Volume, GAME_ROWS[0]);
        assert_eq!(OptionRow::Region, GAME_ROWS[15]);
        assert_eq!(17, GameOptionsModel::ADVANCED_ROW);
        assert_eq!(18, GameOptionsModel::WIDESCREEN_ROW);
        assert_eq!(19, GameOptionsModel::HACKS_ROW);
        assert_eq!(20, GameOptionsModel::ROW_COUNT);
    }
}
