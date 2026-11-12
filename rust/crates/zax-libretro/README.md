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

The **3D** of the Zaxnes application is not in the core yet (step 2), and neither is the editor
mode (the application's only).

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

`src/core.rs` (the game in play) and `src/options.rs` (the core options) are safe Rust. `src/ffi.rs`,
the C interface of `libretro.h`, is the only `unsafe` code of the crate.

Tests: `cargo test -p zax-libretro --profile libretro`. They play the FlappyZaxNes demo, and with
`ZAXNES_LIBRETRO_ROMS` (folders separated like `PATH`) Kirby's Adventure and The Legend of Zelda;
`ZAXNES_LIBRETRO_SHOTS` names a folder for their pictures.
