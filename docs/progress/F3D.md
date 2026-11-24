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
  before the per-field cache, 0.3-0.6 ms after (noisy, desktop).
- `zax-app` with `bundled`: 73 suites ok; `req_pub_002` fails only because the lot's target dir
  `wt-f3d/target-f3d` sits inside the worktree (the asset scan finds its `.obj`). Use a target
  dir outside the worktree for the full run.
- App smoke test (`--profile "3D par défaut" --smoke-emulated`) on Cosmic Epsilon: runs.
- Scale: the TV preset (1.5 m) puts the city about 500 px deep, out of frame as soon as the
  camera turns; the default is the viewer at the camera's place (city 66 px, horizon 28 px).

## Proposal (2026-11-24)

Shown to the owner as a page (renders before/after from the harness, demo folder
`demo-f3d`, next to the worktrees). Decisions asked: default scale (camera place recommended), an option in
the menus (on/off + scale), one eye in the 2D picture of a game in 3D mode, leave the game's own
3D switch to the player, handheld measure and merge on the owner's word.

## Open points (for the proposal)

- Options: on/off, and the viewer's distance (camera place / television).
- The 2D picture of a game in 3D mode still alternates the eyes (double image): show one eye.
- The games' 3D modes are turned on in their own pause menus, as on the console: in both
  *Cosmic Epsilon* (`$A969`, toggles `$6E`) and *JJ* (`$89C8`, increments `$2A`), START pauses,
  SELECT switches the 3D, START resumes. The harness runs poke the switch instead.
- Hand-held cost to measure (Retroid / RG40XXV).

## Owner test 2026-11-24 evening: JJ looks worse in 3D than in 2D

- The owner's captures were *JJ*, played in the app (START, SELECT, START in its pause menu).
- Reproduced in the harness: `--press 600-604:start,750-754:start,900-904:start,1300-1304:start,1340-1344:select,1380-1384:start,1600-1795:right --run 1797` with `ZAXNES_DEFAULT_3D=1`, angles 25,30.
  - Before this run, the four starts put the game in play with `$EF = $FF`.
- The measure is right. While playing, the right eye's sky is the left eye's shifted 19-20 px, with an exact match. The floor goes from about 18 px at the horizon to 0 at the bottom. The 0-1.5 px of the first notes was measured on the title or demo, not in play.
- The rendering is wrong. `apply` moves each shape as one rigid block: one depth, one tilt, one enlargement taken at its middle line. The checkerboard's large greedy shapes then get different tilts and widths, so the floor breaks into steps. A slanted floor needs a trapezoid, which a rigid instance transform cannot give.
- The fix that works is per vertex. A "stereo bend" applied last in the vertex shaders, like `space_bend`: z(y) from a per-line table, and x/y enlarged by (c + z)/c about the camera axis.
  - It applies to the background instances only; sprites keep their own measured offset.
  - It needs a uniform table (240 floats) and a per-instance flag, in the GL and wgpu scene and shadow programs.
- Done 2026-11-24 evening, with the owner's go:
  - The background is moved vertex by vertex. `StereoRelief` gives the depth of each of the 240 rows: a median over ±6 lines, then blended at 0.35 into the rows already shown so the floor does not shiver. The frame carries it, and so does the draw list.
  - The renderer sends it as `uStereoHead` and `uStereoDepth[60]` on GL, and as `stereo_head` and `stereo` in the wgpu frame block (`FRAME_BYTES` 304 → 1280).
  - The flag is `deform_tile.w` of each placement: a background tile in the scene, not the HUD.
  - The scene shaders (`zaxStereo`, `zax_stereo`) move the vertex and carry the normal through the cofactor matrix. `vScene` stays flat for the clipping.
  - `apply` now moves the sprites only.
  - Such a background tile neither casts nor receives a shadow: the shadow map and its lookup work on flat positions. The sprites keep their shadows.
  - Checked: JJ and Cosmic Epsilon from 3 angles, GL and wgpu dx12 (the harness `--features wgpu`). Tests: `zax-engine3d` and `zax-render` (37 suites) pass. Target dir `G:\3DSen\tgt-f3d`, outside the worktree.
- Seen on the way: the default 3D floor of JJ is jagged even without the relief. The tile records hold one scroll per 8-line band, and JJ scrolls its floor line by line. This is a general limit of the default 3D on raster floors, not something this lot introduced.
