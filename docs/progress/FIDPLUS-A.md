# FIDPLUS-A — Fidélité+ part A, the half-cycle machine (progress)

Design: `docs/design/HALF-CYCLE.md`. Branch `feat/fidplus-2` (worktree `wt-fidplus2`).

## Step 1 — skeleton (2026-11-15)

* `zax-nes/src/fidplus.rs`: `MasterClock`, the dividers of each region in master clocks (12/4
  NTSC, PAL-M, PAL-N; 16/5 PAL; 15/5 Dendy), M2 rise at 9/24 of the cycle and fall at its end
  (2A03G duty 15/24), /CS fall one half dot after M2 rise, the four alignment offsets one master
  clock apart (A5 mapping `0 = a2`, `1 = a0`, `2 = a6`, `3 = a4`). PAL and Dendy duty taken as
  the 2A03G's until measured (FIDELITY-LEADS A6).
* `NesCore::set_fidelity_plus` (off by default, not in the state yet): the frame runs through
  `run_fidelity_plus_steps`, its own loop, chosen once per frame as the board path is; no PPU or
  APU batching (`defer_ppu` is not called).
* `nes_bus::FidelityBus<K>`: the bus that loop lends the CPU. Step 1 forwards every access
  unchanged; step 2 splits each cycle at its edges here, out of the standard path.
* `run_frame` split in `run_standard_frame` and `finish_frame`; the standard path pays one test of
  a boolean per frame.
* Acceptance: `tests/fidplus_equivalence_tests.rs` (ignored, needs ROMs): every ROM of
  `ZAXNES_TEST_ROMS` and `ZAXNES_ROMS`, 600 frames with a button pattern, cycles and picture of
  every frame and the savestate at the end equal in both modes.

Measured: callgrind against 6c9cb163 (clean builds), PR-1 set -0.000 % to +0.001 %.

## Step 2a — edges for `$2006` and `$2007` writes (2026-11-15)

* `ppu/edges.rs`: a queue of at most four edges; `run_dots` runs dot by dot while one is
  pending (one test of a counter on the standard path). An edge lands after the work of a dot
  computed from **one latency per signal**, in half master clocks from M2 fall, plus the
  alignment's master-clock offset: `ceil((24 + offset + latency) / 8) - 3` dots after the access
  dot. With the offsets of the netlist mapping, the per-alignment tables reduce to one latency
  each (worked out by hand, used from step 2b on): the `$2007` read latch's D+5 (alignments 0-1)
  / D+6 (2-3) is latency 30, `$2001`'s two dots in all alignments 10, `$2006`'s three 18.
* Second `$2006` write: `v` takes `t` after the work of dot D+3 (latency 18), whatever the
  rendering (the load is never immediate on the chip). The D+4 increment rule of A5 is not written:
  dot D+4 runs with the new `v`. Kept from the standard core until step 3: the hybrid fetch address
  and the `$2006` collision.
* Second `$2006` write on dot 257 while fetching: the horizontal copy takes the written coarse X
  in alignment 2, the old `t`'s AND the written one in alignment 3 (netlist, `g2006x`); 0 and 1
  given the written value as the `$2005` measurement (A4).
* `$2007` write outside rendering, palette included: the data latch at the access, the store and
  the increment after dot D+6 (latency 42; `g2006px` palette writes). `INC $2007`'s first address
  takes the second value with no rule (test `fidplus_a2_inc_2007_follows_from_the_delayed_store`,
  four alignments).
* Judge (`ext-tools/visualnes-bench/fidjudge.sh`, dots whose `v`/`t` differ from the netlist over
  three whole frames, alignments 2 and 3):

  | Program | Standard | Fidélité+ | Left |
  |---|---|---|---|
  | `g2006px` | 1210 | 72 | the trace's first frame start (line 241, 72 dots, both modes) |
  | `g2006x` | 13342 | 74 | first frame start, one `$2006` write on the vertical copy of dot 304 |
  | `g2001px`, `g3e01px` | 114 | 72 | first frame start |
  | `g2005px` | 97 | 75 | first frame start, 3 dots |
  | `g2000`, `oam`, `r2007`, `g2005` | unchanged | unchanged | registers not converted yet |

  The standard core's differences are mostly its prepaid dots seen by the trace (the old `v` while
  it runs ahead), not pictures: the golden and the campaign are its judges.
* Harness: `dot-trace --fidelity-plus`; `ZAXNES_FIDELITY_PLUS=1` for `test-roms`, `golden-core` and
  the renderer.
* `tests/fidplus_equivalence_tests.rs` now checks PAL and Dendy (no edge there): equal to the
  standard core.

Validation of step 2a with `ZAXNES_FIDELITY_PLUS=1`: campaign 215 / 4, AccuracyCoin passes in the
four alignments, golden-core the same as the standard core (SMB3 Japan and Zelda II Europe, the
known moves).

## Step 2b — `$2007` read as edges (2026-11-15)

* The read returns the buffer (the palette at once); the latch is an edge at the start of dot X
  (latency 30: D+5 in alignments 0-1, D+6 in 2-3): in a blanking the byte at `v`, on an odd
  fetching X the byte X - 1 left on the bus; on an even fetching X, after X's work, the byte X's
  fetch read. The increment comes in the middle of X (after X's work): visible on dot X, as the
  netlist shows it (the first version, one dot early, left 2008 runs on `r2007`).
* What stays written is what the netlist says about signals, not about order: the read's increment
  lines and the pipeline's are the same (on a natural coarse-X dot only Y is added, on 256
  nothing), and the horizontal copy of 257 wins over the read's coarse X.
* A bug found on the way: an edge posted from inside an edge was counted down in the same dot.
* `r2007`: 250 708 dots (standard) -> 85 875, all of them the program's post-render lines (the
  netlist's PPUCTRL powers on with bit 2 set, the program never writes `$2000`, A7) and one dot at
  each coarse-Y wrap from 29 (the two-phase wrap, A7, step 3).

## Step 2c — `$2001` as an edge (2026-11-15)

* The rendering switch is an edge after the work of D+2 (latency 10), the line read when it lands;
  the rest is the standard core's own post-switch code. A switch landing on dot 256: the copy of
  dot 257 comes in its first half and follows the old state (after a switch off it is posted as an
  edge after dot 257's work, where the netlist shows it). `g2001px`: equal to the netlist.

## Validation after 2c, and the `ALE + Read` fix

* Campaign 215 / 4 and golden as the standard core; AccuracyCoin failed `ALE + Read` in the four
  alignments: the refetch of an odd-X read was made after X's work and overwritten by X + 1's
  normal read. Now an edge after X + 1's work: 147/147 in alignments 2 and 3.
* Then `$2007 Stress Test` failed in alignments 0 and 1 (146/147): a read whose latch dot passes
  dot 340 took the parity of the unwrapped number. Fixed: **AccuracyCoin 147/147 in the four
  alignments in Fidélité+.** The test's notes describe the PPU DATA state machine half cycle by
  half cycle (it starts at M2 fall; ALE two halves, idle two, Read two). Read against this
  mapping they give A3c's latch dots: M2 falls in the second half of dot D+1 in alignments 2-3
  (Read on the whole of D+5, latch at the start of D+6) and in its first half in 0-1 (Read across
  D+4 / D+5, latch D+5). Step 3 makes that machine state instead of a latency.

Corpus without the netlist's first frame (power-on artefact), alignments 2 and 3, dots whose
`v`/`t` differ over the two whole frames left:

| Program | Standard | Fidélité+ |
|---|---|---|
| `g2006px` | 740 | 0 |
| `g2006x` | 8850 | 2 (the `$2006` write on dot 304) |
| `g2001px`, `g3e01px` | 0 | 0 |
| `g2005px`, `g2000` | 2 | 2 |
| `oam` | 1 | 1 |
| `r2007` | 168 152 | 78 806 (post-render lines, the bench's PPUCTRL, A7) |
| `g2005` | 57 399 | 57 399 (same artefact) |

## Order changed: step 4 before step 3 (2026-11-15)

Step 4 makes the mode usable (alignments 0-1 checked, savestates, the option in the application);
step 3 is the longest and part of what it settles is analogue (the floating AD bus of A5). So 4
first.

## Step 4 — alignments 0-1, savestates, option (2026-11-15)

* Alignments 0 and 1 against the netlist (`PAIRS="0:2 1:0" fidjudge.sh`, the judge now requires a
  zero offset in **every** paired frame): `g2006px` 0 dots (standard 740), `g2005px` 2 as the
  standard. `g2001px`: both modes differ widely; the netlist starts no `v` increment after a
  mid-line switch on in these two alignments, the switch-on race of step 33 (two signals in one
  half step of a zero-delay simulation): not a reference, not followed.
* Savestates: `Ppu2C02::save_edge_state` / `load_edge_state` (count, each edge's dots, kind and
  byte, the data latch). Run-ahead's in-memory state always carries the block. A savestate file
  takes version 12 (13 with the widescreen extension) **only when edges are pending** when it is
  written; every other state keeps version 10/11, which earlier builds read. A Fidélité+ state
  loads in the standard core too (its pending edges still run). Test
  `fidplus_state_tests.rs`: a program writing `$2007` in a loop, states taken with edges pending,
  file and in-memory restores equal to the live machine.
* Option `emu.fidelity_plus` (Emulation tab, desktop and handheld, French and English texts; off,
  takes effect at the next load).

## The standard path's cost, and how it was found (2026-11-15)

Callgrind, clean builds, PR-1 set against 6c9cb163 (before FIDPLUS-A):

| Commit | Worst | Cause |
|---|---|---|
| 7c7d3c52 (step 1) | +0.001 % | — |
| bcfd363e (step 4) | +2.31 % (`metroid`), +1.95 % `mm2`, +1.79 % `smb1` | `step_dot` outlined from `run_dots`: the Fidélité+ loop gave it more callers |
| c5836c29 | +0.995 % `duelito`, +0.61 % `mmc5`, +0.57 % `smb3` (most ROMs -0.2 to -0.7 %) | `step_dot` pinned; `process_dot` and `run_visible_span_through_bus` then outlined (`smb3`) |
| 0b5dbf58 | worse (+3.35 % `duelito`) | `process_dot` and both span functions pinned; the Fidélité+ paths call `step_dot_out_of_line` |
| d66853a5 | +0.344 % `metroid`, +0.29 % `mm2`, +0.28 % `z2` (`smb3`, `mmc5`, `m4` ±0.002 %) | `Fid<K>` (a compile-time copy of the bus and PPU code for Fidélité+): no outlining left; the rest on `render.rs` line 159, a file identical to the base: the PPU's new fields moved the standard layout |
| 2f49ab4f | +0.001 % (`metroid`, `duelito`, `oamstr`, `vblnmi`, `z2`); the 13 ROMs within ±0.002 % | the Fidélité+ fields boxed apart (`edges::FidState`): the PPU's standard layout is the base's again |

Lesson: a field added to a hot struct changes its layout, and the codegen of a loop that never
reads it (here the background shifters). State only a separate path uses goes behind a `Box`.

Found with `callgrind_annotate` per function, base against new: the regressed functions are
always a callee appearing with a body of its own.

## Step 3a — the address latch as state (2026-11-15)

* In the Fidélité+ loop, after an odd fetching dot's work the octal latch takes the address's
  low byte and the fetch is remembered; after the next dot's work the read is checked against the
  address of that moment (the PPU's high lines now, the latch's low byte): when `v` moved between
  the two halves, or the latch took another byte, the byte read is the one at that address.
* What it replaces: the `$2006` "hybrid address" patch (`AccuracyCoin`, `Hybrid Addresses`) and
  the `ALE + Read` refetch edge: a `$2007` read latching on a cadence ALE half puts its byte in the
  latch, and the next read takes it. Both now follow from the bus.
* AccuracyCoin 147/147 in the four alignments, corpus unchanged (`g2006px`, `g2001px`, `g3e01px`
  0 dots; `g2006x` 2; `g2005px`, `g2000` 2; `oam` 1).
* A7 (the two-phase coarse-Y wrap) in Fidélité+ too: coarse Y 30 with the nametable bit flipped
  for one dot, cleared by an edge after the next dot.

## Next: step 3

The PPU DATA read and write state machines as state, on PPU clock halves (AccuracyCoin's
description, checked on the netlist), the AD bus and the address latch: `$2007 Stress Test` in
alignments 0 and 1, the tile latch of A5, the stale fetch, A7's two-phase coarse-Y wrap.

## Final validation (2026-11-15, head 94eaa221)

* Standard: tests 1677 (zax-nes, harness, settings, NesHawk, ignored included with the ROM sets)
  and 871 (zax-app) passed, 0 failed; campaign 215 pass / 4 fail as before; AccuracyCoin 147/147
  in the four alignments; golden byte-identical to the previous validations (the 5 changes against
  the old base are FIDPLUS-2's known corrections: SMB3 Japan, Zelda II Europe).
* Fidélité+ (`ZAXNES_FIDELITY_PLUS=1`): same campaign, AccuracyCoin 147/147 x4, golden
  byte-identical to the standard core's.
* Speed (`bench-core`, 1200 frames): metroid 0.866 ms/frame standard, 0.803 Fidélité+; smb3 1.184
  and 1.151. The Fidélité+ frames cost nothing measurable.
* Standard core against base 6c9cb163 (callgrind, 13 ROMs): within ±0.002 %.

Lot FIDPLUS-A done; merge and push await the owner.
