# Zaxnes libretro core

Zaxnes as a [libretro](https://www.libretro.com/) core, for RetroArch and the other libretro
frontends.

## What it does

- The NES core of Zaxnes (`zax-nes`): 144 mappers, NTSC, PAL and Dendy.
- The **widescreen hacks** of the games that have one (Super Mario Bros. 3, Zelda II, Mega Man,
  Kirby's Adventure, Faxanadu...), with their options. Turning the widescreen on or off switches
  at once for the games whose hack pack carries its states both ways; for the others it applies
  at the next load.
- States, so the frontend's save states, rewind and run-ahead (in 3D too).
- The battery RAM (`.srm`) and the console's work RAM, which RetroAchievements reads.
- Sound at 48 kHz, two NES pads.
- The **120 Hz smooth scrolling** of the application, for a display at 120 Hz (below).

## 3D

A game with a 3D profile plays **in 3D**, drawn with the frontend's OpenGL (a 3.3 core context;
in RetroArch the `glcore` or `gl` video driver): the conversion engine, the profile's Lua scripts
and the renderer of the application. The profiles are looked for in this order:

- the ones the core carries: the profiles made for Zaxnes
  ([zaxnes_3dn](https://github.com/nixxou/zaxnes_3dn): Faxanadu, Metal Force). The core ships no
  game: FlappyZaxNes, its profile included, stays with the application;
- the `.3dn` files of `<RetroArch system folder>/Zaxnes/profiles` (your profiles, imported from
  3dSen by the Zaxnes application for instance).

A profile suits a ROM whose file name is its title, or starts with it, as in the application's
ROM list. A game without one plays in 2D, as does every game with the renderer option on "2D
only".

| RetroPad (pad 1) | 3D, at the places of the application |
|---|---|
| Right stick | turn and tilt the camera |
| L / R | the camera farther / nearer |
| Y (west) | the 2D picture / the 3D; held, the 2D seen head on |
| X (north) | the default view, where the profile puts the camera |
| L2 | held, keeps the camera's angle and distance; pressed, puts them back |

In 2D seen head on the right stick pulls the picture, which gives way to the camera past a
limit, as in the application. The settings a profile's scripts change while playing (the light,
the field of view, the sky...) reach the picture at the next frame.

What the scripts raise through `NotiCenter` shows on the frontend's screen.

In 3D a state carries the machine and the scripts' 64 `PData` values, as the application's
rollback does. The editor mode is the application's only.

## 120 Hz smooth scrolling

With the option on and the game in 2D, the core runs at twice the game's frame rate (120.2 Hz
for NTSC): between two frames it shows the playfield half way along the camera's motion, the
sprites that move with the camera left where they are. The motion is the camera the game's hack
reports (Kirby's Adventure, its hack option "Smooth scrolling (from game RAM)"), or for any game
the scroll of a frame run ahead (the second option). The picture is four times as wide, so that
it slides by quarters of a pixel; its shape stays the game's.

It needs a display at 120 Hz with RetroArch's refresh rate set to it: at 60 Hz the game would run
at half speed. Leave RetroArch's own run-ahead off; the core keeps its pace with it, but its
frame run ahead already takes the latency.

## Core options

| Option | Choices |
|---|---|
| Widescreen (games with a widescreen hack) | enabled, disabled |
| Widescreen ratio | 16:9, 16:10, 3:2, 21:9 |
| Widescreen advanced mode | enabled, disabled |
| Widescreen extra margin | 0% to 300%: the margins and the height that much larger, so that the 3D objects at the edges are whole |
| Widescreen strict edges (3D) | disabled, enabled |
| Widescreen on other versions of a game (restart) | disabled, enabled: the hack of another version of the game, forced |
| Smooth scrolling 120 Hz | disabled, enabled |
| Smooth scrolling of every game | enabled, disabled: a frame run ahead gives the motion |
| Region | Auto, NTSC, PAL, Dendy |
| Sprite limit | 8 per line, Removed, Smart |
| Palette | the eleven palettes of the application |
| Overscan top, bottom, left, right | 0 to 32 pixels left out; left and right without widescreen only |
| Hack: ... | the options of the game's hack, when it has some |
| Renderer (restart) | 3D for games with a profile, 2D only |
| 3D resolution (restart) | 1280x720, 1920x1080, 2560x1440, 3840x2160, 960x540 |
| 3D anti-aliasing | MSAA 4x, Off, MSAA 8x, SMAA |
| 3D shadows | enabled, disabled |
| 3D visual style | Classic, Enhanced |
| 3D reflections (Enhanced style) | disabled, enabled |
| 3D sky | Default, Colour, Gradient, Fantasy |
| 3D optimised Lua (restart) | disabled, enabled: the application's faster Lua paths, `unsafe` code |

The 3D options apply at once; they are the application's settings, so a profile or its scripts
may still change them.

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
does) are safe Rust, as are `src/scene_settings.rs`, `src/head_on_pull.rs` and
`src/overscan.rs`, copies of the application's code (keep them the same). `src/ffi.rs`,
the C interface of `libretro.h`, is the only `unsafe` code of the crate.

Tests: `cargo test -p zax-libretro --profile libretro`. They play the FlappyZaxNes demo, and with
`ZAXNES_LIBRETRO_ROMS` (folders separated like `PATH`) Kirby's Adventure and The Legend of Zelda;
`ZAXNES_LIBRETRO_SHOTS` names a folder for their pictures. The 3D tests draw into an offscreen
OpenGL context (`zax-render`'s `offscreen` feature) and skip without one.
