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
| 3 | REQ-SCN-081: the voxel grid | in progress |
| 4 | REQ-SCN-082: the pixel shadows | todo |
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
