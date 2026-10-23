# Rust port progress
| Task | Model | Status | Tests | Goldens | Notes |
|---|---|---|---|---|---|
| T00 | sonnet | done | 3/3 | readers load 291 rows, render.json, lua-api | MSRV 1.85; 10 skeleton crates |
| T01 | opus | done | 64/64 | nestest 8991/8991 lines | PPU/APU/input/cart stubs; deferred: 4 PPU memory tests (T02), battery RAM (T04/T05), CPU/bus save state (T05) |
| T02 | opus | done | 151/151 (+1 ignored full run) | mapper-0 pixel+tiles 181/222; 41 pending T03 (APU-dependent, listed in `PENDING_T03`) | `Machine` frame runner for T05; full run: `cargo test -p zax-nes --test ppu_golden_tests -- --ignored` (~60 s) |
| T03 | opus | done | 209/209 (+5 ignored: full golden run, MMC1 `cpu_interrupts.nes`, 3 pending-by-design ROM notes) | mapper-0 audio+cycles+pixel+tiles+blargg 221/222 at 1200 frames (bit-exact audio); 1 pending T04 (`dma_4016_read`: input stub, 1-cycle drift) in `PENDING_T04` | `ApuStateTests` deferred to T05 (no save state in APU yet); `AudioReader` handle for the host audio thread; audio setters return `Result<_, AudioError>` |
