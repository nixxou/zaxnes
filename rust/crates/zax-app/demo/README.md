# FlappyZaxNes, the game included with Zaxnes

`FlappyZaxNes.nes` and its 3D profile `flappyzaxnes.3dn` go inside the executable
(`src/demo_game.rs`): at start-up Zaxnes writes them into the ROM folder and the user's profiles.

FlappyZaxNes is © 2026 Nixxou, after *Flappy NES* © 2025 filipemd
(<https://github.com/filipemd/flappy-nes>), under the GNU General Public License v3
(`source/LICENSE`). Its complete source is in `source/`:

| | |
|---|---|
| `source/src/` | the game, 6502 assembly for cc65 (`make`, or the commands of `source/widescreen/build.py`'s header) |
| `source/char.chr` | the graphics (CHR-ROM), drawn by `source/tools/` (`decor.py`, `clouds.py`, `bird.py`, `logo.py`) |
| `source/music/partition.json` | the music, converted to `src/music_data.inc` by `tools/music.py` |
| `source/profile/flappyzaxnes.json` | the 3D profile's description, written by `tools/profile3d.py`; `zax-harness profile-build` makes the `.3dn` from it; its template, `profile/empty.3dn` (an empty profile), is not included |
| `source/widescreen/` | the widescreen hack pack (`rust/crates/zax-nes/patches/flappyzaxnes-widescreen.zxpk`), documented in `rust/docs/hacks/flappyzaxnes/WIDESCREEN-HACK.md` |

Building the ROM: `ca65 -g src/<file>.s -t nes` for each file, then
`ld65 src/*.o -o FlappyZaxNes.nes -t nes -Ln build.lbl`.
