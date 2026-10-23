//! The handheld interface (Rust-only): gamepad only, full-screen lists, no mouse and no
//! keyboard, for 640x480, 720x480 and 720x720 screens. The screens are models
//! ([`rom_list`], [`profile_list`], [`overlay`], [`options`]) that answer a press of a button with
//! a command; [`HandheldUi`] owns them, carries the commands out on the host and draws them.

pub mod defaults;
mod draw;
pub mod exit_combo;
pub mod layout;
pub mod options;
pub mod overlay;
pub mod pad;
pub mod profile_list;
pub mod rom_list;

use std::path::{Path, PathBuf};

use zax_settings::catalogue::SettingId;

use self::exit_combo::{ComboEvent, ExitCombo};
use self::layout::Metrics;
use self::options::{HandheldOptionsModel, OptionsCommand};
use self::overlay::{CameraPreset, OverlayCommand, OverlayModel};
use self::pad::{PadButton, PadState};
use self::profile_list::{ProfileListCommand, ProfileListModel};
use self::rom_list::{ProfileMatch, RomEntry, RomListCommand, RomListModel};
use super::key_config::{self, KeyConfigModel, KeyConfigRow};
use super::profile_chooser::{ProfileChoice, ProfileChooserModel};
use super::zax_ui::{UiEffect, UiFrame};
use crate::application::AppHost;
use crate::emulation::commands::{Command, CommandArgs};
use crate::emulation::game_catalog::GameCatalog as _;
use crate::emulation::host::HostEvent;
use crate::input::BindingDeviceKind;

/// Seconds a binding waits for a button before it gives up.
pub const CAPTURE_SECONDS: f32 = 5.0;

/// The screen in front.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    /// The list of games.
    Home,
    /// The profiles of one game.
    Profiles,
    /// The reduced options.
    Options,
    /// The gamepad bindings of player one.
    Controls,
    /// The in-game menu.
    Overlay,
    /// The game itself, with no interface.
    InGame,
}

/// The list of the bindings and the capture in progress.
#[derive(Debug, Clone, Copy, Default)]
struct ControlsState {
    selected: usize,
    /// Seconds left to the binding being captured, when one is.
    capture_left: Option<f32>,
}

/// The handheld interface.
pub struct HandheldUi {
    screen: Screen,
    /// Where the options go back to: the home screen, or the in-game menu.
    return_to: Screen,
    roms_directory: PathBuf,
    roms: RomListModel,
    library: Vec<ProfileChoice>,
    profiles: Option<ProfileListModel>,
    overlay: OverlayModel,
    options: HandheldOptionsModel,
    controls: ControlsState,
    keys: KeyConfigModel,
    combo: ExitCombo,
    metrics: Metrics,
    heading_font: Option<imgui::FontId>,
    effects: Vec<UiEffect>,
}

impl HandheldUi {
    /// The interface for a display `height` pixels high, listing the ROMs of `roms_directory`.
    #[must_use]
    pub fn new(host: &AppHost, roms_directory: PathBuf, height: f32) -> Self {
        let mut ui = Self {
            screen: Screen::Home,
            return_to: Screen::Home,
            roms_directory,
            roms: RomListModel::default(),
            library: Vec::new(),
            profiles: None,
            overlay: OverlayModel::new(),
            options: HandheldOptionsModel::new(),
            controls: ControlsState::default(),
            keys: KeyConfigModel::new(),
            combo: ExitCombo::new(),
            metrics: Metrics::for_height(height),
            heading_font: None,
            effects: Vec::new(),
        };
        ui.refresh_roms(host);
        ui
    }

    /// The face of the titles.
    pub const fn set_heading_font(&mut self, font: Option<imgui::FontId>) {
        self.heading_font = font;
    }

    /// The screen in front.
    #[must_use]
    pub const fn screen(&self) -> Screen {
        self.screen
    }

    /// Whether the interface holds the input: everything but the game.
    #[must_use]
    pub fn is_modal(&self) -> bool {
        self.screen != Screen::InGame
    }

    /// What the interface asked of the application since the last call.
    pub fn take_effects(&mut self) -> Vec<UiEffect> {
        std::mem::take(&mut self.effects)
    }

    /// Reads the ROM folder again and finds the profile of each game.
    pub fn refresh_roms(&mut self, host: &AppHost) {
        self.library = ProfileChooserModel::new(host.games().library())
            .all()
            .to_vec();
        let mut entries = rom_list::scan(&self.roms_directory);
        for entry in &mut entries {
            entry.profile = self.profile_of(host, entry);
        }
        self.roms.replace(entries);
    }

    /// The profile a game plays with: the one remembered for the file, the one that owns the
    /// file, or else one that carries the game's name.
    fn profile_of(&self, host: &AppHost, entry: &RomEntry) -> Option<ProfileMatch> {
        let path = entry.path.to_string_lossy();
        let remembered = host
            .settings_store()
            .and_then(|store| store.games().profile_for(&path).map(str::to_owned))
            .filter(|id| host.games().exists(id));
        let known = remembered.or_else(|| host.games().game_for_rom(&path));
        let choice = known
            .and_then(|id| self.library.iter().find(|choice| choice.id == id))
            .or_else(|| {
                rom_list::matching_profiles(&self.library, &entry.name)
                    .into_iter()
                    .next()
            })?;
        Some(ProfileMatch {
            id: choice.id.clone(),
            title: choice.title.clone(),
            two_dimensional: choice.is_two_dimensional,
        })
    }

    /// Takes one event of the host: a window asked for by a command.
    pub fn handle_host_event(&mut self, host: &AppHost, event: HostEvent) {
        let HostEvent::UiRequested { command, args } = event else {
            return;
        };
        match command {
            Command::ChooseProfile => {
                let entry = args
                    .text
                    .as_deref()
                    .map(|path| RomEntry::new(PathBuf::from(path)));
                if let Some(entry) = entry {
                    self.open_profiles(&entry);
                }
            }
            Command::ShowOptions => self.open_options(host),
            other => self.effects.push(UiEffect::Log(format!(
                "the handheld interface has nothing to do for '{}'.",
                other.name()
            ))),
        }
    }

    /// One frame of input: the exit combination, the menu button and the presses of the screen.
    pub fn update(&mut self, pad: &PadState, seconds: f32, frame: &mut UiFrame<'_>) {
        self.follow_the_game(frame.host);
        self.update_combo(pad, seconds, frame.host);
        self.update_capture(seconds, frame);
        for button in pad.presses() {
            self.press(button, frame);
        }
    }

    /// The screen follows the game: it hides when one runs and comes back when it stops.
    fn follow_the_game(&mut self, host: &AppHost) {
        let playing = host.current_game_id().is_some() || host.is_loading();
        match (playing, self.screen) {
            (true, Screen::Home | Screen::Profiles) => self.screen = Screen::InGame,
            (false, Screen::InGame | Screen::Overlay) => self.show_home(host),
            _ => {}
        }
    }

    fn show_home(&mut self, host: &AppHost) {
        self.screen = Screen::Home;
        self.return_to = Screen::Home;
        self.refresh_roms(host);
    }

    fn update_combo(&mut self, pad: &PadState, seconds: f32, host: &mut AppHost) {
        let both = pad.is_held(PadButton::Select) && pad.is_held(PadButton::Start);
        match self.combo.update(seconds, both) {
            ComboEvent::Quit => host.enqueue(Command::Quit, CommandArgs::default()),
            ComboEvent::Tap if menu_is_select_start(host) => self.toggle_overlay(host),
            ComboEvent::Tap | ComboEvent::Idle => {}
        }
    }

    /// The menu button or the combination: opens the in-game menu, or closes it.
    fn toggle_overlay(&mut self, host: &mut AppHost) {
        match self.screen {
            Screen::InGame => {
                host.enqueue(Command::Pause, CommandArgs::default());
                self.overlay.reopen();
                self.screen = Screen::Overlay;
            }
            Screen::Overlay => self.carry_out_overlay(OverlayCommand::Resume, host),
            _ => {}
        }
    }

    /// A binding that waits too long is given up.
    fn update_capture(&mut self, seconds: f32, frame: &mut UiFrame<'_>) {
        let Some(left) = self.controls.capture_left else {
            return;
        };
        let left = left - seconds;
        let listening = frame.input.rebinding().is_listening();
        self.controls.capture_left = (listening && left > 0.0).then_some(left);
        if listening && left <= 0.0 {
            frame.input.edit(|_, _, session| session.stop());
        }
    }

    fn press(&mut self, button: PadButton, frame: &mut UiFrame<'_>) {
        // The press that ends a capture bound the button: it is not a press of the screen.
        if self.screen == Screen::Controls && self.controls.capture_left.is_some() {
            if button == PadButton::Menu {
                frame.input.edit(|_, _, session| session.stop());
                self.controls.capture_left = None;
            }
            return;
        }
        if button == PadButton::Menu && self.screen == Screen::InGame {
            self.toggle_overlay(frame.host);
            return;
        }
        match self.screen {
            Screen::Home => {
                if let Some(command) = self.roms.handle(button) {
                    self.carry_out_home(command, frame.host);
                }
            }
            Screen::Profiles => {
                let command = self
                    .profiles
                    .as_mut()
                    .and_then(|profiles| profiles.handle(button));
                if let Some(command) = command {
                    self.carry_out_profiles(command, frame.host);
                }
            }
            Screen::Options => {
                let options = &mut self.options;
                let command = frame
                    .host
                    .update_settings(|settings| options.handle(button, settings));
                if let Some(command) = command {
                    self.carry_out_options(command, frame);
                }
            }
            Screen::Controls => self.press_controls(button, frame),
            Screen::Overlay => {
                if let Some(command) = self.overlay.handle(button) {
                    self.carry_out_overlay(command, frame.host);
                }
            }
            Screen::InGame => {}
        }
    }

    fn carry_out_home(&mut self, command: RomListCommand, host: &mut AppHost) {
        match command {
            RomListCommand::Play(path) => {
                let profile = self
                    .roms
                    .entries()
                    .iter()
                    .find(|entry| entry.path == path)
                    .and_then(|entry| entry.profile.as_ref())
                    .map(|profile| profile.id.clone())
                    .or_else(|| host.games().two_dimensional_mode_id());
                let rom = path.to_string_lossy();
                match profile {
                    Some(id) => host.launch_rom_with_profile(&rom, &id),
                    None => host.enqueue(Command::OpenRom, CommandArgs::of_text(&rom)),
                }
            }
            RomListCommand::ChooseProfile(path) => {
                if let Some(entry) = self
                    .roms
                    .entries()
                    .iter()
                    .find(|entry| entry.path == path)
                    .cloned()
                {
                    self.open_profiles(&entry);
                }
            }
            RomListCommand::Options => self.open_options(host),
        }
    }

    fn open_profiles(&mut self, entry: &RomEntry) {
        self.profiles = Some(ProfileListModel::for_rom(entry, &self.library));
        self.screen = Screen::Profiles;
    }

    fn carry_out_profiles(&mut self, command: ProfileListCommand, host: &mut AppHost) {
        self.screen = Screen::Home;
        if let ProfileListCommand::Choose { rom, id } = command {
            host.launch_rom_with_profile(&rom.to_string_lossy(), &id);
            self.refresh_roms(host);
            self.roms.select_path(&rom);
        }
        self.profiles = None;
    }

    fn open_options(&mut self, host: &AppHost) {
        if !matches!(self.screen, Screen::Options | Screen::Controls) {
            let playing = host.current_game_id().is_some();
            self.return_to = if playing {
                Screen::Overlay
            } else {
                Screen::Home
            };
        }
        self.screen = Screen::Options;
    }

    fn carry_out_options(&mut self, command: OptionsCommand, frame: &mut UiFrame<'_>) {
        match command {
            OptionsCommand::Close => self.screen = self.return_to,
            OptionsCommand::OpenControls => {
                self.controls = ControlsState::default();
                self.screen = Screen::Controls;
            }
            OptionsCommand::StepCamera(step) => {
                let preset = self.overlay.step_camera(step);
                apply_camera(preset, frame.host);
            }
        }
    }

    fn carry_out_overlay(&mut self, command: OverlayCommand, host: &mut AppHost) {
        let close = |ui: &mut Self, host: &mut AppHost| {
            host.enqueue(Command::Resume, CommandArgs::default());
            ui.screen = Screen::InGame;
        };
        match command {
            OverlayCommand::Resume => close(self, host),
            OverlayCommand::SaveState(slot) => {
                host.enqueue(Command::SaveSlot, CommandArgs::of_number(slot));
                close(self, host);
            }
            OverlayCommand::LoadState(slot) => {
                host.enqueue(Command::LoadSlot, CommandArgs::of_number(slot));
                close(self, host);
            }
            OverlayCommand::Rewind => {
                host.enqueue(Command::Rollback, CommandArgs::default());
                close(self, host);
            }
            OverlayCommand::ToggleFastForward => {
                close(self, host);
                host.enqueue(Command::FastForward, CommandArgs::default());
            }
            OverlayCommand::SetCamera(preset) => apply_camera(preset, host),
            OverlayCommand::Options => self.open_options(host),
            OverlayCommand::QuitToList => {
                host.enqueue(Command::Stop, CommandArgs::default());
                self.show_home(host);
            }
            OverlayCommand::ExitToLauncher => host.enqueue(Command::Quit, CommandArgs::default()),
        }
    }

    /// The rows of the list of bindings: the NES buttons of player one.
    fn control_rows(&mut self) -> Vec<KeyConfigRow> {
        self.keys
            .sections()
            .first()
            .map_or_else(Vec::new, |section| section.rows.clone())
    }

    fn press_controls(&mut self, button: PadButton, frame: &mut UiFrame<'_>) {
        let rows = self.control_rows();
        let count = rows.len();
        match button {
            PadButton::Up if count > 0 => {
                self.controls.selected = (self.controls.selected + count - 1) % count;
            }
            PadButton::Down if count > 0 => {
                self.controls.selected = (self.controls.selected + 1) % count;
            }
            PadButton::A => {
                let Some(row) = rows.get(self.controls.selected) else {
                    return;
                };
                let hub = frame.hub;
                let listening = frame.input.edit(|bindings, pads, session| {
                    key_config::begin(
                        session,
                        bindings,
                        pads,
                        hub,
                        row,
                        BindingDeviceKind::Gamepad,
                    )
                });
                self.controls.capture_left = listening.then_some(CAPTURE_SECONDS);
            }
            PadButton::B => self.screen = Screen::Options,
            _ => {}
        }
    }

    /// The list of games.
    #[must_use]
    pub const fn roms(&self) -> &RomListModel {
        &self.roms
    }

    /// The ROM folder shown.
    #[must_use]
    pub fn roms_directory(&self) -> &Path {
        &self.roms_directory
    }

    /// Puts the interface on `screen` with what it needs to be photographed (`--ui-shots`).
    pub fn arrange_for_capture(&mut self, screen: Screen) {
        if self.roms.entries().is_empty() {
            self.roms = draw::sample_roms();
        }
        match screen {
            Screen::Profiles => {
                if let Some(entry) = self.roms.selected_entry().cloned() {
                    self.open_profiles(&entry);
                }
            }
            Screen::Options => self.open_options_for_capture(),
            Screen::Controls => {
                self.controls = ControlsState::default();
                self.screen = Screen::Controls;
            }
            Screen::Overlay => {
                self.overlay.reopen();
                self.screen = Screen::Overlay;
            }
            Screen::Home | Screen::InGame => self.screen = screen,
        }
    }

    fn open_options_for_capture(&mut self) {
        self.return_to = Screen::Home;
        self.screen = Screen::Options;
    }

    /// The exit combination, for the bar the screens draw while it is held.
    #[must_use]
    pub const fn combo(&self) -> &ExitCombo {
        &self.combo
    }
}

/// Whether the setting "Menu = Select + Start" makes a tap of the combination open the menu.
fn menu_is_select_start(host: &AppHost) -> bool {
    host.settings()
        .get(SettingId::InputEmulatedMenuButton)
        .as_bool()
}

/// Puts the camera on `preset`.
fn apply_camera(preset: CameraPreset, host: &mut AppHost) {
    match preset {
        CameraPreset::Orbit => host.enqueue(Command::SetCameraMode, CommandArgs::of_number(0)),
        CameraPreset::FirstPerson => {
            host.enqueue(Command::SetCameraMode, CommandArgs::of_number(1));
        }
        CameraPreset::ResetView => host.enqueue(Command::ResetView, CommandArgs::default()),
    }
}
