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
- [ ] 3. Kirby, iterating, and then the whole movie.
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
