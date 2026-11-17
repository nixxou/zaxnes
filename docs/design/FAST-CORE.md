# FAST-CORE — a fast variant of the Zaxnes core for handhelds (design, 2026-11-17)

## What the owner asked for

* The fastest possible variant of **our** NES engine, aimed at QuickNES-class speed, so that it can
  be offered on handhelds (PortMaster, Android).
* Every Zaxnes feature keeps working: hack packs and built-in patches, widescreen (margins, wide
  window, execution zone, overclock), tile capture and the 3D conversion, Lua profiles,
  achievements, run-ahead, savestates. That is why the FCEUX engine is not the answer.
* Cycle-exact timing may go. Fidelity that costs nothing is kept.
* Commercial boards only is fine. Every other board keeps running on the exact core.
* **Savestates compatible in both directions**: a state saved by the fast core loads in the exact
  core, and the other way round.
* It is an option the player picks (`emu.fast_core`, off by default). There are no lighter Android
  defaults; the option is the same on every platform.

## Where the time goes today

Callgrind, `l2/78069f77` (dev 2026-11-16, bench-core in 3D with the 1.x profiles, 720 frames
including warm-up, Linux x86 build — instruction counts are the A53-relevant measure):

| Game | `run_frame` | PPU `run_dots` | APU (`catch_up_apu`) | CPU + bus, the rest |
|---|---|---|---|---|
| smb1 | 15.7 M Ir/frame | 8.8 M (56 %) | 3.3 M (21 %) | 3.6 M |
| smb3 | 19.8 M | 11.9 M (60 %) | 4.1 M (21 %) | 3.8 M |
| mm2 | 16.5 M | 9.2 M (56 %) | 3.7 M (22 %) | 3.6 M |
| metroid | 15.1 M | 9.3 M (62 %) | 2.1 M (14 %) | 3.7 M |
| zelda2 | 17.3 M | 9.8 M (57 %) | 3.6 M (21 %) | 3.9 M |

FCEUX is measured 4.1 to 5.6 times faster on the same harness (`docs/progress/FCEUX-6.md`
step 7). The target is **about 2 to 3 M Ir/frame** for the core, a factor 6 to 8.

What it costs per unit: the PPU spends about 130 Ir per dot (89 342 dots a frame) even with the
existing idle-dot and visible-span shortcuts; the APU about 130 Ir per CPU cycle it is caught up
over; the CPU about 380 Ir per instruction once its per-cycle `tick` is counted.

## Decision: a fast path inside `NesCore`, on the same state

Three shapes were weighed:

* **A separate engine** (like `zax-fceux`, `zax-neshawk`). Rejected: hacks, widescreen, the wide
  window, tile capture, achievements and Lua all reach `NesCore` through `EngineHandle::zaxnes()`;
  a separate engine would have to reimplement each of them, and its savestates would be a foreign
  format.
* **A second core type** (`NesCore<FastTiming>`). Rejected for now: it doubles the application's
  forwarding (`docs/design/CORE-TIERS.md`, way A) for no gain over the third shape.
* **A fast frame loop inside `NesCore`, on the same structs.** Chosen. `run_frame` takes a third
  branch next to the standard and the Fidélité+ loops. The CPU registers, RAM, the PPU's
  registers, OAM, VRAM, palette, the APU channels and the cartridge are the very same fields;
  only *how they are advanced* changes. Savestates are therefore one format for both cores, every
  accessor the application uses keeps working, and switching core is a flag flip between frames.

The fast loop is chosen once per frame, like CORE-2's board kinds: `with_board_kind!` picks the
compiled copy for the board; a board without a fast copy runs the standard loop that frame.

## The three parts

### 1. CPU: instruction steps, no per-cycle tick

A new `CpuBus` implementation `FastBus<K>` (next to `KindBus` and `FidelityBus`):

* `read`/`write` of RAM and plain PRG-ROM windows add a cycle to a counter and touch memory
  directly. No PPU lag, no APU lag, no cartridge cycle hook on these cycles.
* Accesses to `$2000-$401F`, cartridge registers and the controller ports first **catch up** the
  PPU and the APU to the current cycle (the existing `sync_ppu` / `sync_apu` contract), then do
  the access through the existing code, so every register side effect stays the exact core's.
* A deadline (the next NMI change, the next mapper IRQ, the end of the frame) is checked once per
  instruction, not once per cycle.
* The cartridge's per-cycle hook (`step_cpu_cycle`) is replaced, for the fast boards, by an
  elapsed-cycles call when it matters (MMC1's consecutive-write rule reads the cycle count; it
  does not need a hook per cycle).

The `Cpu6502` itself is unchanged at first (it is already generic over `CpuBus`); a specialised
interpreter is a later step only if the measure asks for it.

### 2. PPU: one visible line at a time

When the PPU is caught up across a visible rendering line, the fast path renders the whole line in
one go instead of 341 dots:

* Background: 33 tiles fetched with the line's `v`, through a per-bank decoded-CHR cache
  (two bit planes into 8 palette indices), invalidated on CHR bank switches and CHR-RAM writes.
* Sprites: the line's eight (or `sprite_limit`'s) sprites evaluated once, at the line, from OAM;
  sprite 0 hit's dot computed, and the flag raised when the CPU catches up past that dot.
* `v` gets the line's increments at once (coarse X 32 times, Y at dot 256, the copy at 257, the
  vertical copy on the pre-render line), so register reads see what the exact core shows at line
  boundaries.
* A register write in the middle of a visible line renders the line up to the write's dot, tile
  aligned, then the rest after it (QuickNES's rule). Raster effects at line granularity (status
  bars, split scrolling) are exact; effects inside a tile are not.
* Boards that watch the PPU bus: `BoardKind::WATCHES_BUS == false` boards need nothing.
  `A12_ONLY` (MMC3) gets one A12 rise per line at the dot the exact core would show, worked out
  from `$2000`'s pattern table bits. Boards that read the bus (MMC2/4 latches, MMC5) get the
  fetched tile numbers from the line renderer.
* Tile capture (`TileRecord`), the wide margins and the backdrop come out of the same line
  renderer, through the same `begin_capture_line` / `end_capture_line` entry points as today.

Fidélité+ rules (OAM decay, palette corruption, address-load delay, `$2006` glitches, …) are not
emulated by the fast path. Their state is carried through untouched (see savestates). The
options are greyed out while the fast core is selected.

### 3. APU: run to the next event

The channels already keep exact state. The fast path advances them by **events** (the next timer
reload of each channel, the next frame-counter step, the end of the batch) instead of cycle by
cycle, and adds each output change to the existing band-limited resampler at its cycle. Same state,
same sound, a fraction of the work. This is the Blip-style model of FCEUX and QuickNES, kept on
our state.

## Savestates both ways

* One format. The fast core saves only at its frame boundary (the start of vblank, where every
  frame ends on both cores), where the PPU's fine pipeline (shift registers, latches) is the same
  as the exact core's or rebuilt by the pre-render line.
* Fields the fast core does not simulate (Fidélité+ edges, OAM decay timers, open-bus decay) are
  saved as it found them; a state loaded from the exact core and saved again by the fast core
  without running keeps them byte for byte.
* A state saved by the exact core in the middle of a frame (a sliced frame, `FIDPLUS-B`) is run to
  its frame end by the exact core before the fast core takes over.
* After a switch the two cores may diverge (their fine timing differs). Movies and netplay must
  not switch core.

Tests:

1. **Round trip without running**: load on one core, save on the other, bytes identical, on every
   commercial ROM of `test-originals`.
2. **Agreement over time**: from the same state, the share of identical frames between the cores
   after 60, 300 and 1 200 frames (the FCEUX-1 table), per ROM, as the compatibility measure.

## Boards, by priority

| Priority | Boards | PPU bus |
|---|---|---|
| 1 | NROM, UxROM, CNROM, AxROM, GxROM, MMC1 | not watched |
| 2 | MMC3 (and 118, 119) | A12 rise per line |
| 3 | MMC2, MMC4, VRC2/4/6/7, FME-7, Namco 163, MMC5 | latches or scanline counters |

Everything else (pirate, multicart, homebrew boards, FDS, VS. System) runs the standard loop.

## Phases

Each phase is one or more commits on `feat/fast-core`, measured with callgrind (`l1.sh` level
first) and with the agreement table; nothing is merged without the owner.

0. **Option and skeleton**: `emu.fast_core`, `NesCore::set_fast_core`, the third branch of
   `run_frame` running the standard steps, `bench-core --fast`, the round-trip and agreement
   tools. No speed change.
1. **CPU**: `FastBus<K>` for priority-1 boards. Expected: most of the CPU + bus share.
2. **PPU**: line renderer, CHR cache, sprite evaluation, capture and margins. Expected: the
   largest share.
3. **APU**: event stepping.
4. **MMC3**, then priority 3.
5. **Handheld measure** on the tablet (Android) and the owner's PortMaster device; the options UI.

## Limits stated up front

* Line-level raster timing: mid-tile effects, `$2007` reads during rendering, the PPU's dot-level
  quirks are approximated. Test ROMs for those will fail on the fast core; commercial games are the
  measure.
* In 3D the profile's Lua becomes the main per-frame cost once the core is fast; the 3D gain is
  capped by it, the 2D gain is not.
* `.bk2` movies recorded on QuickNES do not sync on this core either: it is not QuickNES's timing.

## Every speed lead, and what speed buys back (2026-11-17)

Measured on the fast path after steps 2a and 3a (callgrind, smb1 in 2D, per frame): APU about
6 M Ir before 3a, PPU 6.4 M (span groups 3.1 M, tile capture about 1.2 M, dots 256-340 and the
pre-render line the rest), CPU + bus about 3.4 M.

Core leads, largest first:

1. **PPU, the rest of the line**: dots 256-340 (sprite fetches) and the pre-render line, still per
   dot; sprite evaluation once per line instead of per dot; `$2002` polling (sprite 0 and vblank
   waits) forces fine catch-ups: predict the hit and the flag instead.
2. **APU, audible events**: cached next-event cycle per channel instead of `quiet_cycles` over all
   of them at every event; later, per-channel Blip deltas (QuickNES's model, linear mix).
3. **CPU without a per-cycle tick**: cycles counted, deadlines tested per instruction, a page
   table for RAM/PRG.
4. **Idle-loop skip**: a game spinning on `$2002` or on a RAM flag set by its NMI jumps to the
   next interrupt (the widescreen patches already know some of these loops).
5. **Frames nobody sees**: catch-up and run-ahead frames render no pixels (sprite 0 and the state
   still computed); the tile capture only where a frame is converted or saved.
6. **MMC3 IRQ by line** instead of the A12 watcher on every fetch.
7. **Build**: PGO on the handheld build (LTO fat, one codegen unit and `target-cpu=cortex-a53`
   are already there).

Outside the core, on a handheld in 3D: the profile's Lua hooks, the conversion and the draw
list, then the GPU; the device's own profiler (`logs/zaxnes.log`) has to say which first.

### What the handheld pays in latency today

Measured from the code of `accuracy-done`:

| Mechanism | Where | Latency |
|---|---|---|
| Emulation on a separate core (`ZAXNES_SEPARATE_CORE`, no UI setting) | PortMaster only | +1 displayed frame of input |
| Emulation ahead (`ZAXNES_CORE_AHEAD`) | with the above, scripts that leave the core alone | turns the real-time pad off |
| `emu.pipeline` = Auto | PortMaster default, 3D, when frames are late | +1 displayed frame of picture |
| GPU queue depth 2 = no fence | PortMaster and Android | the driver's own queue (Mali: up to 3) |
| `emu.run_ahead` = 0 | PortMaster (desktop and Android: 1) | no run-ahead gain |
| `emu.realtime_input` (on) | Zaxnes only, off with run-ahead or emulation ahead | gains up to a refresh when there is idle time |
| `video.half_rate_3d` (off) | handheld option | +1 refresh in 3D when on |

Each of these trades latency for speed. A core several times faster lets the handheld drop the
separate core and the pipeline, and afford run-ahead 1: that is where its latency gain is.
