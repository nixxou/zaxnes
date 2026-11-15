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

## Next: step 2b

`$2007` read (latch at the edge, the merge rules to fall out of the order), `$2001` (switch and the
dot-257 copy), `$2000`, `$2005`; the `$2006` write on dot 304.
