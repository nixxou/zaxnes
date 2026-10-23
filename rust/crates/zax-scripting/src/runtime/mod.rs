//! The Lua runtime of one profile: a single Lua 5.4 state shared by every script of the
//! profile (REQ-LUA-010), the script model and the hooks of SPEC-05.
//!
//! One runtime per loaded profile. Creating it installs the base environment (§4.1) and runs
//! every chunk in file order (REQ-LUA-011); the host then applies the profile's settings, builds
//! the patterns — which calls `BuildPattern` through [`ScriptHost`] — and calls
//! [`ProfileScriptRuntime::invoke_start`], which is the order of REQ-LUA-015. Relaunching the
//! same game is [`ProfileScriptRuntime::invoke_end`] followed by `invoke_start`, with the state
//! kept (REQ-LUA-013); changing game drops the runtime and creates another (REQ-LUA-014).
//!
//! A Lua error never escapes: it marks the script in error, is logged once and the sequence
//! carries on with the other scripts (REQ-LUA-060 to REQ-LUA-063). No Rust error crosses into
//! Lua unhandled and no Lua error crosses into the host.
//!
//! The runtime does not own the engine state. Every call that can reach the script API takes it
//! as a parameter and lends it to the Lua state for the length of the call: see
//! [`ProfileScriptRuntime::run`].

mod globals;
mod hooks;

use std::any::Any;
use std::collections::HashMap;
use std::hash::BuildHasherDefault;
use std::time::Duration;

use mlua::{HookTriggers, Lua, MultiValue, Value, VmState};
use zax_engine3d::engine::{ConversionState, FrameRole};
use zax_engine3d::frame::ShapeId;
use zax_engine3d::instance::{InstanceRef, PatternInstance};
use zax_engine3d::registry::PatternId;
use zax_engine3d::script::{ScriptError, ScriptHost};
use zax_engine3d::settings::ConversionSettings;
use zax_profiles::profile::GameProfile;
use zax_profiles::script_order;

use self::globals::{install_api_globals, install_base, protected_call, text_of};
use self::hooks::{
    call_all, call_hook, input_process, publish, update_shapes, with_budget, Audience, Budget,
    Published, PublishedGlobals,
};
use crate::binding::cx::{with_host_context, Cx, Epochs, HostLink, Lent};
use crate::binding::dispatch;
use crate::binding::payload::{FrameKey, FrameSlot, InstanceTarget, Payload, ShapeTarget};
use crate::binding::table::IdHasher;
use crate::environment::{ScriptContext, ScriptEnvironment};
use crate::error::{message_of, LuaScriptError};
use crate::persistent_data;
use crate::script::{LoadedScript, ScriptHook};
use crate::settings::ScriptSettings;

/// The incremental collector's wait before a new cycle, in percent of the heap after the last.
const GC_PAUSE_PERCENT: i32 = 400;

/// The collector's speed against the allocator's, in percent.
const GC_STEP_MULTIPLIER: i32 = 400;

/// The size of a collector step as a power of two in bytes (the default of Lua 5.4).
const GC_STEP_SIZE_LOG2: i32 = 13;

/// How many VM instructions pass between two looks at the execution budget.
const BUDGET_CHECK_INSTRUCTIONS: u32 = 100_000;

/// The Lua values of the handles of one epoch, by cache key.
pub(crate) type FrameHandles = HashMap<FrameSlot, Value, BuildHasherDefault<IdHasher>>;

/// The handles of an epoch that is over, with the epochs that are current now.
pub(crate) struct Retired {
    pub handles: FrameHandles,
    pub epochs: Epochs,
}

/// Everything the runtime owns besides the Lua state. The script API reaches it through the
/// call context while a call into Lua is in progress.
pub(crate) struct RuntimeData {
    pub env: ScriptEnvironment,
    /// The scripts of the profile, in the hook order of REQ-LUA-004.
    pub scripts: Vec<LoadedScript>,
    /// The positions in `scripts` in file order, which is the `InputProcess` order of
    /// REQ-LUA-032.
    pub file_order: Vec<usize>,
    /// The script that owns the script pad (REQ-LUA-032).
    pub input_process_script: Option<usize>,
    pub has_frame_hooks: bool,
    pub frame_counter: i32,
    /// Changes whenever the frames the handles of the frame being converted point into change,
    /// so that a handle kept from an earlier frame reads as gone (REQ-LUA-028).
    pub epoch: u32,
    /// The same for the pinned frame, which only the pin and release calls change.
    pub pin_epoch: u32,
    /// The Lua values issued for the handles of the current epoch and of the current pin epoch,
    /// so that a handle is made once and found again until its epoch ends.
    frame_handles: FrameHandles,
    pinned_handles: FrameHandles,
    pub script_pad_enabled: bool,
    pub script_pad_index: i32,
    pub script_pad_state: i32,
    /// The shape the subjective camera follows (`Camera.FpvShape`).
    pub fpv_shape: Option<ShapeTarget>,
    /// The shared lists of REQ-LUA-048, emptied and refilled by each query.
    pub instance_query: Vec<InstanceTarget>,
    pub pattern_query: Vec<InstanceTarget>,
    pub pinned_query: Vec<ShapeTarget>,
    /// How long one hook call may run before it is cut off (REQ-LUA-064).
    pub hook_budget: Duration,
}

impl RuntimeData {
    /// The key a handle to the frame being converted gets now.
    pub const fn converting_key(&self) -> FrameKey {
        FrameKey {
            pinned: false,
            epoch: self.epoch,
        }
    }

    /// Takes the changes made to the settings model into the pipeline's own slice, when the
    /// engine state is at hand. Returns whether a script asked for a view reset, which the
    /// caller answers with the camera it has at hand.
    #[must_use = "a requested view reset is lost unless the camera answers it"]
    pub fn settle_settings(&mut self, conversion: Option<&mut ConversionSettings>) -> bool {
        match conversion {
            Some(conversion) => self.env.settings.sync(conversion),
            None => self.env.settings.absorb(),
        }

        self.env.settings.take_view_reset()
    }

    /// Puts into `linked` the scripts whose `BuildPattern` runs for `instance`: the active ones
    /// that define it and are linked to the instance (REQ-LUA-030).
    fn scripts_building(&self, instance: &PatternInstance, linked: &mut Vec<usize>) {
        linked.clear();
        linked.extend(
            self.scripts
                .iter()
                .enumerate()
                .filter(|(_, script)| {
                    script.active()
                        && script.defines(ScriptHook::BuildPattern)
                        && script.is_linked_to(instance)
                })
                .map(|(index, _)| index),
        );
    }

    /// Changes the epochs: every frame handle issued so far reads as gone.
    fn invalidate_handles(&mut self) -> [Retired; 2] {
        [self.advance_epoch(), self.advance_pin_epoch()]
    }

    /// Ends the epoch of the frame being converted.
    ///
    /// Returns the handles issued in the epoch, which the caller hands to the weak object cache
    /// so that they stay the one value of their object for as long as a script holds them.
    #[must_use = "the retired handles must be given to the object cache"]
    pub fn advance_epoch(&mut self) -> Retired {
        self.epoch = self.epoch.wrapping_add(1);
        let capacity = self.frame_handles.len();
        let handles = std::mem::replace(
            &mut self.frame_handles,
            FrameHandles::with_capacity_and_hasher(capacity, BuildHasherDefault::default()),
        );
        self.retired(handles)
    }

    /// Ends the epoch of the pinned frame.
    ///
    /// Returns the handles issued in the epoch, as [`Self::advance_epoch`] does.
    #[must_use = "the retired handles must be given to the object cache"]
    pub fn advance_pin_epoch(&mut self) -> Retired {
        self.pin_epoch = self.pin_epoch.wrapping_add(1);
        let handles = std::mem::take(&mut self.pinned_handles);
        self.retired(handles)
    }

    fn retired(&self, handles: FrameHandles) -> Retired {
        Retired {
            handles,
            epochs: Epochs {
                frame: self.epoch,
                pinned: self.pin_epoch,
            },
        }
    }

    /// The Lua values issued for the handles of the epoch `key` is in, or `None` when that epoch
    /// is over.
    pub fn handles_of(&mut self, key: FrameKey) -> Option<&mut FrameHandles> {
        if key.pinned {
            (key.epoch == self.pin_epoch).then_some(&mut self.pinned_handles)
        } else {
            (key.epoch == self.epoch).then_some(&mut self.frame_handles)
        }
    }
}

/// The way for a host to lend its [`ScriptContext`] to the hooks of a runtime it has handed to
/// the conversion engine.
///
/// The engine owns the runtime and calls its hooks, so the host cannot pass the context to the
/// runtime directly. The bridge is a second handle on the same Lua state: the host lends the
/// context for the length of a call into the engine, and every hook that runs during it reaches
/// the context. When the call returns the context is withdrawn, so nothing outlives the borrow.
#[derive(Clone)]
pub struct ScriptBridge {
    lua: Lua,
}

impl ScriptBridge {
    /// Runs `operation` with `context` available to the script API, then withdraws it. The
    /// operation hands its results out through what it captures. A hook that runs outside any
    /// lend sees the headless bridges of the environment.
    pub fn lend(&self, context: &mut ScriptContext<'_>, operation: impl FnOnce()) {
        let lent = self.lua.scope(|scope| {
            // Should the function not be created, the hooks of the operation fall back on the
            // headless bridges.
            if let Ok(call) = scope.create_function_mut(|lua, ()| HostLink::serve(lua, context)) {
                self.lua.set_app_data(HostLink::new(call));
            }
            operation();
            let flushed = flush_host_writes(&self.lua);
            self.lua.remove_app_data::<HostLink>();
            flushed
        });
        debug_assert!(lent.is_ok(), "the closure of a lend never fails");
    }
}

/// The Lua runtime of one profile.
pub struct ProfileScriptRuntime {
    lua: Lua,
    data: RuntimeData,
}

impl ProfileScriptRuntime {
    /// Creates the runtime of `profile`: installs the base environment and runs every chunk in
    /// file order (REQ-LUA-011).
    ///
    /// `state` is the engine state the chunks see; the profile's own settings are applied
    /// afterwards by the host (REQ-LUA-015).
    ///
    /// # Errors
    ///
    /// The base environment could not be installed. A script that fails to compile or to run is
    /// not an error here: it is logged and left without hooks (REQ-LUA-060).
    pub fn create(
        profile: &GameProfile,
        environment: ScriptEnvironment,
        state: &mut ConversionState,
    ) -> Result<Self, LuaScriptError> {
        Self::create_in(profile, environment, state, None)
    }

    /// Like [`ProfileScriptRuntime::create`], with the host `context` available to the chunks
    /// while they run, which is when a chunk's main level can already reach the camera, the
    /// inputs and the memory.
    ///
    /// # Errors
    ///
    /// The base environment could not be installed.
    pub fn create_in(
        profile: &GameProfile,
        environment: ScriptEnvironment,
        state: &mut ConversionState,
        context: Option<&mut ScriptContext<'_>>,
    ) -> Result<Self, LuaScriptError> {
        let mut runtime = Self {
            lua: Lua::new(),
            data: RuntimeData {
                env: environment,
                scripts: Vec::new(),
                file_order: Vec::new(),
                input_process_script: None,
                has_frame_hooks: false,
                frame_counter: 0,
                epoch: 0,
                pin_epoch: 0,
                frame_handles: FrameHandles::default(),
                pinned_handles: FrameHandles::default(),
                script_pad_enabled: false,
                script_pad_index: 0,
                script_pad_state: 0,
                fpv_shape: None,
                instance_query: Vec::new(),
                pattern_query: Vec::new(),
                pinned_query: Vec::new(),
                hook_budget: Duration::from_secs(1),
            },
        };

        // Hooks make short-lived garbage by the thousand, the handles of a frame; the live heap
        // is small, so the collector waits until the heap has grown fourfold and works harder
        // when it runs, instead of starting a cycle every few frames.
        runtime
            .lua
            .gc_inc(GC_PAUSE_PERCENT, GC_STEP_MULTIPLIER, GC_STEP_SIZE_LOG2);
        runtime.build(profile, state, context).map_err(|error| {
            LuaScriptError(format!(
                "The Lua base environment failed to install: {}",
                message_of(&error)
            ))
        })?;
        Ok(runtime)
    }

    fn build(
        &mut self,
        profile: &GameProfile,
        state: &mut ConversionState,
        context: Option<&mut ScriptContext<'_>>,
    ) -> mlua::Result<()> {
        install_base(&self.lua)?;
        dispatch::install(&self.lua)?;
        self.lua.set_app_data(Budget::new());
        install_api_globals(&self.lua)?;
        PublishedGlobals::install(&self.lua)?;

        self.data.env.settings.mirror_all(state.settings_mut());
        let bridge = self.bridge();
        let mut loaded = Ok(Ok(false));
        let mut load = || {
            loaded = self.run(Some(state), None, |cx| load_scripts(cx, profile));
        };
        match context {
            Some(context) => bridge.lend(context, load),
            None => load(),
        }
        self.data.has_frame_hooks = loaded??;

        self.lua.set_global_hook(
            HookTriggers::new().every_nth_instruction(BUDGET_CHECK_INSTRUCTIONS),
            |lua, _| {
                let exceeded = lua
                    .app_data_ref::<Budget>()
                    .is_some_and(|budget| budget.exceeded());
                if exceeded {
                    if let Some(budget) = lua.app_data_ref::<Budget>() {
                        budget.lift();
                    }

                    return Err(mlua::Error::runtime(
                        "the hook used up its execution budget",
                    ));
                }

                Ok(VmState::Continue)
            },
        )
    }

    /// Runs `operation` with the runtime's data, the engine state and the pattern instance
    /// being built lent to the Lua state: the script API reaches them through the call context
    /// for as long as the operation runs, and not a moment longer.
    fn run<R>(
        &mut self,
        state: Option<&mut ConversionState>,
        building: Option<(InstanceRef, &mut PatternInstance)>,
        operation: impl FnOnce(&Cx) -> R,
    ) -> mlua::Result<R> {
        let mut state = state;
        if let Some(state) = state.as_deref_mut() {
            let view_reset = self.data.settle_settings(Some(state.settings_mut()));
            self.answer_view_reset(view_reset);
        }

        let Self { lua, data } = self;
        lua.scope(|scope| {
            let data = scope.create_any_userdata_ref_mut(data)?;
            let state = state
                .map(|state| scope.create_any_userdata_ref_mut(state))
                .transpose()?;
            let build = building
                .map(|(reference, instance)| {
                    scope
                        .create_any_userdata_ref_mut(instance)
                        .map(|handle| (reference, handle))
                })
                .transpose()?;

            lua.set_app_data(Lent { data, state, build });
            let result = operation(&Cx::new(lua));
            let flushed = flush_host_writes(lua);
            lua.remove_app_data::<Lent>();
            flushed?;
            Ok(result)
        })
    }

    /// Reports a failure of the host side of a call: the script errors were logged where they
    /// happened. Returns the text of the first error, if any.
    fn settle(&mut self, result: mlua::Result<mlua::Result<Option<String>>>) -> Option<String> {
        let outcome = result.and_then(|inner| inner);
        match outcome {
            Ok(first) => first,
            Err(error) => {
                let message = message_of(&error);
                self.data
                    .env
                    .log
                    .error(&format!("Lua: the runtime failed: {message}"));
                Some(message)
            }
        }
    }

    // access for the host

    /// The scripts of the profile, in the hook order of REQ-LUA-004.
    #[must_use]
    pub fn scripts(&self) -> &[LoadedScript] {
        &self.data.scripts
    }

    /// The script of this name (`Script:Get`).
    #[must_use]
    pub fn find_script(&self, name: &str) -> Option<&LoadedScript> {
        self.data
            .scripts
            .iter()
            .find(|script| script.name() == name)
    }

    /// The environment the host supplied.
    #[must_use]
    pub const fn environment(&self) -> &ScriptEnvironment {
        &self.data.env
    }

    /// The environment the host supplied, for writing.
    pub fn environment_mut(&mut self) -> &mut ScriptEnvironment {
        &mut self.data.env
    }

    /// The settings model the scripts see (`Setting`).
    #[must_use]
    pub const fn settings(&self) -> &ScriptSettings {
        &self.data.env.settings
    }

    /// The same, for writing. The host calls [`ProfileScriptRuntime::sync_settings`]
    /// afterwards.
    pub fn settings_mut(&mut self) -> &mut ScriptSettings {
        &mut self.data.env.settings
    }

    /// Copies the changes made to the settings since the last call into the pipeline's own
    /// slice (see [`ScriptSettings::sync`]).
    pub fn sync_settings(&mut self, state: &mut ConversionState) {
        let view_reset = self.data.settle_settings(Some(state.settings_mut()));
        self.answer_view_reset(view_reset);
    }

    /// The bridge through which a host lends its context to the hooks of this runtime.
    #[must_use]
    pub fn bridge(&self) -> ScriptBridge {
        ScriptBridge {
            lua: self.lua.clone(),
        }
    }

    /// Runs `operation` on the context the host lent, outside any call into Lua. A failure of
    /// the host side is logged and gives `None`.
    fn with_context<R: 'static>(
        &mut self,
        operation: impl FnOnce(&mut ScriptContext<'_>) -> R + 'static,
    ) -> Option<R> {
        match with_host_context(&self.lua, &mut self.data, operation) {
            Ok(answer) => Some(answer),
            Err(error) => {
                self.data
                    .env
                    .log
                    .error(&format!("Lua: the runtime failed: {}", message_of(&error)));
                None
            }
        }
    }

    fn answer_view_reset(&mut self, requested: bool) {
        if requested {
            self.with_context(|context| context.camera.reset_view());
        }
    }

    /// The frame counter the last [`ScriptHost::set_frame_globals`] published.
    #[must_use]
    pub const fn frame_counter(&self) -> i32 {
        self.data.frame_counter
    }

    /// Whether the script pad is on (`ScriptPad:Enable`, REQ-LUA-091).
    #[must_use]
    pub const fn script_pad_enabled(&self) -> bool {
        self.data.script_pad_enabled
    }

    /// Which player the script pad feeds, 0 or 1 (REQ-LUA-091).
    #[must_use]
    pub const fn script_pad_index(&self) -> i32 {
        self.data.script_pad_index
    }

    /// The last value `InputProcess` returned (REQ-LUA-032).
    #[must_use]
    pub const fn script_pad_state(&self) -> i32 {
        self.data.script_pad_state
    }

    /// The shape the subjective camera follows (`Camera.FpvShape`), which the renderer needs to
    /// find the occluders (SPEC-06 REQ-SCN-022).
    #[must_use]
    pub fn fpv_shape(&self) -> Option<ShapeId> {
        self.data.fpv_shape.map(|target| target.id)
    }

    /// How long one hook call may run before it is cut off (REQ-LUA-064).
    #[must_use]
    pub const fn hook_budget(&self) -> Duration {
        self.data.hook_budget
    }

    /// Sets how long one hook call may run before it is cut off.
    pub fn set_hook_budget(&mut self, budget: Duration) {
        self.data.hook_budget = budget;
    }

    // lifecycle hooks (REQ-LUA-020, 021, 071, 072)

    /// Calls `Start` of every script that is not in error, active or not, in script order
    /// (REQ-LUA-020).
    pub fn invoke_start(&mut self, state: &mut ConversionState) {
        let retired = self.data.invalidate_handles();
        let result = self.run(Some(state), None, |cx| {
            for handles in retired {
                cx.retire(handles)?;
            }
            call_all(cx, Audience::NotInError, ScriptHook::Start)
        });
        self.settle(result);
    }

    /// Calls `End` of every script that is not in error, active or not, in script order
    /// (REQ-LUA-021).
    pub fn invoke_end(&mut self, state: &mut ConversionState) {
        let retired = self.data.invalidate_handles();
        let result = self.run(Some(state), None, |cx| {
            for handles in retired {
                cx.retire(handles)?;
            }
            call_all(cx, Audience::NotInError, ScriptHook::End)
        });
        self.settle(result);
    }

    /// Calls `Save` of every active script, in script order (REQ-LUA-071).
    pub fn invoke_save(&mut self, state: &mut ConversionState) {
        let result = self.run(Some(state), None, |cx| {
            call_all(cx, Audience::Active, ScriptHook::Save)
        });
        self.settle(result);
    }

    /// Calls `Load` of every active script, in script order (REQ-LUA-072).
    pub fn invoke_load(&mut self, state: &mut ConversionState) {
        let result = self.run(Some(state), None, |cx| {
            call_all(cx, Audience::Active, ScriptHook::Load)
        });
        self.settle(result);
    }

    /// Calls `InputProcess` of the script that owns the script pad and returns the action mask
    /// to or into the player's (REQ-LUA-032, SPEC-08 REQ-INP-071).
    pub fn invoke_input_process(
        &mut self,
        state: &mut ConversionState,
        frame_counter: i32,
        time: f32,
    ) -> i32 {
        if !self.data.script_pad_enabled
            || self
                .data
                .input_process_script
                .is_none_or(|index| !self.data.scripts[index].active())
        {
            self.data.script_pad_state = 0;
            return 0;
        }

        if let Err(error) = self.publish_frame_globals(frame_counter, time) {
            self.settle(Err(error));
            return 0;
        }

        match self
            .run(Some(state), None, input_process)
            .and_then(|inner| inner)
        {
            Ok(mask) => mask,
            Err(error) => {
                self.settle(Err(error));
                0
            }
        }
    }

    // persistent data (REQ-LUA-070 to 073)

    /// The 64 entries `PData[1]` to `PData[64]` as single-precision reals, the block a
    /// savestate carries (REQ-LUA-071, REQ-LUA-073).
    ///
    /// # Errors
    ///
    /// The Lua state could not be read.
    pub fn capture_persistent_data(
        &self,
    ) -> Result<[u8; persistent_data::BLOCK_SIZE], LuaScriptError> {
        let mut values = [0.0_f32; persistent_data::ENTRY_COUNT];
        let read = |error: mlua::Error| LuaScriptError(message_of(&error));
        if let Value::Table(table) = self.lua.globals().get::<Value>("PData").map_err(read)? {
            for (offset, value) in values.iter_mut().enumerate() {
                let stored: Value = table.raw_get(offset + 1).map_err(read)?;
                *value = stored_float(&stored);
            }
        }

        Ok(persistent_data::write(&values))
    }

    /// Puts the 64 entries back as Lua floats, which a savestate load does before `Start`
    /// (REQ-LUA-072).
    ///
    /// # Errors
    ///
    /// The Lua state could not be written.
    pub fn restore_persistent_data(&self, block: &[u8]) -> Result<(), LuaScriptError> {
        let values = persistent_data::read(block);
        let failed = |error: mlua::Error| LuaScriptError(message_of(&error));
        if let Value::Table(table) = self.lua.globals().get::<Value>("PData").map_err(failed)? {
            for (offset, value) in values.iter().enumerate() {
                table
                    .raw_set(offset + 1, f64::from(*value))
                    .map_err(failed)?;
            }
        }

        Ok(())
    }

    /// The current value of `PData[index]`, 0 when it is not a number.
    ///
    /// # Errors
    ///
    /// The Lua state could not be read.
    pub fn read_persistent_data(&self, index: i64) -> Result<f64, LuaScriptError> {
        let failed = |error: mlua::Error| LuaScriptError(message_of(&error));
        match self.lua.globals().get::<Value>("PData").map_err(failed)? {
            Value::Table(table) => match table.raw_get::<Value>(index).map_err(failed)? {
                Value::Number(number) => Ok(number),
                #[allow(clippy::cast_precision_loss)]
                // lua_tonumber converts an integer to a double.
                Value::Integer(integer) => Ok(integer as f64),
                _ => Ok(0.0),
            },
            _ => Ok(0.0),
        }
    }

    // camera tracking (REQ-LUA-088, SPEC-06 REQ-SCN-021)

    /// Moves the subjective camera onto the followed shape for the frame being converted: in
    /// FPV, the tracker of SPEC-04 finds the shape again and the target follows it; when the
    /// tracker loses it, the follow stops and the target stays where it was.
    pub fn advance_camera_tracking(&mut self, state: &mut ConversionState) {
        if self.data.fpv_shape.is_none() {
            return;
        }
        if self.with_context(|context| context.camera.mode()) != Some(1) {
            return;
        }

        let data = &mut self.data;
        let found = state.track(FrameRole::Converting).filter(|&id| {
            state
                .converting_frame()
                .get_shape(id)
                .is_some_and(|shape| !shape.instances().is_empty())
        });
        let Some(id) = found else {
            data.fpv_shape = None;
            state.tracker_mut().stop();
            return;
        };

        data.fpv_shape = Some(ShapeTarget {
            key: data.converting_key(),
            id,
        });
        let frame = state.converting_frame();
        let position = frame.shape(id).instances()[0].get_pos(state.settings());
        self.with_context(move |context| context.camera.set_look_at(position));
    }

    // evaluation, for the tests and the acceptance harness

    /// Runs a chunk in the profile's state. The chunk sees exactly what a profile script sees.
    ///
    /// # Errors
    ///
    /// The chunk failed to compile or raised an error.
    pub fn evaluate(
        &mut self,
        state: &mut ConversionState,
        chunk: &str,
    ) -> Result<(), LuaScriptError> {
        self.evaluate_with(state, chunk, |_, _| ())
    }

    /// Runs a chunk that returns one value and gives it back as a number; `NaN` when the value
    /// is not one.
    ///
    /// # Errors
    ///
    /// The chunk failed to compile or raised an error.
    pub fn evaluate_number(
        &mut self,
        state: &mut ConversionState,
        chunk: &str,
    ) -> Result<f64, LuaScriptError> {
        self.evaluate_with(state, chunk, |_, results| match results.back() {
            Some(Value::Number(number)) => *number,
            #[allow(clippy::cast_precision_loss)] // lua_tonumber converts an integer to a double.
            Some(Value::Integer(integer)) => *integer as f64,
            _ => f64::NAN,
        })
    }

    /// Runs a chunk that returns one value and gives it back as text.
    ///
    /// # Errors
    ///
    /// The chunk failed to compile or raised an error.
    pub fn evaluate_text(
        &mut self,
        state: &mut ConversionState,
        chunk: &str,
    ) -> Result<String, LuaScriptError> {
        self.evaluate_with(state, chunk, |lua, results| {
            text_of(lua, results.back().unwrap_or(&Value::Nil))
        })
    }

    fn evaluate_with<R>(
        &mut self,
        state: &mut ConversionState,
        chunk: &str,
        read: impl FnOnce(&Lua, &MultiValue) -> R,
    ) -> Result<R, LuaScriptError> {
        let limit = self.data.hook_budget;
        let outcome = self.run(Some(state), None, |cx| {
            let function = cx.lua.load(chunk).set_name(chunk).into_function()?;
            let results = with_budget(cx.lua, limit, || protected_call(cx.lua, &function, ()))?;
            Ok::<_, mlua::Error>(results.map(|results| read(cx.lua, &results)))
        });

        match outcome.and_then(|inner| inner) {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(message)) => Err(LuaScriptError(message)),
            Err(error) => Err(LuaScriptError(message_of(&error))),
        }
    }

    fn publish_frame_globals(&mut self, frame_counter: i32, time: f32) -> mlua::Result<()> {
        self.data.frame_counter = frame_counter;
        let retired = self.data.advance_epoch();
        Cx::new(&self.lua).retire(retired)?;
        let globals = self.lua.globals();
        globals.set("frame", f64::from(frame_counter))?;
        globals.set("time", f64::from(time))
    }
}

/// A numeric string stores its value, a number its value, anything else 0 (REQ-LUA-071).
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)] // The block holds singles.
fn stored_float(value: &Value) -> f32 {
    match value {
        Value::Number(number) => *number as f32,
        Value::Integer(integer) => *integer as f32,
        Value::String(text) => text
            .to_str()
            .ok()
            .and_then(|text| text.trim().parse::<f64>().ok())
            .map_or(0.0, |parsed| parsed as f32),
        _ => 0.0,
    }
}

/// Runs the chunks of the profile in file order and records the hooks they define
/// (REQ-LUA-011). Returns whether any script defines a frame hook.
fn load_scripts(cx: &Cx, profile: &GameProfile) -> mlua::Result<bool> {
    let mut loaded: Vec<Option<LoadedScript>> = Vec::new();
    let mut by_name: HashMap<String, usize> = HashMap::new();
    let limit = cx.data(|data| data.hook_budget)?;

    for (file_index, source) in profile.scripts.iter().enumerate() {
        let mut script = LoadedScript::new(source, file_index);
        if let Some(&previous) = by_name.get(script.name()) {
            // REQ-LUA-003: keep the last one read and say so in the log.
            let line = format!(
                "Lua: the profile carries two scripts named '{}'; the last one is kept.",
                script.name()
            );
            cx.data(|data| data.env.log.warning(&line))?;
            loaded[previous] = None;
        }

        compile(
            cx,
            &mut script,
            source.source.as_deref().unwrap_or_default(),
            limit,
        )?;
        by_name.insert(script.name().to_owned(), loaded.len());
        loaded.push(Some(script));
    }

    let mut scripts: Vec<LoadedScript> = loaded.into_iter().flatten().collect();
    scripts
        .sort_by(|left, right| script_order::compare_names(Some(left.name()), Some(right.name())));

    let mut file_order: Vec<usize> = (0..scripts.len()).collect();
    file_order.sort_by_key(|&position| scripts[position].file_index());

    // REQ-LUA-032: the script pad comes from the last script loaded that defines InputProcess,
    // the file order deciding.
    let input_process_script = file_order
        .iter()
        .rev()
        .copied()
        .find(|&position| scripts[position].defines(ScriptHook::InputProcess));
    let has_frame_hooks = scripts.iter().any(|script| {
        script.defines(ScriptHook::Update)
            || script.defines(ScriptHook::UpdateS)
            || script.defines(ScriptHook::LateUpdate)
    });

    cx.data(|data| {
        data.scripts = scripts;
        data.file_order = file_order;
        data.input_process_script = input_process_script;
    })?;
    Ok(has_frame_hooks)
}

/// Compiles, runs once and harvests the hooks of one script (REQ-LUA-011): (a) the ten hook
/// globals go to nil, so that a script never inherits another's hook; (b) the chunk runs once,
/// and the globals it defined before an error stay in place (REQ-LUA-060); (c) the hook
/// globals that hold a function become this script's hooks.
fn compile(cx: &Cx, script: &mut LoadedScript, source: &str, limit: Duration) -> mlua::Result<()> {
    let globals = cx.lua.globals();
    for hook in ScriptHook::ALL {
        globals.set(hook.name(), Value::Nil)?;
    }

    let chunk_name = if script.name().is_empty() {
        "=script".to_owned()
    } else {
        format!("={}", script.name())
    };
    let function = match cx.lua.load(source).set_name(chunk_name).into_function() {
        Ok(function) => function,
        Err(error) => {
            script.failed = true;
            let line = format!(
                "Lua: script '{}' failed to compile: {}",
                script.name(),
                message_of(&error)
            );
            cx.data(|data| data.env.log.error(&line))?;
            return Ok(());
        }
    };

    if let Err(message) = with_budget(cx.lua, limit, || protected_call(cx.lua, &function, ()))? {
        script.failed = true;
        let line = format!("Lua: script '{}' failed to run: {message}", script.name());
        cx.data(|data| data.env.log.error(&line))?;
        return Ok(());
    }

    for hook in ScriptHook::ALL {
        if let Value::Function(function) = globals.get::<Value>(hook.name())? {
            script.set_hook(hook, function);
        }
    }

    Ok(())
}

/// Calls `BuildPattern` of the scripts at `linked`, which are linked to the pattern instance
/// `reference` being built.
fn build_pattern(
    cx: &Cx,
    reference: InstanceRef,
    linked: &[usize],
) -> mlua::Result<Option<String>> {
    let mut first = None;
    for &index in linked {
        // A script turned off by an earlier one of this call gets no hook.
        if !cx.data(|data| data.scripts[index].active())? {
            continue;
        }

        publish(
            cx.lua,
            Published::Pattern,
            cx.object(Payload::Instance(InstanceTarget::Pattern(reference)))?,
        )?;
        first = first.or(call_hook(cx, index, ScriptHook::BuildPattern)?);
    }

    Ok(first)
}

/// Makes the writes to the host the script queued.
fn flush_host_writes(lua: &Lua) -> mlua::Result<()> {
    lua.app_data_ref::<HostLink>()
        .map_or(Ok(()), |link| link.flush())
}

impl ScriptHost for ProfileScriptRuntime {
    fn has_frame_hooks(&self) -> bool {
        self.data.has_frame_hooks
    }

    fn set_frame_globals(&mut self, frame_counter: i32, time: f32) -> Result<(), ScriptError> {
        self.publish_frame_globals(frame_counter, time)
            .map_err(|error| ScriptError(message_of(&error)))
    }

    fn invoke_update(&mut self, state: &mut ConversionState) -> Result<(), ScriptError> {
        let result = self.run(Some(state), None, |cx| {
            call_all(cx, Audience::Active, ScriptHook::Update)
        });
        self.finish(result)
    }

    fn invoke_update_shapes(&mut self, state: &mut ConversionState) -> Result<(), ScriptError> {
        let result = self.run(Some(state), None, update_shapes);
        self.finish(result)
    }

    fn invoke_late_update(&mut self, state: &mut ConversionState) -> Result<(), ScriptError> {
        let result = self.run(Some(state), None, |cx| {
            call_all(cx, Audience::Active, ScriptHook::LateUpdate)
        });
        self.finish(result)
    }

    fn invoke_build_pattern(
        &mut self,
        reference: InstanceRef,
        instance: &mut PatternInstance,
    ) -> Result<(), ScriptError> {
        let mut linked = Vec::new();
        self.data.scripts_building(instance, &mut linked);
        if linked.is_empty() {
            return Ok(());
        }

        let result = self.run(None, Some((reference, instance)), |cx| {
            build_pattern(cx, reference, &linked)
        });
        self.finish(result)
    }

    fn invoke_build_patterns(&mut self, pattern: PatternId, instances: &mut [PatternInstance]) {
        let mut linked = Vec::new();
        if !instances.iter().any(|instance| {
            self.data.scripts_building(instance, &mut linked);
            !linked.is_empty()
        }) {
            return;
        }

        // The runtime and the scope are lent once for all the instances; each one is lent in
        // turn as the instance being built.
        let Self { lua, data } = self;
        let outcome = lua.scope(|scope| {
            let data = scope.create_any_userdata_ref_mut(data)?;
            let mut failure = None;
            lua.set_app_data(Lent {
                data,
                state: None,
                build: None,
            });
            let cx = Cx::new(lua);
            for (index, instance) in instances.iter_mut().enumerate() {
                let reference = InstanceRef { pattern, index };
                // The scripts are looked up just before the instance is built, as a script
                // built an earlier instance may have changed the tags of this one.
                let counted = cx.data(|data| data.scripts_building(instance, &mut linked));
                if let Err(error) = counted {
                    failure.get_or_insert(error);
                    continue;
                }

                if linked.is_empty() {
                    continue;
                }

                let build = scope.create_any_userdata_ref_mut(instance)?;
                if let Some(mut lent) = lua.app_data_mut::<Lent>() {
                    lent.build = Some((reference, build));
                }

                // The script that raised an error was logged and disabled by the hook call.
                if let Err(error) = build_pattern(&cx, reference, &linked) {
                    failure.get_or_insert(error);
                }
            }

            let flushed = flush_host_writes(lua);
            lua.remove_app_data::<Lent>();
            flushed?;
            Ok(failure)
        });
        if let Some(error) = outcome.unwrap_or_else(Some) {
            self.settle(Err(error));
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl ProfileScriptRuntime {
    /// The result of a hook sequence for the pipeline: the first error a script raised.
    fn finish(
        &mut self,
        result: mlua::Result<mlua::Result<Option<String>>>,
    ) -> Result<(), ScriptError> {
        match self.settle(result) {
            None => Ok(()),
            Some(message) => Err(ScriptError(message)),
        }
    }
}
