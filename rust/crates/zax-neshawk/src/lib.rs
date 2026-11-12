//! **NesHawk, ported to Rust.** BizHawk's NES core, faithfully — its structure, its per-cycle
//! loop, its state variables and their names, and its quirks.
//!
//! The reference is [BizHawk](https://github.com/TASEmulators/BizHawk) at tag **2.11.1**, which
//! is MIT: `LICENSE-BizHawk` carries its copyright notice and licence text, and every module
//! here names the C# file it comes from. The crate README says what the port is for.
//!
//! The point of the crate is that a `.bk2` movie recorded on NesHawk should play back in Zaxnes
//! frame for frame, which Zaxnes's own core cannot promise; `docs/review/NESHAWK-REWRITE.md`
//! reports what it was verified against.
//!
//! # The shape of the port
//!
//! In the C# the `NES` class, the `PPU`, the `APU` and the board all hold references to each
//! other. Rust cannot do that, so every member that reaches the machine is `impl Nes` — in the
//! module of the component it belongs to, carrying the C# method's name and body. The components
//! themselves ([`cpu::Mos6502x`], [`ppu::Ppu`], [`apu::Apu`], [`boards::Board`]) hold only state
//! and the members that need nothing else. Three structural deviations exist and each is
//! documented where it is: [`cpu::Mos6502xLink::set_address_bus`], [`boards::BoardEnv`], and the
//! lifting of the CPU out of [`nes::Nes`] for the duration of one `execute_one`.
//!
//! # Not ported
//!
//! VS. System, the FDS, NSF and UNIF images, the code-data logger, the memory domains, the cheat
//! engine, the light gun, `Palettes.cs`, and every board outside [`boards`]'s list.

#![forbid(unsafe_code)]
// ---------------------------------------------------------------- the port's own idioms
//
// Every allow below is a consequence of transcribing C# line by line, and each would be
// "fixed" by writing something that no longer reads like `NES.Core.cs`. They are stated here
// once rather than sprinkled over fifteen files.
//
// NesHawk's state is `int` almost throughout — addresses, counters, bank numbers, pixel values —
// and the ports to `u8`, `u16` and `usize` are the C#'s own `(byte)`/`(ushort)` casts, which
// truncate. Spelling each as `try_from(..).unwrap_or(..)` would be noise *and* a behaviour
// change.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss
)]
// `PPUREGS._fv`, `_vt`, `_v`, `_h`, `_ht` are the C#'s names for the *latched* copies of the
// scroll counters. The underscore is part of the hardware's name, not a "this is unused" marker,
// and renaming them would make `install_latches` unreadable beside the reference.
#![allow(clippy::used_underscore_binding, clippy::pub_underscore_fields)]
// `TickPPU_active`, `RunCpuOne` and `RunOneLast` are long because the reference is long, and
// their `if`/`else if` chains repeat arms because the reference's do (the pre-render path and the
// visible path share the `xp == 7` block verbatim, for instance). Splitting them would move
// statements across cycle boundaries, which is the one thing this port must not do.
#![allow(
    clippy::too_many_lines,
    clippy::match_same_arms,
    clippy::if_same_then_else,
    clippy::if_not_else,
    clippy::collapsible_if,
    clippy::needless_late_init,
    clippy::similar_names,
    clippy::many_single_char_names
)]
// `Ppu`'s and `Apu`'s flags are one field each in the C# and are driven independently; a bitfield
// would claim they vary together. `soam_index`/`soam_index_aux`/`soam_index_prev` are likewise
// three separate counters of the reference's.
#![allow(clippy::struct_excessive_bools, clippy::struct_field_names)]
// `(x >> n) & 1 != 0` is `BizHawk.Common.NumberExtensions.Bit`; clippy's preferred
// `x & 0x80 == 0x80` is the same test written so that it no longer looks like the reference.
#![allow(clippy::verbose_bit_mask)]
// Doc comments quote C# identifiers in prose (`PPUON`, `TickPPU_VBL`, `NesCartDB`); back-ticking
// every occurrence of every one would double the comment volume.
#![allow(clippy::doc_markdown, clippy::module_name_repetitions)]
// This is a port of a core, not a library whose surface is designed here: nearly every getter is
// a `#[must_use]` candidate and nearly every field is public because the harness and the engine
// read the machine's state directly, as BizHawk's own debugger does.
#![allow(clippy::must_use_candidate)]

pub mod apu;
pub mod boards;
pub mod cart;
pub mod controllers;
pub mod cpu;
pub mod gamedb;
pub mod nes;
pub mod ppu;

pub use cpu::{Mos6502x, Mos6502xLink};
pub use nes::{BusEvent, BusEventKind, BusLog, DisplayType, LoadError, LoadReport, Nes};
