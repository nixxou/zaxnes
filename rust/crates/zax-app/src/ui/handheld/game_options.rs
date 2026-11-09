//! The options of one game as data (Rust-only, owner request): the rows of the options that make
//! sense for a single ROM, each either the global value ("Use Global") or one of its own, and a
//! first row that puts every one back to the global value. Only the settings that differ from
//! the global ones are kept.

use std::path::PathBuf;

use zax_settings::catalogue::SettingId;
use zax_settings::games::GameOptions;
use zax_settings::model::SettingsModel;

use super::options::{self, OptionRow, OptionsPage, MAX_DEPTH};
use super::overlay::CameraPreset;
use super::pad::PadButton;
use crate::ui::{keys, strings};

/// A row of the main screen of a game's options: the ordinary option rows, and the three the game
/// has of its own — "put everything back", the widescreen and the hacks. RO-3 made this a list
/// because the mockup moves the widescreen into the middle of it, where an index constant per
/// special row would have grown fragile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameRow {
    /// "Put everything back to global", the first row.
    ResetAll,
    /// An option row, drawn and stepped as the global pages draw and step it.
    Option(OptionRow),
    /// The widescreen of the game: a press turns it on with everything or off, a hold opens its
    /// settings.
    Widescreen,
    /// The hacks of the game: its own screen.
    Hacks,
}

/// The rows of the main screen of a game's options, in the order of the owner's mockup (RO-3).
/// The widescreen moved from next to last to between the overscan and the region; the hacks stay
/// last.
pub const GAME_MAIN: [GameRow; 11] = [
    GameRow::ResetAll,
    GameRow::Option(OptionRow::Volume),
    GameRow::Option(OptionRow::RenderScale),
    GameRow::Option(OptionRow::Shadows),
    GameRow::Option(OptionRow::Palette),
    GameRow::Option(OptionRow::Overscan),
    GameRow::Widescreen,
    GameRow::Option(OptionRow::Region),
    GameRow::Option(OptionRow::Page(OptionsPage::Optimizations)),
    GameRow::Option(OptionRow::Page(OptionsPage::Advanced)),
    GameRow::Hacks,
];

/// *Optimisations* of a game: the eight performance rows of the global page. The smooth Lua
/// collector is not among them — it stays global, as it was before RO-3.
pub const GAME_OPTIMIZATION_ROWS: [OptionRow; 8] = [
    OptionRow::FrameCap,
    OptionRow::HalfRate3d,
    OptionRow::CatchUp,
    OptionRow::CatchUpLast3d,
    OptionRow::Pause3dIn2d,
    OptionRow::Pipeline,
    OptionRow::LuaOptimized,
    OptionRow::LuaExperimental,
];

/// *Advanced options* of a game: four pages where the global hub has six. A game has nothing to
/// set on the *NES picture* page (its palette and its overscan are main-screen rows) nor on the
/// *Camera* page (the view belongs to the player).
pub const GAME_ADVANCED_HUB: [OptionRow; 4] = [
    OptionRow::Page(OptionsPage::Display),
    OptionRow::Page(OptionsPage::Rendering3d),
    OptionRow::Page(OptionsPage::LightShadows),
    OptionRow::Page(OptionsPage::Emulation),
];

/// *Advanced options › Display* of a game: the two rows that left the main screen, and the two
/// of RO-2 that belong to the picture as it reaches the screen.
pub const GAME_DISPLAY_ROWS: [OptionRow; 4] = [
    OptionRow::ShowFps,
    OptionRow::Profiler,
    OptionRow::Setting(SettingId::VideoAntiAlias),
    OptionRow::Setting(SettingId::VideoMaxQueuedFrames),
];

/// *Advanced options › 3D rendering* of a game: the visual style, then its two pages.
pub const GAME_3D_ROWS: [OptionRow; 3] = [
    OptionRow::Setting(SettingId::RenderVisualStyle),
    OptionRow::Page(OptionsPage::EnhancedEffects),
    OptionRow::Page(OptionsPage::Geometry),
];

/// *3D rendering › Enhanced effects* of a game: the four switches of AMD-055 REQ-SCN-079.
///
/// Their graded sub-parameters (`ao_*`, `ssr_*`) stay **global** on purpose, as RO-2 decided:
/// they tune an effect rather than decide whether it runs. Adding one is a line here.
pub const GAME_EFFECTS_ROWS: [OptionRow; 4] = [
    OptionRow::Setting(SettingId::RenderStyleVoxelGrid),
    OptionRow::Setting(SettingId::RenderStylePixelShadow),
    OptionRow::Setting(SettingId::RenderStyleAo),
    OptionRow::Setting(SettingId::RenderStyleReflections),
];

/// *3D rendering › Geometry* of a game: the curvature of AMD-043 REQ-SCN-066.
pub const GAME_CURVATURE_ROWS: [OptionRow; 5] = [
    OptionRow::Setting(SettingId::SceneBendLr),
    OptionRow::Setting(SettingId::SceneBendTb),
    OptionRow::Setting(SettingId::SceneArcH),
    OptionRow::Setting(SettingId::SceneArcCenter),
    OptionRow::Setting(SettingId::SceneBend2dMode),
];

/// *Advanced options › Light and shadows* of a game: the backdrop lighting of AMD-049
/// REQ-SCN-075. The shadow switch is a main-screen row and the pixel shadows an Enhanced effect.
pub const GAME_LIGHT_ROWS: [OptionRow; 1] = [OptionRow::Setting(SettingId::RenderBackdropLighting)];

/// *Advanced options › Emulation* of a game: the run-ahead, whose right value depends on each
/// game's own lag. Q-103 still holds it at 0 for a profile whose scripts change the core.
pub const GAME_EMULATION_ROWS: [OptionRow; 1] = [OptionRow::Setting(SettingId::EmuRunAhead)];

/// The rows of `page` in the options of one **game**; empty for a page a game has not got.
// Q-107 §5, §6
#[must_use]
pub const fn page_rows(page: OptionsPage) -> &'static [OptionRow] {
    match page {
        OptionsPage::Optimizations => &GAME_OPTIMIZATION_ROWS,
        OptionsPage::Advanced => &GAME_ADVANCED_HUB,
        OptionsPage::Display => &GAME_DISPLAY_ROWS,
        OptionsPage::Rendering3d => &GAME_3D_ROWS,
        OptionsPage::EnhancedEffects => &GAME_EFFECTS_ROWS,
        OptionsPage::Geometry => &GAME_CURVATURE_ROWS,
        OptionsPage::LightShadows => &GAME_LIGHT_ROWS,
        OptionsPage::Emulation => &GAME_EMULATION_ROWS,
        // AUDIO-2: the choice of audio output belongs to the device, not to a game, so the Audio
        // page is not on a game's hub at all.
        OptionsPage::NesPicture | OptionsPage::Camera | OptionsPage::Audio => &[],
    }
}

/// The option rows of the main screen, the three specials dropped.
pub fn main_option_rows() -> impl Iterator<Item = OptionRow> {
    GAME_MAIN.into_iter().filter_map(|row| match row {
        GameRow::Option(row) => Some(row),
        GameRow::ResetAll | GameRow::Widescreen | GameRow::Hacks => None,
    })
}

/// Every row a game can have a value of its own for: the main screen's, then every page's. The
/// rows that only climb to a page or head a group are not values and are left out.
pub fn all_game_rows() -> impl Iterator<Item = OptionRow> {
    main_option_rows()
        .chain(
            OptionsPage::ALL
                .into_iter()
                .flat_map(|page| page.game_rows().iter().copied()),
        )
        .filter(|row| !matches!(row, OptionRow::Page(_) | OptionRow::Group(_)))
}

/// The settings a row of a game's options changes; nothing for a row a game cannot set.
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
        | OptionRow::Camera
        | OptionRow::Controls
        | OptionRow::Achievements
        | OptionRow::Page(_)
        | OptionRow::Group(_) => &[],
    }
}

/// The one setting of a `Setting` row a game may have, as a `'static` slice of one; nothing for
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

/// Whether `id` is a setting of a row of the game's options; the game's other options (its hacks)
/// belong to other screens.
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
    /// The pages open over the main screen, innermost last: each the page shown and the row of
    /// the screen under it to come back to. `selected` indexes the page shown.
    trail: [(OptionsPage, usize); MAX_DEPTH],
    depth: usize,
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
            trail: [(OptionsPage::Advanced, 0); MAX_DEPTH],
            depth: 0,
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

    /// How many rows the main screen has ([`GAME_MAIN`]).
    pub const ROW_COUNT: usize = GAME_MAIN.len();

    /// The row that opens the advanced options of the game. The three index constants are the
    /// places of the three rows of [`GAME_MAIN`] other code has to reach by name; a unit test
    /// holds each one to the row it stands for.
    pub const ADVANCED_ROW: usize = 9;

    /// The row of the widescreen of the game.
    pub const WIDESCREEN_ROW: usize = 6;

    /// The row that opens the hacks of the game, the last one.
    pub const HACKS_ROW: usize = 10;

    /// The index of the selected row: on the main screen 0 is "reset everything", on a page 0 is
    /// its first row.
    #[must_use]
    pub const fn selected_index(&self) -> usize {
        self.selected
    }

    /// Whether a page of the game's options is shown rather than the main screen.
    #[must_use]
    pub const fn in_advanced(&self) -> bool {
        self.depth > 0
    }

    /// The page shown, if any.
    #[must_use]
    pub fn page(&self) -> Option<OptionsPage> {
        self.depth.checked_sub(1).map(|last| self.trail[last].0)
    }

    /// The pages open over the main screen, outermost first: what the title band spells out.
    #[must_use]
    pub fn path(&self) -> Vec<OptionsPage> {
        self.trail[..self.depth]
            .iter()
            .map(|(page, _)| *page)
            .collect()
    }

    /// The rows of the page shown, or none for the main screen, whose rows are [`GAME_MAIN`] and
    /// not a plain list of option rows.
    #[must_use]
    pub fn page_rows(&self) -> Option<&'static [OptionRow]> {
        self.page().map(OptionsPage::game_rows)
    }

    /// How many rows the screen shown has.
    #[must_use]
    pub fn row_count(&self) -> usize {
        self.page_rows().map_or(Self::ROW_COUNT, <[OptionRow]>::len)
    }

    /// Selects the row `index` of the main screen (back from the hacks, on their row), leaving
    /// any page that was shown.
    pub const fn select(&mut self, index: usize) {
        if index < Self::ROW_COUNT {
            self.depth = 0;
            self.selected = index;
        }
    }

    /// Moves the selection `step` places: round the page shown, skipping its group titles, or
    /// round the main screen, every row of which is selectable.
    fn move_selection(&mut self, step: i32) {
        if let Some(rows) = self.page_rows() {
            self.selected = options::step_selection(rows, self.selected, step);
            return;
        }
        let count = Self::ROW_COUNT;
        let forward = if step < 0 { count - 1 } else { 1 };
        self.selected = (self.selected + forward) % count;
    }

    /// Climbs to `page`, remembering the row to come back to.
    fn open(&mut self, page: OptionsPage) {
        if self.depth >= MAX_DEPTH {
            return;
        }
        self.trail[self.depth] = (page, self.selected);
        self.depth += 1;
        self.selected = self.page_rows().map_or(0, options::first_selectable);
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

    /// The value of `row` as the desktop menu shows it: the game's own, or
    /// "Use Global : &lt;value&gt;".
    #[must_use]
    pub fn value_text(&self, row: OptionRow) -> String {
        if self.is_own(row) {
            options::value_text(row, &self.edited, CameraPreset::Orbit)
        } else {
            let global = options::value_text(row, &self.global, CameraPreset::Orbit);
            strings::format(&strings::get(keys::HANDHELD_USE_GLOBAL), &[&global])
        }
    }

    /// RO-3 §2 rule 1: the value of `row` as a **console** row of a game's options shows it — its
    /// own value plainly, or the shorter "Global: &lt;value&gt;", which the drawing dims. The
    /// marker beside an own value is the drawing's, the colour being RO-2's.
    #[must_use]
    pub fn console_value_text(&self, row: OptionRow) -> String {
        if self.is_own(row) {
            options::value_text(row, &self.edited, CameraPreset::Orbit)
        } else {
            let global = options::value_text(row, &self.global, CameraPreset::Orbit);
            strings::format(&strings::get(keys::HANDHELD_GLOBAL_VALUE), &[&global])
        }
    }

    /// RO-3 §2 rule 2: how many values of its own the game has under `page`, counting the pages
    /// `page` itself climbs to. The console row that opens a page says so; 0 means it says
    /// nothing.
    ///
    /// Computed from the model when the page is drawn, never per frame while playing: a console
    /// page is only drawn while it is shown.
    #[must_use]
    pub fn own_count(&self, page: OptionsPage) -> usize {
        page.game_rows()
            .iter()
            .map(|row| match row {
                OptionRow::Page(under) => self.own_count(*under),
                OptionRow::Group(_) => 0,
                row => usize::from(self.is_own(*row)),
            })
            .sum()
    }

    /// What a console row that opens `page` shows beside it: "2 own" and nothing with none.
    #[must_use]
    pub fn own_count_text(&self, page: OptionsPage) -> String {
        match self.own_count(page) {
            0 => String::new(),
            count => strings::format(
                &strings::get(keys::HANDHELD_OWN_COUNT),
                &[&count.to_string()],
            ),
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
        match button {
            // A page may hold a group title, which up and down skip; every row of the main
            // screen is selectable, so there it is a plain walk round the list.
            PadButton::Up => self.move_selection(-1),
            PadButton::Down => self.move_selection(1),
            // X puts every option of the game back, from wherever it is pressed.
            PadButton::X => return self.reset(),
            // B climbs back one page at a time, and leaves the screen from the main one.
            PadButton::B | PadButton::Menu => match self.depth.checked_sub(1) {
                Some(last) => {
                    self.depth = last;
                    self.selected = self.trail[last].1;
                }
                None => return Some(GameOptionsCommand::Close),
            },
            PadButton::A | PadButton::Left | PadButton::Right
                if matches!(self.selected_row(), Some(OptionRow::Page(_))) =>
            {
                if let Some(OptionRow::Page(page)) = self.selected_row() {
                    self.open(page);
                }
            }
            PadButton::A | PadButton::Left | PadButton::Right
                if self.selected_row() == Some(OptionRow::Overscan) =>
            {
                return Some(GameOptionsCommand::OpenOverscan);
            }
            PadButton::A | PadButton::Right
                if !self.in_advanced() && self.selected == Self::HACKS_ROW =>
            {
                return Some(GameOptionsCommand::OpenHacks);
            }
            PadButton::A if !self.in_advanced() && self.selected == Self::WIDESCREEN_ROW => {
                return Some(GameOptionsCommand::Widescreen);
            }
            PadButton::Right if !self.in_advanced() && self.selected == Self::WIDESCREEN_ROW => {
                return Some(GameOptionsCommand::OpenWidescreen);
            }
            PadButton::A if !self.in_advanced() && self.selected == 0 => return self.reset(),
            PadButton::Left => return self.step(-1),
            PadButton::Right | PadButton::A => return self.step(1),
            _ => {}
        }
        None
    }

    /// The row selected: the page's on a page, the main screen's option row on the main screen,
    /// none on its three rows that are not options.
    #[must_use]
    pub fn selected_row(&self) -> Option<OptionRow> {
        match self.page_rows() {
            Some(rows) => rows.get(self.selected).copied(),
            None => match GAME_MAIN.get(self.selected)? {
                GameRow::Option(row) => Some(*row),
                GameRow::ResetAll | GameRow::Widescreen | GameRow::Hacks => None,
            },
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

    /// The pages to open to reach `row` in the game tree, outermost first.
    fn path_to(row: OptionRow) -> Option<Vec<OptionsPage>> {
        fn search(rows: &[OptionRow], wanted: OptionRow, path: &mut Vec<OptionsPage>) -> bool {
            for candidate in rows {
                if *candidate == wanted {
                    return true;
                }
                if let OptionRow::Page(page) = candidate {
                    path.push(*page);
                    if search(page.game_rows(), wanted, path) {
                        return true;
                    }
                    path.pop();
                }
            }
            false
        }
        let main: Vec<OptionRow> = main_option_rows().collect();
        let mut path = Vec::new();
        search(&main, row, &mut path).then_some(path)
    }

    /// Walks to `row` wherever it is in the game tree: out to the main screen, then down its path.
    fn go_to(model: &mut GameOptionsModel, row: OptionRow) {
        while model.in_advanced() {
            model.handle(PadButton::B);
        }
        let path = path_to(row).unwrap_or_else(|| panic!("{row:?} is not a row of a game"));
        for page in path {
            while model.selected_row() != Some(OptionRow::Page(page)) {
                model.handle(PadButton::Down);
            }
            model.handle(PadButton::A);
        }
        while model.selected_row() != Some(row) {
            model.handle(PadButton::Down);
        }
    }

    #[test]
    fn every_row_starts_on_the_global_value() {
        let model = model(GameOptions::new());
        assert!(!model.has_options());
        assert!(all_game_rows().all(|row| !model.is_own(row)));
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

    /// Every row of the game tree names exactly one setting or a set of its own (the overscan,
    /// the region, the render scale), no setting is a row twice, and a `Setting` row the tree does
    /// not hold names nothing — so it can never become an option of a game.
    #[test]
    fn ro_3_every_row_of_the_game_tree_names_its_settings_once() {
        let mut seen: Vec<SettingId> = Vec::new();
        for row in all_game_rows() {
            let settings = row_settings(row);
            assert!(!settings.is_empty(), "{row:?} names nothing");
            for id in settings {
                // The two composite rows of the main screen share no setting with any other row;
                // the test of the global tree records the two settings they share with a
                // `Setting` row of a page (`ro_3_no_row_is_reachable_from_two_places`).
                assert!(!seen.contains(id), "{} is a row twice", id.key());
                seen.push(*id);
                assert!(is_game_row_setting(*id));
            }
        }
        for row in all_game_rows() {
            if let OptionRow::Setting(id) = row {
                assert!(
                    super::super::advanced::ROW_SETTINGS.contains(&id),
                    "{} is not a console setting",
                    id.key()
                );
            }
        }
        assert_eq!(
            row_settings(OptionRow::Setting(SettingId::LightIntensity)),
            []
        );
        assert!(!is_game_row_setting(SettingId::LightIntensity));
    }

    /// Each of the thirty rows a game can have stores a value of its own, "Use Global" takes it
    /// back, and "Put everything back to global" clears the lot.
    #[test]
    fn ro_3_every_row_stores_its_own_value_and_goes_back_to_global() {
        let steppable: Vec<OptionRow> = all_game_rows()
            // The overscan has a screen of its own rather than a stepped value, and it has its
            // own test above.
            .filter(|row| *row != OptionRow::Overscan)
            .collect();
        assert_eq!(29, steppable.len(), "{steppable:?}");
        let mut model = model(GameOptions::new());
        for row in &steppable {
            let row = *row;
            assert!(!model.is_own(row), "{row:?} starts on the global value");
            assert!(
                model.console_value_text(row).starts_with("Global"),
                "{row:?} shows \"Global: <value>\": {}",
                model.console_value_text(row)
            );
            let choices = model.choices(row);
            assert!(choices.len() > 1, "{row:?} has {} choices", choices.len());
            let global = options::value_text(row, model.global(), CameraPreset::Orbit);
            let other = choices
                .iter()
                .position(|text| *text != global)
                .expect("a value other than the global one");
            model.choose(row, other);
            assert!(model.is_own(row), "{row:?} keeps its own value");
            assert_eq!(
                model.console_value_text(row),
                options::value_text(row, model.edited(), CameraPreset::Orbit),
                "{row:?} shows its own value plainly"
            );
            model.use_global(row);
            assert!(!model.is_own(row), "{row:?}: {:?}", model.options());
            assert_eq!(0, model.options().len());
        }

        // Several at once, then "Put everything back to global".
        for row in &steppable {
            let global = options::value_text(*row, model.global(), CameraPreset::Orbit);
            let choices = model.choices(*row);
            let other = choices
                .iter()
                .position(|text| *text != global)
                .expect("a value other than the global one");
            model.choose(*row, other);
            assert!(model.is_own(*row), "{row:?}");
        }
        model.reset_all();
        assert_eq!(0, model.options().len());
        assert!(!model.has_options());
    }

    /// RO-3 §2: the pages of a game's options open, step, and B climbs back one at a time to the
    /// row that opened each; X still puts everything back from inside a page.
    #[test]
    fn ro_3_the_pages_of_a_game_open_step_and_go_back() {
        let mut model = model(GameOptions::new());
        model.select(GameOptionsModel::ADVANCED_ROW);
        assert!(!model.in_advanced());
        assert_eq!(
            Some(OptionRow::Page(OptionsPage::Advanced)),
            model.selected_row()
        );
        assert_eq!(None, model.handle(PadButton::A));
        assert!(model.in_advanced());
        assert_eq!(0, model.selected_index());
        assert_eq!(GAME_ADVANCED_HUB.len(), model.row_count());

        // Down the hub to *3D rendering*, then into *Enhanced effects*: three levels.
        while model.selected_row() != Some(OptionRow::Page(OptionsPage::Rendering3d)) {
            model.handle(PadButton::Down);
        }
        model.handle(PadButton::A);
        while model.selected_row() != Some(OptionRow::Page(OptionsPage::EnhancedEffects)) {
            model.handle(PadButton::Down);
        }
        model.handle(PadButton::A);
        assert_eq!(
            vec![
                OptionsPage::Advanced,
                OptionsPage::Rendering3d,
                OptionsPage::EnhancedEffects
            ],
            model.path()
        );

        // Down wraps round this page, not round anything above it.
        for _ in 0..GAME_EFFECTS_ROWS.len() {
            model.handle(PadButton::Down);
        }
        assert_eq!(0, model.selected_index());

        // A steps the selected row, and the change is the game's own.
        assert_eq!(Some(GameOptionsCommand::Save), model.handle(PadButton::A));
        assert!(model.is_own(GAME_EFFECTS_ROWS[0]));
        assert_eq!(1, model.options().len());

        // X still puts everything back from inside a page, without leaving it.
        assert_eq!(Some(GameOptionsCommand::Save), model.handle(PadButton::X));
        assert_eq!(0, model.options().len());
        assert_eq!(Some(OptionsPage::EnhancedEffects), model.page());

        // B climbs back one page at a time, each time onto the row that opened it.
        assert_eq!(None, model.handle(PadButton::B));
        assert_eq!(
            Some(OptionRow::Page(OptionsPage::EnhancedEffects)),
            model.selected_row()
        );
        assert_eq!(None, model.handle(PadButton::B));
        assert_eq!(
            Some(OptionRow::Page(OptionsPage::Rendering3d)),
            model.selected_row()
        );
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

    /// RO-3 §2: the main screen is the owner's mockup, row for row — the widescreen between the
    /// overscan and the region, the hacks last — and the three index constants name the three
    /// rows they stand for.
    #[test]
    fn ro_3_the_main_screen_is_the_mockups() {
        assert_eq!(
            [
                GameRow::ResetAll,
                GameRow::Option(OptionRow::Volume),
                GameRow::Option(OptionRow::RenderScale),
                GameRow::Option(OptionRow::Shadows),
                GameRow::Option(OptionRow::Palette),
                GameRow::Option(OptionRow::Overscan),
                GameRow::Widescreen,
                GameRow::Option(OptionRow::Region),
                GameRow::Option(OptionRow::Page(OptionsPage::Optimizations)),
                GameRow::Option(OptionRow::Page(OptionsPage::Advanced)),
                GameRow::Hacks,
            ],
            GAME_MAIN
        );
        assert_eq!(11, GameOptionsModel::ROW_COUNT);
        assert_eq!(
            GameRow::Option(OptionRow::Page(OptionsPage::Advanced)),
            GAME_MAIN[GameOptionsModel::ADVANCED_ROW]
        );
        assert_eq!(
            GameRow::Widescreen,
            GAME_MAIN[GameOptionsModel::WIDESCREEN_ROW]
        );
        assert_eq!(GameRow::Hacks, GAME_MAIN[GameOptionsModel::HACKS_ROW]);
    }

    /// RO-3 §2 rule 2: a row that opens a page counts the game's own values under it, the pages
    /// below included, and says nothing when there are none. The count is of the model, so it
    /// follows a change at once.
    #[test]
    fn ro_3_a_page_row_counts_the_own_values_under_it() {
        let mut model = model(GameOptions::new());
        for page in OptionsPage::ALL {
            assert_eq!(0, model.own_count(page), "{page:?}");
            assert!(model.own_count_text(page).is_empty(), "{page:?}");
        }

        // One value two levels down: *Advanced options* counts it, and so does every page of the
        // path to it, and no other.
        go_to(&mut model, GAME_CURVATURE_ROWS[0]);
        assert_eq!(Some(GameOptionsCommand::Save), model.handle(PadButton::A));
        assert!(model.is_own(GAME_CURVATURE_ROWS[0]));
        assert_eq!(1, model.own_count(OptionsPage::Advanced));
        assert_eq!(1, model.own_count(OptionsPage::Rendering3d));
        assert_eq!(1, model.own_count(OptionsPage::Geometry));
        assert_eq!(0, model.own_count(OptionsPage::Display));
        assert_eq!(0, model.own_count(OptionsPage::Optimizations));
        assert!(model.own_count_text(OptionsPage::Advanced).contains('1'));

        // A second, on another page of the hub: the hub counts two, each page its own one.
        go_to(&mut model, OptionRow::ShowFps);
        assert_eq!(Some(GameOptionsCommand::Save), model.handle(PadButton::A));
        assert_eq!(2, model.own_count(OptionsPage::Advanced));
        assert_eq!(1, model.own_count(OptionsPage::Display));
        assert_eq!(1, model.own_count(OptionsPage::Rendering3d));

        // And "Put everything back to global" empties every count at once.
        model.reset_all();
        for page in OptionsPage::ALL {
            assert_eq!(0, model.own_count(page), "{page:?}");
        }
    }

    /// RO-3 §2: every page of a game is reachable from its main screen, and a page a game has not
    /// got is on neither tree of its options.
    #[test]
    fn ro_3_the_game_tree_has_no_page_a_game_cannot_use() {
        for page in OptionsPage::ALL {
            let reachable = path_to(OptionRow::Page(page)).is_some();
            assert_eq!(
                !page.game_rows().is_empty(),
                reachable,
                "{page:?}: rows {}, reachable {reachable}",
                page.game_rows().len()
            );
        }
        assert_eq!(OptionsPage::NesPicture.game_rows(), []);
        assert_eq!(OptionsPage::Camera.game_rows(), []);
    }
}
