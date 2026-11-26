//! The cheats window and the RAM search window (lot CHEATS), from the game options menu: the
//! game's cheats on or off, typed in, taken from the cheat database, or found in its RAM.

use imgui::{Condition, StyleColor, TableFlags, Ui, WindowFlags};
use zax_cheats::search::{Comparison, Operand, ValueSize};
use zax_cheats::sheet::SheetMatch;
use zax_cheats::CheatList;

use super::{centre, BrowserPurpose, ZaxUi};
use crate::application::AppHost;
use crate::cheat_database::FoundSheet;
use crate::emulation::host::cheats::{self as host_cheats, RamSearchAction, SEARCH_ROWS};
use crate::ui::{keys, strings};

const GOOD: [f32; 4] = [0.45, 0.92, 0.55, 1.0];
const WARNING: [f32; 4] = [1.0, 0.62, 0.2, 1.0];
const BAD: [f32; 4] = [1.0, 0.45, 0.35, 1.0];

/// What the two windows keep between frames: the fields being typed, the sheet looked up.
#[derive(Debug, Default)]
pub struct CheatsWindow {
    /// The game these fields belong to; another game clears them.
    rom: Option<String>,
    description: String,
    codes: String,
    error: Option<String>,
    database: String,
    database_shown: bool,
    sheet: Option<Result<Option<FoundSheet>, String>>,
    size: usize,
    signed: bool,
    comparison: usize,
    operand: usize,
    number: i32,
    selected: Option<u16>,
    value: i32,
    message: Option<(String, bool)>,
}

impl CheatsWindow {
    /// The file browser chose a database: it is looked up again.
    pub fn database_chosen(&mut self, path: &str) {
        path.clone_into(&mut self.database);
        self.database_shown = true;
        self.sheet = None;
    }

    /// Clears what belonged to another game.
    fn follow(&mut self, rom: Option<&str>) {
        if self.rom.as_deref() != rom {
            *self = Self {
                database: std::mem::take(&mut self.database),
                database_shown: self.database_shown,
                size: self.size,
                signed: self.signed,
                ..Self::default()
            };
            self.rom = rom.map(str::to_owned);
        }
    }
}

const SIZES: [(ValueSize, &str); 3] = [
    (ValueSize::Byte, keys::SEARCH_SIZE_BYTE),
    (ValueSize::Word, keys::SEARCH_SIZE_WORD),
    (ValueSize::Bcd, keys::SEARCH_SIZE_BCD),
];

const COMPARISONS: [(Comparison, &str); 6] = [
    (Comparison::Equal, "="),
    (Comparison::NotEqual, "\u{2260}"),
    (Comparison::Less, "<"),
    (Comparison::Greater, ">"),
    (Comparison::LessOrEqual, "\u{2264}"),
    (Comparison::GreaterOrEqual, "\u{2265}"),
];

fn match_text(matched: SheetMatch) -> (&'static str, [f32; 4]) {
    match matched {
        SheetMatch::Exact => (keys::CHEATS_MATCH_EXACT, GOOD),
        SheetMatch::FileName => (keys::CHEATS_MATCH_FILE_NAME, GOOD),
        SheetMatch::SameRegion => (keys::CHEATS_MATCH_SAME_REGION, WARNING),
        SheetMatch::OtherRegion => (keys::CHEATS_MATCH_OTHER_REGION, BAD),
    }
}

fn codes_text(cheat: &zax_cheats::Cheat) -> String {
    cheat
        .codes
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("+")
}

impl ZaxUi {
    /// The game options menu's entries of the two windows; greyed without a game.
    pub(super) fn draw_cheats_entries(&mut self, ui: &Ui, host: &AppHost) {
        let game = host.current_rom_path().is_some();
        if ui
            .menu_item_config(strings::get(keys::MENU_CHEATS))
            .enabled(game)
            .build()
        {
            self.state.cheats = true;
            self.menu_chosen = true;
        }
        if ui
            .menu_item_config(strings::get(keys::MENU_RAM_SEARCH))
            .enabled(game)
            .build()
        {
            self.state.ram_search = true;
            self.menu_chosen = true;
        }
    }

    /// The cheats window.
    pub(super) fn draw_cheats(&mut self, ui: &Ui, host: &mut AppHost) {
        let Some(rom) = host.current_rom_path().map(str::to_owned) else {
            self.state.cheats = false;
            return;
        };
        self.cheats.follow(Some(&rom));
        if self.cheats.database.is_empty() && !self.cheats.database_shown {
            self.cheats.database = host
                .cheat_database_path()
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_default();
            self.cheats.database_shown = true;
        }
        let name = host
            .current_game_title()
            .map_or_else(|| rom.clone(), str::to_owned);
        let title = strings::format(&strings::get(keys::CHEATS_TITLE), &[&name]);
        let mut open = true;
        let window = ui
            .window(format!("{title}###cheats"))
            .size([620.0, 560.0], Condition::FirstUseEver)
            .position(centre(ui, 620.0, 560.0), Condition::FirstUseEver)
            .flags(WindowFlags::NO_COLLAPSE)
            .opened(&mut open)
            .begin();
        if window.is_some() {
            Self::draw_cheat_list(ui, host);
            ui.separator();
            self.draw_cheat_entry(ui, host);
            ui.separator();
            self.draw_cheat_database(ui, host);
            ui.separator();
            if ui.button(strings::get(keys::MENU_RAM_SEARCH)) {
                self.state.ram_search = true;
            }
            ui.same_line();
            ui.text_disabled(strings::get(keys::CHEATS_KEYS_NOTE));
        }
        drop(window);
        self.state.cheats = open;
    }

    fn draw_cheat_list(ui: &Ui, host: &mut AppHost) {
        let mut on = host.cheats_on();
        if ui.checkbox(strings::get(keys::CHEATS_ALL_ON), &mut on) {
            host.set_cheats_on(on);
        }
        let mut list = host.cheats().clone();
        if on && list.any_enabled() && host.achievements.is_enabled() {
            let _colour = ui.push_style_color(StyleColor::Text, WARNING);
            ui.text_wrapped(strings::get(keys::CHEATS_ACHIEVEMENTS_NOTE));
        }
        if host.cheats_game_genie_refused() && !list.rom_patches().is_empty() {
            let _colour = ui.push_style_color(StyleColor::Text, BAD);
            ui.text_wrapped(strings::get(keys::CHEATS_GENIE_REFUSED));
        }
        if list.cheats.is_empty() {
            ui.text_wrapped(strings::get(keys::CHEATS_EMPTY));
            return;
        }
        let mut changed = false;
        let mut remove = None;
        let selected = host.selected_cheat();
        let flags =
            TableFlags::ROW_BG | TableFlags::BORDERS_INNER_V | TableFlags::SIZING_STRETCH_PROP;
        if let Some(_table) = ui.begin_table_with_flags("##cheat_list", 3, flags) {
            for (index, cheat) in list.cheats.iter_mut().enumerate() {
                let _id = ui.push_id_usize(index);
                ui.table_next_row();
                ui.table_next_column();
                let label = host_cheats::first_line(&cheat.description).to_owned();
                let mark = if index == selected { "> " } else { "" };
                if ui.checkbox(format!("{mark}{label}"), &mut cheat.enabled) {
                    changed = true;
                }
                if ui.is_item_hovered() && cheat.description.contains('\n') {
                    ui.tooltip_text(&cheat.description);
                }
                ui.table_next_column();
                ui.text_disabled(codes_text(cheat));
                ui.table_next_column();
                if ui.small_button(strings::get(keys::CHEATS_DELETE)) {
                    remove = Some(index);
                }
            }
        }
        if let Some(index) = remove {
            list.cheats.remove(index);
            changed = true;
        }
        if changed {
            if list.any_enabled() {
                host.set_cheats_on(true);
            }
            host.replace_cheats(list);
        }
    }

    fn draw_cheat_entry(&mut self, ui: &Ui, host: &mut AppHost) {
        let window = &mut self.cheats;
        ui.text(strings::get(keys::CHEATS_ADD_HEADER));
        ui.set_next_item_width(260.0);
        ui.input_text(
            format!(
                "{}##cheat_description",
                strings::get(keys::CHEATS_DESCRIPTION)
            ),
            &mut window.description,
        )
        .build();
        ui.set_next_item_width(260.0);
        let entered = ui
            .input_text(
                format!("{}##cheat_codes", strings::get(keys::CHEATS_CODES)),
                &mut window.codes,
            )
            .enter_returns_true(true)
            .build();
        ui.same_line();
        if ui.button(strings::get(keys::CHEATS_ADD)) || entered {
            match host_cheats::typed_cheat(&window.description, &window.codes) {
                Ok(cheat) => {
                    let mut list = host.cheats().clone();
                    list.cheats.push(cheat);
                    host.set_cheats_on(true);
                    host.replace_cheats(list);
                    window.description.clear();
                    window.codes.clear();
                    window.error = None;
                }
                Err(error) => window.error = Some(error.to_string()),
            }
        }
        if let Some(error) = &window.error {
            let _colour = ui.push_style_color(StyleColor::Text, BAD);
            ui.text_wrapped(error);
        }
        ui.text_disabled(strings::get(keys::CHEATS_CODES_HINT));
    }

    #[allow(clippy::too_many_lines)] // one section, drawn top to bottom
    fn draw_cheat_database(&mut self, ui: &Ui, host: &mut AppHost) {
        if !ui.collapsing_header(
            strings::get(keys::CHEATS_DATABASE_HEADER),
            imgui::TreeNodeFlags::DEFAULT_OPEN,
        ) {
            return;
        }
        ui.set_next_item_width(340.0);
        ui.input_text(
            format!(
                "{}##cheat_database",
                strings::get(keys::CHEATS_DATABASE_PATH)
            ),
            &mut self.cheats.database,
        )
        .build();
        ui.same_line();
        if ui.small_button(strings::get(keys::CHEATS_DATABASE_USE)) {
            host.set_cheat_database(self.cheats.database.trim());
            self.cheats.sheet = None;
        }
        ui.same_line();
        if ui.small_button(strings::get(keys::CHEATS_DATABASE_BROWSE)) {
            self.open_browser(host, BrowserPurpose::CheatDatabase);
        }
        if host.cheat_database_path().is_none() {
            ui.text_disabled(strings::get(keys::CHEATS_BUILTIN_DATABASE));
        }
        if self.cheats.sheet.is_none()
            || ui.small_button(strings::get(keys::CHEATS_DATABASE_LOOK_UP))
        {
            self.cheats.sheet = Some(host.cheat_sheet());
        }
        let mut list = host.cheats().clone();
        let mut add = Vec::new();
        match &self.cheats.sheet {
            None => {}
            Some(Err(error)) => {
                let _colour = ui.push_style_color(StyleColor::Text, BAD);
                ui.text_wrapped(strings::format(
                    &strings::get(keys::CHEATS_DATABASE_ERROR),
                    &[error],
                ));
            }
            Some(Ok(None)) => {
                ui.text_wrapped(strings::get(keys::CHEATS_NO_SHEET));
            }
            Some(Ok(Some(sheet))) => {
                ui.text(strings::format(
                    &strings::get(keys::CHEATS_SHEET),
                    &[&sheet.name],
                ));
                let (text, colour) = match_text(sheet.matched);
                {
                    let _colour = ui.push_style_color(StyleColor::Text, colour);
                    ui.text_wrapped(strings::get(text));
                }
                if !sheet.imported.skipped.is_empty() {
                    ui.text_disabled(strings::format(
                        &strings::get(keys::CHEATS_SKIPPED),
                        &[&sheet.imported.skipped.len().to_string()],
                    ));
                }
                let missing: Vec<usize> = sheet
                    .imported
                    .cheats
                    .iter()
                    .enumerate()
                    .filter(|(_, cheat)| !list.cheats.iter().any(|own| own.codes == cheat.codes))
                    .map(|(index, _)| index)
                    .collect();
                if !missing.is_empty() && ui.small_button(strings::get(keys::CHEATS_ADD_ALL)) {
                    add.extend(missing.iter().copied());
                }
                let height = (ui.frame_height_with_spacing() * 8.0).max(120.0);
                ui.child_window("##sheet")
                    .size([0.0, height])
                    .border(true)
                    .build(|| {
                        for (index, cheat) in sheet.imported.cheats.iter().enumerate() {
                            let _id = ui.push_id_usize(index);
                            let owned = !missing.contains(&index);
                            let disabled = ui.begin_disabled(owned);
                            if ui.small_button(strings::get(keys::CHEATS_ADD)) {
                                add.push(index);
                            }
                            drop(disabled);
                            ui.same_line();
                            ui.text(host_cheats::first_line(&cheat.description));
                            if ui.is_item_hovered() {
                                ui.tooltip_text(format!(
                                    "{}\n{}",
                                    cheat.description,
                                    codes_text(cheat)
                                ));
                            }
                        }
                    });
                // A single one asked is on; "Add all" leaves them off for the player to choose.
                let all = add.len() > 1;
                for index in add.drain(..) {
                    if let Some(cheat) = sheet.imported.cheats.get(index) {
                        let mut cheat = cheat.clone();
                        cheat.enabled = !all;
                        list.cheats.push(cheat);
                    }
                }
            }
        }
        if list != *host.cheats() {
            if list.any_enabled() {
                host.set_cheats_on(true);
            }
            host.replace_cheats(list);
        }
    }

    /// The RAM search window.
    pub(super) fn draw_ram_search(&mut self, ui: &Ui, host: &mut AppHost) {
        let rom = host.current_rom_path().map(str::to_owned);
        self.cheats.follow(rom.as_deref());
        let mut open = true;
        let window = ui
            .window(format!("{}###ram_search", strings::get(keys::SEARCH_TITLE)))
            .size([460.0, 560.0], Condition::FirstUseEver)
            .position(centre(ui, 460.0, 560.0), Condition::FirstUseEver)
            .flags(WindowFlags::NO_COLLAPSE)
            .opened(&mut open)
            .begin();
        if window.is_some() {
            if rom.is_none() {
                ui.text_wrapped(strings::get(keys::SEARCH_NO_GAME));
            } else {
                self.draw_search_body(ui, host);
            }
        }
        drop(window);
        self.state.ram_search = open;
    }

    #[allow(clippy::too_many_lines)] // one window, drawn top to bottom
    fn draw_search_body(&mut self, ui: &Ui, host: &mut AppHost) {
        let view = host.ram_search_view().clone();
        let window = &mut self.cheats;
        let sizes: Vec<String> = SIZES.iter().map(|(_, key)| strings::get(key)).collect();
        ui.set_next_item_width(180.0);
        ui.combo_simple_string(strings::get(keys::SEARCH_SIZE), &mut window.size, &sizes);
        ui.same_line();
        ui.checkbox(strings::get(keys::SEARCH_SIGNED), &mut window.signed);
        if ui.button(strings::get(keys::SEARCH_START)) {
            host.ram_search(RamSearchAction::Start {
                size: SIZES[window.size.min(SIZES.len() - 1)].0,
                signed: window.signed,
            });
            window.selected = None;
        }
        if !view.active {
            ui.text_wrapped(strings::get(keys::SEARCH_NOT_STARTED));
            return;
        }
        ui.same_line();
        ui.text(strings::format(
            &strings::get(keys::SEARCH_COUNT),
            &[&view.count.to_string()],
        ));
        let quick = [
            (
                keys::SEARCH_CHANGED,
                Comparison::NotEqual,
                Operand::Previous,
            ),
            (keys::SEARCH_UNCHANGED, Comparison::Equal, Operand::Previous),
            (
                keys::SEARCH_INCREASED,
                Comparison::Greater,
                Operand::Previous,
            ),
            (keys::SEARCH_DECREASED, Comparison::Less, Operand::Previous),
        ];
        for (index, (key, comparison, operand)) in quick.into_iter().enumerate() {
            if index > 0 {
                ui.same_line();
            }
            if ui.button(strings::get(key)) {
                host.ram_search(RamSearchAction::Filter(comparison, operand));
            }
        }
        let comparisons: Vec<String> = COMPARISONS
            .iter()
            .map(|(_, text)| (*text).to_owned())
            .collect();
        ui.set_next_item_width(60.0);
        ui.combo_simple_string("##comparison", &mut window.comparison, &comparisons);
        ui.same_line();
        let operands = [
            strings::get(keys::SEARCH_TO_PREVIOUS),
            strings::get(keys::SEARCH_TO_DELTA),
            strings::get(keys::SEARCH_TO_VALUE),
        ];
        ui.set_next_item_width(170.0);
        ui.combo_simple_string("##operand", &mut window.operand, &operands);
        if window.operand > 0 {
            ui.same_line();
            ui.set_next_item_width(90.0);
            ui.input_int("##number", &mut window.number).build();
        }
        ui.same_line();
        if ui.button(strings::get(keys::SEARCH_FILTER)) {
            let operand = match window.operand {
                0 => Operand::Previous,
                1 => Operand::PreviousPlus(i64::from(window.number)),
                _ => Operand::Value(i64::from(window.number)),
            };
            let comparison = COMPARISONS[window.comparison.min(COMPARISONS.len() - 1)].0;
            host.ram_search(RamSearchAction::Filter(comparison, operand));
        }
        ui.same_line();
        let disabled = ui.begin_disabled(!view.can_undo);
        if ui.button(strings::get(keys::SEARCH_UNDO)) {
            host.ram_search(RamSearchAction::Undo);
        }
        drop(disabled);
        if view.count > SEARCH_ROWS {
            ui.text_disabled(strings::format(
                &strings::get(keys::SEARCH_SHOWN),
                &[&SEARCH_ROWS.to_string()],
            ));
        }
        let height = -3.0 * ui.frame_height_with_spacing();
        let flags = TableFlags::ROW_BG | TableFlags::BORDERS_INNER_V | TableFlags::SCROLL_Y;
        let mut remove = None;
        if let Some(_table) =
            ui.begin_table_with_sizing("##candidates", 4, flags, [0.0, height], 0.0)
        {
            ui.table_setup_scroll_freeze(0, 1);
            ui.table_setup_column(strings::get(keys::SEARCH_ADDRESS));
            ui.table_setup_column(strings::get(keys::SEARCH_PREVIOUS));
            ui.table_setup_column(strings::get(keys::SEARCH_CURRENT));
            ui.table_setup_column("##remove");
            ui.table_headers_row();
            for (address, previous, now) in &view.rows {
                let _id = ui.push_id_int(i32::from(*address));
                ui.table_next_row();
                ui.table_next_column();
                let chosen = window.selected == Some(*address);
                if ui
                    .selectable_config(format!("${address:04X}"))
                    .selected(chosen)
                    .span_all_columns(false)
                    .build()
                {
                    window.selected = Some(*address);
                    window.value = now
                        .and_then(|now| i32::try_from(now).ok())
                        .unwrap_or_default();
                }
                ui.table_next_column();
                ui.text(previous.to_string());
                ui.table_next_column();
                match now {
                    Some(now) if now != previous => {
                        let _colour = ui.push_style_color(StyleColor::Text, WARNING);
                        ui.text(now.to_string());
                    }
                    Some(now) => ui.text(now.to_string()),
                    None => ui.text_disabled("-"),
                }
                ui.table_next_column();
                if ui.small_button("x") {
                    remove = Some(*address);
                }
            }
        }
        if let Some(address) = remove {
            host.ram_search(RamSearchAction::Remove(address));
            if window.selected == Some(address) {
                window.selected = None;
            }
        }
        if let (Some(address), Some(size)) = (window.selected, view.size) {
            ui.text(format!("${address:04X}"));
            ui.same_line();
            ui.set_next_item_width(90.0);
            ui.input_int(
                format!("{}##value", strings::get(keys::SEARCH_VALUE)),
                &mut window.value,
            )
            .build();
            ui.same_line();
            if ui.button(strings::get(keys::SEARCH_ADD_CHEAT)) {
                match host_cheats::codes_for_value(address, size, i64::from(window.value)).and_then(
                    |codes| {
                        host_cheats::typed_cheat(
                            &format!("${address:04X} = {}", window.value),
                            &codes,
                        )
                        .ok()
                    },
                ) {
                    Some(cheat) => {
                        let added = cheat.description.clone();
                        let mut list: CheatList = host.cheats().clone();
                        list.cheats.push(cheat);
                        host.set_cheats_on(true);
                        host.replace_cheats(list);
                        window.message = Some((
                            strings::format(&strings::get(keys::SEARCH_ADDED), &[&added]),
                            false,
                        ));
                    }
                    None => {
                        window.message = Some((strings::get(keys::SEARCH_BAD_VALUE), true));
                    }
                }
            }
        }
        if let Some((message, error)) = &window.message {
            let _colour = ui.push_style_color(StyleColor::Text, if *error { BAD } else { GOOD });
            ui.text_wrapped(message);
        }
    }
}
