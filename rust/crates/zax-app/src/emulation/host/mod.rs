//! The emulation host: the state machine of SPEC-07, the queue of commands, the frame clock,
//! the save states, the games, and the settings that steer them.
//!
//! One owner holds everything: the settings model, the scene's handle, the catalogue, the
//! mixer, the session of the game being played. Commands and events are enums; the
//! interface drives the host through [`Command`]s and reads it through [`HostEvent`]s.

mod commands;
mod game_options;
mod games;
mod jobs;
mod save_states;

use std::collections::VecDeque;

use time::OffsetDateTime;
use zax_nes::NesRegion;
use zax_nes::RegionTiming;
use zax_render::geometry as render_geometry;
use zax_scripting::{HookTime, NullScriptLog, ScriptLog};
use zax_settings::catalogue::SettingId;
use zax_settings::model::SettingsModel;

use super::commands::{Command, CommandArgs};
use super::emulation_state::EmulationState;
use super::fast_forward_state::FastForwardState;
use super::frame_pacer::{system_clock, CatchUp, Clock, FramePacer};
use super::game_catalog::{GameCatalog, ResolvedGame};
use super::hotkey_tracker::{HotkeyEvents, HotkeyTracker};
use super::notification_center::NotificationCenter;
use super::rollback_history::RollbackHistory;
use super::speed_setting;
use crate::audio::AudioMixer;
use crate::frame_profiler::{Stage, StageClock, StageTimes};
use crate::game_session::{GameSession, SessionEvent, SessionFrame};
use crate::input::routing;
use crate::input::zapper::{ZapperInput, ZapperState};
use crate::input::InputSnapshot;
use crate::persistence::app_settings::AppSettings;
use crate::persistence::battery_ram_store::BatteryRamStore;
use crate::persistence::save_slots::SaveSlotIndex;
use crate::persistence::settings_store::SettingsStore;
use zax_render::scene::SceneState;

use crate::scene_script_bridge;
use crate::user_data_paths::UserDataPaths;

pub use games::{
    delete_slot_question, ConfirmationRequest, ConfirmedAction, DELETE_GAME_QUESTION, NO_LABEL,
    YES_LABEL,
};
pub use jobs::{LoadKind, SlotPatchStatus};

/// Width of the save state and cover thumbnails.
pub const THUMBNAIL_WIDTH: usize = 256;
/// Height of the save state and cover thumbnails.
pub const THUMBNAIL_HEIGHT: usize = 192;

/// What the host tells the interface and the application around it.
#[derive(Debug, Clone, PartialEq)]
pub enum HostEvent {
    /// A command asks the interface for a window or a dialogue.
    UiRequested {
        /// The command.
        command: Command,
        /// Its argument.
        args: CommandArgs,
    },
    /// A setting changed, whoever changed it.
    SettingChanged(SettingId),
    /// The language of the interface changed; the value is the code now in force.
    LanguageChanged(String),
}

/// Produces the log a new session's scripts write to.
pub type ScriptLogFactory = Box<dyn Fn() -> Box<dyn ScriptLog>>;
/// The picture of the game as the player sees it, which the thumbnails of the save states and
/// the covers are cut from. The host owns the view it is given; the application reaches it
/// through [`EmulationHost::view`] to draw into it.
pub trait GameView {
    /// Captures the game view as a 256 x 192 RGB picture, rows top to bottom.
    fn capture_thumbnail(&mut self) -> Option<Vec<u8>>;
}

/// The view of a host that has none (the headless host of the tests that need no picture).
#[derive(Debug, Clone, Copy)]
pub enum NoView {}

impl GameView for NoView {
    fn capture_thumbnail(&mut self) -> Option<Vec<u8>> {
        match *self {}
    }
}
/// The time of day.
pub type Now = Box<dyn Fn() -> OffsetDateTime>;
/// Receives the lines the host logs.
pub type LogFn = Box<dyn Fn(&str)>;

/// What the host is made of.
#[allow(clippy::struct_excessive_bools)] // independent switches, as the C# options record
pub struct EmulationHostOptions<C: GameCatalog, V: GameView = NoView> {
    /// The user data folder.
    pub paths: UserDataPaths,
    /// The settings model, which the host owns from here on.
    pub settings: SettingsModel,
    /// The scene the renderer draws and the scripts drive.
    pub scene: SceneState,
    /// The library of games.
    pub games: C,
    /// The program's own audio mix.
    pub mixer: Option<AudioMixer>,
    /// Where each session's scripts log; the null log by default.
    pub script_log: Option<ScriptLogFactory>,
    /// The picture of the game, for the thumbnails.
    pub view: Option<V>,
    /// The time of day; the system's by default.
    pub now: Option<Now>,
    /// The monotonic clock of the frame pacer; the system's by default.
    pub clock: Option<Clock>,
    /// Where the host logs.
    pub log: Option<LogFn>,
    /// Whether an audio device is open, which turns on the drift correction.
    pub audio_device_open: bool,
    /// The settings and game list files; none when nothing is persisted.
    pub settings_store: Option<SettingsStore>,
    /// Whether deletions wait for the interface's confirmation (REQ-UI-110).
    pub confirm_with_interface: bool,
}

impl<C: GameCatalog, V: GameView> EmulationHostOptions<C, V> {
    /// Options with the required parts; everything else is absent.
    #[must_use]
    pub fn new(paths: UserDataPaths, settings: SettingsModel, scene: SceneState, games: C) -> Self {
        Self {
            paths,
            settings,
            scene,
            games,
            mixer: None,
            script_log: None,
            view: None,
            now: None,
            clock: None,
            log: None,
            audio_device_open: false,
            settings_store: None,
            confirm_with_interface: false,
        }
    }
}

/// The emulation host.
#[allow(clippy::struct_excessive_bools)] // independent flags of the loop, as the C# class
pub struct EmulationHost<C: GameCatalog, V: GameView = NoView> {
    paths: UserDataPaths,
    scene: SceneState,
    games: C,
    mixer: Option<AudioMixer>,
    settings: SettingsModel,
    settings_store: Option<SettingsStore>,
    script_log: ScriptLogFactory,
    view: Option<V>,
    now: Now,
    log: LogFn,
    modal_window_open: bool,
    audio_device_open: bool,
    confirm_with_interface: bool,
    pending: VecDeque<(Command, CommandArgs)>,
    hotkeys: HotkeyTracker,
    battery: BatteryRamStore,
    session: Option<GameSession>,
    current_game: Option<ResolvedGame>,
    /// The `games.json` key of the ROM whose options are in force, while a game plays.
    game_options_rom: Option<String>,
    pause_thumbnail: Option<Vec<u8>>,
    job: Option<jobs::LoadJob>,
    loading: bool,
    confirmation: Option<ConfirmationRequest>,
    events: Vec<HostEvent>,
    state: EmulationState,
    load_progress: f32,
    two_dimensional_view: bool,
    emulated_frames: i64,
    quit_requested: bool,
    scene_scale: f32,
    elapsed: f64,
    was_running_before_suspend: bool,
    stage_clock: StageClock,
    /// The frame clock.
    pub pacer: FramePacer,
    /// The rollback history.
    pub rollback: RollbackHistory,
    /// Whether the rollback snapshots are taken (REQ-LOOP-030); off on a handheld (Rust-only,
    /// owner request): no savestate every 60 frames, and the rollback finds nothing to go back to.
    pub rollback_enabled: bool,
    /// Whether each emulated frame runs on the emulation thread while the previous one is drawn
    /// (Rust-only "emulation on a separate core", handheld default): the game sees its inputs
    /// one displayed frame later, nothing else changes. See `GameSession::prepare_frame_timed`.
    pub separate_core: bool,
    /// Whether, with [`EmulationHost::separate_core`], the next frame may start before the
    /// scripts of the last one have run, for a profile whose scripts leave the core alone
    /// (`GameSession::can_run_ahead`): "emulation ahead", on by default.
    pub core_ahead: bool,
    /// The fast-forward burst.
    pub fast_forward: FastForwardState,
    /// The notification area.
    pub notifications: NotificationCenter,
    /// The save slots.
    pub slots: SaveSlotIndex,
    /// Whether each slot's state fits the built-in ROM patches, by slot, save time and setting.
    slot_patch_cache:
        std::cell::RefCell<std::collections::HashMap<(String, i64, bool), jobs::SlotPatchStatus>>,
    /// The Zapper.
    pub zapper: ZapperState,
    /// How far the frame clock catches up when the command line or the environment says so;
    /// `None` follows the `emu.catch_up` setting (Rust-only).
    pub catch_up: Option<CatchUp>,
    /// Whether this tick leaves the conversion of its last frame to the caller, which runs it
    /// on a thread of its own ([`EmulationHost::deferred_parts`], `GameSession::convert_deferred`)
    /// while it draws the frame before: the pipelined frames of `emu.pipeline` (Rust-only). Only
    /// honoured when [`EmulationHost::can_defer_conversion`].
    pub defer_conversion: bool,
}

/// What the conversion of a pipelined frame works on, and the view the main thread draws into
/// meanwhile.
pub struct DeferredParts<'a, V> {
    pub session: &'a mut GameSession,
    pub settings: &'a mut SettingsModel,
    pub scene: &'a mut SceneState,
    pub view: Option<&'a V>,
}

impl<C: GameCatalog, V: GameView> EmulationHost<C, V> {
    /// Creates the host: loads the slot index, reconciles it with the library, reads the
    /// language and the speed from the settings.
    #[must_use]
    pub fn new(options: EmulationHostOptions<C, V>) -> Self {
        let EmulationHostOptions {
            paths,
            settings,
            scene,
            games,
            mixer,
            script_log,
            view,
            now,
            clock,
            log,
            audio_device_open,
            settings_store,
            confirm_with_interface,
        } = options;
        let now: Now = now.unwrap_or_else(|| Box::new(OffsetDateTime::now_utc));
        let mut slots = SaveSlotIndex::load(&paths, now());
        slots.reconcile(&paths, |game_id| games.exists(game_id));
        let mut host = Self {
            pacer: FramePacer::new(
                f64::from(RegionTiming::for_region(NesRegion::Ntsc).nominal_frames_per_second),
                clock.unwrap_or_else(system_clock),
            ),
            paths,
            scene,
            games,
            mixer,
            settings,
            settings_store,
            script_log: script_log.unwrap_or_else(|| Box::new(|| Box::new(NullScriptLog))),
            view,
            now,
            log: log.unwrap_or_else(|| Box::new(|_| {})),
            modal_window_open: false,
            audio_device_open,
            confirm_with_interface,
            pending: VecDeque::new(),
            hotkeys: HotkeyTracker::new(),
            battery: BatteryRamStore::new(),
            session: None,
            current_game: None,
            game_options_rom: None,
            pause_thumbnail: None,
            job: None,
            loading: false,
            confirmation: None,
            events: Vec::new(),
            state: EmulationState::Stopped,
            load_progress: 0.0,
            two_dimensional_view: false,
            emulated_frames: 0,
            quit_requested: false,
            scene_scale: 1.0,
            elapsed: 0.0,
            was_running_before_suspend: false,
            stage_clock: StageClock::default(),
            rollback: RollbackHistory::new(),
            rollback_enabled: true,
            separate_core: false,
            core_ahead: true,
            fast_forward: FastForwardState::new(),
            notifications: NotificationCenter::new(),
            slots,
            slot_patch_cache: std::cell::RefCell::default(),
            zapper: ZapperState::new(),
            catch_up: None,
            defer_conversion: false,
        };
        // AMD-035 §1: the language in force is the setting's, read once here and kept in step
        // by `on_setting_changed` afterwards. While none is saved, the model's default is the
        // operating-system language (set at start-up), amending AMD-035 §1.
        let language = host.settings.language();
        host.events.push(HostEvent::LanguageChanged(language));
        host.apply_speed();
        host.publish_state();
        host
    }

    // ------------------------------------------------------------------ accessors

    /// The state of the emulation.
    #[must_use]
    pub const fn state(&self) -> EmulationState {
        self.state
    }

    /// Whether a load is in progress (REQ-LOOP-020).
    #[must_use]
    pub const fn is_loading(&self) -> bool {
        self.loading
    }

    /// How far the meshes of the game being loaded are built, 0 to 1.
    #[must_use]
    pub const fn load_progress(&self) -> f32 {
        self.load_progress
    }

    /// The settings model.
    #[must_use]
    pub const fn settings(&self) -> &SettingsModel {
        &self.settings
    }

    /// Changes the settings with `edit`, then applies what changed (REQ-PERS-084): every effect
    /// of a change takes place before this returns, as if each write had notified its
    /// listeners.
    pub fn update_settings<R>(&mut self, edit: impl FnOnce(&mut SettingsModel) -> R) -> R {
        let result = edit(&mut self.settings);
        self.dispatch_setting_changes();
        result
    }

    /// The directory layout of the user data.
    #[must_use]
    pub const fn paths(&self) -> &UserDataPaths {
        &self.paths
    }

    /// The scene.
    #[must_use]
    pub const fn scene(&self) -> &SceneState {
        &self.scene
    }

    /// The scene, for writing.
    pub const fn scene_mut(&mut self) -> &mut SceneState {
        &mut self.scene
    }

    /// The library of games.
    #[must_use]
    pub const fn games(&self) -> &C {
        &self.games
    }

    /// The library of games, for writing.
    pub fn games_mut(&mut self) -> &mut C {
        &mut self.games
    }

    /// The program's audio mix.
    #[must_use]
    pub const fn mixer(&self) -> Option<&AudioMixer> {
        self.mixer.as_ref()
    }

    /// The program's audio mix, for writing.
    pub fn mixer_mut(&mut self) -> Option<&mut AudioMixer> {
        self.mixer.as_mut()
    }

    /// The settings and game list files.
    #[must_use]
    pub const fn settings_store(&self) -> Option<&SettingsStore> {
        self.settings_store.as_ref()
    }

    /// The settings and game list files, for writing.
    pub fn settings_store_mut(&mut self) -> Option<&mut SettingsStore> {
        self.settings_store.as_mut()
    }

    /// The session of the game being played.
    #[must_use]
    pub const fn session(&self) -> Option<&GameSession> {
        self.session.as_ref()
    }

    /// The session, for writing.
    pub fn session_mut(&mut self) -> Option<&mut GameSession> {
        self.session.as_mut()
    }

    /// The session and the scene together, for what draws the scene from the session.
    pub fn session_and_scene(&mut self) -> (Option<&mut GameSession>, &SceneState) {
        (self.session.as_mut(), &self.scene)
    }

    /// As [`Self::session_and_scene`], with the view of the game the scene is drawn into.
    pub fn session_scene_and_view(
        &mut self,
    ) -> (Option<&mut GameSession>, &SceneState, Option<&V>) {
        (self.session.as_mut(), &self.scene, self.view.as_ref())
    }

    /// The session for reading and the scene for writing, for what the session publishes to the
    /// scene.
    pub fn session_and_scene_mut(&mut self) -> (Option<&GameSession>, &mut SceneState) {
        (self.session.as_ref(), &mut self.scene)
    }

    /// The identifier of the game being played.
    #[must_use]
    pub fn current_game_id(&self) -> Option<&str> {
        self.current_game.as_ref().map(|game| game.id.as_str())
    }

    /// The title of the game being played.
    #[must_use]
    pub fn current_game_title(&self) -> Option<&str> {
        self.current_game.as_ref().map(|game| game.title.as_str())
    }

    /// Whether the hot 2D toggle is on (Q-037).
    #[must_use]
    pub const fn two_dimensional_view(&self) -> bool {
        self.two_dimensional_view
    }

    /// Whether the scene shows the flat picture: the toggle, or a game played flat.
    #[must_use]
    pub fn showing_two_dimensional(&self) -> bool {
        self.two_dimensional_view
            || self
                .session
                .as_ref()
                .is_some_and(GameSession::two_dimensional_mode)
    }

    /// Emulated frames run since the host was created.
    #[must_use]
    pub const fn emulated_frames(&self) -> i64 {
        self.emulated_frames
    }

    /// Whether the user asked to quit.
    #[must_use]
    pub const fn quit_requested(&self) -> bool {
        self.quit_requested
    }

    /// How many commands are waiting.
    #[must_use]
    pub fn pending_commands(&self) -> usize {
        self.pending.len()
    }

    /// Times the stages of the emulated frames from now on (the frame profiler) or stops.
    pub const fn set_profiling(&mut self, enabled: bool) {
        self.stage_clock.set_enabled(enabled);
    }

    /// The time of each Lua hook of each script of the game being played since the last call,
    /// the longest first. Turns their timing on or off (`timed`) first, so that a runtime made
    /// since the last call (a reloaded profile) follows.
    pub fn take_lua_hook_times(&mut self, timed: bool) -> Vec<HookTime> {
        let Some(session) = self.session.as_mut() else {
            return Vec::new();
        };
        session
            .with_runtime(&mut self.scene, |runtime, _| {
                let taken = runtime.take_hook_times();
                if runtime.keeps_hook_times() != timed {
                    runtime.set_hook_times(timed);
                }
                taken
            })
            .unwrap_or_default()
    }

    /// The stage times of the emulated frames since the last call: core, capture, conversion,
    /// Lua, rollback (all zero while the profiling is off).
    pub fn take_stage_times(&mut self) -> StageTimes {
        self.stage_clock.take()
    }

    /// The capture of the game view taken when the game was paused (REQ-LOOP-006).
    #[must_use]
    pub fn pause_thumbnail(&self) -> Option<&[u8]> {
        self.pause_thumbnail.as_deref()
    }

    /// The scale of the scene (1 on the desktop), which the Zapper aim divides by.
    pub const fn set_scene_scale(&mut self, scale: f32) {
        self.scene_scale = scale;
    }

    /// The device plugged into `port` of the game being played.
    #[must_use]
    pub fn port_type(&self, port: usize) -> zax_nes::ControllerPortType {
        self.session
            .as_ref()
            .map_or(zax_nes::ControllerPortType::Controller, |session| {
                session.port_type(port)
            })
    }

    /// Whether the pointer drives a Zapper (REQ-INP-061).
    #[must_use]
    pub fn zapper_active(&self) -> bool {
        routing::zapper_active(
            self.state == EmulationState::Running,
            self.port_type(0),
            self.port_type(1),
        )
    }

    /// Whether the interface holds a modal window.
    #[must_use]
    pub const fn modal_window_open(&self) -> bool {
        self.modal_window_open
    }

    /// Tells the host whether the interface holds a modal window (REQ-UI-004 cases 2 and 3). The
    /// loop says so before each frame.
    pub const fn set_modal_window_open(&mut self, open: bool) {
        self.modal_window_open = open;
    }

    /// The picture of the game the host captures its thumbnails from.
    #[must_use]
    pub const fn view(&self) -> Option<&V> {
        self.view.as_ref()
    }

    /// The same, for drawing into it.
    pub const fn view_mut(&mut self) -> Option<&mut V> {
        self.view.as_mut()
    }

    /// Takes the events raised since the last call.
    pub fn take_events(&mut self) -> Vec<HostEvent> {
        std::mem::take(&mut self.events)
    }

    /// The largest save state file this game can produce (REQ-ARCH-013), 0 without a ROM.
    #[must_use]
    pub fn max_save_state_size(&self) -> usize {
        self.session
            .as_ref()
            .filter(|session| session.has_rom())
            .map_or(0, |session| {
                crate::persistence::save_state_file::max_file_size(session.core().max_state_size())
            })
    }

    fn log(&self, message: &str) {
        (self.log)(message);
    }

    // ------------------------------------------------------------------ the loop

    /// Queues a command for the next drain (REQ-LOOP-007).
    pub fn enqueue(&mut self, command: Command, args: CommandArgs) {
        self.pending.push_back((command, args));
    }

    /// One displayed frame: the notifications' timer, the Zapper, the hotkeys, a step of the
    /// load or the queued commands and the emulated frames that are due. Returns how many
    /// emulated frames ran.
    pub fn tick(&mut self, real_seconds: f64, snapshot: &InputSnapshot) -> u32 {
        // The core the emulation thread held during the last drawing comes back before anything
        // of this tick can read or change it: finished, or only captured with emulation ahead.
        self.take_core_back();
        self.elapsed += real_seconds;
        // The notification timer is real time: neither the pause nor the game speed affect it
        // (REQ-UI-080).
        self.notifications.advance(real_seconds);
        // REQ-INP-061: the Zapper is refreshed once per displayed frame, before the hotkeys and
        // before any emulated frame reads its position.
        self.update_zapper(snapshot);
        self.read_hotkeys(snapshot, real_seconds);
        if self.loading {
            self.advance_load();
            self.dispatch_setting_changes();
            // REQ-LOOP-021 step 6: no emulated frame runs before the loading indicator clears.
            self.publish_state();
            return 0;
        }
        self.apply_commands();
        let ahead = self.runs_ahead();
        if ahead {
            // Emulation ahead: the core leaves during the frames, so what reads or changes it
            // runs before them.
            self.advance_battery(real_seconds);
            self.correct_audio_drift();
            self.dispatch_setting_changes();
        }
        let frames = self.run_due_frames(snapshot.player_one, snapshot.player_two);
        if ahead && !self.defers() {
            // A frame captured with nothing due since is converted all the same.
            if let Some(session) = self.session.as_mut() {
                session.finish_captured(&mut self.settings, &mut self.scene, &mut self.stage_clock);
            }
        } else {
            self.advance_battery(real_seconds);
            self.correct_audio_drift();
            self.dispatch_setting_changes();
        }
        if let Some(store) = &mut self.settings_store {
            store.tick(&self.settings, self.elapsed);
        }
        self.publish_state();
        // Last: the prepared frame runs on the emulation thread while the host draws.
        if let Some(session) = self.session.as_mut() {
            session.launch_prepared();
        }
        frames
    }

    /// Whether a tick may leave the conversion of its last frame to the caller
    /// ([`EmulationHost::defer_conversion`]): a game running with its frames ahead.
    #[must_use]
    pub fn can_defer_conversion(&self) -> bool {
        self.state == EmulationState::Running && self.runs_ahead()
    }

    /// The session, the settings and the scene for the conversion a tick left over, and the view;
    /// `None` without a session.
    pub fn deferred_parts(&mut self) -> Option<DeferredParts<'_, V>> {
        let session = self.session.as_mut()?;
        Some(DeferredParts {
            session,
            settings: &mut self.settings,
            scene: &mut self.scene,
            view: self.view.as_ref(),
        })
    }

    /// Whether this tick runs its frames ahead: the core on its own thread, emulation ahead on,
    /// and a profile whose scripts leave the core alone.
    fn runs_ahead(&self) -> bool {
        self.separate_core
            && self.core_ahead
            && self
                .session
                .as_ref()
                .is_some_and(GameSession::can_run_ahead)
    }

    /// Brings the core back from the emulation thread: with emulation ahead its frame is only
    /// captured (converted after the next frame has left), otherwise it is finished.
    fn take_core_back(&mut self) {
        if self.runs_ahead() {
            if let Some(session) = self.session.as_mut() {
                session.capture_in_flight(&mut self.stage_clock);
            }
        } else {
            self.complete_frame_in_flight();
        }
    }

    /// The catch-up in force: the forced one, else the `emu.catch_up` setting.
    #[must_use]
    pub fn effective_catch_up(&self) -> CatchUp {
        self.catch_up.unwrap_or_else(|| {
            CatchUp::from_setting(self.settings.get(SettingId::EmuCatchUp).as_int())
        })
    }

    /// Finishes the frame the emulation thread holds or that was prepared for it, if any.
    pub fn complete_frame_in_flight(&mut self) {
        if let Some(session) = self.session.as_mut() {
            session.complete_frame_timed(
                &mut self.settings,
                &mut self.scene,
                &mut self.stage_clock,
            );
        }
    }

    /// A tick carrying only the two players' actions.
    pub fn tick_actions(
        &mut self,
        real_seconds: f64,
        player_one: crate::input::InputAction,
        player_two: crate::input::InputAction,
    ) -> u32 {
        self.tick(real_seconds, &InputSnapshot::of(player_one, player_two))
    }

    /// Runs the queued commands. A command that queues another one leaves it for the next
    /// drain, which keeps the first-in first-out order of REQ-LOOP-007 observable. Returns how
    /// many commands ran.
    pub fn apply_commands(&mut self) -> usize {
        if self.loading {
            return 0;
        }
        let mut budget = self.pending.len();
        let mut ran = 0;
        while budget > 0 && !self.loading {
            budget -= 1;
            let Some((command, args)) = self.pending.pop_front() else {
                break;
            };
            ran += 1;
            self.execute(command, &args);
        }
        self.dispatch_setting_changes();
        self.publish_state();
        ran
    }

    /// Runs the emulated frames that are due now.
    pub fn run_due_frames(
        &mut self,
        player_one: crate::input::InputAction,
        player_two: crate::input::InputAction,
    ) -> u32 {
        if self.state != EmulationState::Running
            || !self.session.as_ref().is_some_and(GameSession::has_rom)
        {
            return 0;
        }
        self.apply_speed();
        let displayed_rate = f64::from(
            self.settings
                .get(SettingId::VideoTargetFramerate)
                .as_float(),
        );
        self.pacer.max_backlog = self
            .effective_catch_up()
            .frames_per_display(self.pacer.effective_rate(), displayed_rate);
        let due = self.pacer.due_frames();
        let ahead = self.runs_ahead();
        for round in 0..due {
            // The frame of the previous round comes back first.
            self.take_core_back();
            let Some(session) = self.session.as_mut() else {
                break;
            };
            // REQ-LOOP-030: the snapshot is taken at the start of a frame whose core frame
            // counter is a multiple of 60, so before that frame executes.
            let core = session.core();
            if self.rollback_enabled && RollbackHistory::is_snapshot_frame(core.frame_count()) {
                let started = self.stage_clock.start();
                // A failure (no ROM) leaves the history as it was, as before.
                let _ = self
                    .rollback
                    .capture_with(|buffer| core.save_state_to(false, buffer));
                self.stage_clock.stop(Stage::Rollback, started);
            }
            if ahead {
                // The accounting touches the core: it comes before the frame leaves.
                self.count_emulated_frame();
                // The last frame of a pipelined tick is converted by the caller.
                let convert = round + 1 < due || !self.defers();
                self.launch_ahead_frame(player_one, player_two, convert);
                continue;
            }
            // REQ-INP-005: one emulated frame is one sample, fast forward included.
            if self.separate_core {
                self.prepare_session_frame(player_one, player_two);
            } else {
                self.run_session_frame(player_one, player_two);
            }
            self.count_emulated_frame();
        }
        if !ahead {
            self.dispatch_setting_changes();
        }
        due
    }

    /// The accounting of one emulated frame: the count, and the fast-forward burst.
    fn count_emulated_frame(&mut self) {
        self.emulated_frames += 1;
        if self.fast_forward.is_active() {
            self.fast_forward.on_frame_run();
            if !self.fast_forward.is_active() {
                // The factor goes back to the setting as soon as the burst ends, so a held key
                // re-arms without a visible step (REQ-LOOP-016).
                self.apply_speed();
            }
        }
    }

    /// Whether this tick leaves its last conversion to the caller.
    fn defers(&self) -> bool {
        self.defer_conversion && self.runs_ahead()
    }

    /// Emulation ahead: the next frame leaves for the emulation thread, then the frame captured
    /// before it is converted here while it runs, when `convert`.
    fn launch_ahead_frame(
        &mut self,
        player_one: crate::input::InputAction,
        player_two: crate::input::InputAction,
        convert: bool,
    ) {
        let port_byte = self.zapper.port_byte();
        let position = self.zapper.position();
        if let Some(session) = self.session.as_mut() {
            let frame = SessionFrame {
                player_one,
                player_two,
                zapper_byte: port_byte,
                zapper_position: position,
            };
            session.launch_ahead(
                &mut self.settings,
                &mut self.scene,
                frame,
                &mut self.stage_clock,
            );
            if convert {
                session.finish_captured(&mut self.settings, &mut self.scene, &mut self.stage_clock);
            }
        }
    }

    /// Runs one emulated frame of the session right now, outside the frame clock: no rollback
    /// snapshot, no accounting. The input is the sample of that frame.
    pub fn run_session_frame(
        &mut self,
        player_one: crate::input::InputAction,
        player_two: crate::input::InputAction,
    ) {
        self.complete_frame_in_flight();
        let port_byte = self.zapper.port_byte();
        let position = self.zapper.position();
        if let Some(session) = self.session.as_mut() {
            let frame = SessionFrame {
                player_one,
                player_two,
                zapper_byte: port_byte,
                zapper_position: position,
            };
            session.run_frame_timed(
                &mut self.settings,
                &mut self.scene,
                frame,
                &mut self.stage_clock,
            );
        }
    }

    /// Prepares one emulated frame for the emulation thread ([`EmulationHost::separate_core`]):
    /// the scripts' input hook and the input word now, the frame itself at the end of the tick.
    fn prepare_session_frame(
        &mut self,
        player_one: crate::input::InputAction,
        player_two: crate::input::InputAction,
    ) {
        let port_byte = self.zapper.port_byte();
        let position = self.zapper.position();
        if let Some(session) = self.session.as_mut() {
            let frame = SessionFrame {
                player_one,
                player_two,
                zapper_byte: port_byte,
                zapper_position: position,
            };
            session.prepare_frame_timed(
                &mut self.settings,
                &mut self.scene,
                frame,
                &mut self.stage_clock,
            );
        }
    }

    /// Stops the current game, writes the slot index and the settings (REQ-LOOP-052).
    pub fn shutdown(&mut self) {
        self.complete_frame_in_flight();
        self.stop_current_game(true, true);
        if self.slots.save(&self.paths).is_err() {
            self.log("the save slot index could not be written.");
        }
        if let Some(store) = &mut self.settings_store {
            store.save(&self.settings);
        }
    }

    // ------------------------------------------------------------------ states

    fn set_state(&mut self, next: EmulationState) {
        if self.state == next {
            return;
        }
        // REQ-LOOP-006: the capture of the game view is taken before the menu appears.
        if self.state == EmulationState::Running && next == EmulationState::Paused {
            if let Some(capture) = self.view.as_mut().and_then(GameView::capture_thumbnail) {
                self.pause_thumbnail = Some(capture);
            }
            // REQ-PERS-036, third source: the cover of the "2D Mode" entry follows the pause
            // capture every time the menu appears.
            self.refresh_two_dimensional_cover();
        }
        self.state = next;
        self.publish_state();
    }

    fn publish_state(&mut self) {
        // REQ-SCN-047 wants the 3D scene hidden outside a running game. With the classic menu
        // bar of AMD-010 there is no opaque menu covering it, so the scene stays visible while
        // the game is paused, the frozen frame a desktop emulator shows. Q-029.
        self.scene.game_running = self.state != EmulationState::Stopped;
        self.load_progress = self.session.as_ref().map_or(
            if self.state == EmulationState::Stopped {
                0.0
            } else {
                1.0
            },
            |session| session.engine().load_progress(),
        );
        if let Some(mixer) = &mut self.mixer {
            // REQ-AUD-010 and REQ-AUD-020: the console is audible only in Running, and the menu
            // music is in its active phase in every other state.
            mixer.set_emulation_audible(self.state == EmulationState::Running);
            mixer.set_volume(self.settings.effective_volume());
            mixer.set_menu_music_enabled(self.settings.menu_music());
        }
    }

    fn apply_speed(&mut self) {
        // REQ-LOOP-016: during a burst the factor is 1.5 whatever the setting says, which means
        // a setting above +2 is slowed down by fast forward. That is faithful (DEC-07-Q2).
        let factor = if self.fast_forward.is_active() {
            speed_setting::fast_forward_factor()
        } else {
            self.settings.speed_factor()
        };
        self.pacer.speed = factor;
        if let Some(session) = &mut self.session {
            self.pacer.nominal_rate = f64::from(session.nominal_frame_rate());
            // REQ-AUD-011: the audio follows k by time compression with a proportional pitch
            // change; the core retimes its resampler, it never drops samples for that.
            if session.core_mut().set_speed(factor).is_err() {
                self.log("the core refused the speed factor.");
            }
        }
    }

    // ------------------------------------------------------------------ settings

    /// Applies what changed in the settings since the last call: the effects the host owns, the
    /// mirror into the engine and the scene, and the events for the interface. Effects can
    /// change settings in turn, so it runs until nothing is left.
    pub fn dispatch_setting_changes(&mut self) {
        self.keep_game_options_edits();
        for _ in 0..8 {
            let ids = self.collect_setting_changes();
            if ids.is_empty() {
                return;
            }
            for id in ids {
                self.on_setting_changed(id);
            }
        }
    }

    fn collect_setting_changes(&mut self) -> Vec<SettingId> {
        let mut ids = Vec::new();
        if let Some(session) = self.session.as_mut() {
            // The runtime drains the model's changes, mirrors them into the engine and
            // reports them back.
            for event in session.sync_settings(&mut self.settings, &mut self.scene) {
                match event {
                    SessionEvent::SettingChanged(id) => ids.push(id),
                    SessionEvent::SaveSettingsRequested => self.save_settings_file(),
                    SessionEvent::ReloadSettingsRequested => self.reload_settings_file(),
                }
            }
        }
        ids.extend(self.settings.drain_changes().map(|change| change.id));
        ids
    }

    fn save_settings_file(&mut self) {
        if let Some(store) = &mut self.settings_store {
            store.save(&self.settings);
        }
    }

    fn reload_settings_file(&mut self) {
        let now = (self.now)();
        if let Some(store) = &mut self.settings_store {
            store.load(&mut self.settings, now);
        }
    }

    fn on_setting_changed(&mut self, id: SettingId) {
        if let Some(store) = &mut self.settings_store {
            store.on_setting_changed(&self.settings, id);
        }
        match id {
            SettingId::EmuGameSpeed => self.apply_speed(),
            SettingId::EmuLuaSmoothGc => {
                let smooth = self.settings.get(id).as_bool();
                if let Some(session) = &self.session {
                    session.set_smooth_gc(smooth);
                }
            }
            SettingId::EmuTvMode => {
                // REQ-LOOP-017: a region change is applied at once, to the core and to the
                // cadence, even in the middle of a game.
                if let Some(session) = self.session.as_mut().filter(|session| session.has_rom()) {
                    session.core_mut().set_region(self.settings.region());
                }
                self.apply_speed();
            }
            SettingId::AudioVolume | SettingId::AudioMute | SettingId::AudioBgMusic => {
                self.publish_state();
            }
            SettingId::UiLanguage => {
                // AMD-035 §1: the change applies immediately. Everything the interface draws
                // reads the table through the language, and the models that cache their labels
                // watch its generation counter.
                self.events
                    .push(HostEvent::LanguageChanged(self.settings.language()));
            }
            SettingId::VideoSkipFrame => {
                self.scene.settings.skip_render = self.settings.skip_frame();
            }
            SettingId::VideoFrameRate => {
                self.scene
                    .settings
                    .set_target_frame_rate(self.settings.frame_rate());
            }
            SettingId::VideoVSync => self.scene.settings.set_vsync(self.settings.vsync()),
            SettingId::VideoAntiAlias => {
                // AMD-032 §2: the setting takes effect on the next rendered frame, the renderer
                // reallocating its multisampled target by itself. No restart.
                self.scene
                    .settings
                    .set_anti_alias_samples(render_geometry::anti_alias_samples(
                        self.settings.anti_alias(),
                    ));
            }
            _ => {}
        }
        // The fov, shadows, skybox, local camera and light follow the model into the scene
        // (REQ-SCN-009), whoever changed them.
        scene_script_bridge::apply_setting(id, &self.settings, &mut self.scene);
        self.events.push(HostEvent::SettingChanged(id));
    }

    // ------------------------------------------------------------------ Zapper and hotkeys

    fn update_zapper(&mut self, snapshot: &InputSnapshot) {
        if !self.zapper_active() {
            // In pause, or with no Zapper port, moving the mouse changes nothing of what the
            // core sees (REQ-INP-061).
            return;
        }
        let aspect = if snapshot.pointer_size[1] > 0.0 {
            snapshot.pointer_size[0] / snapshot.pointer_size[1]
        } else {
            1.0
        };
        let view = {
            let scene = &self.scene;
            scene.camera.build_view(scene.view_clipping(), aspect)
        };
        let offset = self
            .session
            .as_ref()
            .map_or(zax_render::numerics::Vector3::ZERO, |session| {
                session.zapper_offset().into()
            });
        self.zapper.update(
            true,
            &ZapperInput {
                view: &view,
                pointer: snapshot.pointer,
                size: snapshot.pointer_size,
                right_button: snapshot.right_button,
                offset,
                scene_scale: self.scene_scale,
            },
        );
    }

    #[allow(clippy::cast_possible_truncation)] // the C# `(float)realSeconds`
    fn read_hotkeys(&mut self, snapshot: &InputSnapshot, real_seconds: f64) {
        let events = self.hotkeys.update(
            snapshot.player_one,
            snapshot.player_two,
            real_seconds as f32,
        );
        if events.contains(HotkeyEvents::MENU_LONG) && !self.loading {
            // REQ-INP-101 and REQ-UI-005: a long press resets the view without changing the
            // pause state, and is inactive while the loading indicator is up.
            self.enqueue(Command::ResetView, CommandArgs::default());
        }
        if events.contains(HotkeyEvents::MENU_SHORT) && !self.loading {
            // REQ-INP-101 and REQ-UI-004 case 1: a Menu press made during a load is ignored,
            // not deferred. REQ-LOOP-007 defers the commands that are already in the queue,
            // which is right; the point here is that this one is never issued at all, so the
            // game does not pause the instant the load ends.
            //
            // Cases 2 and 3: a modal window takes the press before the emulation does. What it
            // does with it belongs to the frontend: the browser closes as with "Cancel", a
            // confirmation dialog swallows it (REQ-UI-110).
            let command = if self.modal_window_open() {
                Command::CancelModal
            } else {
                Command::PauseResume
            };
            self.enqueue(command, CommandArgs::default());
        }
        for (flag, command) in [
            (HotkeyEvents::ROLLBACK, Command::Rollback),
            (HotkeyEvents::FAST_FORWARD, Command::FastForward),
            (HotkeyEvents::QUICK_SAVE, Command::QuickSave),
            (HotkeyEvents::QUICK_LOAD, Command::QuickLoad),
        ] {
            if events.contains(flag) {
                self.enqueue(command, CommandArgs::default());
            }
        }
    }

    // ------------------------------------------------------------------ housekeeping

    fn correct_audio_drift(&mut self) {
        const TARGET: usize = 2 * crate::audio::audio_mixer::BLOCK;
        if !self.audio_device_open || self.state != EmulationState::Running {
            return;
        }
        let Some(session) = self.session.as_mut().filter(|session| session.has_rom()) else {
            return;
        };
        let available = session.core().apu().available_samples();
        #[allow(clippy::cast_precision_loss)] // sample counts are far below 2^52
        let error = (available as f64 - TARGET as f64) / TARGET as f64;
        // A proportional correction that saturates at the cap: a full buffer slows the
        // production down, an empty one speeds it up.
        let cap = zax_nes::apu::Apu2A03::MAX_RATIO_ADJUSTMENT;
        let adjustment = (-error * 0.002).clamp(-cap, cap);
        if session
            .core_mut()
            .set_audio_ratio_adjustment(adjustment)
            .is_err()
        {
            self.log("the core refused the audio ratio adjustment.");
        }
    }

    fn advance_battery(&mut self, real_seconds: f64) {
        let Some(session) = self.session.as_mut().filter(|session| session.has_rom()) else {
            return;
        };
        // REQ-PERS-060: written at most every 30 s, and only when the content changed.
        let core = session.core();
        self.battery
            .advance(&self.paths, real_seconds, core.export_battery_ram());
    }

    /// Stops the game being played: the cartridge RAM is written (REQ-PERS-060), the `End`
    /// hooks run and the scene is emptied. With `clear_current_game` the game is forgotten and
    /// the state goes to Stopped; with `dispose_session` the session is dropped.
    fn stop_current_game(&mut self, clear_current_game: bool, dispose_session: bool) {
        if let Some(mut session) = self.session.take() {
            {
                let core = session.core();
                self.battery.flush(&self.paths, core.export_battery_ram());
            }
            session.end(&mut self.settings, &mut self.scene);
            session.engine_mut().clear_frames();
            if dispose_session {
                session.dispose(&mut self.settings, &mut self.scene);
                self.battery.detach();
                if let Some(mixer) = &mut self.mixer {
                    mixer.set_emulation_source(None);
                }
            } else {
                self.session = Some(session);
            }
        }
        if clear_current_game {
            self.current_game = None;
            self.game_options_rom = None;
            self.pause_thumbnail = None;
            self.rollback.clear();
            self.fast_forward.clear();
            // REQ-PERS-012: the speed setting goes back to 0 at the end of a game.
            self.settings.end_of_game();
            // REQ-SCN-013: the end of a game is a view-reset trigger, and it must in addition
            // go back to TPV and put the smoothing time back to 0.4 s. `end_game` does all
            // three.
            self.scene.camera.end_game();
            self.set_state(EmulationState::Stopped);
            self.dispatch_setting_changes();
        }
    }
}
