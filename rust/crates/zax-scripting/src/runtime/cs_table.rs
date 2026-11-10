//! The global `CS` of 3dSen 2.0 (AMD-040 REQ-LUA-112, completed by AMD-059 REQ-LUA-117).
//!
//! In 3dSen 2.0 `CS` opens the whole C# code of the program to the scripts. Zaxnes publishes the
//! paths the amendments name and nothing else **(D)**: `CS.Nes3D.Info`, with the version and the
//! title a 2.0 script reads at the start of a game to pick its framing, and the three tables of
//! REQ-LUA-117.
//!
//! Every level is a **sealed** table: its fields live in a table only its metatable reaches, so
//! reading a field the amendments do not name gives nil without an error, a write anywhere in
//! the tree is dropped — it cannot reach the program, and it cannot disturb what the next script
//! reads either — and the metatable is hidden, so a script cannot take it off.
//!
//! These are plain Lua tables, not host values: nothing of `CS` reaches the binding, which is
//! why it does not appear in the API dump of the harness either.

use mlua::{Lua, Table, Value};

use zax_settings::styles;

use crate::binding::cx::Cx;
use crate::environment::MAX_CAMERA_DISTANCE;

/// What `CS.Nes3D.Info.Version` reads (REQ-LUA-112, raised to `"2.1"` by AMD-063 REQ-LUA-118):
/// a string whose leading decimal integer is at least 2, which is how a 2.0 script knows it may
/// read `Camera.ResetDistance` — the criterion of AMD-040 still holds.
pub(crate) const INFO_VERSION: &str = "2.1";

/// What `CS.Nes3D.Info.Title` reads (REQ-LUA-112): the product name of AMD-029, which a script
/// compares against `"3dSenMaker"` — the profile editor — to pick a field of view.
pub(crate) const INFO_TITLE: &str = "Zaxnes";

/// What `CS.Nes3D.SettingManager.DefaultMsaa` reads (REQ-LUA-117), the default of
/// `Setting.Msaa`.
pub(crate) const DEFAULT_MSAA: f64 = 2.0;

/// Installs the global `CS`.
///
/// # Errors
///
/// A Lua error while the tables are built.
pub(crate) fn install(lua: &Lua) -> mlua::Result<()> {
    let nes3d = lua.create_table_with_capacity(0, 4)?;
    nes3d.raw_set(
        "Info",
        sealed(lua, &[("Version", INFO_VERSION), ("Title", INFO_TITLE)])?,
    )?;
    nes3d.raw_set("CameraControl", camera_control(lua)?)?;
    nes3d.raw_set(
        "SettingManager",
        sealed(lua, &[("DefaultMsaa", DEFAULT_MSAA)])?,
    )?;
    nes3d.raw_set(
        "VisualStyle",
        // The two identifiers `Setting.VisualStyle` compares against (REQ-SCN-078); the name of
        // the Enhanced one is the original's.
        sealed(
            lua,
            &[
                ("Classic", f64::from(styles::CLASSIC_STYLE)),
                ("SoftPixelLight", f64::from(styles::ENHANCED_STYLE)),
            ],
        )?,
    )?;

    let cs = lua.create_table_with_capacity(0, 1)?;
    cs.raw_set("Nes3D", seal(lua, nes3d)?)?;
    lua.globals().set("CS", seal(lua, cs)?)?;
    Ok(())
}

/// The `CameraControl` table (REQ-LUA-117, amended by AMD-063 REQ-LUA-119): `DistanceMax` as
/// before, and `RotOverride`, the **one** field of `CS.Nes3D.CameraControl` whose write has an
/// effect.
///
/// Its leaf therefore carries a pair of real functions instead of a plain table, and the rest of
/// REQ-LUA-117 — every other field nil, every other write dropped — is held inside them.
fn camera_control(lua: &Lua) -> mlua::Result<Value> {
    const ROT_OVERRIDE: &str = "RotOverride";

    let shell = lua.create_table()?;
    let meta = lua.create_table_with_capacity(0, 3)?;
    meta.raw_set(
        "__index",
        lua.create_function(|lua, (_, key): (Value, Value)| {
            let name = key.as_string();
            let name = name.as_ref().and_then(|name| name.to_str().ok());
            let name = name.as_deref().unwrap_or_default();
            if name == "DistanceMax" {
                return Ok(Value::Number(f64::from(MAX_CAMERA_DISTANCE)));
            }
            if name != ROT_OVERRIDE {
                return Ok(Value::Nil);
            }

            // A copy, never a bound vector: writing through one would be a third way of steering
            // the camera, and REQ-LUA-119 asks for "nil or a Vector3 copy".
            let cx = Cx::new(lua);
            match cx.host(|ctx| ctx.camera.rot_override())? {
                Some(forced) => cx.vector3(forced),
                None => Ok(Value::Nil),
            }
        })?,
    )?;
    meta.raw_set(
        "__newindex",
        lua.create_function(|lua, (_, key, value): (Value, Value, Value)| {
            let name = key.as_string();
            let name = name.as_ref().and_then(|name| name.to_str().ok());
            if name.as_deref() != Some(ROT_OVERRIDE) {
                // Every other write is dropped, as REQ-LUA-117 asks.
                return Ok(());
            }

            let cx = Cx::new(lua);
            // Q-180: nil, or whatever any `Vector3` write of the API accepts (a bound vector, a
            // `Vector2` at z = 0); anything else is a Lua error.
            let forced = match value {
                Value::Nil => None,
                value => Some(cx.vec3(&value, ROT_OVERRIDE)?),
            };
            cx.host_write(move |ctx| ctx.camera.set_rot_override(forced))
        })?,
    )?;
    meta.raw_set("__metatable", "CS")?;
    shell.set_metatable(Some(meta))?;
    Ok(Value::Table(shell))
}

/// A sealed table whose fields are `fields`.
fn sealed<V: mlua::IntoLua + Clone>(lua: &Lua, fields: &[(&str, V)]) -> mlua::Result<Value> {
    let inner = lua.create_table_with_capacity(0, fields.len())?;
    for (name, value) in fields {
        inner.raw_set(*name, value.clone())?;
    }
    seal(lua, inner)
}

/// A shell whose `__index` is `fields`: an unnamed field reads nil, a write is dropped and the
/// metatable is hidden (REQ-LUA-112).
fn seal(lua: &Lua, fields: Table) -> mlua::Result<Value> {
    let shell = lua.create_table()?;
    let meta = lua.create_table_with_capacity(0, 3)?;
    meta.raw_set("__index", fields)?;
    meta.raw_set(
        "__newindex",
        lua.create_function(|_, _: mlua::MultiValue| Ok(()))?,
    )?;
    meta.raw_set("__metatable", "CS")?;
    shell.set_metatable(Some(meta))?;
    Ok(Value::Table(shell))
}
