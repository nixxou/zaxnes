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
- [x] 2. Explain the first differing interval.
- [x] 3. Add it to the profile, and iterate 1-3 up to five rounds (two were run; step 3 says why).
- [x] 4. No regression.
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

## Step 2 — the first differing interval, explained

### The interval, and what each machine did in it

The interval is the shortest one this series has had to explain: **the two cycles of the spin loop
before the NMI handler's first instruction.** Frame 1113, with the two machines' spin loops in the
same phase (frame 1112's work ended at the same cycle), in a common frame of reference:

| cycle | BizHawk | Zaxnes |
|---|---|---|
| 144-146 | `$C072 BEQ $C070`, taken, no page crossing, 3 cycles | the same |
| 147-149 | `$C070 LDA $39`, 3 cycles | the same |
| 150 | **`$C072`'s opcode is fetched** | **the interrupt sequence begins** |
| 150-152 | `BEQ $C070` runs again | cycles 2-3 of the sequence |
| 153-159 | the interrupt sequence | the handler's first instruction at 157 |
| 160 | the handler's first instruction | |

The vblank is asserted at scanline 241 **dot 1**, which is inside the *second* cycle of the `BEQ`
at 144-146 — before the `LDA` at 147 has even begun. Both machines declined the NMI at the
`BEQ`'s own boundary (cycle 147). At the `LDA`'s boundary (cycle 150) **Zaxnes takes it and
NesHawk does not**; NesHawk takes it at the next boundary, three cycles later.

So the quantity that differs is **the cycle at which the CPU first sees an NMI it has already been
handed** — not the dot the PPU asserts it on, not the length of the sequence, and not the DMA.

### Both machines' rules, read from their own source

**Zaxnes.** `Ppu2C02` raises /NMI when the vblank flag is set at scanline 241 dot 1
(`nmi_asserted` = `vblank_flag && (control & 0x80) != 0`) and the CPU's view of the line is
`nmi_line_sample`, taken **`NMI_SAMPLE_LEAD = 2` dots before the end of each CPU cycle**
(`Ppu2C02::run_cpu_cycles` and `step_cpu_cycle`). `Cpu6502::end_cycle` then latches the edge and
makes it usable at the **very next** instruction boundary:

```rust
self.poll_irq = self.irq_sample;
let nmi = bus.nmi_asserted();
if nmi && !self.nmi_line { self.nmi_pending = true; }
self.nmi_line = nmi;
self.poll_nmi = self.nmi_pending;
```

with the two-dot lead and the absence of the IRQ's extra cycle of lag both written down as what
`ppu_vbl_nmi` tests 5-8 and `vbl_nmi_timing` tests 5-7 measure (REQ-CORE-116).

**NesHawk**, read through `raw.githubusercontent.com` at tag **2.11.1** (MIT; nothing cloned,
downloaded or copied):

| | |
|---|---|
| `NES/PPU.run.cs` | a `nmi_destiny` flag is latched at cycle 0 of scanline 241 and `nes.cpu.NMI = true` is assigned at scanline 241 PPU cycle **`3 + NMI_offset`**, `NMI_offset` coming from `cpu_stepcounter` |
| `CPUs/MOS 6502X/Execute.cs`, `Fetch1()` | `my_iflag = FlagI; FlagI = iflag_pending; if (!branch_irq_hack) { interrupt_pending = false; if (NMI) { ea = NMIVector; opcode = VOP_NMI; NMI = false; mi = 0; ExecuteOneRetry(); return; } if (IRQ && !my_iflag) { … } } mi = 0; opcode = VOP_Fetch1_NoInterrupt; Fetch1_Real();` — so `NMI` is read **at the cycle the next opcode would be fetched on**, and it is a one-shot (`NMI = false` when taken) |
| `Execute.cs`, `Fetch1_Real()` | `rdy_freeze = !RDY; if (RDY) { branch_irq_hack = false; _link.OnExecFetch(PC); … opcode = _link.ReadMemory(PC++); mi = -1; } address_bus = PC;` — `OnExecFetch` is the hook `event.onmemoryexecute` rides on, and it fires **only** when an opcode is really fetched, which is what makes the ledger's "gap 10" readable (see below) |
| `Execute.cs`, `RelBranch_Stage3()` | `if (RDY) { DummyReadMemory(PC); alu_temp = (byte)PC + (sbyte)opcode2; PC &= 0xFF00; PC \|= (alu_temp & 0xFF); if (alu_temp.Bit(8)) { opcode = VOP_RelativeStuff2; mi = -1; } else { if (!interrupt_pending) branch_irq_hack = true; } }` |
| `NES.Core.cs`, `RunCpuOne()` | nothing assigns `cpu.NMI`: the PPU does |

Zaxnes's CPU can therefore see the assertion from the cycle whose sample point falls at dot 1 or
later — dot 2 in frame 1113's alignment — and NesHawk's from the cycle that begins at dot 3, 4 or
5. **One CPU cycle later**, and a 3-cycle instruction turns one cycle of recognition into three
cycles of handler entry.

### The candidate that had to be eliminated, and the ROM that did it

`branch_irq_hack` is the obvious suspect and the reading above predicts it: it is set for every
taken non-page-crossing branch (`interrupt_pending` is assigned `false` in `Fetch1()` and **never
assigned `true` anywhere in `Execute.cs` or `MOS6502X.cs`**, so the `if (!interrupt_pending)` gate
is always open), and `Fetch1()` then skips the NMI test entirely on the instruction that follows.
Kirby's spin loop is `LDA $39` / `BEQ` taken and non-page-crossing, so the prediction is sharp:
**NesHawk could never take the NMI at the boundary before the `LDA`.**

It does. Measured, with a ROM written for the purpose:

`crates/zax-nes/tests/neshawk_profile_tests.rs`'s **interrupt-phase ROM** is Kirby's construct
isolated — a `LDA $00` / `BEQ` spin loop, an NMI handler that records the low byte of the
*interrupted* PC, 64 frames per variant — in two variants that differ in one bit: variant A's
branch does **not** cross a page (`$8102 BEQ $8100`: `(byte)PC` is `$04`, displacement −4, no
borrow) and variant B's **does** (`$8200 BEQ $81FE`: `(byte)PC` is `$02`, displacement −4, a
borrow). The loop's phase relative to the vblank walks by itself, because a frame is 29 780 or
29 781 cycles and the loop's period is 6 or 7, so 64 frames sweep every phase without a delay
slide having to be written.

The same bytes are given to both machines: the Rust test writes the `.nes` out under
`ZAXNES_DUMP_ROM` and BizHawk is handed that file (`phaserom.lua` reads the two tables back out of
`$0300` and `$0400`).

| | variant A (no page crossing) | variant B (page crossing) |
|---|---|---|
| Zaxnes, both profiles | **21** of 64 answers are the `LDA`, 43 the `BEQ` | 27 of 64 are the `LDA`, 37 the `BEQ` |
| NesHawk | **20** of 64 are the `LDA`, 44 the `BEQ` | **27** of 64 are the `LDA`, 37 the `BEQ` |

**NesHawk names the `LDA` as the displaced instruction on 20 of 64 frames of variant A** — and
every `LDA` boundary of that loop is immediately after a taken non-page-crossing branch. So
`branch_irq_hack` does **not** suppress the NMI in BizHawk 2.11.1 as it runs, whatever the source
reads like, and the branch quirk is eliminated as the mechanism **by measurement against a reading
that predicted the opposite.** (It is also why the two tables have the same *shape* on both
machines — 21 against 20 and 27 against 27 — and differ in which frame each answer falls on:
the rule is "the first instruction boundary at or after a threshold", with a different threshold.)

### What else was eliminated, and how

| candidate | why it is not this |
|---|---|
| the **vblank dot** and the NMI sequence | NesHawk **passes** `ppu_vbl_nmi` `05-nmi_timing`, `06-suppression`, `07-nmi_on_timing` and `08-nmi_off_timing` (status `00`, signature `DE B0 61`), which measure the assertion and the suppression window at one-PPU-clock resolution; Zaxnes passes them too (REQ-CORE-116). The difference is **inside the tolerance of every public test there is**, which is why no lot found it from a test ROM |
| the **branch interrupt quirk** | NesHawk **passes** `cpu_interrupts_v2` `5-branch_delays_irq`, whose own source states the rule in words — *"A taken non-page-crossing branch ignores IRQ during its last clock… The same occurs for NMI, though that's not tested here"* — and the phase ROM shows NesHawk taking the NMI right after such a branch anyway |
| the **frame sequencer and `$4017`** | §4g sent this lot there first. The ledger says the `$4017` write is at the same cycle as Zaxnes's on 4 250 of 8 155 frames and off by exactly the NMI entry's own residual on the rest: it is downstream of the NMI, not upstream |
| the **`$4016` strobe delay** | the same: all eight strobes of a frame are at the NMI's own residual, 33 037 of 65 240 of them at 0 |
| **DMC, DMA, `$2002`, the odd-frame skip** | frame 1100 has `dmc=0/0`, its OAM transfer is over before the frame's work starts, it makes no `$2002` read, and the accumulated frame length over 8 901 frames agrees to the end-point quantisation. There is nothing else in that frame |

### Why this was never going to be found by arithmetic

The two previous lots sized their items in advance and were wrong twice. This one cannot be sized
at all: **one cycle of recognition** moves the whole of a frame's work by **three** cycles
relative to the PPU and the APU, which then changes which cycle every DMC fetch's halt lands on
and whether the frame's OAM transfer costs 513 or 514 — and those are the ±1 and ±4 residuals
NESHAWK-PROFILE-2's tables are full of. The thirty cycles frame 9354 is short of are a sum over
*that*, not over anything with its own budget.

### What step 3 implements

**Item 8, `NMI_SEEN_A_CYCLE_LATE`**: the CPU's view of /NMI lags the PPU's sample by one CPU
cycle. One associated constant, read at `NesBusOf::nmi_asserted` with a one-cycle copy kept in the
bus, plus the batching horizon that copy needs. The magnitude is one cycle from NesHawk's
`3 + NMI_offset` against this core's two-dot lead; if the movie says otherwise, round 2 tries two.

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE-3 step 2/5 done: explain the first differing interval

---

## Step 3 — item 8, and the rounds

### Round 1 — item 8, `NMI_SEEN_A_CYCLE_LATE`

**Frame 9354 is closed, and the 8 968 frames after it with it.** The movie runs out of the
reference window, so this lot produced the next reference itself and the movie now holds to
**frame 18321**.

| | NESHAWK-PROFILE-2's seven items | with item 8 |
|---|---|---|
| frames compared (1 100-10 000) | 8 901 | 8 901 |
| **the last frame with no live difference** | **9 353** | **10 000** — the window's last frame |
| frames with no difference outside the stack page and `$0000-$001F` | 8 175 of 8 901 | **8 792** of 8 901 |
| frames with a live difference | 726 | **109**, every one isolated |
| frames **identical to the byte**, the dead stack included | **0** | **213** |
| all 8 901 lag counts | agree | agree |

That last row is the one worth stopping on. Before this item **not one** of the 8 901 frames was
identical to the byte — the stack page always differed, on every frame since 1 100, and
NESHAWK-PROFILE-2 §5 recorded a one-address-shifted block of it at 4 663-4 665 as "the fossil of
the two machines' stack pointers having been one byte apart". With item 8 there are **213 frames
whose whole 2 KiB agrees**, dead stack and all. The shifted block was the NMI having been taken
one instruction apart.

### The implementation, and the one site

`TimingProfile::NMI_SEEN_A_CYCLE_LATE`, `false` on `Standard`, `true` on `NesHawkCompat`, read at
`nes_bus.rs` `nmi_asserted` — which is what `Cpu6502::end_cycle` calls once per CPU cycle — with:

| | |
|---|---|
| `NesBusOf::nmi_seen` | the line as the previous cycle's sample left it. One `bool`, set in `tick` **before** the PPU runs this cycle's dots, which is what makes it exactly one cycle of lag and nothing else |
| `ppu_horizon` | one more `return 1`: the cycle the copy changes on has to be run in step, or the batching would copy a sample the PPU had already moved past |
| the savestate | written **only** on the profile that has the field, as item 2's reload countdown is, so a `Standard` block stays byte for byte what it always was and `NesCore::STATE_VERSION` does not move. It is *not* transient the way `dma` is: `Cpu6502` saves `nmi_line`, so a copy that reloaded as `false` against an asserted line would manufacture an edge and take the NMI twice |
| `reset` | cleared, with the rest of the per-cycle state |

The `Standard` arm of `nmi_asserted` is the one-line body the function had before the item
existed, and `tick`'s and `ppu_horizon`'s additions are both inside `if P::…`, so that profile's
two functions are the source they were.

### The test

`item_8_the_cpu_sees_the_vblank_nmi_one_cycle_later` counts, on each profile, the cycles from the
`$2000` write that enables the NMI to the first cycle on which **the CPU's own view** of the line
is true, and separately to the first cycle on which **the PPU's own sample** is. It then asserts
three things rather than one number:

* the PPU's sample is at the **same** cycle on both profiles — the item is in the chip, not in the
  picture;
* on `Standard` the CPU sees the line on exactly that cycle, as this core always did;
* on `NesHawkCompat` it sees it **one cycle later**, and that is the whole of item 8.

### Round 2 — item 9, and it is not an item at all

Frame 18322 is **frame 4666's signature a third time**: `$0038`, `$0039`, `$0048`, `$0049` — the
frame semaphore and the counters it drives — with `--converge` reporting "at NO instant of this
frame does the live memory agree", 69 DMC fetches and one OAM transfer. The frame's work crossed
the vblank again.

The cycle ledger over frames 18 250-18 330 then pointed somewhere precise. With item 8 in, the
`$4014` **request** is on the same cycle as NesHawk's on **71 of 72** frames — but `pc$C37C`, one
OAM transfer later, on only **40 of 72**, the rest differing by exactly ±1. One OAM transfer, one
cycle: the transfer's **alignment**. And NesHawk's source says it outright (`NES.Core.cs`, the
`sprdma_countdown` block):

```
sprdma_countdown = 1;                       // at the $4014 write
…                                           // on the cycle it reaches zero:
if (apu.dmc.timer % 2 == 0) cpu_deadcounter = 2; else cpu_deadcounter = 1;
oam_dma_exec = true; oam_dma_index = 0;
```

with the transfer loop then spending `cpu_deadcounter` cycles doing nothing and 512 doing the
get/put pairs. **Nothing in that decision looks at the cycle**: NesHawk aligns the transfer on the
parity of the **DMC's timer**, where this core (and Mesen 2, and the nesdev *DMA* page's "+1 if on
an odd CPU cycle") waits for the first get cycle.

It was implemented — one constant, `OAM_ALIGNS_ON_THE_DMC_TIMER`, read at `run_dma` where a
transfer's get cycles are decided — and the result was that **the 81-frame cycle ledger came out
byte-identical**: not one of the 81 transfers changed length. That is not the inertness items 3
and 7 have; it is stronger, and the reason is arithmetic:

> **The DMC's timer counts down one per CPU cycle and reloads to `rates[i] - 1`, so its period is
> `rates[i]`. All sixteen NTSC rates — 428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128,
> 106, 84, 72, 54 — and all sixteen PAL ones are EVEN. So the timer's parity is locked to the CPU
> cycle's parity from power-on and no reload, and no `$4010` write, can ever unlock it.**

Both machines start from the equivalent state (this core's timer is `rates[0] - 1` at cycle 0,
NesHawk's is `DMC_RATE[0]` at its cycle 0 — the plus-one convention again), so
`apu.dmc.timer % 2 == 0` and "the cycle after the halt is not a get cycle" are **the same
predicate on every reachable state of either machine**. NesHawk's rule is a different *expression*
of this core's rule, not a different rule.

**So item 9 was reverted rather than shipped**, and that is the right answer rather than a
shortcut: a constant whose two arms are provably the same function cannot be given the
differential test the recipe requires — a test that cannot fail is not a test — and shipping it
would have put an untestable `if` and a `sync_apu` on the NesHawk profile's OAM path for nothing.
The proof is the deliverable, and it closes the brief's "OAM DMA alignment" candidate for good
rather than for this movie.

### Why the rounds stop at two

Rounds 3 to 5 are not run, and the reason is the same shape as NESHAWK-PROFILE-2's:

* every candidate the brief named has now been **measured** rather than argued about. The APU
  frame sequencer and `$4017`, the `$4016` strobe delay, the NMI/IRQ polling points, `$2002` and
  the vblank flag, the odd-frame skip, the branch quirk and the OAM alignment: the first is
  downstream of the NMI (step 2's table), the branch quirk is eliminated by the interrupt-phase
  ROM, the alignment is eliminated by the parity of the rate table, and the rest by the ledger's
  own residuals;
* what is left at frame 18 322 is the **phase**, not a rule. The two machines' spin loops are
  still a multiple of three cycles apart on most frames — the drift of the NMI entry runs
  4 771 to 4 778 over the 81-frame window — and that offset was seeded long before frame 1 100
  (step 1 measured a **2 382-cycle** jump at frame 6, where the movie's first reset lands) and is
  self-sustaining: a 3-cycle phase difference in a 6-cycle spin loop reproduces itself exactly,
  and feeds the ±1 of an OAM alignment and the ±4 of a DMC fetch's placement that then mostly
  heal. 213 frames identical to the byte says the *rules* now agree; the phase says the two
  machines did not start from the same place;
* so the next thing worth measuring is **the boot**, not another item on the DMA or the APU, and
  that is a different lot's work: it needs a reference over frames 1 to 400 with the reset in it,
  which `kirby-jp-step1.txt` has and which §5 of the review now asks for by name.

### Kirby, measured

| | `standard` | `neshawk`, NESHAWK-PROFILE-2 | `neshawk`, with item 8 |
|---|---|---|---|
| the last frame with no live difference, 1 100-10 000 | 925 | 9 353 | **10 000** |
| the same, 10 000-30 000 (this lot's own reference) | — | — | **18 321** |
| frames with no live difference, 10 000-30 000 | — | — | **8 261** of 20 001 |
| isolated differing frames below the parting frame, 10 000-30 000 | — | — | **61** |
| all 20 001 lag counts of the second window | — | — | agree |
| the whole 156 312 frames against `kirby-jp.txt`, checksum lines | 2 of 1 563 | 1 of 1 563 | **4** of 1 563 |
| all 1 563 lag counts | agree | agree | agree |

The checksum column still cannot discriminate, for §4f's reason — a checksum covers the stack page
and `$0000-$001F` — and is reported only so that it is not missing.

### The pictures

Under EVAL-SYNC §4's colour-bijection rule, with the eight overscan rows cropped top and bottom
(BizHawk writes 256×224, this core the whole 256×240, REQ-CORE-048):

| | 1500 … 10000, every 500 | 12000 … 30000, every 2000 |
|---|---|---|
| `neshawk`, with item 8 | **16 of 18** (9 500 and 10 000 gained; 5 500 and 8 500 are 4-pixel misses) | **6 of 10** — 12 000 to 18 000 all match, and so do 26 000 and 30 000 |
| `standard` | **0 of 18**, 38 914 to 54 963 pixels out | — |

**Every picture inside the synchronised stretch matches except two, and both of those are four
pixels on a frame whose work RAM has no live difference at all** — frame 8 500 is the one
NESHAWK-PROFILE-2 §4g recorded and frame 5 500 is new, which makes three instances of that class
(frame 400's single pixel being the first). It is a PPU or sprite-evaluation difference and not a
timing one, and it is recorded rather than absorbed.

The **0 of 18 against 16 of 18** is also the validation of the comparison script, which is a
re-implementation (the original was never committed, because it reads pictures of a commercial
game): a script that said MATCH indiscriminately could not produce both columns, and one that said
"no" indiscriminately could not produce the first.

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE-3 step 3/5 done: add it to the profile, and iterate

---

## Step 4 — no regression, and the one real cost

### The `Standard` profile's behaviour: nothing moved

| | |
|---|---|
| `golden-core`, 98 originals × 1 200 frames, release, feature off | `8d508070850b24228285b6f04df6f98c310dbdab65e7577921c91ad4d0689773` — **byte-identical** to lot FCEUX-23's baseline and to both earlier NESHAWK lots' |
| the 30 ROMs of `dmc_dma_during_read4`, `sprdma_and_dmc_dma`, `apu_test`, `dmc_tests`, `--profile standard` | **24 pass, 0 fail, 6 manual** — ROM for ROM, the same column |
| `AccuracyCoin`, `--profile standard` | **128/147** — unchanged |
| `ppu_vbl_nmi`, `vbl_nmi_timing`, `cpu_interrupts_v2`, `--profile standard` | **11/11, 7/7, 6/6** — every one, which is what item 8 had to leave alone and did |
| *Faxanadu*, both dumps, `--profile standard` | **207** and **220** of 832 lines, last agreeing 83 200, every lag count agreeing, final pictures `03d7e6a1…` and `13920624…` — §4's figures to the hash |
| savestates | unchanged to the byte and to the version: the new field is written only on the profile that has it, and `NesCore::STATE_VERSION` did not move |

### The `Standard` profile's speed: +0.77 % and +0.40 %, inside the budget

`bench-core --frames 1200 --warmup 120`, the `core run_frame` line only, **six interleaved rounds
per ROM** against the parent commit `7c0b88b1` built from a clean `git archive` copy, the two
binaries alternating which goes first:

| ROM | parent | this commit | |
|---|---|---|---|
| Super Mario Bros. 3 `[USA][!!]` | 1.15065 ms/frame | 1.15957 | **+0.77 %** |
| Mega Man 3 `[USA][!!]` | 0.89205 | 0.89562 | **+0.40 %** |

Per-round figures: SMB3 parent 1.1518 / 1.1585 / 1.1545 / 1.1493 / 1.1441 / 1.1457 against
1.1563 / 1.1622 / 1.1766 / 1.1563 / 1.1557 / 1.1503; Mega Man 3 parent 0.8939 / 0.8968 / 0.8897 /
0.8982 / 0.8898 / 0.8839 against 0.8934 / 0.8969 / 0.8878 / 0.8948 / 0.9044 / 0.8964. SMB3 is
slower in all six rounds, so this is a real effect and not noise, and it is **the same layout class
NESHAWK-PROFILE-2 measured in the other direction** (−2.24 % there, for a lot whose `Standard`
arms were equally untouched): `golden-core` is byte-identical, every new statement on the
`Standard` path is inside an `if P::…` that folds away, and what is left is **one `bool` in
`NesBusOf`** — `nmi_seen`, placed next to `dmc_from_write` so the two share the padding — plus one
store in `reset`. Both figures are inside the +1 % the brief allows, and nothing was tuned to get
them there.

### The `NesHawkCompat` profile: item 8 costs six public ROMs and four `AccuracyCoin` rows

This is the item's price and it is the largest any item on this profile has carried, so it is
reported first and in full, with the parent commit's own column beside it (the same binary, the
same ROMs, `--profile neshawk`):

| | parent `7c0b88b1` | this commit |
|---|---|---|
| the 30 DMC/DMA/APU ROMs | 19 pass, 5 fail, 6 manual | **19 pass, 5 fail, 6 manual** — unchanged |
| `ppu_vbl_nmi` | 11 pass | **9 pass, 2 fail** (`05-nmi_timing`, and the collection that contains it) |
| `vbl_nmi_timing` | 7 pass | **6 pass, 1 fail** (`7.nmi_timing`, "FAILED #5") |
| `cpu_interrupts_v2` | 6 pass | **3 pass, 3 fail** (`2-nmi_and_brk`, `3-nmi_and_irq`, and the collection) |
| `AccuracyCoin` | 122/147 | **118/147** |

The nine `AccuracyCoin` rows the NesHawk profile now fails and the standard one passes are
`DMC DMA + OAM DMA`, `Delta Modulation Channel`, `DMC Reload Timing`, `Implied Dummy Reads`,
`Interrupt flag latency`, **`NMI Overlap BRK`**, **`NMI Overlap IRQ`**, **`NMI Suppression`** and
**`NMI Timing`** — the last four new, and `DMA Landing on Write` **gained** (it was on
NESHAWK-PROFILE-2's list of six).

**Every one of the six lost ROMs measures the NMI at CPU-cycle resolution, which is exactly the
one cycle item 8 introduces**, so they fail by construction and not by accident. That matters
because it is also a *tension*: step 2 measured NesHawk itself **passing** `05-nmi_timing`,
`06-suppression`, `07-nmi_on_timing`, `08-nmi_off_timing` and `5-branch_delays_irq`. So NesHawk is
one cycle later than this core at the *instruction boundary* the movie measures and not one cycle
later at the *cycle* a test ROM measures — which means item 8 as written is a **coarser model than
NesHawk's mechanism**, right about the effect and blunt about the shape. §5 records that as the
lot's main open question and names what would settle it.

The alternative shape was tried rather than argued about: lagging only the line's **rise** and
letting its fall through at once — which is what NesHawk's one-shot `NMI = false` and its
`nmi_destiny` latch suggest — left *Kirby* and *Faxanadu* exactly where they were and took
`ppu_vbl_nmi` to **6 of 11** and `vbl_nmi_timing` to 4 of 7. It was reverted. The blunt model is
the better of the two measured, and that is the only reason it is the one that shipped.

### *Faxanadu*, and the first time an item has made a second NesHawk movie worse

| | parent, `neshawk` | this commit, `neshawk` | `standard`, both commits |
|---|---|---|---|
| `[USA][!!]` against `fax-usa.txt` | 207 of 832 | **136** of 832 | 207 of 832 |
| `[USA][Rev 1][!!]` against `fax-rev1.txt` | 220 of 832 | **169** of 832 | 220 of 832 |
| all 832 lag counts, both dumps | agree | **agree** | agree |
| the final picture | `03d7e6a1…` / `13920624…` | **the same hashes** | the same hashes |

So nothing *observable* moved — the lag count is frame-exact and the final picture is the same
bytes — and the column that did move is the one §4f already said cannot discriminate: a checksum
over the whole 2 KiB, stack page included. But it moved **down**, on a movie that was also
recorded on NesHawk, and that is reported as the counter-indication it is rather than explained
away.

What it is almost certainly about: **Faxanadu does not wait on an NMI semaphore at all.** The DMA
census of its frame 1000 shows a tight `$2002` polling loop — a read every seven cycles, 72
instructions in — where *Kirby* reads `$2002` **not once**. Every one of those reads clears the
vblank flag, so Faxanadu's frame sync is decided by the *deassertion* of /NMI and the suppression
window around it, which is precisely where a blanket one-cycle lag of the whole line is coarser
than NesHawk's. The measurement that would settle it is a per-frame `ram=` reference for
*Faxanadu*, which does not exist: `fax-usa.txt` carries a checksum every hundred frames and no
RAM, so this lot cannot say whether the 71 lines it lost are live bytes or the stack page.

### The suites

| | passed | failed |
|---|---|---|
| **`zax-nes`**, release, `--include-ignored` | **1 204** | 0 |
| **`zax-harness`**, release, feature off, `--include-ignored` | **128** | 0 |
| the same with **`dma-census`** | **128** | 0 |
| the same with **`fceux-core`** (`--test-threads 1`) | **148** | 0 |

`zax-nes` goes from NESHAWK-PROFILE-2's 1 201 by item 8's own differential test and the
interrupt-phase ROM's two (one of them the ignored writer, which `--include-ignored` runs).
`zax-harness` goes from 127 to 128 by the cycle ledger's format test.

`cargo fmt --all --check` is clean and `cargo clippy --workspace --all-targets -- -D warnings` is
clean in **three** configurations: feature off, with `dma-census`, and with `fceux-core`.

### The disk

`G:` had 8.8 GB free at the start and **8.9 GB** at the end of this step; `C:` 8.9 and 7.6. The
low point was 6.3 GB on `G:`, reached by this lot's own second reference
(`kirby-jp-step1d.txt`, 20 001 frames × 4.2 KiB = **82 MB**) and ten screenshots; the parent
commit's `git archive` copy and its 307 MB build were put on `C:` for that reason and deleted
again. Nothing was skipped for want of space.

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE-3 step 4/5 done: no regression

---
