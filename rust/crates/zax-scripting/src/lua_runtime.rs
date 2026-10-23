//! A bare Lua 5.4 state: no profile, no API. It proves that the interpreter loads and that the
//! language level the embedded profiles need (`goto` and labels, so Lua 5.2 or later) is
//! available.

use mlua::{Lua, Value};

use crate::error::{message_of, LuaScriptError};

/// Owns one Lua 5.4 state. One instance per emulation instance; no shared or static state.
pub struct LuaRuntime {
    lua: Lua,
}

impl Default for LuaRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl LuaRuntime {
    /// Creates a fresh Lua state with the standard libraries opened.
    #[must_use]
    pub fn new() -> Self {
        Self { lua: Lua::new() }
    }

    /// The `_VERSION` string reported by the embedded interpreter, for example "Lua 5.4".
    ///
    /// # Errors
    ///
    /// The interpreter failed.
    pub fn language_version(&self) -> Result<String, LuaScriptError> {
        self.evaluate_string("return _VERSION")
    }

    /// Runs a chunk and discards any result.
    ///
    /// # Errors
    ///
    /// The chunk failed to load or raised an error.
    pub fn execute(&self, chunk: &str) -> Result<(), LuaScriptError> {
        self.lua
            .load(chunk)
            .exec()
            .map_err(|error| LuaScriptError(message_of(&error)))
    }

    /// Runs a chunk and returns its first result converted to a 64-bit integer.
    ///
    /// # Errors
    ///
    /// The chunk failed, returned nothing, or returned a value that is not a number.
    pub fn evaluate_integer(&self, chunk: &str) -> Result<i64, LuaScriptError> {
        match self.run_for_result(chunk)? {
            Value::Integer(integer) => Ok(integer),
            // lua_tointeger only converts a float that holds an integer value.
            #[allow(clippy::cast_possible_truncation, clippy::float_cmp)]
            Value::Number(number) if number == number.trunc() => Ok(number as i64),
            Value::Number(_) => Ok(0),
            other => Err(LuaScriptError(format!(
                "Chunk returned '{}', expected a number.",
                other.type_name()
            ))),
        }
    }

    /// Runs a chunk and returns its first result converted to a string.
    ///
    /// # Errors
    ///
    /// The chunk failed or returned nothing.
    pub fn evaluate_string(&self, chunk: &str) -> Result<String, LuaScriptError> {
        let value = self.run_for_result(chunk)?;
        let text = self
            .lua
            .coerce_string(value)
            .map_err(|error| LuaScriptError(message_of(&error)))?;
        Ok(text.map_or_else(String::new, |text| text.to_string_lossy()))
    }

    fn run_for_result(&self, chunk: &str) -> Result<Value, LuaScriptError> {
        let results: mlua::MultiValue = self
            .lua
            .load(chunk)
            .eval()
            .map_err(|error| LuaScriptError(message_of(&error)))?;
        results
            .into_iter()
            .next()
            .ok_or_else(|| LuaScriptError("Chunk returned no value.".to_owned()))
    }
}
