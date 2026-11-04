# MF-T — Zaxnes against the public test ROMs

The ROMs are **not** in the repository: they come from `assets-free/test-roms/`, whose `README.md` records where each suite was downloaded from and at which commit. The JSON and the per-ROM PNGs of the final frame stay under `rust/target-mf/`, out of git.

To reproduce the whole thing:

```
zax-harness test-roms <roms dir> run.json --shots <dir> --markdown <this file>
```

`--filter <substring>` runs one suite, `--merge <other.json>` keeps another campaign's rows and replaces only the ROMs this run touched, and `--reuse` rewrites this file from a JSON already on disk without emulating anything.

Each suite is read the way its own documentation says, never by guessing:

* **blargg's `$6000` protocol** (`nes-test-roms/*/readme.txt`, "Output at $6000"): the `DE B0 61` signature at `$6001-$6003`, `$80` while running, `$81` as a request for the RESET button at least 100 ms later, anything else a result code with 0 for a pass, and the zero-terminated text from `$6004`. A ROM that prints "Done" rather than "Passed" exits with code 0 without having passed — the readme says those have to be watched or heard — so it is `manual`.
* **the screen**, for the suites that predate that protocol. `sprite_hit_tests_2005.10.05` and `sprite_overflow_tests` draw `PASSED` or `FAILED #n`; `blargg_ppu_tests_2005.09.15b` draws a hexadecimal code whose readme says "A result code of 1 always indicates that all tests were passed".
* **Holy Mapperel's result screen** (`holy-mapperel/README.md`, "Displayed result"): the detected board, the memory sizes and the four digits of `DETAILED TEST RESULT`, of which the README says "Zero is normal; anything else reflects something unexpected".
* **`AccuracyCoin`'s result bytes** at `$0403-$0498`, one per test, plus its own pass tally.

Anything none of those can read is `manual`, and the PNG of the final frame is the evidence.

## Totals

| Result | ROMs |
|---|---:|
| pass | 204 |
| fail | 15 (of which 3 are required to fail, see below) |
| timeout | 0 |
| unsupported | 1 |
| manual | 111 |
| **total** | **331** |

**Lot MF-3 moved eight rows, all of them in `holy-mapperel`**, which now passes in full; see
"What lot MF-3 changed" at the end of this document for the before/after. Everything else in this
document is the campaign as MF-T first measured it.

`manual` is mostly `nes-test-roms/other/`, which is a folder of demos and homebrew rather than tests, and the handful of ROMs that ask a person to press buttons or listen. Each still has a PNG of its final frame.

## Per suite

| Suite | passed / total | fail | timeout | unsupported | manual |
|---|---:|---:|---:|---:|---:|
| `AccuracyCoin` | 0 / 1 | 1 | 0 | 0 | 0 |
| `holy-mapperel` | **40 / 40** | 0 | 0 | 0 | 0 |
| `nes-test-roms/240pee` | 0 / 2 | 0 | 0 | 0 | 2 |
| `nes-test-roms/MMC1_A12` | 0 / 1 | 0 | 0 | 0 | 1 |
| `nes-test-roms/PaddleTest3` | 0 / 1 | 0 | 0 | 0 | 1 |
| `nes-test-roms/apu_mixer` | 4 / 4 | 0 | 0 | 0 | 0 |
| `nes-test-roms/apu_reset` | 6 / 6 | 0 | 0 | 0 | 0 |
| `nes-test-roms/apu_test` | 9 / 9 | 0 | 0 | 0 | 0 |
| `nes-test-roms/blargg_apu_2005.07.30` | 11 / 11 | 0 | 0 | 0 | 0 |
| `nes-test-roms/blargg_litewall` | 0 / 5 | 0 | 0 | 0 | 5 |
| `nes-test-roms/blargg_nes_cpu_test5` | 0 / 2 | 0 | 0 | 0 | 2 |
| `nes-test-roms/blargg_ppu_tests_2005.09.15b` | 4 / 5 | 1 | 0 | 0 | 0 |
| `nes-test-roms/branch_timing_tests` | 3 / 3 | 0 | 0 | 0 | 0 |
| `nes-test-roms/cpu_dummy_reads` | 1 / 1 | 0 | 0 | 0 | 0 |
| `nes-test-roms/cpu_dummy_writes` | 2 / 2 | 0 | 0 | 0 | 0 |
| `nes-test-roms/cpu_exec_space` | 2 / 2 | 0 | 0 | 0 | 0 |
| `nes-test-roms/cpu_interrupts_v2` | 6 / 6 | 0 | 0 | 0 | 0 |
| `nes-test-roms/cpu_reset` | 2 / 2 | 0 | 0 | 0 | 0 |
| `nes-test-roms/cpu_timing_test6` | 1 / 1 | 0 | 0 | 0 | 0 |
| `nes-test-roms/dmc_dma_during_read4` | 2 / 5 | 1 | 0 | 0 | 2 |
| `nes-test-roms/dmc_tests` | 0 / 4 | 0 | 0 | 0 | 4 |
| `nes-test-roms/dpcmletterbox` | 0 / 1 | 0 | 0 | 0 | 1 |
| `nes-test-roms/exram` | 0 / 1 | 0 | 0 | 0 | 1 |
| `nes-test-roms/full_palette` | 0 / 3 | 0 | 0 | 0 | 3 |
| `nes-test-roms/instr_misc` | 5 / 5 | 0 | 0 | 0 | 0 |
| `nes-test-roms/instr_test-v3` | 17 / 17 | 0 | 0 | 0 | 0 |
| `nes-test-roms/instr_test-v5` | 18 / 18 | 0 | 0 | 0 | 0 |
| `nes-test-roms/instr_timing` | 3 / 3 | 0 | 0 | 0 | 0 |
| `nes-test-roms/m22chrbankingtest` | 0 / 1 | 0 | 0 | 0 | 1 |
| `nes-test-roms/mmc3_irq_tests` | 5 / 6 | 1 | 0 | 0 | 0 |
| `nes-test-roms/mmc3_test` | 5 / 6 | 1 | 0 | 0 | 0 |
| `nes-test-roms/mmc3_test_2` | 5 / 6 | 1 | 0 | 0 | 0 |
| `nes-test-roms/mmc5test` | 0 / 1 | 0 | 0 | 0 | 1 |
| `nes-test-roms/mmc5test_v2` | 0 / 1 | 0 | 0 | 0 | 1 |
| `nes-test-roms/nes15-1.0.0` | 0 / 2 | 0 | 0 | 0 | 2 |
| `nes-test-roms/nes_instr_test` | 11 / 11 | 0 | 0 | 0 | 0 |
| `nes-test-roms/nmi_sync` | 0 / 2 | 0 | 0 | 0 | 2 |
| `nes-test-roms/nrom368` | 0 / 2 | 0 | 0 | 0 | 2 |
| `nes-test-roms/ny2011` | 0 / 1 | 0 | 0 | 0 | 1 |
| `nes-test-roms/oam_read` | 1 / 1 | 0 | 0 | 0 | 0 |
| `nes-test-roms/oam_stress` | 1 / 1 | 0 | 0 | 0 | 0 |
| `nes-test-roms/other` | 0 / 63 | 0 | 0 | 1 | 62 |
| `nes-test-roms/pal_apu_tests` | 3 / 10 | 7 | 0 | 0 | 0 |
| `nes-test-roms/ppu_open_bus` | 1 / 1 | 0 | 0 | 0 | 0 |
| `nes-test-roms/ppu_read_buffer` | 1 / 1 | 0 | 0 | 0 | 0 |
| `nes-test-roms/ppu_vbl_nmi` | 11 / 11 | 0 | 0 | 0 | 0 |
| `nes-test-roms/read_joy3` | 1 / 4 | 0 | 0 | 0 | 3 |
| `nes-test-roms/scanline` | 0 / 1 | 0 | 0 | 0 | 1 |
| `nes-test-roms/scanline-a1` | 0 / 1 | 0 | 0 | 0 | 1 |
| `nes-test-roms/scrolltest` | 0 / 1 | 0 | 0 | 0 | 1 |
| `nes-test-roms/soundtest` | 0 / 1 | 0 | 0 | 0 | 1 |
| `nes-test-roms/sprdma_and_dmc_dma` | 0 / 2 | 2 | 0 | 0 | 0 |
| `nes-test-roms/sprite_hit_tests_2005.10.05` | 11 / 11 | 0 | 0 | 0 | 0 |
| `nes-test-roms/sprite_overflow_tests` | 5 / 5 | 0 | 0 | 0 | 0 |
| `nes-test-roms/spritecans-2011` | 0 / 1 | 0 | 0 | 0 | 1 |
| `nes-test-roms/stars_se` | 0 / 1 | 0 | 0 | 0 | 1 |
| `nes-test-roms/stomper` | 0 / 1 | 0 | 0 | 0 | 1 |
| `nes-test-roms/stress` | 0 / 1 | 0 | 0 | 0 | 1 |
| `nes-test-roms/tutor` | 0 / 1 | 0 | 0 | 0 | 1 |
| `nes-test-roms/tvpassfail` | 0 / 1 | 0 | 0 | 0 | 1 |
| `nes-test-roms/vaus-test` | 0 / 1 | 0 | 0 | 0 | 1 |
| `nes-test-roms/vbl_nmi_timing` | 7 / 7 | 0 | 0 | 0 | 0 |
| `nes-test-roms/volume_tests` | 0 / 1 | 0 | 0 | 0 | 1 |
| `nes-test-roms/window5` | 0 / 2 | 0 | 0 | 0 | 2 |

## Every failure and timeout, with its subsystem

One line per row that is not a pass. The subsystem is what the ROM exercises, which is where a fix would go; MF-T diagnoses and does not fix. **The two `holy-mapperel` SNROM rows that stood here are gone: MF-3 step 1 fixed them.**

**3 of these 15 are required to fail** and are marked as such below: they are the revision A half of a pair of MMC3 tests that contradict each other, and REQ-CORE-100 pins mapper 4 to revision B. In each pair the revision B ROM passes. Counting them as defects would be wrong.

| ROM | Result | Subsystem and reading | What it says |
|---|---|---|---|
| `AccuracyCoin/AccuracyCoin.nes` | fail | CPU, PPU and APU together — 89 of 147 pass. Read the per-test table in the JSON: the 22 failures are the five `SH*` stores, `Open Bus`, and most of the DMA and APU-timing pages, and a further 36 are blocked rather than failed. | 89/147 tests passed; stopped the run: Interrupt flag latency (page "CPU Interrupts") — never returned; skipped on a later attempt, PPU Register Mirroring (page "PPU Behavior") — took the run out of the all-test loop; skipped on a later attempt, PPU Register Open Bus (page "PPU Behavior") — took the run out of the all-test loop; skipped on a later attempt, PPU Read Buffer (page "PPU Behavior") — to… |
| `nes-test-roms/blargg_ppu_tests_2005.09.15b/power_up_palette.nes` | fail | PPU palette at power-on. Code `$02`, "palette differs from table" — and the suite's readme says of those reference values that they "are probably unique to my NES", so this is not a defect on its own. Nestopia 1.40 fails it too. | screen: $02 |
| `nes-test-roms/dmc_dma_during_read4/dma_4016_read.nes` | fail | DMC DMA stealing a read | screen: 08 08 08 08 08 / FBF7C7B1 / dma_4016_read / Failed |
| `nes-test-roms/mmc3_irq_tests/5.MMC3_rev_A.nes` | fail | MMC3 IRQ, revision A — **required to fail**. The suite's readme: "The last two ROMs test different revisions of the MMC3, so at most only one will pass on a particular emulator", and `6.MMC3_rev_B` passes. REQ-CORE-100 pins mapper 4 to revision B. | screen: MMC3 IRQ COUNTER REVISION A / FAILED #3 |
| `nes-test-roms/mmc3_test/6-MMC6.nes` | fail | MMC3/MMC6 IRQ reload once the counter has already reached zero — the revision A behaviour, the other half of the pair whose `5-MMC3` passes. **Required to fail** under REQ-CORE-100's revision B. | IRQ shouldn't occur when reloading after counter normally reaches 0 / 6-MMC6 / Failed #3 (code 3) |
| `nes-test-roms/mmc3_test_2/rom_singles/6-MMC3_alt.nes` | fail | MMC3 IRQ, revision A — **required to fail**. AMD-061 (answer to Q-114) keeps revision B for every iNES 1.0 mapper-4 image and says REQ-CORE-114, which is this very ROM, stays required. `5-MMC3` passes. | IRQ shouldn't be set when reloading to 0 due to counter naturally reaching 0 previously / 6-MMC3_alt / Failed #2 (code 2) |
| `nes-test-roms/pal_apu_tests/04.clock_jitter.nes` | fail | APU frame sequencer under **PAL** timing. The suite's readme calls it "PAL NES APU Tests [...] tested on a PAL NES", though its files carry NTSC headers, so the harness runs it on a PAL console; 3 of its 10 pass. | screen: APU CLOCK JITTER / FAILED: #2 |
| `nes-test-roms/pal_apu_tests/05.len_timing_mode0.nes` | fail | APU frame sequencer under **PAL** timing. The suite's readme calls it "PAL NES APU Tests [...] tested on a PAL NES", though its files carry NTSC headers, so the harness runs it on a PAL console; 3 of its 10 pass. | screen: APU MODE 0 LENGTH TIMING / FAILED: #4 |
| `nes-test-roms/pal_apu_tests/06.len_timing_mode1.nes` | fail | APU frame sequencer under **PAL** timing. The suite's readme calls it "PAL NES APU Tests [...] tested on a PAL NES", though its files carry NTSC headers, so the harness runs it on a PAL console; 3 of its 10 pass. | screen: APU MODE 1 LENGTH TIMING / FAILED: #5 |
| `nes-test-roms/pal_apu_tests/07.irq_flag_timing.nes` | fail | APU frame sequencer under **PAL** timing. The suite's readme calls it "PAL NES APU Tests [...] tested on a PAL NES", though its files carry NTSC headers, so the harness runs it on a PAL console; 3 of its 10 pass. | screen: APU FRAME IRQ FLAG TIMING / FAILED: #2 |
| `nes-test-roms/pal_apu_tests/08.irq_timing.nes` | fail | APU frame sequencer under **PAL** timing. The suite's readme calls it "PAL NES APU Tests [...] tested on a PAL NES", though its files carry NTSC headers, so the harness runs it on a PAL console; 3 of its 10 pass. | screen: APU FRAME IRQ TIMING / FAILED: #2 |
| `nes-test-roms/pal_apu_tests/10.len_halt_timing.nes` | fail | APU frame sequencer under **PAL** timing. The suite's readme calls it "PAL NES APU Tests [...] tested on a PAL NES", though its files carry NTSC headers, so the harness runs it on a PAL console; 3 of its 10 pass. | screen: APU LENGTH HALT TIMING / FAILED: #5 |
| `nes-test-roms/pal_apu_tests/11.len_reload_timing.nes` | fail | APU frame sequencer under **PAL** timing. The suite's readme calls it "PAL NES APU Tests [...] tested on a PAL NES", though its files carry NTSC headers, so the harness runs it on a PAL console; 3 of its 10 pass. | screen: APU LENGTH RELOAD TIMING / FAILED: #3 |
| `nes-test-roms/sprdma_and_dmc_dma/sprdma_and_dmc_dma.nes` | fail | OAM DMA against DMC DMA | B8EA17D9 / SPRDMA and DMC DMA / Failed (code 1) |
| `nes-test-roms/sprdma_and_dmc_dma/sprdma_and_dmc_dma_512.nes` | fail | OAM DMA against DMC DMA | B8EA17D9 / SPRDMA and DMC DMA / Failed (code 1) |

## Every ROM, by suite

### `AccuracyCoin`

**0 passed of 1** (1 fail)

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `AccuracyCoin.nes` | fail | 89/147 tests passed; stopped the run: Interrupt flag latency (page "CPU Interrupts") — never returned; skipped on a later attempt, PPU Register Mirroring (page "PPU Behavior") — took the run out of the all-test loop; skipped on a later attempt, PPU Register Open Bus (page "PPU Behavior") — took the run out of the all-test loop; skipped on a later attempt, PPU Read Buffer (page "PPU Behavior") — to… |  |  |  |

### `holy-mapperel`

**40 passed of 40**

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `M0_P32K_C8K_V.nes` | pass | NROM: 32K PRG ROM, 8K PRG RAM OK, 8K CHR ROM OK, detailed 0000 |  |  |  |
| `M0_P32K_CR32K_V.nes` | pass | NROM: 32K PRG ROM, 8K PRG RAM OK, 8K CHR RAM OK, detailed 0000 |  |  |  |
| `M0_P32K_CR8K_V.nes` | pass | NROM: 32K PRG ROM, 8K PRG RAM OK, 8K CHR RAM OK, detailed 0000 |  |  |  |
| `M10_P128K_C64K_S8K.nes` | pass | F*ROM (MMC4): 128K PRG ROM, 8K PRG RAM OK, 64K CHR ROM OK, detailed 0000 |  |  |  |
| `M10_P128K_C64K_W8K.nes` | pass | F*ROM (MMC4): 128K PRG ROM, 8K PRG RAM OK, 64K CHR ROM OK, detailed 0000 |  |  |  |
| `M118_P128K_C64K.nes` | pass | T*SROM (MMC3): 128K PRG ROM, 8K PRG RAM OK, 64K CHR ROM OK, detailed 0000 |  |  |  |
| `M11_P64K_C64K_V.nes` | pass | COLOR DREAMS: 64K PRG ROM, 8K PRG RAM OK, 64K CHR ROM OK, detailed 0000 |  |  |  |
| `M11_P64K_CR32K_V.nes` | pass | COLOR DREAMS: 64K PRG ROM, 8K PRG RAM OK, 32K CHR RAM OK, detailed 0000 |  |  |  |
| `M180_P128K_CR8K_H.nes` | pass | U*ROM (7408): 128K PRG ROM, 8K PRG RAM OK, 8K CHR RAM OK, detailed 0000 |  |  |  |
| `M180_P128K_H.nes` | pass | U*ROM (7408): 128K PRG ROM, 8K PRG RAM OK, 8K CHR RAM OK, detailed 0000 |  |  |  |
| `M1_P128K.nes` | pass | SNROM (MMC1): 128K PRG ROM, 8K PRG RAM OK, 8K CHR RAM OK, detailed 0000 |  |  |  |
| `M1_P128K_C128K.nes` | pass | SKROM (MMC1): 128K PRG ROM, 8K PRG RAM OK, 128K CHR ROM OK, detailed 0000 |  |  |  |
| `M1_P128K_C128K_S8K.nes` | pass | SKROM (MMC1): 128K PRG ROM, 8K PRG RAM OK, 128K CHR ROM OK, detailed 0000 |  |  |  |
| `M1_P128K_C128K_W8K.nes` | pass | SKROM (MMC1): 128K PRG ROM, 8K PRG RAM OK, 128K CHR ROM OK, detailed 0000 |  |  |  |
| `M1_P128K_C32K.nes` | pass | SJROM (MMC1): 128K PRG ROM, 8K PRG RAM OK, 32K CHR ROM OK, detailed 0000 |  |  |  |
| `M1_P128K_C32K_S8K.nes` | pass | SJROM (MMC1): 128K PRG ROM, 8K PRG RAM OK, 32K CHR ROM OK, detailed 0000 |  |  |  |
| `M1_P128K_C32K_W8K.nes` | pass | SJROM (MMC1): 128K PRG ROM, 8K PRG RAM OK, 32K CHR ROM OK, detailed 0000 |  |  |  |
| `M1_P128K_CR8K.nes` | pass | SNROM (MMC1): 128K PRG ROM, 8K PRG RAM OK, 8K CHR RAM OK, detailed 0000 |  |  |  |
| `M1_P512K_CR8K_S32K.nes` | pass | SXROM (MMC1): 512K PRG ROM, 32K PRG RAM OK, 8K CHR RAM OK, detailed 0000 |  |  |  |
| `M1_P512K_CR8K_S8K.nes` | pass | SUROM (MMC1): 512K PRG ROM, 8K PRG RAM OK, 8K CHR RAM OK, detailed 0000 |  |  |  |
| `M1_P512K_S32K.nes` | pass | SXROM (MMC1): 512K PRG ROM, 32K PRG RAM OK, 8K CHR RAM OK, detailed 0000 |  |  |  |
| `M1_P512K_S8K.nes` | pass | SUROM (MMC1): 512K PRG ROM, 8K PRG RAM OK, 8K CHR RAM OK, detailed 0000 |  |  |  |
| `M28_P512K.nes` | pass | INL-ROM (A53): 512K PRG ROM, PRG RAM MISSING, 8K CHR RAM OK, detailed 0000 |  |  |  |
| `M28_P512K_CR32K.nes` | pass | INL-ROM (A53): 512K PRG ROM, PRG RAM MISSING, 32K CHR RAM OK, detailed 0000 |  |  |  |
| `M2_P128K_CR8K_V.nes` | pass | U*ROM: 128K PRG ROM, 8K PRG RAM OK, 8K CHR RAM OK, detailed 0000 |  |  |  |
| `M2_P128K_V.nes` | pass | U*ROM: 128K PRG ROM, 8K PRG RAM OK, 8K CHR RAM OK, detailed 0000 |  |  |  |
| `M34_P128K_CR8K_H.nes` | pass | BNROM: 128K PRG ROM, PRG RAM MISSING, 8K CHR RAM OK, detailed 0000 |  |  |  |
| `M34_P128K_H.nes` | pass | BNROM: 128K PRG ROM, PRG RAM MISSING, 8K CHR RAM OK, detailed 0000 |  |  |  |
| `M3_P32K_C32K_H.nes` | pass | CNROM: 32K PRG ROM, 8K PRG RAM OK, 32K CHR ROM OK, detailed 0000 |  |  |  |
| `M4_P128K.nes` | pass | TNROM (MMC3): 128K PRG ROM, 8K PRG RAM OK, 8K CHR RAM OK, detailed 0000 |  |  |  |
| `M4_P128K_CR32K.nes` | pass | T*ROM (MMC3): 128K PRG ROM, 8K PRG RAM OK, 32K CHR RAM OK, detailed 0000 |  |  |  |
| `M4_P128K_CR8K.nes` | pass | TNROM (MMC3): 128K PRG ROM, 8K PRG RAM OK, 8K CHR RAM OK, detailed 0000 |  |  |  |
| `M4_P256K_C256K.nes` | pass | TSROM (MMC3): 256K PRG ROM, 8K PRG RAM OK, 256K CHR ROM OK, detailed 0000 |  |  |  |
| `M66_P64K_C16K_V.nes` | pass | G*ROM: 64K PRG ROM, 8K PRG RAM OK, 16K CHR ROM OK, detailed 0000 |  |  |  |
| `M69_P128K_C64K_S8K.nes` | pass | J*ROM (FME-7): 128K PRG ROM, 8K PRG RAM OK, 64K CHR ROM OK, detailed 0000 |  |  |  |
| `M69_P128K_C64K_W8K.nes` | pass | J*ROM (FME-7): 128K PRG ROM, 8K PRG RAM OK, 64K CHR ROM OK, detailed 0000 |  |  |  |
| `M78.3_P128K_C64K.nes` | pass | HOLY DIVER: 128K PRG ROM, 8K PRG RAM OK, 64K CHR ROM OK, detailed 0000 |  |  |  |
| `M7_P128K.nes` | pass | A*ROM: 128K PRG ROM, 8K PRG RAM OK, 8K CHR RAM OK, detailed 0000 |  |  |  |
| `M7_P128K_CR8K.nes` | pass | A*ROM: 128K PRG ROM, 8K PRG RAM OK, 8K CHR RAM OK, detailed 0000 |  |  |  |
| `M9_P128K_C64K.nes` | pass | PNROM (MMC2): 128K PRG ROM, 8K PRG RAM OK, 64K CHR ROM OK, detailed 0000 |  |  |  |

### `nes-test-roms/240pee`

**0 passed of 2** (2 manual)

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `240pee-bnrom.nes` | manual | screen (no documented verdict): ABBBBABBBBABBBBABBBBABBBBABBBBAB / CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC / DEEEEEEEEEEEEEEEEF / E                E / E                E / E                E / E                E / E                E / GG    E                E / HIIJ   E                E / KIIL   E                E / MIIN   E                E / OO    E                E / E                E / E            … |  |  |  |
| `240pee.nes` | manual | screen (no documented verdict): ABBBBABBBBABBBBABBBBABBBBABBBBAB / CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC / DEEEEEEEEEEEEEEEEF / E                E / E                E / E                E / E                E / E                E / GG    E                E / HIIJ   E                E / KIIL   E                E / MIIN   E                E / OO    E                E / E                E / E            … |  |  |  |

### `nes-test-roms/MMC1_A12`

**0 passed of 1** (1 manual)

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `mmc1_a12.nes` | manual | screen (no documented verdict): OOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOO / OOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOO / OOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOO / OOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOO / OOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOO / OOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOO / OOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOO / OOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOO / OOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOO / OOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOO / OOOOOOOOOOOOOOOOOO… |  |  |  |

### `nes-test-roms/PaddleTest3`

**0 passed of 1** (1 manual)

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `PaddleTest.nes` | manual | screen (no documented verdict): Value: ND / Paddle value should be 62-F2 / ND means the dial couldn't be / read. / Red block will turn green / when paddle button is pressed |  |  |  |

### `nes-test-roms/apu_mixer`

**4 passed of 4**

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `dmc.nes` | pass | 2. Should be nearly silent. / 3. Should play short tone. / Some clicking might occur between the two tones.Also, a faint tone might be audible through headphones. |  |  |  |
| `noise.nes` | pass | and out, without any tone. / 3. Should play short tone. / Some clicking might occur between the two tones.Also, a faint tone might be audible through headphones. |  |  |  |
| `square.nes` | pass | 2. Should be nearly silent. / 3. Should play short tone. / Some clicking might occur between the two tones.Also, a faint tone might be audible through headphones. |  |  |  |
| `triangle.nes` | pass | 2. Should be nearly silent. / 3. Should play short tone. / Some clicking might occur between the two tones.Also, a faint tone might be audible through headphones. |  |  |  |

### `nes-test-roms/apu_reset`

**6 passed of 6**

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `4015_cleared.nes` | pass | 4015_cleared / Passed | PASS | PASS |  |
| `4017_timing.nes` | pass | Delay after effective $4017 write: 10 / 4017_timing / Passed | PASS | PASS |  |
| `4017_written.nes` | pass | 4017_written / Passed | PASS | FAIL |  |
| `irq_flag_cleared.nes` | pass | irq_flag_cleared / Passed | PASS | PASS |  |
| `len_ctrs_enabled.nes` | pass | len_ctrs_enabled / Passed | PASS | PASS |  |
| `works_immediately.nes` | pass | works_immediately / Passed | PASS | PASS |  |

### `nes-test-roms/apu_test`

**9 passed of 9**

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `apu_test.nes` | pass | 8-dmc_rates / Passed / All 8 tests passed | PASS | PASS |  |
| `1-len_ctr.nes` | pass | 1-len_ctr / Passed | PASS | PASS |  |
| `2-len_table.nes` | pass | 2-len_table / Passed | PASS | PASS |  |
| `3-irq_flag.nes` | pass | 3-irq_flag / Passed | PASS | PASS |  |
| `4-jitter.nes` | pass | 4-jitter / Passed | PASS | PASS |  |
| `5-len_timing.nes` | pass | 5-len_timing / Passed | PASS | PASS |  |
| `6-irq_flag_timing.nes` | pass | 6-irq_flag_timing / Passed | PASS | PASS |  |
| `7-dmc_basics.nes` | pass | 7-dmc_basics / Passed | PASS | PASS |  |
| `8-dmc_rates.nes` | pass | 8-dmc_rates / Passed | PASS | PASS |  |

### `nes-test-roms/blargg_apu_2005.07.30`

**11 passed of 11**

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `01.len_ctr.nes` | pass | screen: $01 | PASS | PASS |  |
| `02.len_table.nes` | pass | screen: $01 | PASS | PASS |  |
| `03.irq_flag.nes` | pass | screen: $01 | PASS | PASS |  |
| `04.clock_jitter.nes` | pass | screen: $01 | PASS | PASS |  |
| `05.len_timing_mode0.nes` | pass | screen: $01 | PASS | PASS |  |
| `06.len_timing_mode1.nes` | pass | screen: $01 | PASS | PASS |  |
| `07.irq_flag_timing.nes` | pass | screen: $01 | PASS | PASS |  |
| `08.irq_timing.nes` | pass | screen: $01 | PASS | PASS |  |
| `09.reset_timing.nes` | pass | screen: $01 | PASS | PASS |  |
| `10.len_halt_timing.nes` | pass | screen: $01 | PASS | PASS |  |
| `11.len_reload_timing.nes` | pass | screen: $01 | PASS | PASS |  |

### `nes-test-roms/blargg_litewall`

**0 passed of 5** (5 manual)

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `blargg_litewall-10c.nes` | manual | screen (no documented verdict): DFOXGPYENWGPYENWGPYENWGPYENWFOXD / DIR[JS\HQZJS\HQZJS\HQZJS\HQZIR[D / DLU^MV_KT]MV_KT]MV_KT]MV_KT]LU^D / DDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDD / DDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDD / DDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDD / $&/8'09%.7'09%.7'09%.7'09%.7&/8$ / $)2;*3<(1:*3<(1:*3<(1:*3<(1:)2;$ / $,5>-6?+4=-6?+4=-6?+4=-6?+4=,5>$ / $$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$ / $$$$$$$$$$$$$$$$$$… |  |  |  |
| `blargg_litewall-9.nes` | manual | nothing readable on screen; read the PNG of the final frame |  |  |  |
| `litewall2.nes` | manual | screen (no documented verdict): AAA BBB CCC AAA BBB CCC AAA BBB / AAA BBB CCC AAA BBB CCC AAA BBB / AAA BBB CCC AAA BBB CCC AAA BBB / BBB AAA BBB AAA BBB AAA BBB AAA / BBB AAA BBB AAA BBB AAA BBB AAA / BBB AAA BBB AAA BBB AAA BBB AAA / AAA BBB AAA BBB AAA BBB AAA BBB / AAA BBB AAA BBB AAA BBB AAA BBB / AAA BBB AAA BBB AAA BBB AAA BBB / BBB AAA BBB AAA BBB AAA BBB AAA / BBB AAA BBB AAA BBB AAA BBB … |  |  |  |
| `litewall3.nes` | manual | screen (no documented verdict): AAADBBBECCCFAAADBBBECCCFAAADBBBE / AAADBBBECCCFAAADBBBECCCFAAADBBBE / AAADBBBECCCFAAADBBBECCCFAAADBBBE / BBB AAA BBB AAA BBB AAA BBB AAA / BBB AAA BBB AAA BBB AAA BBB AAA / BBB AAA BBB AAA BBB AAA BBB AAA / AAA BBB AAA BBB AAA BBB AAA BBB / AAA BBB AAA BBB AAA BBB AAA BBB / AAA BBB AAA BBB AAA BBB AAA BBB / BBB AAA BBB AAA BBB AAA BBB AAA / BBB AAA BBB AAA BBB AAA B… |  |  |  |
| `litewall5.nes` | manual | screen (no documented verdict): AAADBBBECCCFAAADBBBECCCFAAADBBBE / AAADBBBECCCFAAADBBBECCCFAAADBBBE / AAADBBBECCCFAAADBBBECCCFAAADBBBE / BBB AAA BBB AAA BBB AAA BBB AAA / BBB AAA BBB AAA BBB AAA BBB AAA / BBB AAA BBB AAA BBB AAA BBB AAA / AAA BBB AAA BBB AAA BBB AAA BBB / AAA BBB AAA BBB AAA BBB AAA BBB / AAA BBB AAA BBB AAA BBB AAA BBB / BBB AAA BBB AAA BBB AAA BBB AAA / BBB AAA BBB AAA BBB AAA B… |  |  |  |

### `nes-test-roms/blargg_nes_cpu_test5`

**0 passed of 2** (2 manual)

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `cpu.nes` | manual | screen (no documented verdict): Running tests... / ----------------------------- / 01-implied / 02-immediate / 03-zero_page / 04-zp_xy / 05-absolute / 06-abs_xy / 07-ind_x / 08-ind_y / 09-branches / 10-stack / 11-special / ----------------------------- / All tests complete | FAIL | FAIL |  |
| `official.nes` | manual | screen (no documented verdict): Running tests... / ----------------------------- / 01-implied / 02-immediate / 03-zero_page / 04-zp_xy / 05-absolute / 06-abs_xy / 07-ind_x / 08-ind_y / 09-branches / 10-stack / 11-special / ----------------------------- / All tests complete | PASS | PASS |  |

### `nes-test-roms/blargg_ppu_tests_2005.09.15b`

**4 passed of 5** (1 fail)

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `palette_ram.nes` | pass | screen: $01 | PASS | PASS |  |
| `power_up_palette.nes` | fail | screen: $02 | PASS | FAIL |  |
| `sprite_ram.nes` | pass | screen: $01 | PASS | PASS |  |
| `vbl_clear_time.nes` | pass | screen: $01 | PASS | PASS |  |
| `vram_access.nes` | pass | screen: $01 | PASS | PASS |  |

### `nes-test-roms/branch_timing_tests`

**3 passed of 3**

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `1.Branch_Basics.nes` | pass | screen: BRANCH TIMING BASICS / PASSED | PASS | PASS |  |
| `2.Backward_Branch.nes` | pass | screen: BACKWARD BRANCH TIMING / PASSED | PASS | PASS |  |
| `3.Forward_Branch.nes` | pass | screen: FORWARD BRANCH TIMING / PASSED | PASS | PASS |  |

### `nes-test-roms/cpu_dummy_reads`

**1 passed of 1**

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `cpu_dummy_reads.nes` | pass | screen: cpu_dummy_reads / Passed | PASS | PASS |  |

### `nes-test-roms/cpu_dummy_writes`

**2 passed of 2**

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `cpu_dummy_writes_oam.nes` | pass | 03234363C3E3 13335373D3F3 / 1B3B5B7BDBFB / Passed |  |  |  |
| `cpu_dummy_writes_ppumem.nes` | pass | 03234363C3E3 13335373D3F3 / 1B3B5B7BDBFB / Passed |  |  |  |

### `nes-test-roms/cpu_exec_space`

**2 passed of 2**

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `test_cpu_exec_space_apu.nes` | pass | 40FE / 40FF / Passed |  |  |  |
| `test_cpu_exec_space_ppuio.nes` | pass | JMP+RTI TEST OK / JMP+BRK TEST OK / Passed |  |  |  |

### `nes-test-roms/cpu_interrupts_v2`

**6 passed of 6**

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `cpu_interrupts.nes` | pass | 5-branch_delays_irq / Passed / All 5 tests passed | FAIL | FAIL |  |
| `1-cli_latency.nes` | pass | 1-cli_latency / Passed | PASS | PASS |  |
| `2-nmi_and_brk.nes` | pass | 27  36  00 / 2-nmi_and_brk / Passed | FAIL | PASS |  |
| `3-nmi_and_irq.nes` | pass | 25  20 / 3-nmi_and_irq / Passed | FAIL | PASS |  |
| `4-irq_and_dma.nes` | pass | 9 +527 / 4-irq_and_dma / Passed | FAIL | FAIL |  |
| `5-branch_delays_irq.nes` | pass | 09 03 0A / 5-branch_delays_irq / Passed | FAIL | FAIL |  |

### `nes-test-roms/cpu_reset`

**2 passed of 2**

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `ram_after_reset.nes` | pass | ram_after_reset / Passed | PASS | PASS |  |
| `registers.nes` | pass | 34 56 78 FF 0F / registers / Passed | PASS | PASS |  |

### `nes-test-roms/cpu_timing_test6`

**1 passed of 1**

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `cpu_timing_test.nes` | pass | screen: 6502 TIMING TEST (16 SECONDS) / OFFICIAL INSTRUCTIONS ONLY / PASSED | PASS | PASS |  |

### `nes-test-roms/dmc_dma_during_read4`

**2 passed of 5** (1 fail, 2 manual)

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `dma_2007_read.nes` | manual | screen (no documented verdict): 11 22 / 11 22 / 11 22 / 11 22 / 11 22 / 498C5C5F | PASS | PASS |  |
| `dma_2007_write.nes` | pass | screen: 11 11 AA 33 44 55 66 77 / 11 11 AA 33 44 55 66 77 / 11 11 AA 33 44 55 66 77 / 11 11 AA 33 44 55 66 77 / 11 11 AA 33 44 55 66 77 / dma_2007_write / Passed | PASS | PASS |  |
| `dma_4016_read.nes` | fail | screen: 08 08 08 08 08 / FBF7C7B1 / dma_4016_read / Failed | FAIL | PASS |  |
| `double_2007_read.nes` | manual | screen (no documented verdict): 22 33 44 55 66 / 33 44 55 66 77 / D84F6815 | FAIL | FAIL |  |
| `read_write_2007.nes` | pass | screen: 33 11 22 33 09 55 66 77 / 33 11 22 33 09 55 66 77 / read_write_2007 / Passed | PASS | PASS |  |

### `nes-test-roms/dmc_tests`

**0 passed of 4** (4 manual)

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `buffer_retained.nes` | manual | nothing readable on screen; read the PNG of the final frame |  |  |  |
| `latency.nes` | manual | nothing readable on screen; read the PNG of the final frame |  |  |  |
| `status.nes` | manual | nothing readable on screen; read the PNG of the final frame |  |  |  |
| `status_irq.nes` | manual | nothing readable on screen; read the PNG of the final frame |  |  |  |

### `nes-test-roms/dpcmletterbox`

**0 passed of 1** (1 manual)

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `dpcmletterbox.nes` | manual | screen (no documented verdict): @         @    @ d / A         A    A e / \@d`b`@      @ / ]AeacaA      A / @bltr@      @P      @ / AcmusA      AQ      A / @   @       @   R@ / A   A       A   SA / @      @   \@@       X@ / A      A   ]AA       YA / @   @   @   @  @ @ / A   A   A   A  A A / @  @    @   @   @ / A  A    A   A   A / @  @   @          \ / A  A   A          ] / @   @  @   N @  @nd` @tP… |  |  |  |

### `nes-test-roms/exram`

**0 passed of 1** (1 manual)

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `mmc5exram.nes` | manual | screen (no documented verdict): MMC5 Executable ExRAM Test / ________________________________ / This is basically just my / 'copper bars' test program, / except I've modified it so / the code that determines / where each color bar should / be displayed during each / frame is not executed from / ROM, but is copied into the / MMC5's ExRAM during startup / and then executed from there / during each V… | PASS | PASS |  |

### `nes-test-roms/full_palette`

**0 passed of 3** (3 manual)

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `flowing_palette.nes` | manual | nothing readable on screen; read the PNG of the final frame | PASS | PASS |  |
| `full_palette.nes` | manual | nothing readable on screen; read the PNG of the final frame | PASS | PASS |  |
| `full_palette_smooth.nes` | manual | nothing readable on screen; read the PNG of the final frame | PASS | PASS |  |

### `nes-test-roms/instr_misc`

**5 passed of 5**

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `instr_misc.nes` | pass | 04-dummy_reads_apu / Passed / All 4 tests passed | PASS | PASS |  |
| `01-abs_x_wrap.nes` | pass | 01-abs_x_wrap / Passed | PASS | PASS |  |
| `02-branch_wrap.nes` | pass | 02-branch_wrap / Passed | PASS | PASS |  |
| `03-dummy_reads.nes` | pass | 03-dummy_reads / Passed | PASS | PASS |  |
| `04-dummy_reads_apu.nes` | pass | 04-dummy_reads_apu / Passed | PASS | PASS |  |

### `nes-test-roms/instr_test-v3`

**17 passed of 17**

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `all_instrs.nes` | pass | All 15 tests passed | FAIL | FAIL |  |
| `official_only.nes` | pass | All 15 tests passed | PASS | PASS |  |
| `01-implied.nes` | pass | 01-implied / Passed |  |  |  |
| `02-immediate.nes` | pass | 02-immediate / Passed | FAIL | FAIL |  |
| `03-zero_page.nes` | pass | 03-zero_page / Passed | PASS | PASS |  |
| `04-zp_xy.nes` | pass | 04-zp_xy / Passed | PASS | PASS |  |
| `05-absolute.nes` | pass | 05-absolute / Passed | PASS | PASS |  |
| `06-abs_xy.nes` | pass | 06-abs_xy / Passed | FAIL | FAIL |  |
| `07-ind_x.nes` | pass | 07-ind_x / Passed | PASS | PASS |  |
| `08-ind_y.nes` | pass | 08-ind_y / Passed | PASS | PASS |  |
| `09-branches.nes` | pass | 09-branches / Passed | PASS | PASS |  |
| `10-stack.nes` | pass | 10-stack / Passed | PASS | PASS |  |
| `11-jmp_jsr.nes` | pass | 11-jmp_jsr / Passed | PASS | PASS |  |
| `12-rts.nes` | pass | 12-rts / Passed | PASS | PASS |  |
| `13-rti.nes` | pass | 13-rti / Passed | PASS | PASS |  |
| `14-brk.nes` | pass | 14-brk / Passed | PASS | PASS |  |
| `15-special.nes` | pass | 15-special / Passed | PASS | PASS |  |

### `nes-test-roms/instr_test-v5`

**18 passed of 18**

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `all_instrs.nes` | pass | All 16 tests passed |  |  |  |
| `official_only.nes` | pass | All 16 tests passed |  |  |  |
| `01-basics.nes` | pass | 01-basics / Passed |  |  |  |
| `02-implied.nes` | pass | 02-implied / Passed |  |  |  |
| `03-immediate.nes` | pass | 03-immediate / Passed |  |  |  |
| `04-zero_page.nes` | pass | 04-zero_page / Passed |  |  |  |
| `05-zp_xy.nes` | pass | 05-zp_xy / Passed |  |  |  |
| `06-absolute.nes` | pass | 06-absolute / Passed |  |  |  |
| `07-abs_xy.nes` | pass | 07-abs_xy / Passed |  |  |  |
| `08-ind_x.nes` | pass | 08-ind_x / Passed |  |  |  |
| `09-ind_y.nes` | pass | 09-ind_y / Passed |  |  |  |
| `10-branches.nes` | pass | 10-branches / Passed |  |  |  |
| `11-stack.nes` | pass | 11-stack / Passed |  |  |  |
| `12-jmp_jsr.nes` | pass | 12-jmp_jsr / Passed |  |  |  |
| `13-rts.nes` | pass | 13-rts / Passed |  |  |  |
| `14-rti.nes` | pass | 14-rti / Passed |  |  |  |
| `15-brk.nes` | pass | 15-brk / Passed |  |  |  |
| `16-special.nes` | pass | 16-special / Passed |  |  |  |

### `nes-test-roms/instr_timing`

**3 passed of 3**

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `instr_timing.nes` | pass | 2-branch_timing / Passed / All 2 tests passed | PASS | PASS |  |
| `1-instr_timing.nes` | pass | Unofficial instructions... / 1-instr_timing / Passed | PASS | PASS |  |
| `2-branch_timing.nes` | pass | 2-branch_timing / Passed | PASS | PASS |  |

### `nes-test-roms/m22chrbankingtest`

**0 passed of 1** (1 manual)

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `0-127.nes` | manual | screen (no documented verdict): @P`p / AQaq / BRbr / CScs / DTdt / EUeu / FVfv / GWgw / HXhx / IYiy / JZjz / K[k{ / L\l/ / M]m} / N^n~ / O_o |  |  |  |

### `nes-test-roms/mmc3_irq_tests`

**5 passed of 6** (1 fail)

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `1.Clocking.nes` | pass | screen: MMC3 IRQ COUNTER / PASSED | PASS | PASS |  |
| `2.Details.nes` | pass | screen: MMC3 IRQ COUNTER DETAILS / PASSED | PASS | PASS |  |
| `3.A12_clocking.nes` | pass | screen: MMC3 IRQ COUNTER A12 / PASSED | PASS | PASS |  |
| `4.Scanline_timing.nes` | pass | screen: MMC3 IRQ TIMING / PASSED | PASS | PASS |  |
| `5.MMC3_rev_A.nes` | fail | screen: MMC3 IRQ COUNTER REVISION A / FAILED #3 | FAIL | FAIL |  |
| `6.MMC3_rev_B.nes` | pass | screen: MMC3 IRQ COUNTER REVISION B / PASSED | FAIL | PASS |  |

### `nes-test-roms/mmc3_test`

**5 passed of 6** (1 fail)

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `1-clocking.nes` | pass | 1-clocking / Passed | PASS | PASS |  |
| `2-details.nes` | pass | 2-details / Passed | PASS | PASS |  |
| `3-A12_clocking.nes` | pass | 3-A12_clocking / Passed | PASS | PASS |  |
| `4-scanline_timing.nes` | pass | 4-scanline_timing / Passed | FAIL | PASS |  |
| `5-MMC3.nes` | pass | 5-MMC3 / Passed |  |  |  |
| `6-MMC6.nes` | fail | IRQ shouldn't occur when reloading after counter normally reaches 0 / 6-MMC6 / Failed #3 (code 3) |  |  |  |

### `nes-test-roms/mmc3_test_2`

**5 passed of 6** (1 fail)

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `1-clocking.nes` | pass | 1-clocking / Passed |  |  |  |
| `2-details.nes` | pass | 2-details / Passed |  |  |  |
| `3-A12_clocking.nes` | pass | 3-A12_clocking / Passed |  |  |  |
| `4-scanline_timing.nes` | pass | 4-scanline_timing / Passed |  |  |  |
| `5-MMC3.nes` | pass | 5-MMC3 / Passed |  |  |  |
| `6-MMC3_alt.nes` | fail | IRQ shouldn't be set when reloading to 0 due to counter naturally reaching 0 previously / 6-MMC3_alt / Failed #2 (code 2) |  |  |  |

### `nes-test-roms/mmc5test`

**0 passed of 1** (1 manual)

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `mmc5test.nes` | manual | screen (no documented verdict): (((((((((((((((( ((((((((((((((( / ((((((((((((((((A((((((((((((((( / ((((((((((((((((B((((((((((((((( / ((((((((((((((((C((((((((((((((( / ((((((((((((((((D((((((((((((((( / ((((((((((((((((E((((((((((((((( / ((((((((((((((((F((((((((((((((( / ((((((((((((((((G((((((((((((((( / ((((((((((((((((H((((((((((((((( / ((((((((((((((((I((((((((((((((( / ((((((((((((((((J(… |  |  |  |

### `nes-test-roms/mmc5test_v2`

**0 passed of 1** (1 manual)

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `mmc5test.nes` | manual | screen (no documented verdict): ################################ / ################################ / ################################ / ################################ / #######VVLE#LQZ#KJWT#\N[\####### / ################################ / #######ZNPR[\NZ#[N\\RWP[$####### / ################################ / #### KS#XJ\\NZW#\JKUN$##    #### / ####KP#XJ\\NZW#\JKUN$###A   #### / ##################… |  |  |  |

### `nes-test-roms/nes15-1.0.0`

**0 passed of 2** (2 manual)

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `nes15-NTSC.nes` | manual | screen (no documented verdict): LLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLL / LLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLL / LL           -   -            LL / LL                            LL / LL  DDA DDA       DDA DDA DDA LL / LL F  GF  G      F  GF  GF  G LL / LL F  GF  G      F  GF  GF  G LL / LL BEECBEEC      BEECBEECBEEC LL / LL      DDA       DDA         LL / LL     F  G      F  G         LL / LL     F  G      F… |  |  |  |
| `nes15-PAL.nes` | manual | screen (no documented verdict): LLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLL / LLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLL / LL           -   -            LL / LL                            LL / LL  DDA DDA       DDA DDA DDA LL / LL F  GF  G      F  GF  GF  G LL / LL F  GF  G      F  GF  GF  G LL / LL BEECBEEC      BEECBEECBEEC LL / LL      DDA       DDA         LL / LL     F  G      F  G         LL / LL     F  G      F… |  |  |  |

### `nes-test-roms/nes_instr_test`

**11 passed of 11**

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `01-implied.nes` | pass | 01-implied / Passed |  |  |  |
| `02-immediate.nes` | pass | 02-immediate / Passed |  |  |  |
| `03-zero_page.nes` | pass | 03-zero_page / Passed |  |  |  |
| `04-zp_xy.nes` | pass | 04-zp_xy / Passed |  |  |  |
| `05-absolute.nes` | pass | 05-absolute / Passed |  |  |  |
| `06-abs_xy.nes` | pass | 06-abs_xy / Passed |  |  |  |
| `07-ind_x.nes` | pass | 07-ind_x / Passed |  |  |  |
| `08-ind_y.nes` | pass | 08-ind_y / Passed |  |  |  |
| `09-branches.nes` | pass | 09-branches / Passed |  |  |  |
| `10-stack.nes` | pass | 10-stack / Passed |  |  |  |
| `11-special.nes` | pass | 11-special / Passed |  |  |  |

### `nes-test-roms/nmi_sync`

**0 passed of 2** (2 manual)

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `demo_ntsc.nes` | manual | nothing readable on screen; read the PNG of the final frame | PASS | PASS |  |
| `demo_pal.nes` | manual | nothing readable on screen; read the PNG of the final frame | PASS | PASS |  |

### `nes-test-roms/nrom368`

**0 passed of 2** (2 manual)

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `fail368.nes` | manual | nothing readable on screen; read the PNG of the final frame |  |  |  |
| `test1.nes` | manual | nothing readable on screen; read the PNG of the final frame |  |  |  |

### `nes-test-roms/ny2011`

**0 passed of 1** (1 manual)

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `ny2011.nes` | manual | screen (no documented verdict): //////////////////////////////// / /A/////A/////////A/////A//////// / BCD/A/ECF/A//A//GCD/A/ECF/A//A// / HIJBCDKILBCDECFEMIJBCDKILBCDECN/ / OIPHIQMIROIPKISTIIUVIJWIROIUTIUX / IIYIIIIIIIIIYIIIIIIIIIIYIIIIIIII / IZ[\IIIYIIIZ[\IIIIYIIIZ[\IIIYIII / I]^_IIZ[\II]^_IIIZ[\II]^_IIZ[\II / !"#$I]^_I !"#$II]^_I !"#$I]^_II / %&'() !"#$%&'()I !"#$%&'() !"#$I / *+,-.%&'()*+,-.I%&'… |  |  |  |

### `nes-test-roms/oam_read`

**1 passed of 1**

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `oam_read.nes` | pass | ---------------- / oam_read / Passed | PASS | PASS |  |

### `nes-test-roms/oam_stress`

**1 passed of 1**

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `oam_stress.nes` | pass | ---------------- / oam_stress / Passed | PASS | PASS |  |

### `nes-test-roms/other`

**0 passed of 63** (1 unsupported, 62 manual)

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `2003-test.nes` | manual | screen (no documented verdict): TEST 1 READY / 00 |  |  |  |
| `8bitpeoples_-_deadline_console_invitro.nes` | manual | screen (no documented verdict): `abcdefghijklmnopqrstuvwxyz{/}~ / " %"# / & / '() *+ / ST UVWXYZ / [\]^_ / 0123456789:;<=>?:::::::::::::::: / @ABCDEFGHIJKLMNOMMMMMMMMMMMMMMMM / !            ! |  |  |  |
| `BLOCKS.NES` | manual | screen (no documented verdict): $$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$ / $$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$ / $$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$ / $$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$ / $$$$$$$$$$$$$$67$$$$$$$$$$$$$$$$ / $$$$$$$$$$$$$5%%8$$$$$$$$$$$$$$$ / $$$$$$$$$$$$$9:;<$$$$$$$$$$$$$67 / $$$$$$$$$$$$$$$$$$$$$$$$$$$$$5%% / $$$$$$$$$$$$$$$$$$$$$$$$$$$$$9:; / $$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$$ / $$$$$$$$$$$$$$$$$$… |  |  |  |
| `BladeBuster.nes` | manual | screen (no documented verdict): ABCDAE  FBBGABCDABBG / HIJKHL  HIJKHIJKHMNO / PQRSPT  PQUVPWXVPYZ[ / \]^_\   \]!"\#$"\%&' / ()*+(,-.(/0+()*+(,-. / 12341225167512341225 / ABCDA89GFBBGABBGABBGABCD / HIJKH:;KHMNO<=>?HMNOHIJK / PQRSPWXVPYZ[ PT PYZ[PQRS / \]^_\#$"  !"    \%&'\]^_ / ()*+()*+  *+ (  (,-.(/0+ / 123412251234    12251675 |  |  |  |
| `CMC80s.NES` | manual | screen (no documented verdict): *** NINTENDO NES BASIC V.2 *** / 2K RAM SYSTEM  1536 BYTES FREE / READY. / LOAD "CMC'80s",8,1 / SEARCHING FOR CMC'80s |  |  |  |
| `DROPOFF7.NES` | manual | screen (no documented verdict): PQPQ / vTT                      `a`a / ! # %&'(%&) )      +, `a`a / 1 34567 569 9     :;<=pqpq / A C EtG  w9 9     d nn / W  u / a / a               a / a       a       a       a / a   a   a   a   a   a   a   a / a a a a a a a a a a a a a a a a / qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq / pppppppppppppppppppppppppppppppp / qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq / pppppppppppppppp… |  |  |  |
| `Duelito.nes` | manual | screen (no documented verdict): LO       X            O / _G     \      L     G / C     G      G     J / K / < G                 G      G / XYXM YY  X X YY   X  Y    X  Y ' / Q  >  '   D      G      G      C / Z[Z Z[[  Z Z [[[  Z Z[ [  Z Z[ [ / X_ Y    8   \      ? / YXY   Z  [     Z  [    Z  [    Z / XYZX [Y ZZ X[ Y   ZX [ Y  Z X[ Y / YZ[ X ZY [   XZ   [    Z  [    Z / [     Z  [ / XYXXYYY  XXXY … |  |  |  |
| `FLAME.NES` | manual | screen (no documented verdict): ! / E !      3 / VJma^,<  5 / N  -5       ? / ?  2"V/     w' / U tbJN / *J          j'  /Q / %m           gdP*Q / /             bB-?9' / 6k              0"YH$ / _               G*5( / \               /E9 / 9                  N / V                   N! / 6}                   B / N                    p0 / [                     \* / 0n                   wD / /m        … |  |  |  |
| `GENIE.NES` | manual | screen (no documented verdict): BLL            BLL       H / EBCBLLEMIFBLND EBCBLNDMLFJBLND / F BJ  EEEJJ D  F BJ D E JJJ D / LD L DDDH LLD  LD LLDD HH LLD / J  HMN HMF  IF JLN NDNDHM  NDND / EE  GA  GI J  E BD  FD  E  J J / JLN  EB  E  HABDBDB BDF  EB J J / LDLDHLL HL   HD HLL LDLDHLL  LD / ILEHMJD ND BLG JNN NDNDNDNDN ND / J CA GD  J  HGA  J  HAI  FD JFJ / HA E EF  J  B N  J   FD  J  J N / HL HL… |  |  |  |
| `GREYS.NES` | manual | screen (no documented verdict): #&!           !  % ,11/.-,+( / &#               & '100.,,-, / $,                3 '110-+*-0 / )%"'-        -ISB/EN93 %420../13 / (*)*      "2S        5%.753./588 / %,,(     %Er         p*5;95106:; / #,.(     @y           ABB>:769<> / %+0$    "T~           cEGDA@AACE / *//      Iq           oBGFEFIHHI / /0+      5ix         RnWIHJMSPNM / 0-       .ocXljTZ   xwh\LIKRV… |  |  |  |
| `LINUSMUS.NES` | unsupported | Corrupt header: missing iNES signature "NES\x1A". |  |  |  |
| `MOTION.NES` | manual | screen (no documented verdict): o              oeU    j / e    W T / 5  u / _                ~    aA  _ s / %    @ / /                   a / l    >                 5 / /                   V / TZ               ?o   7 bKR   P / mn   j    _  i     bUA 1 J / T                  V    :@] ?  D / ;     ?O         Z   s  ,G / 3J         Z 8 / oO         {   /%    +f     3 / P   b    G    5 P    ` / >Z     … |  |  |  |
| `PCM.demo.wgraphics.nes` | manual | screen (no documented verdict): bI Pb[SS.   w / /b ZkIP / ?b..kc k[ / Y.Y S.bc            P b k[bI Z / S[[\w / Ic C                 b /S[\\w / .SIlc / k.\\S.c                 ?[[?I. / r. Ik / S b??               [IZC. m.Z / Z / .m.bw[Z. IZ kC. / Z.c .m   b[YC  PIZ? / [YYlZIkI.c | PASS | PASS |  |
| `RasterChromaLuma.NES` | manual | screen (no documented verdict): Chroma & Luma Bars Demo V.0.5 by / ccovell@direct.ca !USE LOOPYNES! / Inspired by Mem's Demo. THANX! _ | PASS | PASS |  |
| `RasterDemo.NES` | manual | screen (no documented verdict): !"#$%&'()*+,-./ / 0123456789:;<=>? / @ABCDEFGHIJKLMNO / PQRSTUVWXYZ[\]^_ / `abcdefghijklmno | PASS | PASS |  |
| `RasterTest1.NES` | manual | screen (no documented verdict): 'Colourful and Wavy' demo...  by / Chris Covell (ccovell@direct.ca) / This is a work in progress V.0.1 | PASS | PASS |  |
| `RasterTest2.NES` | manual | screen (no documented verdict): Palette Changing Demo.. V.0.2 by / Chris Covell (ccovell@direct.ca) / Inspired by Mem's Demo. THX!   _ | PASS | PASS |  |
| `RasterTest3.NES` | manual | screen (no documented verdict): Palette Changing Demo.. V.0.3 by / Chris Covell (ccovell@direct.ca) / Inspired by Mem's Demo. THANX! _ | PASS | PASS |  |
| `RasterTest3a.NES` | manual | screen (no documented verdict): Palette Changing Demo.. V0.3a by / Chris Covell (ccovell@direct.ca) / Inspired by Mem's Demo. THANX! _ | PASS | PASS |  |
| `RasterTest3b.NES` | manual | screen (no documented verdict): Palette Changing Demo.. V0.3b by / Chris Covell (ccovell@direct.ca) / Inspired by Mem's Demo. THANX! _ | PASS | PASS |  |
| `RasterTest3c.NES` | manual | screen (no documented verdict): Palette Changing Demo.. V0.3c by / Chris Covell (ccovell@direct.ca) / Inspired by Mem's Demo. THANX! _ | PASS | PASS |  |
| `RasterTest3d.NES` | manual | screen (no documented verdict): Palette Changing Demo.. V0.3d by / Chris Covell (ccovell@direct.ca) / Inspired by Mem's Demo. THANX! _ | PASS | PASS |  |
| `RasterTest3e.NES` | manual | screen (no documented verdict): Scanline Changing Test. V0.3e by / Chris Covell (ccovell@direct.ca) / Inspired by Mem's Demo. THANX! _ | PASS | PASS |  |
| `Retrocoders - Years behind.NES` | manual | nothing readable on screen; read the PNG of the final frame | PASS | PASS |  |
| `S0.NES` | manual | screen (no documented verdict): x x x x x x x x x x x x x x x x / x x x x x x x x x x x x x x x x / x x x x x x x x x x x x x x x x / x x x x x x x x x x x x x x x x / x x x x x x x x x x x x x x x x / x x x x x x x x x x x x x x x x / x x x x x x x x x x x x x x x x / x x x x x x x x x x x x x x x x / x x x x x x x x x x x x x x x x / x x x x x x x x x x x x x x x x / x x x x x x x x x x x x x x … | PASS | PASS |  |
| `SPRITE.NES` | manual | screen (no documented verdict): P Q R S  S P T Q  U V _Q X U / Y Z [ W  W Y Y \  W ] _\ ^ W / 2000 |  |  |  |
| `Sayoonara!.NES` | manual | screen (no documented verdict): D         D           D / D         D   D   D   D / D         D   D   D   D / D         D   D   D   D / D         D       D   D / D         D       D   D / !"# $%&'(%&'()*+,-"./01!234 / 56789:;<=>?@A>?@ABCDEF:GHIJ9KLM / NOPQRSTUVWXYZWXYZ[\]^_S`abcRdef / ghijklmn opq opq rstjulvwxykz{/ / jk / z{ |  |  |  |
| `SimpleParallaxDemo.nes` | manual | screen (no documented verdict): PPPPPPPPP / P       P / P ABCCD P / P EDFCG P / P       P / PPPPPPPPP / 9:9:9:9:9:9:9:9:9:9:9:9:9:9:9:9: / ;<;<;<;<;<;<;<;<;<;<;<;<;<;<;<;< / 9:9:9:9:9:9:9:9:9:9:9:9:9:9:9:9: / ;<;<;<;<;<;<;<;<;<;<;<;<;<;<;<;< |  |  |  |
| `Streemerz_bundle.nes` | manual | screen (no documented verdict): !"#$%&'()*+, / -. /0123456789:;<= / >? @ABCDEFGHIJKLMN / OPQRSTUVWXYZ[\]^_` / a    bc  defgh / ijklmnopqrstuvwx / y |  |  |  |
| `TANESPOT.NES` | manual | screen (no documented verdict): @ABCDEFGHIJKLMN / O                PQRSTUVWXYZ[\]^ / _ !"#$%&'()*+,-./`abcdefghijklmn / o0123456789:;<=>?pqrstuvwxyz{/}~ |  |  |  |
| `TEST.NES` | manual | screen (no documented verdict): BBBBBBBB / BBBBBBBB / BBBBBBBB / BBBBBBBB |  |  |  |
| `apocalypse.nes` | manual | screen (no documented verdict): >   < / > / 7          \       6 / m         I               $ / R / n / & / > / > / , / L / 5   '        \ / ;                       . / ]             R / X    ~ / v / O               P / }                          &H / [ / w    e          d / K     5                   j & |  |  |  |
| `blargg_litewall-2.nes` | manual | screen (no documented verdict): EEFAABCCDAABCCDAABCCDAABCCDEEF / EEFAABCCDAABCCDAABCCDAABCCDEEF / EEFAABCCDAABCCDAABCCDAABCCDEEF / EEFAABCCDAABCCDAABCCDAABCCDEEF | FAIL | PASS |  |
| `blargg_litewall-9.nes` | manual | nothing readable on screen; read the PNG of the final frame |  |  |  |
| `demo jitter.nes` | manual | screen (no documented verdict): EXTRA SPACE AT THE TOP! / VIT     BLAH BLAH ETC. ETC. / EXP       ICONS GO HERE / `abc            defg / pqrs     hi     tuvw / xy |  |  |  |
| `demo.nes` | manual | screen (no documented verdict): EXTRA SPACE AT THE TOP! / VIT     BLAH BLAH ETC. ETC. / EXP       ICONS GO HERE / `abc            defg / pqrs     hi     tuvw / xy |  |  |  |
| `fceuxd.nes` | manual | nothing readable on screen; read the PNG of the final frame |  |  |  |
| `firefly.nes` | manual | screen (no documented verdict): AB AB AB AB AB AB AB AB AB AB A / CDECDECDECDECDECDECDECDECDECDECD / B AB AB AB AB AB AB AB AB AB AB / ECDECDECDECDECDECDECDECDECDECDEC / AB AB AB AB AB AB AB AB AB AB AB / DECDECDECDECDECDECDECDECDECDECDE / AB AB AB AB AB AB AB AB AB AB A / CDECDECDECDECDECDECDECDECDECDECD / B AB AB AB AB AB AB AB AB AB AB / ECDECDECDECDECDECDECDECDECDECDEC / AB AB AB AB AB AB AB A… |  |  |  |
| `high-hopes.nes` | manual | screen (no documented verdict): !"#$%&'(%)*+ / ,-./0..12..34..56789:; / <=>?@ABCDEFGHIJKHLMNOP / QRSTUVWXYZ[\]^V_`abcde / fghijhhklmhnohhpqrhstu / vwwwwwwxywwwwwz{/}ww~ |  |  |  |
| `logo (E).nes` | manual | screen (no documented verdict): (08@HPX`hpx / !)19AIQYaiqy / "*2:BJRZbjrz / #+3;CKS[cks{ / $,4<DLT\dlt/ / %-5=EMU]emu} / &.6>FNV^fnv~ / '/7?GOW_gow |  |  |  |
| `manhole.nes` | manual | screen (no documented verdict): @AB / CD E GHIJ / KLM  NOPQRSTUb / VWXYZ[ \ ]^___`ab / cde   fgh ij_kl__mn / opqrstuvwxyz{/}~___ / _ / _ _         _ / _ _         __ / ____ / ____ / #$%& ()*    +,-.     7 89 / :;<=>                ? / _012 '   / / 3   456 / F |  |  |  |
| `max-300.nes` | manual | screen (no documented verdict): ```````````````````````````````` / ```````````````````````````````` / ```````````````````````````````` / ```````````````````````````````` / ```````````````````````````````` / ```````````````````````````````` / ```````````````````````````````` / ```````````````````````````````` / `WHO`IS`THE`ONE`TRUE`WHORE ````` / `THAT`WOULD`BE`BILL`GATESn`````` / ``````````````````… |  |  |  |
| `midscanline.nes` | manual | screen (no documented verdict): RTG S  UDST / BC / RS / $% / 45 / FG / VW / DE / TU / RTG  TGD / RC  PHR TDS / RD  TT BJHMF / US | PASS | PASS |  |
| `minipack.nes` | manual | screen (no documented verdict): MINIGAME / BOXBOY / ESCAPE FROM PONG / MUNCHIE ATTACK / BOMBER  K / GALAXY PATROL |  |  |  |
| `nescafe.nes` | manual | screen (no documented verdict): !"#$%&'()*+,-./0123456789:;<=>? / @ABCDEFGHIJKLMNOPQRSTUVWXYZ[\]^_ / `abcdefghijklmnopqrstuvwxyz{/}~ |  |  |  |
| `nestest.nes` | manual | screen (no documented verdict): * -- Run all tests / -- Branch tests / -- Flag tests / -- Immediate tests / -- Implied tests / -- Stack tests / -- Accumulator tests / -- (Indirect,X) tests / -- Zeropage tests / -- Absolute tests / -- (Indirect),Y tests / -- Absolute,Y tests / -- Zeropage,X tests / -- Absolute,X tests / Up/Down: select test / Start: run test / Select: Invalid ops! | PASS | PASS |  |
| `nestopia.nes` | manual | nothing readable on screen; read the PNG of the final frame |  |  |  |
| `new-game.nes` | manual | screen (no documented verdict): CDCDCDCDCDCDCDCDCDCDCDCDCDCDCDCD / EFEFEFEFEFEFEFEFEFEFEFEFEFEFEFEF / CDGHGHGHGHGHGHGHGHGHGHGHGHGHGHCD / EFIJIJIJIJIJIJIJIJIJIJIJIJIJIJEF / CDGHCDBK                CDBKGHCD / EFIJEFKK                EFKKIJEF / CDGHGHAABBBBBBKBBBBBBBBBGHAAGHCD / EFIJIJLAKKKKKKBKKKKKKKKKIJLAIJEF / CDGH  CDMMMMCDKBCDCDCDCDCD  GHCD / EFIJ  EFMMMMEFBKEFEFEFEFEF  IJEF / CDGH  CDMMMMCDKBMM… |  |  |  |
| `nintendulator.nes` | manual | nothing readable on screen; read the PNG of the final frame |  |  |  |
| `oam3.nes` | manual | screen (no documented verdict): ................................ / ................................ / ................................ / ................................ / ................................ / ................................ / ................................ / ................................ / ................................ / ................................ / ..................… |  |  |  |
| `oc.nes` | manual | screen (no documented verdict): @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@ / @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@ / @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@ / @@   @         @    @@@@@@@@@@@@ / @@   A         A    @@@@@@@@@@@@ / @@    \@d`bj@      @       @@@@@ / @@    ]AeackA      A       @@@@@ / @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@ / @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@ / @@      ^    t@@bbf\l@@@@@@@@@@@ / @@      _    u@@cc… |  |  |  |
| `physics.0.1.nes` | manual | nothing readable on screen; read the PNG of the final frame |  |  |  |
| `pulsar.nes` | manual | screen (no documented verdict): 00 / A ``00``00``00``00        00 / B 00 00 00 00 00 13 00 00 00 / C ``00``00``00``00 / D ``00``00``00``00 / E ``00``00``00``00 / ``      q00 r33 / ``      q00 r33 / ``  C 4 q00 r33 / ``      q00 r33 / ``  C 4 q00 r33 / ``      q00 r33 / ``      q00 r33 / ``      q00 r33 / ``      q00 r33 / ``      q00 r33 / ``  C 4 q00 r33 / ``      q00 r33 / ``      q00 r33 / ``  … |  |  |  |
| `quantum_disco_brothers_by_wAMMA.nes` | manual | screen (no documented verdict): de  `a`a  hi  de / tu  pqpq  xy  tu / `a  LM    `a  bc / pq  \]    pq  rs / HI        JKhi          noNO / XY        Z[xy          ~ ^_ / JK  hi  dehi  de    fg`a / Z[  xy  tuxy  tu    vwpq / dehibc      bc / tuxyrs      rs / deHInoHILM  hiLM  NOhi  LM / tuXY~ XY\]  xy\]  ^_xy  \] / de`a          HI  NO  NOHI  LM`a / tupq          XY  ^_  ^_XY  \]pq |  |  |  |
| `rastesam4.nes` | manual | screen (no documented verdict): AAB      CAAAAAAAAAAAAAAAAAAAAAA / AB        CAAAAAAAAAAAAAAAAAAAAA / B          CAAAAAAAAAAAAAAAAAAAA / CAAAAAAAAAAAAAAAAAAB / CAAAAAAAAAAAAAAAAB / CAAAAAAAAAAAAAAB / CAAAAAAAAAAAAB / ]             CAAAAAAAAAAB / NN              CAAAAAAAAB    N / CAAAAAAB / CAAAAB / CAAB / CB |  |  |  |
| `read2004.nes` | manual | screen (no documented verdict): __ __ __ __ __ __ __ __ __ __ / __ __ ZZ ZZ ZZ ZZ ZZ ZZ ZZ ZZ / ZZ ZZ ZZ ZZ ZZ ZZ ZZ ZZ ZZ ZZ / ZZ ZZ ZZ ZZ ZZ ZZ ZZ ZZ ZZ ZZ / ZZ ZZ ZZ ZZ ZZ ZZ ZZ ZZ ZZ ZZ / ZZ ZZ ZZ ZZ ZZ ZZ ZZ ZZ ZZ ZZ / ZZ ZZ ZZ ZZ ZZ ZZ ZZ ZZ ZZ ZZ / ZZ ZZ ZZ ZZ ZZ ZZ ZZ ZZ ZZ ZZ / ZZ ZZ ZZ ZZ ZZ ZZ ZZ ZZ ZZ ZZ / ZZ ZZ ZZ ZZ ZZ ZZ ZZ ZZ ZZ ZZ / ZZ ZZ ZZ ZZ ZZ ZZ ZZ ZZ ZZ ZZ / ZZ ZZ ZZ ZZ ZZ Z… | FAIL |  |  |
| `snow.nes` | manual | screen (no documented verdict): G / QTUVW           H       H YZ[^ / PRS G       H                \]_ / d           G       F   H / G     F    IJ                 G / H KL   H        G / G               H         H / G       H / H         F    G    H         3 / H                  H   4 / G     56 / G    F      H    F         78 / 3=                      9: / 4>   &     &  $         ;< / ,/   56?  … |  |  |  |
| `test001.nes` | manual | nothing readable on screen; read the PNG of the final frame |  |  |  |
| `test28.nes` | manual | screen (no documented verdict): B                 d / C                 e / @      @   @ @ / A      A   A A / \@@  @      @ / ]AA  A      A / @   @   @ / A   A   A / @      X@   @  @   N \ / A      YA   A  A   O ] |  |  |  |
| `window2_ntsc.nes` | manual | screen (no documented verdict): F AAAAAAAAAAAAAAAAAAAAAAAAAAABFF / FC___________________________CFF / FC_3'(2_(2_ _#4,,8_3$73_!.7__CFF / FC___________________________CFF / FC_(2_3' 3_! "*&1.4-#_-("$^__CFF / FDAAAAAAAAAAAAAAAAAAAAAAAAAAAEFF |  |  |  |
| `window2_pal.nes` | manual | screen (no documented verdict): F AAAAAAAAAAAAAAAAAAAAAAAAAAABFF / FC___________________________CFF / FC_3'(2_(2_ _#4,,8_3$73_!.7__CFF / FC___________________________CFF / FC_(2_3' 3_! "*&1.4-#_-("$^__CFF / FDAAAAAAAAAAAAAAAAAAAAAAAAAAAEFF |  |  |  |
| `window_old_ntsc.nes` | manual | screen (no documented verdict): FF AAAAAAAAAAAAAAAAAAAAAAAABFFFF / FFC________________________CFFFF / FFC3'(2_(2_ _#4,,8_3$73_!.7CFFFF / FFC________________________CFFFF / FFC(2_3' 3_! "*&1.4-#_-("$^CFFFF / FFDAAAAAAAAAAAAAAAAAAAAAAAAEFFFF |  |  |  |
| `window_old_pal.nes` | manual | screen (no documented verdict): FF AAAAAAAAAAAAAAAAAAAAAAAABFFFF / FFC________________________CFFFF / FFC3'(2_(2_ _#4,,8_3$73_!.7CFFFF / FFC________________________CFFFF / FFC(2_3' 3_! "*&1.4-#_-("$^CFFFF / FFDAAAAAAAAAAAAAAAAAAAAAAAAEFFFF |  |  |  |

### `nes-test-roms/pal_apu_tests`

**3 passed of 10** (7 fail)

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `01.len_ctr.nes` | pass | screen: APU LENGTH COUNTER / PASSED | PASS | PASS |  |
| `02.len_table.nes` | pass | screen: APU LENGTH TABLE / PASSED | PASS | PASS |  |
| `03.irq_flag.nes` | pass | screen: APU FRAME IRQ FLAG / PASSED | PASS | PASS |  |
| `04.clock_jitter.nes` | fail | screen: APU CLOCK JITTER / FAILED: #2 | PASS | PASS |  |
| `05.len_timing_mode0.nes` | fail | screen: APU MODE 0 LENGTH TIMING / FAILED: #4 | PASS | PASS |  |
| `06.len_timing_mode1.nes` | fail | screen: APU MODE 1 LENGTH TIMING / FAILED: #5 | PASS | PASS |  |
| `07.irq_flag_timing.nes` | fail | screen: APU FRAME IRQ FLAG TIMING / FAILED: #2 | PASS | PASS |  |
| `08.irq_timing.nes` | fail | screen: APU FRAME IRQ TIMING / FAILED: #2 | PASS | PASS |  |
| `10.len_halt_timing.nes` | fail | screen: APU LENGTH HALT TIMING / FAILED: #5 | PASS | PASS |  |
| `11.len_reload_timing.nes` | fail | screen: APU LENGTH RELOAD TIMING / FAILED: #3 | PASS | PASS |  |

### `nes-test-roms/ppu_open_bus`

**1 passed of 1**

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `ppu_open_bus.nes` | pass | ppu_open_bus / Passed | PASS | FAIL |  |

### `nes-test-roms/ppu_read_buffer`

**1 passed of 1**

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `test_ppu_read_buffer.nes` | pass | Contemplate on the art while / the test is in progress. / Passed |  |  |  |

### `nes-test-roms/ppu_vbl_nmi`

**11 passed of 11**

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `ppu_vbl_nmi.nes` | pass | 10-even_odd_timing / Passed / All 10 tests passed | FAIL | FAIL |  |
| `01-vbl_basics.nes` | pass | 01-vbl_basics / Passed | PASS | PASS |  |
| `02-vbl_set_time.nes` | pass | 08 V - / 02-vbl_set_time / Passed | PASS | PASS |  |
| `03-vbl_clear_time.nes` | pass | 08 - / 03-vbl_clear_time / Passed | PASS | PASS |  |
| `04-nmi_control.nes` | pass | 04-nmi_control / Passed | PASS | PASS |  |
| `05-nmi_timing.nes` | pass | 09 2 / 05-nmi_timing / Passed | PASS | PASS |  |
| `06-suppression.nes` | pass | 09 V N / 06-suppression / Passed | PASS | PASS |  |
| `07-nmi_on_timing.nes` | pass | 08 - / 07-nmi_on_timing / Passed | PASS | FAIL |  |
| `08-nmi_off_timing.nes` | pass | 0C N / 08-nmi_off_timing / Passed | PASS | PASS |  |
| `09-even_odd_frames.nes` | pass | 00 01 01 02 / 09-even_odd_frames / Passed | PASS | PASS |  |
| `10-even_odd_timing.nes` | pass | 08 08 09 07 / 10-even_odd_timing / Passed | FAIL | PASS |  |

### `nes-test-roms/read_joy3`

**1 passed of 4** (3 manual)

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `count_errors.nes` | manual | screen (no documented verdict): ------------------------------ / ------------------------------ / ------------------------------ / ------------------------------ / ------------------------------ / ------------------------------ / ------------------------------ / ------------------------------ / ------------------------------ / ------------------------------ / ------------------------------ / -----… | FAIL | FAIL |  |
| `count_errors_fast.nes` | manual | screen (no documented verdict): ------------------------------ / ------------------------------ / ------------------------------ / ------------------------------ / ------------------------------ / ------------------------------ / ------------------------------ / ------------------------------ / ------------------------------ / ------------------------------ / ------------------------------ / -----… | FAIL | FAIL |  |
| `test_buttons.nes` | manual | screen (no documented verdict): Press indicated buttons / A | PASS | PASS |  |
| `thorough_test.nes` | pass | screen: thorough_test / Passed | PASS | PASS |  |

### `nes-test-roms/scanline`

**0 passed of 1** (1 manual)

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `scanline.nes` | manual | screen (no documented verdict): This is a test for    / Stars / proper mid-scanline   / Below / PPU write emulation   / Denote / / Errors / This first area       / ****** / toggles D3 of $2001   / ****** / to toggle background  / ****** / rendering at the      / ****** / appropriate times     / ****** / and locations.        / ****** / This second area      / ****** / toggles D4 of $2000   / *****… | PASS | PASS |  |

### `nes-test-roms/scanline-a1`

**0 passed of 1** (1 manual)

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `scanline.nes` | manual | screen (no documented verdict): This is a test for    / Stars / proper mid-scanline   / Below / PPU write emulation   / Denote / / Errors / This first area       / ****** / toggles D3 of $2001   / ****** / to toggle background  / ****** / rendering at the      / ****** / appropriate times     / ****** / and locations.        / ****** / This second area      / ****** / toggles D4 of $2000   / *****… |  |  |  |

### `nes-test-roms/scrolltest`

**0 passed of 1** (1 manual)

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `scroll.nes` | manual | screen (no documented verdict): ]MULTIDIRECTIONAL SCROLLING TEST / ^RUNNING IN NTSC, 60HZ REFRESH / _<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<> | PASS | PASS |  |

### `nes-test-roms/soundtest`

**0 passed of 1** (1 manual)

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `SNDTEST.NES` | manual | screen (no documented verdict): CHANNEL* / REG  * / REG !* / REG "* / REG #* |  |  |  |

### `nes-test-roms/sprdma_and_dmc_dma`

**0 passed of 2** (2 fail)

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `sprdma_and_dmc_dma.nes` | fail | B8EA17D9 / SPRDMA and DMC DMA / Failed (code 1) | PASS | FAIL |  |
| `sprdma_and_dmc_dma_512.nes` | fail | B8EA17D9 / SPRDMA and DMC DMA / Failed (code 1) | PASS | FAIL |  |

### `nes-test-roms/sprite_hit_tests_2005.10.05`

**11 passed of 11**

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `01.basics.nes` | pass | screen: SPRITE HIT BASICS / PASSED | PASS | PASS |  |
| `02.alignment.nes` | pass | screen: SPRITE HIT ALIGNMENT / PASSED | PASS | PASS |  |
| `03.corners.nes` | pass | screen: SPRITE HIT CORNERS / PASSED | PASS | PASS |  |
| `04.flip.nes` | pass | screen: SPRITE HIT FLIPPING / PASSED | PASS | PASS |  |
| `05.left_clip.nes` | pass | screen: SPRITE HIT LEFT CLIPPING / PASSED | PASS | PASS |  |
| `06.right_edge.nes` | pass | screen: SPRITE HIT RIGHT EDGE / PASSED | PASS | PASS |  |
| `07.screen_bottom.nes` | pass | screen: SPRITE HIT SCREEN BOTTOM / PASSED | PASS | PASS |  |
| `08.double_height.nes` | pass | screen: SPRITE HIT DOUBLE HEIGHT / PASSED | PASS | PASS |  |
| `09.timing_basics.nes` | pass | screen: SPRITE HIT TIMING / PASSED | PASS | PASS |  |
| `10.timing_order.nes` | pass | screen: SPRITE HIT ORDER / PASSED | PASS | PASS |  |
| `11.edge_timing.nes` | pass | screen: SPRITE HIT EDGE TIMING / PASSED | PASS | PASS |  |

### `nes-test-roms/sprite_overflow_tests`

**5 passed of 5**

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `1.Basics.nes` | pass | screen: SPRITE OVERFLOW BASICS / PASSED | PASS | PASS |  |
| `2.Details.nes` | pass | screen: SPRITE OVERFLOW DETAILS / PASSED | PASS | PASS |  |
| `3.Timing.nes` | pass | screen: SPRITE OVERFLOW TIMING / PASSED | PASS | PASS |  |
| `4.Obscure.nes` | pass | screen: SPRITE OVERFLOW OBSCURE / PASSED | PASS | PASS |  |
| `5.Emulator.nes` | pass | screen: SPRITE OVERFLOW EMULATION / PASSED | PASS | PASS |  |

### `nes-test-roms/spritecans-2011`

**0 passed of 1** (1 manual)

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `spritecans.nes` | manual | screen (no documented verdict): P Q R S  S P T Q  U V _Q X U / Y Z [ W  W Y Y \  W ] _\ ^ W / 2000-2011 |  |  |  |

### `nes-test-roms/stars_se`

**0 passed of 1** (1 manual)

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `StarsSE.NES` | manual | screen (no documented verdict): DDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDD / EEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEE / FGHIJKLMNO / VWXYZ[\]^_ / QRSTU / DDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDD / EEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEE / DDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDD / EEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEE |  |  |  |

### `nes-test-roms/stomper`

**0 passed of 1** (1 manual)

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `smwstomp.nes` | manual | screen (no documented verdict): NNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNN / NNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNN / NNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNN / NNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNN / LMLMLMLMLMLMLMLMLMLMLMLMLMLMLMLM / IJIJIJIJIJIJIJIJIJIJIJIJIJIJIJIJ / KKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK / KKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK / KKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK / KKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK | PASS | PASS |  |

### `nes-test-roms/stress`

**0 passed of 1** (1 manual)

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `NEStress.NES` | manual | screen (no documented verdict): 01234      First line      78901 / 0              I               1 / AAAD      AAAD  AAAAAAAAAA    1 / A  D      A  D  A        D    1 / A  D AAAD A  D  A   DA   D    1 / A  DA    DA  D  A        D    1 / A  D      A  D  A  DDDD  D    1 / A    DDDA    D  A  D  A  D    1 / A   D    A   D  A  D  A  D    1 / ADDD      AAAD  ADDD  ADDD    1 / 0          000000000000000… |  |  |  |

### `nes-test-roms/tutor`

**0 passed of 1** (1 manual)

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `tutor.nes` | manual | screen (no documented verdict): 12345678901234567890123456789012 / PRESENTING NES 101: / A GUIDE FOR OTHERWISE / EXPERIENCED PROGRAMMERS / TUTORIAL FILE BY / MICHAEL MARTIN / PRESS UP AND DOWN TO SHIFT / THE SPRITE / PRESS A TO REVERSE DIRECTION / CHARACTER SET HIJACKED FROM / COMMODORE BUSINESS MACHINES / (C64'S CHARACTER ROM) / READY. |  |  |  |

### `nes-test-roms/tvpassfail`

**0 passed of 1** (1 manual)

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `tv.nes` | manual | screen (no documented verdict): WARNING / BEFORE PLAYING, READ / THE HEALTH AND SAFETY / PRECAUTIONS BOOKLET FOR / IMPORTANT INFORMATION / ABOUT YOUR HEALTH AND / SAFETY. / THIS GOES DOUBLE FOR / PEOPLE WITH EPILEPSY. / PRESS START BUTTON |  |  |  |

### `nes-test-roms/vaus-test`

**0 passed of 1** (1 manual)

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `vaus-test.nes` | manual | screen (no documented verdict): LM                        LM / NO                        NO / LM                        LM / NO                        NO / KKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK / AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA / AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA / AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA / AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA / AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA |  |  |  |

### `nes-test-roms/vbl_nmi_timing`

**7 passed of 7**

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `1.frame_basics.nes` | pass | screen: PPU FRAME BASICS / PASSED | PASS | PASS |  |
| `2.vbl_timing.nes` | pass | screen: VBL TIMING / PASSED | PASS | PASS |  |
| `3.even_odd_frames.nes` | pass | screen: EVEN ODD FRAMES / PASSED | PASS | PASS |  |
| `4.vbl_clear_timing.nes` | pass | screen: VBL CLEAR TIMING / PASSED | PASS | PASS |  |
| `5.nmi_suppression.nes` | pass | screen: NMI SUPPRESSION / PASSED | PASS | PASS |  |
| `6.nmi_disable.nes` | pass | screen: NMI DISABLE / PASSED | PASS | PASS |  |
| `7.nmi_timing.nes` | pass | screen: NMI TIMING / PASSED | PASS | PASS |  |

### `nes-test-roms/volume_tests`

**0 passed of 1** (1 manual)

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `volumes.nes` | manual | nothing readable on screen; read the PNG of the final frame |  |  |  |

### `nes-test-roms/window5`

**0 passed of 2** (2 manual)

| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |
|---|---|---|---|---|---|
| `colorwin_ntsc.nes` | manual | screen (no documented verdict): F AAAAAAAAAAAAAAAAAAAAAAAAAAAABF / FC____________________________CF / FC__3'(2_(2_ _#4,,8_3$73_!.7__CF / FC____________________________CF / FC__(2_3' 3_! "*&1.4-#_-("$^__CF / FDAAAAAAAAAAAAAAAAAAAAAAAAAAAAEF |  |  |  |
| `colorwin_pal.nes` | manual | screen (no documented verdict): F AAAAAAAAAAAAAAAAAAAAAAAAAAAABF / FC____________________________CF / FC__3'(2_(2_ _#4,,8_3$73_!.7__CF / FC____________________________CF / FC__(2_3' 3_! "*&1.4-#_-("$^__CF / FDAAAAAAAAAAAAAAAAAAAAAAAAAAAAEF |  |  |  |


---

The reference columns are quoted from `nes-test-roms/status.txt`, the collection's own result table, as it ships with the ROMs ([source](https://github.com/christopherpow/nes-test-roms/blob/master/status.txt)). A blank cell means that file does not list the ROM. There is no Mesen 2 column: no published table covers these ROMs row by row, and a guessed column would be worse than none.

---

## Re-run after lot MF-2 (2026-11-04)

Lot MF-2 (`docs/progress/MF-2.md`) changed seven boards and the APU's mixer, so the suites that
could see it were re-run with `zax-harness test-roms --filter`. **Every one of them matches the
campaign above exactly**; no row moved.

| suite | before (this report) | after MF-2 | why it was re-run |
| --- | --- | --- | --- |
| `holy-mapperel` | 32 / 40, 2 fail, 6 unsupported | **32 / 40, 2 fail, 6 unsupported** | its `M11_*`, `M34_*` and `M66_*` boards gained unconditional bus conflicts (MR-BUS-1, MR-66-1, MR-34-2), and its `M1_*`, `M2_*`, `M3_*` boards are the submapper-gated ones |
| `nes-test-roms/mmc3_irq_tests` | 5 / 6, 1 fail | **5 / 6, 1 fail** | the revision A / B gate of AMD-061 §8 (Q-114) |
| `nes-test-roms/mmc3_test` | 5 / 6, 1 fail | **5 / 6, 1 fail** | same |
| `nes-test-roms/mmc3_test_2` | 5 / 6, 1 fail | **5 / 6, 1 fail** | same; test 6 is REQ-CORE-114 and stays required for revision B |
| `nes-test-roms/mmc5test` | 0 / 1, 1 manual | **0 / 1, 1 manual** | the MMC5 audio of AMD-061 §6 and the ExRAM write gate (MR-5-2) |
| `nes-test-roms/mmc5test_v2` | 0 / 1, 1 manual | **0 / 1, 1 manual** | same |
| `nes-test-roms/exram` | 0 / 1, 1 manual | **0 / 1, 1 manual** | MR-5-2 |
| `nes-test-roms/apu_mixer` | 4 / 4 | **4 / 4** | the mixer's polarity and signed clip (AMD-061 §6.2) |
| `nes-test-roms/apu_test` | 9 / 9 | **9 / 9** | same |
| `nes-test-roms/pal_apu_tests` | 3 / 10, 7 fail | **3 / 10, 7 fail** | same |
| `nes-test-roms/m22chrbankingtest` | 0 / 1, 1 manual | **0 / 1, 1 manual** | mapper 22 is a VRC2a, re-checked after step 5 touched the shared A12 watcher |

The three MMC3 revision-A rows that **are required to fail** (`5.MMC3_rev_A.nes`,
`mmc3_test/6-MMC6.nes`, `mmc3_test_2/rom_singles/6-MMC3_alt.nes`) still fail, and that is still
right: AMD-061 §8 gives revision A to a **declared NES 2.0 submapper 4** only, and all three of
those ROMs are iNES 1.0 images, which keep revision B. Their `5-MMC3` / `6.MMC3_rev_B`
counterparts still pass.

**Three suites a reader might expect here are absent because the ROMs do not exist**: nothing in
`assets-free/test-roms` drives **mapper 64** (RAMBO-1) or **mapper 18** (Jaleco SS88006) at all —
`holy-mapperel` has no `M64_*` or `M18_*` image — so steps 5 and 9 have no ROM-level evidence and
rest on the NESdev pages alone. The campaign's mapper coverage is `holy-mapperel`'s board list
plus the MMC1/MMC3/MMC5 suites.

## What lot MF-3 changed

MF-T diagnosed; **MF-3 fixed the mapper half of its findings** (`docs/progress/MF-3.md`). Eight
rows moved, all of them in `holy-mapperel`, which now passes in full. Re-measured with

```
zax-harness test-roms <roms dir> out.json --filter holy-mapperel --no-shots
zax-harness test-roms <roms dir> out.json --filter mmc3_test --no-shots
zax-harness test-roms <roms dir> out.json --filter mmc3_irq_tests --no-shots
```

| ROM | before MF-3 | after | what changed |
|---|---|---|---|
| `holy-mapperel/testroms/M1_P128K.nes` | **fail**, detailed `4000` | **pass**, detailed `0000` | step 1: on an SNROM, bit 4 of the CHR bank register is a second chip enable for the PRG-RAM (nesdev, "MMC1"). The board is identified at load from the three sizes of the board page's table |
| `holy-mapperel/testroms/M1_P128K_CR8K.nes` | **fail**, detailed `4000` | **pass**, detailed `0000` | the same ROM under another name — the two files have identical headers |
| `holy-mapperel/testroms/M69_P128K_C64K_S8K.nes` | unsupported | **pass**, `J*ROM (FME-7)`, detailed `0000` | step 3: mapper 69, the Sunsoft FME-7, with its CPU-cycle IRQ counter and its `$6000` RAM/ROM switching |
| `holy-mapperel/testroms/M69_P128K_C64K_W8K.nes` | unsupported | **pass**, `J*ROM (FME-7)`, detailed `0000` | the same board without the battery |
| `holy-mapperel/testroms/M118_P128K_C64K.nes` | unsupported | **pass**, `T*SROM (MMC3)`, detailed `0000` | step 5: mapper 118, the TxSROM, a variant of the MMC3 board chosen at load |
| `holy-mapperel/testroms/M180_P128K_H.nes` | unsupported | **pass**, `U*ROM (7408)`, detailed `0000` | step 6: mapper 180, the UxROM board with its fixed bank at the other end |
| `holy-mapperel/testroms/M180_P128K_CR8K_H.nes` | unsupported | **pass**, `U*ROM (7408)`, detailed `0000` | the same board with its CHR-RAM declared |
| `holy-mapperel/testroms/M78.3_P128K_C64K.nes` | unsupported | **pass**, `HOLY DIVER`, detailed `0000` | step 7: mapper 78, both wirings, with the iNES-1.0 heuristic for an image that names no submapper |

Note that **all eight pass with `detailed 0000`**, the third digit included: that digit is the IRQ
test, which the suite's README says "measures only gross behavior of the interval timer of the
MMC3 or FME-7", so the FME-7's counter is exercised and not merely present.

### The suites that did not move, and were checked

| suite | before | after |
|---|---|---|
| `nes-test-roms/mmc3_test` | 5 / 6, `6-MMC6` **required to fail** | unchanged |
| `nes-test-roms/mmc3_test_2` | 5 / 6, `6-MMC3_alt` **required to fail** | unchanged |
| `nes-test-roms/mmc3_irq_tests` | 5 / 6, `5.MMC3_rev_A` **required to fail** | unchanged |
| `nes-test-roms/MMC1_A12` | manual | unchanged |

The three required failures are still failing, which is the point: REQ-CORE-100 pins mapper 4 to
revision B and AMD-061's answer to Q-114 restates it, so each of those three is the revision A half
of a contradictory pair whose revision B half passes. MF-3 added the MC-ACC (mapper 4 submapper 3)
and the TxSROM (mapper 118) to that board without moving any of them: an iNES 1.0 mapper-4 image,
which is what all three suites are, is still a plain revision B MMC3.

`nes-test-roms/other/LINUSMUS.NES` is the one `unsupported` row left in the whole campaign, and it
is not a mapper: "Corrupt header: missing iNES signature".

### The regression guard

`rust/crates/zax-harness/tests/test_roms_tests.rs` asserts the outcome of every ROM of three
groups, failures included, when `ZAXNES_TEST_ROMS` points at a copy of `assets-free/test-roms/`.
MF-3 flipped its eight `holy-mapperel` expectations and left the three required MMC3 failures
alone, so all forty `holy-mapperel` rows now read `Outcome::Pass` there.
