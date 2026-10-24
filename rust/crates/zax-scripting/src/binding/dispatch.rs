//! The machinery that turns the tables of [`super::table`] into Lua behaviour: member lookup,
//! the exact errors of REQ-LUA-042, the operators and the object cache.

use std::sync::LazyLock;

use mlua::{AnyUserData, Function, Lua, MetaMethod, MultiValue, UserData, UserDataMethods, Value};

use super::cx::{host_payload, Call, Cx, HostCache};
use super::payload::{Host, HostType, Payload};
use super::table::{Api, MemberKind, OperatorFn, TypeDef};
use crate::values::number_format;

/// The API of SPEC-05 §4, built once: it holds only names and functions, no Lua state.
pub(crate) fn api() -> &'static Api {
    static API: LazyLock<Api> = LazyLock::new(crate::api::build);
    &API
}

/// The Lua functions that stand for the methods and table functions of each type, created once
/// per Lua state so that `obj:Method` costs no allocation.
struct MethodCache(Vec<Vec<Option<Function>>>);

/// Installs the per-state parts of the binding: the weak object cache and the method functions;
/// with `fast`, the raw host values of [`super::raw`] (the "fast Lua binding" setting).
pub(crate) fn install(lua: &Lua, fast: bool) -> mlua::Result<()> {
    lua.set_app_data(HostCache::new(lua)?);

    let mut methods = Vec::with_capacity(HostType::COUNT);
    for _ in 0..HostType::COUNT {
        methods.push(Vec::new());
    }

    for definition in api().declared() {
        let mut functions = Vec::with_capacity(definition.members.len());
        for member in &definition.members {
            functions.push(match &member.kind {
                MemberKind::Method(call) => {
                    let call = std::sync::Arc::clone(call);
                    Some(lua.create_function(move |lua, values: MultiValue| {
                        let cx = Cx::new(lua);
                        let reply = call(&Call::new(&cx, &values))?;
                        Ok(reply.into_iter().collect::<MultiValue>())
                    })?)
                }
                MemberKind::Property { .. } => None,
            });
        }

        methods[definition.id.index()] = functions;
    }

    lua.set_app_data(MethodCache(methods));
    if fast {
        install_raw(lua)?;
    }
    Ok(())
}

/// Builds the metatable of the raw host values from the safe binding's functions.
fn install_raw(lua: &Lua) -> mlua::Result<()> {
    super::lend::install(lua);
    let name = lua
        .create_userdata(Host::new(Payload::Table(HostType::ScriptApi)))?
        .type_name()?
        .unwrap_or_else(|| "Host".to_owned());
    let index_fallback = lua.create_function(|lua, (this, key): (Value, Value)| {
        let cx = Cx::new(lua);
        let payload =
            host_payload(&this).ok_or_else(|| cx.raise("attempt to index a userdata value"))?;
        index(lua, payload, &key)
    })?;
    let new_index_fallback =
        lua.create_function(|lua, (this, key, value): (Value, Value, Value)| {
            new_index(lua, &this, &key, &value)
        })?;
    let mut others = Vec::new();
    others.push((
        "__eq",
        lua.create_function(|_, (left, right): (Value, Value)| Ok(equal(&left, &right)))?,
    ));
    others.push((
        "__call",
        lua.create_function(|lua, values: MultiValue| call_host(lua, &values))?,
    ));
    others.push((
        "__tostring",
        lua.create_function(|lua, this: Value| text_of(lua, &this))?,
    ));
    others.push((
        "__add",
        lua.create_function(|lua, (left, right): (Value, Value)| {
            operate(lua, &left, &right, |definition| definition.add.as_ref())
        })?,
    ));
    others.push((
        "__sub",
        lua.create_function(|lua, (left, right): (Value, Value)| {
            operate(lua, &left, &right, |definition| {
                definition.subtract.as_ref()
            })
        })?,
    ));
    others.push((
        "__mul",
        lua.create_function(|lua, (left, right): (Value, Value)| {
            operate(lua, &left, &right, |definition| {
                definition.multiply.as_ref()
            })
        })?,
    ));
    others.push((
        "__div",
        lua.create_function(|lua, (left, right): (Value, Value)| {
            operate(lua, &left, &right, |definition| definition.divide.as_ref())
        })?,
    ));
    others.push((
        "__unm",
        lua.create_function(|lua, (operand, other): (Value, Value)| {
            operate(lua, &operand, &other, |definition| {
                definition.negate.as_ref()
            })
        })?,
    ));
    let methods = lua.create_table()?;
    if let Some(cache) = lua.app_data_ref::<MethodCache>() {
        for (type_index, functions) in cache.0.iter().enumerate() {
            for (position, function) in functions.iter().enumerate() {
                if let Some(function) = function {
                    methods.raw_set(super::raw::method_key(type_index, position), function)?;
                }
            }
        }
    }
    super::raw::install(
        lua,
        super::raw::Fallbacks {
            index: index_fallback,
            new_index: new_index_fallback,
            others,
            name,
            methods,
        },
    )
}

/// `obj(...)`: the constructor of the type, when it has one.
fn call_host(lua: &Lua, values: &MultiValue) -> mlua::Result<MultiValue> {
    let cx = Cx::new(lua);
    let call = Call::new(&cx, values);
    let payload = call
        .host(1)
        .ok_or_else(|| cx.raise("attempt to call a userdata value"))?;
    let constructor = definition_of(&cx, &payload)?
        .call
        .as_ref()
        .ok_or_else(|| cx.raise("attempt to call a userdata value"))?;
    Ok(constructor(&call)?.into_iter().collect::<MultiValue>())
}

/// `tostring(obj)`: the text of the type, or its name.
fn text_of(lua: &Lua, this: &Value) -> mlua::Result<String> {
    let cx = Cx::new(lua);
    let payload = host_payload(this).ok_or_else(|| cx.raise("this value has no description"))?;
    let definition = definition_of(&cx, &payload)?;
    definition.text.as_ref().map_or_else(
        || Ok(definition.name.to_owned()),
        |text| text(&cx, &payload),
    )
}

fn definition_of(cx: &Cx, payload: &Payload) -> mlua::Result<&'static TypeDef> {
    api()
        .get(payload.host_type())
        .ok_or_else(|| cx.raise("this value has no description"))
}

/// The text a key stands for in an error message.
fn key_text(key: &Value) -> String {
    match key {
        Value::String(text) => text.to_string_lossy(),
        Value::Integer(integer) => integer.to_string(),
        Value::Number(number) => number_format::to_text(*number),
        _ => "?".to_owned(),
    }
}

fn is_number(value: &Value) -> bool {
    matches!(value, Value::Number(_) | Value::Integer(_))
}

/// `obj[key]`: a member first, then the numeric indexer; an unknown member reads as nil, without
/// an error (REQ-LUA-042).
pub(crate) fn index(lua: &Lua, payload: Payload, key: &Value) -> mlua::Result<Value> {
    let cx = Cx::new(lua);
    let definition = definition_of(&cx, &payload)?;

    if let Value::String(name) = key {
        if let Some((position, member)) = definition.member(&name.as_bytes()) {
            return match &member.kind {
                MemberKind::Property { get, .. } => get(&cx, &payload),
                MemberKind::Method(_) => {
                    let methods = lua
                        .app_data_ref::<MethodCache>()
                        .ok_or_else(|| cx.raise("the method table is not installed"))?;
                    Ok(methods.0[definition.id.index()][position]
                        .clone()
                        .map_or(Value::Nil, Value::Function))
                }
            };
        }
    }

    if is_number(key) {
        if let Some(indexer) = &definition.indexer {
            return indexer(&cx, &payload, key);
        }
    }

    Ok(Value::Nil)
}

/// `obj[key] = value`: a writable member, a numeric index, or the exact errors of REQ-LUA-042
/// and TV-LUA-13.
fn new_index(lua: &Lua, this: &Value, key: &Value, value: &Value) -> mlua::Result<()> {
    let cx = Cx::new(lua);
    let payload =
        host_payload(this).ok_or_else(|| cx.raise("attempt to index a userdata value"))?;
    if let Some(assigned) = assign(lua, payload, key, value)? {
        set_host_payload(this, assigned);
    }
    Ok(())
}

/// `obj[key] = value` on the host value of `payload`. Returns the payload the value now holds
/// when it is a value type: the script's own copy, whose assigned component belongs to the
/// userdata it was assigned to (REQ-LUA-044).
pub(crate) fn assign(
    lua: &Lua,
    mut payload: Payload,
    key: &Value,
    value: &Value,
) -> mlua::Result<Option<Payload>> {
    let cx = Cx::new(lua);
    let definition = definition_of(&cx, &payload)?;

    if let Value::String(name) = key {
        if let Some((_, member)) = definition.member(&name.as_bytes()) {
            return match &member.kind {
                MemberKind::Property { set: Some(set), .. } => {
                    set(&cx, &mut payload, value)?;
                    Ok(definition.id.is_value().then_some(payload))
                }
                _ => Err(cx.raise(&format!(
                    "cannot set {}, it is read only",
                    name.to_string_lossy()
                ))),
            };
        }
    }

    if is_number(key) {
        if let Some(setter) = &definition.index_setter {
            return setter(&cx, &payload, key, value).map(|()| None);
        }
    }

    Err(cx.raise(&format!("cannot set {}, no such field", key_text(key))))
}

/// Rewrites the payload of the host value `this`.
fn set_host_payload(this: &Value, payload: Payload) {
    if let Value::UserData(data) = this {
        if let Ok(host) = data.borrow::<Host>() {
            host.set_payload(payload);
        } else {
            super::raw::set_payload(data, payload);
        }
    }
}

fn equal(left: &Value, right: &Value) -> bool {
    let (Some(left), Some(right)) = (host_payload(left), host_payload(right)) else {
        return false;
    };

    if left.host_type() != right.host_type() {
        return false;
    }

    api()
        .get(left.host_type())
        .and_then(|definition| definition.equal.as_ref())
        .map_or(left == right, |compare| compare(&left, &right))
}

/// Applies the operator `pick` selects on the first operand that has one, as Lua does.
fn operate(
    lua: &Lua,
    left: &Value,
    right: &Value,
    pick: impl Fn(&TypeDef) -> Option<&OperatorFn>,
) -> mlua::Result<Value> {
    let cx = Cx::new(lua);
    for operand in [left, right] {
        if let Some(payload) = host_payload(operand) {
            if let Some(operator) = api().get(payload.host_type()).and_then(&pick) {
                return operator(&cx, left, right);
            }
        }
    }

    Err(cx.raise("attempt to perform arithmetic on a userdata value"))
}

impl UserData for Host {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::Index, |lua, this, key: Value| {
            index(lua, this.payload(), &key)
        });
        methods.add_meta_method(
            MetaMethod::NewIndex,
            |lua, this, (key, value): (Value, Value)| {
                if let Some(assigned) = assign(lua, this.payload(), &key, &value)? {
                    this.set_payload(assigned);
                }
                Ok(())
            },
        );
        methods.add_meta_function(MetaMethod::Eq, |_, (left, right): (Value, Value)| {
            Ok(equal(&left, &right))
        });
        methods.add_meta_function(MetaMethod::Call, |lua, values: MultiValue| {
            call_host(lua, &values)
        });
        methods.add_meta_function(MetaMethod::ToString, |lua, this: AnyUserData| {
            text_of(lua, &Value::UserData(this))
        });
        methods.add_meta_function(MetaMethod::Add, |lua, (left, right): (Value, Value)| {
            operate(lua, &left, &right, |definition| definition.add.as_ref())
        });
        methods.add_meta_function(MetaMethod::Sub, |lua, (left, right): (Value, Value)| {
            operate(lua, &left, &right, |definition| {
                definition.subtract.as_ref()
            })
        });
        methods.add_meta_function(MetaMethod::Mul, |lua, (left, right): (Value, Value)| {
            operate(lua, &left, &right, |definition| {
                definition.multiply.as_ref()
            })
        });
        methods.add_meta_function(MetaMethod::Div, |lua, (left, right): (Value, Value)| {
            operate(lua, &left, &right, |definition| definition.divide.as_ref())
        });
        methods.add_meta_function(MetaMethod::Unm, |lua, (operand, other): (Value, Value)| {
            operate(lua, &operand, &other, |definition| {
                definition.negate.as_ref()
            })
        });
    }
}
