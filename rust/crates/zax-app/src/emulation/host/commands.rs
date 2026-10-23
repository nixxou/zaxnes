//! The command model over the host: whether a command can run, its toggle state, and what
//! running it does (SPEC-10 REQ-UI-205).

use zax_nes::ControllerPortType;
use zax_render::camera::CameraMode;

use super::{EmulationHost, HostEvent};
use crate::emulation::commands::{Command, CommandArgs};
use crate::emulation::emulation_state::EmulationState;
use crate::emulation::game_catalog::GameCatalog;
use crate::emulation::notification_center::{NotificationSeverity, SHORT_SECONDS};
use crate::emulation::speed_setting;
use crate::persistence::app_settings::{AppSettings, AppSettingsMut, TvRegionSetting};
use crate::persistence::save_slots::{is_valid_slot_id, numbered_slot_id, QUICK_SLOT_ID};

impl<C: GameCatalog> EmulationHost<C> {
    /// Whether `command` can run now with `args`.
    #[must_use]
    pub fn can_execute(&self, command: Command, args: &CommandArgs) -> bool {
        let loading = self.loading;
        let stopped = self.state == EmulationState::Stopped;
        match command {
            Command::OpenRom
            | Command::ChooseProfile
            | Command::ImportProfile
            | Command::ImportThreeDSen
            | Command::SetCameraMode => !loading,
            Command::LaunchGame => {
                !loading && args.text.as_deref().is_some_and(|text| !text.is_empty())
            }
            Command::DeleteGame => {
                let game = args
                    .text
                    .as_deref()
                    .or_else(|| self.current_game_id())
                    .unwrap_or_default();
                !loading && self.games.can_delete(game)
            }
            Command::CreateGameArt
            | Command::ImportCover
            | Command::Reset
            | Command::ToggleTwoDimensional => !loading && self.current_game.is_some(),
            Command::SaveSlot => self.can_save(args.number),
            Command::LoadSlot => {
                let slot = numbered_slot_id(args.number);
                !loading && is_valid_slot_id(&slot) && self.slots.is_occupied(&slot)
            }
            Command::QuickSave => self.current_game.is_some() && !stopped && !loading,
            Command::QuickLoad => !loading && self.slots.is_occupied(QUICK_SLOT_ID),
            Command::DeleteSlot => {
                let slot = args
                    .text
                    .clone()
                    .unwrap_or_else(|| numbered_slot_id(args.number));
                !loading && self.slots.is_occupied(&slot)
            }
            Command::PauseResume | Command::Stop | Command::Rollback => !stopped && !loading,
            Command::Pause => self.state == EmulationState::Running && !loading,
            Command::Resume => self.state == EmulationState::Paused && !loading,
            Command::FastForward => self.state == EmulationState::Running,
            Command::SpeedUp => self.settings.game_speed() < speed_setting::MAXIMUM,
            Command::SpeedDown => self.settings.game_speed() > speed_setting::MINIMUM,
            Command::SetPortType => self.session.is_some() && matches!(args.number, 0 | 1),
            Command::CancelModal => self.modal_window_open(),
            Command::OpenRecentRom
            | Command::Quit
            | Command::SetSpeed
            | Command::SetRegion
            | Command::ToggleAutoRegion
            | Command::ResetView
            | Command::SetSkybox
            | Command::ToggleLocalCamera
            | Command::ToggleShadows
            | Command::ToggleFullscreen
            | Command::ToggleFpsCounter
            | Command::SetVolume
            | Command::VolumeUp
            | Command::VolumeDown
            | Command::ToggleMute
            | Command::ToggleMenuMusic
            | Command::ShowOptions
            | Command::ShowKeyConfig
            | Command::ShowDataFolder
            | Command::ShowAbout => true,
        }
    }

    /// Whether the toggle `command` is on.
    #[must_use]
    pub fn is_checked(&self, command: Command) -> bool {
        match command {
            Command::PauseResume => self.state == EmulationState::Paused,
            Command::FastForward => self.fast_forward.is_active(),
            Command::ToggleAutoRegion => self.settings.auto_tv_mode(),
            Command::ToggleTwoDimensional => self.showing_two_dimensional(),
            Command::ToggleLocalCamera => self.settings.local_camera(),
            Command::ToggleShadows => self.settings.shadow(),
            Command::ToggleFullscreen => self.settings.fullscreen(),
            Command::ToggleFpsCounter => self.settings.show_fps(),
            Command::ToggleMute => self.settings.mute(),
            Command::ToggleMenuMusic => self.settings.menu_music(),
            _ => false,
        }
    }

    /// Runs `command` at once if it can run. Returns whether it did.
    pub fn invoke(&mut self, command: Command, args: &CommandArgs) -> bool {
        if !self.can_execute(command, args) {
            return false;
        }
        self.execute(command, args);
        true
    }

    /// Runs `command` without asking whether it can.
    pub(super) fn execute(&mut self, command: Command, args: &CommandArgs) {
        match command {
            Command::OpenRom
            | Command::LaunchGame
            | Command::OpenRecentRom
            | Command::ChooseProfile
            | Command::ImportProfile
            | Command::ImportThreeDSen
            | Command::DeleteGame
            | Command::CreateGameArt
            | Command::ImportCover
            | Command::Quit => self.execute_file(command, args),
            Command::SaveSlot
            | Command::LoadSlot
            | Command::QuickSave
            | Command::QuickLoad
            | Command::DeleteSlot => self.execute_state(command, args),
            Command::PauseResume
            | Command::Pause
            | Command::Resume
            | Command::Reset
            | Command::Stop
            | Command::FastForward
            | Command::Rollback
            | Command::SetSpeed
            | Command::SpeedUp
            | Command::SpeedDown
            | Command::SetRegion
            | Command::ToggleAutoRegion
            | Command::SetPortType => self.execute_emulation(command, args),
            Command::ToggleTwoDimensional
            | Command::ResetView
            | Command::SetSkybox
            | Command::SetCameraMode
            | Command::ToggleLocalCamera
            | Command::ToggleShadows
            | Command::ToggleFullscreen
            | Command::ToggleFpsCounter => self.execute_video(command, args),
            Command::SetVolume
            | Command::VolumeUp
            | Command::VolumeDown
            | Command::ToggleMute
            | Command::ToggleMenuMusic => self.execute_audio(command, args),
            Command::ShowOptions
            | Command::ShowKeyConfig
            | Command::ShowDataFolder
            | Command::ShowAbout
            | Command::CancelModal => self.request_ui(command, args),
        }
    }

    fn execute_file(&mut self, command: Command, args: &CommandArgs) {
        match command {
            Command::OpenRom => self.open_rom_or_ask_for_one(args),
            Command::LaunchGame => self.launch_game(args.text.as_deref().unwrap_or_default()),
            Command::DeleteGame => {
                let game = args
                    .text
                    .clone()
                    .or_else(|| self.current_game_id().map(str::to_owned))
                    .unwrap_or_default();
                self.request_delete_game(&game);
            }
            Command::CreateGameArt => {
                self.create_game_art();
            }
            Command::ImportCover => match args.text.as_deref().filter(|text| !text.is_empty()) {
                Some(path) => {
                    self.import_cover(path);
                }
                None => self.request_ui(command, args),
            },
            Command::Quit => self.quit_requested = true,
            // The folder pickers and the imports belong to the interface, which owns the
            // library (AMD-030 §2).
            _ => self.request_ui(command, args),
        }
    }

    fn execute_state(&mut self, command: Command, args: &CommandArgs) {
        match command {
            Command::SaveSlot => {
                self.save_to_slot(&numbered_slot_id(args.number), true);
            }
            Command::LoadSlot => self.load_slot(&numbered_slot_id(args.number)),
            Command::QuickSave => self.quick_save(),
            Command::QuickLoad => self.quick_load(),
            Command::DeleteSlot => {
                let slot = args
                    .text
                    .clone()
                    .unwrap_or_else(|| numbered_slot_id(args.number));
                self.request_delete_slot(&slot);
            }
            _ => {}
        }
    }

    fn execute_emulation(&mut self, command: Command, args: &CommandArgs) {
        match command {
            Command::PauseResume => self.toggle_pause(),
            Command::Pause => self.pause(),
            Command::Resume => self.resume(),
            Command::Reset => self.reset_game(),
            Command::Stop => self.stop_current_game(true, true),
            Command::FastForward => self.request_fast_forward(),
            Command::Rollback => self.perform_rollback(),
            Command::SetSpeed => self.update_settings(|model| model.set_game_speed(args.number)),
            Command::SpeedUp => self.update_settings(|model| {
                let speed = speed_setting::clamp(model.game_speed() + 1);
                model.set_game_speed(speed);
            }),
            Command::SpeedDown => self.update_settings(|model| {
                let speed = speed_setting::clamp(model.game_speed() - 1);
                model.set_game_speed(speed);
            }),
            Command::SetRegion => self.update_settings(|model| {
                model.set_tv_mode(TvRegionSetting::from_raw(args.number));
            }),
            Command::ToggleAutoRegion => self.update_settings(|model| {
                let value = !model.auto_tv_mode();
                model.set_auto_tv_mode(value);
            }),
            Command::SetPortType => {
                if let (Some(session), Ok(port)) =
                    (self.session.as_mut(), usize::try_from(args.number))
                {
                    let kind = if args.number2 == 1 {
                        ControllerPortType::Zapper
                    } else {
                        ControllerPortType::Controller
                    };
                    session.set_port_type(port, kind);
                }
            }
            _ => {}
        }
    }

    fn execute_video(&mut self, command: Command, args: &CommandArgs) {
        match command {
            Command::ToggleTwoDimensional => self.two_dimensional_view = !self.two_dimensional_view,
            Command::ResetView => self.scene.camera.reset_view(),
            Command::SetCameraMode => {
                // SPEC-06 REQ-SCN-006: the two camera modes, with their own setpoints. The
                // menu entry "Mode caméra" of REQ-UI-200 switches between them; REQ-SCN-009's
                // local camera is the third item of the same submenu.
                let mode = if args.number == 1 {
                    CameraMode::Fpv
                } else {
                    CameraMode::Tpv
                };
                self.scene.camera.set_mode(mode);
            }
            Command::ToggleLocalCamera => self.update_settings(|model| {
                let value = !model.local_camera();
                model.set_local_camera(value);
            }),
            Command::SetSkybox => self.update_settings(|model| model.set_skybox(args.number)),
            Command::ToggleShadows => self.update_settings(|model| {
                let value = !model.shadow();
                model.set_shadow(value);
            }),
            Command::ToggleFullscreen => {
                self.update_settings(|model| {
                    let value = !model.fullscreen();
                    model.set_fullscreen(value);
                });
                self.request_ui(command, &CommandArgs::default());
            }
            Command::ToggleFpsCounter => self.update_settings(|model| {
                let value = !model.show_fps();
                model.set_show_fps(value);
            }),
            _ => {}
        }
    }

    fn execute_audio(&mut self, command: Command, args: &CommandArgs) {
        match command {
            Command::SetVolume => self.update_settings(|model| {
                #[allow(clippy::cast_precision_loss)] // a percentage
                model.set_volume(args.number.clamp(0, 100) as f32 / 100.0);
            }),
            Command::VolumeUp => self.update_settings(|model| {
                let volume = (model.volume() + 0.05).min(1.0);
                model.set_volume(volume);
            }),
            Command::VolumeDown => self.update_settings(|model| {
                let volume = (model.volume() - 0.05).max(0.0);
                model.set_volume(volume);
            }),
            Command::ToggleMute => self.update_settings(|model| {
                let value = !model.mute();
                model.set_mute(value);
            }),
            Command::ToggleMenuMusic => self.update_settings(|model| {
                let value = !model.menu_music();
                model.set_menu_music(value);
            }),
            _ => {}
        }
    }

    pub(super) fn request_ui(&mut self, command: Command, args: &CommandArgs) {
        self.events.push(HostEvent::UiRequested {
            command,
            args: args.clone(),
        });
    }

    fn open_rom_or_ask_for_one(&mut self, args: &CommandArgs) {
        match args.text.as_deref().filter(|text| !text.is_empty()) {
            Some(path) => self.open_rom(path),
            None => self.request_ui(Command::OpenRom, args),
        }
    }

    fn can_save(&self, slot_number: i32) -> bool {
        !self.loading
            && self.state != EmulationState::Stopped
            && self.current_game.is_some()
            && is_valid_slot_id(&numbered_slot_id(slot_number))
    }

    fn toggle_pause(&mut self) {
        match self.state {
            EmulationState::Running => self.pause(),
            EmulationState::Paused => self.resume(),
            // REQ-LOOP-004: Pause outside Running and Play outside Paused do nothing.
            EmulationState::Stopped => {}
        }
    }

    pub(super) fn pause(&mut self) {
        if self.state != EmulationState::Running || self.loading {
            return;
        }
        self.set_state(EmulationState::Paused);
    }

    pub(super) fn resume(&mut self) {
        if self.state != EmulationState::Paused || self.loading {
            return;
        }
        self.set_state(EmulationState::Running);
        // REQ-LOOP-013: no catch-up of the time spent paused.
        self.pacer.reset();
    }

    fn reset_game(&mut self) {
        if self.loading {
            return;
        }
        if let Some(id) = self.current_game_id().map(str::to_owned) {
            self.launch_game(&id);
        }
    }

    fn request_fast_forward(&mut self) {
        if self.state != EmulationState::Running {
            return;
        }
        self.fast_forward.request();
        self.apply_speed();
        // REQ-LOOP-016: the notification is refreshed for as long as the key is held.
        self.notifications
            .show("Fast Forward", SHORT_SECONDS, NotificationSeverity::Normal);
    }

    /// Rebuilds the meshes with another algorithm; the state and the core do not change
    /// (REQ-LOOP-024).
    pub fn change_mesh_algorithm(&mut self, algorithm: zax_engine3d::settings::RenderAlgorithm) {
        self.regenerate_meshes(algorithm);
    }

    /// The window lost the focus or was minimised: a running game pauses and will resume.
    pub fn suspend(&mut self) {
        if self.state == EmulationState::Running {
            self.was_running_before_suspend = true;
            self.pause();
        }
    }

    /// The window is back: the game that was running resumes.
    pub fn resume_from_suspend(&mut self) {
        if self.was_running_before_suspend {
            self.was_running_before_suspend = false;
            self.resume();
        }
    }
}
