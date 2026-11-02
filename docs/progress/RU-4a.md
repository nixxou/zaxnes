# RU-4a — curvature, curvature in 2D mode, negative clipping margins, transparent 2D backdrop

Branch `rustupdated`, on top of RU-3b-fix (`553af8b`). Amendments AMD-043 (curvature of space),
AMD-046 (curvature in 2D mode), AMD-047 (negative clipping margins) and AMD-050 (2D mode:
universal-backdrop pixels are transparent) ported to the **Rust** application. The .NET lots
V2-4 (`b78e44c`), V2-6 (`7aeadf8`), V2-7 (`7983a3a`) and V2-12 (`5caf498`) of `dotnetupdated`
are a reading reference only.

## Checklist

| # | Step | Status |
|---|---|---|
| 1 | Map: what exists, where each requirement goes, the spec copies | done — `dffafa3` |
| 2 | AMD-047: the four margins accept any finite real (REQ-PERS-087), Q-080's grid guard | done — `5703fed` |
| 3 | AMD-043 §3: `SpaceBend` — B, its inverse, its Jacobian, the 13 vectors | done — `c17dfb5` |
| 4 | AMD-043 §3: the shader **variants**, the renderer wiring, the subdivisions | done — `<pending>` |
| 5 | AMD-046: the 2D rectangle bends, 32 × 32 only when it must, `scene.bend_2d_mode` | todo |
| 6 | AMD-043 §4: the picking inverse (REQ-INP-124), on input events only | todo |
| 7 | AMD-050: REQ-CORE-048 checked, the opacity mask measured and decided, REQ-SCN-076 | todo |
| 8 | The settings: desktop options, handheld defaults and page, harness options | todo |
| 9 | Closing: draw/vertex counts at default, `PERFORMANCE-HISTORY.md`, fmt, clippy, tests | todo |

The hash of a step is written in the commit of the step that follows (no amend, protocol of this
lot).

## Notes

### State of the working copy at the start

`git status` showed one untracked folder, `tests/`: the **.NET** test tree's `bin/` and `obj/`
build output, left behind by an earlier checkout of `dotnetupdated` in this working copy. It is
not this lot's work and not the Rust application's; it is left untracked and uncommitted.

### What already exists on the branch, checked key by key

| Piece | State |
|---|---|
| `scene.bend_lr`, `scene.bend_tb`, `scene.arc_h`, `scene.arc_center` | **exist** (RU-2): `catalogue.rs`, `table.rs` rows with `Domain::stepped(−180, 180, 30)` and `(0, 1, 1)`, the style parameters `spl.bend_zx` / `spl.bend_zy` / `spl.arc_yx` / `spl.arc_center` of REQ-LUA-114 (`styles.rs`), all four in `ui/options.rs` `EXCLUDED` |
| `scene.bend_2d_mode` | **does not exist.** The lot's brief says RU-2 created it "as Lua surface only", but AMD-046 §2 gives the setting **no Lua access** ("le programme d'origine n'en a pas"), so RU-2 — which added the keys the 2.0 `Setting` members stand for — had no reason to. It is created here, in step 5. |
| the curvature itself | nothing: no `SpaceBend`, no bend uniform, no bend shader chunk |
| the four clip margins | `Domain::range(0, 256)` / `(0, 248)` in `table.rs` `clipping()`; `ConversionSettings::DEFAULT_CLIPPING = (0.1, 8.1, 8.1, 0.1)` |
| the picking | `zax-app/src/input/zapper.rs`: `ray`, `try_intersect`, `to_nes_pixel`, `try_aim`, `ZapperState::update` |
| the opacity mask | nothing; `Ppu2C02::pixels` holds one 6-bit NES colour index per pixel |

### Requirement map

#### AMD-047 — negative clipping margins

| Requirement | Rust crate / file |
|---|---|
| REQ-PERS-087 domain of the four margins | `zax-settings/src/table.rs` (`clipping()`), `zax-settings/src/domain.rs` (a finite-real rule) |
| the formulas unchanged for negative margins | `zax-render/src/geometry.rs` (`VisibleRect`), `camera.rs` (T′, P\*) — read-only check |
| the tile filter of REQ-E3D-002 and the grid margin of Q-080 | `zax-engine3d/src/grid.rs` / `segmenter.rs` |

#### AMD-043 — curvature of space

| Requirement | Rust crate / file |
|---|---|
| REQ-SCN-066 the four settings | already there (RU-2); out of `EXCLUDED` in step 8 |
| REQ-SCN-067 the transform B | `zax-engine3d/src/space_bend.rs` (new): B, B⁻¹, the Jacobian, the transported normal |
| REQ-SCN-068 scope, shadow pass, HUD/skybox flat, subdivision, order with the clipping | `zax-render/src/shaders/bend.glsl` (new), `scene.vert`, `shadow.vert`, `gl/shaders.rs` (the variants), `renderer.rs`, `scene.rs` (`SceneState::bend`) |
| REQ-SCN-028 amended (the normal transported) | `bend.glsl` (the cofactor of the Jacobian) |
| REQ-INP-124 the picking | `zax-app/src/input/zapper.rs` |

#### AMD-046 — curvature in 2D mode

| Requirement | Rust crate / file |
|---|---|
| REQ-SCN-074 the rectangle subdivided and bent, the setting | `zax-render/src/interop.rs` (`picture_quad_wide`), `scene.rs`, `renderer.rs`; `zax-settings` for `scene.bend_2d_mode` |
| REQ-INP-124 amended (2D mode, setting off → REQ-INP-063) | `zax-app/src/input/zapper.rs` |

#### AMD-050 — the 2D rectangle's universal-backdrop pixels

| Requirement | Rust crate / file |
|---|---|
| REQ-CORE-048 amended (P1, the index image) | `zax-nes/src/ppu/render.rs` — already held, checked in step 7 |
| REQ-CORE-049 the opacity mask (P2) | `zax-nes/src/ppu/render.rs` (`span_pixel`, `render_pixel`), `ppu/mod.rs` (the palette) |
| REQ-SCN-076 the fragment rejected | `zax-render/src/gl/picture.rs` (the alpha channel), `scene.frag` — **already discards on alpha 0** |

### The plan for REQ-CORE-049, and why it can be free (step 7's hypothesis, measured there)

The brief asks for a cheap encoding before anything else. Reading the two pixel functions gives
one, and it rests on an identity:

In `span_pixel` and `render_pixel` the palette entry is

    entry = if sprite opaque and (background transparent or sprite in front) { 0x10 | (sprite & 0x0F) }
            else if background opaque { background }
            else { 0 }

- the sprite branch yields `entry ≥ 0x10`, so non-zero;
- the background branch yields `background` whose low two bits are non-zero, so non-zero;
- the third branch is the only one that yields 0.

And `background` is held at 0 unless the background layer is on and, for `x < 8`, unmasked;
likewise `sprite` (`SpanPixels::background`/`background_left`/`sprites`/`sprites_left`, the four
PPUMASK switches of REQ-CORE-049). `SPRITE_OPAQUE` is set by `paint_sprite` only where the
sprite's pixel value is non-zero, and `paint_sprite` looks at no "hidden" flag — which is what
REQ-CORE-049 asks ("quel que soit son drapeau « caché »").

So **`entry != 0` is exactly the opacity bit of REQ-CORE-049**, and `entry & 0x1F == 0` iff
`entry == 0` (entry never exceeds `0x1F`). The index into the palette therefore already carries
the bit, which gives an encoding that adds *no instruction at all* to either pixel function:

1. keep a derived `palette_marked: [u8; 32]` beside `palette`, with
   `palette_marked[0] = palette[0]` and `palette_marked[i] = palette[i] | 0x40` for `i ≥ 1`;
2. make the span's `color_mask` `0x7F` / `0x70` instead of `0x3F` / `0x30` (built once per span),
   and `render_pixel`'s greyscale mask `0x70`;
3. the two functions then read `self.palette_marked[entry & 0x1F] & color_mask` — the same
   indexed load and the same `AND` as today.

`palette_marked` is maintained at the three sites that write `palette` (the one register write,
the power-up fill, the state restore), never per pixel; it is derived, so it is **not** in the
state blob (`ppu/state.rs` writes `palette` only, and `pixels` is not in the blob at all, so no
savestate moves).

The consequence to settle in step 7 is the **interface**: `Ppu2C02::pixels` then holds
`index | opacity << 6`, and `pixel_frame()` no longer hands out 0..63. The PPU goldens hash
`pixel_frame()` (`zax-nes/tests/ppu_golden_tests.rs`, `zax-harness/src/golden_core.rs`), so the
accessor is **renamed** rather than redefined, which makes the compiler visit every call site
instead of letting one silently hash the wrong bytes. The index image of REQ-CORE-048 stays
available, and the goldens keep their hashes.

On the renderer's side nothing new runs: `gl/picture.rs` already resolves each index to RGBA once
per frame **in 2D mode only**, already writes `alpha = 0` for a pixel it leaves out
(`interop::HIDDEN_PIXEL`), and `scene.frag` already discards on `direct.a == 0.0`, in the opaque
pass, with no sorting and no blending — which is REQ-SCN-076's "rejet du fragment" word for word.

### The plan for the shader variants (step 4), and why not the .NET shape

The .NET lot tests a `uBend` uniform per vertex. This lot's point 1 forbids that: the vertex
shaders at curvature 0 must be today's, instruction for instruction. So `bend.glsl` is guarded by
`#define ZAX_BEND` and the renderer holds **two** scene programs and **two** shadow programs,
picking the pair when a curvature setting changes — never per vertex, never per frame.

### Step 2 — AMD-047, the domain of the four margins

A **domain change and nothing else**, which is what the amendment is: no new per-tile work, and
not a line added to any per-frame path.

| Piece | What landed |
|---|---|
| `zax-settings/src/domain.rs` | `Domain::finite_real()` and the `finite_only` flag: every finite real is in the domain, and `coerce` sends NaN and ±∞ to the setting's default before any other rule runs — the general rule of REQ-PERS-003 |
| `zax-settings/src/table.rs` | the four margins take that domain instead of `Domain::range(0, 256)` / `(0, 248)` |
| `zax-engine3d/src/engine.rs` | **no code change**: `is_inside_tall_clip_area` is four comparisons that carry over to negative margins by themselves, and the grid-margin guard already drops a cell in the margin. Both got the comment Q-080's answer calls for (`// Q-080`) |
| `zax-render` | **no code change**: `VisibleRect::from_clipping`, `geometry::effective_target` and `geometry::aim_point` clamp nothing |

The whole write path was walked to be sure no second clamp hides behind the domain:
`ScriptSettings::set_clipping` → `write_real` → the model; `setting_application::to_setting_values`
→ `SettingsModel::apply_profile` → the model. Neither bounds anything of its own, so one domain
is the single gate — which is what makes "a script's value and a profile's value behave alike"
true by construction.

#### The criteria, one by one

| Criterion of AMD-047 §2 | Test |
|---|---|
| a script writes −80 and reads −80; `Clipping.x = -128` puts −128 in `Setting.Clipping[0]` | `zax-scripting/.../amd047_clipping_tests.rs` `req_pers_087_a_script_writes_a_negative_margin_and_reads_it_back` |
| the four margins keep what is written, through both names of each | `req_pers_087_the_four_margins_keep_what_a_script_writes` |
| a NaN write gives the margin's default | `req_pers_087_a_margin_a_script_writes_not_finite_takes_its_default`, and over the model in `zax-settings/tests/validation.rs` `req_pers_087_a_margin_that_is_not_finite_takes_its_default` |
| a profile whose field 4 is (−128, −80, −128, 0) applies them | `zax-profiles/.../setting_application_tests.rs` `req_pers_087_a_profile_applies_its_negative_clip_margins`, through the real `to_setting_values` → `apply_profile` path |
| T′ = (128, 80, 0), E ≈ (128.0, −149.9, −132.8), P\* ≈ (128, 41.5), R = [−128, 384] × [−80, 240] | `zax-render/tests/camera_tests.rs` `req_pers_087_the_camera_follows_its_formulas_with_negative_margins` — **the four values came out right first run**, which is the independent check the amendment's vector is for |
| an empty R raises no error and still renders the rest | `req_pers_087_an_empty_visible_rectangle_still_builds_a_view` |
| the tile emitted at (−7, 100) is kept by REQ-E3D-002 with both margin sets, and only R cuts it | `zax-engine3d/.../intake_and_frame_tests.rs` `req_pers_087_the_border_tile_is_kept_and_only_the_visible_rectangle_cuts_it` |
| Q-080's own end-to-end check: a negative bottom margin uncovers a real border row | `req_pers_087_a_negative_bottom_margin_uncovers_the_border_row` |

One existing test asserted the **old** behaviour and now asserts the new one, naming the
amendment: `zax-settings/tests/validation.rs` `a_clipping_margin_is_clamped` (−4 → 0, 300 → 256)
became `req_pers_087_a_clipping_margin_keeps_any_finite_real`.

`zax-settings` 221 → 229 passed, `zax-scripting` 149 (+4), `zax-engine3d` 193 (+2),
`zax-profiles` 176 (+1), `zax-render` unchanged counts plus 2. `cargo fmt --check` and workspace
clippy clean.

#### SPEC chapter edits

- `SPEC-09` §4.1.5: the "Domaine" column of the four rows, the last sentence of the paragraph
  after the table, and REQ-PERS-087 written out in place (the amendment's §3).
- `SPEC-06` REQ-SCN-003: the list of typical values gained "et des valeurs négatives écrites par
  script (−128, −80)".

### Step 3 — `SpaceBend`, the transform of REQ-SCN-067

`zax-engine3d/src/space_bend.rs` (new, 390 lines), in the platform-free geometry crate because
three callers need it and none of them is the conversion: it is unit-tested without a GL context,
`zax-render` mirrors it in GLSL and builds its uniforms from it (step 4), and `zax-app` inverts it
for the aim (step 6). **Nothing in `zax-engine3d` calls it** — AMD-043 §1 keeps the curvature out
of the profiles and out of the 2D → 3D conversion, so no per-tile path gained a line.

What it publishes:

| Item | Requirement |
|---|---|
| `SpaceBend::new(lr, tb, arc, at_edge, scene_scale)` | the four settings of REQ-SCN-066; `k = θ / (L · s)`, `f = φ / sin φ`, `h = 2 sin²(φ / 2) / k` with `φ = θ / 4` (step 1) |
| `is_identity`, `NONE`, `Default` | the identity at θ = 0, which is what every caller tests before paying for anything |
| `apply` | `B(P)`, the three wrappings of step 3 in order |
| `invert` | `B⁻¹`, `None` where the depth bound bites (REQ-INP-124, AMD-043 §4) |
| `jacobian`, `transform_normal` | the normal transported by the curvature (REQ-SCN-068, REQ-SCN-028 amended), as the cofactor of the Jacobian |
| `curvature`, `recentring`, `offset`, `active`, `arc_pivot` | the per-axis constants the shader reads, computed once per change and not per vertex |

An **idle axis publishes the neutral constants** (`k = 0`, `f = 1`, `h = 0`, `active = 0`), which
is what lets the GLSL mirror of step 4 need no special case for a one-axis or two-axis curvature.

#### The vectors

The thirteen vectors of REQ-SCN-067 **passed on the first run**, all three components of each,
inside the ±0.05 the table allows. That is the third independent computation of them (R1 wrote
them, R2 recomputed them in `R2-review-AMD-039-045-v1.1`, the .NET lot V2-4 implemented them), so
the formula is not in doubt.

Eleven more tests around them, each pinning a clause the table alone does not:

| Test | What it holds |
|---|---|
| `req_scn_067_the_thirteen_vectors_of_the_curvature` | the table, and for the eleven rows without the depth bound, that `B⁻¹` of the **rounded** image is back within 0.05 of P |
| `req_scn_067_no_angle_at_all_is_the_identity` | θ = 0 on all three axes is the identity for `apply`, `invert` and `transform_normal` alike, bit for bit |
| `req_scn_067_an_angle_under_the_threshold_bends_nothing` | the threshold is 10⁻⁴ rad at most (a `const` assertion), below it nothing bends, ten times above it each axis does |
| `req_scn_067_the_curvature_is_continuous_in_the_angle` | each axis walked 0 → 180° in 0.25° steps on a far corner, no step jumping |
| `req_scn_067_the_recentring_keeps_the_picture_about_its_size` | the amendment's own figure: at 60° the half-width is 123.6, not 128 |
| `req_scn_067_the_depth_is_held_at_95_percent_of_the_radius` | two depths past the bound share one image, in both directions, and `invert` refuses there |
| `req_inp_124_the_inverse_recovers_the_point_it_came_from` | eleven settings including three-axis ones, `B⁻¹(B(P))` within 0.01 — far inside the 0.5 unit REQ-INP-124 allows |
| `req_scn_068_the_normal_is_transported_by_the_curvature` | unchanged at `C`, turned by `a = u · k = 45°` at the right edge of a 90° bend, the other way for a negative angle, exactly unchanged at θ = 0 |
| `req_scn_067_the_scene_scale_divides_the_curvature` | `k = θ / (L · s)`, and a scale that is not positive finite reads as 1 |
| `req_scn_066_the_arc_centre_picks_the_pivot` | 0 or `−120 · s` |
| `req_scn_067_the_uniforms_an_idle_axis_publishes_are_the_neutral_ones` | the neutral constants, and the three of an active axis recomputed from step 1 |

Two of **my own** assertions were wrong on the first run and the code was right, which is worth
recording because the direction is easy to get backwards:

1. `k = θ / (L · s)` means halving the scale **doubles** the curvature; I had asserted the
   reverse.
2. A positive angle pushes the edges **away** from the camera (vector 2: the right column lands
   at z = +24.70), so the surface is convex towards the viewer and the outward normal of the
   right edge leans **right**, not left. The test now derives the direction from the vector
   instead of asserting a guess, and checks the turn is `sin 45°` / `−cos 45°`.

`zax-engine3d` 193 → **226 passed**. `cargo fmt --check` and workspace clippy clean (three
`many_single_char_names` allows, the idiom `geometry.rs` already uses for the named quantities of
a specification formula).

### Step 4 — the shader variants and the renderer

**The curvature is a `#define`, not a uniform.** `gl/shaders.rs` was reshaped so a variant
composes in a `const fn`: `Stage` holds its chunks in **fixed slots**, empty ones skipped, and
`scene(solid, srgb_palette, bend)` and `shadow(bend)` replace the four `SCENE*` constants and
`SHADOW`. Eight scene variants and two shadow ones, ten distinct names so the on-disk program
cache files them apart.

`assemble` skips an empty slot **without even a blank line**, which is what keeps the assembled
source of a variant that leaves a define out byte-identical to the shape it had before the slots
existed.

| Piece | Where |
|---|---|
| the GLSL mirror of `SpaceBend` | `shaders/bend.glsl` (new), compiled only into the `ZAX_BEND` variants |
| B applied last, the normal transported, `vBent` emitted | `shaders/scene.vert`, under `#ifdef ZAX_BEND` |
| the shadow lookup at the bent position, the clipping still at the flat one | `shaders/scene.frag`, under the same guard |
| the shadow pass bent | `shaders/shadow.vert` |
| the variant in use, built lazily | `renderer.rs` `ScenePrograms`, `select_bend_variant` |
| the five per-axis uniforms | `renderer.rs` `bind_bend`, from `SceneConstants::bend` |
| the HUD kept flat (REQ-SCN-068) | `render_scene`: the HUD pass switches to the **flat** pair and switches back |
| the backdrop at 32 × 32 per face | `interop::unit_box_subdivided`, `subdivision_for` |

**The curvature costs nothing at 0, and that is checked rather than claimed:**

- `select_bend_variant` is one `is_identity()` test per frame when nothing bends. No program is
  compiled (the curvature pair is built the first time a setting is not 0 — on a handheld that is
  seconds of shader compile a player who never bends anything never pays), no program is swapped,
  no uniform is uploaded, and the backdrop keeps its six quads;
- `req_scn_067_the_flat_variant_carries_nothing_of_the_curvature` preprocesses the assembled
  source the way the GLSL compiler would — `#ifdef ZAX_BEND` branches dropped, `#else` kept, line
  comments removed — and asserts that **none** of the nine identifiers the curvature brings in
  reaches the compiler, over all eight flat variants (solid × sRGB palette × desktop/GLES) and
  both flat shadow ones. The first two drafts of that test failed on the guard text and on my own
  comments, which is the whole reason it preprocesses rather than greps;
- `req_scn_067_a_bent_scene_is_a_different_picture_with_the_same_draws` renders the same scene
  flat and at 90° on the GPU: the draw-call and triangle counts are **equal** and over 1 % of the
  pixels moved. The curvature is a variant, not extra geometry;
- the eleven synthetic render goldens still match exactly (`zax-harness` 18 of 18 render tests).

**A per-axis `uBendOn` is kept inside the variant.** One to three axes may bend; the test is
uniform over a whole draw, so an idle axis costs a comparison instead of a `sin` and a `cos`.
That is not the forbidden "uniform tested per vertex" of the brief — the forbidden one is the
switch between bending and not bending, which is the `#define`.

#### `scene_export`, the harness the criteria ask for

`zax-render/src/scene_export.rs` (new): the scene-space vertices the renderer would place, on the
CPU, without a GL context — what the acceptance criteria of REQ-SCN-068 and REQ-SCN-074 are
written against ("harnais, les sommets exportés…"). It walks the frame plan with a residency that
keeps the meshes themselves, then applies the placement and the curvature in the shader's order.
It is a verification aid: nothing in the renderer calls it.

The subdivision rule lives in **one** place, `interop::subdivision_for(bend_active)`, which both
the renderer and the export read, so the exported geometry cannot drift from the drawn one.

#### Tests (10 in `zax-render/tests/bend_tests.rs`)

| Criterion | Test |
|---|---|
| the flat variant carries nothing of the curvature | `req_scn_067_the_flat_variant_carries_nothing_of_the_curvature` |
| the curvature variant carries it, and the `#define` brings it | `req_scn_067_the_curvature_variant_compiles_the_wrapping_in` |
| ten distinct program names | `req_scn_067_each_variant_has_its_own_name` |
| REQ-SCN-068: a tile's exported vertices are at `B(P)` | `req_scn_068_the_exported_vertices_of_a_tile_are_at_their_bent_position` |
| REQ-SCN-068: the exported HUD is unchanged | `req_scn_068_the_hud_is_not_bent` |
| REQ-SCN-068: the clipping is judged before the curvature, and the corner tile at (0, 8, 0) is kept although `B(P)` leaves R | `req_scn_068_the_clipping_is_judged_before_the_curvature` |
| REQ-SCN-068: 32 × 32 per face, and only while something bends | `req_scn_068_the_backdrop_is_subdivided_only_while_something_bends` |
| REQ-SCN-074: ≥ 33 × 33 vertices, and the single quad at 0 | `req_scn_074_the_two_dimensional_rectangle_is_subdivided_only_while_something_bends` |
| the picture moves, the draws do not | `req_scn_067_a_bent_scene_is_a_different_picture_with_the_same_draws` |
| REQ-SCN-068: the shadow pass takes the curvature | `req_scn_068_the_shadow_pass_takes_the_curvature` |

`zax-render` 148 → **158 passed**, `zax-harness` 44 passed, fmt and workspace clippy clean.

#### The second render backend: Q-104

The Rust application has a **second** renderer behind the `wgpu` feature (Vulkan, DirectX 12),
absent from the default build, which today holds feature parity with the OpenGL one — the
animated deformation of REQ-SCN-043 included. The curvature landed in the **OpenGL backend only**:
the one SPEC-06 REQ-SCN-056 requires, the one of the default build, of R0's captures and of both
handheld targets. The WGSL port is not hard but **cannot be run once on this machine** (naga
validates at module creation, and `render_wgpu_tests.rs` skips without a Vulkan driver), and
shipping WGSL that was never executed would be worse than shipping the gap described.

`spec/changes/Q-104.md` asks the three questions that follow (port it, and in which lot; say so in
the interface meanwhile or hide the settings; is backend parity a per-lot duty). The gap is made
non-silent three ways: a `// Q-104` note at the exact line of `wgpu_backend/renderer.rs` where
`constants.bend` is not read, a test that **asserts the known gap** so a half-done parity fails
loudly (`q_104_the_wgpu_backend_carries_nothing_of_the_curvature_yet`, under the `wgpu` feature),
and this entry.

### Open questions

**Q-104** — the curvature in the `wgpu` backend (see step 4). Next free Q number is **Q-105**.
