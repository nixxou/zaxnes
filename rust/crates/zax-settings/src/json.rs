//! A setting value as the JSON of SPEC-09 §4.5.1, and back: booleans as `true`/`false`, numbers
//! as numbers, enumerations as the label of their domain (the number is read as well), colours as
//! `"#RRGGBBAA"`, vectors and rectangles as arrays of numbers.

use serde_json::{Number, Value};

use crate::meta::{SettingDescriptor, SettingKind};
use crate::num;
use crate::values::{Color, Rect, SettingValue, Vector3};

/// The JSON form of `value` for `descriptor`, or none when the shape has no JSON form (REQ-PERS-016).
#[must_use]
pub fn to_json(descriptor: &SettingDescriptor, value: &SettingValue) -> Option<Value> {
    match value {
        SettingValue::Bool(flag) => Some(Value::Bool(*flag)),
        SettingValue::Int(whole) => Some(Value::from(*whole)),
        SettingValue::Float(real) => float_json(*real),
        SettingValue::Enum(number) => Some(enum_json(descriptor, *number)),
        SettingValue::Color(colour) => Some(Value::String(colour.to_string())),
        SettingValue::Vector3(vector) => vector_json(*vector),
        SettingValue::Text(text) => Some(Value::String(text.clone())),
        SettingValue::Rect(rect) => Some(rect_json(*rect)),
        SettingValue::Opaque(_) => None,
    }
}

/// The value `node` holds for `descriptor`, or none when the node has the wrong type. The caller
/// then keeps the default, silently and for that value alone (REQ-PERS-022).
#[must_use]
pub fn from_json(descriptor: &SettingDescriptor, node: &Value) -> Option<SettingValue> {
    match descriptor.kind() {
        SettingKind::Bool => node.as_bool().map(SettingValue::Bool),
        SettingKind::Int => node
            .as_f64()
            .map(|whole| SettingValue::Int(num::round_to_i32(whole))),
        SettingKind::Float => node
            .as_f64()
            .map(|real| SettingValue::Float(num::to_f32(real))),
        SettingKind::Enum => enum_from_json(descriptor, node),
        // A text that is not a colour is the wrong type of value, so the default is kept.
        SettingKind::Color => node
            .as_str()?
            .parse::<Color>()
            .ok()
            .map(SettingValue::Color),
        SettingKind::Vector3 => {
            let [x, y, z] = numbers::<3>(node)?;
            Some(SettingValue::Vector3(Vector3::new(
                num::to_f32(x),
                num::to_f32(y),
                num::to_f32(z),
            )))
        }
        SettingKind::Rect => {
            let [x, y, width, height] = numbers::<4>(node)?;
            Some(SettingValue::Rect(Rect::new(
                num::truncate_to_i32(x),
                num::truncate_to_i32(y),
                num::truncate_to_i32(width),
                num::truncate_to_i32(height),
            )))
        }
        SettingKind::Text => node
            .as_str()
            .map(|text| SettingValue::Text(text.to_owned())),
        SettingKind::Opaque => None,
    }
}

/// The JSON of an enumerated value: its label when the domain has one, else its number.
fn enum_json(descriptor: &SettingDescriptor, number: i32) -> Value {
    match descriptor.domain().label_of(f64::from(number)) {
        Some(label) => Value::String(label.to_owned()),
        None => Value::from(number),
    }
}

fn enum_from_json(descriptor: &SettingDescriptor, node: &Value) -> Option<SettingValue> {
    if let Some(number) = node.as_f64() {
        return Some(SettingValue::Enum(num::truncate_to_i32(number)));
    }
    let label = node.as_str()?;
    let number = descriptor.domain().value_of(label)?;
    Some(SettingValue::Enum(num::truncate_to_i32(number)))
}

/// The shortest text that reads back as the same float, as System.Text.Json writes it: `90` for
/// `90f`, `0.4` for `0.4f`. A value with no JSON form (NaN, infinity) has none.
fn float_json(value: f32) -> Option<Value> {
    format!("{value}").parse::<Number>().ok().map(Value::Number)
}

fn vector_json(vector: Vector3) -> Option<Value> {
    Some(Value::Array(vec![
        float_json(vector.x)?,
        float_json(vector.y)?,
        float_json(vector.z)?,
    ]))
}

fn rect_json(rect: Rect) -> Value {
    Value::Array(vec![
        Value::from(rect.x),
        Value::from(rect.y),
        Value::from(rect.width),
        Value::from(rect.height),
    ])
}

/// The `N` numbers of an array of exactly `N` elements, or none.
fn numbers<const N: usize>(node: &Value) -> Option<[f64; N]> {
    let elements = node.as_array().filter(|elements| elements.len() == N)?;
    let mut values = [0.0; N];
    for (slot, element) in values.iter_mut().zip(elements) {
        *slot = element.as_f64()?;
    }
    Some(values)
}
