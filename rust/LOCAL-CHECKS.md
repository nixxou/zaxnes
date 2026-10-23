# Local checks (what only a real machine can confirm)

Run from the repository root unless a block says otherwise. `rust/` holds the Rust workspace, the
C# solution is at the root. Blocks are given for PowerShell (Windows) then for a POSIX shell
(Linux); the same environment variables work in both.

## 0. Environment

| Variable | Points at | Used by |
|---|---|---|
| `ZAXNES_ROMS` | folder of commercial `.nes` files (the ones the profiles are for) | ROM-gated tests, the capture campaign below |
| `ZAXNES_PROFILES` | folder that holds `profiles/` (the 101 built-in `.zaxprofile` files) | profile, scripting and engine corpus tests (about 90 `#[ignore]`d tests) |
| `ZAXNES_3DSEN_INSTALL` | a 3dSen PC installation folder | the four 3dSen import tests |
| `ZAXNES_GLES=1` | (flag) | forces GLES 3.0 instead of desktop GL 3.3 in `zax-platform` |

```powershell
$env:ZAXNES_ROMS = 'D:\roms'
$env:ZAXNES_PROFILES = 'D:\zaxnes-data'
$env:ZAXNES_3DSEN_INSTALL = 'C:\Program Files (x86)\Steam\steamapps\common\3dSen PC'
```

```sh
export ZAXNES_ROMS=$HOME/roms ZAXNES_PROFILES=$HOME/zaxnes-data
export ZAXNES_3DSEN_INSTALL=$HOME/3dSen
```

## 1. Whole workspace, ignored tests included

Needs the variables of section 0 (the ignored tests that read them fail without).

```powershell
cd rust
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-features -- --include-ignored
```

`--all-features` turns on `bundled` of `zax-app` and `zax-platform`: SDL2 is built and linked
statically, which needs CMake and the MSVC toolchain on Windows (a C compiler and CMake on
Linux). Without it, run `cargo test --workspace -- --include-ignored` and put SDL2.dll 2.30.x on
`PATH` (Windows) or install `libsdl2-dev` (Linux). The last owner run (Windows, RTX 3060) passed
1264 tests this way. On Linux without a GPU, Mesa llvmpipe works for the GPU tests; 8x MSAA
(`zax-render`, one ignored test) needs a real GPU.

```sh
cd rust && cargo test --workspace -- --include-ignored
```

## 2. Core goldens (291 public test ROMs, 1200 frames)

```powershell
cd rust
cargo test -p zax-nes --release --test ppu_golden_tests -- --ignored
```

Takes about 3 minutes in release. Side by side with the C# harness (the Rust output is compared
to the same golden file as the C# one, `rust/goldens/core-goldens.jsonl`):

```powershell
dotnet run -c Release --project tools/Zax.Harness -- golden-core rust\test-roms $env:TEMP\core-cs.jsonl
cargo run -q --release -p zax-harness -- golden-core test-roms $env:TEMP\core-rs.jsonl   # in rust\
```

The harness tests (`cargo test -p zax-harness`) compare a sample of rows by parsed JSON; the full
set is the two commands above compared with `rust/goldens/core-goldens.jsonl`.

## 3. Lua API dump

```powershell
cargo run -q -p zax-harness -- lua-api --out $env:TEMP\lua-rs.txt          # in rust\
dotnet run --project tools/Zax.Harness -- lua-api --out $env:TEMP\lua-cs.txt
fc.exe /b $env:TEMP\lua-rs.txt rust\goldens\lua-api.txt
```

```sh
cmp /tmp/lua-rs.txt rust/goldens/lua-api.txt && echo identical
```

## 4. Synthetic render goldens (11 scenes)

```powershell
cd rust
cargo test -p zax-harness --test render_tests -- --nocapture      # counts identical, >= 99.5 % within 2 levels
cargo run -q --release -p zax-harness -- render --out $env:TEMP\synthetic
cargo run -q --release -p zax-harness -- compare goldens\render-synthetic $env:TEMP\synthetic --tolerance 2 --min 99.5
```

Expected: 11/11 scenes at 100 % on a real GPU (llvmpipe: 9/11 at 100 %, `framing` 99.998 %,
`heavy` 99.769 %). The C# side writes the same files with
`dotnet run -c Release --project tools/Zax.Harness -- render --out <dir>`.

## 5. Capture campaign, C# against Rust (12 games x 3 angles)

`render --rom` runs a commercial ROM headless, then draws the frame offscreen with the profile
of the game. The two harnesses take the same options; the loop below writes one folder per
engine and compares them. Edit `$games` (ROM file, profile file name in `$env:ZAXNES_PROFILES\profiles`, `--run`
frames, optional `--press` script) for your twelve games and `$angles` for the three camera angles.

```powershell
$games = @(
  @{ rom = 'game01.nes'; profile = 'Game 01.bytes'; run = 600; press = '120:start' }
  # ... eleven more
)
$angles = @('0.81,0.81', '0.4,-0.6', '1.2,0.3')
$rust = 'rust\target\release\zax-harness.exe'
cargo build --release -p zax-harness --manifest-path rust\Cargo.toml
dotnet build -c Release tools\Zax.Harness
foreach ($g in $games) {
  foreach ($a in $angles) {
    $name = '{0}_{1}' -f [IO.Path]::GetFileNameWithoutExtension($g.rom), ($a -replace ',', '_')
    $common = @('--rom', (Join-Path $env:ZAXNES_ROMS $g.rom), '--run', $g.run, '--angles', $a,
                '--size', '1280x720',
                '--profile', (Join-Path $env:ZAXNES_PROFILES "profiles\$($g.profile)"))
    if ($g.press) { $common += @('--press', $g.press) }
    & $rust render --out "captures\rust\$name" @common
    dotnet run -c Release --no-build --project tools/Zax.Harness -- render --out "captures\cs\$name" @common
  }
}
```

Each capture folder holds `<scene>.png` and `render.json`. Compare every folder, tolerance 2
per channel, at least 99.5 % of the pixels (the same measure as the synthetic goldens):

```powershell
Get-ChildItem captures\cs -Directory | ForEach-Object {
  & $rust compare $_.FullName "captures\rust\$($_.Name)" --tolerance 2 --min 99.5
}
```

```sh
for d in captures/cs/*/; do n=$(basename "$d"); rust/target/release/zax-harness compare "$d" "captures/rust/$n"; done
```

`compare` exits non-zero on a missing file, a size difference or a share under `--min`. Also
diff the counts of `render.json` (tiles, draw calls, triangles) between the two folders: they
must be identical.

## 6. Frame time at 1080p (target < 4 ms)

```powershell
cd rust
cargo run -q --release -p zax-harness -- render --out $env:TEMP\bench --size 1920x1080 --bench 300
cargo run -q --release -p zax-harness -- render --out $env:TEMP\bench-rom --rom $env:ZAXNES_ROMS\game01.nes --profile "$env:ZAXNES_PROFILES\profiles\Game 01.bytes" --run 600 --size 1920x1080 --bench 300
```

The last column of each scene line is `<median> ms median, <p99> ms p99`; `render.json` carries
`medianFrameMs` and `p99FrameMs`. Add `--explain` to print the camera and the draw list grouped
by 3D object. The C# command takes the same options.

## 7. The application

```powershell
cd rust
cargo build --release -p zax-app --features bundled
target\release\zaxnes.exe --help
target\release\zaxnes.exe --list-profiles
target\release\zaxnes.exe --rom $env:ZAXNES_ROMS\game01.nes --profile "$env:ZAXNES_PROFILES\profiles\Game 01.bytes"
```

```sh
cd rust && cargo build --release -p zax-app
target/release/zaxnes --help
```

By hand: window opens at the right size and in fullscreen; video is paced to the display
(no tearing, no stutter); audio has no crackle and follows pause and fast-forward; keyboard and a
gamepad (hot-plug, calibration screen) drive the game; the menus and the `--ui-shots <dir>`
screens look as in the C# build; a second launch on the same user folder closes at once. On
Windows the binary is a GUI program: double-click opens no console, and from a terminal
`--help` / `--list-profiles` / a usage error print to that terminal (both to check, the attach
is not exercised by any test). Redirecting (`zaxnes.exe --help > out.txt`) must write to the
file, not the terminal.

## 7b. Linux smoke test (automatic)

`rust/build/smoke-linux.sh` runs the release `zaxnes` under Xvfb + openbox with xdotool: help and
profile list, welcome screen, a public ROM drawing and moving, pause, save/load state, reset,
fast forward and rewind keys, options window, French and back, the 3D default geometry, audio not
silent (SDL disk driver), user-data files, exits with code 0 from Ctrl+Q and from a window close.
Needs `apt install xvfb openbox xdotool imagemagick wmctrl`; exits non-zero on the first failure.

## 7c. Language and native file dialog (Windows, cannot be run on Linux)

Language (Rust-only change, amends AMD-035):

- Delete the user data folder (or start with `--user-data-root <empty folder>`). On a Windows whose display language is French, the first launch is in French; on an English or any other
  language it is in English.
- Pick the other language in Options and restart: the pick wins whatever the Windows language is.
  Choose English on a French Windows, restart: still English (`settings.json` holds `ui.language`).
- Remove `ui.language` from `settings.json` by hand: the Windows language applies again.

Native dialog (Rust-only change; `ZAXNES_NATIVE_DIALOG=0` forces the `ImGui` browser):

- File > Open a ROM opens the Windows Explorer dialog with "NES ROMs", ".nes", ".zip" and "All
  files" filters, starting in the last folder used; the game keeps rendering and the window stays
  responsive behind it. Cancel does nothing; choosing a ROM starts it.
- In full screen the window leaves full screen while the dialog is open (the dialog is never
  hidden behind it) and returns to full screen when it closes; the vsync setting is intact.
- Import a profile (`.3dn`), Import a cover (`.png`) and Import from 3dSen (a folder picker) use the same dialog.
- With `ZAXNES_NATIVE_DIALOG=0` the same menu entries open the `ImGui` browser as before.
- Linux desktop (x86_64): the XDG portal dialog; with no portal or no session bus, or on aarch64
  (RG40XXV), the `ImGui` browser opens (`ZAXNES_NATIVE_DIALOG=1` forces the portal on aarch64).

## 8. Packaging

```powershell
powershell -ExecutionPolicy Bypass -File rust\build\package-windows.ps1     # zip in rust\target\package
```

```sh
rust/build/package-linux-aarch64.sh            # cross build on x86_64 Linux (gcc-aarch64-linux-gnu)
rust/build/package-linux-aarch64.sh --native   # on the device itself
```

Standalone exe (`+crt-static`, `[profile.release]`): after the build, from a Visual Studio prompt

```powershell
dumpbin /dependents target\release\zaxnes.exe
```

must list only system DLLs (KERNEL32, USER32, GDI32, SHELL32, OPENGL32, ...) and none of
`VCRUNTIME140.dll`, `VCRUNTIME140_1.dll`, `MSVCP140.dll`, `api-ms-win-crt-*.dll`, nor `SDL2.dll`.
If one shows up, a C dependency (SDL2 through `cmake`, Lua or Dear ImGui through `cc`) was built
with /MD: look for the flag in its build log. Then start the exe on a clean Windows (a fresh VM
without the Visual C++ Redistributable): it must open. The exe is about 5 MB (7.7 MB before the
release profile on Linux; check the Windows size too).

Unzip the Windows package on a machine without Rust or SDL2 and start `zaxnes.exe`. For the
RG40XXV, copy the aarch64 package to the device and follow `RG40XXV.txt` (GLES 3.0 is selected
automatically on aarch64 Linux; `ZAXNES_GLES=1` forces it elsewhere).

### 8b. Linux aarch64 (RG40XXV), cross build and parity under qemu

```sh
rustup target add aarch64-unknown-linux-gnu
sudo apt install gcc-aarch64-linux-gnu g++-aarch64-linux-gnu cmake qemu-user qemu-user-binfmt \
    libc6-arm64-cross libdrm-dev libgbm-dev libegl-dev libgles-dev
export CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc CXX_aarch64_unknown_linux_gnu=aarch64-linux-gnu-g++ \
       AR_aarch64_unknown_linux_gnu=aarch64-linux-gnu-ar SDL2_TOOLCHAIN=$PWD/rust/build/aarch64-sdl2.cmake
cd rust   # the linker comes from .cargo/config.toml
cargo build --release --target aarch64-unknown-linux-gnu -p zax-nes -p zax-harness
cargo build --release --target aarch64-unknown-linux-gnu -p zax-app --features bundled
Q="qemu-aarch64 -L /usr/aarch64-linux-gnu"
$Q target/aarch64-unknown-linux-gnu/release/zax-harness golden-core test-roms out.jsonl 60   # about 5 min
$Q target/aarch64-unknown-linux-gnu/release/zaxnes --help
CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUNNER="$Q" cargo test --release --target aarch64-unknown-linux-gnu -p zax-nes
```

Compare the JSONL with `goldens/core-goldens.jsonl` as parsed JSON, checkpoint by checkpoint (a 60
frame run only shares the frame 1 and 60 checkpoints). 1200 frames cost about 95 min per full run.
The bundled SDL2 is configured with KMSDRM, GBM, EGL and GLES2 (loaded by dlopen: the device needs
libdrm.so.2, libgbm.so.1, libEGL.so.1, libGLESv2.so.2); libsamplerate is off (no aarch64 library here).

Still to check on the device (nothing graphical or audible ran under qemu): the KMSDRM window
(`SDL_VIDEODRIVER=KMSDRM`), the GLES 3.0 context on the Mali-G31, the controls, the audio (ALSA)
and the performance (frame time at 640x480 on the Cortex-A53).
