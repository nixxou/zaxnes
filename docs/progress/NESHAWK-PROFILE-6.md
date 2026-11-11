# NESHAWK-PROFILE-6 — Q-182 answered by measurement: item 1c is retracted, item 11 ships on

Branch `feat/neshawk-profile`, worktree `cleanroom/wt-bk2`, base `7eb5eceb` (lot NESHAWK-PROFILE-5).

Lot NESHAWK-PROFILE-5 left one question in the way of item 11: **Q-182**, whether a DMC reload
whose request falls on a write cycle costs NesHawk three cycles (item 1c's derivation from three
files of its source) or four (what one measurement of *Kirby*'s frame 11 said). This lot generalised
the measurement instead of re-reading the source, and the measurement is unambiguous.

## Steps

- [x] 1. Item 1c corrected from measurement. `q182b.lua` logs BizHawk's **complete** bus stream for
      whole chosen frames; joined with this core's own census it gives, per fetch, what each machine
      charged. **A NesHawk reload costs four cycles at every slip, 0 to 3** — item 1c is
      **retracted**, item 11 goes **on**.
- [ ] 2. Rounds against the references.
- [ ] 3. No regression.
- [ ] 4. Report and final pass.

## The environment this lot runs in

| | |
|---|---|
| `CARGO_TARGET_DIR` | `<worktree>/rust/target-ev` |
| `TMP` / `TEMP` | `<worktree>/rust/target-ev/tmp` |
| `CARGO_INCREMENTAL` | `0` |
| disk on `G:` at the start | 6.7 GB free (the brief's stop is 5 GB) |
| disk on `C:` at the start | 12 GB free (the brief's stop is 1 GB) |

## The sources read, and their licences

| | |
|---|---|
| **BizHawk** (NesHawk) | MIT, tag **2.11.1**, read through `raw.githubusercontent.com`; nothing cloned. `NES.Core.cs` `RunCpuOne` verbatim (the OAM block, the DMC countdown block, `RunOneFirst`, `ExecuteOne`, `RunOneLast` and the closing `if (!cpu.RDY && !dmc_dma_exec && !oam_dma_exec) cpu.RDY = true;`) |
| **BizHawk 2.11.1 run as a program** | `EmuHawk.exe --movie --lua`, the *Kirby* `.bk2`, NesHawk (confirmed: the run reproduces `q182.txt`'s frame-11 window cycle for cycle) |

---

## Step 1 — item 1c corrected from measurement

### The answer first

1. **A NesHawk DMC reload fetch costs the CPU four cycles whatever the CPU was writing on.** Item
   1c's odd-slip cycle is **not** a NesHawk behaviour: it is retracted, as item 8 was.
2. **The measurement that says so is calibration-free**, and that is what Q-182 was waiting for. It
   does not use the cycle BizHawk's Lua reports a DMA read on — the number the lot before this one
   could not pin. It uses **two CPU writes that bracket one fetch**: of frame 11's twenty-one `$40xx`
   accesses, every **write** lands at residual **0** against BizHawk's own, up to and including the
   `$4007` write the disputed request falls on, and the next CPU-stream event after it — the
   `$4015` read 252 cycles later — lands at **+1**. Exactly one fetch lies between them, the
   slip-1 reload of `$FCC8`, and it is the one this core charges three.
3. **Item 11 goes on with it**, which is what lot NESHAWK-PROFILE-5 shipped off and gated on this
   question.

### The instrument: `q182b.lua`

`q182.lua` logged every System Bus access for 14 cycles after **one** `$4007` write, which settled
one fetch and raised the calibration question. `q182b.lua` (text in `docs/review/`) logs the
**complete** access stream — reads, writes and execute-flagged fetches, each with
`emu.totalexecutedcycles()` — for a **list of whole frames**, and registers its callbacks once and
gates them on a boolean so that the run only has to reach the last wanted frame.

    EmuHawk.exe --movie="<kirby.bk2>" --lua="q182b.lua" "<kirby.nes>"   ->  bizhawk-runs/q182b.txt

The frames are chosen from this core's own census rather than guessed: over frames 10-2 000,
`frame-trace --dmc-census --profile neshawk` finds reload fetches at a slip of **0** (34 275), **1**
(2 114), **2** (416) and **3** (2) — and **frame 11 carries all of 0, 1 and 2**, frame 310 the only
clean slip of 3. So three frames cover the whole table: 10, 11 and 309 in BizHawk's framecount,
which is this tree's frame number minus one. 2.0 MB, 4m17s.

The census grew one column for this, under the `dma-census` feature as every census hook is: a
fetch line now carries the **PC of the instruction it happened inside and that instruction's
opcode-to-opcode gap**. `gap - cost` is the instruction's undisturbed length, which both machines
must agree on, so `gap_bizhawk - (gap - cost)` is what NesHawk charged — with no opcode cycle table
and no calibration.

### What NesHawk does, read off its own stream

The frame-11 window of `$FCC8` — the fetch Q-182 is about — is this (BizHawk's own cycles):

    301501  w 4007 26      <- the STA's write cycle; this core puts the DMC request here
    301502  x/r 81FB 28       PLP opcode fetch
    301503  r 81FC E8         PLP cycle 2, the read of PC+1
    301504  r 81FC E8      <- the DMA's re-reads of the address the CPU had on the bus
    301505  r 81FC E8
    301506  r FCC8 73         the sample fetch
    301507  (nothing)         one more cycle with the CPU stopped
    301508  r 01E3 85         PLP cycle 3
    301509  r 01E4 36         PLP cycle 4
    301510  x/r 81FC E8       the next opcode

`PLP` is four cycles and takes **eight**, so NesHawk stalls the CPU **four** cycles here. The same
shape appears at every slip (`$FCDC`, a `JSR` whose two pushes the DMA lets through; `$FCDD`, a
`STA (zp,X)` whose write it lets through; `$FC30`, the three pushes of an IRQ) and the figure is
**four** every time: the DMA re-reads for two cycles, fetches on the third, and the CPU is stopped
for a fourth cycle on which nothing is reported at all.

### Stall by slip, and by what follows

The join over frames 11, 12 and 310 — 99 of 156 fetches usable, the rest thrown out because the
bracketing instruction is a branch or an indexed read whose length is not fixed:

| slip | this core charges | **NesHawk charges** | n |
|---|---|---|---|
| 0 | 4 | **4** | 72 |
| 1 | 3 | **4** | 6 |
| 1 | 3 | 3 | 10 |
| 2 | 4 | **4** | 8 |
| 3 | 3 | **3** (13 − 3 − 7, the IRQ's own seven cycles) | 1 |

**The slip-1 row splits, and the split is an artefact of the measurement, not of NesHawk.** A slip is
read off *this* core's census, so it is only meaningful while the two CPU streams are in step — and
they are in step only until the first fetch the two machines charge differently, after which the
stream drifts and every later slip in that frame may be read against the wrong cycle. Frame 11 is
the one frame that **starts** in step (frames 1-10 are identical event for event, lot
NESHAWK-PROFILE-5 step 1), and in frame 11 the fetches in order are `$FCC0` (a `$4015`
write started it, item 3), `$FCC1`-`$FCC7` (slip 0, charged 4 by both) and then `$FCC8`, the first
slip of 1 — **and `$FCC8` is charged 4.** Everything after it in that frame, and all of frames 12
and 310, is measured through a drifted stream. The six 4s are the trustworthy ones; the ten 3s are
the drift.

### The bracketing writes, which is the calibration-free part

Every `$40xx` access of frame 11 against BizHawk's own, aligned on nothing (one offset for the whole
frame, fixed by the first write):

| our cycle | access | residual |
|---|---|---|
| 1 673 | `w $4017` | 0 |
| 4 991 / 5 020 / 5 088 | `w $4012` / `w $4013` / `w $4010` | 0 |
| 8 214 … 8 432 | `w $4000`, `w $4002`, `w $4003`, `w $4004`, `w $4006` | 0 |
| **8 470** | **`w $4007`** — the cycle the disputed request falls on | **0** |
| **8 722** | **`r $4015`** | **+1** |

Six writes before the fetch at 0 and the next event after it at +1. One fetch lies between them.
**This is a difference of two CPU accesses on each machine separately, so no assumption about which
cycle a DMA read is reported on enters it** — which is precisely what Q-182 asked for, and it is
answered against item 1c.

### And the source, re-read, does not refuse it

`RunCpuOne`'s closing line is the one the derivation missed:

    if (!cpu.RDY && !dmc_dma_exec && !oam_dma_exec) cpu.RDY = true;

`DMCUnit.Run()` lowers `RDY` inside `RunOneFirst()`, which runs **before** `cpu.ExecuteOne()` — but
`dmc_dma_exec` is still false on the request cycle, because only the countdown block sets it and the
block ran before the request existed. So `RDY` is **put back up at the end of the request cycle**,
the countdown block lowers it again on the next one, and the write microstep the derivation hung the
odd-slip cycle on is not the cycle that decides anything. The `+2` still fires when the CPU is
writing through the countdown (`$FCDC`'s `JSR` shows four re-reads where `$FCC8` shows two), and it
buys back exactly what letting the writes through cost — which is why the answer is **four at every
slip** and not a parity.

So item 1c is a **retraction**, in the shape lot NESHAWK-PROFILE-4 gave item 8: the constant and its
two sites go, the `else` arm — item 1b's four cycles — is what is left, and the reasoning stays
written down here and in `docs/review/NESHAWK-PROFILE.md` so that nobody derives it again.

### The item, as committed

| | |
|---|---|
| retracted | `TimingProfile::DMC_HALT_LANDS_ON_A_WRITE_CYCLE` and its two reads (`nes_bus.rs` `promote_dmc_wait`, `is_a_lone_dmc_fetch`). The retracted item's note stands where the constant was |
| enabled | `NesHawkCompat::DMC_POWER_ON_TIMER = 1019` (item 11), which lot NESHAWK-PROFILE-5 shipped at `0` and gated on Q-182 |
| the test | `the_reload_costs_four_cycles_at_every_slip_item_1c_retracted` replaces `item_1c_…`: the four slips a 6502 can produce answer **`[5, 5, 5, 5]`** on `NesHawkCompat` (the halted read's own cycle included) against `Standard`'s **`[5, 4, 5, 4]`**, so it is differential per slip and a regression of either profile fails it |
| gone with the wait | the `ItemElevenOn` test profile, which existed only to keep item 11 measured while it was off. `item_11_…` measures `NesHawkCompat` directly and pins `1019` |
| `Standard` | untouched: both removed reads were inside `if P::DMC_FETCH_HAS_TWO_DUMMIES`, which is `false` there |

`zax-nes`'s two profile suites: **11 passed, 1 ignored** and **6 passed**, 0 failed.

### *Kirby* on every window

`movie-sync --engine zaxnes --profile neshawk --no-lag --ram-ref <window>`, the figure being the
last frame with no live difference:

| window | frames | lot NESHAWK-PROFILE-5 (shipped) | **this lot** |
|---|---|---|---|
| `step1` | 1 - 400 | 400 | **400** |
| `step1b` | 400 - 1 100 | 1 100 | **1 100** |
| `step1c` | 1 100 - 10 000 | parted at **8 005** | **10 000** (first live difference 1 205, isolated; 8 830 of 8 901 frames clean) |
| `step1d` | 10 000 - 30 000 | — | **30 000** (555 frames identical to the byte) |
| `step1e` | 30 000 - 60 000 | — | **45 095** (4 763 identical to the byte) |
| `step1f` | 60 000 - 100 000 | — | none |
| `step1g` | 100 000 - 156 312 | — | none |

**The movie goes from 8 005 to 45 095**, 5.6 times further, and the parting frame is now in the
window the next step works on.

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE-6 step 1 done: item 1c retracted from measurement, item 11 on, Kirby 8 005 -> 45 095
