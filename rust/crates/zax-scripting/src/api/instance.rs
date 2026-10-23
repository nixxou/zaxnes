//! The type `Instance` of SPEC-05 §4.2 — a pattern instance of the profile or a frame
//! instance — together with its custom data, its texture slot and the bound vectors and
//! matrices of §4.3.

use std::sync::Arc;

use mlua::Value;
use zax_engine3d::engine::ConversionState;
use zax_engine3d::frame::{FrameInstance, FrameShape};
use zax_engine3d::instance::{PatternInstance, RenderStyle};
use zax_engine3d::pattern::EnginePattern;
use zax_engine3d::registry::PatternId;
use zax_engine3d::settings::{Geo3DType, ZLayer};
use zax_engine3d::vectors::{Cell, Vec2, Vec3};
use zax_profiles::tags;

use super::values::text_of;
use super::{assigned_bool, boolean, count, num, type_def};
use crate::binding::access::{
    read_matrix, read_vec2_ref, read_vec3_ref, write_matrix, write_vec2_ref, write_vec3_ref,
};
use crate::binding::cx::{host_payload, try_rect, Call, Cx};
use crate::binding::payload::{
    HostType, InstanceTarget, InstanceVec, Mat4Kind, PatternTarget, Payload, ShapeTarget, Vec2Kind,
    Vec3Ref,
};
use crate::binding::table::TypeDef;

pub(super) fn declare(types: &mut Vec<TypeDef>) {
    types.push(instance());
    types.extend(references());
    types.push(custom_data());
    types.push(slot());
}

/// The instance a member was called on.
const fn target_of(payload: &Payload) -> Option<InstanceTarget> {
    match payload {
        Payload::Instance(target) => Some(*target),
        _ => None,
    }
}

/// Reads from the instance, `fallback` when it is gone.
fn read<R>(
    cx: &Cx,
    this: &Payload,
    fallback: R,
    operation: impl FnOnce(&PatternInstance) -> R,
) -> mlua::Result<R> {
    let Some(target) = target_of(this) else {
        return Ok(fallback);
    };

    Ok(cx
        .with_instance(&target, |view| operation(&view))?
        .unwrap_or(fallback))
}

/// Reads from the frame instance, `fallback` when it is gone or is not one of a frame.
fn read_frame<R>(
    cx: &Cx,
    this: &Payload,
    fallback: R,
    operation: impl FnOnce(&FrameInstance, &ConversionState) -> R,
) -> mlua::Result<R> {
    let Some(target) = target_of(this) else {
        return Ok(fallback);
    };

    Ok(cx
        .with_instance(&target, |view| {
            view.frame()
                .map(|(instance, state)| operation(instance, state))
        })?
        .flatten()
        .unwrap_or(fallback))
}

/// Writes to the instance; a write to a gone instance does nothing.
fn write(
    cx: &Cx,
    this: &Payload,
    operation: impl FnOnce(&mut PatternInstance),
) -> mlua::Result<()> {
    if let Some(target) = target_of(this) {
        cx.with_instance_mut(&target, operation)?;
    }

    Ok(())
}

fn real_member(
    definition: &mut TypeDef,
    name: &'static str,
    get: fn(&PatternInstance) -> f32,
    set: fn(&mut PatternInstance, f32),
) {
    definition.property(
        name,
        move |cx, this| Ok(num(read(cx, this, 0.0, get)?)),
        move |cx, this, value| {
            let wanted = cx.real(value, name)?;
            write(cx, this, |instance| set(instance, wanted))
        },
        None,
    );
}

fn int_member(
    definition: &mut TypeDef,
    name: &'static str,
    fallback: i32,
    get: fn(&PatternInstance) -> i32,
    set: fn(&mut PatternInstance, i32),
) {
    definition.property(
        name,
        move |cx, this| Ok(num(read(cx, this, fallback, get)?)),
        move |cx, this, value| {
            let wanted = cx.integer(value, name)?;
            write(cx, this, |instance| set(instance, wanted))
        },
        None,
    );
}

fn flag(
    definition: &mut TypeDef,
    name: &'static str,
    get: fn(&PatternInstance) -> bool,
    set: fn(&mut PatternInstance, bool),
) {
    definition.property(
        name,
        move |cx, this| Ok(boolean(read(cx, this, false, get)?)),
        move |cx, this, value| {
            let wanted = assigned_bool(value);
            write(cx, this, |instance| set(instance, wanted))
        },
        None,
    );
}

fn geometry(definition: &mut TypeDef, name: &'static str, get: fn(&FrameInstance) -> Vec3) {
    definition.read_only(
        name,
        move |cx, this| {
            let value = read_frame(cx, this, Vec3::ZERO, |instance, _| get(instance))?;
            cx.vector3(value)
        },
        Some(HostType::Vector3),
    );
}

/// A bound vector, matrix or list of the instance: the descriptor of that member.
fn descriptor(
    definition: &mut TypeDef,
    name: &'static str,
    make: impl Fn(InstanceTarget) -> Payload + Send + Sync + 'static,
    result: HostType,
) {
    definition.read_only(
        name,
        move |cx, this| match target_of(this) {
            Some(target) => cx.value(make(target)),
            None => Ok(Value::Nil),
        },
        Some(result),
    );
}

fn bound_vector(definition: &mut TypeDef, name: &'static str, field: InstanceVec) {
    descriptor(
        definition,
        name,
        move |target| Payload::Vec3Ref(Vec3Ref::Instance(target, field)),
        HostType::Vec3Ref,
    );
}

fn shape_of(target: &InstanceTarget) -> Option<ShapeTarget> {
    match target {
        InstanceTarget::Frame(key, id) => Some(ShapeTarget {
            key: *key,
            id: id.shape,
        }),
        InstanceTarget::Pattern(_) => None,
    }
}

/// The registry pattern the instance belongs to: the reference pattern of the shape of a frame
/// instance, the pattern holding a pattern instance.
fn reference_pattern(cx: &Cx, target: &InstanceTarget) -> mlua::Result<Option<PatternId>> {
    match target {
        InstanceTarget::Frame(..) => Ok(shape_of(target)
            .map(|shape| cx.with_shape(&shape, |data, _, _| data.reference_pattern()))
            .transpose()?
            .flatten()
            .flatten()),
        InstanceTarget::Pattern(reference) => cx.state(|state| state.find_pattern(*reference)),
    }
}

/// What carries the pattern-level members of an instance: the shape of a frame instance, the
/// registry pattern of a pattern instance.
enum Carrier<'a> {
    Shape(&'a FrameShape),
    Pattern(&'a EnginePattern),
}

/// What carries them, for writing.
enum CarrierMut<'a> {
    Shape(&'a mut FrameShape),
    Pattern(&'a mut EnginePattern),
}

fn with_carrier<R>(
    cx: &Cx,
    this: &Payload,
    fallback: R,
    operation: impl FnOnce(Carrier<'_>) -> R,
) -> mlua::Result<R> {
    let Some(target) = target_of(this) else {
        return Ok(fallback);
    };

    let found = match target {
        InstanceTarget::Frame(..) => match shape_of(&target) {
            Some(shape) => cx.with_shape(&shape, |data, _, _| operation(Carrier::Shape(data)))?,
            None => None,
        },
        InstanceTarget::Pattern(reference) => {
            let id = cx.state(|state| state.find_pattern(reference))?;
            match id {
                Some(id) => {
                    cx.with_pattern(id, |pattern, _| operation(Carrier::Pattern(pattern)))?
                }
                None => None,
            }
        }
    };
    Ok(found.unwrap_or(fallback))
}

fn with_carrier_mut(
    cx: &Cx,
    this: &Payload,
    operation: impl FnOnce(CarrierMut<'_>),
) -> mlua::Result<()> {
    let Some(target) = target_of(this) else {
        return Ok(());
    };

    match target {
        InstanceTarget::Frame(..) => {
            if let Some(shape) = shape_of(&target) {
                cx.with_shape_mut(&shape, |data| operation(CarrierMut::Shape(data)))?;
            }
        }
        InstanceTarget::Pattern(reference) => {
            let id = cx.state(|state| state.find_pattern(reference))?;
            if let Some(id) = id {
                cx.with_pattern_mut(id, |pattern| operation(CarrierMut::Pattern(pattern)))?;
            }
        }
    }

    Ok(())
}

fn carrier_constraint(carrier: &Carrier<'_>) -> Cell {
    match carrier {
        Carrier::Shape(shape) => shape.constraint_size,
        Carrier::Pattern(pattern) => pattern.constraint_size,
    }
}

/// Whether the instance counts as casting or receiving shadows under the settings
/// (`CastingShadow`, `ReceivingShadow`).
fn effective_shadow(cx: &Cx, this: &Payload, cast: bool) -> mlua::Result<bool> {
    read_frame(cx, this, false, |instance, state| {
        let settings = state.settings();
        if !settings.shadow || instance.layer == ZLayer::Ui {
            return false;
        }

        let layer = settings.layer(instance.effective_layer(settings));
        if cast {
            instance.cast_shadow && layer.cast_shadow
        } else {
            instance.receive_shadow && layer.receive_shadow
        }
    })
}

/// The `t` of the deformation formula: the global `time` as the scripts see it
/// (REQ-LUA-083 point 3), which a script may have reassigned (REQ-LUA-026).
#[allow(clippy::cast_possible_truncation)] // The C# casts the double to float.
fn frame_time(cx: &Cx) -> mlua::Result<f32> {
    Ok(match cx.lua.globals().get::<Value>("time")? {
        Value::Number(time) => time as f32,
        #[allow(clippy::cast_precision_loss)] // lua_tonumber converts an integer to a double.
        Value::Integer(time) => time as f32,
        _ => 0.0,
    })
}

/// The deformation a script predicts through `GetDeform` and `GetDeformation` (SPEC-05
/// REQ-LUA-083 point 3): `d_a = sin(r.x·V[k,0] + W[k,0]) · sin(r.y·V[k,1] + W[k,1]) ·
/// sin(r.z·V[k,2] + W[k,2]) · sin(t·V[k,3] + W[k,3])`, the result scaled by `DeformAmp ⊙ size`.
///
/// The anchor the formula uses is deliberately not the pivot the rendered deformation applies
/// (SPEC-04B REQ-E3D-568): the difference, at most one pixel, is the original's and is kept.
fn deformation(instance: &FrameInstance, point: Vec3, anchor: Vec3, time: f32) -> Vec3 {
    let size = instance.size();
    let ratio = Vec3::new(
        if size.x == 0.0 {
            0.0
        } else {
            (point.x - anchor.x) / size.x
        },
        if size.y == 0.0 {
            0.0
        } else {
            (point.y - anchor.y) / size.y
        },
        if size.z == 0.0 {
            0.0
        } else {
            (point.z - anchor.z) / size.z
        },
    );

    let speed = instance.deform_speed;
    let phase = instance.deform_offset;
    let amplitude = instance.deform_amplitude;
    let mut axes = [0.0_f32; 3];
    for (k, axis) in axes.iter_mut().enumerate() {
        *axis = ((ratio.x * speed[(k, 0)]) + phase[(k, 0)]).sin()
            * ((ratio.y * speed[(k, 1)]) + phase[(k, 1)]).sin()
            * ((ratio.z * speed[(k, 2)]) + phase[(k, 2)]).sin()
            * ((time * speed[(k, 3)]) + phase[(k, 3)]).sin();
    }

    Vec3::new(
        axes[0] * amplitude.x * size.x,
        axes[1] * amplitude.y * size.y,
        axes[2] * amplitude.z * size.z,
    )
}

fn instance() -> TypeDef {
    let mut definition = type_def(HostType::Instance, "Instance");

    instance_transforms(&mut definition);

    instance_render_members(&mut definition);

    instance_flags_and_depth(&mut definition);

    instance_slot_and_links(&mut definition);

    instance_pattern_and_index(&mut definition);

    instance_geometry(&mut definition);

    instance_carrier_size(&mut definition);
    instance_carrier_flags(&mut definition);

    instance_tag_flags(&mut definition);

    methods(&mut definition);
    definition
}

/// The transforms: bound vectors, matrices and the custom data (REQ-LUA-044).
fn instance_transforms(definition: &mut TypeDef) {
    // Transforms: bound vectors, modifiable in place (REQ-LUA-044).
    bound_vector(definition, "Offset", InstanceVec::Offset);
    definition.alias("offset", "Offset");
    bound_vector(definition, "Scale", InstanceVec::Scale);
    definition.alias("scale", "Scale");
    bound_vector(definition, "Rot", InstanceVec::Rotation);
    definition.alias("rot", "Rot");
    bound_vector(definition, "Pivot", InstanceVec::Pivot);
    definition.alias("pivot", "Pivot");
    bound_vector(definition, "DeformAmp", InstanceVec::DeformAmplitude);
    bound_vector(definition, "DeformPivot", InstanceVec::DeformPivot);

    descriptor(
        definition,
        "DeformSpeed",
        |target| Payload::Mat4Ref(target, Mat4Kind::DeformSpeed),
        HostType::Mat4Ref,
    );
    descriptor(
        definition,
        "DeformOffset",
        |target| Payload::Mat4Ref(target, Mat4Kind::DeformOffset),
        HostType::Mat4Ref,
    );
    descriptor(
        definition,
        "texOffset",
        |target| Payload::Vec2Ref(target, Vec2Kind::TexOffset),
        HostType::Vec2Ref,
    );
    descriptor(
        definition,
        "texTilting",
        |target| Payload::Vec2Ref(target, Vec2Kind::TexTilting),
        HostType::Vec2Ref,
    );
    descriptor(
        definition,
        "CData",
        Payload::CustomData,
        HostType::CustomData,
    );
}

/// Opacity, visibility, palette and layer.
fn instance_render_members(definition: &mut TypeDef) {
    real_member(definition, "Alpha", |i| i.alpha, |i, v| i.alpha = v);
    definition.alias("alpha", "Alpha");
    definition.property(
        "Enable",
        |cx, this| Ok(boolean(read(cx, this, false, PatternInstance::enable)?)),
        |cx, this, value| {
            let wanted = assigned_bool(value);
            write(cx, this, |instance| instance.set_enable(wanted))
        },
        None,
    );
    int_member(
        definition,
        "RenderStyle",
        0,
        |i| i.style as i32,
        |i, v| i.style = RenderStyle::from_raw(v),
    );

    // Reading gives the custom palette when it is >= 0 and the shape's otherwise; writing sets
    // the custom palette (§4.2, REQ-E3D-192).
    definition.property(
        "Palette",
        |cx, this| {
            let Some(target) = target_of(this) else {
                return Ok(num(0));
            };

            let palette = cx
                .with_instance(&target, |view| match view.frame() {
                    Some((instance, _)) => instance.palette_table(),
                    None => view.custom_palette.max(0),
                })?
                .unwrap_or(0);
            Ok(num(palette))
        },
        |cx, this, value| {
            let wanted = cx.integer(value, "Palette")?;
            write(cx, this, |instance| instance.custom_palette = wanted)
        },
        None,
    );
    int_member(
        definition,
        "customPalette",
        -1,
        |i| i.custom_palette,
        |i, v| i.custom_palette = v,
    );
    int_member(
        definition,
        "Layer",
        0,
        |i| i.layer as i32,
        |i, v| i.layer = ZLayer::from_raw(v),
    );
    definition.alias("layer", "Layer");

    // Writing true moves the instance to the UI layer; writing false has no effect (§4.2).
    definition.property(
        "UI",
        |cx, this| Ok(boolean(read(cx, this, false, |i| i.layer == ZLayer::Ui)?)),
        |cx, this, value| {
            if assigned_bool(value) {
                write(cx, this, |instance| instance.layer = ZLayer::Ui)?;
            }

            Ok(())
        },
        None,
    );
    definition.alias("ui", "UI");
}

/// The shadow flags, the depth, the geometry and the fill.
fn instance_flags_and_depth(definition: &mut TypeDef) {
    flag(
        definition,
        "CastShadow",
        |i| i.cast_shadow,
        |i, v| i.cast_shadow = v,
    );
    flag(
        definition,
        "RecvShadow",
        |i| i.receive_shadow,
        |i, v| i.receive_shadow = v,
    );
    flag(
        definition,
        "antiShadow",
        |i| i.anti_shadow,
        |i, v| i.anti_shadow = v,
    );
    flag(definition, "flip", |i| i.flip, |i, v| i.flip = v);
    flag(definition, "solid", |i| i.solid, |i, v| i.solid = v);
    flag(
        definition,
        "fullWarp",
        |i| i.full_warp,
        |i, v| i.full_warp = v,
    );
    definition.read_only(
        "CastingShadow",
        |cx, this| Ok(boolean(effective_shadow(cx, this, true)?)),
        None,
    );
    definition.read_only(
        "ReceivingShadow",
        |cx, this| Ok(boolean(effective_shadow(cx, this, false)?)),
        None,
    );

    int_member(definition, "sizeZ", 0, |i| i.depth, |i, v| i.depth = v);
    int_member(
        definition,
        "geo",
        0,
        |i| i.geometry as i32,
        |i, v| i.geometry = Geo3DType::from_raw(v),
    );
    real_member(
        definition,
        "fillrate",
        |i| i.fill_rate,
        |i, v| i.fill_rate = v,
    );
    int_member(
        definition,
        "emptyColor",
        0,
        |i| i.empty_color,
        |i, v| i.empty_color = v,
    );
}

/// The texture slot and the links to the shape and the patterns.
fn instance_slot_and_links(definition: &mut TypeDef) {
    // The texture slot: shared between a pattern instance and its frame instances, and
    // assignable so that an instance borrows another's texture (REQ-LUA-097).
    definition.property(
        "slot",
        |cx, this| {
            let Some(target) = target_of(this) else {
                return Ok(Value::Nil);
            };

            let owner = match target {
                InstanceTarget::Pattern(reference) => {
                    cx.with_instance(&target, |view| view.texture_source.unwrap_or(reference))?
                }
                InstanceTarget::Frame(..) => cx
                    .with_instance(&target, |view| {
                        view.frame().map(|(instance, _)| instance.texture_owner())
                    })?
                    .flatten(),
            };
            owner.map_or(Ok(Value::Nil), |owner| cx.object(Payload::Slot(owner)))
        },
        |cx, this, value| {
            let wanted = match host_payload(value) {
                Some(Payload::Slot(source)) => Some(source),
                None if matches!(value, Value::Nil) => None,
                _ => return Err(cx.invalid("slot")),
            };
            write(cx, this, |instance| instance.texture_source = wanted)
        },
        Some(HostType::Slot),
    );

    definition.read_only(
        "Shape2D",
        |cx, this| match target_of(this).as_ref().and_then(shape_of) {
            Some(shape) => cx.object(Payload::Shape(shape)),
            None => Ok(Value::Nil),
        },
        Some(HostType::Shape),
    );
    definition.alias("shape", "Shape2D");

    // Listed as writable in §4.2, but the link from a frame instance to the pattern instance it
    // copies is the engine's (REQ-E3D-190) and no profile writes it; the write is accepted and
    // ignored rather than raising.
    definition.property(
        "refPattern3D",
        |cx, this| {
            let Some(target) = target_of(this) else {
                return Ok(Value::Nil);
            };

            let reference = cx.with_instance(&target, |view| match view.frame() {
                Some((instance, _)) => InstanceTarget::Pattern(instance.reference()),
                None => target,
            })?;
            reference.map_or(Ok(Value::Nil), |reference| {
                cx.object(Payload::Instance(reference))
            })
        },
        |_, _, _| Ok(()),
        Some(HostType::Instance),
    );

    definition.read_only(
        "refPattern",
        |cx, this| match target_of(this) {
            Some(target) => reference_pattern(cx, &target)?.map_or(Ok(Value::Nil), |id| {
                cx.object(Payload::Pattern(PatternTarget::Registry(id)))
            }),
            None => Ok(Value::Nil),
        },
        Some(HostType::Pattern),
    );
}

/// The carrying pattern and the position in it.
fn instance_pattern_and_index(definition: &mut TypeDef) {
    // The carrying pattern: the registry pattern of a pattern instance, the frame pattern of a
    // frame instance (§4.2, type Pattern). Listed as writable in §4.2, but rebinding an
    // instance to another pattern has no meaning in this model and no profile does it; the
    // write is accepted and ignored rather than raising (REQ-LUA-042 would make it an error
    // otherwise).
    definition.property(
        "pattern",
        |cx, this| {
            let Some(target) = target_of(this) else {
                return Ok(Value::Nil);
            };

            if let Some(shape) = shape_of(&target) {
                return cx.object(Payload::Pattern(PatternTarget::Frame(shape)));
            }

            reference_pattern(cx, &target)?.map_or(Ok(Value::Nil), |id| {
                cx.object(Payload::Pattern(PatternTarget::Registry(id)))
            })
        },
        |_, _, _| Ok(()),
        Some(HostType::Pattern),
    );

    definition.read_only(
        "Index",
        |cx, this| Ok(count(read(cx, this, 0, PatternInstance::index)?)),
        None,
    );
    definition.alias("index", "Index");
    definition.read_only(
        "Sp",
        |cx, this| Ok(boolean(read_frame(cx, this, false, |i, _| i.sprite())?)),
        None,
    );
    definition.read_only(
        "Bg",
        |cx, this| Ok(boolean(read_frame(cx, this, false, |i, _| i.background())?)),
        None,
    );
}

/// The box of the frame instance and its anchor (REQ-LUA-083).
fn instance_geometry(definition: &mut TypeDef) {
    geometry(definition, "BottomLeft", FrameInstance::bottom_left);
    geometry(definition, "TopRight", FrameInstance::top_right);
    geometry(definition, "BL", FrameInstance::bl);
    geometry(definition, "TR", FrameInstance::tr);
    geometry(definition, "size", FrameInstance::size);
    definition.alias("Size", "size");
    geometry(definition, "OriPos", FrameInstance::ori_pos);
    geometry(definition, "OriPosWoLayer", |i| {
        i.get_pos_without_layer(i.deform_pivot)
    });
    geometry(definition, "Pos", |i| i.pos);

    definition.property(
        "Center",
        |cx, this| {
            let value = read_frame(cx, this, Vec3::ZERO, |instance, _| instance.center)?;
            cx.vector3(value)
        },
        |cx, this, value| {
            let Some(target) = target_of(this) else {
                return Ok(());
            };

            let center = cx.vec3(value, "Center")?;
            cx.with_frame_instance_mut(&target, |instance, _| instance.center = center)?;
            Ok(())
        },
        Some(HostType::Vector3),
    );
}

/// The size and the constraint of the carrying pattern.
fn instance_carrier_size(definition: &mut TypeDef) {
    definition.read_only(
        "tSize",
        |cx, this| {
            let size = with_carrier(cx, this, Cell::default(), |carrier| match carrier {
                Carrier::Shape(shape) => shape.tile_size(),
                Carrier::Pattern(pattern) => Cell::new(pattern.width(), pattern.height()),
            })?;
            cx.int_vector2(size)
        },
        Some(HostType::IntVector2),
    );

    definition.property(
        "constraintSize",
        |cx, this| {
            let size = with_carrier(cx, this, Cell::new(99, 99), |carrier| {
                carrier_constraint(&carrier)
            })?;
            cx.int_vector2(size)
        },
        |cx, this, value| {
            let Some(size) = cx.try_int_vector2(value)? else {
                return Err(cx.invalid("constraintSize"));
            };

            set_constraint(cx, this, |_| size)
        },
        Some(HostType::IntVector2),
    );
    definition.property(
        "constraintWidth",
        |cx, this| {
            let size = with_carrier(cx, this, Cell::new(99, 99), |carrier| {
                carrier_constraint(&carrier)
            })?;
            Ok(num(size.x))
        },
        |cx, this, value| {
            let width = cx.integer(value, "constraintWidth")?;
            set_constraint(cx, this, |current| Cell::new(width, current.y))
        },
        None,
    );
    definition.property(
        "constraintHeight",
        |cx, this| {
            let size = with_carrier(cx, this, Cell::new(99, 99), |carrier| {
                carrier_constraint(&carrier)
            })?;
            Ok(num(size.y))
        },
        |cx, this, value| {
            let height = cx.integer(value, "constraintHeight")?;
            set_constraint(cx, this, |current| Cell::new(current.x, height))
        },
        None,
    );

    definition.property(
        "selfConnected",
        |cx, this| {
            let value = with_carrier(cx, this, true, |carrier| match carrier {
                Carrier::Shape(shape) => shape.self_connected,
                Carrier::Pattern(pattern) => pattern.self_connected(),
            })?;
            Ok(boolean(value))
        },
        |cx, this, value| {
            let wanted = assigned_bool(value);
            with_carrier_mut(cx, this, |carrier| match carrier {
                CarrierMut::Shape(shape) => shape.self_connected = wanted,
                CarrierMut::Pattern(pattern) => pattern.set_self_connected(wanted),
            })
        },
        None,
    );
}

/// The flags of the carrying pattern and the tag string.
fn instance_carrier_flags(definition: &mut TypeDef) {
    definition.property(
        "skipGeo",
        |cx, this| {
            let value = with_carrier(cx, this, false, |carrier| match carrier {
                Carrier::Shape(shape) => shape.skip_geometry,
                Carrier::Pattern(pattern) => pattern.skip_geometry,
            })?;
            Ok(boolean(value))
        },
        |cx, this, value| {
            let wanted = assigned_bool(value);
            with_carrier_mut(cx, this, |carrier| match carrier {
                CarrierMut::Shape(shape) => shape.skip_geometry = wanted,
                CarrierMut::Pattern(pattern) => pattern.skip_geometry = wanted,
            })
        },
        None,
    );

    definition.read_only(
        "TagStr",
        |cx, this| {
            let text = read(cx, this, String::new(), |i| i.tag_string().to_owned())?;
            cx.text(&text)
        },
        None,
    );
}

/// The members derived from the tags (SPEC-06) and the animation.
fn instance_tag_flags(definition: &mut TypeDef) {
    // The two tag-derived flags of SPEC-06. The tag string decides them, so a write is accepted
    // without effect; no built-in profile writes either (both P3).
    definition.property(
        "IsStatic",
        |cx, this| {
            Ok(boolean(!read(cx, this, false, |i| {
                i.has_tag(tags::STATIC_TAG)
            })?))
        },
        |_, _, _| Ok(()),
        None,
    );
    definition.property(
        "LAxis",
        |cx, this| {
            Ok(boolean(!read(cx, this, false, |i| {
                i.has_tag(tags::GROUND_AXIS_TAG)
            })?))
        },
        |_, _, _| Ok(()),
        None,
    );
    definition.read_only(
        "AnimEnable",
        |cx, this| {
            Ok(boolean(read(
                cx,
                this,
                false,
                PatternInstance::deform_enabled,
            )?))
        },
        None,
    );
}

fn set_constraint(cx: &Cx, this: &Payload, compute: impl FnOnce(Cell) -> Cell) -> mlua::Result<()> {
    let current = with_carrier(cx, this, Cell::new(99, 99), |carrier| {
        carrier_constraint(&carrier)
    })?;
    let wanted = compute(current);
    with_carrier_mut(cx, this, |carrier| match carrier {
        CarrierMut::Shape(shape) => shape.constraint_size = wanted,
        CarrierMut::Pattern(pattern) => pattern.constraint_size = wanted,
    })
}

/// The frame instance a method was called on, when the receiver is one.
fn frame_receiver(call: &Call) -> Option<InstanceTarget> {
    match call.host(1) {
        Some(Payload::Instance(target @ InstanceTarget::Frame(..))) => Some(target),
        _ => None,
    }
}

fn methods(definition: &mut TypeDef) {
    definition.function(
        "GetPos",
        |call| {
            let Some(target) = frame_receiver(call) else {
                return Ok(Some(call.cx.vector3(Vec3::ZERO)?));
            };

            let pivot = if call.is_none_or_nil(2) {
                None
            } else {
                Some(call.vec3(2, "GetPos")?)
            };
            let position = call
                .cx
                .with_instance(&target, |view| {
                    view.frame().map(|(instance, state)| {
                        instance.get_pos_with(state.settings(), pivot.unwrap_or(instance.pivot))
                    })
                })?
                .flatten()
                .unwrap_or(Vec3::ZERO);
            Ok(Some(call.cx.vector3(position)?))
        },
        Some(HostType::Vector3),
    );
    definition.function(
        "GetOriginalPos",
        |call| {
            let pivot = call.vec3(2, "GetOriginalPos")?;
            let position = frame_position(call, |instance| instance.get_original_pos(pivot))?;
            Ok(Some(call.cx.vector3(position)?))
        },
        Some(HostType::Vector3),
    );
    definition.function(
        "GetPosWoLayer",
        |call| {
            let pivot = call.vec3(2, "GetPosWoLayer")?;
            let position = frame_position(call, |instance| instance.get_pos_without_layer(pivot))?;
            Ok(Some(call.cx.vector3(position)?))
        },
        Some(HostType::Vector3),
    );
    definition.function(
        "CalculatePos",
        |call| {
            if let Some(target) = frame_receiver(call) {
                call.cx
                    .with_frame_instance_mut(&target, |instance, settings| {
                        instance.calculate_pos(settings);
                    })?;
            }

            Ok(None)
        },
        None,
    );
    definition.function(
        "ContainsTag",
        |call| {
            let tag = call.string(2, "ContainsTag")?;
            let found = match call.host(1) {
                Some(Payload::Instance(target)) => call
                    .cx
                    .with_instance(&target, |view| view.contains_tag(&tag))?
                    .unwrap_or(false),
                _ => false,
            };
            Ok(Some(boolean(found)))
        },
        None,
    );
    overlap_and_deformation_methods(definition);
}

/// `Overlap`, `OverlapRect` and the deformation a script predicts (REQ-LUA-083).
fn overlap_and_deformation_methods(definition: &mut TypeDef) {
    definition.function(
        "Overlap",
        |call| {
            let Some(target) = frame_receiver(call) else {
                return Ok(Some(boolean(false)));
            };

            if let Some(Payload::Instance(other @ InstanceTarget::Frame(..))) = call.host(2) {
                let delta = if call.is_none_or_nil(3) {
                    0
                } else {
                    call.integer(3, "Overlap")?
                };
                let overlaps = call
                    .cx
                    .with_frame_pair(&target, &other, |me, other| me.overlap(other, delta))?
                    .unwrap_or(false);
                return Ok(Some(boolean(overlaps)));
            }

            if let Some(rectangle) = try_rect(call.value(2)) {
                return Ok(Some(boolean(frame_flag(call, |instance| {
                    instance.overlap_with_rect(&rectangle)
                })?)));
            }

            Err(call.invalid("Overlap"))
        },
        None,
    );
    definition.function(
        "OverlapRect",
        |call| {
            let rectangle = call.rect(2, "OverlapRect")?;
            Ok(Some(boolean(frame_flag(call, |instance| {
                instance.overlap_rect(&rectangle)
            })?)))
        },
        None,
    );
    definition.function(
        "GetDeform",
        |call| {
            let point = call.vec3(2, "GetDeform")?;
            let time = frame_time(call.cx)?;
            let value = frame_position(call, |instance| {
                deformation(instance, point, instance.ori_pos(), time)
            })?;
            Ok(Some(call.cx.vector3(value)?))
        },
        Some(HostType::Vector3),
    );
    definition.function(
        "GetDeformation",
        |call| {
            let point = call.vec3(2, "GetDeformation")?;
            let time = frame_time(call.cx)?;
            let value = frame_position(call, |instance| {
                deformation(
                    instance,
                    point,
                    instance.get_pos_without_layer(instance.deform_pivot),
                    time,
                )
            })?;
            Ok(Some(call.cx.vector3(value)?))
        },
        Some(HostType::Vector3),
    );
}

/// A vector computed from the frame instance the method was called on, zero when there is none.
fn frame_position(
    call: &Call,
    operation: impl FnOnce(&FrameInstance) -> Vec3,
) -> mlua::Result<Vec3> {
    let Some(target) = frame_receiver(call) else {
        return Ok(Vec3::ZERO);
    };

    Ok(call
        .cx
        .with_instance(&target, |view| {
            view.frame().map(|(instance, _)| operation(instance))
        })?
        .flatten()
        .unwrap_or(Vec3::ZERO))
}

/// A truth value computed from the frame instance the method was called on.
fn frame_flag(call: &Call, operation: impl FnOnce(&FrameInstance) -> bool) -> mlua::Result<bool> {
    let Some(target) = frame_receiver(call) else {
        return Ok(false);
    };

    Ok(call
        .cx
        .with_instance(&target, |view| {
            view.frame().map(|(instance, _)| operation(instance))
        })?
        .flatten()
        .unwrap_or(false))
}

// bound vectors, matrices, custom data and the texture slot

fn vec3_ref_of(payload: &Payload) -> Option<Vec3Ref> {
    match payload {
        Payload::Vec3Ref(reference) => Some(*reference),
        _ => None,
    }
}

fn vec3_component(definition: &mut TypeDef, name: &'static str, axis: usize) {
    definition.property(
        name,
        move |cx, this| {
            let value = match vec3_ref_of(this) {
                Some(reference) => read_vec3_ref(cx, &reference)?,
                None => Vec3::ZERO,
            };
            Ok(num([value.x, value.y, value.z][axis]))
        },
        move |cx, this, value| {
            let Some(reference) = vec3_ref_of(this) else {
                return Ok(());
            };

            let mut current = read_vec3_ref(cx, &reference)?;
            let component = cx.real(value, name)?;
            match axis {
                0 => current.x = component,
                1 => current.y = component,
                _ => current.z = component,
            }

            write_vec3_ref(cx, &reference, current)
        },
        None,
    );
}

fn references() -> [TypeDef; 3] {
    [bound_vec3(), bound_vec2(), bound_matrix()]
}

/// The bound vector `Vec3R` (SPEC-05 §4.3).
fn bound_vec3() -> TypeDef {
    let mut vec3 = type_def(HostType::Vec3Ref, "Vec3R");
    vec3_component(&mut vec3, "x", 0);
    vec3_component(&mut vec3, "y", 1);
    vec3_component(&mut vec3, "z", 2);
    vec3.property(
        "content",
        |cx, this| {
            let value = match vec3_ref_of(this) {
                Some(reference) => read_vec3_ref(cx, &reference)?,
                None => Vec3::ZERO,
            };
            cx.vector3(value)
        },
        |cx, this, value| {
            let wanted = cx.vec3(value, "content")?;
            match vec3_ref_of(this) {
                Some(reference) => write_vec3_ref(cx, &reference, wanted),
                None => Ok(()),
            }
        },
        Some(HostType::Vector3),
    );
    vec3.indexer = Some(Arc::new(|cx, this, key| {
        let index = cx.integer(key, "Vec3R")?;
        let value = match vec3_ref_of(this) {
            Some(reference) => read_vec3_ref(cx, &reference)?,
            None => Vec3::ZERO,
        };
        match index {
            0 => Ok(num(value.x)),
            1 => Ok(num(value.y)),
            2 => Ok(num(value.z)),
            _ => Err(cx.raise(&format!("index {index} is out of range for Vec3R"))),
        }
    }));
    vec3.index_setter = Some(Arc::new(|cx, this, key, value| {
        let index = cx.integer(key, "Vec3R")?;
        let component = cx.real(value, "Vec3R")?;
        let Some(reference) = vec3_ref_of(this) else {
            return Ok(());
        };

        let mut current = read_vec3_ref(cx, &reference)?;
        match index {
            0 => current.x = component,
            1 => current.y = component,
            2 => current.z = component,
            _ => return Err(cx.raise(&format!("index {index} is out of range for Vec3R"))),
        }

        write_vec3_ref(cx, &reference, current)
    }));
    vec3.text = Some(Arc::new(|cx, payload| {
        let reference =
            vec3_ref_of(payload).ok_or_else(|| cx.raise("invalid arguments to ToString"))?;
        let value = read_vec3_ref(cx, &reference)?;
        Ok(format!(
            "({}, {}, {})",
            text_of(value.x),
            text_of(value.y),
            text_of(value.z)
        ))
    }));
    vec3
}

/// The bound 2D vector `Vec2R`.
fn bound_vec2() -> TypeDef {
    let mut vec2 = type_def(HostType::Vec2Ref, "Vec2R");
    vec2_component(&mut vec2, "x", 0);
    vec2_component(&mut vec2, "y", 1);
    vec2.property(
        "content",
        |cx, this| {
            let value = read_vec2(cx, this)?;
            cx.vector2(value)
        },
        |cx, this, value| {
            let Some(wanted) = cx.try_vec2(value)? else {
                return Err(cx.invalid("content"));
            };

            write_vec2(cx, this, wanted)
        },
        Some(HostType::Vector2),
    );
    vec2.indexer = Some(Arc::new(|cx, this, key| {
        let index = cx.integer(key, "Vec2R")?;
        let value = read_vec2(cx, this)?;
        match index {
            0 => Ok(num(value.x)),
            1 => Ok(num(value.y)),
            _ => Err(cx.raise(&format!("index {index} is out of range for Vec2R"))),
        }
    }));
    vec2.index_setter = Some(Arc::new(|cx, this, key, value| {
        let index = cx.integer(key, "Vec2R")?;
        let component = cx.real(value, "Vec2R")?;
        let mut current = read_vec2(cx, this)?;
        match index {
            0 => current.x = component,
            1 => current.y = component,
            _ => return Err(cx.raise(&format!("index {index} is out of range for Vec2R"))),
        }

        write_vec2(cx, this, current)
    }));
    vec2
}

/// The names of the sixteen cells of a `Mat4R`: `mRC` is row R, column C.
const NAMES: [[&str; 4]; 4] = [
    ["m00", "m01", "m02", "m03"],
    ["m10", "m11", "m12", "m13"],
    ["m20", "m21", "m22", "m23"],
    ["m30", "m31", "m32", "m33"],
];

/// The bound matrix `Mat4R`.
fn bound_matrix() -> TypeDef {
    // The numeric index is column-major, i = R + 4·C (§4.3).
    let mut matrix = type_def(HostType::Mat4Ref, "Mat4R");
    for (row, names) in NAMES.iter().enumerate() {
        for (column, name) in names.iter().enumerate() {
            let name: &'static str = name;
            matrix.property(
                name,
                move |cx, this| Ok(num(read_mat4(cx, this)?[(row, column)])),
                move |cx, this, value| {
                    let mut current = read_mat4(cx, this)?;
                    current[(row, column)] = cx.real(value, name)?;
                    write_mat4(cx, this, current)
                },
                None,
            );
        }
    }

    matrix.indexer = Some(Arc::new(|cx, this, key| {
        let index = cx.integer(key, "Mat4R")?;
        let (row, column) = matrix_cell(cx, index)?;
        Ok(num(read_mat4(cx, this)?[(row, column)]))
    }));
    matrix.index_setter = Some(Arc::new(|cx, this, key, value| {
        let index = cx.integer(key, "Mat4R")?;
        let (row, column) = matrix_cell(cx, index)?;
        let mut current = read_mat4(cx, this)?;
        current[(row, column)] = cx.real(value, "Mat4R")?;
        write_mat4(cx, this, current)
    }));
    matrix
}

fn matrix_cell(cx: &Cx, index: i32) -> mlua::Result<(usize, usize)> {
    match usize::try_from(index) {
        Ok(position) if position < 16 => Ok((position & 3, position >> 2)),
        _ => Err(cx.raise(&format!("index {index} is out of range for Mat4R"))),
    }
}

fn vec2_component(definition: &mut TypeDef, name: &'static str, axis: usize) {
    definition.property(
        name,
        move |cx, this| {
            let value = read_vec2(cx, this)?;
            Ok(num([value.x, value.y][axis]))
        },
        move |cx, this, value| {
            let mut current = read_vec2(cx, this)?;
            let component = cx.real(value, name)?;
            if axis == 0 {
                current.x = component;
            } else {
                current.y = component;
            }

            write_vec2(cx, this, current)
        },
        None,
    );
}

fn read_vec2(cx: &Cx, this: &Payload) -> mlua::Result<Vec2> {
    match this {
        Payload::Vec2Ref(target, kind) => read_vec2_ref(cx, target, *kind),
        _ => Ok(Vec2::ZERO),
    }
}

fn write_vec2(cx: &Cx, this: &Payload, value: Vec2) -> mlua::Result<()> {
    match this {
        Payload::Vec2Ref(target, kind) => write_vec2_ref(cx, target, *kind, value),
        _ => Ok(()),
    }
}

fn read_mat4(cx: &Cx, this: &Payload) -> mlua::Result<zax_engine3d::vectors::Mat4> {
    match this {
        Payload::Mat4Ref(target, kind) => read_matrix(cx, target, *kind),
        _ => Ok(zax_engine3d::vectors::Mat4::default()),
    }
}

fn write_mat4(cx: &Cx, this: &Payload, value: zax_engine3d::vectors::Mat4) -> mlua::Result<()> {
    match this {
        Payload::Mat4Ref(target, kind) => write_matrix(cx, target, *kind, value),
        _ => Ok(()),
    }
}

/// `CData`: indices 0 to 3 are row 3 of `DeformSpeed`, 4 and up row 3 of `DeformOffset`
/// (REQ-LUA-082). A negative index raises.
fn custom_data() -> TypeDef {
    let mut definition = type_def(HostType::CustomData, "CData");
    definition.indexer = Some(Arc::new(|cx, this, key| {
        let index = custom_index(cx, key)?;
        let Payload::CustomData(target) = this else {
            return Ok(num(0));
        };

        let value = cx
            .with_instance(target, |view| view.custom_data(index))?
            .unwrap_or(0.0);
        Ok(num(value))
    }));
    definition.index_setter = Some(Arc::new(|cx, this, key, value| {
        let index = custom_index(cx, key)?;
        let wanted = cx.real(value, "CData")?;
        if let Payload::CustomData(target) = this {
            cx.with_instance_mut(target, |instance| instance.set_custom_data(index, wanted))?;
        }

        Ok(())
    }));
    definition
}

fn custom_index(cx: &Cx, key: &Value) -> mlua::Result<usize> {
    let index = cx.integer(key, "CData")?;
    usize::try_from(index)
        .map_err(|_| cx.raise(&format!("index {index} is out of range for CData")))
}

/// The texture slot of REQ-LUA-097: an opaque object with the texture size in pixels.
///
/// Before the geometry is built the slot is the size the default texture will have: 8W × 8H
/// texels of the carrying pattern (SPEC-04B REQ-E3D-426).
fn slot() -> TypeDef {
    let mut definition = type_def(HostType::Slot, "SlotTexture");
    definition.read_only("w", |cx, this| Ok(num(slot_size(cx, this)?.x)), None);
    definition.read_only("h", |cx, this| Ok(num(slot_size(cx, this)?.y)), None);
    definition
}

fn slot_size(cx: &Cx, this: &Payload) -> mlua::Result<Cell> {
    let Payload::Slot(owner) = this else {
        return Ok(Cell::default());
    };

    let found = cx.state(|state| {
        state
            .find_pattern(*owner)
            .and_then(|id| state.patterns().get(id))
            .map(|pattern| Cell::new(8 * pattern.width(), 8 * pattern.height()))
    })?;
    Ok(found.unwrap_or_default())
}
