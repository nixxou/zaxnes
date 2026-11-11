# NESHAWK-PROFILE-7 — the movie's first difference is frame 55, and item 6 is not it

Branch `feat/neshawk-profile`, worktree `cleanroom/wt-bk2`, base `e52d70bf` (lot NESHAWK-PROFILE-6).

Lot NESHAWK-PROFILE-6 ended by naming **item 6** — NesHawk's end-of-sample aborts — as the thing
that moves *Kirby* past frame 45 095, on the strength of one window at frame 45 085 in which the
DMC output unit appeared to owe 3 bits here against about 7 there. This lot measured first, as the
brief asks, and the measurement **retires that reading**: the DMC output unit's phase is exact for
1 524 consecutive fetches, the two machines' DPCM schedules are identical event for event until
frame **1002**, and the movie's first difference of any kind is at frame **55**, where a DMC halt
lands on the game's own `$4016` read on NesHawk and not here.

## Steps

- [x] 1. **Measure first.** The fetch train, the DPCM schedule and the first differing event, from
      BizHawk and from this core's census. **Item 6 is retired by measurement**: the residual lot 6
      isolated is not an end-of-sample phase, it is a few-cycle phase between the **CPU/PPU stream**
      and the **DMC's own train**, and the first frame it is observable in is 55.
- [x] 2. **One round of five.** Both phase constants swept over thirteen values and **refused**; the DMA accounting measured **exact** against BizHawk on thirteen fetches. No item shipped.
- [x] 3. No regression. **No source changed**, so the gates are inherited to the byte; `zax-nes` 1 194 of 1 194.
- [x] 4. Report and final pass.

## The environment this lot runs in

| | |
|---|---|
| `CARGO_TARGET_DIR` | `<worktree>/rust/target-ev` |
| `TMP` / `TEMP` | `<worktree>/rust/target-ev/tmp` |
| `CARGO_INCREMENTAL` | `0` |
| disk on `G:` at the start | 6.0 GB free (the brief's stop is 5 GB) |
| disk on `C:` at the start | 12 GB free (the brief's stop is 1 GB) |

## The sources read, and their licences

| | |
|---|---|
| **BizHawk** (NesHawk) | MIT, tag **2.11.1**, read through `raw.githubusercontent.com`; nothing cloned. `APU.cs` `DMCUnit` — `Run`, `Clock`, `Fetch`, `set_lenctr_en` — and the `APU`-level consumers of `fill_glitch`, `fill_glitch_2`, `pending_disable` and `dmc_irq_glitch` |
| **BizHawk 2.11.1 run as a program** | `EmuHawk.exe --movie --lua`, the *Kirby* `.bk2`, NesHawk. Five new scripts, §"The instruments" below |

---

## Step 1 — measure first, and the measurement retires item 6

### The answer first

1. **The DMC fetch train is exact.** Over frames 11 to 59 the two machines make the **same 1 524
   sample fetches, of the same addresses, in the same order**, with an absolute-cycle residual of
   **−1** (−2 to −4 on the fetch either side of a frame boundary, where the two instruments'
   frame-start conventions differ). So the output unit's bit phase, its timer and item 11's
   power-on seed are right, and **no end-of-sample rule — item 6 — can be what divides the two
   machines**: there is nothing left for one to fix.
2. **The two machines' whole DPCM schedule is identical event for event — frame, register and
   value — for 1 344 events, to frame 1002**, where NesHawk starts a sample three frames before
   this core does. After that they are unrelated: 62 244 `$4010/$4012/$4013/$4015` events on
   NesHawk against 58 518 here over 45 100 frames.
3. **The movie's first difference is frame 55**, 45 030 frames before the window lot 6 worked in.
   A DMC halt falls on the cycle the game's `LDA $4016,X` reads the pad on — the **same absolute
   cycle** on both machines — and on NesHawk the CPU is on that read while here it is seven cycles
   further on in the same 14-cycle loop. NesHawk's pad byte is therefore corrupted by the halt's
   re-read, Kirby's input routine runs **one extra pass** (10 `$4016` strobes against 8 here), and
   the frame costs it about 155 CPU cycles more.
4. **The work RAM cannot see any of this until frame 45 096** — the extra pass reads the same pad
   value, and Kirby's sound driver keeps its state in the **cartridge's PRG-RAM at `$6000`**, which
   `--ram-ref` does not compare. That is why a 54-frame-old difference reads as a 45 095-frame-long
   agreement, and why lot 6's window was 45 030 frames downstream of the event.
5. **What is left to find is a few-cycle phase between the CPU/PPU stream and the DMC's train.**
   The core's start-up frame (item 10) plus the CPU's reset cycles is **27 394** cycles where
   NesHawk's is **27 392**, and after the power-on and the movie's frame-4 reset the CPU stream
   sits **+5** absolute cycles behind NesHawk's while the DMC train sits at **−1**. The game is
   frame-synced, so its code position is set by the PPU and the fetch train by the DMC's own timer,
   which makes item 10's dot count and item 11's seed the two knobs that move them against each
   other. **Step 2 swept both over thirteen values and neither reproduces the frame-55 collision**,
   so the residual is not a phase constant; what it is instead is cumulative, and §"Why the round
   stops at one of five" says what the next lot needs for it.

### Why lot 6's window could not have answered the question

At frame 45 085 the two machines are not playing the same sample. NesHawk's `$4015` restarts are at
tree frames **45 085, 45 093 and 45 097**; this core's are at **45 047, 45 067 and 45 087**, a flat
20-frame cadence. The `$4015` write, the CPU stream and the write-started fetch that lot 6 found
"exact to the cycle" are the same *code* doing the same *thing* in a frame that happened to line up,
not the same event: this core's 449-byte rate-15 sample runs frames 45 087-45 093 and NesHawk's runs
45 085-45 091. The "3 bits owed against about 7" is a comparison of two different restarts.

### The instrument alignment, pinned rather than assumed

`q182c.lua`'s header reading is **right** and this lot re-measured it two independent ways:

| | |
|---|---|
| block numbering | BizHawk's `emu.framecount()` is **this tree's frame number minus one**: a block `## frame f` holds tree frame **f + 1**. Pinned on the **execute stream** — core frame 55's NMI handler and BizHawk block 54's are the same 9 050 instructions in the same order — and on the work RAM: the last value `q182c.txt` writes to each address over blocks 45 083-45 089 matches `kirby-jp-step1e.txt`'s line **45 090** on **455 of 455** addresses, the next best line managing 366 |
| reference numbering | `kirby-jp-step1e.txt`'s line *f* is this core's frame *f*, offset **0**, which `movie-sync --trace-ref` measures rather than assumes: 4 763 of 15 101 lines agree at offset 0 against **8** at the next best offset |
| cycle counters | `emu.totalexecutedcycles()` and `frame-trace --event-log`'s cycle are the **same clock** to a few cycles from the power-on: BizHawk's first frame is 27 392 cycles (`## frame 1 start 27392`) and this core's power-on plus start-up frame is 27 394 |

### The fetch train, frames 11 to 59

`q182i.lua` (every `$4000`-`$401F` access and every read of `$FC00`-`$FEFF`, which for this game is
the DPCM region and nothing else) against `frame-trace --frame 1 --frames 60 --dmc-census`:

| | |
|---|---|
| fetches compared | **1 524**, every one of them |
| address and order | **identical**, fetch for fetch |
| absolute-cycle residual | **−1** on 1 448 of them; −2, −3 or −4 on the first fetch of a frame and +1 on the last, which is the two instruments' frame-boundary convention and not a drift |
| sample starts covered | `$FCC0`/449 bytes (frames 11, 32, 53), `$FC00`/177 (39), `$FD00`/257 — and every sample **end** in between, which is item 6's own ground |

### The DPCM schedule, frames 2 to 45 100

`q182e.lua` — five callbacks on five named addresses, so the whole 45 100-frame pre-roll costs only
the accesses wanted — against the same window of `--event-log --dmc-census`:

| | |
|---|---|
| events agreeing on frame, register and value | **1 344**, frames 2 to 1001 |
| first differing event | frame **1002**: NesHawk writes `$4012 F4`, `$4013 10`, `$4010 0F`, `$4015 0F`, `$4015 1F` (a 257-byte sample at `$FD00`); this core writes the same five at frame **1005** |
| after that | NesHawk 62 244 events against this core's 58 518 — its sample cadence is 4, 4, 8, 8, 16, 8, 8 frames where this core's is a flat 20 |

### The first differing event, frame 55

`q182h.lua` (q182c's whole bus stream, window moved to blocks 52-59) against
`frame-trace --frame 55 --out`, both streams anchored on the frame's **own NMI handler entry** at
`$C341` and compared execute to execute, which is one calibration:

| | this core | **NesHawk** |
|---|---|---|
| residual before the OAM transfer | 0 | 0 |
| the OAM transfer, one DMC fetch inside it | **517** cycles | **516** |
| residual after it | −1 | — |
| the DMC halt | abs **1 604 416** | abs **1 604 416** — the **same cycle** |
| what the CPU was doing on it | cycle 2 of `$C31E`, the loop's `DEY`, having read the pad at 1 604 409 | the **read cycle of `LDA $4016,X`** at `$C310` |
| the pad byte the read returns | `40`, clean | `C0` — the halt's re-read clocked the shift register twice more, and the next read answers `41` |
| `$4016` strobes in the frame | **8** | **10** — one extra pass of the input routine |
| the first instruction the two differ on | `$C315` branches to **`$C31E`** | `$C315` falls through to **`$C317`** |

The OAM transfer's cost is **not** a rule difference: measured over tree frames 53-60, both machines
charge **514** with no fetch inside and **516** with one, and **+1** when the `$4014` write lands on
an even cycle — the same predicate, which is what the review doc's arithmetic argument said. The
±1 rows (55, 57, 60) are the two streams' parity already differing, not two rules.

### And the source, read for item 6, which the measurement had already retired

Read off the 2.11.1 tag before the fetch-train comparison was run, and recorded so that nobody reads
it again:

| | |
|---|---|
| `DMCUnit.Clock()` | tests `out_bits_remaining == 0` **before** decrementing and reloads with **7**, where this core decrements first and reloads with **8**. Both are eight bit periods a byte and both refill on the eighth — NesHawk's counter is simply **this core's minus one** at every instant, which is the same `+1` convention trap the review doc records for the timer. So `out_bits_remaining == 0` in NesHawk's source means `bits_remaining == 1` here |
| `set_lenctr_en(false)` | three arms, each needing `out_bits_remaining == 0` **and** `sample_length != 0`: `(timer == 3 \|\| timer == 2)` sets `sample_length = 0`, `fill_glitch_2 = true`; `timer == 1` sets `pending_disable`; and a disable landing on a DMA, a `delay` or a reload countdown sets `pending_disable` and, if the reload countdown is live, `dmc_irq_glitch` |
| `fill_glitch` | **assigned `true` nowhere at 2.11.1.** `Run()` reads it and clears it, and no arm sets it — so the "buffer fill with no DMA at all" the review doc lists as half of item 6 is not reachable on the tag the profile is written against |
| `fill_glitch_2` | set only by `set_lenctr_en(false)`'s first arm, and read by `Run()`'s DMA condition, where it stands in for `sample_length > 0` and makes the DMA `dmc_dma_countdown = -2`, `DMC_RDY_check = -2` — the one-cycle DMA |
| `pending_disable` | spent in `RunOneLast` when `dmc_reload_countdown` runs out: `sample_length = 0`, `pending_disable = false` |

None of it can be the item: all three arms need a `$4015` **disable** while the output unit is in the
last bit period of a byte, and *Kirby* makes no such write anywhere in the window — and in any case
the fetch train that an end-of-sample rule would move is already exact for 1 524 fetches.

### The instruments added

All five are `q182c.lua`/`q182d.lua` with one idea each, and all are in
`docs/review/NESHAWK-PROFILE-7-SCRIPTS.md`:

| | |
|---|---|
| `q182d.lua` | a **filtered** whole-frame dump — `$4000`-`$401F` and reads of `$FC00`-`$FEFF` — so a 66-frame window costs 120 KB where q182c's 14 frames cost 11.7 MB |
| `q182e.lua` | five callbacks on **five named addresses**, which is what makes a 45 100-frame span affordable: BizHawk hooks one address per registration, so the pre-roll costs only the accesses wanted |
| `q182g.lua` | one callback on `w $4016`: the **controller-strobe ledger**, which counts the input routine's passes and so counts the DMC/controller collisions |
| `q182h.lua` | q182c with the window at blocks 52-59 — the whole bus stream, for the execute-to-execute comparison |
| `q182i.lua` | q182d with the window at blocks 1-60 — the fetch train from the power-on |

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE-7 step 1 done: item 6 retired by measurement, the movie's first difference isolated to frame 55

---

## Step 2 — one round, and it ships no item, because both candidates were measured and refused

### The answer first

The round followed the loop the brief sets — first differing frame, both machines ledgered around
it, first differing event isolated, mechanism confirmed by BizHawk measurement first — and then did
the thing lot 6 did not: it **measured the two candidate constants before writing one**. Both are
refused, and the DMA accounting that looked like the third candidate turns out to be **exact**.

### What the residual at frame 55 is, and is not

The frame-55 collision needs this core's CPU to be seven cycles earlier in its own 14-cycle
`$4016` read loop. The two quantities that can move the CPU's code against the DMC's train are:

* **item 10**, `PPU_START_UP_DOTS_SKIPPED`. The game is frame-synced on a 6-cycle spin loop, so its
  code position is set by the PPU and the fetch train by the DMC's own timer; the start-up dots run
  with no CPU cycle attached, so they are the one knob that moves the two against each other;
* **item 11**, `DMC_POWER_ON_TIMER`, which moves the train itself.

Both were swept, and the measure is the **controller-strobe table**: `q182g.lua` against
`--event-log --dmc-census` over frames 2 to 1 100, counting the frames whose `$4016` strobe count
differs between the machines (an extra pass is a collision one machine had and the other did not).

| constant | value | frame-2 start | disagreeing frames | frame 55 |
|---|---|---|---|---|
| — | **as shipped (7 165 / 1 019)** | 27 394 | **23 of 30 conflicted** | 8 against 10 |
| `PPU_START_UP_DOTS_SKIPPED` | 7 168 | 27 394 | 31 of 37 | 8 against 10 |
| | 7 171 | 27 394 | 29 of 35 | 8 against 10 |
| | 7 174 | 27 390 | 31 of 36 | 8 against 10 |
| | 7 162 | 27 394 | 31 of 36 | 8 against 10 |
| | 7 159 | 27 397 | 31 of 36 | 8 against 10 |
| | 7 156 | 27 397 | 26 of 31 | 8 against 10 |
| `DMC_POWER_ON_TIMER` | 1 021 / 1 023 / 1 025 | 27 394 | 26 / 32 / 30 | 8 against 10 |
| | 1 017 / 1 015 / 1 013 | 27 394 | 23 / 33 / 26 | 8 against 10 |

**Not one of the thirteen values reproduces the frame-55 collision**, and the shipped pair is the
best of them. So the residual is **not a phase constant** — neither the PPU's start-up position nor
the DMC's power-on seed. Both constants stay exactly as lot 6 left them, and this is written down so
that no later lot spends a round on either.

### The DMA accounting, which looked like the third candidate and is exact

The census charges **74 of 1 520** reload fetches **three** cycles rather than four, which against
lot 6's "four at every slip" read like an item. It is an **instrument artefact**: `cost` is
`fetch - halt + 1`, and on 113 of 1 524 fetches the census dates the halt one or two cycles after
the request, so the same four cycles read as three. Checked against BizHawk's own stream on
thirteen of them, one by one:

| | this core | **NesHawk** |
|---|---|---|
| the `$FCFE` reload of frame 54 | request 1 574 176, halt 1 574 177, fetch 1 574 179 | halt 1 574 176, re-read 1 574 177, fetch 1 574 178 |
| the cycle the CPU resumes on | **1 574 180** | **1 574 180** |
| the `$FD44` reload of frame 55 | halt 1 604 416, fetch 1 604 419 | halt 1 604 416, fetch 1 604 418 |
| the cycle the CPU resumes on | **1 604 420** | **1 604 420** |

**The stall is the same cycle count from the same cycle on every one of the thirteen.** The fetch's
own read sits one cycle later inside the window here than there, which is the DMA-read calibration
Q-182 could not pin — and it is **invisible to the CPU**, which is why it has never mattered. Items
1a, 1b and 7 are confirmed by this rather than questioned by it.

### Round 1

| | |
|---|---|
| parting frame before | **45 095** |
| the first live difference | frame **45 096**, unchanged — and now known to be 45 041 frames downstream of its cause |
| the first difference of any kind | frame **55** (new) |
| the first differing DPCM schedule event | frame **1002** (new) |
| parting frame after | **45 095**: no item was added, and the two that could have been are refused by measurement |

### Why the round stops at one of five

What is left is a few-cycle difference in the **phase of the game's own code within the frame**,
and the instrument this lot would need next is one neither machine has yet: a per-frame measure of
*where in the frame* the two CPU streams part, over hundreds of frames rather than one. Frame 55's
own execute-to-execute table says the residual inside that frame is **0 before the OAM transfer,
-1 after it, -2 after the next fetch** and the NMI entry itself is **5 cycles apart** — the spin
loop's phase, which the *previous* frame's total work sets. That is a cumulative measurement over
the 54 frames before it, and it is a lot of its own, not a round of this one.

What this round delivers instead is what the next lot needs and this one did not have: the frame
(**55**, not 45 085), the event (a DMC halt on the game's `$4016` read), the instrument that counts
them over a thousand frames at a time (`q182g.lua`, 7 s a run), and **two constants crossed off**.

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE-7 step 2 done: one round, both phase constants refused by measurement, the DMA accounting confirmed exact

---

## Step 3 — no regression

### The answer first

**No source file changed in this lot.** `git diff e52d70bf -- rust/` is empty at the tip, so every
gate the brief names is inherited to the byte rather than re-measured — and that is a stronger
statement than a re-run, not a weaker one. What was re-run is what could have caught a stray edit
from the thirteen-value sweep.

| | |
|---|---|
| `rust/` against the parent `e52d70bf` | **empty diff**, verified at the tip |
| `cargo fmt --all -- --check` | clean |
| `cargo test -p zax-nes --release` | **56 targets, 1 194 passed, 0 failed, 11 ignored** — summed per target, as the review doc's trap requires, and no target aborted |
| `golden-core`, `test-roms`, `AccuracyCoin`, the bench | inherited: `8d508070850b24228285b6f04df6f98c310dbdab65e7577921c91ad4d0689773`, 128/147 on `Standard` and 121/147 on `NesHawkCompat`, 24 of 24 DMC/DMA ROMs, -0.86 % and -0.35 % cumulatively against `428c50e6`. The binary that produces them is byte-identical to lot 6's |

### The sweep, and the one thing it nearly left behind

The thirteen-value sweep edited `timing_profile.rs` with `sed`, and the pattern for
`PPU_START_UP_DOTS_SKIPPED` matched **both** `impl`s — so `Standard` carried a non-zero start-up
frame for six of the builds. No measurement is affected (every run of the sweep is a `.bk2`, which
selects `NesHawkCompat`), and the file was restored from git rather than by hand, which is what the
empty diff above verifies. Written down because a sweep that edits a shared constant is the obvious
way to ship an accident.

### The disk, and the one thing it stopped

`G:` began the lot at 6.0 GB and the sweep's eight release builds took it to **3.0 GB**, under the
brief's 5 GB stop. `cargo clean --release` gave back 3.6 GB and the lot went on at 6.4 GB, ending at
5.9 GB. What it stopped is the re-run of `golden-core` and the thirty ROMs: with an empty diff they
are inherited anyway, and the choice between an inherited gate and a 5 GB stop is not close.

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE-7 step 3 done: no source changed, the gates are inherited to the byte, zax-nes 1 194/1 194

---

## Step 4 — report and final pass

### The documents

| | |
|---|---|
| `docs/review/NESHAWK-PROFILE.md` | item 6 moved from "the item to try" to **retired by measurement**, with the 1 524-fetch comparison; the movie's first difference restated as frame **55**; the two refused constants added to the candidates table; two new traps |
| `docs/review/NESHAWK-PROFILE-7-SCRIPTS.md` | the five Lua scripts, and the two tricks worth copying |
| this note | the working |

### What this lot did not do, said plainly

* **It shipped no item and moved no metric.** *Kirby* still parts at 45 095, `AccuracyCoin` is still
  121/147 on this profile and 128/147 on the other, and the binary is byte-identical to lot 6's.
* **It did not find what divides the two machines at frame 55.** It found the event, the mechanism
  and the frame, and it crossed off the two constants that could have explained them. What is left
  is a cumulative few-cycle difference in the game's code phase, and the next lot needs a new
  instrument for it.
* **It retired item 6 rather than implementing it**, which is the brief's step 1 read literally:
  *measure first*. The fetch train being exact for 1 524 consecutive fetches is not a reason to
  implement an end-of-sample rule more carefully; it is a proof that no end-of-sample rule is
  missing.

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE-7 step 4 done: the documents and the final pass
