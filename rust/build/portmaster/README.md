# Zaxnes

NES emulator that draws the games in 3D (Rust port), packaged for PortMaster on aarch64
handhelds (Anbernic H700: muOS, Knulli, ROCKNIX). Licence: GPL-3.0 (see `licenses/`).

No game, ROM or profile is included.

## Install

1. Install the port from the PortMaster folder (unzip `Zaxnes.zip` into the `ports` folder of
   the SD card, so you get `ports/Zaxnes.sh` and `ports/zaxnes/`).
2. Copy your own `.nes` files to `zaxnes/roms/`.
3. Copy your profiles to `zaxnes/profiles/`.
4. Start Zaxnes from the Ports menu. Select+Start quits.

## Profiles from 3dSen (convert on a PC first)

Profiles come from your own copy of 3dSen PC. On a PC, with the desktop build of Zaxnes:

    zaxnes --import-3dsen "<3dSen PC install folder>" --userdata <port folder>

This writes the converted profiles to `<port folder>/profiles/`. You can also run it
with any `--userdata` folder and copy its `profiles/` folder to the card.

## Files

- Log of the last run: `ports/zaxnes/log.txt` (shows the GL driver, renderer and display mode).
- Settings, save states and battery saves: `zaxnes/` (the port folder).
- Screenshot for the PortMaster listing: not made yet (`screenshot-placeholder.txt`).
