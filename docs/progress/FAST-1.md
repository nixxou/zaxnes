# FAST-1 — fast core, progress

Branch `feat/fast-core` (worktree `wt-fastcore`), from tag `accuracy-done` (229aee9d).
Design: `docs/design/FAST-CORE.md`.

## State

| Phase | Status | Commit | Notes |
|---|---|---|---|
| design | done | (this commit) | baseline numbers from callgrind `l2/78069f77` |
| 0 option + skeleton | done | (next commit) | `NesCore::set_fast_core`, `Fast<K>`, `with_fast_board_kind!`, `bench-core --fast`, `ZAXNES_FAST_CORE=1` for golden-core, `rust/tools/fast/agree.py`; 98/98 ROMs identical |
| 2a PPU span by groups of 8 dots (exact) | done | | `ppu/fast.rs`; golden identical; core -2 to -6 % |
| 3a APU silent channels skipped (exact) | done | | `apu/fast.rs`; golden identical; core -13 to -18 % with 2a |
| next | | | profile again: APU audible events, PPU end of line, sprite evaluation, CPU |

## Log

* 2026-11-17: design written; baseline smb3 core 19.8 M Ir/frame (PPU 60 %, APU 21 %).
* 2026-11-17: phase 0 done; golden-core 600 frames, standard vs `ZAXNES_FAST_CORE=1`: byte-identical (98 ROMs).
* 2026-11-17: order changed: the PPU and the APU first (the CPU + bus is the smallest share). Steps 2a and 3a exact (golden-core identical, 98 ROMs). bench-core 2D: smb1 0.644 -> 0.561 ms, mm2 0.719 -> 0.588, metroid 0.768 -> 0.647.
* 2026-11-17: step 2b (sprite evaluation per sprite, table colours): smb1 0.635 -> 0.470, mm2 0.699 -> 0.497, metroid 0.750 -> 0.520 ms/frame; golden identical.
* 2026-11-17: step 2c (sprite fetches and prefetch without the per-dot dispatch): smb1 0.649 -> 0.407, mm2 0.712 -> 0.444, metroid 0.763 -> 0.463; golden in play identical (1800 frames, input script).
* 2026-11-17: step 2d (runs of off-line sprites in one loop): smb1 0.396, mm2 0.425, metroid 0.440 ms/frame; identical. One golden-core segfault (fast run, 77th ROM) did not reproduce in three runs nor ROM by ROM: watch it.
* 2026-11-17: step 1a (FastBus, lazy tick, exact): smb1 0.640 -> 0.338, mm2 0.725 -> 0.355, metroid 0.770 -> 0.395 ms/frame (about 2x the standard core); golden in play identical.
* 2026-11-17: step 4a (MMC3 on the fast path, its A12 events replayed at their clocks, exact): smb3 0.938 -> 0.517 ms/frame; 98/98 identical in play.
* 2026-11-17: step 1b (idle loops JMP-self and LDA ram/branch repeated by counting, exact): smb1 0.265, mm2 0.291, smb3 0.472 ms/frame; metroid unchanged (other loop shape).
* 2026-11-17: same-loop comparison (`fceux-bench`, attract mode, 6000 frames, capture on, built with
  `--features fceux-core` in `target-fceux`; `ZAXNES_FAST_CORE=1` selects the fast core there).
  ms/frame, FCEUX / standard / fast: smb1 0.226 / 0.916 / 0.312, smb3 0.211 / 1.071 / 0.380,
  mm2 0.232 / 0.733 / 0.290, metroid 0.231 / 0.733 / 0.369. Gap to FCEUX 1.25-1.8x (was 3.2-5.1x),
  still exact. Also fixed: `movie_sync.rs` did not build with `fceux-core` since PalM/PalN.
* 2026-11-17: step 2e (capture's pattern planes in one CHR view read): smb1 0.236, mm2 0.252, metroid 0.343, smb3 0.431 ms/frame (bench-core 2D); identical.
* 2026-11-17: step 2f (groups decode one tile, 64-bit window, plane pair read): smb1 0.221, mm2 0.253, metroid 0.335, smb3 0.411 ms/frame; identical.
* 2026-11-17: widescreen check (golden-core ZAXNES_GOLDEN_WIDE=64, ADVANCED=1, play script): fast == standard on 98/98 (the extension is active: 97/98 tile hashes differ from the narrow run). check.sh WIDE=1 runs it.
* 2026-11-17: hidden frames (run-ahead, NesCore::set_frame_hidden for hosts) leave pixels unpainted on the fast path, sprite 0 still computed; golden with ZAXNES_GOLDEN_RUN_AHEAD=2 identical (98/98). Gain about 4 % with run-ahead 1. The app does not call set_frame_hidden for its catch-up frames yet.
* 2026-11-17: sprite 0 polls (`LDA $2002 / AND #$40 / BEQ|BNE`, `BIT $2002 / BVC|BVS`): the turns
  whose read finds the PPU before the first dot sprite 0 can cover are counted, not run (exact,
  98/98 identical, widescreen too). Loop detection now looks 3 instructions back.
  fceux-bench fast core: zelda2 0.622 -> 0.371, smb1 0.278 -> 0.266 ms/frame.

* 2026-11-17: `emu.fast_core` in the app (desktop and handheld option rows, EN/FR), off by default.
* 2026-11-17: `tests/fast_core_tests.rs` (ignored, needs ROMs): 320 ROMs on fast boards, 600 frames,
  fast == standard (cycles, picture, wide picture, tiles, sound, state); savestates cross both
  ways (compared with a reload on the same core: the resampler and lead I5 aside); widescreen
  64/16 with the advanced patches and run-ahead 2 identical.
* 2026-11-17: the line's last seven span dots as a short group: callgrind smb1 -0.2 %, mm2 -0.4 %,
  smb3 -0.3 %; golden 98/98, widescreen 98/98. Tried and dropped: MMC3 span ends through the
  peek view with recorded A12 changes (+2.3 % then +0.3 % on smb3: no better than the bus), and
  packed A12 entries to shrink their clearing (smb3 -0.3 % but nrom/mmc1 +0.8 %, inlining).

## Where it stands (end of 2026-11-17)

fceux-bench, attract mode, 6000 frames, capture on, ms/frame (FCEUX / standard / fast):

| Game | FCEUX | standard | fast | fast vs FCEUX |
|---|---|---|---|---|
| smb1 | 0.218 | 0.913 | 0.266 | 1.22x |
| smb3 | 0.201 | 1.072 | 0.348 | 1.73x |
| mm2 | 0.222 | 0.745 | 0.264 | 1.19x |
| metroid | 0.219 | 0.746 | 0.350 | 1.60x |
| zelda2 | 0.238 | 0.853 | 0.371 | 1.56x |

Every step so far is exact: golden-core in play (98 ROMs, 1800 frames, checkpoints every 30),
with a 64-pixel widescreen margin and the advanced patches, and with run-ahead 2, all identical to
the standard core. zax-nes tests 1526 passed, zax-harness 106 passed.

Left, by expected gain: MMC3 lines cut by IRQ deadlines (per-dot heads/tails through the bus);
the per-instruction CPU cost; the app's catch-up frames calling `set_frame_hidden`; approximate
options (line-level timing) if the owner wants to pass FCEUX; the options UI (`emu.fast_core`
setting, greyed Fidelite+ rows); handheld measurement.
