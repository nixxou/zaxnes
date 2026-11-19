# Lot LIBRETRO-OPTIONS: the RetroArch core catches up with the application

Branch `fix/libretro-profile-title` (worktree `G:\3DSen\wt-libretro-title`), from `dev` 47189e3e.
Owner request 2026-11-19 ("niveau option, c'est quand même très en dessous"), plus GitHub issue #2.

Rules: `unsafe` only in `src/ffi.rs` (owner exception); the rest safe Rust. The core's copies of
application code stay identical (`head_on_pull.rs`, `overscan.rs`, `scene_settings.rs`). Option
keys never renamed (players' `Zaxnes.opt` keeps its values). Tests with
`cargo test -p zax-libretro --profile libretro`; real checks in the owner's RetroArch
(`G:\LB\Emulators\RetroArch`) with a scratch `--config`, never the owner's config.

## Done

- L0. Untitled profiles (3dSen 2.0) go by their file name, as in the application (issue #2):
  bb7731f3, log line 32aa1089. Checked in RetroArch with Contra.
- Step 1, options v2 in submenus: 3285054e, d12561a3. RetroArch takes SET_CORE_OPTIONS_V2.
- Step 2a, 3D field of view, light, geometry from the catalogue's libretro values: 97811a59
  (checked in RetroArch: FOV 45, bend 20).
- Step 2b, the aids' All games options (COMMON_OPTIONS moved to zax-nes): d9edd31f.
- Step 2c, core variant, console hardware, VS DIP, sound profile (src/machine.rs): 725e2a60
  (checked in RetroArch: Fidelity+ and Fast start).
- Step 2d, television, composite signal (RF checked on SMB in RetroArch), emphasis in 3D
  (src/picture.rs): 41c2bfd6. Step 2 complete.
- Step 3, presets (table moved to zax_settings::presets): see the commit "the performance /
  fidelity presets". A player with no Zaxnes.opt now gets Balanced, so the Enhanced style by
  default, as in the application.
- Step 4 (issue #2): 3D resolutions 240p to 2160p and a 3D aspect ratio option (16:9, 4:3,
  widescreen ratio): 65aa88f9; the 3D drawn into the core's own framebuffer and copied upside
  down, declared top-left like the 2D (the glcore flip of shader presets no longer needed):
  a2d2c055, 1e1bfeb2. Checked in RetroArch (320x240 4:3, picture upright). Not checked: Mega
  Bezel itself (its glcore_Zaxnes.params flip should now be removed).
- Step 5, Zapper: both ports offer "NES pad" or "Zapper" (SET_CONTROLLER_INFO); the light gun's
  screen x/y aim at the 2D picture shown (crop and widescreen margins taken into account), its
  trigger (or reload, aimed off screen) gives the port byte 0x00, released 0x10; option "Zapper
  crosshair" (Emulation, on; in 3D drawn by scissored clears at the pointer, after the copy). In 3D the aim goes through the
  camera to the picture's plane with the application's code, moved from zax-app's
  input/zapper.rs to zax_render::zapper (zax-app re-exports it); test
  a_zapper_aims_through_the_camera_in_3d. RetroArch only reads a port's device from a remap file
  (Quick Menu > Controls > Port 2 > Device Type saves one), and keeps a pointer to the device
  types the core declares: they are static. Test a_zapper_aimed_at_the_picture_shows_its_crosshair_there;
  RetroArch: Duck Hunt loads with port 2 on Zapper, title screen not skipped (trigger released).
  Not checked by hand: shooting with the mouse (needs the owner at the window).
- Real checks use a scratch RetroArch config whose rgui_config_directory is in the scratchpad,
  so the owner's Zaxnes.opt is no longer written.

## Steps

1. **Options v2 with categories.** `GET_CORE_OPTIONS_VERSION`, then `SET_CORE_OPTIONS_V2` (with
   `SET_CORE_OPTIONS_V2_INTL` left for later), falling back to `SET_VARIABLES`. Categories:
   Image, 3D, Widescreen, Hack aids, Emulation, Audio; a one-line description per option and
   proper value labels. Existing options only, same keys and defaults. Check: RetroArch shows the
   submenus; a frontend without v2 gets the flat list.
2. **Options the application already has.**
   - Image: TV model, composite decoder (sharpness, comb filter, frozen dot crawl), RF noise and
     ghost, hue, saturation, contrast, brightness, colour emphasis in 3D.
   - 3D: geometry (bend / curvature), light intensity, field of view.
   - Hack aids: the common defaults of the options window (`COMMON_OPTIONS`: no slowdown,
     invincible, level select, margin enemies, asleep enemies, grey), with the per-game options
     kept; built-in patches on/off.
   - Emulation: core variant (standard, Fidelity+, fast), cartridge database, power-on RAM, OAM
     corruption / decay, OAMADDR bug, Rainbow network, VS. System DIP switches.
   - Audio: console profile.
   Each reads the same `SettingId` as the application, through the settings model the core
   already builds.
3. **Presets.** A "Preset" option at the top level (Fastest, Fast, Balanced default, Faithful,
   Most faithful), from the application's table (`zax-app/src/ui/presets.rs`, moved where both
   reach it). Every option a preset touches gains a "Preset" value, its default: an explicit
   value wins for that option only. What has no meaning in a core (run-ahead, catch-up, 3D paused
   in 2D, automatic resolution) is left out and the description says so. The two 120 Hz
   scrolling options stay out of the presets (they depend on the display).
4. **Issue #2 requests.**
   - 3D resolutions down to 240p (320x240, 426x240, 640x480 ...).
   - A true 4:3 output in 3D (bezels, CRT): the 3D drawn at 4:3, the aspect reported 4:3.
   - `glcore` picture upside down in 2D with the shader packs' fix: draw the 2D through the
     hardware path too when the HW context exists, so 2D and 3D come out the same way up.
5. **Zapper.** `zax-nes` emulates it; the core declares a lightgun device on port 2
   (`SET_CONTROLLER_INFO`), reads `RETRO_DEVICE_LIGHTGUN` (screen x/y, trigger, offscreen), maps
   it to `ZapperPosition`, optional crosshair drawn by the core (option). Check with Duck Hunt.

## After

Merge into `dev` (owner), rebuild the core for the next release, reply on issue #2 (owner posts).
Out of scope: run-ahead, vsync, fullscreen, shaders and achievements (RetroArch's own); editor
mode (owner decision); smooth scrolling in 3D (a separate idea, application and core alike).
