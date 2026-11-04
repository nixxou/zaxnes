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
- [x] 2/10 — screen readers: nametable → text through the ROM's own CHR font, and the
  documented pass/fail screens of the non-blargg suites.
- [x] 3/10 — Holy Mapperel reader: detected board and the four detailed nibbles.
- [x] 4/10 — AccuracyCoin reader: Start at the menu top, then the `$0400-$0498` result
  table and the pass tally.
- [x] 5/10 — `--markdown` report generator, with the reference column taken from
  `nes-test-roms/status.txt`.
- [x] 6/10 — reader debugging on filtered subsets (does not count as a campaign run).
- [x] 7/10 — campaign run 1/2 over the whole directory; reader fixes.
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

### What step 2 found out about the collection

* **Half of `nes-test-roms/` predates the `$6000` protocol.** `sprite_hit_tests_2005.10.05`,
  `sprite_overflow_tests` and `blargg_ppu_tests_2005.09.15b` never write `$80`, and nothing in
  a file says which era a ROM belongs to. The reader therefore probes for 600 frames and falls
  back to the screen, instead of calling 21 ROMs `timeout` and burning two minutes of budget on
  each. All 21 now read: 20 `pass`, one `fail`.
* **Code 0 is not always a pass.** `tests_done` in blargg's `common/testing.s` prints "Done"
  and exits with 0 when the ROM never called `set_test`, and the readme says exactly those
  ROMs "require that you watch/listen". Those rows are `manual`, not `pass`.
* **The newer builds colour their text with ANSI escapes.** `cpu_dummy_writes` writes
  `ESC [ 0 ; 3 7 m` before every line; a CSI sequence is now dropped whole.
* **Nametable choice has to follow PPUCTRL, not character count.**
  `blargg_ppu_tests_2005.09.15b/vram_access` draws `$01` on the page it shows and leaves the
  debris of its own VRAM test on the other physical page; picking the page with the most text
  read the debris (`R  R               4`) instead of the result.
* **Two fonts, both read off the ROMs' own CHR.** Holy Mapperel masks ASCII to six bits (tile
  `$01` is `A`, `$0D` is `M`, `$30` is `0`, `$3F` is `?`); the pre-`$6000` blargg builds put
  each glyph at its ASCII code. The reader decodes with both and keeps the screen with more
  letters and digits, and records which font won in the row.
* Screen bytes come from `Ppu2C02::read_nametable_byte`, not from `vram()` directly, so a board
  that serves its own nametables (MMC5 `ExRAM`, fill mode) reads correctly. The accessor is
  documented as side-effect free and is the one the tile capture already uses.
* `blargg_ppu_tests_2005.09.15b/power_up_palette` reports `$02`, "Palette differs from table".
  Its readme says the reference values "are probably unique to my NES", so the report must note
  that and not read it as a plain emulation defect.

### Holy Mapperel, step 3

All 40 boards in 6.5 s: **32 pass, 2 fail, 6 unsupported**. Every board the ROM detects
matches the header, and the sizes it measures match the file.

* The six `unsupported` rows are mappers the registry does not list at all — 69 (JLROM/JSROM,
  FME-7), 78.3 (IF-12), 118 (TxSROM) and 180 (UNROM 7408). This is a gap in coverage, not a
  defect, and the report says so.
* The two failures are `M1_P128K.nes` and `M1_P128K_CR8K.nes`, both SNROM, both
  `DETAILED TEST RESULT: 4000`. The README reads digit 4 of the WRAM nibble as "$A000 bit 4
  does not disable WRAM (SNROM)". The other eleven MMC1 boards — SJROM, SKROM, SUROM, SXROM —
  are `0000`, so this is the SNROM-only CHR-register WRAM-disable line and nothing wider.
  A finding for lot MF-2 or a follow-up MR ticket; this lot does not fix emulation.
* `PRG RAM MISSING` is **not** read as a failure: BNROM and the A53 have no work RAM and are
  right to say so. Only the four detailed digits and an explicit `BAD` decide the verdict,
  which is what the README commits to ("Zero is normal; anything else reflects something
  unexpected").

### AccuracyCoin, step 4

**89 of 147 pass, 22 fail, 36 are blocked.** The run takes 6.5 s.

The ROM is fully automatable: the cursor starts at the top of page 1 (`menuCursorYPos = $FF`),
so a single Start runs every test and each test writes one byte between `$0403` and `$0498`.
The 147 test names, their result addresses and their menu coordinates were taken from the
`TestPages:` and `TableTable` tables of the `AccuracyCoin.asm` the ROM ships with. The five
`DRAW` entries share `result_DrawTest` at `$03FF` and are excluded from the ROM's own tally,
which is why the total is 147 and not 152.

**Two different blockages, and the second one is a wall.**

1. `Interrupt flag latency` (page "CPU Interrupts") **never returns**: the ROM's own test
   counter stops advancing and nothing after it runs. Without a remedy this alone costs 66 of
   the 147 results and 96 s of wall-clock.
2. From `PPU Register Mirroring` (page "PPU Behavior") on, **every one of the 36 remaining
   tests takes the run out of the all-test loop** — `RunningAllTests` goes back to zero with
   no result written.

The harness does what the README tells a person to do — "Pressing B will mark the highlighted
test to be skipped. [...] This is useful if any tests are crashing the console or emulator and
you still want to see the table of results" — by power-cycling, walking the menu to the stuck
test, pressing B and running again. That recovers the whole middle of the table past blockage
1. It was verified once, with the skip limit raised to 40, that blockage 2 is **one wall and
not 36 defects**: retiring all 36 one at a time (37 attempts, 46 s) yields 89 pass / 22 fail /
36 blocked, exactly the same counts, and not one of the 36 ever produces a result. The drive
therefore stops after three consecutive non-hanging retirements and reports where the wall
starts, which is the honest reading and costs 6.5 s instead of 46.

Likely single root cause to investigate in a later lot, **not** 36 separate ones: the ROM's
NMI and IRQ vectors point at RAM ("This allows me to create tests that have different NMI/IRQ
routines"), and every blocked page is a PPU/NMI-timing page.

The 22 genuine failures group cleanly: all five `SH*` stores (`SHA`, `SHS`, `SHY`, `SHX`) with
code 7, `Open Bus`, and most of the DMA and APU-timing page.

### Step 6, the filtered probes

Run on subsets, not on the whole directory, so neither of the two campaign runs is spent.

* The suites feared slow are not: `cpu_interrupts_v2` (6/6 pass), `ppu_vbl_nmi` (11/11),
  `instr_timing` (3/3), `cpu_exec_space` (2/2), `oam_stress` (1/1 in 1.1 s),
  `ppu_read_buffer` (1/1 in 1.4 s). Every one stops the instant `$6000` leaves `$80`, so the
  long budgets are only there for a hang.
* **The `PASSED`/`FAILED` screen reading is not specific to the 2005 suites.** The readmes say
  "Tests generally print information on screen, but also report the final result audibly, and
  output text to memory" — the screen and `$6000` carry the same words. `read_joy3/thorough_test`
  draws `Passed` without ever arming the protocol, and was `manual` while the word reader was
  restricted by path. It is now the default for a screen-read ROM, with the hexadecimal code
  of `blargg_ppu_tests_2005.09.15b` the one exception. A screen holding neither word is still
  `manual`, so nothing is invented.
* `read_joy3/count_errors` and `count_errors_fast` draw `Conflicts: 0/1000` and `Errors:
  0/1000`. Nothing in the collection states the threshold, so they stay `manual` — but it is
  worth noting in the report that `status.txt` records NESICIDE and Nestopia at 72/1000 and
  7/1000 respectively, both marked FAIL, where Zaxnes shows 0.
* `stress/NEStress.NES` and the `other/` directory are interactive demos and menus, correctly
  `manual`.

### Campaign runs

- **Run 1/2 (step 7), done.** 331 ROMs in 4 min 21 s: 185 pass, 17 fail, 0 timeout,
  7 unsupported, 122 manual. It found two harness faults, both fixed before run 2.
- Run 2/2 (step 8): the run the committed report comes from.

### What run 1 found, and the two fixes

1. **The PAL suite was being run on an NTSC console.** `nes-test-roms/pal_apu_tests/` is, in
   its own readme, "PAL NES APU Tests [...] tested on a PAL NES and all give a passing
   result" — and all ten files carry a plain NTSC header, so nothing in the ROM says so. Worse,
   the harness was not applying the region at all: `NesCore` detects it but deliberately does
   not apply it (REQ-CORE-013 leaves that to the host, so a player's choice beats a header),
   and the harness *is* the host here. It now calls `set_region` with the detected region, and
   overrides it to PAL for that one suite, on the authority of the suite's own readme and
   directory name. The region used is recorded in every row.
   The seven failures **survive** the fix but their codes change (`#2 #2 #2 #2 #3 #2 #2` under
   NTSC, `#2 #4 #5 #2 #2 #5 #3` under PAL), which is the proof the timing really changed: they
   are genuine PAL APU frame-sequencer failings, not an artefact. 3 of 10 pass.
2. **The screen fallback was eating the budget.** A ROM that never arms `$6000` fell through
   to the screen reader with the whole two-minute budget, and the animated demos of
   `nes-test-roms/other/` never settle, so each burned all 7 200 frames for a `manual` row.
   The fallback is now capped at 1 800 frames; every suite that reports on screen settles
   inside four to nine seconds, so nothing is lost.

### Findings for later lots (this lot does not fix emulation)

* **MMC3 IRQ reload when the counter has already reached zero** — three ROMs, one behaviour:
  `mmc3_test/6-MMC6` (#3), `mmc3_test_2/rom_singles/6-MMC3_alt` (#2),
  `mmc3_irq_tests/5.MMC3_rev_A` (#3). All say an IRQ fires on a reload that should not fire.
* **OAM DMA against DMC DMA** — `sprdma_and_dmc_dma` and `sprdma_and_dmc_dma_512`, both
  code 1 with the same checksum; `dmc_dma_during_read4/dma_4016_read` fails too.
* **PAL APU frame sequencer** — seven of the ten `pal_apu_tests`.
* **MMC1 SNROM WRAM disable** — the two Holy Mapperel SNROM boards, `4000`.
* **`AccuracyCoin`** — see its section above.
* `blargg_ppu_tests_2005.09.15b/power_up_palette` reports `$02`, "palette differs from table";
  its readme says those reference values "are probably unique to my NES", so it is not a
  defect on its own.
* `nes-test-roms/other/LINUSMUS.NES` has no iNES signature (its sibling is `LINUS.NSF`), so it
  is `unsupported` on format, correctly.
