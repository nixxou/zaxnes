# Performance work since the C# port — complete record

This document lists every performance change made to the Rust version of Zaxnes since it was
ported from the C# version 0.2.0 (commit `370216c`), the attempts that were dropped, and what was
learned. It exists for one job: when the C# version is updated for 3dSen 2.0 and that update is
carried over to Rust, nothing listed here may be lost or silently undone.

Read it with:
- `HANDHELD-PERFORMANCE.md`: the chronological log with every device measurement.
- `PROGRESS.md`, section "Rust-only changes": the behaviour changes that are not about speed.
- `ROADMAP.md`, `HANDHELD-RESEARCH.md`: plans and the platform survey.

Commits are on the branch `retroachievements-dev` (worktree `wt-exec-zone`), all local unless
stated otherwise.

## Contents

1. How to use this document during the 3dSen 2.0 update
2. Rules that every change below follows
3. Verification tools
4. Target hardware and what dominates where
5. Changes by subsystem
   - 5.1 NES core (`zax-nes`)
   - 5.2 3D conversion engine (`zax-engine3d`)
   - 5.3 Lua scripting (`zax-scripting`)
   - 5.4 Renderer (`zax-render`)
   - 5.5 Application loop, threads and pacing (`zax-app`)
   - 5.6 Build and packaging
   - 5.7 Instrumentation
6. Settings and environment variables
7. Attempts that were dropped, and why
8. Conclusions
9. Open problems
10. Porting checklist: C# areas and the Rust work they touch

---

## 1. How to use this document during the 3dSen 2.0 update

The Rust crates mirror the C# projects one to one:

| C# 0.2.0 project | Rust crate |
|---|---|
| `src/Zax.Nes` (Cpu, Ppu, Apu, Cartridges, Input, State) | `crates/zax-nes` |
| `src/Zax.Engine3D` | `crates/zax-engine3d` |
| `src/Zax.Scripting` | `crates/zax-scripting` |
| `src/Zax.Render` (Gl, Assets) | `crates/zax-render` |
| `src/Zax.App` (Loop, Ui, Persistence, Input, Audio) | `crates/zax-app` |
| `src/Zax.Platform` | `crates/zax-platform` |
| `src/Zax.Settings` | `crates/zax-settings` |
| `src/Zax.Profiles` | `crates/zax-profiles` |
| (none) | `crates/zax-harness` (tests, benches), `crates/zax-achievements` |

For each C# change of the update:
1. Find the subsystem in section 10 and read the Rust items it names.
2. Port the change so that the invariants listed for those items still hold.
3. Run the checks of section 3 that the subsystem calls for. An optimisation listed as
   "identical output" must still give identical output after the port.

None of the optimisations below changed the C# specification: they are internal to Rust, or
they are settings that are off on the desktop (the C# behaviour) and on by default on handhelds.

---

## 2. Rules that every change below follows

- **Identical output needs no switch.** A change that gives the same pixels, tiles, cycles,
  audio and script behaviour goes in unconditionally, but must pass the goldens.
- **Changed output is a setting.** Anything that can change what the player sees or what a script
  observes is a setting: off (C# behaviour) on the desktop, on by default on handhelds where it
  helps. Owner decision; it also keeps the desktop build a faithful reference.
- **Measure on the target.** x86 wall time misleads for the Cortex-A53 (section 4). Use
  instruction counts (callgrind) or the device profiler.
- **No catch-up set to 0 by default**, and no fidelity/latency trade without the owner's approval.

---

## 3. Verification tools

| Tool | What it checks | When |
|---|---|---|
| `cargo test -p zax-nes --release --test ppu_golden_tests -- --ignored` | 291 ROMs x 1200 frames: pixels, tile records, audio, cycles, blargg results | any core change |
| `zax-harness golden-core <roms dir> <out.jsonl> <frames>` | same digests on any ROM folder; run at two commits and compare | any core change; the reference run used 98 commercial ROMs x 1800 frames |
| `zax-harness render --rom ... --out <dir>` with `--group-draws`, `--no-shadows`, `--size` | PNG captures; compare two builds byte for byte | any renderer or engine change |
| `zax-harness render ... --bench <n>` | GPU/CPU time of the scene passes, "GL draws" count | renderer work |
| `zax-harness bench-core <rom> [--profile] [--load-state] [--widescreen] [--run-ahead]` | CPU cost per emulated frame of each stage (core, run-ahead restore, capture, conversion, Lua, publish, draw list, picture), max and frames over 1 ms. **`--run-ahead` defaults to 0**, so a figure taken before AMD-041 and one taken after it measure the same work | CPU work; also runs under `qemu-aarch64` for relative weights |
| `ZAXNES_LUA_GC=smooth` with `bench-core` | measures Lua with the collector tuning the application uses | always, for Lua numbers (without it the spikes of the original tuning show) |
| `cargo test -p zax-scripting --release --test scripting perf -- --ignored --nocapture --test-threads=1` | Lua binding micro-benchmarks | binding work |
| Lua API dump (`goldens/lua-api.txt`) | the script-visible API is unchanged | binding work |
| In-app profiler (`ui.profiler` or `ZAXNES_PROFILE=1`) | per-stage mean/max per displayed frame, busy time per thread, Lua time per script hook, in `logs/zaxnes.log` | device runs |
| `valgrind --tool=callgrind` on a short `bench-core` | instruction counts, closer to an in-order A53 than x86 time | core and engine work |

An app save state (`.ssN`) holds the core state block in its second section; extracting it gives a
`--load-state` file for the harness, which reproduces a device scene exactly (used for the Zelda II
world map, section 9).

---

## 4. Target hardware and what dominates where

Anbernic RG40XXV: Allwinner H700, 4x Cortex-A53 at 1.512 GHz (in-order, small caches), Mali-G31
(one core), 1 GB RAM, 720x480 (owner's unit), GLES 3.0 context on a GLES 3.2 driver.

Ratios observed between the device and an x86 desktop for the same work:

| Stage | Device / x86 |
|---|---|
| NES core | about 13x |
| Lua | about 9-10x |
| Conversion | about 14x |
| Draw list build (Zelda II world map, 2258 tiles) | about 60x (unexplained, section 9) |

Where the time went, in order of discovery:
1. The core alone did not fit in 16.6 ms (17.4 ms per NES frame), and the catch-up (up to 5
   emulated frames per displayed frame) turned every late frame into a collapse to 5-10 fps.
2. Once the core was faster and on its own thread, the scene's draw calls (one per tile, about
   17 us of CPU each on Mali) dominated.
3. After batching, the main thread (conversion + Lua + draw list + collection + GL submission)
   set the pace; Lua had spikes of 100-300 ms from the garbage collector.
4. Today: heavy scenes are CPU-bound on the main thread (Zelda II world map: draw list 12.6,
   GL submission 9.9, collection 7.1, Lua 5.2 ms per displayed frame), not GPU-bound.

---

## 5. Changes by subsystem

Each entry: what it does, where it lives, whether the output is identical, the switch if any, the
measured gain, and the invariant a port must keep.

### 5.1 NES core (`zax-nes`)

C# counterpart: `src/Zax.Nes/Ppu`, `Apu`, `Cartridges`, `Cpu` (the bus).

**PPU dot dispatch by range, sprite layer built once per line** (`537c0b2`)
- The per-dot `match` became a dispatch by dot range; the sprites of a line are evaluated into a
  per-line buffer once instead of per pixel.
- Identical output. About 3 % on the device (x86 gained more).

**PPU caught up in batches** (`91123ec`)
- The PPU no longer steps with every CPU cycle. It lags and is caught up at sync points: CPU access
  to $2000-$3FFF, OAM DMA ($4014), controller reads ($4016/$4017), any cartridge write,
  vblank/NMI edges, and an IRQ horizon given by the board (MMC3 A12 counter). Idle dots are
  skipped.
- Rambo1 and MMC5 still step every cycle (no IRQ horizon written for them).
- Identical output. x86 core 1.4-2x faster; device only about 11 % (17.4 -> 15.6 ms per frame).
- Invariant: any new register, mapper write path or IRQ source must be a sync point or provide a
  horizon, or the PPU will be observed late.

**PPU fast span on visible lines** (`a0c157a`, extended by `0541e02`)
- On a visible line with rendering on, dots 2 to 255 run in one loop: PPUMASK and fine-X decisions
  hoisted, and when the board's pattern fetches have no side effect (`Mapper::chr_view`, a
  `ChrView` over CHR banks) the fetches read CHR and console VRAM directly
  (`Ppu2C02::run_visible_span`).
- Boards that watch the PPU bus or latch on fetches (MMC2/MMC4, MMC3 and compatibles, Rambo1, MMC5)
  get `run_visible_span_through_bus`: one loop, but each fetch through the full bus path, so the
  board sees the same addresses at the same PPU clock.
- Identical output. FF1 x86 core 1.18 -> 0.87 ms/frame; SMB3 1.36 -> 1.24 ms/frame; device FF1
  15.6 -> 12 ms.
- Invariant: a board returns a `chr_view` only if its fetches are pure reads. A new mapper that
  latches on CHR reads must return `None` (the default) or the through-bus path.

**APU level mixed only when an input may have changed** (`226da76`)
- The mixer output used to be recomputed every CPU cycle. It is now recomputed only after a
  channel timer moved its sequencer, shift register or DMC bit, a frame-counter step, a register
  write, a reset, a loaded state or a region change.
- Identical output, audio included. SMB3 x86 core 1.24 -> 1.13 ms/frame.
- **Kept by RU-3a** (AMD-054, non-linear mixing): the two table lookups of the new law went into
  this same lazy path, where they replace four multiply-adds, an addition chain and a clamp. The
  mixing therefore costs *less* than it did; `nestest` did not move.
- **RU-3a-fix**: RU-3a had added the expansion level to the cache *test*
  (`key == level_key && expansion_level == level_expansion`). That is **not** a cold path — it runs
  about 15 000 times per emulated frame — and the second load, compare and branch measured +13.9 M
  instructions per 300 frames (+1.1 % of the APU) on a board with no expansion audio. `M` is folded
  into the one key instead: `set_expansion_level` stores `NO_LEVEL_KEY` when the value moves, which
  happens only for the MMC5 and only on a real change. The hot test is one comparison again.
- Invariant: **the cache test stays a single comparison of `level_key`.** Anything that can change
  `A`, `M` or the five channel outputs must either be packed into the key or invalidate it
  (`level_key = NO_LEVEL_KEY`) *and* set `level_dirty`; never add a second comparison here.
- Invariant: the mixing stays in `current_level`, never per CPU cycle, and `current_level` keeps
  **one** call site — the per-cycle step. A second one (a cold savestate rebuild was enough) makes
  the inliner leave the function out of line, which costs a call and a prologue on all ~15 000 of
  those cycles; `#[inline]` does not undo it. `Apu2A03::load_state` writes its mix out for that
  reason.

**APU quiet cycles skipped while the frame loop batches** (`0cf347d`)
- During the frame loop the bus lets the APU lag like the PPU and catches it up on $4000-$401F
  accesses (except $4014 and $4016), on the DMC fetch's byte, and at the end of the loop. Runs of
  cycles on which no timer, frame-counter step or level change falls are skipped in one go: the
  timers are counted down and the resampler advanced by the same cycle count
  (`apu/mod.rs: quiet_cycles`, `skip_quiet_cycles`; channels' `quiet_clocks` /
  `skip_quiet_clocks`; resampler `level()` / `skip_cycles`; bus `apu_lag`, `catch_up_apu`).
- Outside the frame loop the APU still runs cycle by cycle.
- Identical output (98 ROMs x 1800 frames, audio digests included). Core -5.5 to -8.5 %.
- Pitfall found: `pending_sequencer_reset` must be decremented in the skip too (audio differed on
  18 ROMs until it was).
- Invariant: any APU state that changes with time must report itself in `quiet_cycles` and be
  advanced in the skip.
- **RU-3a** (AMD-054, MMC5 audio): a cartridge with expansion audio keeps `apu_quiet = 0`, so the
  APU runs cycle by cycle on it and the `M` the bus pushes is never stale. **Only the MMC5 is
  concerned.** `tick` does now step the cartridge *before* the APU catch-up instead of after (a
  move, not a branch); the 291-ROM goldens are identical across that reorder.
- **Measured, and wrong: `catch_up_apu` does not run "rarely".** RU-3a assumed it did and put the
  `expansion_audio` test and the `M` push inside it. It runs on **14 894 of the 29 780 cycles of a
  frame** (nestest, counted) — one cycle in two, because a pulse timer is clocked on alternate
  cycles — so it is a per-cycle path, and one instruction in it is ~15 000 per frame against a
  ±0.1 % gate of ~27 000. RU-3a cost `NesBus::tick` two extra callee-saved registers and +15.7 %
  of its instructions, on every board.
- **RU-3a-fix**, how it is free now. Three things, all of them structure rather than less work:
  - `NesBus::apu_batching` = `deferred_ppu && !expansion_audio`, one field maintained in `attach`,
    `defer_ppu` and `end_deferred_ppu`. `catch_up_apu` tests it **instead of** `deferred_ppu`, not
    on top of it, so the fast path costs exactly what it cost before AMD-054 (12 instructions
    either way, read off the post-LTO assembly).
  - `!apu_batching` — a board with expansion audio, or any board outside the frame loop — falls
    through to `sync_apu`, which turned out to be the same function; there is one, not two.
  - `NesBus::push_expansion_audio` is `#[inline(never)]`. That is the one thing that must stay out
    of line: `sync_apu` is inlined into `tick` twice over (through `catch_up_apu` and through
    `perform_dmc_dma`), and the push's cartridge `Option`, devirtualised `expansion_audio_output`
    and `set_expansion_level` are what raise the register pressure of the whole function.
- Invariant: **nothing MMC5-specific may be inlined into `NesBus::tick`.** Check it by reading
  `tick` in the post-LTO assembly (`cargo rustc -p zax-harness --release --bin zax-harness --
  --emit=asm`, then the prologue between the label and `.seh_endproc`): three `pushq` and a 32-byte
  frame is right, four is the regression. Out-lining `sync_apu` as well was measured and is
  *worse*: a board with expansion audio runs it every cycle, so it pays a second call for nothing
  (`mmc5test` 1.87 -> 1.91 ms/frame).
- Measured, interleaved with the RU-2 and RU-3a binaries: `nestest` 0.87 ms/frame, within the
  machine's spread of both. `mmc5test` 1.83 (RU-2) -> 1.87, `exram` 2.09 -> 2.09: the MMC5 pays one
  out-of-line call per cycle, about +2 %, which is the price of every other board paying nothing.
  No built-in profile targets an MMC5-audio game.

**Plain PRG-ROM windows read without the board's decode** (`6a53e71`)
- The cartridge keeps, for each 8 KiB window of $8000-$FFFF, where it starts in PRG-ROM when a
  read of it is a plain ROM read (`Mapper::rom_windows`, `NO_ROM_WINDOW`,
  `BankWindows::rom_windows`, overrides for NROM and MMC5). Refreshed after every CPU write to the
  cartridge, after reset, an intercept and a loaded state. The bus `read_internal` tries this
  first for addresses at or above $8000.
- PRG-RAM windows, windows that would wrap, and MMC5 reads still go through the board.
- Debug builds check every fast read against the board's own (`debug_assert`).
- Identical output. Core -6 to -8.5 %.
- Invariant: a board whose ROM reads have side effects or depend on more than the bank registers
  must report `NO_ROM_WINDOW`.

**Rollback snapshots recycle their oldest buffer** (`13f8a08`)
- `RollbackHistory::capture_with`, `NesCore::save_state_to`: no allocation per snapshot, same
  bytes.

Combined effect of the core work: SMB1 -16 %, Metroid -15 %, MM2 -12 % on top of the
milestones, device core about 17.4 -> 10-12 ms per NES frame.

### 5.2 3D conversion engine (`zax-engine3d`)

C# counterpart: `src/Zax.Engine3D` (segmentation, frames, draw list, geometry).

**Fast hasher for the per-frame maps** (`5f056be`)
- `zax_engine3d::hashing::FastMap` / `FastSet` (rustc's Fx multiply-rotate) for instances,
  patterns, tile images, meshes, tracker, and the renderer's mesh/texture caches and draw-state
  groups. SipHash was about 18 % of the conversion + Lua + draw-list instructions.
- Identical output (no map relies on its iteration order).
- Zelda II world map x86: draw list 0.24 -> 0.14 ms, conversion 0.14 -> 0.126 ms.
- Invariant: never iterate a `FastMap` where the order reaches the output; sort first.

**Draw list placement matrix computed once per instance** (`c39e2dd`)
- `Pos . R . S` (REQ-E3D-406) is computed once per instance and multiplied by each tile's
  translation, in the same order as the full product, so the floats are identical
  (`InstanceLook::placement` in `draw_list.rs`). Draw list instructions -10 %.
- Invariant: keep the multiplication order, or the floats change.

**No draw list in 2D mode** (`13f8a08`)
- The renderer shows only the core's picture in 2D, so the list is not built (the C# loop built
  it every frame).

**Only the filled grid cells are emptied and scanned** (`608dde6`)
- The widescreen margins made the conversion grid 13x larger (163x93 cells instead of 35x33), and
  every batch emptied all of it, then the segmenter scanned every cell for germs. The frame now
  remembers the cells it fills, empties only those, and hands only those to the segmenter, still
  sorted by position, so the shapes are the same.
- Identical output. MM2 0.086 -> 0.040 ms, SMB1 0.071 -> 0.019 ms per frame (desktop).
- Invariant: the germination order is part of the output (greedy segmentation, REQ-E3D-090); any
  change to germ collection must keep the position sort.

**Wide 2D picture composed three times faster** (`fbdf979`)
- The wide picture was composed from every tile record pixel by pixel, recomputing ranges per
  pixel. The centre range of each row is computed once, tiles wholly in the centre are skipped,
  and the centre of each row is copied whole.
- Identical output (21 runs, seven games, wide/advanced/vertical margin). 0.15-0.24 -> 0.04-0.10 ms.

### 5.3 Lua scripting (`zax-scripting`)

C# counterpart: `src/Zax.Scripting` (the binder, host values, runtime). In C# the binder is
MoonSharp/NLua-style reflection; in Rust it is mlua. The per-call cost of mlua (about 1000
instructions per Lua-to-host call) is the main cost, not the interpreter.

**Binding rounds 1 and 2** (`2a76217` to `7446656`, `4795df3`)
- Frame handles keyed by integers (slot of group and member) and retired into per-epoch weak tables
  keyed by integers instead of 16-byte strings (retirement was 18 % of a frame).
- A frame handle keeps its identity after its epoch (REQ-LUA-043), as the C# binder does; costs
  about 45 us per frame but is required.
- Budget clock started at the first look of the count hook, not at each hook call.
- Member lookup by bytes (no UTF-8 check).
- Frame epochs read from the object cache (no borrow of the runtime data per access).
- The `UpdateS` function and `self` fetched once per script.
- Published hook globals and cached script handles; host writes queued behind the next request;
  the runtime lent once for all instances of a pattern.
- Result (synthetic frame: `Update` + `LateUpdate` + 60 `UpdateS`): 397 -> 203 us on x86.
- Invariant: the Lua API dump must stay byte-identical.

**Smooth incremental collector** (`e205ddd`, setting `emu.lua_smooth_gc`)
- Pause 120 %, step multiplier 400 %, 256-byte steps, instead of the original 400 / 400 / 8 KiB:
  collection is spread over the frames instead of stalling one.
- On by default on desktop and handheld. Scripts see no difference except
  `collectgarbage("count")` (no profile of the corpus calls it).
- Worst Lua frame on Zelda II x86: temple 136 -> 2.5 ms, world 126 -> 1.3 ms; mean equal or lower.
- History: first tried as the generational collector (`9f9136e`); its major collections stalled
  120 ms once the heap reached 49 MB, so it was replaced.
- Note for measurements: the harness starts with the original tuning; use `ZAXNES_LUA_GC=smooth`
  or `bench-core` shows spikes the application does not have.

**Optimised Lua** (setting `emu.lua_optimized`, "Optimized Lua (unsafe)", desktop off, handheld
on; `f0be2a5`, `bbe5584`, `7eea4fa`)
- Owner decision to allow `unsafe` in `zax-scripting`, in documented modules only (the crate is
  `deny(unsafe_code)` elsewhere).
- Step 0 (`binding/lend.rs`): the runtime data, engine state and instance being built are lent to
  Lua calls as pointers in a per-state slot instead of mlua scoped userdata; a busy bit per object
  keeps one mutable borrow at a time.
- Step A (`binding/raw.rs`): host values are bare userdata blocks (magic + payload) sharing one
  metatable whose `__index` / `__newindex` are C functions; getters, setters and method lookups
  skip mlua's userdata entry; other cases fall back to the safe binding.
- Step B: a returned host value is pushed by the metamethod itself, the shared metatable is an
  upvalue and a registry integer key, methods are C closures reading their receiver from the
  block, the state's `Lua` is an upvalue; coroutines go to the mlua function.
- Off runs no `unsafe` code and forces `emu.pipeline` off.
- x86 Lua per frame: Zelda 1 0.65 -> 0.36 ms, Zelda II 0.37 -> 0.20 ms; worst Zelda 1 frame
  36 -> 3 ms (less garbage per access). All tests pass in both modes (`ZAXNES_FAST_LUA=1` runs the
  test suite with it on).
- Invariant: any new host type or member must be reachable through both paths and give the same
  API dump.

**Experimental Lua** (setting `emu.lua_experimental`, off everywhere; `926fb34`)
- On top of the optimised Lua: `UpdateS` hooks that only use fields of `shape` (decided from the
  source when the profile loads) receive one light `shape` per list, pointed at the instance of
  each call, and no `shape2D` (`runtime/light_shapes.rs`).
- Off by default because the source analysis is a heuristic.

### 5.4 Renderer (`zax-render`)

C# counterpart: `src/Zax.Render/Gl` (scene passes, shaders, texture/mesh caches).

**Uniform value cache and binding cache** (`83aa286`)
- `ShaderProgram` keeps the bits last uploaded per uniform and skips identical uploads (FNV hash
  for the name lookup: SipHash was about 2 % of all instructions). Textures and vertex arrays
  are rebound only when they change.
- Identical output. RTX 3060 render 0.44 -> 0.33 ms; Zelda II temple scene on device 29 -> 17 ms.

**Instanced draws through a uniform block** (`cf00717`)
- The scene shader reads each draw's placements from a uniform block (`ZaxInstances`: model
  matrix, normal matrix columns, deformation tile position; 128 instances of 128 bytes per draw;
  one buffer upload per frame). Consecutive items with the same draw state (`DrawStateKey`:
  everything but the placement and the deformation tile position) go out as one
  `glDrawElementsInstanced`.
- Identical output. `RenderStatistics::draw_calls` still counts tile draws (goldens); `batches`
  counts GL calls.
- On its own only -5 to -8 % calls (the list interleaves meshes); the gain comes with grouping.

**Grouped opaque draws** (handheld only; `cf00717`, `5f40b61`, `0998e15`)
- Reorders the opaque bucket by draw state (states in first-appearance order, list order inside a
  state), and the tiles inside each transparent object (depth-only pass then equal-depth colour
  pass, so only exact ties matter; the HUD is never reordered).
- Output changes only on exact depth ties between different states: Zelda II world map 4 pixels
  of 86 400; 22 of 36 captures differ by at most 0.002 %. Hence a platform switch: on with
  `--handheld`, off on desktop, `ZAXNES_GROUP_DRAWS=0/1`, harness `--group-draws`.
- Temple 934 -> 138 GL draws, world map 1559 -> 72, Blaster Master 330 -> 206 (transparent).
- `0998e15`: the batching gives each item a draw-state label once and orders indices, instead of
  moving the draw items.
- Invariant: a new per-draw shader input belongs in `DrawStateKey` (or in the instance block),
  or two different states will be merged.

**Opaque draws without `discard`** (`04fc7d9`)
- A tile wholly inside the clip rectangle (its 8 placed bounding-box corners, with a 0.01 margin;
  `inside_clip` in `frame_plan.rs`) no longer needs the clip test. The clip flag is no longer
  part of `DrawStateKey`: a state is drawn clipped when any of its tiles needs it, so the flag
  neither splits runs nor changes the grouped order. States that never clip and read no colour
  texture use the `ZAX_SOLID` variant of the scene shader (`shaders::SCENE_SOLID`), which has no
  `discard` and lets a tiled GPU reject hidden fragments early.
- Identical output (56 captures, both draw modes). 8K bench RTX 3050: SMB3 2.26 -> 2.09 ms, MM2
  unchanged.
- History: a first version kept `clip` in the key and lost 7 % on MM2 (40 -> 101 GL draws) and
  changed one pixel through the grouped order; the per-state unification fixed both.
- Invariant: any new `discard` in `scene.frag` must be inside `#ifndef ZAX_SOLID`, and the
  condition for using the solid program (`!clip && color_mode != 1`) must exclude it.

**Palette linearised once per frame** (`e155539`)
- The 64 palette entries go through `zaxToLinear3` in a 64x1 pass into an `RGBA32F` texture
  (`gl/linear_palette.rs`, `shaders/linear_palette.frag`); the scene shader reads indexed albedo
  from it instead of three `pow` per fragment. Same GPU, same function, so identical values.
- CPU fallback (`color::rgb_to_linear`) when the float target is incomplete; last-bit differences
  possible there.
- Identical output (56 captures). No gain on the RTX 3050 (MM2 +2 % at 8K: `pow` is cheap there
  and the 128-bit fetch costs a little); aimed at the Mali's slow transcendentals, not measured on
  the device. `ZAXNES_SRGB_PALETTE=1` restores the per-fragment conversion (shader variants
  `SCENE_SRGB_PALETTE`, `SCENE_SOLID_SRGB_PALETTE`) for an A/B on the device.

**Shader program binary cache** (`8efd710`)
- `gl/program_cache.rs`: with `glGetProgramBinary`, each linked program is saved in
  `<userdata>/shader-cache/<hash>.bin` (hash of GL renderer and version strings, program name and
  both sources) and loaded at the next start. Damaged or refused binaries are recompiled.
- Start-up time on the handheld.

**Automatic resolution** (setting `video.dynamic_resolution`, off everywhere by default; `86b3bcd`)
- GL timer queries (`gl/gpu_timer.rs`) read a few frames late; the scale moves among native, 5/6,
  2/3, 1/2. Down after 3 measures over 85 % of the budget, up by probing after about 5 s without a
  late frame.
- Mali-G31 pitfall: the timer answered 4295 ms (the largest 32-bit result); results of 0 or of a
  second or more are ignored, and the timer is given up after 300 results without a measure.
- The existing `video.res_scale` (2/3, 1/2, 1/3, 1/4 through `video.render_method` = Render Tex)
  is the C# feature; only the automatic mode is Rust-only.

**wgpu backend batching** (`63c95cd`, wgpu feature only)
- Texture arrays and multi-draw indirect where the device allows; 80-byte draw blocks and 64-byte
  placements. Not used by the handheld (GL path). `ZAXNES_WGPU_BATCHING=0` disables it.

**The curvature is a shader variant, not a uniform** (RU-4a, AMD-043 REQ-SCN-067)
- `gl/shaders.rs` composes a variant in a `const fn`: `Stage` holds its chunks in **fixed slots**,
  empty ones skipped, and `scene(solid, srgb_palette, bend)` / `shadow(bend)` replace the four
  `SCENE*` constants and `SHADOW`. Eight scene variants and two shadow ones, ten distinct names
  so the on-disk program cache files them apart. `assemble` skips an empty slot without even a
  blank line, so leaving a define out produces the source the stage had before slots existed.
- **At curvature 0 — all four settings of REQ-SCN-066 default to 0 — nothing of the curvature is
  compiled, swapped or uploaded.** `select_bend_variant` is one `is_identity()` test per frame;
  the curvature pair is built the first time a setting is not 0, which on a handheld is seconds
  of shader compile a player who never bends anything never pays.
- Invariant, and the test that holds it:
  `zax-render/tests/bend_tests.rs` `req_scn_067_the_flat_variant_carries_nothing_of_the_curvature`
  preprocesses the assembled source the way the GLSL compiler would — `#ifdef ZAX_BEND` branches
  dropped, `#else` kept, line comments removed — and asserts that none of the nine identifiers
  the curvature brings in reaches the compiler, over all eight flat scene variants (desktop and
  GLES) and both flat shadow ones. **A uniform tested per vertex is the regression**; a per-axis
  `uBendOn` *inside* the variant is not, it is uniform over a whole draw and saves an idle axis a
  `sin` and a `cos`.
- Measured on the `framing` scene (`zax-harness render --frames synthetic:framing`): default 6
  distinct colours, **1798 draw calls, 7704 triangles**; `--bend 90,0,0` 416 colours and the
  **same 1798 draw calls and 7704 triangles**. The curvature adds no draw call and no triangle.
- The subdivisions exist only while something bends (`interop::subdivision_for`, the single home
  of the rule, read by the renderer and by `scene_export`): the backdrop box stays its six quads
  and the 2D rectangle its single quad at curvature 0. In 2D mode the rectangle goes 2 triangles
  → 2048 → 2 again when "Bend 2D Mode" is turned off, in one draw call throughout.
- Q-104: the curvature landed in the **OpenGL backend only**. The `wgpu` backend keeps drawing
  flat, as it did before the amendment, and a test asserts that known gap so a half-done parity
  fails loudly.

**The opacity mask of AMD-050 REQ-CORE-049 costs the PPU nothing** (RU-4a) — a core change, listed
here because it is the one measurement of the lot that had to come out at zero.
- The bit rides in **bit 6 of each byte of the picture**, beside the colour index of REQ-CORE-048
  in bits 0-5 (`ppu::OPAQUE_PIXEL`, `PIXEL_COLOR`). That is the internal layout the requirement's
  note allows, and the only free one: `entry`, the palette index the two pixel functions resolve,
  is non-zero **exactly** where the pixel carries an opaque pixel, so a derived
  `palette_marked[i] = palette[i] | 0x40` (`i ≥ 1`) and a colour mask widened to `0x7F` / `0x70`
  carry the bit with the same indexed load and the same `AND`.
- Measured on the post-LTO assembly, `Ppu2C02::run_dots` (both pixel functions are inlined into
  it): **2231 instructions before, 2231 after**, and with the label numbering, the anonymous
  symbol hashes and the `(%rbx)` displacements normalised the entire diff is five immediates
  (`$48 → $112` three times, `$63 → $127` twice) plus the palette load reading `palette_marked`
  at +32 instead of `palette`. No instruction added, removed or reordered.
- Invariant: **`palette_marked` is maintained where `palette` is written and nowhere else** —
  the one register write, the power-up fill, the state restore. A rebuild in a per-frame path, or
  a second store per pixel, is the regression. It is derived, so it is not in the state blob.
- On the renderer's side nothing new runs in a shader: `gl/picture.rs` already resolved the
  picture to RGBA once per frame **in 2D mode only** and already wrote alpha 0 for a pixel the
  overscan leaves out, and `scene.frag` already discarded on alpha 0 in the opaque pass.
- The 291 PPU golden hashes are unchanged: the two hashers mask with `PIXEL_COLOR` through
  `pixel_frame_indices_into`, and the accessor was **renamed** (`pixel_frame` →
  `pixel_frame_with_mask`) so the compiler visited all eleven call sites.

**The curved Zapper aim runs on a change, not per frame** (RU-4a, AMD-043 REQ-INP-124)
- REQ-INP-061 refreshes the Zapper once per displayed frame and must keep doing so, so the march
  of the inverse is guarded by a memo of its inputs (`AimInputs`: the pointer, the target size,
  the aim plane, the scene scale, the curvature, the camera view). A frame on which none of them
  moved costs **one comparison**.
- At curvature 0 `ZapperState::update` is the two lines it was before the amendment — one
  ray-plane intersection — and the memo is cleared rather than consulted.

### 5.5 Application loop, threads and pacing (`zax-app`)

C# counterpart: `src/Zax.App/Loop` (frame clock, emulation host, game session).

**Emulation on a separate core** (handheld only; `6b5ad4f`; `ZAXNES_SEPARATE_CORE=0/1`)
- `core_worker.rs` runs each emulated frame on a "zaxnes-emulation" thread. The core is moved there
  (never shared; the session holds a ROM-less stand-in) and taken back at the start of the next
  tick, before anything reads or changes it.
- Order per tick: finish frame N (tile capture, conversion, frame hooks), hotkeys and commands,
  prepare N+1 (input hook, input word), battery / audio / settings / publication, then launch N+1
  while the main thread draws N.
- Scripts and core see each other exactly as on one thread (`tests/separate_core_tests.rs`). The
  frame is shown one displayed frame later (input latency + 1 frame), which is why it is a
  platform choice.

**Emulation ahead** (with the separate core, automatic; `e312c87`, `22ec35a`;
`ZAXNES_CORE_AHEAD=0`)
- When no script of the profile names a member that reads or drives the core (`CORE_MEMBERS` in
  `game_session.rs`: `InputProcess`, `WriteMem`, `GetInput`, `GetInputType`, `SetInputType`,
  `ZapperEnable`), frame N+1 runs while frame N is converted and its hooks run.
- `ReadMem` is allowed: the console RAM is copied when the frame comes back (`MemorySnapshot`) and
  scripts read that copy. A read of cartridge space turns ahead off for the session (logged).
- 95 of the 101 3dSen profiles qualify. Same frames as serial (tests).
- Invariant: **every new Lua API member that reads or writes the core must be added to
  `CORE_MEMBERS`**, or the scripts will see the next frame's state. This matters for 3dSen 2.0's
  new API.
- The flag `pipeline.ahead_allowed` it sets also gates `pause_3d_in_2d`, `catch_up_last_3d` and
  `half_rate_3d` (below).
- **Not to be confused with run-ahead** (next entry). "Emulation ahead" is pipelining and changes
  no latency; AMD-041's run-ahead re-simulates frames to cut input latency. They compose.

**Run-ahead** (setting `emu.run_ahead`: 0 to 3, **desktop 1**, handheld 0; AMD-041 REQ-LOOP-064
to 068; RU-3b)
- The displayed frame is N frames in the future of the **real** frame, computed with the current
  inputs; the real state then comes back. The player sees their pad N frames earlier. A real frame
  therefore emulates **N + 1** frames: measured 2.02x at N = 1, 3.04x at N = 2 and 4.09x at N = 3
  of the core's time on `nestest`.
- The sequence of REQ-LOOP-065 lives in **`NesCore::run_frame_ahead`**, not in the session,
  because the "emulation on a separate core" thread *moves* the core for the duration of one frame
  (`core_worker.rs`): the sequence is exactly the unit that thread already ran. `CoreWorker::start`
  takes N as a third argument.
- **N = 0 costs nothing, and that is a rule, not an aspiration.** `run_frame_ahead` is
  `if ahead == 0 { return self.run_frame(word) }` with the sequence `#[inline(never)]` behind it.
  Verified on the post-LTO assembly against RU-3a-fix (`ce78abd`): `NesBus::tick` **119
  instructions / 3 `pushq` / 32-byte frame, identical**, and `NesCore::run_frame` **2 096,
  identical**. The only change anywhere hot is +18 static instructions in
  `Apu2A03::run_cpu_cycles`, three inlined copies of the mute test, each **behind the ring's
  mutex** — so once per output sample (~735 per frame), not per CPU cycle: about 0.008 % of a
  frame. A lot that touches this must keep those three figures.
- The in-memory state of REQ-CORE-121 (`zax_nes::state::CoreState`) is a **reused** buffer, 19.5
  KiB, written by the savestate's own per-component code. `Ppu2C02::save_state` is split into
  `save_machine_state` + `save_frame_records` and the in-memory state takes the machine half only:
  the frame's tile records are cleared by `begin_rendered_frame` at every frame's pre-render line,
  so the emulation never reads them back. Save + restore measured at **0.9 to 1.0 µs per real
  frame** (0.11 % of one emulated frame); keeping the records out is most of why.
- That split is also what makes REQ-LOOP-067 free: after the restore the core still holds the
  **displayed** frame's records, masks and pixels, so `frame_batch()`, the 2D picture and the
  Zapper answer for the displayed frame with nothing copied. The restore (step 5) is **delayed**
  past the conversion and the frame hooks, which REQ-LOOP-067 allows in so many words.
- **Invariant**: every entry point of the session that runs a frame leaves the core in the
  **real** state (`GameSession::put_the_real_frame_back`, four sites). That is what makes
  REQ-LOOP-066 true by construction for the rollback snapshots, the frame steps, the savestates,
  the input recording and the achievements. A lot that adds a frame-running path must add the
  restore to it.
- Frame stepping (J / K) forces N = 0, so it shows the frame it steps to.
- The pixels of a hidden frame **are** rendered. Skipping them was measured at 4.8 % of an
  emulated frame on `nestest`, and the only zero-cost way to get it duplicates the five hottest
  PPU loops through a `const` generic — the wrong trade against the ±0.1 % gate at N = 0 and
  against a Cortex-A53's instruction cache, on handhelds that default to N = 0 anyway.
  REQ-LOOP-068 only permits the omission; the Zapper is unaffected either way, its light being
  composed from the tile records (`sees_light_at`) and not from the pixel buffer.
- The harness runs at **N = 0** unless `--run-ahead` says otherwise (`render`, `bench-core`), and
  `golden-core` has no option at all: the digests and captures are only comparable at N equal
  (REQ-ARCH-012 amended). `bench-core` reports N in its header and the restore as its own stage.
- Q-103, answered by R0 after this lot and implemented in RU-3b-fix: a frame hook's
  `Script:WriteMem` writes into the displayed state and is lost, and the write is not replayed, so
  a profile whose scripts name a member that **changes** the core (`WriteMem`, `SetInputType`; the
  read-only ones and `InputProcess`, which runs before the real frame, are not concerned) runs at
  an **effective** N of 0, the user's setting kept for the next game. One text scan of the sources
  at profile load (`scripts_change_the_core`, beside `scripts_leave_the_core_alone`) and one
  `bool` test in `GameSession::apply_run_ahead_setting`, where the loop already read the setting:
  nothing in the core, the conversion, the draw list or the frame hooks. `set_run_ahead` stays the
  raw setter, so the frame steps and the harness are untouched.

**Pipelined frames** (setting `emu.pipeline`: Off / Auto / Always; desktop Off, handheld Auto;
`ZAXNES_PIPELINE`; `8bc7374`)
- With emulation ahead, the conversion of a frame (Lua hooks, conversion, draw list) runs on a third
  thread while the main thread draws the previous frame's list with a copy of the scene. The UI is
  built before the hand-off.
- Auto (`pipeline.rs`, `PipelineGovernor`): on after 3 frames over 85 % of the budget, off after 2 s
  where the direct path fits in 60 %.
- The Lua runtime changes thread through a documented `unsafe impl Send`
  (`zax-scripting/src/runtime/handoff.rs`); mlua's `send` feature was measured at +16 % Lua time and
  not used. Requires `emu.lua_optimized`.

**Catch-up limit** (setting `emu.catch_up`: None, 1-5; **default 2 on the desktop, 5 on a
handheld** since RU-3a; `9fc03f5`, `893986e`; `--catch-up`, `ZAXNES_CATCH_UP`)
- REQ-LOOP-012 ran up to 5 emulated frames per displayed frame; **AMD-052 REQ-LOOP-074 takes the
  spec's limit to 2**. History on the handheld: None (game slows instead of collapsing), then 2
  (worse: the second frame waits for the emulation thread in series), then back to 5 once the core
  was on its own thread (heavy scenes then keep game speed by showing fewer frames). The owner
  does not want 0 as a default.
- **RU-3a** therefore splits the two figures: `MAX_BACKLOG` = 2 is the spec's limit and the pacer's
  default, `MAX_CATCH_UP` = 5 is the largest the setting offers, and the handheld default stays 5
  (`ui/handheld/defaults.rs`). A lot that touches the pacer must keep them apart: lowering the
  handheld to 2 would undo the measurement above.

**Clock reset on a speed change** (AMD-052 REQ-LOOP-075; RU-3a)
- A change of `emu.game_speed` resets the cadence clock; arming, expiring and re-arming a
  fast-forward burst (about six state changes a second) must **not**. `FramePacer::clock_resets`
  counts the resets, which is what the criterion measures.

**GPU frame queue** (setting `video.max_queued_frames`: 1 or 2, default 2; AMD-052 REQ-LOOP-077;
RU-3a)
- `zax_render::sync::GpuFrameQueue`: a `glFenceSync` after the last draw of a frame, waited on
  before the first draw of the next, so that at depth 1 the GPU holds no finished frame.
  `Running::wait_for_the_gpu_queue` / `close_the_gpu_queue` place it after the tick's emulated
  frames and after the last draw, so it cannot move the emulated cadence.
- **Handheld (D), owner decision**: at the default depth of 2 a handheld issues **no fence call at
  all** and keeps its current behaviour — that depth is the driver's own queue, and the Mali keeps
  three frames of its own. Depth 1 works everywhere. A lot that touches this must not start
  fencing handheld frames at depth 2.

**3D paused in 2D** (setting `emu.pause_3d_in_2d`, desktop off, handheld on; `246bab8`)
- While the 2D picture is shown, a profile whose scripts leave the core alone is neither converted
  nor run through its frame hooks; its input hook still runs. The last skipped frame
  (`skipped_batch`) is converted when the 3D comes back, before it is drawn.
- Changes script timing (hooks do not run in 2D), hence a setting. Measured: SMB3 widescreen in 2D
  main thread 19.8 ms (Lua 6.3, conversion 4.0, collect 2.65) went to near nothing.

**Catch-up converts the shown frame only** (setting `emu.catch_up_last_3d`, desktop off, handheld
on; `d9e5680`)
- When a displayed frame runs several emulated frames, those finished before the last round are
  neither converted nor run through the frame hooks (scripts leaving the core alone only); the input
  hook runs every frame.
- Breaks the catch-up spiral seen in a Zelda II temple: 5 emulated frames per displayed frame, each
  about 11 ms core + 8 ms conversion + 9 ms Lua (`pivot.UpdateS`, about 400 calls per frame), 8 fps.

**3D at 30 fps** (setting `video.half_rate_3d`, handheld options only, off by default; `31c7485`,
`3152ece`)
- While a running game shows its 3D, the window swaps every other refresh and a catch-up converts
  only its shown frame; the 2D picture and the paused menus keep every refresh
  (`EmulationHost::half_rate_3d`, `VideoPacing::half_rate_3d`). Hidden from the desktop options
  window (`ui/options.rs` `EXCLUDED`).
- Owner is not convinced of its value: kept off by default.

**Curvature of space** (settings `scene.bend_lr`, `scene.bend_tb`, `scene.arc_h`,
`scene.arc_center`, `scene.bend_2d_mode`; AMD-043 REQ-SCN-066 and AMD-046 REQ-SCN-074; RU-4a)
- The four angles are **0 on both platforms**, which is their own default, and `bend_2d_mode` is
  on both. Nothing is overridden for a handheld, because at 0 there is nothing to turn off: no
  program is compiled, no surface is subdivided, and the per-frame cost is one `is_identity()`
  test (see 5.4). That is also why the five are offered among the handheld's advanced options.
- The five are read **when they change** — `scene_script_bridge::bend_of` in `apply_setting` and
  at a profile load — never per frame, per tile or per pixel: `SpaceBend::new` evaluates three
  sines, three cosines and three divisions, which belongs nowhere near a frame path.

**No rewind on a handheld**: `EmulationHost::rollback_enabled` false with `--handheld` (the
snapshot every 60 frames caused a spike).

**Handheld options page "Optimizations"** (`3152ece`): frame cap, 3D at 30 fps, catch-up, catch-up
last 3D, pause 3D in 2D, parallel scripts, optimised and experimental Lua.

### 5.6 Build and packaging

- Release profile: `lto = "fat"`, `codegen-units = 1`, `strip = true` (`9b89c15`).
- Windows: `+crt-static` (standalone exe).
- aarch64: `-C target-cpu=cortex-a53` (`13f8a08`; `ZAXNES_TARGET_CPU` reported by the build
  script); golden-core under qemu-aarch64 is identical to the x86 goldens.
- PortMaster launcher sets the `performance` CPU governor for the run and restores it.

### 5.7 Instrumentation

- Frame profiler (`frame_profiler.rs`, setting `ui.profiler`, `ZAXNES_PROFILE=1`): stages input,
  core, capture, conversion, Lua, rollback, host, list, collect, scene, wait, UI, audio, swap,
  sleep; busy time per thread; Lua time per script hook; start-up line with the render and pacing
  settings. Off, each stage clock is one branch.
- `bench-core` (`zax-harness`): same stages headless, max and frames over 1 ms per stage.
- File log `logs/zaxnes.log`, launcher `log.txt` (firmware, GPU, governor).

---

## 6. Settings and environment variables

Rust-only performance settings (none are exposed to Lua; the Lua API dump is unchanged) — with
one exception, `emu.run_ahead`, which is AMD-041's own setting and **is** exposed to Lua
(`Setting.RunAheadFrames`). Handheld values are environment defaults: they apply only while the
user has no saved value.

| Setting | Desktop default | Handheld default | Output identical? |
|---|---|---|---|
| `emu.run_ahead` (AMD-041) | **1** | **0** | no, and by design: the displayed frame is N frames on. Costs N extra emulated frames per real frame |
| `emu.catch_up` | 5 | 5 | n/a (C# behaviour is 5) |
| `emu.lua_smooth_gc` | on | on | yes (except `collectgarbage("count")`) |
| `emu.lua_optimized` | off | on | yes |
| `emu.lua_experimental` | off | off | intended yes (heuristic) |
| `emu.pipeline` | Off | Auto | yes, one frame of latency |
| `emu.pause_3d_in_2d` | off | on | no: hooks do not run in 2D |
| `emu.catch_up_last_3d` | off | on | no: hooks skip caught-up frames |
| `video.half_rate_3d` | off (hidden) | off | no: 30 fps 3D |
| `video.dynamic_resolution` | off | off | no |
| `ui.profiler` | off | off | n/a |

Platform choices without a setting:

| Behaviour | Desktop | Handheld | Override |
|---|---|---|---|
| Grouped draws | off | on | `ZAXNES_GROUP_DRAWS=0/1` |
| Separate core thread | off | on | `ZAXNES_SEPARATE_CORE=0/1` |
| Emulation ahead | with separate core | with separate core | `ZAXNES_CORE_AHEAD=0` |
| Rewind snapshots | on | off | none |
| Linearised palette | on | on | `ZAXNES_SRGB_PALETTE=1` |

Other variables: `ZAXNES_LUA_GC` (harness collector), `ZAXNES_FAST_LUA`, `ZAXNES_EXPERIMENTAL_LUA`
(tests), `ZAXNES_WGPU_BATCHING`, `ZAXNES_GLES`, `ZAXNES_HANDHELD`, `ZAXNES_PROFILE`.

---

## 7. Attempts that were dropped, and why

Core:
- Removing the PRG/CHR `% len` in the board reads: no x86 gain.
- Restricting the cartridge cycle-hook dispatch: no gain.
- Drawing visible spans one tile (8 dots) at a time: identical output but 5-9 % slower.

Lua:
- Lazy `shape2D` (made only when a script reads it, `763a0ff`): reverted. It needed a protected
  metatable on the globals, which scripts can see; C# puts none on `_G`.
- Typed answers instead of boxed ones for host reads: about 0 ns, the cost is the Lua round trip.
- A cache of setting reads: the model read is already cheap.
- Generational collector: majors of the whole heap, 120 ms stalls.
- mlua's `send` feature for the pipelined runtime: +16 % Lua time.

Renderer:
- Clip flag kept in the draw-state key for the solid shader: more runs (MM2 -7 %) and one changed
  pixel through the grouped order (replaced by the per-state unification).
- Front-to-back order of the opaque states: no gain on the RTX 3050; costs CPU for a sort.

Engine and loop:
- Disk cache of meshes: draw-list spikes are 1 frame in 4000 (SMB3, x86); not worth it.
- Catch-up 2 on the handheld: worse than both None and 5 once the core had its own thread.
- Reusing the draw list when nothing changed: not done, too many mutation sources (Lua, settings,
  meshes finishing in the background, state loads); a missed one would show a stale picture.

Measurement pitfall:
- Lua spikes of 35 ms seen in `bench-core` on the SMB3 world map came from the harness's original
  collector tuning; with `ZAXNES_LUA_GC=smooth` (the application's default) the worst is 0.67 ms.

---

## 8. Conclusions

1. **The A53 is not an x86.** An optimisation worth 1.4-2x on x86 gave 11 % on the device (batched
   PPU). Count instructions and cache misses; prefer work that removes memory traffic and branches.
   Memory-heavy stages (draw list, collection) are 30-60x slower than on x86, compute-heavy ones
   about 10-14x.
2. **The catch-up spiral is the first thing to break.** Once a frame overruns, running up to 5
   emulated frames per displayed frame makes everything worse. The fixes that mattered moved work
   off the critical path (core thread, emulation ahead, pipelining) or skipped work nobody sees
   (pause in 2D, convert only the shown frame).
3. **Draw calls were the GPU-side CPU cost.** About 17 us of CPU per draw on the Mali driver;
   instancing alone barely helped because the list interleaves meshes; grouping by state is what
   brought 1559 draws down to 72.
4. **Lua's cost is per call, not per instruction.** mlua's callback entry (about 1000
   instructions) dominated; raw userdata with C metamethods halved Lua time. GC tuning removed the
   worst stalls entirely.
5. **Identical output is achievable for most work.** Every core, engine and most renderer
   changes are bit-identical, verified by goldens and captures. Only grouping, threading latency
   and skipped hooks change output, and they are platform settings.
6. **Script-visible timing is the boundary.** Every trick that skips or reorders frame hooks is
   gated on `scripts_leave_the_core_alone` (the `CORE_MEMBERS` scan).
7. **Today the device is CPU-bound on the main thread** in heavy 3D scenes, not GPU-bound;
   lowering the render scale does not help there.

---

## 9. Open problems

- **Zelda II world map** (owner savestate, logs 21-22): 18 fps, main thread about 47 ms per
  displayed frame: draw list 12.6, GL submission 9.9, collection 7.1, Lua 5.2, conversion 2.7 ms.
  The same draw list costs 0.2 ms on x86 (2258 tiles; `DrawTile` is 432 bytes, of which 164 are an
  inline `Option<DeformParameters>`, so about 1 MB written per list). Unexplained ratio of 60;
  memory traffic is the main suspect. Leads: sub-stage timers inside list/collect/scene in the
  profiler; a smaller `DrawTile` (deformation out of line); reuse of the list when nothing changed
  (risky, section 7).
- **Paused game in 3D**: the list, collection and passes are still rebuilt every displayed frame
  (about 30 ms on the Zelda II map), although nothing changes.
- **Linearised palette** and **solid shader**: gains not measured on the Mali (no device access).
- **Segmentation reuse on scroll** (idea "piste 5"): order-dependent greedy segmentation, hard to
  keep identical; only as a setting with a debug comparison.

---

## 10. Porting checklist: C# areas and the Rust work they touch

| C# area changed by the update | Rust items to re-check | Checks to run |
|---|---|---|
| `Zax.Nes/Ppu` (rendering, registers, timing) | 5.1 batched PPU sync points, fast span (`run_visible_span*`), per-line sprite buffer, sprite limit; the `save_machine_state` / `save_frame_records` split of the savestate (5.5 run-ahead) — a new field goes in one of the two, never in both | PPU goldens, golden-core |
| `Zax.Nes/Apu` | 5.1 lazy level mixing (the non-linear tables of AMD-054 live there; the cache test stays **one** comparison of `level_key` and `current_level` keeps **one** call site), quiet-cycle skipping (`quiet_cycles` must account for any new timed state; a board with expansion audio keeps it off) | golden-core with audio digests |
| `Zax.Nes/Cartridges` (new or changed mapper) | 5.1 IRQ horizon or per-cycle stepping; `chr_view` (only for pure fetches); `rom_windows` (`NO_ROM_WINDOW` when reads have side effects); debug assert in debug build | golden-core, a debug run of the mapper's games |
| `Zax.Nes` bus / CPU | 5.1 sync points ($2000-$3FFF, $4014, $4016/7, cartridge writes), APU catch-up on $4000-$401F, and **`NesBus::tick`'s prologue in the post-LTO assembly** (119 instructions, 3 `pushq`, 32-byte frame: nothing board-specific and nothing of run-ahead may be inlined into it) | goldens |
| `Zax.Engine3D` segmentation / frames | 5.2 filled-cell lists and germ order; `FastMap` iteration order never reaching output | render captures (identical) |
| `Zax.Engine3D` draw list / `DrawTile` fields | 5.2 placement matrix order; 5.4 `DrawStateKey` must include any new per-draw input; renderer `DrawItem` | captures in both draw modes |
| `Zax.Scripting` new API members | 5.3 both binding paths (safe and `raw.rs`), API dump; 5.5 **`CORE_MEMBERS`** if the member touches the core; `light_shapes` analysis if it touches `shape` | scripting tests with and without `ZAXNES_FAST_LUA=1`, API dump, `separate_core_tests` |
| `Zax.Scripting` hook order or new hooks | 5.5 emulation ahead, pipelining, `pause_3d_in_2d`, `catch_up_last_3d` assumptions on when hooks run | `separate_core_tests`, `pause_3d_tests` |
| `Zax.Render` shaders | 5.4 `ZAX_SOLID` (no `discard` outside the guard), **`ZAX_BEND`** (the curvature is a variant; at curvature 0 nothing of it may reach the compiler, and nothing of it may become a per-vertex uniform), linear palette (indexed albedo path), uniform cache, instance block layout | captures, `bend_tests.rs`, `ZAXNES_SRGB_PALETTE=1` comparison |
| `Zax.Nes/Ppu` pixel composition | 5.4 **the opacity mask of REQ-CORE-049 rides in bit 6 of the picture byte**: `palette_marked` is maintained only where `palette` is written, and `run_dots` must stay at 2231 instructions in the post-LTO assembly | PPU goldens, the `run_dots` instruction count |
| `Zax.Render` passes / new draw states | 5.4 grouping, labels and runs; clip unification; transparent object grouping | captures with and without `--group-draws` |
| `Zax.App/Loop` (frame clock, catch-up, pacing) | 5.5 separate core order per tick, ahead, pipeline governor, catch-up limit, half-rate pacing, **run-ahead** (the restore must be added to any new frame-running path, and N = 0 must stay `run_frame`) | app tests (`catch_up_tests`, `separate_core_tests`, `run_ahead_tests`), device profiler |
| `Zax.Settings` (new settings) | handheld defaults (`ui/handheld/defaults.rs`), handheld option rows, desktop `EXCLUDED` | settings catalogue test |
| Lua runtime / GC | 5.3 smooth GC tuning, `unsafe impl Send` handoff | bench-core with `ZAXNES_LUA_GC=smooth` |

Before closing the update: run the full golden suites, the 56-capture comparison in both draw
modes, `bench-core` on SMB1/SMB3/MM2/Metroid/Zelda II against the numbers above, and one device
run with the profiler on a light and a heavy profile.
