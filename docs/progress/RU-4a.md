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
| 4 | AMD-043 §3: the shader **variants**, the renderer wiring, the subdivisions | done — `a697da0` |
| 5 | AMD-046: the 2D rectangle bends, 32 × 32 only when it must, `scene.bend_2d_mode` | done — `c158272` |
| 6 | AMD-043 §4: the picking inverse (REQ-INP-124), on input events only | done — `33c48c1` |
| 7 | AMD-050: REQ-CORE-048 checked, the opacity mask measured and decided, REQ-SCN-076 | done — `<pending>` |
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

### Step 5 — AMD-046, the 2D rectangle, and the settings reaching the scene

**The finding that made this step small.** In 2D mode `FramePlan::collect` returns as soon as the
rectangle is collected, **before** the backdrop: the rectangle is the only object of a 2D-mode
frame. So "Bend 2D Mode" turned off needs no per-item flag at all — it is the flat variant for
the whole frame, which is REQ-SCN-074's "la position de chaque sommet du rectangle DOIT être sa
position plate, quels que soient les réglages de REQ-SCN-066" exactly. `select_bend_variant`
folds the setting in:

    let wanted = !scene.bend.is_identity()
        && (!scene.two_dimensional_mode || scene.bend_two_dimensional_mode);

which also gives "en mode 3D, le réglage n'a aucun effet" for free.

AMD-046 §2's clause "si un backdrop est présent, il reste soumis à REQ-SCN-068, que le réglage
soit actif ou non" is therefore **moot in Zaxnes**: the backdrop is not drawn in 2D mode at all,
which is AMD-009's reading and predates this lot. Nothing was changed there.

| Piece | Where |
|---|---|
| `scene.bend_2d_mode`, bool, U, immediate, default true, no Lua | `zax-settings/src/catalogue.rs`, `table.rs` (new `curvature()` builder, which also holds the four angles) |
| the label, both languages | `ui/strings/english.rs`, `french.rs` (`setting.scene.bend_2d_mode`) |
| the rectangle subdivided to 32 × 32 while it bends | `interop::picture_quad_subdivided`, `wide_quad_for_subdivided_cached` |
| the five settings read **on change** | `zax-app/src/scene_script_bridge.rs` `bend_of`, `push_settings`, `apply_setting` |

**Where the settings are read is the performance point.** `bend_of` evaluates three sines, three
cosines and three divisions. It runs in `apply_setting`, the one path the host takes when a
setting changes, and in `push_settings` at a profile load — never per frame, per tile or per
pixel. The renderer then reads the ready-made `SpaceBend` and tests `is_identity`.

The widescreen quad cache gained the subdivision in its key (`wide_picture_cells`): the margin
alone no longer tells two quads apart now that the same margin can be a quad or a 33 × 33 grid.

#### Tests

`zax-render/tests/bend_two_dimensional_tests.rs` (5), the criteria of REQ-SCN-074 one by one:

| Criterion | Test |
|---|---|
| setting on: every vertex within 0.05 of `B(P)`, at least 1089 **distinct** vertices | `req_scn_074_the_rectangle_bends_and_is_subdivided_when_the_setting_is_on` |
| setting off: every vertex in `z = 0` at its flat position to 10⁻³, over three curvatures | `req_scn_074_the_rectangle_stays_flat_when_the_setting_is_off` |
| the visible NES rows are the same either way (the order clause) | `req_scn_074_the_visible_rows_do_not_depend_on_the_setting` |
| 3D mode: toggling the setting moves no vertex, with and without a curvature | `req_scn_074_the_setting_has_no_effect_in_three_dimensional_mode` |
| on the GPU: one draw call either way, 2 triangles flat → 2048 bent → 2 again with the setting off, and the picture with the setting off is **pixel-identical** to no curvature at all | `req_scn_074_the_setting_decides_the_picture_and_the_mesh` |

`zax-app/tests/bend_settings_tests.rs` (5): the default is the identity on both platforms; a
write of 45 reads 60 **and** the scene gets the curvature of 60 (checked against vector 2 of
REQ-SCN-067); a change of each of the five reaches the scene through `apply_setting` and back to
the identity; the five survive a restart through the real `SettingsStore`; and the switch has no
Lua access at all — neither a `lua_path` nor a style parameter.

`ui/handheld/defaults.rs` gained the assertions the brief asks for: the four angles are 0 on a
handheld **and** on the desktop, and `scene.bend_2d_mode` keeps the spec default on both. Nothing
is overridden for a handheld, because at 0 nothing is compiled and nothing is subdivided.

One existing test turned up the omission the moment the setting existed:
`ui_language_tests.rs` `amd_035_every_command_and_every_shown_setting_has_a_label` failed until
the label was in both tables — which is the check working as intended.

`zax-render` 158 → **163 passed**, `zax-app` 649 → **654**, `zax-settings` 229 passed, fmt and
workspace clippy clean.

#### SPEC chapter edits

- `SPEC-06` §3.15bis (new): REQ-SCN-066, REQ-SCN-067 (the four steps of B) and REQ-SCN-068
  written out, with the amendment to REQ-SCN-028 on the transported normal.
- `SPEC-06` §3.16: the definition of the 2D-mode rectangle and REQ-SCN-074.
- `SPEC-09` §4.1.4: the `scene.bend_2d_mode` row, and the four curvature rows' effect column now
  points at REQ-SCN-067 instead of saying the effect is a later lot's.

### Step 6 — REQ-INP-124, the aim through a bent scene

`zax-app/src/input/zapper.rs`: `try_intersect_bent` / `try_aim_bent`, and the aiming window of
REQ-INP-124 as `in_aiming_window`.

**How the candidates are found.** `B` is invertible wherever the depth bound of REQ-SCN-067 does
not bite, so the candidates are read off the **ray** rather than searched for on the plane: the
ray is walked forward one scene unit at a time over the sphere of 1024 units around `C`, the
preimage of each sample is taken, and a change of sign of `P.z − z₀` between two samples brackets
a candidate that 32 bisections then locate to under 10⁻⁹. Walking **forward** is what makes the
first candidate found the nearest one. Each candidate is checked against its own image (0.25
unit), which rejects the ones an arc-tangent branch invents.

#### The brief's performance point, and what the code actually does

The brief asks for "the inverse transform on input events only, never per frame". REQ-INP-061
refreshes the Zapper **once per displayed frame** and must keep doing so (the trigger follows the
right button every frame), so "on input events" is implemented as a memo rather than by moving
the call:

- at curvature 0, `update` is the two lines it was before the amendment — `try_aim`, one
  ray-plane intersection — and the memo is cleared, not consulted. Instruction for instruction
  unchanged;
- with a curvature, `AimInputs` records the pointer, the target size, the aim plane, the scene
  scale, the curvature and the camera view. A frame on which none of them moved costs **one
  comparison** and returns. The march only runs when the player moves the mouse or the camera,
  which is what the brief means.

`SceneState::effective_bend()` is the single home of "which curvature does this frame apply":
`scene.bend`, or the identity in 2D mode when "Bend 2D Mode" is off. The renderer's
`select_bend_variant`, the CPU export and the Zapper all read it, so AMD-046's amendment of
REQ-INP-124 — a flat 2D mode aims as at curvature 0 — holds by construction rather than by three
copies of the same condition.

#### Tests (16 in `zapper_input_tests.rs`, 31 in all there)

| Criterion | Test |
|---|---|
| REQ-INP-124's own criterion: a ray through `B(P)` aims within 0.5 unit of `P`, over the eleven unbounded vectors of REQ-SCN-067 (plus a 90° case) | `req_inp_124_a_ray_through_the_bent_image_aims_at_the_point_it_came_from` (11 cases) |
| no curvature aims exactly as REQ-INP-063 — the same function, not a special case that agrees | `req_inp_124_no_curvature_aims_as_req_inp_063` |
| the aiming window, its four edges and the scene scale; a candidate only outside it is passed **unbounded**; no candidate at all keeps the last position | `req_inp_124_a_candidate_outside_the_window_is_passed_unbounded` |
| the curved aim is recomputed only on a change, and a reset forgets the memo | `req_inp_124_the_curved_aim_is_recomputed_only_on_a_change` |
| AMD-046's amendment through `effective_bend` | `req_scn_074_a_flat_two_dimensional_mode_aims_as_at_curvature_zero` |

One of my own test premises was wrong and the code was right, which is worth recording: at a
curvature of 90° a ray aimed at a point far off to the left **also** crosses the aim plane at a
point inside the aiming window, and REQ-INP-124 then prefers that nearer in-window candidate —
which is the requirement, not a failure. The test now uses a gentle 30° curvature, where the only
candidate really is outside the window, and says why in place.

`zax-app` 654 → **670 passed**, fmt and workspace clippy clean.

#### SPEC chapter edit

`SPEC-08` §3.7: REQ-INP-124 written out after REQ-INP-064, with AMD-046's amendment of it.

### Step 7 — AMD-050: the opacity mask is **free**, measured

The risky part of the lot, and the encoding step 1 predicted holds. **REQ-CORE-049 is kept**: it
costs the PPU nothing at all, so there was no decision to bring to the owner.

#### The measurement

Method of RU-3a-fix and RU-3b: the post-LTO assembly
(`cargo rustc -p zax-harness --release --bin zax-harness -- --emit=asm`). `span_pixel` and
`render_pixel` are both inlined into `Ppu2C02::run_dots`, which is the symbol that carries the
whole dot pipeline and the one measured.

| | instructions in `run_dots` |
|---|---|
| before (`33c48c1`) | **2231** |
| after | **2231** |

And not merely the same count — the same instructions. With the label numbering, the anonymous
symbol hashes and the `(%rbx)` field displacements normalised (the struct grew by 32 bytes, so
every field after `palette` shifted), the **entire** diff of the two instruction streams is five
immediates:

    movl $48  → movl $112      (0x30 → 0x70, the greyscale colour mask, twice)
    movl $63  → movl $127      (0x3F → 0x7F, the plain colour mask, twice)
    andb $48  → andb $112      (render_pixel's own greyscale mask)

plus the palette load reading `palette_marked` at +32 instead of `palette` — the same
`movzbl …(%rbx,%rax), %eax`. No instruction was added, removed or reordered.

#### Why it is free

`entry`, the palette index the two pixel functions resolve, is non-zero **exactly** where the
pixel carries at least one opaque pixel:

- the sprite branch yields `entry ≥ 0x10`, so non-zero;
- the background branch yields a `background` whose low two bits are non-zero;
- the third branch is the only one that yields 0;
- and `background` / `sprite` are held at 0 unless their layer is on and, for `x < 8`, unmasked —
  the four PPUMASK conditions REQ-CORE-049 names. `SPRITE_OPAQUE` is set by `paint_sprite` only
  where the sprite's pixel value is non-zero, and `paint_sprite` reads no "hidden" flag, which is
  the requirement's "quel que soit son drapeau « caché »".

So the opacity bit is already in the index. `palette_marked` holds `palette[i] | 0x40` for every
`i ≥ 1` and `palette[0]` for entry 0; the span's `color_mask` is widened to `0x7F` / `0x70`. The
shadow table is maintained at the **three** sites that write the palette — one register write, the
power-up fill, the state restore — none of them per pixel. It is derived, so it is not in the
state blob and no savestate moved.

#### What the change touches

| Piece | What |
|---|---|
| `ppu/mod.rs` | `PIXEL_COLOR`, `OPAQUE_PIXEL`, `palette_marked`, `mark`, `marked_palette`, `refresh_marked_palette`, the register write |
| `ppu/render.rs` | the two palette reads, the two colour masks, `fill_backdrop` left on `palette` (rendering off → bit 0, which is also the amendment's ruling on the hardware "palette background" case) |
| `ppu/state.rs` | the shadow table rebuilt on restore |
| `ppu/tiles.rs` | `wide_picture_into`'s margin composition: `entry == 0` is the same rule, in a loop that already matched per pixel and only runs for a widescreen frame. The 256-wide fast path copies `pixels` whole, so the bit rides along for nothing |
| `gl/picture.rs` | the colour from bits 0-5, alpha 0 or 255 from bit 6 — in the loop that **already** resolved the picture to RGBA once per frame in 2D mode and already wrote alpha 0 for a pixel the overscan leaves out |
| `shaders/scene.frag` | **nothing.** It already discards on `direct.a == 0.0`, in the opaque pass, with no sorting, no separate depth pass and no blending — REQ-SCN-076's "rejet du fragment" word for word |

**The accessor was renamed, not redefined.** `pixel_frame()` → `pixel_frame_with_mask()`, so the
compiler visited every one of the eleven call sites instead of letting one silently hash the wrong
bytes. `pixel_frame_indices_into` gives the plain index image of REQ-CORE-048, and the two places
that hash a golden (`zax-harness/src/golden_core.rs`, `zax-nes/tests/ppu_golden_tests.rs`) use it:
**the 291 recorded PPU golden hashes are unchanged**, which is the check that matters most here.

The byte layout is written out **twice** on purpose: `zax-nes::ppu` owns it, and
`zax-render::interop` repeats it beside its own `HIDDEN_PIXEL` because the renderer does not
depend on the core (`zax-nes` is only a dev-dependency of `zax-render`, and adding a real edge
would invert the layering). A test asserts the two agree and that `HIDDEN_PIXEL` uses a bit the
core never sets.

#### Tests

`zax-nes/tests/opacity_mask_tests.rs` (10) — the five cases of vector **V11** of SPEC-02 §5.3 one
by one, including the one the image alone cannot express (every palette colour equal to the
universal backdrop's: the image is uniformly `$0F`, the mask is still all 1); a sprite behind the
background still marking its pixels; the sprite PPUMASK switches gating the bit; the greyscale bit
leaving the mask alone; REQ-CORE-048's index image being colour indices alone; and the mask
surviving a state restore.

`zax-render/tests/two_dimensional_transparency_tests.rs` (6) — the three automatic criteria of
REQ-SCN-076 on GPU captures, with the conditioning the amendment spells out
(`use_nes_bg_color` false, magenta behind the rectangle) and the one Q-085 settles for the
right-hand columns; the transparency holding bent as well as flat; and the two definitions of the
picture byte agreeing.

Two sets of **existing** tests had to change, and both for the right reason: their synthetic
pictures were plain colour indices, which now describe a wholly transparent rectangle (a blanked
screen). `two_dimensional_mode_tests.rs` and `bend_two_dimensional_tests.rs` now build opaque
pixels and say why. The full-workspace run is what caught the second one.

`cargo test --release --workspace --features zax-app/bundled`: **2239 passed, 0 failed**. The 18
render goldens and the PPU goldens all hold. fmt and workspace clippy clean.

#### SPEC chapter edits

- `SPEC-02` REQ-CORE-030: points 4 and 5 of a frame batch.
- `SPEC-02` §3.5.5: REQ-CORE-048 restated at P1, REQ-CORE-049 added after it.
- `SPEC-02` §5.3: vector V11.
- `SPEC-06` §3.16: REQ-SCN-076 before REQ-SCN-074.

### Open questions

**Q-104** — the curvature in the `wgpu` backend (see step 4). Next free Q number is **Q-105**.
