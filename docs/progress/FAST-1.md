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
