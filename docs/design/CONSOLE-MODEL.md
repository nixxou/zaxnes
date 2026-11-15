# CONSOLE-MODEL — Fidélité+ part C (design and progress, 2026-11-15)

Part C of the Fidélité+ goal (`docs/design/HALF-CYCLE.md`, "The goal"): every hardware variant
selectable and consistent, so that each test ROM passes under the console model it was written
for. Common base by the owner's placement rule: choosing a variant costs nothing per cycle.

## Judge

The four rows the default campaign fails exclude each other, or name one console:

| Row | What it needs |
|---|---|
| `blargg_ppu_tests_2005.09.15b/power_up_palette` | blargg's console's palette at power-on (his readme: "probably unique to my NES") |
| `mmc3_irq_tests/5.MMC3_rev_A` | the MMC3 revision A counter (NEC / Sharp MMC3A) |
| `mmc3_test/6-MMC6` | the MMC6's counter, which is revision A's |
| `mmc3_test_2/rom_singles/6-MMC3_alt` | revision A |

Revision A fails `5-MMC3` (both suites) and `6.MMC3_rev_B`: no single model passes all eight.

## C1 (done, 2026-11-15)

* Core: `zax_nes::Mmc3Revision` (`Cartridge`, the header's and the database's, by default; `A`;
  `B`), `NesCore::set_mmc3_revision`, applied to a mapper 4 / 118 board when a ROM is loaded
  (`Cartridge::set_mmc3_irq_revision_a`). The palette at power-on was already a setting
  (`PowerOnPpuRam::Blargg`, lot FIDELITY step 36).
* Harness: `ZAXNES_MMC3_REVISION=a|b`, and `ZAXNES_CONSOLE_MODEL=rom`, which runs each of the four
  rows under its own console (`console_model_of`). Default campaign unchanged: 215 pass, 4 fail.
* Result: with `ZAXNES_CONSOLE_MODEL=rom`, **331 ROMs, 219 pass, 0 fail** (1 unsupported, 111
  manual). With `ZAXNES_MMC3_REVISION=a` alone, the three revision A rows pass and the three
  revision B rows fail, as on the chips.

## Next

* C2 (done): the application's setting `emu.mmc3_revision` ("MMC3 revision": the cartridge's,
  MMC3A / MMC6, MMC3B / MMC3C), next to the PPU revision, read at the next load.
* C3a (done): the letterless RP2A03 as a choice of its own, `emu.cpu_revision` ("CPU revision":
  RP2A03G and later, letterless RP2A03), `NesCore::set_letterless_cpu`, harness
  `ZAXNES_LETTERLESS_CPU=1`; the noise channel takes the letterless chip when this or the audio
  profile says so (FIDELITY-LEADS G4: its own period table, no short mode). Default campaign and
  golden unchanged. The console-model menu's presets do not set it: no reference ties it to one of
  the menu's models (the early Famicoms are not a model of the menu).
* C3b (state, 2026-11-15): a random CPU/PPU alignment and random power-on memories are already
  choices (`emu.cpu_ppu_alignment` "Random", `emu.power_on_ram` and `emu.power_on_ppu_ram`
  "Random", lot FIDELITY step 36), drawn at each load as a console draws them at power-on.
* C3c (blocked): the PAL 2C07 measured as the 2C02 was needs a 2C07 netlist; only the 2C02's
  (`ext-tools/visualnes-src/Core/segdefs.txt`) is here. Getting one is a download, gated by the
  owner. The 2A03 CPU netlist is here too (`cpusegdefs.txt`), which lot A-bis's bench would use.
* C3 (the goal's list, for the record): RP2A03 letter revisions (the DMC/OAM DMA and `$4015` read
  differences), the PAL 2C07 measured as the 2C02 was, random CPU/PPU alignment and RAM at power-on
  as a model choice; each with the test ROM or netlist that judges it.
