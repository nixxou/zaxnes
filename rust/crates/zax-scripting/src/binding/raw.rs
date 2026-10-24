//! Host values as raw Lua userdata with C metamethods (Rust-only performance work, owner
//! decision; the "fast Lua binding" setting).
//!
//! Every host value of SPEC-05 §4.2 and §4.3 is then a block `{ magic, payload }` that Lua owns,
//! with one shared metatable. `__index` and `__newindex` are C functions that read the payload
//! and the member name straight from the Lua stack and call the getter or setter of the API
//! table, without the `mlua` callback machinery (argument conversion, userdata type lookup and
//! borrow), which cost several times the getters themselves.
//!
//! Behaviour is the one of the `mlua` userdata it replaces:
//! - what the fast path does not cover (a numeric index, a member the table lacks, a method
//!   called on another value) runs the safe binding's own Rust functions from the same C
//!   function, so an error names the script line as it would (REQ-LUA-061); only a metamethod
//!   called by hand on a value that is not a host value goes to the `mlua` function kept as
//!   upvalue 1;
//! - an error of a getter or setter is raised by an `mlua` function, so the script and the logs
//!   see the very error object the safe binding raised;
//! - the other metamethods (`__eq`, `__call`, `__tostring`, the operators) are those `mlua`
//!   functions, and `__name` is the name `mlua` gave the userdata type.
//!
//! Soundness rests on these rules:
//! - a block is only read after its metatable was checked to be the shared one, which only
//!   [`push_host`] puts on a userdata, and [`push_host`] writes the whole block first; the
//!   metamethods hold that metatable as their upvalue 3, the registry under an integer key;
//! - the metamethods hold the method functions in a table, their upvalue 4, keyed by
//!   [`method_key`]: C closures that read their receiver from its block (its Lua value is made
//!   only when the method asks for it) and fall back on the `mlua` function of the method;
//! - a value a getter returns is pushed by [`run`] itself when it is a host value
//!   ([`Deferred`]): a block and its metatable, after every Rust value of the getter is gone;
//! - the payload holds plain values and identifiers, never a pointer, and needs no destructor:
//!   Lua frees the block with nothing to run, and a stale handle is caught by its frame epoch;
//! - a C function makes room on the stack before pushing, and drops every Rust value before it
//!   calls a function that may raise a Lua error (the raise and the fallbacks), so the error's
//!   long jump never skips a destructor.

#![allow(unsafe_code)]

use std::cell::{Cell, RefCell};
use std::ffi::{c_int, c_void};
use std::sync::atomic::{AtomicUsize, Ordering};

use mlua::{ffi, AnyUserData, FromLua, Function, IntoLua, Lua, Value};

use super::cx::{Call, Cx, Receiver};
use super::dispatch::{api, assign, index};
use super::payload::Payload;
use super::table::MemberKind;

/// The block of a raw host value.
#[repr(C)]
#[derive(Clone, Copy)]
struct RawHost {
    magic: u64,
    payload: Payload,
}

/// Marks a block written by [`push_host`] (checked in debug builds; the metatable is the test).
const MAGIC: u64 = 0x7a61_786e_6573_6831;

/// The light userdata a getter returns in place of a host value it deferred ([`Deferred`]): the
/// address of this byte.
static DEFERRED_MARK: u8 = 0;

fn deferred_mark() -> *mut c_void {
    std::ptr::addr_of!(DEFERRED_MARK).cast_mut().cast()
}

/// A host value a getter made, which the fast path pushes itself instead of making a `mlua`
/// value of it: [`Cx::value`] takes the first one of a getter run under [`Cx::deferring`] and
/// returns the mark; a second one spoils the deferral and the getter runs again normally.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Deferred {
    Empty,
    Taken(Payload),
    Spoiled,
}

impl Deferred {
    /// What [`Cx::value`] returns for `payload` under the deferral `cell`, if it takes it.
    pub(crate) fn take(cell: &Cell<Self>, payload: Payload) -> Option<Value> {
        if matches!(cell.get(), Self::Empty) {
            cell.set(Self::Taken(payload));
            Some(Value::LightUserData(mlua::LightUserData(deferred_mark())))
        } else {
            cell.set(Self::Spoiled);
            None
        }
    }
}

/// Present in a state whose host values are raw: the registry key of the shared metatable.
pub(crate) struct RawBinding {
    metatable: c_int,
}

/// How many Lua states of the process have raw host values. While none has, nothing here reads
/// a userdata block: with the optimised Lua off, no `unsafe` code of the binding runs.
static RAW_STATES: AtomicUsize = AtomicUsize::new(0);

impl RawBinding {
    fn new(metatable: c_int) -> Self {
        RAW_STATES.fetch_add(1, Ordering::Relaxed);
        Self { metatable }
    }
}

impl Drop for RawBinding {
    fn drop(&mut self) {
        RAW_STATES.fetch_sub(1, Ordering::Relaxed);
    }
}

/// The error a fast getter or setter returned, waiting for the raise function.
#[derive(Default)]
struct PendingError(RefCell<Option<mlua::Error>>);

/// Whether some Lua state of the process has the optimised Lua: one atomic load, which keeps
/// the safe paths as they were while none has.
pub(crate) fn any_state() -> bool {
    RAW_STATES.load(Ordering::Relaxed) != 0
}

/// The registry key of the shared metatable of `lua`, when its host values are raw.
pub(crate) fn metatable_key(lua: &Lua) -> Option<c_int> {
    if RAW_STATES.load(Ordering::Relaxed) == 0 {
        return None;
    }
    lua.app_data_ref::<RawBinding>()
        .map(|binding| binding.metatable)
}

/// The `mlua` functions the raw metatable falls back on.
pub(crate) struct Fallbacks {
    pub index: Function,
    pub new_index: Function,
    /// The other metamethods, by their Lua name (`__eq`, `__call`...).
    pub others: Vec<(&'static str, Function)>,
    /// The `__name` of the safe binding's userdata type.
    pub name: String,
    /// The method functions, keyed by [`method_key`].
    pub methods: mlua::Table,
}

/// The key of method `position` of type `type_index` in [`Fallbacks::methods`].
pub(crate) fn method_key(type_index: usize, position: usize) -> mlua::Integer {
    mlua::Integer::try_from(type_index * 1024 + position + 1).unwrap_or(mlua::Integer::MAX)
}

/// Turns raw host values on for `lua`: builds and registers the shared metatable.
///
/// # Errors
///
/// A Lua error while the metatable is built.
pub(crate) fn install(lua: &Lua, fallbacks: Fallbacks) -> mlua::Result<()> {
    lua.set_app_data(PendingError::default());
    let raise = lua.create_function(|lua, ()| -> mlua::Result<()> {
        let pending = lua
            .app_data_ref::<PendingError>()
            .and_then(|pending| pending.0.borrow_mut().take());
        Err(pending.unwrap_or_else(|| mlua::Error::runtime("an error of the script API")))
    })?;

    let metatable = lua.create_table()?;
    for (name, function) in fallbacks.others {
        metatable.raw_set(name, function)?;
    }
    metatable.raw_set("__name", fallbacks.name)?;
    // The methods become C closures over their `mlua` function.
    let methods = lua.create_table()?;
    for pair in fallbacks.methods.pairs::<mlua::Integer, Function>() {
        let (key, function) = pair?;
        // SAFETY: the closure only runs as a method function of the raw binding, with its
        // `mlua` function, the raise function, the metatable and its key as upvalues.
        let method =
            unsafe { closure(lua, meta_method, function, raise.clone(), (&metatable, key)) }?;
        methods.raw_set(key, method)?;
    }
    let upvalues = (&metatable, &methods);
    // SAFETY: the two closures only run as metamethods of the table built here, with the
    // fallback, the raise function, that table and the methods as upvalues, as `closure`
    // creates them.
    let index = unsafe { closure(lua, meta_index, fallbacks.index, raise.clone(), upvalues) }?;
    // SAFETY: as above.
    let new_index = unsafe { closure(lua, meta_new_index, fallbacks.new_index, raise, upvalues) }?;
    metatable.raw_set("__index", index)?;
    metatable.raw_set("__newindex", new_index)?;

    // SAFETY: `exec_raw` runs the closure with the table on top of the stack, protected;
    // `luaL_ref` pops it into the registry and raises nothing but a memory error, which the
    // protection catches.
    let key = unsafe {
        lua.exec_raw::<c_int>(metatable, |state| {
            let key = ffi::luaL_ref(state, ffi::LUA_REGISTRYINDEX);
            ffi::lua_pushinteger(state, ffi::lua_Integer::from(key));
        })?
    };
    lua.set_app_data(RawBinding::new(key));
    Ok(())
}

/// A C closure of `function` with `fallback`, `raise`, the shared metatable, `fourth` (the
/// method table of a metamethod, the key of a method) and the state's `Lua` (a light userdata)
/// as upvalues 1 to 5.
///
/// # Safety
///
/// `function` must only read its upvalues as these.
unsafe fn closure(
    lua: &Lua,
    function: ffi::lua_CFunction,
    fallback: Function,
    raise: Function,
    (metatable, fourth): (&mlua::Table, impl IntoLua),
) -> mlua::Result<Function> {
    // SAFETY: the five upvalues are on the stack, protected by `exec_raw`. The `Lua` that
    // `get_or_init_from_ptr` returns lives in the state's extra data, at the same address for as
    // long as the state, which outlives every call of the closure.
    unsafe {
        lua.exec_raw::<Function>((fallback, raise, metatable, fourth), |state| {
            let owner: *const Lua = Lua::get_or_init_from_ptr(state);
            ffi::lua_pushlightuserdata(state, owner.cast_mut().cast());
            ffi::lua_pushcclosure(state, function, 5);
        })
    }
}

/// The upvalue of the metamethods that holds the shared metatable.
fn metatable_upvalue() -> c_int {
    ffi::lua_upvalueindex(3)
}

/// The Lua value of a raw host value of `payload`; `metatable` is the registry key of
/// [`metatable_key`].
///
/// # Errors
///
/// A Lua error (memory) while the value is made.
pub(crate) fn value(lua: &Lua, metatable: c_int, payload: Payload) -> mlua::Result<Value> {
    // SAFETY: `exec_raw` protects the allocation; `push_host` leaves exactly the new value on
    // the stack, which `exec_raw` returns.
    unsafe {
        lua.exec_raw::<Value>((), |state| {
            push_host(state, payload, |state| {
                ffi::lua_rawgeti(
                    state,
                    ffi::LUA_REGISTRYINDEX,
                    ffi::lua_Integer::from(metatable),
                );
            });
        })
    }
}

/// Pushes a new raw host value of `payload`; `push_metatable` pushes the shared metatable.
///
/// # Safety
///
/// Two free stack slots; `push_metatable` pushes the shared metatable and nothing else. May raise
/// a memory error.
unsafe fn push_host(
    state: *mut ffi::lua_State,
    payload: Payload,
    push_metatable: impl FnOnce(*mut ffi::lua_State),
) {
    // SAFETY: the caller guarantees the stack room; the block is written whole before the
    // metatable that marks it as a host value is set.
    unsafe {
        let block =
            ffi::lua_newuserdatauv(state, std::mem::size_of::<RawHost>(), 0).cast::<RawHost>();
        block.write(RawHost {
            magic: MAGIC,
            payload,
        });
        push_metatable(state);
        ffi::lua_setmetatable(state, -2);
    }
}

/// The block of the value at `index` when it is a raw host value.
///
/// # Safety
///
/// Called from a metamethod made by `closure`; `index` is a valid stack index; one free stack
/// slot.
unsafe fn block_at(state: *mut ffi::lua_State, index: c_int) -> Option<*mut RawHost> {
    // SAFETY: only stack reads; the block is cast only once its metatable is the shared one
    // (upvalue 3), which `push_host` alone sets, on a block it wrote whole.
    unsafe {
        if ffi::lua_type(state, index) != ffi::LUA_TUSERDATA {
            return None;
        }
        let index = ffi::lua_absindex(state, index);
        if ffi::lua_getmetatable(state, index) == 0 {
            return None;
        }
        let ours = ffi::lua_rawequal(state, -1, metatable_upvalue()) != 0;
        ffi::lua_pop(state, 1);
        if !ours {
            return None;
        }
        let block = ffi::lua_touserdata(state, index).cast::<RawHost>();
        debug_assert_eq!(MAGIC, (*block).magic);
        Some(block)
    }
}

/// The block of `value` when it is a raw host value.
///
/// Every userdata of a runtime's state that `mlua` did not register is a raw host value or a
/// stream of the `io` library (`luaL_Stream`, 16 bytes): no other library of the state makes
/// userdata, and scripts cannot. So the first 8 bytes of an unregistered userdata are in bounds,
/// and only a raw host value carries [`MAGIC`] there (a stream holds a `FILE *`, which no
/// canonical address equals).
fn block_of(value: &AnyUserData) -> Option<*mut RawHost> {
    if RAW_STATES.load(Ordering::Relaxed) == 0 || value.type_id().is_some() {
        return None;
    }
    let block = value.to_pointer().cast::<RawHost>().cast_mut();
    // SAFETY: see above: an unregistered userdata holds at least 8 bytes, read unaligned to make
    // no assumption on a stream's layout.
    let magic = unsafe { block.cast::<u64>().read_unaligned() };
    (magic == MAGIC).then_some(block)
}

/// The payload of `value` when it is a raw host value.
pub(crate) fn payload_of(value: &AnyUserData) -> Option<Payload> {
    // SAFETY: `block_of` returns only blocks written whole by `push_host`.
    block_of(value).map(|block| unsafe { (*block).payload })
}

/// Rewrites the payload of `value` when it is a raw host value (a value type's component
/// assigned by a script, REQ-LUA-044). Returns whether it was one.
pub(crate) fn set_payload(value: &AnyUserData, payload: Payload) -> bool {
    // SAFETY: as in `payload_of`; Lua owns the block and nothing else holds a reference to it.
    block_of(value)
        .map(|block| unsafe { (*block).payload = payload })
        .is_some()
}

/// What the fast path of a metamethod did.
enum Fast {
    /// It pushed this many results.
    Done(c_int),
    /// Its result is the host value of this payload, which [`run`] pushes.
    Host(Payload),
    /// Its result is the method function of this key in the method table, which [`run`] pushes.
    Method(mlua::Integer),
    /// It left the call to the fallback.
    Fallback,
    /// A getter or setter failed: its error waits in [`PendingError`].
    Raise,
}

/// The string at `index`, when it is one.
///
/// # Safety
///
/// `index` is a valid stack index; the bytes live as long as the value stays on the stack.
unsafe fn string_at<'a>(state: *mut ffi::lua_State, index: c_int) -> Option<&'a [u8]> {
    // SAFETY: a Lua string is not converted in place by `lua_tolstring` (the type is checked
    // first) and stays alive while it is on the stack, which outlives the use of the slice.
    unsafe {
        if ffi::lua_type(state, index) != ffi::LUA_TSTRING {
            return None;
        }
        let mut length = 0;
        let text = ffi::lua_tolstring(state, index, &raw mut length);
        Some(std::slice::from_raw_parts(text.cast::<u8>(), length))
    }
}

/// Keeps `error` for the raise function.
fn stash(lua: &Lua, error: mlua::Error) {
    if let Some(pending) = lua.app_data_ref::<PendingError>() {
        *pending.0.borrow_mut() = Some(error);
    }
}

/// Pushes `value`.
///
/// # Safety
///
/// One free stack slot.
unsafe fn push_value(lua: &Lua, value: Value) -> bool {
    lua.exec_raw_lua(|raw| {
        // SAFETY: the caller made room; pushing a value raises nothing but a memory error.
        unsafe { value.push_into_stack(raw).is_ok() }
    })
}

/// `obj[key]` on a raw host value.
fn fast_index(lua: &Lua, state: *mut ffi::lua_State) -> Fast {
    // SAFETY: the metamethod receives (object, key): indices 1 and 2 are valid, and room for
    // the two slots `block_at` uses and the result was made by the caller.
    let (block, key) = unsafe { (block_at(state, 1), string_at(state, 2)) };
    let Some(block) = block else {
        return Fast::Fallback;
    };
    // SAFETY: `block_at` checked the block.
    let payload = unsafe { (*block).payload };
    let found = key.and_then(|key| {
        let definition = api().get(payload.host_type())?;
        definition
            .member(key)
            .map(|(position, member)| (definition, position, member))
    });
    let Some((definition, position, member)) = found else {
        // A numeric index, or a member the type lacks: the safe binding's answer, from here.
        return match stack_value(lua, 2).and_then(|key| index(lua, payload, &key)) {
            Ok(value) => push_result(lua, value),
            Err(error) => {
                stash(lua, error);
                Fast::Raise
            }
        };
    };
    let value = match &member.kind {
        MemberKind::Property { get, .. } => {
            let deferred = Cell::new(Deferred::Empty);
            let first = get(&Cx::deferring(lua, &deferred), &payload);
            let outcome = match (first, deferred.get()) {
                (Ok(Value::LightUserData(mark)), Deferred::Taken(host))
                    if mark.0 == deferred_mark() =>
                {
                    return Fast::Host(host);
                }
                (first, Deferred::Empty) => first,
                // The getter made more than one host value, or kept the one it made: it runs
                // again with plain values.
                _ => get(&Cx::new(lua), &payload),
            };
            match outcome {
                Ok(value) => value,
                Err(error) => {
                    stash(lua, error);
                    return Fast::Raise;
                }
            }
        }
        MemberKind::Method(_) => {
            return Fast::Method(method_key(definition.id.index(), position));
        }
    };
    push_result(lua, value)
}

/// The value at `index` of the stack of the current call.
fn stack_value(lua: &Lua, index: c_int) -> mlua::Result<Value> {
    lua.exec_raw_lua(|raw| {
        // SAFETY: the callers read the arguments of the metamethod running, which are on the
        // stack; reading one raises nothing.
        unsafe { Value::from_stack(index, raw) }
    })
}

/// Pushes the one result of a fast path.
fn push_result(lua: &Lua, value: Value) -> Fast {
    // SAFETY: `run` made room for the result.
    if unsafe { push_value(lua, value) } {
        Fast::Done(1)
    } else {
        Fast::Fallback
    }
}

/// `obj[key] = value` on a raw host value.
fn fast_new_index(lua: &Lua, state: *mut ffi::lua_State) -> Fast {
    // SAFETY: the metamethod receives (object, key, value): indices 1 to 3 are valid.
    let (block, key) = unsafe { (block_at(state, 1), string_at(state, 2)) };
    let Some(block) = block else {
        return Fast::Fallback;
    };
    // SAFETY: `block_at` checked the block.
    let mut payload = unsafe { (*block).payload };
    let Ok(value) = stack_value(lua, 3) else {
        return Fast::Fallback;
    };
    let found = key.and_then(|key| {
        let definition = api().get(payload.host_type())?;
        match &definition.member(key)?.1.kind {
            MemberKind::Property { set: Some(set), .. } => Some((definition, set)),
            _ => None,
        }
    });
    let Some((definition, set)) = found else {
        // A numeric index, a read-only or unknown member: the safe binding's answer, from here.
        let assigned = stack_value(lua, 2).and_then(|key| assign(lua, payload, &key, &value));
        return match assigned {
            Ok(written) => {
                if let Some(written) = written {
                    // SAFETY: `block_at` checked the block, which is still on the stack.
                    unsafe { (*block).payload = written };
                }
                Fast::Done(0)
            }
            Err(error) => {
                stash(lua, error);
                Fast::Raise
            }
        };
    };
    let cx = Cx::new(lua);
    match set(&cx, &mut payload, &value) {
        Ok(()) => {
            if definition.id.is_value() {
                // The value is the script's own copy: the component it assigned belongs to the
                // userdata it assigned it to (REQ-LUA-044).
                // SAFETY: `block_at` checked the block, which is still on the stack.
                unsafe { (*block).payload = payload };
            }
            Fast::Done(0)
        }
        Err(error) => {
            stash(lua, error);
            Fast::Raise
        }
    }
}

/// `receiver:method(...)` on a raw host value: the method of upvalue 4's key, called with the
/// receiver read from its block.
fn fast_method(lua: &Lua, state: *mut ffi::lua_State) -> Fast {
    // SAFETY: a method closure has its key as upvalue 4 and receives at least its receiver slot
    // (nil when the script passed nothing, which `block_at` refuses).
    let (block, key) = unsafe {
        (
            block_at(state, 1),
            ffi::lua_tointeger(state, ffi::lua_upvalueindex(4)),
        )
    };
    let Ok(code) = usize::try_from(key - 1) else {
        return Fast::Fallback;
    };
    let Some(MemberKind::Method(method)) = api()
        .by_index(code / 1024)
        .and_then(|definition| definition.members.get(code % 1024))
        .map(|member| &member.kind)
    else {
        return Fast::Fallback;
    };
    // The receiver is read from its block; called with `.` on another value, every argument is
    // read as the safe binding reads it.
    let first = if block.is_some() { 2 } else { 1 };
    // SAFETY: the arguments are on the stack; reading them raises nothing.
    let top = unsafe { ffi::lua_gettop(state) };
    let Ok(arguments) = (first..=top)
        .map(|index| stack_value(lua, index))
        .collect::<mlua::Result<mlua::MultiValue>>()
    else {
        return Fast::Fallback;
    };
    let make = || stack_value(lua, 1).unwrap_or(Value::Nil);
    let cx = Cx::new(lua);
    let call = match block {
        // SAFETY: `block_at` checked the block, which stays on the stack during the call.
        Some(block) => Call::with_receiver(
            &cx,
            Receiver::new(unsafe { (*block).payload }, &make),
            &arguments,
        ),
        None => Call::new(&cx, &arguments),
    };
    match method(&call) {
        Ok(None) => Fast::Done(0),
        Ok(Some(value)) => {
            // SAFETY: `run` made room for the result.
            if unsafe { push_value(lua, value) } {
                Fast::Done(1)
            } else {
                Fast::Fallback
            }
        }
        Err(error) => {
            stash(lua, error);
            Fast::Raise
        }
    }
}

/// Runs the fast path `fast`, or the fallback (upvalue 1) on the original arguments, or the
/// raise function (upvalue 2).
///
/// # Safety
///
/// Called by Lua as a metamethod closure made by `closure`.
unsafe fn run(state: *mut ffi::lua_State, fast: fn(&Lua, *mut ffi::lua_State) -> Fast) -> c_int {
    // SAFETY: Lua calls the closure with its arguments on the stack; upvalue 5 is the `Lua` that
    // owns this state (see `closure`), which outlives the call. Every Rust value of the fast path
    // is dropped when it returns, before `lua_call` may raise.
    unsafe {
        if ffi::lua_checkstack(state, 4) == 0 {
            return call_upvalue(state, 1);
        }
        let lua = &*ffi::lua_touserdata(state, ffi::lua_upvalueindex(5))
            .cast_const()
            .cast::<Lua>();
        // The fast paths read and push through `mlua`, which works on the state it runs: in a
        // coroutine the `mlua` function, which switches to it, does the work.
        if lua.exec_raw_lua(|raw| raw.state() != state) {
            return call_upvalue(state, 1);
        }
        let outcome = fast(lua, state);
        match outcome {
            Fast::Done(results) => results,
            // Pushed here, with nothing of the getter left: a memory error may raise.
            Fast::Host(payload) => {
                push_host(state, payload, |state| {
                    ffi::lua_pushvalue(state, metatable_upvalue());
                });
                1
            }
            Fast::Method(key) => {
                if ffi::lua_rawgeti(state, ffi::lua_upvalueindex(4), key) == ffi::LUA_TNIL {
                    ffi::lua_pop(state, 1);
                    return call_upvalue(state, 1);
                }
                1
            }
            Fast::Fallback => call_upvalue(state, 1),
            Fast::Raise => {
                ffi::lua_settop(state, 0);
                call_upvalue(state, 2)
            }
        }
    }
}

/// Calls upvalue `upvalue` with every argument on the stack and returns all its results.
///
/// # Safety
///
/// Called from a C closure with function upvalues; nothing Rust is alive in the caller's frame.
unsafe fn call_upvalue(state: *mut ffi::lua_State, upvalue: c_int) -> c_int {
    // SAFETY: the stack holds the arguments; one slot is needed for the function.
    unsafe {
        let arguments = ffi::lua_gettop(state);
        ffi::luaL_checkstack(state, 1, std::ptr::null());
        ffi::lua_pushvalue(state, ffi::lua_upvalueindex(upvalue));
        ffi::lua_insert(state, 1);
        ffi::lua_call(state, arguments, ffi::LUA_MULTRET);
        ffi::lua_gettop(state)
    }
}

unsafe extern "C-unwind" fn meta_index(state: *mut ffi::lua_State) -> c_int {
    // SAFETY: this closure is only the `__index` of the shared metatable.
    unsafe { run(state, fast_index) }
}

unsafe extern "C-unwind" fn meta_method(state: *mut ffi::lua_State) -> c_int {
    // SAFETY: this closure is only a method function made by `install`.
    unsafe { run(state, fast_method) }
}

unsafe extern "C-unwind" fn meta_new_index(state: *mut ffi::lua_State) -> c_int {
    // SAFETY: this closure is only the `__newindex` of the shared metatable.
    unsafe { run(state, fast_new_index) }
}
