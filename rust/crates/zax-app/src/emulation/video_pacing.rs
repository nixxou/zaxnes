//! Keeps the window's buffer-swap synchronisation and frame cap equal to what the settings say
//! (SPEC-06 REQ-SCN-061, SPEC-09 `video.vsync` and `video.frame_rate`).
//!
//! This is the one place that applies them: at start-up from the persisted value, at every
//! change of either setting, and again after a window-state change, since the windowing layer
//! re-applies its own boolean vsync then and would undo the interval 2.
//!
//! The [`VideoPacingRequest`] is what the host asks for rather than the raw settings, because a
//! smoke run must not be paced by the display refresh rate.

use zax_platform::pacing::{self, DisplayPacingState, DisplaySurface};
use zax_settings::catalogue::SettingId;
use zax_settings::model::SettingsModel;

use crate::persistence::app_settings::AppSettings;

/// What the host wants in force on the window right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoPacingRequest {
    /// 0 off, 1 every refresh, 2 every other refresh (SPEC-09).
    pub vsync: i32,
    /// The cap to use when vsync is off; 0 for uncapped.
    pub target_frame_rate: i32,
}

/// Where the request comes from: the settings, or something the host decides.
pub type RequestSource = Box<dyn Fn(&SettingsModel) -> VideoPacingRequest>;

/// Receives one line per application of the pacing.
pub type PacingLog = Box<dyn Fn(&str)>;

/// The pacing applied to a window.
pub struct VideoPacing {
    request: RequestSource,
    log: Option<PacingLog>,
    state: Option<DisplayPacingState>,
    applications: u32,
}

impl VideoPacing {
    /// Creates the pacing; nothing is applied yet. With no `request` the settings
    /// `video.vsync` and `video.frame_rate` are followed as they stand. `log` receives one line
    /// per application.
    #[must_use]
    pub fn new(request: Option<RequestSource>, log: Option<PacingLog>) -> Self {
        Self {
            request: request.unwrap_or_else(|| {
                Box::new(|model| VideoPacingRequest {
                    vsync: model.vsync(),
                    target_frame_rate: model.frame_rate(),
                })
            }),
            log,
            state: None,
            applications: 0,
        }
    }

    /// What the last application put in force.
    #[must_use]
    pub const fn state(&self) -> Option<DisplayPacingState> {
        self.state
    }

    /// How many times the pacing has been applied.
    #[must_use]
    pub const fn applications(&self) -> u32 {
        self.applications
    }

    /// Applies the current request to the window and logs what it did.
    pub fn apply(&mut self, surface: &mut dyn DisplaySurface, model: &SettingsModel) -> DisplayPacingState {
        let request = (self.request)(model);
        let state = pacing::apply(surface, request.vsync, request.target_frame_rate);
        self.state = Some(state);
        self.applications += 1;
        if let Some(log) = &self.log {
            log(&format!("display pacing: {state}"));
        }
        state
    }

    /// Follows a change of setting: both settings take effect immediately, whoever changed
    /// them, the options window, a menu, a libretro core option or a script (REQ-PERS-084).
    pub fn on_setting_changed(&mut self, surface: &mut dyn DisplaySurface, model: &SettingsModel, id: SettingId) {
        if matches!(id, SettingId::VideoVSync | SettingId::VideoFrameRate) {
            self.apply(surface, model);
        }
    }
}
