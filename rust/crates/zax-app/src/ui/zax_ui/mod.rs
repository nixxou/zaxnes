//! The classic interface of AMD-010: the menu bar, the options and key configuration windows,
//! the dialogs, the welcome screen and the on-screen notifications, drawn with Dear `ImGui`. The
//! state is plain data in the models; this type owns it and draws it, one frame at a time.
//!
//! The window never reaches into the application: what it needs for a frame is lent in a
//! [`UiFrame`], and what it asks of the outside (a line for the log, the data folder opened in
//! the file manager) is returned as [`UiEffect`]s for the loop to carry out.

mod dialogs;
mod key_config;
mod menu;
mod options;
mod overlay;

use std::path::{Path, PathBuf};

use zax_platform::input::{GamepadHub, Key};
use zax_profiles::catalogue::ProfileImportOutcome;

use super::commands::UiCommand;
use super::confirm_dialog::ConfirmationView;
use super::file_browser::{self, FileBrowserModel};
use super::file_picker::{FilePicker, PickPoll, PickRequest};
use super::key_config::KeyConfigModel;
use super::menu_bar::MenuBarModel;
use super::menu_shortcuts::{self, ShortcutCommand, ShortcutModifiers};
use super::notifications;
use super::options::{OptionsModel, OptionsTab};
use super::profile_chooser::ProfileChooserModel;
use super::state::UiState;
use super::strings;
use super::text;
use crate::application::AppHost;
use crate::audio::UiSound;
use crate::emulation::commands::{Command, CommandArgs};
use crate::emulation::host::HostEvent;
use crate::emulation::rom_file::INVALID_ROM_MESSAGE;
use crate::input::DesktopInputSource;
use crate::persistence::app_settings::{AppSettings as _, AppSettingsMut as _};

/// What a frame of the interface borrows from the application.
pub struct UiFrame<'a> {
    /// The host: settings, commands, notifications, the library.
    pub host: &'a mut AppHost,
    /// The input layer, whose bindings the key configuration window edits.
    pub input: &'a mut DesktopInputSource,
    /// The connected gamepads.
    pub hub: &'a dyn GamepadHub,
    /// Real seconds since the previous frame.
    pub real_seconds: f32,
    /// The frame rate the counter shows.
    pub frames_per_second: f32,
}

/// What the interface asks the application to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiEffect {
    /// A line for the console log.
    Log(String),
    /// Open this folder in the desktop's file manager (REQ-PERS-072).
    RevealFolder(PathBuf),
    /// A native file dialog is about to open: take the window out of full screen.
    LeaveFullscreen,
    /// The native file dialog closed: put the window back as `ui.fullscreen` says.
    RestoreFullscreen,
}

/// What the file browser is open for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserPurpose {
    /// A ROM to open.
    Rom,
    /// A profile to import.
    Profile,
    /// A cover picture to import.
    Cover,
    /// A 3dSen installation or bundle to import.
    ThreeDSen,
}

/// The interface.
pub struct ZaxUi {
    /// Which windows are open.
    pub state: UiState,
    /// The menus.
    pub menu_bar: MenuBarModel,
    /// The options window.
    pub options: OptionsModel,
    /// The key configuration window.
    pub keys: KeyConfigModel,
    /// The profile chooser.
    pub chooser: ProfileChooserModel,
    /// The file browser.
    pub browser: FileBrowserModel,
    /// Height of the menu bar, which the game view sits under (REQ-UI-200).
    pub chrome_height: f32,
    /// The capture aid of `--ui-shots`: the menu to open on the next frame.
    pub force_open_menu: Option<String>,
    /// The capture aid of `--ui-shots`: the options tab to select on the next frame.
    pub force_options_tab: Option<OptionsTab>,
    heading_font: Option<imgui::FontId>,
    purpose: BrowserPurpose,
    picker: Option<Box<dyn FilePicker>>,
    fps_shown: f32,
    fps_countdown: f32,
    held_keys: Vec<Key>,
    effects: Vec<UiEffect>,
}

impl ZaxUi {
    /// Builds the interface over the library of `host`.
    #[must_use]
    pub fn new(host: &AppHost) -> Self {
        Self {
            state: UiState::default(),
            menu_bar: MenuBarModel::new(),
            options: OptionsModel::new(),
            keys: KeyConfigModel::new(),
            chooser: ProfileChooserModel::new(host.games().library()),
            browser: FileBrowserModel::new(),
            chrome_height: 0.0,
            force_open_menu: None,
            force_options_tab: None,
            heading_font: None,
            purpose: BrowserPurpose::Rom,
            picker: None,
            fps_shown: 0.0,
            fps_countdown: 0.0,
            held_keys: Vec::new(),
            effects: Vec::new(),
        }
    }

    /// The face of the headings (the product name on the welcome screen and in the about
    /// window); the body face when none is set.
    pub const fn set_heading_font(&mut self, font: Option<imgui::FontId>) {
        self.heading_font = font;
    }

    /// Chooses files with `picker` (the platform's dialog) instead of the `ImGui` browser, which
    /// stays the fallback when the picker cannot start or fails.
    pub fn set_file_picker(&mut self, picker: Box<dyn FilePicker>) {
        self.picker = Some(picker);
    }

    /// Takes the answer of the native file dialog, when one is open. A dialog that could not be
    /// shown opens the `ImGui` browser instead.
    pub fn poll_file_picker(&mut self, host: &mut AppHost) {
        if !self.state.native_dialog {
            return;
        }
        let Some(picker) = self.picker.as_mut() else {
            self.state.native_dialog = false;
            return;
        };
        let poll = picker.poll();
        if poll == PickPoll::Pending {
            return;
        }
        self.state.native_dialog = false;
        if picker.needs_windowed() {
            self.effects.push(UiEffect::RestoreFullscreen);
        }
        match poll {
            PickPoll::Chosen(path) => self.on_file_chosen(host, &path),
            PickPoll::Failed => {
                self.log(
                    "the native file dialog is unavailable; using the built-in browser.".into(),
                );
                self.open_imgui_browser(host, self.purpose);
            }
            PickPoll::Pending | PickPoll::Cancelled => {}
        }
    }

    /// What the interface asked of the application since the last call.
    pub fn take_effects(&mut self) -> Vec<UiEffect> {
        std::mem::take(&mut self.effects)
    }

    /// The question the host is asking, if any, as the box shows it (REQ-UI-110).
    #[must_use]
    pub fn confirmation(host: &AppHost) -> Option<ConfirmationView<'_>> {
        host.pending_confirmation().map(ConfirmationView::new)
    }

    /// Opens the profile chooser as the `--ui-shots` capture wants it.
    pub fn open_profile_chooser_for_capture(&mut self) {
        self.open_profile_chooser(None);
    }

    /// Draws one frame of the interface over the game view.
    pub fn draw(&mut self, ui: &imgui::Ui, frame: &mut UiFrame<'_>) {
        self.poll_file_picker(frame.host);
        // The window state is the setting's (REQ-PERS-067); a change made by a shortcut, by the
        // options window or by the menu reaches the interface here.
        self.state.fullscreen = frame.host.settings().fullscreen();
        self.state.has_current_game = frame.host.current_game_id().is_some();
        self.sync_confirmation(frame.host);

        self.chrome_height = if self.state.menu_bar_visible() {
            self.draw_menu_bar(ui, frame.host)
        } else {
            0.0
        };
        if self.state.welcome_visible() {
            self.draw_welcome(ui, frame.host);
        }
        if self.state.options {
            self.draw_options(ui, frame.host);
        }
        if self.state.key_config {
            self.draw_key_config(ui, frame);
        }
        if self.state.about {
            self.draw_about(ui);
        }
        if self.state.file_browser {
            self.draw_file_browser(ui, frame.host);
        }
        if self.state.profile_chooser {
            self.draw_profile_chooser(ui, frame.host);
        }
        // REQ-UI-110: the confirmation box is drawn last, so it sits over everything else.
        if self.state.dialog {
            self.draw_confirm_dialog(ui, frame.host);
        }
        // REQ-UI-206: the OSD is the one layer full screen keeps.
        Self::draw_notification(ui, frame.host);
        self.draw_fps_counter(ui, frame);
    }

    /// Takes one event of the host: a window or a dialogue asked for by a command, or the new
    /// language.
    pub fn handle_host_event(&mut self, host: &mut AppHost, event: HostEvent) {
        match event {
            HostEvent::UiRequested { command, args } => {
                self.handle_ui_request(host, command, &args);
            }
            HostEvent::LanguageChanged(code) => self.language_changed(&code),
            HostEvent::SettingChanged(_) => {}
        }
        self.sync_confirmation(host);
    }

    /// The language of the interface changed: the tables follow at once (AMD-035 §4).
    pub fn language_changed(&mut self, code: &str) {
        strings::set_language(strings::UiLanguage::parse(code));
    }

    /// A command asked the interface for a window or a dialogue.
    pub fn handle_ui_request(&mut self, host: &mut AppHost, command: Command, args: &CommandArgs) {
        match command {
            Command::OpenRom => self.open_browser(host, BrowserPurpose::Rom),
            Command::OpenRecentRom => Self::open_recent(host, args.number),
            Command::ChooseProfile => {
                let rom = args.text.clone().or_else(|| Self::current_rom_path(host));
                self.open_profile_chooser(rom.as_deref());
            }
            Command::ImportProfile => self.open_browser(host, BrowserPurpose::Profile),
            Command::ImportThreeDSen => {
                match args.text.as_deref().filter(|path| !path.is_empty()) {
                    // AMD-030 §2: with a path already given (a drag, a script), import straight
                    // away; without one, ask for the folder.
                    Some(installation) => self.import_three_d_sen(host, installation),
                    None => self.open_browser(host, BrowserPurpose::ThreeDSen),
                }
            }
            Command::ImportCover => self.open_browser(host, BrowserPurpose::Cover),
            Command::CancelModal => self.cancel_topmost_modal(),
            Command::ShowOptions => self.state.options = true,
            Command::ShowKeyConfig => self.state.key_config = true,
            Command::ShowAbout => self.state.about = true,
            Command::ShowDataFolder => self.show_data_folder(host),
            // The host already flipped the setting; the window itself is the caller's job.
            Command::ToggleFullscreen => self.state.fullscreen = host.settings().fullscreen(),
            other => self.log(format!(
                "the interface has nothing to do for '{}'.",
                other.name()
            )),
        }
    }

    /// Runs a command the interface owns (REQ-UI-205).
    pub fn run_ui_command(&mut self, command: UiCommand) {
        match command {
            UiCommand::ToggleDebugOverlay => self.state.debug_overlay = !self.state.debug_overlay,
            UiCommand::CloseWindows => self.state.close_all(),
        }
    }

    /// Runs the command a binding named, when it is one the interface owns. Returns whether it
    /// was.
    pub fn run_named_ui_command(&mut self, name: &str) -> bool {
        UiCommand::from_name(name)
            .map(|command| self.run_ui_command(command))
            .is_some()
    }

    /// The menu shortcuts of REQ-UI-204: the keys pressed this frame, with the modifiers held.
    pub fn handle_shortcuts(
        &mut self,
        host: &mut AppHost,
        pressed: &[Key],
        modifiers: ShortcutModifiers,
    ) {
        for (command, args) in menu_shortcuts::resolve(pressed, modifiers) {
            match command {
                ShortcutCommand::Host(command) => host.enqueue(command, args),
                ShortcutCommand::Ui(command) => self.run_ui_command(command),
            }
        }
    }

    /// The rising edge of every watched key since the previous call, from the keys held now.
    pub fn rising_keys(&mut self, is_down: impl Fn(Key) -> bool) -> Vec<Key> {
        let mut pressed = Vec::new();
        for key in menu_shortcuts::watched_keys() {
            if is_down(key) {
                if !self.held_keys.contains(&key) {
                    self.held_keys.push(key);
                    pressed.push(key);
                }
            } else {
                self.held_keys.retain(|held| *held != key);
            }
        }
        pressed
    }

    /// Forgets the keys held, when the keyboard belongs to something else for a while.
    pub fn release_held_keys(&mut self) {
        self.held_keys.clear();
    }

    /// REQ-UI-067: a binding capture in progress stops.
    pub fn close_capture(input: &mut DesktopInputSource) {
        input.edit(|_, _, session| session.close());
    }

    /// Keeps the dialogue flag in step with the question the host is asking.
    pub fn sync_confirmation(&mut self, host: &AppHost) {
        self.state.dialog = host.pending_confirmation().is_some();
    }

    /// Adds a line to the console log, through the effects.
    pub fn log(&mut self, message: String) {
        self.effects.push(UiEffect::Log(message));
    }

    fn current_rom_path(host: &AppHost) -> Option<String> {
        use crate::emulation::game_catalog::GameCatalog as _;
        host.current_game_id()
            .and_then(|id| host.games().rom_path(id))
    }

    fn recent_roms(host: &AppHost) -> Vec<String> {
        host.settings_store()
            .map_or_else(Vec::new, |store| store.games().recent_roms().to_vec())
    }

    fn open_browser(&mut self, host: &AppHost, purpose: BrowserPurpose) {
        self.purpose = purpose;
        if self.start_native_dialog(host, purpose) {
            return;
        }
        self.open_imgui_browser(host, purpose);
    }

    fn browser_title(purpose: BrowserPurpose) -> String {
        match purpose {
            BrowserPurpose::Rom => text::open_rom_title(),
            BrowserPurpose::Cover => text::import_cover_title(),
            BrowserPurpose::ThreeDSen => text::import_three_d_sen_title(),
            BrowserPurpose::Profile => text::import_profile_title(),
        }
    }

    fn browser_filters(purpose: BrowserPurpose) -> Vec<file_browser::FileFilter> {
        match purpose {
            BrowserPurpose::Rom => file_browser::rom_filters(),
            BrowserPurpose::Cover => file_browser::cover_filters(),
            BrowserPurpose::ThreeDSen => file_browser::three_d_sen_filters(),
            BrowserPurpose::Profile => file_browser::profile_filters(),
        }
    }

    /// Asks the platform's dialog, when there is a picker and it can work. Returns whether the
    /// dialog is open.
    fn start_native_dialog(&mut self, host: &AppHost, purpose: BrowserPurpose) -> bool {
        let Some(picker) = self.picker.as_mut() else {
            return false;
        };
        let request = PickRequest {
            title: Self::browser_title(purpose),
            filters: Self::browser_filters(purpose),
            start_directory: Some(host.settings().last_browse_directory()),
            // AMD-030 §2: the 3dSen import is the one dialog that answers with a folder.
            folder: purpose == BrowserPurpose::ThreeDSen,
        };
        if !picker.start(request) {
            return false;
        }
        if picker.needs_windowed() {
            self.effects.push(UiEffect::LeaveFullscreen);
        }
        self.state.native_dialog = true;
        true
    }

    fn open_imgui_browser(&mut self, host: &AppHost, purpose: BrowserPurpose) {
        let three_d_sen = purpose == BrowserPurpose::ThreeDSen;
        let hint = text::import_three_d_sen_hint();
        self.browser.open(
            &Self::browser_title(purpose),
            Self::browser_filters(purpose),
            Some(&host.settings().last_browse_directory()),
            // AMD-030 §2: the 3dSen import is the one dialog that answers with a folder.
            three_d_sen,
            three_d_sen.then_some(hint.as_str()),
        );
        self.state.file_browser = true;
    }

    fn open_profile_chooser(&mut self, rom_path: Option<&str>) {
        self.chooser.reset(rom_path);
        self.state.profile_chooser = true;
    }

    fn open_recent(host: &mut AppHost, index: i32) {
        let recent = Self::recent_roms(host);
        let Some(path) = usize::try_from(index)
            .ok()
            .and_then(|index| recent.get(index))
        else {
            return;
        };
        if !Path::new(path).is_file() {
            // The file has gone: it leaves the list and the user is told, with the text SPEC-10
            // REQ-UI-083 fixes for an unusable ROM.
            if let Some(store) = host.settings_store_mut() {
                store.games_mut().forget_rom(path);
            }
            host.notifications.show_error(INVALID_ROM_MESSAGE);
            return;
        }
        host.enqueue(Command::OpenRom, CommandArgs::of_text(path));
    }

    /// The file browser answered with `path`.
    fn on_file_chosen(&mut self, host: &mut AppHost, path: &str) {
        // REQ-PERS-066: the folder the dialog closed on is the next one it opens on.
        if let Some(directory) = Path::new(path)
            .parent()
            .map(|directory| directory.to_string_lossy().into_owned())
            .filter(|directory| !directory.is_empty())
        {
            host.update_settings(|model| model.set_last_browse_directory(&directory));
        }
        match self.purpose {
            BrowserPurpose::Rom => host.enqueue(Command::OpenRom, CommandArgs::of_text(path)),
            BrowserPurpose::Cover => host.enqueue(Command::ImportCover, CommandArgs::of_text(path)),
            BrowserPurpose::ThreeDSen => self.import_three_d_sen(host, path),
            BrowserPurpose::Profile => self.import_profile(host, path),
        }
    }

    fn import_three_d_sen(&mut self, host: &mut AppHost, path: &str) {
        let report = host.games_mut().import_from_three_d_sen(Path::new(path));
        let Some(summary) = report.summary else {
            // AMD-030 §3, §5: nothing was found or nothing could be read, and the message says
            // where it looked.
            host.notifications.show_error(&report.message);
            self.log(report.message);
            return;
        };
        // AMD-036 §1: the notification follows the interface language, the log stays English.
        host.notifications
            .show_normal(&notifications::three_d_sen_summary(
                summary.imported,
                summary.already_present,
                summary.failed,
            ));
        self.log(report.message);
        if summary.imported > 0 {
            play(host, UiSound::Confirm);
            self.rebuild_chooser(host);
        }
    }

    fn import_profile(&mut self, host: &mut AppHost, path: &str) {
        let result = host.games_mut().import_profile(Path::new(path));
        if !result.message.is_empty() {
            // REQ-UI-084: an import failure is a notification of normal severity.
            host.notifications.show_normal(&result.message);
            return;
        }
        if matches!(
            result.outcome,
            ProfileImportOutcome::Added | ProfileImportOutcome::Replaced
        ) {
            if let Some(profile) = result.profile {
                self.log(format!("profile '{}' imported.", profile.title));
                play(host, UiSound::Confirm);
                // The chooser is rebuilt from the library, so a new profile shows up at once.
                self.rebuild_chooser(host);
            }
        }
    }

    fn rebuild_chooser(&mut self, host: &AppHost) {
        let rom = self.chooser.rom_path.clone();
        self.chooser = ProfileChooserModel::new(host.games().library());
        self.chooser.reset(rom.as_deref());
    }

    fn cancel_topmost_modal(&mut self) {
        if self.state.dialog {
            // Case 2: "rien". One leaves the box by its buttons only.
            return;
        }
        if self.state.profile_chooser {
            self.state.profile_chooser = false;
        } else if self.state.file_browser {
            self.state.file_browser = false;
        }
    }

    fn show_data_folder(&mut self, host: &AppHost) {
        let root = host.paths().root.clone();
        self.log(format!("user data folder is {}", root.display()));
        self.effects.push(UiEffect::RevealFolder(root));
    }

    /// Plays a sound of the interface (REQ-AUD-040).
    fn activate(host: &mut AppHost, command: Command, args: CommandArgs) {
        play(host, UiSound::Confirm);
        host.enqueue(command, args);
    }
}

/// Plays `sound` on the interface voice, when the host has a mixer.
fn play(host: &mut AppHost, sound: UiSound) {
    if let Some(mixer) = host.mixer_mut() {
        mixer.play_sound(sound);
    }
}

/// The centre of the display for a window of the given size.
fn centre(ui: &imgui::Ui, width: f32, height: f32) -> [f32; 2] {
    let display = ui.io().display_size;
    [(display[0] - width) * 0.5, (display[1] - height) * 0.5]
}
