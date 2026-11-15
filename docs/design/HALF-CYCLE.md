# HALF-CYCLE — the Fidélité+ stepper (design, lot FIDPLUS-2 step 3, 2026-11-15)

## The goal (owner, 2026-11-15)

"In Fidélité+ we must be the most faithful of all emulators, beyond everything done today. No
compromise." Fidélité+ is therefore not one stepper but a mode with five parts, each measured:

| Part | What | Reference it must meet |
|---|---|---|
| A. Half-cycle machine | this note: one master clock, CPU and PPU on φ1/φ2 slots (BeesNES's scheduler model), register accesses on their /CS and M2 edges, AD bus and latches as state | the transistor netlist per dot, A5 corpus, all four alignments, **no calibrated rule** |
| B. Real-time pacing | the machine advanced by host time converted to master clocks (BeesNES `CSystem::Tick`), the picture leaving at the console's 60.0988 Hz, the pad sampled when the game strobes `$4016` (FIDELITY-LEADS A2) | console frame rate exactly; input latency below one frame |
| C. Console model | every hardware variant selectable and consistent: 2C02 letter revisions, RP2A03 letter revisions, MMC3 rev A/B and MMC6, power-on state (random CPU/PPU alignment, RAM, palette), PAL 2C07 measured as the 2C02 was | each test ROM passes under the console model it was written for, including today's four fails (`power_up_palette`, `5.MMC3_rev_A`, `6-MMC6`, `6-MMC3_alt`, which exclude each other in a single model) |
| D. Analog output | audio from the APU at the CPU rate through the measured console chain (E1 profiles, no output-rate approximation of the low-pass); video as the composite signal (H1), not a palette lookup | BeesNES's measured filters; console recordings |
| E. Proof | every claim backed by a measurement: netlist differential, TriCNES/AccuracyCoin, console videos the owner provides; every residual approximation listed in FIDELITY-LEADS | AccuracyCoin 147 in all four alignments; every suite any other emulator passes |

**Placement rule (owner, 2026-11-15)**: what costs performance is gated behind the Fidélité+
option (its own code, the standard core pays nothing); what costs nothing goes in the common base
and benefits everyone. So: A (half-cycle machine) and D's per-CPU-rate audio chain are Fidélité+;
composite video is an option of the shaders lot; B (real-time pacing, pad read at the strobe) and C
(console model) go in the common base; rules found with A that can be written cold are ported to
the standard core too (as A5's were, +0.02 to +0.10 %).

Run-ahead, rewind, 3D, widescreen and the hack packs must keep working in Fidélité+ (they rebuild
from savestates and per-frame records): part B paces the machine but the frame records and the
savestate format stay the boundary they use. Run-ahead stays exact (it replays from a
savestate; only what is shown is ahead); its depth is limited by the slower machine, measured and
shown in the interface. Real-time pacing needs a presentation step that shows the last finished
frame without stutter at any refresh rate (60 Hz, 120 Hz smooth scroll, VRR).

Limits stated up front: effects decided by analogue charge (OAM corruption copy, the AD bus
floating value of A5) are not settled by the netlist; for those the reference is a console
measurement, and without one the mode keeps the best measured rule and lists it.

## Why, and what it is not

The standard core steps one CPU cycle at a time, runs the PPU's dots in batches and places every
event inside a cycle by calculation (read at D, `$2006` load at D+3, `$2007` latch at D+5/D+6,
`$2001` at D+2, the dot-257 and increment rules of A5). Every such rule is exact where it was
measured; what it cannot do is order two events that land inside the same dot without a new rule.
The per-dot differential of A5 found four such interactions in one night (INC `$2007`, dot 257
after `$2001`, the `$2007` read merge, the `$2006` D+4 increment) and one it could not model
from outside (the AD bus held across a rendering switch, the tile latch).

The half-cycle stepper is a **second stepping loop**, chosen per frame as the board path is
(CORE-TIERS, "Fidélité+ in the same model"). It does not replace the standard core, which keeps
its speed; it does not try to be a netlist (analogue charge, OAM corruption races stay measured
rules).

## The clock

One master clock (21.477 MHz NTSC). The loop advances in **half PPU dots** (2 master clocks):

| Signal | Period (master clocks) | Edges the loop acts on |
|---|---|---|
| PPU dot | 4 | pclk0 (first half), pclk1 (second half) |
| CPU cycle | 12 | M2 rise, M2 fall (duty 15/24, 2A03G), φ1/φ2 |
| PPU register access | from M2 | /CS fall = M2 rise + 1 half; /CS rise = M2 fall + RISE (A3c: RISE 1-2) |

The CPU/PPU alignment (0-3, `emu.cpu_ppu_alignment`) is the master-clock offset between the two,
exactly the four phases of the netlist bench (0 = a2, 1 = a0, 2 = a6, 3 = a4).

## State the stepper holds that the standard core does not

* the **AD bus** (`ad`), driven by the ALE half (address low byte), by a /RD half (data), and
  keeping its value otherwise (A5 tile latch: the tile register latches it on a nametable dot even
  with the rendering off);
* the **address latch** (`ale_low`, already in the standard core) and the high address lines as
  separate state, so a stale fetch reads the whole stale address (A5, step 39 correction);
* the **`$2007` data latch** and its write strobe chain (INC `$2007`, A5), the read state machine;
* the mask, control and `v`/`t` loads applied **on their edge** (`$2001` store on /CS rise, `$2006`
  load on the pclk1 after the first pclk0 following the store), not by a delay constant;
* the increment signals of the dot (coarse X, Y, copy 257, copies 280-304) as edge events, so a
  load and an increment on the same dot resolve by order, not by a rule.

## What it shares, unchanged

Boards (`BoardKind`/`AnyBoard`, through `CartAccess`), APU, OAM and sprite evaluation, palette,
tile records (3D, Lua, widescreen, hack packs), savestate primitives. The PPU's **per-dot work**
(`process_dot`'s fetch/evaluation/pixel) is called from the half-dot loop; only what happens
*between* the halves is new. If a PPU unit needs a half-dot split (the fetch's ALE half and /RD
half), it gets it inside the stepper module, not in the standard path.

## Savestates

A Fidélité+ state carries the extra fields (AD bus, latch chain, pending edges) in its own block;
loaded into the standard core it is refused, as between engines (CORE-TIERS core 5).

## Judge

The A5 corpus, in the four alignments: `g2006px`, `g2001px`, `g2005px`, `g3e01px`, `g2000`,
`oam`, `r2007`, `g2006x`, `rmw`, `z2`. Acceptance: **every whole frame equal to the netlist per
dot** (`v`, `t`, accesses; then the `bus` column) with **no A5 rule** in the stepper, alignments 2
and 3 first (0 and 1 carry the switch-on race of step 33). Then AccuracyCoin 147 x4, the campaign
(215 / 4), golden-core (pictures may move only where A5 says the netlist differs).

## Cost

Not bound by the release budget (opt-in), but measured: target under 2x the standard core on the
PR-1 set; the standard path must stay within +0.02 % (the per-frame choice only).

## Plan (part A; parts B to E follow in their own lots)

| Step | What | Estimate |
|---|---|---|
| 1 | Module skeleton, per-frame choice in `NesCore`, option `emu.fidelity_plus` (off), CPU on M2 edges with the standard PPU per dot: equal to the standard core on every golden | 1 day |
| 2 | PPU register edges (/CS fall/rise per alignment), `v`/`t` loads and increments as edges; A5 corpus on alignments 2-3 with A5's rules removed | 2 days |
| 3 | AD bus and address latch as state, `$2007` latch chain; tile latch, stale fetch; `bus` column | 1-2 days |
| 4 | Alignments 0-1, savestates, app option and menu, docs, validation | 1-2 days |

## Step 3 design: the PPU DATA state machines (2026-11-15)

Source: `AccuracyCoin.asm`, notes of `$2007 Stress Test` (the chain of five D latches clocked by
alternate PPU clock halves, the SR latch, the NOR outputs), read against the netlist mapping of A5.
Let E be the PPU half-dot in which M2 falls at the end of the access (`t0.1` in the notes). Then:

| Chain | ALE (address latch takes `v`'s low byte) | Data | `v` increment |
|---|---|---|---|
| `$2007` read | halves E+3, E+4 | Read on halves E+7, E+8: the read buffer takes the bus | t5, with the dot's own increment lines |
| `$2007` write | halves E+3, E+4 | Write on halves E+6, E+7 (suppressed for palette RAM, stored inside) | t5 |

With the alignment offsets (`fidplus::MasterClock`), E is the second half of dot D+1 in alignments
2-3 and its first half in 0-1. That gives back, without a constant per alignment, every latch
dot A3c measured (read: whole of D+5 then latch at the start of D+6 in 2-3, across D+4/D+5 in 0-1)
and step 2's palette increment on D+6.

What becomes state in Fidélité+:
* the two chains (five latches and the SR latch each, stepped on PPU clock halves);
* the AD bus (`ad`) and the external octal latch (`ale_low`), driven by the background/sprite
  cadence (ALE on odd dots, read on even dots) **and** by the chains: when both drive ALE the
  cadence wins (notes, "the background fetch takes priority"); a chain ALE on a cadence read half,
  or the reverse, is the analogue feedback case: the notes and the netlist call it unstable, and
  the mode keeps the best measured value (A3c: the even-X / odd-X rule) and lists it;
* the background fetch split into its ALE half (address high lines and latch) and its read half
  (high lines of that moment, low byte of the latch): the hybrid `$2006` address, the stale fetch
  and the tile latch of A5 are then the bus, not patches;
* A7's coarse-Y wrap in two phases.

The standard path keeps its whole-dot fetch; the split fetch lives in `ppu/edges.rs` (or a
`ppu/fidplus/` module) and runs only in Fidélité+.
