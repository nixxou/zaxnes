# The two timing profiles of the 2A03, and how to add an item to the NesHawk one

Lots **NESHAWK-PROFILE**, **NESHAWK-PROFILE-2**, **NESHAWK-PROFILE-3** and
**NESHAWK-PROFILE-4**, branch `feat/neshawk-profile`, base `b604dc83` (lot EVAL-SYNC-4). The
working is `docs/progress/NESHAWK-PROFILE.md`, `-2.md`, `-3.md` and `-4.md`; the result in context
is `docs/review/MOVIE-SYNC.md` §4f to §4i, and the instruments the third lot needed — which the
fourth reused unchanged — are in `docs/review/NESHAWK-PROFILE-3-SCRIPTS.md`.

## What this is, in three sentences

Zaxnes's core is generic over a **timing profile**: `zax_nes::Standard` is the 2A03 it has always
emulated, and `zax_nes::NesHawkCompat` is one that makes BizHawk NesHawk's choices where they
differ. The `.bk2` player selects the second automatically when a movie's `Core` header says
`NesHawk`, and nothing else ever does. With it, the *Hoshi no Kirby* `.bk2` that parted at frame
926 replays with **no live difference from BizHawk's own work RAM until frame 8 005**, and both
*Faxanadu* dumps agree on **247** and **314** of their 832 checksum lines where the standard
profile manages 207 and 220.

> **NESHAWK-PROFILE-4 changed the headline, and downwards for one movie.** Lot NESHAWK-PROFILE-3
> took *Kirby* to frame **18 321** with item 8, a one-CPU-cycle lag of the whole /NMI line, and
> said in the same breath that the item was blunt. Lot NESHAWK-PROFILE-4 proved it is not a
> NesHawk difference at all — `ppu_vbl_nmi` `05-nmi_timing` measures the vblank NMI's arrival at
> one-PPU-dot resolution over all three alignments, and **NesHawk passes it**, so NesHawk takes
> that NMI on the cycle this core does — and **retracted** it, putting the boot difference it was
> standing in for (item 10) in its place. The trade is stated in full in §4i of
> `docs/review/MOVIE-SYNC.md`: eleven public ROMs and four `AccuracyCoin` rows and 111 and 145
> *Faxanadu* checksum lines bought back, and 10 316 *Kirby* frames given up.

## What it is **not**

**The NesHawk profile is not the more accurate chip.** On every item below, Zaxnes's own profile
agrees with Mesen 2 and with the nesdev wiki and the NesHawk one does not; the NesHawk profile
**fails five public test ROMs and five `AccuracyCoin` rows that the standard profile passes**
(`docs/progress/NESHAWK-PROFILE.md` step 3 has the first table, each failure traced to the item
that causes it). `AccuracyCoin` went 120/147 at NESHAWK-PROFILE, 122/147 at NESHAWK-PROFILE-2,
**118/147** at NESHAWK-PROFILE-3's item 8 and **back to 122/147** when NESHAWK-PROFILE-4 retracted
it; `ppu_vbl_nmi`, `vbl_nmi_timing` and `cpu_interrupts_v2` are **11/11, 7/7 and 6/6 on both
profiles** again. It exists so that a movie recorded against those choices can be replayed against
them, which is a different job from being right.

So: never make it the default, never offer it as a "better accuracy" setting, and never fix a game
with it. If a game needs it, the bug is somewhere else.

## The items, each with its source

Every item is an associated constant of `zax_nes::TimingProfile`, read at exactly the sites listed.
`Standard`'s value is `false` everywhere, and the `else` arm of every site is the code this core had
before the profile existed — not an equivalent of it, the same source.

| | what the NesHawk profile does | read from | sites |
|---|---|---|---|
| `DMC_HALT_ON_REQUEST_CYCLE` (1a) | the halt of a DMC reload lands on the **request cycle itself**, because `DMCUnit.Run()` lowers `RDY` and `RunOneFirst()` runs before `cpu.ExecuteOne()` | `APU.cs` `DMCUnit.Run`, `NES.Core.cs` `RunCpuOne` | `nes_bus.rs` `poll_dmc_dma` |
| `DMC_FETCH_HAS_TWO_DUMMIES` (1b) | a fetch is a **fixed four** cycles — halt, two re-reads, fetch — with no get/put alignment, there being no `% 2` on that path | `APU.cs` `dmc_dma_countdown = 3`, `RunDMCHaltFetch` | `nes_bus.rs` `promote_dmc_wait`, `dmc_fetch_cycles`, `run_dma`, `dma_cycle`, `is_a_lone_dmc_fetch` |
| `DMC_LENGTH_LAGS_THE_FETCH` (2) | the sample length, the loop and the IRQ land **three cycles after** the fetch, so `$4015` bit 4 stays set across them | `APU.cs` `DMCUnit.Fetch` (`dmc_reload_countdown = 3`) and `RunOneLast` | `apu/dmc_channel.rs` `supply_byte`, `step_reload_countdown`, `quiet_clocks`, the savestate; `apu/mod.rs` `step_cpu_cycle` |
| `DMC_ENABLE_WRITES_MID_SAMPLE` (3) | a `$4015` enable with the buffer empty starts a write-style fetch **even mid-sample**, delayed by the **DMC timer's** parity and three cycles long | `APU.cs` `set_lenctr_en` (the `!sample_buffer_filled` sibling test) and `Run`'s `else` branch | `apu/dmc_channel.rs` `set_enabled`; `nes_bus.rs` `after_apu_write` |
| `DMC_HALT_LANDS_ON_A_WRITE_CYCLE` (1c) | the halt lands on the request cycle **even when the CPU is writing on it**, so a request that falls on a run of an **odd** number of write cycles costs the fetch one cycle less than waiting for the next read cycle | `APU.cs` `DMCUnit.Run` (`RDY` lowered unconditionally), `NES.Core.cs` `RunCpuOne` (`if (!cpu.RDY && !cpu.rdy_freeze && countdown == DMC_RDY_check) countdown += 2`), `MOS6502X` `Execute.cs` (a write microstep never sets `rdy_freeze`) | `nes_bus.rs` `promote_dmc_wait`, `is_a_lone_dmc_fetch` |
| `PPU_START_UP_DOTS_SKIPPED` (10) | the PPU's **first frame after a power-on and after a soft reset is a start-up frame of `241 * 341 - 4 = 82 177` dots**, not a whole one of 89 342 — so the first frame boundary comes **2 389 CPU cycles earlier**, and the reset does it again from the top of the frame. 7 165 dots on the NesHawk profile, which is `89 342 - 82 177` | `NES/PPU.cs` `Reset()` (`ppudead = 1`, `start_up_offset = 4`, and `regs_reset()`'s `ppur = new PPUREGS()`), `NES/PPU.run.cs` `NewDeadPPU()`, `PPU.cs` `runppu()` (which clocks the CPU throughout) — against this core's power-on at the start of vblank (Q-003) and its `Ppu2C02::soft_reset`, which does not restart the frame (Q-078) | `nes_core.rs` `power_on` and `soft_reset`, through `NesBusOf::run_ppu_start_up_dots` and `Ppu2C02::run_start_up_dots` |
| `DMC_FETCH_STALLS_AN_OAM_TRANSFER` (7) | a DMC fetch that falls **inside an OAM transfer** costs that transfer **two** cycles where this core's loses one, because NesHawk's `dmc_realign` also skips the OAM cycle *after* the fetch | `NES.Core.cs` `RunCpuOne` (the OAM block's `dmc_dma_countdown != 1 && !dmc_realign` gate, and `dmc_realign = true` on the fetch cycle) | `nes_bus.rs` `run_dma`, the `dmc_may_read` arm |

**BizHawk is MIT** and nothing is copied from it: each item is written from the behaviour its source
describes, and the table names the function so that the reading can be checked. Mesen 2 is GPL-3.0
and the standard arms, which agree with it, carry their attribution where they already did.

**Which item closed *Kirby*** was measured rather than assumed, one item at a time:

| | |
|---|---|
| frame **926** | **1a** does it (1b then takes the residue from 17 frames to 8), **3** does it independently, and **2 is exactly inert** on that movie. `docs/progress/NESHAWK-PROFILE.md` step 5 |
| frame **4666** | **1c** does it, and the movie goes from parting at 4 665 to parting at **9 355** — 3 531 clean frames to 8 175 of 8 901, and 5 370 frames with a live difference to 726. **3's parity correction and 7 are *exactly* inert**: each leaves the 8 901-frame table byte-identical. `docs/progress/NESHAWK-PROFILE-2.md` step 4 |
| frame **9354** | **8** did it, and the movie went from parting at 9 353 to running out of the reference entirely — 8 175 clean frames to 8 792 of 8 901 and 0 frames identical to the byte to 213, and a second reference then took it to 18 321. **Item 8 has since been retracted**: it is not a NesHawk difference (see below), and with it gone the movie parts at 8 005 again. The OAM alignment that looked like the next item is **eliminated by arithmetic**. `docs/progress/NESHAWK-PROFILE-3.md` steps 2 and 3 |
| the **4 778-cycle drift**, and every ROM item 8 cost | **10** does it. BizHawk's own cycle ledger says its first frame is 27 392 cycles and so is the frame the movie's first reset lands in, where this core runs a whole 29 781 — and `2 * (29 781 - 27 392) = 4 778` is the constant drift NESHAWK-PROFILE-3 measured and could not explain. With item 10 the drift of the NMI entry over frames 2-1 201 is **-2 ± 5** instead of 4 776 ± 5, every one of the eleven ROMs and four `AccuracyCoin` rows item 8 cost is back, and *Faxanadu* goes to **247** and **314** of 832 — past the standard profile's own 207 and 220. *Kirby* parts at **8 005**. `docs/progress/NESHAWK-PROFILE-4.md` steps 1 and 2 |

**A parity trap, written down because it cost a lot a round.** NesHawk's DMC `timer` is **this
core's plus one** at every instant: it stores `timer_reload = DMC_RATE[i]` (428 for index 0) and
`Run()` reloads to it, so it runs 428 → 1, while `DmcChannel::clock_timer` stores `rates[i] - 1`
and runs 427 → 0. Both are read after the same cycle's clock. So **every `timer % 2` taken from
NesHawk's source has to be inverted here** — item 3 was written with the source's own text and the
wrong convention, and NESHAWK-PROFILE-2 round 2 corrected it.

## The candidates that are **not** implemented, and why

Read from the same two files, each with the reason it was left out, so nobody re-reads them from
scratch (NESHAWK-PROFILE step 1 items 5 and 6, and NESHAWK-PROFILE-2 step 3):

| | what NesHawk does | why not implemented |
|---|---|---|
| **item 5**, the APU register delay | `RunOneLast`'s `pending_reg` dispatch applies `$4003`, `$4007`, `$4010`, `$4015` and `$4017` at the end of the write's own cycle and **every other APU register on the first cycle with `dmc.timer % 2 == 0`** — so `$4000`-`$4002`, `$4004`-`$4006`, `$4008`, `$400A`-`$400E`, `$4011`, `$4012`, `$4013` all wait nought or one cycle. A second write while one is pending **loses the first**: `WriteReg` overwrites `pending_reg` with no flush | **Measured as CPU-timing-neutral on the one movie there is.** Of the registers it defers only `$4012` and `$4013` can move a DMC fetch, and the sample address they set is not read until the next fetch hundreds of cycles later. It would touch `Apu2A03::write` for every game and grow a pending-write register, a dispatch in `step_cpu_cycle` and a savestate field. **It is still the first item to add for a game whose audio differs**, because the lost-write rule is audible |
| **item 6**, the end-of-sample aborts | `fill_glitch` (a buffer fill with **no DMA at all**), `fill_glitch_2` (a one-cycle DMA then aborted by `RunCpuOne`), `pending_disable` (a `$4015` disable that waits for the reload countdown, with `dmc_irq_glitch` suppressing the IRQ) | **Eliminated by arithmetic for *Kirby*, not by choice.** Both flags are set only at a fetch that *completes* with `out_bits_remaining == 0`, and `DMCUnit.Clock` makes that the last bit period of a byte — seven bit periods and seventy-odd cycles from where a fetch can complete at 72 or 106 cycles a bit. **For a game at rate 14-15 (72 or 54 cycles a bit) with a short sample they are reachable**, and this is the item to try there. `set_lenctr_en(false)`'s first three arms need `sample_length != 0` with `out_bits_remaining == 0`, which *Kirby*'s one `$4015` = 0F of the window does not have |
| the **`$4016` strobe delay** | `RunOneLast`'s *first* action: a strobe is applied on the next cycle with **`dmc.timer % 2 == 1`**, not at the write | A **controller** item, not a DMA one, and nothing in this tree records it. *Kirby* strobes once a frame many cycles before it reads, so it cannot move this movie |
| the **IRQ sampling point** | `cpu.IRQ = _irq_apu \|\| Board.IrqSignal` is step 5 of `RunCpuOne`: after the DMA block and `RunOneFirst`, before `ExecuteOne` | *Kirby*'s MMC3 IRQ enters its handler at the **same dot** on both machines (EVAL-SYNC-4 §2, re-measured at SL 174 dot 288 in NESHAWK-PROFILE-2 step 2) |
| the **OAM transfer's alignment** | `NES.Core.cs`'s `sprdma_countdown` block: `if (apu.dmc.timer % 2 == 0) cpu_deadcounter = 2; else cpu_deadcounter = 1;`, so the extra cycle is decided by the parity of the **DMC's timer** and nothing in the decision looks at the cycle, where this core waits for the first get cycle | **Eliminated by arithmetic, and for every game rather than for one movie.** The DMC's timer counts down one per CPU cycle and reloads to `rates[i] - 1`, so its period is `rates[i]` — and all sixteen NTSC rates and all sixteen PAL ones are **even**. The timer's parity is therefore locked to the CPU cycle's from power-on, no reload and no `$4010` write can unlock it, and NesHawk's rule is a different *expression* of this core's rule. It was implemented, measured byte-identical over 81 frames, and reverted: a constant whose two arms are provably the same function cannot be given a differential test |
| the **branch interrupt quirk** | `Execute.cs` `RelBranch_Stage3` sets `branch_irq_hack` for every taken non-page-crossing branch (`interrupt_pending` is assigned `false` in `Fetch1` and never assigned `true` anywhere), and `Fetch1` then skips the whole interrupt test on the instruction that follows | **Eliminated by measurement against a reading that predicted the opposite.** The interrupt-phase ROM of NESHAWK-PROFILE-3 step 2 puts every `LDA` boundary of its spin loop immediately after such a branch, and NesHawk names the `LDA` as the displaced instruction on **20 of 64** frames. This core's own rule — a taken non-page-crossing branch polls one cycle earlier, `Cpu6502::branch` — is what `cpu_interrupts_v2` `5-branch_delays_irq` states in words, and both cores pass it |
| **item 8**, the CPU's view of /NMI a cycle late | `PPU.run.cs` `TickPPU_VBL` latches `nmi_destiny` and an `NMI_offset` read off `cpu_stepcounter` at cycle 0 of the NMI scanline and assigns `nes.cpu.NMI = true` at cycle `3 + NMI_offset`, so the assignment always lands on the dot after a CPU-cycle boundary and `Fetch1` reads it at the next one | **Implemented by NESHAWK-PROFILE-3, retracted by NESHAWK-PROFILE-4, and the retraction is the stronger result.** `ppu_vbl_nmi` `05-nmi_timing` prints *which instruction* the vblank NMI displaced, over ten lines that each move the whole test one PPU dot later, and checks the ten answers against a CRC — so it sweeps all three alignments at one-dot resolution. **NesHawk passes it**, and so does the standard profile, so NesHawk takes that NMI on the cycle this core does and no item that moves it can pass. Three shapes were implemented and measured before the conclusion was drawn (the table is `docs/progress/NESHAWK-PROFILE-4.md` step 1); all three fail it. What item 8 was really standing in for is **item 10**, the boot |
| the **power-on PPU position**, **`$2002` read timing** | — | the `$2002` read timing is ruled out by measurement in NESHAWK-PROFILE step 1: *Kirby* makes **zero** `$2002` reads in the frames measured. The **power-on PPU position** was ruled out there too — "a power-on offset would have parted at frame 1" — and that was **wrong**: it is item 10, and the reason frame 1 does not part is that the game spends its first frames in a wait loop. NESHAWK-PROFILE-4 step 2 measured it instead of arguing from it |

## The instrument: `frame-trace --dmc-census`

Built by NESHAWK-PROFILE-2 step 2 because aggregate counts could not diagnose frame 4666. It
prints, for one frame, every DMC request with the earliest cycle its halt was allowed on, every
halt, every fetch with the cycles it cost, every OAM request, every APU register write, and every
`$4015` and `$2002` read — each placed against the instruction it happened inside, with the DMC's
own `(timer, bits_remaining, bytes_remaining)` — and then the totals and a histogram of the fetch
costs.

    cargo build -p zax-harness --release --features dma-census
    zax-harness frame-trace <movie> <rom> --frame <n> --dmc-census

It is **an instrument and not emulation**: `crates/zax-nes/src/dma_census.rs` exists only under the
`dma-census` feature, and **every one of its nine call sites in `nes_bus.rs` carries
`#[cfg(feature = "dma-census")]` on the statement itself**, so with the feature off (the default,
and what every suite is run with) the hooks are removed before type-checking and the shipped core is
the core without them. Every site is reached at most once per fetch, once per transfer or once per
register access — never per cycle. Without the feature, `--dmc-census` is a **usage error naming
the flag to rebuild with**: an empty section would read as "this frame has no DMA", which is the
opposite of what it would mean.

## Two more instruments: the cycle ledger, and the interrupt-phase ROM

Lot NESHAWK-PROFILE-4 added none of its own: it ran `bk2cyc1.lua`'s existing output
(`kirby-jp-cyc0.txt`, frames 2 to 1 201 — the boot window, written but never analysed by the third
lot) through `drift.py`, and that is where item 10 came from. **A ledger already taken is worth
reading before a new one is taken.**

Built by NESHAWK-PROFILE-3, because the census could say what a frame's DMA cost and not **what
either machine was doing on a given cycle**.

### `frame-trace --event-log <file> [--event-pc <addr>]…`

The **cycle ledger**: one `F` line per frame carrying the CPU cycle the frame started on, its
length, its instruction count and its DMA bill, then one line per event with the cycle it happened
on *counted from the frame's start* — every NMI and IRQ handler entry, every `--event-pc`, and
(with `--dmc-census`) every `$40xx` access, the `$4016` strobe among them.

    zax-harness frame-trace <movie> <rom> --frame 1100 --frames 8901         --event-log ledger.txt --event-pc C06E --event-pc C37C

It captures no instruction and disassembles nothing, so 8 901 frames cost **19 s**; `--out` and
`--watch` are refused with it rather than served empty. **The format is a contract with the other
machine**: `bk2cyc1.lua` writes the same file from BizHawk, so the two can be aligned event for
event, and `frame_trace.rs`'s `the_cycle_ledger_writes_the_format_the_other_machine_is_read_in`
pins it so that changing it is visibly also a change to that script.

Two calibrations the alignment needs, both **measured and not assumed**:

* the two instruments report an *execute* at the cycle the instruction begins on and a *read* or a
  *write* **one cycle later**. With the +1 applied, `r$4015 → w$4011` comes out at exactly 0 on
  **75 of 75** occurrences of a 101-frame window — which no wrong value could do;
* a frame's own `start` is **not** comparable (BizHawk ends a frame on a PPU condition, the tracer
  notices the boundary after the instruction that crossed it). What is exact is an event's
  *absolute* cycle, and within a frame the cycles between two events. So each frame is anchored on
  its own NMI handler entry.

### The interrupt-phase ROM

`crates/zax-nes/tests/neshawk_profile_tests.rs`'s `phase_program`: a `LDA $00` / `BEQ` spin loop
and an NMI handler that records the low byte of the **interrupted** PC, in two variants that differ
in one bit — whether the taken branch crosses a page. The loop's phase relative to the vblank walks
by itself (a frame is 29 780 or 29 781 cycles, the loop's period 6 or 7), so 64 frames sweep every
phase with no delay slide to write.

    cargo test -p zax-nes --test neshawk_profile_tests interrupt_phase -- --nocapture
    ZAXNES_DUMP_ROM=phase.nes cargo test -p zax-nes --test neshawk_profile_tests         the_interrupt_phase_rom_is_written -- --ignored

The second command writes the `.nes` **BizHawk is then given**, so the two machines answer the same
bytes rather than two copies of the program; `phaserom.lua` reads the tables back. It is what
eliminated the branch quirk.

**What it cannot do**, recorded by NESHAWK-PROFILE-4 so that nobody spends a round on it again:
it cannot settle a question about the NMI *rule*. All three tables — NesHawk's, `Standard`'s and
`NesHawkCompat`'s — are the **same cyclic pattern** at different offsets (NesHawk's is this core's
rotated by 15 frames of the 18-frame period, and the two profiles' are two other rotations), so
what it measures is the loop's **phase**, which the boot sets. That is why NESHAWK-PROFILE-3
compared only its *shape*, and the shape is all it has.

## How to add an item — the whole recipe

1. **Read it, and write down where.** The item goes in the progress note of your lot with the
   NesHawk file and function it came from and what the lines mean. An item nobody can check is not
   an item.
2. **Add one associated constant** to `TimingProfile` in `crates/zax-nes/src/timing_profile.rs`,
   with the doc comment carrying the source; `false` (or "as today") on `Standard`, the NesHawk
   value on `NesHawkCompat`.
3. **Read it at the fewest sites you can**, each as `if P::YOUR_ITEM { … } else { … }` where the
   `else` arm is the existing code **unchanged**. That is what makes the standard profile free: the
   constant folds and the `else` arm is all that is emitted. Do not add a runtime field that the
   standard profile has to test.
4. **New state goes in the struct, and in the savestate only under the profile.**
   `DmcChannel::reload_countdown` is the example: `if P::DMC_LENGTH_LAGS_THE_FETCH {
   writer.write_i32(…) }`, so a standard block stays byte for byte what it always was and
   `NesCore::STATE_VERSION` does not move.
5. **If the state changes between cycles the APU may batch over**, make `quiet_clocks` answer 0
   while it is pending — again behind the constant. Forgetting this is the one error that produces a
   difference only on some games.
6. **Write a differential test** in `crates/zax-nes/tests/neshawk_profile_tests.rs`: measure the
   same thing on both profiles and assert the gap the item predicts. A test that pins only the
   NesHawk number cannot tell a regression of the standard profile from a change of yours.
7. **Re-run the two suites that say you did not break the standard path**, and no others count:
   * `golden-core` over the originals must stay
     `8d508070850b24228285b6f04df6f98c310dbdab65e7577921c91ad4d0689773`;
   * `test-roms --profile standard` over `dmc_dma_during_read4`, `sprdma_and_dmc_dma`, `apu_test`,
     `dmc_tests` and `AccuracyCoin` must be unchanged — 24 pass, 6 manual, 128/147.
   Then record what `--profile neshawk` scores and which item moved it — and then do the one thing
   NESHAWK-PROFILE-4 added, because it is what would have caught item 8: **run the public ROMs
   that measure the quantity your item is about on NesHawk itself.** If NesHawk passes one that
   your item makes the profile fail, stop. The item is a compensation and the difference is
   somewhere else.
8. **Measure the movie.** `movie-sync --profile neshawk` with `--ram-ref` against a per-frame
   reference, and `--profile standard` beside it so the two columns are from the same command.

### Eight traps that cost these four lots time, so they are written down

* **Every `timer % 2` from NesHawk's source must be inverted.** Its DMC timer is this core's plus
  one: `timer_reload = DMC_RATE[i]` reloading to 428 against this core's `rates[i] - 1` reloading
  to 427, both read after the same cycle's clock. Item 3 shipped with the source's own text and the
  wrong convention for a whole lot.
* **The arithmetic does not decide; the measurement does.** Twice now an item has been sized in
  advance, found far too small, and closed the frame anyway: item 1a against a "wrong sign" of 45
  to 55 cycles at frame 926, and item 1c worth **six** cycles in a frame whose miss was measured at
  **about two hundred and twenty**. What these items move is *which cycle* each fetch lands on and
  therefore the interleaving of the whole frame, which is not a sum over fetches. Size an item to
  decide what to read next, never to decide whether to implement it.
* **A differential test of one item can be an instrument for another.** Item 1b's test measured
  slips 1 and 2 and asserted `[5, 5]`; item 1c makes slip 1 cost 4, so the test had to move to the
  **even** slips 2 and 4 to go on testing item 1b's own claim. That the test failed is the system
  working: hold each item's test to a condition the *other* items do not vary.

* **An item whose two arms are the same function is not an item.** NesHawk expresses an OAM
  transfer's alignment on the parity of the DMC's timer and this core on the parity of the cycle,
  and the two are **the same predicate on every reachable state** because every one of the thirty-two
  DMC rate values is even and the timer's parity is therefore locked to the cycle's from power-on.
  The tell was that no differential test could be written for it: a test whose two columns are
  provably equal cannot fail, so there is nothing to ship. Ask that question *before* writing the
  constant.

* **An item can be real on one movie and not be a difference at all.** Item 8 took *Kirby* 8 968
  frames further and made 213 frames agree to the byte where none had, and it is **not** a NesHawk
  difference: the public ROM that asks its question directly, `ppu_vbl_nmi` `05-nmi_timing`, is one
  NesHawk passes. An item that moves a movie is evidence that *something* of that size is missing,
  and no evidence at all about **what**. Before shipping one, find the public ROM that measures the
  quantity and run it on the other machine; if the other machine passes it and your item makes the
  profile fail it, the item is a compensation and the real difference is elsewhere. Here it was two
  numbers away — the boot, item 10 — and the compensation cost eleven ROMs for two lots.
* **A ledger already taken is worth reading before a new one is taken.** Item 10 came out of
  `kirby-jp-cyc0.txt`, frames 2 to 1 201, which lot NESHAWK-PROFILE-3 wrote as a by-product and
  never analysed. Its first six lines carry the whole answer.
* **A reading can be wrong and the measurement is what says so.** NesHawk's `branch_irq_hack` is set
  for every taken non-page-crossing branch and `Fetch1` skips the interrupt test on the instruction
  after it — read verbatim from the 2.11.1 tag, three times, in three phrasings. It predicts that
  NesHawk can never take the NMI at the boundary after such a branch. It does, on 20 of 64 frames of
  a ROM written to ask. **Build the experiment before implementing what the source seems to say**;
  this one was forty lines of 6502 and it saved a round.

* **A hot entry point must be concrete per profile.** Rust monomorphises a generic function in the
  crate that *calls* it, and a generic body with one call site is inlined into its wrapper. A
  generic `run_frame` cost **+3.8 %** on Super Mario Bros. 3 for both reasons together, with no
  emulation changed. `NesCoreOf<Standard>::run_frame` and `NesCoreOf<NesHawkCompat>::run_frame` are
  therefore declared separately, each a one-line forward to an `#[inline(never)]` generic body. A
  new hot entry point needs the same treatment; a cold one does not. `nes_core.rs` carries the
  measurements beside the code.
* **A default type parameter does not help callers.** Rust applies a type parameter's default when
  it resolves a type and **not** when it resolves a path through one, so `NesCore::STATE_VERSION`
  and `NesCore::new()` both stop compiling behind `NesCore<P = Standard>`. The generic structs are
  therefore `NesCoreOf<P>` and `NesBusOf<P>`, with `pub type NesCore = NesCoreOf<Standard>` and
  `pub type NesBus = NesBusOf<Standard>` — plain aliases to a concrete instantiation, which resolve
  everywhere. That is why not one line outside `zax-nes` had to change.

## Where the profile is chosen, and the one place to change it

| | |
|---|---|
| the rule | `MovieFile::recorded_profile` in `zax-harness/src/movie_file.rs`: a `.bk2` whose `Core` header is exactly `NesHawk`, and nothing else — not `QuickNes`, not an `.fm2`, not a header with no `Core` line |
| the override | `--profile standard\|neshawk` on `movie-sync`, `frame-trace` and `test-roms`, through `zax-harness/src/profile_choice.rs`. An unknown value is a usage error, never a silent fall-back |
| where it is read | `movie_sync.rs` `zaxnes_machine` (one `match`, then a `Box<dyn Machine>`) and `frame_trace.rs` `run` → `trace_on::<P>` |
| in a report | `ZaxnesMachine::name()` answers `zaxnes` or `zaxnes/neshawk`, so no figure can be mistaken for the other chip's |
| in a savestate | `TimingProfile::STATE_CORE_NAME` — `"Zaxnes"` or `"Zaxnes-NesHawk"` — at the head of the block, so a block of one profile is refused by the other with the message REQ-CORE-120 fixes, and `STATE_VERSION` never moved |

**The application cannot open a `.bk2` at all** (`grep -rn bk2 crates/zax-app/src/` finds nothing;
the movie dialog admits `.fm2`, `.fcm` and `.mv`), so there is nothing in it that selects a profile
today. The day it gains a `.bk2` importer, `Engine` is already implemented for both profiles and the
rule above is one function call — and the switch must be **for that playback only**, never written
to the game's own engine setting, and said on screen, which is what lot FCEUX-48 settled for `.fm2`.

## The cost of having two chips

| | |
|---|---|
| speed, standard profile | within the +1 % budget all four lots were held to: NESHAWK-PROFILE's `bench-core` on five ROMs read -3.0 %, -3.0 %, -2.2 %, +0.5 %, +0.6 % against its parent, NESHAWK-PROFILE-2's on Super Mario Bros. 3 and Mega Man 3 read **-2.24 %** and **-1.04 %**, NESHAWK-PROFILE-3's read **+0.77 %** and **+0.40 %**, and NESHAWK-PROFILE-4's read **-1.25 %** and **+0.27 %** against it, with `golden-core` byte-identical throughout. **Cumulatively since `428c50e6`, the parent of the last three lots: -2.94 % and -1.50 %** — the whole series is faster than it started. Item 8's `bool` is gone from `NesBusOf` and item 10 adds no field at all |
| behaviour, standard profile | `golden-core` over 98 ROMs x 1 200 frames **byte-identical** in all four lots (`8d508070...`); `AccuracyCoin` 128/147 unchanged; 24 of 24 DMC/DMA ROMs unchanged; `ppu_vbl_nmi` 11/11, `vbl_nmi_timing` 7/7 and `cpu_interrupts_v2` 6/6 unchanged |
| memory | none: a profile is a zero-sized type, and the two cores are the same size |
| binary | **+147 408 bytes of `.text`** (+3.0 %) for a whole second CPU, bus, APU and DMA — a fourteenth of the 2 MB that would have put it behind a feature, so it is behind none |
| savestates | unchanged for the standard profile, to the byte and to the version |
| tests | `zax-nes` 1 142 passing with the ignored ones (the same 1 269 `#[test]` items as before NESHAWK-PROFILE-4; the earlier lots' totals were counted by a different invocation), of which eight are the items' own differential tests in `tests/neshawk_profile_tests.rs` and six the architecture's in `tests/timing_profile_tests.rs` |
