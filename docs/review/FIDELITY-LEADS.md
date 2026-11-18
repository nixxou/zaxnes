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
  dots a cycle) and the clones keep the previous model until a measurement of their own. A 2C07
  netlist exists (owner, 2026-11-15): Visual 2C07, `https://www.qmtpro.com/~nes/chipimages/visual2c07/`
  (same site as Visual 2C02; its `transdefs.js` differs from the 2C02's, 1.22 MB against 1.13 MB;
  its `nodenames.js`, dated 2016-09-01, looks partly carried over from the 2C02, e.g.
  `vpos_eq_261`). Licence not stated on the page (visual6502 lineage). It would unblock C3c and
  D2c as a measurement tool outside the repository, like VisualNes; the download waits on the
  owner. The
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

- **Standard core against the 2C02 corpus, 2026-11-16** (`ext-tools/visualnes-bench/ntscjudge.py`,
  harness `dot-trace --power-on-vblank`, the netlist's first frame left out, two frames paired,
  alignments 2 and 3 alike): `g2001px`, `g3e01px` 0 differing dots; `oam` 1; `g2000`, `g2005px` 2;
  `g2006x` one per register access (the recording instant, after A8); `g2006px` 740, the two-dot
  recording instants of its `$2006` loads (Zaxnes records `v` at the write, the netlist at the
  load); `r2007` with `--bus-conflicts` ~11 000 a frame on its two first frames (83 000 without),
  its third netlist frame pairing with none (already so on 2026-11-15). `g2005` at phase 17: 1
  differing dot on its paired frame (the judge rejected it as `r2007`: the third netlist frame of
  both pairs with Zaxnes frame 4, not 3, likely an odd-frame dot skip around that frame). Not
  judged: `rmw` (its traces do not pair with this bench).

### A6. M2 duty and edge placement outside NTSC — open, Fidélité+ (2026-11-15)
- Fidélité+ places every access on the M2 and /CS edges (`fidplus::MasterClock`): M2 low for 9/24
  of the CPU cycle, high for 15/24 (2A03G, nesdev "CPU pin out"; BeesNES's phi1/phi2 slots use the
  same split). The 2A07 (PAL, 16 master clocks a cycle) and the Dendy clone (15) are given the
  same duty for lack of a measurement; their netlist or a scope capture would settle it.
- Earlier 2A03 letters are reported with a different duty (to check against nesdev before use:
  the 2A03E and earlier); part C's console model should carry it per revision.

### A8. A second `$2006` write on a copy dot takes the CPU's open bus in alignment 3 — done in Fidélité+ (2026-11-15) and the standard core (2026-11-16)
- Netlist, `g2006x` (`$2006` written alternately `$08` and `$D7` on dots 257 and 304), alignments
  2 and 3: the copy of that dot (horizontal on 257, the last vertical one on the pre-render line's
  304) takes, for the bits the write's low byte drives, the **written byte** in alignment 2 and
  the **CPU data bus as it stood before the write** (`$20`, the high byte of `$2006`) in
  alignment 3. The same open bus as the `$2005` write on these dots and the `$2000` glitch (A4).
  (A first reading as "the old `t` AND the written byte" fitted dot 257 only.)
- Fidélité+: `post_address_load`, under `emu.ppu_2000_glitch`; `g2006x` 0 dots in both
  alignments.
- Standard core, done 2026-11-16 (`Ppu2C02::address_write_on_copy`, cold, called from the second
  `$2006` write on dots 257 / 304): the same rule, and on the PAL 2C07 its dot-257 form (open bus
  on dot-accumulator phase 4). Judge: `ext-tools/visualnes-bench/ntscjudge.py` with the harness's
  new `dot-trace --power-on-vblank` (the 2C02 traces of FIDPLUS-2 were taken against the power-on
  at vblank, before `emu.power_on_top`; the old `fidjudge.sh` no longer finds a phase without it).
  The netlist trace holds three frames, two paired: `g2006x` 4 412 differing dots a frame in both
  alignments, exactly one per register access (the recording instant), against 4 425 before A8:
  no rendering difference left in the standard core.

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

### B1. PPU position at power-on — done, on by default (FIDELITY step 27)
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
- **On by default since FIDELITY step 27** (`emu.power_on_top`, with `emu.oam_corruption`;
  the owner checked the moved games by eye). The fixtures of the older test files keep the
  vblank power-on they were written against (`tests/common::machine_with`).

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
- **A hardware measurement (2026-11-15)**: FireSide's `oam_decay_test.nes` (NESdev Discord; A
  holds rendering off and counts the frames, its release prints OAM): OAM stays intact about **7
  frames** on their console (model and PPU revision not given), i.e. ~117 ms on NTSC, on the side
  of blargg's ~123 ms and BeesNES's 8 frames, not of nesdev's "not much longer than a vblank". Read
  in its source (`main.s`): the counter counts the first frame of a press while rendering is still on
  (PPUMASK is written with the old value before the mask is cleared), so the true figure may be 6;
  its readout writes back each byte it reads, so a readout re-seeds OAM. Other consoles' results
  are asked for on the thread; the `$E3` bytes it shows are the attribute bytes' missing bits 2-4. Asked
  whether OAM goes all at once or row by row, they answer: **random bits decay, more of them the
  longer rendering stays off**. Zaxnes's `emu.oam_decay` drops a whole row at once after 8 frames
  without access: a gradual, per-bit model (a per-bit retention time drawn once per console, bits
  past it lost) would be closer. Asked what a decayed bit becomes: **each bit has a "resting
  state", 0 or 1, it ends in** (a DRAM cell's discharged level, true or complement cell), and **the
  first 8 bytes never decay** in that test. Likely reason, from its source: with rendering off the
  PPU keeps reading OAM at OAMADDR, which refreshes that row, and OAMADDR is 0 there (after the DMA
  and after the readout's 256 writes); proposed check: `$40` to `$2003` before turning rendering
  off should keep bytes `$40-$47` instead. One console, by hand: to confirm. Model it would give:
  per bit, a resting value and a retention time drawn once per console; past it without an access
  the bit takes its resting value; the row at OAMADDR exempt while rendering is off.
- **Speculative per-bit model (lot FIDPLUS-E, 2026-11-16), replaces the whole-row model of step
  20 under the same option (`emu.oam_decay`, still off).** `Ppu2C02::refresh_oam_row`,
  `move_oam_address`, `decay_oam_row`, `OAM_RETENTION_US` (`ppu/mod.rs`):
  - per bit, a retention drawn once from a log-normal law (DRAM retention), a 256-quantile table:
    none before 125 ms (above blargg's ~123 ms), median 266 ms (16 NTSC frames), all by 566 ms;
  - per bit, a resting value (0 or 1); both draws are a fixed hash of the bit's index (SplitMix64),
    so a run replays identically and nothing new goes in the savestate;
  - the OAMADDR row is held refreshed while rendering is off (hypothesis above): a row is settled
    when OAMADDR enters it and stamped when OAMADDR leaves it; a `$2004` access to it decays nothing;
  - no decay at all on the 2C07 (nesdev: the PAL PPU refreshes OAM itself); Dendy at its dot rate.
  - Measured with our ROM (`ext-tools/oamdecay-rom/oamdecay.nes`, harness `ZAXNES_OAM_DECAY=1`):
    0 bits lost for 1-7 frames, 6 at 8, 25 at 9, 69 at 10, 257 at 12, 816 at 15, 1 453 at 20,
    1 798 from 30 frames (878 to 0, 920 to 1; the other bits already sat at their resting value);
    the OAMADDR row (ROW31 in the ROM) keeps its bytes.
  - (Superseded below.) Not measured: the law, its median and spread, the share of 0 and 1 resting values, temperature
    (one internal scale, no option, owner 2026-11-16). One plausible console, not a measured one.
- **Measured on hardware, model recalibrated (2026-11-16)**: FireSide ran our `oamdecay.nes` on
  their NES (a G-or-later PPU, NTSC; NESdev Discord, a video of the result screen). Bits lost, TO 0
  (after an all-`$FF` fill) / TO 1 (after an all-`$00` fill), out of 1800 measurable bits outside
  row 31:

  | frames | 1-6 | 7 | 8 | 9 | 10 | 12 | 15 | 20 | 30 | 45 | 60 | 90 | 120 |
  |---|---|---|---|---|---|---|---|---|---|---|---|---|---|
  | TO 0 | 0 | 0 | 4 | 9 | 11 | 39 | 90 | 110 | 230 | 510 | 839 | 1405 | 1406 |
  | TO 1 | 0 | 2 | 4 | 5 | 6 | 9 | 21 | 29 | 86 | 205 | 350 | 433 | 432 |
  | ROW31 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

  - **ROW31 = 0 throughout**: the row OAMADDR is parked on is held refreshed with rendering off.
    The hypothesis above, and the model's held row, are confirmed.
  - **Much slower than our guess**: half the losses by ~50 frames (0.85 s), all by 90 (1.5 s);
    the guess had a 266 ms median and 566 ms maximum. The curve is not log-normal: a slow tail
    from 7 frames, then most bits between 30 and 90 frames.
  - **Resting values are lopsided**: 1406 to 0 against 432 to 1 (76 % / 24 %), not 50/50.
  - **1406 + 432 = 1838 > 1800**: 38 bits (2 %) were lost in both passes, i.e. they end at the
    complement of what was written rather than at a fixed level (or are noisy near the threshold).
  - Model now (`OAM_RETENTION_US`, `oam_bit_rest`): the retention table is the pooled empirical
    quantiles of this run, linear between the sampled frame counts (the 60-90 stretch is
    unsampled); resting classes 0 / 1 / flips in proportions 1368 / 394 / 38 of 1800. A flipping
    bit flips each time it outlives its retention unrefreshed (a refresh between two blanks starts
    it over); repeated flips are a guess.
  - **125 ms floor kept**: the measurement has 2 bits lost at 7 frames (~116 ms), but blargg's
    `oam_stress` (passes on hardware) leaves rows 2-31 unrefreshed ~6.7 frames on average between
    its refreshes and **fails with the raw quantiles** (code 1, 5 bytes). With the floor it
    passes, decay on and off: test `fidplus_oam_stress_passes_with_the_oam_decay`. Either
    FireSide's console is at the fast edge (it may not pass `oam_stress` itself, worth asking) or
    the 7-frame point includes the ROM's own fill/read overhead.
  - Still one console at one (unknown) temperature. Other consoles' runs would give the spread.
- **Reading OAM on old PPUs (NESdev Discord, 2026-11-16)**: a tester ran a decay ROM on a
  letterless and an E-revision PPU and read all `$00`; reply on the thread: PPU revisions before G
  do not support reading OAMDATA (`$2004`). Zaxnes already models it (`pre_g_revision`). So any
  `$2004`-based decay measurement, ours included, needs a G-or-later PPU; FireSide's figures come
  from such a console.

### D2c. PAL composite and the colour/emphasis leads it found (2026-11-16)
- Done: the 2C07's signal from its netlist (ANALOG-OUTPUT D2c). Leads left: (1) the DAC voltages of
  the 2C07 are the 2C02's (no measurement); (2) the 2C07's 2-pixel left/right border at the
  blanking level is not in the core's picture; (3) done: the same bench on VisualNes's 2C02 netlist
  (`benchvid <visualnes-src/Core>`, 1 024 records, `vidmodel_ntsc.py`) matches Zaxnes's 2C02
  generator on every colour, emphasis and phase but `$xE`/`$xF`, which output the blanking level
  with no emphasis there too; `composite::signal` fixed (128 records before, 0 after); (4) Dendy's PPU (UA6538) keeps the NTSC model.
- The bench's line parity is the netlist's `vpos` bit 0; the phase origin is the bench's
  half-clock counter. Whether a real 2C07 powers up with another relation between its /5 dot
  divider and its /6 subcarrier divider (which would move the pixel's phase) is not known.

### C3c-collision. PAL `$2006` load colliding with an increment of `v` (2026-11-16, rule done)
- Visual 2C07, `g2006x` at reset phases e0/e3/e6, line 54 (and 148, 223): `v` = `$210C`,
  `t` = `$2A08`; second `$2006` write `$D7` on dot 253. Netlist: the load lands on dot 256 and
  leaves `v` = `$2005`, `t` = `$2805` (dots 256-281). Zaxnes: the load lands on dot 257 (one dot
  late), the collision (`address_load_collision`, the 2C02's rule `t &= v`, `v` = `$310D` after
  the dot-256 increments) gives `t` = `$2005`, then `v` = `t`.
- So on the 2C07: `v` = written AND incremented (`$2AD7 & $310D`), but `t` keeps the written
  nametable-Y bit (`$2805` = `$2005 | $0800`), and the load is a dot earlier for this write's
  cycle phase. One value pattern only: a dedicated program (several written values, nametable
  bits set and clear, writes on dots 248-256 at every cycle phase) is needed before a rule.
  Cost today: 14 long runs over five frames at each reset phase, about 4 900 dots a frame.
- **Dedicated program `g2006w` (2026-11-16)**: second `$2006` writes looping over `$2108`,
  `$2AD7`, `$2D5A`, `$25A5` (the four nametable cases), 51-cycle loop sliding over every dot;
  Visual 2C07 at reset phases e0, e3, e6, e9 (`ext-tools/visual2c07-tools`: `c3ccoll.py`
  extracts each write near dot 256, `c3crule.py` checks the rule). **All 193 collisions fit**:
  * on the dot-256 increment (X and Y): `v` = written AND incremented, every field;
  * on a coarse-X increment: coarse X **and nametable X** (`$041F`) written AND incremented,
    the rest written (the 2C02 rule of A3c ANDs coarse X only, `$001F`);
  * `t` = `v` with the **written nametable bits** (`$0C00`) kept; on the 2C02 `t` = `v`.
- **Done in the standard core** (`Ppu2C02::address_load_collision` and the `$2006` write, PAL
  only): judge `c3cjudge.py` at e3 (dot phase 2, phase -14), long runs (> 2 dots) on rendered
  lines per aligned frame **55-60 before, 3-6 after**; golden-core 98 games x 1800 frames
  identical; zax-nes tests green.
- **Final state (2026-11-16)**: `c3cphases.py`, every reset phase, the aligned frames: **0 runs
  longer than 2 dots** on rendered lines at e0 (dot phase 0, phase 9; frames 1-2 not paired,
  the sliding program's access sequence differs there), e3 (2, -14), e6 (1, -14) and e9 (0, -14).
  golden-core 98 games x 1800 identical, zax-nes tests and clippy green.
- **The two leads below**, seen in the 3-6 runs, are both done:
  * (done 2026-11-16) a write on dot 254 ended at `$2858` in Zaxnes, `$2C58` in the netlist. **Not
    a delay**: counted from the write's own dot on both sides (`c3cdelay.py`; the netlist's A
    line comes when /CS rises, after the access's dots), the delays agree write for write (2
    or 3 dots, the same cycle phases). The load collided on dot 256 and the prepaid run made
    dot 257's horizontal copy before the `$2006` write set `v = t`, losing it; the 2C02 hides
    it (`t` = `v` after its collision). Fixed in the `$2006` write: PAL, load on dot 256, the
    copy is made again after the assignment.
  * (done 2026-11-16, `Ppu2C02::first_address_write_on_copy`, judge 3-6 -> 1-2 long runs a
    frame, golden-core identical) a **first** `$2006` write on dot 257: the netlist's horizontal copy already takes the new
    nametable-X bit from `t`, Zaxnes copies the old one (22 dots, until the next copy).

### C3c-corpus. The PAL corpus at the four reset phases (2026-11-16)
- All eleven programs (`g2000`, `g2001px`, `g2005`, `g2005px`, `g2006px`, `g2006w`, `g2006x`,
  `g3e01px`, `oam`, `r2007`, `rmw`) traced on Visual 2C07 + the 2A03 netlist at reset phases e0,
  e3, e6, e9 (`runcorpus.ps1`) and judged with `c3cphases.py` (Zaxnes dot phase / offset searched
  per trace, then runs longer than 2 dots on rendered lines of the aligned frames).
- **`$2007` read on a pre-render vertical copy dot** (`r2007` e0, 240 lines off): a read whose
  X+Y increment latches on dot 278, the first PAL vertical copy dot, turns `v` = `$7040`,
  `t` = `$2000` into `v` = `$0001`, `t` = `$0000`. The increment of `$7040` is `$0061`; the copy
  and the increment meet on the vertical bits and **both registers keep the AND** (`$0061 &
  $2000`), as in the `$2006` load collision (low side wins); the horizontal bits keep the
  increment. At e3/e6/e9 the same read latches on 277, before the copy, and `t` stays. A read
  latching on dot 300 fits the same rule at every phase (`$2000 & $3002` keeps `t`). Done:
  `Ppu2C02::read_increment_meets_vertical_copy`, PAL. **Odd latch dots alike**: `g2007v`,
  aimed at the window (four `t` patterns from `$2000` / `$2005`, six reads latching on dots
  268-291), at e3: a read latching on 289 turns `$7BC2` / `$7BC0` into `$0BC3` / `$0BC0`, one
  on 285 `$5A81` / `$5A80` into `$4A82` / `$4A80`; `g2007v` matches at the four phases.
- **`$2001` switch delay by dot phase** (`oam` e0, `g2001px` e9, `g3e01px` e9): each program
  switches at one dot accumulator per reset phase, so the four phases of `oam` and `g2001px`
  cover switch on and off at every accumulator. Measured by sweeping the delay table
  (`c3csweep.py`, harness override `ZAX_PAL_2001`, dot-trace build only): **3 dots on
  accumulators 0-2, 4 on 3-4**, the split of the `$2007` read latch (5 / 6). The previous
  {0, 2} -> 3 table left 58 311 dots in long runs at `oam` e0 and 371 915 at `g2001px` e9;
  the new one 0 on all eight traces. Done in `switch_rendering`.
- **Final state**: the eleven programs and `g2007v` at every phase, 0 runs longer than 2 dots in
  the aligned frames, with the dot phase each reset phase takes (e0 0, e3 2, e6 1, e9 0): a
  search from dot phase 0 (`corpus-phases-final.txt`) can align the accesses of `g2001px`,
  `g3e01px` and `oam` at e3/e6 on dot phase 0 too and then shows false runs. `g2005` e0 and
  `rmw` e0 align on dot phases 1 and 3 and pair one frame each (the sliding programs' access
  sequences differ in the others). golden-core: the
  291 test ROMs identical to `core-goldens.jsonl`, the 18 games of `demo/roms` x 1800 frames
  identical to the previous build; zax-nes tests green.
- **Leads left**:
  * (done 2026-11-16) the NTSC 2C02 does the same: `g2007n` (`g2007v` timed for 262 lines) on
    the transistor netlist at reset phases e0/e2/e4/e6, nine events of e0 and e4 on dots
    280-292 all fit the AND rule (`andrule.py`), dot 280 (the first copy dot, `v` taken before
    the copy) included; Zaxnes matches at the four phases (alignment 0 at e0/e2, 2 or 3 with
    the power-on in vblank at e4/e6). The old NTSC `r2007` traces now show one frame of long
    runs, a pairing artifact: the aligned frame's preceding pre-render line is not aligned
    (offsets -1/-2), so Zaxnes's read latches on 280 where the netlist's latches on 286;
    golden-core identical;
  * (2026-11-16) **the NTSC corpus at the four reset phases** (`runcorpus_ntsc.ps1` added e0/e2
    for `g2000`, `g2005`, `g2006x`, `g3e01px`, `oam`, `r2007`; judge `c3cntsc.py`): e4 takes
    alignment 3 and e6 alignment 2, power-on in vblank, and every program is clean there but
    `r2007` (the pairing artifact above) and `rmw` (unpairable with this bench: its `v` follows
    the netlist's power-on `$2001`; what it measures, the bytes read back and copied to
    `$2005`, is `00 01 02 01 01 05 06 07 00` in the netlist at the four NTSC and four PAL reset
    phases and in Zaxnes on both regions, `INC $2007` over 03 04 leaving 01 01). At e0/e2
    (alignments 0/1) `g2000`, `g2005`, `g2006x`, `r2007` are clean; `g3e01px` and `oam` hit
    step 33's switch-on race on every switch on (`A 0 86 W2001=0A`: `v` `$0006` -> `$0000`, a
    horizontal copy from `t`, then fetches), present or not with the bench's hold time: not
    trusted, not ported, as before. **Settled at half-step resolution** (adversarial review
    point 7, `visualnes-bench/benchdbg.cpp`, `g3e01px` e0): `v` takes `t` in the very
    half-step where /CS rises and `rendering_1`/`rendering_2` go high, and the copy node
    `copy_vramaddr_hscroll` is never seen high; on a normal dot 257 the same node is high for
    four half-steps (`v` `$1C01` -> `$1800`). The switch-on copy is a pulse inside one
    zero-delay settling step, an evaluation-order race of the switch-level simulation, not a
    timing the chip holds;
  * (2026-11-16, review point 3) **the PAL bench's M2 duty**: `benchdotpal` drove the 2C07
    with the 2A03's M2 (15/24); the 2A07's is 19/32 (nesdev "CPU ALL"). Bench
    `benchdotpal57` (the review's glue variant: /CS and the hold time follow M2 falling at
    19/32; the 2A03 netlist keeps its own phi2), whole PAL corpus re-traced at e0/e3/e6/e9
    (`visual2c07-tools/dot57`, `runcorpus57.ps1`, judge `c3cphases57.py`, sweep
    `c3csweep57.py` with `ZAX_PAL_2001` / `ZAX_PAL_LATCH`). **Both PAL tables move**: the
    `$2001` switch takes 4 dots on dot accumulator 4 only (3 on 0-3), the `$2007` latch 6 on
    accumulator 4 only (5 on 0-3); the earlier {3, 4} splits leave 890 605 and 1 185 385
    differing dots there. Done (`switch_rendering`, `read_buffer_from_fetch`): **the eleven
    programs and `g2007v` at the four phases, 0 long runs** on the 19/32 bench; golden-core
    identical. The `$2006` delays (2/3) and every collision rule are unchanged by the duty.
    The 19/32 bench is now the PAL reference; the 15/24 traces in `dot/` are kept for history;
  * (2026-11-16, review items 5 and 8) **first `$2006` write on the last vertical copy dot,
    NTSC measured**: `g2006v` (`mk2006v.py`, a `$2006` pair timed on the pre-render line's
    end, first bytes `$2D`/`$3B`/`$1A`/`$25`) on the 2C02 netlist at e0/e1/e3/e4/e5/e6/e7:
    at e3 a first write of `$3B` on dot 304 shows `t` = `$2000` for that dot (open bus `$20`)
    and the copy takes it, `v` `$2D00` -> `$2400`; Zaxnes's `first_address_write_on_vertical_copy`
    gives the same with alignment 3 (open bus), clean at e0/e3/e4/e6. **A `$2007` read
    latching on the pre-render line** (310:338 on the 2C07, the latch on 311:2) takes the
    fetching path, X+Y and the fetched byte (`read_latches_on_pre_render`); `r2007` clean at
    the four PAL phases, pre-render line included. The judges (`c3cphases*`, `c3cntsc`,
    `c3csweep*`) now count the pre-render line (but the first judged frame's, which holds
    the power-on `v`), print how many frames they judged and say NO FRAME JUDGED / CLEAN /
    DIFFERS (review item 4);
  * (2026-11-16, review point 6) **the benches' CHR memory**: `handleChrBus` latches the low
    address on ALE's rising edge and drives the data on /RD's falling edge, where a 74LS373 is
    transparent while ALE is high and the memory answers the level. A level-sensitive variant
    (`visualnes-bench/macros_hold_lvl.cpp`, `benchdot2lvl.exe`), NTSC `r2007` e4 against the
    same run on the edge bench: **`v` and `t` identical on all 851 968 dots** (no C3c rule
    depends on it); 1 806 dots differ in `inbuf`, 114 in the ALE address, mostly the ALE +
    Read case (edge bench: refetch at `$20CA` with the residue byte; level variant: `$2000`).
    The variant does not model the contention of PPU and memory on AD0-7 when ALE and /RD
    overlap, so neither bench is authoritative on bus values; AccuracyCoin (`ALE + Read`,
    `$2007 Stress Test`, written against a console) passes with the current model. Bus and
    buffer values stay judged by hardware tests, not by the netlist; C1's open question is
    still the SMB3 Japan line 193 video (I1);
  * dedicated programs for the value patterns the corpus measures once (`$2001` values other
    than `$0A`/`$1E`/`$00`, a read on the copy dots with `t` having more vertical bits set).

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
  before decimation (per-step cost) — **done 2026-11-15 (FIDPLUS-D1d), differently**: the
  low-pass, its zero and BeesNES's output pole run at the output rate as a filter fitted to their
  analog response (`console_profile::BandFilter`: the model's two poles, impulse invariant, behind
  a 32-tap correction fitted by least squares at setup, weighted by our resampler kernel's
  response, 14 samples of delay; the expansion audio is delayed as much). Test: within 0.002 dB and
  0.2 degree of the model up to 0.35 of the output rate, all 14 profiles, 44.1 and 48 kHz
  (prototype: 0.001 dB); the chain before was 1.6 to 3.9 dB off at the band's top. The output pole
  and the forced low-pass are BeesNES's at its fitting rate, 44.1 kHz (`output / 2 + 100` =
  22 150 Hz, default output `LSNAudioOptions.h:96`, recordings at 44 100 Hz `README.txt:63`),
  fixed whatever our rate. Why not per step: a profile kernel (the sinc through the low-pass)
  needs a tail of 50+ samples for the 3.7 kHz Twin Famicom (2 samples of time constant at 48 kHz,
  4 at 96 kHz), more than the 64-sample ring of the standard state holds. Cost: about 40
  multiply-adds per output sample, inside the profile option only; (2) the overall profile volume (0.34 to 4.5) is not applied,
  only its sign: our output is normalised to full scale, applying it would clip; (3) BeesNES's
  sample box applies HPF0 inside its windowed-sinc resampler, ours after our own resampler (same
  one-pole formula, a few samples of phase apart) — **closed 2026-11-15 (FIDPLUS-D1c), not an
  approximation**: the sample box runs HPF0 at the output rate right after its sinc
  (`LSNSampleBox.h:636-645`, `ProcessHpf` `:285-291`), with the same `a (y1 + x - x1)` and
  `tau = 1 / f` as ours, at the same rate; only its place against the low-pass differs, and both
  are linear and time-invariant, so the order does not change the signal (test
  `fidplus_d1c_hpf0_order_against_the_low_pass_is_immaterial`, 20 000 samples, < 1e-12). Both
  resamplers are linear-phase: a fixed delay, no phase difference; (4) the expansion audio (VRC6, MMC5, N163...)
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
  Run-ahead's in-memory restore and the expansion signal — **done 2026-11-15 (FIDPLUS-D1e)**: the
  in-memory state carries the profile block too, and the block now holds the expansion audio's own
  band-limited signal and its level `M`. Before, with a profile and a board with expansion audio
  (5B, VRC6, MMC5, N163, FDS...), every load reset `M` to 0 but kept the signal, so the board's
  level was added a second time when pushed back (an offset and a thump, 1.6e-3 in the test), and
  run-ahead left the filters 4e-5 apart; both now give the same samples as the machine that went
  on (tests `fidplus_profile_expansion_state_tests`); (6) the
  N20914398 profile was measured with a Sunsoft 5B cartridge and has no low-pass and no
  inversion — kept as BeesNES lists it. Found by D1c, for D1d: BeesNES never leaves the CPU-rate
  low-pass out — a profile with it disabled (N20914398, Twin Famicom 475711 SCART) or above the
  output's Nyquist (PAL PM608755, 64 kHz) gets one at `output / 2 + 100` Hz
  (`LSNApu2A0X.h:725-726`), and every profile adds one more pole there (`m_pfOutputPole[1]`,
  `:733-737`, `:541-546`); ours drops them (about -3 dB each at 24.1 kHz for 48 kHz output, the
  top of the band). Zeros: only N13721267 (144.4 kHz), N34169630 (145.2 kHz) and PM313006
  (188.8 kHz) have one; it lifts the band edge by under 0.1 dB.

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

### G5. CPU / APU get-put parity and the DMC timer at power-on — done as a choice (2026-11-16)

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

- Done 2026-11-16 as a choice, default unchanged: `emu.cpu_apu_parity` ("CPU/APU parity":
  Zaxnes, 2A03 netlist, Random drawn at each load), `NesCore::set_netlist_get_put`, applied at
  power-on only (the reset button keeps the running parity): the other get / put parity and the
  DMC output timer at 431 with 8 bits left, as the CPU differential aligns them. Test
  `fidplus_cpu_apu_parity_tests` (an OAM DMA started on the same cycle takes one cycle more or
  less). Whether a console fixes it or draws it stays open; the choice covers both.
- 2026-11-17, fidelity audit (`ext-tools/fidelity-audit`, gap 6): on the audit's bench (the
  "hold" variant of the netlist bench, reset released at power-on with no extra hold) the 2A03
  netlist aligns with Zaxnes at **DMC timer 1014, 8 bits, the default parity** (`dmcdma`,
  `cpuflow`, `apuL5`/`apuL4`/`apuF`/`apuG`: zero gap); the option's 431 + flipped parity puts the
  DMAs 156 to 166 cycles early there. Both are the netlist, from all-zero nodes: the difference is
  the time from power-on to the reset's release, which sets where the DMC timer and the get / put
  parity stand at the first opcode. So the netlist has no single power-on phase; a console's comes
  from its reset circuit (RC, CIC) and is not one value either. Not changed: the option keeps the
  CPU differential's pair, documented as one bench's. Lead (Fidélité+): the `Random` choice could
  draw the parity and the DMC timer over its whole range rather than between two pairs — no
  hardware reference picks a distribution.

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

### H1. Composite / RF signal, CRT gamma, phosphor decay — partly done (H2, H4)
- BeesNES: NTSC-CRT / PAL-CRT (LMP88959), own L. Spiro filters, measured CRT gamma, phosphor
  decay, signal noise (README "Visual Samples"); palette generator palgen-persune.
- Zaxnes: librashader slang shaders (branch feat/librashader). These are generic shaders, not a
  signal model. Candidate shaders/palettes to port; not a core item.

### H4. Signal options and television colours — done (REWRITE-UI step 23, 2026-11-18)
- Source: BeesNES (L. Spiro, MIT, `ext-tools/bees-src`). Its plain "palette" output differs from a
  raw palette because `CBeesNes::BakedPalette()` bakes into every entry a measured CRT gamma curve
  to linear light (`CrtProper2` on NTSC: power 2.31 above 0.36, blended with Rec. 709 below;
  `CrtProper` on PAL: BT.1886 annex 1, alpha 2.6/3.0, lift 0.0181 x 8), a conversion from the
  tube's primaries and white to the monitor's (no white adaptation by default) with gamut mapping
  "preserve hue" (towards the luminance until every channel fits), then the sRGB encoding; plus a
  5x internal upscale and a gamma-aware convolution down. Default palettes `2C02G_wiki.fpal` /
  `2C07_wiki.fpal`. The README's "HDMI" / "HDMI Mod" captions name no option in the code.
- Zaxnes (`zax-nes/src/tv.rs`): `video.tv_model` = modern screen / NTSC tube (RCA ColorTrak
  E13169GM, patchy68k's measures, CrtProper2) / Japanese tube (Toshiba CF2005, CrtProper2) / PAL
  tube (Sony Trinitron 6900 K, DariusG's measures, CrtProper). Applied to the 64-colour palette
  (`zax_engine3d::palette`, 2D and 3D) and to the composite decoder's output, with the TV knobs
  (tint, colour, contrast, brightness on YIQ / YUV).
- Decoder (`composite.rs`): `video.signal_kind` composite / S-Video (luma = the pixel's mean
  level, chroma = the rest: no cross-colour) / RF (wider filters, snow, a ghost 20 samples late);
  `video.decoder_sharpness` (luma 18/12/8, chroma 36/24/16 samples); `video.comb_filter`: a
  3-line comb, not BeesNES's — the 2C02 subcarrier moves 4 of 12 phases a line, so three lines
  cancel the chroma of a vertically flat area exactly; the line difference is high-passed over a
  cycle so that vertical luma edges stay luma. `video.frozen_dot_crawl`: constant phase.
- Approximations to check: the RF snow and ghost amplitudes are by eye; S-Video's split is
  perfect (a real Y/C separation of a NES needs a mod and has its own filters); the knob ranges;
  BeesNES's phosphor decay and its gamma-aware resampling are not taken (shaders' business).
- Open: BeesNES's `2C02G_wiki` palette as a `video.nes_palette` choice; its other measured sets
  (Panasonic CT-36D30B, Sony CXA2025/2060 decoders: `LSN_DM_*` demodulator matrices).

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

- I1 update (2026-11-16, replay on the 2C02 netlist, `ext-tools/smb3-netlist/RAPPORT.md`): the
  game's line-193 sequence (`$A82F`: `$2006` = `$0B`, `$00`, `LDA $2007`, then `$2000`/`$2005`;
  state dumped with Mesen) rebuilt as a sliding NROM program (`mksmb3.py`, `smb3p0.nes`) and
  run on three cartridge models (edge, transparent 74LS373, transparent + memory delay, the
  last one honouring AccuracyCoin's ALE + Read). **C1's timing is confirmed**: latch X = R+5
  (e0/e2) or R+6 (e4/e6), the shape a function of X alone, `v`/`t` identical on all three
  benches. Zaxnes equals the delay bench at every X but X = 8k+6 (the netlist re-reads the
  attribute byte through the open latch -> palette 1, dark blue; Zaxnes black) and X = 8k
  (invisible here); the video shows no dark blue, so not ported. The video's classes match the
  alignment-0/1 weights (22/33/34/11 % against 19/33/33/15 %: the console is in that family,
  which also explains "never empty": X = 239 exists only in 2/3). Still unexplained by every
  digital model: the dash shape at X = 231/232 and the white at X = 235 (ALE / /RD analog
  contention). Hardware test proposed: film `smb3p0.nes` on a console. New lead (C2): the
  netlist loads `v` in the middle of W+3 and W+4's ALE already carries the new address, where
  Zaxnes fetches W+4 with the old one (`211 netlist bus=2BF0 | zaxnes bus=2BEF`), invisible
  to `dotcmp.py` (`v`/`t` only); to measure with a dedicated program.
- I9 (2026-11-17) **`$2004` read during sprite evaluation: the netlist ahead by 2 to 5 dots —
  open** (fidelity audit `oam`, `latmap.py oam 2004 net\oam-e0.txt 129000 5`, replayed after
  I6-I7, same figures). Dots 1-64, 257-320 and 321-0 fit at the usual k = +1 (128/128, 107/116,
  42/42); in 65-256 only 282 of 469 do, and the misses are the netlist showing the byte Zaxnes
  shows 2-3 dots later on line 35 (sprites 7-8 in range), 5 on line 48 (11-12), 2 on line 63
  (14-15), so the evaluation's OAM address runs ahead in the netlist by an amount that depends
  on which sprites are in range. Not trusted yet: on that bench `$2003`/`$2004` writes do not
  reach the cells (OAM set by `sprite_write`, `OAMADDR` never written), and the map pools 19
  phases. Next: a dedicated per-dot trace of the OAM address node (`spr_ptr`) on one line with
  a known in-range set, against Zaxnes's exact evaluation (`ppu-exact-sprite-eval`), which
  passes AccuracyCoin's evaluation rows.
  Done the same day (`visualnes-bench/benchspr.cpp`, nodes `spr_addr0-7`, `spr_ptr0-5`, program
  `oamn`, OAM by `sprite_write(a, a)` and Y = 100 for sprites 50-59): the pointer moves as
  expected (2 dots a sprite, 8 for a copy: on the line labelled 48, copies of `$28`-`$2F` on dots
  67-106, then `+4` every 2 dots), but the copied set is Y = 40 and 44 with Y = 48 rejected —
  the set every emulator, and the games, select on line 47 (a sprite at Y is shown on lines
  Y + 1 to Y + 8). The bench's OAM labelling or cell mapping is one line off from what its
  `vpos` says for the background, which the dot-perfect `scr` match vouches for; the 2-5 dot
  "lead" follows from a different in-range set, not from Zaxnes's evaluation. Kept open only as
  a bench question; no Zaxnes change.
- I8 (2026-11-17) **Alignment-dependent races of the 2C02 netlist that hardware tests contradict
  or that are a simulation artefact — not ported** (fidelity audit, gaps 3 and 4). Both replayed
  on a second bench whose /CS is decoded from the 2A03's own M2 (`visualnes-bench/benchdot2m2`,
  `fidelity-audit/benchaudm2`, `macros_m2.cpp`) besides the hold-time bench: **identical tables
  on both**, so neither comes from the /CS model.
  * `$2002` / start of vblank (`vblA`, `vblC`): alignment 0 (e2) is Zaxnes's, 14 frames of 14.
    Alignment 1 (e0) takes the NMI on a read at 241:2 (Zaxnes suppresses it); alignments 2-3
    (e6/e4) also read 0 and suppress the flag on 240:340, and take the NMI on 241:2. A model of
    exactly that (the /NMI sample one dot later in alignments 1-3, the suppression on 340 in 2-3)
    **fails `ppu_vbl_nmi` 02/05-08, `vbl_nmi_timing` 2/3/5/6/7, `cpu_interrupts` 2 and
    AccuracyCoin's `NMI at VBlank end`** in alignments 1 and 2 — hardware-verified tests that
    pass on consoles of any alignment. Reverted, as I5: the netlist's timing of this race is not
    the console's. Zaxnes keeps one table for the four alignments.
  * `$2001` toggles in alignments 0/1 (`scr` e0, 54 runs): at a switch on, on the first dot that
    fetches, the netlist ANDs the low 12 bits of `v` with the address the pipeline puts on the bus
    at that dot (`$7114` -> `$7004` with the low pattern address `$0007`, `$7509` -> `$7501` with
    `$0557` for tile `$55`, `$7110` -> `$7100` with `$0107`; nothing on a nametable address dot,
    `$2000 | v`), bits 12-14 kept; at a switch off landing on an increment dot, the increment is
    partial (coarse X wraps without flipping the nametable, X without Y on 256, a coarse-Y carry on
    113). This is step 33's switch-on race (A3c): the review's half-step trace showed `v` change
    inside one zero-delay settling step with no copy node high, an evaluation-order effect of the
    switch-level simulation. The AND model is recorded here, not ported; a console test (a switch
    on in each alignment, `v` read back through `$2007`) would settle it.
- I7 (2026-11-17) **`$2007` increment on dot 257, and `$2000` on the last vertical copy dot —
  fixed** (fidelity audit, `scr` e4). (1) A `$2007` read or write whose increment lands on dot 257
  meets the horizontal copy: **`v` and `t` both keep the AND** of their horizontal bits (the
  incremented `v`'s and `t`'s), as on the vertical copy dots; the write on 43:251 turns
  `$0131` / `$3F6F` into `$1122` / `$3B62`, the read on 129:251 `$5111` / `$316F` into
  `$6102` / `$3162`. A5's rule (the copy wins, Y only) is the case where the AND gives `t`, which
  is what Zelda II's split had. (2) A `$2000` write on 261:304 reaches that dot's vertical copy
  (`v` takes the written nametable-Y bit, `$396F` -> `$316F`), like the first `$2006` write there.
  `increment_meets_horizontal_copy`, `control_write_on_vertical_copy`, the Fidélité+ edge keeps
  `v` from before the dot (`v_before_dot`). With I6: **`scr` e4 against alignment 3, Fidélité+,
  0 differing dots in 4 frames** (136 W2007 runs, 1 W2000 run and 16 inherited before); e0
  against alignment 1 has only I8's `$2001` runs left.
- I6 (2026-11-17) **The increment of a `$2007` write made while rendering — fixed** (fidelity
  audit, gap 2). The 2C02 netlist (audit `scr` at reset phase e4) gives the write the read's
  timing: `v` takes its X+Y increment on the read's latch dot X (`D + 6` in alignments 2 and 3,
  `D + 5` in 0 and 1), merged with the pipeline's own increments as the read's is (A5): on a
  dot that increments coarse X anyway one X plus Y, on 256 nothing more, on 257 the AND of I7. Line 8,
  a write on dot 112 leaves `v` = `$652F` to dot 117 and makes it `$7530` on 118; line 41, a
  write on dot 66 turns `$1539` into `$253A` on 72, not `$153A` + X + Y. Zaxnes made it at
  the access: 136 of 150 writes differed for five dots, 14 for a whole line. Now: Fidélité+
  posts an edge (`EdgeKind::WriteIncrement`, after X's work) — `scr` e4 `W2007` class 136 → 0;
  the standard path does the same with its run-ahead (`write_increment_from_fetch`) under
  `emu.ppu_bus_conflicts`, off by default like the read model (C1) — 147 → 11 runs of one dot,
  where the per-dot trace records X before the increment that follows its work (`v` the next
  fetch uses is the netlist's). AccuracyCoin has no `$2007`-write-while-rendering test (its
  `$2007 Stress Test` reads), so the netlist is the reference. The byte is still stored at the
  access: the trace compares `v`/`t`, not the address a write during rendering reaches, which
  remains to measure (at X - 2 the netlist's bus shows `$2700`, not a fetch address). Duelito,
  which writes `$2007` with rendering on in its first frames, changes picture (golden row
  updated); 18 games, MMC5 set and AccuracyCoin 147/147 unchanged.
- I5 (2026-11-17) **DMC IRQ latency: the netlist against the hardware; Zaxnes keeps the
  hardware's** (fidelity audit `ext-tools/fidelity-audit`, gap 1). The 2A03 netlist (audit
  bench `benchaud`, and `benchirq` with the nodes) raises `set_pcm_irq`/`pcm_irq` two cycles
  after the DMA read R of a sample's last byte and the 6502's `c_irq` on R+3, 78 cases of 78;
  Zaxnes raises the flag on R. A model with the flag on R+2 (what `$4015` reads; blargg's
  `7-dmc_basics` #19 needs it there at the latest) and /IRQ on R+3 made the audit's `cpuflow`
  match to the cycle (0 of 41 705, 79 IRQ latencies of 79) — and **failed AccuracyCoin's
  `Interrupt flag latency` test B** (`AccuracyCoin.asm` `TEST_IFlagLatency_PageBoundaryTest`:
  a taken branch crossing a page, the DMC DMA between its cycles 2 and 3, must take the IRQ at
  the poll before cycle 4, i.e. one cycle after the read). AccuracyCoin is verified on the
  console, and the DMA's own placement is confirmed by both (audit `dmcdma`, 66 DMA, and the
  DMA tests of blargg and AccuracyCoin), so the netlist's IRQ path is what disagrees with the
  hardware here, as for the seven unofficial opcodes the audit lists. **Reverted; Zaxnes keeps
  the flag on the read.** To revisit only with a console measurement of that latency.
- I4 (2026-11-17) **Sprite phase and idle dot of the PPU bus**. **Fixed**: each sprite slot's
  two garbage nametable fetches (ALE 257 + 8k and 259 + 8k, both `0x2000 | v` in the 2C02
  netlist; Mesen reads an attribute address on the second, the netlist does not) are now put
  on the bus (`notify_discarded_read`, no memory read: the byte is discarded), and a `$2006`
  load landing on one of them reaches it (W+4, write on 269 seen on 273). The idle dot 0 of
  lines 1-239 (and of line 0 when not skipped) shows the coming low-plane pattern address, as
  nesdev describes and the netlist shows (`1002` on `g2000` line 10), address lines only.
  Bus comparison (`g2000`, `g2005px`, `g2006x`, `g2006w` e4): every dot 0 and every
  garbage fetch now matches; the sprite pattern addresses still differ only because these
  programs leave OAM uninitialised and the netlist's power-on OAM differs. **Consequences**: an
  A12 watcher now sees A12 fall between sprite slots, which is what the Acclaim MC-ACC's ÷8
  prescaler divides — it counted one line in eight before, now one per line (test
  `mr_4_3_the_mc_acc_counts_one_line_per_eight_falls_while_rendering`); with 8x16 sprites
  mixing both tables an MMC3's low run between two high slots can now reach the filter, as on
  the hardware. `Mmc5Ext`'s line-start test counts pattern fetches since the last
  **attribute** fetch (threshold 18: the line's last group's two plus the sixteen sprite
  fetches), since the garbage fetches are nametable reads whose address the pre-render line's
  vertical copy changes mid-phase. **Line 240, measured at the pins** (`benchpins`: the 14
  address pins, ALE, /RD and /WR every half-step, `g2000` e4): dot 0 gives an ALE with the
  pattern address (`1000`, /RD high), and from dot 1 the pins show `v` (`0802`) and hold it
  through vblank until the pre-render line's first fetch — now raised to the A12 watchers
  (`Ppu2C02::post_render_bus`). **Cost** (callgrind, 300 frames, clean builds of `0602c292`
  and `f1b67088`): Super Mario Bros. 3 (MMC3) +0.20 %, Mega Man 2 -0.06 %, Zelda II -0.09 %.
  The bus events are compiled out of the kinds whose board never watches the bus
  (`BoardKind::WATCHES_BUS`), the MMC3 kind skips the garbage fetch that cannot move A12
  (`BoardKind::A12_ONLY`), and the rendering `match` keeps its range-compare shape (an arm of
  its own for dot 0 or 339 had made it a dispatch, +33 M on Mega Man 2). What is left on the
  MMC3 is its watcher seeing seven more A12 events a line, the price of the MC-ACC's count.
- I3 (2026-11-17) **Line end of the PPU bus, and the MMC5's first detected line (Q-121)**.
  **Fixed**: the nametable reads of a line's end are dots 337 and 339 and the next line's dot 1
  (nesdev, MMC5: "two matching dummy nametable byte reads at the end of each scanline, followed
  by a third matching nametable byte read at the beginning of the next scanline"; Mesen
  `NesPpu` the same), where this core read 337, 338 and 340; dot 1's read now gives the third
  tile its byte. A `$2006` load landing on 337 or 339 (writes on 333/335) reaches those dummy
  fetches too. Bus comparison (`g2000`, `g2005px`, `g2006x`, `g2006w` NTSC e4): background,
  dot 1, pre-render and line end all 0 mismatches. **Q-121 reverses**: line 239 now leaves two
  reads, not three, so the pre-render line is detected only when its dot 1 repeats line 239's
  address (nothing moved `v` in the vblank); the usual frame (a `$2006` in the NMI) is first
  detected on line 0 and `$5203` = N fires on line N — Mesen (`DetectScanlineStart`) and
  `BizHawk` (`ExROM`, `irq_target + 1` from its pre-render line 0) agree, while MF-4c's claim
  that Mesen gave N-1 was wrong. Programs that leave `v` alone (the `mapper_irq_tests` fixture,
  `mmc5test`, `firefly`) keep N-1. A state load rebuilds the repeat from the PPU's `v`
  (`Mapper::after_state_load`), so run-ahead fires on the same line (test
  `q_121_the_mmc5_target_line_survives_a_state_load`). Castlevania III: picture at frame 3 600
  and audio from 1 200 move back, the exact reverse of MF-4c step 5. Both MMC5 variants checked
  (the split column skips the repeats as before). **Left**: the sprite phase's garbage nametable
  fetches (257/259 + 8k) are still not put on the bus; adding them touches the MMC3 A12 filter
  and the MMC5 split's `chr_reads_since_nt`, to study on its own.
- I2 (2026-11-16) **PPU address bus against the 2C02 netlist** (`visualnes-bench/buscmp.py`:
  every ALE address of the netlist on fetching lines against the Zaxnes trace's bus, by fetch
  phase and dot % 8; the netlist's power-on `$2000`, never written by these programs, has the
  background table bit and 8x16 sprites set, so bit 12 differences and the sprite phase are
  power-on artifacts). **Fixed**: a `$2006` load landing just before a nametable or attribute
  fetch (W+4 = 8k+1 / 8k+3, also dot 1 and 321) now makes that fetch with the new `v`, as the
  netlist (`delay_address_load`); this core reads the byte on that first dot, so the tile or
  attribute came from the old address. `g2006x`: 1 232 background mismatches -> 0; `g2006w`
  (four targets, every dot), `g2000`, `g2005px`: 0 but dot 1 (`g2006px` and the other
  rendering-off programs have no fetch to compare once their first frame starts). golden-core: RasterTest1/2 only,
  2 pixels at x 254-255 of line 210 (rows updated). **Left as leads** (boards watching the bus
  only): the first nametable fetch of a line (dots 1-2) is not put on the bus (its byte comes
  from dot 340's read, same address), and the line end reads at 337/338/340 where the chip
  fetches at 337/339 then dot 1; Zaxnes's MMC5 line detection (`nt_read_counter`, three
  identical reads) is tuned to the current sequence, so the two must change together (adding
  the dot-1 read alone would move MMC5 detection by two dots). Both MMC5 boards depend on it:
  the light `Mmc5` (line detection) and `Mmc5Ext` (`board-mmc5-exattr-split`, on by default),
  whose split column counts nametable fetches with a new address and finds the line start by
  the run of pattern reads before it (`chr_reads_since_nt`, i.e. the sprite phase's reads), so
  a faithful line end or sprite-phase NT fetches must be checked on both, with the split games.
  The W+4 fix itself changes no fetch count, only the address one fetch carries after a
  mid-line `$2006` write (MMC5 tests green, light/full pixel comparison included); the sprite phase's garbage
  nametable fetches (257-320) are not put on the bus either.
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
- I6. (lot REWRITE-UI, 2026-11-18) **Fidelity+ and the widescreen overclock.** The widescreen
  extension's overclock (`$5006`) holds the PPU still while the CPU runs extra cycles to write the
  margin tiles; Fidelity+ lands `$2006`/`$2007` effects a number of dots after the access, and no
  dot runs during a stall, so the writes piled up unlanded: black blocks and bands in the margins
  and the picture (Blaster Master widescreen, owner's savestate). Fix: no edges while
  `WideExtension::stalling()`; those writes take the standard core's timing. An approximation
  inside an approximation (the overclock has no hardware counterpart); fidplus test files pass.
- I5. (lot FAST-1, 2026-11-17) **A savestate reload is not always the run that never stopped.**
  `fast_core_tests.rs`, `fast_1_savestates_cross_both_ways`: after 300 frames with a button
  pattern, a state saved and loaded on a fresh core continues identically on the standard and
  the fast core (320 ROMs), but on `RasterChromaLuma.NES` (test ROM) the first frame after the
  load differs from the same machine left running, **standard core included**. Something that
  frame depends on is not in the state (candidates: the picture buffer the game only partly
  redraws, a mid-frame emphasis/palette record, a value outside the PPU block). The sound after
  a load also restarts differently (the resampler's position is not in the state), expected.
  Not yet looked into; the test compares against the reload on the same core.

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

### J38. FDS (mapper 20): Mesen 2's drive, its choices kept — partial
- Hardware: the RAM adapter (timer IRQ, drive serial interface, `$4030-$4033`), the wavetable
  channel with its modulator, and a real disk drive turning at its own speed.
- Zaxnes: a port of Mesen 2 (`Core/NES/Mappers/FDS/Fds.cpp`, `FdsAudio.h`, `ModChannel.h`,
  `BaseFdsChannel.h`, `FdsLoader.h`, `FdsInputButtons.h`) in
  `zax-nes/src/cartridges/mappers/fds.rs`, behind `board-fds`, loaded only from a `.fds`/`.qd`
  packaged with the player's `disksys.rom` (`zax_nes::rom::package_fds`); a bare iNES 20 stays
  refused. Mesen's choices kept:
  - the drive: 50 000 cycles to reach the start of a side, 149 between bytes (Mesen's note on
    *Ai Senshi Nicol*), the gap before each block made up (`AddGaps`: 28 300 bits, then 976), a
    `.fds` block's CRC a placeholder (`$4D $62`), an IRQ at the end of the disk (*Kosodate
    Gokko*);
  - unanswered bits of `$4030`/`$4032` read as open bus with `$40` assumed (`OPEN_BUS_40`)
    rather than the last value on the bus;
  - the audio's level through the 2A03 pulse mixer's curve with a gain of 2.4 (`FDS_GAIN`), no
    low-pass filter (the console has one, ~2 kHz, which Mesen leaves out too);
  - the automatic disk changes (`ProcessAutoDiskInsert`, the `$E445` header match, the
    `$4032` polling counter) — Mesen matches at the `$E445` read itself, Zaxnes at the end of
    that frame (the drive waits 50 000 cycles before it reads, so nothing reads the disk in
    between); no fast-forward while a disk loads (Mesen's `FdsFastForwardOnLoad` is not ported).
- Saves: the written disk's changed 256-byte blocks in `<rom>.flash` (`ZXFD`, v1), applied over
  the original image at the next launch; Mesen keeps an IPS patch instead.
- Not verified on hardware or on a real BIOS: no `disksys.rom` was available to this lot, so
  only the unit tests (`zax-nes/tests/fds_tests.rs`) cover it; `fdsirqtests` (in
  `cleanroom/assets-free/test-roms`) is the first check to run once the owner supplies one.
- Cost: a few cycles per CPU cycle on mapper 20 only; status **partial**.

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
* 559 (`mapper_559.c`): puNES maps its four nametable registers with `memmap_nmt_1k`, whose
  values (`$E0`/`$E1` at power-on) are read here as a CIRAM page from D0; what the upper bits
  select on the board is not documented anywhere this lot can read.
* 555 (`mapper_555.c`): the timer's length is `(DIP | $10) << 25` CPU cycles and the DIP is 0,
  so a round lasts about five minutes (puNES's default); puNES's on-screen countdown is a host
  overlay and is not reproduced. `$5000-$57FF` is the work RAM's 4 KiB bank 2, so an image whose
  header gives 8 KiB or less has it alias `$6000-$6FFF`; nesdev's page gives no RAM size and the
  header's is used as it is. The 8 KiB CHR-RAM beside the CHR-ROM is assumed, as for the TQROM.
* 471 (`mapper_471.c`): puNES asserts when the pattern address it tracks through the line goes
  from the `$0000` half to the `$1000` half (background vs sprite tables, plus `$2006`); here the
  same edge is the MMC3's filtered A12 rise, which agrees for the one-rise-per-line layouts and
  may differ for a game rendering 8x16 sprites from both halves.
* 558 (`mapper_558.c`): the EEPROM is mounted only when the NES 2.0 header declares 512 bytes of
  PRG-NVRAM, as puNES decides; an iNES 1.0 dump of an EEPROM cartridge gets the bare port
  (`$5200` D2 read back) and cannot save.
* 272 (`mapper_272.c`): puNES steps the counter on every eighth dot of a rendered line, from
  dot 5 (42 steps a line, 84 = two lines). Zaxnes has no per-dot hook for a board, so a step is
  a low-plane pattern fetch, of which a rendered line has the same 42 (34 background slots and 8
  sprite slots). The phase inside the line differs by a few dots; the pre-render line's fetches,
  which puNES's line test excludes, and `$2007` reads of the low planes also step it here.
* 359/540 (`mapper_359.c`): the A12 mode counts raw rises of the address the PPU drives, behind
  puNES's five-CPU-cycle filter; puNES derives the rise from the background and sprite pattern
  addresses at fixed dots instead, which agrees on ordinary layouts.
* 528 (`mapper_528.c`): the IRQ is Zaxnes's `VrcIrq` (the VRC4/VRC6/VRC7 counter); puNES's VRC7
  asserts one CPU cycle after the overflow (`irq_delay = 1`), Zaxnes on the overflow itself.
* 556 (`mapper_556.c`): on a switch of chip puNES re-runs the new chip's whole fix, mirroring
  included; here the windows are rebuilt but the mirroring stays the last one either chip wrote
  until the new chip writes its own.
* 351 (`mapper_351.c`): puNES appends the CHR-ROM to the PRG-ROM at load and, in the CHR-RAM
  mode (`$5002` D0), lets the PRG banks reach it (a multicart keeps an UNROM game there) while
  the PPU sees CHR-RAM. This core's PRG windows wrap on the PRG-ROM alone and its CHR memory is
  one chip, so that mode maps the image's own CHR and PRG banks past the PRG-ROM wrap. The DIP
  switches read 0 (J33 for the open-bus bits).
* 560 (`mapper_560.c`): in bank 0 puNES's read is `chrrom_byte(((address & $3FF) << 13) &
  $10000) | base`, the closing parenthesis putting `| base` outside the read; the evident
  intent, the byte at `(A3 as A16) | base`, is what this board reads. The nametables are the
  board's own copy of the console's 2 KiB, mirrored as the header says.
* 406, 451 (`SST39SF040.c`): puNES's program command stores the byte (`data[address] = value`);
  a flash cell can only go from 1 to 0, so this board `AND`s it, as `FlashRom` does for 111 and
  682. The erase busy time is puNES's, one CPU cycle per erased byte (65 536 for a sector, the
  chip size for a chip erase), not the datasheets' milliseconds; the toggle bit and `& $77` read
  are puNES's too. Any command byte other than those listed falls back to the first unlock step
  (puNES's `default:` arm), and the chip address is the PRG-ROM offset through the MMC3 banks,
  so a game that unlocks through a window not mapped as puNES maps it would differ only if the
  two cores' bank maps differed.
* 547 (`mapper_547.c`): puNES turns a pattern fetch to the QT byte's bank when its address
  equals the PPU's current background address (`bck_adr`); this board does it for every fetch
  the core tags `ChrFetch::Background`, which also covers `$2007` CHR reads and host peeks, so a
  `$2007` read of CHR-RAM after a tile fetch whose QT byte has D6 set reads the kanji ROM here
  where puNES would read the CHR-RAM. The QT byte is latched by every nametable read below
  `$x3C0` the PPU bus carries, `$2007` reads included, as `extcl_rd_nmt_547` does. The work RAM
  is the header's, wrapped (puNES sizes it the same way).
* 561 (`mapper_561.c`, `extcl_cpu_init_pc_561`): puNES runs the copier's trainer by writing
  `JSR init` / `JMP ($FFFC)` at `$0700` and starting the CPU there; this board does the same
  through `Mapper::boot_program`, after the CPU's reset sequence, so the seven reset cycles
  happen before the stub instead of being skipped. puNES also clears `$4015` D6 and the APU IRQ
  line at that point; the APU here has just been reset and both are already clear. The FDS-style
  timer is puNES's signed 16-bit counter, which wraps while `irq.reg` D7 is clear exactly as
  puNES's does. A soft reset keeps the copier RAM (puNES reloads it on power-up only).
* 562 (`mapper_562.c`): the Venus Turbo Game Doctor's BIOS is embedded as puNES carries it, by
  the owner's decision (2026-11-15): the 2 KiB of `bios[]` (declared 4 KiB, read through
  `& $7FF`) in `data/vtgd_bios.bin`, a firmware dump and not emulator code. The boot follows
  561's notes above. The `$4420` read-back tracks the last pattern fetch on the board's bus and
  the last CHR write, as `extcl_rd_chr_562`/`extcl_wr_chr_562` do; a CHR write takes the
  permission mapped before it changes the lock, as puNES's `chr_wr` does. The copier DRAM is the
  image's PRG (wrapped to its size) and is saved in the state, not persisted.
* 355 (`mapper_355.c`): the PIC16C54 is MAME's core as puNES runs it: configuration word 0 (no
  watchdog), T0CKI never driven, one instruction cycle per four CPU cycles (a 1.79 MHz / 4 =
  447 kHz instruction clock, where the board's real oscillator is not documented). The PIC sees
  the CPU's address on reads and writes of `$4020-$FFFF` only, as puNES's `extcl_cpu_*_mem_355`
  hooks; the board's real address bus also carries `$0000-$401F`. Port B reads take the address
  as it was at the PIC's instruction, not at a precise cycle inside it.
* 532 (`mapper_532.c`, UNIF `CHINA_ER_SAN2`): an emulator convention (Nintendulator, VirtuaNES;
  Mesen 2 knows no such board), ported as puNES runs it: the RAM snapshot at the pre-render line
  (`ppu.c`) and the glyphs painted when the frame is done (`gfx.c`). Kept as written: the
  attribute colour is read at PPU `$03C0 + …`, in the pattern table, where an attribute table
  was evidently meant; unsigned wraps of the row arithmetic. Not kept: puNES writes glyph rows
  that fall before the picture before its buffer; they are not drawn here. The overlay is on the
  2D picture only; the 3D conversion's tiles do not carry it. The RAM snapshot is taken at the
  first CPU instruction boundary once the PPU is on the pre-render line, puNES's at dot 0.
* UNIF (`rom/unif.rs`): puNES compares the image's board name as a prefix of its table's names
  (`strncasecmp(table, board, strlen(board))`); this loader compares the whole name. Single-
  screen `MIRR` values (2, 3) have no NES 2.0 header bit and are left to the board. ROM chunks
  that are not whole 16/8 KiB banks are padded by repeating them, as a smaller chip mirrors.
* 186 (Study Box, Mesen 2 `StudyBox.h`): Mesen's model, itself a reconstruction (its own
  comments mark several `$4201`/`$4202` bits as guesses): command `$86` past the last page goes
  back to the first page, command 0 walks back from page 255 when at page 0, `$5000-$7FFF` are
  mapped from power-on by `$4200` = 0 where Mesen maps `$5000` only once `$4200` is written. The
  tape's timings use the NTSC CPU clock whatever the region. The tape's audio is mixed at full
  scale (a WAV sample as the mixer's `M`) and advances at the WAV's rate against the CPU clock,
  where Mesen resamples it to the host's output; 8-bit and 16-bit PCM, channels averaged. The
  whole tape (audio included) is in the image a savestate carries when it includes the ROM.
* 256, 436 (`OneBus.c`): as puNES, the VT02/VT03 are modelled in the console's ordinary 2 bpp
  rendering. In the 4 bpp modes (`$2010` D1/D2) the banks select the de-interleaved low planes
  puNES selects, but the PPU still draws two planes, so those games show what puNES shows, not
  what the chip shows. puNES clocks the IRQ counter at dot 256 of each visible line when `$410B`
  D7 or `$2010` D1 is set and on A12 rises otherwise; this board counts A12 rises in both modes
  (no board hook sees dot 256). The PCM channel reads its bytes through the cartridge only: an
  address below `$6000` (`$4030` D0-D1 = 3 or 2) reads 0 where puNES reads the console's RAM or
  registers. Its byte reaches the APU's `$4011` on the cycle it is fetched, the APU being kept in
  step with the cartridge as for expansion audio. The VT09/VT369 extras (GPIO at `$4140`, the
  `$3000` window, the relative bank of `$4160`) and the iNES-only VT09 detection are absent;
  submapper 2 and 4 detection by PRG CRC on iNES 1.0 images is puNES's.
* 394 (`mapper_394.c`): the MMC3 is this core's `Mmc3`, whose power-on banks (R6 = R7 = 0) are
  not puNES's; a switch from the ASIC back to the MMC3 re-applies the last `$A000` value written
  to it for the mirroring, where puNES's `extcl_after_mapper_init_MMC3` re-applies its own
  `mmc3.mirroring`, which is the same value. The outer CHR bank is ORed over the chip's bank as
  puNES does (`base | (value & mask)`), not masked into it.
* 446 (`mapper_446.c`): the chips are this core's (`Mmc1`, `Mmc3`, `Vrc2Vrc4` wired as 22, 23 and
  25, `Vrc6` wired as 24), reset on the `$5000` lock as puNES's `init_*(…, HARD)`; their windows go
  through the outer bank by `BankWindows::remap_*_banks` over the chip's already-wrapped banks,
  which equals puNES's `base | (value & mask)` on the bank before wrapping whenever the image
  holds the whole outer bank. puNES's TLSROM personality is a plain MMC3 with a 7-bit CHR mask,
  and so is this one (no TxSROM nametable control). SUROM, PNROM, HNROM and VRC1 have no
  `fix_all` arm in puNES and leave the windows as they were here too. The VRC2's `$6000` latch
  read (puNES `extcl_cpu_rd_mem_VRC2and4`) is whatever `Vrc2Vrc4` answers for mapper 22. The board
  reports expansion audio always (0 unless the VRC6 runs), so the APU is stepped every cycle for
  every 446 image. The flash chip uses the shared `JedecFlash` (`SST39SF040.c`, 128 KiB sectors).
* 342 (`mapper_342.c`, COOLGIRL): puNES runs the scanline work (the MMC3 counter, the MMC5 line
  compare, mapper 163's latch at lines 127 and 239) at dot 260 of each rendered line. No hook
  sees PPU dots here, so `CoolGirl` finds the lines on its own bus as the MMC5 does (three
  identical nametable reads end a line) and runs that work on the first fetch at or past 259
  dots into a line; a gap of two lines leaves the frame. With rendering off nothing counts,
  which matches the hardware's view of the bus but not puNES's. The `$5204` in-frame bit uses
  the same tracker. The save area (the top 8 MiB of the 1 GiB bank space, puNES's
  `SAVE_FLASH_SIZE`) is in the savestate and, since the flash-save batch, in the host's
  `<rom>.flash` file as puNES keeps it in the game's save file (`Mapper::export_save_area`; only
  the written 8 KiB pages are stored). Unwritten save pages read 0, as puNES's
  zeroed buffer does, not `$FF` like an erased chip. The PRG flash outside the save area is
  read-only here, as puNES's ROM is. Mapper 75's `$9000` write sets `chr.e` from `chr.a`'s other
  bits, a reading of puNES's code kept as written.

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
