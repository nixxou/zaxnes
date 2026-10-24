//! The lend of the runtime's data, the engine state and the pattern instance being built to
//! the calls into Lua, as raw pointers (Rust-only performance work, owner decision).
//!
//! The host lends them for the length of one call into Lua; every access from the script API
//! goes through [`with_data`], [`with_state`] or [`with_build`], which turn the pointer back into
//! a `&mut` for the length of a closure. Before this module the lend went through `mlua` scoped
//! userdata, whose per-access type check and borrow bookkeeping cost more than most getters.
//!
//! Soundness rests on three rules, each enforced here:
//! - a pointer is only stored by [`lend`] (or [`lend_build`]), which takes the `&mut` it comes
//!   from for the life of the returned guard, so the owner cannot touch the object while it is
//!   lent, and the guard takes the pointer back when it drops (a panic included);
//! - at most one `&mut` per lent object exists at a time: a busy flag per object makes a second,
//!   nested access fail with the error `mlua` gave for the same conflict;
//! - a lend nested in another one saves and restores the outer lend and its flags.
//!
//! The lend lives in the Lua state's application data ([`LendSlot`]), so two runtimes never see
//! each other's objects.

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

/// The lend of one Lua state, installed once with the binding.
#[derive(Default)]
pub(crate) struct LendSlot {
    current: Cell<Option<Lend>>,
    busy: Cell<u8>,
}

/// Why a lent object could not be reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LendError {
    /// Nothing is lent: the call was not made under a lend.
    NotLent,
    /// The object is not part of this lend (no engine state).
    Absent,
    /// The object is already borrowed by an access still running.
    Busy,
}

/// Installs the lend slot of `lua`. It is never removed nor replaced, which [`slot`] relies on.
pub(crate) fn install(lua: &Lua) {
    lua.set_app_data(LendSlot::default());
}

/// The lend slot of `lua`, without keeping the application data borrowed: the closure an access
/// runs may itself use the application data of the state.
fn slot(lua: &Lua) -> Result<&LendSlot, LendError> {
    let pointer = lua
        .app_data_ref::<LendSlot>()
        .map(|slot| NonNull::from(&*slot))
        .ok_or(LendError::NotLent)?;
    // SAFETY: the slot is a boxed application datum that `install` puts in once and nothing
    // removes or replaces, so it lives, at the same address, as long as the Lua state, which
    // outlives the borrow of `lua`. It is only used through its `Cell`s, on this thread.
    Ok(unsafe { pointer.as_ref() })
}

/// Restores the lend that was in force before [`lend`] when it drops.
pub(crate) struct LendGuard<'a> {
    lua: &'a Lua,
    previous: Option<Lend>,
    previous_busy: u8,
    _lent: PhantomData<&'a mut ()>,
}

impl Drop for LendGuard<'_> {
    fn drop(&mut self) {
        if let Ok(slot) = slot(self.lua) {
            slot.current.set(self.previous);
            slot.busy.set(self.previous_busy);
        }
    }
}

/// Lends `data`, `state` and the instance being built to the calls into `lua` until the guard
/// drops.
pub(crate) fn lend<'a>(
    lua: &'a Lua,
    data: &'a mut RuntimeData,
    state: Option<&'a mut ConversionState>,
    build: Option<(InstanceRef, &'a mut PatternInstance)>,
) -> LendGuard<'a> {
    let lend = Lend {
        data: NonNull::from(data),
        state: state.map(NonNull::from),
        build: build.map(|(reference, instance)| (reference, NonNull::from(instance))),
    };
    let (previous, previous_busy) = slot(lua).map_or((None, 0), |slot| {
        (slot.current.replace(Some(lend)), slot.busy.replace(0))
    });
    LendGuard {
        lua,
        previous,
        previous_busy,
        _lent: PhantomData,
    }
}

/// Restores the instance being built that was in force before [`lend_build`] when it drops.
pub(crate) struct BuildGuard<'a> {
    lua: &'a Lua,
    previous: Option<(InstanceRef, NonNull<PatternInstance>)>,
    _lent: PhantomData<&'a mut ()>,
}

impl Drop for BuildGuard<'_> {
    fn drop(&mut self) {
        if let Ok(slot) = slot(self.lua) {
            if let Some(mut lend) = slot.current.get() {
                lend.build = self.previous;
                slot.current.set(Some(lend));
            }
        }
    }
}

/// Lends `instance` as the instance being built under the current lend of `lua`, until the
/// guard drops.
///
/// # Errors
///
/// [`LendError::NotLent`] when nothing is lent.
pub(crate) fn lend_build<'a>(
    lua: &'a Lua,
    reference: InstanceRef,
    instance: &'a mut PatternInstance,
) -> Result<BuildGuard<'a>, LendError> {
    let slot = slot(lua)?;
    let mut lend = slot.current.get().ok_or(LendError::NotLent)?;
    let previous = lend.build.replace((reference, NonNull::from(instance)));
    slot.current.set(Some(lend));
    Ok(BuildGuard {
        lua,
        previous,
        _lent: PhantomData,
    })
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

/// Runs `operation` on the runtime data lent to `lua`.
pub(crate) fn with_data<R>(
    lua: &Lua,
    operation: impl FnOnce(&mut RuntimeData) -> R,
) -> Result<R, LendError> {
    let slot = slot(lua)?;
    let lend = slot.current.get().ok_or(LendError::NotLent)?;
    let _busy = Busy::take(slot, DATA)?;
    // SAFETY: the pointer comes from the `&mut RuntimeData` that `lend` holds for the life of
    // the guard whose lend is current; the DATA bit, held until this access ends, keeps any
    // other `&mut` to it from existing meanwhile.
    Ok(operation(unsafe { &mut *lend.data.as_ptr() }))
}

/// Runs `operation` on the engine state lent to `lua`.
pub(crate) fn with_state<R>(
    lua: &Lua,
    operation: impl FnOnce(&mut ConversionState) -> R,
) -> Result<R, LendError> {
    let slot = slot(lua)?;
    let state = slot
        .current
        .get()
        .ok_or(LendError::NotLent)?
        .state
        .ok_or(LendError::Absent)?;
    let _busy = Busy::take(slot, STATE)?;
    // SAFETY: as in `with_data`, for the `&mut ConversionState` of the lend and the STATE bit.
    Ok(operation(unsafe { &mut *state.as_ptr() }))
}

/// Whether the current lend of `lua` carries the engine state.
pub(crate) fn has_state(lua: &Lua) -> bool {
    slot(lua)
        .ok()
        .and_then(|slot| slot.current.get())
        .is_some_and(|lend| lend.state.is_some())
}

/// Runs `operation` on the instance being built under the lend of `lua`, when `reference` is it.
pub(crate) fn with_build<R>(
    lua: &Lua,
    reference: InstanceRef,
    operation: impl FnOnce(&mut PatternInstance) -> R,
) -> Result<Option<R>, LendError> {
    let slot = slot(lua)?;
    match slot.current.get().ok_or(LendError::NotLent)?.build {
        Some((building, instance)) if building == reference => {
            let _busy = Busy::take(slot, BUILD)?;
            // SAFETY: as in `with_data`, for the `&mut PatternInstance` of the lend and the
            // BUILD bit.
            Ok(Some(operation(unsafe { &mut *instance.as_ptr() })))
        }
        _ => Ok(None),
    }
}
