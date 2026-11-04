# MR-1b — second, sourced pass over every MR-1 finding

Role R3 (clean-side reviewer). Branch `mapper-review`, worktree plays `impl/`.
**No emulation behaviour changes.** The only code touched is `#[ignore]`d tests.

Lot MR-1 (`docs/review/MR-1-mappers.md`, `docs/progress/MR-1.md`) compared our boards with
Mesen 2 **without web access**, so it could not settle contradictions. MR-1b re-examines
every one of its 50 findings against the public sources: the NESdev wiki mapper and chip
pages, the NESdev forums, test-ROM documentation, and NesCartDB for which cartridges use a
board or submapper.

Rules of this lot:

- a finding moves *into* `our bug` only with a cited source or a test-ROM fact, and leaves
  `our bug` the same way;
- `unknown` is kept only when the public sources really do not settle it, and the entry says
  what would settle it;
- where the sources contradict the **spec**, a `Q-NNN` ticket, not a bug (next free number
  was Q-112 at the start of the lot);
- every `our bug` verdict carries a **performance cost class** (`none` / `negligible` /
  `measurable` / `risk`) derived by reading the code path, plus how the fix can be confined
  so unaffected boards pay nothing. No benchmarks here — R0 measures the chosen fixes.

Finding ids continue MR-1's numbering (`MR-<board>-<n>`).

## Checklist (n = 12)

- [x] 1/12 — Setup: progress file, re-read MR-1's report, Q-110/Q-111, the test file and the
      fixture; source-gathering plan per board
- [x] 2/12 — Shared: MR-BUS-1, bank-window wrapping, MR-PWR-1, MR-A12-1
- [ ] 3/12 — MMC1 (MR-1-1, MR-1-2, MR-1-3, MR-1-4) and the three non-findings
- [ ] 4/12 — MMC3 / MMC6 (MR-4-1 .. MR-4-6)
- [ ] 5/12 — RAMBO-1 (MR-64-1 .. MR-64-6)
- [ ] 6/12 — MMC5 (MR-5-1 .. MR-5-5) and the audio points of Q-111
- [ ] 7/12 — VRC2 / VRC4 (MR-21-1, MR-21-2, MR-21-3)
- [ ] 8/12 — Discrete boards: 11, 13, 71, 30 (MR-11-1, MR-13-1/2, MR-71-1/2, MR-30-1 .. 6)
- [ ] 9/12 — Multicarts: 34, 41, and the Q-110 boards 15 / 28 / 101; 87, 156, 225, 228
- [ ] 10/12 — 16, 18, 32, MMC2/MMC4, and the boards MR-1 called equivalent outright
- [ ] 11/12 — Report section "MR-1b — sourced re-check", the new authoritative summary
      table, the test changes, any Q ticket
- [ ] 12/12 — `cargo fmt --check`, `clippy -p zax-nes --all-targets -D warnings`,
      `cargo test --release -p zax-nes`; journal; close

## Notes

- `CARGO_TARGET_DIR=<worktree>/rust/target-mr` for every cargo invocation, foreground, one at
  a time (CLAUDE.md headless rule).
- MR-1's known limitation still holds: `cargo clippy --workspace --all-targets` cannot build
  here because `zax-app/src/demo_game.rs` `include_bytes!`es a gitignored `.nes`. Clippy is
  therefore scoped to `-p zax-nes --all-targets`, as MR-1 did.
- Tests added by this lot carry `#[ignore = "MR-1b: MR-<board>-<n>"]`; tests MR-1 added that
  this lot withdraws are removed or re-labelled, with the reason in the report.
- Never stage `rust/crates/zax-render/tests/*_tmp.rs`, a root `tests/`, `.claude/`,
  `CLAUDE.md`, or `rust/target-mr/`.

### Step notes

- **Step 1** — the ground rules of the lot, settled before any source is read.
  - MR-1's summary table has **50 entries**: 37 `our bug`, 8 `unknown`, 2 `Mesen in error`,
    1 `equivalent`, 2 spec questions (`Q-110` covering 7 points on mappers 15/28/101,
    `Q-111` covering 2 MMC5 audio points). The narrative also calls 7 boards equivalent
    outright (0, 66, 87, 156, 225, 228, MMC2/MMC4) plus the shared bank-window wrapping;
    those are re-checked too, since the lot asks for `equivalent` as well.
  - Tests: `rust/crates/zax-nes/tests/mr1_review_tests.rs`, 26 tests, 25 ignored. The
    fixture is `tests/common/mapper_fixture.rs` — `MapperRom::new(mapper).prg(n).chr(n)
    .mirroring(bits).battery().submapper(s).load()`, every 8 KiB PRG bank and 1 KiB CHR bank
    filled with its own number, plus `prg_windows()`, `chr_windows()`, `mapper_write`,
    `mmc1_write`, `mmc3_bank`. **No PRG-RAM-size knob** (why MR-1-3 has no test) and no raw
    byte-6/byte-7 knob beyond `mirroring(bits)`.
  - Hot-path facts for the cost classes, read from `PERFORMANCE-HISTORY.md` §5.1 and
    §10, not guessed:
    - **a cartridge write is already a PPU sync point** (§5.1 "PPU caught up in batches"), so
      work added on a register write costs a batched PPU catch-up that already happens;
    - `Cartridge::cpu_write` (`mod.rs:390-410`) is the single funnel for every CPU write in
      $4020-$FFFF, and it already calls `refresh_rom_windows()` after each one;
    - `read_rom_window` (`mod.rs:343-353`) is `#[inline]` and already indexes
      `rom_windows[address >> 13]`, so the ROM byte MR-BUS-1 needs is one load away;
    - `Mapper::chr_view` purity is an invariant: a board that gains a side effect on a CHR
      fetch loses the fast span and moves to `run_visible_span_through_bus`. MMC2/MMC4,
      MMC3 and compatibles, RAMBO-1 and MMC5 are already on the through-bus path, so a
      change inside their `ppu_read` costs nothing new; a *new* board gaining one would be
      the expensive case;
    - `A12Watcher::MIN_RISE_INTERVAL` feeds `Mmc3::ppu_irq_horizon` (`mmc3.rs:167`), the
      bound on how far the PPU may lag; per-board thresholds must move both together;
    - expansion audio forces `apu_quiet = 0` (only the MMC5), so MMC5 audio changes are
      already on a per-cycle path;
    - `NesBus::tick`'s prologue is pinned in the post-LTO assembly (§10): nothing
      board-specific may be inlined into it.

- **Step 2** — the shared findings. MR-BUS-1 **confirmed and strengthened**: the NES 2.0
  submapper page states the result is "the bitwise AND of the written value and the value in
  ROM", so the AND is normative and the submapper is the gate; mapper 11's page adds that
  every commercial Color Dreams release is submapper 2, and UNROM 512's that submapper 0
  without the battery bit means conflicts (Mesen's odd condition, now sourced). Cost
  negligible: a `bus_conflicts` bool resolved at load, one branch in `cpu_write`, which is
  already a PPU sync point.
  Two `unknown`s **closed as equivalent**: the A12 filter is specified in **M2 falling edges**
  (three on the MMC3, sixteen on the RAMBO-1) and our 10-dot constant brackets the MMC3
  figure; and the power-on bank values are documented as *unspecified*, so MR-PWR-1 and
  MR-4-4 have nothing to discover.
  Bank-window wrapping stays equivalent.
