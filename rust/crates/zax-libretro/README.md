# Zaxnes libretro core

Zaxnes as a [libretro](https://www.libretro.com/) core, for RetroArch and the other libretro
frontends.

## What it does

- The NES core of Zaxnes (`zax-nes`): 144 mappers, NTSC, PAL and Dendy.
- The **widescreen hacks** of the games that have one (Super Mario Bros. 3, Zelda II, Mega Man,
  Kirby's Adventure, Faxanadu...), with their options. Turning the widescreen on or off switches
  at once for the games whose hack pack carries its states both ways; for the others it applies
  at the next load.
- States, so the frontend's save states, rewind and run-ahead.
- The battery RAM (`.srm`) and the console's work RAM, which RetroAchievements reads.
- Sound at 48 kHz, two NES pads.

## 3D

A game with a 3D profile plays **in 3D**, drawn with the frontend's OpenGL (a 3.3 core context;
in RetroArch the `glcore` or `gl` video driver): the conversion engine, the profile's Lua scripts
and the renderer of the application. The profiles are looked for in this order:

- the ones the core carries: FlappyZaxNes's, and the profiles made for Zaxnes
  ([zaxnes_3dn](https://github.com/nixxou/zaxnes_3dn): Faxanadu, Metal Force);
- the `.3dn` files of `<RetroArch system folder>/Zaxnes/profiles` (your profiles, imported from
  3dSen by the Zaxnes application for instance).

A profile suits a ROM whose file name is its title, or starts with it, as in the application's
ROM list. A game without one plays in 2D, as does every game with the renderer option on "2D
only".

| Control (pad 1) | 3D |
|---|---|
| Right stick | turn and tilt the camera |
| L2 / R2 | bring the camera nearer / farther |
| L3 | the 2D picture / the 3D |
| R3 | the camera back where the profile puts it |

In 3D a state carries the machine and the scripts' 64 `PData` values, as the application's
rollback does. The editor mode is the application's only.

## Core options

| Option | Choices |
|---|---|
| Widescreen (games with a widescreen hack) | enabled, disabled |
| Widescreen ratio | 16:9, 16:10, 3:2, 21:9 |
| Widescreen advanced mode | enabled, disabled |
| Region | Auto, NTSC, PAL, Dendy |
| Sprite limit | 8 per line, Removed, Smart |
| Palette | the eleven palettes of the application |
| Hack: ... | the options of the game's hack, when it has some |
| Renderer (restart) | 3D for games with a profile, 2D only |
| 3D resolution (restart) | 1280x720, 1920x1080, 2560x1440, 3840x2160, 960x540 |

The picture has square pixels, like the application's 2D picture: keep RetroArch's aspect ratio
on "Core provided" so that the widescreen keeps its shape.

## Building

```
cd rust/crates/zax-libretro
make
```

`make` runs `cargo build --profile libretro -p zax-libretro` in the workspace and copies the
result as `zaxnes_libretro.dll`, `.so` or `.dylib`. The `libretro` profile is the release one with
`zax-engine3d` at opt-level 2 (rustc 1.99.1 crashes generating that crate's machine code at 3,
which a library like this one asks for). `zaxnes_libretro.info` is the core's information file
for the frontends.

## Code

`src/core.rs` (the game in play), `src/options.rs` (the core options), `src/profiles.rs` (the
profile of a game) and `src/three_d.rs` (the 3D, put together as the harness's headless runner
does) are safe Rust. `src/ffi.rs`,
the C interface of `libretro.h`, is the only `unsafe` code of the crate.

Tests: `cargo test -p zax-libretro --profile libretro`. They play the FlappyZaxNes demo, and with
`ZAXNES_LIBRETRO_ROMS` (folders separated like `PATH`) Kirby's Adventure and The Legend of Zelda;
`ZAXNES_LIBRETRO_SHOTS` names a folder for their pictures. The 3D tests draw into an offscreen
OpenGL context (`zax-render`'s `offscreen` feature) and skip without one.
