//! The value types of SPEC-05 §4.3 — `Vector3`, `Vector2`, `Quaternion`, `Color`,
//! `IntVector2`, `IntVector3`, `Rect` — and their class tables.
//!
//! They are copies: a value read from an object and then changed leaves the object alone
//! (REQ-LUA-044). The operators and the equality tolerances are the ones §4.3 fixes
//! (REQ-LUA-096, REQ-LUA-102).

use mlua::Value;
use zax_engine3d::vectors::{Cell, Rect, Vec2, Vec3};

use super::{boolean, num, type_def};
use crate::binding::cx::{try_color, try_quaternion, Call, Cx};
use crate::binding::payload::{HostType, Payload};
use crate::binding::table::TypeDef;
use crate::quaternion::Quat;
use crate::values::{number_format, LuaColor, LuaIntVector3};

const VECTOR_EQUALITY_TOLERANCE: f32 = 1e-10;
const QUATERNION_EQUALITY_TOLERANCE: f32 = 1e-6;
const DEGREES: f32 = 180.0 / std::f32::consts::PI;
const RADIANS: f32 = std::f32::consts::PI / 180.0;

pub(super) fn declare(types: &mut Vec<TypeDef>) {
    types.extend(vector3());
    types.extend(vector2());
    types.extend(quaternion());
    types.extend(color());
    types.extend(int_vector2());
    types.extend(int_vector3());
    types.extend(rectangle());
}

pub(super) fn text_of(value: f32) -> String {
    number_format::to_text(f64::from(value))
}

/// A component of a value type, read as a real and written as a real.
fn real_component(
    definition: &mut TypeDef,
    name: &'static str,
    read: fn(&Payload) -> f32,
    write: fn(&mut Payload, f32),
) {
    definition.property(
        name,
        move |_, payload| Ok(num(read(payload))),
        move |cx, payload, value| {
            write(payload, cx.real(value, name)?);
            Ok(())
        },
        None,
    );
}

/// A component of an integer value type.
fn int_component(
    definition: &mut TypeDef,
    name: &'static str,
    read: fn(&Payload) -> i32,
    write: fn(&mut Payload, i32),
) {
    definition.property(
        name,
        move |_, payload| Ok(num(read(payload))),
        move |cx, payload, value| {
            write(payload, cx.integer(value, name)?);
            Ok(())
        },
        None,
    );
}

fn vec3_of(payload: &Payload) -> Vec3 {
    match payload {
        Payload::Vector3(value) => *value,
        _ => Vec3::ZERO,
    }
}

fn vector3() -> [TypeDef; 2] {
    [vector3_value(), vector3_class()]
}

/// The type `Vector3`.
fn vector3_value() -> TypeDef {
    let mut definition = type_def(HostType::Vector3, "Vector3");
    real_component(
        &mut definition,
        "x",
        |p| vec3_of(p).x,
        |p, v| {
            if let Payload::Vector3(vector) = p {
                vector.x = v;
            }
        },
    );
    real_component(
        &mut definition,
        "y",
        |p| vec3_of(p).y,
        |p, v| {
            if let Payload::Vector3(vector) = p {
                vector.y = v;
            }
        },
    );
    real_component(
        &mut definition,
        "z",
        |p| vec3_of(p).z,
        |p, v| {
            if let Payload::Vector3(vector) = p {
                vector.z = v;
            }
        },
    );
    definition.read_only(
        "magnitude",
        |_, p| Ok(num(vec3_of(p).squared_length().sqrt())),
        None,
    );
    definition.read_only(
        "sqrMagnitude",
        |_, p| Ok(num(vec3_of(p).squared_length())),
        None,
    );
    definition.read_only(
        "normalized",
        |cx, p| {
            let value = vec3_of(p);
            let length = value.squared_length().sqrt();
            cx.vector3(if length <= 1e-5 {
                Vec3::ZERO
            } else {
                value * (1.0 / length)
            })
        },
        Some(HostType::Vector3),
    );
    definition.equal = Some(std::sync::Arc::new(|a, b| {
        (vec3_of(a) - vec3_of(b)).squared_length() < VECTOR_EQUALITY_TOLERANCE
    }));
    definition.add = Some(std::sync::Arc::new(|cx, a, b| {
        cx.vector3(operand3(cx, a, "+")? + operand3(cx, b, "+")?)
    }));
    definition.subtract = Some(std::sync::Arc::new(|cx, a, b| {
        cx.vector3(operand3(cx, a, "-")? - operand3(cx, b, "-")?)
    }));
    definition.negate = Some(std::sync::Arc::new(|cx, a, _| {
        let value = operand3(cx, a, "-")?;
        cx.vector3(Vec3::new(-value.x, -value.y, -value.z))
    }));
    // k * v and v * k; the component-by-component product is not part of §4.3.
    definition.multiply = Some(std::sync::Arc::new(|cx, a, b| {
        if let Some(factor) = scalar(a) {
            return cx.vector3(operand3(cx, b, "*")? * factor);
        }

        if let Some(factor) = scalar(b) {
            return cx.vector3(operand3(cx, a, "*")? * factor);
        }

        Err(cx.invalid("Vector3 *"))
    }));
    definition.divide = Some(std::sync::Arc::new(|cx, a, b| {
        let Some(divisor) = scalar(b) else {
            return Err(cx.invalid("Vector3 /"));
        };

        cx.vector3(operand3(cx, a, "/")? * (1.0 / divisor))
    }));
    definition.text = Some(std::sync::Arc::new(|_, p| {
        let value = vec3_of(p);
        Ok(format!(
            "({}, {}, {})",
            text_of(value.x),
            text_of(value.y),
            text_of(value.z)
        ))
    }));
    definition
}

/// The class table `Vector3`: the constants, the functions and the constructor.
fn vector3_class() -> TypeDef {
    let mut table = type_def(HostType::Vector3Class, "Vector3Classe");
    for (name, value) in [
        ("zero", Vec3::ZERO),
        ("one", Vec3::ONE),
        ("back", Vec3::new(0.0, 0.0, -1.0)),
        ("forward", Vec3::new(0.0, 0.0, 1.0)),
        ("up", Vec3::new(0.0, 1.0, 0.0)),
        ("down", Vec3::new(0.0, -1.0, 0.0)),
        ("left", Vec3::new(-1.0, 0.0, 0.0)),
        ("right", Vec3::new(1.0, 0.0, 0.0)),
    ] {
        table.read_only(
            name,
            move |cx, _| cx.vector3(value),
            Some(HostType::Vector3),
        );
    }

    table.function(
        "Cross",
        |call| {
            let a = call.vec3(1, "Cross")?;
            let c = call.vec3(2, "Cross")?;
            Ok(Some(call.cx.vector3(Vec3::new(
                (a.y * c.z) - (a.z * c.y),
                (a.z * c.x) - (a.x * c.z),
                (a.x * c.y) - (a.y * c.x),
            ))?))
        },
        Some(HostType::Vector3),
    );
    table.function(
        "Dot",
        |call| {
            let a = call.vec3(1, "Dot")?;
            let c = call.vec3(2, "Dot")?;
            Ok(Some(num((a.x * c.x) + (a.y * c.y) + (a.z * c.z))))
        },
        None,
    );
    table.function(
        "Distance",
        |call| {
            let a = call.vec3(1, "Distance")?;
            let c = call.vec3(2, "Distance")?;
            Ok(Some(num(Vec3::distance(a, c))))
        },
        None,
    );
    table.function(
        "Lerp",
        |call| {
            let a = call.vec3(1, "Lerp")?;
            let c = call.vec3(2, "Lerp")?;
            let t = call.real(3, "Lerp")?.clamp(0.0, 1.0);
            Ok(Some(call.cx.vector3(a + ((c - a) * t))?))
        },
        Some(HostType::Vector3),
    );

    // Vector3(x, y [, z]); no argument gives the zero vector (§4.3).
    table.call = Some(std::sync::Arc::new(|call| {
        if call.is_none_or_nil(2) {
            return Ok(Some(call.cx.vector3(Vec3::ZERO)?));
        }

        let x = call.real(2, "Vector3")?;
        let y = call.real(3, "Vector3")?;
        let z = narrow(call.optional_number(4, "Vector3", 0.0)?);
        Ok(Some(call.cx.vector3(Vec3::new(x, y, z))?))
    }));
    table.call_result = Some(HostType::Vector3);
    table
}

/// The operand of a `Vector3` operator, or `invalid arguments to Vector3 <what>`.
fn operand3(cx: &Cx, value: &Value, what: &str) -> mlua::Result<Vec3> {
    cx.try_vec3(value)?
        .ok_or_else(|| cx.invalid(&format!("Vector3 {what}")))
}

/// The number an operator operand is, as a single-precision real.
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)] // The C# casts to float.
fn scalar(value: &Value) -> Option<f32> {
    match value {
        Value::Number(number) => Some(*number as f32),
        Value::Integer(integer) => Some(*integer as f32),
        _ => None,
    }
}

#[allow(clippy::cast_possible_truncation)] // The C# casts the double to float.
const fn narrow(value: f64) -> f32 {
    value as f32
}

fn vec2_of(payload: &Payload) -> Vec2 {
    match payload {
        Payload::Vector2(value) => *value,
        _ => Vec2::ZERO,
    }
}

fn operand2(cx: &Cx, value: &Value) -> mlua::Result<Vec2> {
    Ok(cx.try_vec2(value)?.unwrap_or_default())
}

fn vector2() -> [TypeDef; 2] {
    let mut definition = type_def(HostType::Vector2, "Vector2");
    real_component(
        &mut definition,
        "x",
        |p| vec2_of(p).x,
        |p, v| {
            if let Payload::Vector2(vector) = p {
                vector.x = v;
            }
        },
    );
    real_component(
        &mut definition,
        "y",
        |p| vec2_of(p).y,
        |p, v| {
            if let Payload::Vector2(vector) = p {
                vector.y = v;
            }
        },
    );
    definition.read_only(
        "magnitude",
        |_, p| {
            let value = vec2_of(p);
            Ok(num(((value.x * value.x) + (value.y * value.y)).sqrt()))
        },
        None,
    );
    definition.equal = Some(std::sync::Arc::new(|a, b| {
        let (a, c) = (vec2_of(a), vec2_of(b));
        let (dx, dy) = (a.x - c.x, a.y - c.y);
        (dx * dx) + (dy * dy) < VECTOR_EQUALITY_TOLERANCE
    }));
    definition.add = Some(std::sync::Arc::new(|cx, a, b| {
        let (a, c) = (operand2(cx, a)?, operand2(cx, b)?);
        cx.vector2(Vec2::new(a.x + c.x, a.y + c.y))
    }));
    definition.subtract = Some(std::sync::Arc::new(|cx, a, b| {
        let (a, c) = (operand2(cx, a)?, operand2(cx, b)?);
        cx.vector2(Vec2::new(a.x - c.x, a.y - c.y))
    }));
    definition.negate = Some(std::sync::Arc::new(|cx, a, _| {
        let value = operand2(cx, a)?;
        cx.vector2(Vec2::new(-value.x, -value.y))
    }));
    definition.multiply = Some(std::sync::Arc::new(|cx, a, b| {
        if let Some(factor) = scalar(a) {
            let value = operand2(cx, b)?;
            return cx.vector2(Vec2::new(value.x * factor, value.y * factor));
        }

        if let Some(factor) = scalar(b) {
            let value = operand2(cx, a)?;
            return cx.vector2(Vec2::new(value.x * factor, value.y * factor));
        }

        Err(cx.invalid("Vector2 *"))
    }));
    definition.divide = Some(std::sync::Arc::new(|cx, a, b| {
        let Some(divisor) = scalar(b) else {
            return Err(cx.invalid("Vector2 /"));
        };

        let value = operand2(cx, a)?;
        cx.vector2(Vec2::new(value.x / divisor, value.y / divisor))
    }));
    definition.text = Some(std::sync::Arc::new(|_, p| {
        let value = vec2_of(p);
        Ok(format!("({}, {})", text_of(value.x), text_of(value.y)))
    }));

    let mut table = type_def(HostType::Vector2Class, "Vector2Classe");
    table.read_only(
        "zero",
        |cx, _| cx.vector2(Vec2::ZERO),
        Some(HostType::Vector2),
    );
    table.read_only(
        "one",
        |cx, _| cx.vector2(Vec2::ONE),
        Some(HostType::Vector2),
    );
    table.call = Some(std::sync::Arc::new(|call| {
        if call.is_none_or_nil(2) {
            return Ok(Some(call.cx.vector2(Vec2::ZERO)?));
        }

        let x = call.real(2, "Vector2")?;
        let y = call.real(3, "Vector2")?;
        Ok(Some(call.cx.vector2(Vec2::new(x, y))?))
    }));
    table.call_result = Some(HostType::Vector2);
    [definition, table]
}

fn quat_of(payload: &Payload) -> Quat {
    match payload {
        Payload::Quaternion(parts) => Quat::from_parts(*parts),
        _ => Quat::new(0.0, 0.0, 0.0, 0.0),
    }
}

fn quaternion_value(cx: &Cx, value: Quat) -> mlua::Result<Value> {
    cx.value(Payload::Quaternion(value.parts()))
}

/// `Euler = Ry(y) · Rx(x) · Rz(z)` with `Rₐ(θ) = (sin(θ/2)·â, cos(θ/2))` (§4.3).
fn from_euler(x: f32, y: f32, z: f32) -> Quat {
    let qx = Quat::from_axis_angle(Vec3::new(1.0, 0.0, 0.0), x * RADIANS);
    let qy = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), y * RADIANS);
    let qz = Quat::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), z * RADIANS);
    qy.multiply(qx).multiply(qz)
}

/// The angles `(ax, ay, az)` in `[0, 360)` such that [`from_euler`] rebuilds the same rotation,
/// `ax` being the principal `asin` value (§4.3, type `Quaternion`).
fn to_euler(q: Quat) -> Vec3 {
    let q = if q.length_squared() > 0.0 {
        q.normalize()
    } else {
        q
    };

    // Entries of R = Ry(ay)·Rx(ax)·Rz(az) read off the quaternion.
    let sin_x = (2.0 * ((q.w * q.x) - (q.y * q.z))).clamp(-1.0, 1.0);
    let x = sin_x.asin();
    let (y, z);
    if sin_x.abs() > 1.0 - 1e-6 {
        // cos(ax) is zero: ay and az are no longer independent, so az is taken as 0.
        y = (2.0 * ((q.w * q.y) - (q.x * q.z))).atan2(1.0 - (2.0 * ((q.y * q.y) + (q.z * q.z))));
        z = 0.0;
    } else {
        z = (2.0 * ((q.x * q.y) + (q.w * q.z))).atan2(1.0 - (2.0 * ((q.x * q.x) + (q.z * q.z))));
        y = (2.0 * ((q.x * q.z) + (q.w * q.y))).atan2(1.0 - (2.0 * ((q.x * q.x) + (q.y * q.y))));
    }

    let wrap = |degrees: f32| {
        let wrapped = degrees % 360.0;
        if wrapped < 0.0 {
            wrapped + 360.0
        } else {
            wrapped
        }
    };
    Vec3::new(wrap(x * DEGREES), wrap(y * DEGREES), wrap(z * DEGREES))
}

fn quaternion_argument(call: &Call, index: usize, member: &str) -> mlua::Result<Quat> {
    try_quaternion(call.value(index))
        .map(Quat::from_parts)
        .ok_or_else(|| call.invalid(member))
}

fn quaternion() -> [TypeDef; 2] {
    [quaternion_value_type(), quaternion_class()]
}

/// The type `Quaternion`.
fn quaternion_value_type() -> TypeDef {
    let mut definition = type_def(HostType::Quaternion, "Quaternion");
    real_component(
        &mut definition,
        "x",
        |p| quat_of(p).x,
        |p, v| {
            if let Payload::Quaternion(parts) = p {
                parts[0] = v;
            }
        },
    );
    real_component(
        &mut definition,
        "y",
        |p| quat_of(p).y,
        |p, v| {
            if let Payload::Quaternion(parts) = p {
                parts[1] = v;
            }
        },
    );
    real_component(
        &mut definition,
        "z",
        |p| quat_of(p).z,
        |p, v| {
            if let Payload::Quaternion(parts) = p {
                parts[2] = v;
            }
        },
    );
    real_component(
        &mut definition,
        "w",
        |p| quat_of(p).w,
        |p, v| {
            if let Payload::Quaternion(parts) = p {
                parts[3] = v;
            }
        },
    );
    definition.read_only(
        "eulerAngles",
        |cx, p| cx.vector3(to_euler(quat_of(p))),
        Some(HostType::Vector3),
    );
    definition.equal = Some(std::sync::Arc::new(|a, b| {
        quat_of(a).dot(quat_of(b)).abs() > 1.0 - QUATERNION_EQUALITY_TOLERANCE
    }));
    definition.multiply = Some(std::sync::Arc::new(|cx, a, b| {
        let left = try_quaternion(a).map_or(Quat::new(0.0, 0.0, 0.0, 0.0), Quat::from_parts);
        // q1 * q2 applies q2 first, then q1 (§4.3).
        if let Some(right) = try_quaternion(b) {
            return quaternion_value(cx, left.multiply(Quat::from_parts(right)));
        }

        if let Some(vector) = cx.try_vec3(b)? {
            return cx.vector3(left.rotate(vector));
        }

        Err(cx.invalid("Quaternion *"))
    }));
    definition.text = Some(std::sync::Arc::new(|_, p| {
        let value = quat_of(p);
        Ok(format!(
            "({}, {}, {}, {})",
            text_of(value.x),
            text_of(value.y),
            text_of(value.z),
            text_of(value.w)
        ))
    }));
    definition
}

/// The class table `Quaternion`: the identity and the constructors.
fn quaternion_class() -> TypeDef {
    let mut table = type_def(HostType::QuaternionClass, "QuaternionClasse");
    table.read_only(
        "identity",
        |cx, _| quaternion_value(cx, Quat::IDENTITY),
        Some(HostType::Quaternion),
    );
    table.function(
        "Euler",
        |call| {
            if let Some(angles) = call.cx.try_vec3(call.value(1))? {
                return Ok(Some(quaternion_value(
                    call.cx,
                    from_euler(angles.x, angles.y, angles.z),
                )?));
            }

            let x = call.real(1, "Euler")?;
            let y = call.real(2, "Euler")?;
            let z = call.real(3, "Euler")?;
            Ok(Some(quaternion_value(call.cx, from_euler(x, y, z))?))
        },
        Some(HostType::Quaternion),
    );
    table.function(
        "AngleAxis",
        |call| {
            let angle = call.real(1, "AngleAxis")?;
            let axis = call.vec3(2, "AngleAxis")?;
            let length = axis.squared_length().sqrt();
            if length <= 1e-6 {
                return Ok(Some(quaternion_value(call.cx, Quat::IDENTITY)?));
            }

            let unit = Vec3::new(axis.x / length, axis.y / length, axis.z / length);
            Ok(Some(quaternion_value(
                call.cx,
                Quat::from_axis_angle(unit, angle * RADIANS),
            )?))
        },
        Some(HostType::Quaternion),
    );
    table.function(
        "Lerp",
        |call| {
            let a = quaternion_argument(call, 1, "Lerp")?;
            let mut c = quaternion_argument(call, 2, "Lerp")?;
            let t = call.real(3, "Lerp")?.clamp(0.0, 1.0);
            if a.dot(c) < 0.0 {
                c = c.negate();
            }

            let mixed = Quat::new(
                (a.x * (1.0 - t)) + (c.x * t),
                (a.y * (1.0 - t)) + (c.y * t),
                (a.z * (1.0 - t)) + (c.z * t),
                (a.w * (1.0 - t)) + (c.w * t),
            );
            Ok(Some(quaternion_value(call.cx, mixed.normalize())?))
        },
        Some(HostType::Quaternion),
    );
    table.function(
        "Slerp",
        |call| {
            let a = quaternion_argument(call, 1, "Slerp")?;
            let c = quaternion_argument(call, 2, "Slerp")?;
            let t = call.real(3, "Slerp")?.clamp(0.0, 1.0);
            Ok(Some(quaternion_value(call.cx, Quat::slerp(a, c, t))?))
        },
        Some(HostType::Quaternion),
    );
    table
}

fn color_of(payload: &Payload) -> LuaColor {
    match payload {
        Payload::Color(value) => *value,
        _ => LuaColor::BLACK,
    }
}

fn operand_color(value: &Value) -> LuaColor {
    try_color(value).unwrap_or(LuaColor::new(0.0, 0.0, 0.0, 0.0))
}

fn scale_color(cx: &Cx, value: LuaColor, factor: f32) -> mlua::Result<Value> {
    cx.color(LuaColor::new(
        value.r * factor,
        value.g * factor,
        value.b * factor,
        value.a * factor,
    ))
}

fn color() -> [TypeDef; 2] {
    let mut definition = type_def(HostType::Color, "Color");
    real_component(
        &mut definition,
        "r",
        |p| color_of(p).r,
        |p, v| {
            if let Payload::Color(color) = p {
                color.r = v;
            }
        },
    );
    real_component(
        &mut definition,
        "g",
        |p| color_of(p).g,
        |p, v| {
            if let Payload::Color(color) = p {
                color.g = v;
            }
        },
    );
    real_component(
        &mut definition,
        "b",
        |p| color_of(p).b,
        |p, v| {
            if let Payload::Color(color) = p {
                color.b = v;
            }
        },
    );
    real_component(
        &mut definition,
        "a",
        |p| color_of(p).a,
        |p, v| {
            if let Payload::Color(color) = p {
                color.a = v;
            }
        },
    );
    definition.equal = Some(std::sync::Arc::new(|a, b| color_of(a) == color_of(b)));
    definition.add = Some(std::sync::Arc::new(|cx, a, b| {
        let (a, c) = (operand_color(a), operand_color(b));
        cx.color(LuaColor::new(a.r + c.r, a.g + c.g, a.b + c.b, a.a + c.a))
    }));
    definition.subtract = Some(std::sync::Arc::new(|cx, a, b| {
        let (a, c) = (operand_color(a), operand_color(b));
        cx.color(LuaColor::new(a.r - c.r, a.g - c.g, a.b - c.b, a.a - c.a))
    }));
    definition.multiply = Some(std::sync::Arc::new(|cx, a, b| {
        if let Some(factor) = scalar(a) {
            return scale_color(cx, operand_color(b), factor);
        }

        if let Some(factor) = scalar(b) {
            return scale_color(cx, operand_color(a), factor);
        }

        let (a, c) = (operand_color(a), operand_color(b));
        cx.color(LuaColor::new(a.r * c.r, a.g * c.g, a.b * c.b, a.a * c.a))
    }));
    definition.text = Some(std::sync::Arc::new(|_, p| {
        let value = color_of(p);
        Ok(format!(
            "({}, {}, {}, {})",
            text_of(value.r),
            text_of(value.g),
            text_of(value.b),
            text_of(value.a)
        ))
    }));

    let mut table = type_def(HostType::ColorClass, "ColorClasse");
    table.read_only(
        "black",
        |cx, _| cx.color(LuaColor::BLACK),
        Some(HostType::Color),
    );
    table.read_only(
        "white",
        |cx, _| cx.color(LuaColor::WHITE),
        Some(HostType::Color),
    );
    table.call = Some(std::sync::Arc::new(|call| {
        let r = call.real(2, "Color")?;
        let g = call.real(3, "Color")?;
        let b = call.real(4, "Color")?;
        let a = narrow(call.optional_number(5, "Color", 1.0)?);
        Ok(Some(call.cx.color(LuaColor::new(r, g, b, a))?))
    }));
    table.call_result = Some(HostType::Color);
    [definition, table]
}

fn cell_of(payload: &Payload) -> Cell {
    match payload {
        Payload::IntVector2(value) => *value,
        _ => Cell::default(),
    }
}

fn operand_cell(cx: &Cx, value: &Value) -> mlua::Result<Cell> {
    Ok(cx.try_int_vector2(value)?.unwrap_or_default())
}

fn int3_of(payload: &Payload) -> LuaIntVector3 {
    match payload {
        Payload::IntVector3(value) => *value,
        _ => LuaIntVector3::default(),
    }
}

fn int_vector2() -> [TypeDef; 2] {
    let mut definition = type_def(HostType::IntVector2, "IntVector2");
    int_component(
        &mut definition,
        "x",
        |p| cell_of(p).x,
        |p, v| {
            if let Payload::IntVector2(cell) = p {
                cell.x = v;
            }
        },
    );
    int_component(
        &mut definition,
        "y",
        |p| cell_of(p).y,
        |p, v| {
            if let Payload::IntVector2(cell) = p {
                cell.y = v;
            }
        },
    );
    definition.equal = Some(std::sync::Arc::new(|a, b| cell_of(a) == cell_of(b)));
    definition.add = Some(std::sync::Arc::new(|cx, a, b| {
        cx.int_vector2(operand_cell(cx, a)? + operand_cell(cx, b)?)
    }));
    definition.subtract = Some(std::sync::Arc::new(|cx, a, b| {
        cx.int_vector2(operand_cell(cx, a)? - operand_cell(cx, b)?)
    }));
    definition.divide = Some(std::sync::Arc::new(|cx, a, b| {
        let Some(divisor) = scalar_number(b) else {
            return Err(cx.invalid("IntVector2 /"));
        };

        let divisor = number_format::to_i32(divisor);
        let value = operand_cell(cx, a)?;
        match (value.x.checked_div(divisor), value.y.checked_div(divisor)) {
            (Some(x), Some(y)) => cx.int_vector2(Cell::new(x, y)),
            _ => Err(cx.raise("IntVector2 / 0")),
        }
    }));
    definition.text = Some(std::sync::Arc::new(|_, p| {
        let value = cell_of(p);
        Ok(format!("({}, {})", value.x, value.y))
    }));

    let mut table = type_def(HostType::IntVector2Class, "IntVector2Classe");
    table.read_only(
        "zero",
        |cx, _| cx.int_vector2(Cell::default()),
        Some(HostType::IntVector2),
    );
    table.read_only(
        "one",
        |cx, _| cx.int_vector2(Cell::new(1, 1)),
        Some(HostType::IntVector2),
    );
    table.call = Some(std::sync::Arc::new(|call| {
        if call.is_none_or_nil(2) {
            return Ok(Some(call.cx.int_vector2(Cell::default())?));
        }

        let x = call.integer(2, "IntVector2")?;
        let y = call.integer(3, "IntVector2")?;
        Ok(Some(call.cx.int_vector2(Cell::new(x, y))?))
    }));
    table.call_result = Some(HostType::IntVector2);
    [definition, table]
}

/// The types `IntVector3` and its class table.
fn int_vector3() -> [TypeDef; 2] {
    let mut three = type_def(HostType::IntVector3, "IntVector3");
    int_component(
        &mut three,
        "x",
        |p| int3_of(p).x,
        |p, v| {
            if let Payload::IntVector3(vector) = p {
                vector.x = v;
            }
        },
    );
    int_component(
        &mut three,
        "y",
        |p| int3_of(p).y,
        |p, v| {
            if let Payload::IntVector3(vector) = p {
                vector.y = v;
            }
        },
    );
    int_component(
        &mut three,
        "z",
        |p| int3_of(p).z,
        |p, v| {
            if let Payload::IntVector3(vector) = p {
                vector.z = v;
            }
        },
    );
    three.equal = Some(std::sync::Arc::new(|a, b| int3_of(a) == int3_of(b)));

    let mut three_table = type_def(HostType::IntVector3Class, "IntVector3Classe");
    three_table.call = Some(std::sync::Arc::new(|call| {
        let x = call.integer(2, "IntVector3")?;
        let y = call.integer(3, "IntVector3")?;
        let z = call.integer(4, "IntVector3")?;
        Ok(Some(call.cx.value(Payload::IntVector3(LuaIntVector3 {
            x,
            y,
            z,
        }))?))
    }));
    three_table.call_result = Some(HostType::IntVector3);
    [three, three_table]
}

/// The number an operator operand is, as a double.
#[allow(clippy::cast_precision_loss)] // lua_tonumber converts an integer to a double.
const fn scalar_number(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => Some(*number),
        Value::Integer(integer) => Some(*integer as f64),
        _ => None,
    }
}

fn rect_of(payload: &Payload) -> Rect {
    match payload {
        Payload::Rect(value) => *value,
        _ => Rect::default(),
    }
}

fn rectangle() -> [TypeDef; 2] {
    let mut definition = type_def(HostType::Rect, "Rect");
    real_component(
        &mut definition,
        "x",
        |p| rect_of(p).x,
        |p, v| {
            if let Payload::Rect(rectangle) = p {
                rectangle.x = v;
            }
        },
    );
    real_component(
        &mut definition,
        "y",
        |p| rect_of(p).y,
        |p, v| {
            if let Payload::Rect(rectangle) = p {
                rectangle.y = v;
            }
        },
    );
    real_component(
        &mut definition,
        "width",
        |p| rect_of(p).width,
        |p, v| {
            if let Payload::Rect(rectangle) = p {
                rectangle.width = v;
            }
        },
    );
    real_component(
        &mut definition,
        "height",
        |p| rect_of(p).height,
        |p, v| {
            if let Payload::Rect(rectangle) = p {
                rectangle.height = v;
            }
        },
    );
    definition.equal = Some(std::sync::Arc::new(|a, b| rect_of(a) == rect_of(b)));
    definition.function(
        "Overlaps",
        |call| {
            let mine = call
                .host(1)
                .map(|payload| rect_of(&payload))
                .unwrap_or_default();
            let other = call.rect(2, "Overlaps")?;
            Ok(Some(boolean(mine.overlaps(&other))))
        },
        None,
    );
    definition.text = Some(std::sync::Arc::new(|_, p| {
        let value = rect_of(p);
        Ok(format!(
            "({}, {}, {}, {})",
            text_of(value.x),
            text_of(value.y),
            text_of(value.width),
            text_of(value.height)
        ))
    }));

    let mut table = type_def(HostType::RectClass, "RectClasse");
    table.call = Some(std::sync::Arc::new(|call| {
        let x = call.real(2, "Rect")?;
        let y = call.real(3, "Rect")?;
        let width = call.real(4, "Rect")?;
        let height = call.real(5, "Rect")?;
        Ok(Some(
            call.cx
                .value(Payload::Rect(Rect::new(x, y, width, height)))?,
        ))
    }));
    table.call_result = Some(HostType::Rect);
    [definition, table]
}
