//! The menu bar (REQ-UI-200).

use imgui::Ui;

use super::ZaxUi;
use crate::application::AppHost;
use crate::emulation::commands::{Command, CommandArgs};
use crate::ui::menu_bar::{Menu, MenuBarModel, MenuItem, MenuItemKind};

impl ZaxUi {
    /// Draws the bar and returns its height.
    pub(super) fn draw_menu_bar(&mut self, ui: &Ui, host: &mut AppHost) -> f32 {
        let Some(_bar) = ui.begin_main_menu_bar() else {
            return 0.0;
        };
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
                chosen = chosen.or(draw_items(ui, items, host));
            }
        }
        if let Some((command, args)) = chosen {
            Self::activate(host, command, args);
        }
        height
    }
}

/// Draws the entries of a menu and returns the command the user chose, if any.
fn draw_items(ui: &Ui, items: &[MenuItem], host: &AppHost) -> Option<(Command, CommandArgs)> {
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
        let activated = ui
            .menu_item_config(&item.label)
            .shortcut(&item.shortcut)
            .selected(ticked)
            .enabled(enabled)
            .build();
        if activated {
            if let Some(command) = item.command {
                chosen = chosen.or(Some((command, item.args.clone())));
            }
        }
    }
    chosen
}
