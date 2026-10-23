# Rust port progress
| Task | Model | Status | Tests | Goldens | Notes |
|---|---|---|---|---|---|
| T00 | sonnet | done | 3/3 | readers load 291 rows, render.json, lua-api | MSRV 1.85; 10 skeleton crates |
| T01 | opus | done | 64/64 | nestest 8991/8991 lines | PPU/APU/input/cart stubs; deferred: 4 PPU memory tests (T02), battery RAM (T04/T05), CPU/bus save state (T05) |
