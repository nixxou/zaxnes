# V2-ENHANCED-B — progress

Session: clean side (R3), branch `dotnetupdated`. Two lots, in order: **V2-19** then **V2-20**.
Normative text: `spec/changes/AMD-055-style-enhanced.md` v1.3 (+ R2 reviews v1.0/1.1/1.2).
Previous session: `docs/progress/V2-ENHANCED-A.md` (V2-17 at 9258baf, V2-18 at 556e5bc).

Resume rule: continue from the first unchecked step. Never redo a step marked done.

## Lot V2-19 — Enhanced style: screen-space floor reflections

Scope: REQ-SCN-084 (floor definition, parameters, invariants) and the remaining screen-pass
part of the amended frame order of REQ-SCN-058.

- [x] 1/5 Settings and UI check: the four `ssr.*` keys, the `Reflections` tick box and the
      visibility rule (`spl.reflections` = 1) landed with V2-17; confirm, do not redo.
      — **done**, nothing to change (see Notes).
- [x] 2/5 Floor mask: the scene framebuffer gets an optional second attachment carrying the
      pre-curvature floor flag and the bent scene normal; the renderer computes the per-instance
      `xz` extent of REQ-SCN-084 and the scene shader writes the mask. — **done**, with 3/5
- [x] 3/5 `ReflectionPass`: screen-space ray trace (linear march then bisection), the distance
      blur, the two attenuations and the linear blend; frame order (after the opaque objects,
      **before** the occlusion). — **done**, see commit below
- [x] 4/5 Acceptance tests: the close-up R invariants and the variants R-m, R-s, R-h.
      — **done**, lot commit below
- [x] 5/5 AMD-055 + R2 reviews in `docs/spec/changes/` (LF), journal entry, lot commit.
      — **done**, lot commit below (the four files were already there and are byte-identical
      to `spec/changes/`, LF)

### V2-19 (floor reflections)

- Step 1: `render.style.ssr_*`, the `Reflections` tick box, the EN/FR texts and the visibility
  rule were all delivered by V2-17 (`SettingsCatalogue`, `OptionsModel`, `UiStrings.*`,
  `EmulationHost.ApplyStyle`). Nothing to add for this lot.
- `LinearColorBuffer` gained an **optional second colour attachment**, the floor mask: the bent
  scene normal in `rgb`, the floor flag of REQ-SCN-084 in `a`. It is allocated only for the
  frames whose reflections are on, and its draw buffer is turned on **around the opaque pass
  alone** (`SetFloorMaskTarget`) — so the skybox, the transparent and the HUD passes need no
  second fragment output, and what they write cannot land in it.
- The mask is cleared to zero by a second clear with the colour attachment masked out, rather
  than by `glClearBufferfv`: the background colour of REQ-SCN-046 has already been written at
  that point and must survive.
- The floor **extent** test (box of at least 6 x s units in both `xz` dimensions) is computed
  per object on the CPU, on the runs of tiles that share one instance — the objects of AMD-012 —
  with `GrowBounds`, which transforms the local box by the placement matrix alone and is
  therefore exactly the « avant courbure » the requirement asks for.
- The floor **facing** test is in the scene shader, on a new varying holding the scene normal
  **before** the curvature; the normal *stored* in the mask is the bent one, because the ray is
  traced on the geometry as rendered (REQ-SCN-080, second bullet).
- `ReflectionPass` + `ReflectionShaders`: four fragment passes — trace, two separable blur
  passes, linear mix. The march is in view space, one uniform world step at a time, each step
  projected; the refinement is a bisection on the same crossing test (McGuire and Mara 2014 as
  the orienting reference). The blur radius is read per pixel from a travel buffer the trace
  writes, so it grows linearly with the distance travelled as REQ-SCN-084 asks.
- The reflection is stored **premultiplied** by its intensity, so the blur can mix a reflecting
  pixel with a non-reflecting one without dragging a black fringe in; the mix is a
  `ONE, ONE_MINUS_SRC_ALPHA` blend on the colour and `ZERO, ONE` on the alpha, which is never a
  colour (REQ-SCN-035).
- A pixel that reflects nothing has travel 0, hence blur radius 0, hence every tap on itself:
  the blur cannot spread a reflection outside the floor, which is what the scope invariants of
  R-m and R-h need.
- `SceneShaders.ScreenDepth` is the depth reconstruction, now shared by the occlusion and the
  reflections.
- Frame order: reflections **then** occlusion (REQ-SCN-058 amended), so the occlusion darkens a
  reflection already laid down and never the other way round.
- **Q-093** written: (a) REQ-SCN-084 defines a floor surface through « une instance
  (SPEC-04B §2) », and the backdrop is not one, yet the scope bullet lists what is never a floor
  and leaves the backdrop out; the backdrop is treated as an object of its own (with its default
  size its `xz` box is 256 x 1, so the question is moot). (b) the scope bullet says the skybox
  « est reflétée » when already drawn, but it is at infinite distance behind the whole scene, so
  a ray of finite maximum length never crosses it: in practice the sky is not reflected.
  (c) the `ssr.clarity` invariant is not measurable in the close-up R itself — see below.

#### V2-19, step 4 (acceptance tests)

- `tests/Acceptance/Amd055ReflectionTests.cs`, 8 tests for the six automatic invariants of §2.5.
- The march's **sample positions must not depend on `ssr.length`**, or the invariant « a longer
  ray lengthens the modified zone » fails: a first attempt with a span of `L / steps` made the
  near field coarser as the ray grew and *lost* reflections at length 2. The progression is now
  geometric, fitted on the **longest** ray any setting can ask for (360 × s), so a longer ray
  only adds samples at the far end.
- « The modified zone » is measured as the pixels the reflection raises by at least **half** of
  what it raises the most. Counting every pixel that changes at all does not move: the faint
  fringe is already there at `ssr.length` = 1.
- The HUD of the variant R-h is put on the **cell row 2**, not at the centre of the frame: the
  HUD plane of REQ-SCN-045 is 109 units in front of the eye and the camera of the close-up R is
  only 40 units from the floor, so a motif at the centre lands behind the floor and is hidden.
- The block of the variant R-s is identified in the capture by its **grey** pixels (the rest of
  that variant is a pure red cube and a black skybox), which avoids depending on how a scale of
  0.5 on `z` moves the shape's own centre.
- `ssr.clarity` is measured on a declared variant **R-c** (the cube raised 12 units), see
  **Q-093** (c): in the close-up R the cube sits *on* the floor, so every ray that reaches it has
  travelled some 5 units out of 200 and the blur radius — proportional to that travel — stays
  under a third of a pixel, and is exactly zero where the reflection is strongest. Measured
  widths on R-c: 2.482 / 2.460 / 2.340 / 2.145 / 2.048 pixels for clarity 1 to 5.
- Test results: `Zax.Render.Tests` 162/162; `Zax.Acceptance.Tests` filtered to
  render/scene/settings 296 passed, 1 skipped (the Q-062 stress test, skipped as usual).
- One visual check: the reflection of a floating red cube is recognisable on the floor, with no
  hard edge at the end of the ray or at the edge of the screen.

## Lot V2-20 — Enhanced style: multi-scale ambient occlusion (MSVO)

Scope: the MSVO mode of REQ-SCN-083 (`spl.ao` = 1), ported from `third_party/MiniEngine-SSAO`
(MIT) and adapted to OpenGL 3.3 fragment passes, with the `ao.*` parameters and the invariants
of the amendment; the Depth fallback **(D)** for that mode is removed.

- [x] 1/4 Port: the MiniEngine multi-scale occlusion as OpenGL 3.3 fragment passes (depth
      pyramid, per-scale occlusion, blur and upsample), with the MIT attribution header.
      — **done**, lot commit below
- [x] 2/4 Wiring: `spl.ao` = 1 selects it, `ao.thickness` acts in that mode only, the greying
      rule of the UI follows, `ao.radius` / `ao.strength` keep their normative meaning.
      — **done**, lot commit below
- [x] 3/4 Acceptance tests: the close-up A invariants in MSVO mode, including the
      `ao.thickness` criterion that replaces the fallback identity. — **done**, lot commit below
- [x] 4/4 Journal entry, lot commit. — **done**, lot commit below

### V2-20 (multi-scale occlusion, MSVO)

- `src/Zax.Render/Gl/MsvoShaders.cs` and `MsvoPass.cs`, both carrying the MIT attribution
  header of `third_party/README.md`. Four fragment passes: linear-depth pyramid, per-level
  occlusion, bilateral upsample chain, then the **same** apply pass as the Depth mode, so the
  ceiling `O ≤ 0.1 × ao.strength` is enforced in one place.
- Kept from the MiniEngine: the pyramid of linear depths; the sphere-penetration test
  (`TestSamplePair` / `TestSamples`) with its thickness table, its per-offset weights and its
  rejection fall-off; the checkerboard of offsets and the axial / diagonal / L-shaped grouping;
  the bilateral upsample that multiplies each level into the next finer one.
- Adapted: OpenGL 3.3 has no compute shaders and no group-shared memory (REQ-SCN-056), so each
  pass samples the depth textures directly instead of a cached tile; the original's sphere has a
  fixed radius of five **pixels** per level while REQ-SCN-083 binds `r` in **scene units**, so
  the offsets are scaled per pixel by `r × focal / depth` and the levels take r/4, r/2 and r;
  the depth is kept in view units, which makes the test exactly `0.5 + Δz / (2 r T)`; the
  accentuation is left at 0, since the first invariant wants a lone plane untouched; and the
  upsample blur is a 3 × 3 depth-weighted tent instead of the group-shared `SmartBlur`.
- `ao.thickness` is the rejection fall-off, 1 to 10 (`StyleSettings.OcclusionThickness`): a
  greater value rejects fewer near samples, so the darkening can only grow with it — the
  monotony REQ-SCN-083 requires holds by construction.
- `OcclusionPass` now answers the **Depth** mode alone; `MsvoPass` answers MSVO. The renderer
  tries the MSVO pass first and falls through to the Depth one.
- `OptionsModel.MsvoPortAvailable` is now true and `IsInert` greys `ao.thickness` exactly when
  `spl.ao` is not MSVO, which is what AMD-055 §4 asks ("grisé sauf en mode MSVO effectif"). The
  EN/FR hint texts are unchanged and still read true.
- Tests: the fallback-identity test of V2-18 is replaced by the MSVO form of the fourth
  invariant (`ao.thickness` moves at least one pixel by more than 2 sRGB levels, and the contact
  ratio does not grow), plus three tests for the *Conformité* clause — the invariants of §2.4,
  the scope (skybox and HUD), and the three qualities — in MSVO mode. 11 tests in
  `Amd055OcclusionTests`.
- Test results: `Zax.Render.Tests` 162/162; `Zax.App.Tests` 488/488; `Zax.Acceptance.Tests`
  filtered to render/scene/settings 305 passed, 1 skipped.
- One visual check: at `ao.strength` = 5 the darkening around the foot of the cube is wide and
  progressive, with no hard contour, no band and no fixed noise pattern.

## Notes

- Start state: clean tree at 556e5bc (V2-18), no previous work on these two lots.
- Highest Q ticket in `spec/changes/` at the start: `Q-092`. Next free number: `Q-093`.
- AMD-055 and the three R2 reviews are **already** in `impl/docs/spec/changes/` (V2-17), so
  step 3 of each lot's instructions is a check, not a copy.
- The `third_party/README.md` entry « MiniEngine-SSAO (MIT) » already exists (V2-18); what the
  fallback of REQ-SCN-083 depends on is the *port*, which is lot V2-20.
