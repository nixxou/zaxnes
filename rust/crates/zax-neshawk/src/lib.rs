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

#![forbid(unsafe_code)]

pub mod cpu;

pub use cpu::{Mos6502x, Mos6502xLink};
