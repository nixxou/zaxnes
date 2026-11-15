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
* C3c (unblocked 2026-11-15): the PAL 2C07 measured as the 2C02 was. The owner found Visual 2C07
  (`https://www.qmtpro.com/~nes/chipimages/visual2c07/`) and approved its download:
  `ext-tools/visual2c07` (the three .js files), converted by `ext-tools/visual2c07-tools/convert.py`
  to VisualNes's text format (16 940 transistors, 53 043 segments; the 2C02 has 16 758 / 35 615;
  367 node names, renumbered for this die). Step 1, the PPU alone on VisualNes's simulator
  (`visual2c07-tools/bench2c07.cpp`, master clock on `clk0`, CPU writes driven on the pins):
  312 lines of 341 dots, a dot every 10 master half-clocks (the /5 divider), `vbl_flag` set on
  line 241 dot 1 and cleared on line 311 dot 1, every frame 1 063 920 half-clocks with rendering on
  (`bkg_enable` and `spr_enable` read back high): **no odd-frame dot skip**. Zaxnes's PAL timing
  (`region_timing.rs`: 312, vblank 241, pre-render 311, no skip) agrees on all of it. Next: the
  register-write and read timing with a CPU on the bus. The 2A07 is not in the netlist set; the
  2A03 netlist clocked at 12/16 of the master would give the PAL CPU rate, its M2 duty left as
  the 2A03's (to state as an approximation).
  Step 2 (2026-11-15), a PAL console on the netlists (`visual2c07-tools/benchpal.cpp`,
  `datadefs_pal.cpp`, `macros_pal.cpp`): the 2C07 wired to VisualNes's 2A03 as the NTSC console
  is (pins found by the 2C07's own names), the CPU's clock input separate from the PPU's on a
  common grid (master edge every 3 ticks, CPU input edge every 4: CPU = master / 16, the 2A03's
  M2 duty of 15/24 standing in for the 2A07's, `/CS` decoded from M2 as on the NTSC bench). A
  loop writing `$2000 = $80`: the writes of the first frame are ignored (the 2C07 has the
  after-reset window, as Zaxnes models it up to the pre-render line), the first NMI vector fetch
  at CPU cycle 58 931 (line 241), then 33 245 and 33 249 cycles apart (33 247.5 expected, the
  difference the loop's 7-cycle instruction boundaries). The bench is sound; next, the A-lot
  programs run on it against Zaxnes's PAL mode (register-write dots, read merge, OAMADDR defects,
  OAM refresh on lines 265-310, logic defects).
  Step 3 (2026-11-15), the per-dot judge (`benchdotpal`, `paljudge.py`, `palglitch.py`; harness
  `dot-trace --region pal --dot-phase`): `g2000` at the five reset phases (0, 3, 6, 9, 12 grid
  steps), accesses all on the netlist's dots at four of them. **The 2C07 has no `$2000` dot-257
  glitch**: 31 212 to 37 140 differing dots with it, 23 322 without (the rest is the bench's
  power-on frame, as on NTSC, and 3 dots a frame: line 239 dot 256 and the pre-render's vertical
  copy, line 311 dots 278-279, to look at next). Zaxnes: no glitch on PAL (`write_control`), test
  `fidelity_2000_write_on_dot_257_takes_nametable_x_from_open_bus`.
  Step 4 (2026-11-15), `g2001px` (rendering switched on and off along the lines): **the 2C07
  switches the rendering four dots after a `$2001` write, the 2C02 two** (a switch on written on
  dot 12: no increment on dot 16, fetches again from dot 17). Delay 4 on PAL: 8 170 differing dots
  a frame left, against 54 000 to 97 000 with 2, 3 or 5; then the dot-257 copy follows the state
  before a switch landing on it, as on the 2C02 (`late_switch`, the write on dot 252 with four
  dots): 0 differing dots on whole frames but the pre-render's 2 (line 311, dots 278-279). Test
  `fidplus_c3c_pal_rendering_switch_takes_four_dots`.
  Step 5, the pre-render's vertical copy: **the 2C07 copies `t`'s vertical bits from dot 278**,
  the 2C02 from 280 (`v` holds the copy at the end of dot 278 in every frame, `vcopy.py`). Window
  278-302 on PAL (`Ppu2C02::vertical_copy_dots`; its end shifted too, unmeasured): `g2001px` 0
  differing dots on every whole frame, `g2000` 1 a frame (line 239 dot 256). Test
  `fidplus_c3c_pal_vertical_copy_starts_on_dot_278`. Still open from the earlier steps:
  the first frame's `$2007` increments seen 4-5 dots later on the netlist (line 241).
  Corpus at reset phase e3 so far (alignment 3 against 0, differing dots, whole frames): `g2005`
  31 543 / 25 103 and `g2005px` 7 508 / 2 034 (the alignment-3 `$2005` open-bus effects look
  absent on the 2C07 too), `g2006px` 7 104 both.
* C3 (the goal's list, for the record): RP2A03 letter revisions (the DMC/OAM DMA and `$4015` read
  differences), the PAL 2C07 measured as the 2C02 was, random CPU/PPU alignment and RAM at power-on
  as a model choice; each with the test ROM or netlist that judges it.
