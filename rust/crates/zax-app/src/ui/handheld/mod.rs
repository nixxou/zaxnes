//! The handheld interface (Rust-only): gamepad only, full-screen lists, no mouse and no
//! keyboard, for 640x480, 720x480 and 720x720 screens. The screens are models
//! ([`rom_list`], [`profile_list`], [`overlay`], [`options`]) that answer a press of a button with
//! a command; [`HandheldUi`] owns them, carries the commands out on the host and draws them.
//! In the console mode of the desktop the mouse acts too, through the same presses
//! ([`pointer`]).

pub mod compat;
pub mod defaults;
mod draw;
pub mod exit_combo;
pub mod game_options;
pub mod hacks;
pub mod keyboard;
pub mod layout;
pub mod options;
pub mod overlay;
pub mod overscan;
pub mod pad;
pub mod pointer;
pub mod profile_list;
pub mod rom_list;
pub mod states;

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use zax_platform::ui_image::{PictureUpdate, UiPicture, FIRST_PICTURE_ID};
use zax_render::numerics::Vector3;
use zax_settings::catalogue::SettingId;
use zax_settings::model::SettingsModel;
use zax_settings::values::SettingValue;

use self::exit_combo::{ComboEvent, ExitCombo};
use self::game_options::{GameOptionsCommand, GameOptionsModel};
use self::hacks::{HacksCommand, HacksModel, HacksView};
use self::layout::Metrics;
use self::options::{HandheldOptionsModel, OptionsCommand};
use self::overlay::{CameraPreset, OverlayCommand, OverlayModel};
use self::overscan::{OverscanCommand, OverscanModel};
use self::pad::{PadButton, PadState};
use self::pointer::{PointerInput, Zone};
use self::profile_list::{ProfileListCommand, ProfileListModel};
use self::rom_list::{ProfileMatch, RomEntry, RomListCommand, RomListModel};
use self::states::{SlotState, StatesCommand, StatesMode, StatesModel};
use super::key_config::{self, KeyConfigModel, KeyConfigRow};
use super::profile_chooser::{ProfileChoice, ProfileChooserModel};
use super::zax_ui::{UiEffect, UiFrame};
use crate::application::AppHost;
use crate::emulation::commands::{Command, CommandArgs};
use crate::emulation::game_catalog::GameCatalog as _;
use crate::emulation::host::{HostEvent, SlotPatchStatus};
use crate::input::BindingDeviceKind;
use crate::persistence::save_slots::numbered_slot_id;
use crate::ui::{keys, strings};

/// Seconds a binding waits for a button before it gives up.
pub const CAPTURE_SECONDS: f32 = 5.0;

/// The texture id of the thumbnail the save state screen shows.
pub const STATE_PICTURE_ID: usize = FIRST_PICTURE_ID;

/// The screen in front.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    /// The list of games.
    Home,
    /// The profiles of one game.
    Profiles,
    /// The reduced options.
    Options,
    /// The options of one game, from the list of games.
    GameOptions,
    /// The hacks of one game (Rust-only): from the list, its options or the in-game menu.
    Hacks,
    /// The gamepad bindings of player one.
    Controls,
    /// The overscan (Rust-only): from the options, or a game's from its options.
    Overscan,
    /// The in-game menu.
    Overlay,
    /// The save state slots, to save or to load (from the in-game menu).
    States,
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
    game_options: Option<GameOptionsModel>,
    /// The hacks of one game, while their screen is open.
    hacks: Option<HacksModel>,
    /// The widescreen button held (owner request: released, it turns the widescreen on with
    /// everything or off; held, it shows its settings).
    widescreen_hold: Option<WidescreenHold>,
    /// Where the hacks go back to: the list, the game's options or the in-game menu.
    hacks_return: Screen,
    controls: ControlsState,
    /// The overscan's screen; a game's goes back to its options, the other to the options.
    overscan: OverscanModel,
    keys: KeyConfigModel,
    combo: ExitCombo,
    metrics: Metrics,
    heading_font: Option<imgui::FontId>,
    effects: Vec<UiEffect>,
    camera_memory: CameraMemory,
    /// How long Y has been held in game, while it is, and whether the hold already acted.
    flat_view: Option<(f32, bool)>,
    /// The game the list rests on and for how long, until it is studied (owner request).
    resting_on: Option<(PathBuf, f32)>,
    /// Where the head-on 2D picture of a held Y stands.
    head_on: HeadOn,
    /// Whether the game was in 2D at the last look: going into it puts the picture head on,
    /// leaving it gives the camera back.
    was_two_dimensional: bool,
    states: StatesModel,
    /// The slot and the save time of the thumbnail shown, and whether it has one.
    shown_picture: Option<(i32, i64, bool)>,
    /// The changes of the pictures, for the renderer.
    pictures: Vec<PictureUpdate>,
    /// Whether the host waits for an answer to a question, which the interface then asks.
    confirming: bool,
    /// Where the save state screen goes back to: the in-game menu, or the game when a shortcut
    /// of the desktop opened it.
    states_return: Screen,
    /// Whether this is a handheld, rather than the console mode of the desktop (F1).
    device: bool,
    /// The sizes the desktop pins, its fonts being baked once whatever the window becomes.
    fixed_metrics: Option<Metrics>,
    /// The body face of the desktop's console mode; a handheld's is the atlas default.
    body_font: Option<imgui::FontId>,
    /// Where the rows and the hints of the last drawn screen lie, for the mouse.
    zones: RefCell<Vec<Zone>>,
}

/// L2 in game (Rust-only, owner request): held, it keeps the camera's angles and distance; a
/// short press puts them back.
#[derive(Debug, Default)]
struct CameraMemory {
    /// How long L2 has been held, while it is.
    held: Option<f32>,
    /// Whether this hold already kept the camera.
    kept: bool,
    /// The kept angles and distance.
    saved: Option<(Vector3, f32)>,
}

/// The head-on 2D picture a held Y asks for: it waits for the 2D mode to show, then stays until
/// the game leaves the 2D mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeadOn {
    Off,
    /// Asked for, the switch to 2D not done yet.
    Waiting,
    /// Shown.
    Shown,
}

/// How long L2 is held to keep the camera, and Y to see the game in 2D from the front.
const CAMERA_KEEP_SECONDS: f32 = 0.6;
/// How long the widescreen button is held before its settings open.
const WIDESCREEN_HOLD_SECONDS: f32 = 0.5;
/// Seconds the list rests on a game before its ROM is studied (owner request).
const STUDY_AFTER_SECONDS: f32 = 1.0;

/// The widescreen of a game asked for with a button (A in the in-game menu or on the widescreen
/// row of its options, L2 on the list), which may be held.
#[derive(Debug, Clone)]
pub(super) struct WidescreenHold {
    button: PadButton,
    rom: PathBuf,
    title: Option<String>,
    /// The screen its settings go back to.
    return_to: Screen,
    held: f32,
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
            game_options: None,
            hacks: None,
            widescreen_hold: None,
            hacks_return: Screen::Home,
            overscan: OverscanModel::new(false),
            controls: ControlsState::default(),
            keys: KeyConfigModel::new(),
            combo: ExitCombo::new(),
            metrics: Metrics::for_height(height),
            heading_font: None,
            effects: Vec::new(),
            camera_memory: CameraMemory::default(),
            resting_on: None,
            flat_view: None,
            head_on: HeadOn::Off,
            was_two_dimensional: false,
            states: StatesModel::new(StatesMode::Save, 1),
            states_return: Screen::Overlay,
            device: true,
            fixed_metrics: None,
            body_font: None,
            shown_picture: None,
            pictures: Vec::new(),
            confirming: false,
            zones: RefCell::new(Vec::new()),
        };
        ui.refresh_roms(host);
        ui
    }

    /// The face of the titles.
    pub const fn set_heading_font(&mut self, font: Option<imgui::FontId>) {
        self.heading_font = font;
    }

    /// Makes this the console mode of the desktop (Rust-only, owner request): the sizes of its
    /// fonts, baked for `metrics`, and the body face. Holding Select and Start does not quit
    /// there.
    pub const fn use_on_desktop(&mut self, metrics: Metrics, body_font: Option<imgui::FontId>) {
        self.device = false;
        self.fixed_metrics = Some(metrics);
        self.metrics = metrics;
        self.body_font = body_font;
    }

    /// The sizes of the screens for a display `height` pixels high.
    fn metrics_for(&self, height: f32) -> Metrics {
        self.fixed_metrics
            .unwrap_or_else(|| Metrics::for_height(height))
    }

    /// The body face, for the drawing.
    pub(super) const fn body_font(&self) -> Option<imgui::FontId> {
        self.body_font
    }

    /// Leaves the console mode of the desktop: a menu open over the game closes and the game
    /// goes on; with no game, the list is what comes back next time.
    pub fn leave_for_desktop(&mut self, host: &mut AppHost) {
        self.game_options = None;
        self.hacks = None;
        self.flat_view = None;
        if host.current_game_id().is_some() {
            if self.screen != Screen::InGame {
                host.enqueue(Command::Resume, CommandArgs::default());
            }
            self.screen = Screen::InGame;
        } else {
            self.screen = Screen::Home;
            self.return_to = Screen::Home;
        }
    }

    /// Opens the save state screen over the game in play, which pauses (a shortcut of the
    /// desktop); going back resumes it. False with no game.
    pub fn open_states(&mut self, mode: StatesMode, host: &mut AppHost) -> bool {
        if host.current_game_id().is_none() || host.is_loading() {
            return false;
        }
        match self.screen {
            Screen::InGame => {
                host.enqueue(Command::Pause, CommandArgs::default());
                self.states_return = Screen::InGame;
            }
            Screen::States => {}
            _ => self.states_return = Screen::Overlay,
        }
        self.states = StatesModel::new(mode, self.overlay.slot());
        self.screen = Screen::States;
        true
    }

    /// The screen in front.
    #[must_use]
    pub const fn screen(&self) -> Screen {
        self.screen
    }

    /// Whether the interface holds the input: everything but the game.
    #[must_use]
    pub fn is_modal(&self) -> bool {
        self.screen != Screen::InGame || self.confirming
    }

    /// The changes of the interface's pictures since the last call, for the renderer.
    pub fn take_picture_updates(&mut self) -> Vec<PictureUpdate> {
        std::mem::take(&mut self.pictures)
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
        // The widescreen of the games without one of their own: the global value.
        let mut global = host.settings().clone();
        global.clear_session_layer();
        global.clear_game_values();
        let global_on = global.get(SettingId::VideoWidescreen).as_int() != 0;
        for entry in &mut entries {
            entry.profile = self.profile_of(host, entry);
            Self::find_widescreen(host, entry);
            entry.widescreen_on = host
                .game_options(&entry.path.to_string_lossy())
                .iter()
                .find(|(id, _)| *id == SettingId::VideoWidescreen)
                .map_or(global_on, |(_, value)| value.as_int() != 0);
        }
        self.roms.replace(entries);
    }

    /// Whether a widescreen hack fits the game of `entry`, without reading its file (owner
    /// request): what the study of the ROM found (its hacks screen, its start), or the CRC the
    /// cache keeps, or else the patch its profile or its name says, not studied yet.
    fn find_widescreen(host: &AppHost, entry: &mut RomEntry) {
        use crate::persistence::hack_compat_cache::Certificate;
        entry.crc = host.known_rom_crc(&entry.path);
        let certificate = host.rom_certificate(&entry.path);
        entry.widescreen = match &certificate {
            Some(Certificate::Fits { .. }) => true,
            Some(Certificate::Unfit) => false,
            None => entry
                .crc
                .is_some_and(zax_nes::builtin_patches::widescreen_capable),
        };
        entry.widescreen_other = (!entry.widescreen)
            .then(|| hacks::other_rom_patch(entry.profile_title(), &entry.name))
            .flatten()
            .map(|patch| patch.name);
        entry.widescreen_unstudied =
            certificate.is_none() && !entry.widescreen && entry.widescreen_other.is_some();
        let own = host.game_options(&entry.path.to_string_lossy());
        entry.widescreen_forced = entry.widescreen_other.is_some_and(|patch| {
            own.iter()
                .any(|(id, value)| *id == SettingId::EmuForcedPatch && value.as_text() == patch)
        });
    }

    /// The list shows what the study of the ROM in `path` found, once its hacks screen opened
    /// or its game started.
    fn note_study(&mut self, path: &Path, host: &AppHost) {
        if let Some(entry) = self
            .roms
            .entries_mut()
            .iter_mut()
            .find(|entry| entry.path == path)
        {
            Self::find_widescreen(host, entry);
        }
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
        self.confirming = frame.host.pending_confirmation().is_some();
        self.follow_the_game(frame.host);
        self.update_combo(pad, seconds, frame.host);
        self.update_capture(seconds, frame);
        self.update_camera_memory(pad, seconds, frame.host);
        self.update_flat_view(pad, seconds, frame.host);
        self.update_widescreen_hold(pad, seconds, frame.host);
        self.update_resting_study(seconds, frame.host);
        let presses: Vec<PadButton> = pad.presses().collect();
        for button in presses.iter().copied() {
            // A key that is A and Start at once (Enter) plays the game once, and Start with
            // Select held is the exit combination.
            let both = button == PadButton::Start
                && (presses.contains(&PadButton::A) || pad.is_held(PadButton::Select));
            if both && self.screen == Screen::Home {
                continue;
            }
            self.press(button, frame);
        }
    }

    /// The widescreen button held: past [`WIDESCREEN_HOLD_SECONDS`] its settings open; let go
    /// before, the widescreen is turned on with everything or off.
    fn update_widescreen_hold(&mut self, pad: &PadState, seconds: f32, host: &mut AppHost) {
        let Some(mut hold) = self.widescreen_hold.take() else {
            return;
        };
        if pad.is_held(hold.button) {
            hold.held += seconds;
            if hold.held >= WIDESCREEN_HOLD_SECONDS {
                self.open_hacks(
                    &hold.rom,
                    hold.title.as_deref(),
                    hold.return_to,
                    host,
                    HacksView::Widescreen,
                );
            } else {
                self.widescreen_hold = Some(hold);
            }
            return;
        }
        let mut hacks = HacksModel::for_rom(host, &hold.rom, hold.title.as_deref());
        hacks.toggle_widescreen_all();
        hacks.apply(host);
        self.note_widescreen(&hold.rom, hacks.widescreen_on());
        // The options of the game are read again, with the widescreen as it is now.
        if hold.return_to == Screen::GameOptions && self.screen == Screen::GameOptions {
            self.open_game_options(&hold.rom, host);
            if let Some(options) = &mut self.game_options {
                options.select(GameOptionsModel::WIDESCREEN_ROW);
            }
        }
    }

    /// Starts a press of the widescreen button `button` for the game in `rom`.
    fn hold_widescreen(
        &mut self,
        button: PadButton,
        rom: PathBuf,
        title: Option<String>,
        return_to: Screen,
    ) {
        self.widescreen_hold = Some(WidescreenHold {
            button,
            rom,
            title,
            return_to,
            held: 0.0,
        });
    }

    /// The mouse of the desktop's console mode (owner request): what it did over the screen
    /// drawn last frame, as presses of the pad. Nothing in game.
    pub fn update_pointer(&mut self, input: &PointerInput, frame: &mut UiFrame<'_>) {
        if self.screen == Screen::InGame && frame.host.pending_confirmation().is_none() {
            return;
        }
        let presses = pointer::presses(&self.zones.borrow(), input);
        for button in presses {
            self.press(button, frame);
        }
    }

    /// The list resting [`STUDY_AFTER_SECONDS`] on a game not studied yet studies its ROM (owner
    /// request): a few milliseconds, and the list then shows whether a widescreen hack fits it.
    fn update_resting_study(&mut self, seconds: f32, host: &AppHost) {
        let selected = (self.screen == Screen::Home && !self.confirming)
            .then(|| self.roms.selected_entry().map(|entry| entry.path.clone()))
            .flatten();
        let Some(path) = selected else {
            self.resting_on = None;
            return;
        };
        let held = match self.resting_on.take() {
            Some((rested, held)) if rested == path => held + seconds,
            _ => 0.0,
        };
        if held < STUDY_AFTER_SECONDS {
            self.resting_on = Some((path, held));
            return;
        }
        // Studied once: the same game stays without a new reading until the list moves.
        self.resting_on = Some((path.clone(), f32::NEG_INFINITY));
        if host.rom_certificate(&path).is_some() {
            return;
        }
        if let Ok(rom) = crate::emulation::rom_file::read(&path) {
            host.certify_rom(&path, &rom.image);
            self.note_study(&path, host);
        }
    }

    /// Y in game (owner request): a short press switches between 2D and 3D; a hold shows the game
    /// in 2D seen head on, its picture filling the height of the view, as on a television. The
    /// camera is left alone: back in 3D, it has the angle and the zoom it had.
    fn update_flat_view(&mut self, pad: &PadState, seconds: f32, host: &mut AppHost) {
        self.follow_head_on(host);
        if self.screen != Screen::InGame {
            self.flat_view = None;
            return;
        }
        if pad.is_held(PadButton::Y) {
            let (held, acted) = self.flat_view.get_or_insert((0.0, false));
            *held += seconds;
            if *held >= CAMERA_KEEP_SECONDS && !*acted {
                *acted = true;
                if !host.showing_two_dimensional() {
                    host.enqueue(Command::ToggleTwoDimensional, CommandArgs::default());
                }
                self.head_on = HeadOn::Waiting;
                self.follow_head_on(host);
            }
        } else if let Some((_, acted)) = self.flat_view.take() {
            if !acted {
                host.enqueue(Command::ToggleTwoDimensional, CommandArgs::default());
            }
        }
    }

    /// The head-on picture shows once the game is in 2D, and goes when it leaves it.
    fn follow_head_on(&mut self, host: &mut AppHost) {
        let two_dimensional = host.showing_two_dimensional();
        self.head_on = match (self.head_on, two_dimensional) {
            (HeadOn::Waiting | HeadOn::Shown, true) => HeadOn::Shown,
            (HeadOn::Shown, false) | (HeadOn::Off, _) => HeadOn::Off,
            (HeadOn::Waiting, false) => HeadOn::Waiting,
        };
        // Head on whenever the game goes into 2D (owner request: always from the front, filling
        // the height), not only once asked for; a pull on the picture gives it to the camera
        // until the next time (`crate::head_on_pull`).
        if two_dimensional != self.was_two_dimensional {
            host.scene_mut().picture_head_on = two_dimensional;
        }
        self.was_two_dimensional = two_dimensional;
    }

    /// L2 in game: a hold keeps the camera, a short press puts it back.
    fn update_camera_memory(&mut self, pad: &PadState, seconds: f32, host: &mut AppHost) {
        let memory = &mut self.camera_memory;
        if self.screen != Screen::InGame {
            memory.held = None;
            memory.kept = false;
            return;
        }
        if pad.is_held(PadButton::L2) {
            let held = memory.held.get_or_insert(0.0);
            *held += seconds;
            if *held >= CAMERA_KEEP_SECONDS && !memory.kept {
                memory.kept = true;
                let current = host.scene().camera.current();
                memory.saved = Some((current.angles, current.distance()));
                host.notifications
                    .show_normal(&strings::get(keys::HANDHELD_CAMERA_KEPT));
            }
        } else if memory.held.take().is_some() && !std::mem::take(&mut memory.kept) {
            if let Some((angles, distance)) = memory.saved {
                let setpoint = host.scene_mut().camera.current_mut();
                setpoint.angles = angles;
                setpoint.set_distance(distance);
                host.notifications
                    .show_normal(&strings::get(keys::HANDHELD_CAMERA_RESTORED));
            }
        }
    }

    /// The thumbnail of the slot selected on the save state screen, read again when the
    /// selection or the slot's state changes.
    fn follow_the_state_picture(&mut self, host: &AppHost) {
        // Called by the drawing, which runs in the screenshot runs too.
        if self.screen != Screen::States {
            return;
        }
        let slot = self.states.slot();
        let record = host.slots.find(&numbered_slot_id(slot));
        let saved_at = record
            .filter(|record| record.is_occupied())
            .map_or(0, |record| record.saved_at);
        if self
            .shown_picture
            .is_some_and(|(shown, at, _)| (shown, at) == (slot, saved_at))
        {
            return;
        }
        let picture = (saved_at != 0).then(|| state_picture(host, slot)).flatten();
        self.shown_picture = Some((slot, saved_at, picture.is_some()));
        self.pictures.push(PictureUpdate {
            id: STATE_PICTURE_ID,
            picture,
        });
    }

    /// Whether the save state screen has a thumbnail to show for its selected slot.
    #[must_use]
    pub fn state_picture_shown(&self) -> bool {
        self.shown_picture
            .is_some_and(|(slot, _, shown)| shown && slot == self.states.slot())
    }

    /// The save state screen.
    #[must_use]
    pub const fn states(&self) -> &StatesModel {
        &self.states
    }

    /// The screen follows the game: it hides when one runs and comes back when it stops.
    fn follow_the_game(&mut self, host: &AppHost) {
        let playing = host.current_game_id().is_some() || host.is_loading();
        let hacks_of_the_list = self.hacks_return != Screen::Overlay;
        match (playing, self.screen) {
            (true, Screen::Home | Screen::Profiles | Screen::GameOptions) => {
                self.game_options = None;
                self.screen = Screen::InGame;
            }
            (true, Screen::Overscan) if self.overscan.is_game() => {
                self.game_options = None;
                self.screen = Screen::InGame;
            }
            (true, Screen::Hacks) if hacks_of_the_list => {
                self.game_options = None;
                self.hacks = None;
                self.screen = Screen::InGame;
            }
            (false, Screen::Hacks) if !hacks_of_the_list => {
                self.hacks = None;
                self.show_home(host);
            }
            (false, Screen::InGame | Screen::Overlay | Screen::States) => self.show_home(host),
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
            // The PortMaster convention; a desktop has its window to close.
            ComboEvent::Quit if self.device => {
                host.enqueue(Command::Quit, CommandArgs::default());
            }
            ComboEvent::Tap if menu_is_select_start(host) => self.toggle_overlay(host),
            ComboEvent::Quit | ComboEvent::Tap | ComboEvent::Idle => {}
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
        // A question of the host (a save state made with the built-in patches set the other
        // way, for instance) is answered first: A agrees, B declines.
        if frame.host.pending_confirmation().is_some() {
            match button {
                PadButton::A => {
                    frame.host.confirm_pending();
                    // A patch forced from the list shows on it at once.
                    if self.screen == Screen::Home {
                        self.refresh_roms(frame.host);
                    }
                }
                // The question's second answer, if it has one.
                PadButton::X => {
                    frame.host.alternative_pending();
                }
                // A message with one button goes with B as well.
                PadButton::B | PadButton::Menu => frame.host.cancel_pending(),
                _ => {}
            }
            return;
        }
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
            Screen::GameOptions => {
                let command = self
                    .game_options
                    .as_mut()
                    .and_then(|options| options.handle(button));
                if let Some(command) = command {
                    self.carry_out_game_options(command, frame.host);
                }
            }
            Screen::Hacks => {
                let command = self.hacks.as_mut().and_then(|hacks| hacks.handle(button));
                if let Some(command) = command {
                    self.carry_out_hacks(command, frame.host);
                }
            }
            Screen::Controls => self.press_controls(button, frame),
            Screen::Overscan => self.press_overscan(button, frame.host),
            Screen::Overlay => {
                if let Some(command) = self.overlay.handle(button) {
                    self.carry_out_overlay(command, frame.host);
                }
            }
            Screen::States => {
                let host = &*frame.host;
                let command = self.states.handle(button, |slot| slot_state(host, slot));
                if let Some(command) = command {
                    self.carry_out_states(command, frame.host);
                }
            }
            // X: the default view (owner request); Y and L2 are held buttons, followed by
            // `update_flat_view` and `update_camera_memory`.
            Screen::InGame => {
                if button == PadButton::X {
                    frame
                        .host
                        .enqueue(Command::ResetView, CommandArgs::default());
                }
            }
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
                self.note_study(&path, host);
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
            RomListCommand::GameOptions(path) => self.open_game_options(&path, host),
            RomListCommand::Options => self.open_options(host),
            RomListCommand::Widescreen(path) => {
                // L1 opens the widescreen's settings, no quick toggle on the list (owner request).
                let title = self.profile_title_of(&path);
                self.open_hacks(
                    &path,
                    title.as_deref(),
                    Screen::Home,
                    host,
                    HacksView::Widescreen,
                );
            }
            RomListCommand::Hacks(path) => {
                let title = self.profile_title_of(&path);
                self.open_hacks(&path, title.as_deref(), Screen::Home, host, HacksView::Aids);
            }
        }
    }

    /// The title of the 3D profile the list found for the game in `path`, if any.
    fn profile_title_of(&self, path: &Path) -> Option<String> {
        self.roms
            .entries()
            .iter()
            .find(|entry| entry.path == path)
            .and_then(RomEntry::profile_title)
            .map(str::to_owned)
    }

    /// Opens the hacks of the game in `path` (whose profile is called `title`, if it has one);
    /// B goes back to `return_to`.
    fn open_hacks(
        &mut self,
        path: &Path,
        title: Option<&str>,
        return_to: Screen,
        host: &AppHost,
        view: HacksView,
    ) {
        self.hacks = Some(HacksModel::for_rom(host, path, title).with_view(view));
        self.note_study(path, host);
        self.hacks_return = return_to;
        self.screen = Screen::Hacks;
    }

    fn carry_out_hacks(&mut self, command: HacksCommand, host: &mut AppHost) {
        let Some(hacks) = self.hacks.as_ref() else {
            return;
        };
        let rom = hacks.rom().to_path_buf();
        match command {
            HacksCommand::Save => {
                hacks.apply(host);
                let on = hacks.widescreen_on();
                self.note_widescreen(&rom, on);
            }
            HacksCommand::Close => {
                self.hacks = None;
                self.screen = self.hacks_return;
                // The options of the game are read again, with the hacks as they are now.
                if self.screen == Screen::GameOptions {
                    self.open_game_options(&rom, host);
                    if let Some(options) = &mut self.game_options {
                        options.select(GameOptionsModel::HACKS_ROW);
                    }
                }
            }
        }
    }

    /// The list shows the widescreen of the game in `path` as `on`.
    fn note_widescreen(&mut self, path: &Path, on: bool) {
        if let Some(entry) = self
            .roms
            .entries_mut()
            .iter_mut()
            .find(|entry| entry.path == path)
        {
            entry.widescreen_on = on;
        }
    }

    /// The hacks screen, while it is open.
    #[must_use]
    pub const fn hacks(&self) -> Option<&HacksModel> {
        self.hacks.as_ref()
    }

    fn open_game_options(&mut self, path: &Path, host: &AppHost) {
        let name = RomEntry::new(path.to_path_buf()).name;
        let options = host.game_options(&path.to_string_lossy());
        self.game_options = Some(GameOptionsModel::new(
            path.to_path_buf(),
            name,
            host.settings(),
            options,
        ));
        self.screen = Screen::GameOptions;
    }

    fn carry_out_game_options(&mut self, command: GameOptionsCommand, host: &mut AppHost) {
        match command {
            GameOptionsCommand::Save => {
                if let Some(options) = &self.game_options {
                    host.set_game_options(&options.rom().to_string_lossy(), options.options());
                }
            }
            GameOptionsCommand::Close => {
                self.game_options = None;
                self.screen = Screen::Home;
            }
            GameOptionsCommand::OpenOverscan => {
                self.overscan = OverscanModel::new(true);
                self.screen = Screen::Overscan;
            }
            GameOptionsCommand::Widescreen | GameOptionsCommand::OpenWidescreen => {
                if let Some(rom) = self
                    .game_options
                    .as_ref()
                    .map(|options| options.rom().clone())
                {
                    let title = self.profile_title_of(&rom);
                    if command == GameOptionsCommand::Widescreen {
                        self.hold_widescreen(PadButton::A, rom, title, Screen::GameOptions);
                    } else {
                        self.open_hacks(
                            &rom,
                            title.as_deref(),
                            Screen::GameOptions,
                            host,
                            HacksView::Widescreen,
                        );
                    }
                }
            }
            GameOptionsCommand::OpenHacks => {
                if let Some(rom) = self
                    .game_options
                    .as_ref()
                    .map(|options| options.rom().clone())
                {
                    let title = self.profile_title_of(&rom);
                    self.open_hacks(
                        &rom,
                        title.as_deref(),
                        Screen::GameOptions,
                        host,
                        HacksView::Aids,
                    );
                }
            }
        }
    }

    /// Whether the game in `path` has options of its own, for the hint of the list.
    fn has_game_options(host: &AppHost, path: &Path) -> bool {
        host.game_options(&path.to_string_lossy())
            .iter()
            .any(|(id, _)| game_options::is_game_row_setting(*id))
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
        if !matches!(
            self.screen,
            Screen::Options | Screen::Controls | Screen::Overscan
        ) {
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
            OptionsCommand::OpenOverscan => {
                self.overscan = OverscanModel::new(false);
                self.screen = Screen::Overscan;
            }
            OptionsCommand::StepCamera(step) => {
                let preset = self.overlay.step_camera(step);
                apply_camera(preset, frame.host);
            }
        }
    }

    /// A press on the overscan's screen: a game's values go to its options, the others to the
    /// preferences.
    fn press_overscan(&mut self, button: PadButton, host: &mut AppHost) {
        let overscan = &mut self.overscan;
        let command = if overscan.is_game() {
            let Some(options) = self.game_options.as_mut() else {
                self.screen = Screen::Home;
                return;
            };
            overscan.handle(button, |id, step| options.step_setting(id, step))
        } else {
            host.update_settings(|settings| {
                overscan.handle(button, |id, step| overscan::step(id, settings, step))
            })
        };
        match command {
            Some(OverscanCommand::Changed) if self.overscan.is_game() => {
                self.carry_out_game_options(GameOptionsCommand::Save, host);
            }
            Some(OverscanCommand::Close) => {
                self.screen = if self.overscan.is_game() {
                    Screen::GameOptions
                } else {
                    Screen::Options
                };
            }
            _ => {}
        }
    }

    fn carry_out_overlay(&mut self, command: OverlayCommand, host: &mut AppHost) {
        let close = |ui: &mut Self, host: &mut AppHost| {
            host.enqueue(Command::Resume, CommandArgs::default());
            ui.screen = Screen::InGame;
        };
        match command {
            OverlayCommand::Resume => close(self, host),
            OverlayCommand::ChooseState(mode, slot) => {
                self.states_return = Screen::Overlay;
                self.states = StatesModel::new(mode, slot);
                self.screen = Screen::States;
            }
            OverlayCommand::ToggleFastForward => {
                close(self, host);
                host.enqueue(Command::FastForward, CommandArgs::default());
            }
            OverlayCommand::SetCamera(preset) => apply_camera(preset, host),
            OverlayCommand::Options => self.open_options(host),
            OverlayCommand::Widescreen => {
                if let Some(rom) = host.current_rom_path().map(PathBuf::from) {
                    let title = host.current_game_title().map(str::to_owned);
                    self.hold_widescreen(PadButton::A, rom, title, Screen::Overlay);
                }
            }
            OverlayCommand::Hacks => {
                if let Some(rom) = host.current_rom_path().map(PathBuf::from) {
                    let title = host.current_game_title().map(str::to_owned);
                    self.open_hacks(
                        &rom,
                        title.as_deref(),
                        Screen::Overlay,
                        host,
                        HacksView::Aids,
                    );
                }
            }
            OverlayCommand::QuitToList => {
                host.enqueue(Command::Stop, CommandArgs::default());
                self.show_home(host);
            }
            OverlayCommand::ExitToLauncher => host.enqueue(Command::Quit, CommandArgs::default()),
        }
    }

    fn carry_out_states(&mut self, command: StatesCommand, host: &mut AppHost) {
        let (command, slot) = match command {
            StatesCommand::Back => {
                if self.states_return == Screen::InGame {
                    host.enqueue(Command::Resume, CommandArgs::default());
                }
                self.screen = self.states_return;
                return;
            }
            StatesCommand::Save(slot) => (Command::SaveSlot, slot),
            StatesCommand::Load(slot) => (Command::LoadSlot, slot),
        };
        // The menu then shows the slot used last.
        self.overlay.set_slot(slot);
        host.enqueue(command, CommandArgs::of_number(slot));
        host.enqueue(Command::Resume, CommandArgs::default());
        self.screen = Screen::InGame;
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

    /// Shows the ROMs of `directory` from now on (the ROM folder of the options changed).
    pub fn set_roms_directory(&mut self, host: &AppHost, directory: PathBuf) {
        if directory != self.roms_directory {
            self.roms_directory = directory;
            self.refresh_roms(host);
        }
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
            Screen::GameOptions => {
                if let Some(entry) = self.roms.selected_entry().cloned() {
                    let options = vec![
                        (SettingId::RenderShadow, SettingValue::Bool(false)),
                        (SettingId::EmuCatchUp, SettingValue::Enum(3)),
                    ];
                    self.game_options = Some(GameOptionsModel::new(
                        entry.path,
                        entry.name,
                        &SettingsModel::new(),
                        options,
                    ));
                    self.screen = Screen::GameOptions;
                }
            }
            Screen::Controls => {
                self.controls = ControlsState::default();
                self.screen = Screen::Controls;
            }
            Screen::Overscan => {
                self.open_options_for_capture();
                self.overscan = OverscanModel::new(false);
                self.screen = Screen::Overscan;
            }
            // The hacks of Mega Man 2 (USA), told by the CRC of its data, the widescreen on.
            Screen::Hacks => {
                if let Some(entry) = self.roms.selected_entry().cloned() {
                    let mut hacks = HacksModel::new(
                        entry.path,
                        Some(0x0FCF_C04D),
                        &SettingsModel::new(),
                        Vec::new(),
                        |patch| {
                            patch
                                .options
                                .iter()
                                .map(|option| (*option, option.default))
                                .collect()
                        },
                    );
                    hacks.toggle_widescreen();
                    self.hacks = Some(hacks);
                    self.hacks_return = Screen::Home;
                    self.screen = Screen::Hacks;
                }
            }
            Screen::Overlay => {
                self.overlay.reopen();
                self.screen = Screen::Overlay;
            }
            // Twice in a row: the screen that loads, then the one that saves.
            Screen::States => {
                let mode = if self.screen == Screen::States {
                    StatesMode::Save
                } else {
                    StatesMode::Load
                };
                self.states = StatesModel::new(mode, 1);
                self.screen = Screen::States;
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

/// How the numbered slot `slot` stands for the game being played.
fn slot_state(host: &AppHost, slot: i32) -> SlotState {
    let id = numbered_slot_id(slot);
    if !host.slots.is_occupied(&id) {
        return SlotState::Empty;
    }
    match host.slot_patch_status(&id) {
        SlotPatchStatus::Fits => SlotState::Fits,
        SlotPatchStatus::OtherComponents => SlotState::OtherMode,
        SlotPatchStatus::OtherVersion => SlotState::OtherVersion,
    }
}

/// The thumbnail of the numbered slot `slot`, as the renderer takes it: none when it has no
/// readable one.
fn state_picture(host: &AppHost, slot: i32) -> Option<UiPicture> {
    let file = host.slot_thumbnail_file(&numbered_slot_id(slot));
    let image = zax_render::assets::png_image::decode(&std::fs::read(file).ok()?).ok()?;
    Some(UiPicture {
        width: u32::try_from(image.width).ok()?,
        height: u32::try_from(image.height).ok()?,
        rgba: image.pixels,
    })
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
