# NESHAWK-PROFILE-10 — the wall at 9 353

Branch `feat/neshawk-profile`, worktree `cleanroom/wt-bk2`, base `08a881b8` (lot NESHAWK-PROFILE-9).

## Steps

- [x] 1. **Ship the exact configuration**: items 12, 1c and 13 **on**. Every comparison of this lot
      is against it; the baseline it reproduces is lot 9's **9 353**.
- [x] 2. **The wall at 9 353.** The clock parts **9 326 frames earlier than the work does**: the
      first frame whose residual leaves −1 is BizHawk frame **27**, and the bus stream of it shows
      8 363 instructions in step to the cycle and no DMA at all, parting on whether the vblank NMI
      displaces the spin loop's `BEQ`. The candidate history named first is **closed three ways**.
- [ ] 3. **Rounds** — until frame 156 312 or five rounds.
- [ ] 4. **No regression**, report, final pass.

## The environment this lot runs in

| | |
|---|---|
| `CARGO_TARGET_DIR` | `<worktree>/rust/target-ev` |
| `TMP` / `TEMP` | `<worktree>/rust/target-ev/tmp` |
| `CARGO_INCREMENTAL` | `0` |
| disk at the start | `G:` 32 GB free (stop at 5), `C:` 8.8 GB (stop at 1) |

---

## Step 1 — the exact configuration ships

### The answer first

The three items lot NESHAWK-PROFILE-9 measured and shipped off are **on**:
`DMC_HALT_LANDS_ON_A_WRITE_CYCLE` (1c), `RESET_LANDS_ON_THE_FRAME_BOUNDARY_DOT` (12) and
`OAM_DMA_WAITS_AT_THE_END` (13). The choice is the one lot 9 named: each is measured right on
BizHawk's own stream, and the 45 095 the all-off profile reaches is **two errors cancelling**, which
is not a configuration a later difference can be measured against.

`movie-sync --engine zaxnes --profile neshawk --no-lag --frames 10000 --ram-ref kirby-jp-step1c.txt`
reproduces lot 9's figure exactly: **the last frame with no live difference is 9 353**, 237 frames
identical to the byte and 8 192 of 8 901 with no difference outside the stack page.

### What moved

| | |
|---|---|
| `crates/zax-nes/src/timing_profile.rs` | the three constants on `NesHawkCompat`, each note rewritten to say it ships **on** and what it costs |
| `crates/zax-nes/tests/neshawk_profile_tests.rs` | the `ItemsWaiting` test profile is **gone** — it was `NesHawkCompat` plus the three items, so with them on it is `NesHawkCompat`. The three tests measure the shipped profile directly, as item 11's did when it shipped, and the three "the shipped profile does not carry it" assertions are deleted rather than inverted |

`cargo test -p zax-nes --release`: every target passes, `neshawk_profile_tests` 13 of 13 with 1
ignored. Item 1c reads `[4, 5, 4, 5]` against `Standard`'s `[5, 4, 5, 4]`, item 13's four phases sum
to 1 027 on each, and item 12's before-reset frame is short exactly where the boundary dot is an
instruction's first cycle.

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE-10 step 1/4 done: the exact configuration ships, the baseline is 9 353

---

## Step 2 — the wall at 9 353, and the clock that parts 9 326 frames before it

### The answer first

1. **The wall is not where the clock parts.** Over BizHawk frames 25-9 360 (`q185.lua`, 9 336
   frames, four late-registered callbacks, 48 s) the four marks read a constant **-1** — the two
   machines in step — to BizHawk frame **26**, and frame **27** is the first to leave it. The movie
   runs on to 9 353 because the residual *recovers*: 1 866 of the 9 336 frames read -1 on all four
   marks, in runs.
2. **The first difference is three cycles at the NMI boundary, with nothing else in the frame.** The
   bus stream of BizHawk frames 25-27 (`q186.lua`, 2.1 MB) against `frame-trace --out`: from frame
   26's `$C06E` to the boundary the two machines run the **same 8 363 instructions at a residual of
   exactly -1**, every one, and those frames carry **`dmc=0/0`** — no DMC DMA at all, the one OAM
   transfer being inside the frame's work before `$C06E`. They part on **whether the vblank NMI
   displaces the spin loop's `BEQ $C070`**: NesHawk runs it and this core does not.
3. **It is not a phase constant, and this is the first lot to ask at one-dot resolution.** Every
   earlier sweep moved the start-up dots in **multiples of three** — lot 7's 7 156...7 174 step 3 and
   lot 8's reset biases 0, 3, 6, 9, 12 — so all twenty values are **one residue class mod 3** and
   the CPU/PPU *sub-cycle* phase had never been varied at all. Swept at one-dot steps over the
   thirteen values 7 158-7 172, measured two ways, the best is 7 163 — and it leaves **376 of
   3 985** in-step frames still disagreeing by exactly 3, and **does not move the wall at 9 353 by
   one frame**.
4. **Nor is it a non-uniform CPU/dot sequence**: `NES.Core.cs`'s `cpu_sequence_NTSC = { 3, 3, 3, 3,
   3 }` is uniform — the same three dots a cycle this core runs. (`cpu_sequence_PAL = { 3, 3, 3, 4,
   3 }` is this core's 16/5.)
5. **And what the source does describe at that site is item 8, which lot 4 already refuted.**
   `PPU.run.cs` `TickPPU_VBL` asserts `nes.cpu.NMI = true` at `ppur.status.cycle == 3 + NMI_offset`
   of the NMI scanline, with `NMI_offset` = 0, 2, 1 for `cpu_stepcounter` 0, 1, 2 — so the
   assignment **always lands on a dot where `cpu_stepcounter` is 0**, the first dot at or after 3
   that begins a CPU cycle. Against this core's assertion at dot 1 of the vblank scanline that is 3
   to 5 dots later and **exactly one CPU cycle in every alignment** — which is item 8, the item
   lot NESHAWK-PROFILE-3 implemented in three shapes and lot NESHAWK-PROFILE-4 retracted because
   `ppu_vbl_nmi` `05-nmi_timing` sweeps that very question at one-dot resolution and **NesHawk
   passes it**. A fourth reading does not beat that measurement.

So the three cycles at the NMI boundary are a **symptom**, and its cause is upstream of the NMI.

### The marks, over the window

`frame-trace --frame 26 --frames 9336 --profile neshawk --event-log --event-pc C3A1 --event-pc C06E
--event-pc C074` (19 s) against `bizhawk-runs/q185.txt`, `zaxnes - bizhawk` on the absolute cycle,
BizHawk frame *f* against this core's *f+1*:

| BizHawk frame | 25-26 | 27 | 28 | 29 | 30 | 31 | 32 | 33 | 34 | 35-36 |
|---|---|---|---|---|---|---|---|---|---|---|
| nmi, rti, c06e, c074 | **-1** | -4,-5,-2,-2 | 1,1,4,4 | -2,-3,-3,-3 | -3,-3,-3,-3 | -3,-4,-5,-4 | -3,-3,-1,0 | -4,164,163,167 | 1,-1,-1,-1 | **-1** |

| | |
|---|---|
| frames all four at -1 | **1 866** of 9 336 (20.0 %) |
| the nmi mark alone, most common residuals | -1 (2 802), -3 (2 304), -2 (1 406), -4 (1 221), 0 (971) |

### The bus stream across the boundary, BizHawk frame 27

Execute-to-execute, the window frame 26's `$C06E` (774 214 / 774 213, in step) to the handler. The
first index whose PC differs is **none**; the first whose residual leaves -1 is index **8 363**, the
handler itself:

| index | 8 361 | 8 362 | 8 363 |
|---|---|---|---|
| bizhawk | 799 297 `$C070 LDA $39` | 799 300 `$C072 BEQ` | 799 310 `$C341 PHA` |
| zaxnes | 799 296 `$C070 LDA $39` | 799 299 `$C072 BEQ` (**displaced**) | 799 306 `$C341 PHA` |
| residual | -1 | -1 | **-4** |

BizHawk's own records say which it is, with no inference: `799300 x C072`, reads at `C072`, `C073`,
`C074` — the taken branch's three cycles — then `799303 r C070`, `799304 r C070`, the pushes at
799 305-799 307, the vector at 799 308-799 309 and the handler at 799 310. This core's handler is
`799299 + 7`, so the seven cycles ran **in place of** the `BEQ`. That is lot NESHAWK-PROFILE-3
step 1's mechanism exactly, now with the phase exact and the CPU streams in step to the cycle.

### The one-dot sweep, which no earlier lot took

`PPU_START_UP_DOTS_SKIPPED` at one-dot steps, the whole 9 336-frame window each time. Two
statistics: the frames whose four marks all read -1, and — restricted to the frames where **`$C06E`
itself is at -1**, so that the two machines entered that frame's loop together — the residue
`(the next frame's NMI - this frame's $C06E) mod 6`, which is read off **one machine's own clock**
and so carries no calibration at all. A residue gap of **3** is the `BEQ` running on one machine and
being displaced on the other.

| dots | 7 158 | 7 159 | 7 160 | 7 161 | 7 162 | **7 163** | 7 164 | **7 165** | 7 166 | 7 167 | 7 168 | 7 169 | 7 170 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| four marks at -1, % | 0.1 | 0.0 | 0.5 | 0.9 | 1.5 | **42.0** | 30.6 | *20.0* | 13.5 | 9.4 | 5.8 | 4.2 | 0.3 |
| in-step frames | — | — | — | 492 | 603 | **3 985** | 3 116 | *2 255* | 1 822 | — | 1 026 | — | — |
| same residue, % | — | — | — | 30.5 | 28.5 | **90.2** | 83.5 | *71.9* | 59.4 | — | 40.8 | — | — |
| gap 3 | — | — | — | 303 | 399 | **376** | 494 | *613* | 712 | — | 585 | — | — |

**7 163 is refused, and the reasons are three.** It has no reading — `PPU.run.cs` `NewDeadPPU()` is
verbatim `if (ppur.status.cycle == 241 * 341 - start_up_offset - 1) ...; runppu(); if
(ppur.status.cycle == 241 * 341 - start_up_offset) { ... frame_is_done = true; }` with
`start_up_offset = 4`, so the start-up frame is **82 177** dots and `89 342 - 82 177 = 7 165` is the
shipped value and the right one. It does not move the wall. And 7 163 and 7 166 are the **same
residue class mod 3** yet score 90.2 % and 59.4 %, so what the statistic responds to is the
whole-cycle offset and not the phase — the gap of 3 survives every phase, which is the result.

What it costs and buys, for the record (`--ram-ref`, the same three windows):

| | 7 165 (shipped) | 7 163 |
|---|---|---|
| `step1` identical to the byte | **318** of 400 | 220 |
| `step1b` identical | 217 of 701 | **240** |
| `step1c` identical | 237 of 8 901 | **4 392** |
| `step1c` no difference outside the stack page | **8 192** | 7 387 |
| parting frame | **9 353** | **9 353** |

### A trap this step met, and it is about the metric

**The byte-identical count can be moved eighteen-fold by a constant with no reading and no effect
on the parting frame.** 7 163 takes `step1c` from 237 frames identical to the byte to 4 392 and
costs `step1` 98 of its 318, leaving 9 353 exactly where it was. So that column is not a safe
target to optimise against: it measures *when* each machine wrote the RAM down as much as whether
they ran the same frame. The parting frame and the first differing instruction are the figures that
survive a knob.

### The instruments

| | |
|---|---|
| `q185.lua` | the four marks over BizHawk frames 25-9 360, registered **late** at frame 25 so the boot runs with no Lua call. 48 s, 818 KB, `bizhawk-runs/q185.txt` |
| `q186.lua` | q184's whole bus stream over BizHawk frames 25-27. 5 s, 2.1 MB, an intermediate |

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE-10 step 2/4 done: the clock parts at BizHawk frame 27, and the NMI boundary is closed three ways

---

## Step 3 — round 1: the PPU/CPU alignment, and four candidates closed

### The answer first

**Round 1 produced no item, and that is the result.** Step 2 left a paradox worth stating plainly:
at BizHawk frame 27 the two machines run the same 8 363 instructions at a residual of **exactly
-1**, with **no DMA of any kind** in the span, and then disagree about whether the vblank NMI
displaces the spin loop's `BEQ $C070`. Three of the four quantities that could produce that are the
same on both machines by measurement, and the fourth — the PPU's dot at a given CPU cycle — is
refused by every experiment that can move it.

| the candidate | the experiment | the answer |
|---|---|---|
| a **constant** dot phase | `PPU_START_UP_DOTS_SKIPPED` at **one-dot** steps over 7 158-7 172, two statistics, the whole 9 336-frame window each time (step 2) | **refused.** The best, 7 163, has no reading, leaves 376 of 3 985 in-step frames disagreeing by exactly 3, and does not move the wall by one frame |
| an **odd** net dot offset | the constant is applied **twice** — once at the power-on and once at the reset — so every value above gives an **even** net offset `2δ` and ±1 dot was unreachable. Swept as a **post-reset-only** bias of -2, -1, +1, +2 | **exactly inert**: all four give the same 1 866 frames, the same first-off frame 27, byte for byte. The control says the knob is live: a bias of **+300** takes it to **0** of 9 336. The post-reset boot's 7-cycle `$2002` wait re-quantises any bias smaller than a loop iteration — lot 8's finding, now at one-dot resolution |
| the **odd-frame dot skip's parity** | BizHawk's own frame lengths alternate 29 781/29 780 over the early window, summing to exactly 59 561 cycles over two frames (= 178 683 dots = 89 342 + 89 341), so the skip is live; this core's parity flipped at the NesHawk start-up | **refused.** 20.0 % → 25.5 % of frames at -1 and the spin residue 71.9 % → 78.6 %, but frame 27 is unchanged to the cycle (`-4, -5, -2, -2`), `step1c` stays at 237 identical and the wall stays at **9 353**. A statistic without a structure |
| the **NMI's recognition rule** | read off 2.11.1 (step 2 point 5), plus `cpu_sequence_NTSC = { 3, 3, 3, 3, 3 }` | **refused, and it refutes itself.** What `TickPPU_VBL` describes is a deferral to the first dot at or after 3 that begins a CPU cycle, which works out to **exactly one CPU cycle in all three alignments** — item 8, the item `ppu_vbl_nmi` `05-nmi_timing` refuses because **NesHawk passes that ROM** |

### Why the fourth candidate cannot be read any further

The reading admits exactly two alignments of NesHawk's `(sl, cycle)` against this core's `(SL,
DOT)`, and **neither** of them explains frame 27:

| if NesHawk's `sl 242, cycle 0` is | then its NMI assertion is | consequence |
|---|---|---|
| this core's `SL 241, dot 1` | one CPU cycle after this core's, in **all three** alignments | item 8 — and `05-nmi_timing` refuses it |
| this core's `SL 240, dot 339` | the first cycle-start dot ≥ 1, which this core's own sample point already produces | **no difference at all** — and frame 27 refuses it |

So the three cycles at frame 27 are not in the NMI path. They are a symptom of a difference in the
PPU's position that is neither constant, nor even, nor the dot skip's parity — which is as far as
this lot's instruments reach. The next lot's window is unchanged and its candidate list is shorter
by four.

### What the rounds did not reach

The brief allowed five rounds to frame 156 312. Round 1 spent itself eliminating the candidate
history named first, which was the right order — it is the one every lot since
NESHAWK-PROFILE-3 has come back to — and it is now closed by measurement rather than by argument.
The two candidates the brief named after it are **untouched** and are the next lot's: the
**OAM/DMC interleave** (item 13's unmeasured half — NesHawk with a DMC fetch owed on the transfer's
own halt cycle, which is what probably costs *Faxanadu* its 45 and 104 checksum lines) and the
**MMC3 IRQ polling point**. Neither can act in BizHawk frames 25-27, which carry `dmc=0/0` and no
IRQ, so neither could have been round 1.

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE-10 step 3/4 done: round 1, no item, four candidates closed by measurement
