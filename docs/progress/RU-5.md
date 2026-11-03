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
| 4 | REQ-SCN-082: the pixel shadows | done — hash in the commit of step 5 |
| 5 | REQ-SCN-080 and REQ-SCN-083: the screen-space infrastructure and the ambient occlusion | todo |
| 6 | REQ-SCN-084: the floor reflections | todo |
| 7 | GLES capability probing, handheld page, desktop options group, harness options | todo |
| 8 | The per-effect measurements | todo |
| 9 | Closing: `PERFORMANCE-HISTORY.md`, intended image changes, fmt, clippy, full tests, goldens | todo |

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

## Where this session stopped, and what the next one picks up

**Steps 1 to 4 are done and committed** (`a9216ce`, `98a08ce`, `c5dfc8d`, and step 4 whose hash
goes into the commit of step 5); the tree builds, **2348 tests pass** (2329 at the close of step
3), fmt and workspace clippy are clean, the eleven synthetic captures at default settings are
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

### The two untracked items in the working tree

1. `tests/` — the .NET test tree's build output, as in RU-4a and RU-4b. Not this lot's.
2. `rust/crates/zax-render/tests/dump_sources_tmp.rs` — the shader-dump helper of step 1,
   deliberately not committed. It returns immediately without `ZAX_DUMP_DIR`. The baseline dump it
   is compared against lives outside the repository, so a new session has to retake the baseline
   from the commit it starts at rather than from step 1's; the committed form of the check is
   `req_scn_081_no_classic_variant_carries_anything_of_the_grid`, which needs no dump at all.
