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
pub const GAME_ROWS: [OptionRow; 13] = [
    OptionRow::Volume,
    OptionRow::RenderScale,
    OptionRow::Shadows,
    OptionRow::Palette,
    OptionRow::Overscan,
    OptionRow::FrameCap,
    OptionRow::ShowFps,
    OptionRow::Profiler,
    OptionRow::CatchUp,
    OptionRow::LuaOptimized,
    OptionRow::LuaExperimental,
    OptionRow::Pipeline,
    OptionRow::Region,
];

/// The settings a row of [`GAME_ROWS`] changes.
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
        OptionRow::Region => &[SettingId::EmuAutoTvMode, SettingId::EmuTvMode],
        OptionRow::Language | OptionRow::Camera | OptionRow::Controls => &[],
    }
}

/// Whether `id` is a setting of a row of [`GAME_ROWS`]; the game's other options (its hacks)
/// belong to other screens.
#[must_use]
pub fn is_game_row_setting(id: SettingId) -> bool {
    GAME_ROWS.iter().any(|row| row_settings(*row).contains(&id))
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
    /// Open the game's overscan (Rust-only).
    OpenOverscan,
}

/// The options of one game and the selection; row 0 is "reset everything".
#[derive(Debug, Clone)]
pub struct GameOptionsModel {
    rom: PathBuf,
    name: String,
    selected: usize,
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

    /// The rows of the screen: "reset everything", the rows of [`GAME_ROWS`], then "Hacks".
    pub const ROW_COUNT: usize = GAME_ROWS.len() + 2;

    /// The row that opens the hacks of the game, the last one.
    pub const HACKS_ROW: usize = GAME_ROWS.len() + 1;

    /// The index of the selected row, 0 being "reset everything".
    #[must_use]
    pub const fn selected_index(&self) -> usize {
        self.selected
    }

    /// Selects the row `index` (back from the hacks, on their row).
    pub const fn select(&mut self, index: usize) {
        if index < Self::ROW_COUNT {
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
        const LIMIT: usize = 24;
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
        let count = Self::ROW_COUNT;
        match button {
            PadButton::Up => self.selected = (self.selected + count - 1) % count,
            PadButton::Down => self.selected = (self.selected + 1) % count,
            PadButton::A | PadButton::Right if self.selected == Self::HACKS_ROW => {
                return Some(GameOptionsCommand::OpenHacks);
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

    /// The row of [`GAME_ROWS`] selected, none on the first and last rows.
    fn selected_row(&self) -> Option<OptionRow> {
        GAME_ROWS.get(self.selected.checked_sub(1)?).copied()
    }

    /// The global values with the game's own over them.
    #[must_use]
    pub const fn edited(&self) -> &SettingsModel {
        &self.edited
    }

    /// Steps the game's own `id` (a setting of a row's own screen, the overscan's) by `step`.
    pub fn step_setting(&mut self, id: SettingId, step: i32) {
        super::overscan::step(id, &mut self.edited, step);
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
        let row = *GAME_ROWS.get(self.selected.checked_sub(1)?)?;
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
        for row in GAME_ROWS {
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
}
