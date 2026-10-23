//! The error of the Lua runtime.

/// Raised when a Lua chunk fails to load or fails during execution.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct LuaScriptError(pub String);

/// The text of an error mlua reports: what Lua itself said, without the wrapping mlua adds
/// around an error raised by a host function.
pub(crate) fn message_of(error: &mlua::Error) -> String {
    match error {
        mlua::Error::CallbackError { cause, .. } | mlua::Error::WithContext { cause, .. } => {
            message_of(cause)
        }
        mlua::Error::RuntimeError(message) => message.clone(),
        mlua::Error::SyntaxError { message, .. } => message.clone(),
        other => other.to_string(),
    }
}
