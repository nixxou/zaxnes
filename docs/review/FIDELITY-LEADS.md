# Fidelity leads — everything that wins or loses fidelity

Owner rule (2026-11-13): every lead, reference, approximation or known defect of Zaxnes that touches
fidelity to the console goes here, **whatever its cost in work or performance**, so that the core split
(Base / Officiel / Extra / Super-Extra and **Fidélité+**) can implement the whole of it later. Free and
correct items go into the default core; costly ones are recorded and wait for Fidélité+.

Each entry: what the hardware does, what Zaxnes does, references (file:line), cost, status.
Line numbers are those of branch `feat/fidelity` at `7b52bbcb` and of the external sources as cloned
on 2026-11-13 (BeesNES `43d2ad8c`, `G:/3DSen/ext-tools/bees-src`; Mesen 2 in
`cleanroom/third_party/mesen2`).

Status keys: **open** (nothing done), **partial**, **done (option)**, **done**, **rejected (reason)**.

---

## A. Clocking and scheduling

### A1. Half-phase CPU and PPU (PHI1 / PHI2) — open, Fidélité+
- Hardware: each CPU cycle has two halves; reads latch on PHI2, IRQ/NMI lines are sampled on given
  halves; the PPU dot has two halves too.
- Zaxnes: one step per CPU cycle, three dots batched, events placed by calculation:
  `NMI_SAMPLE_LEAD` (`zax-nes/src/ppu/mod.rs:86`), interrupt promotion in `end_cycle`
  (`cpu/cpu6502.rs:321`, rules at `:733`). Proved equivalent to Mesen's rule for NMI
  (`docs/review/NMI-MESEN.md`, 57 ROMs instruction-locked).
- BeesNES: master-clock scheduler, five slots (`Src/System/LSNSystem.h:1002-1018`), CPU
  `Tick`/`TickPhi2` (`Src/Cpu/LSNCpu6502.cpp:43-75`: IRQ seen low on PHI2 becomes the PHI1 flag; NMI
  edge detected on PHI2), PPU `Tick`/`TickPhi2` (`Src/Ppu/LSNPpu2C0X.h:137-230`).
- Gain: none measured on any test ROM today; would remove the calibrated constants below (A3).
- Cost: rewrite of the CPU loop and of the bus; large perf cost (BeesNES compensates with branchless
  code everywhere, README "Performance").

### A2. Real-time pacing and input read at the real instant — open, Fidélité+ (app side)
- Hardware: the pad is read when the game strobes `$4016`; the picture leaves at 60.0988 Hz (NTSC)
  as it is drawn.
- Zaxnes: whole frames, paced on the display/audio; input sampled once per frame; frame rate rounded.
- BeesNES: real clock with microsecond resolution, ~29 780.506887 CPU cycles per frame at
  60.098814 fps NTSC, ~33 247.485977 at 50.006979 PAL (README "Accuracy"); pad polled when strobed
  (`Src/Cpu/LSNCpu6502.cpp:60-68`, `PollPort` on the put cycle).
- Gain: latency (up to one frame) and exact speed. Not visible in emulated state, only in feel.
- Cost: app loop (zax-app), audio resampling to follow a free clock; core unaffected.

### A3. Our calibrated shortcuts (to remove in Fidélité+) — partial
Each one is exact on every test we have, but is a modelling shortcut, not the mechanism:
- **Register accesses lead the hardware by one dot** (empirical, used three times): `$2001` switch
  +2 dots (`switch_rendering`, `ppu/mod.rs:604`), reload missed on the switch-on dot
  (`skip_background_reload`, `:633`), `$2007` buffer from the fetch 5 dots on
  (`read_buffer_from_fetch`, `:1656`, "four of the state machine, one of this core's lead"),
  `$2006` load +4 dots (`delay_address_load`, `:1768`). AccuracyCoin documents 2 to 5 dots depending
  on the CPU/PPU alignment; we model a single alignment.
- **Prepaid dots** (`prepaid_dots`, `take_prepaid` `:1254`): the PPU runs ahead at a register write
  and is owed back. Correct while no other event lands inside the prepaid window (none observed).
- **NMI sample lead** (`NMI_SAMPLE_LEAD = 2`, `:86`) and **odd-frame skip decided on dot 340**
  (`SKIP_DECISION_DOT`, `:121`).
- **Sprite output units followed only on disturbed lines** (`ppu/sprite_hw.rs`); the ordinary line
  is painted from per-slot X/pattern (`rebuild_sprite_line`, `ppu/render.rs:976`). Exact on
  AccuracyCoin; a real counter/shifter per sprite on every line is the hardware.
- **Batched PPU spans** (`run_visible_span`, `ppu/render.rs`), idle-dot skipping.

### A3b. Where a CPU access lands inside the CPU cycle — study `study/access-phase` (2026-11-13)
- **Zaxnes**: every CPU cycle runs its 3 PPU dots first, then the access (`NesBus::read`/`write`:
  `tick()` then the access); reads and writes land at the same instant; the "+1 dot" of A3 is
  added by hand where tests demanded it.
- **Mesen 2** (`Core/NES/NesCpu.cpp:294-321`): the PPU is run to +5 master clocks before a read
  and +7 before a write (`_startClockCount` 6 -/+ 1), minus `_ppuOffset` (1 by default, 0-3 when
  `RandomizeCpuPpuAlignment`). Worked out by hand: with offsets 0 and 1 a write sees the same dot
  as a read; with 2 and 3 it sees one more.
- **BeesNES**: the bus access is on the CPU's PHI2 slot, +6 master clocks (`LSNSystem.h:1002-1018`),
  reads and writes alike, one alignment.
- **Hardware**: undocumented. Forum (nesdev t=8216, ReaperSMS): the PPU likely latches read data
  at M2's rising edge and write data at M2's falling edge; M2 is high 15/24 of the cycle on the
  2A03E/G/H. Nobody has measured it; Visual2C02's `/WR` phase lasts ~7.5 master clocks.
- **Measured in Zaxnes** (harness knobs on the study branch, `ZAXNES_PHASE`, `ZAXNES_WRITE_LEAD`):
  (1) moving the CPU/PPU alignment by 1 or 2 dots at power-on changes **no** test ROM result in
  the default build (the ROMs sync themselves to the dot); with all options on, blargg
  `5.Emulator` then passes (it fails only on alignment 0). (2) Making every `$2000`/`$2001`/
  `$2005`/`$2006`/`$2007` write one dot later changes **no** result either, AccuracyCoin included
  (147 kept). **Conclusion: our test ROMs cannot tell where a write lands within one dot; the
  "+1" constants are not proved by them beyond the three cases that needed it.**
- **Ground truth available**: VisualNes (Sour, `github.com/SourMesen/VisualNes`, archived; cloned
  in `ext-tools/visualnes-src`) runs the transistor netlists of the 2A03 and the 2C02 together on
  one master clock. Its PPU chip select is approximated (`/CS` low 11 half-steps from phi0,
  `Core/macros.cpp` `halfStep`); the 2A03 netlist has the real M2 pin (`phi2`, node 10743), so the
  74LS139 decode `/CS = !(A13 & !A14 & !A15 & M2)` can be wired exactly. A headless bench
  (`scratchpad/vn/bench.cpp`, logging `/w200x`, `/r2002`, `bkg_enable`, `vbl_flag`, `v`, `t`,
  `hpos`, `pclk0/1`, M2 and `/CS` per half master clock) is written but **not run: building and
  running downloaded code needs the owner's permission**. Measuring it per alignment (reset held
  0-3 extra master clocks) would give, for the first time, the exact dot each register write and
  read takes effect on — the basis for removing every calibrated constant of A3 and for A4.

### A3c. Transistor-level measurements (VisualNes, owner-approved build, 2026-11-13)
Bench: `G:/3DSen/ext-tools/visualnes-bench` (outside the repo: VisualNes publishes no licence, so no
derived code enters Zaxnes; only the measured facts below). 2A03 + 2C02 netlists on one master
clock; 1 half-step = 1/2 master clock; 1 dot = 8 half-steps; 1 CPU cycle = 24 half-steps; M2 high
15 half-steps (2A03G 15/24 duty). Four CPU/PPU alignments measured (reset held 0, 2, 4, 6 extra
half-steps: M2 rises at sub-phase 2, 0, 6, 4 of a dot). `hpos` = nesdev dot (vblank flag rises at
the start of hpos 1 of line 241; fetch pairs start on odd hpos).
- **PPU chip select**: VisualNes approximates /CS (11 half-steps from phi0). Replaced by the
  74LS139 decode from the 2A03's M2 pin, /CS falling one half-step after M2 rises (the 139's
  propagation ~ one half master clock) and rising one half-step before M2 falls. That last
  choice is forced: the zero-delay netlist drops R/W and the data bus on the very half-step M2
  falls, so the CPU's hold time cannot be simulated; on a console /CS rises ~1 half-step *after*
  M2 falls. **Every "store" time below may therefore be 1-2 half-steps early** (up to a quarter
  dot), which decides the dot only when a store lands right before a pclk edge.
- **Reads** (`$2002`): the flag is sampled, and cleared, when /CS falls (M2 rise + 1).
- **Writes**: the register stores when /CS rises (M2 rise + 14 here, ~+16 on a console). In one
  CPU cycle a write therefore acts **~1.7 dots after a read samples**. Zaxnes applies both at the
  same instant and is calibrated on reads (blargg vblank suite), which is the mechanism behind
  the "+1 dot" register-write lead of A3.
- **`$2001`**: `bkg_enable_out` and `/rendering_disabled` change on the store itself, any
  alignment, no internal delay; fetches (ALE) start at the next odd dot (0-2 dots) or stop at once.
  nesdev's "3-4 dots after the write" counts from the start of the CPU cycle (store at ~2.9 dots).
- **`$2006` second write**: `v` loads on the pclk1 edge following the first pclk0 edge after the
  store, i.e. in the *middle* of a dot: dot D+3 in alignments 0/4/6, D+2 in alignment 2 (D = the
  dot where a read in the same cycle samples, which is Zaxnes's access dot). The mid-dot load is
  the "hybrid address" of AccuracyCoin (low byte already latched by ALE with the old `v`, high
  lines driven from the new `v`). Zaxnes's `delay_address_load` (+3, hybrid check, +1) equals the
  majority alignment exactly.
- **`$2000` dot-257 glitch, measured** (program sliding `STA $2000` #$01 over every dot, rendering
  on): the copy of dot 257 takes nametable X from the open bus (`$20`) **only in alignment 4**
  (M2 rising at sub-phase 6, /CS falling in the last half-step of dot 257) and only for D = 257;
  the three other alignments never show it. Exactly nesdev's "for specific CPU-PPU alignments",
  "starts on dot 257". Implemented on by default (`emu.ppu_2000_glitch`), as alignment 4.
- **`$2006` collision, measured** (alternating targets `$2108`/`$2AD7`): when the load lands on a
  coarse-X increment (dots 8..248 by eights, 328, 336) coarse X = written AND incremented, every
  other field written; on dot 256 (X and Y increments) the whole address = written AND
  incremented. **`t` gets the same value** (not in nesdev): the copy of dot 257 and later lines
  keep the damage. Write at D%8 = 5 in alignments 0/4/6, D%8 = 6 in alignment 2. Implemented on by
  default (`emu.ppu_2006_glitch`, `Ppu2C02::address_load_collision`).
- **`$2006` load delay split from `emu.ppu_bus_conflicts`** into `emu.ppu_2006_delay`, on by
  default: AccuracyCoin 143 -> **144** (`Hybrid Addresses`), campaign unchanged, **golden-core
  unchanged on all 98 games including SMB3 Japan**: the SMB3 line-193 damage of the old option came
  from the `$2007` read model (C1), not from the `$2006` delay. C1 stays off and wrong in detail.
- **Also seen, not implemented**: the first `$2006` write and `$2005`/`$2000` writes put the open
  bus into `t` for the first half-steps of the write (`t` = `$2008` before `$2A08`); harmless unless
  a copy (dot 257, pre-render 280-304) falls in that window — the same mechanism as the `$2000`
  glitch, for `$2005`/`$2006`. Lead for Fidelity+. Alignment 2 loads `$2006` one dot earlier in
  general; an alignment option (A4) would carry it.
- **`$2007` read during rendering, measured** (step 25; `LDA $2007` sliding over every dot, distinct
  VRAM contents): the buffer latches at the start of dot X = D + 5 (alignments 0, 2, 6) or D + 6
  (alignment 4) and takes the PPU data bus: on an even X the byte the fetch of X reads, on an odd
  X the byte of the fetch of X - 1. `v` takes an X+Y increment in the middle of X. On an odd X,
  ALE and /RD are active together, so the next fetch reads with that byte as its low address byte
  (`ALE + Read`). The old model buffered the *disturbed* fetch's byte in the odd case and
  incremented `v` after it; fixed (`read_buffer_from_fetch`, X = D + 6 as alignment 4).
  AccuracyCoin with `emu.ppu_bus_conflicts`: **146** (`$2007 Stress Test` and `ALE + Read` pass;
  only `OAM Corruption`, which has its own switch). Default unchanged (144), golden unchanged.
- **SMB3 Japan line 193 still does not match the console video** with the option on, at X = D + 6
  or D + 5 (empty frames 11-19 %, never on the video; 16 px 0-7 %, 22 % on the video). The read
  model now follows the netlist, so the gap is elsewhere: the video's origin is unconfirmed, or
  another approximation on that line (sprites, the refetch only modelled for the background, the
  alignment). Decisive next test: rebuild that line's register sequence (`$2006` at dots 200/212,
  `$2007` read at 224, `v` = `$0B02`) and its nametable/pattern bytes in a VisualNes NROM program
  and compare the pixels with Zaxnes. Until then `emu.ppu_bus_conflicts` stays off by default.
- **Step 33: the CPU's hold time on the bench, and the alignments mapped.** The netlist joins
  the CPU's and the PPU's data, A0-A2 and R/W pins into one node each and the bench raised /CS one
  half-step *before* M2 falls, so every write was stored 1-2 half-steps early (above). A bench
  variant (`mkhold.py`, `benchhold.exe`; still outside the repo) splits those pins: the PPU sees
  the CPU's values held RISE+1 half-steps after M2 falls, /CS falls at M2 rise + 1 and rises at
  M2 fall + RISE (the 74LS139 decode, ~1 half master clock late). **Calibration on a console
  measurement**: the `$2007` read latch comes out at D+5/5/6/6 (phases a2/a0/a6/a4) with RISE = 1
  or 2, exactly TriCNES's table that AccuracyCoin checks; RISE = 0 gives the old 5/5/5/6.
  **Mapping**: TriCNES's alignment is `PPUClock & 3` at the CPU's access, so 1, 2, 3, 0 are one
  master clock apart in that order; M2 rises at half-steps 0, 2, 4, 6 of the dot in a2, a0, a6,
  a4; with 3 = a4 (the `$2000` glitch and the `$2007` latch), **0 = a2, 1 = a0, 2 = a6,
  3 = a4**. AccuracyCoin's `$2002 Flag Timing` (sprite flag seen in alignments 2 and 3, "measured
  on consoles") agrees: those are the two latest reads in the dot.
  Measured with RISE = 1 (and 2 where it matters), alignments 0/1/2/3:
  - `$2001`, fetches and `v` increments: stop and restart on the third dot after the write in all
    four (the increment of D+2 happens, D+3 does not). TriCNES's extra dot in alignment 2 is not
    there; AccuracyCoin 147 without it. **Done**: two dots everywhere.
  - `$2001`, picture: switch off shows the backdrop from pixel D+1/1/2/2, switch on the tiles from
    D+3 in all four. `$3E01` gives the same edges (C3's one-dot wrong value does not move them).
    **Done** (`switch_pixel_edge`: pixel D+1 redrawn at once in alignments 0 and 1, pixel D+2 of a
    switch on set at the end of its line, no dot run ahead).
  - `$2006` second write, rendering on: `v` loads on D+3 in all four, the collision with the
    coarse-X increment at D%8 = 5 in all four (coarse X = target AND incremented old). TriCNES's
    fourth dot in alignment 2 is not there. **Done**: three dots everywhere.
  - `$2006` with rendering off into palette RAM: pixel D+2 in all four (as step 32).
  - `$2000` dot-257 glitch: a4 only, D = 257. TriCNES's alignment 0 on dot 258 is not there.
    **Done**: alignment 3 only (AccuracyCoin 147 in alignment 0).
  - `$2005` copy of dot 257: bus in a4 only; fine X pixel D-1 old in all four, pixel D the bus in
    a4 and the new value elsewhere: step 32's rules confirmed (the step-32 mix in a0/a2 was the
    analysis mixing frames; `pxana2.py`/`bdana2.py` take each write's own frame).
  - OAM corruption: the netlist produces it only when the write is stored in the second half of
    the dot (a6 and a4 with the hold time; a2 and a4 on the old bench). AccuracyCoin's author
    (console) has it in all four alignments, with rows differing between {0, 3} and {1, 2} from
    dot 257: a race the zero-delay netlist cannot reproduce. TriCNES's model kept; its seed in
    alignment 2 kept on its dot (one later than the switch, as alignment 1): the differential
    probe against the step-29 model differs on 1 try in about 16 000 (a vblank re-enable).
  - Seen, not ported: in a0/a2 a switch on sometimes clears low bits of `v` (`500C` -> `5004`),
    present or not depending on RISE: a race, not trusted.
- **Next**: a per-dot differential harness (same program in Zaxnes and VisualNes, alignment matched
  through the `$2002` race, compare PPU address bus and `v` per dot) to settle `$2001` per
  alignment, `$2007` reads, and the `$2000`/`$2006` glitches of plan 1.4/1.5 before enabling them.

### A4. CPU/PPU alignment variants — done (`emu.cpu_ppu_alignment`, default 3, steps 28-29)
- Hardware: four CPU/PPU clock alignments chosen at power-on; several tests (AccuracyCoin
  `BG Serial In`, `ALE + Read`, `Hybrid Addresses`, `OAM Corruption`) say results depend on it.
- Zaxnes: option "Alignment 0-3" in TriCNES's numbering; the old fixed behaviour is alignment 3.
  Per-alignment delays from TriCNES (`$2001` 2/2/3/2, `$2006` load 4/4/5/4, `$2007` latch, `$2000`
  glitch on `t`) and the OAM corruption rules of D1. AccuracyCoin 147/147 on all four.
- `$2005` (step 30): TriCNES applies the write 1/1/2/1 dots after it, at the start of the dot,
  and in the meantime `t` and fine X hold the CPU data bus as it was before the write (the
  `$2000` mechanism). Program P5 (a first write of `$F8`, bus `$20`, sliding a dot per frame
  across dot 257 of line 100) and P6 (a second write sliding across dot 304 of the pre-render
  line), run in both emulators with the same power-on phase (`ZAXNES_PHASE=-11`) and paired by
  write: before the fix one write position differed in each alignment, the one where this
  core's write lands on dot 257 (304), after the copy: TriCNES's copy already had the written
  value (alignments 0, 1 and 3) or the bus value (coarse X 4, coarse Y 4, alignment 2). Now
  0 differences on P5 and P6 in all four alignments (`scroll_write_on_copy`; the bus value is
  under `emu.ppu_2000_glitch`, renamed "$2000/$2005 open-bus glitch").
- `$2005` fine X (step 31): program P7 (palette and nametables set, fine X 3 in vblank, a first
  write of `$06` sliding a dot per frame across line 100, bus `$20`), line 100's colours
  compared between the two emulators per frame: alignments 1 and 3 already matched; in
  alignment 0 TriCNES shows the new fine X one pixel earlier (pixel `D - 1`, `D` this core's
  write dot), in alignment 2 pixel `D` takes the open bus's fine X (36 frames each). Fixed by
  `fine_x_write_mid_line` (pixel `D - 1` drawn again in alignment 0; dot `D + 1` run with the
  bus's fine X and owed back in alignment 2, under `emu.ppu_2000_glitch`): 0 differences on P7,
  P5 and P6 in all four alignments.
- **Step 32: the transistor netlist overrules TriCNES on `$2005`** (VisualNes, owner-approved,
  measurement only; bench `ext-tools/visualnes-bench`, `g2005.bin` + `g2005.py`, `g2005px.bin` +
  `benchpx.cpp` + `pxana.py`; D = the dot where /CS falls = this core's access dot; pixel =
  hpos - 2 as VisualNes draws it):
  - Recopy of dot 257: a first write on D = 257 gives the written coarse X in netlist
    alignments a0, a2, a6 and the **bus value** (`$20 >> 3` = 4) in **a4**; D = 258 gives the old
    value everywhere. a4 is the alignment this core follows by default (the `$2007` latch at D+6
    and the `$2000` glitch at D = 257 both single it out, steps 24-28), i.e. TriCNES's
    alignment 3. So the open bus belongs to alignment 3, where `$2000` glitches too (same
    mechanism), not to TriCNES's alignment 2. Mesen 2 (`ProcessTmpAddrScrollGlitch`) and
    BeesNES (`GlitchyVUpdate`) also take the bus on dot 257 in the one alignment they emulate.
  - Fine X pixel: pixel D - 1 keeps the old fine X in all four netlist alignments (TriCNES's
    "alignment 0 a pixel earlier" appears nowhere); pixel D shows the new fine X in a6, the
    **bus's fine X in a4**, a mix (about 75 % new) in a0 and a2 (two /CS phases in those runs).
  - Done: `scroll_write_on_copy` and `fine_x_write_mid_line` now apply the open bus in
    alignment 3 (under `emu.ppu_2000_glitch`), the written value in 0, 1 and 2; the a0 pixel rule
    is gone. The dot-304 vertical copy follows by the same mechanism, not measured.
- Mapping done in step 33 (A3c): 0 = a2, 1 = a0, 2 = a6, 3 = a4, with the CPU's hold time on the
  bench (the "one D+6 alignment" of the old bench was its early /CS); `$2001`, `$2006` and the
  `$2000` glitch now follow the netlist in all four alignments.
- Still open: random alignment at power-on.

---

## B. Power-on and reset

### B1. PPU position at power-on — open (no perf cost; moves every game)
- nesdev ("PPU power up state"): the PPU starts at the top of the picture.
- Zaxnes: starts at vblank (241, 1) (see `docs/review/NMI-MESEN.md`).
- Mesen 2: (-1, 340). BeesNES: scanline 0, **dot 11** (`Src/Ppu/LSNPpu2C0X.h:247-256`), chosen so
  that blargg `test_apu_2` polls on the right phase **and** `sprite_overflow_tests/5.Emulator` passes
  with OAM corruption always on (row corruption lands on a harmless row).
- Gain: follows the doc; would let `emu.oam_corruption` go on by default (see D1).
- Cost: zero perf; changes golden-core of every game, TAS sync (bk2), widescreen recipes, savestate
  compatibility of power-on states. Needs a full campaign and the owner's eye.
- **Done as an option on branch `feat/fidelity-poweron`** (`emu.power_on_top`, off; harness
  `ZAXNES_POWER_ON_TOP=1`): `Ppu2C02::reset` puts the PPU at line 0 dot 11 before the CPU's 7 reset
  cycles (so dot 32 at the first opcode). Measured: test ROMs unchanged (212/7); **with it,
  `emu.oam_corruption` no longer breaks blargg `sprite_overflow_tests/5.Emulator`** (213/6 and
  AccuracyCoin 147 with all options), so both could go on by default together. golden-core: all 98
  games move (`cycles`, first frame 27 392 cycles instead of 29 782; audio of 64), 23 dumps of 7
  families show a different picture at frame 1200 (Excitebike, Mega Man 4, Rockman 4/5, Metal
  Force, SMB3 Japan, Zelda II Europe): a phase shift, to check by eye. Reset length settled (B2,
  no change); the ignored writes after power-on are modelled (B5, step 34). **A random PPU
  position is not needed on the NES-001**: its reset line holds the PPU at line 0 dot 0 until the
  CPU starts (netlist, step 34), so the position at the first opcode is fixed; only the sub-dot
  alignment varies (A4, "Random" since step 36).

### B2. Reset length 7 vs 8 cycles — settled, no change (step 34)
- Zaxnes 7 cycles (`cpu/cpu6502.rs:173-178`); Mesen 8. Sets the DMA get/put parity. nesdev gives no
  count. With 8 + B1, Zaxnes locks instruction by instruction on MesenCE (NMI-MESEN.md).
- **Netlist (step 34, `benchreset.cpp`)**: after /RES rises the 2A03 makes one cycle (a read of
  whatever is on its address bus) before the 7 of the interrupt sequence (opcode fetch dropped,
  dummy read, three stack reads, vector low, vector high), so the first opcode is the 9th M2;
  the 10th when /RES rises less than 7 half-steps before M2 does (the release is asynchronous).
  BeesNES does 7 like this core (the dropped opcode fetch is its first cycle) with the PPU at dot
  11; Mesen 8 with the PPU at (-1, 340).
- The extra cycle only shifts the CPU against the PPU and the APU, and that phase is arbitrary on
  a console (CIC release, button press). What is measurable of it passes on console-checked tests:
  blargg `apu_reset` (all six, `4017_timing` "Delay after effective $4017 write: 10", inside the
  documented 9-12), `blargg_apu_2005.07.30/09.reset_timing`, `cpu_reset/registers` and
  AccuracyCoin. So the cycle stays folded into the power-on phase of B1; no change.

### B5. The PPU's reset line — done as an option (`emu.ppu_reset_line`, off since step 40, step 34)
- nesdev ("PPU power up state"): writes to PPUCTRL, PPUMASK, PPUSCROLL and PPUADDR are ignored
  ~29 658 CPU cycles after reset. TriCNES models it (`PPU_RESET`, ignored writes to `$2000`,
  `$2001`, `$2005`, `$2006`, cleared on dot 1 of the pre-render line) but sets it off: its author is
  "not even 100% certain my console has this behavior". Mesen 2, BeesNES and NesHawk: not modelled.
- **Netlist (step 34, `benchreset2.cpp`)**: the 2C02 has it. With /RES low the PPU holds its
  counters at line 0 dot 0, clears PPUMASK and sets `after_reset_reg`; after the release `$2001`
  writes do nothing until `after_reset_reg` clears on **dot 1 of line 261**, and the next write
  takes. The PPU restarts from line 0 dot 0 with the CPU's reset sequence, so a Reset press puts it
  where power-on does. Whether it happens depends on the console's wiring: the front-loading NES
  (NES-001) drives the PPU's /RST from the reset line; the top-loader and the Famicom (button on
  the CPU only) do not — which explains TriCNES's doubt.
- Done: `after_reset` (set by a power-on or a Reset press when the line is on, cleared on dot 1 of
  the pre-render line, in the state as bit 1 of the write-toggle byte so older states load), the
  four writes ignored, and the Reset button sends the PPU back to its power-on position. Golden-core
  and the test-ROM campaign unchanged (no game of the 98 writes those registers that early);
  AccuracyCoin 147.
- Unit-test machines that poke the PPU right after power-on now switch the line off
  (`tests/common`), as they already switch `power_on_top` off.
- **Step 40: off by default.** The closing perf run showed the homebrew `Duelito`
  (`nes-test-roms/other`, not in the campaign's verdicts: it has no pass/fail) half as costly per
  frame: with the line on it writes PPUCTRL/PPUMASK before the pre-render line and stays on a grey
  screen; off, it plays as before. It is right for a front-loading NES only if that program fails
  there too, which is not known; the top-loader and the Famicom run it. TriCNES and NesHawk leave
  the behaviour out, so the option stays for whoever wants the front-loader, off by default.
  Harness: `ZAXNES_PPU_RESET_LINE=1`.

### B3. Random power-on state — done as options (step 36)
- Zaxnes: `emu.power_on_ram` (zeros / FCEUX pattern / random) for work RAM (lot CARTDB).
- Not randomized: palette RAM, OAM, PPU registers, CPU/PPU alignment (A4), APU state.
- BeesNES: "start from known state or random state" (README), bus/PPU bus randomizable
  (`Src/System/LSNSystem.h:324-325`).
- **Step 36**: `emu.power_on_ppu_ram` (zeros / blargg's console / random) for the palette RAM,
  OAM and nametable RAM (`PowerOnPpuRam`, `Ppu2C02::fill_power_on_memories`, mirrors kept).
  The power-on palette differs per console: blargg's `power_up_palette` reads his (NesHawk copies
  it; TriCNES keeps it as `BlarggPalette` but uses its author's, a different one); FCEUX fills
  the PPU memories with zeros by default (`FCEU_MemoryRand(..., true)` in `ppu.cpp`; its
  `00 00 00 00 FF FF FF FF` pattern is for work RAM only). **Zeros stay the default**: with
  blargg's table the campaign gains `power_up_palette` (214 / 5) but every game's first pictures
  change (golden-core `pixel_sha256` of all 98; cycles unchanged), for one console's values.
- **Step 36**: `emu.cpu_ppu_alignment` gains "Random", drawn at each load (`power_on_seed`).
- PPU registers and APU state at power-on: left as they are (the PPU's are cleared by its reset
  line, B5; the APU's power-on is covered by blargg `apu_reset`).

### B4. Taken-branch delay of NMI — keep, documented divergence
- Zaxnes delays IRQ **and** NMI on a taken non-crossing branch (`cpu/cpu6502.rs:733-745`, per
  `cpu_interrupts_v2`); Mesen delays IRQ only. Zaxnes follows the documentation.

---

## C. PPU — rendering pipeline

### C1. `$2007` read/write state machine and bus conflicts — done (option, off by default), model wrong in detail
- Hardware: ALE/Read state machine 3-4 dots after the CPU access; collisions with the fetch cadence
  (AccuracyCoin `$2007 Stress Test`, `ALE + Read`; source comments of AccuracyCoin.asm 16329+).
- Zaxnes: `emu.ppu_bus_conflicts` (`read_buffer_from_fetch` `ppu/mod.rs:1656`,
  `refetch_with_low_byte` `:1726`). Off by default since step 17.
- Known defect: on *Super Mario Bros. 3* (Japan) title, line 193, it produces empty frames and a
  segment left of the last square, which neither a console video nor any emulator shows; the
  video's "2 px + gap + 8 px" shape (36 % of frames) is reproduced by no emulator. Study branch
  `study/smb3-split`. Evidence in memory `fidelity-lot` and `docs/progress/FIDELITY.md`.
- Mesen: `$2007` read ignored for 6 dots after a read (`NesPpu.cpp:375-398`), increment delayed one
  dot (`:1505-1510`), no ALE model. BeesNES: immediate buffer (`LSNPpu2C0X.h:1239-1270`).

### C2. `$2006` delayed load and hybrid address — done (option, same switch as C1)
- Zaxnes `delay_address_load` (`ppu/mod.rs:1768`).
- **Not done: the increment-collision glitch** — a `$2006` load landing on the dot-257 or on a
  coarse-X increment dot ANDs the new value with the incremented one (nesdev "PPU registers": "at most one axis of scrolling will be set to the bitwise AND of the written value and the current value"). Mesen
  (`NesPpu.cpp:1463-1490`, option `EnablePpu2006ScrollGlitch`) and BeesNES
  (`LSNPpu2C0X.h:160-182`) both have it. Free (cold path). → plan step.
- **Not done (documented, nesdev "PPU registers": "Only writes that start on dot 257 and continue through dot 258", "writes the value of open bus"): `$2000` write at dot 257 updates `v` from the open bus** (BeesNES `GlitchyVUpdate`,
  `LSNPpu2C0X.h:2003-2015`). Free.
- **Both deferred to the Fidelity+ core (decision of plan step 1.4/1.5, 2026-11-13)**: no test ROM
  checks either (AccuracyCoin has none), Mesen has both off by default
  (`EnablePpu2000ScrollGlitch`, `EnablePpu2006ScrollGlitch`, `SettingTypes.h:651-652`), and
  *Super Mario Bros. 3* writes `$2000`/`$2006` around dots 254-258 of its split line — with the
  one-dot lead of our register accesses (A3) the glitch would fire on the wrong side of dot 257
  and break a game that works today. To do together with the alignment variants (A4) and the
  half-phase clock (A1), which decide on which dot a write lands; then calibrate on the SMB3
  console video (95 frames, `study/smb3-split`).

### C3. `$2001` effects — done
- nesdev "PPU registers": "Toggling rendering takes effect approximately 3-4 dots after the write"; PPUMASK writes are ignored "until the first pre-render scanline" (check Zaxnes).
- Two-dot delay (`switch_rendering`), stale BG shifters with serial input 0/1, reload missed
  (`skip_background_reload`), sprite units (`sprite_hw.rs`). BeesNES: one-dot `CDelayedValue`
  (`LSNPpu2C0X.h:2040`); Mesen one dot.
- Not modelled: the **left-column red/green show delay** BeesNES has commented out
  (`m_dvLeftShowRedGreenDelay`, `LSNPpu2C0X.h:139`); the "wrong value for one dot when writing
  `$2001` directly" that AccuracyCoin avoids with `$3E01` (asm 16384) — open, cost free. Step 33
  netlist (hold-time bench): `$2001` and `$3E01` give the same switch-off/on pixel edges in all four
  alignments, so the wrong value does not reach the rendering switch there; its effect on
  greyscale/emphasis for one dot is not measured.

### C4. Sprite evaluation — done (`ppu-exact-sprite-eval` in default)
- Hardware-exact evaluation state machine; misaligned OAM2 address, frozen increment, overflow bug.
- Open: sprite-overflow bit difference in *The Legend of Zelda* vs Mesen (to recheck since exact
  eval is in default).

### C5. `$2002` / PPU open bus decay — settled, no change (FIDPLUS-1)
- Zaxnes: per-bit decay stamps (`DECAY_DOTS`, `ppu/mod.rs:82`, `apply_decay`). Differs from Mesen on
  some reads (NMI-MESEN.md). To settle against the doc.
- **FIDPLUS-1 (2026-11-14)**: the differences NMI-MESEN.md lists are the five low bits of `$2002` in
  eight ROMs, values the programs drop. nesdev gives the decay as "about 600 ms" and console-dependent
  (`ppu_open_bus` only asks for it within a second); Zaxnes decays after 3.2 M dots (about 600 ms),
  Mesen after its own frame count, both inside that range, both passing `ppu_open_bus` and
  AccuracyCoin's open-bus rows. No reference tells them apart: nothing to change.

### C6. Palette read on pre-G PPUs — done as an option (`emu.ppu_revision`, 2C02G default, step 37)
- Hardware (nesdev "PPU registers": palette read "supported by the 2C02G, 2C02H, and PAL PPUs"): before 2C02G, `$2007` reads at `$3F00-$3FFF` do not return palette RAM (BeesNES TODO,
  `LSNPpu2C0X.h:1242-1246`). Zaxnes always returns palette (`ppu/mod.rs:1617`). Free; needs a
  "PPU revision" setting (see G1).
- **Step 37**: nesdev also says "On PPUs that do not support reading palette RAM, this memory range
  behaves the same as the rest of PPU memory": with "2C02 before G" the read returns the buffer and
  refills it from the nametable byte underneath (`read_data`), as for any other address. Not
  measurable on the netlist (Visual2C02 is a 2C02G die). Default 2C02G, unchanged behaviour.

---

## D. PPU — OAM

### D1. OAM row corruption — done (option `emu.oam_corruption`, on by default)
- Zaxnes `rendering_switched` (`ppu/mod.rs:677`), `corrupt_oam_rows` (`:801`); `emu.oam_corruption`.
- Off because blargg `5.Emulator` test 3 fails with it; BeesNES has the same model
  (`OamCorruptionSeed`, `LSNPpu2C0X.h:1955-1971`) **always on** and passes thanks to B1.

- **Transistor measurements (VisualNes, step 7, 2026-11-14).** Bench `benchoam.cpp` (outside the
  repo): OAM filled with byte = address, program switching rendering off then on 18 dots later every
  4 lines, sliding one dot per try (`STA $2001` #$00 / #$1E, 455-cycle loop), OAM read back two
  lines after and restored; 4 alignments, ~650 tries each.
  - `/rendering_disabled` changes 2 dots after the dot where a read of the same cycle samples (D+2)
    in alignments 0, 4, 6 and D+1 in alignment 2; inside the dot it changes at half-step 0, 6, 4, 2
    (alignments 0, 2, 4, 6).
  - **The netlist corrupts OAM only when the switch lands in the second half of a dot**
    (alignments 2 and 4: 588 and 528 tries changed OAM); alignments 0 and 6 (switch in the first
    half): 3 and 2 changes in ~650 tries. In the clear phase (dots 1-64), a switch on odd dot
    2k+1 overwrites row k with row 0, as AccuracyCoin's notes say; on even dots the netlist also
    copies row k over row 0 and row k+1, and the re-enable 18 dots later (also in the clear phase)
    adds its own copy, so the pairs are entangled.
  - **Not taken as the reference**: AccuracyCoin's author measured the corruption on all four
    alignments of a console (delay 2 or 3 dots); a corruption that exists in only half of the
    netlist's phases is the mark of the zero-delay simulation (DRAM bit lines and precharge have
    no analogue behaviour there). Zaxnes keeps AccuracyCoin's model (row = secondary OAM address
    at the switch, copied from row 0 at the next rendered dot), which passes `OAM Corruption`.
  - Re-run with the re-enable ~96 dots after the switch (`oam2.bin`): identical results, so the
    copies come from the switch off, not from the re-enable.
  - Dots 257-320 (sprite fetches): the netlist overwrites **row 0 with row k** (k = secondary OAM
    address, frozen on dots 3-7 of each 8-dot group, as AccuracyCoin describes), plus row k+1 with
    row k on the dots where the address climbs. **AccuracyCoin measures the opposite direction on a
    console** (row k gets row 0) in that very phase, and Zaxnes passes that test. Evaluation phase
    (65-256): mixed, sprite-dependent copies in both directions.
  - **Conclusion**: the netlist's secondary OAM address per dot matches AccuracyCoin's notes
    exactly, but the copy itself (whether it happens, its direction, the extra row) is decided by
    charge on the OAM bit lines, which a zero-delay switch-level simulation does not model. The
    copy rule therefore stays AccuracyCoin's console measurement. What is still unknown on
    hardware: the even dots of the clear phase (the netlist's extra copies), and the evaluation
    phase beyond "rounded up to a multiple of 4". Settling them needs a console, not the netlist.

- **Other emulators (2026-11-14).** BeesNES (`OamCorruptionSeed`/`OamCorruptionApply`,
  `LSNPpu2C0X.h:1955-1990`): Zaxnes's rule exactly (dot/2 in 1-64, address rounded up to 4 in
  65-256, the 8-dot pattern in 257-320, row 0 copied at the next rendered dot), no alignment. Mesen 2
  (`NesPpu::SetOamCorruptionFlags`, GitHub): the same in 0-63 and 256-319, **nothing in 65-255**.
  NesHawk (BizHawk 2.11.1): none (a commented-out `$2003` glitch only). puNES (`src/core/ppu.c`):
  none. nesdev "OAM internals": the mechanism (columns precharged in the first half of every dot,
  a row selected in the second; changing rows without precharge writes the old row's data into
  the new one, "usually" row 0 onto the next; depends on process, voltage, temperature, CPU/PPU
  alignment and register decoding) — which is what the netlist shows in its two "second half"
  alignments.
- **TriCNES** (Chris Siebert, AccuracyCoin's author; MIT; `github.com/100thCoin/TriCNES`, copy in
  `G:/3DSen/ext-tools/tricnes-src/Emulator.cs`): the most detailed model found, alignment = its
  `PPUClock & 3` (AccuracyCoin's numbering). Read 2026-11-14:
  - `$2001` (`Emulator.cs:10055-10066`): rendering delay 2/2/3/2 dots, OAM-corruption seed delay
    2/3/3/2, emphasis delay 2/1/1/2 for alignments 0/1/2/3.
  - **Re-enabling rendering outside vblank with a corruption pending: on alignments 1 and 2 the
    corruption does not happen** (`:10096-10108`; re-enabled in vblank it happens at the
    pre-render line on all four). This matches the netlist (corruption in 2 of 4 alignments when
    re-enabled mid-frame) — the netlist result is not an artefact on that point.
  - Clear phase (`:3255-3325`): seed index = secondary OAM address, taken on odd and even dots
    with different paths (instant vs delayed switch).
  - Evaluation (`:3327-3545`): odd or even dot matters (`OamCorruptedOnOddCycle`, OAM address
    incremented again), whether the last object was in Y or X range matters, rounding up to 4
    only when the address is not a multiple of 4, alignment 0/3 vs 1/2 branches, and dot 256 gives
    row 0 or 1 ("I have no idea").
  - Sprite fetches (`:3554-3560`, `:3721-3727`): seed taken on alignments 0/3 at one point of the
    dot and 1/2 at another.
  - Write delays (`:10170-10225`): `$2005` 1/1/2/1, `$2006` load 4/4/5/4 for alignments 0-3: **one
    alignment is a dot later**. The netlist also shows one alignment differing for `$2001` and
    `$2006` (a2, one dot *earlier*, with store times that may be early); so the claim of step 28
    that write timings do not depend on the alignment is **not settled** and must be revisited
    (mapping TriCNES 0-3 onto the netlist's a0/a2/a4/a6 first).
  - Palette RAM corruption (D5, `:10077-10091`): rendering off during the first 2 dots of a
    nametable fetch (dot & 7 < 2, dot <= 250) with `v` >= `$3C00` corrupts the palette (Fiskbit's
    explanation in the comment: the AT input of the address mux briefly sources from `v`).
  - Other source to read: the MiSTer NES core (a fork has an "accurate PPU OAM corruption/decay"
    switch), Breaks (2C02 reverse-engineered from the die), the nesdev threads t14353, t18339,
    t18284.
- **Differential port of TriCNES (steps 29a/29b, 2026-11-14).** TriCNES built and run headless
  (patched copy in the scratchpad: OAM2 index masked to 0-31 to avoid its out-of-range crash, a
  hook logging each `CorruptOAM`). Three NROM programs run in both emulators with OAM filled with
  byte = address, the switch-off sliding one dot per frame, OAM read back and restored after the
  re-enable: P1 off near line 100 and on in vblank, P1b the same at line 20, P3 off at line 60 and
  **on again ten lines later** (mid-frame). Compared per (line, dot) of the switch.
  - Result: 0 differences on 10 of the 12 program × alignment pairs, apart from 3 single events
    (a2 and a3, dot 65) where the two runs did not have the same previous event, which matters
    there (below); P3 in alignments 1 and 2 differs on 41 events each, by choice (next point).
  - **TriCNES fails AccuracyCoin's own `OAM Corruption` test in alignment 2** (TriCNES run on the
    ROM, 3 billion master clocks, result byte `$47B` = `$0E`, code 3: OAM must still be intact
    right after the switch-off; 147/147 in alignments 0 and 3, 145 in 1 and 2). The cause is its
    extra row of the clear phase, copied at once. AccuracyCoin's results were measured on a
    console in every alignment, so on that point the console contradicts TriCNES: Zaxnes keeps
    that extra row pending (copied at the re-enable, dropped by a mid-frame re-enable in
    alignments 1 and 2, which is where P3 then differs from TriCNES). With that, AccuracyCoin is
    147/147 in all four alignments.
  - Rules found (now in `rendering_switched`): the seed is a dot later in alignment 1; extra rows
    on even dots of the clear (row k-1), on every dot in alignment 2; in alignment 2, and on even
    dots in alignment 1, the evaluation row is copied **at once** (the evaluation still sees the
    rendering on), so a mid-frame re-enable, which drops the pending seed in alignments 1 and 2,
    keeps it; in alignment 2 the row after it is the pending one when the evaluation copied a byte
    on the dot before; dot 65 takes no new row and copies the row the previous switch-off seeded
    (TriCNES `Dot == 64` path that leaves `PPU_OAMCorruptionIndex` as it was); alignment 1's last
    fetch dot (320) seeds row 0.
  - Still TriCNES's model, not a console measurement: where TriCNES says "I have no idea" (dot 256)
    the port copies it. The stale-row behaviour of dot 65 is the most fragile point.


### D2. OAM decay when rendering is off — done as an option (`emu.oam_decay`, off, step 20)
- Zaxnes (step 20): `Ppu2C02::refresh_oam_row` / `refresh_oam_rows_on` (`ppu/mod.rs`), one clock
  stamp per 8-byte row, checked only on a `$2004` access, an OAM DMA byte and the rendering coming
  back; the clock of the last switch-off stands for the rendering's own refresh. 8 NTSC frames
  (714 736 dots; 708 214 on PAL/Dendy for the same 133 ms). Decayed bytes as BeesNES: each byte
  its own index, attribute bytes `& $E3`. Zero cost per dot.
- **Approximations left**: (1) the stamps are not in the savestate (format unchanged), so a load
  restarts every row's delay — run-ahead and rewind, which load a state every frame, never let OAM
  decay; this is why the option is off by default. Fidelity+: put the 33 stamps in the state
  (format bump). (2) Rendering switched on only during vblank (never on a visible line) counts as
  a refresh at its switch-off although no evaluation ran. (3) Real decay is per bit, towards a
  console-dependent value, gradual (BeesNES has a commented float model); ours is all at once.
  (4) PAL's forced refresh on lines 265-310 is not modelled separately (covered when rendering is
  on). (5) The rendering's refresh on the pre-render line and the dots 1-64 clear are not
  distinguished.
- Hardware: OAM is DRAM refreshed by rendering; with rendering off it decays. blargg `oam_stress` and
  `sprdma_and_dmc_dma` (verified on hardware) keep it for ~123 ms and ~37 ms.
- BeesNES: decay after 8 NTSC frames (`OamDecayRate`, `LSNPpu2C0X.h:1774-1792`; `DecayOam` called
  from `Tick` `:148-158`).
- Zaxnes: none. Free if computed lazily (time since rendering went off, applied on the next OAM read
  or rendering start). Value of decayed bits to settle (BeesNES float decay per byte).

- **Doc vs tests conflict on the duration**: nesdev "PPU OAM" says decay starts "quickly", lasting
  "at least as long as an NTSC vertical blank interval (~1.3ms), but not much longer"; BeesNES keeps
  8 frames because blargg tests verified on hardware keep OAM for ~123 ms. To settle before coding
  (console-dependent; maybe a setting). nesdev also: the 2C07 (PAL) "forcibly refreshes OAM during
  scanlines 265 through 310", so no decay on PAL while rendering stays enabled.

### D2b. Performance lead found on the way (lot FIDELITY, not fidelity) — tried and dropped (FIDPLUS-1)
- Callgrind `f4185fa8` vs `27e2c3e2`: the APU stage costs +0.8 to +0.9 % on every game (+17 to
  +21 M instructions per 600 frames) while the PPU stage gains 0.24 %; totals are -0.03 to
  -0.14 %. The APU changes of the lot are frame-counter (`Frame Counter IRQ` I-M) and DMC
  implicit-abort code that runs a few times a frame, so the cost is likely codegen around
  `Apu2A03::run_cpu_cycles` / `step_cpu_cycle` (match arms `3 =>` / `_ =>`, `frame_irq_clear_at`).
  Line by line (smb3, joined on source text): `self.cycles += 1;` in `step_cpu_cycle`
  (`apu/mod.rs:690`) +13.6 M and `if self.frame_irq_status` in `compose_status` (`:1001`) +6.8 M,
  the rest flat: register allocation of the per-CPU-cycle path, not the new code itself. Leads:
  bit 6 branchless (`u8::from(..) << 6`); compose `readable_status` only when a `$4015` read is
  coming instead of every cycle (bigger change).
- **FIDPLUS-1 (2026-11-14), dropped.** The batch idea holds (a `$4015` read only sees the value
  latched on the last cycle of the APU batch run before it), but avoiding the other compositions
  costs more than they do: two inlined copies of the step (`step_cycle::<LATCH>`) took the APU stage
  **+3.2 to +4.4 %** (games +0.3 to +0.7 %); one call site with a `bool` still **+0.24 to +0.29 %**
  (games +0.03 to +0.06 %), callgrind against `6d252c10`. The composition is a handful of loads and
  ORs the per-cycle path absorbs; nothing to gain here.

### D3. 2C02G/H OAMADDR bug at pre-render — done (option `emu.oamaddr_bug`, on, step 19)
- Hardware: at dot 1 of the pre-render line with rendering on, if OAMADDR >= 8 the 8 bytes at
  `OAMADDR & $F8` are copied over OAM 0-7 (2C02G/H; nesdev "PPU registers" OAMADDR).
- BeesNES `PreRenderOamGlitches` (`LSNPpu2C0X.h:1990-2001`), README "Hardware bugs... OAMADDR bugs
  (writing fewer than 8 bytes on the 2C02G)".
- Zaxnes: `Ppu2C02::oamaddr_row_copy` (`ppu/mod.rs`), skipped on PAL. Free (pre-render line, once per frame).

### D4. 2C02G: writes to OAMADDR corrupt OAM — done as an option (`emu.oamaddr_write_corruption`, off, step 37)
- nesdev "PPU registers" OAMADDR: "On the 2C02G, writes to OAMADDR corrupt OAM"; on 2C03/2C04/2C05/
  2C07 "OAMADDR works as intended"; the pre-render copy of D3 is "required for 2C02 compatibility" on
  the Dendy. Zaxnes: none.
- nesdev details (forum threads cited there): "this usually seems to copy sprites 8 and 9 (address
  $20) over the 8-byte row at the target address", "the source address ... seems to come from the
  previous value on the CPU BUS", "There may be other possible behaviors as well."
- **Netlist (step 37, `benchoam2.cpp`, hold-time bench, rendering off, `STA $2003` so the bus held
  `$20`)**: in phase a0 every write copies row `$20` over the row at the written address (the first
  byte, and the byte at the written address, ORed with the target's) — nesdev's description; in
  a4 the copy runs the other way, the row of the *previous* OAMADDR over row `$20`. Phase-dependent:
  a race again.
- **Done as an option, off by default**: `oamaddr_write_corruption` copies the row the CPU's
  previous bus byte points to over the written row (nesdev's console description; the OR details
  and the reverse phase not modelled). Switched on it fails blargg `oam_read`, `oam_stress`,
  `cpu_dummy_writes_oam`, `ppu_read_buffer` and AccuracyCoin's `Address $2004 behavior` in all four
  alignments — tests that pass on their authors' consoles, so those consoles do not show it as
  modelled. Off under "2C02 before G" whatever the switch says.

### D5. Palette RAM corruption when rendering is turned off mid-screen — done as an option (`emu.palette_corruption`, alignment 2 only, step 32)
- nesdev "PPU registers" PPUMASK: "Turning rendering off mid-screen can corrupt palette RAM."
- TriCNES (MIT, `Emulator.cs` `CorruptPalettes`, step 29 reading) models it, **in alignment 2
  only** ("consistent on alignment 2, and very hit or miss on other alignments"): triggered when
  rendering stops on a visible dot during a nametable fetch with `v` at `$3C00` or above, or when
  a `$2006` load leaves palette RAM (`v` from `$3F0x` with x not 0 to below `$3F00`) on a visible
  dot; the colours damaged follow a per-colour-index × low-nybble-of-`v` table found with a test
  cart, which the author calls "good enough" and partly not understood ("not sure what's going
  on"); sprite palettes not done; plus one discoloured pixel on every alignment, not modelled.
- AccuracyCoin has no test for it (its table lists "Palette Corruption" unimplemented, "because
  it relies on a specific cpu/ppu clock alignment"). So no reference to check a port against
  but TriCNES itself; a transistor netlist cannot settle it either (analog races: TriCNES quotes
  "magic numbers", "probably a temperature thing").
- **Step 32, ported** (`ppu/palette_corruption.rs`): the table is generated from TriCNES's
  source by `conv_palette.py` (138 assignments, not retyped); triggers in `switch_rendering`
  (`switch_off_corrupts_palettes`: TriCNES's write dot is this core's write dot, measured, but
  its `v` is the one before this dot's coarse-X increment) and after `$2006` loads
  (`palette_load_corruption`); the colour index is the next pixel's (`next_pixel_color_index`,
  no shift on dot 1). Differential against TriCNES in alignment 2 (scratchpad `tric/`, programs
  P10 `$2006` into palette RAM and out, P11 rendering off on line 99 with nametable 3, fine Y 3):
  P11 0 palette differences, P10 one. Behind `emu.palette_corruption` (on; it acts only in
  alignment 2).
- **Palette background** (same step, no switch: documented hardware): rendering off with `v` in
  `$3F00-$3FFF` shows the entry `v` points to (nesdev "PPU palettes"; Mesen 2 `DrawPixel`;
  AMD-050 already names the case, opacity bit 0). `backdrop_entry` in the renderer.
- **Edges measured on the netlist (step 32, `g2001px.bin`, `g2006px.bin`, `bdana.py`)**: in the
  default alignment (a4) a `$2001` switch off made on D shows the backdrop from pixel D + 2 (as
  this core already drew it), a switch on shows tiles from pixel D + 3 (done in step 33 without a
  dot run ahead: the pixel is set at the end of its line; per alignment in A3c), a `$2006` load
  with rendering off shows its palette
  background from pixel D + 2 (this core loaded at once: `delay_backdrop_address` runs two dots,
  every alignment; TriCNES's 2/3/4/3 dropped). Other netlist alignments: switch off D + 1 (a6,
  a0 and a2 mostly), switch on D + 3 (a2 mostly D + 2), `$2006` D + 2 (a2 D + 1 or 2) — left as
  the step 29 timings for alignments 0 to 2 until the alignments are mapped.
- **Still open**: re-enabling rendering in the middle of a tile fetch. TriCNES reads on the
  second dot of each fetch pair, with an address made of the pattern-address register's high
  byte (left by the last fetch, sprite fetches included) and the octal latch's low byte (left
  by the last ALE, `$2006`/`$2007` included); this core reads on the first dot and skips the
  fetch whose first dot had rendering off. P12 (rendering on mid-line) differs on 64 of 379
  frames in every alignment, all of this kind. Needs those two registers modelled, hot path
  included, and a netlist measurement first.
- **Step 39, measured and done (no switch)**. Netlist with the CPU's hold time (`bench2007h`,
  `g2001px.bin`, four phases, `visualnes-bench/s39`; scripts `onlag.py`, `staleana.py`,
  `aleoff.py`):
  - a `$2001` write on `D` releases the rendering on `D + 2` in all four phases (switch off and
    on alike, the nearest write before every edge);
  - no ALE happens while the rendering is off; a switch off on `h` leaves the ALE of `h` when `h`
    is odd (it comes first within the dot), else that of `h - 1`;
  - a switch on landing on an odd dot (the dot of a fetch's ALE) does that fetch: in a0 and a2
    (alignments 1 and 0) the ALE comes after the release and the fetch reads its own address; in
    a4 and a6 (alignments 3 and 2) the ALE came before, and the read of the next dot puts the
    high byte of the fetch's own address on the pins (the nametable range for a nametable fetch,
    `$23xx` for an attribute) and the low byte the octal latch kept, that of the last ALE:
    TriCNES's octal-latch read, with the high byte the fetch's rather than the previous one's;
  - on an even dot the next fetch is the usual one, in every phase.
  This core ran that odd dot under the old mask and skipped its fetch in every alignment.
  `first_fetch_on` (cold, on the `$2001` path) now does it, with the stale low byte in
  alignments 2 and 3; the last ALE's low byte (`ale_low`) is taken where the bus stops — the
  switch off, the end of a rendered frame (the nametable fetch of dot 339), `$2007` accesses —
  and kept in the savestate in spare bits of two existing bytes (format unchanged). Approximation
  kept: a switch off whose last ALE was on dot 255 takes its address after the dot-256 Y
  increment. The board sees the read, as on the console. Test
  `fidelity_switch_on_fetch_reads_the_stale_low_byte_in_alignments_2_and_3`. AccuracyCoin
  147/147 in the four alignments, campaign 213/6, golden identical to step 37.

---

## E. APU and audio output

### E1. Console output chain — done as an option (`audio.console_profile`, none, step 22)
- Hardware: the mixed signal goes through RC filters that differ per console model and revision.
- Zaxnes: one DC-blocking high-pass at 20 Hz (`apu/band_limited_resampler.rs:6`, `:104`).
- BeesNES: measured profiles per console serial (`Src/Audio/LSNAudioOptions.cpp:20-36`): LPF,
  pole-zero, three HPFs, per-channel volumes (pulse 1/2, triangle, noise, DMC), invert, exact measured
  CPU clock of each unit (e.g. 1 789 806.677 Hz). Models: NES front-loaders (6 serials, one with
  Sunsoft 5B), top-loader, Twin Famicom (3), PAL NES (2). Chain built in `Src/Apu/LSNApu2A0X.h:249-261,
  446-450, 725-741`. MIT, reusable with notice. Verified by the author against recordings (README
  "Audio Accuracy", MDFourier).
- Cost: a few biquads per output sample; zero when the option is off.
- **Done (branch `feat/fidelity-poweron`, step 22)**: `apu/console_profile.rs`, the 14 profiles
  verbatim; gains in the mix (exact nesdev formulas, cold `profile_mix`, only when the level
  moves), filters once per output sample in `BandLimitedResampler::drain` in place of the 20 Hz
  DC blocker, letterless noise (N0260073). Default "none" is bit-identical (golden-core audio
  unchanged). **Approximations left, for Fidelity+**: (1) BeesNES's low-pass runs at the CPU rate
  as a pole-zero filter; ours at the output rate, pole pre-warped, **zero dropped** (it sits at
  144-189 kHz, above any output Nyquist) — to do properly, run it on the band-limited signal
  before decimation (per-step cost); (2) the overall profile volume (0.34 to 4.5) is not applied,
  only its sign: our output is normalised to full scale, applying it would clip; (3) BeesNES's
  sample box applies HPF0 inside its windowed-sinc resampler, ours after our own resampler (same
  one-pole formula, a few samples of phase apart); (4) the expansion audio (VRC6, MMC5, N163...)
  is added after the gains and is not weighted per console, while BeesNES lets the mapper
  post-process between HPF1 and HPF2 (`PostHpf`, `LSNApu2A0X.h:1919-1931`); (5) filter state is
  not in the savestate (a load restarts it at rest: inaudible click at most); (6) the
  N20914398 profile was measured with a Sunsoft 5B cartridge and has no low-pass and no
  inversion — kept as BeesNES lists it.

### E2. Per-console clock — rejected (no effect while the pacing follows the display; step 38)
- Each console's crystal differs (E1 table). Option "measured clock of console X" changes speed and
  pitch slightly.
- **Step 38**: this core paces at the nominal 60 frames a second (`RegionTiming::
  nominal_frames_per_second`), locked to the display, and derives the audio samples per frame from
  it (`apu/mod.rs`, `nominal_frames_per_second * speed`): it already departs from the NTSC 60.0988 Hz
  by 1 640 ppm on purpose. BeesNES's measured clocks differ from the nominal 1 789 773 Hz by tens of
  ppm (1 789 806.677 Hz: +19 ppm). Such an option would change nothing a player can hear or see
  while the pacing follows the display; it only makes sense with a pacing on the real clock (a
  Fidelity+ item: A2 "real-time pacing"), where it should be added then.

### E3. APU sub-cycle — reference
- BeesNES claims the APU completely sub-cycle accurate (README). Our APU passes every APU test ROM we
  run; nothing known missing. Re-check with BeesNES code when Fidélité+ starts.

---

## F. CPU

### F1. Per-half-cycle bus activity reference — done as a test (step 23)
- `Research/Instructions/Ricoh 2A03 Instructions NES.xlsx` (BeesNES, 15 069 rows): every opcode,
  every half-cycle, bus address and R/W, notes. Use: generate a table and compare with Zaxnes's
  dummy reads/writes per opcode in a test (cheap, no runtime cost).
- **Done (branch `feat/fidelity-poweron`, step 23)**: `rust/tools/cpu_bus_reference.py` turns the
  xlsx into `zax-nes/tests/data/cpu_bus_reference.txt`; `tests/cpu_bus_reference_tests.rs` runs
  each of the 244 non-jam opcodes on a recording bus in two scenarios (no page crossed / branch not
  taken; every indexed access across a page / branch taken across a page) and compares direction
  and address class (PC, stack, vectors, data) of every access: **all 488 match**. Not checked
  (would need the table's pseudo-code evaluated, i.e. a second CPU): exact addresses and values,
  the SH* high-byte AND and RDY cases (covered by AccuracyCoin), interrupt polling points (the table
  marks them, "Poll interrupts", per cycle: a lead for checking our NMI/IRQ latency per opcode),
  and the jams' `$FFFE`/`$FFFF` read pattern. A stronger reference would be the SingleStepTests
  `nes6502` suite (exact address/value per cycle, 10 000 cases per opcode, about a GB of JSON; not
  downloaded).

### F2. Unofficial opcodes, SHA/SHS/SHX/SHY with DMA — done
- Passed in AccuracyCoin (Zaxnes 147 with options). BeesNES "fully supported".

---

## G. Hardware variants

### G1. PPU / CPU revisions — done as a menu (console model, FIDPLUS-1)
- BeesNES exposes none of its hardware-bug behaviours as user settings (only video, audio profile,
  region); OAM decay is a compile-time define (`LSN_INT_OAM_DECAY`, `LSNPpu2C0X.h:33`), OAM corruption
  is always on, despite the README's "buggy and fixed states".
- 2C02 letter revisions (pre-G palette read C6, G/H OAMADDR bug D3, OAM corruption differences),
  RP2A03 letter revisions (BeesNES flag `bRp2A02` in audio profiles), 2C07 PAL, 2C05 Vs.
- Zaxnes: none selectable. A "console model" setting would group C6, D3, E1, E2.
- **FIDPLUS-1 (2026-11-15)**: menu bar, Emulation > Console model
  (`zax-app/src/persistence/console_model.rs`). A model is a shortcut, not a new setting: it sets
  the region (detection off), `emu.ppu_revision` (C6) and `emu.oamaddr_bug` (D3), only where a
  reference names the value (nesdev: palette reads on 2C02G, 2C02H and PAL PPUs; the OAMADDR defect
  on the 2C02G/H). NES/Famicom 2C02G-H: G, defect on; 2C02 before G: before G, defect off; PAL NES
  2C07: G (palette reads), defect off; Dendy, PAL-M, PAL-N: region only. The menu ticks the model
  whose values are all in force, so a hand change leaves none ticked; nothing is persisted.
  Left out: E1 (`audio.console_profile` is one measured console, not a model; the player picks it
  separately), E2 (rejected), the RP2A03 letter revisions (only the "letterless" noise of the
  audio profiles, already tied to those). The PAL 2C07's OAMADDR behaviour is not documented here:
  "off" is the absence of the 2C02G/H defect, not a measured 2C07 behaviour.

### G2. Regions PAL-M and PAL-N famiclones — done (FIDPLUS-1)
- BeesNES: per-region cycle tables (`Src/Ppu/LSNCreateCycleTablePalM.inl`, `...PalN.inl`) and
  master clocks (`LSNSystem.h:1182-1190`). Zaxnes: NTSC, PAL, Dendy, Multi (`region_timing.rs:45-47`).
- **FIDPLUS-1 (2026-11-15)**: `NesRegion::PalM = 4` and `PalN = 5` (persisted numbers), one timing
  row each in `region_timing.rs`, from BeesNES (MIT):
  - PAL-M: master 3 067 875 000 / 143 Hz, CPU / 12 = **1 787 806 Hz**, PPU / 4 (3 dots per cycle),
    NTSC geometry (262 lines, vblank 241, `LSNLSpiroNes.h:213-221`), **no** odd-frame dot skip
    (`CPalMPpu`, `LSNPpu2C0X.h:2823`), 60 fps nominal;
  - PAL-N: master 42 984 675 / 2 Hz, CPU / 12 = **1 791 028 Hz**, PPU / 4, PAL's 312 lines with the
    Dendy's 51 post-render lines (vblank 291, `LSNLSpiroNes.h:224-232`), **with** the odd-frame dot
    skip (`CPalNPpu`, `LSNPpu2C0X.h:2830`), 50 fps nominal;
  - both: NTSC frame-counter steps (`LSNApu2A0X.h:152-176`) and NTSC noise/DMC periods
    (`LSNApuUnit.cpp:27-28`); pulse duty sequences 1 (25 %) and 2 (50 %) **swapped**
    (`_bSwapDuty`, `LSNSystem.h:1182,1190`; `GetDuty`, `LSNApu2A0X.h:1891`), applied on the
    `$4000`/`$4004` write (`PulseChannel::set_swaps_duty`). The MMC5 squares keep the normal order.
  - FCEUX and NesHawk have neither region: PAL-M runs their NTSC and PAL-N their Dendy (the
    geometries), without the clock or duty differences; NesHawk blocks keep the host's choice.
  - Single source: BeesNES. The odd-frame skip of PAL-N and the duty swap have no second
    reference here. The handheld options row still offers Auto/NTSC/PAL only; the menu bar offers
    all five.

---

## H. Video output (shaders lot, not core)

### H1. Composite / RF signal, CRT gamma, phosphor decay — open
- BeesNES: NTSC-CRT / PAL-CRT (LMP88959), own L. Spiro filters, measured CRT gamma, phosphor
  decay, signal noise (README "Visual Samples"); palette generator palgen-persune.
- Zaxnes: librashader slang shaders (branch feat/librashader). These are generic shaders, not a
  signal model. Candidate shaders/palettes to port; not a core item.

---

## I. Open questions from tests and comparisons

- I1. SMB3 Japan line 193 shapes (C1): console video shows "2 px + gap + 8 px" 36 %, 8 px 31 %,
  16 px 22 %, holes 12 %, never empty; BeesNES/puNES/NesHawk/Zaxnes default: 8 or 16 px; Mesen: empty
  75 %. Same `$2006`/`$2007` timing in Mesen and Zaxnes (W 212, R 224, v=$0B02). Branch
  `study/smb3-split`.
  Other versions (2026-11-13): the split IRQ code of the fixed bank is byte-identical in Japan and
  USA (same addresses, `LDA $2007` at `$F824/$F902/$F9D9/$FA63/$FC09`; the `$F9C7` handler writes
  `$2001` = `$10`, sprites on, before its `$2006`/`$2007`, so that read happens while fetching);
  Europe has the same routines shifted. With `emu.ppu_bus_conflicts` on/off, frames 200-2500: USA
  never differs; Europe differs on line 194 on some frames (200, 300, 350, 900, 1500). Why USA does
  not move is not checked (IRQ landing dot). A PAL console capture of the European title screen is a
  second source to compare with, PAL timing being less verified (I4).
  **PAL console video** (French PAL NES, composite AV mod, RetroTINK 2X, Elgato HD60 S, 25 fps,
  492x360, title screen 400 frames): the white separator line (line 194) is damaged **only on its
  first ~8 pixels**, on 34 % of the frames (17 % fully dark, 17 % partial), normal elsewhere. Zaxnes
  Europe with `emu.ppu_bus_conflicts` (step 25), frames 200-359: 34 % damaged at x 0-7 of line 194
  (26 % fully dark, 8 % `###.###.`), never elsewhere; option off: never damaged. Same place, same
  rate: first hardware support for the transistor `$2007` read model. Limits: one video, every other
  frame, composite blur, x calibration within ~1-2 px, shapes of the partial cases not compared.
  **Floor position, same video (2026-11-14)**: lot FIDELITY moved the title floor of SMB3 (USA and
  Europe, white line + checkerboard) from line 195 up to line 194, with every fidelity setting off
  (a non-optional timing change, not singled out; found by `widescreen_zone_tests`). Zaxnes
  frames put through the video's pipeline (x2 line doubling, bicubic to 295 rows, grey), falling
  edges only: ratio (hill bottom -> white line bottom) / (white line bottom -> first checker
  block bottom) = 0.603 for line 195, 0.510 for line 194, **0.496 on the console** (median of 400
  frames). Line 194 is right; the test's golden was updated.
- I2. *The Legend of Zelda* sprite-overflow bit (C4).
- I3. `$2002` open-bus bits and `$2007` reads during rendering vs Mesen (C5).
- I4. Mega Man 2 widescreen not eyeballed; PAL/Dendy fidelity work unverified.

## J. Cartridge boards (lot MAPPERS-N, 2026-11-15)

Boards read from Mesen2 (`cleanroom/third_party/mesen2/Core/NES/Mappers`). Only **mapper 250** is
checked on a real game (*Queen Bee V*); every other board of the lot has unit tests from its source
and no local ROM, so each item below is a lead to verify the day a dump is at hand.

### J1. Outer registers and the RESET button — documented divergence
Mesen2 clears the outer registers of several multicarts on a soft reset (`MMC3_134.h` `Reset`,
`Mapper226.h:24-33`, `ColorDreams46.h` `Reset`, `MMC3_217.h`, `Bmc80013B.h`, `Bmc810544CA1.h`).
This build keeps its rule that a board cannot see RESET (`Mapper::soft_reset`), so those registers
survive a reset and only a power cycle clears them. The exceptions are the boards whose page makes
the reset reaction their function: 60 (`Mapper60.h:18-27`), 230 (`Mapper230.h:19-34`), 233
(`Mapper233.h:14-24`) and 313 (`ResetTxrom.h:9-14`, nesdev "NES 2.0 Mapper 313"). A multicart menu
reached by RESET on a real cartridge may therefore not come back here without a power cycle.

### J2. Open bus answered by a board — approximation
Mapper 212's protection read (`Mapper212.h:19-25`) ORs `$80` into whatever the bus holds; a board
here cannot see the bus, so the high address byte stands in for it (what an absolute read leaves).
Mappers 63 and 235 answer `None` for their unmapped chip, which is the bus's own open-bus value.

### J3. Mapper 224 PRG mode 1 — Mesen2 swaps R6 and R7
`MMC3_224.h:26-31` puts R6 at `$A000` and R7 at `$C000` in PRG mode 1, the reverse of every MMC3.
This build keeps the MMC3 order and only adds the outer bit; one of the two is wrong for a game that
uses mode 1. A dump of a KT-008 cartridge decides it.

### J4. DIP switches — fixed at 0
Mappers 59, 236, 286 and 287 read DIP switches (Mesen2 `GetDipSwitches()`, default 0). This build has
no DIP setting for boards, so the menus a switch would select are unreachable.

### J5. FK23C (176) — subtypes inferred from sizes, soft reset not taken
`Fk23C.h` boots the second 512 KiB when PRG and CHR are both 1 MiB (subtype 1) and swaps `$46`/`$47`
on 16 MiB CHR-RAM images (subtype 2); both are taken. Its soft-reset clear of the PRG base (with the
WRAM config, CHR-RAM selected and a battery) is not, per J1. iNES 1.0 images get Mesen2's 256 KiB of
CHR-RAM and 32 KiB of WRAM.

### J6. A12-clocked counters of 117, 222 and 176 — filter value
They use this build's MMC3 filter (`A12Watcher::MMC3_FILTER_DOTS`), as Mesen2 uses its own single
`A12Watcher` for all boards. Not compared on a game.

### J7. Mapper 253's only local dump is a bad dump
`roms-owner/Excitebike_1_/… vs._excitebike_fds_conversion_b1.nes`: 144 KiB of PRG whose last 16 KiB
are zeros and whose vectors point there; no emulator can run it. The board is tested by unit tests.

### J8. Mapper 198 and 199 left out — superseded by J20 and J21
Mesen2's own comment on `MMC3_198.h` is "Most likely incorrect/incomplete"; 199 needs per-window
CHR-RAM selection by page number. Neither was taken at first; both were later implemented from
nesdev and puNES instead of Mesen2 (J20, J21).

### J9. Mapper 217's NROM mode is held
`MMC3_217.h:46-55` maps the `$5000` bit-7 page pair once, so Mesen2 drops it at the chip's next PRG
update; this build holds it until `$5000` is written again. Which one the BMC-SPC009 does is open.

### J10. Mapper 303 left out — superseded by J25
`Kaiser7017.h` acknowledges its IRQ on a **read** of `$4030`. That needs `Mapper::cpu_read_override`
in `IrqBoard`, the hook whose body cost `mmc5exram` +1.098 % in MF-UNL-12 (`board-292` parked), so the
board was not taken at first. It was later taken (J25): the `IrqBoard` forward is `#[inline(never)]`
and reached only for reads the cached ROM windows do not serve, so `mmc5exram` (an MMC5, not an
`IrqBoard`) does not cross it; the final callgrind run of the lot measures it.

### J11. Mapper 108 follows nesdev's submappers, not Mesen2
Mesen2 `Bb.h` takes one register at every `$8000-$FFFF` address of `(addr & $9000) == $8000` plus
`$F000+`, powering on at `$FF`. nesdev ("INES Mapper 108") and puNES `mapper_108.c:37-87` split the
FDS conversions into four submappers: the register from `$F000` (1), `$E000` (2) or `$8000` (3, 4),
power-on 0, `$6000` fixed to the last bank on 4, CHR banked only from CHR-ROM. Mesen2's reading hangs
the Hackset's Bubble Bobble conversion (`AF799590`, submapper 1 in the cartridge DB) on a black
screen; FCEUX `108.cpp` (writes at `$8000-$8FFF` and `$F000-$FFFF`) runs it, and so does this build
since. Falsion (submapper 2) has no local dump.

### J12. iNES 1.0 CHR-RAM of the MAPPERS-N boards is 8 KiB
REQ-CORE-012 gives an iNES 1.0 image without CHR-ROM 32 KiB of CHR-RAM. On the 111 boards of
`MAPPERS_N_CHR_RAM_8K` (`registry.rs`) this build gives 8 KiB, as Mesen2 (`BaseMapper`,
`GetChrRamSize` 0x2000 unless overridden), FCEUX (`108.cpp`, one 8 KiB `setchr8(0)` page) and
BizHawk (`Cart.VramSize = 8`, `Mapper108.cs`) do: their CHR registers wrap inside it, and a stray
bank number otherwise shows an empty page. Boards that override the size keep their own (6/8/17,
176, 191, 253). NES 2.0 images keep their declared size.

### J13. FK23C CHR-RAM bit without CHR-RAM
Mesen2 `Fk23C.h:160` selects CHR-RAM for the `$5xx0` bit 5 only when the cartridge has CHR-RAM
("_selectChrRam && _chrRamSize > 0"). The first port missed that condition, so the NES 2.0 dump of
Zhi Li Xiao Zhuang Yuan (CHR-ROM only, `$5FF0 = $60`) read zeros; it now matches Mesen2 and FCEUX.

### J14. Battery PRG-RAM powers on as zeros (open, owner's call)
Without a save file, a cartridge's PRG-RAM starts all zero here (`cartridges/mod.rs`,
`prg_ram: vec![0; …]`), as in FCEUX. Mesen2 initialises it like every NES memory from
`RamPowerOnState`, whose default is `Random` (`SettingTypes.h:659`, `NesConfig`). Real SRAM powers on
undefined. *Famicom Jump II - Saikyou no 7 Nin* (`3F15D20D`, mapper 153) takes all-zero save RAM
for valid saved data and jumps into it (`JMP $7E6A` from a bank-2 table): it crashes to a grey
screen here and in FCEUX, and boots to its title with an all-`$FF` or random save RAM
(`zax-harness render --sav`). The mapper itself is right. Changing the power-on state touches
every battery game, the golden and determinism, so it is left to the owner: a seeded pattern
for battery RAM without a save file is the likely fix.
**Applied (owner: "fais ce qui te semble le mieux", 2026-11-15):** `NesCore::power_on_battery_ram`
fills the battery slice with `PowerOnRam::Random(BATTERY_RAM_SEED)` when no save was imported;
movie replay (`movie_sync`) sets it back to zero, FCEUX's convention. Mappers 16, 157, 159 and 164
(serial EEPROM arrays) and 19 (Namco 163 internal RAM) keep zero: a non-zero 163 RAM exposed a
three-byte save/restore mismatch in `req_core_090_the_mapper_state_survives_a_save_and_restore`,
left for a later lot. The 2 KiB work RAM keeps FCEUX's pattern (owner, 2026-11-13); *Dancing
Blocks* (mapper 143) reads its VRAM ring head from uninitialised work RAM and boots only with zero
or random work RAM — mapper 143 itself is right.

### J16. Cartridge database EEPROM sizes (`bNNN`)
The database writes a serial EEPROM's size in bytes in the save column (`b128`, `b256`, `b512`, 53
entries). The parser read them as 0 KiB, so *Battle Rush* (Datach, `983D8175`) never got the
external 24C01 that nesdev ("INES Mapper 157") signals with byte 10 `$10`, and waited forever for
its acknowledge. The size now reaches byte 10's non-volatile nibble and counts as a correction on
its own. Only mapper 157 reads it (`EepromPort::new`); the other 52 entries (16, 80, 159, 164, 529,
558) now get NES 2.0 headers from the database too, which the final golden comparison checks.

### J17. Mapper 111 with CHR-ROM is the parallel MMC1 (puNES only)
nesdev ("INES Mapper 111") gives the number to the Chinese *Ninja Ryukenden* translation, a
"non-serialized version of MMC1" with 256 KiB of CHR-ROM, coexisting with GTROM when the image has
CHR-ROM. Mesen2 and FCEUX implement only GTROM under 111; the behaviour (`Mmc1Parallel`) is
puNES's (`mapper_111.c`), the only source, and the Hackset dump (`15E688C0`) now plays its intro.
The page also mentions a later MMC3 version of the translation, not handled.

### J18. Mapper 234 (Maxi 15): register 1's range, mirroring on a read
Register 1 is decoded at `$FFE8-$FFF7` as puNES does (`mapper_234.c`, `$FFE8`/`$FFF0` under
`$FFF8`); Mesen2 adds `$FFF8` (`Mapper234.h`, `AddRegisterRange(0xFFE8, 0xFFF8)`). The read path
gives a board no mutable access to the cartridge memory, so a mirroring change latched by a
**read** reaches the nametables at the board's next write. The three Hackset dumps (Maxi 15 Rev 1,
Maxi 15, Maxi 1P) go from the menu into a game correctly. Every PRG read of the board goes
through `cpu_read_override` (no cached window), its cost bounded to this board.

### J19. Mapper 252: Mesen2's decode, puNES's CHR-RAM
Mesen2 (`Waixing252.h`) has no CHR-RAM for 252 and lets the game write into CHR-ROM ("CHR needs to
be writeable (according to Nestopia's source)"), and decodes no mirroring register. puNES
(`mapper_252.c`) maps 2 KiB of CHR-RAM for register values matching `mask`/`compare` (`$FE`/`$06`,
switched by CHR writes through windows holding `$88`, `$C2` or `$C8`) and decodes the VRC4's `$9000`
mirroring. This board takes Mesen2's register and IRQ decode and puNES's CHR-RAM and mirroring; the
cartridge database gives the dump 2 KiB of CHR-RAM. The two good Hackset dumps (`san_guo_zhi_ch`,
`_b2`) show the Waixing logo and the Chinese menu, whose text is drawn in that RAM; `_b1` is a bad
dump.

### J20. Mapper 198: nesdev/puNES, not Mesen2; two translation patches glitch
Mesen2's own header calls its `MMC3_198.h` "Most likely incorrect/incomplete" (4 KiB of WRAM
mirrored at `$5000-$7FFF`, CHR-RAM only while every CHR register is 0). This board follows nesdev
("INES Mapper 198": 640 KiB PRG, 8 KiB battery WRAM at `$6000`, 4 KiB more at `$5000-$5FFF`, 8 KiB
CHR-RAM) and puNES (`mapper_198.c`, banks from `$50` folding onto `$40-$4F`). The Hackset's
*Tenchi o Kurau II* `[T-ChS]` (`7DE81BC5`, the cartridge database's dump) shows its title and
password screen cleanly; the later `[T+ChSFIX2]` and `[T+ChTFIX2]` patches show stray glyphs on
the password screen, not investigated (translation hacks; their CHR use may differ).

### J37. MAPPERS-P batch 1 (55, 81, 122, 237, 389, 517): puNES readings kept as they are
Every board of `multicart_p.rs` follows its puNES file; nesdev has no page for most of them. Three
places where puNES itself is incomplete are copied, not invented around:
* 517 (`mapper_517.c`): the ADC behind `$6000` is the microphone of a karaoke cartridge. puNES
  feeds it no sample (its TODO); Zaxnes runs the same state machine on a silent input, so the start
  bits read 0, 1, 1 and the data bits read 0. A real microphone input would need a host source.
* 237 (`mapper_237.c`): with A0 set the board reads its DIP switches instead of ROM; puNES has no
  DIP setting and returns 0, so the menu always shows the first variant. Same here.
* 55 (`mapper_055.c`): `$6000-$6FFF` shows PRG-ROM from offset `$8000` and `$7000-$7FFF` is 4 KiB of
  RAM, which is puNES's reading of the BTL-MARIO1-MALEE2 board; nesdev describes 2 KiB of RAM
  mirrored. Unverified on a dump (none in the Hackset).
* 122 runs puNES's `mapper_184.c` with the `$6000-$7FFF` decode puNES gives it.
* Batch 2 adds two DIP-switch boards with the switches at 0, as puNES's default: 360 (the switch
  *is* the game selector, so only the first game runs) and 338's two DIP reads. 352 and 381 pick
  the game by counting resets: `Mapper::soft_reset` is the hook, and its default doc already
  allows the reset-detecting multicarts; a power cycle starts again at 0.
* 380 (submapper 0) reads `(A0-A7) | DIP` with A8 latched; the DIP is 0, so a menu that offers
  a choice by switch shows its first one.
* 400's `$8000-$BFFF` register drives an LED on the cartridge; puNES stores it and draws nothing,
  Zaxnes does not store it.
* 428 reads `(open bus & $FC) | DIP` at `$6000-$7FFF`; this core's open bus is a whole byte, so
  the read is plain open bus and its two low bits are the bus's rather than the switches' 0.
* 414 disables `$C000-$FFFF` when the latched address ANDs with the DIP value; at 0 it never does.
* 327 and 372 have a CHR-RAM mode that puNES maps over CHR-ROM on images that carry both; this
  core has one CHR memory per cartridge, so the mode is honoured on CHR-RAM images only.
* 334's DIP read at `$6002` is `(open bus & $FE) | 0`; the high address byte stands in for the bus
  (J33's convention).
* 401's DIP switches choose which register bits drive PRG A19-A20 and can disable PRG; at 0 the
  `reg[1]` D5-D6 lines are used and nothing is disabled.
* 416: `extcl_cpu_every_cycle_416` resets the counter on every cycle that does not reach `$1000`
  (`if (enable && ++counter == 0x1000) {...; return;} counter = 0;`), so as written its IRQ never
  fires. `IrqP` counts while enabled, asserts at `$1000` and clears when disabled; no dump in the
  Hackset to settle it.
* 357 runs its *SMB2J* layout only (DIP switch 0); the other three games are selected by the DIP.
* 422/534 (puNES `mapper_126.c`): the DIP read over `$8000-$FFFF` (`$6001` D0, low two bits
  from the switches) is not modelled, since the switches are 0 and the ROM windows stay cached;
  534's two-cycle A12 IRQ delay (`irqA12_delay = 2`) is not either; the "old dumps" CHR XOR is
  read the non-old way. No dump in the Hackset.
* 432: puNES's NROM mode reads `mmc3.reg[5 + (bank & 1)]`, R5 (a CHR register) for `$8000`;
  nesdev ("NES 2.0 Mapper 432") gives R6 for `$8000`/`$C000` and R7 for `$A000`/`$E000`, which
  this board follows. puNES also leaves out `$6001` D5 as PRG/CHR A19 on images of 1 MiB and more
  (nesdev, submapper 0); it is applied here. Only submapper 0 is decoded.
* 344: puNES picks a second chip order for one dump by its PRG CRC (`0xAB2ACA46`); this board
  keeps order 0 for all.
* 344 and 432 read 0 for the DIP switches / solder pad.
* 353, game 0: puNES picks the two-table or four-table CIRAM layout from **R0** D7
  (`mmc3.reg[0] & 0x80`), where its 370 and the `TxSROM` use the `$8000` CHR-inversion bit; this
  board follows the `TxSROM` convention. R0 D7 is kept where puNES uses it for PRG A17 and the
  CHR-RAM mode of game 2. No dump in the Hackset.
* 281/282/295 apply puNES's outer bank to ROM nametables too (`mirroring_swap_jyasic_28x`); with
  no dump in the Hackset this is untested on a real image.
* 100 (`mapper_100.c`): puNES's `irqA12_delay = 1` for this board and its jump to a trainer's
  `$7000` on power-up are not modelled; the chip's IRQ is the standard MMC3's and the trainer is
  loaded where the core loads it. puNES's select value `$47` loads `prg[1]`, as written.

### J36. Mapper 682 (Rainbow): the ESP is modelled from the protocol document
The ESP (`rainbow_esp.rs`) follows Broke Studio's `rainbow-net.md` and `mapper-doc.md`; no
emulator's ESP was read (the Broke Studio Mesen fork that has one is not local, and fetching it
needs the owner's consent). What is a reading or a choice here:
* a message goes out at once (`$4192` bit 7 always reads 1) and its answer is ready on the same
  write, where the real ESP takes milliseconds over its serial link; a game that waits for the
  answer sees it sooner, a game that does not wait is unaffected;
* `$4191` bit 6 ("data ready") is "an answer is queued"; the queue keeps the 64 newest answers;
  the Wi-Fi IRQ (`$4161` bit 0) is "a message is in FPGA RAM, not acknowledged" with `$4190` bit 1;
  messages are processed and delivered only with `$4190` bit 0 set;
* `MESSAGE_FROM_SERVER` carries the server's payload after the identifier; TCP is framed
  `[length][payload]` in both directions, UDP is one datagram per message;
* the file system is in memory and in the savestate, **not on the host's disk** (the host does not
  persist the flash save file either yet); auto paths are `drive/path/index`, manual names are
  folded to upper case; `FILE_GET_FS_INFO` reports 4 MiB per drive;
* random numbers are a seeded xorshift (repeatable), not the ESP's hardware generator;
* the access point, IP (127.0.0.1 with the network on), MAC and scan result are fixed values; Wi-Fi
  "connected" follows the option, `WIFI_CONNECT` does nothing;
* not done: TCP_S (TLS) and HTTPS downloads (no TLS in the core: answered "unsupported"), ICMP
  pings (answered "start failed"), the SD card (reported absent, drive 1 is a second in-memory
  drive), the debug log output;
* the network is behind `emu.rainbow_network`, **off by default** (owner, 2026-11-15); with it on
  a connection is not part of a savestate (loading one disconnects), so rewind, run-ahead and
  replays cannot restore it.

### J35. Mapper 682 (Rainbow): what phase 1 leaves out, and two readings
Mesen2 mainline (`Homebrew/Rainbow.cpp`) and Broke Studio's `mapper-doc.md` are the sources.
Phase 1 (`board-682`) has the CPU map, the five PRG modes and the two low-bank modes, the FPGA RAM
and its port, the CHR banks and sources, the four nametable sources with fill, attribute and
background extended modes, the window, both IRQs, the vectors, the three generated OAM routines and
the VRC6-style audio (this core's own `Vrc6Audio`, its `$9000-$B002` ports at `$41A0-$41A8`'s
offsets, scaled by `$41AA` / 15 and silenced when neither of `$41A9`'s two pins is set — the
mixing level relative to the APU is the VRC6's, not measured on the board). Phase 2 added the
bus snoop (`NesBus`, one flag set at attach): `$2003`/`$2004` writes feed the OAM Y copy the
sprite extended mode evaluates, as Mesen2 does, so an OAM **DMA** does not reach it either; a
`$4011` read with `$415A` bit 2 acknowledges the CPU IRQ and answers the 6-bit DAC sum doubled
(Mesen2 `GetLastOutput() << 1`; here the VRC6 generators' sum, Mesen2's own audio differs in its
mixing). The 3D capture replays both nametable fetches of each column (previous line's fetches 41
and 42 for tiles 0 and 1) and puts the per-fetch state back afterwards. Phase 4 added the S29 flash
(Mesen2 `FlashS29.h`): program, unlock bypass, sector and chip erase, software ID, on PRG-ROM and
CHR-ROM. The chip model is taken from the ROM size as Mesen2 does (1 MiB S29AL008 and any other
size, 2 MiB AL016, 4 MiB JL032, 8 MiB GL064S); the last 64 KiB's smaller sectors follow it. A
command byte that is not one of the known ones at the third unlock cycle resets the sequence here
(Mesen2 leaves the state until the next write). PRG writes go through this core's `FlashRom`, so
they reach the flash save file (8 KiB tracking sectors) and the savestate; **CHR flash writes change
CHR-ROM in place and are neither in the save file nor in a savestate** (Mesen2 keeps both through
its ROM patch). Phase 3 added the ESP (J36). Two readings: `$4151` returns the scanline
IRQ flag as it was **before** the read acknowledges it (Mesen2 clears first, so its bit 0 always
reads 0, which contradicts the doc's "IRQ pending" bit); the PPU-idle reset after three CPU cycles
without a fetch sets the scanline to -1 as Mesen2 does. `cpu_peek_override` clones the board to
read without side effects (debugger path only). No dump at hand: the test is synthetic.

### J34. Mapper 284 (Drip): rendering fetches only, and the audio level
Mesen2 (`UnlDripGame.h`, `MapperReadVram`) substitutes the attribute byte only on a
`PpuRenderingRead`. This core's nametable path (`read_nametable_from_vram`, the MMC5's) is the
same for a rendering fetch and a `$2007` read, so with extended attributes on a CPU read of an
attribute byte returns the substituted palette too; the "last tile" is whatever nametable address
the board last saw on the PPU bus. A game reads attributes back rarely, and never while it
streams them. The PCM channels' level is a guess: Mesen2 adds `(sample - $80) * volume * 3` to
its VRC7 channel; here each channel's `(sample - $80) * volume` is multiplied by 4, so the two at
full swing span about the DMC's own range. The attribute memory powers on at 0 (Mesen2: the
console's power-on RAM pattern). No dump at hand; the board widens the MMC5's six `Board` arms
by one each, behind `board-284`.

### J33. Mapper 260: nesdev and puNES over Mesen2
nesdev ("NES 2.0 Mapper 260") and puNES (`mapper_260.c`) agree: a six-bit 16 KiB PRG base and
a seven-bit 8 KiB CHR base, "opposite-masked" with the MMC3's banks, the lock in `$5000` bit 7
for every `$5xxx` write, and a `$8000-$FFFF` latch whose bit 2 gives "0 … vertical mirroring,
and 1 … horizontal mirroring" in modes 4-7. Mesen2 (`BmcHpxx.h`) takes narrower bases
(`$5001 & $18`, `& $10`), locks on bit 7 of any `$5xxx` write, and maps bit 2 the other way
(vertical when set); puNES stores the latch as `value & 3`, so its bit 2 never sets. This board
follows the page on all four. The `$5000` read's bits 7-2 are the bus, for which the high
address byte stands; `$5003` is undocumented and dropped. No dump at hand.

### J32. Mapper 328: deterministic "random" bits
Mesen2 (`Rt01.h`) and puNES (`mapper_328.c`) both answer `$CE80-$CEFF` and `$FE80-$FEFF` with
`0xF2 | (rand() & 0x0D)`. The C library's `rand()` would make two runs differ; here the three
bits are a hash of the CPU cycle count, the same on a replay, a rewind and a savestate load. The
test cartridge's check reads bits that vary, which this keeps; what the hardware returns there
(an unconnected bus, a counter?) is unknown. No dump at hand.

### J31. Mapper 266: the page's inner banks, and the DAC not emulated
nesdev ("NES 2.0 Mapper 266") keeps the VRC4's own 8 KiB inner banks under `$900C`'s PRG
A16-A15 ("PRG A13 and A14 stay connected to VRC4"); puNES (`mapper_266.c`) takes the inner bank
from the window number instead (a fixed 32 KiB), and Mesen2 (`CityFighter.h`) has its own
register file and a 9-bit CPU counter. This board follows the page, the IRQ being the VRC4's
(the page: the game uses its "pseudo-scanline mode during speech output").
**Open:** the 4-bit DAC at `$980C` is not emulated, so the speech is silent; puNES low-pass
filters `(pcm - 7) * 4096` into its mix, Mesen2 writes `(value & $0F) << 3` to `$4011`, and the
page calls its purpose unclear. No dump at hand.

### J30. Mapper 529: Mesen2's T230 is another board, and the 93C56 is not emulated
nesdev ("NES 2.0 Mapper 529") and puNES (`mapper_529.c`) agree: a VRC4e whose `$A00x` register
is one 16 KiB bank at `$8000`, the last bank at `$C000`, `$800x` without effect, and a 93C56
EEPROM (`$8800` mask writes: D0 data, D1 clock, D2 chip select; `$5000` read on D0) when the
NES 2.0 header declares 256 bytes of PRG-NVRAM. Mesen2's `T230.h` reads a different board (an
`$A000` pair of 8 KiB banks, a PRG mode, an outer bank on CHR-RAM carts) and is not followed.
**Open:** the 93C56 (x16 organisation, nesdev: "ORG pin is unconnected") is not emulated; the
existing `Eeprom93C66` is the x8 part of mapper 164 and its nine address bits would desync an
x16 stream. With no EEPROM, `$5000` answers 1, as puNES does for an image without one, and a
game that saves there cannot. No dump at hand.

### J29. Mapper 530: puNES's whole VRC4 against Mesen2's subset
puNES (`mapper_530.c`) runs the AX5705 as a full VRC4 (VRC4f wiring after `A |= (A & 8) << 9`,
IRQ and the `$9000` mirroring included) with the two data-line crossings; Mesen2 (`Ax5705.h`)
decodes the same banks and crossings but has no IRQ and only `$8008` bit 0 for the mirroring.
This board takes puNES's reading, through `Vrc2Vrc4` itself (a new `new_vrc4f` constructor
behind `board-530`). No dump at hand.

### J28. Mapper 513: the 32 KiB of CHR-RAM are not saved
Mesen2 (`Sachen9602.h`) marks the 32 KiB of CHR-RAM battery-backed (`ForceChrBattery`); this
core saves PRG-RAM only, so the CHR-RAM starts fresh at every power-on. Mesen2 passes the chip
`value & 0x1F` for R0-R5 and puNES `value & 0x3F`: the same pages with 32 KiB. No dump at hand.

### J27. Mapper 323: the outer bank's fourth bit, and the RAM under the register
Mesen2 (`FaridSlrom.h`) takes the outer bank from bits 4-6 (`(value & 0x70) >> 1`) and makes
`$6000-$7FFF` a register range, so the RAM does not take the byte; puNES (`mapper_323.c`) takes
bits 4-7 (`(reg & 0xF0) >> 1`) and lets the RAM take every write. This board follows puNES on
both: the extra bit only reaches past 1 MiB, where Mesen2 would wrap, and a register sharing the
bus with an enabled RAM chip leaves the chip written. No dump at hand.

### J26. Mappers 53, 487, 518 and 519 (`MiscN`): Mesen2's readings, sources compared
* 53 (`Supervision.h`): the 32 KiB EPROM is recognised by Mesen2's CRC of the first 32 KiB; an
  image without it powers on at 16 KiB banks `$80`/`$81` wrapped by the ROM, as Mesen2 does.
* 487 (`Mapper487.h`): Mesen2's comments are the only description at hand (no nesdev page
  read for it); the NINA-03 register is ignored in the second half, as there.
* 518 (`Dance2000.h`): Mesen2 switches CHR page 0 on every VRAM address change; here on the
  addresses the board hears (`on_ppu_address_bus`: fetches and `$2006`), which a scroll write
  alone does not drive. `$5200` without bit 2 changes the mode but rebuilds nothing until the
  next `$5000` write, as in Mesen2.
* 519 (`Eh8813A.h`): the DIP switches are all off, so the DIP mode reads the address with its low
  nibble cleared.
None of the four has a dump at hand.

### J25. Mapper 303 (KS7017): the `$4030` status read
Mesen2 (`Kaiser7017.h` `ReadRegister`) answers `$4030` with 1 while the IRQ is pending, 0 else,
and acknowledges it; puNES (`mapper_303.c`) acknowledges and computes the same value but returns
the work-RAM read at `$4030` instead, which looks like an oversight. This board answers the full
byte as Mesen2 does (bits 7-1 zero, where the hardware's open bus is unknown). No dump at hand.

### J24. Mappers 31, 103, 175 and 307 (`FineRom`): where the sources part
* 31: nesdev ("INES Mapper 031") sets only `$5FFF` to `$FF` at power-on and says it does not
  change on reset; Mesen2 (`NsfCart31.h`) leaves the seven others at 0, puNES (`mapper_031.c`)
  powers them on at `$F8-$FE` (the last 32 KiB). This board takes Mesen2's zeros and keeps all
  eight across a reset. No dump at hand.
* 103: Mesen2 (`Mapper103.h`) and puNES (`mapper_103.c`) agree (RAM at `$6000` and over
  `$B800-$D7FF`, always writable); Mesen2 keeps `$8000`'s low four bits, puNES the byte, which
  only matters past 128 KiB. *Doki Doki Panic* (FDS conversion, Hackset) plays world 1-1.
* 175: nesdev has no register description. Mesen2 (`Kaiser7022.h`) and FCEUX (`175.cpp`) apply
  `$A000`'s bank on a read of `$FFFC`, puNES (`mapper_175.c`) on any read of `$F000-$FFFF`; this
  board follows the two. FCEUX also moves CHR and `$E000` at once; Mesen2 waits for the read,
  as here. Registers at exactly `$8000`/`$A000` (Mesen2, FCEUX) where puNES masks `$F000`.
* 307: Mesen2 (`Kaiser7037.h`) and puNES (`mapper_307.c`) agree on the 4 KiB layout and the
  CIRAM page per table. A write to `$7000-$7FFF` (ROM) or `$A000-$AFFF` is dropped. No dump.

### J23. Mapper 188: the microphone is controller 1's A and B, its level 0
nesdev ("INES Mapper 188"): `$6000-$7FFF` reads bit 0 "0: A button is pressed", bit 1 "0: B
button is pressed", bit 2 "1-bit ADC microphone input", bits 7-3 open bus. Mesen2
(`BandaiMicrophone.h:37-46`) reads the two buttons and the level from key bindings of their own;
puNES (`mapper_188.c:49-53`) returns a constant 3. Here the buttons are controller 1's A and B,
handed to the board once per frame (`Mapper::set_input_word`), the level stays 0 (there is no
microphone input), and bits 7-3 are the high byte of the address, which an absolute read leaves
on the bus but an indexed read across a page or a DMA would not. The external ROM selected in a
128 KiB image (no expansion cartridge) reads open bus, as nesdev recommends and Mesen2 does,
where puNES and "several emulators" map a bank. Karaoke Studio and its two expansion cartridges
(Hackset `[!]` dumps) reach their song lists.

### J22. Mapper 168: nesdev/puNES board, two of three versions stuck in the core
The board follows nesdev ("INES Mapper 168") and puNES (`mapper_168.c`): CHR banks
`(reg & $0F) ^ 8` (the page allows either convention), the control bit taken from A7 or D2,
the free-running M2 counter whose bit 10 drives the IRQ, and the protection of U2 (banks 8-15)
cleared on a 1-to-0 edge of the control bit. A protected CHR read returns 0, where the hardware
leaves the PPU's open bus. Mesen2 (`Racermate.h`) and FCEUX (`boards/168.cpp`) are simpler: no
XOR, no IRQ, no protection, and FCEUX decodes `$B000` alone.
RacerMate Challenge II v3.12 (`041553C3`) reaches the race screen. v3.11 (`74920C13`) and v5.01
(`AAEF2264`) run in FCEUX but not here: the screen stays on the backdrop, and the decompressor at
`$FCA1-$FCF4` keeps rebuilding the same list from `$FB29`, eventually reading code at `$DFxx`.
This is **not the board**: replacing it with FCEUX's model (no XOR, no IRQ, no protection) gives
the same result. The two cores start one frame apart (`$00CE` 8F/80 at frame 5, power-on
phase), so `movie-sync` reports that first divergence and not the real one. The cause is in the
core (CPU, PPU or timing during long writes to `$2007` with rendering off) and remains to be found.

### J21. Mapper 199: implemented from nesdev and puNES, **not verified**
nesdev ("INES Mapper 199", Waixing FS309): 8 KiB of unbanked CHR-RAM and 32 KiB of battery PRG-RAM
of which 12 KiB are mapped at `$5000-$7FFF`; puNES (`mapper_199.c`) places `$5000-$5FFF` on 4 KiB
page 2. Mesen2's `MMC3_199.h` is a different board (CHR-RAM per page below 8, extra PRG
registers); the page says some Chinese emulators confuse 199 with 176. The Hackset's three "199"
dumps (`ED481B7C`, `D871D3E6`, `44C20420`) are exactly that: the cartridge database gives them 176
submapper 2, and run as 176 they boot (Dragon Ball Z Gaiden, Dragon Ball Z II, Sangokushi II), which
also verifies `Fk23c` on three more games. No genuine FS309 dump (Chéngjísīhán, Tāngmǔ Lìxiǎn jì,
Tūnshí Tiāndì: Sānguó Wàizhuàn) was at hand.

### J15. Mapper 15: CHR-RAM write protection only on the K-1029
MR-15-1 (from nesdev: "CHR-RAM is write-protected in PRG banking modes 0 and 3") write-protected
every mapper-15 image. The same nesdev page says the K-1029 multicarts are the only genuine
cartridges, that other images "claiming mapper 15" are mapper hacks, and that emulators "must not
enforce CHR-RAM write-protection" for them and must give them 8 KiB of PRG-RAM. Mesen2
(`Mapper15.h:29`) protects every image; puNES (`mapper_015.c`) and FCEUX never protect. This
build keeps MR-15-1 for the K-1029 and lifts it for an image declaring battery RAM or NES 2.0
PRG-RAM (cartridge database included), which is not a K-1029. Bao Xiao San Guo (`CE102393`,
8 KiB battery RAM in the database) writes its CHR in mode 0 and now shows its logo.
