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
- See the final lines of `rust/PROGRESS.md` and `git log --oneline` for anything committed after
  this document (the core work may have continued).

## 5. Next steps, in order (see `ROADMAP.md`)

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
