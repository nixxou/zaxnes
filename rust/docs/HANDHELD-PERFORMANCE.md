# Handheld performance — status and next steps

Target: Anbernic RG40XXV (Allwinner H700, 4x Cortex-A53 @ 1.512 GHz, Mali-G31, 1 GB RAM,
720x480 output on the owner's unit), PortMaster port, firmware SDL2 (video driver `mali`),
GLES 3.0 context on an OpenGL ES 3.2 driver (`v1.r20p0`). See `HANDHELD-RESEARCH.md` for the
platform survey and `ROADMAP.md` for the ordered plan.

## 1. What runs today

- Package: `bash rust/build/package-linux-aarch64.sh` -> `rust/target/package/Zaxnes.zip`
  (zig + cargo-zigbuild, glibc <= 2.31 checked, firmware `libSDL2-2.0.so.0`, `target-cpu=cortex-a53`,
  release + fat LTO). Unzip into the SD card `ports/` folder (`ports/Zaxnes.sh`, `ports/zaxnes/`).
  The zip must be named `Zaxnes.zip` (matches `port.json` "name"); manual unzip is the reliable path.
- Launcher `rust/build/portmaster/Zaxnes.sh`: finds its own folder, writes `ports/zaxnes/log.txt`
  (date, firmware, device, GPU, glibc, CPU governor), forces the `performance` governor for the
  run, exports `ZAXNES_PROFILE=1` (temporary diagnostic), runs `--handheld`. gptokeyb was removed:
  its Select+Start kill switch killed the process before the app could save settings.
- The app writes its own log to `ports/zaxnes/logs/zaxnes.log` (flushed per line): GL/SDL report
  and, every 2 s with `ZAXNES_PROFILE=1`, mean/max per frame stage.
- Verified on the device: hardware Mali driver, menus at a steady 60 fps (UI ~0.7-1.5 ms),
  AMD-037 derived ids for the 45 identifier-less profiles, user data in `ports/zaxnes`.

## 2. Measurements (owner's device logs)

Profiler line fields: `core` (NES emulation), `capture` (tile records), `conv` (3D conversion),
`lua`, `rollback`, `scene` (3D render CPU submit), `ui`, `swap` (vsync wait), and "emulated N per
displayed frame". All values are per displayed frame.

| Run | emulated / displayed | core | conv | lua | scene | display fps |
|---|---|---|---|---|---|---|
| Before any core work (5e7ff3d) | 5.0 | ~90 ms | 3 ms | 5 ms | 4 ms | 6-10 |
| PPU micro-optimisations (4fec238 / 537c0b2) | 5.0 | ~87 ms | 3 ms | 5 ms | 4 ms | ~10 |
| Same, heavy 3D profile | 5.0 | ~87 ms | 15 ms | 25 ms (spikes 190-260 ms) | 60 ms | ~5 |
| Batched PPU (91123ec), 2D-ish game | 5.0 | ~78 ms | 4 ms | 4 ms | 14 ms | ~10 |
| Batched PPU (91123ec), heavy 3D profile | 5.0 | ~78 ms | 15-18 ms | 25-55 ms | 70-95 ms | 4-5 |

- Core: about 17.4 ms per NES frame on the A53 (x86 release 1.9-2.9 ms; qemu-aarch64 13-17 ms).
  The budget for everything is 16.6 ms, so the core alone cannot keep real time.
- When a frame exceeds 16.6 ms the loop runs up to 5 emulated frames per displayed frame
  (REQ-LOOP-012 catch-up), which makes the next frame slower still: the display collapses.
- Heavy 3D profile: scene render 60 ms (one draw call per object, ~17 us CPU per draw on Mali),
  Lua 25 ms with large spikes (likely full GC cycles), conversion 15 ms per 5 frames.

## 3. Tools

- `ZAXNES_PROFILE=1` (app): per-stage profiler in the log and under the FPS counter.
- `cargo run --release -p zax-harness -- bench-core <rom.nes> [options]`: ms per emulated frame
  of each stage without a display (x86, or the aarch64 build under
  `qemu-aarch64 -L /usr/aarch64-linux-gnu`: relative weights only).
- `cargo test -p zax-nes --release --test ppu_golden_tests -- --ignored`: 291 ROMs x 1200
  frames, pixel / tiles / audio / cycles / blargg must stay identical. Mandatory after any
  core change.
- `zax-harness golden-core <roms dir> <out.jsonl> 1200`: run it on your own ROM folder at an old
  and a new commit and compare the files: identical output on real games.
- Lua binding benchmark: `cargo test -p zax-scripting --release --test scripting perf -- --ignored --nocapture --test-threads=1`.

## 4. Work done on performance (commits on `rust-port`)

- Lua binding rounds 1-2 (`2a76217` ... `4795df3`, `7f542da`): synthetic frame 397 -> ~203 us on
  x86; frame handles keep identity (REQ-LUA-043); lazy `shape2D` reverted (scripts may set a
  metatable on `_G`, as in C#).
- Profiler, file log, launcher diagnostics, cortex-a53 build (`13f8a08`, `5e7ff3d`); rollback
  snapshots reuse the evicted buffer; no draw list built in 2D mode.
- Core milestone 1 (`537c0b2`): PPU dot dispatch by range, per-line sprite buffer. Bit-identical,
  but only ~3 % on the device.
- Core milestone 2 (`91123ec`): the PPU is caught up in batches at sync points (CPU access to
  $2000-$3FFF, $4014, $4016/$4017 reads, any cartridge write, vblank/NMI edges, an MMC3 IRQ
  horizon; Rambo1 and MMC5 still step every cycle), idle dots skipped. Full goldens and
  `cargo test -p zax-nes --release` pass. bench-core x86 core ms/frame (spritecans / nes15 /
  smwstomp / rastesam4): 2.97 / 2.30 / 1.98 / 1.88 -> 1.48 / 1.53 / 1.45 / 1.34 (approximate,
  shared CPU). Device: core 78 ms per 5 frames = ~15.6 ms per NES frame (was ~17.4): only ~11 % on the A53
  although x86 gained 1.4-2x. The in-order A53 (small caches) does not profit like x86; count
  instructions and cache misses (callgrind with `--cache-sim=yes`) rather than x86 wall time.
  The core now fits in 16.6 ms on its own, which makes the multi-core step (5.2) the biggest
  expected win: emulation alone on one core could reach ~60 emulated frames/s.
- Profiling before milestone 2 (callgrind, nes15): ~23 M instructions per frame; PPU 62 %
  (~160 instructions per dot), APU 17 %, CPU + bus + PRG reads 15 %. Each PPU VRAM fetch costs
  ~116 instructions (A12 notification through the `Board` match, `ppu_read`, `banks()`,
  `read_chr` with a modulo); the `Board` match lines alone are ~2 M instructions per frame.
  Removing the PRG/CHR `% len` gave no x86 gain (dropped).
- Recommended next core steps: (1) in `run_dots` (`zax-nes/src/ppu/mod.rs`) a tight loop for
  visible lines with rendering on, dots 1-256: registers cannot change inside a batch, so hoist
  mask / fine-X checks and work per 8-dot tile with direct CHR / nametable reads (expected
  ~160 -> ~50 instructions per dot); (2) batch the APU the same way (sync on $4000-$4017 and on
  frame / DMC IRQ cycles); (3) IRQ horizons for Rambo1 and MMC5 so they batch too.
- `91123ec` checked on Windows afterwards: core goldens, fmt, clippy and the workspace tests
  (`--all-features`) pass.
- Core milestone 3 (PPU fast span): on a visible line with rendering on, dots 2 to 255 run in
  one loop without the per-dot dispatch, with the PPUMASK / fine-X decisions hoisted and the
  fetches reading CHR and console VRAM directly when the board's fetches have no side effect
  (`Mapper::chr_view`: every board but MMC2/MMC4, MMC3 and compatibles, Rambo1, MMC5, which keep
  the full path). Goldens and `cargo test -p zax-nes --release` pass. FF1 (MMC1, 2D, no
  profile) bench-core x86 core 1.18 -> 0.87 ms/frame; callgrind before: core 25.4 M
  instructions per frame (PPU 51 %, APU 19.5 %, ~185 instructions per CPU cycle in the APU).
- Device run of `a0c157a` (owner): FF1 2D core ~12 ms per frame (was ~15.6), 60 emulated and
  displayed fps, smooth. Zelda II 3D profile: core 12.7, lua 6, conv 1.1, scene 29 ms per
  displayed frame, ~20 fps; without catch-up the game then runs at a third of its speed (felt
  worse than the old 5-frame catch-up, which kept the speed at ~5 fps). The scene is now the
  first cost: 934 draw calls for 27 distinct objects in the first palace (harness:
  `render --rom` with the press script in the notes below).
- Draw state caching (5.3, first step, pixel-identical): `ShaderProgram` keeps the bits last
  uploaded per uniform and skips identical uploads, with an FNV hasher for the name lookups
  (SipHash was ~2 % of all instructions); the scene draws rebind textures and vertex arrays only
  when they change. Zelda II frame identical (tolerance 0), 12 games x 3 angles identical to
  the previous captures; RTX 3060 render 0.44 -> 0.33 ms median.
  Zelda II press script (no save, registers "A"): `100:start,200:start,300:a,340:select,
  370:select,400:select,440:start,540:start,640:start`, `--run 1300`.
- Device run of `83aa286` (owner): Zelda II temple scene 29 -> 17 ms, world map scene ~50 ms
  (+ conv 3.6, lua 5.5) at ~14 fps, battle area scene ~10 ms. World map: 1559 draw calls for 13
  distinct objects; 611 tiles carry a deformation (water).
- Draw-call batching (5.3): the scene shader reads each draw's placements from a uniform block
  (`ZaxInstances`: model matrix, normal matrix columns, deformation tile position; 128
  instances of 128 bytes per draw, one buffer upload per frame) and consecutive items with the
  same draw state (`DrawStateKey`: everything but the placement and the deformation tile) go
  out as one `glDrawElementsInstanced`. Pixel-identical (Zelda II temple and world map, 36
  campaign captures at tolerance 0). `RenderStatistics::draw_calls` still counts tile draws
  (the goldens); `batches` counts the GL calls (`render --bench` prints "GL draws").
  The tile list interleaves meshes (each tile emits its parts in turn), so on its own this
  only saves ~5-8 %: temple 934 -> 888, world 1559 -> 1443.
- Grouped opaque draws (setting, handheld default): `SceneRenderer::set_group_opaque_draws`
  reorders the opaque bucket by draw state (groups in first-appearance order, list order inside
  a group). Temple 934 -> 138 GL draws, world map 1559 -> 72. Opaque surfaces are depth-tested
  without blending, so only exact depth ties between two different states change: Zelda II
  world map 4 pixels of 86 400, campaign 22 of 36 captures differ by at most 0.002 %. On by
  default with `--handheld`, off on desktop; `ZAXNES_GROUP_DRAWS=0/1` forces it, harness
  `--group-draws`; the profiler start-up line shows it.
- Device run of `cf00717` (owner): temple core 12.8 / lua 7.3 / conv 1.1 / scene 9 ms (~32 fps),
  world map 12.7 / 4.7 / 3.4 / 13.8 (~28 fps, scene was 50), battle 12.7 / 1 / 0.8 / 5.7
  (~47 fps). The core is now the largest stage everywhere. Rewind removed on handhelds (owner).
- Emulation on a separate core (5.2, setting, handheld default): `core_worker.rs` runs each
  emulated frame on a "zaxnes-emulation" thread. The session's core is moved there (never
  shared: `GameSession::core` holds a ROM-less stand-in meanwhile) and taken back at the start
  of the next tick, before anything reads or changes it. Order per tick: finish frame N (tile
  capture, conversion, frame hooks), hotkeys and commands, prepare N+1 (input hook, input word),
  battery / audio / settings / publication, then launch N+1 while the main thread draws N. The
  scripts and the core see each other exactly as on one thread (test
  `separate_core_tests.rs`: same frames, memory and picture); the frame is shown one displayed
  frame later (input latency + 1 frame). `ZAXNES_SEPARATE_CORE=0/1` forces it; the profiler
  start-up line shows it. Expected on the device (cf00717 numbers): battle ~max(12.7, 8) ms
  -> 60 fps, temple ~21 ms, world map ~22 ms.
- Catch-up limit (pacing 5.5, first half): on a handheld the frame clock no longer catches up
  (`CatchUp::Off`: a displayed frame runs only the frames it stands for, effective rate over
  displayed rate rounded up, so 1 at 60 Hz, 2 at 30 Hz or in fast forward). A late frame now
  slows the game a little instead of collapsing into 5 emulated frames per displayed frame.
  Desktop keeps REQ-LOOP-012 (5). `--catch-up <0-5>` / `ZAXNES_CATCH_UP` override it; the
  profiler start-up line shows the value. Owner remark (FF1, no profile, 2D, still slow): in 2D
  only the core runs, so multi-core cannot help there; the core itself must get faster.

## 5. Next steps (see `ROADMAP.md`)

Execution order revised with the owner (2D games without a profile are slow too, and there the
core is the only stage): 5.5 catch-up limit (done), then more of 5.1 (core, measured in
instructions), 5.2 multi-core, 5.3 draw-call batching, 5.4 Lua, then 5.6 only if needed.

Rule: an optimisation that changes any output is a setting (on/off, platform default, per game
where useful); an optimisation with identical output needs no switch but must pass the goldens.

1. Structural core work, identical output. Profile with instruction counts first
   (`valgrind --tool=callgrind` on a short `bench-core` run: closer to an in-order A53 than x86
   wall time). Candidates: lazy/batched PPU and APU (run them up to the current CPU cycle only
   when the CPU touches $2000-$2007/$4014/$4015-$4017, at mapper A12/IRQ events, NMI edge and
   frame end), a lighter CPU -> bus -> PPU/APU/mapper chain on every cycle, fast paths for RAM
   and PRG ROM reads, background output per 8-pixel span when nothing can change mid-tile.
   Goal: >= 2x on the device (core <= 9 ms per NES frame).
2. Multi-core: emulation on its own thread; conversion, Lua and render of frame N-1 on the main
   thread; Lua reads a per-frame snapshot of NES memory. Setting "emulation on a separate core".
   Check the C# semantics of when hooks see memory (SPEC-05, SPEC-07).
3. Draw-call batching (pixel-identical): per-object parameters in one UBO filled once per frame,
   sort opaque items by program/texture/mesh, instancing of identical items. Measure first with
   `render --explain`. Also the base for a future Vulkan/DX12 (wgpu) backend.
4. Lua: per-hook/per-script/per-API profiling; incremental GC steps every frame instead of the
   current "wait for 4x heap growth" pause (removes the spikes); dedicated userdata types per host
   object; reuse temporary `Vector3`.
5. Handheld pacing settings: regular 30 fps (2 emulated per displayed frame) and capped catch-up.
6. Fallbacks, only if 1-2 are not enough: "fast PPU" (line-based) or an external core
   (FCEUmm / QuickNES; check the exact licence: GPL-2.0-or-later or LGPL only), as settings.

Expected outcome: with a 2x core, light profiles reach 30 fps at full game speed (60 for the
lightest); heavy 3D profiles also need steps 3 and 4.

## 6. Building the handheld package from a Windows PC

The package script is bash. Two ways:
- WSL (Ubuntu): `sudo apt install cmake zip python3-pip` then `pip install ziglang`,
  `cargo install cargo-zigbuild`, `rustup target add aarch64-unknown-linux-gnu`, and run
  `bash rust/build/package-linux-aarch64.sh` from the repository.
- Native Windows: zig and cargo-zigbuild run on Windows, but the script and the SDL2 link step
  (`build/aarch64-sdl2-zig.cmake`) were only tested on Linux; a PowerShell port is not written.
