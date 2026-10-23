//! The globals `Camera`, `Backdrop`, `Light`, `Video`, `Profile` and `NotiCenter` of SPEC-05
//! §4.2.

use mlua::Value;
use zax_engine3d::engine::FrameRole;
use zax_engine3d::vectors::Vec3;
use zax_settings::catalogue::SettingId;

use super::settings::with_settings;
use super::{assigned_bool, boolean, num, type_def};
use crate::binding::cx::{host_payload, try_color, Call, Cx};
use crate::binding::payload::{HostType, Payload, Vec3Ref};
use crate::binding::table::TypeDef;
use crate::environment::{ScriptContext, ScriptNotification, ScriptNotificationSeverity};
use crate::values::number_format;

pub(super) fn declare(types: &mut Vec<TypeDef>) {
    types.push(camera());
    types.push(backdrop());
    types.push(light());
    types.push(video());
    types.push(profile());
    types.push(notifications());
}

/// Camera's members are methods: calling one with `.` leaves the first argument in the
/// receiver's place and must raise (REQ-LUA-081).
fn require(call: &Call, kind: HostType, member: &str) -> mlua::Result<()> {
    if call.host(1) == Some(Payload::Table(kind)) {
        Ok(())
    } else {
        Err(call.invalid(member))
    }
}

/// Points the subjective camera at the followed shape, if any and if it is still part of the
/// frame being converted (`Camera.FpvShape`, REQ-LUA-088).
pub(crate) fn apply_camera_tracking(cx: &Cx) -> mlua::Result<()> {
    let Some(shape) = cx.data(|data| data.fpv_shape)? else {
        return Ok(());
    };

    let position = cx
        .with_shape(&shape, |data, _, state| {
            data.instances()
                .first()
                .map(|instance| instance.get_pos(state.settings()))
        })?
        .flatten();
    let Some(position) = position else {
        return Ok(());
    };

    // REQ-SCN-021: the tracker of SPEC-04 is what finds the shape again on the next frames.
    let role = if shape.key.pinned {
        FrameRole::Pinned
    } else {
        FrameRole::Converting
    };
    cx.state(|state| {
        state.start_tracker(role, shape.id);
    })?;
    cx.host_write(move |ctx| ctx.camera.set_look_at(position))?;
    Ok(())
}

fn camera() -> TypeDef {
    let mut definition = type_def(HostType::CameraApi, "CameraAPI");

    camera_properties(&mut definition);

    camera_methods(&mut definition);
    definition
}

/// The properties of the camera (REQ-LUA-088).
fn camera_properties(definition: &mut TypeDef) {
    // The orientation of the current mode, as a bound vector a script may keep (REQ-LUA-044,
    // "Antarctic Adventure").
    definition.read_only(
        "Rot",
        |cx, _| cx.value(Payload::Vec3Ref(Vec3Ref::CameraRotation)),
        Some(HostType::Vec3Ref),
    );
    definition.read_only(
        "Target",
        |cx, _| cx.value(Payload::Vec3Ref(Vec3Ref::CameraTarget)),
        Some(HostType::Vec3Ref),
    );
    definition.property(
        "Distance",
        |cx, _| Ok(num(cx.host(move |ctx| ctx.camera.distance())?)),
        |cx, _, value| {
            let distance = cx.real(value, "Distance")?;
            cx.host_write(move |ctx| ctx.camera.set_distance(distance))
        },
        None,
    );
    definition.property(
        "Mode",
        |cx, _| Ok(num(cx.host(move |ctx| ctx.camera.mode())?)),
        |cx, _, value| {
            let mode = cx.integer(value, "Mode")?.clamp(0, 1);
            cx.host_write(move |ctx| ctx.camera.set_mode(mode))
        },
        None,
    );
    definition.property(
        "LookAt",
        |cx, _| cx.vector3(cx.host(move |ctx| ctx.camera.look_at())?),
        |cx, _, value| {
            let target = cx.vec3(value, "LookAt")?;
            cx.host_write(move |ctx| ctx.camera.set_look_at(target))
        },
        Some(HostType::Vector3),
    );
    definition.property(
        "FpvShape",
        |cx, _| {
            let shape =
                cx.data(|data| data.fpv_shape.filter(|shape| shape.key.epoch == data.epoch))?;
            shape.map_or(Ok(Value::Nil), |shape| cx.object(Payload::Shape(shape)))
        },
        |cx, _, value| {
            if matches!(value, Value::Nil) {
                return cx.data(|data| data.fpv_shape = None);
            }

            if let Some(Payload::Shape(shape)) = host_payload(value) {
                cx.data(|data| data.fpv_shape = Some(shape))?;
                return apply_camera_tracking(cx);
            }

            Err(cx.invalid("FpvShape"))
        },
        Some(HostType::Shape),
    );
    definition.read_only(
        "IsFPV",
        |cx, _| Ok(boolean(cx.host(move |ctx| ctx.camera.mode())? == 1)),
        None,
    );
    definition.read_only(
        "IsTPV",
        |cx, _| Ok(boolean(cx.host(move |ctx| ctx.camera.mode())? == 0)),
        None,
    );
}

/// The methods of the camera (REQ-LUA-088).
fn camera_methods(definition: &mut TypeDef) {
    definition.function(
        "SetTarget",
        |call| {
            require(call, HostType::CameraApi, "SetTarget")?;
            let target = match call.cx.try_vec3(call.value(2))? {
                Some(target) => target,
                None => Vec3::new(
                    call.real(2, "SetTarget")?,
                    call.real(3, "SetTarget")?,
                    call.real(4, "SetTarget")?,
                ),
            };
            call.cx
                .host_write(move |ctx| ctx.camera.set_look_at(target))?;
            Ok(None)
        },
        None,
    );
    definition.function(
        "GetTarget",
        |call| {
            require(call, HostType::CameraApi, "GetTarget")?;
            let target = call.cx.host(move |ctx| ctx.camera.look_at())?;
            Ok(Some(call.cx.vector3(target)?))
        },
        Some(HostType::Vector3),
    );
    definition.function(
        "SetFPV",
        |call| {
            require(call, HostType::CameraApi, "SetFPV")?;
            call.cx.host_write(move |ctx| ctx.camera.set_mode(1))?;
            if call.is_none_or_nil(2) {
                return Ok(None);
            }

            if let Some(Payload::Shape(shape)) = call.host(2) {
                call.cx.data(|data| data.fpv_shape = Some(shape))?;
                apply_camera_tracking(call.cx)?;
                return Ok(None);
            }

            Err(call.invalid("SetFPV"))
        },
        None,
    );
    definition.function(
        "SetTPV",
        |call| {
            require(call, HostType::CameraApi, "SetTPV")?;
            call.cx.host_write(move |ctx| ctx.camera.set_mode(0))?;
            Ok(None)
        },
        None,
    );
    definition.function(
        "ToggleMode",
        |call| {
            require(call, HostType::CameraApi, "ToggleMode")?;
            call.cx.host(|ctx| {
                let mode = i32::from(ctx.camera.mode() == 0);
                ctx.camera.set_mode(mode);
            })?;
            Ok(None)
        },
        None,
    );
    definition.function(
        "ResetView",
        |call| {
            require(call, HostType::CameraApi, "ResetView")?;
            call.cx.host_write(move |ctx| ctx.camera.reset_view())?;
            Ok(None)
        },
        None,
    );
}

fn backdrop_axis(
    definition: &mut TypeDef,
    name: &'static str,
    axis: usize,
    read: fn(&mut ScriptContext<'_>) -> Vec3,
    write: fn(&mut ScriptContext<'_>, Vec3),
) {
    definition.property(
        name,
        move |cx, _| {
            let value = cx.host(read)?;
            Ok(num([value.x, value.y, value.z][axis]))
        },
        move |cx, _, value| {
            let component = cx.real(value, name)?;
            cx.host(move |ctx| {
                let mut current = read(ctx);
                match axis {
                    0 => current.x = component,
                    1 => current.y = component,
                    _ => current.z = component,
                }

                write(ctx, current);
            })
        },
        None,
    );
}

fn backdrop() -> TypeDef {
    // Backdrop (REQ-LUA-090).
    let mut definition = type_def(HostType::BackdropApi, "BackdropAPI");
    definition.property(
        "Enable",
        |cx, _| Ok(boolean(cx.host(move |ctx| ctx.backdrop.enabled())?)),
        |cx, _, value| {
            let enabled = assigned_bool(value);
            cx.host_write(move |ctx| ctx.backdrop.set_enabled(enabled))
        },
        None,
    );
    definition.property(
        "Color",
        |cx, _| cx.color(cx.host(move |ctx| ctx.backdrop.color())?),
        |cx, _, value| {
            let Some(color) = try_color(value) else {
                return Err(cx.invalid("Color"));
            };

            cx.host_write(move |ctx| ctx.backdrop.set_color(color))
        },
        Some(HostType::Color),
    );
    definition.property(
        "RecvShadow",
        |cx, _| Ok(boolean(cx.host(move |ctx| ctx.backdrop.receive_shadow())?)),
        |cx, _, value| {
            let receive = assigned_bool(value);
            cx.host_write(move |ctx| ctx.backdrop.set_receive_shadow(receive))
        },
        None,
    );
    for (name, axis) in [("SizeX", 0), ("SizeY", 1), ("SizeZ", 2)] {
        backdrop_axis(
            &mut definition,
            name,
            axis,
            |ctx| ctx.backdrop.size(),
            |ctx, value| ctx.backdrop.set_size(value),
        );
    }

    for (name, axis) in [("OfsX", 0), ("OfsY", 1), ("OfsZ", 2)] {
        backdrop_axis(
            &mut definition,
            name,
            axis,
            |ctx| ctx.backdrop.offset(),
            |ctx, value| ctx.backdrop.set_offset(value),
        );
    }

    definition.read_only(
        "Size",
        |cx, _| cx.value(Payload::Vec3Ref(Vec3Ref::BackdropSize)),
        Some(HostType::Vec3Ref),
    );
    definition.read_only(
        "Offset",
        |cx, _| cx.value(Payload::Vec3Ref(Vec3Ref::BackdropOffset)),
        Some(HostType::Vec3Ref),
    );
    definition
}

/// The light (type `Lumière`, SPEC-09 §4.1.3).
fn light() -> TypeDef {
    let mut definition = type_def(HostType::Light, "Lumiere");
    definition.property(
        "Direction",
        |cx, _| cx.vector3(cx.data(|data| data.env.settings.light_direction())?),
        |cx, _, value| {
            let direction = cx.vec3(value, "Direction")?;
            with_settings(cx, |settings| settings.set_light_direction(direction))
        },
        Some(HostType::Vector3),
    );
    definition.property(
        "Color",
        |cx, _| cx.color(cx.data(|data| data.env.settings.light_color())?),
        |cx, _, value| {
            let Some(color) = try_color(value) else {
                return Err(cx.invalid("Color"));
            };

            with_settings(cx, |settings| settings.set_light_color(color))
        },
        Some(HostType::Color),
    );
    definition.property(
        "Intensity",
        |cx, _| Ok(num(cx.data(|data| data.env.settings.light_intensity())?)),
        |cx, _, value| {
            let intensity = cx.real(value, "Intensity")?;
            with_settings(cx, |settings| settings.set_light_intensity(intensity))
        },
        None,
    );
    definition
}

/// Video (REQ-LUA-095): `SceneScale` reads 1 and a write triggers the view reset.
fn video() -> TypeDef {
    let mut definition = type_def(HostType::VideoApi, "VideoAPI");
    definition.property(
        "SceneScale",
        |cx, _| {
            Ok(num(cx.data(|data| {
                data.env.settings.real(SettingId::SceneSceneScale)
            })?))
        },
        |cx, _, value| {
            let scale = cx.real(value, "SceneScale")?;
            with_settings(cx, |settings| {
                settings.write_real(SettingId::SceneSceneScale, scale);
            })
        },
        None,
    );
    definition
}

/// Profile metadata (REQ-LUA-095): stored for the session, no effect on the library.
fn profile() -> TypeDef {
    fn text_member(
        definition: &mut TypeDef,
        name: &'static str,
        read: fn(&crate::environment::ScriptProfileInfo) -> &str,
        write: fn(&mut crate::environment::ScriptProfileInfo, String),
    ) {
        definition.property(
            name,
            move |cx, _| {
                let text = cx.data(|data| read(&data.env.profile).to_owned())?;
                cx.text(&text)
            },
            move |cx, _, value| {
                let text = cx.string(value, name)?;
                cx.data(|data| write(&mut data.env.profile, text))
            },
            None,
        );
    }

    let mut definition = type_def(HostType::ProfileApi, "ProfilAPI");
    text_member(&mut definition, "Title", |p| &p.title, |p, v| p.title = v);
    text_member(&mut definition, "Maker", |p| &p.maker, |p, v| p.maker = v);
    text_member(
        &mut definition,
        "GameDev",
        |p| &p.game_dev,
        |p, v| p.game_dev = v,
    );
    text_member(
        &mut definition,
        "Des",
        |p| &p.description,
        |p, v| p.description = v,
    );
    definition.read_only(
        "Id",
        |cx, _| {
            let id = cx.data(|data| data.env.profile.id.clone())?;
            cx.text(&id)
        },
        None,
    );
    definition.read_only(
        "Ver",
        |cx, _| {
            let version = cx.data(|data| data.env.profile.version.clone())?;
            cx.text(&version)
        },
        None,
    );
    definition.read_only(
        "IsV2Format",
        |cx, _| Ok(boolean(cx.data(|data| data.env.profile.is_v2_format())?)),
        None,
    );
    definition.function(
        "RenewId",
        |call| {
            require(call, HostType::ProfileApi, "RenewId")?;
            call.cx
                .data(|data| data.env.profile.renew_id())?
                .map_err(mlua::Error::external)?;
            Ok(None)
        },
        None,
    );
    definition
}

/// Raises a notification for the host (REQ-LUA-093).
fn notify(cx: &Cx, message: String, seconds: f32, severity: i32) -> mlua::Result<()> {
    cx.data(|data| {
        if let Some(callback) = data.env.notify.as_mut() {
            callback(ScriptNotification {
                message,
                seconds,
                severity: ScriptNotificationSeverity::from_raw(severity),
            });
        }
    })
}

/// Notifications (REQ-LUA-093): table functions, called with `.`.
fn notifications() -> TypeDef {
    let mut definition = type_def(HostType::NotiApi, "NotiAPI");
    definition.function(
        "Notify",
        |call| {
            let message = call.string(1, "Notify")?;
            #[allow(clippy::cast_possible_truncation)] // The C# casts the double to float.
            let seconds = call.optional_number(2, "Notify", 4.0)? as f32;
            let severity = number_format::to_i32(call.optional_number(3, "Notify", 0.0)?);
            notify(call.cx, message, seconds, severity)?;
            Ok(None)
        },
        None,
    );
    definition.function(
        "Show",
        |call| {
            let message = call.string(1, "Show")?;
            let severity = number_format::to_i32(call.optional_number(2, "Show", 0.0)?);
            notify(call.cx, message, 4.0, severity)?;
            Ok(None)
        },
        None,
    );
    definition
}
