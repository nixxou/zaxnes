//! The hand-off of a runtime between threads, and the lend of a host context to its hooks
//! (Rust-only performance work, owner decision: the pipelined frames of the application run a
//! frame's scripts on a thread of their own).
//!
//! `mlua` makes a Lua state `!Send` unless its `send` feature is on, which puts a lock on every
//! call into the state. The runtime does not need that: it is used by one thread at a time and
//! changes thread only when its owner is handed over whole. So [`ProfileScriptRuntime`] is
//! declared `Send` here instead, which rests on one rule, kept by this crate:
//!
//! **every handle on the runtime's Lua state is owned by the runtime**, so that moving the
//! runtime moves them all. The `Lua` itself, the functions, tables and values the runtime and
//! its application data keep: all of them live inside it. The only handle that ever leaves it is
//! the one [`lend_context`] takes, and that one lives only while the call holds its `owner` (and
//! with it the runtime) borrowed exclusively, on the calling thread, and is dropped before it
//! returns. No public function of the crate gives out a handle on the state.
//!
//! Everything else the runtime holds is `Send` on its own (the host bridges of
//! [`ScriptEnvironment`](crate::ScriptEnvironment) are `Send` traits).

#![allow(unsafe_code)]

use mlua::Lua;
use zax_engine3d::engine::ConversionEngine;

use super::{flush_host_writes, ProfileScriptRuntime};
use crate::binding::cx::HostLink;
use crate::environment::ScriptContext;

// SAFETY: see the module documentation: every handle on the Lua state is owned by the runtime
// (or, for the length of a `lend_context` call, by that call while it holds the runtime
// borrowed), so moving the runtime moves every handle, and no two threads ever reach the state.
// The rest of the runtime is `Send` by itself.
unsafe impl Send for ProfileScriptRuntime {}

/// Lends `context` to the hooks of the runtime the engine of `owner` holds (found by
/// `engine_of`, through [`ScriptHost::inner`](zax_engine3d::script::ScriptHost::inner)) for the
/// length of `operation`, then withdraws it. Without a runtime, `operation` simply runs.
///
/// The engine owns the runtime and calls its hooks, so the host cannot pass the context to the
/// runtime directly: the lend reaches the runtime's state through a second handle, taken here
/// and dropped before the call returns (see the module documentation).
pub fn lend_context<T, R>(
    owner: &mut T,
    engine_of: impl FnOnce(&T) -> &ConversionEngine,
    context: &mut ScriptContext<'_>,
    operation: impl FnOnce(&mut T) -> R,
) -> R {
    let lua = engine_of(owner)
        .script_host()
        .inner()
        .downcast_ref::<ProfileScriptRuntime>()
        .map(|runtime| runtime.lua.clone());
    match lua {
        Some(lua) => lend_on(&lua, context, || operation(owner)),
        None => operation(owner),
    }
}

/// Runs `operation` with `context` available to the script API of `lua`, then withdraws it. A
/// hook that runs outside any lend sees the headless bridges of the environment.
pub(super) fn lend_on<R>(
    lua: &Lua,
    context: &mut ScriptContext<'_>,
    operation: impl FnOnce() -> R,
) -> R {
    let lent = lua.scope(|scope| {
        // Should the function not be created, the hooks of the operation fall back on the
        // headless bridges.
        if let Ok(call) = scope.create_function_mut(|lua, ()| HostLink::serve(lua, context)) {
            lua.set_app_data(HostLink::new(call));
        }
        let result = operation();
        let flushed = flush_host_writes(lua);
        lua.remove_app_data::<HostLink>();
        debug_assert!(flushed.is_ok(), "the writes of a lend are made");
        Ok(result)
    });
    let Ok(result) = lent else {
        unreachable!("the closure of a lend never fails");
    };
    result
}
