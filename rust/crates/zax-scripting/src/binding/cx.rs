//! The context of a call from Lua into the host: access to the runtime and the engine state
//! the call was made under, argument checking (REQ-LUA-041) and error raising.

use std::any::Any;
use std::cell::RefCell;

use mlua::{AnyUserData, Function, Lua, MultiValue, Table, Value};
use zax_engine3d::engine::ConversionState;
use zax_engine3d::instance::{InstanceRef, PatternInstance};
use zax_engine3d::vectors::{Cell, Rect, Vec2, Vec3};

use super::lend::{self, LendError, LendSlot};
use super::payload::{FrameKey, FrameSlot, Host, Payload, Vec3Ref};
use crate::environment::ScriptContext;
use crate::runtime::{Retired, RuntimeData};
use crate::values::{number_format, LuaColor};

/// What the host lends to the Lua state for the duration of one call into it: the runtime's
/// data, the engine state when there is one, and the pattern instance being built.
pub(crate) struct Lent {
    pub data: AnyUserData,
    pub state: Option<AnyUserData>,
    pub build: Option<(InstanceRef, AnyUserData)>,
}

/// A request for the host context: run against the lent [`ScriptContext`], and answer with a
/// boxed value.
type HostRequest = Box<dyn FnOnce(&mut ScriptContext<'_>) -> Box<dyn Any + Send> + Send>;

/// A change of the host the script asked for, which nothing in the script waits for.
type HostWrite = Box<dyn FnOnce(&mut ScriptContext<'_>) + Send>;

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
    answer: RefCell<Option<Box<dyn Any + Send>>>,
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
    fn run<R: Send + 'static>(
        &self,
        operation: impl FnOnce(&mut ScriptContext<'_>) -> R + Send + 'static,
    ) -> mlua::Result<R> {
        self.pending
            .replace(Some(Box::new(move |context| Box::new(operation(context)))));
        self.call.call::<()>(())?;
        self.answer
            .take()
            .ok_or_else(|| mlua::Error::runtime("the host context gave no answer"))?
            .downcast::<R>()
            .map(|answer| *answer)
            .map_err(|_| mlua::Error::runtime("the host context gave another answer"))
    }
}

/// Runs `operation` on the host context lent to `lua`, or on the headless bridges of the
/// runtime's environment when the host lent none.
pub(crate) fn with_host_context<R: Send + 'static>(
    lua: &Lua,
    data: &mut RuntimeData,
    operation: impl FnOnce(&mut ScriptContext<'_>) -> R + Send + 'static,
) -> mlua::Result<R> {
    match lua.app_data_ref::<HostLink>() {
        Some(link) => link.run(operation),
        None => Ok(data.env.with_headless_context(operation)),
    }
}

/// How many retirements of an epoch pass between two sweeps of the tables of retired epochs.
const SWEEP_INTERVAL: u32 = 128;

/// What maps a reference object to its Lua value, so that the same object always has the same
/// value for as long as a script holds it (REQ-LUA-043).
pub(crate) struct HostCache {
    /// The weak table of the objects that are not handles into a frame, by cache key.
    pub objects: Table,
    /// The tables of the handles of epochs that are over, by group of [`FrameSlot`]. Each is
    /// weak and keyed by the member, so that retiring a handle makes no string.
    retired: Table,
    /// The metatable that makes a table of `retired` weak.
    weak: Table,
    retirements: std::cell::Cell<u32>,
    epochs: std::cell::Cell<Epochs>,
}

/// The epochs of the frame being converted and of the pinned frame: a frame handle reads as gone
/// once its epoch is over. The runtime's data holds them too; the cache has them at hand
/// without a borrow of the data.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Epochs {
    pub frame: u32,
    pub pinned: u32,
    /// The counter of the frame whose epoch ended last, which a handle of that epoch still
    /// reads while it is the published frame (REQ-LUA-028).
    pub previous_counter: Option<i32>,
}

impl Epochs {
    pub const fn of(self, pinned: bool) -> u32 {
        if pinned {
            self.pinned
        } else {
            self.frame
        }
    }
}

impl HostCache {
    pub fn new(lua: &Lua) -> mlua::Result<Self> {
        let weak = lua.create_table()?;
        weak.raw_set("__mode", "v")?;
        let objects = lua.create_table()?;
        objects.set_metatable(Some(weak.clone()))?;
        Ok(Self {
            objects,
            retired: lua.create_table()?,
            weak,
            retirements: std::cell::Cell::new(0),
            epochs: std::cell::Cell::new(Epochs::default()),
        })
    }

    /// The table of handles of `group`, made when `create` is set and it is not there.
    fn group(&self, lua: &Lua, group: u64, create: bool) -> mlua::Result<Option<Table>> {
        if let Value::Table(table) = self.retired.raw_get::<Value>(lua_key(group))? {
            return Ok(Some(table));
        }

        if !create {
            return Ok(None);
        }

        let table = lua.create_table()?;
        table.set_metatable(Some(self.weak.clone()))?;
        self.retired.raw_set(lua_key(group), &table)?;
        Ok(Some(table))
    }

    /// Drops the tables of the groups none of whose handles is held any more.
    fn sweep(&self) -> mlua::Result<()> {
        let mut empty = Vec::new();
        for entry in self.retired.pairs::<i64, Table>() {
            let (key, table) = entry?;
            if table.is_empty() {
                empty.push(key);
            }
        }

        for key in empty {
            self.retired.raw_set(key, Value::Nil)?;
        }

        Ok(())
    }
}

/// The Lua integer that stands for `key`, whatever its top bit.
fn lua_key(key: u64) -> i64 {
    i64::from_ne_bytes(key.to_ne_bytes())
}

/// The context of one call from Lua into the host.
pub(crate) struct Cx<'a> {
    pub lua: &'a Lua,
    /// The deferral of the host value a fast getter returns (`raw::Deferred`).
    defer: Option<&'a std::cell::Cell<super::raw::Deferred>>,
}

impl<'a> Cx<'a> {
    pub const fn new(lua: &'a Lua) -> Self {
        Self { lua, defer: None }
    }

    /// A context whose first host value is deferred into `cell` (`raw::Deferred`).
    pub const fn deferring(lua: &'a Lua, cell: &'a std::cell::Cell<super::raw::Deferred>) -> Self {
        Self {
            lua,
            defer: Some(cell),
        }
    }

    fn not_available(what: &str) -> mlua::Error {
        mlua::Error::runtime(format!("{what} is not available here"))
    }

    /// The pointer lend of the optimised Lua, when one is in force for this call.
    fn pointer_lend(&self) -> Option<&LendSlot> {
        if !super::raw::any_state() {
            return None;
        }
        lend::slot(self.lua).filter(|slot| slot.is_lent())
    }

    /// The error of a pointer-lent object that cannot be reached: `what` names it.
    fn lend_error(error: LendError, what: &str) -> mlua::Error {
        match error {
            LendError::Absent => Self::not_available(what),
            LendError::Busy => mlua::Error::UserDataBorrowMutError,
        }
    }

    fn lent(&self) -> mlua::Result<mlua::AppDataRef<'_, Lent>> {
        self.lua
            .app_data_ref::<Lent>()
            .ok_or_else(|| Self::not_available("the script API"))
    }

    /// Runs `operation` on the runtime's data. The data is lent to the Lua state for the
    /// length of a call into it, so it can only be reached through a closure.
    pub fn data<R>(&self, operation: impl FnOnce(&mut RuntimeData) -> R) -> mlua::Result<R> {
        if let Some(slot) = self.pointer_lend() {
            return lend::with_data(slot, operation)
                .map_err(|error| Self::lend_error(error, "the runtime data"));
        }
        self.lent()?
            .data
            .borrow_mut_scoped::<RuntimeData, R>(operation)
    }

    /// Runs `operation` on the camera, backdrop, inputs and memory the host lent to this call.
    /// The operation outlives the borrow of the host, so it owns what it captures.
    pub fn host<R: Send + 'static>(
        &self,
        operation: impl FnOnce(&mut ScriptContext<'_>) -> R + Send + 'static,
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
        operation: impl FnOnce(&mut ScriptContext<'_>) + Send + 'static,
    ) -> mlua::Result<()> {
        if let Some(link) = self.lua.app_data_ref::<HostLink>() {
            return link.queue(Box::new(operation));
        }

        self.data(|data| with_host_context(self.lua, data, operation))?
    }

    /// Runs `operation` on the engine state, which only a hook with access to it has.
    pub fn state<R>(&self, operation: impl FnOnce(&mut ConversionState) -> R) -> mlua::Result<R> {
        if let Some(slot) = self.pointer_lend() {
            return lend::with_state(slot, operation)
                .map_err(|error| Self::lend_error(error, "the conversion state"));
        }
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
        if let Some(slot) = self.pointer_lend() {
            return lend::has_state(slot);
        }
        self.lent().is_ok_and(|lent| lent.state.is_some())
    }

    /// Runs `operation` on the pattern instance being built, when `reference` is it.
    pub fn build_target<R>(
        &self,
        reference: InstanceRef,
        operation: impl FnOnce(&mut PatternInstance) -> R,
    ) -> mlua::Result<Option<R>> {
        if let Some(slot) = self.pointer_lend() {
            return lend::with_build(slot, reference, operation)
                .map_err(|error| Self::lend_error(error, "the pattern instance"));
        }
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
    pub fn retire(&self, retired: Retired) -> mlua::Result<()> {
        let cache = self.cache()?;
        cache.epochs.set(retired.epochs);
        let mut handles = retired.handles;
        if handles.is_empty() {
            return Ok(());
        }

        let mut groups: Vec<(u64, Table)> = Vec::new();
        for (slot, handle) in handles.drain() {
            let position =
                if let Some(position) = groups.iter().position(|(group, _)| *group == slot.group) {
                    position
                } else {
                    let table = cache
                        .group(self.lua, slot.group, true)?
                        .ok_or_else(|| Self::not_available("the object cache"))?;
                    groups.push((slot.group, table));
                    groups.len() - 1
                };
            groups[position].1.raw_set(lua_key(slot.member), handle)?;
        }

        let retirements = cache.retirements.get().wrapping_add(1);
        cache.retirements.set(retirements);
        if retirements % SWEEP_INTERVAL == 0 {
            cache.sweep()?;
        }

        Ok(())
    }

    /// The epoch the frame being converted, or the pinned frame, is in now.
    pub fn epoch_of(&self, pinned: bool) -> mlua::Result<u32> {
        Ok(self.cache()?.epochs.get().of(pinned))
    }

    /// The epochs in force now.
    pub fn epochs(&self) -> mlua::Result<Epochs> {
        Ok(self.cache()?.epochs.get())
    }

    fn cache(&self) -> mlua::Result<mlua::AppDataRef<'_, HostCache>> {
        self.lua
            .app_data_ref::<HostCache>()
            .ok_or_else(|| Self::not_available("the object cache"))
    }

    /// A fresh Lua value for `payload`.
    pub fn value(&self, payload: Payload) -> mlua::Result<Value> {
        if let Some(cell) = self.defer {
            if let Some(mark) = super::raw::Deferred::take(cell, payload) {
                return Ok(mark);
            }
        }
        if let Some(metatable) = super::raw::metatable_key(self.lua) {
            return super::raw::value(self.lua, metatable, payload);
        }
        Ok(Value::UserData(
            self.lua.create_userdata(Host::new(payload))?,
        ))
    }

    /// A Lua value for `payload` that the host keeps (an object cache): never deferred, since
    /// the deferral mark must not outlive the getter that made it.
    fn kept_value(&self, payload: Payload) -> mlua::Result<Value> {
        Cx::new(self.lua).value(payload)
    }

    /// The Lua value of a reference object: the same object always gives the same value
    /// (REQ-LUA-043), which is what lets a script use it as a table key (REQ-LUA-045).
    pub fn object(&self, payload: Payload) -> mlua::Result<Value> {
        if let (Some(frame), Some(slot)) = (payload.frame_key(), payload.frame_slot()) {
            return self.frame_object(payload, frame, slot);
        }

        let Some(key) = payload.cache_key() else {
            return self.value(payload);
        };

        let cache = self.cache()?;
        let key = self.lua.create_string(key)?;
        if let Value::UserData(existing) = cache.objects.raw_get::<Value>(&key)? {
            return Ok(Value::UserData(existing));
        }

        let created = self.kept_value(payload)?;
        cache.objects.raw_set(&key, &created)?;
        Ok(created)
    }

    /// The Lua value of a handle into a frame. A handle into the current epoch of a frame is made
    /// once: the runtime keeps its value until the epoch is over. One of an epoch that is over
    /// is looked for among the retired handles, which a script may still hold.
    fn frame_object(
        &self,
        payload: Payload,
        frame: FrameKey,
        slot: FrameSlot,
    ) -> mlua::Result<Value> {
        let known = self.data(|data| {
            data.handles_of(frame)
                .map(|handles| handles.get(&slot).cloned())
        })?;
        match known {
            Some(Some(handle)) => return Ok(handle),
            Some(None) => {
                let created = self.kept_value(payload)?;
                self.data(|data| {
                    if let Some(handles) = data.handles_of(frame) {
                        handles.insert(slot, created.clone());
                    }
                })?;
                return Ok(created);
            }
            None => {}
        }

        let cache = self.cache()?;
        let handles = cache
            .group(self.lua, slot.group, true)?
            .ok_or_else(|| Self::not_available("the object cache"))?;
        if let Value::UserData(existing) = handles.raw_get::<Value>(lua_key(slot.member))? {
            return Ok(Value::UserData(existing));
        }

        let created = self.kept_value(payload)?;
        handles.raw_set(lua_key(slot.member), &created)?;
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
        Value::UserData(data) => match data.borrow::<Host>() {
            Ok(host) => Some(host.payload()),
            Err(_) => super::raw::payload_of(data),
        },
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
    /// The receiver of a call of the raw binding, read from its block; `values` then holds the
    /// arguments from 2 on.
    receiver: Option<Receiver<'a>>,
    nil: Value,
}

/// The receiver of a raw method call: its payload at hand, its Lua value made only when a method
/// asks for it.
pub(crate) struct Receiver<'a> {
    payload: Payload,
    value: std::cell::OnceCell<Value>,
    make: &'a dyn Fn() -> Value,
}

impl<'a> Receiver<'a> {
    pub fn new(payload: Payload, make: &'a dyn Fn() -> Value) -> Self {
        Self {
            payload,
            value: std::cell::OnceCell::new(),
            make,
        }
    }
}

impl<'a> Call<'a> {
    pub const fn new(cx: &'a Cx<'a>, values: &'a MultiValue) -> Self {
        Self {
            cx,
            values,
            receiver: None,
            nil: Value::Nil,
        }
    }

    /// A call of the raw binding: `receiver` is argument 1, `rest` the arguments from 2 on.
    pub const fn with_receiver(
        cx: &'a Cx<'a>,
        receiver: Receiver<'a>,
        rest: &'a MultiValue,
    ) -> Self {
        Self {
            cx,
            values: rest,
            receiver: Some(receiver),
            nil: Value::Nil,
        }
    }

    /// The argument `index` (1-based), nil when the script passed fewer.
    pub fn value(&self, index: usize) -> &Value {
        let Some(receiver) = &self.receiver else {
            return index
                .checked_sub(1)
                .and_then(|position| self.values.get(position))
                .unwrap_or(&self.nil);
        };
        match index {
            0 => &self.nil,
            1 => receiver.value.get_or_init(|| (receiver.make)()),
            _ => self.values.get(index - 2).unwrap_or(&self.nil),
        }
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
        match &self.receiver {
            Some(receiver) if index == 1 => Some(receiver.payload),
            _ => host_payload(self.value(index)),
        }
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
