# NESHAWK-PROFILE-9 — the reset lands on the frame's PPU dot (item 12)

Branch `feat/neshawk-profile`, worktree `cleanroom/wt-bk2`, base `f1c0faa8` (lot NESHAWK-PROFILE-8).

## Steps

- [x] 1. **Make the reset land where NesHawk's does.** Item 12: a frame the movie taps RESET after
      ends at the boundary dot. The four marks' residual is **−1 on every frame 2 to 9**, before
      the reset and after it, where it was −1 then +2.
- [x] 2. **Item 1c on**, and the two items together are **not** the shipped pair's 45 095: frame
      40's collision is closed, frame 55's is not, and the movie parts at **9 353**. The cycle
      residual is nevertheless −1 three frames further than before, and the first difference has
      moved from frame 55 to tree frame **13**.
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

---

## Step 2 — item 1c on, and what the two items are worth together

### The answer first

1. **Item 1c is on** (`DMC_HALT_LANDS_ON_A_WRITE_CYCLE = true`), its gate closed by step 1. The
   test profile that carried it while it was off is gone, as item 11's was; `NesHawkCompat` is
   measured directly, `[4, 5, 4, 5]` against `Standard`'s `[5, 4, 5, 4]`.
2. **The two items together are not the shipped pair.** The movie parts at **9 353** where the two
   wrongs that cancelled each other reached **45 095** — which is what lot 8 said would happen and
   is written down as a trap in `docs/review/NESHAWK-PROFILE.md`: *"when two errors cancel, fixing
   one makes things worse"*. Both halves are measured right on their own, so neither goes back.
3. **What is better is the clock, and that is the quantity the rounds work on.** The four marks
   hold their −1 to BizHawk frame **12** where the shipped pair left it at frame 10 and item 12
   alone leaves it at 10, and frame 40's strobe collision is **gone**. What is worse is the strobe
   table's own count, 28 conflicted frames of 1 098 and **all 28 disagreeing** against lot 7's
   23 of 30.

### The movie, every window

`movie-sync --engine zaxnes --profile neshawk --no-lag --frames <end> --ram-ref <window>`; the
figure is the last frame with no live difference. The first two columns are lot 8's.

| window | frames | shipped (12 off, 1c off) | 1c on alone (lot 8) | **12 alone** | **12 + 1c** |
|---|---|---|---|---|---|
| `step1` | 1 - 400 | 400 | 400 | 400 | 400 (288 identical to the byte) |
| `step1b` | 400 - 1 100 | 1 100 | parted at 925 | 1 100 (157 identical) | **1 100** (363 identical) |
| `step1c` | 1 100 - 10 000 | 10 000 | parted at 1 100 | parted at **4 665** | parted at **9 353** |
| `step1d` | 10 000 - 30 000 | 30 000 | — | none | — |
| `step1e` | 30 000 - 60 000 | **45 095** | — | — | — |

Item 12 **alone** is the worst of the three (4 665): it moves the whole machine 3 cycles and
nothing else, so every fetch whose halt NesHawk charges three cycles and this core four is now
charged against the right CPU position and still wrongly. The pair is the configuration where both
measured rules hold; the residual is a third difference, not a reason to retract either.

### The controller strobes, frames 3 to 1 100

`frame-trace --frame 2 --frames 1099 --profile neshawk --dmc-census --event-log` against
`bizhawk-runs/q182g.txt` (lot 7's), `w $4016` counted per frame, this core's tree frame *f* against
BizHawk's *f−1*:

| | shipped pair (lot 7) | **12 + 1c** |
|---|---|---|
| conflicted frames (either side above 8 strobes) | 30 | **28** |
| of which disagreeing | 23 | **28** |
| tree frame 40 | 8 against 10 | **8 against 8** |
| tree frame 55 | 8 against 10 | 8 against 10 |
| the first disagreeing frame | — | tree frame **16** (12 against 8: **this** core takes two extra passes) |

### Where the residual is now, and it is not frame 55

The four marks, `zaxnes − bizhawk`, with both items on. Frames 2 to 12 are a constant **−1** — the
reset included — and the first mark that leaves it is BizHawk frame 12's `RTI` at **+1**, inside
this core's tree frame 13:

| BizHawk frame | 2 - 9 | 10 | 11 | 12 | 13 | 14 | 15 | 16 |
|---|---|---|---|---|---|---|---|---|
| 12 only | −1 | −1/−2 | −3 | +2/+3 | −6/−4/19 | 0/1 | −2/1 | −2/0/3 |
| **12 + 1c** | **−1** | −1/−2 | **−1** | **−1/+1** | −6/−4/3 | 0/−3 | **0/+317** | −1/+2 |

The **+317** of BizHawk frame 15 is this core doing a whole extra pass of the input routine and
more, which the strobe table sees as tree frame 16's 12 strobes against 8. So the movie's first
difference has moved from frame 55 (lot 7) to tree frame **13**, with its first *visible* symptom
at tree frame 16, and that is round 1's window.

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE-9 step 2 done: item 1c on, the pair parts at 9 353, the first difference is tree frame 13
