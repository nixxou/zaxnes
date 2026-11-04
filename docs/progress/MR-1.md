# MR-1 — adversarial review of our mapper boards against Mesen 2

Role R3 (clean-side reviewer). Branch `mapper-review`, worktree plays `impl/`.
**No emulation behaviour changes.** The only code added is `#[ignore]`d tests that
document a finding, plus the report `docs/review/MR-1-mappers.md`.

Reference: `third_party/mesen2/` (GPL v3, commit `b9fa69d`), `Core/NES/Mappers/...`.
Spec prevails over Mesen: a difference where we follow a REQ becomes a Q ticket
(next free number is Q-110).

## Checklist (n = 10)

- [x] 1/10 done `42c158ec` — Inventory: our board -> Mesen file mapping table; shared
      infrastructure read (`registry.rs`, `board.rs`, `banked.rs`, `mappers/mod.rs`,
      `a12_watcher.rs`, `cartridges/mod.rs`, header/submapper handling, `BaseMapper`)
- [x] 2/10 done `42c158ec` — Discrete boards, part 1: 0, 2, 3, 7, 11, 13, 66, 71
- [x] 3/10 done `5c9eaca4` — Discrete boards, part 2: 15, 28, 30, 34, 41, 87/101, 156, 225, 228
- [x] 4/10 done `b8eebc27` — MMC1 (mapper 1), incl. SNROM/SUROM/SXROM and submappers
- [x] 5/10 done `b8eebc27` — MMC2 / MMC4 (mappers 9, 10)
- [x] 6/10 done — MMC3 + A12 watcher + RAMBO-1 (mappers 4, 64), MMC6
- [ ] 7/10 in progress — MMC5 (mapper 5) and its expansion audio + mix
- [ ] 8/10 todo — VRC2/VRC4 (21/22/23/25/27), Bandai FCG (16),
      Jaleco SS88006 (18), Irem G-101 (32)
- [ ] 9/10 todo — Shared infrastructure the next lots need; summary table; report pass
- [ ] 10/10 todo — `cargo fmt --check`, `clippy -D warnings`,
      `cargo test --release -p zax-nes`; journal; close

## Notes

- `CARGO_TARGET_DIR=<worktree>/rust/target-mr` for every cargo invocation,
  foreground, one at a time (CLAUDE.md headless rule).
- Findings are numbered `MR-<board>-<n>` where `<board>` is the iNES mapper number,
  e.g. `MR-4-1`. Shared-infrastructure findings use `MR-A12-<n>` / `MR-HDR-<n>`.
- Ignored tests carry `#[ignore = "MR-1: MR-<board>-<n>"]` so the suite stays green.
- The step-1 mapping table lives in the report, section "Board inventory".

### Step notes

- **Step 1** — inventory in the report. `third_party/` lives at
  `cleanroom/third_party/`, not inside the worktree. Next free Q number confirmed: Q-110.
  REQ-CORE-100 makes *nesdev-documented* behaviour normative and pins some details itself
  (mapper 13 CHR-RAM 16 KiB, mapper 4 "rev B", mapper 16 EEPROM = P3, mapper 30 CHR-RAM
  32 KiB); REQ-CORE-114 lets `mmc3_test_2` test 6 ("MMC3 alt", rev A) fail. That decides
  several comparisons before any code is read.
- **Known limitation of the checks**: `cargo clippy --workspace --all-targets` cannot
  build in this worktree — `zax-app/src/demo_game.rs` `include_bytes!`es
  `demo/FlappyZaxNes.nes` and `demo/flappyzaxnes.3dn`, which `.gitignore:71` (`*.nes`)
  keeps out of the repository. Pre-existing, unrelated to MR-1, and not fixable without
  adding a ROM. Clippy is therefore run as `-p zax-nes --all-targets -D warnings`, which
  covers every file this lot touches.
- **Step 2** — 8 boards. Findings: MR-BUS-1 (bus conflicts absent, shared by mappers
  2/3/7/11/30, our bug), MR-11-1 (Color Dreams PRG field is 4 bits not 2, our bug),
  MR-13-1 (CPROM CHR-RAM 16 KiB not 32, our bug, spec-pinned), MR-13-2 (CPROM forced
  vertical mirroring, unknown), MR-71-1 (mirroring bit polarity inverted vs Mesen,
  unknown — deliberately not called a bug, see the report), MR-71-2 (decode range,
  equivalent). Mappers 0, 3, 7, 66 otherwise equivalent.
  4 ignored tests in `tests/mr1_review_tests.rs`, all confirmed failing for the stated
  reason; 1 non-ignored guard test. No `mapper_3` protect finding: Mesen's CNROM-with-diodes
  is mapper 185, out of scope.
- **Step 3** — 9 boards. The decisive discovery: amendment **AMD-021** (points R2-001,
  R2-002, R2-003) specifies mappers 28, 15 and 101 normatively, and our code follows it to
  the letter while **Mesen contradicts it** on seven observable points. Those are therefore
  spec questions, not bugs: filed as `spec/changes/Q-110.md`. Three "our bug" findings I
  had drafted for mappers 15 and 28 were withdrawn on reading the amendment — check
  `spec/changes/` for the board *before* classifying anything.
  Real findings: MR-34-1 (BNROM decodes $7FFD-$7FFF as registers where it is PRG-RAM;
  visible/crash), MR-41-1/2/3 (Caltron 6-in-1 has the `..MC CPPP` field layout shifted by
  one: PRG 2 bits instead of 3, mirroring read from a PRG bit, inner CHR gate missing —
  games 5 and 6 of the cartridge cannot start), MR-30-1..6 (UNROM 512: byte-6 $09
  four-screen collapsed, work RAM that should not exist, submapper 3/4, flash, four-screen
  CHR-RAM window). Mappers 87, 225, 228, 156 equivalent — one line each.
  6 new ignored tests (10 total), all confirmed failing for the stated reason.
- **Steps 4 and 5** (one commit) — MMC1, MMC2/MMC4. MMC1: 4 findings. MR-1-1 (the reset
  bit must bypass the two-cycle write filter; Mesen cites Shinsenden), MR-1-2 (the 512 KiB
  outer PRG bit comes from the *last-written* CHR register, and the same value feeds the
  SXROM PRG-RAM bank), MR-1-3 (SOROM's 16 KiB PRG-RAM selector is CHR bit 3, we use bits
  2-3 — **no test**: `MapperRom` has no PRG-RAM-size knob, which a fix must add first),
  MR-1-4 (submapper 5 fixed-PRG SEROM/SHROM not handled). 3 new ignored tests.
  MMC2/MMC4: **equivalent**, and worth the words it took to show it — the asymmetric MMC2
  latch ranges match, and Mesen's deferred `_needChrUpdate` is the same observable
  behaviour as our immediate update, not a timing difference. One unknown left: Mesen
  flips the latch on a bare $2006 write, we need a real fetch.
- **Step 6** — MMC3 + MMC6, the A12 watcher, RAMBO-1. MMC3 core is equivalent (rev B is
  right and spec-pinned; the register 0/1 bit-0 masking is equivalent, not a bug).
  6 MMC3 findings: MR-4-1 (**MMC6 $A001 read/write bits swapped** — StarTropics save
  corruption, the worst of the step), MR-4-2 (A12 must fall before the first rise counts;
  one spurious counter clock per power-on/reset), MR-4-3 (submapper 3 is MC-ACC, not
  MMC3), MR-4-4/MR-4-5 (power-on register and $A001 values, unknown), MR-4-6 (MMC6 RAM is
  1 KiB).
  A12 watcher: Mesen has **three** different thresholds (shared class >= 11 dots, MMC3
  >= 3 CPU cycles, RAMBO-1 >= 31 dots) and ours is one constant of 10. The MMC3 spread is
  unknown/approximation; the RAMBO-1 gap is a real bug (MR-64-3). Note recorded that
  `MIN_RISE_INTERVAL` feeds `Mmc3::ppu_irq_horizon`, so the two must change together.
  RAMBO-1: 5 findings. MR-64-1 (**not the MMC3 counter** — always decrements, reload is
  latch+2/+1; every IRQ one scanline early), MR-64-2 (2 KiB CHR keeps bit 0 — Mesen is
  deliberately unlike its own MMC3 here), MR-64-3 (filter), MR-64-4 (IRQ delay 2/1 cycles),
  MR-64-5 (`_forceClock` on leaving cycle mode, Skull & Crossbones). One point where Mesen
  looks wrong and we are right: its $8000 does not re-map.
  5 new ignored tests (18 total), each confirmed failing for the stated reason.
