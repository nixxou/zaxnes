//! The savestate: NesHawk's `SyncState`, as a serialised block.
//!
//! Ported in spirit from the `SyncState(Serializer ser)` members BizHawk 2.11.1 gives `NES`,
//! `PPU`, `APU`, `MOS6502X` and every board — one member per class, listing that class's fields.
//! MIT; see `LICENSE-BizHawk`.
//!
//! # Why this is not a clone
//!
//! Until this file existed the engine's savestate was `Nes: Clone`, which is exact but is not a
//! *format*: it cannot go to disk, cannot be handed to another process, and cannot be refused
//! when it was written by another engine. [`Engine::try_load_state`] needs all three
//! (REQ-CORE-090, REQ-CORE-091, REQ-CORE-120).
//!
//! # The one structural difference from the C#, and what it buys
//!
//! BizHawk's `Serializer` is a single visitor that both reads and writes, so a class lists its
//! fields once. This port cannot do that in one method: [`crate::nes::Nes`] must be saved from a
//! `&self` ([`Engine::save_state_to`]'s signature) and restored into a `&mut self`, and no single
//! Rust method has both. So [`SyncState`] has two members — and [`sync_struct`] generates **both
//! from one field list**, so the field list is still written once and the two orders cannot drift
//! apart: they are the same token sequence.
//!
//! What that macro adds over the C# is a guarantee the C# has not got. Each generated body starts
//! with an **exhaustive** destructuring of the struct — `let Self { a, b, c } = self`, with no
//! `..` — so a field added to a struct later and not named in its `sync_struct!` list is a
//! **compile error**, not a field silently missing from every savestate. BizHawk has shipped that
//! bug more than once; here it cannot happen. A field deliberately left out is named in the
//! macro's `skip` list, which is part of the same exhaustive pattern, so leaving one out is a
//! decision recorded in code rather than an omission.
//!
//! [`Engine`]: zax_core_api::Engine
//! [`Engine::save_state_to`]: zax_core_api::Engine::save_state_to
//! [`Engine::try_load_state`]: zax_core_api::Engine::try_load_state

/// Writes a machine's state, byte by byte, in the order the [`SyncState`] members visit it.
#[derive(Debug)]
pub struct StateWriter {
    /// The block being built.
    out: Vec<u8>,
    /// Whether the frame buffer ([`crate::ppu::Ppu::xbuf`]) is part of this block; see
    /// [`StateWriter::full`].
    full: bool,
}

impl StateWriter {
    /// A writer appending to `out`, whose earlier content is kept (the caller has written its
    /// header into it).
    ///
    /// `full` asks for the frame buffer as well. It is `false` for the rollback path, which is
    /// what `zax-nes`'s own in-memory snapshot does too — REQ-CORE-121's block "leaves out the
    /// half of the PPU state that *is* the frame just run", because the next frame overwrites it
    /// anyway. It is `true` for a state the user saves, where the picture has to come back at
    /// once rather than on the next frame.
    #[must_use]
    pub fn new(out: Vec<u8>, full: bool) -> Self {
        Self { out, full }
    }

    /// Whether this block carries the frame buffer.
    #[must_use]
    pub const fn full(&self) -> bool {
        self.full
    }

    /// The finished block.
    #[must_use]
    pub fn finish(self) -> Vec<u8> {
        self.out
    }

    /// Bytes written so far, header included.
    #[must_use]
    pub fn len(&self) -> usize {
        self.out.len()
    }

    /// Whether nothing has been written at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.out.is_empty()
    }

    /// Appends raw bytes.
    pub fn raw(&mut self, bytes: &[u8]) {
        self.out.extend_from_slice(bytes);
    }

    /// Overwrites four bytes at `at` with `value`, little-endian — how the caller back-patches a
    /// length it could not know when it wrote the header. Does nothing if `at` is out of range,
    /// which cannot happen for a caller that wrote those four bytes itself.
    pub fn patch_u32(&mut self, at: usize, value: u32) {
        if let Some(slot) = self.out.get_mut(at..at + 4) {
            slot.copy_from_slice(&value.to_le_bytes());
        }
    }
}

/// Reads a machine's state back, in the same order.
///
/// A truncated or corrupt block never panics and never allocates on a bad length: it sets
/// [`StateReader::failed`], and every later read is a no-op that leaves the field as it was. The
/// caller checks `failed` once at the end, which is why a half-restored machine cannot escape —
/// the engine reloads the ROM on a failure, as REQ-CORE-091 requires.
#[derive(Debug)]
pub struct StateReader<'a> {
    data: &'a [u8],
    pos: usize,
    failed: bool,
    full: bool,
}

impl<'a> StateReader<'a> {
    /// A reader over `data`, positioned at its start. `full` must be what the block's header
    /// says the writer's `full` was.
    #[must_use]
    pub const fn new(data: &'a [u8], full: bool) -> Self {
        Self {
            data,
            pos: 0,
            failed: false,
            full,
        }
    }

    /// Whether the block carries the frame buffer.
    #[must_use]
    pub const fn full(&self) -> bool {
        self.full
    }

    /// Whether any read ran past the end of the block, or a length was impossible.
    #[must_use]
    pub const fn failed(&self) -> bool {
        self.failed
    }

    /// Bytes consumed so far.
    #[must_use]
    pub const fn position(&self) -> usize {
        self.pos
    }

    /// Bytes left unread.
    #[must_use]
    pub const fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    /// Marks the block bad, for a caller that found an inconsistency of its own (a board that is
    /// not the one the block was taken on, say).
    pub const fn fail(&mut self) {
        self.failed = true;
    }

    /// The next `n` bytes, or `None` — and the reader marked bad — when there are not that many.
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        if self.failed {
            return None;
        }
        if let Some(slice) = self.data.get(self.pos..self.pos.saturating_add(n)) {
            self.pos += n;
            return Some(slice);
        }
        self.failed = true;
        None
    }
}

/// One class's state, as its `SyncState` member lists it.
///
/// Both members visit exactly the same fields in exactly the same order, because
/// [`sync_struct`] writes both from one list.
pub trait SyncState {
    /// Appends this value's state to `w`.
    fn write_state(&self, w: &mut StateWriter);
    /// Reads this value's state back from `r`, leaving it unchanged where `r` has failed.
    fn read_state(&mut self, r: &mut StateReader<'_>);
}

// ------------------------------------------------------------------- the primitives

/// `impl SyncState` for an integer, as its little-endian bytes.
macro_rules! int_state {
    ($($ty:ty),*) => { $(
        impl SyncState for $ty {
            fn write_state(&self, w: &mut StateWriter) {
                w.raw(&self.to_le_bytes());
            }
            fn read_state(&mut self, r: &mut StateReader<'_>) {
                const N: usize = size_of::<$ty>();
                if let Some(bytes) = r.take(N) {
                    let mut buf = [0u8; N];
                    buf.copy_from_slice(bytes);
                    *self = <$ty>::from_le_bytes(buf);
                }
            }
        }
    )* };
}

int_state!(u8, u16, u32, u64, i8, i16, i32, i64);

impl SyncState for bool {
    fn write_state(&self, w: &mut StateWriter) {
        w.raw(&[u8::from(*self)]);
    }
    fn read_state(&mut self, r: &mut StateReader<'_>) {
        if let Some(bytes) = r.take(1) {
            *self = bytes[0] != 0;
        }
    }
}

impl SyncState for usize {
    /// As a `u64`, so that a block does not depend on the host's pointer width.
    fn write_state(&self, w: &mut StateWriter) {
        (*self as u64).write_state(w);
    }
    fn read_state(&mut self, r: &mut StateReader<'_>) {
        let mut wide = *self as u64;
        wide.read_state(r);
        match usize::try_from(wide) {
            Ok(value) => *self = value,
            Err(_) => r.fail(),
        }
    }
}

impl<T: SyncState, const N: usize> SyncState for [T; N] {
    fn write_state(&self, w: &mut StateWriter) {
        for item in self {
            item.write_state(w);
        }
    }
    fn read_state(&mut self, r: &mut StateReader<'_>) {
        for item in self.iter_mut() {
            item.read_state(r);
        }
    }
}

impl<A: SyncState, B: SyncState> SyncState for (A, B) {
    fn write_state(&self, w: &mut StateWriter) {
        self.0.write_state(w);
        self.1.write_state(w);
    }
    fn read_state(&mut self, r: &mut StateReader<'_>) {
        self.0.read_state(r);
        self.1.read_state(r);
    }
}

impl<T: SyncState + Default> SyncState for Vec<T> {
    /// Length-prefixed, and the length is checked against what is left of the block before
    /// anything is allocated: a corrupt `u32` must not become a four-gigabyte `Vec`.
    fn write_state(&self, w: &mut StateWriter) {
        (self.len() as u32).write_state(w);
        for item in self {
            item.write_state(w);
        }
    }
    fn read_state(&mut self, r: &mut StateReader<'_>) {
        let mut len = 0u32;
        len.read_state(r);
        if r.failed() {
            return;
        }
        let Ok(len) = usize::try_from(len) else {
            r.fail();
            return;
        };
        // One byte is the smallest any `T` can take, so a length beyond what is left of the block
        // is impossible whatever `T` is.
        if len > r.remaining() {
            r.fail();
            return;
        }
        self.clear();
        self.reserve(len);
        for _ in 0..len {
            let mut item = T::default();
            item.read_state(r);
            self.push(item);
        }
    }
}

impl SyncState for String {
    fn write_state(&self, w: &mut StateWriter) {
        (self.len() as u32).write_state(w);
        w.raw(self.as_bytes());
    }
    fn read_state(&mut self, r: &mut StateReader<'_>) {
        let mut len = 0u32;
        len.read_state(r);
        if r.failed() {
            return;
        }
        let Ok(len) = usize::try_from(len) else {
            r.fail();
            return;
        };
        match r.take(len).map(|bytes| std::str::from_utf8(bytes).ok()) {
            Some(Some(text)) => {
                self.clear();
                self.push_str(text);
            }
            Some(None) => r.fail(),
            None => {}
        }
    }
}

// ---------------------------------------------------------------------- the macros

/// `impl SyncState` for a struct, from one list of its fields.
///
/// Generates both members of the trait from the same token sequence, so the write order and the
/// read order are the same by construction. The list is **exhaustive**: every field of the struct
/// must appear either in the first group, which is saved, or after `skip:`, which is not — and a
/// field in neither is a compile error (`pattern does not mention field`). See this module's
/// header for why that guarantee is the point of the macro.
///
/// `skip:` is for a field that is not machine state: a `&'static` lookup table, the ROM image, a
/// load-time constant the caller restores from the cartridge, or a tracing instrument. Every
/// `skip:` group in this crate carries a comment saying which.
/// `inherent` generates `write_sync_fields`/`read_sync_fields` **inherent** methods instead of
/// the trait impl, for the one struct ([`crate::ppu::Ppu`]) that has a field the generated code
/// cannot handle: its `SyncState` impl is written by hand and calls these for everything else, so
/// the exhaustive check still covers every other field.
macro_rules! sync_struct {
    ($ty:ty { $($field:ident),* $(,)? }) => {
        $crate::state::sync_struct!($ty { $($field),* } skip: { });
    };
    ($ty:ty { $($field:ident),* $(,)? } skip: { $($skipped:ident),* $(,)? }) => {
        impl $crate::state::SyncState for $ty {
            fn write_state(&self, __w: &mut $crate::state::StateWriter) {
                // Exhaustive on purpose: a new field named in neither group will not compile.
                let Self { $($field,)* $($skipped: _,)* } = self;
                $( $crate::state::SyncState::write_state($field, __w); )*
            }
            fn read_state(&mut self, __r: &mut $crate::state::StateReader<'_>) {
                let Self { $($field,)* $($skipped: _,)* } = self;
                $( $crate::state::SyncState::read_state($field, __r); )*
            }
        }
    };
    (inherent $ty:ty { $($field:ident),* $(,)? } skip: { $($skipped:ident),* $(,)? }) => {
        impl $ty {
            fn write_sync_fields(&self, __w: &mut $crate::state::StateWriter) {
                // Exhaustive on purpose: a new field named in neither group will not compile.
                let Self { $($field,)* $($skipped: _,)* } = self;
                $( $crate::state::SyncState::write_state($field, __w); )*
            }
            fn read_sync_fields(&mut self, __r: &mut $crate::state::StateReader<'_>) {
                let Self { $($field,)* $($skipped: _,)* } = self;
                $( $crate::state::SyncState::read_state($field, __r); )*
            }
        }
    };
}

/// `impl SyncState` for a field-less enum, as a one-byte index its caller spells out.
///
/// The indices are written in the macro rather than taken from the variants' order because they
/// are a **persisted format**: reordering the variants of an enum must not silently change what
/// an old block means. The `match self` arms are still exhaustive, so a variant added later and
/// not given an index is a compile error.
///
/// An index naming no variant marks the block bad rather than picking a variant, which is what
/// makes a block from a different build a refusal instead of a wrong machine.
macro_rules! sync_enum {
    ($ty:ty { $($index:literal => $variant:path),* $(,)? }) => {
        impl $crate::state::SyncState for $ty {
            fn write_state(&self, __w: &mut $crate::state::StateWriter) {
                let index: u8 = match self { $( $variant => $index, )* };
                $crate::state::SyncState::write_state(&index, __w);
            }
            fn read_state(&mut self, __r: &mut $crate::state::StateReader<'_>) {
                let mut index = 0u8;
                $crate::state::SyncState::read_state(&mut index, __r);
                if __r.failed() {
                    return;
                }
                match index {
                    $( $index => *self = $variant, )*
                    _ => __r.fail(),
                }
            }
        }
    };
}

pub(crate) use {sync_enum, sync_struct};
