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
//! - what the fast path does not cover (a numeric index, a member the table lacks, a value that
//!   is not a host value) goes to the `mlua` functions the safe binding uses, kept as upvalues;
//! - an error of a getter or setter is raised by an `mlua` function, so the script and the logs
//!   see the very error object the safe binding raised;
//! - the other metamethods (`__eq`, `__call`, `__tostring`, the operators) are those `mlua`
//!   functions, and `__name` is the name `mlua` gave the userdata type.
//!
//! Soundness rests on these rules:
//! - a block is only read after its metatable was checked to be the shared one, which only
//!   [`push_host`] puts on a userdata, and [`push_host`] writes the whole block first;
//! - the payload holds plain values and identifiers, never a pointer, and needs no destructor:
//!   Lua frees the block with nothing to run, and a stale handle is caught by its frame epoch;
//! - a C function makes room on the stack before pushing, and drops every Rust value before it
//!   calls a function that may raise a Lua error (the raise and the fallbacks), so the error's
//!   long jump never skips a destructor.

#![allow(unsafe_code)]

use std::cell::RefCell;
use std::ffi::{c_int, c_void};
use std::sync::atomic::{AtomicUsize, Ordering};

use mlua::{ffi, AnyUserData, FromLua, Function, IntoLua, Lua, Value};

use super::cx::Cx;
use super::dispatch::{api, method_function};
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

/// The registry key of the shared metatable: the address of this byte.
static METATABLE_KEY: u8 = 0;

fn metatable_key() -> *const c_void {
    std::ptr::addr_of!(METATABLE_KEY).cast()
}

/// Present in a state whose host values are raw.
pub(crate) struct RawBinding;

/// How many Lua states of the process have raw host values. While none has, nothing here reads
/// a userdata block: with the optimised Lua off, no `unsafe` code of the binding runs.
static RAW_STATES: AtomicUsize = AtomicUsize::new(0);

impl RawBinding {
    fn new() -> Self {
        RAW_STATES.fetch_add(1, Ordering::Relaxed);
        Self
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

/// Whether the host values of `lua` are raw.
pub(crate) fn enabled(lua: &Lua) -> bool {
    RAW_STATES.load(Ordering::Relaxed) != 0 && lua.app_data_ref::<RawBinding>().is_some()
}

/// The `mlua` functions the raw metatable falls back on.
pub(crate) struct Fallbacks {
    pub index: Function,
    pub new_index: Function,
    /// The other metamethods, by their Lua name (`__eq`, `__call`...).
    pub others: Vec<(&'static str, Function)>,
    /// The `__name` of the safe binding's userdata type.
    pub name: String,
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
    // SAFETY: the two closures only run as metamethods of the table built here, with the
    // fallback and the raise function as upvalues, as `closure` creates them.
    let index = unsafe { closure(lua, meta_index, fallbacks.index, raise.clone()) }?;
    // SAFETY: as above.
    let new_index = unsafe { closure(lua, meta_new_index, fallbacks.new_index, raise) }?;
    metatable.raw_set("__index", index)?;
    metatable.raw_set("__newindex", new_index)?;

    // SAFETY: `exec_raw` runs the closure with the table on top of the stack, protected; storing
    // it in the registry under a light userdata key raises nothing but a memory error, which the
    // protection catches.
    unsafe {
        lua.exec_raw::<()>(metatable, |state| {
            ffi::lua_rawsetp(state, ffi::LUA_REGISTRYINDEX, metatable_key());
        })?;
    }
    lua.set_app_data(RawBinding::new());
    Ok(())
}

/// A C closure of `function` with `fallback` and `raise` as upvalues 1 and 2.
///
/// # Safety
///
/// `function` must only read its upvalues as these two functions.
unsafe fn closure(
    lua: &Lua,
    function: ffi::lua_CFunction,
    fallback: Function,
    raise: Function,
) -> mlua::Result<Function> {
    // SAFETY: the two upvalues are on the stack, protected by `exec_raw`.
    unsafe {
        lua.exec_raw::<Function>((fallback, raise), |state| {
            ffi::lua_pushcclosure(state, function, 2);
        })
    }
}

/// The Lua value of a raw host value of `payload`.
///
/// # Errors
///
/// A Lua error (memory) while the value is made.
pub(crate) fn value(lua: &Lua, payload: Payload) -> mlua::Result<Value> {
    // SAFETY: `exec_raw` protects the allocation and the metatable lookup; `push_host` leaves
    // exactly the new value on the stack, which `exec_raw` returns.
    unsafe {
        lua.exec_raw::<Value>((), |state| {
            push_host(state, payload);
        })
    }
}

/// Pushes a new raw host value of `payload`.
///
/// # Safety
///
/// Two free stack slots; the metatable is registered. May raise a memory error.
unsafe fn push_host(state: *mut ffi::lua_State, payload: Payload) {
    // SAFETY: the caller guarantees the stack room; the block is written whole before the
    // metatable that marks it as a host value is set.
    unsafe {
        let block =
            ffi::lua_newuserdatauv(state, std::mem::size_of::<RawHost>(), 0).cast::<RawHost>();
        block.write(RawHost {
            magic: MAGIC,
            payload,
        });
        ffi::lua_rawgetp(state, ffi::LUA_REGISTRYINDEX, metatable_key());
        ffi::lua_setmetatable(state, -2);
    }
}

/// The block of the value at `index` when it is a raw host value.
///
/// # Safety
///
/// `index` is a valid stack index; two free stack slots.
unsafe fn block_at(state: *mut ffi::lua_State, index: c_int) -> Option<*mut RawHost> {
    // SAFETY: only stack reads and a registry read; the block is cast only once its metatable is
    // the shared one, which `push_host` alone sets, on a block it wrote whole.
    unsafe {
        if ffi::lua_type(state, index) != ffi::LUA_TUSERDATA {
            return None;
        }
        let index = ffi::lua_absindex(state, index);
        if ffi::lua_getmetatable(state, index) == 0 {
            return None;
        }
        ffi::lua_rawgetp(state, ffi::LUA_REGISTRYINDEX, metatable_key());
        let ours = ffi::lua_rawequal(state, -1, -2) != 0;
        ffi::lua_pop(state, 2);
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
    let (Some(block), Some(key)) = (block, key) else {
        return Fast::Fallback;
    };
    // SAFETY: `block_at` checked the block.
    let payload = unsafe { (*block).payload };
    let Some(definition) = api().get(payload.host_type()) else {
        return Fast::Fallback;
    };
    let Some((position, member)) = definition.member(key) else {
        return Fast::Fallback;
    };
    let value = match &member.kind {
        MemberKind::Property { get, .. } => {
            let cx = Cx::new(lua);
            match get(&cx, &payload) {
                Ok(value) => value,
                Err(error) => {
                    stash(lua, error);
                    return Fast::Raise;
                }
            }
        }
        MemberKind::Method(_) => match method_function(lua, definition.id, position) {
            Some(function) => Value::Function(function),
            None => return Fast::Fallback,
        },
    };
    // SAFETY: room for the result was made by the caller.
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
    let (Some(block), Some(key)) = (block, key) else {
        return Fast::Fallback;
    };
    // SAFETY: `block_at` checked the block.
    let mut payload = unsafe { (*block).payload };
    let Some(definition) = api().get(payload.host_type()) else {
        return Fast::Fallback;
    };
    let Some((_, member)) = definition.member(key) else {
        return Fast::Fallback;
    };
    let MemberKind::Property { set: Some(set), .. } = &member.kind else {
        return Fast::Fallback;
    };
    let value = lua.exec_raw_lua(|raw| {
        // SAFETY: index 3 is the assigned value; reading it raises nothing.
        unsafe { Value::from_stack(3, raw) }
    });
    let Ok(value) = value else {
        return Fast::Fallback;
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

/// Runs the fast path `fast`, or the fallback (upvalue 1) on the original arguments, or the
/// raise function (upvalue 2).
///
/// # Safety
///
/// Called by Lua as a metamethod closure made by `closure`.
unsafe fn run(state: *mut ffi::lua_State, fast: fn(&Lua, *mut ffi::lua_State) -> Fast) -> c_int {
    // SAFETY: Lua calls the closure with its arguments on the stack; `get_or_init_from_ptr`
    // returns the `Lua` that owns this state, which outlives the call. Every Rust value of the
    // fast path is dropped when it returns, before `lua_call` may raise.
    unsafe {
        if ffi::lua_checkstack(state, 4) == 0 {
            return call_upvalue(state, 1);
        }
        let outcome = {
            let lua = Lua::get_or_init_from_ptr(state);
            fast(lua, state)
        };
        match outcome {
            Fast::Done(results) => results,
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

unsafe extern "C-unwind" fn meta_new_index(state: *mut ffi::lua_State) -> c_int {
    // SAFETY: this closure is only the `__newindex` of the shared metatable.
    unsafe { run(state, fast_new_index) }
}
