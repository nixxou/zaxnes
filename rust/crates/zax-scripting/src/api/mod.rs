//! The complete script-visible API of SPEC-05 §4 declared against the binding layer: every
//! global of §4.1, every host type of §4.2, every bound vector, value type and list of §4.3 and
//! the constant tables of §4.4.
//!
//! The declarations are data: each member is a name, a getter and, when it is writable, a
//! setter. That is what REQ-LUA-051's static check verifies against, and what makes an unknown
//! member read as nil and an unknown assignment raise the exact message of REQ-LUA-042.

mod frame;
mod input;
mod instance;
mod scene;
mod settings;
mod shape;
mod values;

use mlua::Value;

use crate::binding::cx::truthy;
use crate::binding::payload::HostType;
use crate::binding::table::{Api, TypeDef};

/// The host-object globals of §4.1, in the order they are installed. The same table feeds the
/// API description, so the two can never drift apart (REQ-LUA-051).
pub(crate) const GLOBAL_OBJECTS: [(&str, HostType); 30] = [
    ("Frame", HostType::FrameApi),
    ("FrameManager", HostType::FrameManagerApi),
    ("Palette", HostType::PaletteApi),
    ("Pattern", HostType::PatternApi),
    ("Script", HostType::ScriptApi),
    ("NotiCenter", HostType::NotiApi),
    ("Video", HostType::VideoApi),
    ("Gamepad", HostType::GamepadApi),
    ("ScriptPad", HostType::ScriptPadApi),
    ("Layer", HostType::LayerConfig),
    ("Clipping", HostType::Clipping),
    ("Setting", HostType::SettingApi),
    ("Light", HostType::Light),
    ("Camera", HostType::CameraApi),
    ("Backdrop", HostType::BackdropApi),
    ("Profile", HostType::ProfileApi),
    ("Vector3", HostType::Vector3Class),
    ("Vector2", HostType::Vector2Class),
    ("Quaternion", HostType::QuaternionClass),
    ("Color", HostType::ColorClass),
    ("IntVector2", HostType::IntVector2Class),
    ("IntVector3", HostType::IntVector3Class),
    ("Rect", HostType::RectClass),
    ("Gamepad1", HostType::Player),
    ("Gamepad2", HostType::Player),
    ("ZapperOffset", HostType::Vec3Ref),
    // Reassigned by the runtime at every hook call (REQ-LUA-025, REQ-LUA-026).
    ("shape", HostType::Instance),
    ("shape2D", HostType::Shape),
    ("pattern", HostType::Instance),
    ("self", HostType::ScriptObj),
];

/// The globals of §4.1 that are plain functions rather than objects.
pub(crate) const FUNCTION_GLOBALS: [&str; 5] =
    ["print", "clear", "GetPData", "SetPData", "RenewId"];

/// The constant tables of §4.4 and their entries (REQ-LUA-046).
pub(crate) const CONSTANT_TABLES: [(&str, &[(&str, i32)]); 3] = [
    (
        "TLayer",
        &[
            ("UNIDENTIFIED", 0),
            ("UI", 1),
            ("L1", 2),
            ("L2", 3),
            ("L3", 4),
            ("L4", 5),
            ("COUNT", 6),
        ],
    ),
    (
        "Align",
        &[("FRONT", 0), ("CENTER", 1), ("BACK", 2), ("COUNT", 3)],
    ),
    ("CamMode", &[("TPV", 0), ("FPV", 1)]),
];

/// Declares every host type.
pub(crate) fn build() -> Api {
    let mut types = Vec::new();
    instance::declare(&mut types);
    shape::declare(&mut types);
    frame::declare(&mut types);
    scene::declare(&mut types);
    input::declare(&mut types);
    settings::declare(&mut types);
    values::declare(&mut types);
    Api::new(types)
}

/// A number, always as a Lua float (REQ-LUA-040).
pub(super) fn num(value: impl Into<f64>) -> Value {
    Value::Number(value.into())
}

/// A count or an index as a Lua float.
pub(super) fn count(value: usize) -> Value {
    Value::Number(f64::from(u32::try_from(value).unwrap_or(u32::MAX)))
}

/// A boolean.
pub(super) const fn boolean(value: bool) -> Value {
    Value::Boolean(value)
}

/// The truth value being assigned.
pub(super) const fn assigned_bool(value: &Value) -> bool {
    truthy(value)
}

/// Starts the declaration of a type.
pub(super) fn type_def(id: HostType, name: &'static str) -> TypeDef {
    TypeDef::new(id, name)
}
