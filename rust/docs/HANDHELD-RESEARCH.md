# Handheld distribution and UI research (Anbernic H700 / RK3566)

Scope: how standalone apps and emulators are shipped for low-end Anbernic
handhelds (RG40XXV, RG35XX family, RG CubeXX, RG353...), how they are
installed and launched, and how their UI is adapted to a small gamepad-only
screen. Goal: decide what handheld-specific work the Zaxnes build needs.

Research date: 2026-10-23. Method: web search plus the PortMaster GitHub raw
files. Several sites (portmaster.games, muos.dev, knulli.org) were not directly
reachable from the research sandbox; their content was taken from the
PortMaster GitHub mirrors and from search-result excerpts. Items marked
**[UNCERTAIN]** are not confirmed from a primary source. Nothing here was tested
on a device.

Target: Allwinner H700 (4x Cortex-A53 @ ~1.5 GHz), Mali-G31 MP2, 1 GB RAM
(RG40XXV, RG35XX/28XX; the RG34XX family has 2 GB), 640x480. Variants: 720x480
(RG34XX), 720x720 (RG CubeXX). Rockchip RK3566 devices (RG353 etc.) have
Mali-G52 and are better served by the same ports.

---------------------------------------------------------------------------

## 1. Firmwares and how third-party apps are installed and launched

| Firmware | Base | H700 support | How a standalone app appears |
|---|---|---|---|
| Anbernic stock OS | Vendor Linux, 4.9 BSP kernel, libmali | all | Unofficial; users usually replace it. Stock "apps" menu possible but not a distribution channel for us. |
| muOS (MustardOS) | own frontend + RetroArch-first | RG35XX/Plus/H/SP/Pro, RG34XX, RG28XX, RG40XX H/V, CubeXX | "Applications" (.muxapp) or Ports via PortMaster |
| Knulli | Batocera fork, EmulationStation | RG35XX family, RG40XX, etc. (not every model listed, see note) | `.sh` files in the `ports` ROM folder, shown in ES |
| ROCKNIX (ex-JELOS) | mainline-ish, Panfrost | listed as supported since 20250115 (RG35XX 2024/Plus/H/SP, RG40XX V/H, CubeXX, RG34XX, RG28XX) | `.sh` in `/roms/ports`, ES (EmulationStation) |
| spruceOS | MinUI-derived | H700 Anbernic listed "Stable" in v4.4.0 | Ports via PortMaster (`Roms/ports`) |
| NextUI / MinUI family (pvaibhav/NextUI H700 builds, CrossMix-OS) | MinUI | H700 builds exist, pre-release at rc11 | "Paks" with `launch.sh`; PortMaster installable via Pak Store |

Notes
- PortMaster officially supports: ArkOS, AmberELEC, ROCKNIX, muOS, Knulli,
  dArkOS (full); UnofficialOS (partial). JELOS is discontinued in favour of
  ROCKNIX. Source: PortMaster FAQ (PortMaster-Website `docs/faq.html`).
- NextUI H700 builds changed button numbering for SDL apps (rc11: all H700
  models use TrimUI numbering; `export SDL_JOYSTICK_H700_FIXED_LAYOUT=0` in a
  pak's `launch.sh` restores old numbers). Lesson: use SDL GameController API
  (mapped names), not raw joystick button indices. Source: NextUI h700-rc11 release notes.

### Folder layout per firmware (from PortMaster docs/FAQ)

| Firmware | PortMaster install dir | Ports `.sh` + port folders | Autoinstall dir |
|---|---|---|---|
| ROCKNIX / AmberELEC / UnofficialOS | `/roms/ports/PortMaster` | `/roms/ports/` | `/roms/ports/PortMaster/autoinstall/` |
| muOS | `/mnt/mmc/MUOS/PortMaster` (control.txt falls back to `/$directory/ports/PortMaster`) | folders `/mnt/mmc/ports/`, launcher scripts `/mnt/mmc/ROMS/Ports/` (union mount `/mnt/union/ROMS/Ports`) | `/mmc/MUOS/PortMaster/autoinstall/` |
| Knulli | `/userdata/system/.local/share/PortMaster` | `/userdata/roms/ports/` | `/userdata/system/.local/share/PortMaster/autoinstall` |
| ArkOS | `/roms/tools/PortMaster` | `/roms/ports` | `/roms/tools/PortMaster/autoinstall/` |

Source: https://portmaster.games/faq.html and /installation.html (mirrored at
https://github.com/PortsMaster/PortMaster-Website/tree/main/docs). Zip contents
dropped by hand into the ports folders "may break the port" per the FAQ; the
supported route is PortMaster install (sets permissions and metadata).

### muOS native applications (.muxapp)

- A `.muxapp` is a zip containing exactly one top-level folder; the Archive
  Manager (files placed in an `ARCHIVE` folder at the root of SD1/SD2/USB)
  extracts it to `MUOS/application/<AppName>/`. Archives extract relative to
  `/`, so the archive must contain the full destination path
  (handler chosen by the top-level folder name). Sources:
  https://muos.dev/help/archive , https://community.muos.dev/t/adding-new-custom-archive-path-support-for-muxzip/1014
  (**[UNCERTAIN]** exact internal layout across releases; the format changed
  around 2601 "Jacaranda" and older packages needed updating).
- Entry point is `mux_launch.sh` inside the app folder; muOS wraps apps for CPU
  governor, SDL environment and `HOME`. Newer firmware: `SETUP_APP`; older:
  `act_go` + `SETUP_SDL_ENVIRONMENT`. Optional header comments `ICON`, `GRID`,
  `HELP`. Source: https://community.muos.dev/t/application-runner/1282 (FAQ for
  2601.0 Jacaranda / 2606.0 Andromeda).
- Real example (Super Mario War Rust port): native aarch64 binary needing
  GLIBC_2.34 (built on ubuntu-22.04-arm), uses the device's own libSDL2,
  `RUNPATH=$ORIGIN/lib`, `HOME` set to a folder inside the app, Menu button
  quits, launcher restores framebuffer mode on exit.
  Source: https://github.com/KyleJamesWalker/supermariowar-rust/pull/9
- Ports (PortMaster) are the other muOS route; PortMaster zips work unchanged.

### Knulli

- Batocera-derived; games live under `/userdata/roms/<system>/` (share
  partition). A `.sh` placed in `roms/ports/` appears in the Ports system;
  "Update gamelist" refreshes ES. Knulli wiki: https://knulli.org/play/add-games/game-storage/ ,
  https://knulli.org/systems/portmaster/ (scraper identifies ports by launch-script filename).
- Real examples ship a plain zip that unpacks `roms/ports/Name.sh` and
  `roms/ports/name/` without PortMaster (Halo port, OpenCrossing). Source:
  https://github.com/kirklandsig/halo-ce-anbernic-rg35xx
- Knulli image has no gcc/make/apt (SSH as root for testing). Source:
  https://github.com/Miolonixc/rg40xx-demos (Knulli 2.173 era).
- **[UNCERTAIN]** The Knulli wiki lists RG40XX V on its device page
  (https://knulli.org/devices/anbernic/rg40xx-v/, kernel 4.9.170 Allwinner BSP
  and Mali G31 per search excerpt); the 20240721 release notes only mention an
  "RG40XX" image. Models are supported individually; check the compat list.

### ROCKNIX

- Ports in `/roms/ports` run as root (no sudo needed); PortMaster fully
  supported. Uses Panfrost (open Mesa) rather than libmali, so GL/GLES
  behaviour differs from stock/muOS/Knulli (see section 4).

### Where user data goes

Convention in PortMaster and in non-PortMaster ports: everything inside the
port folder (`ports/<name>/conf`, `save`, `log.txt`) so it is portable and
backed up with the SD card. Redirect `XDG_DATA_HOME`/`HOME` into the folder.

---------------------------------------------------------------------------

## 2. PortMaster (most relevant distribution channel)

Sources: https://portmaster.games/packaging.html (read from
https://raw.githubusercontent.com/PortsMaster/PortMaster-Website/main/docs/packaging.html),
https://github.com/PortsMaster/PortMaster-New (README),
https://raw.githubusercontent.com/PortsMaster/PortMaster-GUI/main/PortMaster/control.txt ,
`.../device_info.txt`, https://portmaster.games/porting.html.

### Package layout (one zip per port)

```
portname/                  (name: [a-z0-9][a-z0-9._]*, unique project-wide)
  port.json
  README.md
  screenshot.png|jpg       (gameplay, 4:3, >= 640x480)
  gameinfo.xml             (ES metadata; cover.png|jpg optional)
  Port Name.sh             (capitalised, ends .sh, unique)
  portname/                (same name as the containing dir)
    licenses/              (license files for game, libs, gptokeyb, etc.)
    <binary>.aarch64, libs.aarch64/, data...
```
Generated zip is `portname.zip` containing `Port Name.sh` and `portname/`.

### port.json (version 4 in current docs)

```json
{ "version": 4, "name": "defblade.zip",
  "items": ["Defblade.sh", "defblade"], "items_opt": [],
  "attr": { "title": "...", "porter": ["..."], "desc": "...", "inst": "Ready to run...",
            "genres": ["platformer"], "image": null, "rtr": true, "exp": false,
            "runtime": [], "store": [], "availability": "full",
            "reqs": [], "arch": ["aarch64"], "min_glibc": "" } }
```
`rtr: true` = "ready to run" (no user-supplied files). A ROM/BIOS-needing
port is `rtr: false` with install instructions in `inst`. If a runtime
(Godot/Mono/Java) is used, no arch is entered. There are generators at
portmaster.games/port-json.html and port-markdown.html.

### Launch script (current template)

```bash
#!/bin/bash
XDG_DATA_HOME=${XDG_DATA_HOME:-$HOME/.local/share}
if [ -d "/opt/system/Tools/PortMaster/" ]; then controlfolder="/opt/system/Tools/PortMaster"
elif [ -d "/opt/tools/PortMaster/" ]; then controlfolder="/opt/tools/PortMaster"
elif [ -d "$XDG_DATA_HOME/PortMaster/" ]; then controlfolder="$XDG_DATA_HOME/PortMaster"
else controlfolder="/roms/ports/PortMaster"; fi
source $controlfolder/control.txt
[ -f "${controlfolder}/mod_${CFW_NAME}.txt" ] && source "${controlfolder}/mod_${CFW_NAME}.txt"
get_controls
GAMEDIR=/$directory/ports/portname/
CONFDIR="$GAMEDIR/conf/"; mkdir -p "$CONFDIR"; cd $GAMEDIR
> "$GAMEDIR/log.txt" && exec > >(tee "$GAMEDIR/log.txt") 2>&1
export XDG_DATA_HOME="$CONFDIR"                       # or bind_directories ~/.x $GAMEDIR/conf/.x
export LD_LIBRARY_PATH="$GAMEDIR/libs.${DEVICE_ARCH}:$LD_LIBRARY_PATH"
export SDL_GAMECONTROLLERCONFIG="$sdl_controllerconfig"
$GPTOKEYB "portexe.${DEVICE_ARCH}" -c "./portname.gptk.$ANALOG_STICKS" &
pm_platform_helper "$GAMEDIR/portexe.${DEVICE_ARCH}"
./portexe.${DEVICE_ARCH}
pm_finish
```

### Variables provided by `control.txt` / `device_info.txt`

- `$directory` (`roms` or `roms2`), `$controlfolder`, `$ESUDO` (empty if no
  sudo, e.g. root-only firmwares), `$ESUDOKILL`.
- `$CFW_NAME` (muOS, knulli, ROCKNIX, spruce, ArkOS...), `$DEVICE_NAME`,
  `$DEVICE_ARCH` (aarch64/armhf/x86_64), `$DEVICE_CPU`, `$CFW_GLIBC`
  (digits, e.g. 231), `$GPU_DRIVER` (mali / panfrost / ...) and
  `$GPU_DRIVER_VERSION`, `$DEVICE_RAM`/`$DEVICE_RAM_MB`.
- `$DISPLAY_WIDTH`, `$DISPLAY_HEIGHT`, `$ASPECT_X/Y`, `$DISPLAY_ORIENTATION`,
  `$DEVICE_REFRESH_RATE`; `$ANALOG_STICKS` (0/1/2; note some older docs use
  `$ANALOGSTICKS`), `$ANALOG_TRIGGERS`, `$DEVICE_TOUCH`, `$DEVICE_HAS_RUMBLE`,
  `$DEVICE_CAPABILITIES` (space list such as `640x480 4:3 1gb analog_0 analog_1`;
  tokens `lowres`/`hires`/`wide`/`square`, `touch`, ...). The H700 is excluded
  from the "ultra" tier.
- `$sdl_controllerconfig` / `SDL_GAMECONTROLLERCONFIG_FILE`: SDL mapping string
  for the built-in pad (from PortMaster's `gamecontrollerdb.txt`, overridden by
  the CFW's ES input config when present).
- `$GPTOKEYB`, `$GPTOKEYB2` (gptokeyb2 with libinterpose preload).
- Helpers (funcs.txt): `bind_directories`, `bind_files`, `pm_message`,
  `pm_platform_helper`, `pm_finish`.

### Controls: gptokeyb / gptokeyb2 vs native SDL

- `gptokeyb` creates a virtual keyboard/mouse (uinput) from the pad using a
  `.gptk` mapping, AND provides the standard **Select+Start kill hotkey**
  (hotkey differs per device; called with only the process name it acts purely
  as a quitter: `./gptokeyb "app"`). It also works with apps that already read
  the pad through SDL; the common pattern for SDL-gamepad-native ports is
  `$GPTOKEYB "exe" -c "./x.gptk" &` with a minimal/empty mapping so that only
  the exit combo is active (e.g. SDLPoP script uses it this way).
- `gptokeyb2` (https://github.com/PortsMaster/gptokeyb2): rewrite with a state
  machine; INI-style `.gptk` files with `[config]`, `[controls]`,
  `[controls:hotkey]` etc., can emulate keyboard, mouse or an Xbox360 gamepad,
  and has an interactive text-input engine. Used via `$GPTOKEYB2` plus
  `libinterpose`.
- Native SDL2 GameController API with `SDL_GAMECONTROLLERCONFIG` from the
  script is fully supported and is what most 3D/native ports do. Several
  H700 ports (Halo, Paprium, Super Mario War) additionally implement their own
  exit combo (Select+Start or Menu+Start) in-process.
- Needs `chmod 666 /dev/uinput` (control.txt does this via `$ESUDO` when
  sudo exists) if gptokeyb is used.

### Runtimes

Squashfs runtimes (frt Godot 3, Mono, Java via WestonPack, Love2D, Qt5/Qt6,
x11/SDL3/SFML/Allegro WestonPack variants) are downloaded by `harbourmaster`
and mounted; a C++/Rust native binary does not need one. Native ports ship
`libs.aarch64/` next to the binary and set `LD_LIBRARY_PATH`.

### Technical rules (docs + observed practice)

- PortMaster must not install or upgrade OS libraries; non-standard libs live
  in the port folder.
- No Vulkan, no X11, no display manager; KMS/DRM or SDL2 output; OpenGL ES
  (2.0 universally; 3.x on Mali blobs, see section 4). Desktop GL needs
  gl4es (up to GL 2.1 emulation) -> not applicable to a GLES-capable renderer.
- glibc baseline: not stated as a fixed rule in the docs; `port.json` has
  `min_glibc`; ports are commonly built against **glibc 2.31 (Debian
  bullseye)** or lower (Halo port: "other PortMaster firmware needs glibc
  2.30+ and SDL2 2.0.18+"; Paprium links symbols up to 2.29; Super Mario War
  needs 2.34 and is thus muOS-only). Tooling: Debian bullseye arm64 chroot /
  clang toolchain in https://github.com/Knifethrower/PM-Porting-Tools .
  Safe baseline: build against glibc <= 2.31.
- Ports are built per-arch; the binary is named with `${DEVICE_ARCH}` suffix.
  H700 on muOS/Knulli is aarch64-first (Minecraft Bedrock, Halo, Paprium are
  arm64-only). **[UNCERTAIN]** the stock Anbernic OS userland on some H700 models is
  armhf; PortMaster scripts must not assume aarch64 on stock. A BaseOS issue
  notes a 32-bit userland in the H700 stock image.
- ES metadata: `gameinfo.xml` + screenshot are copied to ES on install.

### Submission and licensing

- Fork https://github.com/PortsMaster/PortMaster-New , disable Actions on the
  fork, run `tools/prepare_repo.sh`, create `ports/<name>/`, run
  `python3 tools/build_release.py --do-check`, split files >90 MB with
  `tools/build_data.py` (50 MB chunks), open a PR; crew reviews; zips for
  testing are built into `releases/`.
- Repo is MIT, but each port carries its own `licenses/` directory (game,
  gptokeyb, libs). Open-source and "engine reimplementation requiring user
  assets" ports are accepted; the README must credit original authors. Port
  must not bundle commercial game data or BIOS/ROMs; ROM-needing ports set
  `rtr: false`. **[UNCERTAIN]** whether the project accepts an NES emulator
  that requires user ROMs (PortMaster has emulator-like ports e.g. standalone
  engines that need game files; verify with the crew on their Discord before
  investing).
- Contact: PortMaster Discord (ports-help channel referenced in FAQ).

---------------------------------------------------------------------------

## 3. Existing standalone apps/emulators on these devices and their UI approach

What was verifiably found (primary sources are the project READMEs):

- **OpenCrossing (GameCube decomp, GLES 3.2 via GX translation)** on H700
  (Mali-G31): ~56 fps average on RG34XX SP (low 40s in heavy scenes). Ships
  as zip for muOS/Knulli (`ports/` + `ROMS/Ports/Animal Crossing.sh`), or
  PortMaster. Auto-detects panel; handles 640x480, 720x480, 720x720
  (letterboxed). In-game FPS target, render scale, shadow/particle quality
  options. Menu/Function opens settings; Select+Start exits. First launch
  slow (shader cache). https://github.com/TechProGabe/OpenCrossing-Anbernic
- **Halo CE port on RG35XX H (Knulli)**: render_scale 0.75 (480x360 upscaled)
  default, dynamic resolution down to 0.5; ~38-60 fps depending on scene at
  scale 1.0; Mali driver costs ~17 us CPU per draw call (dedicated GL thread);
  uses program-binary cache, fp16 shaders, 16-bit textures, restructured
  shadows to avoid render-target switches; thermal governor throttles at 70 C;
  CPU/GPU clocks pinned in the launcher and restored on exit; quit = hold
  MENU/SELECT + START; button mapping from ES config; launcher offers update
  prompt. muOS crash fix: muOS's SDL took over the port's crash handler.
  https://github.com/kirklandsig/halo-ce-anbernic-rg35xx
- **Minecraft Bedrock port**: launcher menu (Play, Versions, Install APK,
  Settings, ...); picks older version because menus are "the right size on a
  small screen"; settings for FPS cap, render distance, UI scale, VSync;
  adapts by measured capability rather than firmware name.
  https://github.com/DankMiimer/minecraft-bedrock-handheld-port
- **Paprium (Mega Drive) H700 port**: packaged as muOS `.muxupd` plus manual
  zip and experimental Knulli tarball; Select+Start exit, Select+R1 reset;
  firmware SDL2; ROM in `gamedata/`.
  https://github.com/VVmImo/paprium-h700-portmaster
- **PPSSPP / DraStic / Flycast / Mupen64Plus standalone**: found no primary
  source with H700-specific UI details. Review sites say PSP and Saturn are
  at the edge of the H700, N64/Dreamcast mostly fine. **[UNCERTAIN]** For
  these emulators the general pattern (from experience, not sourced here) is:
  they ship their existing gamepad-navigable UIs (PPSSPP has a console-style
  UI; DraStic a menu driven by the pad), launched through PortMaster or a
  firmware-specific pak/app.
- **RetroArch / libretro**: all three main CFWs (muOS, Knulli, ROCKNIX) run
  RetroArch for the bulk of emulation (guide: heldgames.com/guides/muos-vs-knulli-vs-onion-os).
  Its UIs (RGUI/Ozone) are full-screen lists driven by D-pad/A/B.
  Standalone N64 can beat the core on H700 due to lower overhead (same guide).
  No source found on hardware-render core performance specific to Mali-G31;
  **[UNCERTAIN]**. Vulkan is not available on these devices' blobs; GLES
  hardware-render cores (mupen64plus-next GLES) are the norm.
- **Dear ImGui on handhelds**: no PortMaster-specific write-up found. Dear ImGui
  itself: gamepad navigation is enabled with
  `io.ConfigFlags |= ImGuiConfigFlags_NavEnableGamepad` and requires the
  backend to set `io.BackendFlags |= ImGuiBackendFlags_HasGamepad` and feed
  `ImGuiKey_Gamepad_*` events (the stock SDL2 backend does this using
  SDL_GameController). `ImGuiConfigFlags_NavEnableSetMousePos` is suggested for
  TV/console-style use. Menu bars are mouse-oriented (nav works but is
  awkward); full-screen windows with lists are the pattern consoles use.
  Source: imgui.cpp comments / https://github.com/ocornut/imgui/issues/787 .
  (Fonts scale via `io.FontGlobalScale`/ `style.ScaleAllSizes()` or by
  re-baking at a larger size.)

---------------------------------------------------------------------------

## 4. Technical constraints

### GPU / GLES

- Mali-G31 MP2 is Bifrost; hardware: OpenGL ES 3.2 capable silicon. Firmware
  userspace differs:
  - **Stock, muOS, Knulli**: Allwinner BSP kernel 4.9.x with the proprietary
    **libmali** blob. OpenGL ES 3.0 is stated for libmali on RG40XX H
    (rg40xx-demos); OpenCrossing's GLES 3.2 translation is reported working
    on Knulli (community-tested) -> the blob exposes at least 3.0, probably
    3.2 **[UNCERTAIN: exact version string not found; query `eglinfo`/`GL_VERSION`
    on device]**.
  - **ROCKNIX**: mainline kernel with **Panfrost** (Mesa). G31 supported by
    Panfrost since Mesa 20.3; GLES 3.1 claimed for the RG40XX H (low-quality
    source, **[UNCERTAIN]**). PortMaster `device_info.txt` reports
    `GPU_DRIVER=mali|panfrost`, so launcher can branch. Halo notes a
    rendering quirk (parts of picture not updating unless
    `debug.instance_models=false`) on ROCKNIX with Mali-G52 (RK3566), i.e.
    driver-specific bugs exist.
- Desktop GL core 3.3 is **not available**; the handheld build must use
  GLES 3.0 (the existing aarch64 GLES path in the repo is the right target).
  Avoid GLES 3.1/3.2-only features (compute, geometry/tessellation, advanced
  blend).
- Mali driver costs (Halo notes): high per-draw CPU cost (~17 us), prefer
  batching; tiled GPU: avoid render-target switches (shadow maps, post
  passes), use program-binary cache, 16-bit textures, fp16 where safe.
- Shader compile stalls on first run; cache programs on disk.

### SDL2 / windowing

- No X11/Wayland (except WestonPack runtime). Apps use SDL2 with KMSDRM or the
  firmware's patched SDL (muOS, Knulli ship their own libSDL2 with GPU driver
  glue; ports commonly use "the firmware's SDL2" - the Paprium and Super Mario
  War ports explicitly do; Halo requires SDL2 >= 2.0.18). **Recommendation
  from practice: do not bundle your own libSDL2; use the system one**, and
  fall back only if absent. (Mali GBM quirk in mainline SDL was fixed years
  ago - SDL 2017 commit "Wrong GBM format".)
- Make sure the app does not hard-set `SDL_VIDEODRIVER`; let the firmware's
  SDL pick (muOS sets SDL environment via `SETUP_APP`).
- Joystick: some muOS setups need `SDL_JOYSTICK_DEVICE=/dev/input/eventN`
  because udev enumeration finds no pad (Super Mario War launcher). Use
  SDL GameController with `SDL_GAMECONTROLLERCONFIG` from PortMaster's
  `$sdl_controllerconfig` where available.

### glibc / arch

- Target glibc <= 2.31 for broad PortMaster coverage; Rust: build for
  `aarch64-unknown-linux-gnu` with a Debian 11 sysroot (or `cargo zigbuild
  --target aarch64-unknown-linux-gnu.2.31`). Statically link everything
  except libc, libSDL2, EGL/GLES (loaded from the firmware). 32-bit armhf
  only matters for stock OS / old RK3326 devices.

### Screen modes

- 640x480 (RG35XX, RG40XX H/V, RG28XX at 640x480), 720x480 (RG34XX),
  720x720 (RG CubeXX), 480x320 on RG280-class (n/a). Use
  `$DISPLAY_WIDTH/$DISPLAY_HEIGHT` or SDL display mode at runtime; do not
  assume 4:3. Square/wide layouts need UI that adapts (device_info tags
  `square`, `wide`, `lowres`, `hires`).

### Exiting, hotkeys, power

- PortMaster convention: **Select+Start** quits (gptokeyb); some devices use
  Minus+Start or Menu+Start. Native muOS apps: Menu/Guide button commonly
  quits. In-process combos documented: Halo (hold MENU/SELECT + START),
  Paprium (Select+Start), OpenCrossing (Select+Start; Menu opens settings;
  note that on modded stock OS Select and Menu are swapped).
- Volume/brightness are handled by the firmware's own input daemon (e.g.
  `oga_events` on ArkOS needed a restart after ports; modern CFWs handle it
  independently). The app should not grab those keys and should not exclusively
  grab the input device. Sleep/suspend/power button handled by the CFW; the
  Super Mario War notes and OpenCrossing README mention lid/power-button wake
  quirks on muOS clamshell models. Pause emulation when the window loses focus
  / on `SDL_APP_WILLENTERBACKGROUND` if triggered.
- Battery: no portable API; read `/sys/class/power_supply/*/capacity` if you
  want to show it. **[UNCERTAIN]** paths differ per firmware.
- Cleanup on exit: restore CPU governor/GPU clocks if changed, kill helper
  processes (`pm_finish` does this for gptokeyb), clear tty.

---------------------------------------------------------------------------

## 5. Recommendations for Zaxnes

### 5.1 Packaging, in order

1. **PortMaster port first** (also installable by hand via the autoinstall
   folder). It is the one format that covers muOS, Knulli, ROCKNIX, spruce,
   ArkOS/dArkOS with a single zip and handles launcher integration, game
   metadata and updates. Deliverables:
   `zaxnes/` (port folder), `Zaxnes.sh`, `port.json` (`arch: ["aarch64"]`,
   `rtr: false` because the user must supply NES ROMs; `min_glibc: "2.31"`),
   `README.md` with credits and "Compile" section, `gameinfo.xml`,
   `screenshot.png` (640x480 gameplay), `licenses/`, `libs.aarch64/` for
   anything not provided by the firmware (Lua is statically linked in Rust, so
   probably nothing).
2. **Direct zips for muOS (.muxapp) and Knulli/ROCKNIX (`roms/ports/Zaxnes.sh`
   + folder)** as a secondary convenience; the same launcher script can be
   shared (detect firmware via `$CFW_NAME` if PortMaster's control.txt exists,
   else fall back to path guessing). The muOS `.muxapp` needs `mux_launch.sh`
   rather than the PortMaster script.
3. Do not target stock OS initially.
4. Before submission, ask the PortMaster crew (Discord) whether a ROM-requiring
   emulator is acceptable; legal posture: no ROMs/BIOS shipped, GPL/other
   licenses listed in `licenses/`. (**[UNCERTAIN]**, see section 2.)

### 5.2 Launcher script behaviour

- Source `control.txt`, `get_controls`, `cd` into game dir, tee log to
  `log.txt`.
- Export `XDG_DATA_HOME=$GAMEDIR/conf` (saves, settings, shader cache) so all
  user data stays in the port folder; put the ROM list directory at a fixed
  path: `$GAMEDIR/roms/` (also scan `/$directory/nes` or the CFW's NES rom
  folders if present, optionally).
- `export SDL_GAMECONTROLLERCONFIG="$sdl_controllerconfig"`.
- Pass handheld info to the app via environment/args: `ZAXNES_HANDHELD=1`,
  `ZAXNES_DISPLAY=${DISPLAY_WIDTH}x${DISPLAY_HEIGHT}`, `ZAXNES_GPU=$GPU_DRIVER`,
  `ZAXNES_ANALOG_STICKS=$ANALOG_STICKS`, `ZAXNES_RAM_MB=$DEVICE_RAM_MB`. Better:
  also auto-detect handheld mode inside the binary (GLES + KMSDRM + no
  keyboard) so the app works when started by hand.
- Prefer **native SDL GameController** input (no gptokeyb mapping needed) and
  implement the exit combo and menu inside the app; still start
  `$GPTOKEYB "zaxnes.aarch64" &` with a near-empty `.gptk` only as a safety net
  (guaranteed Select+Start kill if the app hangs) and call `pm_finish`.
- Optionally set the CPU governor to performance and restore it on exit (as
  Halo does); clean up on crash paths.
- Create/seed config on first launch (default quality preset from RAM/GPU).

### 5.3 Handheld UI mode (640x480, gamepad only)

Implement a dedicated mode (auto when `ZAXNES_HANDHELD`, GLES target on
aarch64, or `--handheld`), not a rescaled desktop UI:

- Enable ImGui gamepad nav: `NavEnableGamepad`, backend `HasGamepad`, SDL
  GameController -> `ImGuiKey_Gamepad_*`; D-pad/left stick navigates, A =
  activate, B = back, X/Y = secondary, L1/R1 = tab switch. Consider
  `NavEnableSetMousePos` off; no mouse cursor.
- No menu bar. Replace with full-screen windows (list menus) occupying the
  whole 640x480 (or the actual size; support 720x480 / 720x720). Structure:
  Start screen -> ROM list (fixed folder, sorted, with search by letter jump on
  L/R) -> in-game; in-game overlay "Menu" (hotkey: Menu/Guide button, or
  Select+X) with Resume, Save/Load state, Reset, Graphics preset, Profile
  options, Controls, Quit.
- Larger fonts: base font ~20-24 px at 640x480 (font bake at runtime from
  height, `FontGlobalScale` from `h/480`), row height >= 36 px, generous
  padding, high contrast; keep to one column, avoid sliders that need fine
  control (use stepped +/- values with D-pad left/right) and avoid combo
  popups (use inline option cycling).
- No free file browser: only the fixed ROM folder(s) (plus optional extra
  folders from a text config). Remember last selected ROM and options.
- Text entry: avoid. If needed (search), provide a simple on-screen
  keyboard (grid of letters navigable by D-pad) or A-Z jump.
- Always show control hints on the bottom line (A Select, B Back, ...).
- Exit: quit combos (Select+Start) must work in menus and in-game; confirm
  dialog optional; Menu button opens overlay. Battery/time indicator optional.
- Handle square 720x720 and wide 720x480 layouts by deriving sizes from the
  actual framebuffer height/width, not constants.
- Avoid sticking points: no hover tooltips, no drag-and-drop, no window
  resizing, no multi-window layouts.

### 5.4 Render defaults for Mali-G31 / 1 GB (tunable; tested values need on-device validation)

- GLES 3.0 context, no desktop GL, no compute/geometry shaders. Prefer
  single render pass for the 3D scene and a single upscale blit.
- Internal render scale default **0.75** at 640x480 (Halo uses 0.75 by
  default and dynamic 0.5-1.0; OpenCrossing exposes render scale), with
  options 0.5 / 0.75 / 1.0. Provide a dynamic-resolution option later.
- Shadows **off by default** on H700; option "low" with a 1024 (or smaller)
  shadow map and a single cascade; shadow pass costs a render-target switch
  on a tiler.
- **No MSAA** (tiler can do 4x MSAA cheaply in theory, but on this stack and
  1 GB RAM it is a risk; keep off by default, maybe offer 2x/4x as experimental).
  No SSAO/bloom/post chain by default.
- Cap to 60 fps with vsync; also a 30 fps option; keep the NES emulation
  running at its own fixed rate decoupled from render cost (frameskip
  rendering if the 3D scene falls behind: emulate frames, skip draws).
- Limit draw calls (batch tiles/sprites into few meshes), use compressed or
  16-bit textures (ETC2 is mandatory in GLES 3.0), fp16/mediump in fragment
  shaders where there is no visual harm, cache program binaries on disk
  (`GL_OES_get_program_binary` is core in GLES 3.0).
- Memory: aim for < 300 MB resident; 1 GB total is shared with the OS and
  frontend (ES on Knulli uses more).
- Detect `GPU_DRIVER` (mali/panfrost) and pick presets; log GL_VENDOR /
  GL_RENDERER / GL_VERSION to `log.txt` for support.
- Audio: use SDL2 `alsa`/default; no bundled audio libs; small buffer sizes
  appropriate for A53.

### 5.5 Native vs. libretro

- **Standalone port first.** Reasons: (1) 3D profiles (Lua-scripted) and a custom
  UI need their own GL context, input and menu; (2) PortMaster supports native
  GLES binaries well; (3) all H700 firmwares are RetroArch-centric but users
  also run ports regularly; (4) there is precedent for heavier native GLES apps
  at 40-60 fps (OpenCrossing, Halo).
- **Plan a libretro core later** (not first) because the core crates are already
  separable: hardware-render libretro cores (GLES 3.0 via `RETRO_HW_CONTEXT_OPENGLES3`)
  are possible, RetroArch supplies a gamepad menu, controller mapping, saves,
  hotkeys, sleep/power and ES integration on every CFW. Costs: RetroArch's own
  GL context is shared with the core (state restore after each frame, FBO
  provided by the frontend), no custom ImGui UI, config via core options only,
  and users need a core build per RetroArch version/ABI (aarch64 `.so` must
  match the CFW's RetroArch glibc). **[UNCERTAIN]** HW-render core performance on H700
  vs. standalone is not documented in the sources reviewed; the N64 guide
  notes standalone can beat the core due to overhead. A libretro core would be
  the best way to reach people on stock/Onion-like systems where PortMaster is
  absent, but only after the standalone build proves 3D performance.

### 5.6 Concrete work list for the handheld build

1. Cargo target + CI for `aarch64-unknown-linux-gnu` (glibc <= 2.31 sysroot),
   GLES 3.0 renderer path validated; strip and package.
2. `--handheld` / auto-detect switch: UI scale, font size, gamepad nav, no menu
   bar, full-screen list menus, ROM list from fixed folder.
3. SDL GameController -> ImGui gamepad backend; in-app exit combo
   (Select+Start) and menu hotkey; use mapped names only.
4. Resolution handling for 640x480 / 720x480 / 720x720; render scale setting.
5. Settings defaults/presets for Mali-G31; on-disk shader program binary
   cache; log GL info.
6. Packaging scripts: PortMaster zip (`Zaxnes.sh`, `port.json`, `gameinfo.xml`,
   `licenses/`), muOS `.muxapp` with `mux_launch.sh`, plain Knulli/ROCKNIX zip.
7. On-device validation matrix: RG40XXV on muOS and Knulli (blob) and ROCKNIX
   (Panfrost); one RK3566 device optional.
8. Optional: gamepad-only first-run screen explaining ROM folder.

---------------------------------------------------------------------------

## Source list

- PortMaster packaging: https://portmaster.games/packaging.html (read via https://raw.githubusercontent.com/PortsMaster/PortMaster-Website/main/docs/packaging.html)
- PortMaster porting/restrictions: https://portmaster.games/porting.html (docs/porting.html in the same repo)
- PortMaster FAQ and installation: https://portmaster.games/faq.html , https://portmaster.games/installation.html
- PortMaster-New README (submission): https://github.com/PortsMaster/PortMaster-New
- control.txt / device_info.txt / funcs.txt: https://github.com/PortsMaster/PortMaster-GUI/tree/main/PortMaster
- gptokeyb2: https://github.com/PortsMaster/gptokeyb2
- Legacy packaging notes: https://github.com/christianhaitian/PortMaster/blob/main/docs/packaging.md
- PM-Porting-Tools (glibc 2.31 chroot, lint): https://github.com/Knifethrower/PM-Porting-Tools
- muOS archive/apps: https://muos.dev/help/archive , https://community.muos.dev/t/application-runner/1282 , https://github.com/MustardOS/frontend
- muOS .muxapp example: https://github.com/KyleJamesWalker/supermariowar-rust/pull/9
- Knulli: https://knulli.org/play/add-games/game-storage/ , https://knulli.org/systems/portmaster/ , https://knulli.org/devices/anbernic/rg40xx-v/
- H700 native ports: https://github.com/TechProGabe/OpenCrossing-Anbernic , https://github.com/kirklandsig/halo-ce-anbernic-rg35xx , https://github.com/VVmImo/paprium-h700-portmaster , https://github.com/DankMiimer/minecraft-bedrock-handheld-port , https://github.com/Miolonixc/rg40xx-demos
- NextUI H700 release notes: https://newreleases.io/project/github/pvaibhav/NextUI/release/h700-rc11
- spruceOS v4.4.0: https://newreleases.io/project/github/spruceUI/spruceOS/release/v4.4.0
- ROCKNIX H700 support: https://retrohandhelds.gg/rocknix-adds-anbernic-xx-support-rpcs3-for-retroid-pocket-5/
- Dear ImGui gamepad nav: https://github.com/ocornut/imgui/issues/787
- Firmware comparison: https://heldgames.com/guides/muos-vs-knulli-vs-onion-os
