# V2-ENHANCED-A — progress

Session: clean side (R3), branch `dotnetupdated`. Two lots, in order: **V2-17** then **V2-18**.
Normative text: `spec/changes/AMD-055-style-enhanced.md` v1.3 (+ R2 reviews v1.0/1.1/1.2).

Resume rule: continue from the first unchecked step. Never redo a step marked done.

## Lot V2-17 — Enhanced style: settings, voxel grid, pixel shadows

Scope: REQ-SCN-078 to REQ-SCN-082, the amended REQ-SCN-058 (frame order, style part),
REQ-UI-061, REQ-SCN-075 (AMD-049 sentence), REQ-LUA-114 / AMD-040 / AMD-043 doc changes.
Out of scope: REQ-SCN-083 (V2-18) and REQ-SCN-084 (next session).

- [x] 1/7 Settings: the 13 SPEC-09 lines of AMD-055 §4 — `render.visual_style` domain
      (0 Classic, 4 Enhanced, 1 accepted and read back 4, anything else 0) and the twelve
      `render.style.*` keys; `VisualStyleParameters` for style 4 (12 + 4 curvature ids) with
      1 accepted; not exposed in libretro. Tests in `Zax.Settings.Tests`. — **done**, 7f38c22
- [x] 2/7 Lua and app settings: `Setting.VisualStyle` reads back 4, `Get/Set/Reset` accept 4
      and 1, `AppSettings` accessors for the twelve parameters. — **done**, 7f38c22
- [x] 3/7 UI (amended REQ-UI-061): the `Style` row after `Skybox`, the collapsible « Style »
      group with the twelve parameters, visibility rules (`spl.ao` ≠ 0, `spl.reflections` = 1),
      `ao.thickness` greyed under the Depth fallback, EN/FR texts of §4. — **done**, 7f38c22
- [x] 4/7 Renderer settings and host wiring: `RenderSettings.Style`, hot apply at the next
      rendered frame, Enhanced with every effect off = Classic. — **done**, 7f38c22
- [x] 5/7 Voxel grid (REQ-SCN-081): shader modulation of the albedo for tiles (local mesh
      frame) and backdrop (scene frame), before lighting; `BuildDepth` plumbed to the renderer.
      — **done**, see 6/7
- [x] 6/7 Pixel shadows (REQ-SCN-082): shadow evaluated at the voxel centre of the fragment,
      opaque tiles only. — **done**, a11341e
- [x] 7/7 Acceptance tests, AMD-055 + R2 reviews copied to `docs/spec/changes/`, journal
      entry, lot commit. — **done**, lot commit below

## Lot V2-18 — Enhanced style: screen-space ambient occlusion

Scope: REQ-SCN-083 in Depth mode, the screen-pass part of the amended frame order.
MSVO is rendered as Depth — declared fallback **(D)**: the MiniEngine sources are in
`third_party/MiniEngine-SSAO`, but the OpenGL 3.3 port itself is a later lot.

- [x] 1/5 `LinearColorBuffer`: depth as a sampleable texture, with the MSAA depth resolve.
      — **done**, 1c0mmit
- [ ] 2/5 AO pass: hemisphere SSAO from depth (Chapman 2013 as the orienting reference),
      per-pixel kernel rotation, range check, depth-aware blur; radius, strength ceiling and
      quality of REQ-SCN-083; applied as a linear multiply before the sRGB encode.
- [ ] 3/5 Frame order (amended REQ-SCN-058): after the opaque objects, before the sorted
      transparent ones; scope rules (no skybox, no HUD, no transparent objects).
- [ ] 4/5 Acceptance tests: the close-up A invariants.
- [ ] 5/5 Journal entry, lot commit.

## Notes

- Start state: clean tree at `65ff2bf` (V2-16), no previous work on these lots.
- Highest Q ticket in `spec/changes/` at the start: `Q-091`. Next free number: `Q-092`.
- MARCHING already carries flat per-triangle normals in the vertex attributes
  (`TileMesh.AddTriangle`), so `aNormal` is the flat normal REQ-SCN-081 and REQ-SCN-082 ask for
  in both algorithms.
- The scene scale `s` is 1 on desktop and the renderer keeps positions in unscaled scene units,
  so the backdrop cell side `N × s` applied to `p = scene × s` reduces to `N` on the raw scene
  position.
- There are no tunnel and no TV models to exclude from the effects: AMD-031 removed them.
- Steps 1 to 4 landed in one commit: the acceptance tests of AMD-040 and AMD-043 that assert
  `Setting.VisualStyle = 1` → 1 and « no style on the renderer » had to be amended at the same
  time, so there was no green intermediate state between them.
- `SettingDomain.AliasedChoice` is new: it resolves a written alias before the list is
  consulted, which is what REQ-SCN-078 asks of 1 → 4.
- Steps 5 and 6 share one commit: both effects need the same two new varyings (the raw mesh
  position and normal), so splitting them would have left the shader half-wired.
- `uGridCell` = 0 is the "grid off" signal and is read by every draw call, so `zaxVoxelGrid`
  leaves early on it: the branch is uniform and `fwidth()` in `zaxGridAxis` stays valid.
- The two tick-box parameters stay whole numbers (SPEC-09 type « entier », domain 0 à 1) and
  the panel draws them with `DrawTickBox`, as AMD-055 §4 asks ("case").

### V2-17, step 7 (acceptance tests)

- Tests: `tests/Acceptance/Amd055Tests.cs` (5, no GL: the criteria of REQ-SCN-078 and
  REQ-SCN-079) and `tests/Acceptance/Amd055CaptureTests.cs` (9 GL captures: REQ-SCN-080,
  REQ-SCN-081 close-up G and wide framing B, REQ-SCN-082).
- `SceneFixture.Pattern` took three new optional arguments (`texture`, `scale`, `rotation`,
  then `offset`) and a `FlatDirectTexture` helper: the close-ups need a custom texture in the
  direct format and the object transform of the third criterion of REQ-SCN-081.
- The cameras of the close-ups are placed in **FPV** (REQ-SCN-009): TPV re-centres the target
  on the visible rectangle and aims at a point of the `z` = 0 plane (REQ-SCN-007, -008), so
  « camera at (x, y, z), aiming at the centre » is not reachable with it. Measured points are
  found by projecting through the view the renderer used, never by a hard-coded pixel.
- The engine places a layer-L1 shape of depth 8 on `z` ∈ [0, 8], not [−4, 4] (front alignment,
  REQ-E3D-205); the fixtures pass `offset.z = −4` to put the block where the close-ups say it
  is. `Offset.X/Y` move the tile centre, `Offset.Z` the shape's own centre.
- `SettingsModel.Set*` returns whether the value **changed**, not whether it was accepted, so
  a test that writes an out-of-domain value must assert on the read-back alone.
- The groove lines of the wide framing B are followed two columns at a time, with the window
  centred on the row the previous slope predicts and the centre taken as a darkness-weighted
  centroid: at 90° the line climbs by more than a pixel per column near the edges of the face,
  and the criterion wants it straight to less than one pixel at zero curvature.
- **Q-092** written: (a) the four points of the close-up G are given both as « the tile adjacent
  to the centre of the face » and as the tile centred on (132, 132), which are two different
  tiles; the coordinates were followed and the choice is unobservable. (b) REQ-SCN-082 says the
  shadow edges fall « in the middle of a voxel », which holds for the two **near** edges only:
  the plate is one voxel thick, its upper face casts 10 units instead of 29/3, so the far edges
  land exactly on +14 — which is what the criterion's own « 6 and 14 » boundary needs.
