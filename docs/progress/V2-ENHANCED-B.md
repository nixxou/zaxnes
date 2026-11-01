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
- [ ] 4/5 Acceptance tests: the close-up R invariants and the variants R-m, R-s, R-h.
- [ ] 5/5 AMD-055 + R2 reviews in `docs/spec/changes/` (LF), journal entry, lot commit.

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

## Lot V2-20 — Enhanced style: multi-scale ambient occlusion (MSVO)

Scope: the MSVO mode of REQ-SCN-083 (`spl.ao` = 1), ported from `third_party/MiniEngine-SSAO`
(MIT) and adapted to OpenGL 3.3 fragment passes, with the `ao.*` parameters and the invariants
of the amendment; the Depth fallback **(D)** for that mode is removed.

- [ ] 1/4 Port: the MiniEngine multi-scale occlusion as OpenGL 3.3 fragment passes (depth
      pyramid, per-scale occlusion, blur and upsample), with the MIT attribution header.
- [ ] 2/4 Wiring: `spl.ao` = 1 selects it, `ao.thickness` acts in that mode only, the greying
      rule of the UI follows, `ao.radius` / `ao.strength` keep their normative meaning.
- [ ] 3/4 Acceptance tests: the close-up A invariants in MSVO mode, including the
      `ao.thickness` criterion that replaces the fallback identity.
- [ ] 4/4 Journal entry, lot commit.

## Notes

- Start state: clean tree at 556e5bc (V2-18), no previous work on these two lots.
- Highest Q ticket in `spec/changes/` at the start: `Q-092`. Next free number: `Q-093`.
- AMD-055 and the three R2 reviews are **already** in `impl/docs/spec/changes/` (V2-17), so
  step 3 of each lot's instructions is a check, not a copy.
- The `third_party/README.md` entry « MiniEngine-SSAO (MIT) » already exists (V2-18); what the
  fallback of REQ-SCN-083 depends on is the *port*, which is lot V2-20.
