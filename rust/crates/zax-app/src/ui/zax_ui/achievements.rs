//! RetroAchievements in the desktop interface (Rust-only, owner request): its menu and the
//! window of the game's achievements.

use imgui::{Condition, Ui, WindowFlags};

use super::{centre, ZaxUi};
use crate::application::AppHost;
use crate::ui::options::OptionsTab;
use crate::ui::{achievements_view, keys, strings};

impl ZaxUi {
    /// The RetroAchievements menu: the account, the game's achievements and the options.
    /// Returns whether it is open. An entry chosen over the console mode takes the desktop back,
    /// where its window shows.
    pub(super) fn draw_achievements_menu(&mut self, ui: &Ui, host: &mut AppHost) -> bool {
        let Some(_menu) = ui.begin_menu(strings::get(keys::ACH_MENU)) else {
            return false;
        };
        ui.text_disabled(achievements_view::account_text(&host.achievements));
        ui.separator();
        if ui
            .menu_item_config(strings::get(keys::ACH_LIST))
            .enabled(host.achievements.game().is_some())
            .build()
        {
            self.state.achievements = true;
            self.menu_chosen = true;
        }
        if ui.menu_item(strings::get(keys::ACH_ACCOUNT)) {
            self.state.options = true;
            self.force_options_tab = Some(OptionsTab::Achievements);
            self.menu_chosen = true;
        }
        true
    }

    /// The window of the game's achievements.
    pub(super) fn draw_achievement_list(&mut self, ui: &Ui, host: &mut AppHost) {
        let mut open = self.state.achievements;
        let window = ui
            .window(format!("{}###achievements", strings::get(keys::ACH_LIST)))
            .size([520.0, 560.0], Condition::FirstUseEver)
            .position(centre(ui, 520.0, 560.0), Condition::FirstUseEver)
            .flags(WindowFlags::NO_COLLAPSE)
            .opened(&mut open)
            .begin();
        if window.is_some() {
            achievements_view::draw_list(ui, &host.achievements);
        }
        self.state.achievements = open;
    }
}
