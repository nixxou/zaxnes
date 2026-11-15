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

### A5. Per-dot differential against the netlist — in progress (lot FIDPLUS-2, 2026-11-15)
- **Instrument.** Zaxnes: feature `ppu-dot-trace` (never in `default`), harness `dot-trace`
  (`v`, `t`, PPU bus address and CPU register access after every dot; the PPU runs every dot
  through `step_dot`, no span). Netlist: `ext-tools/visualnes-bench/benchdot.cpp` (hold-time
  bench of step 33, one line per dot at the end of the dot, accesses at the /CS-fall dot),
  `mkines.py` (the bench's cartridge as an iNES image), `dotcmp.py` (frames paired by their
  access sequence, then `v`/`t` compared at every dot). Outside the repo: VisualNes has no licence.
- **Phase**: the netlist's power-on puts the CPU 7 dots later against the PPU than Zaxnes's;
  `dot-trace --phase 7` (or -14) makes every access land on the same dot (4 412 of 4 412 in
  `g2006px`, TriCNES alignment 3 = a4). The other alignments use their own reset hold.
- **`g2006px` (rendering on and off, `$2006`/`$2007`), alignment 3**: once the access dot itself
  is set aside (Zaxnes records it before applying the access), 370 dots of 89 342 differ a frame,
  all with rendering off and none observable:
  - `$2006` second write: the netlist loads `v` on D+3 with rendering off too; Zaxnes on D+1. A
    CPU access can only follow 3 dots later, and the rendering-off pixel at `v` was already
    matched on D+2 in step 32 (`emu.ppu_2006_delay` covers rendering on).
  - `$2007` write: the netlist moves `v` and stores to VRAM about 4 dots after the CPU write;
    Zaxnes at once. Observable only by a second `$2007` access 3 dots later, i.e. a
    read-modify-write instruction — measured below.
  - Line 241 of the first frame: the netlist's `v` = `$0802` from its power-on, not a model point.
- **`INC $2007`, rendering off, measured** (`mkrmw.py`: `$2000-$2007` = 0..7, `v` = `$2002`,
  `INC $2007`, read back): the netlist leaves **00 01 02 01 01 05 06 07** in all four alignments:
  the read moves `v` to `$2003`, the dummy write (00) and the write (01) come 3 dots apart, and
  the PPU stores each one ~4 dots after its CPU write from its data latch, which the second write
  has already replaced, so `$2003` gets 01 too. TriCNES: the same in alignments 0 and 1, `$2004`
  unchanged in alignment 2, garbled in 3 (its latch chain, `Emulator.cs:2484-2594`). Zaxnes
  before: 00 01. **Done**: `Ppu2C02::rewrite_pending_vram_write` (two `$2007` writes 1 to 4 dots
  apart outside rendering: the first address gets the second value), on, no option: a cold path
  (one test per `$2007` write), no state change; test `fidplus2_rmw_2007_tests.rs`.
  Not measured: palette addresses (internal RAM, kept one write each), rendering on (C1's path),
  the `$2007` read + write pairs of other RMW opcodes (same mechanism expected), PAL.
- **Frame parity.** Which frame is odd is a power-on fact, not the program's: the netlist's and
  Zaxnes's were opposite, which moves the CPU one dot against the PPU at every pre-render line
  where rendering is on (the skipped dot). `dot-trace --flip-parity` (diagnostics only) matches
  them; with it and the phase, every access of every frame lands on the same dot.
- **`g2001px` (`$2001` switched every ~20 dots, rendering on), alignments 2 and 3**: 33 runs a
  frame, all starting on dot 257 after a write landing on dot 254: the switch reaches the
  fetches and increments on D+3 = 257, but **the horizontal copy of dot 257 still follows the old
  state**: switched off, the netlist copies; switched on, it does not. Zaxnes did the opposite
  in both, which leaves the wrong coarse X and nametable for the rest of the line and the next
  one (visible). **Done** in `switch_rendering` (alignments 2 and 3, settled on the write, nothing
  tested per dot); test `fidplus2_dot_257_copy_follows_the_state_before_a_switch_on_dot_256`.
  Golden unchanged.
  After it: **0 dots differ** in every whole frame, both alignments. Alignments 0 and 1: the
  switch-on race of step 33 (a mid-line horizontal copy, present or not with the hold time) makes
  the comparison diverge; not trusted, not ported.
- **`g2005px`**: 1 dot a frame in alignments 1, 2 and 3 (line 239, dot 256: the netlist's Y
  carry still settling at the end of the dot; the next dot agrees). The netlist bench crashes
  (VisualNes segfault) in alignment 0 on this program. **`g3e01px`**: 0 dots.
- **`r2007` (`$2007` read on every 21st dot, rendering on), alignment 3, `emu.ppu_bus_conflicts`
  (C1) on**: two rules missing from step 25's read model, both visible (the next fetches):
  - when X (the latch dot, D+6 here) is a dot that increments coarse X anyway (every eighth, 328,
    336), the read's X increment **merges** with it: one X increment, plus the read's Y. Dot 256
    included (natural X and Y, plus the read's Y: fine Y steps twice);
  - when X is 257 (odd: the residue case), the horizontal copy of 257 overrides the read's X
    increment: only its Y is left.
  **Done** in `read_buffer_from_fetch`, on by default. With C1 (on by default) they change Zelda
  II Europe's title screen (golden frame 600, ~9 400 pixels: the landscape below the split). The
  title does, on line 141, `$2006` = `$2A`, `$00` then two `$2007` reads 12 dots apart, the reads
  landing between dots 247 and 275 (600 frames traced). `mkz2.py` replays that sequence on the
  netlist, sliding over dots 245-287, alignments 2 and 3: **without** the rules, Zaxnes diverges
  on the two frames whose reads latch on dots 254-257 (33 847 and 2 383 dots); **with** them, every
  frame matches (only the 1-2 dot recording instants). Owner: "je te fais confiance". Visible lines then agree with the
  netlist in every whole frame (no run longer than the 1-2 dot recording instant of an
  increment). Lines 240-261 still differ: the program never writes `$2000` and the netlist powers
  on with PPUCTRL's +32 increment (seen with `INC $2007` too), so its vblank reads step by 32 — a
  bench artefact.
- **`g2006x` (second `$2006` writes alternating `$2108`/`$2AD7`, rendering on), alignments 2 and
  3**: the netlist loads `v` in the middle of D+3, so when D+4 is a coarse-X increment dot the new
  address is incremented; Zaxnes loaded after running D+4 and lost that increment (the rest of
  the line one tile left, visible). **Done**: `Ppu2C02::increment_after_delayed_load` (cold, NTSC).
  Long runs 639 -> 20 a frame. Left: a write landing on dot 257 (load on 260): `v` differs on
  dots 258-261 only (netlist 3100 / 3117 per alignment, Zaxnes 3108), in the garbage nametable
  fetches of the sprite phase; no picture effect, not ported.
- **`g2005`**: whole frames agree in alignments 2 and 3 (the third netlist frame pairs badly, a
  pairing artefact of the sliding program).
- **Regions**: every rule of A5 that depends on dots (dot 257, the read merge) applies to NTSC
  and Multi only (`Ppu2C02::netlist_measured_region`): the netlist is a 2C02; the PAL 2C07 (3.2
  dots a cycle) and the clones keep the previous model until a measurement of their own. The
  `INC $2007` rule (a data latch, dots 1 to 4 apart) applies everywhere.
- **Bench artefacts to keep in mind**: PPUCTRL/PPUMASK power-on values of the netlist (write
  `$2000` after the warm-up); the PPU ignores writes during the warm-up; the `bus` column also
  shows the netlist's pattern-table select of its power-on PPUCTRL.
- **Cost** (callgrind, clean builds, `3d5319ab` against FIDPLUS-1 `6289064d`): +0.02 % to +0.10 %
  on the PR-1 set (metroid +0.10 %, mm2 +0.09 %, smb1 +0.08 %, smb3 +0.04 %). A first version
  testing dot 257 on every dot of `run_render_dot` cost +2 to +2.9 % (it changed the inlining of
  `run_dots`); moving the rule to the `$2001` write removed it.
- **`g2000`, `oam`** (alignments 2 and 3): 0 or 1 dot a frame once the power-on frame is past.
- **PPU bus addresses (`buscmp.py`, `g2001px`, alignment 3)**: about 5 100 of 24 700 ALE
  addresses a frame are not the address Zaxnes reads on that dot or the next, in three families:
  - the garbage nametable fetches of dots 257-320 (and dots 1, 337-340): Zaxnes does not put them
    on its bus. No picture effect; only a board snooping the bus there would see it (MMC5's
    in-frame detection counts the dot-337/339 pair, which Zaxnes models separately);
  - the pattern rows of empty sprite slots (tile `$FF`, row 6 on the netlist, 7 in Zaxnes on
    line 0): no A12 effect;
  - **open lead**: the first pattern fetch after a `$2001` switch on (3 or 6 dots after the write,
    ~310 a frame) uses as tile number what lies on the multiplexed AD bus — the low byte of the
    last address latched (`$15`, `$10`, `$18`: the nametable column of the last fetch before the
    switch off) or the last byte read (`$CA`, the high plane) — where Zaxnes keeps the tile of the
    last real nametable fetch. Probably visible (one wrong 8-pixel group after a mid-line switch
    on, alignments 2 and 3); the same mechanism as step 39's stale low byte, for the tile latch.
    To model with its own measurement (which bytes, which dots, per alignment) before porting.
    **First attempt (2026-11-15), dropped**: "the tile register latches the AD bus value held at
    the switch-off on every nametable dot passed with the rendering off" made the bus comparison
    worse (5 100 -> 5 700 unmatched a frame) and missed the cases it targeted (dot 173 `00E0`).
    The real rule needs the netlist's tile-register node and AD-bus node logged per half-step
    (`benchdot` logs only the ALE address), not inferred from the address column.
    **Measured (2026-11-15, `benchdot2`: AD bus `db0-7` per dot; `inbuf0-7` turned out to be the
    CPU-side buffer, not the tile register)**, alignment 3, `g2001px`:
    - the AD bus keeps a value while the rendering is off; switch-off right after an ALE whose
      read did not come (line 0, ALE `$200E` on 153): the bus holds `$0E`, and the first pattern
      fetch after the switch-on uses tile `$0E`: **the tile register latches the bus on the
      nametable dot (170) although the rendering is off**;
    - but the value the bus holds is not simply "the last ALE low byte or the last read byte":
      switch-off after the ALE `$0008` of dot 199, the bus shows `$11` (the low byte of the
      nametable address of dot 193) for the whole off stretch: charge left on the pins by an
      earlier phase, which a zero-delay netlist reproduces only by accident;
    - a switch-on on an odd dot reads its stale fetch with the **whole** stale address (`$0008`,
      a pattern byte `$CA` used as the tile), not the stale low byte under the current fetch's
      high lines that step 39 models.
    Not ported: the bus value is a floating-node effect (not trustworthy at transistor level
    without analogue charge), and the effect needs a mid-line rendering switch inside a 4-dot
    window. Kept as an open lead for the half-cycle stepper, which will hold the AD bus as state.
- **Next**: that tile-latch lead; `g2006px` with rendering on; C1 against the SMB3 Japan
  line-193 video.

---

### A6. M2 duty and edge placement outside NTSC — open, Fidélité+ (2026-11-15)
- Fidélité+ places every access on the M2 and /CS edges (`fidplus::MasterClock`): M2 low for 9/24
  of the CPU cycle, high for 15/24 (2A03G, nesdev "CPU pin out"; BeesNES's phi1/phi2 slots use the
  same split). The 2A07 (PAL, 16 master clocks a cycle) and the Dendy clone (15) are given the
  same duty for lack of a measurement; their netlist or a scope capture would settle it.
- Earlier 2A03 letters are reported with a different duty (to check against nesdev before use:
  the 2A03E and earlier); part C's console model should carry it per revision.

### A8. A second `$2006` write on a copy dot takes the CPU's open bus in alignment 3 — done in Fidélité+, open for the standard core (2026-11-15)
- Netlist, `g2006x` (`$2006` written alternately `$08` and `$D7` on dots 257 and 304), alignments
  2 and 3: the copy of that dot (horizontal on 257, the last vertical one on the pre-render line's
  304) takes, for the bits the write's low byte drives, the **written byte** in alignment 2 and
  the **CPU data bus as it stood before the write** (`$20`, the high byte of `$2006`) in
  alignment 3. The same open bus as the `$2005` write on these dots and the `$2000` glitch (A4).
  (A first reading as "the old `t` AND the written byte" fitted dot 257 only.)
- Fidélité+: `post_address_load`, under `emu.ppu_2000_glitch`; `g2006x` 0 dots in both
  alignments. The standard core does not apply it yet: a cold rule on the write, a candidate for
  the common base.

### A7. The coarse-Y wrap takes two phases — open, Fidélité+ step 3 (2026-11-15)
- Netlist, `r2007`, alignments 2 and 3: when a Y increment wraps coarse Y from 29 (fine Y 7), `v`
  shows for **one dot** coarse Y 30 with the vertical nametable bit already flipped (`$7FAF` ->
  `$07D0` -> `$0410`), then coarse Y 0: the increment ripples first, the "30" detection clears it
  in the next phase. Seen on the natural increment of dot 256 and on a `$2007` read's increment.
- Only a fetch made during that dot can see it: the refetch of an odd-X `$2007` read latch
  (`ALE + Read`) takes its high address lines from that `v`, so it would read row 30 (the attribute
  table's rows). Not modelled; step 3 (AD bus and address latch per half dot) is where it belongs.
- Also seen with the same bench: `r2007` never writes `$2000`, and the netlist's PPUCTRL powers on
  with bit 2 set (+32 increments in vblank) where Zaxnes has 0: the post-render lines of that
  program are not comparable.

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
  post-process between HPF1 and HPF2 (`PostHpf`, `LSNApu2A0X.h:1919-1931`) — **done 2026-11-15
  (owner, an exception to AMD-061 §6.2 inside the profile option, FIDPLUS-D1a)**: with a profile,
  the expansion level is a band-limited signal of its own (`expansion_ring`), added between the
  second and the third high-pass; without one, the clip of AMD-061 is unchanged (golden identical,
  audio included); (5) filter state not in the savestate — **done 2026-11-15 (FIDPLUS-D1b)**:
  a savestate taken with a profile carries the filters' memory (format 14 / 15: a flags byte, then
  the edges and / or the 64-byte filter block; 12 and 13 still read, no profile keeps 10 / 11), and
  a load gives the same samples as the machine that went on (test, exact). Found on the way: every
  recomputation of the resampling ratio (speed, the host's fine pacing adjustment, a load's region)
  rebuilt the filters at rest, a click each time; the same profile at the same rate now keeps them.
  Open: run-ahead's in-memory restore keeps the filters where the frames ahead left them (their
  audio is not heard; a few samples of filter memory apart); (6) the
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

### G3. MMC3 revision A / MMC6 as a console-model choice — done (FIDPLUS-C, 2026-11-15)

- Revision A (NEC / Sharp MMC3A, and the MMC6's counter) asserts on a reload to zero only when
  the counter was moving (non-zero before the clock or a `$C001` reload pending); revision B at
  every clock that leaves it at zero. The core had both (NES 2.0 submapper 4 = revision A, Mesen's
  `(count > 0 || _irqReload)`); FIDPLUS-C makes the revision a choice (`emu.mmc3_revision`,
  `NesCore::set_mmc3_revision`, harness `ZAXNES_MMC3_REVISION`).
- Judge: revision A passes `5.MMC3_rev_A`, `6-MMC6`, `6-MMC3_alt` and fails `5-MMC3` (both suites)
  and `6.MMC3_rev_B`, revision B the reverse. With each row under its own console
  (`ZAXNES_CONSOLE_MODEL=rom`, blargg's power-on palette for `power_up_palette`) the campaign is
  219 pass, 0 fail.
- Open: the MMC6's other differences (its 1 KiB PRG-RAM with per-half protection) already follow
  NES 2.0 submapper 1; an iNES 1.0 MMC6 game (StarTropics) needs the cartridge database, which has
  it. Not yet modelled: the MMC3's A12 filter differences between revisions, if any (no test ROM
  separates them).

### G4. Letterless RP2A03: the noise channel's last period — done; the reset of `$4017` — open (FIDPLUS-C, 2026-11-15)

- BeesNES (MIT, `LSNApuUnit.cpp` `m_ui16NoisePeriodsOld`) gives the letterless RP2A03 a noise
  period table whose last entry is 2046 CPU cycles, not 4068, and no short mode
  (`Write400E`, `OldNoiseTable`). Zaxnes had the missing short mode only (`audio.console_profile`'s
  letterless consoles, the N0260073). Done: `LETTERLESS_PERIODS` in `apu/noise_channel.rs`, taken
  with the flag; a reset keeps the chip's table (it used to fall back to the default).
- Done (owner, 2026-11-15, an exception to Q-078): a reset leaves the frame sequencer running on
  a letterless NTSC chip when that chip is chosen (`emu.cpu_revision` or a letterless audio
  profile), as nesdev "CPU power up state" and BeesNES (`LSNApu2A0X.h:576`) have it; every other
  chip keeps Q-078's 2A03G reset. Test `fidplus_c_a_reset_leaves_the_letterless_sequencer_running`.
- Open: the letterless chip is reached only through the audio profile of one measured console; a
  console-model choice of its own (with the other RP2A03 letter differences, DMA timings) is part
  of C3.

### G5. CPU / APU get-put parity and the DMC timer at power-on — open (2026-11-15)

- The 2A03 netlist differential (`docs/progress/CPU-NETLIST-DIAG.md`) finds the CPU identical
  cycle for cycle once two power-on values are set as the netlist's: the get / put parity (Zaxnes
  starts on the other one: OAM DMA 514 / 513 where the netlist takes 513 / 514, and the DMC load's
  halt one read apart) and the DMC output unit's timer (431 with 8 bits remaining, against this
  core's 427).
- Open: whether a console fixes the parity at reset (the netlist does) or draws it at power-on.
  The CPU / PPU alignment is already a choice (`emu.cpu_ppu_alignment`, random among four); a
  "CPU / APU alignment" choice (random, either) would treat the APU the same way. No test ROM in
  the campaign tells the two apart (all pass either way); a game that depends on it would be one
  timing an OAM DMA's length or a DMC halt's repeated read without syncing first.

### G6. Open-bus bits of a controller read just after a DMC fetch — open (2026-11-15)

- 2A03 netlist differential, program `dmc4016` (`docs/progress/CPU-NETLIST-DIAG.md`, step 5): when
  a DMC DMA halts a `$4016` read (`4016R 4016R C000R 4016R`), the netlist's real read returns `$00`
  (bits 5-7 = the sample byte `$00` the fetch left on the pins), Zaxnes `$40` (bits 5-7 from the
  internal bus, which the fetch does not update). Seven such reads in 30 000 cycles; every other
  `$4016` read is identical.
- Zaxnes takes the controller ports' open bits from the internal bus on purpose (`nes_bus.rs`,
  `AccuracyCoin` `Implied Dummy Reads`: JSR operands read from `$4016`/`$4017` "leech off" an opcode
  read from `$4015`, which only the internal bus carries). Both observations fit a rule where the
  halt and dummy re-reads of `$4016` load the internal bus from the pins; not changed until a model
  satisfies both, with `AccuracyCoin` 147/147 kept.
- Step 6 of the diagnostic (2026-11-15): the netlist is no judge here. VisualNes has no controller
  and floats the external bus on every read of `$2000`-`$7FFF` that is not a chip (`mCpuRead`,
  `floatBits`), so its `$4016` / `$4017` bits are a floating bus in a switch-level simulation (its
  `$4017` reads `$10`). Program `g6jmp` (opcode from `$4015`, operand from `$4016`): the netlist
  takes the pins (`$40`), while `AccuracyCoin`'s hardware-verified `Implied Dummy Reads` expects the
  internal bus in that same sequence (`$21`, where the pins would give `$E1` for `NOP $EA`). Zaxnes
  follows the hardware test; the DMC case alone has no hardware reference yet. Lead kept open for a
  console measurement (a test ROM reading `$4016` right after a DMC fetch, run on hardware).

## H. Video output (shaders lot, not core)

### H1. Composite / RF signal, CRT gamma, phosphor decay — open
- BeesNES: NTSC-CRT / PAL-CRT (LMP88959), own L. Spiro filters, measured CRT gamma, phosphor
  decay, signal noise (README "Visual Samples"); palette generator palgen-persune.
- Zaxnes: librashader slang shaders (branch feat/librashader). These are generic shaders, not a
  signal model. Candidate shaders/palettes to port; not a core item.

### H3. Colour emphasis on screen — done in the 2D picture (FIDPLUS-D2b, 2026-11-15)
- Found while preparing D2b: the picture carried colour index and opacity only; PPUMASK bits 5-7
  never reached the screen (REQ-CORE-047 keeps them out of the tile records and the palette, and
  nothing else carried them). Survey (`fidplus_d2b_emphasis_survey_tests`, 1800 frames each, 348
  ROMs): `Duelito` (blue emphasis on every frame), `firefly`, `rastesam4`, `RasterDemo`,
  `Sayoonara!`, `CMC80s`, the palette demos, blargg's `litewall`, `240pee`, `mmc5exram`; *Metroid*
  one frame at power-on.
- Now: the PPU logs each write that changes the bits with the first pixel it reaches (delay
  2/1/1/2 dots by alignment, TriCNES's, not yet measured on the netlist), resets at the frame wrap;
  the host expands it into a plane only for a frame with emphasis (nothing per pixel otherwise).
  Colours: the chosen 64-colour palette scaled per channel by TriCNES's ratio (its 512-colour
  table, MIT, `zax-engine3d/src/emphasis_table.rs`; `palette::nes_color_emphasized`). Our own
  composite decoder's ratio was tried first and was too far from it (red emphasis on ``: green
  0.89 and blue 0.31, TriCNES 0.75 and 0.67). Option `video.emphasis_3d` (owner, off): the 3D
  scene takes the frame's most common emphasis.
- Open: the delay against the netlist's video pins; 512-entry palettes
  (Mesen's, FirebrandX's own emphasis rows) as tables instead of the ratio.

### H2. The 2C02's composite signal — generator and reference decoder done (FIDPLUS-D2a, 2026-11-15)
- nesdev "NTSC video": per pixel, 8 half-clock samples of a square wave at the colour subcarrier
  (12 phases per cycle), high or low level of the pixel's row; hue 0 always high, 13 always low,
  `$xE`/`$xF` row 1's low (black); emphasis bits attenuate the samples in their colour's half
  cycle. Levels used (volts): low 0.228 0.312 0.552 0.880, high 0.616 0.840 1.100 1.100,
  attenuated low 0.192 0.256 0.448 0.712, high 0.484 0.668 0.896 0.896. **To check against the
  page and the 2C02 netlist's video pins** (written from the page's figures; a mistake would show as
  a palette shift).
- `zax-nes/src/composite.rs`: `signal`, `pixel_samples`, the phase per dot (+8) and per line (+4,
  +8 on the odd frame's short line), `decode_uniform` (YIQ over a subcarrier cycle, FCC matrix).
  Tests: grey rows carry no chroma, every hue is high half a cycle, decoded white / black / red /
  green / blue are what their names say, emphasis darkens.
- Next (D2b): the picture through the generator and a line decoder (colour artefacts, dot crawl),
  Fidélité+ only; D2c the PAL 2C07 (its phase alternation).

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
