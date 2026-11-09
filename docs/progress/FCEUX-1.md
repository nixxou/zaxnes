# FCEUX-1 — FCEUX as a second emulation engine: proof of concept and design

Lot FCEUX-1, role R3 (clean side). Branch `feat/fceux-core` in the worktree `wt-fceux`; fork of
FCEUX in `cleanroom/fceux-zax`, branch `zax`.

This lot is the **proof of concept and the design**, not the integration into the application. It
ends with the FCEUX core running under Zaxnes's harness in 2D, a first tile-capture prototype, and
a design note for the lots that would follow.

## Steps

- [x] 1/8 — Inventory (read only): the FCEUX core/driver boundary, its build system and its
  dependencies, its global-state model; and the public API of `zax-nes` that `zax-app` and
  `zax-harness` use.
- [ ] 2/8 — The fork and a core-only build: FCEUX's core as a library with no GUI, no Lua, no Qt,
  no SDL, for Windows x64 (MSVC) then Linux x64 (WSL).
- [ ] 3/8 — A minimal C API (`zax/zaxfceux.h` + `.cpp` in the fork).
- [ ] 4/8 — The Rust side: crate `zax-fceux` behind the Cargo feature `fceux-core` (off by
  default), plus a `zax-harness fceux-run` subcommand writing `golden-core`'s digests.
- [ ] 5/8 — Compare the engines over the 291 public test ROMs and the 98 originals.
- [ ] 6/8 — Tile-capture prototype in the fork (old PPU), fed to Zaxnes's 3D path.
- [ ] 7/8 — Design note `docs/review/FCEUX-ENGINE.md`.
- [ ] 8/8 — Final pass.

## Environment of the session

| Item | Value |
|---|---|
| Worktree | `cleanroom/wt-fceux`, branch `feat/fceux-core`, base commit `c0a97023` |
| `CARGO_TARGET_DIR` | `<worktree>/rust/target-fx` |
| `TMP` / `TEMP` | `<worktree>/rust/target-fx/tmp` |
| C/C++ build trees | `cleanroom/fceux-zax/build-*` |
| Free space on `G:` at the start | 22 GB (floor: 5 GB) |

Toolchains found, none downloaded:

| Tool | Where | Version |
|---|---|---|
| CMake | VS 2022 Community, `Common7/IDE/CommonExtensions/Microsoft/CMake/CMake/bin` | 3.30.5-msvc23 |
| Ninja | same, `.../CMake/Ninja` | shipped with VS |
| MSVC | VS 2022 Community, `VC/Tools/MSVC/14.43.34808` | 14.43, x64 via `vcvars64.bat` |
| g++ (Linux) | WSL `Ubuntu-24.04` | 13.3.0 |
| CMake (Linux) | WSL `Ubuntu-24.04` | 3.28.3 |
| zlib headers (Linux) | WSL, `/usr/include/zlib.h` | system |

Untracked inputs put in place (both ignored, never staged — `rust/.gitignore:2`, `.gitignore:71`):

- `rust/crates/zax-app/demo/FlappyZaxNes.nes` and `flappyzaxnes.3dn`, copied from
  `3DSen/wt-rust/rust/crates/zax-app/demo/`.
- `rust/test-roms/` — **the directory did not exist in this worktree**. The 291 public ROMs the
  goldens were taken over are not in `assets-free/test-roms/` either (that folder holds
  `nes-test-roms/`, `holy-mapperel/` and `AccuracyCoin/`, 331 files, under a different layout).
  They were copied from `3dsen/wt-rust/rust/test-roms/` (27 MB, 291 `.nes`), whose paths match the
  `rom` field of `rust/goldens/core-goldens.jsonl` (291 rows) exactly.

---

# Step 1 — Inventory

## 1.1 FCEUX: what is the core and what is a driver

Read-only in `cleanroom/third_party/fceux` at `ffc23f08` (branch `master`).

The single CMake target is an executable built from `SOURCES = SRC_CORE + SRC_DRIVERS_COMMON +
SRC_DRIVERS_SDL` (`src/CMakeLists.txt:725`), and `SRC_DRIVERS_SDL` is in fact the **Qt** driver
(`src/CMakeLists.txt:690-720`: `ConsoleWindow`, `TasEditor/`, `sdl-sound.cpp`, `AviRecord.cpp`…).
There is no library target and no core-only build upstream: `src/drivers/libretro/` holds only a
prebuilt `fceux_libretro.so` with no sources, so the boundary has to be drawn by hand.

### The emulation core

| Area | Files | Notes |
|---|---|---|
| CPU | `src/x6502.cpp`, `x6502.h`, `ops.inc` | 6502 with the cycle-accurate `X6502_Run` |
| PPU (both) | `src/ppu.cpp` (2527 lines), `ppu.h`, `pputile.inc` | **old** PPU and **new** PPU in one file, switched by the global `newppu` (`ppu.cpp:559`); `FCEUPPU_Loop` branches at `ppu.cpp:1771-1772` |
| APU | `src/sound.cpp`, `sound.h`, `filter.cpp`, `fcoeffs.h`, `fir/` | |
| Cartridge / boards | `src/cart.cpp`, `ines.cpp`, `unif.cpp`, `fds.cpp`, `nsf.cpp`, `vsuni.cpp`, `src/boards/` (~190 files + `mapinc.h`, `emu2413.c`) | the ~270 mappers; `ines-correct.h`, `ines-bad.h` are the ROM database |
| Machine | `src/fceu.cpp` | `FCEUI_Emulate` (`fceu.cpp:729`), `FCEUI_LoadGameVirtual` (`:414`), `FCEUI_Initialize` (`:607`), `ResetNES` (`:903`), `PowerNES` (`:1000`) |
| Save-states | `src/state.cpp`, `state.h` | `FCEUSS_SaveMS(EMUFILE*, int compressionLevel)` (`state.h:37`), `FCEUSS_LoadFP(EMUFILE*, ENUM_SSLOADPARAMS)` (`state.h:39`) — both take a stream, so a caller buffer works without touching the filesystem |
| Movies | `src/movie.cpp`, `movie.h`, `oldmovie.cpp` | `.fm2` playback and recording; reads `RAMInitOption`/`RAMInitSeed` (`movie.cpp:47-48, 432-434`) |
| Video buffer | `src/video.cpp`, `video.h` | owns `XBuf`/`XDBuf`, see §1.3 |
| Input core | `src/input.cpp`, `input.h`, `src/input/` (22 devices) | the core side of the ports; the *drivers* only feed it bytes |
| Palette | `src/palette.cpp`, `src/palettes/` | |
| Cheats / Game Genie | `src/cheat.cpp` | `FCEU_ApplyPeriodicCheats` is called from `FCEUI_Emulate` (`fceu.cpp:828`), so `cheat.cpp` is **not** optional |
| Utilities | `src/utils/{endian,memory,md5,crc32,general,xstring,guid,ioapi,unzip,mutex,ConvertUTF,timeStamp}` | `unzip.cpp`+`ioapi.cpp` are an in-tree minizip; `backward.cpp` is a crash handler, not needed |
| Streams | `src/emufile.cpp`, `emufile.h`, `emufile_types.h` | `EMUFILE_MEMORY` is what makes ROM-from-memory and state-to-buffer possible |
| Files | `src/file.cpp`, `file.h` | `FCEU_fopen` and the archive support — the one core file that genuinely wants minizip |

Pulled in by the core but cuttable with a stub, not real emulation:
`debug.cpp`, `debugsymboltable.cpp`, `conddebug.cpp`, `asm.cpp`, `ld65dbg.cpp`, `profiler.cpp`,
`drawing.cpp` (on-screen messages), `wave.cpp` (WAV recording), `netplay.cpp`, `config.cpp`.

### The drivers (excluded)

`src/drivers/Qt/` (the whole GUI, the debugger windows, TasEditor, AVI, netplay),
`src/drivers/win/` (the Windows UI — but see §1.4: its `zlib/` subdirectory is the vendored zlib),
`src/drivers/sdl/`, `src/drivers/common/` (`vidblit`, `hq2x/3x`, `scale2x/3x`, `nes_ntsc` — host
scalers), `src/drivers/videolog/`, `src/lua/` + `src/lua-engine.cpp`, `src/attic/`.

The contract between the two sides is `src/driver.h`: every `FCEUD_*` symbol there is **supplied by
the driver** and must be stubbed by us (`FCEUD_PrintError:34`, `FCEUD_SetPalette:30`,
`FCEUD_UTF8fopen:12`, `FCEUD_OpenArchive:18`, `FCEUD_TraceInstruction:339`,
`FCEUD_UpdatePPUView:348`, `FCEUD_FlushTrace:342` — called once a frame from `fceu.cpp:834` —
`FCEUD_SendData`/`FCEUD_RecvData:48-49`, `FCEUD_SetInput:181`, …), while every `FCEUI_*` symbol is
provided by the core and is what we may call.

### What the core alone needs

- **zlib** — unavoidable in three places: `state.cpp:39,458,709` (`compress2`/`uncompress` for
  savestates), `file.cpp:38,331-340` (`gzopen` for `.gz` ROMs), `video.cpp:58` (PNG screenshots).
  A vendored copy exists in-tree at `src/drivers/win/zlib/` (zlib **1.1.4**, with `compress2` at
  `zlib/zlib.h:623` and `uncompress` at `:638`), and `src/CMakeLists.txt:352-377` already wires it
  as the "internal zlib" fallback. **No download needed.** On Linux the system zlib is present.
- **minizip** — only `file.cpp:192-249` (`unzOpen` … `unzClose`), for ROMs inside a `.zip`. The
  in-tree `src/utils/unzip.cpp` + `ioapi.cpp` are a copy of it, so this too needs no download; but
  since the C API loads a ROM **from memory**, archive support is dead weight and will be cut with
  a guard rather than compiled (decided at step 2).
- Nothing else: no Qt, no SDL, no Lua (the `_S9XLUA_H` blocks in `fceu.cpp:817-838` vanish when the
  macro is undefined), no libarchive (`vcpkg.json` lists it for the Qt driver only), no x264/libav.

### The global-state model, and what it implies

FCEUX keeps the whole machine in **file-scope globals**, not in an instance: `FCEUGI *GameInfo`
(`fceu.cpp:235`), `uint8 *RAM` (`fceu.cpp:355`), `uint8 PPU[4]`, `NTARAM[0x800]`, `*vnapage[4]`
(`ppu.h:16-18`), `int32 Wave[]`, `WaveHi[]`, `WaveFinal[]` (`sound.cpp:38-40`), `u8 *XBuf`,
`*XDBuf` (`video.cpp:65-67`), `int newppu` (`ppu.cpp:559`), `int RAMInitSeed`, `RAMInitOption`
(`fceu.cpp:919-920`), `int EmulationPaused` (`fceu.cpp:249`), plus the per-board statics in each of
`src/boards/*.cpp`. `RefreshLine` even keeps the background shift pipeline in function-level
`static` variables (`ppu.cpp:1095-1096`).

Consequences, all of which the C API must enforce rather than hope for:

1. **One live instance per process.** `create` must refuse a second one while the first exists.
2. **No two threads in the core at once.** Zaxnes drives its own core on a worker thread by moving
   a `Box<NesCore>` into it (`rust/crates/zax-app/src/core_worker.rs:16,83`); that pattern still
   works for FCEUX (the handle may *move* between threads) but the handle must not be `Sync`, and
   run-ahead cannot run two engines side by side.
3. **No two games at once**, so no "compare both engines live in one process" — the comparison of
   step 5 runs FCEUX and Zaxnes in separate processes or sequentially.
4. A reset of state between ROMs is `FCEUI_CloseGame` + load, not a fresh object; anything the
   boards leak stays leaked.

## 1.2 Where the old PPU fetches tiles and sprites (for step 6)

- Background: `static void RefreshLine(int lastpixel)` (`ppu.cpp:1094`) renders tiles
  `firsttile..lasttile` of the current `scanline` and includes `pputile.inc` once per tile, in ten
  places, one per MMC5/hook/PEC586/VRC5 variant (`ppu.cpp:1171,1174,1185,1192,1197,1208,1213,1222,1228,1233`).
  `pputile.inc` is therefore the single capture point. In it, per tile: `vadr` the pattern address
  (`pputile.inc:52`), `C[0]` and `C[8]` the two bit planes (`:132,135`), `cc` the attribute bits
  (`:66-67`), `RefreshAddr` the nametable address, `vofs` the fine Y, `X1` the tile column, and the
  global `scanline`. Note the pipeline: the pixels written at iteration `X1` come from the tiles
  fetched two iterations earlier (`pputile.inc:17-41`, guarded by `X1 >= 2`), and `XOffset` is the
  fine X — so the screen position of a record has to be derived, not read.
- Sprites: `static void FetchSpriteData(void)` (`ppu.cpp:1420`), twice over (no-hook loop at
  `:1437`, `PPU_hook` loop at `:1490`). There, per sprite: the OAM index `n` (63 down to 0, so
  priority order is the reverse), `spr->y`, `spr->x`, `spr->atr` (sub-palette in bits 0-1, H flip
  `V_FLIP`/bit 6, V flip bit 7, "behind background" bit 5), `vadr`, and `C[0]`/`C[8]`.
  **`SPRB`, what FCEUX keeps afterwards, drops the OAM index and `y`** (`:1473-1480`), so the hook
  has to sit inside `FetchSpriteData` and not downstream.
- `RefreshSprites` (`ppu.cpp:1559`) composes them into `sprlinebuf`; `DoLine` (`:1314`) is the
  per-line driver. The old PPU works **per line**, so a tile crossing a mid-line CHR switch is
  fetched with whatever bank the line ended on — one of the approximations step 6 has to record.

## 1.3 The core's frame and sound buffers

- `XBuf` is `256 * 256` bytes, allocated in `FCEU_InitVirtualVideo` (`video.cpp:127`), stride 256,
  240 visible rows. Its value encoding is documented at `video.cpp:60-64`: indices 0-63 are
  reserved for FCEUX's own overlay colours, 64-127 is "the most-used emphasis setting per frame",
  128-195 no emphasis, 196-255 all emphasis bits on. The plain NES colour index is therefore
  `XBuf[i] & 0x3F`, and the **per-pixel emphasis bits live in the parallel `XDBuf`**
  (`video.cpp:67,127`, written at `ppu.cpp:1324` and `:2228`) — which is what the C API will expose
  beside the indices.
- Sound: `FCEUI_Emulate` hands back `WaveFinal` and a sample count from `FlushEmulateSound`
  (`fceu.cpp:831,871-872`; `sound.cpp:40,1036,1107`). Samples are `int32`, mono, at the rate given
  to `FCEUI_Sound(int Rate)` (`driver.h:154`, only 44100/48000/96000 supported, 0 = silent).

## 1.4 Zaxnes: the core API that `zax-app` and `zax-harness` use

`rust/crates/zax-nes` is `#![forbid(unsafe_code)]` (`src/lib.rs:1`); the public surface is
`NesCore` (`src/nes_core.rs:47`, 1629 lines) plus the types re-exported at `src/lib.rs:30-36`.

| What | Signature | Where |
|---|---|---|
| Load | `load_rom(&mut self, image: &[u8], name: &str) -> RomLoadResult` | `nes_core.rs:346` |
| Close | `close_rom(&mut self)` | `:421` |
| Power / reset | `reset(&mut self)`, `soft_reset(&mut self)` | `:465`, `:480` |
| Run a frame | `run_frame(&mut self, input_word: u16) -> i64` (CPU cycles) | `:694` |
| Run-ahead | `run_frame_ahead(&mut self, input_word: u16, ahead: u8) -> i64`, `restore_real_frame(&mut self) -> bool` | `:774`, `:822` |
| Pixel frame | `pixel_frame_with_mask(&self) -> &[u8]` (index in bits 0-5, opacity in bit 6), `pixel_frame_indices_into(&self, &mut Vec<u8>)` | `:1070`, `:1075` |
| Widened picture | `wide_picture_into(&self, &mut Vec<u8>) -> usize` | `:1083` |
| Tile records | `tile_records(&self) -> &[TileRecord]`, `frame_batch(&self) -> FrameBatch<'_>` | `:1097`, `:913` |
| Video callback | `set_video_callback(&mut self, Option<VideoCallback>)` — invoked once per frame, synchronously | `:334` |
| Audio | `read_audio(&self, destination: &mut [f32], stereo: bool) -> Result<usize, AudioError>`, `set_sample_rate(&mut self, i32)`, `audio_reader()` | `:670`, `:633`, `:676` |
| Save-states | `save_state(&self, include_rom_image: bool) -> Result<Vec<u8>, NoRomLoaded>`, `save_state_to(…, &mut Vec<u8>)`, `try_load_state(…)`, `max_state_size()` | `:1228`, `:1311`, `:1485`, `:1352` |
| Rollback | `capture_core_state(&self, &mut CoreState)`, `restore_core_state(&mut self, &CoreState)`, `max_core_state_size()` | `:1401`, `:1432`, `:1379` |
| Memory (Lua, achievements) | `read_memory(&self, address: u16) -> u8`, `write_memory(&mut self, u16, u8)` | `:1140`, `:1145` |
| Region | `set_region(&mut self, NesRegion)`; detected (not applied) by `load_rom` | `:606` |
| Input ports | `port_type(&self, usize)`, `set_port_type(&mut self, usize, ControllerPortType)` | `:1110`, `:1118` |
| Counters | `cycle_count(&self) -> i64`, `set_frame_count(&mut self, i64)` | `:1134`, `:1102` |
| Zaxnes-only | `set_widescreen_margin`, `set_widescreen_vertical_margin`, `set_hack_options`, `hack_options`, `options_patch`, `original_image`, `set_sprite_limit`, `bus_mut`, `cartridge_mut`, `vs_system` | `:933`, `:1053`, `:945`, `:1035`, `:1028`, `:221`, `:939`, `:1191`, `:1208`, `:448` |

**The input word** (`run_frame`'s argument): low byte = port 0, high byte = port 1
(`input/mod.rs:230`), and inside a byte the eight buttons are *A, B, Select, Start, Up, Down, Left,
Right* from bit 0 to bit 7 (`input/mod.rs:225-226`, REQ-CORE-022).

**The tile record** (`src/tile_record.rs:15`, REQ-CORE-033/034/035, SPEC-02 §4.1) — the exact
layout FCEUX's capture must produce:

| Field | Type | Meaning |
|---|---|---|
| `plane0`, `plane1` | `u64` | the 16 CHR bytes **without flips applied**; row `r` (0 = top) in bits `8r..8r+7`, bit 7 of that byte = column 0. `u64::from_le_bytes(rows)` (`tile_record.rs:110`) |
| `x`, `y` | `i16` | screen position of the top-left pixel; background `-7..255` / `-1..238`, sprite `0..255` / `1..239` |
| `index` | `i32` | background: REQ-CORE-040; sprite: 0..127 (REQ-CORE-043) |
| `mirror_h`, `mirror_v` | `bool` | OAM attribute bits 6 and 7; **always false for background** |
| `hidden` | `bool` | OAM attribute bit 5, "behind background"; false for background |
| `palette` | `u8` | sub-palette 0..3 |
| `background` | `bool` | true for a background tile |

Order matters: `FrameBatch::records` is "chronological by capture line, and for one line the
sprites before the background row" (`frame_batch.rs:17-19`, REQ-CORE-044). The batch also carries
`palette: [u8; 32]` (PPU palette RAM at end of frame, REQ-CORE-045), `mask_x`/`mask_y`
(REQ-CORE-046), and the widescreen fields `margin`, `margin_y`, `visible_rows`, `margin_palette`
— all four of which are Zaxnes-only and stay empty for FCEUX.

**The harness side.** `zax-harness golden-core <roms dir> <out.jsonl> [frames]`
(`crates/zax-harness/src/main.rs:29`, `golden_core.rs:45`) writes one JSON object a ROM with:
`rom`, `rom_sha256`, `mapper`, `load_ok`, `region`, `sample_rate`, `frames`, and at the checkpoints
`[1, 60, 300, 600, 1200, 1800, 3600]` (`golden_core.rs:29`) the keys `pixel_sha256` (SHA-256 of the
240×256 index image), `tiles_sha256` (`"<count>:<hex>"` over every field of every record in order,
`golden_core.rs:239-255`), `cycles`, `audio_i16_sha256_samples` (`"<hex>:<count>"`, samples
quantised to `i16` by `quantise`, `:233`); then `blargg_status` and `blargg_text` when `$6001-$6003
== DE B0 61` (`:149-161`). `fceux-run` (step 4) has to produce the same shape so the two engines
compare key by key.

`zax-app` drives the core through `Box<NesCore>` handed to a worker thread
(`core_worker.rs:16,26-29,83,99`) and otherwise calls it from `game_session.rs` (1629 lines),
`emulation/host/{jobs,save_states}.rs`, `achievements/mod.rs`, `input/routing.rs`,
`frame_profiler.rs` — the methods used being `frame_count`, `wide_picture_into`,
`run_frame_ahead`, `port_type`/`set_port_type`, `save_state_to`, `original_image`, `frame_batch`,
`export_battery_ram`, `read_memory`/`write_memory`, `reset`/`soft_reset`, `restore_real_frame`,
`hack_options`, `options_patch`, `load_rom`, `cartridge`, `bus`, `ppu`. That list is the shape of
the trait step 7 has to propose.

## 1.5 Consequences carried into the next steps

1. The core-only library is `SRC_CORE` minus `lua-engine.cpp`, plus the vendored zlib, with
   `src/driver.h`'s `FCEUD_*` stubbed and `_S9XLUA_H`, `__QT_DRIVER__`, `__WIN_DRIVER__` all
   undefined.
2. ROM loading from memory goes through `FCEUFILE::SetStream(new EMUFILE_MEMORY(...))`
   (`file.h:71`) and `iNESLoad(name, fp, OverwriteVidMode)` (`fceu.cpp:485`) directly, bypassing
   `FCEU_fopen` and therefore archives and the filesystem entirely.
3. Save-state to and from a caller buffer goes through `FCEUSS_SaveMS`/`FCEUSS_LoadFP` over an
   `EMUFILE_MEMORY`.
4. The frame is `XBuf` (indices, `& 0x3F`) plus `XDBuf` (emphasis), both `256 * 256` with 240 rows
   used.
5. The capture hook of step 6 goes in `pputile.inc` (background) and inside the two loops of
   `FetchSpriteData` (sprites), because those are the only points where the OAM index, `y` and both
   bit planes are all still in scope.
6. The one-instance rule is a hard constraint of the design, not a simplification: `create` must
   enforce it and the Rust handle must not be `Sync`.
