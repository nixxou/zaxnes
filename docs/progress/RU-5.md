# RU-5 — the Enhanced visual style (AMD-055)

Branch `rustupdated`, on top of RU-4b (`9cb306c`). Amendment AMD-055 (REQ-SCN-078 to REQ-SCN-084:
style selection, parameters, base lighting and the order of effects, voxel grid and pixel
shadows, ambient occlusion, floor reflections) ported to the **Rust** application. The .NET lot of
AMD-055 on `dotnetupdated` is a reading reference only.

## Checklist

| # | Step | Status |
|---|---|---|
| 1 | Map: what exists, where each requirement goes, the "before" measurements | done — `a9216ce` |
| 2 | REQ-SCN-078: the style reaches the renderer, and the zero assertions | done — `98a08ce` |
| 3 | REQ-SCN-081: the voxel grid | done — `c5dfc8d` |
| 4 | REQ-SCN-082: the pixel shadows | done — `be7de5c` |
| 5 | REQ-SCN-080 and REQ-SCN-083: the screen-space infrastructure and the ambient occlusion | done — `1f3d8ec` |
| 6a | REQ-SCN-084: the floor definition, on the CPU | done — `276b98a` |
| 6b | REQ-SCN-084: the mask, the trace and the composite | done — `86a810f` |
| 7a | REQ-UI-061 and REQ-SCN-079: the desktop "Style" group, the labels, the handheld page | done — hash in the next commit |
| 7b | The harness options (`--style`, `--style-param`) | done — hash in the next commit |
| 7c | The GL capability finding, the graceful fallback, the `wgpu` line of Q-104 | todo |
| 8 | The per-effect measurements | todo |
| 9 | Closing: the SPEC chapter edits, `PERFORMANCE-HISTORY.md`, intended image changes, fmt, clippy, full tests, goldens | todo |

**The checklist went from 10 steps to 9 after step 2**, so the commit of step 3 says "3/9" while
steps 1 and 2 say "/10". The reason is written out under "The checklist was renumbered" below.

The hash of a step is written in the commit of the step that follows (no amend, protocol of this
lot).

## Notes

### State of the working copy at the start

`git status` showed one untracked folder, `tests/`: the **.NET** test tree's `bin/`, `obj/` and
`TestResults/` build output, left behind by an earlier checkout of `dotnetupdated` in this working
copy. Same finding as RU-4a and RU-4b; it is not this lot's work and is left untracked and
uncommitted. No other uncommitted change, so nothing had to be reviewed or discarded.

### What already exists on the branch, checked key by key

| Piece | State |
|---|---|
| `render.visual_style` | **exists** (RU-2): `catalogue.rs` line 102, `table.rs`, domain accepting 0 and 4 with 1 read back as 4; in `ui/options.rs` `EXCLUDED` |
| the twelve `render.style.*` keys | **exist** (RU-2): `catalogue.rs` lines 213-224, `table.rs` lines 133-157 with the defaults and domains of AMD-055 §4 (`ao` 0-2 d.0, `ao_strength`/`radius`/`thickness` 1-5 d.3, `ao_quality` 0-2 d.1, `pixel_shadow` 0-1 d.0, `voxel_grid` 0-3 d.0, `reflections` 0-1 d.0, `ssr_shine`/`clarity`/`length` 1-5 d.3, `ssr_quality` 0-3 d.2); all thirteen in `EXCLUDED` |
| the Lua surface | **exists** (RU-2): `zax-settings/src/styles.rs` maps the sixteen Enhanced parameter identifiers (four curvature + twelve of REQ-SCN-079) onto those settings; `canonical_style` already turns 1 into 4 |
| the style's **render effect** | **nothing**. `RenderSettings` (`zax-render/src/scene.rs`) carries no style field; no screen pass, no depth texture, no noise texture |
| the shader variants | `gl/shaders.rs` `scene(solid, srgb_palette, bend, centroid)` — four axes, sixteen names — plus `backdrop(bend)` and `shadow(bend)`; `Stage` holds six fixed slots and `assemble` skips an empty one **without even a blank line** |
| the program slots | `renderer.rs` `ScenePrograms`: `built: Vec<ShaderProgram>`, `slots: [[usize; 2]; 8]` keyed on `(bend, centroid, backdrop)`, `slot` refreshed by the setters, `current()` two indexings and no branch |
| the linear buffer | `gl/linear_buffer.rs`: single-sample `Attachments { framebuffer, color: Texture, depth: Renderbuffer }` and multisampled `SampleAttachments` of two renderbuffers. **The single-sample depth is a renderbuffer, so it cannot be sampled** — the one allocation the screen-space passes have to change |
| the frame order | `renderer.rs` `render_scene`: clear/skybox, opaque runs, transparent objects, HUD, then `linear.resolve_to(target)`. The two screen passes of REQ-SCN-080 go between the opaque runs and the transparent objects |
| the local frame of a vertex | `scene.vert` has `aPosition` and `aNormal`, the mesh's local frame of REQ-E3D-504 — exactly the frame REQ-SCN-081 and REQ-SCN-082 evaluate in, before the deformation, the object transform and the curvature. **No new attribute is needed**, only varyings inside the new variants |
| `scene_export` | `zax-render/src/scene_export.rs`, the CPU twin of the placement, for the criteria written against exported vertices |
| the capability probe | nothing of the kind; `LinearColorBuffer::new` reads `GL_MAX_SAMPLES` and `allocate_samples` lowers the count until the framebuffer is complete — the shape a float-target probe should follow |
| the `wgpu` backend | behind a non-default feature; `Q-104` already records that the curvature is OpenGL-only there |

### Requirement map

| Requirement | Rust crate / file |
|---|---|
| REQ-SCN-078 style selection | `zax-render/src/scene.rs` (`RenderSettings::visual_style` and the style parameters), `zax-app/src/scene_script_bridge.rs` (read **on change**), `renderer.rs` (`select_style_resources`) |
| REQ-SCN-079 parameters | already persisted (RU-2); `zax-render` receives them as one `StyleSettings` value, `ui/options.rs` shows them, `ui/handheld/defaults.rs` gives them their handheld defaults |
| REQ-SCN-080 base lighting, order | `renderer.rs` `render_scene` (the two passes between the opaque runs and the transparent ones), `gl/linear_buffer.rs` (the resolved colour and depth) |
| REQ-SCN-081 voxel grid | `zax-render/src/shaders/voxel_grid.glsl` (new), `scene.vert` / `scene.frag` / `backdrop.frag` under a new guard, `gl/shaders.rs` (a fifth axis) |
| REQ-SCN-082 pixel shadows | `scene.vert` / `scene.frag` under a guard of their own |
| REQ-SCN-083 ambient occlusion | `zax-render/src/gl/screen_effects.rs` (new), `shaders/ao.frag`, `shaders/ao_blur.frag` |
| REQ-SCN-084 floor reflections | the same module, `shaders/ssr.frag`, `shaders/ssr_blur.frag`; the floor test on the instance's pre-curvature normal and `xz` box, in `frame_plan.rs` |
| the GLES probe | `gl/api.rs` / `gl/screen_effects.rs`, reported through `statistics.rs` |
| the harness options | `zax-harness/src/render/mod.rs` |

### The design this lot rests on, and why

Five decisions, each forced by point 1 or point 2 of the brief (Classic costs exactly what it
costs today; each effect pays only when on).

1. **The style is not a uniform anywhere.** Enhanced with every effect off must be Classic, so
   there is no `uStyle` to test: what the effects need is compiled into **shader variants**, as
   AMD-043's curvature and AMD-058's centroid already are, and the passes simply do not run. The
   slot table of `ScenePrograms` is the place the new axes go, and its key widens.
2. **The two mesh-space effects are two independent variant axes**, `ZAX_VOXEL_GRID` and
   `ZAX_PIXEL_SHADOW`, not one "Enhanced" axis: REQ-SCN-079 defaults both to 0 and they are
   switched separately, so a player who turns the grid on must not pay for the shadow snapping.
3. **The two screen-space effects are passes, not variants**, and they own every resource they
   need (`ScreenEffects`): allocated the first time a frame asks for one, released when no frame
   wants either. At Classic, and at Enhanced with `spl.ao` = 0 and `spl.reflections` = 0, the
   module is `None` and the only cost is the `Option` test that decides not to run it.
4. **The depth the passes read is a texture the frame rendered into.** The single-sample
   attachments of `LinearColorBuffer` keep their depth **renderbuffer** unless a screen pass wants
   depth, which is the one allocation difference between Classic and Enhanced; `begin_pass` takes
   the wish as an argument, so Classic allocates exactly what it allocates today.
5. **The floor test of REQ-SCN-084 is per instance, not per fragment**, and it is evaluated on the
   normal and the `xz` box **before curvature**, which is what the requirement asks. It therefore
   belongs with the draw list, where the placement already is, and reaches the pass as a mask.

### The "before" measurements (step 1)

Taken on this machine, in release, nothing else running. **No commercial ROM is available on this
side** (`ZAXNES_ROMS` is unset and the original program's folders are out of bounds), so the ROM
criteria and the review criteria of AMD-055 are R0's.

**Post-LTO instruction counts** (`cargo rustc -p zax-harness --release --bin zax-harness --
--emit=asm`, the method of RU-3a-fix, RU-3b, RU-4a and RU-4b). All four are **identical to
RU-4b's closing figures**, which is the check that this lot starts where that one ended:

| Symbol | instructions |
|---|---|
| `Ppu2C02::run_dots` (the core control: nothing of this lot touches it) | **2231** |
| `DrawListBuilder::build` | **1095** |
| `SceneRenderer::render_scene` (`FramePlan::collect` and `collect_tile` inlined) | **3510** |
| `Construction::segment` (the mesh control) | **159** |

**The GPU at default settings** (Classic), `zax-harness render --anti-alias 0 --bench 60`,
640 × 480, median frame time:

| Scene | draw calls | triangles | resident meshes | colours | ms |
|---|---|---|---|---|---|
| framing | 1798 | 21 576 | 2 | 3 | 0.42 |
| sky | 0 | 0 | 2 | 1 | 0.08 |
| shadow | 164 | 1968 | 4 | 6 | 0.17 |
| no-shadow | 160 | 1920 | 6 | 4 | 0.12 |
| layer-shadow-off | 160 | 1920 | 8 | 4 | 0.12 |
| nudge | 144 | 1728 | 10 | 4 | 0.20 |
| alpha | 184 | 2208 | 12 | 8 | 0.12 |
| hud | 16 | 192 | 13 | 4 | 0.10 |
| clip | 168 | 2016 | 14 | 5 | 0.19 |
| backdrop | 165 | 1980 | 15 | 10 | 0.18 |
| heavy | 3852 | 46 224 | 79 | 16 | 1.44 |

The same at the two resolutions the brief's point 4 names, for the per-effect figures of step 9 to
be read against:

| Scene | 1280 × 720 | 1920 × 1080 |
|---|---|---|
| framing | 0.46 | 0.45 ms |
| shadow | 0.20 | 0.21 ms |
| no-shadow | 0.16 | 0.17 ms |
| backdrop | 0.20 | 0.25 ms |
| heavy | 1.08 | 1.14 ms |

(The scenes are small enough that these are dominated by the draw submission, not by the fill;
that is why a screen-space pass, whose cost **is** fill, is measured separately in step 9.)

**The 80 assembled shader sources at default** (16 scene variants + 2 shadow + 2 backdrop, × 2
stages × 2 contexts) are dumped **outside the repository** for the length of this lot, so that
every later step can prove byte identity of the ones it must not touch — the method RU-4b used for
its 40. How to reproduce the dump, since the helper is deliberately not committed: a test file in
`crates/zax-render/tests/` that walks `shaders::scene(solid, srgb, bend, centroid)`,
`shaders::shadow(bend)` and `shaders::backdrop(bend)`, calls `shaders::assemble(stage, fragment,
embedded)` for both stages and both contexts, and writes each result to
`$ZAX_DUMP_DIR/<name>.<desktop|gles>.<vert|frag>`. The permanent, committed form of the same check
is the preprocessing test of step 2, which asserts that **none** of the identifiers the Enhanced
effects bring in reaches the compiler in any Classic variant — the shape
`req_scn_067_the_flat_variant_carries_nothing_of_the_curvature` established.

### Step 2 — the style reaches the renderer, and the three zeros

**The resource lifecycle moved to step 3 and the step was renamed.** Step 2 as the brief sketched
it bundled "the lazy resource lifecycle" with the style selection, but the only resources the
style needs are the screen passes' targets, and *where* they are allocated is the order clause of
REQ-SCN-080 — step 3's subject. Splitting them keeps both steps small, which is what this lot's
protocol is for; nothing is dropped, and step 3 carries the lifecycle with the order it belongs
to.

#### `zax-render/src/style.rs` (new), the normative quantities in one place

Platform-free, like `color::backdrop_linear` of AMD-049: every number AMD-055 derives from a
parameter is written once, unit-tested without a GL context, and read where it is needed.

| Item | Requirement |
|---|---|
| `StyleSettings`, twelve fields + `enhanced` | REQ-SCN-079's table, with its defaults |
| `StyleSettings::style_of`, `identifier` | REQ-SCN-078: 4 and 1 in, 4 out, everything else Classic |
| `voxel_grid_active`, `pixel_shadow_active`, `ao_active`, `reflections_active` | the four predicates the renderer tests once a frame |
| `screen_effects_active` | the predicate the brief's point 2 rests on |
| `voxel_cell` | `N = 2^spl.voxel_grid` (REQ-SCN-081) |
| `ao_radius_units`, `ao_ceiling` | 8/14/20/26/32 units; `min(0,95 ; 0,3 × strength)` (REQ-SCN-083 v1.4) |
| `ao_samples`, `ao_half_resolution` | 6/10/16; half except High |
| `ssr_intensity`, `ssr_max_length`, `ssr_blur_fraction` | `0,045 × n`; `40 × n`; `0,003 × (10 − n)` (REQ-SCN-084 v1.5) |
| `ssr_steps`, `ssr_refinements`, `ssr_half_resolution` | 12/24/48/48; 2/3/5/5; half except Ultra |

**There is no style uniform, and that is the design.** Classic and Enhanced-with-everything-off
have to be the same picture *and the same cost*, so the switch cannot be a uniform a fragment
tests: the effects are shader variants (steps 4 and 5) and passes that do not run (steps 6 and
7), and the predicates above are what decides, once a frame.

Every published figure of the amendment reproduces, first run — the five radii, the five ceilings
including the two top rungs that share 0.95, the three `ssr.*` tables and the "16 pixels at
1080p" the amendment prints beside the default blur (0.015 × 1080 = 16.2).

#### The wiring, and where the thirteen lookups are paid

`RenderSettings::style` carries the value; `scene_script_bridge::style_of` builds it and
`is_style_setting` routes a change to it. **Thirteen model lookups is more than any frame path
should do**, so they are paid exactly where AMD-043's three sines and cosines are paid: in
`apply_setting`, once per change, and in `push_settings` at a profile load. The renderer then
reads the ready-made value and tests four booleans.

The curvature is deliberately **not** read there: REQ-SCN-078 keeps it outside the style (reading
A, §3 point 4), so `bend_of` stays its only home and a change of style cannot lose a curvature
setting — which a test asserts over the styles 4, 0, 1 and 7.

#### The three zeros, observable from outside

`RenderStatistics` gained `style_programs`, `screen_effect_bytes` and `screen_effect_passes`.
They are **0 in Classic and 0 in Enhanced until an effect is turned on**, which is the brief's
points 1 and 2 stated as counters rather than as a frame time. The first two are resident counts,
so `begin_frame` leaves them alone; the pass count is per frame.

#### Tests

| Criterion | Test |
|---|---|
| REQ-SCN-078 on the **setting**: 4 → 4, 1 → 4, 0/2/7/−1/5 → 0 | `zax-app/tests/amd055_style_setting_tests.rs` `req_scn_078_the_style_identifier_round_trips` |
| the style and the twelve reach the scene on a change and on the wholesale push; Classic keeps every parameter | `req_scn_078_the_style_and_its_parameters_reach_the_scene_when_they_change` |
| §3 point 4: a change of style keeps the curvature | `req_scn_078_a_change_of_style_keeps_the_curvature` |
| REQ-SCN-079 first criterion: `ao.strength` 9 → 5, 1,4 → 1, 0 → 1; `spl.voxel_grid` 4 → 3; `spl.ao` 1,4 → 1; the thirteen across a restart | `req_scn_079_the_parameters_are_bounded_snapped_and_persisted` |
| a settings **file** of 1 reads back 4, of 7 reads back 0 | `req_scn_078_a_settings_file_of_one_reads_back_four` |
| REQ-SCN-080: Enhanced with every effect off is the Classic picture, **at tolerance 0** | `zax-render/tests/amd055_style_tests.rs` `req_scn_080_enhanced_with_every_effect_off_is_the_classic_picture` |
| the three zeros, and the same draws, triangles, meshes and casters | `req_scn_080_enhanced_with_every_effect_off_allocates_and_submits_nothing` |
| Classic with every parameter turned up is the Classic picture, and still zero | `req_scn_078_classic_with_every_parameter_turned_up_is_the_classic_picture` |
| §3 point 4 on the GPU: a bent scene draws the same in both styles, and 60° does change it | `req_scn_078_the_curvature_acts_in_both_styles` |
| the twelve defaults, the four predicates, and every derived quantity | 9 unit tests in `style.rs` |

The picture test asserts **tolerance 0**, not the ±2 sRGB levels REQ-SCN-080 allows: the same
program draws the frame with the same uniforms, so anything but an exact match would mean
something new is running.

`cargo test --release -p zax-render`: **220 passed, 0 failed**. `-p zax-app --test
amd055_style_setting_tests`: 5 passed. fmt and workspace clippy clean.

**The 80 assembled shader sources are byte identical to step 1's**, all 80, which is the belt on
"no shader changed in this step".

*(The dump helper of step 1 is recreated in `crates/zax-render/tests/dump_sources_tmp.rs` while
the lot runs and is **not committed**; it returns immediately without `ZAX_DUMP_DIR`. Whoever
resumes will see it as an untracked file beside the untracked `tests/` folder of the .NET tree.)*

### The checklist was renumbered after step 2 — 10 steps became 9

The brief's step 3 was "base lighting and order of passes (REQ-SCN-080)". Read against the code,
REQ-SCN-080 has almost no code of its own: its first sentence ("en Enhanced, l'éclairage des
tiles, du backdrop et du HUD est celui de Classic") is already true and step 2 measured it, and
the rest of the requirement **constrains** the four effects rather than adding a fifth thing — the
grid and the pixel shadows before the curvature, the two screen passes after it, in that order.
Its only standalone deliverable is the infrastructure the two screen passes share (the resolved
colour and depth, the lazy lifecycle of their targets), and *that* cannot be built and tested
without its first consumer, which is the occlusion.

So a step 3 of its own would have been a stub, and this lot's rule is that every step builds,
passes and is committed. The order clause instead travels with each effect, which is also where
it can be measured:

| # | Step | Where REQ-SCN-080 is honoured |
|---|---|---|
| 3 | the voxel grid | before the curvature, in the mesh's local frame; the HUD, the sky and the 2D rectangle excluded |
| 4 | the pixel shadows | the same frame, the same exclusions |
| 5 | the screen-space infrastructure **and** the occlusion | the resolved colour and depth, after the opaque run and after the curvature, before the transparent objects |
| 6 | the reflections | before the occlusion, both after the opaque run |

The new checklist is therefore **9 steps**, and the table at the top of this file is the one to
follow. Nothing of the brief is dropped; what moved is where it is implemented and measured.

### Step 3 — REQ-SCN-081, the voxel grid

#### The shape: two independent variant axes

`ZAX_VOXEL_GRID` and `ZAX_PIXEL_SHADOW` are **two** axes, not one "Enhanced" axis, because
REQ-SCN-079 switches `spl.voxel_grid` and `spl.pixel_shadow` separately and both default to 0: a
player who turns the grid on must not pay for the voxel snapping of step 4.

| Piece | What landed |
|---|---|
| `shaders/voxel_grid.glsl` (new) | `zaxVoxelGrid(p, n, cell)`, the formula of REQ-SCN-081, compiled only into the `ZAX_VOXEL_GRID` variants |
| `style.rs` | `voxel_grid_factor`, the platform-free CPU twin, plus `GROOVE_HALF_WIDTH` and `BRIGHT_EDGE_END` |
| `gl/shaders.rs` | `SceneVariant`, a **named-field** key replacing six positional bools; `scene(variant)` with its 64 names; `backdrop(bend, voxel_grid)`; `SLOTS` 6 → 10 |
| `renderer.rs` | `ProgramKey` (5 axes, 32 slots); `select_style_variants`; `draw_hud` extracted; `ScenePrograms::style_programs` |
| `frame_plan.rs` | `SceneConstants::voxel_cell` |
| `scene.vert`, `scene.frag`, `backdrop.frag` | `vLocal` / `vLocalNormal` and the modulation, each under its guard |

**`SceneVariant` instead of six bools.** `scene(solid, srgb, bend, centroid, grid, shadow)` is
six positional booleans at every call site, which is a bug waiting to happen; the struct names
them and `SceneVariant::FLAT` names the pair the renderer is created with. The three existing
test files that build variants gained a four-argument `variant(…)` helper, so what they assert is
unchanged and the two new axes are visibly off in them.

**The names of the sixteen old variants are unchanged**, because a name goes into the key of the
on-disk program cache: a player who uses neither effect keeps every binary that cache holds, the
promise AMD-058 made for a player who uses no anti-aliasing. A test rebuilds all 64 names from
their suffixes with `format!` and compares, so the hand-written table cannot drift.

#### Where the grid is evaluated, and the three frames it is not

REQ-SCN-081 fixes the frame, and the Rust already had everything: `scene.vert` holds `aPosition`
and `aNormal`, which **are** the mesh's local frame of REQ-E3D-504, before the deformation,
before the object transform and before the curvature. **No vertex attribute was added** — only
two varyings, inside the variants.

| Surface | What it reads |
|---|---|
| a tile (opaque or transparent) | `vLocal`, `vLocalNormal`, cell `N` |
| the backdrop, lighting on | `vScene`, `vLocalNormal`, cell `N × scene scale` — `backdrop.frag`, on the albedo `c` **before** the model, so a groove does not darken the specular term (AMD-055 §3 point 5) |
| the backdrop, lighting off | the same, through `scene.frag` at colour mode 2 |
| the HUD, the skybox, the 2D rectangle | **nothing**: see below |

The scene normal of a backdrop face **is** its mesh normal: the backdrop's placement is a scale
and a translation with no rotation, and a non-uniform scale changes an axis-aligned normal's
length but not its direction. So one varying serves both.

The three exclusions cost nothing:

- the **skybox** has its own program, so it is excluded by construction;
- the **HUD** draws in its own pass, which already switched to the flat variant for REQ-SCN-068;
  the same switch now drops the grid and the pixel shadows, so it is three setters instead of one;
- the **2D-mode rectangle** is the only object of a 2D-mode frame (the finding RU-4a step 5
  recorded), so `select_style_variants` simply does not select the variant in 2D mode — the whole
  frame then draws plain, and the exclusion is free.

The **tunnel and the TV models** of REQ-SCN-053 and REQ-SCN-054, which REQ-SCN-081 also excludes,
**do not exist in Zaxnes**: AMD-031 retired the TV box, and `scene.tv_box` is a profile field that
applies nothing (`zax-profiles`, `amd_031_the_tv_box_field_survives_a_round_trip_and_applies_nothing`).
That exclusion is therefore moot here, and the same will hold for REQ-SCN-083 and REQ-SCN-084.

#### The one per-fragment uniform test, and why it is allowed

`scene.frag`, inside the grid variant only, reads
`uColorMode == 2 ? vScene : vLocal`. That is a uniform that is **constant over a whole draw**, the
same kind as the per-axis `uBendOn` RU-4a kept inside the curvature variant; the forbidden one is
a uniform that switches between having an effect and not having it, and that one is the `#define`.
It buys the backdrop being correct in both positions of `render.backdrop_lighting` rather than
only in one.

#### What the implementation reproduces, and the two measurements that were wrong

**Every ratio of the acceptance table came out right**, on the CPU twin and on the GPU:

| Point (local, `N` = 4) | Published | CPU twin | GPU (min/max over the face) |
|---|---|---|---|
| (0 ; 2), centre of a groove | 0,62 ± 0,03 | 0.620 | present |
| (2 ; 2), centre of a cell | 1,00 ± 0,01 | 1.000 | over a quarter of the face exactly 1 |
| (0,4 ; 2), bright edge | 1,048 ± 0,02 | 1.048 | present; the brightest pixel 1.12 to 1.17 |
| (0 ; 0), crossing | 0,384 ± 0,03 | 0.384 | **0.378** at `N` = 2, 4 and 8 alike |

The sRGB columns (133 / 166 / 169 / 107) follow from the ratios at the albedo of the close-up G,
and recovering the **166** of the "grid off" column is what pins the fixture: it is
`0,503 × (0,1812 + 0,577)`, the ambient of REQ-SCN-028 plus `N·L = 1/√3` on a face of normal
(0, 0, −1) lit from (1, −1, 1). The close-up G itself is **not** buildable on this side — it wants
a custom texture in the direct RGB format (188, 188, 188), which the synthetic fixture cannot make
— so the sRGB columns on the GPU are R0's.

**Two of my own measurements were wrong and the code was right**, which is worth recording
because both are easy to repeat:

1. **A rectangle of the capture is not a surface.** The first GPU test measured ratios over a
   window and read a minimum of 0.08, far under the 0.384 floor an axis-aligned face allows. The
   window took in a second, dimmer surface at the face's edge, which is already 0.154 of the face
   with the grid **off**. The test now masks to the pixels that were the front face in the
   grid-off capture, so every ratio is one surface against itself — and the minimum is 0.378.
2. **Halving the cell does not increase the groove area.** The second test counted groove
   *pixels* and expected them to double from `N` = 8 to `N` = 4 to `N` = 2; they did not move
   (8213, 8823, 8727). That is the formula: the groove is a band of 3.5 % of a **cell**, so
   halving the cell halves every line's width as it doubles their number. The criterion is about
   **spacing**, so the test counts the groove *lines* that cross one row of the face — and picks
   the row whose mean ratio is highest, because a row that happens to sit on a horizontal groove
   is dark end to end and crosses no vertical line at all.

A third finding, in an existing test rather than mine: `ZAX_PI`, a constant of the backdrop model,
is a **substring** of the new `ZAX_PIXEL_SHADOW`, so `req_scn_075_no_tile_fragment_carries_the_model`
reported a leak that was not one. `common::mentions` now compares whole GLSL tokens, and says in
place why.

#### The brief's point 1, measured

- **All 80 Classic sources are identical to step 1's as the compiler sees them**, checked by
  running the `ZAX_VOXEL_GRID`/`ZAX_PIXEL_SHADOW`-undefined branch of the preprocessor over both
  dumps and diffing. The source *text* differs, and cannot not: a `#ifdef` line is text. That is
  the same situation AMD-058 recorded for `ZAX_CENTROID`, and the committed form of the check is
  `req_scn_081_no_classic_variant_carries_anything_of_the_grid`, which preprocesses the assembled
  source the way the compiler would and asserts that none of `zaxVoxelGrid`, `uVoxelCell`,
  `vLocal` and `vLocalNormal` reaches it — over all 32 Classic scene variants × 2 contexts and the
  two backdrop programs without the grid axis, and in reverse for the grid variant.
- **The eleven synthetic captures at default settings are identical to step 1's at tolerance 0**,
  all eleven, and the draw-call, triangle and resident-mesh counts are unchanged.
- `style_programs` is **0** in Classic and 0 in Enhanced at `spl.voxel_grid` = 0, and 2 or more
  once the grid is on.
- `SLOTS` went 6 → 10, which changes no assembled source: `assemble` skips an empty slot without
  even a blank line.

#### Tests (12 in `zax-render/tests/amd055_voxel_grid_tests.rs`)

| Criterion | Test |
|---|---|
| REQ-SCN-081's four-point table, ratios and sRGB, on the twin | `req_scn_081_the_four_points_of_the_acceptance_table` |
| the factor is exactly 1 past 16 % of a cell, at the three cell sizes | `req_scn_081_the_factor_is_exactly_one_away_from_an_edge` |
| an axis perpendicular to the face cuts no groove | `req_scn_081_an_axis_perpendicular_to_the_face_cuts_no_groove` |
| `k` and `e` interpolated between `m` = 0,8 and 0,9 | `req_scn_081_the_alignment_interpolates_k_and_e` |
| the edges are the multiples of `N`, one through the tile centre for all three | `req_scn_081_the_edges_are_the_multiples_of_the_cell` |
| **the published ratios are what the shader draws**, at `N` = 2, 4 and 8 | `req_scn_081_the_published_ratios_are_what_the_shader_draws` |
| third criterion: the spacing doubles as the cell halves | `req_scn_081_the_three_settings_space_the_grooves_two_four_and_eight_units_apart` |
| fourth criterion: `scale = (2, 1, 1)` and `rot = (0, 0, 30)` move the grooves with the object | `req_scn_081_the_grid_follows_the_object` |
| the scope: 0 is Classic to the pixel, nothing compiled, and the sky and the HUD are untouched | `req_scn_081_the_grid_is_off_at_zero_and_never_reaches_the_hud_or_the_sky` |
| AMD-055 §3 point 5: the backdrop takes the grid, in both positions of `render.backdrop_lighting` | `req_scn_075_the_backdrop_takes_the_grid_on_its_albedo` |
| REQ-SCN-080's order clause: the grid is painted on the geometry, and survives a 90° bend | `req_scn_080_the_grid_is_painted_on_the_geometry_not_on_the_screen` |
| the brief's point 1: no Classic variant carries anything of the grid | `req_scn_081_no_classic_variant_carries_anything_of_the_grid` |

`cargo test --release --workspace --features zax-app/bundled`: **2329 passed, 0 failed**, 83
ignored (2297 at the close of RU-4b). fmt and workspace clippy clean. The render goldens and the
PPU goldens hold.

Four pre-existing clippy ceilings were hit by the additions and are recorded rather than worked
around: `SceneVariant` and `ProgramKey` pass the four-bool limit (the same `allow` `Passes` and
`ScenePrograms` already carry, each bool being an independent variant axis); `scene()` passes the
100-line ceiling, which is a table of 64 names a `const fn` cannot build and has no logic to
split out; and `render_scene` passed it again, so the HUD pass moved out into `draw_hud` — the
same remedy RU-4b applied when `bind_frame_uniforms` moved out.

#### What is left for R0 on this requirement

1. The **close-up G** itself, on the GPU: 1920 × 1080, the direct texture (188, 188, 188), the
   camera at (128 ; 120 ; −34), and the four sRGB values 133 / 166 / 169 / 107 ± 3. The ratios are
   confirmed here; what is left is the absolute column, which needs a texture the synthetic
   fixture cannot make.
2. The **wide framing B** of REQ-SCN-080: the backdrop at 90° of left-right curvature, the groove
   lines' deviation from their chord at `2,661 × |y − 120|` pixels. The order clause is confirmed
   here (the grid survives the bend and is painted on the geometry); the numbers want the harness
   at 1920 × 1080, which step 7 gives it the options for.
3. The **review criterion**: Super Mario Bros. 1-1 at `spl.voxel_grid` = 2 in a close view — the
   toy-brick look, the sky and the score unchanged. No commercial ROM is available on this side.

### Step 4 — REQ-SCN-082, the pixel shadows

The shadow of an opaque tile is read at the **centre of the fragment's voxel** instead of at the
fragment, so it changes by squares of one voxel. The variant axis `ZAX_PIXEL_SHADOW` already
existed (step 3 declared it, keyed it, selected it and dropped it for the HUD, and emitted `vLocal`
and `vLocalNormal` under the shared guard); what landed here is the body, and the one piece of
plumbing it needed.

| Piece | What landed |
|---|---|
| `zax-engine3d` `tile_mesh.rs` | `TileMesh::build_depth()`, the depth `D` of REQ-E3D-504, set by the two meshers (`builder.build(field.depth())`), 0 for a mesh that is not a tile's |
| `zax-render` `style.rs` | `voxel_center`, the CPU twin of the snapping; `voxel_z_offset`; `VOXEL_CENTRE_REACH` |
| `shaders/pixel_shadow.glsl` (new) | `zaxVoxelCenter` and `zaxVoxelShadowPoint`, compiled only into the `ZAX_PIXEL_SHADOW` variants |
| `scene.vert`, `scene.frag` | `flat` `vModelLinear` and `vDeformTile`, and the override of the shadow point — **every added line inside a `#ifdef ZAX_PIXEL_SHADOW` block**, no existing line touched |
| `gl/shaders.rs` | `SLOTS` 10 → 12; `Stage::of` took a const generic so a stage may list fewer chunks than slots; the fragment stage gained `DEFORMATION`, `BEND` (when the frame bends) and `PIXEL_SHADOW`, each under the pixel-shadow guard |
| `gl/mesh_cache.rs` | `GpuMesh::build_depth`, beside `axis_aligned_normals` — the shape AMD-057 laid for a per-run property of the mesh |
| `renderer.rs` `draw_run` | `uVoxelZOffset`, uploaded only while the variant is in use |

#### Why the fragment stage gets a placement, and why it is three varyings and not twenty

The snapping has to happen per fragment: `⌊p⌋ + 0,5` varies across one face. So the fragment stage
has to transform a point **like a vertex** (REQ-SCN-082 point 3), and a fragment can read neither
`gl_InstanceID` nor the instance block it indexes.

The placement is **affine**, which is the whole trick: only the *difference* between the snapped
point and the fragment's own has to be carried through it, so the translation of the model matrix
cancels and `mat3(model)` is enough. `zaxVoxelShadowPoint` therefore takes the fragment's own
`vScene` and adds `mat3(model) × (delta + deform(snapped) − deform(p))` — nine components plus the
three of the deformation's tile position, instead of the sixteen of the matrix plus four. The
budget is comfortable either way (37 components against the 60 GLES 3.0 guarantees), but the
cheaper form is also the clearer one: it says the snapping is a *displacement*.

#### The one piece of plumbing: the construction depth `D`

On `z` the voxel edges of the local frame are offset by `D/2` (REQ-E3D-504: the corner of voxel
`(i, j, k)` is at `(i − 4, j − 4, k − D/2)`), and `D` is a property of the **build**, not of the
mesh's extent — a `sizeZ` = 1 plate does not span the box of a deeper build, and a sparse tile does
not reach its own edges, so `bounds_max.z` is no substitute. It is computed where the mesh is
built, carried on `TileMesh`, copied onto `GpuMesh` and read per **run** in `draw_run`, beside
`wall_clip_bias(mesh)` which does exactly that for AMD-057.

Only `D/2 mod 1` changes the result — write `D/2 = m + f` and the two whole `m` cancel — so one
number out of two reaches the shader: **0 for an even depth, 0.5 for an odd one**. For a cylinder
it stays the **initial** depth and not the effective `D'` of REQ-E3D-452, which is what that
requirement says ("le champ de voxels et le centrage local des maillages restent ceux de la
profondeur initiale").

#### Two uniform tests inside the variant, both constant over a draw

`uColorMode != 2` excludes the **backdrop**, which with `render.backdrop_lighting` off draws
through `scene.frag` and *does* receive shadows; without the test it would snap in the unit box's
own local frame, where a whole face reads one single voxel centre. `uDeform != 0` skips the
deformation, exactly as the vertex stage does. Both are uniforms that are constant over a whole
draw — the kind RU-4a's notes allow; the forbidden one is a uniform that switches between having an
effect and not having it, and that one is the `#define`.

The other exclusions of REQ-SCN-082 cost nothing: the **HUD** and the **transparent objects** carry
`uReceiveShadow` = 0 (REQ-SCN-040), the **2D-mode rectangle** is the only object of a 2D-mode frame
so `select_style_variants` does not select the variant at all, and the tunnel and TV models do not
exist in Zaxnes (the step-3 finding).

#### The brief's point 1, measured

- **Every added line of `scene.vert` and `scene.frag` is inside a `#ifdef ZAX_PIXEL_SHADOW`
  block**, and no existing line moved — which is the structural proof that the Classic sources are
  unchanged as the compiler sees them. The committed form of the check is
  `req_scn_082_no_classic_variant_carries_anything_of_the_pixel_shadow`, which preprocesses the
  assembled source the way the compiler would, over the 64 Classic variants × 2 contexts and the
  four backdrop programs, and asserts that none of `zaxVoxelShadowPoint`, `zaxVoxelCenter`,
  `uVoxelZOffset`, `vModelLinear` and `vDeformTile` reaches it.
- **The eleven synthetic captures at default settings are byte identical to step 3's**, all eleven,
  and the counts of `render.json` (draw calls, triangles, resident meshes, colours) with them. The
  baseline was retaken from `c5dfc8d` in a throwaway `git worktree` — the step-1 dump lives outside
  the repository and was gone with that session — and compared by SHA-256.
- `SLOTS` 10 → 12 changes no assembled source: `assemble` skips an empty slot without even a blank
  line, and `Stage::of` now pads.

#### The instruction counts, and the 3510 that was not reproducible

| Symbol | step 3 (`c5dfc8d`), re-measured here | step 4 | recorded in step 1 |
|---|---|---|---|
| `Ppu2C02::run_dots` | 2231 | **2231** | 2231 |
| `DrawListBuilder::build` | 1095 | **1095** | 1095 |
| `SceneRenderer::render_scene` | 3548 | **3559** (+11, **+0.31 %**) | 3510 |
| `Construction::segment` | 159 | **159** | 159 |

The step-1 figure for `render_scene` — 3510 — **does not reproduce on this machine today**: the
step-3 commit itself measures 3548 with the same method and the same script, so 38 of the 49
instructions a naive reading would have blamed on this step are a toolchain or build difference
between the two sessions. That is why the baseline column is a fresh measurement of `c5dfc8d` and
not the recorded number: **a figure from another session is not a baseline**. Against the real
baseline the step costs **+11 instructions, 0.31 %**, which is the one new boolean test per draw
run (`draw_run` is inlined into `render_scene` at three sites: the opaque loop and the two passes
of REQ-SCN-040). The other three symbols are untouched, as they must be.

Two detours worth recording, because both looked like the explanation and neither was: taking the
uniform upload out of line (`#[inline(never)]`) changed nothing, and narrowing `GpuMesh`'s new
field from an `i32` to a `bool` — which keeps the handle at twelve bytes rather than sixteen —
changed nothing either. Both were measured and both reverted to the simpler code.

#### Tests (14 in `zax-render/tests/amd055_pixel_shadow_tests.rs`, 4 in `zax-engine3d`)

| Criterion | Test |
|---|---|
| REQ-SCN-082 point 2: a GREEDY face snaps the two axes of its plane, in all six orientations | `req_scn_082_a_greedy_face_snaps_the_two_axes_of_its_plane` |
| the tie rule — smallest index — at 45°, on a degenerate normal and on an oblique MARCHING one | `req_scn_082_the_tie_is_broken_by_the_smaller_index` |
| point 2 on `z`: even depth to the half coordinates, odd to the whole ones; a `sizeZ` = 1 plate is one voxel | `req_scn_082_the_z_axis_follows_the_construction_depth` |
| Notes: the point moves 0,7 unit at most, and never leaves the face | `req_scn_082_the_point_moves_at_most_seven_tenths_of_a_unit` |
| Notes: the cells are those of REQ-SCN-081 with `N` = 1 | `req_scn_082_the_cells_are_those_of_the_voxel_grid_at_one` |
| **the acceptance criterion, both bullets**: no cell holds both lit and shadowed pixels with the effect on, several cells of each edge do with it off | `req_scn_082_the_shadow_changes_by_whole_voxel_cells` |
| the criterion's own geometry: floor at `rot = (90, 0, 0)`, light (1, −3, 1), plate **29 units** above, camera at 45° of elevation | `req_scn_082_the_rotated_floor_of_the_criterion_quantises_too` |
| the scope: 0 is Classic to the pixel, Classic ignores the parameter, nothing compiled, `style_programs` 0 then ≥ 2 | `req_scn_082_the_snapping_is_off_at_zero_and_in_classic` |
| "sans objet sur le backdrop": a frame whose only receiver is the backdrop is the same picture both ways, to the pixel | `req_scn_082_the_backdrop_keeps_its_own_shadows` |
| the HUD, a transparent object and the 2D-mode rectangle: the same picture both ways | `req_scn_082_the_hud_the_transparent_objects_and_the_two_dimensional_mode_are_left_out` |
| REQ-SCN-080's order clause: the snapping is before the curvature, so a bent frame quantises on the mesh's own cells | `req_scn_080_the_snapping_happens_before_the_curvature` |
| the brief's point 1: no Classic variant carries anything of the snapping | `req_scn_082_no_classic_variant_carries_anything_of_the_pixel_shadow` |
| the backdrop program has no such axis | `req_scn_082_the_backdrop_program_has_no_pixel_shadow_axis` |
| the deformation and the curvature reach the fragment stage only for the snapping, and the curvature only when the frame bends | `req_scn_082_the_fragment_stage_takes_the_placement_only_for_the_snapping` |
| REQ-E3D-504: each mesher carries the depth of its field; the bounds run `−D/2` … `+D/2`; a hand-built mesh and an empty tile report 0 | 4 tests in `zax-engine3d/tests/engine3d/amd055_build_depth_tests.rs` |
| the derived quantities | 1 unit test in `style.rs` |

#### How the criterion is measured, and the three readings that were wrong

The criterion's discriminating bullet is the **second** one: the shadows of REQ-SCN-030 are already
hard, so the sharpness of an edge tells the two styles apart not at all. What tells them apart is
that with the effect on *no voxel cell of the floor holds both lit and shadowed pixels*. So the
measurement is a **census of cells**: each 1-voxel cell of each floor tile is sampled at nine
points of its local square, each point is projected through the frame's own camera (and curvature)
exactly as the vertex stage projects it, and the cell is counted as lit, shadowed, or **mixed**.

Three readings were wrong before the test said anything true, and all three are easy to repeat:

1. **A plate has two faces, and the wrong one shifts the whole grid.** A floor tile of `sizeZ` = 1
   is a closed box: its two faces are one unit apart, and a perspective camera at 30 units
   magnifies that unit into several pixels at the edge of the view. Sampling the far face reported
   mixed cells all over a picture that had none. The test now picks the face from the geometry —
   the one whose outward normal points at the eye — and not from the capture.
2. **A cell's outer ring is not the cell.** The criterion says "pour chaque cellule de 1 voxel du
   sol **entièrement visible**", and the outer ring of each tile box is not: a box is closed on
   itself (REQ-E3D-501), so at the joint with its neighbour the lateral wall of one stands one unit
   from the face of the other, and a wall is a face of its own — with its own two snapped axes. The
   census skips that ring, and the last stubborn mixed cell went with it. Sampling the middle
   *third* of a cell rather than the middle *half* is the same lesson one scale down: widening the
   samples to 1/6 … 5/6 brought twelve false mixed cells back.
3. **A light of simple ratios hides the effect.** With a light of (1, −1, 1) and a whole number of
   units between caster and floor, the shadow's edges fall exactly **on** cell edges, so there is
   nothing for the snapping to move and the two styles draw the same picture. That is why the
   criterion puts its plate at **29** units under a light of (1, −3, 1): the shadow lands at
   29/3 = 9,667 units, with its edges in the middle of a voxel. The flat fixture here uses a light
   of (1, −0,7, 2,6) for the same reason, and the rotated one the criterion's own.

The curvature fixture needed one more turn of the same screw: at 30° the bend moves the shadow off
the part of the floor the camera sees, and at 15° it falls on cell edges again. **10°** is the
angle at which it both moves every pixel and leaves its edges inside voxels.

#### What is left for R0 on this requirement

1. The **absolute sRGB column** of the criterion (195 ± 3 lit, 118 ± 3 shadowed) at 1920 × 1080
   with the albedo and colours of the close-up G. Those want the direct texture (188, 188, 188)
   the synthetic fixture cannot make — the same limit step 3 recorded for the grid. What is
   confirmed here is the clause the numbers serve: every pixel of a cell carries the **same** value
   (compared exactly, which is stronger than the ±2 sRGB levels the criterion allows).
2. The **review criterion**: "Super Mario Bros." 1-1 at `spl.pixel_shadow` = 1 — block shadows cut
   into one-NES-pixel squares aligned on the game's pixels, the score and the sky unchanged. No
   commercial ROM is available on this side.

### Step 5 — REQ-SCN-080 and REQ-SCN-083: the screen-space infrastructure, and the occlusion

The sharpened design recorded under "Step 5, sharpened" below is what landed, all eight points of
it, and nothing of it had to be revised. What follows is what the implementation added to it.

| Piece | What landed |
|---|---|
| `gl/screen_effects.rs` (new) | `ScreenEffects`, the module the renderer holds as an `Option`: three programs, a pair of `R8` targets, the lifecycle |
| `shaders/screen_depth.glsl` (new) | `zaxViewDepth`, `zaxViewSample`, `zaxProjectView` — the depth and the view-space position both screen passes read |
| `shaders/ao.frag` (new) | the hemisphere kernel, the per-pixel rotation, the range check |
| `shaders/ao_blur.frag` (new) | the depth-weighted 4 × 4 box |
| `shaders/ao_composite.frag` (new) | `1 − O`, blended `GL_ZERO, GL_SRC_COLOR` |
| `gl/linear_buffer.rs` | `DepthAttachment`, `begin_pass(…, sample_depth)`, `resolve_samples`, `bind_for_draw`, `drawn`/`capacity`/`drawn_scale`/`depth_texture` |
| `gl/api.rs` | `blit_colour_and_depth`, the colour **and** depth resolve of REQ-SCN-080's last bullet |
| `gl/shaders.rs` | `SCREEN_VERTEX` (the resolve pass's own vertex stage) and the three `ProgramSource`s |
| `renderer.rs` | `select_screen_effects` (the lifecycle, once a frame), `run_screen_effects` (the pass block), `projection_terms` |

#### The frame order, and the consequence worth stating

The passes run in `render_scene` between the opaque runs and the transparent ones, which is
REQ-SCN-080's order exactly. The real consequence, now that it is implemented: with
anti-aliasing **and** a screen effect on, the transparent objects and the HUD are drawn into the
**resolved single-sample** target, so they are not multisampled. That follows from the
requirement's own order ("après résolution du MSAA éventuel") and is recorded in
`resolve_samples`'s own doc comment and in `PERFORMANCE-HISTORY.md` at the close of the lot.

#### The brief's point 1 and point 2, measured

- **The eleven synthetic captures at default settings are byte identical to step 4's**, all
  eleven, at `--anti-alias 0` *and* at the default anti-aliasing, and `render.json` with them
  (draw calls, triangles, resident meshes, colours). Compared by SHA-256 against a throwaway
  `git worktree` of `290c6fc`, the method step 4 established.
- `req_scn_083_no_geometry_program_carries_anything_of_the_occlusion` asserts the shader side
  structurally rather than by preprocessing: the occlusion is three **separate** programs, not a
  variant axis, so no scene, backdrop or shadow source can carry anything of it — over the 64
  Classic scene variants × 2 contexts, the four backdrop programs and the shadow programs.
- The three zeros of step 2 still hold, and now have a fourth reading: `style_programs`,
  `screen_effect_bytes` and `screen_effect_passes` are **0** in Classic, 0 in Enhanced at
  `spl.ao` = 0, and 3 / > 0 / 3 once the occlusion is on — then **0 again** when the style goes
  back to Classic, which is the release half of the lifecycle asserted rather than assumed.

#### The instruction counts, and the 973 that had to come back out of line

| Symbol | baseline `290c6fc` | step 5 |
|---|---|---|
| `Ppu2C02::run_dots` | 2231 | **2231** |
| `DrawListBuilder::build` | 1095 | **1095** |
| `Construction::segment` | 159 | **159** |
| `SceneRenderer::render_scene` | 3559 | **3382** (**−177, −5.0 %**) |

Three of the four are untouched, as they must be. `render_scene` took work, and the record is
worth keeping because the first measurement was alarming and the reason was not a real cost:

1. **Inlined, the pass block put 4532 − 3559 = +973 instructions (+27 %) into `render_scene`.**
   None of them *runs* in Classic — the branch is not taken — but a once-a-frame function R0
   counts should not carry the Enhanced path through its instruction cache to prove it.
   `#[inline(never)]` on `run_screen_effects` brought it to **3709**.
2. The remaining +150 were mostly **cold allocation code**: `begin_pass` gained the
   `sample_depth` argument, and `allocate` was inlined into it — and had been in RU-4b too.
   `#[inline(never)]` on `allocate`, which runs only when the size, the sample count or the depth
   wish changes, brought `render_scene` to **3382**, *below* the shipped baseline.
3. **At equal inlining** the step's own cost is **+57 instructions, +1.71 %**: the baseline
   patched with the same one line on `allocate` measures **3325** against this step's 3382. Those
   57 are the `is_some` branch, the depth wish threaded through `begin_pass`'s reallocation test,
   and the `resolved` test in `resolve_to`. Dynamically a Classic frame executes a handful of
   them, once per frame.

Both figures are reported because they answer different questions, and neither should be quoted
alone: against what RU-4b ships, this hot function is **5 % smaller**; against itself at equal
inlining it is **1.71 % larger** in *static size*. The brief's gate is callgrind's **executed**
count on a profile that uses nothing new, which is R0's to run — and the executed Classic path
grows by one not-taken branch and two comparisons per frame, not by 57.

#### Four decisions inside the pass, and the one measurement that drove each

1. **No noise texture.** REQ-SCN-083 leaves the kernel's per-pixel rotation free, so an
   interleaved-gradient hash of `gl_FragCoord` (Jimenez 2014) does it: one fewer resource to
   allocate, release, count and carry on a handheld, and the invariant it has to meet — no fixed
   pattern surviving the blur — is a property of the blur either way.
2. **No colour target for the composite.** `glBlendFunc(GL_ZERO, GL_SRC_COLOR)` makes the
   destination `dst × src`, so one full-screen triangle outputting `1 − O` multiplies the linear
   buffer in place. No second colour target, no copy, and the alpha is multiplied by 1.
3. **`R8`, not a float target.** The occlusion is one scalar in `[0, 1]`, and `GL_R8` is
   colour-renderable in OpenGL 3.3 core *and* required renderable in GLES 3.0 — so the only
   capability left for step 7 to probe is the multisampled **depth** blit, not a target format.
4. **A pixel with no plane is left alone, never darkened.** Where an axis has neither neighbour
   on a surface — the sharp corner of a silhouette — `ao.frag` answers "no occlusion" rather than
   invent a normal. Three pixels of the plane of the close-up A are in that case, and it is the
   first invariant ("1,00 ± 0,02 en tout pixel du sol") that found them.

#### Tests (12 in `zax-render/tests/amd055_occlusion_tests.rs`)

| Criterion | Test |
|---|---|
| §2.4 first invariant: a plane alone is 1,00 ± 0,02 everywhere — it comes out **exactly 1** over all 144 094 of its pixels | `req_scn_083_a_plane_alone_is_never_darkened` |
| second invariant: the foot of the cube is `< 1` and `≥ 0,10`, and the plane at `2r` = 40 units is 1,00 ± 0,02 | `req_scn_083_the_foot_of_the_cube_is_darkened_and_the_far_plane_is_not` |
| third invariant: `ao.strength` 1 ≥ 0,70 and 5 ≥ 0,05 (the ceiling read as a floor), non-increasing in the setting, strictly decreasing 1 → 4 | `req_scn_083_the_strength_is_the_darkening_ceiling` |
| fourth invariant under the fallback **(D)**: `spl.ao` 1 and 2 are the same capture, and `ao.thickness` changes no pixel | `req_scn_083_msvo_is_rendered_as_depth_and_the_thickness_does_nothing` |
| fifth invariant and the brief's point 2: `spl.ao` = 0 is Classic **to the pixel**, nothing compiled, allocated or submitted — and all of it released when the style goes back | `req_scn_083_the_occlusion_is_off_at_zero_and_in_classic` |
| the A-h variant: the Gradient skybox's and the HUD's pixels do not move, at tolerance 0 | `req_scn_083_the_skybox_and_the_hud_are_left_alone` |
| REQ-SCN-083's quality rungs: half resolution except High (the bytes), and the three agree to within a quarter at the contact | `req_scn_083_the_three_qualities_agree_and_only_high_is_full_resolution` |
| the 2D mode runs no pass, allocates no target, compiles no program, and draws the Classic picture | `req_scn_083_the_two_dimensional_mode_runs_no_pass` |
| REQ-SCN-080, 4th bullet: a transparent object does not occlude — and the same slab made opaque does | `req_scn_080_a_transparent_object_neither_occludes_nor_is_darkened` |
| REQ-SCN-080's order clause: the pass works on the **rendered** depth, so after the curvature | `req_scn_080_the_occlusion_works_on_the_bent_depth` |
| REQ-SCN-080's last bullet: with anti-aliasing on the pass works on the resolved frame, and the sample count is unchanged | `req_scn_080_the_occlusion_works_on_the_resolved_frame` |
| the brief's point 1 on the shaders: no geometry program carries anything of the occlusion | `req_scn_083_no_geometry_program_carries_anything_of_the_occlusion` |

#### Four fixture readings that were wrong, and the implementation that was right

This step was resumed from an uncommitted working tree: the implementation was complete and
sound, and **eight of its twelve tests failed on their own fixtures**. A scratch probe reproduced
every published invariant on the first run — a plane alone at exactly 1,0000, the five strengths
at 0,8152 / 0,6350 / 0,4527 / 0,4257 / 0,4257, MSVO identical to Depth, `spl.ao` = 0 identical to
Classic — which is what said the fault was in the measuring and not in the measured. All four
causes are easy to repeat:

1. **The background outnumbers the plane.** `plane_shade` took the commonest colour of the whole
   capture, but the plane covers 144 094 of the 307 200 pixels and the black background the other
   163 106 — so the "plane shade" was black, and every ratio was a division by zero. The helper
   now skips the background, which is what the probe did and the test did not.
2. **A depth test selects the plane too.** `is_cube` picked the instance to displace by its build
   depth, and the plane is built at `D` = 8 exactly like the cube; the other form read the
   transform, which is the thing the predicate exists to *set*. It now selects on the mesh's
   extent — 8 units against 96 — which neither the depth nor the transform can confuse.
3. **The HUD bucket is a layer, not a screen position.** `frame_plan` files a tile under
   `SceneContainer::Interface` when its pattern's layer is `ProfileLayer::Ui`; the fixture used
   `L1` and put the tile above the visible rectangle, which puts it on the sky and in the *scene*
   bucket. (The same misreading is recorded in the voxel-grid test of step 3, where the comment
   "this fixture's tile is in the scene bucket" states it correctly.)
4. **A background record at a cell of the plane replaces the plane's own tile there** — one screen
   cell carries one background tile — which left a **hole** whose rim is real geometry at a right
   angle. The occlusion darkened that rim correctly, in the transparent capture as in the opaque
   one, so the clause "a transparent object does not occlude" could not be read at all: the probe
   showed 144 opaque tiles with no slab and **143** with it. The slab is now a **sprite**, which
   coexists with the background tile it covers (REQ-CORE-043), and the measurement then separates
   cleanly: the plane's face is at ratio **1,0000 over all 141 527 pixels** with the transparent
   slab and **0,5578 over 12 411** with the same slab made opaque.

A fifth reading was wrong in the same family: asserting that the *whole capture* does not move
with a transparent object present. The plane is a closed slab of `D` = 8 (REQ-E3D-501), so its own
lateral walls meet its face at a right angle and the occlusion darkens that corner — correctly,
and in both captures. The clause is about the plane's **face**, and `plane_shade` is its mask.
The bent fixture needed the mirror of that lesson: under curvature the plane does not carry one
lighting term, so its "one shade" is a narrow band that need not touch the cube — that test
measures over "not the background" instead (`worst_geometry_ratio`).

`cargo test --release -p zax-render`: **261 passed, 0 failed**, 1 ignored (220 at the close of
step 2). fmt and workspace clippy clean.

#### What is left for R0 on this requirement

1. The **absolute column** of the close-up A at 1920 × 1080, which wants the direct texture
   (188, 188, 188) the synthetic fixture cannot make — the same limit steps 3 and 4 recorded. The
   ratios, which are what §2.4's invariants are written as, are all confirmed here.
2. The **review criterion**: a commercial ROM in a close view at `spl.ao` = 2. No commercial ROM
   is available on this side.
3. The **MSVO** mode is rendered as Depth, the amendment's declared fallback **(D)**, so
   `ao.thickness` has no observable effect; step 7 greys it, and `third_party/README.md` gains no
   entry in this lot.

### Step 6a — REQ-SCN-084, the floor definition on the CPU

The step the "Step 6, sharpened" notes below asked for: the half of the floor definition that
**cannot** be computed on the GPU, with no GL code and no change to any per-frame path.

| Piece | What landed |
|---|---|
| `style.rs` | `FLOOR_NORMAL_THRESHOLD`, `FLOOR_EXTENT_UNITS`, `floor_vertical`, `floor_plane_axes`, `is_floor_normal`, `is_floor_extent` |
| `frame_plan.rs` | `FramePlan::floor_surfaces` (per opaque item) and `floor_boxes` (the reused accumulator); `mark_floor_surfaces(local_camera, scene_scale)` |
| `scene_export.rs` | `ExportedObject::floor`, so §2.5's criteria are measurable without a GL context |

#### Why the definition has to be two tests, and why neither can move

The invariants decide it, and the test file says so at the top:

- **R-s** puts a block of 8 × 8 × 8 units beside the same block at `scale = (1 ; 1 ; 0,5)`. Both
  show the same pixels, at the same depth, with the same normal (0, 1, 0) — so **nothing
  reconstructed from the depth buffer can tell them apart**. The extent is a property of the
  placement and is computed where the placement is.
- **R-m** is the same 8 × 8-tile shape without rotation: its `xz` box measures 64 × 8, both over
  6, so it **passes** the extent test, and no pixel of its front face (0, 0, −1) may move. So the
  normal is tested **per fragment** — a per-instance mask alone would reflect the wall.

That is the whole shape of the requirement: a per-instance flag **and** a per-fragment normal
test, which is what makes the mask pass of step 6b necessary rather than a convenience.

#### Four readings settled here, each with its reason

1. **The box is the instance's, not the tile's.** A shape is drawn one item per tile record (the
   finding that cost the first run of these tests: an 8 × 8 block exports **64** objects, not
   one), so the boxes are accumulated per `DrawItem::instance` and then read back per item. The
   discriminating fixture is a floor of 8 × 1 tiles at `scale = (0,5 ; 1 ; 1)`: its tiles are 4
   units wide, under the threshold, and its instance is 32, over it. Only the accumulation
   answers right.
2. **The backdrop is an object of its own** — `Q-093 (a)`, **answered** by R1 on 2026-11-01, so no
   new ticket. It carries no instance, so `None` is its key in the accumulator. At its default
   size of 256 × 240 × 1 its `z` dimension is 1, under the threshold, so by default it is not a
   floor and the reading is not observable; a test pins both halves, default and `Size.z` = 6.
3. **Over the opaque bucket alone.** Only what is already drawn can be reflected: the transparent
   objects and the HUD come after the pass (REQ-SCN-080) and the shadow-only tiles of REQ-SCN-041
   are not drawn at all. A translucent floor and a floor on the interface layer are therefore
   never marked, which a test asserts on the same shape as the close-up R's floor.
4. **The local camera swaps the vertical, not the camera's angles** (v1.4, Q-094 then Q-096):
   `(0, 0, −1)` and the `xy` footprint, "quelle que soit l'inclinaison". The discriminating pair
   is **one** shape at **one** depth — 8 × 8 tiles of `D` = 3 is a floor laid flat for an ordinary
   camera (box 64 × 3 × 64) and not for a local one, and standing up (64 × 64 × 3) it is the other
   way round. Tilting the camera to the −60° of the Punch-Out!! measurement changes nothing.

#### The brief's point 1

Nothing of this step runs in a frame. `mark_floor_surfaces` has no caller in the renderer yet
(step 6b wires it, behind `reflections_active`), `FramePlan`'s two new fields allocate nothing
(`Vec::new`, `FastMap::default`) and no per-frame path reads them. Measured, post-LTO, against the
step-5 figures: `Ppu2C02::run_dots` **2231**, `DrawListBuilder::build` **1095**,
`SceneRenderer::render_scene` **3382**, `Construction::segment` **159** — all four **identical**.

#### Tests (9 in `zax-render/tests/amd055_reflection_floor_tests.rs`, 1 unit test in `style.rs`)

| Criterion | Test |
|---|---|
| the close-up R's floor: `rot = (90, 0, 0)`, `xz` box 64 × 64, a face of scene normal (0, 1, 0) | `req_scn_084_the_floor_of_the_close_up_is_a_floor_surface` |
| the box is the instance's: tiles 4 units wide, instance 32 | `req_scn_084_the_box_is_the_instances_and_not_the_tiles` |
| **R-s**, all three cases: 8 units reflects, `scale = (1 ; 1 ; 0,5)` does not, `D` = 3 does not | `req_scn_084_the_block_of_eight_units_reflects_and_a_shallower_one_does_not` |
| **R-m**: the wall passes the extent and is excluded by its face alone | `req_scn_084_the_wall_passes_the_extent_and_is_excluded_by_its_face` |
| the local camera, both directions on one shape, and the −60° tilt | `req_scn_084_the_local_camera_lays_the_image_plane_flat` |
| Q-093 (a): the backdrop at its default size and at `Size.z` = 6 | `req_scn_084_the_backdrop_is_an_object_of_its_own` |
| "dès que sa profondeur atteint 6 unités": 5 no, 6 yes, 8 yes | `req_scn_084_a_game_floor_crosses_the_threshold_at_a_depth_of_six` |
| the order clause: the box is taken before curvature, so 60° and 90° mark the same instances | `req_scn_080_the_floor_definition_is_measured_before_the_curvature` |
| the scope: a transparent object and the HUD are never floor surfaces | `req_scn_080_a_transparent_object_and_the_hud_are_never_floor_surfaces` |
| the two thresholds, the two verticals, the two axis pairs, the scene scale | `style.rs` `req_scn_084_the_two_halves_of_the_floor_definition` |

One assertion of mine was wrong and the code was right, worth recording because it is the same
trap as step 3's: **a tile is 8 units wide, so a full tile always passes the extent test on both
screen axes.** The first draft tried to contrast the ordinary and the local camera on a `D` = 8
shape, where both answer yes; the discriminating depth is **3**, which is under the threshold and
therefore moves with the pair of axes measured.

`cargo test --release -p zax-render`: **271 passed, 0 failed**, 1 ignored (261 at the close of
step 5). fmt and workspace clippy clean. One pre-existing clippy ceiling was hit and is recorded
rather than worked around: `ExportedObject` passes the four-bool limit, the same `allow`
`SceneVariant` and `ProgramKey` carry.

The AMD-055 question tickets this lot implements against — **Q-092 to Q-100**, all answered — are
copied into `docs/spec/changes/` with this step, which rounds out the spec copy steps 1 to 5 cited
but had not taken.

### Step 6b — REQ-SCN-084, the mask, the trace and the composite

| Piece | What landed |
|---|---|
| `shaders/floor_mask.frag` (new) | the `+Y ≥ 0,9` test of the floor definition, into one `R8` channel |
| `shaders/ssr.frag` (new) | the march, the bisection, the distance-proportional blur and the two attenuations |
| `shaders/ssr_composite.frag` (new) | `(1 − i) · surface + i · reflet`, premultiplied, masked again at full resolution |
| `shaders/screen_depth.glsl` | `zaxViewNormal`, `zaxNeighbour`, `zaxDither`, `uTexel` and the two spiral constants **moved in** from `ao.frag`: both passes read the same frame the same way |
| `shaders/scene.vert` | `vSceneNormal` under `ZAX_FLOOR_MASK` — the scene normal **before** the bend |
| `gl/shaders.rs` | `FLOOR_MASK_DEFINE`, `floor_mask(bend)`, `SSR`, `SSR_COMPOSITE` |
| `gl/screen_effects.rs` | split into two independent halves (`Occlusion`, `Reflections`), `select`, `begin_mask`, `trace`, `composite_reflections`, `programs()` |
| `gl/linear_buffer.rs`, `gl/api.rs` | `color_texture()`; `polygon_offset` |
| `frame_plan.rs` | `floor_runs` and `lay_out_floor` |
| `renderer.rs` | `ScenePrograms::mask`, `set_floor_mask`, `install_mask`; `bind_mask_uniforms`, `draw_floor_mask`, `mark_floors`, `draw_transparent` |
| `style.rs` | `ssr_blur_taps`, `ssr_longest` |

#### Five decisions inside the pass, and what drove each

1. **The mask is a geometry pass, and it had to be.** The two halves of the floor definition are a
   per-instance flag and a per-fragment normal (step 6a), so the mask is an `R8` target the
   floor-eligible runs are **re-drawn** into, with the frame's **own depth texture** attached and
   depth writes off. Nothing of the Classic framebuffer changes, and the pass does not exist at
   `spl.reflections` = 0.
2. **Its vertex stage is `scene.vert` itself**, under `ZAX_FLOOR_MASK`. That is what makes the mask
   carry the animated deformation, the placement, the curvature and the wall bias of AMD-057 of
   the frame it masks — it cannot land where the frame did not draw — and it lets the pass reuse
   `Passes::draw_run` unchanged, with its winding, its normal sign (Q-069) and its instance
   ranges. The fragment stage is thirty lines.
3. **The floor runs get placements of their own.** A floor-marked stretch of an opaque run is
   contiguous in the instance data, but `glBindBufferRange` wants an offset that is a multiple of
   `GL_UNIFORM_BUFFER_OFFSET_ALIGNMENT` — 256 bytes, two placements — and a stretch that starts
   mid-run has none. `lay_out_floor` appends each stretch at an aligned offset: 128 bytes per
   floor tile in a buffer streamed once a frame, against widening `draw_run`.
4. **`RGBA16F`, premultiplied.** The trace writes `rgb = w · c` and `a = w`, so the composite is
   `glBlendFunc(GL_ONE, GL_ONE_MINUS_SRC_ALPHA)` — the requirement's blend, in place, with no
   second colour target. Premultiplied because the composite samples the half-resolution target
   back up with a linear filter, and only a premultiplied value filters correctly across the edge
   of the reflected area. `RGBA16F` is the format `LinearColorBuffer` has always rendered into
   (GLES 3.0 through `EXT_color_buffer_float`), so **no capability is new**.
5. **The mask is applied a second time, at full resolution, in the composite.** Not a belt: the
   trace runs at half of each dimension, so the linear upsample would spread the reflection half
   an output pixel past the floor's silhouette — and "aucun pixel qui n'est pas une surface de sol
   n'est modifié" is three of the six invariants. With the mask in the composite it is true pixel
   by pixel.

#### The march, and the clause it is shaped by

Linear march then bisection (McGuire and Mara 2014, the family the amendment names), with two
choices worth recording:

- the positions are **quadratic** in the step index, `360 × (i/n)²` scene units, and fitted to the
  **longest** ray any setting can ask for (`style::ssr_longest`), truncated at the setting's own.
  Quadratic because the first steps are then fractions of a unit — which is what resolves a
  reflection at the contact, where the invariant of the red channel is read — and the last are
  tens of units, where the blur has widened past them. Fitted to the longest ray so that raising
  `ssr.length` only ever **adds** positions at the far end: spreading one step count over each
  setting's own length would make a short ray finer and a long one coarser, and a coarser march
  can lose a reflection a finer one found;
- the **thickness test is after the bisection, not during the march**. During the march the step
  is coarse and the first positive crossing can be far behind the surface; rejecting it there is
  what made a cube more than a dozen units off the floor stop reflecting at all. After the
  refinement the distance is the real one, and a hit still far from every surface is a ray that
  passed behind a thin object.

#### The brief's point 1 and point 2, measured

- **The twenty-two synthetic captures at default settings are byte identical to step 6a's** — the
  eleven at `--anti-alias 0` and the eleven at the default MSAA — and both `render.json` with
  them, compared by SHA-256 against a throwaway `git worktree` of `276b98a`.
- `req_scn_084_no_geometry_program_carries_anything_of_the_reflections` preprocesses the assembled
  source the way the compiler would, over the 64 Classic scene variants × 2 contexts, the four
  backdrop programs and the two shadow programs, and asserts that none of `vSceneNormal`,
  `uFloorUp`, `uIntensity`, `uMaxLength`, `uReferenceLength` and `zaxBlurredColour` reaches it —
  and in reverse for the mask.
- The four counters: `style_programs` and `screen_effect_bytes`/`passes` are **0** in Classic, 0
  in Enhanced at `spl.reflections` = 0 and `spl.ao` = 0, **3 / > 0 / 3** with the reflections
  alone, **3 / > 0 / 3** with the occlusion alone, **6 / > 0 / 6** with both — and the bytes and
  the passes go back to 0 when the style returns to Classic. Each effect pays its own share and
  not the other's, which is point 2 for the pair that are passes.

#### The instruction counts, and the 120 that hid in one branch

| Symbol | step 6a (`276b98a`) | step 6b |
|---|---|---|
| `Ppu2C02::run_dots` | 2231 | **2231** |
| `DrawListBuilder::build` | 1095 | **1095** |
| `Construction::segment` | 159 | **159** |
| `SceneRenderer::render_scene` | 3382 | **3411** (+29, **+0.86 %**) |

Three of the four are untouched. `render_scene` took four measurements to settle, and the lesson
is worth keeping:

1. **The first shape of the mask program cost +126 (+3.7 %)**, and none of it was where it looked.
   `#[inline(never)]` on the floor marking moved 3; taking `CommonUniforms` out of the pass block
   moved 2 — the hypothesis that a spilled `SceneConstants` was the cost was simply wrong.
2. **It was the program key.** `floor_mask` as a sixth axis of `ProgramKey` put a branch inside
   `ScenePrograms::current_key`, and **every `set_*` of the opaque loop refreshes the cached slot
   through that function** — `set_backdrop` alone runs once per opaque run. Installing the one
   mask program at all thirty-two slots of its curvature instead of normalising the axes away
   brought `render_scene` to 3433; giving it a field of its own (`ScenePrograms::mask`, and
   `set_floor_mask` writing the cached slot straight) took the key back to exactly what it was and
   brought it to **3402**.
3. The remaining +29 are the one `is_some_and` that decides not to mark the floors, the call to
   `bind_mask_uniforms`, the two extra arguments of `run_screen_effects` — and 9 of them are
   `draw_transparent`, extracted because `render_scene` hit clippy's line ceiling again (the same
   remedy `draw_hud` was in step 3).

Dynamically a Classic frame executes one `Option` test more than at step 6a. The gate is R0's
callgrind count on a profile that uses nothing new, and that profile takes none of these branches.

#### Tests (13 in `zax-render/tests/amd055_reflection_tests.rs`)

| Criterion | Test |
|---|---|
| §2.5 first invariant: the red channel at the foot of the cube grows ≥ 10 % — measured **+94 %** | `req_scn_084_the_floor_reflects_the_red_cube_at_its_foot` |
| fourth invariant, `ssr.shine`: the gap grows monotonically (139, 417, 680, 951, 1223) | `req_scn_084_the_shine_grows_the_gap_monotonically` |
| fourth invariant, `ssr.clarity`, as far as the configuration allows (`Q-106`) | `req_scn_084_the_clarity_narrows_the_blur` |
| fourth invariant, `ssr.length`, as far as the configuration allows (`Q-106`) | `req_scn_084_the_length_never_shortens_the_reflected_area` |
| fifth invariant and point 2: 0 is Classic to the pixel, nothing compiled, allocated or submitted, and released on the way back | `req_scn_084_the_reflections_are_off_at_zero_and_in_classic` |
| second invariant, **R-m**: not one pixel of the wall's front face moves | `req_scn_084_no_pixel_of_the_wall_is_modified` |
| third invariant, **R-s**: the 8-unit block moves on 177 pixels, the 4-unit one on 0 | `req_scn_084_the_eight_unit_block_reflects_and_the_four_unit_one_does_not` |
| sixth invariant, **R-h**: the Gradient skybox's and the HUD's pixels do not move | `req_scn_084_the_skybox_and_the_hud_are_left_alone` |
| REQ-SCN-080's order clause: a bent frame reflects, on the depth the curvature left | `req_scn_080_the_reflections_work_on_the_bent_depth` |
| REQ-SCN-080's last bullet: with anti-aliasing the pass works on the resolved frame, sample count unchanged | `req_scn_080_the_reflections_work_on_the_resolved_frame` |
| the 2D mode runs no pass, allocates no target, compiles no program | `req_scn_080_the_two_dimensional_mode_runs_no_pass` |
| REQ-SCN-058 as amended: the reflections first, then the occlusion; each effect alone pays its own share | `req_scn_080_the_reflections_run_before_the_occlusion` |
| point 1 on the shaders: no geometry program carries anything of the reflections | `req_scn_084_no_geometry_program_carries_anything_of_the_reflections` |

#### Five fixture readings that were wrong, and the implementation that was right

The step-5 lesson repeated itself, and the probe-first rule is what kept it short. All five are
easy to repeat:

1. **A background cube over the floor is not drawn.** One screen cell carries one background tile,
   so the cube replaced the floor's own tile there — the step-5 finding, met again. The cube is a
   **sprite**.
2. **A sprite drawn with palette 1 of a tile of colour 2 reads PPU byte 22**, not byte 2. The
   fixture set byte 2, so the cube was **black**, and a black cube reflects nothing. That one cost
   three runs: the reflection machinery was reporting three passes and 921 600 bytes of targets
   while changing no pixel at all.
3. **A sprite cannot be moved in `z`**, and a pattern of another layer is placed at that layer's
   depth. Either way the cube ended up twelve units in front of the block of the variant R-s — too
   far for its reflection to land on an eight-unit face. R-s is therefore **two background tiles
   of the same layer**, the cube ten units along `x` and eight up, which is the variant's own
   placement ("sa base au niveau de la face supérieure du bloc, sa face (−1, 0, 0) à 2 unités du
   bord `+x` du bloc") and gives 177 pixels against 0.
4. **A resolved multisampled depth is not the depth a re-rasterised triangle writes.** With
   anti-aliasing on, the mask's `LEQUAL` against the resolved depth marked **nothing**, so a
   multisampled frame reflected nothing. `GL_POLYGON_OFFSET_FILL` at −16 units of the last place
   pulls the mask geometry a hair towards the eye and settles it, at both sample counts.
5. **An 8-unit cube resting on a floor cannot test `ssr.length` or `ssr.clarity`**, and no fixture
   this side can build does: every ray travels about fifteen units of the forty the shortest
   setting already allows, and the blur — proportional to the distance travelled — stays under one
   texel. Seven configurations were measured, down to 5° of camera elevation and up to 30 tiles of
   floor; `Q-106` carries the figures, names what a corrected configuration would need (a tall
   reflector and a grazing view), and leaves both invariants to R0.

#### What is left for R0 on this requirement

1. The **absolute column** of the close-up R at 1920 × 1080, which wants the two direct-format
   textures the synthetic fixture cannot make — the same limit steps 3 to 5 recorded. The ratios
   are confirmed here.
2. The invariants of **`ssr.length` and `ssr.clarity`** (`Q-106`), on the published configuration.
3. The **review criteria**: the close-up R's look (sharp at the contact, blurring and fading with
   distance, no hard edge), "Super Mario Bros." 1-1 with the reflections on, and the four
   `ssr.quality` values giving the same general aspect. No commercial ROM is available on this
   side.

#### What step 6b owes this one

`mark_floor_surfaces` is called from `render_scene` only when `reflections_active()`, **after**
`lay_out_instances` so that `floor_surfaces` is index-aligned with the reordered opaque bucket,
and the mask pass re-draws the marked items — contiguous sub-ranges of each opaque run, so a
sub-range maps to a contiguous range of the instance data and needs no second upload.

### Step 7a — REQ-UI-061 and REQ-SCN-079: the desktop group, the labels, the handheld page

**Step 7 is split into three**, because its four deliverables touch three different crates and the
protocol of this lot wants small committed steps: **7a** the interface (`zax-settings`,
`zax-app`), **7b** the harness options (`zax-harness`), **7c** the capability finding and the
`wgpu` line (`zax-render`, the tickets and the documents).

| Piece | What landed |
|---|---|
| `zax-settings/src/table.rs` | `STYLE_PARAMETERS`, a table of its own; the four lists given their named rungs with `with_labels` |
| `zax-render/src/style.rs` | `MSVO_PORTED` and `thickness_acts` — the one constant the occlusion pass and the options window have to agree on |
| `zax-app/src/ui/options.rs` | the thirteen out of `EXCLUDED`; `STYLE`, `SWITCHES`, `style_heading`, `is_style_row`, `style_group_shown`, `style_row_visible`, `style_inert_note`, `style_help_of`; `move_style_after_skybox` |
| `zax-app/src/ui/zax_ui/options.rs` | the collapsible group, the skipped rows, the greyed row with its note, `draw_switch` |
| `zax-app/src/ui/keys.rs`, `strings/english.rs`, `strings/french.rs` | the group title, the thirteen labels, the thirteen help sentences, the nine new value labels, the note — both languages |
| `zax-app/src/ui/handheld/advanced.rs` | `is_named_list` and `cycle`, so a whole-number list with no bounds can be stepped |
| `zax-app/src/ui/handheld/defaults.rs` | the two quality rungs on their cheap setting |
| `docs/spec/SPEC-10-ui.md` | REQ-UI-061 written out as AMD-055 §3 point 6 amends it |

#### Four decisions, and what forced each

1. **The four lists are a stepped domain with named rungs, never `Domain::choice`.** REQ-SCN-079's
   first criterion is a *normalisation* criterion — `ao.strength` 9 → 5, `spl.voxel_grid` 4 → 3,
   `spl.ao` 1,4 → 1 — and `Domain::choice` sends a value outside its set to the **default**, which
   would answer 3, 0 and 0. `Domain::stepped(…).with_labels(…)` keeps the bounds, the step and the
   `Snap` rule and only names the rungs: the idiom `scene.arc_center` already uses. The eight
   ladders of 1 to 5 keep their numeric rungs, §4 giving them no value labels.
2. **The panel's order is not the catalogue's, and only the panel moves.** REQ-UI-061 as amended
   puts the `Style` row **after the `Skybox` row**; the catalogue has `render.visual_style` long
   before `scene.skybox`, because the catalogue's order is the order of the **settings file**.
   `move_style_after_skybox` reorders the built list of the Video tab — five lines, one tab, once
   at start-up — rather than moving the keys in the catalogue, which would change the order of
   every written settings file for a presentation reason. The `Skybox` row carries its colour
   button with it (REQ-UI-063 makes them one row), so the style goes after the pair.
3. **The two switches are check boxes over whole numbers.** §4 shows `spl.pixel_shadow` and
   `spl.reflections` as a "case"; the model keeps them as the 0 and 1 Lua reads and writes
   (`Setting:SetVisualStyleParameter(4, "spl.reflections", 1)`), so what the check box changes is
   only the widget (`options::SWITCHES`, `draw_switch`).
4. **`ao.thickness` is greyed by one constant, not by a comment.** REQ-SCN-079 greys it "sauf en
   mode MSVO effectif", and REQ-SCN-083's fallback **(D)** makes that mode never effective for
   now. Two very distant places have to agree on that — the occlusion pass, which renders MSVO as
   Depth, and the options window, which greys the row — so `style::MSVO_PORTED` says it once and
   `style::thickness_acts` reads it. The day the MIT port lands, that constant is what changes,
   and the test asserts its value so that whoever changes it is sent to the greying rule.

#### The handheld side: two findings

- **The page needed code after all.** The handheld's advanced page picks up every *steppable*
  setting of the four desktop tabs by construction, and RU-4b's note said that would be enough
  here too. It was not: `render.visual_style` is an `aliased_choice` (the values 0 and 4, with 1
  accepted), which has **no minimum and no maximum**, so the page's filter — "a number whose
  maximum is above its minimum" — dropped it, and its numeric step would have clamped it to 0.
  `is_named_list` admits a whole number whose domain publishes named values but no bounds, and
  `cycle` steps it as a list, writing a number rather than a choice. The twelve parameters are
  *not* in that case: their domain is stepped, so they were picked up unchanged.
- **The handheld shows all thirteen rows**, without REQ-SCN-079's visibility rules. Those rules
  are written for "la grille de paramètres **desktop**", the page's rows are a static list that
  its selection indexes, and the page already lists several dozen settings of four tabs. Filtering
  them per value would mean making `rows()` depend on the model and keeping the selection stable
  across a change — machinery for a page that is explicitly the advanced one.

#### The handheld defaults

Classic is the default on a handheld **because it is the setting's own default** (REQ-SCN-078), and
at Classic the whole amendment costs nothing; the four effect switches are off by REQ-SCN-079's own
defaults, so none of them needs a handheld default either. What does need one is the two
**qualities**: they cost nothing while their effect is off, but they decide what the effect costs
the moment a player turns it on, so a handheld lands on the cheap rung — `ao.quality` Low (6
samples at half resolution against Medium's 10) and `ssr.quality` Low (12 march steps and 2
refinements against High's 48 and 5). The desktop keeps the amendment's 1 and 2.

#### Tests (10 in `zax-app/tests/amd055_style_options_tests.rs`)

| Criterion | Test |
|---|---|
| REQ-UI-061: the thirteen are rows of the window, in the §4 order, on the Video tab | `req_ui_061_the_thirteen_settings_are_rows_of_the_options_window` |
| REQ-UI-061: the `Style` row is right after the `Skybox` row and its colour, the twelve follow it — and the catalogue's own order is the other one | `req_ui_061_the_style_row_comes_after_the_skybox_row` |
| §3 point 6: one row opens the collapsible group, titled "Style" in both languages, and only in Enhanced (identifier 4 **and** the accepted 1) | `req_ui_061_only_the_first_parameter_opens_the_collapsible_group` |
| REQ-SCN-079's visibility rules and the amended criterion: 0 in Classic, **four** at the defaults, eight with `spl.ao` ≠ 0, **twelve** with both | `req_scn_079_four_parameters_are_visible_by_default_and_twelve_with_both_effects_on` |
| REQ-SCN-079: `ao.thickness` greyed at all three `spl.ao` values while MSVO is not ported, the only row that is, the note in both languages — and the value still bounded and read back | `req_scn_079_the_thickness_is_greyed_while_msvo_is_rendered_as_depth` |
| §4: the thirteen labels and the thirteen help sentences, in both languages | `req_scn_079_the_labels_and_the_help_come_from_the_amendment` |
| §4 and REQ-PERS-081: the four lists name their rungs, the ladders keep their numbers, the snapping survives (4 → 3), the translations | `req_scn_079_the_four_lists_name_their_values_and_keep_their_rungs` |
| §4: the two switches are check boxes over whole numbers of domain 0 to 1 | `req_scn_079_the_two_switches_are_check_boxes_over_whole_numbers` |
| the window **draws** the group: the header, the two check boxes and the greyed row, and no value moves by being drawn | `req_ui_061_the_window_draws_the_style_group` |
| the brief's handheld condition: the thirteen are on the advanced page, each steppable, the named values shown | `req_scn_078_the_handheld_advanced_page_offers_the_style_and_its_parameters` |

Two existing suites enforce the rest for free: `ui_language_tests`
`amd_035_every_command_and_every_shown_setting_has_a_label` requires a `setting.<key>` in **both**
tables for every shown row — which is what makes a label forgotten here a failure rather than a key
on screen — and `amd_035_every_published_key_is_in_both_tables` does the same for the group title,
the thirteen help sentences and the note, which are in `keys::PUBLISHED`.

`cargo test --release -p zax-app --features bundled`: **all suites pass, 0 failed** (the 10 new
ones included); `-p zax-settings`: 230 passed, 0 failed. fmt and workspace clippy (`--all-targets
-- -D warnings`) clean.

Two clippy ceilings were hit by the additions and dealt with rather than silenced: the twelve-row
table pushed `render_style` past the 100-line limit, so the rows became the module-level
`STYLE_PARAMETERS` and the builder is a loop of ten lines; and that constant's own type tripped
`type_complexity`, so it has a name (`StyleParameter`).

#### What this step does **not** do, and why

**The `wgpu` backend is not greyed in the options window.** The brief's point 6 asks for the style
to read back as set and render as Classic there, *with one log line* — which is step 7c — and not
for the rows to be greyed. Greying them would need the **live** renderer's verdict in the window,
and the window has only the settings model: `video.backend` is a wish read at start-up that falls
back to OpenGL when no Vulkan driver answers (`application::open_window`), so the setting alone
cannot say what is drawing. Plumbing the live backend into the options window is more machinery
than point 6 asks for; the log line says it instead.

### Step 7b — the harness options

Point 7 of the brief: "Add harness options to select the style and each Enhanced parameter, so R0
can capture each effect separately."

| Piece | What landed |
|---|---|
| `--style <classic\|enhanced>` | `render.visual_style`, Classic by default; the numbers 0, 4 and the accepted 1 taken too |
| `--style-param <id>=<v>,…` | the twelve parameters of REQ-SCN-079, by the **Lua identifiers** of the amendment; repeatable, comma-separated |
| `Options::style_settings()` | the style the run renders with, built through a settings model |
| `render.json` | four keys — `visualStyle`, `styleProgramsResident`, `screenEffectBytes`, `screenEffectPasses` — written **only** in Enhanced |

#### Three decisions

1. **The values go through a settings model, not into the fields.** `--style-param ao.strength=9`
   has to land on 5, `spl.voxel_grid=4` on 3 and `spl.ao=1,4` on 1 — REQ-SCN-079's first criterion
   — and the style 1 has to read back as 4. Writing the fields of `StyleSettings` directly would
   mean re-implementing that normalisation; `Options::style_settings` writes into a
   `SettingsModel` instead and reads the result, so the flag takes **the same code path** a write
   from the options window or from `Setting:SetVisualStyleParameter` takes. That is what makes an
   R0 capture comparable with the application's picture rather than merely similar.
2. **The identifiers come from `zax_settings::styles`**, the very table the Lua methods use, so a
   flag and a script cannot name a parameter differently. The four **curvature** identifiers of
   AMD-043 are in that table and are rejected here with a message naming `--bend`: the curvature
   is not part of the style (REQ-SCN-078) and reaches the scene through `SceneState::bend`, so a
   `--style-param spl.bend_zx=30` that parsed would have been silently ignored.
3. **A misspelt `--style` is an error, not Classic.** The *setting* answers Classic for every
   unknown value (REQ-SCN-078, and a test of step 2 pins it); a capture campaign must not quietly
   produce the wrong picture because a flag was mistyped, so the flag is strict where the setting
   is lenient.

#### `render.json` stays byte identical at the default settings

The four new keys are written only when Enhanced is selected — the idiom `--bench` already uses
for its two — so a capture at the default settings produces the entry of before the amendment,
byte for byte, which is what point 7 asks of every default capture. What they buy is that a
campaign capture of one effect carries the three counters the brief's points 1 and 2 are read on,
beside the picture they belong to.

`Options::style_settings` is the twin of `zax_app::scene_script_bridge::style_of` and cannot share
it: `zax-render` owns `StyleSettings` and does not depend on `zax-settings`, so neither crate can
hold the conversion for both.

#### Tests (3 in `zax-harness/tests/render_report_tests.rs`)

| Criterion | Test |
|---|---|
| REQ-SCN-078: Classic by default, both names and the three numbers (1 reading back as 4), and five bad values refused | `req_scn_078_the_style_flag_is_classic_by_default_and_takes_both_names` |
| REQ-SCN-079: the twelve named and normalised — 1,4 → 1, 9 → 5, 4 → 3, 0 → 1 — the defaults without the flag, a parameter set in Classic kept but inert, and the four refusals | `req_scn_079_the_style_parameters_are_named_and_normalised` |
| the brief's points 1 and 2 through the harness: Enhanced with every effect off is the Classic PNG **byte for byte** with three zeros, the grid adds programs and no pass, the occlusion adds 3 programs and 3 passes, and a Classic report has none of the four keys | `req_scn_080_the_harness_captures_each_effect_separately` |

`cargo test --release -p zax-harness`: **48 passed, 0 failed** — the C# golden captures
(`render_tests`) and the ROM report among them. fmt and harness clippy (`--all-targets -- -D
warnings`) clean.

## Where this session stopped, and what the next one picks up

**Steps 1 to 6 are done and committed** (`a9216ce`, `98a08ce`, `c5dfc8d`, `be7de5c`, `1f3d8ec`,
`276b98a`, `86a810f`); **steps 7 to 9 are not started.** Step 7 is the GLES
capability probing, the handheld page, the desktop options group and the harness options — its
notes are under "Steps 7 to 9" below, and this step adds one finding to them: **no capability is
new**, because the two formats the passes use (`R8` and `RGBA16F`) are the two the renderer
already depends on, and the one thing that did need care — the resolved multisampled depth — is
settled by a polygon offset rather than by a probe. What step 7 still owes is the greying of
`ao.thickness`, the thirteen rows coming out of `EXCLUDED`, the handheld defaults, the harness
`--style` / `--style-param` options and the `wgpu` line of Q-104.

*(Historical note, left as written: the paragraph below was the step-6a handoff.)*

**Steps 1 to 5 and 6a are done and committed** (`a9216ce`, `98a08ce`, `c5dfc8d`, `be7de5c`,
`1f3d8ec`, and step 6a in this commit); **step 6b and steps 7 to 9 are not started.** The step-6
design below ("Step 6, sharpened" and "Step 6 — REQ-SCN-084") still holds for 6b, with the
handoff under "What step 6b owes this one" above.

*(Historical note, left as written: the paragraph below was the step-5 handoff.)*

**Steps 1 to 5 are done and committed** (`a9216ce`, `98a08ce`, `c5dfc8d`, `be7de5c`, and step 5 in
this commit); **steps 6 to 9 are not started.** The step-6 design below ("Step 6 — REQ-SCN-084")
still holds and is the place to start.

*(Historical note, left as written: the paragraph below was the step-4 handoff, and said steps 5
to 9 were not started. Step 5 was then implemented in a session that was cut before it could
commit; this commit is that work, reviewed, corrected on its four fixture faults and measured.)*

**Steps 1 to 4 are done and committed** (`a9216ce`, `98a08ce`, `c5dfc8d`, `be7de5c`); the tree
builds, **2348 tests pass** (2329 at the close of step 3), fmt and workspace clippy are clean, the eleven synthetic captures at default settings are
**byte identical to step 3's**, and the render and PPU goldens hold. **Steps 5 to 9 are not
started.** Nothing is half-built, and the working tree holds no uncommitted change of this lot
(only the two untracked items named in the step-1 notes).

The design work for the steps that follow was done while step 3 was being written, and is recorded
here so that it is not re-derived. None of it is committed code. **The step-4 design below is kept
as it was written, before the step was implemented**: where it and the step-4 notes above differ,
the notes are what landed — the varying is `mat3(model)` and not the whole `mat4`, because the
placement is affine and only the displacement has to travel through it.

### Step 4 — REQ-SCN-082, the pixel shadows: the design, and its one piece of plumbing

The variant axis already exists: `ZAX_PIXEL_SHADOW` is declared in `gl/shaders.rs`, is a bit of
`ProgramKey`, is selected by `select_style_variants`, is dropped for the HUD by `draw_hud`, and
`vLocal` / `vLocalNormal` are already emitted under the shared guard. **What is left is the body.**

The snapping has to happen **per fragment** (`⌊p⌋ + 0,5` varies across one face), so the
fragment stage has to transform the snapped point as a vertex. That needs three things it does
not have:

1. **the model matrix**, as `flat out mat4 vModel` — the fragment shader cannot read
   `gl_InstanceID`, so the vertex stage has to hand it over. Four varying slots, inside the
   variant only;
2. **the deformation**, as `flat out vec4 vDeformTile` plus the `DEFORMATION` chunk in the
   fragment stage (REQ-SCN-082 point 3 says "déformation animée, puis échelle, rotation et
   position, dans l'ordre de REQ-E3D-406"). `uDeform` already gates it, as in the vertex stage;
3. **the construction depth `D`** — this is the one piece of plumbing. On `z` the voxel edges are
   offset by `D/2` (REQ-SCN-082 point 2), and `D` is a property of the **pattern**, not of the
   mesh's extent, so `bounds_max.z` is *not* a safe substitute (a `sizeZ` = 1 plate does not span
   the box). The route to follow is the one AMD-057 already laid for `axis_aligned_normals`:
   compute it once where the mesh is built, carry it on `TileMesh`, copy it onto `GpuMesh`, and
   read it per **run** in `Passes::draw_run`, beside `wall_clip_bias(mesh)` which does exactly
   that today. Only `fract(D/2)` matters — 0 for even `D`, 0.5 for odd — so one `f32` uniform
   (`uVoxelZOffset`) serves.

The varying budget is comfortable: the variant's varyings come to about 37 components (`vScene` 3,
`vNormal` 3, `vUv` 2, `vBent` 3, `vLocal` 3, `vLocalNormal` 3, `vModel` 16, `vDeformTile` 4),
under the 60 GLES 3.0 guarantees.

Two clauses to get right, both easy to miss:

- the **two axes** are those of smallest `|n_a|`, ties broken by the smallest index in the order
  `x`, `y`, `z`; the third is left alone so the point stays on the face. On a GREEDY face that
  picks exactly the two axes whose `|n_a|` = 0; on a MARCHING surface it always picks two, with no
  plate where the effect would vanish;
- the bias and the normal offset of REQ-SCN-031 are applied **to the displaced point**, not to the
  fragment, or the shadow acne REQ-SCN-031 avoids comes back (REQ-SCN-082, Notes).

Scope: opaque tiles only. Transparent tiles and the HUD already have `receive_shadow` false, so
the `uReceiveShadow != 0` branch excludes them by construction. The **backdrop** does not:
with `render.backdrop_lighting` off it draws through `scene.frag` at colour mode 2 and does
receive shadows, so the snapping must be gated on `uColorMode != 2` — the same uniform test the
grid already makes, constant over a draw.

The acceptance criterion is the plate over the floor (`rot = (90, 0, 0)` floor, a `sizeZ` = 1
plate 29 units above it, light (1, −3, 1)): with the effect on, every pixel of a fully visible
1-voxel cell has the same value to ±2 sRGB levels, and the shadow boundary follows cell edges at
exactly 6 and 14 units from the floor centre; with it off, at least one cell of each shadow edge
holds both lit and shadowed pixels. **That last clause is what makes the criterion discriminate** —
the shadows of REQ-SCN-030 are already hard, so edge sharpness does not tell the two styles apart.

### Step 5, sharpened — read against the GL code after step 4 landed

The design recorded at the end of step 3 (below, "Step 5 — REQ-SCN-080 and REQ-SCN-083") still
holds. What follows is what reading `gl/linear_buffer.rs`, `render_scene` and REQ-SCN-083 again
settled, so that it is not re-derived; **none of it is committed code**.

1. **The composite needs no colour texture.** The pass multiplies the scene colour by `1 − O`
   "en espace linéaire, avant l'encodage sRGB", and a pass cannot sample the attachment it draws
   into. It does not have to: `glBlendFunc(GL_ZERO, GL_SRC_COLOR)` makes the destination
   `dst × src`, so a full-screen triangle that outputs `(1 − O, 1 − O, 1 − O, 1)` multiplies the
   linear buffer in place and leaves the alpha alone (`dst_a × 1`). No second colour target, no
   copy, and the composite is one draw of three vertices.
2. **No noise texture.** REQ-SCN-083 asks for a kernel "rotation du noyau par pixel" and leaves the
   pattern free. An interleaved-gradient hash of `gl_FragCoord` gives that without a resource to
   allocate, release and count — one fewer thing for the handheld, and the invariant it has to meet
   ("aucun motif fixe ne reste visible après lissage") is a property of the blur either way.
3. **`R8` and not a float target.** The occlusion is one scalar in `[0, 1]`. `GL_R8` is
   colour-renderable in OpenGL 3.3 core *and* required renderable in GLES 3.0, so the only
   capability left to probe in step 7 is the **multisampled depth blit** of point 5 — not the
   render-target format. Two targets are needed: the raw occlusion and the blurred one (the blur
   reads the first and writes the second; a 4 × 4 depth-weighted box in one pass).
4. **The depth texture is the one allocation Classic cannot share**, as the step-1 map said. The
   shape: `begin_pass` takes "a screen pass wants to sample the depth" and `allocate` creates a
   `DEPTH_COMPONENT24` **texture** instead of a renderbuffer only then. With the argument false the
   function is byte identical to today's, which is what keeps the brief's point 1. A change of the
   wish has to reallocate, so it belongs beside `capacity` and `samples` in the comparison that
   decides to release.
5. **MSAA is resolved before the passes, colour *and* depth** (REQ-SCN-080, last bullet). The
   colour blit already exists in `resolve_to`; the depth needs `GL_DEPTH_BUFFER_BIT` with identical
   rectangles, which is where step 7's probe goes. The real consequence to state when it lands:
   with anti-aliasing **and** a screen effect on, the transparent objects and the HUD are drawn
   into the **resolved single-sample** target, so they are not multisampled. That follows from the
   requirement's own order and is worth a line in the notes and in `PERFORMANCE-HISTORY.md`.
6. **The pass does not run in 2D mode.** REQ-SCN-083 excludes the rectangle of AMD-046 §2 from
   being *modified* while letting it occlude; in 2D mode it is the only object of the frame, so not
   running the pass satisfies both clauses at once and costs nothing — the same reasoning
   `select_style_variants` already uses for the voxel grid. The tunnel and the TV models do not
   exist in Zaxnes (step 3).
7. **The skybox and the cleared background are excluded by the depth test, not by a mask.** Both
   leave the far plane in the depth buffer, so `depth >= 1` gives `O = 0` — which is also the
   requirement's own reason ("la skybox … à distance infinie … n'occulte jamais"). The HUD and the
   transparent objects are drawn after the pass, so they need nothing.
8. **The radius is in scene units**, `ao_radius_units(scene scale)`, and the view matrix is built
   from scene coordinates, so view-space distances *are* scene units: the radius goes into the
   shader as it comes out of `style.rs`, with no conversion. The normal is reconstructed from the
   depth gradient (there is no normal buffer and REQ-SCN-083 asks for none).

The invariants of REQ-SCN-083 that the tests have to carry, in the order they are cheapest to
measure: a plane alone is 1.00 ± 0.02 everywhere; a pixel at the foot of the cube is `< 1` and
`≥ 0.10`; a pixel at `2 r` = 40 units is 1.00 ± 0.02; `ao.strength` 5 is everywhere `≥ 0.05` and 1
is everywhere `≥ 0.70`, non-increasing in the setting and strictly decreasing from 1 to 4; under
the fallback **(D)** `spl.ao` = 1 and 2 are the same capture; `spl.ao` = 0 is Classic; and in the
A-h variant the skybox and the HUD pixels do not move. The close-up A wants the direct texture
(188, 188, 188) at 1920 × 1080, which this side cannot build — the same limit as the grid's
close-up G and the pixel shadows' sRGB column — so the **ratios** are measurable here and the
absolute columns are R0's.

### Step 5 — REQ-SCN-080 and REQ-SCN-083: the infrastructure, and the occlusion

The one allocation Classic cannot share is named in the step-1 map: the single-sample depth of
`LinearColorBuffer` is a **renderbuffer**, so it cannot be sampled. The shape to follow:

- `begin_pass` takes "does a screen pass want depth" as an argument, and allocates the depth as a
  **texture** only then. With the argument false, `allocate` is byte-identical to today's, which
  is what keeps the brief's point 1 ("no extra framebuffer, render target, texture or buffer
  allocated" in Classic);
- a new `gl/screen_effects.rs` owns everything else — the half-resolution targets of
  `ao_half_resolution` and `ssr_half_resolution`, the blur targets, the noise texture — allocated
  the first time a frame asks for either effect and **released when no frame wants either**, which
  is the lifecycle the brief's point 1 asks for. `screen_effect_bytes` reports their size and is
  already in `RenderStatistics`;
- the two passes run in `render_scene` between the opaque runs and the transparent ones, in the
  order REQ-SCN-058 (as amended) gives: **reflections first, then occlusion**. With MSAA on, the
  colour and the depth are **resolved** before they are read (REQ-SCN-080, last bullet), so the
  transparent objects and the HUD then draw into the resolved single-sample target. That is a real
  consequence to state in the notes when it lands: with anti-aliasing **and** a screen effect on,
  the transparent objects and the HUD are drawn without multisampling. Resolving the depth needs
  a `GL_DEPTH_BUFFER_BIT` blit from the multisampled framebuffer, which is where the capability
  probe of the brief's point 3 belongs.

REQ-SCN-083 itself: hemisphere SSAO from depth, the public technique the amendment names (John
Chapman 2013) — a kernel drawn in the normal's hemisphere and tightened near the point, rotated
per pixel, a range check, then a depth-aware blur. `style.rs` already publishes every normative
quantity (`ao_radius_units`, `ao_ceiling`, `ao_samples`, `ao_half_resolution`). The **MSVO** mode
is rendered as Depth, the declared fallback **(D)**, until the MIT port of REQ-SCN-083 exists;
`ao.thickness` therefore has no observable effect and is greyed, and `third_party/README.md` gains
no entry in this lot.

### Step 6, sharpened — read against the code after step 5 landed

The design below ("Step 6 — REQ-SCN-084, the reflections") still holds. What follows is what
reading REQ-SCN-084 against `frame_plan.rs`, `camera.rs` and the step-5 pass settled, so that it
is not re-derived. **None of it is committed code.** Suggested split, because the step is the
largest left: **6a** the floor definition (no GL, unit-testable), **6b** the passes.

#### The floor test needs *two* tests, and that is what fixes the shape

The invariants decide this, and neither test can be dropped:

- **Variante R-s** puts an 8 × 8 × 8 block and the *same block at `scale = (1 ; 1 ; 0,5)`* (4 units
  in `z`) side by side: the first reflects, the second must not. Both have a top face of normal
  (0, 1, 0), and both are the same pixels of the same depth buffer — so **no reconstruction from
  depth can tell them apart**. The eligibility is a property of the **instance**, and it has to be
  computed on the CPU, where the placement is.
- **Variante R-m** is the same 8 × 8-tile shape *without rotation*: its `xz` box measures 64 × 8,
  so it **passes** the bounding-box test, and its front face has the scene normal (0, 0, −1). Not
  one of its pixels may be modified. So the **+Y ≥ 0,9 test is per fragment** and is what excludes
  it — a per-instance mask alone would reflect the wall.

So the pass needs a **floor mask** that is the *conjunction* of a per-instance flag and a
per-fragment normal test. That rules out the two cheap shortcuts and leaves two honest options:

1. **A second colour attachment** written by the scene program during the opaque pass, under a new
   variant axis — but that changes the framebuffer the Classic path allocates and the MRT setup of
   both the single-sample and the multisampled attachments, which is a lot of machinery pressed
   against the brief's point 1.
2. **A mask pass of its own**, after the opaque pass: bind an `R8` target with the frame's **depth
   texture** as its depth attachment (which step 5 already allocates on demand), depth test
   `LEQUAL` with depth writes off, and re-draw **only the floor-eligible runs** with a fragment
   stage that writes 1 where the pre-curvature scene normal has `+Y ≥ 0,9`. Nothing of the Classic
   framebuffer changes, and the pass does not exist when `spl.reflections` = 0.

**Option 2 is the one to take.** Its cost is one extra geometry pass over the floor instances only,
paid only when the effect is on, which is exactly what point 2 of the brief asks; and it reuses the
depth texture and the `ScreenEffects` lifecycle step 5 built rather than widening `begin_pass`
again.

#### Where each quantity already is

| What REQ-SCN-084 needs | Where it is today |
|---|---|
| the instance's `xz` box **before curvature** | `DrawItem::bounds_min`/`bounds_max` (the mesh's local box) through `DrawItem::model` (local → scene, curvature **excluded** — REQ-E3D-406, DEC-R2-T4). Accumulate per `DrawItem::instance` over the **opaque** bucket |
| the per-fragment pre-curvature scene normal | the scene normal the vertex stage already computes before the bend; the grid of step 3 emits `vLocalNormal` under its guard, and the same shape serves |
| the local-camera swap of v1.4 (`+Y` → `(0, 0, −1)`, footprint in `xy`) | `CameraSetpoint::local_camera` (`camera.rs:162`) |
| every normative number | `style.rs` already: `ssr_intensity`, `ssr_max_length`, `ssr_blur_fraction`, `ssr_steps`, `ssr_refinements`, `ssr_half_resolution` — all published and unit-tested in step 2 |
| the lifecycle, the targets, the composite blend | `gl/screen_effects.rs` (step 5); `ScreenEffects::PROGRAMS` becomes a value, not a constant, the day AO and SSR can be on independently — its doc comment says so |

**`6 × scene scale` is a threshold on both dimensions, and `z` is the object's depth.** The
amendment is explicit that the same shape at `D` = 3, or at `scale = (1 ; 1 ; 0,5)`, measures 3 or
4 units in `z` and does not reflect. So the test is `min(extent_x, extent_z) ≥ 6 × scale`, on the
**scene-space** box, and `D` reaches it through the mesh's own bounds (`−D/2 … +D/2`, the step-4
finding) and the instance's scale.

#### Three clauses easy to get wrong

1. **Only the already-rendered scene may be reflected**, and the pass runs before the transparent
   objects and the HUD — so they are neither reflected nor reflective, which falls out of
   REQ-SCN-080's order for free, exactly as it did for the occlusion. The shadow-only tiles of
   REQ-SCN-041 are not drawn at all, so they are not reflected either.
2. **The two attenuations must have no sharp edge** — towards the ray's maximum length and towards
   the screen edge. The amendment leaves their width and shape free but forbids a visible edge,
   which is a review criterion; a `smoothstep` on both is the cheap honest answer.
3. **The order between the two screen passes**: the reflections run **first**, then the occlusion
   (the step-5 notes and the comment already in `render_scene` say so). The composite of the
   reflections therefore darkens nothing the occlusion has not yet seen.

#### What R0 keeps on this requirement

The close-up R wants two **direct-format** textures — the floor at (124, 124, 124) and the cube at
(255, 0, 0) — at 1920 × 1080, which the synthetic fixture cannot make, the same limit steps 3, 4
and 5 recorded. The **ratios and the masks** are measurable here: R-m (no pixel of the wall moves),
R-s (the 8-unit block moves, the 4-unit one does not), the monotonicity of `ssr.shine` and
`ssr.length`, `spl.reflections` = 0 being Classic to the pixel, and R-h (the skybox and the HUD do
not move). The **+10 % red channel** at the contact is the one invariant that wants the absolute
albedos, and its own Note computes the margin (a factor of 4,9, so 30 % of the full red suffices).

### Step 6 — REQ-SCN-084, the reflections

Screen-space ray trace, linear march then bisection (McGuire and Mara 2014); `style.rs` publishes
`ssr_intensity`, `ssr_max_length`, `ssr_blur_fraction`, `ssr_steps`, `ssr_refinements` and
`ssr_half_resolution`. The decision already taken, in the step-1 map: **the floor test is per
instance, not per fragment** — a face whose scene normal before curvature has `+Y ≥ 0.9` and whose
instance's `xz` bounding box before curvature measures at least `6 × scene scale` in both
dimensions. Both quantities live with the placement, so the test belongs in `frame_plan.rs` and
reaches the pass as a mask, never as a per-fragment computation. The local-camera case of
REQ-SCN-009 (v1.4) swaps `+Y` for the image plane's normal `(0, 0, −1)` and measures the footprint
in `xy`; `SceneCamera::local_camera` is the flag to read.

### Steps 7 to 9

Step 7 (the GLES probe, the handheld page, the desktop options group, the harness options) has one
finding already in hand: the thirteen settings are all in `ui/options.rs` `EXCLUDED`, with a
comment that names this lot as the one that takes them out, and `ui/handheld/advanced.rs` picks up
every steppable setting of the four desktop tabs by construction — so the handheld page needs no
code, as it needed none for `render.backdrop_lighting` in RU-4b step 9. The desktop side needs the
collapsible "Style" group of REQ-UI-061 as amended, with the visibility rules of REQ-SCN-079
(the `ao.*` rows only when `spl.ao` ≠ 0, the `ssr.*` rows only when `spl.reflections` = 1), and
`ao.thickness` greyed under the fallback.

**Q-104 still has to be appended to**, not replaced: the brief's point 6 says Enhanced is
OpenGL-only in this lot as the curvature was, so under `wgpu` the style reads back as set and
renders as Classic, with one log line. Nothing of that is written yet.

### The untracked items in the working tree

*(Step 5 added a third: `rust/crates/zax-render/tests/probe_tmp.rs`, the scratch probe that
measured the invariants and found the four fixture faults. Like the dump helper it returns
immediately without its environment variable — `ZAX_PROBE` — and is deliberately not committed.)*


1. `tests/` — the .NET test tree's build output, as in RU-4a and RU-4b. Not this lot's.
2. `rust/crates/zax-render/tests/dump_sources_tmp.rs` — the shader-dump helper of step 1,
   deliberately not committed. It returns immediately without `ZAX_DUMP_DIR`. The baseline dump it
   is compared against lives outside the repository, so a new session has to retake the baseline
   from the commit it starts at rather than from step 1's; the committed form of the check is
   `req_scn_081_no_classic_variant_carries_anything_of_the_grid`, which needs no dump at all.
