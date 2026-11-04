# SM-1 — SMAA with depth edge detection, `video.anti_alias` = 3 (AMD-060)

Worktree `wt-smaa`, branch `smaa-depth`, from `reworkoptions` at `1a1086c` (the close of RU-7).

Build directory: `rust/target-sm`. `CARGO_TARGET_DIR=<worktree>/rust/target-sm`, and the release
executable is built with `--features zax-app/bundled,zax-app/wgpu`.

## Checklist

| # | Step | Status |
|---|---|---|
| 1 | The reference and the specs read; the design (pass order, colour space, targets, tables, bind layouts); the test plan | done — this commit |
| 2 | The setting's domain 0..3, the options window, the Lua mapping, the persistence — no rendering yet | todo |
| 3 | OpenGL: the two tables and the depth edge detection | todo |
| 4 | OpenGL: the blending weights, the neighbourhood blending, the lifecycle, the fallback | todo |
| 5 | `wgpu`: the three passes, the lifecycle, the fallback | todo |
| 6 | The parity measurements and the costs | todo |
| 7 | Closing: notes, `PERFORMANCE-HISTORY.md`, fmt, clippy, full tests, release build | todo |

The hash of a step is written in the commit of the step that follows (no amend, protocol of this
lot).

## Notes

### State of the working copy at the start

`git status` clean, `git log` at `1a1086c`. No `*_tmp.rs` scratch probe and no untracked `tests/`
directory this time — the worktree is fresh, so RU-7's notes about the four uncommitted probes and
the leftover .NET build output do not apply here. Nothing to review, keep or discard.

`third_party/smaa/` **is present**: `SMAA.hlsl`, `Textures/AreaTex.h`, `Textures/SearchTex.h`,
`LICENSE.txt` and a `README.txt` recording the upstream commit
(`71c806a838bdd7d517df19192a20f0c61b3ca29d`) with a SHA-256 per file. The lot proceeds.

### What the reference computes, pass by pass

Read from `SMAA.hlsl` (MIT) rather than transcribed. `SMAA_RT_METRICS` is
`(1/width, 1/height, width, height)`.

1. **`SMAADepthEdgeDetectionPS`** — three point samples of the depth (the pixel, its left and its
   top neighbour), `delta = |P − (Pleft, Ptop)|`, `edges = step(SMAA_DEPTH_THRESHOLD, delta)`, and
   a `discard` when both are zero. Output: two channels, `r` the **west** edge and `g` the
   **north** one. The `discard` is why the edge target has to be cleared every frame.
2. **`SMAABlendingWeightCalculationPS`** — for a pixel with a north edge, the diagonal search
   (`SMAACalculateDiagWeights`, `SMAASearchDiag1/2`, `SMAAAreaDiag`) and, if no diagonal was found,
   the horizontal search (`SMAASearchXLeft/XRight` through the **search** table, then `SMAAArea`
   through the **area** table, then `SMAADetectHorizontalCornerPattern`); symmetrically for a west
   edge. Output: four channels, `rg` the horizontal pair and `ba` the vertical one.
3. **`SMAANeighborhoodBlendingPS`** — reads four weights (the right neighbour's `a`, the bottom
   neighbour's `g`, and its own `x` and `z`), early-outs when they sum under `1e-5`, else picks the
   axis with the larger weight, normalises the pair and mixes the pixel with the chosen neighbour.

The two tables are `AreaTex` 160 × 560 `R8G8` (`AREATEX_PITCH = 320`, 179 200 bytes) and
`SearchTex` 64 × 16 `R8` (1 024 bytes), both read with `SMAA_AREATEX_PIXEL_SIZE` /
`SMAA_SEARCHTEX_PACKED_SIZE`, i.e. in the table's own coordinate space and **not** through
`SMAA_RT_METRICS`. `SMAASearchLength` already carries a vertical flip of the search table
("The texture is flipped vertically"), which is part of the table's encoding and not a convention
of the host API.

Which sampler each source wants, which the reference states through its `LinearSampler` /
`PointSampler` split and which a port has to get right because nothing in the shader says so:
**depth point**, **edges / area / search / blend / colour linear**. The neighbourhood blending
"exploits bilinear filtering to mix current pixel with the chosen neighbor", and the horizontal and
vertical searches depend on bilinear reads of the edge target (`@PSEUDO_GATHER4`).

### The design

#### 1. `SMAA_RT_METRICS.y` is **negative** here, and that is the whole of the y convention

The one design decision that saves the port from the class of fault RU-7 met twice (the dither
mirror of its step 4b, the two row mirrors of its 4d). SMAA is written for Direct3D, where a
texture coordinate's `y` grows **downward**; an OpenGL framebuffer's grows upward. The algorithm is
**not** symmetric under a `y` flip: `SMAAArea(dist, e1, e2, offset)` splits a trapezoid's coverage
between the two pixels on either side of the edge, so swapping "up" and "down" mirrors the areas
and the result is subtly — and wrongly — displaced along every vertical edge.

Every place the reference crosses the screen in `y` does it as `± offset · SMAA_RT_METRICS.y`.
So passing `SMAA_RT_METRICS = (1/w, −1/h, w, h)` makes "north" mean `+y`, which in an OpenGL
texture **is** up on screen, and the whole of the reference's arithmetic then runs unchanged in the
host's own coordinate space. Nothing else flips: `pixcoord` and the `d` distances are taken through
`abs`, and the two tables are indexed in their own space.

The one thing that must follow from it: `SMAASampleLevelZeroOffset`'s integer texel offsets are in
the texture's own space and would **not** pick up the sign, so every offset read is written in the
arithmetic form the reference's own `SMAA_HLSL_3` path uses —
`texture(tex, coord + offset * SMAA_RT_METRICS.xy)` — and never as `textureLodOffset`. On the
`wgpu` side the framebuffer origin is already top-left, so `SMAA_RT_METRICS.y` is `+1/h` there and
the same sources are correct with one number changed. The two backends therefore differ in exactly
one uniform.

#### 2. The depth threshold: the reference's constant is unusable here, and the slope is the answer

`SMAA_DEPTH_THRESHOLD` is `0.001` on the raw window depth, and the reference says plainly that it
"depends on the depth range of the scene". Measured against this projection
(`geometry::NEAR_PLANE` 0.3, `FAR_PLANE` 5000, and the Direct3D-style `[0, 1]` clip depth that
makes the drawn geometry occupy the **upper half** of the buffer, `screen_depth.glsl`):
`w ≈ 1 − 0.15/d`, so `dw/dd = 0.15/d²`. At the default reset distance of 190 scene units that is
`4.2 · 10⁻⁶` per unit — the reference's `0.001` would ask for a depth step of **240 scene units**,
where the whole NES picture is 256 × 240 and a voxel is **one** unit. At `d = 10` the same constant
means 0.67 units. A fixed threshold on the window depth is therefore not a threshold at all here:
it finds everything near the eye and nothing at the camera distances Zaxnes actually uses.

What replaces it, and why it is the faithful rather than the convenient answer:

- **The window depth is exactly linear in screen space across a planar surface.** That is a
  property of the perspective projection, not an approximation. So the first difference of the
  window depth is **constant** over a plane, at any angle and any resolution, and the test "is this
  step larger than the local slope?" is exact: it never fires inside a flat face. This is precisely
  AMD-060's own acceptance criterion ("les pixels intérieurs à la face sont identiques à l'octet").
  The slope is read from the pixel two away — `offset[2]` of `SMAAEdgeDetectionVS`, which the
  reference already computes and which only its luma detector uses.
- **The absolute floor is expressed as a relative distance jump**, so it is scale- and
  distance-free. From `w ≈ 1 − 0.15/d` comes `0.15/d ≈ 1 − w`, hence a jump of `ε · d` in distance
  is a jump of `ε · (1 − w)` in window depth. The floor is therefore
  `ZAX_SMAA_DEPTH_RELATIVE · (1 − w)`, with `1 − w` read straight off the sample.

So: `threshold = ZAX_SMAA_DEPTH_RELATIVE · (1 − w) + ZAX_SMAA_DEPTH_SLOPE · slope`, with the
reference's `step(threshold, delta)` and its `discard` unchanged. Both constants are constants of
the shader file, as `ao_blur.frag`'s `ZAX_AO_BLUR_SHARPNESS` is, and for the reason that file gives:
REQ-SCN-085 fixes the *detector* (the depth, never the luma or the colour) and leaves its
sensitivity free. Starting values 0.002 and 1.5, to be settled by the measurements of step 3.
**Q-109** records the departure from the reference's constant and the reasoning.

The cost against the reference's own detector: two extra point samples of the depth per pixel, at
offsets the reference's vertex shader already provides.

#### 3. Where the three passes go in the frame, and in which colour space

The order REQ-SCN-085 leaves to the implementation. What the frame does today
(`renderer.rs::render_scene`) and where SMAA lands in it:

| | |
|---|---|
| 1 | the sky and the opaque objects, into the linear `RGBA16F` buffer of AMD-021 R2-009 |
| 2 | the screen passes of AMD-055 REQ-SCN-080 — the reflections, then the occlusion |
| 3 | the transparent objects of REQ-SCN-040 |
| **3a** | **the three SMAA passes** |
| 4 | the HUD of REQ-SCN-044 |
| 5 | the sRGB encode of SPEC-06 §2, one full-screen triangle into the host framebuffer |

**After the screen passes** because SMAA smooths the frame as it will be seen, and the occlusion
multiplies the colour in place: a reflection or a darkening laid on top of an already-blended edge
would undo the blend. **After the transparent objects** because they are scene geometry with
silhouettes of their own, which MSAA smooths at levels 1 and 2 and which nothing in AMD-060
excludes. **Before the HUD** because REQ-SCN-085 says the 2D elements superimposed on the scene
"NE DOIVENT PAS être traités", and in this renderer the HUD is not a flat overlay: it is a plane of
**extruded tile boxes** placed by `interop::hud_matrix` (`frame_plan.rs`), so adjacent HUD tiles sit
at different depths and a depth detector run after it would find edges *inside* the status bar and
blend its pixel art — exactly the objection AMD-032 §2 raised against post-processing filters. It is
also the order AMD-055 REQ-SCN-080 already chose for the same reason, which RU-7 measured as "no
effect of the lot moves the HUD by one level".

The menus, the notifications and every other overlay are drawn by the UI layer **after** step 5,
into the host framebuffer, so they are outside the renderer's reach and untouched by construction.

**In linear space, before the sRGB encode.** Three reasons, in the order they decide it:

1. SMAA's weights are **coverage fractions**, and a coverage-weighted average of two surfaces is an
   average of radiance. Linear is the physically correct space for it; gamma space is what the
   reference does because it is handed an already-encoded LDR image, which is a practical
   compromise and not a principle.
2. The frame already blends in linear (AMD-021 R2-009), and REQ-SCN-083's composite says the same
   thing in the same words — "en espace linéaire, avant l'encodage sRGB".
3. Levels 1 and 2 resolve MSAA samples of the same linear `RGBA16F` buffer, so a linear blend makes
   level 3's edges the same *kind* of edge as the two levels it sits beside. A gamma-space blend
   would make 3 look different from 1 and 2 in a way no requirement asks for.

#### 4. The targets, and the one thing the third pass cannot do in place

The neighbourhood blending reads the frame's colour and writes a mix of two of its texels, so —
unlike REQ-SCN-083's composite, which is a blend state — it **cannot** write into the texture it
reads. So it writes into an `RGBA16F` target of its own, with the frame's depth attached, and
**leaves that framebuffer bound**: the HUD then draws into it with no change to any draw path, and
the only thing the rest of the renderer has to be told is that the sRGB encode must sample that
texture instead of the frame's own. One `Option<Texture>` on `LinearColorBuffer`, read in
`resolve_to` and cleared in `begin_pass`.

| Target | Format | Size | Bytes/texel |
|---|---|---|---|
| edges | `RG8` | the frame's | 2 |
| blend weights | `RGBA8` | the frame's | 4 |
| the blended colour | `RGBA16F` | the frame's | 8 |
| the area table | `RG8` | 160 × 560 | — (179 200 total) |
| the search table | `R8` | 64 × 16 | — (1 024 total) |

14 bytes per pixel of the frame, 29.0 MB at 1080p, plus 180 KB of tables that do not grow with the
resolution. **MSAA 4× costs 4 × (8 + 4) = 48 bytes per pixel** of extra attachments, 99.5 MB at
1080p: level 3 is a third of level 1's memory and one sample of geometry instead of four, which is
AMD-060 §1's own argument made concrete.

Every format is GL 3.3 core **and** GLES 3.0 colour-renderable without an extension: `RG8` and
`RGBA8` are in the required colour-renderable list of both, and `RGBA16F` is the one the linear
buffer has always used (on GLES 3.0 through `EXT_color_buffer_float`, which the targeted platforms
have — `linear_buffer.rs` already depends on it). The depth is the `DEPTH_COMPONENT24` **texture**
`begin_pass(sample_depth = true)` already allocates for AMD-055, so level 3 asks for nothing the
Enhanced style does not already ask for.

#### 5. The tables are committed as binary, and pinned by their hash

`third_party/` is **not** tracked by this repository (`git ls-files third_party` is empty), so a
build script cannot read `AreaTex.h` at build time on a machine that has only the repository. The
two tables are therefore extracted once and committed as the raw bytes the headers hold, in the
`R8G8` and `R8` order the headers state, with a test that pins each blob's length and SHA-256 — so
"reproduce the two tables byte for byte" is a checked property and not a claim. The extraction
command and the hashes go in this file and in `THIRD-PARTY-NOTICES.md`, beside the MIT notice.

#### 6. The lifecycle, the statistics and the fallback

A `gl/smaa.rs` module held as an `Option` by `SceneRenderer`, exactly as `ScreenEffects` is and for
the reason that file's header gives: at `video.anti_alias` ≠ 3 the whole cost is the `is_none` test
that decides not to run it, and no program, target or table exists. Built on the first frame at 3,
dropped on the first frame that is not — no restart, which is SPEC-09 §4.1.1's "imm." effect.

The fallback of REQ-SCN-086 is `ScreenEffects`' own, line for line: a program that will not link or
a target the driver refuses gives up the mode for the session, writes **one** diagnostic line
(`RenderDiagnostics::smaa_unavailable`, the twin of `screen_effects_unavailable`) and the frame is
drawn without anti-aliasing. A test seam (`fail_smaa`) reaches it on a machine whose driver refuses
nothing, for the reason RU-5 and RU-7 both gave: an unreachable fallback is an untested one.

Two statistics, `smaa_passes` and `smaa_bytes`, and they are written into `render.json` **only when
`video.anti_alias` = 3** — the shape AMD-055's four keys already use ("a capture at the default
settings produces the entry of before the amendment, byte for byte"), which is what keeps the
levels 0, 1 and 2 captures byte-identical down to the JSON.

#### 7. What the `wgpu` side needs that OpenGL does not

From RU-7's experience, settled before any code:

- `SMAA_RT_METRICS.y` is `+1/h` there (top-left origin), which is the single numeric difference.
- The three passes are three render passes, not three framebuffer binds, so `smaa_passes` reads the
  same 3 on both backends; the depth resolve of RU-7 is **not** involved, because level 3 is
  single-sample by REQ-SCN-085 and there is nothing to resolve.
- `discard` exists in WGSL, so the edge pass keeps it, and its target is cleared by the pass's
  `LoadOp::Clear`.
- Two samplers (one `Nearest`, one `Linear`) and five texture bindings in one bind-group layout.
- The blended target is a colour attachment with the frame's single-sample depth attached read-only,
  so the HUD pass that follows loads both.

### The test plan

| # | What it checks | Where |
|---|---|---|
| 1 | the domain 0..3: persistence, the field-32 round trip, the clamp of out-of-range values, the four labels in each UI language, `Setting.Msaa` never writing 3 and reading 3 as "no multisampling", the Lua API golden unchanged | `zax-settings`, `zax-profiles`, `zax-app` and `zax-scripting` tests (step 2) |
| 2 | the two tables byte for byte: length and SHA-256 of each blob | `zax-render/tests/amd060_smaa_tests.rs` |
| 3 | the three passes are in the GLSL and in the WGSL, with the reference's constants, the `discard`, the two tables' metrics and the **negative** `y` metric on the GL side only | the same file, over `shaders::assemble` and `wgpu_backend::expand_wgsl` |
| 4 | AMD-060's acceptance criteria: strictly more distinct colours at 3 than at 0 on a scene with hard geometric edges, **and** the interior of a single textured face byte-identical to the capture at 0 | `zax-harness/tests/amd060_smaa_tests.rs` |
| 5 | "≠ 3 does nothing": the renders at 0, 1 and 2 byte-identical to `1a1086c`, and `smaa_passes` / `smaa_bytes` / the program count at 0 | the same file + a worktree comparison in step 6 |
| 6 | allocate on the first frame at 3, release on leaving it, no restart | `zax-render/tests/amd060_smaa_tests.rs` |
| 7 | the fallback: refused, drawn without anti-aliasing, said once, never retried | the same file |
| 8 | the HUD is not treated: not one of its pixels moves between 0 and 3 | `zax-harness/tests/amd060_smaa_tests.rs` |
| 9 | parity, as RU-7 measured it: both backends on the same scenes, share of pixels within 2 levels and max channel difference, Vulkan and DX12 | `zax-harness/tests/parity_wgpu_tests.rs` |

### The "before" figures (step 1)

To be taken at the head of step 2, so that step 7 can show the OpenGL path did not move: the four
post-LTO instruction counts RU-7 tracked (`Ppu2C02::run_dots` 2231, `DrawListBuilder::build` 1095,
`SceneRenderer::render_scene` 3401, `Construction::segment` 159) and the per-crate test counts.

### What is left for R0

Nothing yet. The closing section of AMD-060 is R0's; what R0 needs will be gathered here in step 7.
