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
