//! RetroAchievements in the host (Rust-only, owner request): the settings, the frames, the
//! sign-in, and what the system reports for the notification area and the settings file.

use zax_settings::catalogue::SettingId;

use super::{EmulationHost, EmulationState, GameCatalog, GameView};
use crate::achievements::{self, Credentials};
use crate::emulation::notification_center::{NotificationSeverity, DEFAULT_SECONDS};
use crate::persistence::save_state_file::SaveStateContent;

impl<C: GameCatalog, V: GameView> EmulationHost<C, V> {
    /// Turns the system on or off and passes it the options, as the settings say.
    pub(super) fn apply_achievement_settings(&mut self) {
        let setting = |id| self.settings.get(id).as_bool();
        let (enabled, encore, unofficial) = (
            setting(SettingId::AchievementsEnabled),
            setting(SettingId::AchievementsEncore),
            setting(SettingId::AchievementsUnofficial),
        );
        self.achievements.set_options(encore, unofficial);
        let username = self
            .settings
            .get(SettingId::AchievementsUsername)
            .as_text()
            .to_owned();
        let token = self
            .settings
            .get(SettingId::AchievementsToken)
            .as_text()
            .to_owned();
        let was_enabled = self.achievements.is_enabled();
        self.achievements.set_enabled(enabled, &username, &token);
        if enabled && !was_enabled {
            // A game already running when the system turns on gets its achievements too.
            if let Some(core) = self
                .session
                .as_ref()
                .filter(|session| session.has_rom() && session.core_is_home())
                .map(crate::game_session::GameSession::core)
            {
                if let Some(image) = achievements::hash_image(core) {
                    self.achievements.start_game(core, image);
                }
            }
        }
    }

    /// Signs in to RetroAchievements; the password is not kept.
    pub fn sign_in_to_achievements(&mut self, username: &str, password: &str) {
        self.achievements.sign_in(username, password);
    }

    /// Signs out of RetroAchievements and forgets the saved token.
    pub fn sign_out_of_achievements(&mut self) {
        self.achievements.sign_out();
        self.collect_achievement_reports();
    }

    /// Evaluates the frame the core just ended, when it is here.
    pub(super) fn evaluate_achievements(&mut self) {
        if !self.achievements.is_enabled() {
            return;
        }
        if let Some(session) = self
            .session
            .as_ref()
            .filter(|session| session.has_rom() && session.core_is_home())
        {
            self.achievements.after_frame(session.core());
        }
    }

    /// The achievements' work of a displayed frame, the core back from the emulation thread.
    pub(super) fn update_achievements(&mut self, real_seconds: f64) {
        if !self.achievements.is_enabled() {
            return;
        }
        self.evaluate_achievements();
        let running = self.state == EmulationState::Running;
        let core = self
            .session
            .as_ref()
            .filter(|session| session.has_rom() && session.core_is_home())
            .map(crate::game_session::GameSession::core);
        self.achievements.tick(core, real_seconds, running);
        self.collect_achievement_reports();
    }

    /// The game's achievements at the end of a launch or of a save state's load: a new game
    /// loads them, the same one starts them over; a save state puts its progress back.
    pub(super) fn start_achievements(&mut self, state: Option<&SaveStateContent>) {
        if !self.achievements.is_enabled() {
            return;
        }
        let Some(core) = self
            .session
            .as_ref()
            .filter(|session| session.has_rom() && session.core_is_home())
            .map(crate::game_session::GameSession::core)
        else {
            return;
        };
        if let Some(image) = achievements::hash_image(core) {
            self.achievements.start_game(core, image);
        }
        let path = self.current_rom_path().map(std::path::PathBuf::from);
        self.achievements.set_rom_path(path);
        if let Some(state) = state {
            let progress = (!state.achievements.is_empty()).then_some(&state.achievements[..]);
            self.achievements.restore_progress(core, progress);
        }
    }

    /// Passes on what the system reported: the notices, the account to keep, the unlock sound
    /// and the log.
    fn collect_achievement_reports(&mut self) {
        for notice in self.achievements.take_notices() {
            let severity = if notice.error {
                NotificationSeverity::Error
            } else {
                NotificationSeverity::Normal
            };
            self.notifications
                .show(&notice.text, DEFAULT_SECONDS, severity);
        }
        match self.achievements.take_credentials() {
            Some(Credentials::Keep { username, token }) => {
                self.settings
                    .set(SettingId::AchievementsUsername, username.as_str());
                self.settings
                    .set(SettingId::AchievementsToken, token.as_str());
            }
            Some(Credentials::Forget) => {
                self.settings.set(SettingId::AchievementsToken, "");
            }
            None => {}
        }
        if self.achievements.take_sound()
            && self.settings.get(SettingId::AchievementsSound).as_bool()
        {
            if let Some(mixer) = &mut self.mixer {
                let jingle = achievements::unlock_jingle(mixer.sample_rate());
                mixer.play_clip(Some(jingle));
            }
        }
        for line in self.achievements.take_log() {
            self.log(&format!("rcheevos: {line}"));
        }
    }
}
