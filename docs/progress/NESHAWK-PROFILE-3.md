# NESHAWK-PROFILE-3 — where the thirty cycles of frame 9354 come from

Branch `feat/neshawk-profile`, worktree `cleanroom/wt-bk2`, base `7c0b88b1` (lot NESHAWK-PROFILE-2).

Lot NESHAWK-PROFILE-2 took the *Kirby* `.bk2` to frame **9 353** on the `NesHawkCompat` profile and
then stopped, with a diagnosis and a negative: in frame 9 354 the game's music driver runs its
`STA $0641,X` for `$0643`/`$0644` about **thirty CPU cycles earlier** on Zaxnes than on BizHawk, and
**nothing left in NesHawk's DMC, DMA or APU-register paths supplies thirty cycles on that frame**.
Its §4g therefore sent the next lot somewhere else: the APU frame sequencer and `$4017`, the `$4016`
strobe delay, the NMI/IRQ polling points, `$2002`/vblank timing, the odd-frame skip, and any 6502
instruction whose cycle count or dummy accesses differ in NesHawk's core.

This lot does not guess among them: step 1 **measures the cycle of the same events on both
machines** and bisects back to the first one that differs.

## Steps

- [x] 1. Measure where the cycles come from, on both machines.
- [ ] 2. Explain the first differing interval.
- [ ] 3. Add it to the profile, and iterate 1-3 up to five rounds.
- [ ] 4. No regression.
- [ ] 5. Report and final pass.

## The environment this lot runs in

| | |
|---|---|
| `CARGO_TARGET_DIR` | `<worktree>/rust/target-ev` |
| `TMP` / `TEMP` | `<worktree>/rust/target-ev/tmp` (Windows path form) |
| `CARGO_INCREMENTAL` | `0` |
| `ZAXNES_ROMS` | `G:/3dsen/demo/roms` |
| `ZAXNES_TEST_ROMS` | `G:/3dsen/cleanroom/assets-free/test-roms` |
| `ZAXNES_PROFILES` | `G:/3dsen/cleanroom/assets-neutral` |
| disk on `G:` at the start | 8.8 GB free (the brief's stop is 5 GB) |
| disk on `C:` at the start | 8.9 GB free (the brief's stop is 1 GB) |

## The sources read, and their licences

| | |
|---|---|
| **BizHawk** (NesHawk) | MIT. Read through `raw.githubusercontent.com`; nothing cloned or downloaded. The files are named per item below. |
| **Mesen2** | GPL-3.0, for contrast only. |
| **nesdev wiki** | the pages named per item. |

---


## Step 1 — the cycle ledger, and the first event whose cycle differs

### The answer first

The first event whose cycle differs between the two machines is the **NMI handler's own entry**,
and the difference is **exactly one instruction of the game's frame-sync spin loop — three CPU
cycles — with NesHawk entering it later.** On the 1 451 frames of 1 100-10 000 whose *previous*
frame's work ended at exactly the same CPU cycle on both machines — so that the two are in step
when the vblank arrives — the handler is entered at the same cycle on **1 085** and three cycles
later on NesHawk on **366**. Nothing else in a frame differs when the NMI entry does not: the OAM
transfer request, the four `$4016` strobe pairs, the `$4017` write, the `$4015` read and the
`$4011` write all land on the same cycle, frame after frame.

And the brief's first question — whether a *whole-frame* cycle total ever differs — is answered
**no, never**:

| | |
|---|---|
| accumulated CPU cycles, frame 1 100 to frame 10 000, BizHawk | 265 046 497 |
| the same, Zaxnes | 265 046 494 — the 3 are the two ends' own quantisation (see below), not a drift |
| `zaxnes.start - bizhawk.start` over the 8 901 frames | stays inside **4 773 ... 4 783** and comes back to 4 776, never walks |
| BizHawk's own frame lengths over the span | exactly **29 780** (4 403 frames) and **29 781** (4 497) |
| all 8 901 lag counts | agree (as NESHAWK-PROFILE-2 already had) |

So the two PPUs run in lockstep and no frame is ever a different length. Whatever the machines
differ by, they differ by it *inside* a frame.

### The two instruments, and the one calibration they needed

| | |
|---|---|
| `zax-harness frame-trace --event-log <file> [--event-pc <addr>]...` | the **cycle ledger**: one `F` line per frame with the CPU cycle it started on, its length, its instruction count and its DMA bill, then one line per event with the cycle it happened on *counted from the frame's start* — every NMI and IRQ handler entry, every `--event-pc`, and (with `--dmc-census`) every `$40xx` access. It captures no instruction and disassembles nothing, so 8 901 frames cost **19 s**; `--out` and `--watch` are refused with it rather than served empty |
| `G:/3DSen/ext-tools/scripts/bk2cyc1.lua` | the same file, written by BizHawk. `emu.totalexecutedcycles()` at the frame's start and inside every callback; `event.onmemoryexecute` for the two interrupt vectors (read from `$FFFA`/`$FFFE`) and for each watched PC; `event.onmemorywrite` for each of `$4000`-`$4017`; `event.onmemoryread` for `$4015` and `$2002`. 8 901 frames cost **66 s** and 5.4 MB |
| `cyccheck.lua` | validation only, and run **first**: it prints the memory-domain list, every register name `emu.getregisters()` answers, and the two vectors, and exercises all three callback kinds over four frames. That is how the domain (`"System Bus"`), the register names (`A`/`X`/`Y`/`S`, and **no** combined `P`) and the fact that a callback's `emu.totalexecutedcycles()` is the cycle of the access were established rather than assumed |

Two things were added to the core's census so that the ledger could carry them: a `$4016` write is
now a census event of its own (`DmaEventKind::PortStrobe` — it is not an APU register, but NesHawk
applies a strobe on the DMC's own timer, so a census that places fetches against the cycle has to
place it beside them), and `FrameTrace` carries the frame's instruction count and its DMC bill as
fields rather than deriving the first from the step vector.

**The calibration, measured and not assumed.** The two instruments report an *execute* at the
cycle the instruction begins on and a *read* or a *write* **one cycle later**. That is the whole
of the difference between them, and it was measured rather than guessed: with the +1 applied,
`r$4015 -> w$4011` comes out at exactly 0 on **75 of 75** occurrences in frames 1 100-1 200 and
`w$400C -> r$4015` on 56 of 57 — which no wrong calibration could do. (The first pass used a wrong
value, +2 for a read, and it showed up at once as a `-1` on 563 of 1 163 `r$4015` rows against a
clean 0 everywhere else; the rule is that a calibration which is not uniform is not a
calibration.)

### What is comparable, and what is not

A frame's own `start` is **not** comparable: BizHawk ends a frame on a PPU condition and the
tracer notices the boundary only after the instruction that crossed it, so the two differ by 0 to
6 cycles by construction. Two things are exact:

* the **absolute** cycle of any event, because both ledgers carry `start` from the same counter
  they time events with — so `zaxnes_absolute - bizhawk_absolute - bias` is a constant whenever
  neither machine has gained a cycle, and that constant is **4 774**;
* within a frame, the cycles between any two events.

The comparison is therefore: anchor each frame on its own NMI handler entry, and ask whether every
other event of that frame sits at the same distance from it.

### The bisection, in three rounds

**Frame 1100 is the ideal case and it was taken first**: its header reads `dmc=0/0 oam=1`, so it
has **no DMC DMA at all**, and its one OAM transfer is at cycle 58 — before the frame's work
starts. Every event of it is at the same cycle on both machines **except** `$C06E`, the `STA $39`
with which the main loop announces the frame's work is finished, which Zaxnes reaches **3 cycles
early**. A stretch of about 11 740 cycles of plain CPU execution, no DMA, no interrupt, and three
cycles apart: whatever this is, it is not on the paths lots NESHAWK-PROFILE and NESHAWK-PROFILE-2
read.

| round | the skeleton | what it said |
|---|---|---|
| 1 | 40 PCs, each executed exactly once in frame 1100, spread evenly over instructions 729-4266 | 0 at `pc$C39F` (instruction 756), -3 at `pc$D86A` (1284) and at every one of the 34 PCs after it |
| 2 | 29 PCs over instructions 756-1284 | 0 at `pc$C3A1` — the NMI handler's `RTI` — and **-3 at the very next event**. The gap is `RTI` plus the spin loop: BizHawk spends **14** cycles from `$C3A1` to `$C074`, Zaxnes **11** |
| 3 | the five PCs of the spin loop itself | the mechanism, below |

The skeletons were picked mechanically from a full `frame-trace --out` of the frame — every PC
executed exactly once in the whole frame, sampled evenly by instruction index — so no round
depended on reading the game's code.

### The mechanism, read off round 3

The game's frame-sync loop is three instructions:

```
C06E  85 39     STA $39      the main loop: the frame's work is done
C070  A5 39     LDA $39      3 cycles          \  the spin, 6 cycles round,
C072  F0 FC     BEQ $C070    3 cycles, taken   /  with a boundary every 3
C074  68        PLA          and on from here when the NMI has raised $39
```

In frame 1100, with the two machines' loops **3 cycles out of phase**:

| | BizHawk | Zaxnes |
|---|---|---|
| the `RTI` at `$C3A1` | nmi+2936 | nmi+2931 — the same cycle |
| returns to | **`$C072`**, so a taken `BEQ` (3) + `LDA` (3) + `BEQ` not taken (2) | **`$C070`**, so `LDA` (3) + `BEQ` not taken (2) |
| reaches `$C074` | nmi+2950 | nmi+2942 — **3 cycles earlier** |

and from `$C074` the frame's work is one fixed instruction stream, so it ends 3 cycles apart at
`$C06E`, which puts the *next* frame's spin loop 3 cycles out of phase again. **That is a stable
fixed point, not a drift** — which is why the difference never accumulates and why 8 175 of 8 901
frames still agree byte for byte.

Frame 1113 is the same thing with the phase removed, and it is the decisive measurement.
Frame 1112's work ended at the **same** cycle on both, so the two spin loops are in the same
phase; the vblank then arrives, and:

| | BizHawk | Zaxnes |
|---|---|---|
| `LDA $39` at `$C070` begins | absolute 33 111 147 | 33 115 927 (SL 241 **dot 5**) — the same cycle |
| the vblank/NMI is asserted at | SL 241 dot 1, i.e. during the **second cycle of the `BEQ` before it** | the same |
| the next thing that happens | the `BEQ` at `$C072` **runs** (3 cycles), *then* the interrupt | the interrupt, **in place of** the `BEQ` |
| the handler's first instruction | 33 111 160 = `LDA` + 3 + 3 + 7 | 33 115 937 = `LDA` + 3 + 7 |
| so the handler is entered | **3 cycles later** | |

Both machines declined the interrupt at the `BEQ`'s own polling point; at the `LDA`'s, **Zaxnes
takes it and NesHawk does not**. The vblank was asserted before that `LDA` even began, so this is
not about the PPU's dot: it is about **when the CPU first sees an NMI it has already been handed**
— one cycle of recognition delay, which a 3-cycle instruction turns into three cycles of handler
entry.

### Why this is the lot's item and not noise

* It is **the only difference left** on a frame with no DMC DMA, and it is the *first* event of a
  frame to differ. Every candidate the two previous lots implemented acts later, inside the work.
* It fires on **366 of 1 451** in-step frames — a quarter of them — where the whole of item 1c
  fires six times in a frame and item 7 once.
* It moves **the whole frame's work by 3 cycles relative to the PPU and the APU**, which is the
  kind of quantity frame 9354 is short of: the +-1 of an OAM transfer's alignment and the +-4 of a
  DMC fetch's placement that NESHAWK-PROFILE-2's tables are full of are *consequences* of this
  phase, not causes. The residual distribution says so: the `oam` request agrees on 7 523 of
  7 896 frames and `pc$C37C`, one OAM transfer later, on only 4 136 of 8 155.

Step 2 reads NesHawk's and this core's interrupt polling for the exact rule.

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE-3 step 1/5 done: measure where the cycles come from, on both machines

---
