//! The classic interface of AMD-010: the menu bar, the options and key configuration windows,
//! the dialogs, the welcome screen and the on-screen notifications, drawn with Dear `ImGui`. The
//! state is plain data in the models; this type owns it and draws it, one frame at a time.
//!
//! The window never reaches into the application: what it needs for a frame is lent in a
//! [`UiFrame`], and what it asks of the outside (a line for the log, the data folder opened in
//! the file manager) is returned as [`UiEffect`]s for the loop to carry out.

mod dialogs;
mod hacks;
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
use zax_settings::catalogue::SettingId;

use super::handheld::hacks::HacksModel;

/// Space between the notification and the bottom of the window.
const NOTIFICATION_MARGIN: f32 = 48.0;

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
    /// The frame profiler's line shown under the counter (empty when it is off).
    pub profile_line: &'a str,
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
    /// The ROM folder of the console mode (Rust-only).
    RomsFolder,
    /// A 3dSen installation or bundle to import.
    ThreeDSen,
}

impl BrowserPurpose {
    /// Whether the answer is a folder rather than a file.
    const fn wants_folder(self) -> bool {
        matches!(self, Self::ThreeDSen | Self::RomsFolder)
    }
}

/// The interface.
#[allow(clippy::struct_excessive_bools)] // independent flags of the menu bar and the windows
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
    /// Whether the menu bar was drawn this frame: during a game it shows only under the pointer.
    pub menu_bar_drawn: bool,
    /// Whether a menu of the bar was open, or the bar under the pointer, last frame.
    menu_engaged: bool,
    /// The height of the bar when it was last drawn, the band that reveals it.
    menu_bar_height: f32,
    /// Whether an entry of the bar the desktop carries out (a command of the host, the hacks)
    /// was chosen since the front end last asked.
    menu_chosen: bool,
    /// The capture aid of `--ui-shots`: the menu to open on the next frame.
    pub force_open_menu: Option<String>,
    /// What a shortcut or a menu entry asked of the console mode, for the front end.
    console_request: Option<UiCommand>,
    /// The capture aid of `--ui-shots`: the options tab to select on the next frame.
    pub force_options_tab: Option<OptionsTab>,
    heading_font: Option<imgui::FontId>,
    purpose: BrowserPurpose,
    picker: Option<Box<dyn FilePicker>>,
    fps_shown: f32,
    fps_text: String,
    fps_countdown: f32,
    held_keys: Vec<Key>,
    effects: Vec<UiEffect>,
    /// The note being typed for the frame on screen of the movie (`state.movie_note`).
    movie_note: String,
    /// The hacks of the game in play, while their window is open (`state.hacks`).
    hacks: Option<HacksModel>,
    /// The hacks of the game in play as the game options menu shows them, studied once as the
    /// menu opens (the study reads the ROM and its hack packs' signatures).
    menu_hacks: Option<HacksModel>,
    /// The text of the folder field of the options while it is being typed.
    folder_text: String,
    /// Whether the folder field has the focus (its text is then the one being typed).
    folder_editing: bool,
}

/// The band at the top of the window that reveals the hidden menu bar before it was ever drawn.
const MENU_BAR_REVEAL: f32 = 24.0;

/// Whether the pointer is in the window, within `band` pixels of its top.
fn pointer_at_top(ui: &imgui::Ui, band: f32) -> bool {
    let [x, y] = ui.io().mouse_pos;
    let [width, _] = ui.io().display_size;
    (0.0..=band.max(MENU_BAR_REVEAL)).contains(&y) && (0.0..=width).contains(&x)
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
            menu_bar_drawn: false,
            menu_engaged: false,
            menu_bar_height: MENU_BAR_REVEAL,
            menu_chosen: false,
            force_open_menu: None,
            force_options_tab: None,
            console_request: None,
            heading_font: None,
            purpose: BrowserPurpose::Rom,
            picker: None,
            fps_shown: 0.0,
            fps_text: String::from("0.0 FPS"),
            fps_countdown: 0.0,
            movie_note: String::new(),
            hacks: None,
            menu_hacks: None,
            folder_text: String::new(),
            folder_editing: false,
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

        // During a game the bar hides until the pointer reaches the top of the window, or while
        // one of its menus is open, and then lies over the view (owner request); with no game
        // it stays, above the welcome screen.
        let auto_hide = self.state.has_current_game;
        let shown = self.state.menu_bar_visible()
            && (!auto_hide
                || self.menu_engaged
                || self.force_open_menu.is_some()
                || pointer_at_top(ui, self.menu_bar_height));
        let bar = if shown {
            let (height, engaged) = self.draw_menu_bar(ui, frame.host);
            self.menu_bar_height = height;
            self.menu_engaged = engaged;
            height
        } else {
            self.menu_engaged = false;
            0.0
        };
        self.menu_bar_drawn = shown;
        self.chrome_height = if auto_hide { 0.0 } else { bar };
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
        if self.state.movie_note {
            self.draw_movie_note(ui, frame.host);
        }
        if self.state.hacks {
            self.draw_hacks(ui, frame.host);
        }
        // REQ-UI-110: the confirmation box is drawn last, so it sits over everything else.
        if self.state.dialog {
            self.draw_confirm_dialog(ui, frame.host);
        }
        // REQ-UI-206: the OSD is the one layer full screen keeps.
        Self::draw_notification(ui, frame.host, NOTIFICATION_MARGIN);
        self.draw_fps_counter(ui, frame);
    }

    /// The menu bar over the console mode of the desktop (owner request): as during a game, it
    /// shows while the pointer is at the top of the window or one of its menus is open, and
    /// lies over the screen. Returns whether an entry the desktop carries out was chosen, for
    /// which the front end leaves the console mode.
    pub fn draw_menu_bar_over_console(&mut self, ui: &imgui::Ui, host: &mut AppHost) -> bool {
        self.state.fullscreen = host.settings().fullscreen();
        self.state.has_current_game = host.current_game_id().is_some();
        let shown = self.state.menu_bar_visible()
            && (self.menu_engaged || pointer_at_top(ui, self.menu_bar_height));
        if shown {
            let (height, engaged) = self.draw_menu_bar(ui, host);
            self.menu_bar_height = height;
            self.menu_engaged = engaged;
        } else {
            self.menu_engaged = false;
        }
        self.menu_bar_drawn = shown;
        std::mem::take(&mut self.menu_chosen)
    }

    /// Whether a menu of the bar was open, or the bar under the pointer, last frame: the screen
    /// under it then leaves the pointer alone.
    #[must_use]
    pub const fn menu_engaged(&self) -> bool {
        self.menu_engaged
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
            Command::AnnotateMovie => {
                self.movie_note.clear();
                self.state.movie_note = true;
            }
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
            // The console mode belongs to the front end, which takes the request.
            UiCommand::ToggleConsole | UiCommand::SaveStates | UiCommand::LoadStates => {
                self.console_request = Some(command);
            }
        }
    }

    /// What was asked of the console mode since the last call: switching to it or back, or one
    /// of its save state screens.
    pub fn take_console_request(&mut self) -> Option<UiCommand> {
        self.console_request.take()
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
                // The shortcut of a window toggles it.
                ShortcutCommand::Host(Command::ShowOptions) if self.state.options => {
                    self.state.options = false;
                }
                ShortcutCommand::Host(command) => host.enqueue(command, args),
                ShortcutCommand::Ui(command) => self.run_ui_command(command),
            }
        }
    }

    /// The keys that go down now (not yet seen by [`Self::rising_keys`]) with the modifiers of a
    /// shortcut: that press belongs to the application and not to the game.
    pub fn consumed_keys(&self, is_down: impl Fn(Key) -> bool) -> Vec<Key> {
        let modifiers = ShortcutModifiers {
            control: is_down(Key::ControlLeft) || is_down(Key::ControlRight),
            shift: is_down(Key::ShiftLeft) || is_down(Key::ShiftRight),
            alt: is_down(Key::AltLeft) || is_down(Key::AltRight),
        };
        let fresh: Vec<Key> = menu_shortcuts::watched_keys()
            .into_iter()
            .filter(|key| is_down(*key) && !self.held_keys.contains(key))
            .collect();
        menu_shortcuts::resolve_keys(&fresh, modifiers)
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
            BrowserPurpose::RomsFolder => text::roms_folder_title(),
        }
    }

    fn browser_filters(purpose: BrowserPurpose) -> Vec<file_browser::FileFilter> {
        match purpose {
            BrowserPurpose::Rom => file_browser::rom_filters(),
            BrowserPurpose::Cover => file_browser::cover_filters(),
            BrowserPurpose::ThreeDSen => file_browser::three_d_sen_filters(),
            BrowserPurpose::Profile => file_browser::profile_filters(),
            BrowserPurpose::RomsFolder => Vec::new(),
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
            start_directory: Some(Self::browser_start(host, purpose)),
            // AMD-030 §2: the 3dSen import answers with a folder, and so does the ROM folder.
            folder: purpose.wants_folder(),
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
            Some(&Self::browser_start(host, purpose)),
            // AMD-030 §2: the 3dSen import answers with a folder, and so does the ROM folder.
            purpose.wants_folder(),
            three_d_sen.then_some(hint.as_str()),
        );
        self.state.file_browser = true;
    }

    /// The folder a dialog opens on: the ROM folder in force for the ROM folder, else the last
    /// one browsed (REQ-PERS-066).
    fn browser_start(host: &AppHost, purpose: BrowserPurpose) -> String {
        let folder = host.settings().get(SettingId::UiRomsFolder);
        if purpose == BrowserPurpose::RomsFolder && !folder.as_text().is_empty() {
            return folder.as_text().to_owned();
        }
        host.settings().last_browse_directory()
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
            BrowserPurpose::RomsFolder => {
                host.update_settings(|model| model.set(SettingId::UiRomsFolder, path));
            }
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
        if self.state.movie_note {
            self.state.movie_note = false;
        } else if self.state.profile_chooser {
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
