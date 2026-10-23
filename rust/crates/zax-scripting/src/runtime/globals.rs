//! The base environment of a profile's Lua state (SPEC-05 §3.7, §4.1): the sandbox, the
//! dialect of `print` and `tostring`, the persistent data and the host-object globals.

use mlua::{Function, IntoLuaMulti, Lua, MultiValue, Value};

use crate::api::{CONSTANT_TABLES, GLOBAL_OBJECTS};
use crate::binding::cx::{Call, Cx};
use crate::binding::payload::{HostType, Payload, Vec3Ref};
use crate::error::message_of;
use crate::persistent_data;
use crate::values::number_format;

/// The `pcall` the state was created with, kept apart so that a script cannot replace the one
/// the runtime relies on.
pub(crate) struct Pcall(pub Function);

/// The `tostring` the state was created with (REQ-LUA-052).
pub(crate) struct OriginalToString(pub Function);

/// Keeps the libraries REQ-LUA-050 allows and removes the rest: no `io`, no `debug`, no
/// `package`, no `require`, no file access; `os` keeps only `clock`, `time` and `date`. The
/// 5.1 globals REQ-LUA-101 point 4 asks for are added back.
const SANDBOX: &str = r#"
io = nil
debug = nil
package = nil
require = nil
dofile = nil
loadfile = nil

local clock, time, date = os.clock, os.time, os.date
os = { clock = clock, time = time, date = date }

-- 5.1 compatibility (REQ-LUA-101 point 4).
unpack = table.unpack
table.getn = function (t) return #t end
loadstring = function (s, name) return load(s, name) end
math.pow = function (x, y) return x ^ y end
math.ldexp = function (m, e) return m * 2.0 ^ e end
math.log10 = function (x) return math.log(x, 10) end
"#;

/// Calls `function` in protected mode. The outer result is a failure of the host itself; the
/// inner one is the script's own: its values, or the text of the error it raised.
pub(crate) fn protected_call(
    lua: &Lua,
    function: &Function,
    arguments: impl IntoLuaMulti,
) -> mlua::Result<Result<MultiValue, String>> {
    let pcall = lua
        .app_data_ref::<Pcall>()
        .ok_or_else(|| mlua::Error::runtime("the protected call is not installed"))?
        .0
        .clone();
    let mut call = arguments.into_lua_multi(lua)?;
    call.push_front(Value::Function(function.clone()));

    let mut results: MultiValue = pcall.call(call)?;
    match results.pop_front() {
        Some(Value::Boolean(true)) => Ok(Ok(results)),
        _ => Ok(Err(match results.pop_front() {
            Some(Value::String(text)) => text.to_string_lossy(),
            Some(Value::Error(error)) => message_of(&error),
            Some(other) => format!("{} error object", other.type_name()),
            None => "error".to_owned(),
        })),
    }
}

/// The text of one value: a number through the dialect's own format, anything else through the
/// `tostring` the state was created with (REQ-LUA-052).
#[allow(clippy::cast_precision_loss)] // lua_tonumber converts an integer to a double.
pub(crate) fn text_of(lua: &Lua, value: &Value) -> String {
    match value {
        Value::Number(number) => number_format::to_text(*number),
        Value::Integer(integer) => number_format::to_text(*integer as f64),
        other => {
            let original = lua
                .app_data_ref::<OriginalToString>()
                .map(|original| original.0.clone());
            original
                .and_then(|tostring| tostring.call::<mlua::String>(other.clone()).ok())
                .map_or_else(
                    || other.type_name().to_owned(),
                    |text| text.to_string_lossy(),
                )
        }
    }
}

/// Sandboxes the libraries and installs the functions and values of §4.1 that are not host
/// objects: `print`, `tostring`, `clear`, `GetPData`, `SetPData`, `RenewId`, `PData`,
/// `pdata_count`, `frame` and `time`.
pub(crate) fn install_base(lua: &Lua) -> mlua::Result<()> {
    lua.load(SANDBOX).set_name("=sandbox").exec()?;

    let globals = lua.globals();
    lua.set_app_data(Pcall(globals.get("pcall")?));
    lua.set_app_data(OriginalToString(globals.get("tostring")?));

    // print and tostring have to write an integral float without a decimal part, as the
    // single numeric type of the original dialect did (REQ-LUA-052, REQ-LUA-101 point 1).
    globals.set(
        "print",
        lua.create_function(|lua, values: MultiValue| {
            let line = values
                .iter()
                .map(|value| text_of(lua, value))
                .collect::<Vec<_>>()
                .join("\t");
            Cx::new(lua).data(|data| data.env.log.print(&line))
        })?,
    )?;
    globals.set(
        "tostring",
        lua.create_function(|lua, value: Option<Value>| {
            Ok(text_of(lua, &value.unwrap_or(Value::Nil)))
        })?,
    )?;

    // Emptied the scripting console of the original; nothing to do (REQ-LUA-053).
    globals.set("clear", lua.create_function(|_, _: MultiValue| Ok(()))?)?;
    globals.set(
        "GetPData",
        lua.create_function(|lua, values: MultiValue| {
            let cx = Cx::new(lua);
            let index = Call::new(&cx, &values).integer(1, "GetPData")?;
            match lua.globals().get::<Value>("PData")? {
                Value::Table(table) => table.raw_get::<Value>(persistent_data::wrap_index(index)),
                _ => Ok(Value::Nil),
            }
        })?,
    )?;
    globals.set(
        "SetPData",
        lua.create_function(|lua, values: MultiValue| {
            let cx = Cx::new(lua);
            let call = Call::new(&cx, &values);
            let index = call.integer(1, "SetPData")?;
            if let Value::Table(table) = lua.globals().get::<Value>("PData")? {
                table.raw_set(persistent_data::wrap_index(index), call.value(2).clone())?;
            }

            Ok(())
        })?,
    )?;
    globals.set(
        "RenewId",
        lua.create_function(|lua, _: MultiValue| {
            Cx::new(lua)
                .data(|data| data.env.profile.renew_id())?
                .map_err(mlua::Error::external)
        })?,
    )?;

    // PData[1..64] = 0 and pdata_count = 64 (REQ-LUA-070).
    let persistent = lua.create_table_with_capacity(persistent_data::ENTRY_COUNT, 0)?;
    for index in 1..=persistent_data::ENTRY_COUNT {
        persistent.raw_set(index, 0.0)?;
    }

    globals.set("PData", persistent)?;
    globals.set("pdata_count", persistent_data::ENTRY_COUNT_REAL)?;

    // frame and time are 0 before the first converted frame (REQ-LUA-023).
    globals.set("frame", 0.0)?;
    globals.set("time", 0.0)?;
    Ok(())
}

/// The payload of the host-object global `name`, or `None` for the globals a hook assigns.
fn global_payload(name: &str, kind: HostType) -> Option<Payload> {
    match kind {
        // shape, shape2D, pattern and self start out nil; the runtime assigns them
        // (REQ-LUA-025, REQ-LUA-026).
        HostType::Instance | HostType::Shape | HostType::ScriptObj => None,
        // Gamepad1 and Gamepad2 are the two player objects (§4.1).
        HostType::Player => Some(Payload::Player(usize::from(name == "Gamepad2"))),
        // ZapperOffset is a bound vector a script may change component by component.
        HostType::Vec3Ref => Some(Payload::Vec3Ref(Vec3Ref::ZapperOffset)),
        other => Some(Payload::Table(other)),
    }
}

/// Installs the globals of §4.1 that are host objects or constant tables (REQ-LUA-046,
/// REQ-LUA-080).
pub(crate) fn install_api_globals(lua: &Lua) -> mlua::Result<()> {
    let cx = Cx::new(lua);
    let globals = lua.globals();

    for (name, kind) in GLOBAL_OBJECTS {
        let value = match global_payload(name, kind) {
            Some(payload) => cx.object(payload)?,
            None => Value::Nil,
        };
        globals.set(name, value)?;
    }

    // The constant values are pushed as floats like every other number the host hands to Lua
    // (REQ-LUA-040, REQ-LUA-046).
    for (name, entries) in CONSTANT_TABLES {
        let table = lua.create_table_with_capacity(0, entries.len())?;
        for (key, value) in entries {
            table.raw_set(*key, f64::from(*value))?;
        }

        globals.set(name, table)?;
    }

    Ok(())
}
