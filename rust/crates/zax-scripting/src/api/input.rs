//! The globals `ScriptPad` and `Gamepad`, the player objects and the VR state of SPEC-05 §4.2
//! (REQ-LUA-091, REQ-LUA-092).

use std::sync::Arc;

use zax_engine3d::vectors::Vec3;

use super::{boolean, num, type_def};
use crate::binding::cx::Call;
use crate::binding::payload::{HostType, Payload};
use crate::binding::table::TypeDef;
use crate::values::LuaXrState;

pub(super) fn declare(types: &mut Vec<TypeDef>) {
    types.push(script_pad());
    types.extend(gamepad());
    types.push(xr_state());
}

fn require(call: &Call, kind: HostType, member: &str) -> mlua::Result<()> {
    if call.host(1) == Some(Payload::Table(kind)) {
        Ok(())
    } else {
        Err(call.invalid(member))
    }
}

fn script_pad() -> TypeDef {
    let mut definition = type_def(HostType::ScriptPadApi, "ScriptPadAPI");
    definition.function(
        "Enable",
        |call| {
            require(call, HostType::ScriptPadApi, "Enable")?;
            let enabled = call.boolean(2);
            call.cx.data(|data| data.script_pad_enabled = enabled)?;
            Ok(None)
        },
        None,
    );
    definition.property(
        "PadIndex",
        |cx, _| Ok(num(cx.data(|data| data.script_pad_index)?)),
        |cx, _, value| {
            let index = cx.integer(value, "PadIndex")?.clamp(0, 1);
            cx.data(|data| data.script_pad_index = index)?;
            Ok(())
        },
        None,
    );
    definition.read_only(
        "State",
        |cx, _| Ok(num(cx.data(|data| data.script_pad_state)?)),
        None,
    );
    definition.function(
        "Pressed",
        |call| {
            require(call, HostType::ScriptPadApi, "Pressed")?;
            let mask = call.integer(2, "Pressed")?;
            Ok(Some(boolean(
                call.cx.data(|data| data.script_pad_state)? & mask != 0,
            )))
        },
        None,
    );

    // No VR on desktop: the hand and head data are zero vectors (REQ-LUA-091).
    definition.function(
        "HandPos",
        |call| {
            require(call, HostType::ScriptPadApi, "HandPos")?;
            Ok(Some(call.cx.vector3(Vec3::ZERO)?))
        },
        Some(HostType::Vector3),
    );
    definition.function(
        "RotPos",
        |call| {
            require(call, HostType::ScriptPadApi, "RotPos")?;
            Ok(Some(call.cx.vector3(Vec3::ZERO)?))
        },
        Some(HostType::Vector3),
    );
    definition.read_only(
        "LeftHand",
        |cx, _| cx.value(Payload::XrState(LuaXrState::default())),
        Some(HostType::XrState),
    );
    definition.alias("LH", "LeftHand");
    definition.read_only(
        "RightHand",
        |cx, _| cx.value(Payload::XrState(LuaXrState::default())),
        Some(HostType::XrState),
    );
    definition.alias("RH", "RightHand");
    definition.read_only(
        "Head",
        |cx, _| cx.value(Payload::XrState(LuaXrState::default())),
        Some(HostType::XrState),
    );
    definition
}

fn gamepad() -> [TypeDef; 2] {
    // Gamepad (§4.2, type ManettesAPI).
    let mut pads = type_def(HostType::GamepadApi, "ManettesAPI");
    pads.read_only(
        "State",
        |cx, _| Ok(num(cx.data(|data| data.env.input.global_state())?)),
        None,
    );
    pads.function(
        "Pressed",
        |call| {
            require(call, HostType::GamepadApi, "Pressed")?;
            let mask = call.integer(2, "Pressed")?;
            Ok(Some(boolean(
                call.cx.data(|data| data.env.input.global_state())? & mask != 0,
            )))
        },
        None,
    );
    pads.function(
        "GetInput",
        |call| {
            require(call, HostType::GamepadApi, "GetInput")?;
            let port = call.integer(2, "GetInput")?;
            let port = call.cx.check_bounds(port, 2, "GetInput")?;
            Ok(Some(num(call
                .cx
                .data(|data| data.env.input.port_byte(port))?)))
        },
        None,
    );
    pads.element = Some(HostType::Player);
    pads.indexer = Some(Arc::new(|cx, _, key| {
        let index = cx.integer(key, "Gamepad")?;
        let index = cx.check_bounds(index, 2, "Gamepad")?;
        cx.value(Payload::Player(index))
    }));

    // One player (§4.2, type Joueur).
    let mut player = type_def(HostType::Player, "Joueur");
    player.read_only(
        "State",
        |cx, this| match this {
            Payload::Player(index) => Ok(num(cx.data(|data| data.env.input.player_state(*index))?)),
            _ => Ok(num(0)),
        },
        None,
    );
    player.function(
        "Pressed",
        |call| {
            let Some(Payload::Player(index)) = call.host(1) else {
                return Err(call.invalid("Pressed"));
            };

            let mask = call.integer(2, "Pressed")?;
            Ok(Some(boolean(
                call.cx.data(|data| data.env.input.player_state(index))? & mask != 0,
            )))
        },
        None,
    );
    [pads, player]
}

fn xr_vector(definition: &mut TypeDef, name: &'static str, read: fn(&LuaXrState) -> Vec3) {
    // XRState is a copy (§4.3): writing a field changes only that copy, and desktop has no VR
    // data to change anyway (REQ-LUA-091).
    definition.property(
        name,
        move |cx, this| match this {
            Payload::XrState(state) => cx.vector3(read(state)),
            _ => cx.vector3(Vec3::ZERO),
        },
        |_, _, _| Ok(()),
        Some(HostType::Vector3),
    );
}

/// `XRState` (§4.2): a value type whose four vectors are zero on desktop.
fn xr_state() -> TypeDef {
    let mut definition = type_def(HostType::XrState, "XRState");
    xr_vector(&mut definition, "pos", |state| state.position);
    xr_vector(&mut definition, "vel", |state| state.velocity);
    xr_vector(&mut definition, "acc", |state| state.acceleration);
    xr_vector(&mut definition, "rot", |state| state.rotation);
    definition
}
