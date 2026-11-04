# SM-1 — SMAA with depth edge detection, `video.anti_alias` = 3 (AMD-060)

Worktree `wt-smaa`, branch `smaa-depth`, from `reworkoptions` at `1a1086c` (the close of RU-7).

Build directory: `rust/target-sm`. `CARGO_TARGET_DIR=<worktree>/rust/target-sm`, and the release
executable is built with `--features zax-app/bundled,zax-app/wgpu`.

## Checklist

| # | Step | Status |
|---|---|---|
| 1 | The reference and the specs read; the design (pass order, colour space, targets, tables, bind layouts); the test plan | done — `4465826` |
| 2 | The setting's domain 0..3, the options window, the Lua mapping, the persistence — no rendering yet | done — `607b1d1b` |
| 3 | OpenGL: the two tables and the depth edge detection | done — `beb3ac0d` |
| 4 | OpenGL: the blending weights, the neighbourhood blending, the lifecycle, the fallback | done — `0badc10e` |
| 5 | `wgpu`: the three passes, the lifecycle, the fallback | done — this commit |
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

#### 2. The depth threshold: the reference's constant is unusable here, and the second difference is the answer

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
  property of the perspective projection, not an approximation — it is why a depth buffer may be
  interpolated linearly at all. So the **second** difference of the window depth is *rigorously
  zero* over a plane, at any angle, any resolution and any distance, which makes AMD-060's own
  acceptance criterion ("les pixels intérieurs à la face sont identiques à l'octet") an identity
  rather than a tolerance. The third sample it needs is the pixel two away — `offset[2]` of
  `SMAAEdgeDetectionVS`, which the reference already computes and which only its luma detector
  uses.
- **Both differences have to clear the threshold** (`min` of the two), and each rejects what the
  other admits: the second difference alone marks the pixel *behind* the contour as well, and the
  first alone fires on a slope. Step 3 measured both failure modes.
- **The absolute floor is expressed as a relative distance jump**, so it is scale- and
  distance-free. From `w ≈ 1 − 0.15/d` comes `0.15/d ≈ 1 − w`, hence a jump of `ε · d` in distance
  is a jump of `ε · (1 − w)` in window depth. The floor is therefore
  `ZAX_SMAA_DEPTH_RELATIVE · (1 − w)`, with `1 − w` read straight off the sample.

So: `threshold = ZAX_SMAA_DEPTH_FLOOR + ZAX_SMAA_DEPTH_RELATIVE · (1 − w)`, and the quantity
compared against it is `min(first difference, second difference)` — the reference's
`step(threshold, ·)` and its `discard` unchanged. The constants are constants of the shader file,
as `ao_blur.frag`'s `ZAX_AO_BLUR_SHARPNESS` is, and for the reason that file gives: REQ-SCN-085
fixes the *detector* (the depth, never the luma or the colour) and leaves its sensitivity free.
**Q-109 §1** records the departure from the reference's constant and the reasoning.

The cost against the reference's own detector: two extra point samples of the depth per pixel, at
offsets the reference's vertex shader already provides.

**Step 3 corrected the shape of this term after measuring it** — the design as first written
compared the step to the local *slope* (`delta > 1.5 · slope`), and that version found **no
horizontal contour at all** on the close-up R. Its own section records the measurement and the fix;
the paragraph above is the design as built.

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

### Step 2 — the setting's domain, and nothing of the renderer

AMD-060 §2 clause by clause, with no shader and no target: the mode is selectable and persisted
before anything renders it, so that step 3 has a switch to render against.

#### What landed

| Piece | What landed |
|---|---|
| `zax-settings/src/table.rs` | the fourth rung, `opt(3, "SMAA_DEPTH")`, appended to the choice domain; the **libretro** list split off and kept at the three of AMD-032 §2 |
| `zax-render/src/geometry.rs` | `SMAA_DEPTH_LEVEL`, `smaa_depth(option)`, and `anti_alias_samples`'s doc saying why level 3 falls into the one-sample arm |
| `zax-app/src/persistence/app_settings.rs` | `set_anti_alias` clamps to `0..=SMAA_DEPTH_LEVEL` |
| `zax-app/src/ui/strings/{english,french}.rs` | `option.SMAA_DEPTH` — "SMAA (depth)" / "SMAA (profondeur)" — and the row's tooltip rewritten in both languages: what the fourth level does, that it costs about a third of MSAA 4×'s memory, and that it leaves the status bar's own outline alone |
| `zax-scripting/src/settings.rs`, `api/settings.rs`, `lib.rs` | `read_msaa`, the "en lecture, la valeur 3 se lit comme « aucun multi-échantillonnage »" half of AMD-060 §2's Lua clause, on the member's own read path |
| `zax-settings/tests/amd060_anti_alias_domain.rs` (new) | five tests: the fourth rung, the untouched default / field 32 / Lua path, a settings file carrying it, a value outside the domain, the libretro list |
| `zax-app/tests/anti_alias_labels_tests.rs` | RO-5's `LEVELS` grows a fourth row; the three above it unchanged is what the file now checks |
| `zax-app/tests/video_option_tests.rs` | the domain test takes 3 and −4; `req_scn_085_the_fourth_level_asks_for_no_multisampling` |
| `zax-profiles/tests/.../profile_round_trip_tests.rs` | field 32 round-trips 0, 1, 2 **and 3** |
| `zax-scripting/tests/.../two_zero_api_tests.rs` | two tests: `Setting.Msaa` never writes 3 to either setting and reads 3 as 0; `Setting.AntiAlias` carries the fourth level both ways |
| `.gitignore` | `rust/target-sm/`, beside RU-7's `rust/target-ro/` |

#### The libretro list is **not** extended, and that is a decision

AMD-060 §2 says the domain goes 0..2 → 0..3 and that "la fenêtre d'options propose la quatrième
valeur". It says nothing of libretro. A core option's value strings are what `RetroArch` persists
(`core_options::CoreOption` has one string per value, which is at once what it stores and what it
displays), AMD-032 §7 lists the libretro table among what does not change, and RO-5's own test
pins those three strings. So the domain has four rungs and the libretro enumeration has three: the
fourth level is reachable from the settings file, the options window, the console pages and Lua,
and a libretro front end keeps the enumeration it already stored.
`req_scn_086_the_libretro_enumeration_is_not_extended` asserts it, so it is a checked choice rather
than an omission — and a one-line change if R0 wants the other.

#### AMD-060 §2's Lua clause, read

"`Setting.Msaa` ne produit jamais la valeur 3 en écriture ; en lecture, la valeur 3 se lit comme
« aucun multi-échantillonnage »." The first half was **already** true and is now asserted: AMD-059
REQ-LUA-116's own mapping (`normalise_msaa`: `v ≤ 1 → 0`, `2 ≤ v ≤ 3 → 2`, `v ≥ 4 → 4`) cannot
produce 3, and RO-2 part 4's side effect maps those onto None or FFAA, so neither `video.msaa_2_0`
nor `video.anti_alias` can become 3 through the member. The second half had no existing reader, so
it is `read_msaa`, one comparison on the member's read path: 3 is the one value of that member's
domain `normalise_msaa` never produces *and* the one value of `video.anti_alias` that means no
multisampling, so a 3 reaching the member is the one number it could otherwise read as the opposite
of the truth. `Setting.AntiAlias` — the member that *is* the setting — carries 3 in both
directions, which is what "aucune fonction ajoutée ni retirée" leaves it doing.

The Lua API golden is untouched: no member was added or removed, and `goldens/lua-api.txt` is in
the workspace suite.

#### The "before" figures, measured here

Post-LTO assembly of the harness binary, **default** features, so the column is the OpenGL path —
the method RU-3a-fix through RU-7 used, with the script written fresh for this lot and kept out of
the tree with the scratch probes.

| Symbol | RU-7 recorded | SM-1 step 2 |
|---|---|---|
| `Ppu2C02::run_dots` | 2231 | **2231** |
| `DrawListBuilder::build` | 1095 | **1095** |
| `SceneRenderer::render_scene` | 3401 | **3401** |
| `Construction::segment` | 159 | **159** |

All four to the instruction, which is the check that this lot starts where RU-7 ended.

**One rule of the script is worth writing down, because getting it wrong looked exactly like a
regression.** A function's body has to stop at `.seh_endproc`. Without that cut
`DrawListBuilder::build` reads **1187** — a cold/unwind tail follows its epilogue and the naive
block scan swallows it — while the other three read their RU-7 values unchanged. Three symbols
agreeing and one off by 92 is a convincing-looking fault, and it is the script's.

#### Tests

`cargo fmt --all --check` and `cargo clippy --workspace --all-targets --features zax-app/wgpu --
-D warnings` clean. In release, one crate at a time:

| Crate | Result |
|---|---|
| `zax-settings` | 268 passed, 0 failed (5 of them this step's new file) |
| `zax-profiles` | 349 passed, 0 failed, 68 ignored (1 new) |
| `zax-scripting` | all suites passed (2 new) |
| `zax-app --features bundled` | all suites passed (1 new, 2 amended) |
| `zax-render` | all suites passed |

#### A build note for whoever resumes this

A fresh worktree has no `rust/crates/zax-app/demo/FlappyZaxNes.nes` or `flappyzaxnes.3dn`: both are
`.gitignore`d build products of `demo/source`, and `demo_game.rs` `include_bytes!`s them, so
`zax-app` does not compile without them. They were copied from `impl/`. They are ignored, so they
cannot be staged by accident. `zax-app` and its tests also need `--features bundled` here, or
`link.exe` cannot find `SDL2.lib`.

### Step 3 — the two tables, the depth edge detection, and the lifecycle around it

The filter's first pass, its data, and the whole of REQ-SCN-086's lifecycle — so that step 4 has a
module to put two more passes into and nothing of the plumbing left to argue about.

#### What landed

| Piece | What landed |
|---|---|
| `src/assets/smaa/area.bin`, `search.bin`, `README.md` (new) | the reference's two byte arrays, with their shapes, their SHA-256 and the extraction command |
| `src/shaders/smaa.glsl` (new) | what the three passes share: `uRtMetrics` with the **mirrored** `y`, `uDrawnScale`, and the two reads (`zaxSmaaFetch`, `zaxSmaaFetchOffset`), written as multiply-adds so the `y` sign carries |
| `src/shaders/smaa_edges.frag` (new) | `SMAADepthEdgeDetectionPS` with the threshold of Q-109 §1, the reference's `discard` and its `r` = west / `g` = north encoding |
| `src/gl/smaa.rs` (new) | the `Smaa` module: the vertex array, the edge program, the two tables uploaded `LINEAR`, the `RG8` target allocated at the attachments' capacity and cleared each frame, `rt_metrics`, `bytes`, and two unit tests |
| `src/gl/shaders.rs` | `SMAA_AREA_TABLE`, `SMAA_SEARCH_TABLE`, the `SMAA` chunk and `SMAA_EDGES` — which deliberately does **not** take `SCREEN_DEPTH`: this detector reconstructs no position, so the projection uniforms of AMD-055's passes would be four numbers it never reads |
| `src/renderer.rs` | `smaa`, `smaa_given_up`, `fail_smaa`; `select_smaa` / `prepare_smaa` / `build_smaa`; the depth-texture wish; `run_smaa` with the frame order written out; the two resource counters |
| `src/scene.rs` | `RenderSettings::anti_alias` and `set_anti_alias(level)`, which derives the sample count — see below |
| `src/statistics.rs` | `smaa_passes`, `smaa_bytes`, `smaa_programs` |
| `src/diagnostics.rs` | `smaa_unavailable`, the one line of REQ-SCN-086's fallback |
| `zax-harness/src/render/mod.rs` | the three keys in `render.json`, written **only** at level 3 |
| `zax-app` | `set_anti_alias(level)` at the three call sites that used to convert the level to a sample count themselves |
| `zax-render/tests/amd060_smaa_tests.rs` (new) | six tests: the tables' bytes, the sources, the metrics, the three counters at the four levels, the allocate/release round trip, the fallback |

#### The scene had no way to tell level 3 from level 0, and that is what `set_anti_alias` fixes

The first thing this step ran into. `RenderSettings` carried **only** `anti_alias_samples`, and the
app and the harness each converted the setting into a sample count on the way in
(`set_anti_alias_samples(geometry::anti_alias_samples(level))`). Levels 0 and 3 both mean one
sample, so the renderer could not tell them apart — and the renderer is where the filter lives.

So `RenderSettings` gains the level and one setter for both: `set_anti_alias(level)` stores it and
derives the sample count through the same `geometry::anti_alias_samples`, so the two can never
disagree. The three production call sites now pass the level and the conversion rule lives in one
place. `set_anti_alias_samples` stays for the tests that want a sample count and nothing else, with
its doc saying so.

#### The detector as designed found **no horizontal contour at all**, and the measurement is the step

The one real finding of this step, and it cost a throwaway probe to get.

The design section above first wrote the threshold as "is this step larger than the local slope?"
— `delta > ZAX_SMAA_DEPTH_SLOPE · slope`, with the slope read from the pixel two away. The argument
for it was sound (the window depth is linear in screen space over a plane, so `delta == slope`
there) and the result was wrong. Measured on the close-up R of AMD-055 §2.5 at 320 × 240, with the
edge pass redirected into the linear buffer so that the capture *is* the edge map:

| detector | west edges | north edges |
|---|---|---|
| `delta > 1.5 · slope` (as designed) | 2 845 (3.70 %) | **0 (0.00 %)** |
| `min(delta, second difference) > threshold` (as built) | 810 (1.06 %) | **1 094 (1.42 %)** |

Zero north edges, on a frame whose most obvious contour is the cube's horizontal top silhouette.
The cause, measured on the same frame with the raw differences written out as colour: the floor is
a plane seen at a shallow angle, so its own per-pixel step reaches **3.4 · 10⁻⁴** — while the
cube's horizontal silhouette jumps **3.2 · 10⁻⁴**. The same size. No threshold on the first
difference separates a slope that steep from a step that small.

The **second** difference does, exactly: it is rigorously zero over a plane (the window depth being
linear in screen space is why), and at a silhouette it is the jump. But it is not enough on its own
either — along a row `… bg bg | obj obj …` it is the same size at the first object pixel (right) and
at the second (wrong), so the contour would come out two pixels wide where the second pass's weights
are built for one. The first difference is zero at that second pixel. So **both** have to clear the
threshold, which is one `min` and keeps the shape of the reference's own
`edges = step(threshold, ·)`.

Three things worth drawing from it, and they are why this is written out at length:

1. **The first version would have passed every static test in the file.** It had the right
   constants, the right offsets, the right `discard` and the mirrored `y`; what it did not have was
   a frame. This is RU-7's own finding in a new place — "a device finds what reading cannot" — and
   here the device was not even needed, only a *picture*.
2. **The fix is strictly better than the design, not a retreat from it.** The `min` also finds a
   detail one pixel wide (the second difference is twice the jump there), which the slope comparison
   missed by construction. Q-109 §1 records that as a gain.
3. **Half the probe's time went on the probe.** The redirection `detect_edges` was given drew into
   framebuffer **0** rather than into the frame, because `screen_effects::allocate` binds its new
   framebuffer and then unbinds to `None` — so on the first frame the redirection was undone between
   the bind and the draw, and the capture was the ordinary scene. The tell was that a shader writing
   solid green changed nothing: those 2 845 "west" pixels are the red cube's, read through an
   `r > 127` test. Rendering twice fixed it. Written down because the same shape of mistake will
   meet whoever next redirects a screen pass for a look.

#### What a frame at any other level pays for this step

`render_scene` went from **3401 instructions to 3400** — one fewer, which is 0.03 % and the wrong
sign to worry about. The whole of what it carries: `select_smaa` is one integer comparison and one
assignment of `None` to an `Option` that is already `None`; `begin_pass`'s depth-texture argument
gains an `|| self.smaa.is_some()`; the frame gains one `is_some` that decides not to call
`run_smaa` (which is `#[inline(never)]` for exactly that reason); and `smaa_bytes` /
`smaa_programs` are assigned 0 once a frame. No program is built, no table is uploaded, no target
is allocated and no pass is submitted.

| Symbol | RU-7 | step 2 | step 3 |
|---|---|---|---|
| `Ppu2C02::run_dots` | 2231 | 2231 | **2231** |
| `DrawListBuilder::build` | 1095 | 1095 | **1095** |
| `SceneRenderer::render_scene` | 3401 | 3401 | **3400** |
| `Construction::segment` | 159 | 159 | **159** |

#### Tests

| Criterion | Test |
|---|---|
| AMD-060 §4, the tables to the byte | `amd060_smaa_tests.rs` `req_scn_085_the_two_tables_are_the_reference_s_own_bytes` (length + SHA-256) |
| their shape against the reference's own constants | 1 unit test in `gl/smaa.rs` |
| the detector reads the **depth**, carries Q-109 §1's threshold and both differences, and reads neither the colour nor the luma | `req_scn_085_the_edge_detection_reads_the_depth_and_mirrors_y` |
| `SMAA_RT_METRICS.y` is negative | 1 unit test in `gl/smaa.rs`, plus the pass running over a target of the frame's size |
| REQ-SCN-086: nothing at levels 0, 1 and 2; everything on the first frame at 3; one sample at 3 | `req_scn_086_nothing_of_the_filter_exists_at_the_other_levels` |
| REQ-SCN-086: allocated and released without a restart, twice over | `req_scn_086_the_targets_are_allocated_and_released_without_a_restart` |
| REQ-SCN-086's fallback: refused, no pass, no target, no MSAA, said once, and the picture of level 0 | `req_scn_086_a_refused_resource_draws_the_frame_without_anti_aliasing` |

`cargo fmt --all --check` and `cargo clippy --workspace --all-targets --features zax-app/wgpu --
-D warnings` clean. In release, one crate at a time: `zax-render` all suites passed (6 new
integration tests, 2 new unit tests), `zax-harness` all suites passed, and `zax-settings`,
`zax-profiles`, `zax-scripting` and `zax-app --features bundled` are unchanged from step 2.

#### Two more environment notes for whoever resumes

- **`rust/test-roms/` is `.gitignore`d too**, and `zax-harness`'s `bench_core_tests` needs
  `nestest.nes` from it. Copied from `impl/`, like the demo artefacts of step 2.
- **The G: drive filled up.** `target-sm` accumulated **1.14 GB of linker PDBs** (187 of them) on
  top of a 36 MB `--emit=asm` dump, and the harness build then failed with
  `rustc-LLVM ERROR: IO failure on output stream: no space on device`. `release/deps/*.pdb` and the
  `.s` are safe to delete between runs — rustc regenerates them — and `impl/`'s own 30 GB of build
  directories are not this lot's to touch.

### Step 4 — the blending weights, the neighbourhood blending, and the frame carrying on

The other two passes of REQ-SCN-085, and with them the mode's first picture. Both acceptance
criteria of AMD-060 are read off a capture in this step.

#### What landed

| Piece | What landed |
|---|---|
| `src/shaders/smaa_weights.frag` (new, 300 lines) | the whole of `SMAABlendingWeightCalculationPS` at the reference's **High** preset: the diagonal search and its bilinear decode, the four horizontal and vertical searches through the search table, `SMAAArea` and `SMAAAreaDiag` through the area table, and the two corner patterns |
| `src/shaders/smaa_blend.frag` (new) | `SMAANeighborhoodBlendingPS`, with its early-out — which is where the second acceptance criterion lives — and the two colour taps written out as `mix`es |
| `src/gl/smaa.rs` | `weights` and `blend` programs; the `RGBA8` weight target and the `RGBA16F` blend target with the frame's **own depth** attached; `filter` (the three passes in one submission) replacing `detect_edges`; `allocate` out of line; `blended_texture`; `PROGRAMS` 1 → 3 |
| `src/gl/linear_buffer.rs` | `redirect_color`, the one thing the rest of the renderer has to be told |
| `src/renderer.rs` | `run_smaa` calls `filter` and sets the redirection; the framebuffer is **not** rebound afterwards, on purpose |
| `src/gl/shaders.rs`, `src/lib.rs` | `SMAA_WEIGHTS` and `SMAA_BLEND` |
| `zax-render/tests/amd060_smaa_tests.rs` | two tests: AMD-060's two acceptance criteria, with a `striped_face` fixture built for the second |

#### The third pass cannot work in place, and that decides the shape of the whole frame

REQ-SCN-083's composite is a **blend state** — `glBlendFunc(GL_ZERO, GL_SRC_COLOR)` multiplies the
linear buffer where it stands, no second target and no copy. The neighbourhood blending cannot be:
it writes a *mix of two texels of the texture it reads*, so source and destination have to be
different objects. That is the one unavoidable cost of this mode, and it is what the extra
`RGBA16F` target is.

What follows from it is the nicest part of the step: the filter **leaves its own target bound**,
with the frame's depth still attached. So the HUD of REQ-SCN-044 draws into it with **no change to
any draw path** — same program, same depth test, same blend — and the only thing the rest of the
renderer has to be told is that the sRGB encode must sample that texture instead of the frame's
own. One `Option<Texture>` on `LinearColorBuffer`, set by `run_smaa` and cleared by every
`begin_pass`.

#### Three places the port could not be the reference, and one it deliberately is not

1. **`SMAAMovc` is `mix(a, b, bvec)`.** The reference's conditional move is two `if`s over a
   `float2`; GLSL's `mix` with a boolean selector is the same thing in one instruction, and it is
   used in both arms of the diagonal weights and for the crossing-edge removal.
2. **The neighbourhood blending does its own bilinear.** The reference "exploits bilinear filtering
   to mix current pixel with the chosen neighbor" — it samples at `texcoord + fraction · metrics`
   and lets the sampler return `(1 − f)·here + f·there`. The frame's colour texture is **`NEAREST`**
   here and has been since AMD-021 R2-009; the resolve pass reads it at exact texel centres, where
   the two filters agree. Changing a sampler state the whole renderer shares, to save a `mix`, was
   the wrong trade — so the two taps are written out. Same two texels, same weight.
3. **The two tables are read `LINEAR`, the edge and weight targets too.** Nothing in either shader
   says which filter it wants; the reference says it through its `LinearSampler` / `PointSampler`
   split, and getting it wrong is silent. The horizontal and vertical searches read **four edges at
   a time** through the filter (`@PSEUDO_GATHER4`) and `SMAAArea` interpolates between two
   distances, so `NEAREST` anywhere but the depth would have given a plausible, wrong picture.
4. **High, not Ultra.** `SMAA_PRESET_HIGH` — 16 steps, 8 diagonal, corners at 25 %. The preset is a
   cost, the mode's whole argument (AMD-060 §1) is that it costs one sample where MSAA 4× costs
   four, and 16 steps handle a line 64 pixels long perfectly. Ultra's 32 is for a resolution far
   above what a 1080p frame of 256 × 240 voxels shows.

#### The two acceptance criteria, measured

`close_up_r` of AMD-055 §2.5 at 320 × 240, release, this machine. Level 1 is in the table because a
figure means more with something to read it against.

| | distinct colours at 0 | at 1 (MSAA 4×) | **at 3** | level 3 moves | level 1 moves |
|---|---|---|---|---|---|
| a cube on a floor | 3 | 12 | **68** | 611 px (0.80 %) | 409 px (0.53 %) |
| a face head-on | 3 | 13 | **81** | 568 px (0.74 %) | 449 px (0.59 %) |

**The first criterion** — "la capture à 3 a strictement plus de couleurs distinctes qu'à 0" — holds
by a wide margin, and the mode puts *more* gradations on a silhouette than MSAA 4× does (68 against
12), which is what a coverage-area filter should do against four fixed samples. Every colour of the
unfiltered capture survives: the filter adds gradations between the frame's own colours, it does not
replace them. All three are assertions.

**The second criterion** — "sur une scène dont une seule face plane porte une texture contrastée,
les pixels intérieurs à la face sont identiques à l'octet à ceux de la capture à 0" — holds to the
bit, over **10 000-odd interior pixels** of a 10 × 10-tile surface textured with alternating columns
one texel wide, which is the worst case any filter can be given and exactly the pixel art AMD-032 §2
refused a post-processing filter over. The pixels compared are those at least three columns and
three rows inside the face's own silhouette: three because the first pass reaches two texels
(`offset[2]`) and the third mixes over one more.

It holds for a reason rather than by tuning, and the reason is worth repeating: the window depth is
linear in screen space across a plane, the second difference is therefore zero there, and the
`RGBA16F` source and target plus the reference's own early-out mean an unmarked pixel is **copied**,
not recomputed.

#### What a frame at any other level pays for this step

`render_scene` goes from **3401 instructions at `1a1086c` to 3409** — plus 8, **0.24 %**, against
the lot's 1 % bar. The cause is identified and it is `LinearColorBuffer`: `begin_pass` clears the
redirection (one store) and `resolve_to` tests it (`self.redirect.unwrap_or(attachments.color)`),
and both of those are inlined into `render_scene`.

The reset was tried in `resolve_to` and in `select_smaa` instead — where it would land in `render`,
which R0 does not count — and that bought **one** instruction (3408). It was put back in
`begin_pass`, because one store is not worth three places to clear a field from and the invariant
"a frame starts with no redirection" is worth being able to read off one line.

| Symbol | `1a1086c` | step 3 | step 4 |
|---|---|---|---|
| `Ppu2C02::run_dots` | 2231 | 2231 | **2231** |
| `DrawListBuilder::build` | 1095 | 1095 | **1095** |
| `SceneRenderer::render_scene` | 3401 | 3400 | **3409** (+0.24 %) |
| `Construction::segment` | 159 | 159 | **159** |

Everything else is as step 3 left it: no program, no table, no target and no pass at levels 0, 1
and 2, and `run_smaa` is `#[inline(never)]` so none of its body is in the counted function.

#### Tests

| Criterion | Test |
|---|---|
| REQ-SCN-085's first acceptance criterion, with MSAA 4× beside it and every unfiltered colour surviving | `amd060_smaa_tests.rs` `req_scn_085_the_filter_puts_more_colours_on_a_geometric_edge` |
| REQ-SCN-085's second acceptance criterion, on a face textured one texel at a time | `req_scn_085_the_interior_of_a_textured_face_is_untouched` |
| the three passes, the three programs and the 14 bytes a pixel | the lifecycle tests of step 3, with their counts raised from 1 and 2 |

`cargo fmt --all --check` and `cargo clippy --workspace --all-targets --features zax-app/wgpu --
-D warnings` clean. `zax-render` and `zax-harness` all suites passed in release, one at a time.

#### What this step did **not** do, and where it goes

- **The HUD-untouched check needs a HUD**, and `zax-render`'s fixture builder has no interface
  container — the synthetic set of the harness has a `hud` scene, so that assertion goes in step 6
  with the rest of the harness-wide measurements. The *order* that makes it true is built and
  documented (`run_smaa`'s four numbered reasons, Q-109 §2); what is missing is the picture that
  proves it.
- **The byte-identity of levels 0, 1 and 2 against `1a1086c`** is step 6's worktree comparison, as
  RU-7's own was. The instruction counts and the three zeroed counters are what this step has.

### Step 5 — the `wgpu` path, and the bug it found in the OpenGL one

The three passes on the second backend of REQ-SCN-056, their lifecycle and their fallback. **And
the one finding of this lot so far: putting the same algorithm beside itself with the other `y`
sign showed that the OpenGL path's two vertical searches did nothing at all.** That is written out
first, because it is the reason this step matters more than a port usually does.

#### The two vertical searches of the second pass were dead on OpenGL

`zaxSmaaSearchYUp` and `zaxSmaaSearchYDown` walk `± (0, 2) · uRtMetrics.xy` and stop at a bound
(`offset2.zw`) built the same way, so **both the step and the bound flip with the sign of
`uRtMetrics.y`** — and the operator between them does not. With the reference's own
`texcoord.y > end` and `uRtMetrics.y = −1/h`, the bound sits *above* the start and the condition is
false on the first test: zero iterations, `e` left at its initial value, and the distance `d` the
minimum. A vertical contour was therefore blended over **one** pixel instead of over the length of
its run.

The design note of step 1 (point 1) is what missed it: "every place the reference crosses the
screen in `y` does it as `± offset · SMAA_RT_METRICS.y`, so the whole of its arithmetic runs
unchanged". True of the arithmetic, false of the **comparisons** — an inequality is not covariant
under a sign change of its two sides. The fix multiplies the comparison through by the metric:

| | |
|---|---|
| reference (`y` downward) | `texcoord.y > end` |
| both backends now | `(texcoord.y − end) · uRtMetrics.y > 0` |

which is the reference's own inequality where `y` grows downward and the mirrored one where it
grows up, written identically in the GLSL and in the WGSL so that neither can drift back.
`zaxSmaaSearchXLeft` / `XRight` need nothing of the kind: `uRtMetrics.x` is positive on both.

**Why nothing caught it before, and what catches it now.** The filter still anti-aliased, still
passed both acceptance criteria of AMD-060 (more colours at 3 than at 0; the interior of a textured
face byte-identical), still allocated and released correctly, and still *looked* right — the
horizontal searches and the whole diagonal half were working. What exposed it was the one
measurement a second backend makes possible: the two pictures of one scene.

| close-up R, 320 × 240 | gl colours at 3 | wgpu colours at 3 | px gl moves | px wgpu moves | differ | within 2 |
|---|---|---|---|---|---|---|
| a cube on a floor, **before** | 68 | 98 | 611 | 682 | 0.3737 % | 99.6458 % |
| a cube on a floor, **after** | **104** | 98 | **680** | 682 | **0.1003 %** | **99.8997 %** |
| a face head-on, **before** | 81 | 118 | 568 | 783 | 0.4271 % | 99.6497 % |
| a face head-on, **after** | **119** | 118 | **783** | 783 | **0.0143 %** | **99.9883 %** |

On the head-on face — whose level-0 captures are **byte-identical** between the backends, so there
is no rasterisation fringe to hide behind — the two backends now blend **782 of the same 783
pixels** and the whole frame differs on 11 pixels of 76 800, worst 33 levels. Before the fix they
differed on 328 pixels by up to 86 levels. The two ports agree because they are now the same
algorithm.

**The bar this suggests for step 6**, and the one that would have caught this: the level-3
disagreement has to be of the same *order* as the level-0 control's, not an order worse. "At most
three times the share the unfiltered pictures differ on, and at most 0.03 % where they are
identical" passes after the fix (0.1003 % against 3 × 0.0482 %; 0.0143 % against 0.03 %) and fails
before it on both scenes. The usual share-within-2-levels bar does **not** catch it: 99.6458 % is
above RU-7's own 99.5 %.

**Three things to draw from it**, since this is the second lot in a row where the finding is a
coordinate convention:

1. a mirrored axis is safe through multiply-adds and **unsafe through comparisons**. The lot's
   design said so of texel offsets (`textureLodOffset`) and not of loop bounds;
2. the GL figures of step 4's table (68 and 81 colours, 611 and 568 pixels moved) were measured
   with the dead searches and are **superseded** by the row above. Nothing else in step 4 changes:
   the byte-identity of the levels 0, 1 and 2, the instruction counts, the two acceptance criteria
   and the fallback all held before the fix and hold after it;
3. a second implementation is a test the first cannot write. RU-7 said "a device finds what reading
   cannot"; this is its sequel — *another backend finds what one device cannot*.

#### What landed

| Piece | What landed |
|---|---|
| `wgpu_backend/shaders/smaa.wgsl` (new) | what the three passes share: the block (two `vec4`), the six bindings, the full-screen triangle and the reads — the depth and the frame colour by `textureLoad`, the edges, the weights and the two tables by `textureSampleLevel` through one filtering sampler |
| `wgpu_backend/shaders/smaa_edges.wgsl` (new) | `SMAADepthEdgeDetectionPS` with Q-109 §1's threshold, both differences and the `discard` |
| `wgpu_backend/shaders/smaa_weights.wgsl` (new, 330 lines) | the whole of `SMAABlendingWeightCalculationPS` at the High preset |
| `wgpu_backend/shaders/smaa_blend.wgsl` (new) | `SMAANeighborhoodBlendingPS` and its early-out |
| `wgpu_backend/smaa.rs` (new) | the `Smaa` module: one layout, one sampler, **one** block, the two tables, the three pipelines, the three targets, the three bind groups, the resolve group of the filtered colour, `bytes`, `blended`, `filter`, `allocate` and three unit tests |
| `wgpu_backend/renderer.rs` | `smaa`, `smaa_targets`, `smaa_given_up`, `fail_smaa`; `select_smaa` / `prepare_smaa`; the readable-depth wish; `Buckets` grown by three variants and read through three predicates; `run_smaa` (the three passes **and** the HUD pass that follows them); the resolve group redirection; the two counters |
| `wgpu_backend/mod.rs` | `smaa_sources()`, for the test that reads what the passes compute |
| `shaders/smaa_weights.frag` | the fix above, with the measurement in its comment |
| `tests/amd060_smaa_tests.rs` | five tests: the WGSL carries the reference's arithmetic, the sign-covariant comparison on **both** sources, the `wgpu` lifecycle on each API, both acceptance criteria on each API, the fallback on each API |

#### Three decisions this backend forced, and what each cost

1. **The HUD needs a pass of its own here.** On OpenGL the filter leaves its framebuffer bound with
   the frame's depth attached and `draw_hud` needs no change at all. `wgpu` has no bound
   framebuffer, so `run_smaa` opens a pass over the same two attachments (the filtered colour, the
   frame's own depth, both `Load`) and draws the HUD bucket into it. One render pass more at
   level 3, not one draw more — and the frame order of Q-109 §2 is the same order on both.
2. **`Buckets` had to split once more.** It was `All` / `Opaque` / `Blended`, where `Blended` is the
   transparent objects **and** the HUD. REQ-SCN-085 puts the filter between those two, so the enum
   grows `Scene` (opaque + transparent, a filtered frame with no screen pass), `Transparent` (the
   same frame with one) and `Hud`, and `draw_buckets` reads three `const fn` predicates instead of
   two `!=`. The three old variants select exactly the three old sets, so levels 0, 1 and 2 draw
   what they drew — which is what the harness goldens and `ru_7_the_classic_frame_agrees_between_
   the_backends` assert.
3. **One uniform block, not one per pass.** `screen_effects` needs a slot per pass group because
   `wgpu` applies every `write_buffer` of a submission before the command buffers run. The three
   passes of this filter read the *same* two `vec4`, so one block written once a frame is all they
   need — and the reason to write it down is that the opposite mistake is invisible.

#### What the port could not be the WGSL of the GLSL, line for line

All three are this language's and none changes the arithmetic: no `out` parameters (the two
diagonal searches return a struct), no swizzle assignment (`c.yxwz = decode(c)` and `weights.rg`
become explicit components, which is why the two weight pairs are two `vec2` rather than one
`vec4`), and no `inout` (the corner patterns take the weights and return them multiplied). The one
thing checked rather than assumed: **a data texture's row 0 sits at `v = 0` under both APIs**, so
the two tables are read with the reference's own coordinates on both and only *framebuffers* mirror
between the backends.

#### The fallback on this backend is asked, not caught

`wgpu` reports a refused allocation through the device's error scope rather than by handing back an
error, so REQ-SCN-086's fallback is put to the adapter once in `new` — does it render into
`Rg8Unorm` and `Rgba8Unorm`? — exactly as RU-7 put the `R16Float` pyramid of the MSVO fallback
**(D)**. `fail_smaa` is the test seam that reaches it on a machine that refuses nothing, and the
diagnostic line is the OpenGL path's own wording, written once.

#### Tests

`cargo fmt --all --check` and `cargo clippy --workspace --all-targets --features zax-app/wgpu --
-D warnings` clean. In release, one crate at a time:

| Crate | Result |
|---|---|
| `zax-render --features wgpu` | all suites passed; `amd060_smaa_tests` is 13 tests (5 new) |
| `zax-harness --features wgpu` | all suites passed, the 26 of `parity_wgpu_tests` and the two golden suites among them |

No Rust outside `wgpu_backend/` changed this step, so `SceneRenderer::render_scene` is step 4's
3409 instructions unchanged; the only OpenGL change is two loop conditions in a fragment shader.

### What is left for R0

Nothing yet beyond the libretro decision above and the three questions of **Q-109** (the depth
threshold, the HUD clause of REQ-SCN-085, and the 2D mode). The closing section of AMD-060 is R0's;
what R0 needs will be gathered here in step 7.

One thing R0 should know now rather than in step 7: **the figures of step 4's own table are
superseded by step 5's** (the dead vertical searches), and the cost figures of step 6 are therefore
measured on the fixed filter. Step 4's instruction counts, byte counts and pass counts are
unaffected.
