# NESHAWK-PROFILE-11 — the first difference is tree frame 14, and the PPU/CPU phase is pinned

Branch `feat/neshawk-profile`, worktree `cleanroom/wt-bk2`, base `02830ad7` (lot NESHAWK-PROFILE-10).

## Steps

- [x] 1. **The one-dot difference before frame 27.** The brief's lead — a per-frame one-dot
      difference from the odd-frame dot skip — is **refused**: the two machines' frames are the
      same length **in dots** through tree frame **230**. The real first difference is **14 frames
      earlier than lot 10's window**, at tree frame **14**, and the two candidates that can produce
      it are both closed by measurement: the NMI's recognition **dot** (**item 8d**, implemented and
      refused by six ROMs NesHawk passes) and a uniform sub-cycle PPU/CPU phase (**exactly inert**,
      with a control).
- [x] 2. **Round 1** — one item, **item 14**: a PPU dot bias on the **soft reset's** start-up run
      alone, measured at **−1**, which takes the first difference from tree frame 14 to tree frame
      **38** and costs nothing anywhere. **Ships off**: no reading, and it does not move the wall.
- [x] 3. **No regression**: `golden-core` byte-identical, every suite on its documented figure,
      2 501 passed / 0 failed, and the bench **faster** than `428c50e6` on both ROMs (−3.54 %,
      −1.25 %) where lot 10 read +2.32 % and +1.43 %.

## The environment this lot runs in

| | |
|---|---|
| `CARGO_TARGET_DIR` | `<worktree>/rust/target-ev` |
| `TMP` / `TEMP` | `<worktree>/rust/target-ev/tmp` |
| `CARGO_INCREMENTAL` | `0` |
| disk at the start | `G:` 32 GB free (stop at 5), `C:` 8.3 GB (stop at 1) |

---

## Step 1 — the first difference, and the two knobs that cannot be it

### The answer first

1. **The movie's first difference of any kind is tree frame 14, not frame 28.** Lot 10's window
   began at BizHawk frame 25 because `q185.lua` was registered there. The ledger that covers the
   frames before it **was already on disk**: `kirby-jp-cyc0.txt`, tree frames 2-1201, written by
   lot NESHAWK-PROFILE-3. Against `frame-trace --event-log --dmc-census` it says tree frames
   **3 to 13 are in step on every event** — 422 events, residual 0 — and tree frame **14**'s NMI
   entry is the first to leave it, at **−6** cycles and with `SP:EF` here against NesHawk's `SP:ED`.
2. **The brief's lead is refused, and by this core's own clock.** Each frame's length **in PPU
   dots** is computable here with no cross-machine calibration (two NMI entries' cycles and their
   `SL:dot`), and BizHawk's per-frame CPU-cycle count is exact (its frame ends on a PPU dot — item
   12). Those dot lengths reproduce **every** one of BizHawk's cycle counts, with one fitted
   starting phase, from tree frame 2 to tree frame **230** — 229 binary predictions, one free
   parameter, and the first mismatch at 231 is downstream of the divergence. A single one-dot error
   anywhere in that run would break the fit from there on. The odd-frame dot skip's condition is
   **not** the difference.
3. **Frame 14 is a one-instruction displacement at one-dot resolution.** The bus stream of BizHawk
   frames 11-13 (`q187.lua`) against `frame-trace --out`: tree frames 12 and 13 run the **same
   14 025 instructions at residual 0**, every one, and the whole difference is the NMI at SL 241.
   `$C50C BNE $C504` runs three cycles beginning on dots **339 (SL 240), 1 and 4**; this core's
   sample of /NMI is taken on a cycle's **first** dot, so the penultimate cycle samples on dot 1 —
   the dot the vblank flag rises on — the NMI is pending at the 6502's poll, and `$C504 JSR $C555`
   is preempted. NesHawk runs the `JSR` and preempts `$C555` six cycles later.
4. **Item 8d — the NMI's recognition *dot* — was implemented and is refused by NesHawk itself.**
   It is the shape item 8 never took: not a one-**cycle** lag of the whole /NMI line but a
   **dot**-grained delay of the vblank assertion alone, which in the alignments where the delayed
   dot stays inside the same CPU cycle changes nothing. Swept at one-dot steps over 0-7 it moves
   the parting event from tree frame 14 to tree frame **38** at 1 and 2 — and it makes the profile
   **fail six public ROMs that NesHawk passes**, measured on BizHawk this lot (`q189.lua`) rather
   than taken on trust. The recipe's gate says stop, so it is **retracted**, as item 8 was.
5. **A uniform sub-cycle PPU/CPU phase is exactly inert, and the reason is the game.** A dot bias
   applied at the **power-on only** — the odd net offset lot 10 named as unreachable, since
   `PPU_START_UP_DOTS_SKIPPED` is applied twice — gives **byte-identical** answers at −2, −1, +1,
   +2 and +3, with **−3 (one whole cycle) as the control** that proves the knob live. The boot's
   `$2002` wait loop re-quantises it: moving the whole PPU moves the dot the flag rises on **and**
   the dot the loop sees it on, so the phase is restored. In a frame-synced game the PPU/CPU
   sub-cycle phase is **not a free parameter** — it is pinned by the game's own vblank wait, and
   only a change to the *rule* relating the flag to the CPU can move it. That is item 8 and 8d, and the
   ROMs refuse it.

So the three cycles at lot 10's frame 27 and the six at frame 14 are the same symptom, the two
knobs that could produce it are both closed, and **the cause is upstream of the NMI**. No item
ships; the candidate list is shorter by two and the window is 14 frames earlier.

### The marks, tree frames 2 to 40

`frame-trace --frame 1 --frames 80 --profile neshawk --dmc-census --event-log --event-pc C06E
--event-pc C37C --event-pc 80FC --event-pc 804C` against `bizhawk-runs/kirby-jp-cyc0.txt`, every
event on its **absolute** cycle. In step reads 0 (BizHawk's counter is 1-based where `CYC` is a
0-based index, and the two instruments date a read or a write one cycle apart from an execute).
The boot's `r$2002` spin loop is dropped: its count moves across a frame boundary by construction.

| tree frame | 3-13 | 14 | 15 | 16 | 17 | 18-27 | 28 | 29 | 30 |
|---|---|---|---|---|---|---|---|---|---|
| residual | **0** | **−6** | +0/+1 | +2 | −1 | **0** | −4 | +2 | −2 |

Frame 14's sixteen events: `−6` on eleven of them, `−5` on three. Lot 10's frame 28 (its BizHawk
frame 27) is the *second* departure, not the first.

### The frame length in dots, and why it closes the lead

This core's dot length of the span between tree frame *f*'s NMI entry and *f+1*'s is
`3 × (cycles between them) − (dot of the second − dot of the first)`, both read off its own census.
BizHawk's cycle length per frame is `start(f+1) − start(f)` from the same ledger, and is exact.
Predicting the second from the first needs one fitted starting residue:

| | |
|---|---|
| frames predicted | **229** (tree frames 2-230) |
| free parameters | 1 (the starting residue mod 3) |
| mismatches | **0** before frame 231 |
| this core's dot lengths over 392 spans | 89 342 × 208, 89 341 × 184 |
| the skip's first frame | tree frame **12**, on both machines |

### The bus stream across tree frames 12-14

`q187.lua` (BizHawk frames 11-13, 2.0 MB, 112 198 lines) against `frame-trace --out`, anchored on
each frame's own `$C341` NMI handler entry:

| | instructions compared | first PC mismatch | residual |
|---|---|---|---|
| tree frame 12 | 7 062 | none | **0** throughout |
| tree frame 13 | 6 963 | none | **0** throughout |
| tree frame 14 | 8 996 | idx 939 | **−6** from the handler itself |

**A trap, and it is this core's tracer.** `frame-trace --out` prints an instruction's **own** start
cycle, and a DMA halt that lands on that instruction's first cycle is charged after it — so every
OAM transfer and every DMC fetch shows as a **single-instruction** residual of −516 or −3/−4 that
recovers immediately. Frame 12 has 26 such blips and frame 13 another 26, and not one of them is a
difference: the run-length structure of the residual is what to read, never a per-instruction list.

### Item 8d, the sweep, and the gate that refuses it

Item **8d**, named so because it is item 8's question at dot resolution and not a new one — the
number **14** belongs to step 2's shipped-off constant.

`VBLANK_NMI_LINE_DELAY_DOTS`: dots of the vblank scanline during which the assertion is not yet
visible to the CPU's sample of /NMI, read at the one line of a copy of `Ppu2C02::run_cpu_cycles`
(the `Standard` path untouched, and the value reaching it through `if P::… == 0` in
`NesBusOf::catch_up_ppu`, which folds). State-free: scanline and dot alone, no savestate byte.

| delay, dots | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 |
|---|---|---|---|---|---|---|---|---|
| first tree frame off | **14** | **38** | **38** | 4 | 4 | 4 | 4 | 4 |
| frames in step | 3-13 | **3-37** | **3-37** | 3 | 3 | 3 | 3 | 3 |

`1` and `2` differ only for a cycle beginning exactly on dot 2 of the vblank line, which this
window never meets; `2` is NesHawk's own number (`PPU.run.cs` `TickPPU_VBL` assigns `nes.cpu.NMI`
at `ppur.status.cycle == 3 + NMI_offset`, the first dot **at or after 3** that begins a CPU cycle,
which at this core's sample point is `dot <= 2`). `3` and up break tree frame 4 because they
suppress a cycle beginning on dot 3, which NesHawk asserts on.

**The gate, measured on BizHawk this lot.** `test-roms --filter nmi --profile neshawk` with item 8d
on: 22 ROMs, **12 pass, 8 fail** against the shipped profile's 20 pass / 0 fail. Six of the eight
were then put to NesHawk itself (`q189.lua`, the verdict read off the **screen** — see the trap
below), and **NesHawk passes every one**:

| ROM | NesHawk | profile with item 8d |
|---|---|---|
| `ppu_vbl_nmi/05-nmi_timing` | `00 4 / 01 4 / 02 3 / … Passed` | fail |
| `ppu_vbl_nmi/06-suppression` | `Passed` | fail |
| `ppu_vbl_nmi/08-nmi_off_timing` | `Passed` | fail |
| `vbl_nmi_timing/5.nmi_suppression` | `PASSED` | fail (`#4`) |
| `vbl_nmi_timing/6.nmi_disable` | `PASSED` | fail (`#3`) |
| `vbl_nmi_timing/7.nmi_timing` | `PASSED` | fail (`#6`) |

Plus `ppu_vbl_nmi.nes` (the combined ROM, at test 5 of 10) and `cpu_interrupts_v2/3-nmi_and_irq`.
**Retracted**: the whole change is reverted and the tree is `02830ad7`'s code exactly.

**A trap, and it cost this step a BizHawk run.** `romresult.lua` reads blargg's `$6000` protocol
and exits on the first frame past 60 where the status byte is not `$80`. For these ROMs BizHawk's
`System Bus` returns the open-bus **`$F2`** for the whole `$6000` page, so it exits at frame 61 and
reports `status=F2 sig=F2 F2 F2` — which is *not started*, not a verdict, and reads as a failure.
`q188.lua` waits for the protocol's own signature `DE B0 61` and never sees it either, at frame
2 400. **Read the screen**: every one of these ROMs prints its verdict there, and that is also what
this side's `test-roms` reads, so the two machines are then compared on the same evidence.

### The uniform phase, and the control it needed

A dot bias added to `PPU_START_UP_DOTS_SKIPPED` at the **power-on only**, so that the net offset is
**odd** — the value lot 10 recorded as unreachable, since the constant is applied at the power-on
*and* at the reset. Measured on the event stream of tree frames 2-80, which is far sharper than lot
10's four marks:

| bias, dots | −3 | −2 | −1 | 0 | +1 | +2 | +3 |
|---|---|---|---|---|---|---|---|
| first tree frame off | 4 | 14 | 14 | **14** | 14 | 14 | 14 |
| the rest of the table | moved | **byte-identical to 0** | | | | | |

−3 is one whole CPU cycle and it moves everything, which is the control lot 10's own trap demands
before an inert result counts. The five sub-cycle values are byte-identical: **the game pins the
phase**. This is lot 10's post-reset finding confirmed at the power-on, with the mechanism named —
the `$2002` wait loop exits on the dot the flag rises on, so moving the PPU moves both ends of the
comparison and the phase is restored.

### The instruments

| | |
|---|---|
| `kirby-jp-cyc0.txt` | **already on disk**, tree frames 2-1201, lot NESHAWK-PROFILE-3's by-product. It is what moved the window from frame 28 to frame 14, and no new reference was taken for it |
| `q187.lua` | the whole bus stream of BizHawk frames 11-13 (tree 12-14), q186's format. 2.0 MB, an intermediate |
| `q188.lua` | NesHawk's `$6000` verdict, waiting for the `DE B0 61` signature. **It finds the page unreadable**, which is the result it was written to establish |
| `q189.lua` | NesHawk's verdict read off the **screen** at frame 900, `bizhawk-runs/q189-<rom>.png`. The gate above is its six shots |
| `rust/target-ev/tmp/marks.py` | the ledger diff: every event on its absolute cycle, per-frame residual histogram, the first non-zero. A scratch instrument, not committed |

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE-11 step 1/3 done: the first difference is tree frame 14, and both knobs that could cause it are closed

---

## Step 2 — round 1: the one knob that moves the clock, and it ships off

### The answer first

**Round 1 produced one item, measured sharply, and it ships `off` for want of a reading.** Step 1
left exactly one live hypothesis: a **uniform PPU/CPU phase**, which `ppu_vbl_nmi` `05-nmi_timing`
provably **cannot refuse** — that ROM synchronises on `$2002` and times in CPU cycles, so a
constant offset cancels in both of its terms, where every *rule*-shaped candidate for the same dot
(item 8, and step 1's item 8d) moves one term and not the other and is refused by it.

The knob is a dot bias on the **soft reset's** start-up run alone. At **−1** the movie's first
difference of any kind goes from tree frame **14** to tree frame **38**, and the frames in between
are in step on *every* event of the ledger. It is **sharply peaked and signed**, it **costs
nothing anywhere**, and it **does not move the wall at 9 353**.

### The sweep

`frame-trace --frames 80 --profile neshawk --dmc-census --event-log` against `kirby-jp-cyc0.txt`,
the first tree frame whose events leave residual 0:

| bias, dots | −5 | −4 | −3 | **−2** | **−1** | 0 (shipped) | +1 | +2 | +21 |
|---|---|---|---|---|---|---|---|---|---|
| first tree frame off | 8 | 8 | 8 | **38** | **38** | 14 | 11 | 11 | 7 |
| frames in step | 11-15 | 10-15 | — | **3-37** | **3-37** | 3-13 | — | — | — |

The window is exactly `{−1, −2}` — one cycle-grid position — which is what a real sub-cycle phase
looks like and what a fitted constant does not.

### Why no other shape of the same knob is reachable

| the knob | the experiment | the answer |
|---|---|---|
| `PPU_START_UP_DOTS_SKIPPED` itself | 7 164 and 7 166, the whole event stream each time | **exactly inert.** It is applied at the power-on **and** at the soft reset, so every value gives an **even** net offset — lot 10's finding, re-measured at one-dot steps |
| a bias at the **power-on** only | −3, −2, −1, 0, +1, +2, +3 | **exactly inert** over −2 to +3, **byte-identical**; **−3**, one whole CPU cycle, moves everything, which is the control an inert result needs. The post-reset boot's `$2002` wait re-quantises it: moving the whole PPU moves the dot the flag rises on **and** the dot the loop sees it on |
| a bias at the **soft reset** only | the table above | **live, peaked and signed.** A dot dropped *after* that wait moves only the first of the two, and nothing in the game can put it back |

**This corrects lot NESHAWK-PROFILE-10's reading of its own post-reset sweep.** It recorded ±1 and
±2 as "exactly inert" on the four once-a-frame marks over 9 336 frames. On the event stream the
same sizes are the difference between parting at frame 14 and parting at frame 38. The marks are
quantised by the spin loop; the ledger is not.

### What it is worth, and what it is not

| | shipped (0) | at −1 |
|---|---|---|
| first difference of any kind | tree frame **14** | tree frame **38** |
| frames in step on every event | 3-13 | **3-37** |
| `step1c` identical to the byte | 237 of 8 901 | **459** |
| `step1c` no difference outside the stack page | **8 192** | 8 190 |
| the parting frame | **9 353** | **9 353** |
| `test-roms --filter nmi --profile neshawk` | 20 pass / 0 fail | **20 pass / 0 fail** |

**It cannot cost a ROM, and that is structural rather than lucky**: the site is the soft reset,
which only a movie's RESET column reaches, so every ROM of every suite runs the code it ran before.

### Why it ships off: three readings, none of which predicts it

| the reading | what it says | why it is not this |
|---|---|---|
| `PPU.cs` `Reset()`'s **`idleSynch = true`** | `PPU.run.cs` skips the dot with `if (ppur.status.sl == 0 && idleSynch && evenOddDestiny && chopdot) { ppur.status.cycle++; } else { runppu(); }` — the dot is stepped over rather than run, so `true` is the skip **armed** — and `idleSynch = !idleSynch` toggles it once a frame. NesHawk's reset arms it; `Ppu2C02::soft_reset` sets `odd_frame = false`, the same flag with the opposite value | **A real difference of the right size, and the wrong shape.** Implemented faithfully (`arm_odd_frame_skip`, called under the constant after `soft_reset`) the movie still parts at tree frame **14**: flipping the parity makes the cumulative dot count **alternate** 0/−1 rather than accumulate, because both machines then skip once every two frames. Reverted |
| `PPU.run.cs` `NewDeadPPU()` | `if (ppur.status.cycle == 241 * 341 - start_up_offset - 1) …; runppu(); if (ppur.status.cycle == 241 * 341 - start_up_offset) { … }` with `start_up_offset = 4`, verbatim | **82 177 dots, which is the shipped 7 165 and not 7 164.** And a change to that constant is applied twice, so it cannot produce an odd offset at all |
| `evenOddDestiny = PPUON` at `ppur.status.cycle == 339` | NesHawk latches "rendering enabled" one dot after this core's `SKIP_DECISION_DOT = 338` | One dot, but it only moves anything when a `$2001` write lands **on that dot**, which is not a constant phase. It is the brief's own original lead in its last form |

So the constant carries the measurement, the sweep, the control and the three readings, and ships
`0` — **item 11's shape exactly**: correct in isolation is not the same as shippable, and the
knowledge is kept in code rather than deleted. What a reading would have to say is now narrow:
**one PPU dot, at the soft reset, once, and not at the power-on.**

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE-11 step 2/3 done: round 1, item 14 measured at -1 dot and shipped off

---

## Step 3 — no regression, and the final pass

### The answer first

**Nothing moved, and the diff says why before any suite does**: the whole source change is one new
associated constant that is `0` on *both* profiles, the `+ P::SOFT_RESET_PPU_DOT_BIAS` that reads
it at a single cold site, and a test. `golden-core` is byte-identical, every suite is on its
documented figure, and the bench is **faster** than the fixed reference on both ROMs.

### The suites

| | before | now |
|---|---|---|
| `golden-core`, 98 originals × 1 200 frames | `8d508070…0689773` | **`8d508070850b24228285b6f04df6f98c310dbdab65e7577921c91ad4d0689773`** |
| `test-roms --profile standard` | 212 pass / 7 fail | **212 / 7** |
| `test-roms --profile neshawk` | 205 pass / 14 fail | **205 / 14** |
| `AccuracyCoin`, standard / neshawk | 128/147, 119/147 | **128/147, 119/147** |
| `test-roms --filter nmi --profile neshawk` | 20 pass / 0 fail | **20 / 0** |
| *Faxanadu* USA / Rev 1, standard | 207, 220 of 832 | **207, 220** |
| *Faxanadu* USA / Rev 1, neshawk | 202, 210 of 832 | **202, 210** |
| *Kirby*, `--ram-ref` frames 1 100-10 000 | parts at 9 353 | **9 353** |
| the whole suite | 2 500 passed, 0 failed, 88 ignored, 144 targets | **2 501 passed, 0 failed, 88 ignored, 144 targets** (item 14's test is the one more), summed per target |

The ignored run is **69 passed, 6 failed over 87 targets**, and all six failures are
`three_d_sen_real_install_tests`, which panic with *"ZAXNES_3DSEN_INSTALL is not set"* — this
environment does not name a 3dSen installation. Nothing in them touches the core.

### The speed, and it is inside the budget this time

`bench-core --frames 600`, three interleaved rounds, this HEAD against **`428c50e6`** itself — the
fixed reference the cumulative figure is kept against, built from `git checkout 428c50e6 --
rust/crates` into the same target directory:

| | `428c50e6` | this lot | |
|---|---|---|---|
| Super Mario Bros. 3 | 1.3631, 1.3039, 1.3331 → **1.3334 ms** | 1.2815, 1.2779, 1.2992 → **1.2862 ms** | **−3.54 %** |
| Mega Man 3 | 0.8914, 0.9004, 0.8998 → **0.8972 ms** | 0.8864, 0.8881, 0.8835 → **0.8860 ms** | **−1.25 %** |

**Both faster, and this reverses lot NESHAWK-PROFILE-10's +2.32 % and +1.43 %** — which that lot
reported as over budget and argued was code layout rather than emulation, on the evidence that its
order control inverted the ranking and one binary's own spread matched the difference. This lot
changed no per-frame path at all (a constant folding to `+0` in a function called once per reset),
so the swing of nearly six points between two lots that both changed nothing is the honest measure
of what this bench resolves. **The cumulative figure against the fixed reference is now −3.54 %
and −1.25 %**, inside the +1 % budget.

### The final pass

| | |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --release --workspace --exclude zax-app --exclude zax-platform --all-targets -- -D warnings` | clean |
| `cargo test --release --workspace --exclude zax-app --exclude zax-platform` | 2 501 passed, 0 failed |
| the ignored tests, with `ZAXNES_ROMS`, `ZAXNES_TEST_ROMS`, `ZAXNES_PROFILES` | 69 passed, 6 failed — all six the installation tests above |

### What this lot did not do, said plainly

* **The wall at 9 353 did not move**, and nothing this lot found bears on it. Item 14 buys the
  clock 24 frames and not one frame of the work.
* **The brief's rounds 2 to 5 were not run.** Round 1 spent itself on the window step 1 opened, and
  the two candidates the brief named after the skip — the **OAM/DMC interleave** (item 13's
  unmeasured half, the likely cause of *Faxanadu*'s 45 and 104 lines) and the **MMC3 IRQ polling
  point** — are untouched, as they were after lot 10. Neither can act in tree frames 12-14: those
  frames' one OAM transfer is in step to the cycle and carries no DMC fetch inside it.
* **Tree frame 38 is the next window**, and it is a different mechanism from frame 14's: with the
  bias at −1 its residual is `+2` on eleven of fourteen events with the **NMI itself at 0**, so it
  is not an NMI displacement. That is the shape of a DMA cost, which is where the brief's two
  remaining candidates live.
* **Item 14's reading is open**, narrowly: one PPU dot, at the soft reset, once, not at the
  power-on.

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE-11 step 3/3 done: no regression, the bench faster than the reference on both ROMs
