# NESHAWK-PROFILE-12 — item 14 ships, and the hunt resumes at tree frame 38

Branch `feat/neshawk-profile`, worktree `cleanroom/wt-bk2`, base `52dfcbbf` (lot NESHAWK-PROFILE-11).

## Steps

- [x] 1. **Ship item 14 at −1** — measured by lot 11, costless, and the clock every later
      measurement of this lot is taken against. First difference of any kind: tree frame **38**.
- [x] 2. **Tree frame 38** is the **OAM/DMC interleave**, the candidate the brief named — item 13's
      unmeasured half. **Item 15**: NesHawk services a DMC fetch that falls inside an OAM transfer
      from the **request cycle itself**, so its fetch-read spacing stays the sample period across a
      transfer where this core's loses a cycle on each in-transfer fetch. Measured off BizHawk's
      own stream with no calibration, read in `RunCpuOne` second, and it takes frame 38's residual
      from **+2 to +1**. It costs nothing anywhere and moves no RAM byte.
- [ ] 3. **Rounds** — next first difference, same loop, up to five rounds, *Faxanadu* re-checked
      after every item.
- [ ] 4. **No regression** and the final pass.

## The environment this lot runs in

| | |
|---|---|
| `CARGO_TARGET_DIR` | `<worktree>/rust/target-ev` |
| `TMP` / `TEMP` | `<worktree>/rust/target-ev/tmp` |
| `CARGO_INCREMENTAL` | `0` |
| disk at the start | `G:` 31 GB free (stop at 5), `C:` 8.3 GB (stop at 1) |

---

## Step 1 — item 14 ships at −1

### The answer first

`NesHawkCompat::SOFT_RESET_PPU_DOT_BIAS` is **−1**, where lot 11 left it at `0` for want of a
source reading. Nothing else changed: the site, the constant, the mechanism and the differential
test are lot 11's, and the test was already parametric, so shipping the item moved its **value**
there and not its logic.

**Why it ships now, when item 11's shape ships off.** The two are not the same shape at all. Item
11 is right in isolation and **expensive** — it costs the movie 7 080 frames and an `AccuracyCoin`
row. Item 14 is right in isolation and **free**, structurally: the site is the soft reset, which
only a movie's RESET column reaches, so every ROM of every suite runs the code it ran before. The
one thing against it is that the reading is open, and that is a reason to keep reading rather than
a reason to run a clock known to be one dot out — which is the operative point for this lot, since
an item hunt whose window opens on such a clock measures that dot and not the item.

### The measurement, re-taken on this tree

`frame-trace --frame 1 --frames 80 --profile neshawk --dmc-census --event-log --event-pc C06E
--event-pc C37C --event-pc 80FC --event-pc 804C` against `bizhawk-runs/kirby-jp-cyc0.txt`, every
event on its **absolute** cycle (`marks.py`):

| | shipped `0` (lot 11) | **shipped `−1` (this lot)** |
|---|---|---|
| frames in step on every event | 3-13 | **3-37** |
| first difference of any kind | tree frame **14**, −6 | tree frame **38**, **+2** |
| frame 38's residual | — | +2 on 11 events of 14, +0 on 3 |
| the first non-zero event | frame 14's `nmi` | frame 38's `pc$C37C`, +2 |
| `item_14_…` differential test | 0 vs 0 | **−1 vs 0**, PPU 7 158 dots apart, pass |

The reading is recorded as still open in the constant's own note, with **`PPU.cs` `Reset()`'s
`idleSynch = true`** named as the best candidate and the reason it does not yet predict the item
written beside it: implemented faithfully its effect **alternates** 0/−1 rather than accumulating,
because both machines then skip once every two frames, so what is missing is whatever makes the
first skip after the reset land on only one of them.

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE-12 step 1/4 done: item 14 ships at -1 dot and the first difference is tree frame 38

---

## Step 2 — round 1: tree frame 38 is the OAM/DMC interleave (item 15)

### The answer first

**The brief's named candidate is the answer, and half of frame 38's two cycles are now gone.**
Tree frame 38's residual has a shape no other difference in this movie has had: the frame's `nmi`
entry and its `$4014` write are at **0**, every event from `pc$C37C` to `r$4015` is at **+2**, and
the frame's own `pc$C06E` is back at **0**. So two cycles are spent inside the OAM transfer and
given back before the frame ends — a DMA cost, in the one window where a DMA is running.

**Item 15, measured off BizHawk's own stream and with no cross-machine calibration at all.** The
DMC's requests are spaced by the sample period, so the interval between consecutive **fetch reads**
says where each fetch sits relative to its own request, *inside one machine*:

| | FE56→57 | 57→58 | 58→59 | 59→5A | 5A→5B |
|---|---|---|---|---|---|
| NesHawk (`q191.lua`) | 432 | 432 | 432 | 432 | 432 |
| this core, before | 432 | **431** | 432 | 432 | 432 |
| this core, with item 15 | 432 | 432 | 432 | 432 | 432 |

The two fetches NesHawk makes **inside** tree frame 38's transfer sit at the same offset from their
request as the three it makes outside it. This core's were each one cycle late, twice — and those
two cycles are the whole of the frame's +2.

**The mechanism, off the bus and then out of the source.** At the fetch inside the transfer
NesHawk's bus carries, cycle by cycle: `r 0228`+`r C363`, `w 2004`+`r C363`, `r FE56`, nothing,
`r 0229`. The halt and **both** re-reads ride on cycles the transfer was going to run anyway,
beginning on the request cycle itself; only the fetch and the realign cost it, which is item 7.
`NES.Core.cs` `RunCpuOne` is why: its OAM block and its DMC block are two independent blocks of the
*same* cycle, and the OAM block is skipped only on the fetch's cycle
(`oam_dma_exec && apu.dmc_dma_countdown != 1 && !dmc_realign`) and on the realign's. This core's
transfer loop can only date a halt on the cycle it is **about to** run, and a request is raised
from inside one of the loop's own `tick`s, so its halt landed one cycle late and the fetch with it.
One site, `nes_bus.rs` `promote_dmc_wait`: inside a transfer the fetch owes two cycles, not three.

### What it is worth

| | before | with item 15 |
|---|---|---|
| tree frame 38's residual | **+2** on 11 events of 14 | **+1** on 11 of 14 |
| first difference of any kind | tree frame 38 | tree frame **38** (unmoved) |
| frames in step on every event | 3-37 | 3-37 |
| `golden-core` | `8d508070…` | **`8d508070…`** byte-identical |
| `test-roms --profile standard` | 212 pass / 7 fail | **212 / 7** |
| `test-roms --profile neshawk` | 205 pass / 14 fail | **205 / 14** |
| *Faxanadu* USA / Rev 1 | 202 / 210 of 832 | **202 / 210** |
| `step1` / `step1b` / `step1c` identical to the byte | 219 / 342 / 459 | **219 / 342 / 459** |
| the parting frame | 9 353 | **9 353** |

**It is exactly inert on the work RAM** — every window byte-identical, measured by building the
item off and on — and the control that the knob is nevertheless live is the ledger itself: frame
38 goes from +2 to +1. That is the same shape as item 14: right, free, and buying the clock rather
than the work.

### The differential test

`item_15_a_fetch_inside_an_oam_transfer_is_serviced_from_the_request`. The fetch's own cycle is not
observable from outside a halted read, so the test measures what is: the cycles a transfer costs,
against **each profile's own** no-fetch base at the same `$4014` parity (the trap item 7's test
fell into). The `delay` sweep moves the write against the DMC's periodic request on **write**
cycles, which no DMA unit may halt, so the request's cycles do not move with it. Over the sixteen
phases where two requests fall inside one transfer:

| | 345 | 346 | 347 | 348 | 349-360 |
|---|---|---|---|---|---|
| neshawk, item 15 **on** | 4 | 4 | 4 | 4 | 4 |
| neshawk, item 15 **off** | 5 | 5 | 3 | 3 | 4 |
| standard | 3 | 3 | 4 | 4 | 4 |

Two cycles per inside fetch at **every** phase with the item on, where without it the halt's late
landing shows as 5 and 3; and `Standard`'s own column is asserted beside it, so a regression of the
standard path fails the test rather than passing it quietly.

### What is left at frame 38, and it is a second item

The remaining **+1** is *not* the same thing, and the sweep says so: the phases where the NesHawk
profile charges **three** cycles for an inside fetch (`+6` for two) read exactly the same with item
15 off, so they predate it. Frame 38's second fetch comes due at **+517** cycles from the `$4014`
write, past the transfer's nominal 514 — in its *last* cycles, where frame 37's second fetch
(`+491`, and frame 37 is in step both before and after) is comfortably inside. BizHawk's own stream
measures that case directly: with a fetch at the end the last `w 2004` is followed by the fetch,
one idle cycle and then the CPU (frame 38, cycles 1 097 681-1 097 684), where with no fetch at the
end it is followed by one idle cycle and then the CPU (frame 36, 1 067 901-1 067 903). So **a fetch
whose own cycle is the transfer's last costs NesHawk one cycle, not two**: `dmc_realign` only skips
the OAM block, there is no OAM block left to skip, and item 13's trailing wait cycle is spent
either way — the realign and the wait are the same cycle. That is round 2's item.

### The instruments

| | |
|---|---|
| `q191.lua` | the whole bus stream of BizHawk frames 35-37 (tree 36-38), q187's format. 2.2 MB, an intermediate |
| `rust/target-ev/tmp/ev38.py` | `marks.py`'s per-event residual for a named frame range, which is what showed frame 38's 0 / +2 / 0 shape. A scratch instrument, not committed |

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE-12 step 2/4 done: item 15 -- a fetch inside an OAM transfer is serviced from the request cycle
