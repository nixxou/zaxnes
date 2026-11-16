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
- [x] 2. Rounds against the references. **One of five.** It isolates the residual to the DMC
      **output unit's bit phase** at a `$4015` restart — not a DMA cost — and names **item 6** as
      what moves it. *Kirby* parts at **45 095**.
- [x] 3. No regression. `Standard` **byte-identical** and **faster** than `428c50e6`; the NesHawk
      profile keeps every ROM NesHawk passes and loses **one** `AccuracyCoin` row, named.
- [x] 4. Report and final pass.

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

---

## Step 2 — one round, and it isolates the next item rather than shipping one

### The answer first

The round ran the loop the brief sets — references in order, first live-difference frame, both
machines ledgered around it, first differing event isolated, mechanism confirmed by **BizHawk
measurement first** — and it ends one step short of an item, for a reason worth more than a
guess: the residual is **not** a DMA *cost* any more. It is the DMC **output unit's bit phase**,
which has drifted **five bit periods** by frame 45 085 while the CPU stream is still exact to the
cycle. That is **item 6**'s ground — the end-of-sample aborts — which
`docs/review/NESHAWK-PROFILE.md` already names as "the item to try" for a game at rate 14-15 with
a short sample, which is exactly what this is.

### Round 1

| | |
|---|---|
| parting frame before | **45 095** (`step1e`, after step 1) |
| the first live difference | frame **45 096**: 22 live bytes at once, all in `$061F`-`$06A1` — the object table — and from 45 098 the game's own counters run **one apart** (`$0647` 17 against 18, `$0665` FD against FE), so it is one event's worth of timing and not a divergence of state |
| the lag count | **15 on both machines** across the boundary, so it is not a lag frame |
| parting frame after | **45 095**, unchanged: no item was added |

### The ledgers, and what they say

`bk2cyc3.lua` (a copy of `bk2cyc2.lua` with the window moved to frames 45 080-45 110; callbacks
registered once and gated, so the 45 000 frames before it cost one short Lua call per watched
access) against `frame-trace --event-log --dmc-census` over the same frames. Residuals are
NMI-anchored per frame, and a register write's own calibration is **+1**:

| frames | what the ledger says |
|---|---|
| 45 080 - 45 083 | **exact**, every event, register writes at +1 and the `$4016` strobes with them |
| 45 084 | the frame the sample is **about** to start: `pc$C3A1` −1, `pc$C074` −4 |
| 45 085 - 45 096 | the `$4016` strobes jitter by **±4 cycles** — individually, not cumulatively (frame 45 089 reads +5 +5 +5 +5 +1 +1 +5 +5) — and `pc$C3A1`, the NMI handler's `RTI`, by −4 to +5, with one frame at **−325** |
| 45 097 | the register **sequence** itself parts: this core writes `$4008` where NesHawk writes `$400A`/`$400B`, so the music driver has taken a different path |
| the frame it starts in | **45 085**, the frame whose NMI handler writes `$4012 F3`, `$4013 1C`, `$4010 0F`, `$4015 0F`, `$4015 1F` — a 449-byte DPCM sample at **rate 15**, 54 cycles a bit, restarted by a `$4015` write |

Frames 45 080-45 084 play **no DPCM at all** (`dmc=0/0`) and are exact. The jitter begins on the
cycle the sample does.

### The first differing event, measured

`q182c.lua` — `q182b.lua` with the callbacks registered **late**, which is what makes a window at
frame 45 000 affordable at all (q182b's pre-roll would be four billion Lua calls; this one advances
with nothing registered and arms at the window) — dumped BizHawk's whole bus stream for frames
45 083-45 096. Aligned per frame on the `$4017` write the NMI handler makes:

| | this core | **NesHawk** |
|---|---|---|
| the `$4015` write that restarts the sample | aligned to the cycle | — |
| the write-started fetch of `$FCC0` (item 3) | cycle 2 414 | the same cycle, **±1** |
| the **next** fetch, `$FCC1` | **111 cycles later** | **372 cycles later** |
| so the output unit's bits owed at the restart | **3** (`dmc(1,3,449)` at the halt) | **~7** (372 / 54) |
| every fetch after it | 432 cycles apart | 432 cycles apart |

**The fetch train's period agrees and its phase does not.** The `$4015` write, the CPU stream and
the write-started fetch are all exact; what differs is how many bits the output unit still owed
when the write filled the buffer — five bit periods, 270 cycles. It is not a cost and no cost item
can move it.

### Why the round stops here, and what the next one does

The phase of the output unit's bit counter is set where a sample **ends**: `DMCUnit.Clock`'s
behaviour with an empty buffer, and `set_lenctr_en(false)`'s three arms — `fill_glitch`,
`fill_glitch_2` and `pending_disable` — which are **item 6**, read in lot NESHAWK-PROFILE step 1
and deferred with a written reason: *"Eliminated by arithmetic for Kirby, not by choice… For a game
at rate 14-15 (72 or 54 cycles a bit) with a short sample they are reachable, and this is the item
to try there."* Frame 45 085 is a rate-15 sample restarted mid-byte, so the arithmetic that
eliminated them for frames 1-10 000 does not hold here, and the deferral's own condition is met.

Implementing item 6 is a change to `DmcChannel`'s end-of-sample state and its savestate, with the
thirty DMC/DMA ROMs and `AccuracyCoin` to re-measure on **both** profiles; it is a lot of its own,
not a round of this one. What this round delivers instead is the thing the next lot needs and did
not have: the frame (45 085, not 45 096), the quantity (the output unit's bits owed at a `$4015`
restart: 3 against ~7), the instrument that measures it (`q182c.lua`, which can be pointed at any
frame), and the item's name.

So the rounds stop at **one of five**, with the movie at **45 095**.

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE-6 step 2 done: one round, the residual isolated to the DMC output unit's bit phase, item 6 named

---

## Step 3 — no regression

### The answer first

The `Standard` profile does not move, in behaviour or in speed. The `NesHawkCompat` profile keeps
everything NesHawk itself passes and loses **one** `AccuracyCoin` row, named and traced below —
which is the retraction showing up exactly where a retraction of a DMA-cost item should.

### The `Standard` profile: nothing moved

| | |
|---|---|
| `golden-core`, 98 originals × 1 200 frames, release, feature off | `8d508070850b24228285b6f04df6f98c310dbdab65e7577921c91ad4d0689773` — **byte-identical** to lot FCEUX-23's baseline and to all five earlier NESHAWK lots' |
| the 30 ROMs of `dmc_dma_during_read4`, `sprdma_and_dmc_dma`, `apu_test`, `dmc_tests` | **24 pass, 0 fail, 6 manual** — the same column, suite for suite (3+2+19 pass; 2+4 manual) |
| `ppu_vbl_nmi` / `vbl_nmi_timing` / `cpu_interrupts_v2` | **11/11, 7/7, 6/6** |
| `AccuracyCoin` | **128/147**, the same nineteen rows failing |
| *Faxanadu*, `[USA][!!]` and `[USA][Rev 1][!!]` | **207** and **220** of 832, every compared lag count agreeing |
| savestates | unchanged to the byte and to the version: the retraction **removes** a constant and adds no field, and item 11 never had one |

That `golden-core` is byte-identical is the whole argument, and it is cheap to believe: both removed
reads sat inside `if P::DMC_FETCH_HAS_TWO_DUMMIES`, which is `false` on `Standard`, so the compiler
had already folded the arm away.

### The `NesHawkCompat` profile

| | lot NESHAWK-PROFILE-5 | **this lot** |
|---|---|---|
| `ppu_vbl_nmi` / `vbl_nmi_timing` / `cpu_interrupts_v2` | 11/11, 7/7, 6/6 | **11/11, 7/7, 6/6** — the brief's precondition, met |
| the 30 DMC/DMA/APU ROMs | 19 pass, 5 fail, 6 manual | **19 pass, 5 fail, 6 manual**, ROM for ROM |
| `AccuracyCoin` | 122/147 | **121/147** |
| *Faxanadu*, both dumps | 247 and 314 of 832 | **247** and **314** |
| *Kirby*, the seven windows | 400 / 1 100 / 8 005 / — / — / — / — | **400 / 1 100 / 10 000 / 30 000 / 45 095** / none / none |

### The one row that moved, named rather than rounded

The parent's own two source files (`timing_profile.rs`, `nes_bus.rs` at `7eb5eceb`) were checked
out, built and run against this lot's, so the delta is a measurement and not a subtraction of two
notes:

| | |
|---|---|
| the row | **`DMA + $4015 Read`** (page "APU Registers and DMA tests") — the only one. `DMA Landing on Write` and `DMC Reload Timing`, the two whose names sound like item 1c, were **already failing** at `7eb5eceb` |
| why it is expected | the row times what a `$4015` read sees across a DMA, and the retraction changes a reload's cost by one cycle at an odd slip. `Standard` still **passes** it (128/147, the same nineteen rows) |
| the gate the recipe sets | *"run the public ROMs that measure the quantity your item is about on NesHawk itself; if NesHawk passes one that your item makes the profile fail, stop."* **It does not fire**: NesHawk's own `AccuracyCoin` run (`bizhawk-runs/coin-neshawk.txt`) scores 67/147 and leaves **69 rows `NOT RUN`**, `DMA + $4015 Read` among them — NesHawk never reaches that page, so it offers no evidence either way, and what decides is the direct measurement of NesHawk's own bus stream in step 1 |
| the trade | one `AccuracyCoin` row of the profile that is **not** the accurate chip (it already fails six others `Standard` passes), against **37 090 frames** of the one movie the profile exists for |

### The `Standard` profile's speed

`bench-core`, default 1 200 frames after 120 of warm-up, feature off, **interleaved** parent/this
round by round, three rounds each, against a clean `git archive` of **`428c50e6`** — the parent the
whole series is measured from — extracted and built on `C:` (see the disk note), and deleted after:

| | parent 428c50e6 | this lot | |
|---|---|---|---|
| Super Mario Bros. 3 `(USA)` | 1.2914 / 1.2992 / 1.3183 → **1.30297** | 1.2863 / 1.2895 / 1.2993 → **1.29170** | **−0.86 %** |
| Mega Man 3 `[USA][!!]` | 0.9477 / 0.9416 / 0.9441 → **0.94447** | 0.9449 / 0.9427 / 0.9360 → **0.94120** | **−0.35 %** |

Both **negative** and well inside the +1 % cumulative budget, and the sign is the expected one: the
retraction takes two folded-away comparisons out of `promote_dmc_wait` and one out of
`is_a_lone_dmc_fetch`, and nothing was added to a per-cycle path. This core beats `428c50e6` on both
ROMs, as it did at the end of lot NESHAWK-PROFILE-5.

### The disk, and the one thing it stopped

**`G:` fell to 1.5 GB during step 2, under the brief's 5 GB stop.** 521 MB of regenerable
`target-ev/debug` and the temp directory were deleted, which brought it to **2.0 GB** — still under
the stop, so **no further build was started on `G:`**. What that would have blocked is the bench's
parent tree, which needs a second full release build; it was run on **`C:`** instead (12 GB free,
the brief's stop being 1 GB), with the `git archive` tree, its `CARGO_TARGET_DIR` and its `TMP` all
there, and all three deleted afterwards. So every suite the brief names was run; the only thing the
disk changed is **where** the parent was built. `C:` ended at 12 GB and `G:` at 2.0 GB.

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE-6 step 3 done: Standard byte-identical and faster, one AccuracyCoin row named

---

## Step 4 — report and final pass

### The documents

| | |
|---|---|
| `docs/review/MOVIE-SYNC.md` | **§4k** added — Q-182 answered, item 1c retracted, item 11 on, frame 45 095, and what the movie parts on now. The short version's *Kirby* paragraph updated to 45 095 |
| `docs/review/NESHAWK-PROFILE.md` | the banner, the three-sentence summary, the item table (item 1c's row **gone**, item 11's marked on), the "which item closed *Kirby*" table (a frame-45 095 row), the candidates table (item 1c moved there as a **retraction**, with what the derivation missed), the `AccuracyCoin` history (121/147 and which row), three new traps, and the cost-of-two-chips table |
| `docs/review/NESHAWK-PROFILE-6-SCRIPTS.md` | **new**: `q182b.lua` and `q182c.lua` in full, `bk2cyc3.lua`'s three changed lines, the census column's contract, and how to read a reload out of the stream (an opcode fetch is reported twice, a DMA re-read only once, and a write shares its cycle with the re-read the `+2` is paying for) |
| `spec/changes/Q-182.md` | **not touched** — this lot's rules say not to edit `spec/`. The answer is in §4k, in this note and at the retracted item's place in `timing_profile.rs`; closing the ticket is the owner's |

### fmt, clippy and the suites

| | |
|---|---|
| `cargo fmt -p zax-nes -p zax-harness -- --check` | clean |
| `cargo clippy -p zax-nes -p zax-harness --release --all-targets -- -D warnings` | clean; and again with `--features dma-census`, which is the only build the new census column exists in |
| `cargo test -p zax-nes -p zax-harness --release` | **75 targets, 1 321 passed, 0 failed, 12 ignored** — counted by summing the per-target `test result:` lines and grepping for `failed` and `FAILED`, which is the trap NESHAWK-PROFILE-4's short count set |
| the same `--features zax-harness/fceux-core` | **75 targets, 1 331 passed, 0 failed, 22 ignored** |
| the same `-- --ignored`, with `ZAXNES_ROMS`, `ZAXNES_TEST_ROMS` and `ZAXNES_PROFILES` set | **12 passed, 0 failed** |

### The brief's rules, checked

| | |
|---|---|
| the `Standard` path must not change behaviour or speed | `golden-core` **byte-identical**; the bench **−0.86 %** and **−0.35 %** against `428c50e6`. Both removed reads were inside a `false` constant's arm, so nothing was emitted to remove |
| safe Rust only | no `unsafe` added; `zax-nes` keeps `#![forbid(unsafe_code)]` |
| generics and associated constants, no runtime flags in hot paths | the lot **removes** an associated constant and adds none. Item 11 is read at one site, `DmcChannel::reset`, once per machine |
| diagnostic hooks behind an off-by-default cfg/feature | the census's new `pc$XXXX gap=` group is inside `#[cfg(feature = "dma-census")]` code, on a `DmcFetch` line alone; nothing else was added |
| no downloads | none. BizHawk read through `raw.githubusercontent.com` and run from `ext-tools` |
| nothing staged that must not be | the four commits carry source, tests and documents only. `git status` shows `rust/target-ev/` untracked and nothing else |
| no user-name paths in committed files | the Lua scripts' own `G:/3DSen/...` output paths are the owner's shared folders, not a user directory; the scratch paths used while measuring are in no committed file |

### What this lot did not do, said plainly

* **It did not reach frame 156 312**, and it stopped the rounds at one of five. The reason is in
  step 2: the residual stopped being a DMA cost, and what it became — the DMC output unit's bit
  phase at a `$4015` restart — is **item 6**, a lot's worth of work on `DmcChannel`'s
  end-of-sample state and its savestate with thirty ROMs to re-measure on both profiles. A second
  round would have re-measured the same five bit periods.
* **It did not close Q-182's ticket**, only answered it; `spec/` is out of this lot's bounds.
* **It did not keep item 1c under question.** The item is gone, not disabled: a constant whose
  value is `false` on both profiles is not an item, and the whole derivation is preserved in the
  candidates table so that nobody re-derives it.
* **The one `AccuracyCoin` row is a real cost and is not smoothed.** `DMA + $4015 Read` was
  passing on the NesHawk profile and now fails. `Standard` keeps it, NesHawk's own run never
  reaches that page, and the trade is 37 090 frames of the movie the profile exists for.

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE-6 step 4 done: the documents, the Lua texts, and the final pass
