# Zaxnes

NES emulator that draws the games in 3D (Rust port), packaged for PortMaster on aarch64
handhelds (Anbernic H700: muOS, Knulli, ROCKNIX). Licence: GPL-3.0 (see `licenses/`).

No game, ROM or profile is included.

## Install

1. Install the port from the PortMaster folder (unzip `Zaxnes.zip` into the `ports` folder of
   the SD card, so you get `ports/Zaxnes.sh` and `ports/zaxnes/`).
2. Copy your own `.nes` files to `zaxnes/roms/`.
3. Copy your profiles to `zaxnes/profiles/`.
4. Start Zaxnes from the Ports menu. Hold Select+Start for one second to quit (or "Exit to
   launcher" in the in-game menu): the settings and saves are written before it closes.

## Controls in game

| Button | Action |
|---|---|
| D-pad, A, B, Select, Start | the NES pad (A and B are the buttons labelled A and B) |
| X | default camera view |
| Y | 2D / 3D |
| Y held | 2D seen from the front (default view) |
| L1 / R1 | zoom out / zoom in |
| Right stick | turn the camera |
| R2 | fast forward |
| L2 held | keep the camera angle and zoom |
| L2 short press | put the kept camera angle and zoom back |
| Menu (or Select+Start tapped) | in-game menu |
| Select+Start held one second | quit |

In the game list, X chooses the profile of the selected ROM: the three closest titles first,
then "2D Mode" and the 3D default, then every profile by title (left/right move by a page). The
choice is kept for that ROM.

In the in-game menu, left/right on "Save state" or "Load state" picks the slot, and A opens the
list of the slots, with the date and the picture of each: purple to save, blue to load, as the
title says. A state made with the built-in patches set the other way shows in orange (Zaxnes
asks before switching them), one of another version of the built-in hack in red (it cannot load).

A pad saved with the old default layout moves to this one; a layout changed in Options >
Controls is kept.

## Profiles from 3dSen (convert on a PC first)

Profiles come from your own copy of 3dSen PC. On a PC, with the desktop build of Zaxnes:

    zaxnes --import-3dsen "<3dSen PC install folder>" --userdata <port folder>

This writes the converted profiles to `<port folder>/profiles/`. You can also run it
with any `--userdata` folder and copy its `profiles/` folder to the card.

## Files

- Logs of the last run (both are replaced at each start):
  - `ports/zaxnes/log.txt`: the launcher's lines (firmware, device, GPU driver) followed by
    everything the program printed. If the port folder cannot be written, it is
    `/tmp/zaxnes-log.txt`.
  - `ports/zaxnes/logs/zaxnes.log`: the program's own copy (SDL video driver, GL context,
    GL_RENDERER and a WARNING when it is a software renderer, the user data folder, and the
    frame profiler every two seconds when it is on). If that folder cannot be written, the program tries
    `ports/zaxnes/logs/` next to the binary, then the system's temporary folder, and says so
    on the console.
- The frame profiler (Options > Performance measures, off by default; or `ZAXNES_PROFILE=1` in
  `Zaxnes.sh`): each line gives
  the mean/max milliseconds of every stage of a frame (input, core, capture, conversion, Lua,
  rollback, host, scene, UI, audio, swap, sleep) and the frame rate; with "Show the FPS" on, a
  short version is drawn under the counter.
- Settings, save states and battery saves: `zaxnes/` (the port folder).
- Screenshot for the PortMaster listing: not made yet (`screenshot-placeholder.txt`).
