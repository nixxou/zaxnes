# MF-T — the public test-ROM campaign

Lot MF-T: a `zax-harness test-roms` command that runs the public NES test suites
(`nes-test-roms`, `holy-mapperel`, `AccuracyCoin`) headless and reports one result per ROM,
plus the Markdown report `docs/review/MF-T-test-roms.md` and a `cargo test` subset gated on
`ZAXNES_TEST_ROMS`.

No change to emulation behaviour or performance: every reader goes through the core's
existing public accessors (`NesCore::read_memory`, `ppu().vram()`, `map_nametable`,
`pixel_frame_with_mask`, `run_frame`, `soft_reset`). **No new core accessor was needed.**

ROM directory (not in git): `assets-free/test-roms/`. Outputs stay under `rust/target-mf/`.

## Steps

- [x] 1/10 — `test-roms` subcommand skeleton: ROM discovery, header/mapper triage, frame
  budget loop, blargg `$6000` reader, JSON rows, final-frame PNGs.
- [ ] 2/10 — screen readers: nametable → text through the ROM's own CHR font, and the
  documented pass/fail screens of the non-blargg suites.
- [ ] 3/10 — Holy Mapperel reader: detected board and the four detailed nibbles.
- [ ] 4/10 — AccuracyCoin reader: Start at the menu top, then the `$0400-$0498` result
  table and the pass tally.
- [ ] 5/10 — `--markdown` report generator, with the reference column taken from
  `nes-test-roms/status.txt`.
- [ ] 6/10 — reader debugging on filtered subsets (does not count as a campaign run).
- [ ] 7/10 — campaign run 1/2 over the whole directory; reader fixes.
- [ ] 8/10 — campaign run 2/2 and the committed report `docs/review/MF-T-test-roms.md`.
- [ ] 9/10 — the gated `cargo test` subset (skipped without `ZAXNES_TEST_ROMS`).
- [ ] 10/10 — journal entry and close.

## Notes

### Result protocols used

- **blargg** (`instr_test-v5`, `mmc3_test`, `apu_test`, `ppu_vbl_nmi`, …), from each suite's
  `readme.txt`: signature `DE B0 61` at `$6001-$6003`; `$6000` is `$80` while running, `$81`
  when the ROM wants the reset button pressed at least 100 ms later, any other value is the
  result code (0 = pass); zero-terminated text from `$6004`. The harness waits for the
  signature *and* for `$80` to have been seen before it believes a code, so the power-on
  contents of work RAM are never mistaken for a result.
- **Holy Mapperel** (`holy-mapperel/README.md`, "Displayed result"): the ROM has no memory
  protocol, it draws `cur_mapper`, the sizes and the four detailed nibbles on screen. The
  harness reads the nametable and decodes the glyphs through the font in the ROM's own CHR,
  so the detected board and the `WRAM/PRG/IRQ/CHR` code come out as text.
- **AccuracyCoin** (`AccuracyCoin/README.md` + the shipped `AccuracyCoin.asm`): the cursor
  starts at the top of page 1 (`menuCursorYPos = $FF`), so pressing Start runs
  `AutomaticallyRunEveryTestInROM`. Each test writes its byte to its own address in
  `$0400-$0498`: `$FF` = skipped, bit 0 set = pass, otherwise the error code is the byte
  shifted right twice. `$0038` is the pass tally and `$0037` the total once the result table
  is drawn.
- **screen-only ROMs**: when the suite's documentation names the text that means pass, the
  harness reads it from the nametable; otherwise the row is `manual` and the PNG is the
  evidence.

### Reference column

`assets-free/test-roms/nes-test-roms/status.txt` is the collection's own result table
(NESICIDE, Nestopia 1.40, Nintendulator 0.975). It ships with the ROMs, so the report quotes
it with its source URL and invents nothing. No published Mesen 2 table covers these ROMs
row by row, so there is no Mesen 2 column.

### Local environment

`rust/test-roms/` is git-ignored and empty in this worktree, so the four `bench_core_tests`
fail on a missing `nestest.nes` before anything of this lot is touched. The ROM is public and
sits in `assets-free/test-roms/nes-test-roms/other/nestest.nes`; copying it into the ignored
directory makes `cargo test --release -p zax-harness -p zax-nes` green (53 binaries, 0
failures) and stages nothing.

### Reader bugs found while writing the readers

* **blargg `$81`, step 1.** A soft reset keeps the work RAM, so `$6000` still holds the `$81`
  that asked for it when the ROM resumes. A reader that only clears its own "a reset is due"
  flag fires again seven frames later, for ever: `cpu_reset/ram_after_reset` timed out after 96
  resets and `cpu_reset/registers` reported a bogus `Failed #3`. The reader now also forgets
  that `$80` was seen, so it waits for the ROM to re-arm the protocol. Both ROMs pass with one
  reset each. **This was a harness bug, not an emulation one** — nothing in the core changed.

### Campaign runs

- Run 1/2 (step 7): debug pass over the whole directory.
- Run 2/2 (step 8): the run the committed report comes from.
