//! The menu bar (REQ-UI-200).

use std::path::PathBuf;

use imgui::Ui;

use super::ZaxUi;
use crate::application::AppHost;
use crate::emulation::commands::{Command, CommandArgs};
use crate::ui::commands::UiCommand;
use crate::ui::handheld::game_options::{GameOptionsModel, GAME_ROWS};
use crate::ui::handheld::options;
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
        engaged |= draw_game_options(ui, host);
        match chosen {
            Some(Chosen::Host(command, args)) => Self::activate(host, command, args),
            Some(Chosen::Ui(command)) => self.run_ui_command(command),
            None => {}
        }
        (height, engaged)
    }
}

/// The options of the game in play (Rust-only, owner request): a submenu per option with its
/// values, "Use Global" first, and an entry that puts everything back. Greyed with no game.
/// Returns whether the menu is open.
fn draw_game_options(ui: &Ui, host: &mut AppHost) -> bool {
    let title = strings::get(keys::HANDHELD_HINT_GAME_OPTIONS);
    let Some(rom) = host.current_rom_path().map(str::to_owned) else {
        // Greyed: there is no game to set options for.
        drop(ui.begin_menu_with_enabled(&title, false));
        return false;
    };
    let options = host.game_options(&rom);
    let colour =
        (!options.is_empty()).then(|| ui.push_style_color(imgui::StyleColor::Text, GAME_OWN));
    let menu = ui.begin_menu(&title);
    drop(colour);
    let Some(_menu) = menu else {
        return false;
    };
    let name = host.current_game_title().unwrap_or_default().to_owned();
    let mut model = GameOptionsModel::new(PathBuf::from(&rom), name, host.settings(), options);
    let mut changed = false;
    if ui
        .menu_item_config(strings::get(keys::HANDHELD_GAME_OPTIONS_RESET))
        .enabled(model.has_options())
        .build()
    {
        model = GameOptionsModel::new(
            PathBuf::from(&rom),
            String::new(),
            host.settings(),
            Vec::new(),
        );
        changed = true;
    }
    ui.separator();
    for (index, row) in GAME_ROWS.iter().enumerate() {
        let _id = ui.push_id_usize(index);
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
    if changed {
        host.set_game_options(&rom, model.options());
    }
    true
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
