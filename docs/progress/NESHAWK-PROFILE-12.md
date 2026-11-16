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
- [x] 3. **Round 2** — **item 16**, item 15's other end: a fetch standing on the transfer's own
      wait cycle costs it **one** cycle, not two. Frame 38 closes completely, and every event of
      tree frames **3 to 40** is now in step. The first difference of any kind is tree frame 40
      and it is **no longer a cycle displacement at all** — this core runs an extra `$4016` input
      pass there.
- [x] 4. **No regression**: `golden-core` byte-identical, every suite on its documented figure,
      2 503 passed / 0 failed, *Faxanadu* and every *Kirby* window unmoved, and the bench
      **faster** than `428c50e6` on both ROMs (-4.23 %, -2.59 %).

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

---

## Step 3 — round 2: item 16 closes tree frame 38, and the wall changes character

### The answer first

**Item 16 is item 15's other end, and frame 38 is now closed.** The residual item 15 left was not
the same thing, and the sweep said so before the item was written: the phases on which the NesHawk
profile charges three cycles for an inside fetch read exactly the same with item 15 off, so they
predate it. What separates frame 38 from frame **37** — same instruction, same two inside fetches,
frame 37 in step throughout — is *where* the second fetch comes due: frame 38's is at **+517**
cycles from the `$4014` write, past the transfer's nominal 514, and frame 37's at **+491**.

**BizHawk measures that case directly, both halves inside one machine:**

| | last `w $2004` | then | then | the CPU runs |
|---|---|---|---|---|
| a fetch at the end (tree frame 38) | 1 097 681 | `r $FE57`, the fetch | one idle cycle | 1 097 684 |
| no fetch at the end (tree frame 36) | 1 067 901 | one idle cycle | — | 1 067 903 |

So the fetch adds **one** cycle there, not two. Execute to execute across the whole transfer
(`x $C360` to `x $C365`) NesHawk spends **523** cycles on tree frame 38 and **524** on tree frame
37 — one cycle apart on the same instruction with the same two fetches, which is this item and
nothing else. This core spent 524 on both.

**The reading agrees and is short.** Item 7's second cycle is `dmc_realign`, and all `dmc_realign`
does in `RunCpuOne` is skip the **OAM block** on the cycle after the fetch; once the 512th access
has run, `oam_dma_exec` is already false and there is no block left to skip, so the realign costs
nothing. What the CPU still loses is item 13's own cycle — and the fetch is standing on it. One
site, `nes_bus.rs` `run_dma`: the trailing `tick` is not spent when a fetch has already spent it.

### What it is worth

| | item 15 only | **with item 16** |
|---|---|---|
| tree frame 38's residual | +1 on 11 events of 14 | **0** |
| frames in step on every event | 3-37 | **3-40** |
| first difference of any kind | tree frame 38, a cycle residual | tree frame **40**, an extra `$4016` pass |
| `golden-core` | `8d508070…` | **`8d508070…`** byte-identical |
| `test-roms --profile standard` | 212 / 7 | **212 / 7** |
| `test-roms --profile neshawk` | 205 / 14 | **205 / 14** |
| *Faxanadu* USA / Rev 1 | 202 / 210 | **202 / 210** |
| `step1` / `step1b` / `step1c` identical | 219 / 342 / 459 | **219 / 342 / 459** |
| the parting frame | 9 353 | **9 353** |

Like items 14 and 15 it is **exactly inert on the work RAM** and costs nothing; what it buys is
the clock.

### The two items' tests, and the trap they walked into

Item 16 **moved item 15's test**, which is the trap `docs/review/NESHAWK-PROFILE.md` records as
*"a differential test of one item can be an instrument for another"* — item 1b's test had to move
to the even slips when item 1c shipped, and this is the same shape. At item 15's phases 345 and 346
the second fetch lands on the transfer's wait cycle, so item 16 fires there too and those phases
read 3 where item 15 alone reads 4. **Item 15's test now runs the part of the sweep item 16 leaves
alone**, phases 347-360, where it reads 4 with the item and 2 without it either way.

No lead isolates item 16 from item 15 — item 15 moves *where* the fetch lands and item 16 prices
the last place it can land — so item 16's test pins the **shipped** vector and carries both
off-tables in a comment beside it:

| leads 505-514 | |
|---|---|
| both items on (shipped) | `[2, 2, 2, 2, 2, 2, 2, 1, 1, 0]` |
| item 16 off, item 15 on | `[2, 2, 2, 2, 2, 2, 2, 2, 2, 0]` |
| item 16 on, item 15 off | `[2, 2, 2, 2, 2, 0, 0, 2, 2, 0]` |
| `Standard`, unchanged by either | `[2, 2, 2, 2, 2, 2, 1, 1, 3, 3]` |

The last row is what keeps both tests differential rather than pins: a regression of the standard
path fails them.

### Round 3's window, and why it is a different hunt

The first difference of any kind is tree frame **40**, and for the first time in five lots it is
**not a cycle residual**: every event of frames 3-40 agrees on its absolute cycle, and the
instrument stops on an **event-stream** difference — BizHawk writes `$4017` where this core writes
a sixth `$4016`. This core runs **six** strobe pairs in frame 40 (frame-relative 944/950,
1096/1102, 1287/1293, 1435/1441, **1602/1608, 1756/1762**) where NesHawk runs five. That is
Kirby's input routine taking an extra pass — the same shape lot NESHAWK-PROFILE-7 measured at
frame 55 and lot 8 at frame 40, where a DMC halt lands on the game's own `LDA $4016,X` read on one
machine and not the other and the pad byte comes back corrupted. Lot 8 recorded that item 1c alone
closed frame 40's collision; with items 14, 15 and 16 on it is **back**, which is itself a result —
the three items moved the clock under it. Round 3 is that collision, and it needs the `$4016` bus
stream of frame 40 rather than the event ledger, which cannot walk past it.

### The instruments

| | |
|---|---|
| `q191.lua` | the same stream as step 2; the end-of-transfer window is its last 600 lines |
| the one-fetch `lead` sweep | in item 16's own test: rate 0 puts exactly one request inside a 514-cycle transfer, and `lead` places it anywhere in the transfer. It is what showed the `+2 → +1` edge at leads 512-513 and the control at 514 |

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE-12 step 3/4 done: round 2, item 16 closes tree frame 38 and the wall is now frame 40's extra input pass

---

## Step 4 — no regression, and the final pass

### The answer first

**All three items cost nothing, anywhere, and the two deterministic instruments say so.**
`golden-core` over the 98 originals is byte-identical, every ROM suite is on its documented figure
on **both** profiles, *Faxanadu* and every *Kirby* window are unmoved, and the bench is **faster**
than the fixed reference on both ROMs. The lot's whole effect is the clock, which is what it was
for.

### The suites

| | before this lot | **after** | |
|---|---|---|---|
| `golden-core`, 98 ROMs x 1 200 frames | `8d508070850b24228285b6f04df6f98c310dbdab65e7577921c91ad4d0689773` | **identical** | ok |
| `test-roms --profile standard` | 212 pass / 7 fail | **212 / 7** | ok |
| `test-roms --profile neshawk` | 205 pass / 14 fail | **205 / 14** | ok |
| `AccuracyCoin`, standard | 128/147 | **128/147** | ok |
| `AccuracyCoin`, neshawk | 119/147 | **119/147** | ok |
| `ppu_vbl_nmi` + `vbl_nmi_timing`, both profiles | 18/18 | **18/18** | ok |
| the NMI ROMs NesHawk passes (`--filter nmi`), both profiles | 20 pass / 0 fail | **20 / 0** | item 8d's gate |
| `cpu_interrupts_v2` standard / neshawk | 6/6 and 4/6 | **6/6** and **4/6** | ok |
| `sprdma_and_dmc_dma` standard / neshawk | 2/2 and 0/2 | **2/2** and **0/2** | item 13's own cost |
| the DMC/DMA ROMs (`--filter dmc`) standard / neshawk | 8 pass, and 4 pass / 4 fail | **8**, and **4 / 4** | ok |
| *Faxanadu* USA / Rev 1, of 832 | 202 and 210 | **202** and **210** | ok |
| *Kirby* `step1` / `step1b` / `step1c` identical to the byte | 219 / 342 / 459 | **219 / 342 / 459** | ok |
| *Kirby* `step1c` with no difference outside the stack page | 8 190 of 8 901 | **8 190** | ok |
| the parting frame | 9 353 | **9 353** | unmoved |
| the first difference of any kind, on the clock | tree frame **14** | tree frame **40** | **the lot's result** |

**The NMI row is the one that matters most, and it is deliberate.** It is the gate that refused
item 8d last lot. Both of this lot's items are on the DMA path rather than the interrupt path, so
it was not expected to move — but the recipe says to run the public ROMs that measure the quantity
the item is about, and `sprdma_and_dmc_dma`, `dmc_dma_during_read4`, `apu_test` and
`cpu_interrupts_v2` are those ROMs for items 15 and 16. **None of them moved in either
direction**, which is itself a statement about the items: they fire only when a DMC fetch and an
OAM transfer overlap, and of the whole 331-ROM suite only `sprdma_and_dmc_dma` arranges that — and
it was already failing on this profile for item 13's reason, with the same two ROMs and the same
codes.

### The whole test suite

`cargo test --release --workspace --exclude zax-app --exclude zax-platform` (those two do not link
here), summed per target rather than read off the last line:

| | |
|---|---|
| **2 503 passed, 0 failed, 88 ignored** over **144** targets | lot 11's 2 501 plus items 15 and 16's own differential tests |
| the count of `test result: ok` lines | **144** — every target reported, so the total is not a short one |
| the ignored ones, with `ZAXNES_ROMS`, `ZAXNES_TEST_ROMS` and `ZAXNES_PROFILES` set | **69 passed, 6 failed** |

**The six failures are environmental, and are named rather than absorbed**: all six are
`zax-profiles`'s `three_d_sen_real_install_tests`, and all six panic on
`ZAXNES_3DSEN_INSTALL is not set: it names a 3dSen installation (or a stand-in) folder`. That
variable names the original program's installation, which this side does not read and was not
given one for; nothing in the six touches the NES core, the timing profile or the DMA path.

### The speed, and it is inside the budget again

`bench-core --frames 600`, three interleaved rounds against **`428c50e6` itself** — the fixed
reference the last six lots are measured on — built from `git checkout 428c50e6 -- rust/crates`
into the same target directory, with the tree restored afterwards:

| | `428c50e6` | this lot | |
|---|---|---|---|
| Super Mario Bros. 3 | 1.1705 ms/frame | 1.1210 | **-4.23 %** |
| Mega Man 3 | 0.8666 ms/frame | 0.8442 | **-2.59 %** |

Both **faster**, and inside the +1 % budget. **The honest caveat is the one lots 10 and 11 wrote
down**: the two binaries' own spreads over three rounds are +0.68 % and +2.09 % on Super Mario
Bros. 3 and +2.31 % and +1.52 % on Mega Man 3, the same order as the difference, so this figure is
layout. What makes that more than an assertion here is that the deterministic instrument agrees —
`golden-core` is byte-identical — and that although this lot's items *are* on the DMA path, which
these two ROMs reach, they are on it only inside `NesCoreOf<NesHawkCompat>::run_dma`, a
monomorphisation `bench-core` never instantiates.

### The final pass

| | |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --release --workspace --exclude zax-app --exclude zax-platform --all-targets -- -D warnings` | clean |
| `cargo test --release` | 2 503 / 0 / 88 |
| the ignored ones | 69 / 6, the six as above |
| disk at the end | `G:` 31 GB free, `C:` 8.4 GB |

### What this lot did not do, said plainly

* **The wall at 9 353 did not move**, and none of the three items was expected to move it: each is
  exactly inert on the work RAM, measured by building it off and on rather than argued. What they
  bought is 26 frames of clock, and the reason that is worth a lot is the one the brief gave — an
  item hunt whose window opens on a machine already out of step measures the drift and not the item.
* **Item 14's source reading is still open.** It ships on its measurement and on its costing
  nothing, with `PPU.cs` `Reset()`'s `idleSynch = true` recorded in the constant as the best
  candidate and the reason it does not yet predict the item written beside it. It is the only item
  in the table that ships without a reading, and that is said where the item is.
* **Round 3 was not run.** The brief allowed up to five rounds; two were taken, and the third's
  window is named and characterised instead: tree frame 40's extra `$4016` input pass, which needs
  a bus stream of that frame's `$4016` accesses rather than the event ledger, because the ledger's
  own alignment cannot walk past an event-stream difference.
* **The `+6` phases of item 15's own sweep are unmeasured.** A fetch due on a transfer's *first*
  cycles — its halt — costs the NesHawk profile three cycles where `Standard` spends two, and that
  reads the same with items 15 and 16 off, so it predates both. No BizHawk measurement was taken of
  it, because the movie's frames 3-40 never arrange it; it is the remaining third of the interleave,
  and it is written down here rather than fixed by symmetry with the two that were measured.

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE-12 step 4/4 done: no regression, the bench faster than the reference, the documents
