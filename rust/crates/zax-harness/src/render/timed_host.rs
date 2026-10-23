//! The Lua runtime with the frame hooks timed, which is the budget of REQ-LUA-110: steps 1 to 5
//! of REQ-LUA-025, not the conversion around them (port of `TimedScriptHost`).

use std::any::Any;
use std::time::Instant;

use zax_engine3d::engine::ConversionState;
use zax_engine3d::instance::{InstanceRef, PatternInstance};
use zax_engine3d::registry::PatternId;
use zax_engine3d::script::{ScriptError, ScriptHost};
use zax_scripting::ProfileScriptRuntime;

/// A [`ProfileScriptRuntime`] that records how long the hooks of each frame take.
pub struct TimedHost {
    /// The runtime every hook is forwarded to.
    pub runtime: ProfileScriptRuntime,
    per_frame: Vec<f64>,
    current: f64,
}

impl TimedHost {
    /// Wraps `runtime`.
    #[must_use]
    pub const fn new(runtime: ProfileScriptRuntime) -> Self {
        Self {
            runtime,
            per_frame: Vec::new(),
            current: 0.0,
        }
    }

    /// Frames measured.
    #[must_use]
    pub fn frames(&self) -> usize {
        self.per_frame.len()
    }

    /// Mean hook time of a frame, in milliseconds.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // A frame count is far below 2^52.
    pub fn mean(&self) -> f64 {
        if self.per_frame.is_empty() {
            0.0
        } else {
            self.per_frame.iter().sum::<f64>() / self.per_frame.len() as f64
        }
    }

    /// 99th percentile of the hook time of a frame, in milliseconds.
    #[must_use]
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )] // A frame count is far below 2^52 and the rank is within the vector.
    pub fn p99(&self) -> f64 {
        let mut sorted = self.per_frame.clone();
        sorted.sort_by(f64::total_cmp);
        let rank = (sorted.len() as f64 * 0.99).ceil() as usize;
        sorted
            .get(rank.saturating_sub(1).min(sorted.len().saturating_sub(1)))
            .copied()
            .unwrap_or(0.0)
    }

    fn measure<R>(&mut self, step: impl FnOnce(&mut ProfileScriptRuntime) -> R) -> R {
        let before = Instant::now();
        let result = step(&mut self.runtime);
        self.current += before.elapsed().as_secs_f64() * 1000.0;
        result
    }
}

impl ScriptHost for TimedHost {
    fn has_frame_hooks(&self) -> bool {
        self.runtime.has_frame_hooks()
    }

    fn set_frame_globals(&mut self, frame_counter: i32, time: f32) -> Result<(), ScriptError> {
        // Step 1 opens the frame, so the accumulator starts here.
        self.current = 0.0;
        self.measure(|runtime| runtime.set_frame_globals(frame_counter, time))
    }

    fn invoke_update(&mut self, state: &mut ConversionState) -> Result<(), ScriptError> {
        self.measure(|runtime| runtime.invoke_update(state))
    }

    fn invoke_update_shapes(&mut self, state: &mut ConversionState) -> Result<(), ScriptError> {
        self.measure(|runtime| runtime.invoke_update_shapes(state))
    }

    fn invoke_late_update(&mut self, state: &mut ConversionState) -> Result<(), ScriptError> {
        let result = self.measure(|runtime| runtime.invoke_late_update(state));
        // Step 5 closes the frame.
        self.per_frame.push(self.current);
        result
    }

    fn invoke_build_pattern(
        &mut self,
        reference: InstanceRef,
        instance: &mut PatternInstance,
    ) -> Result<(), ScriptError> {
        self.runtime.invoke_build_pattern(reference, instance)
    }

    fn invoke_build_patterns(&mut self, pattern: PatternId, instances: &mut [PatternInstance]) {
        self.runtime.invoke_build_patterns(pattern, instances);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
