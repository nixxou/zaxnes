//! RetroAchievements in the desktop interface (Rust-only, owner request): the game's entry in the
//! menu of the game's options, and the window of the game's achievements.

use imgui::{Condition, Ui, WindowFlags};

use super::{centre, ZaxUi};
use crate::application::AppHost;
use crate::ui::{achievements_view, keys, strings};

impl ZaxUi {
    /// The game's achievements, at the end of the game options menu (RO-2 part 1, owner
    /// decision): the account as a line of text, then the entry that opens the list, enabled
    /// only while RetroAchievements knows the game being played.
    ///
    /// The **account** entry is not here: it has to stay reachable with no game, and this menu is
    /// greyed without one, so it is an entry of the Tools menu
    /// (`UiCommand::AchievementsAccount`). Nothing else of the feature changed.
    pub(super) fn draw_achievements_entry(&mut self, ui: &Ui, host: &AppHost) {
        ui.text_disabled(achievements_view::account_text(&host.achievements));
        if ui
            .menu_item_config(strings::get(keys::ACH_LIST))
            .enabled(host.achievements.game().is_some())
            .build()
        {
            self.state.achievements = true;
            self.menu_chosen = true;
        }
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
