# `zax-neshawk` — BizHawk's NesHawk core, ported to Rust

A **faithful port**, not a reimplementation. The reference is
[BizHawk](https://github.com/TASEmulators/BizHawk) at tag **2.11.1**:

* `src/BizHawk.Emulation.Cores/CPUs/MOS 6502X/` — the CPU;
* `src/BizHawk.Emulation.Cores/Consoles/Nintendo/NES/` — the machine, the PPU, the APU, the
  boards and the controller ports.

The port follows NesHawk's structure, its per-cycle loop, its state variables **and their
names**, and its quirks, line by line wherever that is the simplest faithful translation. Where
NesHawk's naming breaks Rust convention — `rdy_freeze`, `alu_temp`, `mi`, `AbsIdx_RMW_Stage6_SLO`
— the NesHawk name wins, because the point of this crate is that a reader can hold it beside the
C# and see the same machine. The lints that object are allowed at module level with that reason
given.

The goal is that this engine agrees with BizHawk's NesHawk frame for frame on real TAS movies,
so that a `.bk2` recorded on NesHawk plays back in Zaxnes. `docs/review/NESHAWK-REWRITE.md`
reports what it was verified against.

## Licence

Zaxnes is GPL-3.0; BizHawk's original work is MIT, which is compatible with it. BizHawk's
copyright notice and licence text are in **`LICENSE-BizHawk`**, as the MIT licence requires, and
every ported file's header names the C# file it comes from.

## Safe Rust only

No `unsafe`. The FFI exception in this workspace belongs to `zax-fceux` and does not extend here.
