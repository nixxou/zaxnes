# Zaxnes Rust port — plan

The orchestrator reads this file once, then runs the tasks **in order, one agent at a time**, with the model given for each task. Each brief below is passed verbatim to the task agent, preceded by the **common preamble** (§1) and followed by the current `rust/PROGRESS.md`.

Model choice: `opus` only where cycle-exact timing, geometry or GPU work makes mistakes expensive; `sonnet` for ordinary porting; `haiku` for mechanical, well-tested code.

## 1. Common preamble (prepend to every brief)

> You are porting one part of Zaxnes from C# (`src/`, reference, do not modify) to Rust (`rust/`). Read `rust/RUST-PORT-PLAN.md` §2 (Rust rules) only, then do your task and nothing else.
> Token economy: read only the files listed in your task; use `grep -n` and ranged reads; never read a file twice; spec: only the cited sections; pipe build/test output through `tail -n 30`; `cargo test -q`; run only your crate's tests. No refactors outside your task, no extra features.
> Port the C# unit tests of the code you port (same names; keep the `REQ-…` ids). Make `cargo clippy -p <crate> --all-targets -- -D warnings` and `cargo test -p <crate>` pass.
> Do not commit: the orchestrator commits. Finish with a report of at most 10 lines: what was ported, test counts, golden parity, open points. No code in the report.

## 2. Rust rules (the agents read this section)

- Workspace `rust/Cargo.toml`, edition 2021, stable; `rust/rust-toolchain.toml` pins the channel.
- Crates (`rust/crates/<name>`), mirroring the C# projects:

  | Crate | C# source | Kind |
  |---|---|---|
  | `zax-nes` | `src/Zax.Nes` | lib, no I/O, `forbid(unsafe_code)` |
  | `zax-settings` | `src/Zax.Settings` | lib, `forbid(unsafe_code)` |
  | `zax-profiles` | `src/Zax.Profiles` | lib, `forbid(unsafe_code)` |
  | `zax-scripting` | `src/Zax.Scripting` | lib, `mlua` (Lua 5.4, `vendored`) |
  | `zax-engine3d` | `src/Zax.Engine3D` | lib, `forbid(unsafe_code)` |
  | `zax-render` | `src/Zax.Render` | lib, `glow` (GL 3.3 core and GLES 3.0) |
  | `zax-platform` | `src/Zax.Platform` | lib, `sdl2` (window, GL context, input, gamepads, audio) |
  | `zax-app` | `src/Zax.App` | bin `zaxnes`, UI with `imgui` + `imgui-glow-renderer` |
  | `zax-harness` | `tools/Zax.Harness` | bin, headless tools (golden dumps, meshes, render, stress) |
  | `zax-goldens` | — | test-only helper that reads `rust/goldens/` |

  Add a dependency only when the task needs it; prefer well-known crates. `serde`/`serde_json` for JSON, `thiserror` for errors, `sha2` in tests and the harness.
- Workspace lints (in `rust/Cargo.toml`, `[workspace.lints]`): `clippy::all` and `clippy::pedantic` warn; `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`, `dbg_macro` deny (allowed in `#[cfg(test)]` code via `clippy.toml` `allow-unwrap-in-tests = true`, `allow-expect-in-tests = true`). Builds use `-D warnings`. Justified local `#[allow(clippy::…)]` need a one-line reason.
- `unsafe`: forbidden except in `zax-render`, `zax-platform`, `zax-scripting`, confined to small modules, each block with `// SAFETY:`.
- Hot paths (CPU step, PPU dot, APU tick, mesh building): no allocation per frame, no `Rc<RefCell>` on the hot path, no trait objects where an enum does; mappers as an `enum` or a `Box<dyn Mapper>` created once.
- Floating point: where the C# uses `float`, use `f32`; where it uses `double`, use `f64`; keep the same operation order where the goldens hash the result (audio, geometry).
- The core crates never touch the file system or host paths; the app and harness pass bytes in.
- Tests needing commercial ROMs or game profiles read `ZAXNES_ROMS` / `ZAXNES_PROFILES` and are `#[ignore]`d with a reason when absent. Public test ROMs are in `rust/test-roms/` (not committed); their tests skip with a message if a file is missing.
- Comments: English, short, in the style of the C# file, citing `REQ-…`/`AMD-…`. Adapted 3DNes_Engine code keeps `// Portions adapted from 3DNes_Engine (c) Geod Studio, GPL-3.0`.

## 3. Goldens

`rust/goldens/README.md` describes the files. `core-goldens.jsonl`: per public test ROM, SHA-256 of the PPU pixel frame, of the tile records and of the i16-quantised audio, plus cycle counts, at frames 1/60/300/600/1200, and the blargg status/text. `render-synthetic/`: offscreen captures of the built-in synthetic scenes (640×480) with `render.json`. `lua-api.txt`: the script-visible API dump.

## 4. Tasks

### T00 — workspace bootstrap · `sonnet`
Create the workspace and every crate as an empty skeleton (lib or bin, `forbid(unsafe_code)` where §2 says so), workspace lints, `clippy.toml`, `rust-toolchain.toml`, `rust/.gitignore` (`target/`, `test-roms/`), and the crate `zax-goldens` with typed readers for `core-goldens.jsonl`, `render.json` and `lua-api.txt` (serde) plus a unit test that loads each file. Add `rust/check.sh`: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace -q`. Read: `rust/goldens/README.md`, the `.csproj` files of `src/` (dependencies only). Done: `rust/check.sh` passes.

### T01 — 6502 CPU and bus · `opus`
Port `src/Zax.Nes/Cpu/` and `src/Zax.Nes/NesBus.cs` (with the minimal stubs of PPU/APU/cartridge the bus needs, behind the same interfaces the C# uses) into `zax-nes`. Port `tests/Zax.Nes.Tests` CPU tests. nestest: run `rust/test-roms/nestest.nes` from `$C000` and compare with `nestest.log` line by line (PC, registers, cycle count). Spec: `docs/spec/SPEC-02-coeur-nes.md`, CPU sections only, and AMDs that cite REQ-CORE CPU ids (`grep -l "REQ-CORE" docs/spec/changes`). Done: nestest log identical to the last official line; CPU tests pass.

### T02 — PPU and tile records · `opus`
Port `src/Zax.Nes/Ppu/`, `TileRecord.cs`, `FrameBatch.cs`, `NesColors.cs`, `VideoInfo.cs`, `RegionTiming.cs`, `NesRegion.cs`, `NametableMirroring.cs` and the NROM cartridge (`Cartridges/Mappers/Nrom.cs` with `Cartridge.cs`, `BankedCartridge.cs`, `ChrFetch.cs`, `A12Watcher.cs`) and the iNES/NES 2.0 loader (`src/Zax.Nes/Rom/`). Port the PPU tests of `tests/Zax.Nes.Tests`. Golden: for every row of `core-goldens.jsonl` whose ROM is mapper 0, `pixel_sha256` and `tiles_sha256` must match at every checkpoint (tile hash serialisation: see `rust/goldens/README.md`). Spec: SPEC-02 PPU sections. Done: those goldens and PPU tests pass.

### T03 — APU and resampler · `opus`
Port `src/Zax.Nes/Apu/` (channels, frame counter, DMC with its DMA, mixer, resampler, `ReadAudio`). Port the APU tests. Golden: `audio_i16_sha256_samples` and `cycles` for mapper-0 rows (quantisation rule in the goldens README). If bit-exact audio is out of reach because of a float-order difference you cannot find, stop after documenting the first diverging frame in `PROGRESS.md`. Spec: SPEC-02 APU sections, SPEC-11 where it concerns the core. Done: audio goldens of mapper-0 rows pass.

### T04 — mappers and input ports · `sonnet`
Port the remaining mappers of `src/Zax.Nes/Cartridges/Mappers/` (MMC1, MMC2, MMC3, MMC5, UxROM, CNROM, AxROM, GxROM, CPROM, Color Dreams, Codemasters, Action 52/53, Bandai FCG, Irem G101, Jaleco J87/SS88006, Rambo-1, VRC2/4, UNROM 512, Caltron, 225, 15, 34) and `MapperRegistry.cs`, and `src/Zax.Nes/Input/` (standard pad, Zapper, ports). Port their tests. Escalate only MMC3/MMC5 IRQ timing issues to `opus` if they fail twice. **Milestone**: every row of `core-goldens.jsonl` matches (pixel, tiles, audio, cycles, blargg). Done: 291/291 rows or a precise list of the remaining rows in `PROGRESS.md`.

### T05 — core facade and state · `sonnet`
Port `NesCore.cs`, `NesModule.cs`, `ControllerPortType.cs`, `ZapperPosition.cs` and `src/Zax.Nes/State/` (save states with their format version and core name, battery RAM import/export, rewind support the C# provides). Port the remaining `tests/Zax.Nes.Tests`. Add `zax-harness golden-core <roms dir> <out.jsonl> [frames]` that reproduces `core-goldens.jsonl` (compared as parsed JSON, row by row; key order and spacing do not matter). Done: all `zax-nes` tests pass; harness output equals the golden rows for the ROMs present.

### T06 — settings · `haiku`
Port `src/Zax.Settings/` to `zax-settings` (catalogue, descriptors, keys, coercion, persistence format, `ui.language`) and all of `tests/Zax.Settings.Tests`. Spec: SPEC-09 only where a test needs clarification. Done: all settings tests pass.

### T07 — profiles · `sonnet`
Port `src/Zax.Profiles/` (`Wire/` protobuf reader and tile cipher, `Catalogue/`, `Library/` with the executable-folder rule of AMD-033, `Import/` 3dSen import of AMD-030 including the UnityFS reader and LZ4) to `zax-profiles`, and `tests/Zax.Profiles.Tests`. Tests that read built-in profiles become `#[ignore]` with `ZAXNES_PROFILES`. Spec: SPEC-03, AMD-030, AMD-033, cited sections only. Done: profile tests pass (ignored ones listed in `PROGRESS.md`).

### T08 — Lua scripting · `sonnet`
Port `src/Zax.Scripting/` to `zax-scripting` with `mlua` (Lua 5.4, vendored; profiles use `goto`). Port `tests/Zax.Scripting.Tests`. Add `zax-harness lua-api [--out]` and make its output identical to `rust/goldens/lua-api.txt`. Spec: SPEC-05 §4 only. Done: tests pass, API dump identical.

### T09 — 3D conversion engine · `opus`
Port `src/Zax.Engine3D/` (tile conversion, geometries, greedy / marching / multicolor meshing, layers, transparency per object, mirrored transforms of AMD-028, sealed walls of AMD-027) to `zax-engine3d`, and `tests/Zax.Engine3D.Tests`. Add `zax-harness meshes` with the C# options and output files (`summary.json`, `frame<NNNN>.obj/.json`). Spec: SPEC-04, SPEC-04B, cited sections only. Done: engine tests pass; for synthetic frames, the harness output has the same counts as the C# tests expect.

### T10 — renderer · `opus`
Port `src/Zax.Render/` to `zax-render` with `glow`: shaders for GL 3.3 core **and** GLES 3.0 (same GLSL with a version header), scene pass, shadow map 4096 with the 2048/1024 fallback and 1-texel bias (AMD-034), skybox, HUD layer, anti-aliasing and resolution scale (AMD-032). Port `tests/Zax.Render.Tests` that do not need a GPU; GPU tests are `#[ignore]` unless an OpenGL context is available (try Mesa llvmpipe via EGL surfaceless; if unavailable, leave them ignored). Add `zax-harness render --frames synthetic` and compare with `rust/goldens/render-synthetic/` (per-channel tolerance 2, ≥ 99.5 % pixels) when a context exists. Spec: SPEC-06, AMD-032, AMD-034. Done: non-GPU tests pass; GPU comparison done or marked "to check on a real machine".

### T11 — platform · `sonnet`
Port `src/Zax.Platform/` to `zax-platform` with `sdl2`: window and GL/GLES context creation (desktop GL 3.3 core; GLES 3.0 when `ZAXNES_GLES=1` or on aarch64 Linux), swap interval / pacing (`DisplayPacing.cs`), window icon, keyboard / mouse / gamepad input, audio output. Keep it buildable without a display (no test needs a window). Done: crate builds and its pure tests pass.

### T12 — application loop · `sonnet`
Port `src/Zax.App/` except `Ui/`: `ZaxApplication`, `Loop/` (emulation host, commands, games, rewind, save states, fast-forward), `GameSession`, `Persistence/`, `Input/`, `Audio/`, frame pacing and render scaling, `SingleInstance`, `ProfileResolver`, library catalogue, scene script bridge, user-data paths with portable mode and `--userdata`. Port the non-UI tests of `tests/Zax.App.Tests` (use the C# `LoopHarness` approach). Done: those tests pass.

### T13 — user interface · `sonnet`
Port `src/Zax.App/Ui/` with `imgui`: menu bar, options window (all tabs), dialogs, welcome screen, notifications, file browser, profile chooser, key configuration, About, and the English/French tables (`UiKeys`, `UiStrings.*`, AMD-035/036; same keys, English default, immediate switch). Port the UI tests of `tests/Zax.App.Tests` (models, key parity of the two tables). Done: tests pass; `zaxnes --ui-shots <dir>` is wired if a GL context is available, otherwise noted.

### T14 — command line, packaging · `haiku`
Port `Program.cs`/`AppOptions.cs`: `--help`, `--rom`, `--profile`, `--list-profiles`, `--import-3dsen`, `--userdata`, `--bench`, vsync overrides (AMD-026), portable mode, console messages in English. Packaging scripts in `rust/build/`: Windows x64 zip (exe, `profiles/README.txt`, licences, `HOWTO-RUN.md`) and Linux aarch64 (`cargo build --target aarch64-unknown-linux-gnu`, SDL2 KMSDRM notes for the RG40XXV). Update `THIRD-PARTY-NOTICES.md` with the Rust crates and licences (`cargo metadata`). Done: `zaxnes --help` output matches the C# text; scripts present.

### T15 — final parity pass · `sonnet`
Run `rust/check.sh` (workspace milestone) and every golden comparison. Write in `rust/PROGRESS.md`: a parity table (core goldens n/291, render n/11 or "needs GPU", Lua API identical, test counts per crate vs C#), the list of what must be checked on a real machine (window, audio, gamepad, GPU captures, commercial ROMs and profiles through `ZAXNES_ROMS`/`ZAXNES_PROFILES`, `--bench` < 4 ms at 1080p, RG40XXV), and the known gaps. Add `rust/LOCAL-CHECKS.md` with the exact commands for the user to run those checks locally, side by side with the C# harness. Done: report.

## 5. `rust/PROGRESS.md` format

```
# Rust port progress
| Task | Model | Status | Tests | Goldens | Notes |
|---|---|---|---|---|---|
| T00 | sonnet | done | 4/4 | — | |
```
One line per task, a few lines of notes at most below the table.
