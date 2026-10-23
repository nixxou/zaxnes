//! The context of a call from Lua into the host: access to the runtime and the engine state
//! the call was made under, argument checking (REQ-LUA-041) and error raising.

use std::any::Any;
use std::cell::RefCell;

use mlua::{AnyUserData, Function, Lua, MultiValue, Table, Value};
use zax_engine3d::engine::ConversionState;
use zax_engine3d::instance::{InstanceRef, PatternInstance};
use zax_engine3d::vectors::{Cell, Rect, Vec2, Vec3};

use super::payload::{Host, Payload, Vec3Ref};
use crate::environment::ScriptContext;
use crate::runtime::{FrameHandles, RuntimeData};
use crate::values::{number_format, LuaColor};

/// What the host lends to the Lua state for the duration of one call into it: the runtime's
/// data, the engine state when there is one, and the pattern instance being built.
pub(crate) struct Lent {
    pub data: AnyUserData,
    pub state: Option<AnyUserData>,
    pub build: Option<(InstanceRef, AnyUserData)>,
}

/// What the host answers to a request. The common answers are held in place; only an answer of
/// another type is boxed.
pub(crate) enum Answer {
    Unit,
    Real(f32),
    Int(i32),
    Flag(bool),
    Vector(Vec3),
    Boxed(Box<dyn Any>),
}

/// A type the host can answer a request with.
pub(crate) trait Reply: Sized + 'static {
    fn pack(self) -> Answer;
    fn unpack(answer: Answer) -> Option<Self>;
}

macro_rules! reply {
    ($($type:ty => $variant:ident),* $(,)?) => {$(
        impl Reply for $type {
            fn pack(self) -> Answer {
                Answer::$variant(self)
            }

            fn unpack(answer: Answer) -> Option<Self> {
                match answer {
                    Answer::$variant(value) => Some(value),
                    _ => None,
                }
            }
        }
    )*};
}

macro_rules! boxed_reply {
    ($($type:ty),* $(,)?) => {$(
        impl Reply for $type {
            fn pack(self) -> Answer {
                Answer::Boxed(Box::new(self))
            }

            fn unpack(answer: Answer) -> Option<Self> {
                match answer {
                    Answer::Boxed(value) => value.downcast::<Self>().ok().map(|value| *value),
                    _ => None,
                }
            }
        }
    )*};
}

reply!(f32 => Real, i32 => Int, bool => Flag, Vec3 => Vector);
boxed_reply!(u8, LuaColor);

impl Reply for () {
    fn pack(self) -> Answer {
        Answer::Unit
    }

    fn unpack(answer: Answer) -> Option<Self> {
        matches!(answer, Answer::Unit).then_some(())
    }
}

/// A request for the host context: run against the lent [`ScriptContext`], and answer.
type HostRequest = Box<dyn FnOnce(&mut ScriptContext<'_>) -> Answer>;

/// A change of the host the script asked for, which nothing in the script waits for.
type HostWrite = Box<dyn FnOnce(&mut ScriptContext<'_>)>;

/// How many writes wait for the next request before they are made anyway.
const QUEUED_WRITES_LIMIT: usize = 256;

/// The way to the [`ScriptContext`] a host lent for the length of a call. The context is
/// borrowed, so it cannot be stored in the Lua state: what is stored is a function, valid for the
/// scope of the lend, which runs the pending request against the context.
///
/// A write does not wait for the host: it is queued, and the host makes it before the next
/// request, so that every read sees the writes before it, and at the end of the call into Lua.
/// A write costs a queue entry where a request costs a call into Lua.
pub(crate) struct HostLink {
    call: Function,
    pending: RefCell<Option<HostRequest>>,
    answer: RefCell<Option<Answer>>,
    writes: RefCell<Vec<HostWrite>>,
}

impl HostLink {
    pub const fn new(call: Function) -> Self {
        Self {
            call,
            pending: RefCell::new(None),
            answer: RefCell::new(None),
            writes: RefCell::new(Vec::new()),
        }
    }

    /// The body of the function behind the link.
    pub fn serve(lua: &Lua, context: &mut ScriptContext<'_>) -> mlua::Result<()> {
        let link = lua
            .app_data_ref::<Self>()
            .ok_or_else(|| mlua::Error::runtime("the host context was withdrawn"))?;
        let mut writes = link.writes.take();
        for write in writes.drain(..) {
            write(context);
        }

        link.writes.replace(writes);
        if let Some(request) = link.pending.take() {
            link.answer.replace(Some(request(context)));
        }

        Ok(())
    }

    /// Queues `write` for the next request, or makes it now when too many wait.
    fn queue(&self, write: HostWrite) -> mlua::Result<()> {
        let waiting = {
            let mut writes = self.writes.borrow_mut();
            writes.push(write);
            writes.len()
        };
        if waiting >= QUEUED_WRITES_LIMIT {
            self.flush()?;
        }

        Ok(())
    }

    /// Makes the queued writes.
    pub fn flush(&self) -> mlua::Result<()> {
        if self.writes.borrow().is_empty() {
            return Ok(());
        }

        self.call.call::<()>(())
    }

    /// Runs `operation` on the lent context.
    fn run<R: Reply>(
        &self,
        operation: impl FnOnce(&mut ScriptContext<'_>) -> R + 'static,
    ) -> mlua::Result<R> {
        self.pending
            .replace(Some(Box::new(move |context| operation(context).pack())));
        self.call.call::<()>(())?;
        self.answer
            .take()
            .ok_or_else(|| mlua::Error::runtime("the host context gave no answer"))
            .and_then(|answer| {
                R::unpack(answer)
                    .ok_or_else(|| mlua::Error::runtime("the host context gave another answer"))
            })
    }
}

/// Runs `operation` on the host context lent to `lua`, or on the headless bridges of the
/// runtime's environment when the host lent none.
pub(crate) fn with_host_context<R: Reply>(
    lua: &Lua,
    data: &mut RuntimeData,
    operation: impl FnOnce(&mut ScriptContext<'_>) -> R + 'static,
) -> mlua::Result<R> {
    match lua.app_data_ref::<HostLink>() {
        Some(link) => link.run(operation),
        None => Ok(data.env.with_headless_context(operation)),
    }
}

/// The weak table that maps a reference object to its Lua value.
pub(crate) struct HostCache(pub Table);

/// The context of one call from Lua into the host.
pub(crate) struct Cx<'a> {
    pub lua: &'a Lua,
}

impl<'a> Cx<'a> {
    pub const fn new(lua: &'a Lua) -> Self {
        Self { lua }
    }

    fn not_available(what: &str) -> mlua::Error {
        mlua::Error::runtime(format!("{what} is not available here"))
    }

    fn lent(&self) -> mlua::Result<mlua::AppDataRef<'_, Lent>> {
        self.lua
            .app_data_ref::<Lent>()
            .ok_or_else(|| Self::not_available("the script API"))
    }

    /// Runs `operation` on the runtime's data. The data is lent to the Lua state for the
    /// length of a call into it, so it can only be reached through a closure.
    pub fn data<R>(&self, operation: impl FnOnce(&mut RuntimeData) -> R) -> mlua::Result<R> {
        self.lent()?
            .data
            .borrow_mut_scoped::<RuntimeData, R>(operation)
    }

    /// Runs `operation` on the camera, backdrop, inputs and memory the host lent to this call.
    /// The operation outlives the borrow of the host, so it owns what it captures.
    pub fn host<R: Reply>(
        &self,
        operation: impl FnOnce(&mut ScriptContext<'_>) -> R + 'static,
    ) -> mlua::Result<R> {
        if let Some(link) = self.lua.app_data_ref::<HostLink>() {
            return link.run(operation);
        }

        self.data(|data| with_host_context(self.lua, data, operation))?
    }

    /// Makes `operation`, a change of the camera, the backdrop or the machine that has no answer,
    /// before the next request to the host, and at the latest when the call into Lua ends.
    pub fn host_write(
        &self,
        operation: impl FnOnce(&mut ScriptContext<'_>) + 'static,
    ) -> mlua::Result<()> {
        if let Some(link) = self.lua.app_data_ref::<HostLink>() {
            return link.queue(Box::new(operation));
        }

        self.data(|data| with_host_context(self.lua, data, operation))?
    }

    /// Runs `operation` on the engine state, which only a hook with access to it has.
    pub fn state<R>(&self, operation: impl FnOnce(&mut ConversionState) -> R) -> mlua::Result<R> {
        let lent = self.lent()?;
        lent.state
            .as_ref()
            .ok_or_else(|| Self::not_available("the conversion state"))?
            .borrow_mut_scoped::<ConversionState, R>(operation)
    }

    /// Runs `operation`, which may fail with a Lua error of its own, on the engine state.
    pub fn try_state<R>(
        &self,
        operation: impl FnOnce(&mut ConversionState) -> mlua::Result<R>,
    ) -> mlua::Result<R> {
        self.state(operation)?
    }

    /// True when the call was made with the engine state at hand.
    pub fn has_state(&self) -> bool {
        self.lent().is_ok_and(|lent| lent.state.is_some())
    }

    /// Runs `operation` on the pattern instance being built, when `reference` is it.
    pub fn build_target<R>(
        &self,
        reference: InstanceRef,
        operation: impl FnOnce(&mut PatternInstance) -> R,
    ) -> mlua::Result<Option<R>> {
        let lent = self.lent()?;
        match &lent.build {
            Some((building, target)) if *building == reference => Ok(Some(
                target.borrow_mut_scoped::<PatternInstance, R>(operation)?,
            )),
            _ => Ok(None),
        }
    }

    // errors

    /// A Lua error carrying the position of the calling script line, the form REQ-LUA-061 logs.
    pub fn raise(&self, message: &str) -> mlua::Error {
        let position = self
            .lua
            .inspect_stack(1, |frame| {
                let source = frame.source();
                match (source.short_src, frame.current_line()) {
                    (Some(name), Some(line)) => format!("{name}:{line}: "),
                    _ => String::new(),
                }
            })
            .unwrap_or_default();
        mlua::Error::runtime(format!("{position}{message}"))
    }

    /// `invalid arguments to <member>` (REQ-LUA-041).
    pub fn invalid(&self, member: &str) -> mlua::Error {
        self.raise(&format!("invalid arguments to {member}"))
    }

    /// Checks a 0-based index against a count, raising a Lua error (REQ-LUA-045).
    pub fn check_bounds(&self, index: i32, count: usize, member: &str) -> mlua::Result<usize> {
        match usize::try_from(index) {
            Ok(position) if position < count => Ok(position),
            _ => Err(self.raise(&format!(
                "index {index} is out of range for {member} (count {count})"
            ))),
        }
    }

    // pushing

    /// Hands the handles of an epoch that is over to the weak object cache: a script that still
    /// holds one keeps the one value of its object (REQ-LUA-043), whatever the epoch.
    pub fn retire(&self, handles: FrameHandles) -> mlua::Result<()> {
        if handles.is_empty() {
            return Ok(());
        }

        let cache = self.cache()?;
        for (key, handle) in handles {
            cache.raw_set(self.lua.create_string(key)?, handle)?;
        }
        Ok(())
    }

    fn cache(&self) -> mlua::Result<Table> {
        self.lua
            .app_data_ref::<HostCache>()
            .map(|cache| cache.0.clone())
            .ok_or_else(|| Self::not_available("the object cache"))
    }

    /// A fresh Lua value for `payload`.
    pub fn value(&self, payload: Payload) -> mlua::Result<Value> {
        Ok(Value::UserData(
            self.lua.create_userdata(Host::new(payload))?,
        ))
    }

    /// The Lua value of a reference object: the same object always gives the same value
    /// (REQ-LUA-043), which is what lets a script use it as a table key (REQ-LUA-045).
    pub fn object(&self, payload: Payload) -> mlua::Result<Value> {
        let Some(key) = payload.cache_key() else {
            return self.value(payload);
        };

        // A handle into the current epoch of a frame is made once: the runtime keeps its value
        // until the epoch is over, which spares the weak table a string key and two accesses.
        if let Some(frame) = payload.frame_key() {
            let known = self.data(|data| {
                data.handles_of(frame)
                    .map(|handles| handles.get(&key).cloned())
            })?;
            match known {
                Some(Some(handle)) => return Ok(handle),
                Some(None) => {
                    let created = self.value(payload)?;
                    self.data(|data| {
                        if let Some(handles) = data.handles_of(frame) {
                            handles.insert(key, created.clone());
                        }
                    })?;
                    return Ok(created);
                }
                None => {}
            }
        }

        let cache = self.cache()?;
        let key = self.lua.create_string(key)?;
        if let Value::UserData(existing) = cache.raw_get::<Value>(&key)? {
            return Ok(Value::UserData(existing));
        }

        let created = self.value(payload)?;
        cache.raw_set(&key, &created)?;
        Ok(created)
    }

    /// A Lua string.
    pub fn text(&self, text: &str) -> mlua::Result<Value> {
        Ok(Value::String(self.lua.create_string(text)?))
    }

    /// A `Vector3` value.
    pub fn vector3(&self, value: Vec3) -> mlua::Result<Value> {
        self.value(Payload::Vector3(value))
    }

    /// A `Vector2` value.
    pub fn vector2(&self, value: Vec2) -> mlua::Result<Value> {
        self.value(Payload::Vector2(value))
    }

    /// A `Color` value.
    pub fn color(&self, value: LuaColor) -> mlua::Result<Value> {
        self.value(Payload::Color(value))
    }

    /// An `IntVector2` value.
    pub fn int_vector2(&self, value: Cell) -> mlua::Result<Value> {
        self.value(Payload::IntVector2(value))
    }

    // reading

    /// The number at hand. A string, even a numeric one, is refused (REQ-LUA-041).
    #[allow(clippy::cast_precision_loss)] // lua_tonumber converts an integer to a double.
    pub fn number(&self, value: &Value, member: &str) -> mlua::Result<f64> {
        match value {
            Value::Number(number) => Ok(*number),
            Value::Integer(integer) => Ok(*integer as f64),
            _ => Err(self.invalid(member)),
        }
    }

    /// The number at hand, as a single-precision real.
    #[allow(clippy::cast_possible_truncation)] // The C# casts the double to float.
    pub fn real(&self, value: &Value, member: &str) -> mlua::Result<f32> {
        Ok(self.number(value, member)? as f32)
    }

    /// The number at hand, truncated towards zero (REQ-LUA-040).
    pub fn integer(&self, value: &Value, member: &str) -> mlua::Result<i32> {
        Ok(number_format::to_i32(self.number(value, member)?))
    }

    /// The string at hand; a number is refused (REQ-LUA-041).
    pub fn string(&self, value: &Value, member: &str) -> mlua::Result<String> {
        match value {
            Value::String(text) => Ok(text.to_string_lossy()),
            _ => Err(self.invalid(member)),
        }
    }

    /// The value at hand read as a `Vector3`. A bound vector is accepted wherever a `Vector3`
    /// is expected, because the corpus passes one straight through (`shape:GetPos(shape.Pivot)`);
    /// it is read by copy, so the callee cannot write back through it.
    pub fn try_vec3(&self, value: &Value) -> mlua::Result<Option<Vec3>> {
        match host_payload(value) {
            Some(Payload::Vector3(vector)) => Ok(Some(vector)),
            Some(Payload::Vec3Ref(reference)) => self.read_vec3_ref(&reference).map(Some),
            Some(Payload::Vector2(vector)) => Ok(Some(Vec3::new(vector.x, vector.y, 0.0))),
            _ => Ok(None),
        }
    }

    /// The value at hand as a `Vector3`, or an error (REQ-LUA-041).
    pub fn vec3(&self, value: &Value, member: &str) -> mlua::Result<Vec3> {
        self.try_vec3(value)?.ok_or_else(|| self.invalid(member))
    }

    /// The value at hand read as a `Vector2`.
    #[allow(clippy::cast_precision_loss)] // The C# widens an int to a float.
    pub fn try_vec2(&self, value: &Value) -> mlua::Result<Option<Vec2>> {
        match host_payload(value) {
            Some(Payload::Vector2(vector)) => Ok(Some(vector)),
            Some(Payload::IntVector2(cell)) => Ok(Some(Vec2::new(cell.x as f32, cell.y as f32))),
            Some(Payload::Vector3(_) | Payload::Vec3Ref(_)) => Ok(self
                .try_vec3(value)?
                .map(|three| Vec2::new(three.x, three.y))),
            _ => Ok(None),
        }
    }

    /// The value at hand read as an `IntVector2`; a real vector is truncated.
    #[allow(clippy::cast_possible_truncation)] // The C# casts the float components to int.
    pub fn try_int_vector2(&self, value: &Value) -> mlua::Result<Option<Cell>> {
        match host_payload(value) {
            Some(Payload::IntVector2(cell)) => Ok(Some(cell)),
            Some(Payload::Vector2(_) | Payload::Vector3(_) | Payload::Vec3Ref(_)) => Ok(self
                .try_vec2(value)?
                .map(|pair| Cell::new(pair.x as i32, pair.y as i32))),
            _ => Ok(None),
        }
    }

    /// The value at hand as a `Rect`, or an error.
    pub fn rect(&self, value: &Value, member: &str) -> mlua::Result<Rect> {
        try_rect(value).ok_or_else(|| self.invalid(member))
    }

    /// The vector a `Vec3R` designates, or zero when its owner is gone (REQ-LUA-028).
    pub fn read_vec3_ref(&self, reference: &Vec3Ref) -> mlua::Result<Vec3> {
        super::access::read_vec3_ref(self, reference)
    }
}

/// The payload of the host value `value`, or `None` for any other Lua value.
pub(crate) fn host_payload(value: &Value) -> Option<Payload> {
    match value {
        Value::UserData(data) => data.borrow::<Host>().ok().map(|host| host.payload()),
        _ => None,
    }
}

/// The value at hand read as a `Quaternion`, as `[x, y, z, w]`.
pub(crate) fn try_quaternion(value: &Value) -> Option<[f32; 4]> {
    match host_payload(value) {
        Some(Payload::Quaternion(parts)) => Some(parts),
        _ => None,
    }
}

/// The value at hand read as a `Color`.
pub(crate) fn try_color(value: &Value) -> Option<LuaColor> {
    match host_payload(value) {
        Some(Payload::Color(color)) => Some(color),
        _ => None,
    }
}

/// The value at hand read as a `Rect`.
pub(crate) fn try_rect(value: &Value) -> Option<Rect> {
    match host_payload(value) {
        Some(Payload::Rect(rectangle)) => Some(rectangle),
        _ => None,
    }
}

/// The arguments of a method or a table function, numbered from 1 as the stack of a Lua call
/// is; argument 1 is the receiver.
pub(crate) struct Call<'a> {
    pub cx: &'a Cx<'a>,
    values: &'a MultiValue,
    nil: Value,
}

impl<'a> Call<'a> {
    pub const fn new(cx: &'a Cx<'a>, values: &'a MultiValue) -> Self {
        Self {
            cx,
            values,
            nil: Value::Nil,
        }
    }

    /// The argument `index` (1-based), nil when the script passed fewer.
    pub fn value(&self, index: usize) -> &Value {
        index
            .checked_sub(1)
            .and_then(|position| self.values.get(position))
            .unwrap_or(&self.nil)
    }

    /// True when the argument is absent or nil.
    pub fn is_none_or_nil(&self, index: usize) -> bool {
        matches!(self.value(index), Value::Nil)
    }

    /// True when the argument is a number.
    pub fn is_number(&self, index: usize) -> bool {
        matches!(self.value(index), Value::Number(_) | Value::Integer(_))
    }

    /// The payload of the host value at `index`.
    pub fn host(&self, index: usize) -> Option<Payload> {
        host_payload(self.value(index))
    }

    /// The number at `index` (REQ-LUA-041).
    pub fn number(&self, index: usize, member: &str) -> mlua::Result<f64> {
        self.cx.number(self.value(index), member)
    }

    /// The number at `index` as a single-precision real.
    pub fn real(&self, index: usize, member: &str) -> mlua::Result<f32> {
        self.cx.real(self.value(index), member)
    }

    /// The number at `index`, truncated towards zero.
    pub fn integer(&self, index: usize, member: &str) -> mlua::Result<i32> {
        self.cx.integer(self.value(index), member)
    }

    /// The number at `index`, or `fallback` if the argument is absent or nil.
    pub fn optional_number(&self, index: usize, member: &str, fallback: f64) -> mlua::Result<f64> {
        if self.is_none_or_nil(index) {
            Ok(fallback)
        } else {
            self.number(index, member)
        }
    }

    /// The string at `index`; a number is refused.
    pub fn string(&self, index: usize, member: &str) -> mlua::Result<String> {
        self.cx.string(self.value(index), member)
    }

    /// The string at `index`, or `fallback` if the argument is absent or nil.
    pub fn optional_string(
        &self,
        index: usize,
        member: &str,
        fallback: &str,
    ) -> mlua::Result<String> {
        if self.is_none_or_nil(index) {
            Ok(fallback.to_owned())
        } else {
            self.string(index, member)
        }
    }

    /// The truth value at `index`.
    pub fn boolean(&self, index: usize) -> bool {
        truthy(self.value(index))
    }

    /// The `Vector3` at `index`, or an error.
    pub fn vec3(&self, index: usize, member: &str) -> mlua::Result<Vec3> {
        self.cx.vec3(self.value(index), member)
    }

    /// The `Rect` at `index`, or an error.
    pub fn rect(&self, index: usize, member: &str) -> mlua::Result<Rect> {
        self.cx.rect(self.value(index), member)
    }

    /// `invalid arguments to <member>`.
    pub fn invalid(&self, member: &str) -> mlua::Error {
        self.cx.invalid(member)
    }
}

/// Lua truthiness: everything but nil and false.
pub(crate) const fn truthy(value: &Value) -> bool {
    !matches!(value, Value::Nil | Value::Boolean(false))
}
