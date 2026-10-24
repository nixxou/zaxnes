//! What the conversion pipeline needs from the Lua runtime (REQ-E3D-320 to 324) and the rule
//! that links a script to an instance (REQ-E3D-321).

use std::any::Any;

use zax_profiles::tags;

use crate::engine::ConversionState;
use crate::instance::{InstanceRef, PatternInstance};
use crate::registry::PatternId;

/// An error a script hook raised. The pipeline counts it and carries on, because a Lua error
/// must not interrupt the conversion (REQ-E3D-324).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct ScriptError(pub String);

/// What the conversion pipeline needs from the Lua runtime: the hook call points of
/// REQ-E3D-320 to 324, in the order the pipeline calls them.
///
/// The pipeline owns *when* the hooks run; the runtime (SPEC-05) owns the order of the
/// scripts, the per-script lists and the error handling. The frame hooks receive the engine
/// state, which is what the script API reads and changes (SPEC-05 REQ-LUA-084 to 086).
///
/// A hook may fail: the pipeline counts the error in
/// [`ConversionFrame::script_error_count`](crate::frame::ConversionFrame::script_error_count)
/// and carries on (REQ-E3D-324).
///
/// `Send`: the engine, runtime included, may be handed to another thread for the length of a
/// frame (the host's pipelined frames).
pub trait ScriptHost: Send {
    /// True when at least one script defines a frame hook. The pipeline uses it only to skip
    /// work; the result must not depend on it.
    fn has_frame_hooks(&self) -> bool;

    /// Step 1: publishes the frame counter and the frame time as the globals `frame` and
    /// `time` (REQ-E3D-320, SPEC-05 REQ-LUA-025).
    ///
    /// # Errors
    ///
    /// The error a script raised.
    fn set_frame_globals(&mut self, frame_counter: i32, time: f32) -> Result<(), ScriptError>;

    /// Step 2: calls `Update` of every active script, in script order.
    ///
    /// # Errors
    ///
    /// The error a script raised.
    fn invoke_update(&mut self, state: &mut ConversionState) -> Result<(), ScriptError>;

    /// Steps 3 and 4: builds the per-script list of linked instances out of the scriptable
    /// instances of the frame being converted, in the order of REQ-E3D-220
    /// (`state.converting_frame().scriptable_instances()`, REQ-E3D-321), and calls `UpdateS`
    /// once per instance, grouped by script (SPEC-05 REQ-LUA-025 steps 3 and 4).
    ///
    /// # Errors
    ///
    /// The error a script raised.
    fn invoke_update_shapes(&mut self, state: &mut ConversionState) -> Result<(), ScriptError>;

    /// Step 5: calls `LateUpdate` of every active script, in script order.
    ///
    /// # Errors
    ///
    /// The error a script raised.
    fn invoke_late_update(&mut self, state: &mut ConversionState) -> Result<(), ScriptError>;

    /// Calls `BuildPattern` for every active script linked to the pattern instance
    /// `reference`, before its geometry is built (REQ-E3D-322, SPEC-05 REQ-LUA-030).
    /// Modifications the hook makes are permanent for the pattern.
    ///
    /// # Errors
    ///
    /// The error a script raised.
    fn invoke_build_pattern(
        &mut self,
        reference: InstanceRef,
        instance: &mut PatternInstance,
    ) -> Result<(), ScriptError>;

    /// Calls `BuildPattern` for each of `instances`, the instances of the pattern `pattern` in
    /// order, as [`ScriptHost::invoke_build_pattern`] does for one. A Lua error must not stop the
    /// conversion (REQ-E3D-324): the host disables the faulty script (SPEC-05 §3.8), and the
    /// instances that follow are still built.
    ///
    /// A host that has work to share between the instances overrides it.
    fn invoke_build_patterns(&mut self, pattern: PatternId, instances: &mut [PatternInstance]) {
        for (index, instance) in instances.iter_mut().enumerate() {
            // The error is already logged by the host and the script disabled.
            let _ = self.invoke_build_pattern(InstanceRef { pattern, index }, instance);
        }
    }

    /// The host as [`Any`], so that the application can reach its concrete type.
    fn as_any(&self) -> &dyn Any;

    /// The host as [`Any`], for writing.
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// The host behind this one when it wraps another (a timing wrapper), this host otherwise:
    /// what a host lends its context to.
    fn inner(&self) -> &dyn Any {
        self.as_any()
    }
}

/// A script host that does nothing, used when no profile script is loaded and by the tests of
/// the pipeline alone.
#[derive(Debug, Clone, Copy, Default)]
pub struct NullScriptHost;

impl ScriptHost for NullScriptHost {
    fn has_frame_hooks(&self) -> bool {
        false
    }

    fn set_frame_globals(&mut self, _frame_counter: i32, _time: f32) -> Result<(), ScriptError> {
        Ok(())
    }

    fn invoke_update(&mut self, _state: &mut ConversionState) -> Result<(), ScriptError> {
        Ok(())
    }

    fn invoke_update_shapes(&mut self, _state: &mut ConversionState) -> Result<(), ScriptError> {
        Ok(())
    }

    fn invoke_late_update(&mut self, _state: &mut ConversionState) -> Result<(), ScriptError> {
        Ok(())
    }

    fn invoke_build_pattern(
        &mut self,
        _reference: InstanceRef,
        _instance: &mut PatternInstance,
    ) -> Result<(), ScriptError> {
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// True when a script whose tag string is `script_tags` is linked to `instance` (REQ-E3D-321,
/// SPEC-05 REQ-LUA-005, REQ-LUA-006): the string contains `*` anywhere, or one of its tags
/// equals one of the instance's (exact, case-sensitive).
///
/// The Lua runtime uses this to build the per-script lists the pipeline asks for in
/// [`ScriptHost::invoke_update_shapes`]. It lives here because REQ-E3D-321 states the rule and
/// the pipeline's ordering depends on it.
#[must_use]
pub fn is_linked(script_tags: Option<&str>, instance: &PatternInstance) -> bool {
    let Some(script_tags) = script_tags else {
        return false;
    };

    script_tags.contains(tags::WILDCARD)
        || tags::parse(Some(script_tags))
            .into_iter()
            .any(|tag| instance.has_tag(tag))
}
