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

## Next: step 2

PPU register edges: in Fidélité+ the PPU's register handlers stop prepaying dots
(`prepaid_dots`, the A3/A5 delay constants) and post their effects as edge events at the half dot
the netlist gives; the dot loop resolves them in order. Judge: A5 corpus, alignments 2-3, with the
A5 rules removed from the Fidélité+ path.
