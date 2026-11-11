# NESHAWK-PROFILE-8 — the CPU phase is born at the movie's reset, and item 1c is right after all

Branch `feat/neshawk-profile`, worktree `cleanroom/wt-bk2`, base `c20c8cb1` (lot NESHAWK-PROFILE-7).

## Steps

- [x] 1. **Where the CPU phase is born.** The four once-a-frame cycle marks of frames 1-56 on both
      machines; the residual is a **constant** and leaves it at frame **12**; the first instruction
      whose start cycle differs is frame 11's `$C562 STA ($18,X)`, 10 cycles here against **9**
      there. Two differences found, both measured: **item 1c is right** (705 fetches, no
      exceptions) and the movie's **reset lands 3 CPU cycles late** here.
- [ ] 2. Explain and fix: item 1c restored, with a differential test.
- [ ] 3. Rounds.
- [ ] 4. No regression, report, final pass.

## The environment this lot runs in

| | |
|---|---|
| `CARGO_TARGET_DIR` | `<worktree>/rust/target-ev` |
| `TMP` / `TEMP` | `<worktree>/rust/target-ev/tmp` |
| `CARGO_INCREMENTAL` | `0` |
| disk on `G:` at the start | 5.5 GB free (the brief's stop is 5 GB) — the big dumps went to the scratchpad on `C:` |
| disk on `C:` at the start | 11 GB free (the brief's stop is 1 GB) |

## The sources read, and their licences

| | |
|---|---|
| **BizHawk 2.11.1 run as a program** | `EmuHawk.exe --movie --lua`, the *Kirby* `.bk2`, NesHawk. Four new scripts (`q183.lua`, `q183b`-`q183d`), §"The instruments added" |
| **BizHawk** (NesHawk), MIT, tag 2.11.1 | nothing new read this lot: the item restored is the one lot NESHAWK-PROFILE-5 read off `NES.Core.cs` and `MOS 6502X/Execute.cs`, and the measurement below is what settles it |

---

## Step 1 — where the CPU phase is born

### The answer first

1. **The CPU-phase residual is a constant, and it is not 0.** `zaxnes − bizhawk` on an execute, over
   the NMI handler entry, its `RTI` at `$C3A1`, `$C06E` (the last instruction before the frame-sync
   spin loop) and `$C074` (the loop's exit) of every frame 1 to 56: **−1** on every mark of frames 3
   and 4, **+2** on every mark of frames 7 to 11, and from frame **12** it jitters (−5 to +7).
2. **The jitter starts inside frame 11**, the movie's first frame with any DMC fetch (69) or OAM
   transfer (1) at all. Aligned execute to execute, the residual leaves +2 at **four** of those 69
   fetches and reaches +6 by the frame's end; frames 1-10 have nothing that could move it.
3. **The first instruction whose start cycle differs** is frame 11's instruction 5000,
   `$C562 STA ($18,X)`: **10** cycles here, **9** on NesHawk, with one DMC reload fetch on it.
   NesHawk's halt lands on the instruction's own **write** cycle — the stream carries
   `w 68E7` and the DMA's first dummy read on the *same* cycle — and the fetch then costs the CPU
   **three** cycles where this core charges four.
4. **That is item 1c, and lot NESHAWK-PROFILE-6 retracted it wrongly.** Tabulated from BizHawk's own
   stream over 36 frames and **705** fetches, with no cross-machine calibration anywhere: a fetch
   whose halt passes **0** CPU writes costs **4** (648 of 648), **1** costs **3** (46 of 46) and
   **2** costs **4** (11 of 11). An **odd slip costs one cycle less**, which is item 1c's own words.
5. **And the +2 itself is a second, deeper difference: the movie's reset lands 3 CPU cycles late
   here.** NesHawk's frame ends at a fixed PPU dot, **mid-instruction**; this core's ends after the
   instruction that straddles it. At the `.bk2`'s reset (after frame 4) NesHawk therefore begins its
   7 reset cycles at its cycle 116 735 — the cycle this core's straddling instruction *starts* on —
   and this core begins them 3 cycles later. The reset restarts the PPU (item 10), the game is
   NMI-synced and the DMC's train is CPU-counter-clocked, so **the game's code runs 3 cycles late
   against every DMC fetch for the rest of the movie** — which is what puts NesHawk's halts on
   write cycles and this core's on read cycles in the first place.

### The four marks, and what they are

Read off `frame-trace --frame 55 --out`; each is executed **once a frame**, which is what makes the
ledger four `event.onmemoryexecute` registrations rather than an `onmemoryexecuteany`:

| | |
|---|---|
| `$C341` | the NMI handler's first instruction (the vector's own target) |
| `$C3A1` | its `RTI` |
| `$C06E` | `STA $39`, the **last instruction before the spin loop**, reached by `JSR $C067` from the main thread at `$E5BC` |
| `$C070`/`$C072` | `LDA $39` / `BEQ $C070` — the 6-cycle spin loop the next NMI breaks |
| `$C074` | `PLA`, the loop's exit |

### The residual, frames 1 to 56

`q183.lua` against `frame-trace --frame 1 --frames 57 --event-log --event-pc C3A1 --event-pc C06E
--event-pc C074`. Both instruments date an execute at the instruction's start, so the marks compare
with **no offset applied**. (Frames 1, 2, 5, 6 reach no mark: the power-on and the reset put the
game in its boot code, and frames 12 on leave the `$C067` wait routine for good.)

| frames | nmi | rti | c074 | c06e |
|---|---|---|---|---|
| 3, 4 (before the reset) | **−1** | −1 | −1 | −1 |
| 7, 8, 9, 10, 11 (after it) | **+2** | +2 | +2 | +2 |
| 12 | **+1** | +1 | — | — |
| 13-56 | −5 … +7 | | | |
| **40** | +2 | **+165** | +162 | +164 |
| **55** | +2 | **−160** | −160 | −160 |

Frames 40 and 55 are the collisions: one machine's input routine runs an extra pass and the other's
does not, and the frame costs it ~160 cycles. **Frame 40 is this core's extra pass and frame 55 is
NesHawk's** — so the residual is not one-sided, which is what a phase difference looks like.

### Inside frame 11, execute to execute

`q183b.lua` (the whole bus stream of tree frames 11-12) against `frame-trace --frame 11 --frames 2
--out`, 9 083 instructions aligned, the ±4 and −512 one-instruction spikes (a fetch or a transfer
dated at a different point *inside* an instruction, the CPU resuming on the same cycle) filtered out:

| instruction | PC | zax | biz | residual |
|---|---|---|---|---|
| 0 | `$C341` | 293 044 | 293 042 | +2 |
| 5 000 | `$C562 STA ($18,X)` | 310 583 | 310 580 | +2 → **+3** |
| 5 107 | `$C562 STA ($18,X)` | 311 016 | 311 012 | +4 |
| 5 909 | `$C50A DEC $00` | 314 473 | 314 468 | +5 |
| 6 318 | `$C555` (`JSR $C504`) | 316 203 | 316 197 | +6 |
| frame 12's `$C341` | | 322 823 | 322 822 | **+1** |

The NMI is PPU-driven, so the frame boundary resets the accumulated +6 to a phase — and the phase
is what frames 13-56 jitter in.

### The cost of a fetch on NesHawk, from BizHawk's own stream

`q183c.lua`, tree frames 11-46, 25.7 MB. For every read of `$FC00`-`$FEFF` (this game's sample data
and nothing else) the containing instruction's **nominal** length is the shortest duration that PC is
ever seen to take in the window, so `stall = duration − nominal` is a difference of two BizHawk
numbers. The writes that passed **through** the DMA are the cycles carrying a `w` **and** an `r`:
NesHawk's `MOS 6502X` write microstep ignores `RDY`, so a write that shares its cycle with a DMA
dummy read is exactly this core's **slip**. PCs with a page-crossing variant are dropped (their two
clean durations differ by one, so `min` would inflate the stall): 1 572 PCs keep a reliable nominal.

| slip | stall 3 | stall 4 | total |
|---|---|---|---|
| **0** | 0 | **648** | 648 |
| **1** | **46** | 0 | 46 |
| **2** | 0 | **11** | 11 |

**No exceptions in 705 fetches.** `stall = 4 − (slip mod 2)`, which is item 1c.

Why lot 6 measured the opposite: it read the slip off **this core's** census and the stall off
BizHawk's stream. The slip is a property of what the **CPU** is doing on the request cycle, and the
two machines' CPUs are +2 cycles apart in the instruction from the reset onwards — so a fetch this
core calls slip 1 is one NesHawk calls slip 0 or 2. Its own table said so and was read the other
way: "slip 1 came out 4 six times and **3 ten times**", and the ten were the measurement.

### The reset, which is where the +2 comes from

`q183d.lua`, the whole bus stream of tree frames 3-8. Frames 1-10 make no DMC fetch and no OAM
transfer, so nothing else in the window can move the residual.

| | **NesHawk** | this core (its own clock; the residual before the reset is −1) |
|---|---|---|
| the last instruction before the reset | `$C072` at 116 732, ending 116 734 | `$C070` at 116 734, ending 116 736 |
| the 7 reset cycles | 116 735 … 116 741 | 116 737 … 116 743 |
| the first instruction after it (`$FFF0 SEI`) | 116 742 | 116 744 |
| the reset's own length | **7 cycles** | **7 cycles** |

NesHawk's reset begins on the cycle this core's last instruction *starts* on. Its frame boundary
falls **mid-instruction** as a rule — of the six boundaries in the window, one lands on an
instruction's last cycle (116 734), one on an instruction's first (203 688) and the rest inside one —
because BizHawk interleaves the CPU and the PPU per cycle and ends a frame on a PPU condition, where
this core's `run_frame_profiled` loop breaks **after** the instruction `vblank_just_started()` fired
inside. The quantity is confirmed a second way, with no cross-machine calibration: the cycles from
NesHawk's boundary to its own NMI entry are **10** at tree frame 8 and this core's straddling
instruction is **10** cycles before its own NMI entry; at tree frame 4 the two read 11 and 10.

### The instruments added

| | |
|---|---|
| `q183.lua` | the four-mark ledger of frames 1-56 — four `onmemoryexecute` registrations, 2.7 s a run |
| `q183b.lua` | the whole bus stream of tree frames 11-12 (1.3 MB), for the execute-to-execute alignment |
| `q183c.lua` | the same over tree frames 11-46 (25.7 MB), for the 705-fetch slip table |
| `q183d.lua` | the same over tree frames 3-8, for the reset |

Full texts in `docs/review/NESHAWK-PROFILE-8-SCRIPTS.md`.

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE-8 step 1 done: the phase is born at the reset, and item 1c is right on 705 fetches
