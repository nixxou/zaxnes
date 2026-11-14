# FIDELITY — fidelity to the console and its documentation (progress)

Owner, 2026-11-13: work on the gaps found against Mesen, the goal being the console and the
documentation rather than Mesen itself, and `AccuracyCoin` passing entirely if possible.
Branch `feat/fidelity`, from `dev` at `27e2c3e2`. Study behind it: `docs/review/NMI-MESEN.md`
(branch `feat/core-tiers`).

Reference before the lot: test ROMs 212 pass / 7 fail / 1 unsupported / 111 manual, AccuracyCoin
129/147 (132 with `ppu-exact-sprite-eval`), golden-core `bc8497d3…`.

| Step | What | AccuracyCoin | Test ROMs | golden-core |
|---|---|---|---|---|
| 1 | Secondary OAM not cleared on the pre-render line (nesdev, "PPU sprite evaluation") | 129 | same | same |
| 2 | `$4015` bit 6 raised for two cycles even with the frame IRQ inhibited (`Frame Counter IRQ` I-M) | **130** | same | same |
| 3 | A DMC fetch caught by a `$4015` switch-off still halts the CPU for 1, 3 or 4 cycles (`Explicit DMA Abort`; `DMA_DMC_GHOST` in `nes_bus.rs`) | **131** | same | same |
| 4 | The fetch a one-byte sample asks for and loses costs one halted cycle, none on a write cycle (`Implicit DMA Abort`) | **132** | same | same |
| 5 | A DMA read that also reaches `$4016`/`$4017`: the register's answer is on the bus for the other read, and only the driven pins conflict (`APU Register Activation` 6 and 7) | **133** | same | same |
| 6 | A DMC fetch that also reaches `$4015` leaves the address's own byte on the pins; a controller port keeps its own low bits in a conflict (`DMC DMA Bus Conflicts`) | **134** | same | same |
| 7 | `ppu-exact-sprite-eval` in `default` (measured +0.04 % to +0.13 % at MF-4e, inside the per-release budget; to re-measure) | **137** | same | same |
| 8 | `$2004` during rendering: the secondary OAM is read back on even dots once the copy is over, full or not, and dots 321-340 answer from its index 0 (`$2004 Stress Test`) | **138** | same | same |
| 9 | OAM row corruption when the rendering is switched off inside a line (`OAM Corruption`), as a setting: `NesCore::set_oam_corruption`, `emu.oam_corruption` in the desktop window and the console menu, **off by default** (owner, 2026-11-13, as in Mesen 2 whose `EnablePpuOamRowCorruption` is false). `ZAXNES_OAM_CORRUPTION=1` runs the campaign with it: **139**, golden-core unchanged, but blargg's `sprite_overflow_tests/5.Emulator` fails its test 3 (rendering switched off at line 126 dot 8, row 5 marked) — the model is incomplete, as Mesen's own note says | 138 (139 with the setting) | same | same |
| 11 | The secondary OAM address loses the steps of the fetch dots the rendering was off for, and `$2004` at dots 321-340 reads there (`Misaligned OAM2 Address`) | **139** (140 with the setting) | same | same |
| 12 | **A `$2001` write that switches the rendering takes effect two dots later** (`Ppu2C02::switch_rendering`: the two dots run at once under the old mask and are owed back to the next catch-up, nothing per dot); the odd-frame skip is decided on dot 340 instead of 338 to match; the secondary OAM's overflowed flag across a switch (`Frozen OAM2 Increment`) | **140** (141 with the setting) | same (`ppu_vbl_nmi` 10 needed the 340) | same |
| 13 | The high plane of the background shifters shifts in a 1; a rendering switched back on for the very dot of a shifter reload comes one dot too late for it (`Ppu2C02::skip_background_reload`, cold path) (`BG Serial In`) | **141** (142 with the setting) | same | same |
| 14 | **The sprite output units as the hardware has them, for the lines a switched rendering disturbs** (`ppu/sprite_hw.rs`: a counter and a shifter per sprite; the counter runs rendering or not, the shifter only while the PPU renders, dot 339 sets them counting). Followed from a switch-off on a line that renders until a clean fetch window, brought up to date only at a `$2001` switch and at a sprite-line rebuild (`Stale Sprite Shift Regs`, `Stale BG Shift Registers`) | **143** (144 with the OAM setting) | same | same |
| 15 | A `$2007` read made while the PPU fetches fills the read buffer with **the byte the fetch of five dots later reads** (four of the state machine, one of this core's lead), and its increment of `v` comes after (`Ppu2C02::read_buffer_from_fetch`, prepaid dots) (`$2007 Stress Test`) | **144** (145) | same | **3 dumps move, picture only**: *Super Mario Bros. 3* (Japan, three dumps) reads `$2007` at dots 223-233 of line 193, its split line |
| 16 | The fetch a `$2007` read collides with is made at the address latch's stale low byte (`ALE + Read`); the second `$2006` write loads `v` four dots later, and at a hybrid address when it lands inside a nametable or attribute fetch (`Hybrid Addresses`). Steps 15 and 16 are behind **one core setting, `emu.ppu_bus_conflicts`, on by default**; off, both accesses act at once as before the lot | **146** (**147 / 147** with the OAM setting) | same | the same 3 *Super Mario Bros. 3* (Japan) dumps, picture only; nothing else moves |
| 18 | **Performance**: the high plane's serial 1s leave the hot path. `shift_background` shifts the four registers plainly again (one vector shift); the 1s are counted when the rendering goes off and set when it comes back (`Ppu2C02::serial_ones`, cold). The `| 1` cost +170 M instructions on a 600-frame *Super Mario Bros. 3* run (~+0.7 %) | 143 (147 with both settings) | same | same |
| 18b | The reloads record nothing for the serial count either: the last reload is found back from the switch-off, on the cold path, so the span loop keeps `ppu_clock` in a register (the first version of step 18 cost +84 M on *Mega Man 2*) | 143 (147) | same | same |
| 19 | **2C02G/H OAMADDR defect** (nesdev "PPU registers"): with OAMADDR >= 8 when rendering starts, the row at `OAMADDR & $F8` is copied over OAM 0-7; not on PAL. Setting `emu.oamaddr_bug`, **on by default**; test fixtures now leave OAMADDR at 0 as a game does | 143 (147) | same | same |
| 20 | **OAM decay** (nesdev "PPU OAM"; BeesNES `RefreshOamRow`): with the rendering off, an 8-byte row nothing accessed for 8 NTSC frames (~133 ms) decays, each byte reading as its own index. Checked lazily on `$2004`, OAM DMA and the rendering's return; nothing per dot. Setting `emu.oam_decay`, **off by default**: its clock stamps are not in the savestate, so run-ahead and rewind would never let it happen. blargg `oam_stress` and `sprdma_and_dmc_dma` still pass with it on | 143 (147) | same | same |
| 21 | *(branch `feat/fidelity-poweron`)* **Power-on at the top of the picture** (nesdev "PPU power up state"; dot 11 as BeesNES): setting `emu.power_on_top`, **off by default**. With it on, blargg `5.Emulator` passes with `emu.oam_corruption` on; every game's timeline moves (golden-core: 98 games, 23 pictures at frame 1200) | 143 (147) | same | same |
| 22 | *(branch `feat/fidelity-poweron`)* **Console audio profiles** (BeesNES, MIT): setting `audio.console_profile`, 14 measured NES/Famicom models, **none by default** (output bit-identical, golden unchanged). Channel gains in the mix, the console's low-pass and three high-passes once per output sample, letterless noise on N0260073. Approximations in FIDELITY-LEADS E1 | 143 (147) | same | same |
| 23 | *(branch `feat/fidelity-poweron`)* **CPU bus activity checked against BeesNES's 2A03 table** (xlsx, MIT): generator `rust/tools/cpu_bus_reference.py`, test `cpu_bus_reference_tests.rs`; 244 opcodes x 2 scenarios, direction and address class of every access, all match. Test only, no runtime change | 143 (147) | same | same |
| 24 | *(branch `study/access-phase`)* **PPU write timings measured on the transistor netlists** (VisualNes, owner-approved): `$2006` delay split out and on (`emu.ppu_2006_delay`), `$2006` collision AND glitch on (`emu.ppu_2006_glitch`, `t` damaged too), `$2000` dot-257 open-bus glitch on (`emu.ppu_2000_glitch`); golden unchanged on 98 games | **144** (147) | same | same |
| 25 | *(branch `feat/ppu-2007-read`)* **`$2007` read during rendering from the transistor measurements**: buffer = PPU data bus at dot D+6 (alignment 4), byte of X-1 on an odd X, X+Y increment before the disturbed fetch. Option `emu.ppu_bus_conflicts` still off: SMB3 J line 193 does not match the console video yet | 144 (**146** with the option, 147 all) | same | same |
| 26 | **`emu.ppu_bus_conflicts` on by default**: a PAL console video of *Super Mario Bros. 3* (Europe, French NES, composite) shows the damage the step-25 model predicts on its split line, line 194, x 0-7, on 34 % of the frames; Zaxnes Europe with the option: 34 %, same place; off: never (`docs/review/FIDELITY-LEADS.md`, I1). The Japanese line 193 video stays unexplained (origin unknown) | **146** (147 with OAM corruption) | same (212 pass) | 3 dumps, picture only: *Super Mario Bros. 3* (Japan, Rev 1, Rev 1 VC) |
| 27 | *(branch `feat/fidelity-align`)* **`emu.power_on_top` and `emu.oam_corruption` on by default, together**: the PPU powers on at the top of the picture (nesdev), which puts blargg's `5.Emulator` OAM corruption on a harmless row, so the corruption model (AccuracyCoin's console measurements) can be on. OAM corruption measured on the transistor netlists first: the netlist reproduces the secondary OAM address per dot but not the copy itself (only 2 alignments of 4, opposite direction in the sprite-fetch phase), so AccuracyCoin's rule stays (`docs/review/FIDELITY-LEADS.md`, D1). `ppu_golden_tests` keeps the vblank power-on its goldens were taken with | **147** | **213 pass / 6 fail** (`sprite_overflow_tests/5.Emulator` passes) | all 98 games: `cycles` and audio (first frame 27 392 cycles); 17 pictures at frame 1200, a phase shift only (Excitebike, Metal Force, Rockman 4/5, Zelda II Europe, SMB3 Japan), checked by eye |
| 28 | **`emu.cpu_ppu_alignment`** (Emulation tab, "Alignment 0-3"): the four CPU/PPU clock alignments, read side first (`$2007` read latch dot, `$2000` glitch on `t`). Tests keep the vblank power-on they count from | 147 | same | unchanged (option at its default) |
| 29 | *(29a)* alignment numbering and delays taken from TriCNES (AccuracyCoin's author, MIT): `$2001` rendering 2/2/3/2 dots and OAM seed 2/3/3/2, `$2006` load 4/4/5/4; old behaviour = alignment 3, the default. *(29b)* OAM corruption rules ported by running TriCNES and Zaxnes on the same programs (switch-off sliding a dot per frame, re-enable in vblank or ten lines on): immediate copy of the evaluation row in alignments 1 and 2, stale row at dot 65, pending seed dropped by a mid-frame re-enable in alignments 1 and 2. One TriCNES rule rejected: it fails AccuracyCoin's own `OAM Corruption` test in alignment 2 (`docs/review/FIDELITY-LEADS.md`, D1) | **147 in all four alignments** | 213 pass / 6 fail | same as step 26 |
| 30 | `$2005` write landing on a copy of `t` (dot 257, pre-render dot 304) reaches that copy, with the CPU open bus in alignment 2 (TriCNES, measured on two sliding programs: 0 differences in all four alignments); open-bus part under `emu.ppu_2000_glitch`, renamed "$2000/$2005 open-bus glitch" (`docs/review/FIDELITY-LEADS.md`, A4) | **147 in all four alignments** | 213 pass / 6 fail | same as step 26 |
| 31 | `$2005` fine X on the pixel of the write's dot: one pixel earlier in alignment 0, open-bus fine X for one pixel in alignment 2 (TriCNES, line colours compared per frame: 0 differences in all four alignments). Perf of steps 27-30 (clean builds `2ba2ed38` → `dff85e91`, callgrind): totals −0.074 % to +0.013 %, PR-1 set −0.49 % to +0.04 % | **147 in all four alignments** | 213 pass / 6 fail | same as step 26 |
| 32 | **Transistor netlist check of the TriCNES-only points (VisualNes)**: the `$2005` open bus (dot-257 copy and fine X pixel) belongs to alignment 3, the default, not to 2 (Mesen 2 and BeesNES agree); TriCNES's alignment-0 pixel rule dropped. Palette background (documented); `$2006` load with rendering off shown two dots later (netlist). The netlist's switch-on pixel (a pixel later) left open: it cost AccuracyCoin's `$2007 Stress Test`. TriCNES's palette corruption ported for alignment 2 behind `emu.palette_corruption` (table generated from its source). Open: re-enable in mid-fetch (octal latch), netlist ↔ alignment 0-2 mapping (`docs/review/FIDELITY-LEADS.md`, A4, D5) | **147 in all four alignments** | 213 pass / 6 fail | same as step 26 |
| 33 | **Netlist with the CPU's hold time; alignments mapped (0 = a2, 1 = a0, 2 = a6, 3 = a4)**: calibrated on the console-checked `$2007` latch. `$2001` acts two dots after the write and `$2006` loads three in all four alignments (TriCNES's extra dot in alignment 2 dropped), the `$2000` glitch in alignment 3 only (TriCNES's alignment 0 dropped), `$2001` picture edges per alignment (switch off D+1/1/2/2, on D+3; no dot run ahead), OAM seed of alignment 2 kept on TriCNES's dot (`docs/review/FIDELITY-LEADS.md` A3c) | **147 in all four alignments** | 213 pass / 6 fail | same as step 32 (alignment 3 unchanged) |

## Rows still failing

None: **147 / 147 in the default build** since step 27.

* `emu.oam_corruption`: on by default since step 27, with `emu.power_on_top` (before, it failed
  blargg's `sprite_overflow_tests/5.Emulator`, whose corruption landed on a row that mattered).
* `emu.ppu_bus_conflicts`: on by default since step 26 (history: it was off while the split
  line of *Super Mario Bros. 3* (Japan) disagreed with a console video and with Mesen;
  the PAL video settled it, see step 26).

## Performance

Callgrind, clean builds from `git archive`, dev `27e2c3e2` against `56bba39c` (both settings as
then: bus conflicts on): total +0.42 % (mm2), +0.52 % (zelda2), +0.53 % (metroid), +0.55 % (smb1),
+0.71 % (smb3); PPU stage +1.0 % to +1.7 %; PR-1 set +0.48 % to +0.95 %.

After steps 18 and 18b (`f4185fa8` against `27e2c3e2`, bus conflicts off as shipped): total
**-0.14 % (metroid), -0.08 % (smb1), -0.06 % (zelda2), -0.03 % (smb3, mm2)**; PPU stage -0.18 to
-0.24 %; APU stage +0.8 to +0.9 % (register allocation of `step_cpu_cycle`, see
`docs/review/FIDELITY-LEADS.md` D2b); PR-1 set -0.13 % to +0.48 % (`oamstr` and `vblnmi` the
highest). Steps 19 and 20 run once a frame or on an OAM access only.

## Still to do outside AccuracyCoin

* Power-on: the PPU "at the top of the picture" (nesdev, "PPU power up state") instead of the
  start of vblank; moves every game's timeline (golden-core, TAS sync, widescreen recipes).
* The sprite-overflow bit of `$2002` in *The Legend of Zelda* (set in Mesen, clear here).
* `$2002` open-bus bits and `$2007` reads during rendering: differ from Mesen, both follow test
  ROMs; to be settled against the documentation.
* Performance: done for the lot (games net negative); the APU lead is recorded (D2b).
