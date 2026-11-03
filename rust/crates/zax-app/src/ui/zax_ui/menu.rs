//! The menu bar (REQ-UI-200).

use std::path::PathBuf;

use imgui::Ui;
use zax_settings::catalogue::SettingId;

use super::ZaxUi;
use crate::application::AppHost;
use crate::emulation::commands::{Command, CommandArgs};
use crate::ui::commands::UiCommand;
use crate::ui::handheld::game_options::{self, GameOptionsModel, GameRow};
use crate::ui::handheld::hacks::{HackRow, HacksModel, HacksView};
use crate::ui::handheld::options::{OptionRow, OptionsPage};
use crate::ui::handheld::{options, overscan};
use crate::ui::menu_bar::{Menu, MenuBarModel, MenuItem, MenuItemKind, MenuWarning};
use crate::ui::{keys, strings};

/// The title of the game options menu when the game has some (owner request).
const GAME_OWN: [f32; 4] = [0.3, 0.9, 1.0, 1.0];

impl ZaxUi {
    /// Draws the bar and returns its height, and whether one of its menus is open or the
    /// pointer is over it.
    pub(super) fn draw_menu_bar(&mut self, ui: &Ui, host: &mut AppHost) -> (f32, bool) {
        let Some(_bar) = ui.begin_main_menu_bar() else {
            return (0.0, false);
        };
        let mut engaged = ui.is_window_hovered();
        let height = ui.window_size()[1];
        // The capture aid of --ui-shots: the popup identifier a menu-bar entry opens is the one
        // its label hashes to in this very window, so asking for it here is the same thing a
        // click would do.
        if let Some(forced) = self.force_open_menu.take() {
            ui.open_popup(forced);
        }
        let mut chosen = None;
        for Menu { title, items } in self.menu_bar.menus() {
            if let Some(_menu) = ui.begin_menu(title) {
                engaged = true;
                chosen = chosen.or(draw_items(ui, items, host));
            }
        }
        let (open, hacks) = self.draw_game_options(ui, host);
        engaged |= open;
        if hacks {
            self.open_hacks(host, HacksView::Widescreen);
            self.menu_chosen = true;
        }
        match chosen {
            Some(Chosen::Host(command, args)) => {
                Self::activate(host, command, args);
                self.menu_chosen = true;
            }
            Some(Chosen::Ui(command)) => self.run_ui_command(command),
            None => {}
        }
        (height, engaged)
    }
}

impl ZaxUi {
    /// The options of the game in play (Rust-only, owner request): the widescreen (a click turns
    /// it on with everything or off, its settings a window), the hacks (a submenu of the aids), a
    /// submenu per option with its values, "Use Global" first, an entry that puts everything
    /// back, and — since RO-2 part 1 — the game's achievements at the end. Greyed with no game.
    /// Returns whether the menu is open, and whether the widescreen's settings were asked for.
    fn draw_game_options(&mut self, ui: &Ui, host: &mut AppHost) -> (bool, bool) {
        let title = strings::get(keys::HANDHELD_HINT_GAME_OPTIONS);
        let Some(rom) = host.current_rom_path().map(str::to_owned) else {
            // Greyed: there is no game to set options for, nor achievements to list.
            drop(ui.begin_menu_with_enabled(&title, false));
            return (false, false);
        };
        let options = host.game_options(&rom);
        let own = options
            .iter()
            .any(|(id, _)| game_options::is_game_row_setting(*id));
        let colour = own.then(|| ui.push_style_color(imgui::StyleColor::Text, GAME_OWN));
        let menu = ui.begin_menu(&title);
        drop(colour);
        let Some(_menu) = menu else {
            return (false, false);
        };
        // The hacks are studied as the menu opens, and kept while it is open.
        let path = std::path::Path::new(&rom);
        if ui.is_window_appearing()
            || self
                .menu_hacks
                .as_ref()
                .is_none_or(|hacks| hacks.rom() != path)
        {
            self.menu_hacks = Some(
                HacksModel::for_rom(host, path, host.current_game_title())
                    .with_view(HacksView::Aids),
            );
        }
        let name = host.current_game_title().unwrap_or_default().to_owned();
        let mut model = GameOptionsModel::new(PathBuf::from(&rom), name, host.settings(), options);
        let mut changed = false;
        let mut hacks = false;
        // RO-3 §3: the first level is the console main page, row for row and in its order — the
        // widescreen between the overscan and the region, the hacks last, and the two submenus
        // where the console climbs to a page.
        for (index, row) in game_options::GAME_MAIN.iter().enumerate() {
            let _id = ui.push_id_usize(index);
            match row {
                GameRow::ResetAll => {
                    if ui
                        .menu_item_config(strings::get(keys::HANDHELD_GAME_OPTIONS_RESET))
                        .enabled(model.has_options())
                        .build()
                    {
                        model.reset_all();
                        changed = true;
                    }
                    ui.separator();
                }
                GameRow::Widescreen => {
                    hacks |= self
                        .menu_hacks
                        .as_mut()
                        .is_some_and(|model| draw_widescreen(ui, host, model));
                }
                GameRow::Hacks => {
                    ui.separator();
                    if let Some(aids) = self.menu_hacks.as_mut() {
                        draw_aids(ui, host, aids);
                    }
                }
                GameRow::Option(OptionRow::Page(page)) => {
                    ui.separator();
                    changed |= draw_game_page(ui, host, &mut model, *page);
                }
                GameRow::Option(row) => {
                    changed |= draw_game_rows(ui, host, &mut model, std::slice::from_ref(row), 0);
                }
            }
        }
        ui.separator();
        self.draw_achievements_entry(ui, host);
        if changed {
            host.set_game_options(&rom, model.options());
        }
        (true, hacks)
    }
}

/// A submenu per row of `rows`, labelled "Name: value" as the console page labels them, with
/// "Use Global" first and a tick on the value in force. Returns whether a value changed.
///
/// `id_base` keeps the identifier of a row of one group apart from the row of another group at
/// the same level of the menu, whatever their labels.
fn draw_game_rows(
    ui: &Ui,
    host: &AppHost,
    model: &mut GameOptionsModel,
    rows: &[options::OptionRow],
    id_base: usize,
) -> bool {
    let mut changed = false;
    for (index, row) in rows.iter().enumerate() {
        let _id = ui.push_id_usize(id_base + index);
        let own = model.is_own(*row);
        let label = format!("{}: {}", options::label(*row), model.value_text(*row));
        let colour = own.then(|| ui.push_style_color(imgui::StyleColor::Text, GAME_OWN));
        let submenu = ui.begin_menu(&label);
        drop(colour);
        let Some(_submenu) = submenu else {
            continue;
        };
        let global =
            GameOptionsModel::new(PathBuf::new(), String::new(), host.settings(), Vec::new());
        if *row == options::OptionRow::Overscan {
            changed |= draw_game_overscan(ui, model, &global);
            continue;
        }
        // Q-103: a profile whose scripts change the core runs at 0 whatever this row says, and
        // the note is the only place the menu can explain it.
        if let Some(note) = crate::ui::options::forced_run_ahead_note(
            SettingId::EmuRunAhead,
            *row == options::OptionRow::Setting(SettingId::EmuRunAhead)
                && host
                    .session()
                    .is_some_and(crate::game_session::GameSession::forces_run_ahead_to_zero),
        ) {
            ui.text_disabled(&note);
            ui.separator();
        }
        let use_global = global.value_text(*row);
        if ui.menu_item_config(&use_global).selected(!own).build() {
            model.use_global(*row);
            changed = true;
        }
        ui.separator();
        let current = own.then(|| model.value_text(*row));
        for (choice, text) in model.choices(*row).iter().enumerate() {
            let _choice = ui.push_id_usize(choice);
            if ui
                .menu_item_config(text)
                .selected(current.as_deref() == Some(text.as_str()))
                .build()
            {
                model.choose(*row, choice);
                changed = true;
            }
        }
    }
    changed
}

/// One page of the game tree as a submenu (RO-3 §3): its title in the colour of a game's own
/// value as soon as one value under it — on this page or on a page of it — is the game's own, and
/// one entry per row, a page of its own becoming a submenu in its turn. Returns whether a value
/// changed.
fn draw_game_page(
    ui: &Ui,
    host: &AppHost,
    model: &mut GameOptionsModel,
    page: OptionsPage,
) -> bool {
    let own = model.own_count(page) > 0;
    let colour = own.then(|| ui.push_style_color(imgui::StyleColor::Text, GAME_OWN));
    let submenu = ui.begin_menu(strings::get(page.title_key()));
    drop(colour);
    if submenu.is_none() {
        return false;
    }
    let mut changed = false;
    for (index, row) in page.game_rows().iter().enumerate() {
        let _id = ui.push_id_usize(index);
        match row {
            OptionRow::Group(key) => {
                ui.separator();
                ui.text_disabled(strings::get(key));
            }
            OptionRow::Page(under) => changed |= draw_game_page(ui, host, model, *under),
            row => changed |= draw_game_rows(ui, host, model, std::slice::from_ref(row), 0),
        }
    }
    changed
}

/// The game's overscan (Rust-only, owner request): a submenu per side, "Global" (its region's)
/// or 0 to 32 pixels, and one for the mode, "Use Global" first. Returns whether a value changed.
fn draw_game_overscan(ui: &Ui, model: &mut GameOptionsModel, global: &GameOptionsModel) -> bool {
    let mut changed = false;
    for (index, id) in overscan::GAME_ROWS.iter().enumerate() {
        let _id = ui.push_id_usize(index);
        let own = model.edited().has_game_value(*id);
        let value = overscan::value_text(*id, model.edited());
        let label = format!("{}: {value}", overscan::label(*id));
        let colour = own.then(|| ui.push_style_color(imgui::StyleColor::Text, GAME_OWN));
        let submenu = ui.begin_menu(&label);
        drop(colour);
        let Some(_submenu) = submenu else {
            continue;
        };
        let current = model.edited().get(*id).as_int();
        if *id == SettingId::VideoOverscanMode {
            let global_value = global.edited().get(*id).as_int();
            let use_global = strings::format(
                &strings::get(keys::HANDHELD_USE_GLOBAL),
                &[&overscan::value_text(*id, global.edited())],
            );
            if ui.menu_item_config(&use_global).selected(!own).build() {
                model.set_setting(*id, global_value);
                changed = true;
            }
            ui.separator();
        }
        for (value, text) in overscan::choices(*id) {
            let _value = ui.push_id_int(value);
            let selected = value == current && (own || *id != SettingId::VideoOverscanMode);
            if ui.menu_item_config(&text).selected(selected).build() {
                model.set_setting(*id, value);
                changed = true;
            }
        }
    }
    changed
}

impl ZaxUi {
    /// Opens the hacks window on the game in play, showing the rows of `view`.
    pub(super) fn open_hacks(&mut self, host: &AppHost, view: HacksView) {
        let Some(rom) = host.current_rom_path() else {
            return;
        };
        self.hacks = Some(
            HacksModel::for_rom(host, std::path::Path::new(rom), host.current_game_title())
                .with_view(view),
        );
        self.state.hacks = true;
    }
}

/// The widescreen of the game in play: a check turning it on with its advanced mode and every
/// option of it, or off, and an entry that opens its settings in a window. Returns whether the
/// settings were asked for. RO-3 §3 split this from the aids, which the console main page puts at
/// the other end of the list.
fn draw_widescreen(ui: &Ui, host: &mut AppHost, hacks: &mut HacksModel) -> bool {
    if hacks.is_capable()
        && ui
            .menu_item_config(strings::get(keys::HACK_WIDESCREEN))
            .selected(hacks.widescreen_all_on())
            .build()
    {
        hacks.toggle_widescreen_all();
        hacks.apply(host);
    }
    ui.menu_item(strings::get(keys::MENU_WIDESCREEN_SETTINGS))
}

/// The hacks of the game in play: a submenu with a check per aid, which works with or without the
/// widescreen.
fn draw_aids(ui: &Ui, host: &mut AppHost, hacks: &mut HacksModel) {
    let mut changed = false;
    let rows = hacks.rows();
    let menu = ui.begin_menu_with_enabled(strings::get(keys::MENU_AIDS), !rows.is_empty());
    if let Some(_menu) = menu {
        for row in rows {
            if let HackRow::PatchOption(_) = row {
                if ui
                    .menu_item_config(hacks.label(row))
                    .selected(hacks.is_on(row))
                    .build()
                {
                    hacks.step(row, 1);
                    changed = true;
                }
            }
        }
    }
    if changed {
        hacks.apply(host);
    }
}

/// What the user chose in a menu: a command of the host, or one of the interface.
enum Chosen {
    Host(Command, CommandArgs),
    Ui(UiCommand),
}

/// Draws the entries of a menu and returns the command the user chose, if any.
fn draw_items(ui: &Ui, items: &[MenuItem], host: &AppHost) -> Option<Chosen> {
    let mut chosen = None;
    for (index, item) in items.iter().enumerate() {
        if item.kind == MenuItemKind::Separator {
            ui.separator();
            continue;
        }
        let _id = ui.push_id_usize(index);
        if let Some(submenu) = item.submenu {
            if let Some(_menu) = ui.begin_menu(&item.label) {
                let children = MenuBarModel::children(submenu, host);
                chosen = chosen.or(draw_items(ui, &children, host));
            }
            continue;
        }
        let enabled = MenuBarModel::is_enabled(item, host);
        let ticked = MenuBarModel::is_checked(item, host);
        // A state made with the built-in hack set the other way (orange) or with another
        // version of it (red).
        let _warning = match item.warning {
            MenuWarning::None => None,
            MenuWarning::Switch => {
                Some(ui.push_style_color(imgui::StyleColor::Text, [1.0, 0.65, 0.2, 1.0]))
            }
            MenuWarning::Blocked => {
                Some(ui.push_style_color(imgui::StyleColor::Text, [0.95, 0.35, 0.3, 1.0]))
            }
        };
        let activated = ui
            .menu_item_config(&item.label)
            .shortcut(&item.shortcut)
            .selected(ticked)
            .enabled(enabled)
            .build();
        if activated {
            if let Some(command) = item.command {
                chosen = chosen.or(Some(Chosen::Host(command, item.args.clone())));
            } else if let Some(command) = item.ui_command {
                chosen = chosen.or(Some(Chosen::Ui(command)));
            }
        }
    }
    chosen
}
