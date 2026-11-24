# Lot F3D: relief of the Famicom 3D System games in the default 3D

Branch `feat/famicom-3d-system`, worktree `wt-f3d` (from `dev` d02d6dd7). Owner asked (2026-11-24)
that the default 3D show these games with the relief their two eyes give, at the real depth (not
an exaggeration), and left the lot to R0 "until a concrete proposal".

## What the games do

The Famicom 3D System glasses read OUT1 of `$4016` (nesdev: 0 = the next field is for the left
eye, 1 = the right eye). The game draws the left picture, then the right one. Both are the same
scene shifted sideways: the shift (disparity) is the depth.

Measured on the owner's dumps (Hackset `[!]`):

- *Cosmic Epsilon* (Asmik, 1989, mapper 4): 3D switch `$6E` (`$FF`), shutter written at
  `$FF6F` in the NMI, scanlines 246-254. Disparity per line: floor 1 px at the bottom to 8 px at
  the horizon, city skyline 16 px, score bar 0, enemies about 0. Everything behind the screen.
- *JJ* (Square, 1987, mapper 2): 3D switch `$EF` (`01` or `$FF`, set from `$2A`), shutter
  written in vblank (lines 251-253). The checkerboard scrolls 3-6 px a field; once the scrolling
  is cancelled the disparity is 0 to 1.5 px. Little relief in the scene measured.
- *Attack Animal Gakuen* (mapper 2): shutter code found at `$C0D7`/`$C149`, not captured yet.

Both games write the bit during the vertical blank that starts the field, before the picture,
so the bit at the end of a core frame belongs to that frame's picture.

## What the lot does

1. `zax-nes`: `InputPorts` keeps OUT0-OUT2 of the last `$4016` write (transient, outside the
   savestate, format unchanged); `NesCore::expansion_output`, `EngineHandle::expansion_output`.
2. `zax-engine3d::stereo`:
   - `StereoTracker`: the last 4 fields (eye, tile records, picture copied only while the eyes
     alternate). Active after 6 alternations.
   - The measure, on a right field with a left field on each side: per line, the shift that best
     lines up the line (sprite boxes masked, lines without texture or with disagreeing
     comparisons dropped, outliers against ±6 lines dropped); per sprite slot, from the records.
     The mean of the two comparisons cancels the scrolling. Cached per right field.
   - `apply`: each shape gets the perceived depth `z = D·d / (e − d)` (D the viewer's distance,
     e the eyes' distance), a slope fit for a background shape over several lines (shear by
     rotation + stretch), and an enlargement `(c + z)/c` along the default camera's rays so the
     default view shows the picture unchanged.
   - `Viewing::default()`: the viewer at the default camera's place (189.65 px, eyes 61.6 px):
     the relief a player sees from there. `Viewing::television()`: 1.5 m of a 27 cm TV.
3. `zax-app` `GameSession`: when the profile is empty and the 3D is on (the "3D by default"
   entry) and the game alternates the eyes, the left fields only are converted and the relief is
   applied. Nothing changes for any other game.
4. Harness `render --rom`: same path; `ZAXNES_DEFAULT_3D` (empty profile as default 3D),
   `ZAXNES_F3D_OFF`, `ZAXNES_F3D_TV`, `ZAXNES_F3D_DEBUG`.

## Checks

- Unit tests in `stereo.rs` (detection, scrolling cancelled, perceived depth, interpolation,
  one measure per right field).
- `cargo test --no-fail-fast -p zax-engine3d -p zax-nes -p zax-harness`: 94 suites ok (with
  `rust/test-roms` copied from `cleanroom/impl`).
- Clippy `--no-deps` clean on `zax-engine3d`; the remaining clippy errors of `zax-app`,
  `zax-harness` and `zax-nes` are in files the lot does not touch (newer toolchain lints, also on
  `dev`).
- Cost: only while a 3D System game alternates the eyes (otherwise a 3-bit store per `$4016`
  write and one comparison per frame). Cosmic Epsilon, 1200 frames: about 0.75 ms per frame
  before the per-field cache.

## Open points (for the proposal)

- Options: on/off, and the viewer's distance (camera place / television).
- The 2D picture of a game in 3D mode still alternates the eyes (double image): show one eye.
- The games' 3D modes are turned on in their own menus (as on the console); not yet found in
  Cosmic Epsilon's menus (the test pokes `$6E`).
- Hand-held cost to measure (Retroid / RG40XXV).
