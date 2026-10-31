<p align="center">
  <img src="branding/zaxnes-logo.svg" alt="Zaxnes" width="420">
</p>

# Zaxnes

**Zaxnes** is a free NES emulator that plays your games **in 3D**. The flat tiles and sprites of
each frame become little 3D blocks, set in a scene with a real camera, lights and shadows: you can
turn around the level, zoom in, or go back to plain 2D at any time.

For a number of games, Zaxnes can also play in **widescreen**: the level is shown past the edges
of the original screen (see [Widescreen](#widescreen-experimental)).

It runs on **Windows**, **Linux** and **handheld consoles** through PortMaster (Anbernic H700
devices such as the RG40XXV and the RG35XX family, on muOS, Knulli or ROCKNIX).

> **Independent project, not affiliated with Geod Studio or Nintendo. NES is a trademark of
> Nintendo.**

## The 3D profiles: you need 3dSen

How a game looks in 3D is described by a **3D profile**, one per game. Zaxnes does not ship any:
the profiles are the work of the [3dSen](https://store.steampowered.com/app/1147940) team at Geod
Studio, and they are what makes the magic happen.

**3dSen is a great app, worth every penny.** Over a hundred NES classics hand-tuned into gorgeous 3D
dioramas, years of passionate work by a small studio, and the original idea Zaxnes owes everything
to. If you enjoy NES games in 3D, it is the best few dollars you can spend, and buying it is how
this whole scene keeps going.

**[Get 3dSen on Steam](https://store.steampowered.com/app/1147940)**, then in Zaxnes use
**File ▸ Import from 3dSen…**: Zaxnes reads the profiles of *your* installation and copies
them into your own profile folder. Nothing is downloaded, nothing is redistributed, and your 3dSen
installation is left untouched. On a handheld, run the import on a PC and copy the `profiles`
folder to the card.

> **3dSen version 2 is not supported yet.** Its new profile format cannot be imported for now;
> the import works with the profiles of the earlier versions.

**Some community profiles are free:** players share the profiles they made in the
[Profile Repository](https://itch.io/board/872690/profile-repository) of the 3dSen Maker forum on
itch.io. Import them with **File ▸ Import a profile…**.

Without a profile, every game still plays, in **2D**, or in a **default 3D** that gives every tile
the same depth.

### Making your own profiles

Zaxnes has **no profile editor of its own yet**. To make or change a profile, use
[3dSen Maker](https://geod.itch.io/3dsen-maker), Geod Studio's free profile editor, then import the
profile in Zaxnes (**File ▸ Import a profile…**).

## Widescreen (experimental)

Some games can be played in **16:9** (or 16:10, 3:2, or the shape of your window): the level and
the enemies are shown **past the edges of the original screen**, where the NES never drew them.

**This is experimental.** It works through most of these games, but corrections are still to be
made: expect glitches here and there, mostly at the edges of the screen.

**It is not a recompilation.** The game is not rewritten for a PC, as a "recomp" does: it still runs
on the emulated NES. The widescreen is a combination of two things:

- a **small ROM hack applied in memory** to your own ROM when the game starts (nothing is written
  to your file, and Zaxnes ships no game data), which teaches the game to prepare the extra columns
  of the screen;
- **special registers added to the emulated console**, which the hack uses to draw those columns
  past the original 256 pixels.

Because the hack is small and only touches what the screen needs, it is **compatible with most
ROM hacks** of these games: translations, difficulty or graphics hacks keep working in widescreen.

Turn it on per game: on the ROM list, the games that support it carry a **[W]**; **L1** (or the
game's options) opens its widescreen settings.

Games supported so far:

- *Adventure Island*
- *Blaster Master*
- *Castlevania*
- *Excitebike*
- *Final Fantasy*
- *Mega Man*, *Mega Man 2*, *3*, *4*, *5* and *6*
- *Metal Force*
- *Metroid*
- *Ninja Gaiden III*
- *Super Mario Bros.* and *Super Mario Bros. 3*
- *Zelda II: The Adventure of Link*

## Also in Zaxnes

- **RetroAchievements**, in softcore mode: sign in from the options, and save states, rewind and
  fast forward stay allowed. The ROM list shows how far you got in each game.
- **Save states** with thumbnails, **rewind** and **fast forward**.
- **Movies**: record a game and replay it exactly.
- A **console interface** made for the gamepad, on handhelds and on the desktop (F1).
- Vulkan and DirectX 12 on the desktop, besides OpenGL; in Vulkan, ReShade can use the depth of
  the 3D scene.

The user guide, in French, is [HOWTO-RUN.md](HOWTO-RUN.md): menus, shortcuts, options, where your
files go, and the command line. The handheld controls are in the
[PortMaster README](rust/build/portmaster/README.md).

## Licence and credits

Zaxnes is free software under the **GNU General Public License v3** ([LICENSE](LICENSE)). Part of
its 3D conversion is adapted from **3DNes_Engine** (© Geod Studio), an earlier version of that
engine published under the GPL-3.0; the files concerned say so. Zaxnes contains **no file taken
from 3dSen**.

Everything Zaxnes shows or plays is made by its own code: the skies, the interface sounds and the
font come from the program or from your system. [ASSETS-LICENSES.md](ASSETS-LICENSES.md) and
[THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md) list the assets and the third-party code.

> Zaxnes 0.3.0 is written in Rust. Versions 0.1.0 and 0.2.0 were written in C#; that code is kept
> on the [`archive-dotnetversion`](../../tree/archive-dotnetversion) branch.

## For developers

The code is a Cargo workspace in [`rust/`](rust):

```
cd rust
cargo build --release -p zax-app --features bundled
```

How to build the release packages, run the tests, and how the code is laid out are in
[`rust/README.md`](rust/README.md) (the technical part of the former README, moved there). The
widescreen's emulator side is described in
[`rust/docs/WIDESCREEN-EXTENSION.md`](rust/docs/WIDESCREEN-EXTENSION.md), and each game's hack in
[`rust/docs/hacks/`](rust/docs/hacks); the written specification is in [`docs/spec/`](docs/spec).

## Download

Every [release](../../releases) has one package per platform:

| Package | For |
|---|---|
| `Zaxnes-<version>-win-x64.zip` | Windows 10/11: a single `zaxnes.exe`, nothing to install |
| `Zaxnes-<version>-linux-x64.tar.gz` | Linux (needs SDL2, `libsdl2-2.0-0` on Debian/Ubuntu) |
| `Zaxnes-<version>-portmaster-aarch64.zip` | PortMaster handhelds: unzip into the `ports` folder of the SD card |

**No package contains a game or a 3D profile.** You need your own, legally owned ROMs.
