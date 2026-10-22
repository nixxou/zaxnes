# Public NES test ROMs

This folder holds the **public** test ROMs the automated suite uses (SPEC-02 §3.12,
REQ-ACC-001). Nothing here is produced by this project and **no file is committed**: the
repository's `.gitignore` keeps the binaries out, and each test that needs one reports
*skipped*, naming the missing file, when it is absent (`RomFactAttribute`).

Commercial game ROMs are **never** placed here. The reference games of REQ-ACC-020 and the
101 titles of REQ-CORE-115 are supplied by the user from dumps they legally own
(SPEC-12 §2).

## How to populate the folder

Everything below comes from the public repository
<https://github.com/christopherpow/nes-test-roms>, which collects the test ROMs published
by their authors (mainly Shay "blargg" Green and Kevin Horton) together with their
documentation. The nesdev wiki page "Emulator tests" links to the same material.

```sh
# From impl/tests/roms
git clone --depth 1 https://github.com/christopherpow/nes-test-roms.git _src

# Lot L1a — CPU (REQ-CORE-110)
cp _src/other/nestest.nes .
cp _src/other/nestest.log .

# Lot L1b — CPU branch timing and the PPU (REQ-CORE-111, 112, 116)
cp -r _src/branch_timing_tests _src/sprite_hit_tests_2005.10.05       _src/sprite_overflow_tests _src/blargg_ppu_tests_2005.09.15b       _src/vbl_nmi_timing _src/ppu_vbl_nmi _src/ppu_open_bus       _src/ppu_read_buffer _src/oam_read .

# Lot L1c — APU (REQ-CORE-113, and the DMA parts of REQ-CORE-111)
cp -r _src/apu_test _src/blargg_apu_2005.07.30 _src/apu_reset       _src/dmc_dma_during_read4 _src/sprdma_and_dmc_dma _src/cpu_interrupts_v2 .

# Lot L1d — mappers (REQ-CORE-100, 110, 111, 114) and the homebrew that boots them
cp -r _src/mmc3_test_2 _src/mmc3_test _src/mmc3_irq_tests _src/instr_test-v5       _src/instr_timing _src/instr_misc _src/cpu_timing_test6 _src/cpu_dummy_reads       _src/MMC1_A12 _src/m22chrbankingtest _src/mmc5test _src/mmc5test_v2 _src/exram       _src/240pee _src/nes15-1.0.0 .
cp _src/other/test28.nes _src/other/Streemerz_bundle.nes _src/other/fceuxd.nes       _src/other/oam3.nes _src/other/firefly.nes _src/other/BladeBuster.nes       _src/other/quantum_disco_brothers_by_wAMMA.nes other/
```

The CPU trace comparison needs **both** files: `nestest.nes` and the reference log
`nestest.log` that ships beside it. `ppu_vbl_nmi` is used through its
`rom_singles/` subfolder, which the clone brings along.

## Files the suite looks for

| File | Used by | Requirement | State |
|---|---|---|---|
| `nestest.nes` + `nestest.log` | `NestestTests` | REQ-CORE-110 | passes (all 8 991 lines) |
| `branch_timing_tests/*.nes` (3) | `PpuRomTests` | REQ-CORE-111 | passes |
| `sprite_hit_tests_2005.10.05/*.nes` (11) | `PpuRomTests` | REQ-CORE-112 | passes |
| `blargg_ppu_tests_2005.09.15b/{palette_ram,sprite_ram,vram_access,vbl_clear_time}.nes` | `PpuRomTests` | REQ-CORE-112 | passes |
| `ppu_vbl_nmi/rom_singles/01..03` | `PpuRomTests` | REQ-CORE-112 | passes |
| `ppu_vbl_nmi/rom_singles/04..10` | `PpuRomTests` | REQ-CORE-116 | passes |
| `vbl_nmi_timing/*.nes` (7) | `PpuRomTests` | REQ-CORE-116 | passes |
| `oam_read/oam_read.nes` | `PpuRomTests` | REQ-CORE-116 | passes |
| `ppu_open_bus/ppu_open_bus.nes` | `PpuRomTests` | REQ-CORE-116 | passes |
| `sprite_overflow_tests/*.nes` (5) | `PpuRomTests` | REQ-CORE-116 (P3) | passes |
| `ppu_vbl_nmi/ppu_vbl_nmi.nes` | `MapperRomTests` | REQ-CORE-116 | passes (MMC1) |
| `ppu_read_buffer/test_ppu_read_buffer.nes` | `MapperRomTests` | REQ-CORE-116 | passes (CNROM) |
| `instr_test-v5/all_instrs.nes` | `MapperRomTests` | REQ-CORE-110 | passes (MMC1) |
| `instr_test-v5/rom_singles/01..16` | `MapperRomTests` | REQ-CORE-110 | passes |
| `instr_timing/instr_timing.nes` | `MapperRomTests` | REQ-CORE-111 | passes (MMC1) |
| `instr_misc/instr_misc.nes` | `MapperRomTests` | REQ-CORE-110 | passes (MMC1) |
| `cpu_timing_test6/cpu_timing_test.nes` | `MapperRomTests` | REQ-CORE-111 | passes, B held, verdict read off the screen |
| `apu_test/rom_singles/1..8` | `ApuRomTests` | REQ-CORE-113 | passes |
| `blargg_apu_2005.07.30/01..11` | `ApuRomTests` | REQ-CORE-113 | passes |
| `cpu_interrupts_v2/rom_singles/{1-cli_latency,4-irq_and_dma}.nes` | `ApuRomTests` | REQ-CORE-111 | passes |
| `apu_reset/*.nes` (6) | `ApuRomTests` | REQ-CORE-004 | power-on half passes; reset half pending |
| `cpu_interrupts_v2/rom_singles/{2,3,5}.nes` | `ApuRomPendingTests` | REQ-CORE-111 | pending — CPU interrupt hijacking |
| `dmc_dma_during_read4/*.nes` (5) | `ApuRomPendingTests` | REQ-CORE-113 (P3) | pending — DMA model |
| `sprdma_and_dmc_dma/*.nes` (2) | `ApuRomPendingTests` | — | pending — DMA model |
| `apu_test/apu_test.nes` | `MapperRomTests` | REQ-CORE-113 | passes (MMC1) |
| `mmc3_test_2/rom_singles/1..5` | `MapperRomTests` | REQ-CORE-114 | passes |
| `mmc3_test_2/rom_singles/6-MMC3_alt.nes` | `MapperRomTests` | REQ-CORE-114 | fails with code 2 — revision A rule, allowed |
| `mmc3_test/6-MMC6.nes` | `MapperRomTests` | REQ-CORE-100 | fails — an iNES header cannot name an MMC6 |
| `mmc3_irq_tests/{1,2,3,4,6}.nes` | `MapperRomTests` | REQ-CORE-114 | passes |
| `mmc3_irq_tests/5.MMC3_rev_A.nes` | `MapperRomTests` | REQ-CORE-100 | fails — the counter is revision B |
| `cpu_dummy_reads/cpu_dummy_reads.nes` | `MapperRomTests` | REQ-CORE-100 (CNROM) | passes, verdict read off the screen |
| `MMC1_A12/mmc1_a12.nes`, `240pee/*`, `m22chrbankingtest/0-127.nes`, `mmc5test*/`, `exram/`, `nes15-1.0.0/`, `other/{test28,Streemerz_bundle,fceuxd,oam3,firefly,BladeBuster,quantum_disco_brothers_by_wAMMA}.nes` | `MapperRomTests` | REQ-CORE-100 | boot and draw (mappers 1, 2, 3, 4, 5, 7, 11, 22, 28, 34) |
| `cpu_interrupts_v2/cpu_interrupts.nes` | `ApuRomPendingTests` | REQ-CORE-111 | runs, stops on test 2 — CPU interrupt hijacking |

The mappers of REQ-CORE-100 that no public ROM here covers — 9, 10, 13, 15, 16, 18, 21, 23,
25, 27, 30, 32, 41, 64, 66, 71, 87, 101, 225, 228 — are pinned by the synthetic boards of
`MapperFixture` instead: bank switching vectors, mirroring control, PRG-RAM, IRQ timing and a
savestate round-trip per board (`MapperTests`, `MapperIrqTests`). The acceptance criterion of
REQ-CORE-100 asks for a public ROM per mapper; there is no network access on the clean side, so
only the ten mappers above could be covered that way.

"Pending" means the ROM cannot run yet, and the reason is structural rather than a gap in
the CPU or the PPU:

- `apu_reset` keeps its reset counter in RAM across the reset (`run_at_reset.s`:
  `num_resets_` lives in `nv_ram`), and `4017_written` also needs the last `$4017` value to
  survive. A reset here is a power cycle (REQ-CORE-004, DEC-02-Q3), so only the power-on half
  of each of those six tests can run — it does, and is asserted;
- `dmc_dma_during_read4` and `sprdma_and_dmc_dma` measure what the hardware does when a DMC
  sample fetch collides with a CPU read or with a sprite DMA. The fetch here halts the CPU for
  four cycles on the cycle the APU asks for the byte, which does not reproduce the hardware's
  repeated reads; REQ-CORE-113 makes the first suite P3 and the second is not required at all.
  `dmc_dma_during_read4` has no `$6000` status byte either: its framework reports only on
  screen and over a devcart serial link;
- `cpu_interrupts_v2` tests 2, 3 and 5 measure CPU interrupt sequencing (an NMI hijacking a
  `BRK`, the cycle a taken branch polls /IRQ on) that lot L1a left out and documented; tests 1
  and 4, the IRQ and DMA parts REQ-CORE-111 shares with the APU, pass.

### Result protocols

Two protocols are in use, and `TestRomRunner` reads both without any emulator hook:

- the **shell protocol** of the later suites (`ppu_vbl_nmi`, `oam_read`, `ppu_open_bus`,
  `ppu_read_buffer`): `$6001-$6003` hold `$DE $B0 $61`, `$6000` holds `$80` while the test
  runs and the final code afterwards, 0 meaning "passed", and the printed text is also a
  zero-terminated string at `$6004`;
- the **2005-era protocol** (`branch_timing_tests`, `sprite_hit_tests`,
  `sprite_overflow_tests`, `vbl_nmi_timing`, `blargg_ppu_tests`): a low-memory result
  variable, 1 meaning "passed", read once the CPU has parked in its one-instruction endless
  loop. The shipped support sources put that variable at `$00F8` (`validation.a`:
  `result = $f8`); `blargg_ppu_tests_2005.09.15b` ships no support source and uses `$00F0`,
  which was located by running the four ROMs. `blargg_apu_2005.07.30` ships its support
  sources but not the console, and uses `$00F0` as well, located the same way.

`power_up_palette` is excluded on purpose: the palette starts at 0 (REQ-CORE-112).
`cpu_reset/ram_after_reset` is expected to **fail**, because a reset here is a power cycle
(REQ-CORE-004, DEC-02-Q3).

## Licences

The ROMs of `nes-test-roms` are distributed by their authors as freely redistributable
test material; several carry their own `readme.txt` or `license.txt` next to the binary,
which the clone above brings along. Keep those files next to the ROMs you copy, and consult
them before redistributing anything. This project redistributes none of them.
