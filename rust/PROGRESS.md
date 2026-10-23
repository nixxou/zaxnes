# Rust port progress
| Task | Model | Status | Tests | Goldens | Notes |
|---|---|---|---|---|---|
| T00 | sonnet | done | 3/3 | readers load 291 rows, render.json, lua-api | MSRV 1.85; 10 skeleton crates |
| T01 | opus | done | 64/64 | nestest 8991/8991 lines | PPU/APU/input/cart stubs; deferred: 4 PPU memory tests (T02), battery RAM (T04/T05), CPU/bus save state (T05) |
| T02 | opus | done | 151/151 (+1 ignored full run) | mapper-0 pixel+tiles 181/222; 41 pending T03 (APU-dependent, listed in `PENDING_T03`) | `Machine` frame runner for T05; full run: `cargo test -p zax-nes --test ppu_golden_tests -- --ignored` (~60 s) |
| T03 | opus | done | 209/209 (+5 ignored: full golden run, MMC1 `cpu_interrupts.nes`, 3 pending-by-design ROM notes) | mapper-0 audio+cycles+pixel+tiles+blargg 221/222 at 1200 frames (bit-exact audio); 1 pending T04 (`dma_4016_read`: input stub, 1-cycle drift) in `PENDING_T04` | `ApuStateTests` deferred to T05 (no save state in APU yet); `AudioReader` handle for the host audio thread; audio setters return `Result<_, AudioError>` |
| T04 | sonnet | done | 319/319 (+6 ignored; 110 new: mapper 57, irq 13, rom 16, tile 3, input 11, zapper 10; `background_row_split` ignored: needs `ZAXNES_ROMS`) | 291/291 rows (pixel, tiles, audio, cycles, blargg) at frames 60 and 1200, empty `PENDING_T04`, `load_failure` compared | 31 mappers in `cartridges/registry.rs` + `Board` enum dispatching a `Mapper` trait; `InputPorts` real (pad, Zapper; `read` takes a light probe, `NesBus::read_input_port` wires the PPU). Deferred to T05 (need `NesCore`/save state): the 31-mapper save/restore theory, battery import/export tests, 2 `with/without video callback` state-equality tests, Zapper position-source tests, `GetPortType` on a core |
| T05 | sonnet | done | 384/384 zax-nes+harness (+5 ignored, C# pending or ROM-gated) | `zax-harness golden-core` = 291/291 rows at 1200 frames (parsed JSON) | `Machine` folded into `NesCore`; save state format v4 for all components; video callback gets the batch only (no memory read inside callback) |
| T06 | haiku | done | 177/177 (71 facts + 106 inline rows, rstest per row; catalogue table checked in one loop) | n/a (no goldens) | `SettingId` enum with stable keys; `SettingsModel` events drained via `drain_changes()`; `SettingsFile`/`GameConfigFile` own their storage; `StorageError` replaces bool results; JSON via `serde_json` (`preserve_order`) |
| T07 | sonnet | done | 156/156 (+54 ignored, need `ZAXNES_PROFILES`; 4 also need `ZAXNES_3DSEN_INSTALL`) | n/a (no goldens; wire vectors of SPEC-03 §5 pass; corpus byte-identity 101/101 untested here) | `zax-profiles`: model, protobuf codec, catalogue, library, 3dSen/UnityFS import. Shared tiles/patterns/variants are `Arc` (identity = reference keys); per-node `recorded_fields` replaces `ProfileWireLayout` (`FieldPresence::{All, AsRecorded}`). `ProfileFileSystem` trait (+ `list_directories` for the installation search); no physical impl in the crate (tests have one). Ignored: `profile_corpus_tests` (17), `profile_round_trip_tests` (5), `profile_wire_vector_tests` (6), `tile_texture_and_tag_tests` (3), `script_order_tests` (4), `setting_application_tests` (4), `profile_validation_tests` (6), `catalogue_and_library_tests` (5), `three_d_sen_real_install_tests` (4) |
| T08 | sonnet | done | 112/112 zax-scripting (+7 ignored, corpus needs `ZAXNES_PROFILES`) + 5 harness | `zax-harness lua-api` byte-identical to `goldens/lua-api.txt` (test in harness) | `zax-scripting` on mlua 5.4 vendored; one `Host` userdata over data-driven `TypeDef` tables shared by runtime and API dump; frame-local handles with epochs. Open: CoreScriptInput/Memory not ported (app implements the traits); SlotTexture w/h uses the 8W x 8H fallback; BuildPattern has only the instance (state members raise "not available here"); `Camera.FpvShape` reads nil after its frame epoch changes; ViewReset via flag; C# "Disposed runtime" test not portable |
| T09 | opus | done | 191/191 zax-engine3d (+6 ignored, need `ZAXNES_PROFILES`) + 3 harness `meshes` | n/a (no mesh goldens); `meshes` on synthetic frames gives the engine-test counts (prism 12 tri, cube 24, Q-067 sealed pair 10+10) | `zax-engine3d`: pipeline, segmentation, tracker, learned-state codec, geometry (voxels, greedy/multicolour/marching, sealed sides), draw list. Registry patterns are generational handles (`PatternId`), frame tiles/shapes/instances are frame-local handles; `ScriptHost` trait gets `&mut ConversionState` in frame hooks (T08 implements it); hooks return `Result<_, ScriptError>`. Pinned-frame restore copies the shape into the converting frame (no shared ownership). `zax-harness meshes` = C# options and outputs |
| T10 | opus | done | 124/124 zax-render (+1 ignored: 8× MSAA, llvmpipe caps GL_MAX_SAMPLES at 4) + 3 harness `render`; GPU tests run on Mesa llvmpipe (EGL surfaceless), incl. a GLES 3.0 vs GL 3.3 comparison | `zax-harness render --frames synthetic` vs `goldens/render-synthetic`: render.json counts identical for 11/11 scenes; pixels within 2 levels: 9 scenes 100 %, framing 99.998 %, heavy 99.769 % (z-fighting of overlapping coplanar sprites) | `zax-render` on glow: safe scene/camera/colour code, GL calls only in `gl/api.rs` (+ `offscreen.rs`, feature `offscreen`, EGL for tests/harness); shaders as `.glsl` files with a GL 3.3 / GLES 3.0 header; `SceneState` persistent + per-frame `SceneFrame` borrowing the draw list and picture; `SceneRenderer::new(Rc<glow::Context>, Option<Box<dyn SceneAssets>>)`; PNG via `png` crate. Not ported: harness `--rom`/JSON frames/`--explain`/`--bench`, `ISceneRenderer` interface |
| T11 | sonnet | done | 30 zax-platform pure tests (pacing, icon, wave, controls, calibration, labels, keys, audio PCM, GL flavour); no test opens a window | n/a (no goldens) | `zax-platform` on `sdl2` 0.38, system SDL2 by default; feature `bundled` (static) for Windows/aarch64, else ship `SDL2.dll` (T14). `HostWindow` = window + GL 3.3 core / GLES 3.0 (`ZAXNES_GLES=1` or aarch64 Linux, `gl_flavor()`), `glow_context()`, implements `DisplaySurface` + `WindowIconTarget`, `swap_and_pace`, `pump_events`. Key codes stay GLFW numbers (scancode table). `AudioOutput::open` + `pump(fill)` over an SDL queue (no audio thread). Not run against a real display/device. Dropped: `PlatformModule` kept as two consts |
| T12 | sonnet | done | 296/296 zax-app integration tests (20 files; all non-UI tests of `Zax.App.Tests` incl. input, audio, persistence, rollback, pacing, public-ROM nestest replay); clippy pedantic + fmt clean | n/a (no goldens; savestate replay byte-identical on synthetic ROM and nestest) | `EmulationHost<C: GameCatalog>` owns settings, store, mixer, session and pacer; commands/events are enums (`Command`, `HostEvent`); the Lua runtime borrows the host settings model per call. Only `Rc<RefCell>` are `SharedScene`/`SharedCore` (zax-scripting traits are `'static`). `application.rs` has `ApplicationUi` + `NoUi` for T13; `main.rs` is thin, argument parsing is T14. Touched outside: `zax-platform::framebuffer` (offscreen target, `unsafe`+SAFETY), `ProfileScriptRuntime::fpv_shape`, `ThreeDSenImportError` re-export. Open: core error to Stopped not ported (no exception path); `Setting:Save/Load` from Lua run at the end of the hook; UI tests, AppOptions/CLI tests, `--font` tests left to T13/T14. `--all-features` only turns on `bundled` (static SDL2), T15 documents it in LOCAL-CHECKS.md |
| T13 | sonnet | done | 65 UI tests in zax-app (+1 ignored: `AppOptions.Usage`, T14); zax-app 361 in all | n/a (no goldens); `zaxnes --ui-shots <dir>` writes the 7 screens (checked under Xvfb + llvmpipe) | `imgui` 0.12 (tables-api); upstream `imgui-glow-renderer` (glow 0.14) and `imgui-sdl2-support` (sdl2 0.37) cannot link with glow 0.16/sdl2 0.38, so the GL renderer and the glyph remap are in `zax-platform` (`imgui_renderer`, `imgui_glyphs`) and the SDL input mapping in `ui/platform.rs`. `ApplicationUi` trait dropped: the loop holds `ImguiFrontend`; the host owns the game view (`GameView`, generic `V`) and takes the modal flag by value (no `Rc`); only `--ui-shots` is parsed in `main` until T14. Language is thread-local state. |
| T14 | haiku | done | 386/386 zax-app (+24 new: 5 options, 11 `--import-3dsen`, 8 print-and-exit/window cases; `AppOptions.Usage` enabled, byte-compared with `AppOptions.cs`) | n/a (no goldens) | `app_options.rs` (`parse`, typed `OptionsError`, `USAGE`), `three_d_sen_command.rs`, `main.rs` modes (help, list-profiles, import, single instance). `rust/build/package-windows.ps1` (zip, not run: no pwsh here) and `package-linux-aarch64.sh` (not run: no aarch64 toolchain here). `THIRD-PARTY-NOTICES.md` Rust section: 69 crates. Open: Windows console attach (C# is WinExe, Rust is console subsystem; attaching needs unsafe), no `--bench` in C#, `REQ-PUB-006` system-font half not ported, RG40XXV untested |

Harness: render ROM mode (--rom/--run/--press/--profile/--sav/--dump-frame/--frames) ported

T12 refactor: script bridges borrow core/scene per hook call (no Rc<RefCell>)

Fixes after Windows check: fake profile file system keys on separator-independent paths (+ component-wise `StreamingAssets/aa` join); toolchain pinned to 1.99.1 (clippy, rustfmt), `assert_is_empty` and `manual_midpoint` lints fixed (resampler arithmetic untouched); the G14 test expectation was already correct.

T10 fix: WGL offscreen context for Windows (hidden window, GL 3.3 core; GLES 3.0 via `WGL_EXT_create_context_es2_profile`, else a clear "unsupported" error), EGL kept elsewhere (133 zax-render + harness tests pass on llvmpipe); zax-render compiles and is clippy-clean for x86_64-pc-windows-msvc (checked in container), zax-harness stops at the `mlua-sys` build script (vendored Lua needs the MSVC toolchain); GPU tests and 8x MSAA to be run on Windows by the owner.
Windows check (owner, b6bb4e7, RTX 3060, rustc 1.99.1): WGL GPU tests 125/125 incl. 8x MSAA and GLES 3.0 (WGL_EXT_create_context_es2_profile); synthetic render goldens 11/11 scenes 100 % on real GPU; zax-harness builds with MSVC; zax-platform `bundled` 30/30; workspace fmt/clippy clean, 1264 tests passed with --include-ignored.

T15 (this container: Linux x86_64, Mesa llvmpipe EGL, SDL2 dev libs, rustc 1.99.1): Windows console attach (`zax-platform::console`, `windows_subsystem = "windows"`; module compiles and is clippy-clean for x86_64-pc-windows-msvc alone, the full `zax-platform`/`zax-app` cannot be checked for Windows here: `imgui-sys`/mlua's vendored Lua need the MSVC `lib.exe`); harness `render --bench <frames>` and `--explain` ported (`draw_list_report.rs`, `OffscreenContext::finish`), plus a new `compare` command (the measure of the golden-image tests, for C#-vs-Rust captures).

## Summary

### Parity

| Check | Result here (Linux, llvmpipe) | Owner's machine (Windows, RTX 3060, b6bb4e7) |
|---|---|---|
| `rust/check.sh` (fmt, clippy pedantic `-D warnings`, `cargo test --workspace`) | clean, 0 failures | fmt/clippy clean, 1264 passed with `--include-ignored` |
| Core goldens, `ppu_golden_tests -- --ignored` (1200 frames) | 291/291 rows | to rerun (section 2 of LOCAL-CHECKS.md) |
| Core goldens, `zax-harness golden-core` vs `core-goldens.jsonl` | 291/291 rows equal (parsed JSON; 290 ROMs run, 1 load failure row) | to rerun |
| Render synthetic (11 scenes, counts identical, pixels within 2 levels) | 9/11 at 100 %; `framing` 99.998 %, `heavy` 99.769 % (llvmpipe z-fighting of coplanar sprites) | 11/11 at 100 % |
| Lua API dump (`lua-api` vs `goldens/lua-api.txt`) | byte-identical | byte-identical (test) |
| MSAA 8x | not verifiable (llvmpipe caps `GL_MAX_SAMPLES` at 4; 1 ignored test) | passes |

### Tests per crate (Rust, default run, this container) against the C# test projects

Rust counts are `#[test]` results (rstest rows and table-driven loops count as one or many as ported); C# counts are `[Fact]`/`[Theory]` methods, with `[InlineData]` rows in brackets (grep, not a test run).

| Rust crate | passed (+ ignored) | C# project | `[Fact]`/`[Theory]` methods [InlineData rows] |
|---|---|---|---|
| zax-nes | 383 (+5) | Zax.Nes.Tests | 349 [354] |
| zax-settings | 177 | Zax.Settings.Tests | 90 [106] |
| zax-profiles | 157 (+54, need `ZAXNES_PROFILES`) | Zax.Profiles.Tests | 124 [63] |
| zax-scripting | 118 (+7) | Zax.Scripting.Tests | 44 |
| zax-engine3d | 191 (+6) | Zax.Engine3D.Tests | 181 [13] |
| zax-render | 124 (+1) | Zax.Render.Tests | 67 [37] |
| zax-platform | 30 | (none; `Zax.Platform` is covered by the app and acceptance tests) | - |
| zax-app | 386 | Zax.App.Tests + Acceptance | 328 [89] + 581 [262] |
| zax-harness | 32 | (harness has no C# tests) | - |
| zax-goldens | 3 | - | - |

Fixes after Windows check 3: render.json string escaping; --ui-shots ignores host cursor.

The acceptance project (581 methods) checks requirement traceability and architecture rules of the C# solution; its Rust counterparts are the integration tests of `zax-app` and the golden comparisons above, not a method-for-method port.

### To check on a real machine (commands in `rust/LOCAL-CHECKS.md`)

- Window, GL context (desktop 3.3 core and GLES 3.0), vsync/pacing, fullscreen, icon, single-instance lock.
- Audio output (no crackle, pause, fast-forward), keyboard, gamepad (hot-plug, calibration).
- GPU captures: the synthetic goldens on the real GPU, 8x MSAA, GLES 3.0 against GL 3.3.
- Commercial ROMs and profiles through `ZAXNES_ROMS` / `ZAXNES_PROFILES` / `ZAXNES_3DSEN_INSTALL` (the ignored tests: profile corpus byte-identity 101/101, scripting and engine corpora, 3dSen import).
- The C#-vs-Rust capture campaign: 12 games x 3 angles with `render --rom`, compared by `zax-harness compare` (tolerance 2 per channel).
- `render --bench` below 4 ms median at 1920x1080 (not measurable on llvmpipe; here framing takes about 15 ms at 640x480 in software).
- Windows console attach at runtime (terminal, pipe and Explorer launches), packaging (`package-windows.ps1`, `package-linux-aarch64.sh`: neither has been run), the RG40XXV.

### Known gaps

- Lua: `CoreScriptInput`/`CoreScriptMemory` are implemented by the app, not by `zax-scripting`; `SlotTexture` w/h uses the 8W x 8H fallback; `BuildPattern` only has the instance (state members raise "not available here"); `Camera.FpvShape` reads nil after its frame epoch changes; ViewReset goes through a flag; the C# "Disposed runtime" test is not portable; `Setting:Save/Load` from Lua run at the end of the hook.
- Application: core error to `Stopped` not ported (no exception path); `REQ-PUB-006` system-font half not ported; `ISceneRenderer` interface not ported.
- Render: llvmpipe differences on overlapping coplanar sprites (heavy scene, framing edge pixels); 8x MSAA unverifiable here.
- Platform: Windows console attach and every window, audio and gamepad path were never run against real hardware by the agent; the aarch64 package and the RG40XXV are untested on a device; `zax-app`/`zax-platform` cannot be cross-checked for Windows from Linux (MSVC tools).
- Build: `--all-features` on `zax-app`/`zax-platform` enables `bundled` (static SDL2, needs CMake and MSVC), otherwise ship SDL2.dll 2.30.x.
