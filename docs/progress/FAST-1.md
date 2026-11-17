# FAST-1 — fast core, progress

Branch `feat/fast-core` (worktree `wt-fastcore`), from tag `accuracy-done` (229aee9d).
Design: `docs/design/FAST-CORE.md`.

## State

| Phase | Status | Commit | Notes |
|---|---|---|---|
| design | done | (this commit) | baseline numbers from callgrind `l2/78069f77` |
| 0 option + skeleton | done | (next commit) | `NesCore::set_fast_core`, `Fast<K>`, `with_fast_board_kind!`, `bench-core --fast`, `ZAXNES_FAST_CORE=1` for golden-core, `rust/tools/fast/agree.py`; 98/98 ROMs identical |
| 1 CPU (`FastBus`) | next | | |

## Log

* 2026-11-17: design written; baseline smb3 core 19.8 M Ir/frame (PPU 60 %, APU 21 %).
* 2026-11-17: phase 0 done; golden-core 600 frames, standard vs `ZAXNES_FAST_CORE=1`: byte-identical (98 ROMs).
