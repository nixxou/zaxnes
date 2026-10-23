//! The hook call points of SPEC-05 §3.3 to §3.5: which scripts receive a hook, in which order,
//! and what an error in a hook does (REQ-LUA-060 to REQ-LUA-064).

use std::cell::Cell;
use std::time::{Duration, Instant};

use mlua::{Lua, MultiValue, Value};
use zax_engine3d::frame::FrameInstanceId;

use super::globals::{protected_call, protected_hook_call};
use crate::binding::cx::Cx;
use crate::binding::payload::{InstanceTarget, Payload, ShapeTarget};
use crate::script::ScriptHook;
use crate::values::number_format;

/// The execution budget of the hook being run (REQ-LUA-064).
pub(crate) struct Budget {
    deadline: Cell<Option<Instant>>,
}

impl Budget {
    pub const fn new() -> Self {
        Self {
            deadline: Cell::new(None),
        }
    }

    /// True once the deadline of the running hook has passed.
    pub fn exceeded(&self) -> bool {
        self.deadline
            .get()
            .is_some_and(|deadline| Instant::now() > deadline)
    }

    /// Lifts the deadline: whatever runs next is not cut off by the hook that just ended.
    pub fn lift(&self) {
        self.deadline.set(None);
    }
}

/// Runs `operation` under the execution budget of REQ-LUA-064: a script that never gives
/// control back is cut off as if it had raised an error, so the emulation resumes.
pub(crate) fn with_budget<R>(lua: &Lua, limit: Duration, operation: impl FnOnce() -> R) -> R {
    if let Some(budget) = lua.app_data_ref::<Budget>() {
        budget.deadline.set(Some(Instant::now() + limit));
    }

    let result = operation();
    if let Some(budget) = lua.app_data_ref::<Budget>() {
        budget.lift();
    }

    result
}

/// Marks the script in error and logs it once (REQ-LUA-061, REQ-LUA-062).
fn fail(cx: &Cx, index: usize, hook: ScriptHook, message: &str) -> mlua::Result<()> {
    cx.data(|data| {
        data.scripts[index].failed = true;
        let line = format!(
            "Lua: script '{}' hook {} raised: {message}",
            data.scripts[index].name(),
            hook.name()
        );
        data.env.log.error(&line);
    })
}

/// Calls `hook` of the script at `index`. An absent hook is simply not called (REQ-LUA-022).
/// Returns the text of the error the hook raised.
pub(crate) fn call_hook(cx: &Cx, index: usize, hook: ScriptHook) -> mlua::Result<Option<String>> {
    let (function, limit) =
        cx.data(|data| (data.scripts[index].hook(hook).cloned(), data.hook_budget))?;
    let Some(function) = function else {
        return Ok(None);
    };

    cx.lua
        .globals()
        .set("self", cx.object(Payload::Script(index))?)?;
    match with_budget(cx.lua, limit, || protected_hook_call(cx.lua, &function))? {
        Ok(()) => Ok(None),
        Err(message) => {
            fail(cx, index, hook, &message)?;
            Ok(Some(message))
        }
    }
}

/// Which scripts a lifecycle hook goes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Audience {
    /// Every script that is not in error, enabled or not (`Start` and `End`, REQ-LUA-020, 021).
    NotInError,
    /// Every active script.
    Active,
}

/// Calls `hook` of every script of `audience`, in script order. The state of a script is read
/// just before its call, so a script turned off by an earlier one gets no hook (REQ-LUA-007).
/// Returns the first error a script raised; the others still receive the hook.
pub(crate) fn call_all(
    cx: &Cx,
    audience: Audience,
    hook: ScriptHook,
) -> mlua::Result<Option<String>> {
    let count = cx.data(|data| data.scripts.len())?;
    let mut first = None;
    for index in 0..count {
        let receives = cx.data(|data| match audience {
            Audience::NotInError => !data.scripts[index].failed(),
            Audience::Active => data.scripts[index].active(),
        })?;
        if receives {
            first = first.or(call_hook(cx, index, hook)?);
        }
    }

    Ok(first)
}

/// Steps 3 and 4 of REQ-LUA-025: the per-script lists of linked instances, then the `UpdateS`
/// calls grouped per script.
pub(crate) fn update_shapes(cx: &Cx) -> mlua::Result<Option<String>> {
    // Step 3: the per-script lists, in the instance order of REQ-E3D-220. A script turned off
    // during step 2 gets no UpdateS at all.
    let any = cx.data(|data| {
        let mut any = false;
        for script in &mut data.scripts {
            script.pending.clear();
            any |= script.active() && script.defines(ScriptHook::UpdateS);
        }

        any
    })?;
    if !any {
        return Ok(None);
    }

    let epoch = cx.try_state(|state| {
        cx.data(|data| {
            let frame = state.converting_frame();
            for &id in frame.scriptable_instances() {
                let instance = frame.instance(id);
                for script in &mut data.scripts {
                    if script.active()
                        && script.defines(ScriptHook::UpdateS)
                        && script.is_linked_to(instance)
                    {
                        script.pending.push(id);
                    }
                }
            }

            data.epoch
        })
    })?;

    // Step 4: the calls are grouped per script, all of its instances then the next script. A
    // script turned off during this step keeps the calls left in its list; only an error drops
    // them (REQ-LUA-025 notes, REQ-LUA-061).
    let count = cx.data(|data| data.scripts.len())?;
    let mut first = None;
    for index in 0..count {
        let pending = cx.data(|data| {
            let script = &mut data.scripts[index];
            (!script.failed()).then(|| std::mem::take(&mut script.pending))
        })?;
        let Some(pending) = pending else {
            continue;
        };

        let key = crate::binding::payload::FrameKey {
            pinned: false,
            epoch,
        };
        for id in &pending {
            if cx.data(|data| data.scripts[index].failed())? {
                break;
            }

            first = first.or(call_update_s(cx, index, key, *id)?);
        }

        cx.data(|data| {
            let script = &mut data.scripts[index];
            script.pending = pending;
            script.pending.clear();
        })?;
    }

    Ok(first)
}

fn call_update_s(
    cx: &Cx,
    index: usize,
    key: crate::binding::payload::FrameKey,
    id: FrameInstanceId,
) -> mlua::Result<Option<String>> {
    let globals = cx.lua.globals();
    globals.set(
        "shape",
        cx.object(Payload::Instance(InstanceTarget::Frame(key, id)))?,
    )?;
    globals.set(
        "shape2D",
        cx.object(Payload::Shape(ShapeTarget { key, id: id.shape }))?,
    )?;
    call_hook(cx, index, ScriptHook::UpdateS)
}

/// Calls `InputProcess` of the script that owns the script pad and returns the action mask to
/// or into the player's (REQ-LUA-032, SPEC-08 REQ-INP-071). Anything that is not a number
/// contributes 0.
pub(crate) fn input_process(cx: &Cx) -> mlua::Result<i32> {
    let selected = cx.data(|data| {
        let selected = data
            .input_process_script
            .filter(|&index| data.script_pad_enabled && data.scripts[index].active());
        if let Some(index) = selected {
            Some((
                data.scripts[index].hook(ScriptHook::InputProcess).cloned(),
                data.hook_budget,
                data.script_pad_index,
                index,
            ))
        } else {
            data.script_pad_state = 0;
            None
        }
    })?;
    let Some((Some(function), limit, pad_index, name)) = selected else {
        return Ok(0);
    };

    let outcome = with_budget(cx.lua, limit, || {
        protected_call(cx.lua, &function, f64::from(pad_index))
    })?;
    let mask = match outcome {
        Ok(results) => mask_of(&results),
        Err(message) => {
            fail(cx, name, ScriptHook::InputProcess, &message)?;
            0
        }
    };

    cx.data(|data| data.script_pad_state = mask)?;
    Ok(mask)
}

fn mask_of(results: &MultiValue) -> i32 {
    match results.front() {
        Some(Value::Number(number)) => number_format::to_i32(*number),
        Some(Value::Integer(integer)) => {
            i32::try_from(*integer).unwrap_or(if *integer < 0 { i32::MIN } else { i32::MAX })
        }
        _ => 0,
    }
}
