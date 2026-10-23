//! The layers over the game view: the welcome screen (REQ-UI-207), the notifications
//! (REQ-UI-206) and the frame counter (REQ-UI-095).

use std::fmt::Write as _;

use imgui::{Condition, Ui, WindowFlags};

use super::{play, UiFrame, ZaxUi};
use crate::application::AppHost;
use crate::audio::UiSound;
use crate::emulation::commands::{Command, CommandArgs};
use crate::emulation::notification_center::NotificationSeverity;
use crate::persistence::app_settings::AppSettings as _;
use crate::ui::{notifications, strings, text};

/// How often the counter takes a new value, in seconds.
const REFRESH_SECONDS: f32 = 0.5;

/// The colour of a notification of `severity`.
#[must_use]
pub const fn colour_of(severity: NotificationSeverity) -> [f32; 4] {
    match severity {
        NotificationSeverity::Error => [1.0, 0.3, 0.3, 1.0],
        NotificationSeverity::Warning => [1.0, 0.9, 0.3, 1.0],
        NotificationSeverity::Normal => [1.0, 1.0, 1.0, 1.0],
    }
}

impl ZaxUi {
    pub(super) fn draw_welcome(&mut self, ui: &Ui, host: &mut AppHost) {
        let display = ui.io().display_size;
        let width = 520.0_f32.min(260.0_f32.max(display[0] - 80.0));
        let height = 360.0_f32.min(200.0_f32.max(display[1] - 80.0));
        let flags = WindowFlags::NO_DECORATION
            | WindowFlags::NO_SAVED_SETTINGS
            | WindowFlags::NO_FOCUS_ON_APPEARING
            | WindowFlags::NO_NAV;
        let window = ui
            .window("##welcome")
            .position(
                [
                    display[0] * 0.5,
                    display[1] * 0.5 + self.chrome_height * 0.5,
                ],
                Condition::Always,
            )
            .position_pivot([0.5, 0.5])
            .size([width, height], Condition::Always)
            .bg_alpha(0.65)
            .flags(flags)
            .begin();
        if window.is_none() {
            return;
        }
        {
            let _heading = self.heading_font.map(|font| ui.push_font(font));
            ui.text(text::PRODUCT_NAME);
        }
        ui.text_disabled(text::version_line());
        ui.separator();
        ui.text(text::welcome_hint());
        ui.spacing();
        if ui.button(strings::get(&Command::OpenRom.label_key())) {
            Self::activate(host, Command::OpenRom, CommandArgs::default());
        }
        ui.spacing();
        ui.separator();
        ui.text(text::welcome_recent());
        let recent = Self::recent_roms(host);
        if recent.is_empty() {
            ui.text_disabled(text::welcome_no_recent());
            return;
        }
        let mut chosen = None;
        for (index, rom) in recent.iter().enumerate() {
            let name = std::path::Path::new(rom)
                .file_name()
                .map_or_else(|| rom.clone(), |name| name.to_string_lossy().into_owned());
            let _id = ui.push_id_usize(index);
            if ui.selectable(name) {
                chosen = Some(index);
            }
        }
        if let Some(index) = chosen {
            play(host, UiSound::Confirm);
            Self::open_recent(host, i32::try_from(index).unwrap_or(i32::MAX));
        }
    }

    pub(in crate::ui) fn draw_notification(ui: &Ui, host: &AppHost, bottom_margin: f32) {
        let Some(notification) = host.notifications.current() else {
            return;
        };
        let display = ui.io().display_size;
        let flags = WindowFlags::NO_DECORATION
            | WindowFlags::ALWAYS_AUTO_RESIZE
            | WindowFlags::NO_SAVED_SETTINGS
            | WindowFlags::NO_FOCUS_ON_APPEARING
            | WindowFlags::NO_NAV
            | WindowFlags::NO_INPUTS;
        let window = ui
            .window("##notification")
            .position(
                [display[0] * 0.5, display[1] - bottom_margin],
                Condition::Always,
            )
            .position_pivot([0.5, 1.0])
            .bg_alpha(0.5)
            .flags(flags)
            .begin();
        if window.is_some() {
            // AMD-036 §1: the text the loop produced is the English entry of the table, so it is
            // translated here, when it is drawn, and follows a language change while it is up.
            ui.text_colored(
                colour_of(notification.severity),
                notifications::translate(&notification.text),
            );
        }
    }

    pub(in crate::ui) fn draw_fps_counter(&mut self, ui: &Ui, frame: &UiFrame<'_>) {
        if !frame.host.settings().show_fps() {
            return;
        }
        self.fps_countdown -= frame.real_seconds;
        if self.fps_countdown <= 0.0 {
            self.fps_countdown = REFRESH_SECONDS;
            self.fps_shown = frame.frames_per_second;
            self.fps_text.clear();
            let _ = write!(self.fps_text, "{:.1} FPS", self.fps_shown);
        }
        let flags = WindowFlags::NO_DECORATION
            | WindowFlags::ALWAYS_AUTO_RESIZE
            | WindowFlags::NO_SAVED_SETTINGS
            | WindowFlags::NO_FOCUS_ON_APPEARING
            | WindowFlags::NO_NAV
            | WindowFlags::NO_INPUTS;
        let window = ui
            .window("FPS")
            .position(
                [ui.io().display_size[0] - 12.0, self.chrome_height + 12.0],
                Condition::Always,
            )
            .position_pivot([1.0, 0.0])
            .bg_alpha(0.45)
            .flags(flags)
            .begin();
        if window.is_some() {
            ui.text(&self.fps_text);
            if !frame.profile_line.is_empty() {
                ui.text(frame.profile_line);
            }
        }
    }
}
