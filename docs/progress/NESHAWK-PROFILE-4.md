# NESHAWK-PROFILE-4 — item 8's real shape, and the phase seeded at the boot

Branch `feat/neshawk-profile`, worktree `cleanroom/wt-bk2`, base `92b23d71` (lot NESHAWK-PROFILE-3).

Lot NESHAWK-PROFILE-3 took the *Kirby* `.bk2` to frame **18 321** with item 8,
`NMI_SEEN_A_CYCLE_LATE` — a blanket one-CPU-cycle lag of the CPU's view of /NMI — and said in
the same breath that the item is **right about the effect and blunt about the shape**: it costs
the NesHawk profile `ppu_vbl_nmi` `05-nmi_timing`, `vbl_nmi_timing` `7.nmi_timing`,
`cpu_interrupts_v2` `2-nmi_and_brk` and `3-nmi_and_irq`, four `AccuracyCoin` rows and 71 of
*Faxanadu*'s checksum lines, **and NesHawk itself passes every one of them.**

This lot has two problems, in this order:

1. **Item 8 refined** against that oracle — the ROMs NesHawk passes and the profile does not —
   until the profile matches NesHawk on each and *Faxanadu* is back to 207/220, with *Kirby*
   keeping at least 18 321.
2. **The boot and the reset**, where the 3-cycle phase of the two machines' spin loops was
   seeded: frame 6, the movie's first reset, where the absolute drift jumps 2 382 cycles.

## Steps

- [x] 1. Item 8 refined against the oracle, with unit tests and the ROM-by-ROM table.
      **The answer is that it cannot be refined: it is not an NMI difference at all, and it is
      retracted.** Step 2 found what it was standing in for.
- [x] 2. The boot and the reset, ledgered on both machines over frames 0-12 — **item 10**.
- [x] 3. Kirby, iterating — **one round**, and it told the drift and the phase apart.
- [x] 4. No regression, and the standard profile is **faster** than both parents.
- [x] 5. Report and final pass.

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
| disk on `C:` at the start | 7.6 GB free (the brief's stop is 1 GB) |

## The sources read, and their licences

| | |
|---|---|
| **BizHawk** (NesHawk) | MIT. Read through `raw.githubusercontent.com` at tag **2.11.1**; nothing cloned or downloaded. The files and functions are named per item below |
| **Mesen2** | GPL-3.0, for contrast only |
| **nesdev wiki** | the pages named per item |

---

## Step 1 — item 8 against the oracle, and why it has no finer shape

### The answer first

**Item 8 is not a NesHawk NMI difference, and no refinement of it can be.** It is retracted —
constant, bus field, savestate byte, batching horizon and test — and step 2's item 10 is what it
was standing in for. The argument is three lines of measurement and one of logic:

1. `ppu_vbl_nmi` **`05-nmi_timing`** measures *which instruction the vblank NMI displaced*, over
   ten lines that each move the whole test **one PPU dot later**, and checks the ten answers
   against a CRC. Its own source says so: *"Prints which instruction NMI occurred after. Test is
   run one PPU clock later each line."* Ten one-dot steps sweep all three alignments of the CPU's
   cycle grid against the scanline several times over.
2. **NesHawk passes it** (NESHAWK-PROFILE-3 step 2 measured that directly, `status=00
   sig=DE B0 61`), and so does the `Standard` profile — so on every alignment that test probes,
   NesHawk takes the vblank NMI at **the same CPU cycle this core does**.
3. Therefore **any** item that moves the NMI's arrival on **any** alignment fails `05-nmi_timing`
   unless the move happens never to cross an instruction boundary in ten sweeps, which is not a
   model, it is a coincidence. Item 8 moves it on all three alignments.

The same reading explains the *shape* of item 8's price exactly, which is what makes it an
argument rather than an assertion: a **uniform** one-cycle lag preserves every *relative*
question — did the NMI happen at all, was it suppressed, was it disabled in time — and breaks
every *absolute* one. And that is precisely the split: the profile kept `06-suppression`,
`07-nmi_on_timing`, `08-nmi_off_timing`, `vbl_nmi_timing` 5 and 6, and lost `05-nmi_timing`,
`vbl_nmi_timing` `7.nmi_timing`, `cpu_interrupts_v2` `2-nmi_and_brk` and `3-nmi_and_irq` — the
four that time the NMI against the instruction stream.

### The source reading that said otherwise, and where it goes wrong

NesHawk's vblank NMI is two steps (`NES/PPU.run.cs` `TickPPU_VBL`, MIT, read at tag 2.11.1):

```csharp
if (ppur.status.cycle == 0 && ppur.status.sl == 241 + preNMIlines)
{
    nmi_destiny = reg_2000.vblank_nmi_gen && Reg2002_vblank_active;
    if (cpu_stepcounter == 2) { NMI_offset = 1; }
    else if (cpu_stepcounter == 1) { NMI_offset = 2; }
    else { NMI_offset = 0; }
}
else if (ppur.status.cycle <= 2 && nmi_destiny)
{
    nmi_destiny &= reg_2000.vblank_nmi_gen && Reg2002_vblank_active;
}
else if (ppur.status.cycle == (3 + NMI_offset) && ppur.status.sl == 241 + preNMIlines)
{
    if (nmi_destiny) { nes.cpu.NMI = true; }
    nes.Board.AtVsyncNmi();
}
```

with `runppu()` (`PPU.cs`) running `nes.RunCpuOne()` on every third dot and resetting
`cpu_stepcounter` there. The three offsets are not arbitrary: they put the assignment on the **dot
after a CPU-cycle boundary** in all three alignments — dot 3, 4 or 5 — so `MOS6502X.Fetch1()`
reads it at the next boundary. Reading that against this core's "vblank flag at scanline 241 dot
1, sampled `NMI_SAMPLE_LEAD = 2` dots before the end of each cycle" gives *NesHawk one CPU cycle
later on two of the three alignments and the same cycle on the third* — a finer item than the
shipped one, and the obvious candidate.

**It is wrong, and the ROMs say where.** The reading fixes NesHawk's assignment dot relative to
*its own* scanline-241 cycle 0, and nothing in the file says which dot of this core's scanline 241
that is: `Reg2002_vblank_active` is raised at the line transition in `TickPPU_preVBL`, one dot
before cycle 0, and this core raises its flag *at* dot 1. The one free integer in the model —
call it the latch dot — is exactly what the measurement has to supply, and the measurement says
the two machines agree.

### The three refinements, measured rather than argued about

Each was implemented, built and run against all four suites and against *Kirby*'s 1 100-10 000
window. The table is the oracle the brief asked for, ROM by ROM:

| | NesHawk | `Standard` | item 8 as shipped | uniform 4 dots of lead | the flag at 4 dots, the enable at 2 |
|---|---|---|---|---|---|
| `ppu_vbl_nmi` | 11 pass | 11 | 9 | **6** | **6** |
| ... `05-nmi_timing` | pass | pass | **fail** | **fail** | **fail** |
| ... `06-suppression` | pass | pass | pass | **fail** | **fail** |
| ... `07-nmi_on_timing` | pass | pass | pass | **fail** | **fail** |
| ... `08-nmi_off_timing` | pass | pass | pass | **fail** | **fail** |
| `vbl_nmi_timing` | 7 pass | 7 | 6 | **4** | **4** |
| ... `5.nmi_suppression` | pass | pass | pass | **fail #6** | **fail #6** |
| ... `6.nmi_disable` | pass | pass | pass | **fail #4** | **fail #3** |
| ... `7.nmi_timing` | pass | pass | **fail #5** | **fail #6** | **fail #6** |
| `cpu_interrupts_v2` | 6 pass | 6 | 3 | **4** | **4** |
| ... `2-nmi_and_brk` | pass | pass | **fail** | pass | pass |
| ... `3-nmi_and_irq` | pass | pass | **fail** | **fail** | **fail** |
| `AccuracyCoin` | — | 128/147 | 118/147 | — | — |
| *Kirby*, the last frame with no live difference, 1 100-10 000 | — | 925 | **10 000** | — | 8 005 |
| *Kirby*, frames with no live difference | — | — | 8 792 of 8 901 | — | 6 835 |

NesHawk's column is NESHAWK-PROFILE-3 step 2's own measurement for `ppu_vbl_nmi` 05-08 (run on
BizHawk through a config that forces the core, `romresult.lua`) and the brief's statement for the
rest.

**Not one of the three is better than the others in the way that matters**: every one of them
fails `05-nmi_timing`, which is the one ROM that asks the question the item is about. The
`07`/`08` column is the instructive one — all four of `ppu_vbl_nmi` 05 to 08 write `$2000` within
a couple of cycles of the vblank, so lagging the *enable* with the flag fails them, and the third
refinement separates the two leads for exactly that reason. It buys `2-nmi_and_brk` and nothing
else.

### What was eliminated along the way

| | |
|---|---|
| **the `$2000`-enable path** | NesHawk's is `write_2000`'s `NMI_PendingInstructions = 2`, which `runppu()` spends after `nes.RunCpuOne()` — so the CPU meets an enable turned on during vblank **on the cycle after the write**, which is exactly this core's two dots of lead. Not a difference |
| **the destiny freeze as a suppression rule** | NesHawk stops ANDing `nmi_destiny` down after cycle 2, so a `$2002` read or a `$2000` write later than that cannot cancel the assertion. Four dots of lead on the **flag alone** reproduces that in all three alignments — worked through in the refinement's own doc comment — and it still fails the ROMs, because the arrival moves |
| **the interrupt-phase ROM as an oracle** | it cannot settle this one. Its 64-entry table is the same cyclic pattern on all three machines — NesHawk's is this core's rotated by 15 frames, and the `Standard` and `NesHawkCompat` tables are two other rotations of it — so it measures the **phase**, which the boot sets, and not the rule. That is why NESHAWK-PROFILE-3 compared only its shape |

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE-4 step 1 done: item 8 refined against the oracle - and retracted

---

## Step 2 — the boot and the reset: **item 10**, the PPU's start-up frame

### The answer first, and it is one line of BizHawk's own ledger

The cycle ledger `kirby-jp-cyc0.txt` (BizHawk, frames 2 to 1 201, written by `bk2cyc1.lua` in lot
NESHAWK-PROFILE-3) carries the absolute CPU cycle each frame started on. Its first six lines are
the whole of this step:

| | BizHawk `start` | its frame's length | Zaxnes at `92b23d71` | |
|---|---|---|---|---|
| frame 2 | 27 392 | — | 29 781 | so **frame 1 is 27 392 cycles on BizHawk and 29 781 here** |
| frame 3 | 57 173 | 29 781 | 59 564 | |
| frame 4 | 86 953 | 29 780 | 89 343 | |
| frame 5 | 116 734 | 29 781 | 119 131 | |
| frame 6 | 144 126 | **27 392** | 148 905 | and **frame 5 — the frame the movie's first reset lands in — is 27 392 again** |
| frame 7 | 173 907 | 29 781 | 178 684 | |

`29 781 - 27 392 = 2 389`, twice, and `2 * 2 389 = 4 778`. **That is the whole of the constant
drift NESHAWK-PROFILE-3 measured at 4 774 to 4 783 and could not explain.** The drift does not
"jump 2 382 cycles at frame 6" because something happens at frame 6: it is 2 389 from the power-on
and 2 389 more from the reset, and the frames in between are the same length on both machines.

### What NesHawk does, from its own source

| | |
|---|---|
| `NES/PPU.cs` `Reset()` | `regs_reset(); ppudead = 1; idleSynch = true; ... start_up_offset = 4;` |
| `NES/PPU.cs` `regs_reset()` | `ppur = new PPUREGS();` — so the PPU's **position** goes back to the top of the frame |
| `NES/PPU.run.cs` `NewDeadPPU()` | counts dots until `ppur.status.cycle == 241 * 341 - start_up_offset`, raising the vblank flag one dot earlier, then `ppudead--; ppu_init_frame(); do_vbl = true; nes.frame_is_done = true;`. Its own comment: *"not quite emulating all the NES power up behavior ... it needs to wait for vblank"* |
| `PPU.cs` `runppu()` | runs `nes.RunCpuOne()` on every third dot throughout, so **the CPU runs for the whole of the dead frame** |

`241 * 341 - 4 = 82 177` dots, and the CPU runs on dots 3, 6, ... 82 176 — **27 392 cycles**, which
is the ledger's number to the cycle. And because `Reset()` is what a **soft** reset calls, the
reset does it again: `ppudead = 1` and a PPU whose position has been put back to the top.

This core does neither. Its PPU powers on at *the start of vblank* (Q-003, REQ-CORE-020 notes), so
its first frame is a whole one; and `Ppu2C02::soft_reset` deliberately leaves the position alone —
"a RESET press does not restart the frame" (Q-078).

### The item

**Item 10, `PPU_START_UP_DOTS_SKIPPED`** = 0 on `Standard`, **7 165** on `NesHawkCompat`: the dots
the PPU is advanced by at a power-on and at every soft reset, after being put back to the top of
the frame.

`89 342 - 82 177 = 7 165` — this core's first frame is a whole frame of 89 342 dots from its
power-on position to the next vblank flag, NesHawk's dead frame is 82 177 dots, and the difference
is the item. Nothing is tuned: the two numbers are the two machines' own.

| | |
|---|---|
| the sites | `nes_core.rs` `power_on` and `soft_reset`, each one `if P::PPU_START_UP_DOTS_SKIPPED > 0`, through `NesBusOf::run_ppu_start_up_dots` then `Ppu2C02::run_start_up_dots` |
| the position | `run_start_up_dots` puts the PPU back to `(vblank_scanline, 1)` **before** counting, which is `regs_reset()`'s `new PPUREGS()`. At a power-on it is already there; at a soft reset it is not, and that half of the item is worth everything (below) |
| where it runs | **before** the CPU's own reset cycles, because that is where NesHawk's is: `NewDeadPPU()` clocks the CPU through `runppu()` for the whole dead frame |
| new state | **none.** Not one field, on either profile, so the savestate is untouched to the byte and `NesCore::STATE_VERSION` does not move |
| the `Standard` path | the two `if`s fold away and `Ppu2C02::run_start_up_dots` has no caller, so that profile's `power_on`, `soft_reset` and PPU are the source they were |

**The repositioning was measured, not assumed.** Counting the 7 165 dots from wherever the reset
found the PPU — the first implementation — takes *Faxanadu*, which never presses RESET, to its
best figures ever (247 and 314 of 832) and *Kirby*, which presses RESET after frame 5, to **no
agreement at all**: 0 of 8 901 frames. With the position put back first, *Kirby* comes back and
*Faxanadu* is unchanged, which is what a difference that only a reset can see looks like.

### What it is worth

| `--profile neshawk` | parent `92b23d71` (item 8) | this commit (item 10) |
|---|---|---|
| `ppu_vbl_nmi` | 9 pass, 2 fail | **11 pass, 0 fail** |
| `vbl_nmi_timing` | 6 pass, 1 fail | **7 pass, 0 fail** |
| `cpu_interrupts_v2` | 3 pass, 3 fail | **6 pass, 0 fail** |
| `AccuracyCoin` | 118/147 | **122/147** |
| the 30 DMC/DMA/APU ROMs | 19 pass, 5 fail, 6 manual | **19 pass, 5 fail, 6 manual** |
| *Faxanadu* `[USA][!!]` against `fax-usa.txt` | 136 of 832 | **247** of 832 |
| *Faxanadu* `[USA][Rev 1][!!]` against `fax-rev1.txt` | 169 of 832 | **314** of 832 |
| *Kirby*, the last frame with no live difference, 1-400 | 400 | **400** |
| the same, 400-1 100 | 1 100 | **1 100** |
| the same, 1 100-10 000 | **10 000** | 8 005 |
| *Kirby*, frames with no live difference, 1 100-10 000 | **8 792** of 8 901 | 6 839 of 8 901 |
| *Kirby*, frames identical to the byte | 213 | **349** |
| every lag count, every window, both movies | agree | **agree** |
| the absolute drift of the NMI entry, frames 2-1 201 | **4 773 ... 4 783** | **-7 ... +3** |

**All six public ROMs and all four `AccuracyCoin` rows item 8 cost are back**, and both
*Faxanadu* dumps are now **better than the `Standard` profile's own** 207 and 220 — the first time
anything on this profile has beaten it on that movie. The drift is gone: ±5 of quantisation
instead of 4 778.

**And *Kirby* is worse: 8 005 where item 8 reached 10 000, and 18 321 across both windows.** That
is the cost of this step and it is stated plainly here rather than in a footnote. The brief asked
for item 8 refined *with Kirby keeping 18 321*, and the two halves of that cannot both be had: the
18 321 was bought with a model that `05-nmi_timing` proves is not NesHawk's. Step 3 is where the
next item for *Kirby* has to come from, and §4 of the review records the trade for the owner to
rule on.

### The two measurements that back it

* the frame lengths, both machines, frames 1 to 19 — BizHawk's from `kirby-jp-cyc0.txt`, this
  core's from `frame-trace --event-log`. With item 10 in, Zaxnes's frame 1 is 27 394 cycles and
  its frame 5 is 27 387 against BizHawk's 27 392 and 27 392, the few cycles being the frame
  boundary's own quantisation (`run_frame` stops at the end of the instruction that crossed it);
* the absolute cycle of the **NMI handler's entry**, which is an instruction boundary on both
  machines and is quantised by nothing. Over frames 2 to 1 201 the drift `zaxnes - bizhawk` goes
  from a constant 4 776 ± 5 to **-2 ± 5**, and the residual of the `nmi` tag against its own
  frame's anchor is 0 on 1 169 of 1 169 frames.

### The differential test

`item_10_the_ppu_runs_a_start_up_frame_after_a_power_on_and_a_reset` measures, on each profile,
the CPU cycles of the **first** frame from a power-on, of the second, and of the first after a
soft reset. It asserts four things rather than one number: the second frame is a frame long on
both (the item is the start-up and not the frame rate); `Standard`'s first frame is a frame long,
as this core's always was; `NesHawkCompat`'s is 2 389 cycles shorter and is **the same length
again after a soft reset**; and `Standard`'s is not, because its reset does not restart the frame.
A fifth assertion covers the half a frame-level measurement cannot see — that
`Ppu2C02::run_start_up_dots` lands on the same `(scanline, dot)` whether the reset found the PPU
at the top of the frame or 1 234 cycles into it.

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE-4 step 2 done: the boot and the reset - item 10, the start-up frame

---

## Step 3 — Kirby, one round, and why the rounds stop there

### Round 1 — the three cycles survive item 10, and that is the finding

The cycle ledger of frames 1 100 to 10 000 was taken again on the new configuration and aligned
against BizHawk's `kirby-jp-cyc1.txt` with `drift.py`. Two things came out of it.

**The drift is gone.** The NMI entry's absolute drift `zaxnes - bizhawk` was a constant
**4 776 ± 5** for three lots; it is now centred on **0** — 1 838 of 8 901 frames at exactly 0, the
spread running −9 to +4. There is no longer an offset between the two machines' clocks, which is
what item 10 was for.

**And the three cycles are still there, unchanged.** Frame 1 100 is the frame NESHAWK-PROFILE-3
step 1 bisected because it is the ideal case — `dmc=0/0`, no DMC DMA at all, its one OAM transfer
over before the frame's work starts — and on the new configuration it has **exactly one** event
off its own anchor:

    frame 1100: pc$C06E@32738587 (-3)
    frame 1101: pc$C06E@32768169 (-3)

which is §4h's signature to the cycle. The `nmi` tag's residual is 0 on **1 169 of 1 169** frames,
so the handler is entered on the same cycle on both machines — and its `RTI` still returns to a
different instruction of the spin loop, which means the phase is carried in the **pushed PC** from
the frame before. So:

> **The 4 778-cycle drift and the 3-cycle spin-loop phase were two different things wearing one
> number.** NESHAWK-PROFILE-3 §5 named the boot as the phase's seed because the drift jumped
> there. The drift was the boot, item 10 removes it, and the phase is untouched.

### Why the rounds stop at one

* the brief's round loop is *ledger → first differing event → source → item*, and this round's
  first differing event is one NESHAWK-PROFILE-3 already bisected to two instructions of the spin
  loop and already explained — as a **fixed point**, not as a rule. A 3-cycle phase difference in
  a 6-cycle loop reproduces itself exactly, feeds the ±1 of an OAM transfer's alignment and the ±4
  of a DMC fetch's placement, and mostly heals. There is no *new* event to read a source about;
* the item that closes it is **not** on the NMI, and this lot spent its first step proving that
  rather than assuming it. Item 8 moved the NMI and closed the phase on most frames, and
  `05-nmi_timing` says NesHawk's NMI is this core's. What is left is where the *program's* position
  in the frame is set, and the two candidates nobody has measured are named in §5: the **6502's own
  reset sequence** (NesHawk's `MOS 6502X` reset against this core's seven cycles) and the **cycle
  the `.bk2`'s `Reset` command is applied on**. Both are now measurable in a way they were not
  before, because the drift is zero;
* and the brief's own precondition for going further — *"while Kirby keeps at least 18 321"* — is
  not met and cannot be met by this lot's result. Producing further references for 30 000-60 000
  would be measuring a movie that parts at 8 005 against frames it cannot reach, so **no new
  reference was produced and no disk was spent on one.** The three that exist cover everything
  this configuration can say.

### Kirby, measured, every window that exists

| | `standard` | `neshawk`, item 8 (`92b23d71`) | `neshawk`, item 10 |
|---|---|---|---|
| the last frame with no live difference, 1-400 | 400 | 400 | **400** |
| the same, 400-1 100 | 925 | 1 100 | **1 100** |
| the same, 1 100-10 000 | 925 | **10 000** | 8 005 |
| the same, 10 000-30 000 | — | **18 321** | none |
| frames with no live difference, 1 100-10 000 | — | **8 792** of 8 901 | 6 839 of 8 901 |
| frames identical to the byte, 1 100-10 000 | 0 | 213 | **349** |
| frames identical to the byte, 1-400 | — | 192 | **275** |
| all lag counts, every window | agree | agree | **agree** |

The 1-400 and 400-1 100 windows — the ones that contain the boot and the reset — are **clean on
both**, and item 10 takes the first of them from 192 frames identical to the byte to **275**. The
windows that lose are the later ones, where item 8's compensation was doing the work.

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE-4 step 3 done: one round, and the drift and the phase told apart

---

## Step 4 — no regression, and the standard profile is faster

### The `Standard` profile's behaviour: nothing moved

| | |
|---|---|
| `golden-core`, 98 originals × 1 200 frames, release, feature off | `8d508070850b24228285b6f04df6f98c310dbdab65e7577921c91ad4d0689773` — **byte-identical** to lot FCEUX-23's baseline and to all three earlier NESHAWK lots' |
| the 30 ROMs of `dmc_dma_during_read4`, `sprdma_and_dmc_dma`, `apu_test`, `dmc_tests`, `--profile standard` | **24 pass, 0 fail, 6 manual** — ROM for ROM, the same column |
| `AccuracyCoin`, `--profile standard` | **128/147** — unchanged |
| `ppu_vbl_nmi`, `vbl_nmi_timing`, `cpu_interrupts_v2`, `--profile standard` | **11/11, 7/7, 6/6** — unchanged |
| *Faxanadu*, both dumps, `--profile standard` | **207** and **220** of 832 lines, every lag count agreeing — §4's figures |
| savestates | unchanged **to the byte and to the version**, and this time trivially: item 10 adds no field on either profile, and item 8's byte is gone from the NesHawk block |

### The `Standard` profile's speed: faster than both parents

`bench-core --frames 1200 --warmup 120`, the `core run_frame` line only, **six interleaved rounds
per ROM** against two clean `git archive` builds — `92b23d71` (this lot's parent) and `428c50e6`
(the parent of all four NESHAWK-PROFILE lots' code) — the three binaries run in the same order
inside each round:

| ROM | `428c50e6` | `92b23d71` | this commit | vs the parent | **cumulative** |
|---|---|---|---|---|---|
| Super Mario Bros. 3 `[USA][!!]` | 1.17347 ms/frame | 1.15337 | **1.13893** | **−1.25 %** | **−2.94 %** |
| Mega Man 3 `[USA][!!]` | 0.91843 | 0.90222 | **0.90463** | **+0.27 %** | **−1.50 %** |

Per-round figures, in `428c50e6` / `92b23d71` / this order — SMB3: 1.1661/1.1448/1.1434,
1.1645/1.1526/1.1408, 1.1696/1.1582/1.1400, 1.1934/1.1605/1.1366, 1.1661/1.1515/1.1292,
1.1811/1.1526/1.1436. Mega Man 3: 0.9134/0.8948/0.8952, 0.9339/0.9081/0.9292,
0.9164/0.8974/0.9067, 0.9027/0.8968/0.8989, 0.9090/0.9003/0.8939, 0.9352/0.9159/0.9039.

**The owner's budget is +1 % cumulative since the last release and the cumulative figures are
−2.94 % and −1.50 %**, so there is nothing to stop for. SMB3 is faster than the parent in all six
rounds, which is the layout effect NESHAWK-PROFILE-3 paid +0.77 % for being handed back: item 8's
`bool` is gone from `NesBusOf` and item 10 adds no field. The parent builds were put on a **short**
path and deleted afterwards — a `git archive` under the session's own scratchpad makes `link.exe`
fail with `LNK1104`, which cost one build before it was noticed.

### The `NesHawkCompat` profile: eleven ROMs and four rows back

| | parent `92b23d71` | this commit |
|---|---|---|
| the 30 DMC/DMA/APU ROMs | 19 pass, 5 fail, 6 manual | **19 pass, 5 fail, 6 manual** — unchanged |
| `ppu_vbl_nmi` | 9 pass, 2 fail | **11 pass, 0 fail** |
| `vbl_nmi_timing` | 6 pass, 1 fail | **7 pass, 0 fail** |
| `cpu_interrupts_v2` | 3 pass, 3 fail | **6 pass, 0 fail** |
| `AccuracyCoin` | 118/147 | **122/147** |
| *Faxanadu* `[USA][!!]` | 136 of 832 | **247** of 832 |
| *Faxanadu* `[USA][Rev 1][!!]` | 169 of 832 | **314** of 832 |

The four `AccuracyCoin` rows back are `NMI Timing`, `NMI Suppression`, `NMI Overlap BRK` and
`NMI Overlap IRQ` — the four item 8 cost. 122/147 is NESHAWK-PROFILE-2's figure, so the profile is
back to the score it had before item 8 and no further: item 10 costs that suite nothing either way.

**Both *Faxanadu* dumps are now better than the `Standard` profile's own 207 and 220.** That is the
first time anything on this profile has beaten the standard chip on that movie, and §5 records that
it cannot be read as proof of anything: the column is a checksum over the whole 2 KiB, stack page
included, which §4f said cannot discriminate.

### The suites

| | passed | failed |
|---|---|---|
| **`zax-nes`**, release, `--include-ignored` | **1 142** | 0 |
| **`zax-harness`**, release, feature off, `--include-ignored` | **128** | 0 |
| the same with **`dma-census`** | **128** | 0 |
| the same with **`fceux-core`** (`--test-threads 1`) | **148** | 0 |

`zax-nes`'s total is reported as this lot measured it; the tree has the **same 1 269 `#[test]`
items** as `92b23d71` (item 8's differential test out, item 10's in), so the earlier lots' "1 204"
was a different invocation and not a different suite.

`cargo fmt --all --check` is clean and `cargo clippy --workspace --all-targets -- -D warnings` is
clean in **three** configurations: feature off, with `dma-census`, and with `fceux-core`.

### The disk

`G:` had 8.8 GB free at the start and **8.8 GB** at the end; `C:` 7.6 and 7.6. The low point was
8.2 GB on `G:`, reached by the two parent commits' `git archive` copies and builds, which were
deleted again. Nothing was skipped for want of space, and **no new per-frame reference was
produced**, which is where the earlier lots spent theirs.

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE-4 step 4 done: no regression, and the standard profile is faster

---

## Step 5 — report and final pass

### The documents

| | |
|---|---|
| `docs/review/MOVIE-SYNC.md` **§4i** | *Item 8 was not a difference, and the boot was.* The result against every window first, then why item 8 has no finer shape (`05-nmi_timing` at one-dot resolution over all three alignments, which NesHawk passes), the three shapes that were implemented and measured before that was concluded, the boot out of a ledger the third lot had already written, item 10 with its source on both sides, the repositioning measured by the one movie that presses RESET, and the three cycles that survive. The header's lot list gained a NESHAWK-PROFILE-4 paragraph; §4h's and §5's two main unknowns are struck and answered — one of them *"the next thing worth measuring is the boot"*, which is what this lot did; §5 gained **five** of its own, the first of which is a **choice for the owner** rather than an unknown; the commands block has this lot's |
| `docs/review/NESHAWK-PROFILE.md` | the item table loses **8** and gains **10**; "what it is **not**" now says five ROMs and five `AccuracyCoin` rows rather than eleven and nine, because the profile's score went back up; the per-movie attribution gains item 10's row; the candidate list gains **item 8 itself**, with the ROM that closed it, and corrects NESHAWK-PROFILE step 1's "a power-on offset would have parted at frame 1", which was wrong; the interrupt-phase ROM gains a paragraph on **what it cannot do**; the recipe's traps are now **eight**, the two new ones being *an item can be real on one movie and not be a difference* and *a ledger already taken is worth reading before a new one is taken*; step 7 of the recipe gains the check that would have caught item 8; the cost table carries this lot's figures and the cumulative ones |
| `docs/review/NESHAWK-PROFILE-3-SCRIPTS.md` | **unchanged, and that is the point.** This lot added no instrument: it ran the third lot's `bk2cyc1.lua` output and its `drift.py` and `align.py`, and the ROMs. There is no fourth scripts document to commit |

### fmt, clippy and the suites

| | |
|---|---|
| `cargo fmt --all --check` | clean |
| clippy `-D warnings --all-targets`, whole workspace, feature **off** | clean |
| the same with **`dma-census`** | clean |
| the same with **`fceux-core`** | clean |
| `zax-nes` release, `--include-ignored` | **1 142** passed, 0 failed (with `ZAXNES_ROMS`, `ZAXNES_TEST_ROMS` and `ZAXNES_PROFILES` set) |
| `zax-harness` release, `--include-ignored` | **128** passed, 0 failed; **128** with `dma-census`; **148** with `fceux-core` |

### What this lot did not do, said plainly

| | |
|---|---|
| **the brief's own precondition** | *"while Kirby keeps at least 18 321"* is **not met**: the movie parts at 8 005. It cannot be met together with the brief's other precondition — that the profile match NesHawk on `ppu_vbl_nmi` 05, `vbl_nmi_timing` 7, `cpu_interrupts_v2` 2 and 3 and the four `AccuracyCoin` rows — because the 18 321 was bought by item 8 and item 8 is what fails those ROMs. The two were measured against each other rather than one being assumed, the evidence is §4i's and step 1's, and §5 puts the choice to the owner in one paragraph. **If the owner wants the 18 321 back it is one `bool`**, and the price list is §4i's table |
| **item 8 and item 10 together** | measured and worse: the last clean frame of 1 100-10 000 falls to **4 064**. Both supply the same missing three cycles on most frames and together they overshoot. Measured in the shape item 10 had before its repositioning was added; not re-measured in the final shape, because item 8 fails the ROMs whatever item 10 does |
| **rounds 2 to 5 of step 3** | not run, and step 3 says why: this round's first differing event is one NESHAWK-PROFILE-3 had already bisected and explained as a fixed point, and there is no new event to read a source about. The two unmeasured candidates are named in §5 |
| **further per-frame references** | **not produced.** 30 000-60 000 would be 30 001 frames × 4.2 KiB = 126 MB of reference for a configuration that parts at 8 005 and could not reach them. The disk was left where it was found |
| **the whole 156 312 frames against `kirby-jp.txt`** | not run. The checksum column cannot discriminate (§4f) and it would have been four hours of movie for a figure the note could not interpret; the four `--ram-ref` windows are what this lot reports |
| **the pictures** | not taken. The screenshot comparison reads pictures of a commercial game, it has been validated four ways in the three earlier lots, and on a configuration that parts at 8 005 it would have measured the parting rather than the item |
| **item 10's structure** | an approximation of NesHawk's, exact in its dot count and not in its mechanism: NesHawk's dead frame ignores register writes and does not render, and this core's 7 165 dots do neither. §5 records it. The ~29 658-cycle window the hardware ignores PPUCTRL in is still unmodelled on both profiles (Q-078) |
| **the spec** | not edited. NESHAWK-PROFILE step 2's proposed REQ-CORE-124 wording still stands: it points at `docs/review/NESHAWK-PROFILE.md`, which is where the items are |
| **a `cargo test --workspace`** | impossible here (`LNK1181: cannot open input file 'SDL2.lib'`), so the suites are run per crate as the four lots before this one did. `cargo clippy --workspace --all-targets` does not link, so it still covers `zax-app` and `zax-platform` and all their test targets, in all three configurations |

### The brief's rules, checked

| | |
|---|---|
| the `Standard` path's behaviour | `golden-core` byte-identical; 24 of 24 DMC/DMA ROMs, `AccuracyCoin` 128/147, `ppu_vbl_nmi` 11/11, `vbl_nmi_timing` 7/7, `cpu_interrupts_v2` 6/6 and both *Faxanadu* dumps unchanged; savestates unchanged to the byte and to the version |
| the `Standard` path's speed | **−1.25 %** and **+0.27 %** over six interleaved rounds against a clean build of the parent, and **−2.94 %** and **−1.50 %** cumulatively since `428c50e6` — inside the +1 % budget with room, and reported cumulatively as the brief asked |
| no `if profile == NesHawk` in a hot path | item 10 is read as `if P::PPU_START_UP_DOTS_SKIPPED > 0` at **two** sites, both of them `power_on` and `soft_reset`, which run once a reset. Item 8's three sites — one of them `tick`, which runs every CPU cycle — are gone |
| safe Rust only | no `unsafe` added anywhere; `zax-nes` keeps `#![forbid(unsafe_code)]` |
| diagnostic hooks behind a cfg/feature off by default | none added. The cycle ledger needs no feature, as before; the temporary env-var hook used for the dot sweep is **not in the commit**, and the commands block says so |
| no downloads | none. BizHawk was read through `raw.githubusercontent.com` at tag **2.11.1**; nothing cloned, nothing fetched. The test ROMs and their **sources** are `assets-free`'s, already in the tree |
| nothing staged that must not be | no ROM, movie, trace, dump, screenshot, `rust/target-*`, `.claude/` or `CLAUDE.md`; every commit staged files by name |
| no absolute path with a user name in a committed file | checked across both commits and all three documents |
| `spec/` not edited | checked |
| scripts and outputs outside the worktree | the Lua scripts and the forced-NesHawk config stay in `G:/3DSen/ext-tools/scripts/`, every output in `G:/3DSen/test-movies/bizhawk-runs/`, and the two Python analysers were regenerated from `docs/review/NESHAWK-PROFILE-3-SCRIPTS.md` into the session's scratchpad rather than into the tree |

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE-4 step 5 done: report and final pass
