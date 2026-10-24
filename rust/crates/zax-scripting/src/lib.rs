#![deny(unsafe_code)]

//! zax-scripting: the Lua 5.4 runtime of the profile scripts (SPEC-05).
//!
//! One [`ProfileScriptRuntime`] per loaded profile owns a Lua state, the script model and the
//! hooks of §3. It is the [`ScriptHost`](zax_engine3d::script::ScriptHost) the conversion
//! pipeline calls. The API the scripts see is declared once, as data, in [`api`]: the runtime
//! installs it and [`ScriptApiSurface`] reads it back for the coverage checks of REQ-LUA-051 and
//! REQ-LUA-080.

mod api;
mod binding;
mod environment;
mod error;
mod lua_runtime;
mod persistent_data;
mod quaternion;
mod runtime;
mod script;
mod settings;
mod surface;
mod values;

pub use environment::{
    CollectingScriptLog, DetachedScriptBackdrop, DetachedScriptCamera, DetachedScriptInput,
    DetachedScriptMemory, NullScriptLog, ScriptBackdrop, ScriptCamera, ScriptContext,
    ScriptEnvironment, ScriptInput, ScriptLog, ScriptMachine, ScriptMemory, ScriptNotification,
    ScriptNotificationSeverity, ScriptProfileInfo,
};
pub use error::LuaScriptError;
pub use lua_runtime::LuaRuntime;
pub use runtime::{ProfileScriptRuntime, ScriptBridge};
pub use script::{LoadedScript, ScriptHook};
pub use settings::{ScriptLight, ScriptSettings};
pub use surface::{ScriptApiMember, ScriptApiSurface, ScriptApiType};
pub use values::{number_format, LuaColor, LuaIntVector3, LuaXrState};

/// The persistent-data block a savestate carries (SPEC-05 REQ-LUA-073, §4.6).
pub mod persistent {
    pub use crate::persistent_data::{read, wrap_index, write, BLOCK_SIZE, ENTRY_COUNT};
}

/// Stable identifier of this module, used by the headless harness and tests.
pub const MODULE_NAME: &str = "Zax.Scripting";

/// Short human-readable role of this module.
pub const MODULE_ROLE: &str = "Lua 5.4 runtime for profile scripts";
