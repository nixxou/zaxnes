# NESHAWK-PROFILE-9 — the reset lands on the frame's PPU dot (item 12)

Branch `feat/neshawk-profile`, worktree `cleanroom/wt-bk2`, base `f1c0faa8` (lot NESHAWK-PROFILE-8).

## Steps

- [x] 1. **Make the reset land where NesHawk's does.** Item 12: a frame the movie taps RESET after
      ends at the boundary dot. The four marks' residual is **−1 on every frame 2 to 9**, before
      the reset and after it, where it was −1 then +2.
- [ ] 2. Item 1c on, the strobe counts of frames 40 and 55, *Kirby* on every window, the tests.
- [ ] 3. Rounds, to frame 156 312 or five of them.
- [ ] 4. No regression, report, final pass.

## The environment this lot runs in

| | |
|---|---|
| `CARGO_TARGET_DIR` | `<worktree>/rust/target-ev` |
| `TMP` / `TEMP` | `<worktree>/rust/target-ev/tmp` |
| `CARGO_INCREMENTAL` | `0` |
| disk at the start | `G:` 7.8 GB free (stop at 5), `C:` 9.1 GB (stop at 1) |

## The sources read, and their licences

| | |
|---|---|
| **BizHawk** (NesHawk), MIT, tag 2.11.1 | `NES/NES.Core.cs` `FrameAdvance` and `do_single_step`; `NES/PPU.run.cs` `TickPPU_preVBL` and `NewDeadPPU`; `NES/PPU.cs` `Reset`, `NESSoftReset`; `NES/PPU.regs.cs` `regs_reset`; `CPUs/MOS 6502X/MOS6502X.cs` `NESSoftReset` |
| **BizHawk run as a program** | nothing new: lot 8's `q183.lua` output (`bizhawk-runs/q183.txt`) is the reference this step is held against |

---

## Step 1 — item 12, the frame before a reset ends at the boundary dot

### The answer first

1. **Where NesHawk ends a frame, read off the source.** `PPU.run.cs` `TickPPU_preVBL()` sets
   `nes.frame_is_done = true` at the dot `ppur.status.sl` reaches `241 + preNMIlines`, and the CPU
   is clocked from inside that same per-dot loop — so the frame ends **mid-instruction** as a rule.
   `NES.Core.cs` `FrameAdvance` then opens the next frame with `if (resetSignal) {
   Board.NesSoftReset(); cpu.NESSoftReset(); apu.NESSoftReset(); ppu.NESSoftReset(); }` before any
   emulation, and `resetSignal = controller.IsPressed("Reset")` is read at the **end** of the
   frame, which is the one-frame delay the `.bk2` player already models
   (`MovieFile::reset_after_frame`, `relocate_resets`).
2. **And the instruction in flight is abandoned**: `MOS 6502X`'s `NESSoftReset()` is `opcode =
   VOP_RESET; mi = 0; iflag_pending = true; FlagI = true;` — whatever cycles of it had run are
   spent, the rest never run. That is the whole of the +2: this core's frame loop breaks *after*
   the straddling instruction, so its reset is 3 cycles late.
3. **The mechanism, and it is not a constant.** `TimingProfile::RESET_LANDS_ON_THE_FRAME_BOUNDARY_DOT`,
   read in `NesCoreOf::run_frame_profiled_gen`'s loop through
   `NesBusOf::frame_boundary_is_the_next_cycle`: before each instruction, if the boundary dot falls
   inside the **next** CPU cycle, the frame ends there. `BEFORE_RESET` is a **const** parameter, so
   neither profile's ordinary frame carries the test at all — `run_frame_profiled` instantiates it
   `false` and `NesCoreOf::run_frame_before_soft_reset` `true`.
4. **It lands on NesHawk's own cycle.** Our frame 4 now ends at 116 734 (its clock, which reads one
   less than BizHawk's) and the seven reset cycles take it to 116 741, against NesHawk's last
   cycle 116 734 and reset cycles 116 735 to 116 741.

### The four marks, before and after

`frame-trace --frame 1 --frames 56 --profile neshawk --event-log --event-pc C3A1 --event-pc C06E
--event-pc C074` against `bizhawk-runs/q183.txt` (lot 8's), `zaxnes − bizhawk` on the absolute
cycle, BizHawk frame *f* against this core's *f+1*:

| BizHawk frame | 2 | 3 | *(the reset)* | 6 | 7 | 8 | 9 | 10 | 11 |
|---|---|---|---|---|---|---|---|---|---|
| lot 8 (all four marks) | −1 | −1 | | −1 | **+2** | **+2** | **+2** | +2 | +2 |
| this lot | −1 | −1 | | −1 | **−1** | **−1** | **−1** | −1/−2 | −3 |

The gate the brief set — the residual after the reset equal to the residual before it — is met on
every mark of frames 6 to 9. From frame 10 it moves again, and that is the **other** half: frame 10
is the first frame with any DMA at all, and item 1c is still off. That is step 2.

### What the frame's length does

| | before | after |
|---|---|---|
| frame 4 (the one before the reset) | 29 781 cycles | **29 778** |
| the reset's own 7 cycles | 116 737 … 116 743 | **116 734 … 116 740** |
| frame 5 (the start-up frame) | 27 387 | 27 387, starting at **116 741** |

### The one thing the mechanism does not do

The loop can only end a frame **between** instructions. When the boundary dot falls inside an
instruction rather than on its first cycle, NesHawk spends that instruction's first cycles and
abandons the rest, and this core ends the frame before it — so the reset would be up to (that
instruction's length − 1) cycles early. The movie's one reset is not that case (`$C070`'s first
cycle *is* the dot's), and the honest fix for the general one is a cycle-stepped CPU rather than a
constant. The constant's own note says so.

### Where it is read, and what else moved

| | |
|---|---|
| `crates/zax-nes/src/timing_profile.rs` | the constant, with the four functions it is read from |
| `crates/zax-nes/src/ppu/mod.rs` | `frame_boundary_is_the_next_cycle` (the dot arithmetic, exact on both regions) and `capture_frame_snapshot` — a frame that ends before the dot has not had the dot's own palette and mask copy taken, and the 2D picture is composed from it |
| `crates/zax-nes/src/nes_bus.rs` | `frame_boundary_is_the_next_cycle`, which catches the PPU up first |
| `crates/zax-nes/src/nes_core.rs` | `run_frame_profiled_gen<const BEFORE_RESET>`, `run_frame_before_soft_reset` |
| `crates/zax-harness/src/movie_sync.rs` | `Machine::run_before_soft_reset` (defaulted to `run`, so FCEUX and `Standard` are untouched), `resets_next`, `step`'s and the probe's use of it |
| `crates/zax-harness/src/frame_trace.rs` | the same break in `trace_frame` — the instrument that writes the marks — and in `play` |

### The test

`item_12_the_frame_before_a_reset_ends_at_the_boundary_dot`, differential: the same state runs the
ordinary frame and the before-reset frame on both profiles. The minimal ROM's `JMP $8000` loop is 3
cycles, so the dot walks through all three over consecutive frames:

| after *n* frames | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 |
|---|---|---|---|---|---|---|---|---|
| gap, `Standard` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| gap, `NesHawkCompat` | 0 | **3** | **3** | **3** | 0 | 0 | 0 | 0 |

A gap of 0 is a dot that falls inside the `JMP` rather than on its first cycle; the test asserts
that the before-reset frame has passed the boundary dot **exactly when** the gap is 0, which is
what says the early break is the thing being measured, and that at least one frame of the eight
takes it.

### A trap this step met

`(the dots to the boundary) <= (the dots of one cycle)` is not the predicate: **zero** dots away
means the dot was the one just processed, so the boundary has already been taken and it is the
*next* one, a whole frame off, that this frame ends on. That state is reachable at the start of a
frame whose predecessor's last cycle ended exactly on the dot, and reading it as "now" made such a
frame end after **no instructions at all** (`pair::<NesHawkCompat>(5)` in the test, which caught
it). The predicate is `1..=`.

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE-9 step 1 done: item 12, the reset lands on the boundary dot, the four marks −1 across it
