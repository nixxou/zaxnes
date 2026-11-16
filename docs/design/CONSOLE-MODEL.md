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
  `fidplus_c3c_pal_vertical_copy_starts_on_dot_278`.
  Step 6, `r2007` (`$2007` reads during rendering; the judge needs `--bus-conflicts`, the
  application's default): the read's X and Y increment comes **5 dots after the access on the 2C07
  in 60 % of the reads and 6 in 40 %** (`r2007lat.py`; the 2C02: always 6), whatever the dot: the
  read lands at another point of its dot from one cycle of the 16-dots-for-5-cycles pattern to the
  next. Of the ten two-of-five splits of the dot accumulator, only "6 on 3 and 4" fits (8 400
  differing dots a frame, almost all the 1-2-dot recording instants; 49 000 to 102 000 for the
  others); the A5 merges with the pipeline's own increments (dots multiple of 8, dot 257) hold on
  PAL too. Rendered lines: one long run left over five frames. Test
  `fidplus_c3c_pal_2007_read_latch_follows_the_cycle_phase`. Left: on lines 240-310 the
  increment of a read with rendering off shows 4-5 dots later than in Zaxnes (for `rmw`).
  Step 7, `g2005` / `g2005px` and `g2006x`, the same five-cycle pattern again:
  * a first `$2005` write on the dot-257 copy takes the CPU's open bus only on the cycle whose dot
    accumulator is 4 (of the 32 sets of phases, {4} fits best; the written value otherwise):
    both programs 0-1 differing dots a frame (`Ppu2C02::open_bus_on_copies`). The `$2000` glitch
    stays absent on every phase ({4} alone: 330-659 dots a frame against 1);
  * the second `$2006` write's load comes 2 dots after the write on phase 0, 3 on the others (2 in
    20 % of the loads, 3 in 80 %; the 2C02 always 3, `w2006lat.py`), and the A5 increment after a
    delayed load holds on PAL: `g2006x` 14 700 -> 4 900 differing dots a frame, long runs on
    rendered lines 2 113 -> 42 (left: a `$2006` load landing on the dot-257 copy, which the
    2C07 takes from the written byte; only Fidelite+'s `post_address_load` models that case).
  Tests `fidplus_c3c_pal_2005_on_dot_257_takes_the_open_bus_one_cycle_in_five`,
  `fidplus_c3c_pal_2006_load_delay_follows_the_cycle_phase`.
  Step 8, `oam` (rendering switched off and on mid-line, as `g2001px` on other dots): the 2C07's
  switch delay is 3 dots on the cycle whose dot accumulator is 0, 4 on 3 and 4 (phases 1 and 2
  not reached at reset phase e3, kept at 4; netlist traces at e0, e6 and e9 queued to reach them).
  `oam`, `g2001px` and `g3e01px`: 0 differing dots on every whole frame.
  Step 9 (2026-11-16), the corpus at the other reset phases (e0, e6, e9; `phases2.py`, which
  among the dot phases that align every access keeps the one whose picture differs least):
  * one dot phase per reset phase, the same for every program: e0 -> 0, e3 -> 2 (`r2007`,
    `g2000`, `g2001px`, `oam`, `g2006x` all fit 2 only), e6 -> 1;
  * the `$2001` switch is three dots on phase 2 too (at e3 / dot phase 2 `g2001px` and `oam` match
    with three, not four): three on phases 0 and 2, four on 1, 3 and 4. Phase 1's four is
    confirmed at e6 (0 differing dots);
  * a second `$2006` write on dot 257: the 2C07's horizontal copy takes the written byte, the
    CPU's open bus on phase 4, as `$2005` there. `g2006x` long runs on rendered lines 42 -> 14
    at e0, e3 and e6 (`Ppu2C02::write`, register 6);
  * left: `g2006x`'s write whose `t` reads `$2805` on the netlist (lines 54, 148, 223: dots
    256-281); every program at e9 parts from the netlist from the first frame (no access near;
    a power-on difference, not a register rule); `r2007` / `oam` at e0 likewise.
  Step 10 (2026-11-16), closing C3c. At reset phase e3 the whole corpus fits dot phase 2:
  `g2001px`, `oam`, `g3e01px` 0 differing dots a frame, `g2000`, `g2005`, `g2005px` 0-1, `rmw` 167
  and `g2006px` ~1 400 with no long run on a rendered line, `g2006x` 14 long runs (the dot-256
  collision, FIDELITY-LEADS C3c-collision). At e9, `g2000` (1 a frame) and `oam` (0) fit with
  the rules as they stand; `g2001px` does not at any dot phase, power-on offset (+-40) or
  alignment (0-3): its `$2001` writes land on the same dots as at e3 but the netlist switches
  the rendering at another point of the dot. Zaxnes's PAL rules are per dot accumulator phase
  (five positions of the CPU cycle in the dot); this reset phase looks to sit between two of
  them, which a dot-granular switch cannot express. Recorded as the limit of the model; a
  half-dot switch would be a Fidelite+ refinement.
  Still open from the earlier steps:
  the first frame's `$2007` increments seen 4-5 dots later on the netlist (line 241).
  Corpus at reset phase e3 so far (alignment 3 against 0, differing dots, whole frames): `g2005`
  31 543 / 25 103 and `g2005px` 7 508 / 2 034 (the alignment-3 `$2005` open-bus effects look
  absent on the 2C07 too), `g2006px` 7 104 both.
* C3 (the goal's list, for the record): RP2A03 letter revisions (the DMC/OAM DMA and `$4015` read
  differences), the PAL 2C07 measured as the 2C02 was, random CPU/PPU alignment and RAM at power-on
  as a model choice; each with the test ROM or netlist that judges it.
