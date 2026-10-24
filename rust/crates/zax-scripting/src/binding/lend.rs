//! The lend of the runtime's data and of the engine state to the frame calls into Lua, as raw
//! pointers: part of the optimised Lua (Rust-only setting `emu.lua_optimized`, owner decision).
//! Without it, the same objects are lent through `mlua` scoped userdata (`cx::Lent`), whose
//! per-access type check and borrow bookkeeping cost more than most getters.
//!
//! The host lends them for the length of one call into Lua; every access from the script API
//! goes through [`with_data`], [`with_state`] or [`with_build`], which turn the pointer back into
//! a `&mut` for the length of a closure. Soundness rests on three rules, each enforced here:
//! - a pointer is only stored by [`lend`], which takes the `&mut` it comes from for the life of
//!   the returned guard, so the owner cannot touch the object while it is lent, and the guard
//!   takes the pointer back when it drops (a panic included);
//! - at most one `&mut` per lent object exists at a time: a busy flag per object makes a second,
//!   nested access fail with the error `mlua` gave for the same conflict;
//! - a lend nested in another one saves and restores the outer lend and its flags.
//!
//! The lend lives in the Lua state's application data ([`LendSlot`]), installed only for a
//! runtime with the optimised Lua, so two runtimes never see each other's objects.

#![allow(unsafe_code)]

use std::cell::Cell;
use std::marker::PhantomData;
use std::ptr::NonNull;

use mlua::Lua;
use zax_engine3d::engine::ConversionState;
use zax_engine3d::instance::{InstanceRef, PatternInstance};

use crate::runtime::RuntimeData;

/// What one call into Lua has been lent.
#[derive(Clone, Copy)]
struct Lend {
    data: NonNull<RuntimeData>,
    state: Option<NonNull<ConversionState>>,
    build: Option<(InstanceRef, NonNull<PatternInstance>)>,
}

/// Busy flags, one bit per lent object.
const DATA: u8 = 1;
const STATE: u8 = 2;
const BUILD: u8 = 4;

/// The lend of one Lua state, installed once with the optimised binding.
#[derive(Default)]
pub(crate) struct LendSlot {
    current: Cell<Option<Lend>>,
    busy: Cell<u8>,
}

impl LendSlot {
    /// Whether a call into Lua holds a lend now.
    pub fn is_lent(&self) -> bool {
        self.current.get().is_some()
    }
}

/// Why a lent object could not be reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LendError {
    /// The object is not part of this lend (no engine state).
    Absent,
    /// The object is already borrowed by an access still running.
    Busy,
}

/// Installs the lend slot of `lua`. It is never removed nor replaced, which [`slot`] relies on.
pub(crate) fn install(lua: &Lua) {
    lua.set_app_data(LendSlot::default());
}

/// The lend slot of `lua`, when its runtime has the optimised Lua, without keeping the
/// application data borrowed: the closure an access runs may itself use the application data.
pub(crate) fn slot(lua: &Lua) -> Option<&LendSlot> {
    let pointer = lua
        .app_data_ref::<LendSlot>()
        .map(|slot| NonNull::from(&*slot))?;
    // SAFETY: the slot is a boxed application datum that `install` puts in once and nothing
    // removes or replaces, so it lives, at the same address, as long as the Lua state, which
    // outlives the borrow of `lua`. It is only used through its `Cell`s, on this thread.
    Some(unsafe { pointer.as_ref() })
}

/// Restores the lend that was in force before [`lend`] when it drops.
pub(crate) struct LendGuard<'a> {
    slot: &'a LendSlot,
    previous: Option<Lend>,
    previous_busy: u8,
    _lent: PhantomData<&'a mut ()>,
}

impl Drop for LendGuard<'_> {
    fn drop(&mut self) {
        self.slot.current.set(self.previous);
        self.slot.busy.set(self.previous_busy);
    }
}

/// Lends `data`, `state` and the instance being built to the calls into Lua that reach `slot`
/// until the guard drops.
pub(crate) fn lend<'a>(
    slot: &'a LendSlot,
    data: &'a mut RuntimeData,
    state: Option<&'a mut ConversionState>,
    build: Option<(InstanceRef, &'a mut PatternInstance)>,
) -> LendGuard<'a> {
    let lend = Lend {
        data: NonNull::from(data),
        state: state.map(NonNull::from),
        build: build.map(|(reference, instance)| (reference, NonNull::from(instance))),
    };
    LendGuard {
        slot,
        previous: slot.current.replace(Some(lend)),
        previous_busy: slot.busy.replace(0),
        _lent: PhantomData,
    }
}

/// Holds a busy bit and clears it when it drops, so that a panicking access frees its object.
struct Busy<'a> {
    slot: &'a LendSlot,
    bit: u8,
}

impl<'a> Busy<'a> {
    fn take(slot: &'a LendSlot, bit: u8) -> Result<Self, LendError> {
        let flags = slot.busy.get();
        if flags & bit != 0 {
            return Err(LendError::Busy);
        }
        slot.busy.set(flags | bit);
        Ok(Self { slot, bit })
    }
}

impl Drop for Busy<'_> {
    fn drop(&mut self) {
        self.slot.busy.set(self.slot.busy.get() & !self.bit);
    }
}

/// The lend in force; callers check [`LendSlot::is_lent`] first.
fn current(slot: &LendSlot) -> Result<Lend, LendError> {
    slot.current.get().ok_or(LendError::Absent)
}

/// Runs `operation` on the runtime data lent through `slot`.
pub(crate) fn with_data<R>(
    slot: &LendSlot,
    operation: impl FnOnce(&mut RuntimeData) -> R,
) -> Result<R, LendError> {
    let lend = current(slot)?;
    let _busy = Busy::take(slot, DATA)?;
    // SAFETY: the pointer comes from the `&mut RuntimeData` that `lend` holds for the life of
    // the guard whose lend is current; the DATA bit, held until this access ends, keeps any
    // other `&mut` to it from existing meanwhile.
    Ok(operation(unsafe { &mut *lend.data.as_ptr() }))
}

/// Runs `operation` on the engine state lent through `slot`.
pub(crate) fn with_state<R>(
    slot: &LendSlot,
    operation: impl FnOnce(&mut ConversionState) -> R,
) -> Result<R, LendError> {
    let state = current(slot)?.state.ok_or(LendError::Absent)?;
    let _busy = Busy::take(slot, STATE)?;
    // SAFETY: as in `with_data`, for the `&mut ConversionState` of the lend and the STATE bit.
    Ok(operation(unsafe { &mut *state.as_ptr() }))
}

/// Whether the lend through `slot` carries the engine state.
pub(crate) fn has_state(slot: &LendSlot) -> bool {
    slot.current.get().is_some_and(|lend| lend.state.is_some())
}

/// Runs `operation` on the instance being built under the lend through `slot`, when
/// `reference` is it.
pub(crate) fn with_build<R>(
    slot: &LendSlot,
    reference: InstanceRef,
    operation: impl FnOnce(&mut PatternInstance) -> R,
) -> Result<Option<R>, LendError> {
    match current(slot)?.build {
        Some((building, instance)) if building == reference => {
            let _busy = Busy::take(slot, BUILD)?;
            // SAFETY: as in `with_data`, for the `&mut PatternInstance` of the lend and the
            // BUILD bit.
            Ok(Some(operation(unsafe { &mut *instance.as_ptr() })))
        }
        _ => Ok(None),
    }
}
