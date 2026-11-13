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

### A4. CPU/PPU alignment variants — open, Fidélité+
- Hardware: four CPU/PPU clock alignments chosen at power-on; several tests (AccuracyCoin
  `BG Serial In`, `ALE + Read`, `Hybrid Addresses`, `OAM Corruption`) say results depend on it.
- Zaxnes: one fixed alignment. An option "alignment 0-3" (or random) would expose them all.
- Cost: A1 or a per-alignment table of the delays of A3.

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
  Force, SMB3 Japan, Zelda II Europe): a phase shift, to check by eye. Still open: reset length
  7 vs 8 (B2) on top of it, and whether the hardware ignores `$2000`/`$2001`/`$2005`/`$2006` writes
  for ~29 658 cycles after power-on (not modelled, Q-078).

### B2. Reset length 7 vs 8 cycles — open
- Zaxnes 7 cycles (`cpu/cpu6502.rs:173-178`); Mesen 8. Sets the DMA get/put parity. nesdev gives no
  count. With 8 + B1, Zaxnes locks instruction by instruction on MesenCE (NMI-MESEN.md).

### B3. Random power-on state — partial
- Zaxnes: `emu.power_on_ram` (zeros / FCEUX pattern / random) for work RAM (lot CARTDB).
- Not randomized: palette RAM, OAM, PPU registers, CPU/PPU alignment (A4), APU state.
- BeesNES: "start from known state or random state" (README), bus/PPU bus randomizable
  (`Src/System/LSNSystem.h:324-325`).

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
  `$2001` directly" that AccuracyCoin avoids with `$3E01` (asm 16384) — open, cost free.

### C4. Sprite evaluation — done (`ppu-exact-sprite-eval` in default)
- Hardware-exact evaluation state machine; misaligned OAM2 address, frozen increment, overflow bug.
- Open: sprite-overflow bit difference in *The Legend of Zelda* vs Mesen (to recheck since exact
  eval is in default).

### C5. `$2002` / PPU open bus decay — partial
- Zaxnes: per-bit decay stamps (`DECAY_DOTS`, `ppu/mod.rs:82`, `apply_decay`). Differs from Mesen on
  some reads (NMI-MESEN.md). To settle against the doc.

### C6. Palette read on pre-G PPUs — open (revision option)
- Hardware (nesdev "PPU registers": palette read "supported by the 2C02G, 2C02H, and PAL PPUs"): before 2C02G, `$2007` reads at `$3F00-$3FFF` do not return palette RAM (BeesNES TODO,
  `LSNPpu2C0X.h:1242-1246`). Zaxnes always returns palette (`ppu/mod.rs:1617`). Free; needs a
  "PPU revision" setting (see G1).

---

## D. PPU — OAM

### D1. OAM row corruption — done (option, off by default)
- Zaxnes `rendering_switched` (`ppu/mod.rs:677`), `corrupt_oam_rows` (`:801`); `emu.oam_corruption`.
- Off because blargg `5.Emulator` test 3 fails with it; BeesNES has the same model
  (`OamCorruptionSeed`, `LSNPpu2C0X.h:1955-1971`) **always on** and passes thanks to B1.

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

### D2b. Performance lead found on the way (lot FIDELITY, not fidelity)
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

### D3. 2C02G/H OAMADDR bug at pre-render — done (option `emu.oamaddr_bug`, on, step 19)
- Hardware: at dot 1 of the pre-render line with rendering on, if OAMADDR >= 8 the 8 bytes at
  `OAMADDR & $F8` are copied over OAM 0-7 (2C02G/H; nesdev "PPU registers" OAMADDR).
- BeesNES `PreRenderOamGlitches` (`LSNPpu2C0X.h:1990-2001`), README "Hardware bugs... OAMADDR bugs
  (writing fewer than 8 bytes on the 2C02G)".
- Zaxnes: `Ppu2C02::oamaddr_row_copy` (`ppu/mod.rs`), skipped on PAL. Free (pre-render line, once per frame).

### D4. 2C02G: writes to OAMADDR corrupt OAM — open (revision option)
- nesdev "PPU registers" OAMADDR: "On the 2C02G, writes to OAMADDR corrupt OAM"; on 2C03/2C04/2C05/
  2C07 "OAMADDR works as intended"; the pre-render copy of D3 is "required for 2C02 compatibility" on
  the Dendy. Zaxnes: none.

### D5. Palette RAM corruption when rendering is turned off mid-screen — open
- nesdev "PPU registers" PPUMASK: "Turning rendering off mid-screen can corrupt palette RAM."
  Zaxnes: none; no emulator studied models it. Mechanism to research.

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

### E2. Per-console clock — open (free)
- Each console's crystal differs (E1 table). Option "measured clock of console X" changes speed and
  pitch slightly.

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

### G1. PPU / CPU revisions — open
- BeesNES exposes none of its hardware-bug behaviours as user settings (only video, audio profile,
  region); OAM decay is a compile-time define (`LSN_INT_OAM_DECAY`, `LSNPpu2C0X.h:33`), OAM corruption
  is always on, despite the README's "buggy and fixed states".
- 2C02 letter revisions (pre-G palette read C6, G/H OAMADDR bug D3, OAM corruption differences),
  RP2A03 letter revisions (BeesNES flag `bRp2A02` in audio profiles), 2C07 PAL, 2C05 Vs.
- Zaxnes: none selectable. A "console model" setting would group C6, D3, E1, E2.

### G2. Regions PAL-M and PAL-N famiclones — open (free)
- BeesNES: per-region cycle tables (`Src/Ppu/LSNCreateCycleTablePalM.inl`, `...PalN.inl`) and
  master clocks (`LSNSystem.h:1182-1190`). Zaxnes: NTSC, PAL, Dendy, Multi (`region_timing.rs:45-47`).

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
- I2. *The Legend of Zelda* sprite-overflow bit (C4).
- I3. `$2002` open-bus bits and `$2007` reads during rendering vs Mesen (C5).
- I4. Mega Man 2 widescreen not eyeballed; PAL/Dendy fidelity work unverified.
