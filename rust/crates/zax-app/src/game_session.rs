//! One game being played: the core, the conversion engine, the Lua runtime and the bridges
//! between them (SPEC-07 §3.1, SPEC-05 REQ-LUA-015).
//!
//! # Ownership
//!
//! The settings model belongs to the host. The Lua runtime keeps a model of its own inside
//! `Setting`, so the session lends the host's model to the runtime for the length of each call
//! that can run a script (`lend`), by exchanging it with a placeholder, and gives it back
//! afterwards. The core and the scene are shared with the script bridges, which are `'static`
//! trait objects of the scripting crate; see [`SharedCore`].

use std::cell::{Cell, Ref, RefCell, RefMut};
use std::rc::Rc;
use std::sync::mpsc::{channel, Receiver, Sender};

use zax_engine3d::draw_list::FrameDrawList;
use zax_engine3d::engine::ConversionEngine;
use zax_engine3d::error::EngineError;
use zax_engine3d::palette::Rgb24;
use zax_engine3d::settings::RenderAlgorithm;
use zax_engine3d::vectors::Vec3;
use zax_nes::rom::RomLoadResult;
use zax_nes::{ControllerPortType, FrameBatch, NesCore, TileRecord, ZapperPosition};
use zax_profiles::profile::GameProfile;
use zax_profiles::setting_application;
use zax_scripting::{
    LuaScriptError, ProfileScriptRuntime, ScriptEnvironment, ScriptInput, ScriptLight, ScriptLog,
    ScriptMemory, ScriptProfileInfo, ScriptSettings,
};
use zax_settings::catalogue::{Port, SettingId};
use zax_settings::model::SettingsModel;

use crate::input::input_action::InputAction;
use crate::input::routing;
use crate::scene_script_bridge::{self, SceneBackdropBridge, SceneCameraBridge, SharedScene};

/// The core, shared between the session and the script bridges that read and write its memory
/// and its ports. The core is never running while a script executes (the conversion runs after
/// `run_frame` has returned), so the borrows never overlap.
pub type SharedCore = Rc<RefCell<NesCore>>;

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

/// The state the script input bridge shares with the session.
#[derive(Debug, Default)]
struct ScriptInputCells {
    players: Cell<[i32; 2]>,
    zapper_offset: Cell<Vec3>,
}

/// The inputs as scripts see them: the players' action masks, the ports of the core and the
/// Zapper aim offset.
struct CoreScriptInput {
    core: SharedCore,
    cells: Rc<ScriptInputCells>,
}

impl ScriptInput for CoreScriptInput {
    fn player_state(&self, index: usize) -> i32 {
        self.cells.players.get().get(index).copied().unwrap_or(0)
    }

    fn global_state(&self) -> i32 {
        let [one, two] = self.cells.players.get();
        one | two
    }

    fn port_byte(&self, port: usize) -> i32 {
        if port < 2 {
            i32::from(self.core.borrow().bus().input().port_byte(port))
        } else {
            0
        }
    }

    fn port_type(&self, port: usize) -> i32 {
        self.core
            .borrow()
            .port_type(port)
            .map_or(0, |kind| kind as i32)
    }

    fn set_port_types(&mut self, first: i32, second: i32) {
        let mut core = self.core.borrow_mut();
        for (port, kind) in [first, second].into_iter().enumerate() {
            let kind = if kind.clamp(0, 1) == 1 {
                ControllerPortType::Zapper
            } else {
                ControllerPortType::Controller
            };
            if core.set_port_type(port, kind).is_err() {
                // Both ports exist, so this cannot happen; the type is simply not changed.
            }
        }
    }

    fn zapper_offset(&self) -> Vec3 {
        self.cells.zapper_offset.get()
    }

    fn set_zapper_offset(&mut self, offset: Vec3) {
        self.cells.zapper_offset.set(offset);
    }
}

/// The core's CPU address space as scripts see it.
struct CoreScriptMemory(SharedCore);

impl ScriptMemory for CoreScriptMemory {
    fn read(&self, address: u16) -> u8 {
        self.0.borrow().read_memory(address)
    }

    fn write(&mut self, address: u16, value: u8) {
        self.0.borrow_mut().write_memory(address, value);
    }
}

/// What `GameSession::create` needs to know besides the profile.
pub struct SessionRequest<'a> {
    /// The profile to play.
    pub profile: &'a GameProfile,
    /// The title to show when the profile has none.
    pub title: &'a str,
    /// The scene the camera and backdrop bridges drive.
    pub scene: SharedScene,
    /// Where `print` and the script errors go.
    pub log: Box<dyn ScriptLog>,
    /// Whether the profile's settings are applied to the model: true for a different game
    /// (REQ-PERS-005), false when the same game is relaunched (REQ-PERS-006).
    pub different_game: bool,
    /// Whether the game is played flat; `None` means "when the profile has no pattern".
    pub two_dimensional_mode: Option<bool>,
}

/// One game being played.
pub struct GameSession {
    core: SharedCore,
    engine: ConversionEngine,
    input: Rc<ScriptInputCells>,
    events: Receiver<SessionEvent>,
    title: String,
    frame_counter: i32,
    started: bool,
    aimed: Option<ZapperPosition>,
    batch_records: Vec<TileRecord>,
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
        let core: SharedCore = Rc::new(RefCell::new(NesCore::new()));
        let mut engine = ConversionEngine::new();
        engine.use_geometry();
        engine.two_dimensional_mode = two_dimensional_mode.unwrap_or(
            profile.permanent_patterns.is_empty() && profile.temporary_patterns.is_empty(),
        );
        let cells = Rc::new(ScriptInputCells::default());
        let (sender, events) = channel();
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
        scene_script_bridge::push_settings(settings.model(), &mut scene.borrow_mut());
        let environment = ScriptEnvironment {
            settings,
            camera: Box::new(SceneCameraBridge(Rc::clone(&scene))),
            backdrop: Box::new(SceneBackdropBridge(Rc::clone(&scene))),
            input: Box::new(CoreScriptInput {
                core: Rc::clone(&core),
                cells: Rc::clone(&cells),
            }),
            memory: Box::new(CoreScriptMemory(Rc::clone(&core))),
            log,
            profile: ScriptProfileInfo {
                title: profile.title.clone().unwrap_or_else(|| title.to_owned()),
                maker: profile.author.clone().unwrap_or_default(),
                game_dev: profile.publisher.clone().unwrap_or_default(),
                description: profile.description.clone().unwrap_or_default(),
                id: profile.id.clone().unwrap_or_default(),
                version: profile.version_text.clone().unwrap_or_default(),
            },
            notify: None,
        };
        // REQ-LUA-015 step 1: every chunk runs before anything else.
        let runtime = ProfileScriptRuntime::create(profile, environment, engine.state_mut())?;
        engine.set_script_host(Box::new(runtime));
        let mut session = Self {
            core,
            engine,
            input: cells,
            events,
            title: title.to_owned(),
            frame_counter: 0,
            started: false,
            aimed: None,
            batch_records: Vec::new(),
        };
        // The runtime holds the model from here until the last line of this function.
        let loaded = session.load_profile(profile, different_game);
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
    ) -> Result<(), SessionError> {
        self.with_runtime(|runtime, _| {
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
            let ports = [
                model.get(SettingId::InputPortType(Port::ALL[0])).as_int(),
                model.get(SettingId::InputPortType(Port::ALL[1])).as_int(),
            ];
            runtime
                .environment_mut()
                .input
                .set_port_types(ports[0], ports[1]);
        });
        // Step 3: the patterns with their BuildPattern hooks. The settings were applied above,
        // through the model, so the engine must not copy them a second time.
        self.sync_runtime_settings();
        self.engine.load_profile(profile, false)?;
        self.sync_runtime_settings();
        Ok(())
    }

    fn with_runtime<R>(
        &mut self,
        operation: impl FnOnce(
            &mut ProfileScriptRuntime,
            &mut zax_engine3d::engine::ConversionState,
        ) -> R,
    ) -> Option<R> {
        self.engine
            .with_script_host::<ProfileScriptRuntime, R>(operation)
    }

    fn sync_runtime_settings(&mut self) {
        self.with_runtime(ProfileScriptRuntime::sync_settings);
    }

    fn exchange_model(&mut self, model: &mut SettingsModel) {
        self.with_runtime(|runtime, _| std::mem::swap(runtime.settings_mut().model_mut(), model));
    }

    /// Lends the host's `model` to the Lua runtime for the length of `operation`, then mirrors
    /// what the scripts changed and gives the model back.
    fn lend<R>(&mut self, model: &mut SettingsModel, operation: impl FnOnce(&mut Self) -> R) -> R {
        self.exchange_model(model);
        let result = operation(self);
        self.sync_runtime_settings();
        self.exchange_model(model);
        result
    }

    /// The core, for reading.
    #[must_use]
    pub fn core(&self) -> Ref<'_, NesCore> {
        self.core.borrow()
    }

    /// The core, for writing.
    #[must_use]
    pub fn core_mut(&self) -> RefMut<'_, NesCore> {
        self.core.borrow_mut()
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
        self.core.borrow().timing().nominal_frames_per_second
    }

    /// Loads a ROM image into the core (REQ-LOOP-021 step 4).
    pub fn load_rom(&mut self, image: &[u8], name: &str) -> RomLoadResult {
        let result = self.core.borrow_mut().load_rom(image, name);
        if result.success {
            self.frame_counter = 0;
        }
        result
    }

    /// Whether a ROM is loaded.
    #[must_use]
    pub fn has_rom(&self) -> bool {
        self.core.borrow().is_running()
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
            .borrow_mut()
            .try_load_state(block, 0)
            .map(|_| ())
            .map_err(|error| error.to_string());
        if restored.is_ok() {
            self.engine.clear_frames();
        }
        restored
    }

    /// Runs the `Start` hooks, once.
    pub fn start(&mut self, model: &mut SettingsModel) {
        if self.started {
            return;
        }
        self.started = true;
        self.lend(model, |session| {
            session.with_runtime(ProfileScriptRuntime::invoke_start);
        });
    }

    /// Runs the `End` hooks and resets what a game leaves in the scene and the settings.
    pub fn end(&mut self, model: &mut SettingsModel) {
        if !self.started {
            return;
        }
        self.started = false;
        self.lend(model, |session| {
            session.with_runtime(|runtime, state| {
                runtime.invoke_end(state);
                runtime.environment_mut().backdrop.reset();
                runtime.settings_mut().end_game();
            });
        });
    }

    /// Resets the game: `End`, a power cycle of the core, `Start`.
    pub fn reset(&mut self, model: &mut SettingsModel) {
        self.end(model);
        self.core.borrow_mut().reset();
        self.frame_counter = 0;
        self.start(model);
    }

    /// The scripts' state for a save state: runs `Save`, then reads `PData`.
    ///
    /// # Errors
    /// The Lua state could not be read.
    pub fn capture_script_state(
        &mut self,
        model: &mut SettingsModel,
    ) -> Result<Vec<u8>, LuaScriptError> {
        self.lend(model, |session| {
            session
                .with_runtime(|runtime, state| {
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
        block: &[u8],
    ) -> Result<(), LuaScriptError> {
        let restored = self
            .with_runtime(|runtime, _| runtime.restore_persistent_data(block))
            .unwrap_or(Ok(()));
        self.frame_counter = 0;
        self.start(model);
        self.lend(model, |session| {
            session.with_runtime(ProfileScriptRuntime::invoke_load);
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
        player_one: InputAction,
        player_two: InputAction,
        zapper_byte: u8,
        zapper_position: ZapperPosition,
    ) {
        self.lend(model, |session| {
            session.run_frame_lent(player_one, player_two, zapper_byte, zapper_position);
        });
    }

    fn run_frame_lent(
        &mut self,
        player_one: InputAction,
        player_two: InputAction,
        zapper_byte: u8,
        zapper_position: ZapperPosition,
    ) {
        let counter = self.frame_counter;
        #[allow(clippy::cast_precision_loss)] // the C# `FrameCounter / 60f`
        let time = counter as f32 / 60.0;
        let (pad, pad_index) = self
            .with_runtime(|runtime, state| {
                (
                    runtime.invoke_input_process(state, counter, time),
                    runtime.script_pad_index(),
                )
            })
            .unwrap_or((0, 0));
        let (first, second) = routing::merge_script_pad(
            player_one,
            player_two,
            InputAction::from_bits(u32::try_from(pad).unwrap_or(0)),
            pad_index,
        );
        self.input.players.set([
            i32::try_from(first.bits()).unwrap_or(0),
            i32::try_from(second.bits()).unwrap_or(0),
        ]);
        if self.aimed != Some(zapper_position) {
            self.aimed = Some(zapper_position);
            self.core
                .borrow_mut()
                .set_zapper_position_source(Some(Box::new(move |_| zapper_position)));
        }
        // REQ-INP-004: a Zapper port carries the Zapper byte and no controller button at all.
        let (palette, mask_x, mask_y) = {
            let mut core = self.core.borrow_mut();
            let word = routing::core_word(
                first,
                second,
                core.port_type(0).unwrap_or_default(),
                core.port_type(1).unwrap_or_default(),
                zapper_byte,
            );
            core.run_frame(word);
            let batch = core.frame_batch();
            self.batch_records.clear();
            self.batch_records.extend_from_slice(batch.records);
            (batch.palette, batch.mask_x, batch.mask_y)
        };
        // The conversion is synchronous, right after the frame, as REQ-LOOP-060 (c) requires.
        let batch = FrameBatch {
            records: &self.batch_records,
            palette,
            mask_x,
            mask_y,
        };
        self.engine.convert(&batch, counter);
        // The subjective camera follows the shape a script picked, through the tracker of
        // SPEC-04 (REQ-LUA-088, REQ-SCN-021).
        self.with_runtime(ProfileScriptRuntime::advance_camera_tracking);
        self.engine.publish();
        self.frame_counter += 1;
    }

    /// The device plugged into `port`.
    #[must_use]
    pub fn port_type(&self, port: usize) -> ControllerPortType {
        self.core.borrow().port_type(port).unwrap_or_default()
    }

    /// Plugs `kind` into `port`.
    pub fn set_port_type(&self, port: usize, kind: ControllerPortType) {
        if self.core.borrow_mut().set_port_type(port, kind).is_err() {
            // Only ports 0 and 1 exist; the command model has already checked the number.
        }
    }

    /// Both ports back to a standard pad.
    pub fn reset_port_types(&self) {
        for port in 0..2 {
            self.set_port_type(port, ControllerPortType::Controller);
        }
    }

    /// The Zapper aim offset the scripts set.
    #[must_use]
    pub fn zapper_offset(&self) -> Vec3 {
        self.input.zapper_offset.get()
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

    /// Copies the core's 256 x 240 indexed picture into `destination`.
    pub fn copy_nes_picture(&self, destination: &mut Vec<u8>) {
        destination.clear();
        destination.extend_from_slice(self.core.borrow().pixel_frame());
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
    pub fn set_render_algorithm(&mut self, model: &mut SettingsModel, algorithm: RenderAlgorithm) {
        self.lend(model, |session| {
            session.engine.set_render_algorithm(algorithm)
        });
    }

    /// Mirrors the settings changed since the last call into the engine, and reports what the
    /// scripts and the model changed.
    pub fn sync_settings(&mut self, model: &mut SettingsModel) -> Vec<SessionEvent> {
        self.lend(model, |_| ());
        self.events.try_iter().collect()
    }

    /// The events the scripts raised since the last call.
    pub fn take_events(&mut self) -> Vec<SessionEvent> {
        self.events.try_iter().collect()
    }

    /// Ends the game and drops the session.
    pub fn dispose(mut self, model: &mut SettingsModel) {
        self.end(model);
    }
}
