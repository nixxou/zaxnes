//! One game being played: the core, the conversion engine, the Lua runtime and the bridges
//! between them (SPEC-07 §3.1, SPEC-05 REQ-LUA-015).
//!
//! # Ownership
//!
//! The settings model belongs to the host. The Lua runtime keeps a model of its own inside
//! `Setting`, so the session lends the host's model to the runtime for the length of each call
//! that can run a script (`lend`), by exchanging it with a placeholder, and gives it back
//! afterwards.
//!
//! The session owns the core and the script inputs; the host owns the scene and passes it to
//! every call that can run a script. The runtime keeps none of them: each call into the
//! conversion engine that can reach a hook borrows them for its length (`scripted`), through
//! `zax_scripting::lend_context`, so the compiler rules out a conflicting access.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};

use zax_engine3d::draw_list::FrameDrawList;
use zax_engine3d::engine::ConversionEngine;
use zax_engine3d::error::EngineError;
use zax_engine3d::palette::Rgb24;
use zax_engine3d::settings::RenderAlgorithm;
use zax_engine3d::vectors::Vec3;
use zax_nes::movie::{Movie, MovieRun};
use zax_nes::rom::RomLoadResult;
use zax_nes::{ControllerPortType, FrameBatch, NesBus, NesCore, TileRecord, ZapperPosition};
use zax_profiles::profile::GameProfile;
use zax_profiles::setting_application;
use zax_render::scene::SceneState;
use zax_scripting::{
    lend_context, LuaScriptError, ProfileScriptRuntime, ScriptContext, ScriptEnvironment,
    ScriptInput, ScriptLight, ScriptLog, ScriptMemory, ScriptProfileInfo, ScriptSettings,
};
use zax_settings::catalogue::{Port, SettingId};
use zax_settings::model::SettingsModel;

use crate::core_worker::CoreWorker;
use crate::frame_profiler::{Stage, StageClock, Thread};
use crate::input::input_action::InputAction;
use crate::input::routing;
use crate::scene_script_bridge::{self, SceneBackdropBridge, SceneCameraBridge};

/// Why a session could not be created.
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    /// The Lua runtime could not be created.
    #[error(transparent)]
    Script(#[from] LuaScriptError),
    /// The profile cannot be loaded by the engine.
    #[error(transparent)]
    Engine(#[from] EngineError),
}

/// What a session reports to the host after the scripts ran.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionEvent {
    /// A setting changed, whoever changed it.
    SettingChanged(SettingId),
    /// `Setting:Save()`: write the settings file (REQ-PERS-015).
    SaveSettingsRequested,
    /// `Setting:Load()`: read the settings file again (REQ-PERS-015).
    ReloadSettingsRequested,
}

/// Tells the session an event happened. A closed channel means the session is being dropped, so
/// nobody is left to tell.
fn notify(sender: &Sender<SessionEvent>, event: SessionEvent) {
    if sender.send(event).is_err() {
        // The session, and with it the runtime that calls this, is going away.
    }
}

/// What the scripts read of the inputs and write of the Zapper aim.
#[derive(Debug, Default)]
struct ScriptInputState {
    players: [i32; 2],
    zapper_offset: Vec3,
}

/// The machine as scripts see it, borrowed for one call into the engine: the players' action
/// masks, the ports and the CPU address space of the core, and the Zapper aim offset.
struct CoreMachine<'a> {
    core: &'a mut NesCore,
    input: &'a mut ScriptInputState,
    /// The console RAM of the frame being converted while the core runs the next one
    /// (emulation ahead); `None` reads the core itself.
    snapshot: Option<&'a MemorySnapshot>,
}

/// The console RAM as it stood at the end of a captured frame, which the scripts' `ReadMem`
/// reads while the core is away running the next frame: the same bytes a read of the core would
/// have given right after that frame.
struct MemorySnapshot {
    ram: [u8; NesBus::RAM_SIZE],
    /// Whether a read asked for something the copy does not hold (cartridge space).
    missed: AtomicBool,
}

impl Default for MemorySnapshot {
    fn default() -> Self {
        Self {
            ram: [0; NesBus::RAM_SIZE],
            missed: AtomicBool::new(false),
        }
    }
}

impl MemorySnapshot {
    /// What `NesCore::read_memory` would read: the RAM and its mirrors, 0 for $2000-$401F
    /// (REQ-CORE-081); the cartridge space is not held, which `missed` records.
    fn read(&self, address: u16) -> u8 {
        if address < 0x2000 {
            return self.ram[usize::from(address) & (NesBus::RAM_SIZE - 1)];
        }
        if address < 0x4020 {
            return 0;
        }
        self.missed.store(true, Ordering::Relaxed);
        0
    }
}

impl ScriptInput for CoreMachine<'_> {
    fn player_state(&self, index: usize) -> i32 {
        self.input.players.get(index).copied().unwrap_or(0)
    }

    fn global_state(&self) -> i32 {
        let [one, two] = self.input.players;
        one | two
    }

    fn port_byte(&self, port: usize) -> i32 {
        if port < 2 {
            i32::from(self.core.bus().input().port_byte(port))
        } else {
            0
        }
    }

    fn port_type(&self, port: usize) -> i32 {
        self.core.port_type(port).map_or(0, |kind| kind as i32)
    }

    fn set_port_types(&mut self, first: i32, second: i32) {
        for (port, kind) in [first, second].into_iter().enumerate() {
            let kind = if kind.clamp(0, 1) == 1 {
                ControllerPortType::Zapper
            } else {
                ControllerPortType::Controller
            };
            if self.core.set_port_type(port, kind).is_err() {
                // Both ports exist, so this cannot happen; the type is simply not changed.
            }
        }
    }

    fn zapper_offset(&self) -> Vec3 {
        self.input.zapper_offset
    }

    fn set_zapper_offset(&mut self, offset: Vec3) {
        self.input.zapper_offset = offset;
    }
}

impl ScriptMemory for CoreMachine<'_> {
    fn read(&self, address: u16) -> u8 {
        match self.snapshot {
            Some(snapshot) => snapshot.read(address),
            None => self.core.read_memory(address),
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        self.core.write_memory(address, value);
    }
}

/// Runs `operation` with the scene, the core and the script inputs lent to the script API.
fn with_context<R>(
    scene: &mut SceneState,
    core: &mut NesCore,
    input: &mut ScriptInputState,
    snapshot: Option<&MemorySnapshot>,
    operation: impl FnOnce(&mut ScriptContext<'_>) -> R,
) -> R {
    let mut camera = SceneCameraBridge(&mut scene.camera);
    let mut backdrop = SceneBackdropBridge(&mut scene.backdrop);
    let mut machine = CoreMachine {
        core,
        input,
        snapshot,
    };
    operation(&mut ScriptContext {
        camera: &mut camera,
        backdrop: &mut backdrop,
        machine: &mut machine,
    })
}

/// What `GameSession::create` needs to know besides the profile.
pub struct SessionRequest<'a> {
    /// The profile to play.
    pub profile: &'a GameProfile,
    /// The title to show when the profile has none.
    pub title: &'a str,
    /// The scene the camera and backdrop bridges drive.
    pub scene: &'a mut SceneState,
    /// Where `print` and the script errors go.
    pub log: Box<dyn ScriptLog>,
    /// Whether the profile's settings are applied to the model: true for a different game
    /// (REQ-PERS-005), false when the same game is relaunched (REQ-PERS-006).
    pub different_game: bool,
    /// Whether the game is played flat; `None` means "when the profile has no pattern".
    pub two_dimensional_mode: Option<bool>,
}

/// The inputs of one emulated frame.
#[derive(Debug, Clone, Copy)]
pub struct SessionFrame {
    /// The first player's actions.
    pub player_one: InputAction,
    /// The second player's actions.
    pub player_two: InputAction,
    /// The Zapper byte of its port.
    pub zapper_byte: u8,
    /// Where the Zapper aims.
    pub zapper_position: ZapperPosition,
}

/// What [`GameSession::capture_batch`] keeps of a frame's batch besides its records: palette,
/// edge masks, widescreen margin and visible rows.
type CapturedBatch = ([u8; 32], i32, i32, (i32, zax_nes::VisibleRows, i32));

/// One game being played.
pub struct GameSession {
    core: Box<NesCore>,
    engine: ConversionEngine,
    input: ScriptInputState,
    events: Receiver<SessionEvent>,
    title: String,
    frame_counter: i32,
    started: bool,
    aimed: Option<ZapperPosition>,
    batch_records: Vec<TileRecord>,
    pipeline: FramePipeline,
    /// `video.widescreen_strict_edges`: whether the engine is told where each band of a
    /// widescreen frame stops being drawn, so that a shape cut there stays unfinished.
    strict_wide_edges: bool,
    /// The movie being recorded or played (see `zax_nes::movie`).
    movie: Option<MovieRun>,
}

/// The state of the "emulation on a separate core" setting (Rust-only, handhelds): the next
/// frame is prepared (input hook, input word) during the host's tick and runs on the emulation
/// thread while the main thread draws the frame before it; the next tick takes the core back
/// and finishes the frame (tile capture, conversion, frame hooks) before anything else.
#[derive(Default)]
struct FramePipeline {
    worker: Option<CoreWorker>,
    /// A core without a ROM that stands in `GameSession::core` while the real one is away.
    spare: Option<Box<NesCore>>,
    /// The input word of the prepared frame that has not started yet.
    prepared: Option<u16>,
    /// Whether the emulation thread holds the core.
    in_flight: bool,
    /// The picture of the last finished frame, kept while the core is away.
    picture: Vec<u8>,
    /// Its rows.
    picture_height: usize,
    /// Whether the profile's scripts leave the core alone (no input hook, no memory or port
    /// access), so that the next frame may run while the scripts of the last one do
    /// ([`GameSession::launch_ahead`]).
    ahead_allowed: bool,
    /// The palette and edge masks of a frame captured but not converted yet; its tile records
    /// are in `GameSession::batch_records`.
    pending: Option<CapturedBatch>,
    /// The RAM of the captured frame, for the scripts' `ReadMem` while the core is away.
    memory: MemorySnapshot,
}

/// The members of the script API that read or drive the core: a profile whose scripts name
/// none of them cannot tell whether the next frame has already started. `ReadMem` is not one:
/// it reads the RAM copied when the frame came back ([`MemorySnapshot`]).
const CORE_MEMBERS: [&str; 6] = [
    "InputProcess",
    "WriteMem",
    "GetInput",
    "GetInputType",
    "SetInputType",
    "ZapperEnable",
];

/// Whether no script of `profile` names a member of [`CORE_MEMBERS`].
fn scripts_leave_the_core_alone(profile: &GameProfile) -> bool {
    profile.scripts.iter().all(|script| {
        script
            .source
            .as_deref()
            .is_none_or(|source| CORE_MEMBERS.iter().all(|member| !source.contains(member)))
    })
}

impl GameSession {
    /// Creates the session of a profile: the Lua chunks run first (REQ-LUA-015 step 1), then the
    /// profile's settings reach the model (step 2), then the patterns and their `BuildPattern`
    /// hooks (step 3). `model` is the host's settings model.
    ///
    /// # Errors
    /// The runtime could not be installed, or a pattern of the profile is invalid.
    pub fn create(
        request: SessionRequest<'_>,
        model: &mut SettingsModel,
    ) -> Result<Self, SessionError> {
        let SessionRequest {
            profile,
            title,
            scene,
            log,
            different_game,
            two_dimensional_mode,
        } = request;
        let mut core = NesCore::new();
        let mut input = ScriptInputState::default();
        let mut engine = ConversionEngine::new();
        engine.use_geometry();
        engine.two_dimensional_mode = two_dimensional_mode.unwrap_or(
            profile.permanent_patterns.is_empty() && profile.temporary_patterns.is_empty(),
        );
        let (sender, events) = channel();
        let optimized = model.get(SettingId::EmuLuaOptimized).as_bool();
        let mut settings = ScriptSettings::new(
            std::mem::replace(model, SettingsModel::new()),
            ScriptLight::Model,
        );
        let changes = sender.clone();
        settings.set_on_changed(move |id| notify(&changes, SessionEvent::SettingChanged(id)));
        let saves = sender.clone();
        settings.set_on_save_requested(move || notify(&saves, SessionEvent::SaveSettingsRequested));
        settings
            .set_on_load_requested(move || notify(&sender, SessionEvent::ReloadSettingsRequested));
        // The scene starts from the settings as they stand; the profile's own then reach it
        // through the changes the model reports.
        scene_script_bridge::push_settings(settings.model(), scene);
        let environment = ScriptEnvironment {
            settings,
            log,
            profile: ScriptProfileInfo {
                title: profile.title.clone().unwrap_or_else(|| title.to_owned()),
                maker: profile.author.clone().unwrap_or_default(),
                game_dev: profile.publisher.clone().unwrap_or_default(),
                description: profile.description.clone().unwrap_or_default(),
                id: profile.id.clone().unwrap_or_default(),
                version: profile.version_text.clone().unwrap_or_default(),
            },
            optimized,
            ..ScriptEnvironment::default()
        };
        // REQ-LUA-015 step 1: every chunk runs before anything else, with the scene and the
        // machine at hand.
        let runtime = with_context(scene, &mut core, &mut input, None, |context| {
            ProfileScriptRuntime::create_in(profile, environment, engine.state_mut(), Some(context))
        })?;
        engine.set_script_host(Box::new(runtime));
        let mut session = Self {
            core: Box::new(core),
            engine,
            input,
            events,
            title: title.to_owned(),
            frame_counter: 0,
            started: false,
            aimed: None,
            batch_records: Vec::new(),
            strict_wide_edges: false,
            movie: None,
            pipeline: FramePipeline::default(),
        };
        session.pipeline.ahead_allowed = scripts_leave_the_core_alone(profile);
        // The runtime holds the model from here until the last line of this function.
        let loaded = session.load_profile(profile, different_game, scene);
        session.exchange_model(model);
        loaded?;
        Ok(session)
    }

    /// REQ-LUA-015 step 2, REQ-LOOP-021 step 2 and REQ-PERS-005: the profile's P settings land in
    /// the model, which mirrors them into the engine slice and into the scene. This runs after
    /// the chunks, so an assignment made at the main level of a chunk is overwritten, as
    /// REQ-LUA-015 demands. Relaunching the same game skips it, so the scripts' changes survive
    /// (REQ-PERS-006).
    fn load_profile(
        &mut self,
        profile: &GameProfile,
        different_game: bool,
        scene: &mut SceneState,
    ) -> Result<(), SessionError> {
        let ports = self
            .engine
            .with_script_host::<ProfileScriptRuntime, _>(|runtime, _| {
                if different_game {
                    let values = profile
                        .settings
                        .as_ref()
                        .map(setting_application::to_setting_values)
                        .unwrap_or_default();
                    runtime.settings_mut().model_mut().apply_profile(values);
                }
                // Both ports go back to Gamepad at every load of a different game (REQ-PERS-012,
                // DEC-09-Q6); the input layer stays the authority and takes its types from the model.
                let model = runtime.settings().model();
                [
                    model.get(SettingId::InputPortType(Port::ALL[0])).as_int(),
                    model.get(SettingId::InputPortType(Port::ALL[1])).as_int(),
                ]
            });
        if let Some([first, second]) = ports {
            CoreMachine {
                core: &mut self.core,
                input: &mut self.input,
                snapshot: None,
            }
            .set_port_types(first, second);
        }
        // Step 3: the patterns with their BuildPattern hooks. The settings were applied above,
        // through the model, so the engine must not copy them a second time.
        self.sync_runtime_settings(scene);
        let mut loaded = Ok(());
        self.scripted(scene, |engine| {
            loaded = engine.load_profile(profile, false);
        });
        loaded?;
        self.sync_runtime_settings(scene);
        Ok(())
    }

    /// Runs `operation` on the engine with the scene, the core and the script inputs lent to the
    /// script API for its length. The borrows end with the call: the runtime keeps none.
    fn scripted(&mut self, scene: &mut SceneState, operation: impl FnOnce(&mut ConversionEngine)) {
        let Self {
            core,
            input,
            engine,
            pipeline,
            ..
        } = self;
        // While the core is away, the scripts read the RAM copied when their frame came back.
        let snapshot = pipeline.in_flight.then_some(&pipeline.memory);
        with_context(scene, core, input, snapshot, |context| {
            lend_context(engine, |engine| engine, context, operation);
        });
    }

    /// Runs `operation` on the Lua runtime and the conversion state it works on, or returns `None`
    /// when the session has no runtime. The scripts it runs see `scene` and the core. Used by the
    /// acceptance harness to evaluate chunks; the settings model is not lent, so what a chunk
    /// writes to the settings stays in the runtime.
    pub fn with_runtime<R>(
        &mut self,
        scene: &mut SceneState,
        operation: impl FnOnce(
            &mut ProfileScriptRuntime,
            &mut zax_engine3d::engine::ConversionState,
        ) -> R,
    ) -> Option<R> {
        let mut result = None;
        self.scripted(scene, |engine| {
            result = engine.with_script_host::<ProfileScriptRuntime, R>(operation);
        });
        result
    }

    fn sync_runtime_settings(&mut self, scene: &mut SceneState) {
        self.with_runtime(scene, ProfileScriptRuntime::sync_settings);
    }

    fn exchange_model(&mut self, model: &mut SettingsModel) {
        self.engine
            .with_script_host::<ProfileScriptRuntime, _>(|runtime, _| {
                std::mem::swap(runtime.settings_mut().model_mut(), model);
            });
    }

    /// Lends the host's `model` to the Lua runtime for the length of `operation`, then mirrors
    /// what the scripts changed and gives the model back.
    fn lend<R>(
        &mut self,
        model: &mut SettingsModel,
        scene: &mut SceneState,
        operation: impl FnOnce(&mut Self, &mut SceneState) -> R,
    ) -> R {
        self.exchange_model(model);
        let result = operation(self, scene);
        self.sync_runtime_settings(scene);
        self.exchange_model(model);
        result
    }

    /// The core, for reading.
    #[must_use]
    pub fn core(&self) -> &NesCore {
        &self.core
    }

    /// The core, for writing.
    #[must_use]
    pub fn core_mut(&mut self) -> &mut NesCore {
        &mut self.core
    }

    /// Sets `video.widescreen_strict_edges` (see the field `strict_wide_edges`).
    pub fn set_strict_wide_edges(&mut self, strict: bool) {
        self.strict_wide_edges = strict;
    }

    /// The conversion engine.
    #[must_use]
    pub const fn engine(&self) -> &ConversionEngine {
        &self.engine
    }

    /// The conversion engine, for writing.
    pub fn engine_mut(&mut self) -> &mut ConversionEngine {
        &mut self.engine
    }

    /// The title of the game.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Emulated frames since the game started or was reset.
    #[must_use]
    pub const fn frame_counter(&self) -> i32 {
        self.frame_counter
    }

    /// The Lua runtime.
    #[must_use]
    pub fn runtime(&self) -> Option<&ProfileScriptRuntime> {
        self.engine.script_host_as::<ProfileScriptRuntime>()
    }

    /// Puts the scripts' Lua collector in its smooth or its original tuning (see
    /// `ProfileScriptRuntime::set_smooth_gc`).
    pub fn set_smooth_gc(&self, smooth: bool) {
        if let Some(runtime) = self.runtime() {
            runtime.set_smooth_gc(smooth);
        }
    }

    /// The names of the scripts, in hook order.
    #[must_use]
    pub fn script_names(&self) -> Vec<String> {
        self.runtime()
            .map(|runtime| {
                runtime
                    .scripts()
                    .iter()
                    .map(|script| script.name().to_owned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Nominal frames per second of the core's region.
    #[must_use]
    pub fn nominal_frame_rate(&self) -> i32 {
        self.core.timing().nominal_frames_per_second
    }

    /// Loads a ROM image into the core (REQ-LOOP-021 step 4). The core patches a ROM a
    /// built-in patch was made for (the file is left alone).
    pub fn load_rom(&mut self, image: &[u8], name: &str) -> RomLoadResult {
        let result = self.core.load_rom(image, name);
        if let Some(patch) = self.core.builtin_patch() {
            crate::app_log::line(&format!("{name}: built-in patch applied ({patch})."));
        }
        if result.success {
            self.frame_counter = 0;
        }
        result
    }

    /// Whether a ROM is loaded.
    #[must_use]
    pub fn has_rom(&self) -> bool {
        self.pipeline.in_flight || self.core.is_running()
    }

    /// Whether the next frame may run on the emulation thread while the scripts of the last one
    /// run here: the profile's scripts leave the core alone.
    #[must_use]
    pub const fn can_run_ahead(&self) -> bool {
        self.pipeline.ahead_allowed
    }

    /// Takes the core back from the emulation thread, if it is away, and captures the frame it
    /// ran (tile records, palette, picture) for [`GameSession::finish_captured`]: the core is
    /// free again for the host, the conversion of the frame comes later. "Emulation ahead".
    pub fn capture_in_flight(&mut self, clock: &mut StageClock) {
        if !self.pipeline.in_flight {
            return;
        }
        self.pipeline.in_flight = false;
        let Some(finished) = self.pipeline.worker.as_ref().and_then(CoreWorker::finish) else {
            crate::app_log::error("Zaxnes: the emulation thread stopped during a frame.");
            return;
        };
        self.pipeline.spare = Some(std::mem::replace(&mut self.core, finished.core));
        clock.add(Stage::Core, finished.elapsed);
        clock.add_thread(Thread::Emulation, finished.elapsed);
        let captured = self.capture_batch(clock);
        self.pipeline.pending = Some(captured);
        self.pipeline
            .memory
            .ram
            .copy_from_slice(self.core.bus().ram());
        self.pipeline.picture_height = self.core.wide_picture_into(&mut self.pipeline.picture);
    }

    /// Starts the next frame on the emulation thread at once ("emulation ahead"): its input word
    /// (the scripts have no input hook) and the core leave now, and the frame captured before it
    /// is converted afterwards by [`GameSession::finish_captured`], while it runs.
    pub fn launch_ahead(
        &mut self,
        model: &mut SettingsModel,
        scene: &mut SceneState,
        frame: SessionFrame,
        clock: &mut StageClock,
    ) {
        if clock.enabled() != self.engine.last_hook_time().is_some() {
            self.engine.set_hook_timing(clock.enabled());
        }
        let word = self.lend(model, scene, |session, scene| {
            session.begin_frame_lent(scene, frame, clock)
        });
        self.pipeline.prepared = Some(word);
        self.launch_prepared();
        if !self.pipeline.in_flight {
            // No thread: the frame captured before is converted first, then this one runs here
            // and is captured like one that came back.
            if let Some(word) = self.pipeline.prepared.take() {
                self.finish_captured(model, scene, clock);
                let started = clock.start();
                self.core.run_frame(word);
                clock.stop(Stage::Core, started);
                let captured = self.capture_batch(clock);
                self.pipeline.pending = Some(captured);
                self.pipeline.picture_height =
                    self.core.wide_picture_into(&mut self.pipeline.picture);
            }
        }
    }

    /// Converts the frame [`GameSession::capture_in_flight`] captured, if any: conversion, frame
    /// hooks, camera tracking, publication.
    pub fn finish_captured(
        &mut self,
        model: &mut SettingsModel,
        scene: &mut SceneState,
        clock: &mut StageClock,
    ) {
        let Some((palette, mask_x, mask_y, margin)) = self.pipeline.pending.take() else {
            return;
        };
        self.lend(model, scene, |session, scene| {
            session.convert_batch_lent(scene, clock, palette, (mask_x, mask_y), margin);
        });
        if self.pipeline.memory.missed.swap(false, Ordering::Relaxed) {
            // A script read cartridge space, which the copy does not hold: from now on the next
            // frame waits for the scripts again.
            self.pipeline.ahead_allowed = false;
            crate::app_log::error(
                "Zaxnes: a script read cartridge memory; emulation ahead is off for this game.",
            );
        }
    }

    /// The conversion a pipelined frame left to its own thread ([`EmulationHost`]'s
    /// `defer_conversion`): the captured frame is converted, then its draw list is built.
    ///
    /// [`EmulationHost`]: crate::emulation::host::EmulationHost
    pub fn convert_deferred(
        &mut self,
        model: &mut SettingsModel,
        scene: &mut SceneState,
        clock: &mut StageClock,
    ) {
        self.finish_captured(model, scene, clock);
        let started = clock.start();
        self.engine.build_draw_list();
        clock.stop(Stage::DrawList, started);
    }

    /// Whether a frame was captured and waits for its conversion.
    #[must_use]
    pub const fn has_captured_frame(&self) -> bool {
        self.pipeline.pending.is_some()
    }

    /// Swaps the draw list built last with `list`, which the caller then draws.
    pub fn exchange_draw_list(&mut self, list: &mut zax_engine3d::draw_list::FrameDrawList) {
        self.engine.exchange_draw_list(list);
    }

    /// Puts the frame counter back to zero.
    pub const fn reset_frame_counter(&mut self) {
        self.frame_counter = 0;
    }

    /// Restores a core state block, clearing the engine's frames (they described the old
    /// state).
    ///
    /// # Errors
    /// The text of the refusal.
    pub fn restore_core(&mut self, block: &[u8]) -> Result<(), String> {
        let restored = self
            .core
            .try_load_state(block, 0)
            .map(|_| ())
            .map_err(|error| error.to_string());
        if restored.is_ok() {
            self.engine.clear_frames();
        }
        restored
    }

    /// Starts recording a movie from the current state (the core is home).
    ///
    /// # Errors
    /// No ROM loaded.
    pub fn start_movie_recording(&mut self) -> Result<(), String> {
        let state = self
            .core
            .save_state(true)
            .map_err(|error| error.to_string())?;
        self.movie = Some(MovieRun::record(self.core.frame_count(), state));
        Ok(())
    }

    /// Loads the start state of `movie` and plays it from there.
    ///
    /// # Errors
    /// The start state does not load.
    pub fn start_movie_playback(&mut self, movie: Movie) -> Result<(), String> {
        self.restore_core(&movie.start_state)?;
        self.movie = Some(MovieRun::play(self.core.frame_count(), movie));
        Ok(())
    }

    /// Ends the movie being recorded or played, and gives it.
    pub fn stop_movie(&mut self) -> Option<Movie> {
        self.movie.take().map(MovieRun::into_movie)
    }

    /// The movie being recorded or played.
    #[must_use]
    pub const fn movie(&self) -> Option<&MovieRun> {
        self.movie.as_ref()
    }

    /// Writes `text` on the frame on screen of the movie; returns that frame.
    pub fn annotate_movie(&mut self, text: &str) -> Option<u32> {
        let frame_count = self.core.frame_count();
        self.movie.as_mut()?.annotate(frame_count, text)
    }

    /// Runs the `Start` hooks, once.
    pub fn start(&mut self, model: &mut SettingsModel, scene: &mut SceneState) {
        if self.started {
            return;
        }
        self.started = true;
        self.lend(model, scene, |session, scene| {
            session.with_runtime(scene, ProfileScriptRuntime::invoke_start);
        });
    }

    /// Runs the `End` hooks and resets what a game leaves in the scene and the settings.
    pub fn end(&mut self, model: &mut SettingsModel, scene: &mut SceneState) {
        if !self.started {
            return;
        }
        self.started = false;
        self.lend(model, scene, |session, scene| {
            session.with_runtime(scene, ProfileScriptRuntime::invoke_end);
            scene.backdrop.reset();
            session.with_runtime(scene, |runtime, _| runtime.settings_mut().end_game());
        });
    }

    /// Resets the game: `End`, a power cycle of the core, `Start`.
    pub fn reset(&mut self, model: &mut SettingsModel, scene: &mut SceneState) {
        self.end(model, scene);
        self.core.reset();
        self.frame_counter = 0;
        self.start(model, scene);
    }

    /// The scripts' state for a save state: runs `Save`, then reads `PData`.
    ///
    /// # Errors
    /// The Lua state could not be read.
    pub fn capture_script_state(
        &mut self,
        model: &mut SettingsModel,
        scene: &mut SceneState,
    ) -> Result<Vec<u8>, LuaScriptError> {
        self.lend(model, scene, |session, scene| {
            session
                .with_runtime(scene, |runtime, state| {
                    runtime.invoke_save(state);
                    runtime
                        .capture_persistent_data()
                        .map(|block| block.to_vec())
                })
                .unwrap_or_else(|| Ok(Vec::new()))
        })
    }

    /// Puts the scripts' state back and runs `Start` then `Load` (REQ-LUA-072, DEC-R2-T6).
    ///
    /// # Errors
    /// The Lua state could not be written.
    pub fn complete_script_restore(
        &mut self,
        model: &mut SettingsModel,
        scene: &mut SceneState,
        block: &[u8],
    ) -> Result<(), LuaScriptError> {
        let restored = self
            .with_runtime(scene, |runtime, _| runtime.restore_persistent_data(block))
            .unwrap_or(Ok(()));
        self.frame_counter = 0;
        self.start(model, scene);
        self.lend(model, scene, |session, scene| {
            session.with_runtime(scene, ProfileScriptRuntime::invoke_load);
        });
        restored
    }

    /// The `retro_unserialize` path of REQ-LUA-075 and DEC-R2-D5: the profile stays loaded, so
    /// the entries go back and `Load` runs, with no `End` and no `Start`.
    ///
    /// # Errors
    /// The Lua state could not be written.
    pub fn restore_script_state_in_place(
        &mut self,
        model: &mut SettingsModel,
        scene: &mut SceneState,
        block: &[u8],
    ) -> Result<(), LuaScriptError> {
        let restored = self
            .with_runtime(scene, |runtime, _| runtime.restore_persistent_data(block))
            .unwrap_or(Ok(()));
        self.lend(model, scene, |session, scene| {
            session.with_runtime(scene, ProfileScriptRuntime::invoke_load);
        });
        restored
    }

    /// Runs one emulated frame: the script pad, the core, the conversion, the publication.
    ///
    /// The script pad runs before the core sees the frame and is additive: it adds bits to the
    /// player it is attached to and can never mask a physical input (REQ-INP-071, 072).
    pub fn run_frame(
        &mut self,
        model: &mut SettingsModel,
        scene: &mut SceneState,
        player_one: InputAction,
        player_two: InputAction,
        zapper_byte: u8,
        zapper_position: ZapperPosition,
    ) {
        let frame = SessionFrame {
            player_one,
            player_two,
            zapper_byte,
            zapper_position,
        };
        self.run_frame_timed(model, scene, frame, &mut StageClock::default());
    }

    /// [`GameSession::run_frame`] with its stages timed on `clock` when the clock is on (the
    /// frame profiler): core, tile-record capture, conversion and Lua hooks.
    pub fn run_frame_timed(
        &mut self,
        model: &mut SettingsModel,
        scene: &mut SceneState,
        frame: SessionFrame,
        clock: &mut StageClock,
    ) {
        self.complete_frame_timed(model, scene, clock);
        if clock.enabled() != self.engine.last_hook_time().is_some() {
            self.engine.set_hook_timing(clock.enabled());
        }
        self.lend(model, scene, |session, scene| {
            let word = session.begin_frame_lent(scene, frame, clock);
            let started = clock.start();
            session.core.run_frame(word);
            clock.stop(Stage::Core, started);
            session.finish_frame_lent(scene, clock);
        });
    }

    /// Prepares the next frame for the emulation thread ("emulation on a separate core"): the
    /// frame before it is finished first, then the input hook of the scripts runs and the input
    /// word is computed, as at the start of [`GameSession::run_frame_timed`]. The frame itself
    /// runs from [`GameSession::launch_prepared`] and is finished by
    /// [`GameSession::complete_frame_timed`].
    pub fn prepare_frame_timed(
        &mut self,
        model: &mut SettingsModel,
        scene: &mut SceneState,
        frame: SessionFrame,
        clock: &mut StageClock,
    ) {
        self.complete_frame_timed(model, scene, clock);
        if clock.enabled() != self.engine.last_hook_time().is_some() {
            self.engine.set_hook_timing(clock.enabled());
        }
        let word = self.lend(model, scene, |session, scene| {
            session.begin_frame_lent(scene, frame, clock)
        });
        self.pipeline.prepared = Some(word);
    }

    /// Hands the core and the prepared frame to the emulation thread, which is started the first
    /// time. Without a thread (the system refused one) the frame stays prepared and runs on the
    /// calling thread when it is completed.
    pub fn launch_prepared(&mut self) {
        let Some(word) = self.pipeline.prepared.take() else {
            return;
        };
        if self.pipeline.worker.is_none() {
            match CoreWorker::spawn() {
                Ok(worker) => {
                    self.pipeline.worker = Some(worker);
                    self.pipeline.spare = Some(Box::new(NesCore::new()));
                }
                Err(error) => {
                    crate::app_log::error(&format!(
                        "Zaxnes: no emulation thread ({error}); the emulation stays on the main thread."
                    ));
                }
            }
        }
        let (Some(worker), Some(spare)) = (&self.pipeline.worker, self.pipeline.spare.take())
        else {
            self.pipeline.prepared = Some(word);
            return;
        };

        // The 2D mode shows the core's picture while the next frame runs (emulation ahead has
        // kept the picture of the frame it captured).
        if self.pipeline.pending.is_none() {
            self.pipeline.picture_height = self.core.wide_picture_into(&mut self.pipeline.picture);
        }
        let core = std::mem::replace(&mut self.core, spare);
        match worker.start(core, word) {
            Ok(()) => self.pipeline.in_flight = true,
            Err(core) => {
                self.pipeline.spare = Some(std::mem::replace(&mut self.core, core));
                self.pipeline.prepared = Some(word);
            }
        }
    }

    /// Finishes the frame [`GameSession::prepare_frame_timed`] prepared, if any: takes the core
    /// back from the emulation thread (or runs the frame here when it never left), then captures
    /// the tile records, converts the frame and runs the frame hooks. Whatever reads or changes
    /// the core calls this first.
    pub fn complete_frame_timed(
        &mut self,
        model: &mut SettingsModel,
        scene: &mut SceneState,
        clock: &mut StageClock,
    ) {
        if self.pipeline.in_flight {
            self.pipeline.in_flight = false;
            let Some(finished) = self.pipeline.worker.as_ref().and_then(CoreWorker::finish) else {
                crate::app_log::error("Zaxnes: the emulation thread stopped during a frame.");
                return;
            };
            self.pipeline.spare = Some(std::mem::replace(&mut self.core, finished.core));
            clock.add(Stage::Core, finished.elapsed);
            clock.add_thread(Thread::Emulation, finished.elapsed);
        } else if let Some(word) = self.pipeline.prepared.take() {
            // A frame captured ahead of this one is converted first: its records are pending.
            self.finish_captured(model, scene, clock);
            let started = clock.start();
            self.core.run_frame(word);
            clock.stop(Stage::Core, started);
        } else {
            self.finish_captured(model, scene, clock);
            return;
        }
        // The frame before the one that came back, captured ahead, is converted first.
        self.finish_captured(model, scene, clock);
        self.lend(model, scene, |session, scene| {
            session.finish_frame_lent(scene, clock);
        });
    }

    /// The scripts' input hook and the input word of the next frame (the first half of a frame).
    fn begin_frame_lent(
        &mut self,
        scene: &mut SceneState,
        frame: SessionFrame,
        clock: &mut StageClock,
    ) -> u16 {
        let SessionFrame {
            player_one,
            player_two,
            zapper_byte,
            zapper_position,
        } = frame;
        let counter = self.frame_counter;
        let started = clock.start();
        #[allow(clippy::cast_precision_loss)] // the C# `FrameCounter / 60f`
        let time = counter as f32 / 60.0;
        let (pad, pad_index) = self
            .with_runtime(scene, |runtime, state| {
                (
                    runtime.invoke_input_process(state, counter, time),
                    runtime.script_pad_index(),
                )
            })
            .unwrap_or((0, 0));
        clock.stop(Stage::Lua, started);
        let (first, second) = routing::merge_script_pad(
            player_one,
            player_two,
            InputAction::from_bits(u32::try_from(pad).unwrap_or(0)),
            pad_index,
        );
        self.input.players = [
            i32::try_from(first.bits()).unwrap_or(0),
            i32::try_from(second.bits()).unwrap_or(0),
        ];
        if self.aimed != Some(zapper_position) {
            self.aimed = Some(zapper_position);
            self.core
                .set_zapper_position_source(Some(Box::new(move |_| zapper_position)));
        }
        // REQ-INP-004: a Zapper port carries the Zapper byte and no controller button at all.
        let word = routing::core_word(
            first,
            second,
            self.core.port_type(0).unwrap_or_default(),
            self.core.port_type(1).unwrap_or_default(),
            zapper_byte,
        );
        let frame_count = self.core.frame_count();
        self.movie
            .as_mut()
            .map_or(word, |movie| movie.input(frame_count, word))
    }

    /// The tile capture, the conversion and the frame hooks of the frame the core just ran (the
    /// second half of a frame).
    fn finish_frame_lent(&mut self, scene: &mut SceneState, clock: &mut StageClock) {
        let (palette, mask_x, mask_y, margin) = self.capture_batch(clock);
        self.convert_batch_lent(scene, clock, palette, (mask_x, mask_y), margin);
    }

    /// Copies the tile records of the frame the core just ran into `batch_records`; returns its
    /// palette, edge masks and widescreen margin.
    fn capture_batch(&mut self, clock: &mut StageClock) -> CapturedBatch {
        let started = clock.start();
        let batch = self.core.frame_batch();
        self.batch_records.clear();
        self.batch_records.extend_from_slice(batch.records);
        let captured = (
            batch.palette,
            batch.mask_x,
            batch.mask_y,
            (batch.margin, batch.visible_rows, batch.margin_y),
        );
        clock.stop(Stage::Capture, started);
        captured
    }

    /// The conversion, frame hooks and camera tracking of the captured frame.
    fn convert_batch_lent(
        &mut self,
        scene: &mut SceneState,
        clock: &mut StageClock,
        palette: [u8; 32],
        (mask_x, mask_y): (i32, i32),
        (margin, visible_rows, margin_y): (i32, zax_nes::VisibleRows, i32),
    ) {
        let visible_rows = if self.strict_wide_edges || margin == 0 {
            visible_rows
        } else {
            // Every band counts as drawn across the whole margin.
            #[allow(clippy::cast_possible_truncation)] // At most 512.
            let margin = margin as i16;
            [(-margin, 256 + margin); 30]
        };
        let counter = self.frame_counter;
        let records = std::mem::take(&mut self.batch_records);
        // The conversion is synchronous, right after the frame, as REQ-LOOP-060 (c) requires.
        let batch = FrameBatch {
            records: &records,
            palette,
            mask_x,
            mask_y,
            margin,
            visible_rows,
            margin_y,
        };
        let started = clock.start();
        self.scripted(scene, |engine| {
            engine.convert(&batch, counter);
        });
        if let Some(started) = started {
            // The frame hooks are timed inside the conversion and counted apart.
            let hooks = self.engine.last_hook_time().unwrap_or_default();
            clock.add(Stage::Conversion, started.elapsed().saturating_sub(hooks));
            clock.add(Stage::Lua, hooks);
        }
        self.batch_records = records;
        // The subjective camera follows the shape a script picked, through the tracker of
        // SPEC-04 (REQ-LUA-088, REQ-SCN-021).
        let started = clock.start();
        self.with_runtime(scene, ProfileScriptRuntime::advance_camera_tracking);
        clock.stop(Stage::Lua, started);
        self.engine.publish();
        self.frame_counter += 1;
    }

    /// The device plugged into `port`.
    #[must_use]
    pub fn port_type(&self, port: usize) -> ControllerPortType {
        self.core.port_type(port).unwrap_or_default()
    }

    /// Plugs `kind` into `port`.
    pub fn set_port_type(&mut self, port: usize, kind: ControllerPortType) {
        if self.core.set_port_type(port, kind).is_err() {
            // Only ports 0 and 1 exist; the command model has already checked the number.
        }
    }

    /// Both ports back to a standard pad.
    pub fn reset_port_types(&mut self) {
        for port in 0..2 {
            self.set_port_type(port, ControllerPortType::Controller);
        }
    }

    /// The Zapper aim offset the scripts set.
    #[must_use]
    pub fn zapper_offset(&self) -> Vec3 {
        self.input.zapper_offset
    }

    /// The list of tiles to draw for the published frame.
    pub fn build_draw_list(&mut self) -> &FrameDrawList {
        self.engine.build_draw_list()
    }

    /// Whether the game is played flat.
    #[must_use]
    pub fn two_dimensional_mode(&self) -> bool {
        self.engine.two_dimensional_mode
    }

    /// Copies the core's indexed picture (256 x 240, larger with widescreen margins) into
    /// `destination`; returns its rows.
    pub fn copy_nes_picture(&self, destination: &mut Vec<u8>) -> usize {
        destination.clear();
        if self.pipeline.in_flight || self.pipeline.pending.is_some() {
            destination.extend_from_slice(&self.pipeline.picture);
            self.pipeline.picture_height
        } else {
            self.core.wide_picture_into(destination)
        }
    }

    /// The shape the subjective camera follows.
    #[must_use]
    pub fn followed_shape(&self) -> Option<zax_engine3d::frame::ShapeId> {
        self.runtime().and_then(ProfileScriptRuntime::fpv_shape)
    }

    /// The colour of the first palette entry of the newest frame, behind the NES picture.
    #[must_use]
    pub fn nes_background_color(&self) -> Rgb24 {
        let state = self.engine.state();
        state
            .published_frame()
            .unwrap_or_else(|| state.converting_frame())
            .palette()
            .color(0)
            .unwrap_or_default()
    }

    /// Emulated time of the newest frame, in seconds.
    #[must_use]
    pub fn emulated_time(&self) -> f32 {
        let state = self.engine.state();
        state
            .published_frame()
            .unwrap_or_else(|| state.converting_frame())
            .time()
    }

    /// Switches the meshing algorithm; the geometry is rebuilt (REQ-LOOP-024).
    pub fn set_render_algorithm(
        &mut self,
        model: &mut SettingsModel,
        scene: &mut SceneState,
        algorithm: RenderAlgorithm,
    ) {
        self.lend(model, scene, |session, scene| {
            session.scripted(scene, |engine| engine.set_render_algorithm(algorithm));
        });
    }

    /// Mirrors the settings changed since the last call into the engine, and reports what the
    /// scripts and the model changed.
    pub fn sync_settings(
        &mut self,
        model: &mut SettingsModel,
        scene: &mut SceneState,
    ) -> Vec<SessionEvent> {
        self.lend(model, scene, |_, _| ());
        self.events.try_iter().collect()
    }

    /// The events the scripts raised since the last call.
    pub fn take_events(&mut self) -> Vec<SessionEvent> {
        self.events.try_iter().collect()
    }

    /// Ends the game and drops the session.
    pub fn dispose(mut self, model: &mut SettingsModel, scene: &mut SceneState) {
        self.end(model, scene);
    }
}

// The pipelined frames (`emu.pipeline`) lend the session, the settings and the scene to the
// conversion thread for the length of a frame.
const _: fn() = || {
    fn send<T: Send>() {}
    send::<GameSession>();
    send::<SettingsModel>();
    send::<SceneState>();
};

#[cfg(test)]
mod tests {
    use zax_profiles::script::ProfileScript;

    use super::*;

    fn profile(sources: &[&str]) -> GameProfile {
        GameProfile {
            scripts: sources
                .iter()
                .map(|source| ProfileScript {
                    source: Some((*source).to_owned()),
                    ..ProfileScript::default()
                })
                .collect(),
            ..GameProfile::default()
        }
    }

    #[test]
    fn only_scripts_that_leave_the_core_alone_run_ahead() {
        assert!(scripts_leave_the_core_alone(&GameProfile::default()));
        assert!(scripts_leave_the_core_alone(&profile(&[
            "function UpdateS() shape.Offset.z = 2 end"
        ])));
        // `ReadMem` reads the copy of the RAM taken when the frame came back.
        assert!(scripts_leave_the_core_alone(&profile(&[
            "function Update() local lane = Script:ReadMem(0x70) end"
        ])));
        for member in CORE_MEMBERS {
            let source = format!("function Update() local v = Script.{member} end");
            assert!(
                !scripts_leave_the_core_alone(&profile(&["function Start() end", &source])),
                "{member}"
            );
        }
    }
}
