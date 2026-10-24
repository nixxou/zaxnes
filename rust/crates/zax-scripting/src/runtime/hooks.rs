//! The hook call points of SPEC-05 §3.3 to §3.5: which scripts receive a hook, in which order,
//! and what an error in a hook does (REQ-LUA-060 to REQ-LUA-064).

use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use mlua::{Function, Lua, MultiValue, Table, Value};
use zax_engine3d::frame::FrameInstanceId;

use super::globals::{protected_call, protected_hook_call};
use crate::binding::cx::Cx;
use crate::binding::payload::{InstanceTarget, Payload, ShapeTarget};
use crate::script::ScriptHook;
use crate::values::number_format;

/// Whether the hook calls are timed per script (the frame profiler of the application). Read
/// before each call, so that the timing costs one load while it is off.
pub(crate) static TIMED: AtomicBool = AtomicBool::new(false);

/// The time and the number of calls of each hook of each script since the last take, kept in
/// the Lua state's app data while the timing is on.
#[derive(Debug, Default)]
pub(crate) struct HookTimes(pub Vec<[(Duration, u32); ScriptHook::COUNT]>);

/// Runs `call`, a call of `hook` of the script at `index`, timing it when [`TIMED`].
fn timed<R>(cx: &Cx, index: usize, hook: ScriptHook, call: impl FnOnce() -> R) -> R {
    if !TIMED.load(Ordering::Relaxed) {
        return call();
    }
    let started = Instant::now();
    let result = call();
    let elapsed = started.elapsed();
    if let Some(mut times) = cx.lua.app_data_mut::<HookTimes>() {
        if times.0.len() <= index {
            times
                .0
                .resize(index + 1, [(Duration::ZERO, 0); ScriptHook::COUNT]);
        }
        let slot = &mut times.0[index][hook as usize];
        slot.0 += elapsed;
        slot.1 += 1;
    }
    result
}

/// The execution budget of the hook being run (REQ-LUA-064).
///
/// The clock is not read when a hook starts: most hooks end before the first look at the budget,
/// which is the first count of VM instructions that passes, and the time of that look is when
/// the budget starts. A hook that never ends is cut off one such count late, which is a
/// millisecond.
pub(crate) struct Budget {
    limit: Cell<Option<Duration>>,
    started: Cell<Option<Instant>>,
}

impl Budget {
    pub const fn new() -> Self {
        Self {
            limit: Cell::new(None),
            started: Cell::new(None),
        }
    }

    /// Starts the budget of a hook that may run for `limit`.
    fn arm(&self, limit: Duration) {
        self.limit.set(Some(limit));
        self.started.set(None);
    }

    /// True once the running hook has used up its budget. The first look starts the clock.
    pub fn exceeded(&self) -> bool {
        let Some(limit) = self.limit.get() else {
            return false;
        };

        if let Some(started) = self.started.get() {
            started.elapsed() > limit
        } else {
            self.started.set(Some(Instant::now()));
            false
        }
    }

    /// Lifts the limit: whatever runs next is not cut off by the hook that just ended.
    pub fn lift(&self) {
        self.limit.set(None);
        self.started.set(None);
    }
}

/// Runs `operation` under the execution budget of REQ-LUA-064: a script that never gives
/// control back is cut off as if it had raised an error, so the emulation resumes.
pub(crate) fn with_budget<R>(lua: &Lua, limit: Duration, operation: impl FnOnce() -> R) -> R {
    if let Some(budget) = lua.app_data_ref::<Budget>() {
        budget.arm(limit);
    }

    let result = operation();
    if let Some(budget) = lua.app_data_ref::<Budget>() {
        budget.lift();
    }

    result
}

/// The globals a hook call publishes before it runs.
#[derive(Clone, Copy)]
pub(crate) enum Published {
    Script,
    Shape,
    Shape2D,
    Pattern,
}

/// The globals table and the names of [`Published`], made once so that publishing a global costs
/// no string and no lookup of the table.
pub(crate) struct PublishedGlobals {
    table: Table,
    names: [mlua::String; 4],
}

impl PublishedGlobals {
    /// Installs the names in `lua`.
    pub fn install(lua: &Lua) -> mlua::Result<()> {
        let names = ["self", "shape", "shape2D", "pattern"].map(|name| lua.create_string(name));
        let [script, shape, shape_2d, pattern] = names;
        lua.set_app_data(Self {
            table: lua.globals(),
            names: [script?, shape?, shape_2d?, pattern?],
        });
        Ok(())
    }
}

/// Sets the global `which` for the hook about to run.
pub(crate) fn publish(lua: &Lua, which: Published, value: Value) -> mlua::Result<()> {
    let published = lua
        .app_data_ref::<PublishedGlobals>()
        .ok_or_else(|| mlua::Error::runtime("the hook globals are not installed"))?;
    published
        .table
        .raw_set(&published.names[which as usize], value)
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

/// What the call of a hook of a script needs, which stays the same from one call to the next.
struct HookCall {
    function: Function,
    limit: Duration,
    handle: Value,
}

/// The function of `hook` of the script at `index`, with the budget and the `self` of its calls,
/// or `None` when the script has no such hook (REQ-LUA-022).
fn prepare_hook(cx: &Cx, index: usize, hook: ScriptHook) -> mlua::Result<Option<HookCall>> {
    let (function, limit, handle) = cx.data(|data| {
        let script = &data.scripts[index];
        (
            script.hook(hook).cloned(),
            data.hook_budget,
            script.handle.clone(),
        )
    })?;
    let Some(function) = function else {
        return Ok(None);
    };

    let handle = if let Some(handle) = handle {
        handle
    } else {
        let handle = cx.object(Payload::Script(index))?;
        cx.data(|data| data.scripts[index].handle = Some(handle.clone()))?;
        handle
    };
    Ok(Some(HookCall {
        function,
        limit,
        handle,
    }))
}

/// Runs a prepared hook. Returns the text of the error it raised.
fn run_hook(
    cx: &Cx,
    index: usize,
    hook: ScriptHook,
    call: &HookCall,
) -> mlua::Result<Option<String>> {
    publish(cx.lua, Published::Script, call.handle.clone())?;
    match with_budget(cx.lua, call.limit, || {
        protected_hook_call(cx.lua, &call.function)
    })? {
        Ok(()) => Ok(None),
        Err(message) => {
            fail(cx, index, hook, &message)?;
            Ok(Some(message))
        }
    }
}

/// Calls `hook` of the script at `index`. An absent hook is simply not called (REQ-LUA-022).
/// Returns the text of the error the hook raised.
pub(crate) fn call_hook(cx: &Cx, index: usize, hook: ScriptHook) -> mlua::Result<Option<String>> {
    match prepare_hook(cx, index, hook)? {
        Some(call) => timed(cx, index, hook, || run_hook(cx, index, hook, &call)),
        None => Ok(None),
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
        let call = prepare_hook(cx, index, ScriptHook::UpdateS)?;
        for id in &pending {
            let Some(call) = &call else {
                break;
            };

            // Only an error puts a script in error, and it ends the calls of its list.
            let error = call_update_s(cx, index, call, key, *id)?;
            let failed = error.is_some();
            first = first.or(error);
            if failed {
                break;
            }
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
    call: &HookCall,
    key: crate::binding::payload::FrameKey,
    id: FrameInstanceId,
) -> mlua::Result<Option<String>> {
    timed(cx, index, ScriptHook::UpdateS, || {
        publish(
            cx.lua,
            Published::Shape,
            cx.object(Payload::Instance(InstanceTarget::Frame(key, id)))?,
        )?;
        publish(
            cx.lua,
            Published::Shape2D,
            cx.object(Payload::Shape(ShapeTarget { key, id: id.shape }))?,
        )?;
        run_hook(cx, index, ScriptHook::UpdateS, call)
    })
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

    let outcome = timed(cx, name, ScriptHook::InputProcess, || {
        with_budget(cx.lua, limit, || {
            protected_call(cx.lua, &function, f64::from(pad_index))
        })
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
