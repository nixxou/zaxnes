# RU-4b — the rest of the Classic rendering

Branch `rustupdated`, on top of RU-4a (`a3c2687`). Amendments AMD-048 (wrapped solid section:
empty depth interval), AMD-049 (backdrop lighting), AMD-051 (robustness of conversion, loading
and setting writes), AMD-057 (walls between neighbouring tiles kept, sprite depth by index) and
AMD-058 (joints without cracks with anti-aliasing) ported to the **Rust** application. The .NET
lots V2-8 (`627917b`), V2-9 (`1f0f7c6`), V2-13 (`4722940`), V2-29 (`439fd0d`) and V2-30
(`1e9b2b2`) of `dotnetupdated` are a reading reference only.

## Checklist

| # | Step | Status |
|---|---|---|
| 1 | Map: what exists, where each requirement goes, the "before" measurements | done — `12561b1` |
| 2 | AMD-051 part 1: REQ-E3D-644, REQ-E3D-646, REQ-E3D-524, REQ-PERS-088 | done |
| 3 | AMD-051 part 2: REQ-LOOP-073, the loading that no longer advances | done |
| 4 | AMD-048: the empty depth interval of REQ-E3D-450, TV-Y9 and TV-Y10 | done |
| 5 | AMD-057 walls: the Q-067 seal removed, the wall depth, the measurement, the handheld decision | done |
| 6 | AMD-057 sprite depth: `0.001 × (i mod 64) ∓ 0.1` | todo |
| 7 | AMD-058: the joints under MSAA, a shader variant for `anti_alias` ≥ 1 only | todo |
| 8 | AMD-049: REQ-SCN-075 as a shader variant, the setting, the vectors | todo |
| 9 | The settings: desktop options, handheld defaults and page, libretro, harness options | todo |
| 10 | Closing: measurements, intended image changes, `PERFORMANCE-HISTORY.md`, fmt, clippy, tests, goldens | todo |

**The checklist went from 9 steps to 10 after step 1** (so the commit of step 1 says "1/9"):
AMD-051 holds five independent requirements across four crates, and the machine is unstable —
two commits for it is two chances to resume from, not one.

The hash of a step is written in the commit of the step that follows (no amend, protocol of this
lot).

## Notes

### State of the working copy at the start

`git status` showed one untracked folder, `tests/`: the **.NET** test tree's `bin/` and `obj/`
build output, left behind by an earlier checkout of `dotnetupdated` in this working copy. Same
finding as RU-4a; it is not this lot's work and is left untracked and uncommitted.

### What already exists on the branch, checked key by key

| Piece | State |
|---|---|
| the Q-067 seal | **exists**: `zax-engine3d/src/geometry/tile_sides.rs`, `ObjectGeometry::sealed_mesh` / `full_sides`, `DrawListBuilder::add_instance` (a per-instance `FastMap` of neighbours) and `sealed_sides`, `greedy::build(..., sealed_sides)` |
| the sprite nudge | `zax-render/src/geometry.rs` `SPRITE_NUDGE = 0.1`, applied in `frame_plan::collect_tile`; `DrawTile.tile` already carries `FrameTile::ord()`, the sprite slot 0..127 of REQ-CORE-043 — **no new plumbing is needed for AMD-057 §2** |
| REQ-E3D-450 | `voxel_construction.rs` `segment`, `for z in zm.min(zp)..=zm.max(zp)` with a `// Q-015` comment: exactly the reading AMD-048 replaces |
| mesh indices | `TileMesh.triangles: Vec<u32>` and `glow::UNSIGNED_INT` everywhere: the Rust **has no 16-bit index path at all**, so REQ-E3D-524 already holds and the brief's point 4 asks for no change (checked by a test in step 2) |
| the atomic write | `zax-app/src/user_data_paths.rs` `write_atomic` (a temporary beside the file, then a rename) is already used by the settings store, the save-slot index, the bindings, the savestates, the resume point, the battery RAM and the achievements library; `zax-settings/src/games.rs` writes the library the same way. REQ-PERS-088 looks **already met**; step 2 walks every writer and pins it |
| the texture upload failure | `frame_plan::collect_tile` does `residency.texture(texture)?` — a `?` that **aborts the whole frame**, which is what REQ-E3D-644 forbids |
| the backdrop | `frame_plan::collect_backdrop` pushes one opaque item with `color_mode: 2`; `scene.frag` shades it with the diffuse term of REQ-SCN-028, like a tile |
| anti-aliasing | `video.anti_alias` to `geometry::anti_alias_samples` to `LinearColorBuffer::begin_pass`: a multisampled copy of the linear buffer, blit-resolved. Nothing in the shaders depends on the sample count today |
| `vUv` | the **integer texel coordinates** of REQ-E3D-506, floored into an index in `scene.frag` — the extrapolation AMD-058 is about |

### Requirement map

#### AMD-048 — wrapped solid section, empty depth interval

| Requirement | Rust crate / file |
|---|---|
| REQ-E3D-450 amended | `zax-engine3d/src/geometry/voxel_construction.rs` (`segment`) |
| TV-Y9, TV-Y10 | `zax-engine3d/tests/engine3d/cylinder_vector_tests.rs` |

#### AMD-049 — backdrop lighting

| Requirement | Rust crate / file |
|---|---|
| REQ-SCN-075, the model | `zax-render/src/color.rs` (the CPU twin, platform-free) and `zax-render/src/shaders/backdrop.glsl` (new), compiled only into the backdrop variant |
| the variant and the eye | `zax-render/src/gl/shaders.rs`, `renderer.rs` (`ScenePrograms`, the backdrop run), `SceneConstants` (the eye) |
| the setting | `zax-settings` `render.backdrop_lighting`, `zax-app/src/ui/handheld/defaults.rs` |

#### AMD-051 — robustness

| Requirement | Rust crate / file |
|---|---|
| REQ-E3D-644 texture not allocatable | `zax-render/src/frame_plan.rs` (`collect_tile`), `zax-render/src/gl/texture_cache.rs` (a texel budget), `zax-render/src/statistics.rs`, `zax-harness` `--atlas-budget` |
| REQ-E3D-645 | **retired**; the number stays attributed |
| REQ-E3D-524 | no change (32-bit indices already); a test for the 8 x 8 x 96 checkerboard |
| REQ-E3D-646 object not ready | `zax-engine3d/src/engine.rs` / `geometry/builder.rs` (a waiting counter), `zax-harness` `--delay-mesh` |
| REQ-LOOP-073 loading stalled | `zax-app/src/emulation/host/jobs.rs`, the message tables of `ui/strings/`, `zax-harness` `--stall-load` |
| REQ-PERS-088 atomic writes | `zax-app/src/user_data_paths.rs` and its callers, `zax-settings/src/games.rs` |

#### AMD-057 — the walls, and the sprite depth

| Requirement | Rust crate / file |
|---|---|
| the drawn mesh is the whole mesh | `zax-engine3d/src/draw_list.rs`, `geometry/{tile_sides,greedy,object_geometry,builder}.rs` |
| REQ-SCN-065 held another way | `zax-render/src/shaders/scene.vert` (the wall depth), `geometry.rs` (the constant) |
| REQ-SCN-037 amended | `zax-render/src/frame_plan.rs` (`collect_tile`), `geometry.rs` (`SPRITE_INDEX_NUDGE`) |

#### AMD-058 — the joints under anti-aliasing

| Requirement | Rust crate / file |
|---|---|
| REQ-SCN-065 for `anti_alias` 0, 1 and 2 | `zax-render/src/shaders/scene.{vert,frag}` under a new `ZAX_CENTROID` guard, `gl/shaders.rs`, `renderer.rs` (the variant chosen from the sample count) |

### The "before" measurements (step 1)

Taken on this machine, in release, nothing else running. **No commercial ROM is available on this
side** (`ZAXNES_ROMS` is unset and the original program's folders are out of bounds), so the
real-profile figures come from `zax-harness meshes`, which lays the profile's own patterns over
the background grid and exports the tiles the draw list really asks for. R0 re-runs the ROM
scenes of AMD-057 §1 and AMD-058 §1.

**Drawn geometry**, `zax-harness meshes --profile <assets-neutral/profiles/...>`, synthetic frame 0
(`frame0000.obj`, so these are the *sealed* variants of Q-067):

| Profile | vertices | faces |
|---|---|---|
| Super Mario Bros 3 (U) (PRG1) | 85 208 | 42 604 |
| Legend Of Zelda | 50 312 | 25 156 |
| Castlevania (U) (PRG 1) | 19 596 | 9 798 |
| Mega Man 2 | 56 240 | 28 120 |
| Metroid | 680 216 | 340 108 |
| Contra | 61 604 | 30 802 |
| Excitebike | 32 736 | 16 368 |
| Ninja Gaiden III | 103 700 | 51 850 |
| Zelda II | 92 812 | 46 406 |
| Blaster Master (U) | 249 050 | 118 404 |

**The draw list on the CPU**, `zax-harness bench-core test-roms/other/BladeBuster.nes --frames 600
--warmup 120 --3d` (982 tile records a frame), best of 3: **0.0370 ms/frame** (0.0407, 0.0373,
0.0370).

**Post-LTO instruction counts** (`cargo rustc -p zax-harness --release --bin zax-harness --
--emit=asm`, the method of RU-3a-fix, RU-3b and RU-4a):

| Symbol | instructions |
|---|---|
| `DrawListBuilder::build` (`add_instance` and `sealed_sides` inlined) | **1574** |
| `Construction::segment` (REQ-E3D-450's home) | **159** |
| `Ppu2C02::run_dots` (a control: nothing here touches the core) | **2231** |

**The GPU at default settings**, `zax-harness render --out ... --anti-alias 0`, 640 x 480:

| Scene | draw calls | triangles | colours |
|---|---|---|---|
| framing | 1798 | 7704 | 4 |
| shadow | 164 | 808 | 6 |
| nudge | 144 | 768 | 4 |
| backdrop | 165 | 812 | 6 |
| heavy | 3852 | 17 160 | 23 |

The 40 assembled shader sources at default (8 scene variants x 2 stages x 2 contexts, plus the
2 shadow ones) are kept outside the repository for the length of this lot, so that steps 6 and 7
can prove byte identity of the ones they must not touch.

### The three design decisions this lot rests on

1. **AMD-057's seam is held by depth, not by geometry.** The walls come back whole (REQ-E3D-501),
   and a face parallel to the extrusion axis — normal plus or minus x or y **in the mesh's local
   frame**, so the object's rotation does not matter — is pushed away from the eye by a fixed
   scene distance in the vertex shader. That is the one means AMD-057 §2 leaves open which does
   not touch the walls themselves, and it is where the .NET lot V2-29 ended up as well.
2. **AMD-049 is a shader variant of the backdrop draw, not a branch on `uColorMode`.** The .NET
   lot tests `uColorMode == 2` per fragment; the brief forbids that. The backdrop is a single
   opaque run with its own draw state, so one extra program per (palette, curvature) pair — built
   lazily — carries the model and no tile fragment ever sees it.
3. **AMD-058 is a `centroid` variant keyed on the sample count.** With `video.anti_alias` = 0 the
   flat programs stay in use, so the compiler is given today's source; from 1 upwards a variant
   whose `vUv` is interpolated at the centroid of the covered samples is selected. The 2D-mode
   rectangle, a single quad with a binary cut-out (REQ-SCN-076), keeps the pixel centre.

### Step 2 — AMD-051: the degradations that cost nothing while everything works

Four of the five requirements. REQ-LOOP-073 is step 3.

#### REQ-E3D-644 — a texture that cannot be allocated

The defect was real and the amendment names it exactly: `frame_plan::collect_tile` ended
`residency.texture(texture)?`, a `?` that **aborted the whole frame** on an allocation failure —
"le moteur NE DOIT PAS lever d'exception ni interrompre la frame", word for word what it did.

| Piece | What landed |
|---|---|
| `frame_plan.rs` | `Residency::texture` returns `Option<Texture>` instead of `Result`: a refusal is not an error. `collect_tile` leaves the tile out, reports once per pattern, and goes on to the next tile |
| `gl/texture_cache.rs` | a texel budget (`set_budget`), the texels each entry occupies so the retention collector gives them back, and a per-frame `refused` set so the other tiles of one object cost no second GL call |
| `diagnostics.rs` (new) | the line budget both requirements of AMD-051 share: at most one line per subject, at most 100 per game, then one summary per 3600 rendered frames (one minute at the nominal 60 of SPEC-07 §2) |
| `statistics.rs` | `texture_texels` and `patterns_without_texture` |
| `renderer.rs` | `set_texture_budget`, `take_diagnostics`, `report_late_object`; `invalidate_caches` resets the budget of lines, because REQ-E3D-644 counts them "par partie" |
| `application.rs` | the lines are logged in `draw_scene`, beside the shadow-map fallback of AMD-034 |
| `zax-harness render` | `--atlas-budget <texels>`, and the lines printed under the scene's row |

**The cost on a healthy frame.** `Ok(Some(t))` became `Some(t)`: the same indexed lookup and the
same branch. `TextureCache::begin_frame` clears an empty `FastSet`, which is a length store. The
diagnostics are touched **only** from the refusal branch; `take()` hands back a `Vec::new()`,
which does not allocate.

**The one thing this requirement cost.** REQ-E3D-644 bounds its diagnostic at "au plus une fois
par **pattern**", and the renderer — where the allocation fails — had no way to name one:
`ObjectTexture` carries no identity and `DrawTile` carried only its frame instance. `DrawTile`
therefore gained `pattern: PatternId` (8 bytes), read **once per instance** from `InstanceLook`
and copied per tile beside the `FrameTile` and the `Mat4` it already copies. The alternative —
keying the diagnostic by texture — would log once per *3D object*, which for a pattern of eight
objects is eight lines where the requirement allows one. The draw-list cost of the field is
measured in step 10.

#### REQ-E3D-646 — an object that is not ready

The first clause (drawn on the first frame after its meshes arrive, never forgotten) was already
true: `DrawListBuilder::build` tests `is_ready` every frame. The second — a diagnostic after 600
consecutive emulated frames of waiting — is watched from the branch that **already exists** for
an object that is not ready, so a healthy frame reaches none of it:

- `DrawListBuilder::waiting` is a map from `InstanceRef` to (first frame, last frame, reported),
  filled by `note_waiting`, which is `#[cold]`;
- `forget_ready_objects` costs **one `is_empty` test per frame** when nothing waits, and nothing
  per instance;
- a frame counter that goes backwards (a savestate restore, a reset) restarts the window instead
  of reporting a wait that never happened;
- the late objects travel `ConversionEngine::take_late_objects` → `GameSession` →
  `Graphics::report_late_object` → the shared line budget. They are taken **before** the next
  build, because the list the renderer is drawing borrows the session for the whole frame: a
  diagnostic found during a build is logged on the frame after it.

`zax-harness render --delay-mesh <frames>` holds the meshes back through the `defer_transfer`
the builder already published for exactly this (REQ-E3D-600's two halves).

#### REQ-E3D-645 and REQ-E3D-524 — the index width

**Nothing to do, and that is the finding.** `TileMesh.triangles` is a `Vec<u32>` and every draw
is `glow::UNSIGNED_INT`: the Rust **has no 16-bit index path at all**, so REQ-E3D-524 holds as
written and the brief's point 4 ("32-bit indices only for the meshes that exceed 65 535 vertices;
the 16-bit path stays for every other mesh") has nothing to choose between — there is no 16-bit
path to keep. The existing vector test of the 8 × 8 × 96 checkerboard (18 432 rectangles, 73 728
vertices) gained the assertions that make the point: the largest index is 73 727, **past
`u16::MAX`**, and no index points past the vertex list.

#### REQ-PERS-088 — the scope of the atomic write

Walked writer by writer. `user_data_paths::write_atomic` (a `<name>.tmp` beside the file, then a
rename) was **already** used by the settings store, the games library, the slot index, the
bindings, the savestates, the resume point, the battery RAM and the achievements library; the
ordering clauses hold too (`persistence.rs` copies `<name>.v<N>.bak` before the first migrated
rewrite and renames `*.corrupt-<stamp>` before any replacement).

Two files were **not** covered and now are: the slot thumbnail and the resume-point thumbnail
(`std::fs::write` → `write_atomic`). They travel with a savestate, so an interruption left a
complete state beside half a PNG. Neither is on a frame path.

#### Tests

| Criterion | Test |
|---|---|
| REQ-E3D-644: the budget refuses one pattern, the other is drawn, the next frame is produced | `zax-render/tests/amd051_robustness_tests.rs` `req_e3d_644_a_texture_that_cannot_be_allocated_leaves_its_tiles_out_and_draws_the_rest` |
| one line per pattern, nothing when all is well, nothing on the next frames | `req_e3d_644_the_diagnostic_is_one_line_per_pattern_and_nothing_when_all_is_well` |
| a budget that fits changes the picture in no pixel | `req_e3d_644_a_budget_that_fits_everything_changes_nothing` |
| REQ-E3D-646 through the shared line budget, once per object | `req_e3d_646_a_late_object_is_reported_once_through_the_shared_line_budget` |
| REQ-E3D-646: 10 frames of delay, drawn on the next frame, no diagnostic | `zax-engine3d/.../geometry_lifetime_tests.rs` `req_e3d_646_an_object_delayed_ten_frames_is_drawn_on_the_next_frame_and_not_reported` |
| REQ-E3D-646: 700 frames of waiting give exactly one line, at frame 600 ± 1 | `req_e3d_646_an_object_that_waits_six_hundred_frames_is_reported_exactly_once` |
| the watch remembers nothing on a healthy frame | `req_e3d_646_a_healthy_frame_remembers_no_waiting_object` |
| REQ-E3D-524: the largest index is past 16 bits | `marching_vector_tests.rs` `req_e3d_524_the_mesher_accepts_more_than_sixty_five_thousand_vertices` (extended) |
| REQ-PERS-088: a replacement never shortens, makes its folder, leaves no temporary; the settings and the slot index | `zax-app/tests/amd051_atomic_write_tests.rs` (3) |

`cargo test --release --workspace --features zax-app/bundled`: **2255 passed, 0 failed** (2245 at
the close of RU-4a: ten new tests). fmt and workspace clippy clean.

One pre-existing clippy limit was hit and is worth recording: `render::Options::set` passed the
100-line ceiling with the two new flags, so it is split in two — `set` for what is played,
`set_scene` for how it is drawn.

### Step 3 — AMD-051 REQ-LOOP-073: the loading that no longer advances

**One rule covers both branches of the requirement**, which is what makes 51-N1 impossible to
reintroduce. `LoadJob` keeps `still_seconds` and the progress the last tick saw; `load_is_still`
resets the clock when **either** the stage changed **or** the loading progress of REQ-E3D-600
moved, and times out at 10 s otherwise. So:

- a step of REQ-LOOP-021 or REQ-LOOP-044 that exposes no measure of progress has 10 s to finish
  (neither its stage nor the progress moves while it runs) — the second branch;
- the wait for the meshes, which does expose one, is only interrupted when that measure stops
  moving, **never for being long**. That is the clause 51-N1 of `R2-review-AMD-050-053-v1.1`
  says a literal reading of v1.1 would have broken: REQ-ACC-031 excludes the initial mesh build
  from every budget and REQ-E3D-600 builds every object of every pattern of a profile, so a
  conformant implementation must load a 1 854-pattern profile on a slow machine.

The time is the `real_seconds` the loop already hands `EmulationHost::tick`, so the measure is
real time (REQ-UI-080's own clock) and a test controls it exactly.

| Piece | What landed |
|---|---|
| `host/jobs.rs` | `LOADING_TIMEOUT_SECONDS = 10.0`, `LOADING_TIMED_OUT_MESSAGE`, `advance_load(real_seconds)`, `load_is_still` |
| `host/mod.rs` | `advance_load` gets the frame's real time; `load_progress_stall` |
| `host/jobs.rs` `meshes_ready` | reads the stall when it is set — one `Option` test, on a path a load takes once per displayed frame |
| `ui/keys.rs`, `strings/english.rs`, `strings/french.rs`, `ui/notifications.rs` | `notify.loading_timed_out`: `Loading timed out.` / « Délai de chargement dépassé. » (AMD-035, AMD-036), in `FIXED_KEYS` so it is translated where it is shown |

**The seam the criterion asks for.** AMD-051's criterion is "le harnais bloque l'étape 6 de
REQ-LOOP-021". In the Rust the `zax-harness` CLI does **not** drive the application's load jobs
at all (it runs the core, the engine and the renderer directly), so a `--stall-load` flag would
have no host to stall. The seam is instead `EmulationHost::load_progress_stall`, driven by the
`LoopHarness` of `zax-app/tests`, which already owns a controllable clock — the same shape as the
`GeometryBuilder::defer_transfer` the codebase already publishes for REQ-E3D-600. The three
`zax-harness` options AMD-051 §3 point 8 names therefore land as: `--atlas-budget` **in the CLI**
(it drives the renderer), `--delay-mesh` **in the CLI** (it drives the engine), and the load stall
**in the test harness**. Reported rather than faked.

**Nothing of this is on a frame path.** `advance_load` runs only while the loading indicator is
up, and `tick` returns before any emulated frame while it is (REQ-LOOP-021 step 6).

#### Q-105 — the gravity of the notification

REQ-LOOP-073 point 1 says the notification has « gravité **normale** » and §3 point 5 of the same
amendment adds the SPEC-07 §4 row with « **erreur** ». The two are different observable
behaviours: REQ-UI-081 **drops** a normal notification while an error is on screen. R2 raised
exactly this as **51-N2** and recommended "erreur"; v1.2 fixed 51-N1 and left 51-N2.

Provisional choice: **erreur**, through `fail_job` → `show_error`, the path REQ-LOOP-023 and
REQ-LOOP-046 already take, which is also the state this requirement goes to. `// Q-105` at the
line, and `req_loop_073_the_notification_is_an_error_so_it_is_never_dropped` says which assertion
to invert if R0 answers otherwise.

**Answered by R0 the same day (2026-11-02): the provisional choice is kept** — gravity *erreur*,
4 s, through `fail_job`; the "gravité normale" of REQ-LOOP-073 point 1 is an inadvertence, and
correcting the amendment's text is R1's to do. No code moved. Q-105 is closed; the next free Q
number is **Q-106**.

#### Tests (5 in `zax-app/tests/amd051_loading_timeout_tests.rs`)

| Criterion | Test |
|---|---|
| the harness blocks step 6 → the notification at 10 s ± 1 s, and the state is Stopped with no current game | `req_loop_073_a_mesh_wait_that_stops_advancing_times_out_after_ten_seconds` |
| the gravity (Q-105) | `req_loop_073_the_notification_is_an_error_so_it_is_never_dropped` |
| the second criterion: a command of REQ-UI-205 runs within the second | `req_loop_073_a_command_runs_within_a_second_of_the_timeout` |
| 51-N1: 30 s of a load whose measure creeps forward is never interrupted, and still finishes | `req_loop_073_a_wait_whose_progress_keeps_moving_is_never_interrupted` |
| an ordinary launch is untouched, and shows no notification | `req_loop_073_an_ordinary_launch_is_untouched` |

`cargo test --release --workspace --features zax-app/bundled`: **2260 passed, 0 failed**, 83
ignored. fmt and workspace clippy clean.

#### SPEC chapter edits

- `SPEC-07` §3.3: REQ-LOOP-073 written out after REQ-LOOP-024.
- `SPEC-07` §4, the message table: the `Loading timed out.` row.
- `SPEC-04B` §3.6 / §3.7: REQ-E3D-644 after REQ-E3D-564, REQ-E3D-646 after REQ-E3D-601,
  REQ-E3D-645 as **retired**.
- `SPEC-09` §3.2: REQ-PERS-088 after REQ-PERS-024.

### Step 4 — AMD-048: the empty depth interval

**One expression**, and the comment that says what it costs the rest of the spec:

```rust
for z in zm.min(zp)..=zm.max(zp)   // Q-015 (AMD-008)
for z in zm..=zp                   // AMD-048: empty when zm > zp
```

`Construction::segment` already held the loop; a Rust inclusive range is empty when its start
exceeds its end, so the amended rule *is* the natural expression and the one the old reading had
to work around. Nothing new runs anywhere: the two `min`/`max` calls are gone.

**TV-Y4 is unchanged, both cases, and that was not a hope.** The existing vector test passed on
the first run after the change, which is the fourth independent confirmation of R2's §1.1 and
§1.2 (R1 wrote the vector, R2 recomputed it by hand, V2-8 implemented it, this lot runs it): the
region-2 points whose bounds are inverted are followed by a point of the **same `u` and the same
interval** — `(3, 2)` at D = 4, `(7, 2)` and `(7, 3)` at D = 8 — which writes the very same
voxels. Q-015 and Q-082 §4.1 both left that point out of their own tables; the test now says so
in place, so the decision cannot quietly reopen.

#### The two new vectors

| Vector | What it pins | Result |
|---|---|---|
| **TV-Y9** (odd depth, the ring ropes) | `HCYLINDER`, `D = 3`, a 2-pixel segment at image rows 3 and 4 **in two colours**: exactly two voxels on the middle slice, `(4, 1)` with the texel of row 3 and `(3, 1)` with that of row 4, `D' = 2`, 6 rectangles, local box `x ∈ [−4, −3]`, `y ∈ [−1, 1]`, `z ∈ [−0.5, 0.5]`; a whole tile of such columns gives the `8 × 2 × 1` AMD-048 publishes | **right first run** |
| **TV-Y10** (even depth, R2's point 48-1) | a full row at `D = 4`: 20 voxels, `i = 2…5` over `k ∈ [0,3]`, `i = 1` and `i = 6` over `k ∈ {1,2}`, **none at `i = 0` or `i = 7`**, 18 rectangles, box `x ∈ [−3, 3]` — 6 units wide instead of 8 | voxels and rectangles right first run; **my two box assertions were wrong** |

The two mistakes were mine and the code was right, which is worth recording because the
convention is easy to get backwards: the mesh's `y` is `j − 4`, the tile height centred like its
width, so a row at image `y = 0` (hence `j = 7`) gives `y ∈ [3, 4]`, not `[7, 8]`. TV-Y5 already
said so and I had not read it closely enough.

TV-Y9 also pins the asymmetry AMD-048 assumes (its Notes, R2's point 48-5): with the three other
`(solid, fullWarp)` pairs the **same** two-pixel segment still fills all three slices, through
REQ-E3D-448 or REQ-E3D-449, which the amendment does not touch. Q-082 §2 had observed the four
pairs agreeing; after AMD-048 they do not, and that is intended.

Two cases AMD-048's own Notes leave unlocked (R2's point 48-10) are reachable from here but have
no published vector, so nothing was invented for them: `solid + fullWarp + antiShadow`, and the
half types in wrapped solid mode. The amended rule applies to them as written (the code has one
loop for all of them) and R0's differential is what would pin them.

#### The vertex counts of the affected profiles (the brief's point 5)

Re-run of the same `zax-harness meshes` sweep as step 1. **Four of the ten profiles move, and in
both directions** — losing the outermost voxels of a section changes how the greedy mesher merges
what is left, so a thinner object is not always a smaller mesh:

| Profile | vertices before | after | faces before | after |
|---|---|---|---|---|
| Super Mario Bros 3 (U) (PRG1) | 85 208 | **85 728** | 42 604 | **42 864** |
| Contra | 61 604 | **61 716** | 30 802 | **30 858** |
| Zelda II | 92 812 | **92 512** | 46 406 | **46 256** |
| Blaster Master (U) | 249 050 | **249 222** | 118 404 | **118 490** |
| Legend Of Zelda, Castlevania, Mega Man 2, Metroid, Excitebike, Ninja Gaiden III | — | unchanged | — | unchanged |

The largest move is +0.6 % (SMB3) and −0.3 % (Zelda II). Both are **mesh generation**, cached per
tile image like any other, so nothing of this is a per-frame cost.

`zax-engine3d` 229 → **231 passed**; the whole workspace **2262 passed, 0 failed**. fmt and
workspace clippy clean.

#### SPEC chapter edits

- `SPEC-04B` REQ-E3D-450: the amended text, the five Notes AMD-048 §2 and R2's points 48-3 to
  48-7 ask for, and TV-Y9 / TV-Y10 as acceptance criteria.
- `SPEC-04B` §5.3: TV-Y9 and TV-Y10 written out, in §5.3's own `(j, k)` / `(i, k)` convention
  (R2's point 48-2) and with the local boxes and rectangle counts §5.3 gives everywhere else
  (48-6).
- `AMD-008`, the Q-015 row: the answer replaced, with the reason the old one was wrong.

### Step 5 — AMD-057: the walls come back, and the measurement decides the handheld

#### What was removed

The whole Q-067 seal, as AMD-057 §2 withdraws AMD-027 §1 and §2:

| Gone | Where |
|---|---|
| `geometry/tile_sides.rs` | deleted (the mask, `full_sides`, `is_sealed`, `opposite`) |
| the `sealed_sides` parameter | `greedy::build`, `fill_mask` |
| `full_sides`, `sealed_mesh`, the variant map | `ObjectGeometry` (now `drawn_mesh`, which **is** `mesh`) |
| `mesh_of(.., .., mask)` and `full_sides_of` | `PatternGeometryBuilder` (now `mesh_of(&self, ..)` — it no longer needs `&mut`, because there is no variant to build) |
| the per-instance neighbour map and `sealed_sides` | `DrawListBuilder` |
| `tests/engine3d/sealed_side_tests.rs` | replaced by `amd057_wall_tests.rs`, which asserts the opposite |

#### How REQ-SCN-065 is held instead

AMD-057 §2 leaves the means free except for removing the walls. The one chosen is the **depth of
the inner walls**, in the scene vertex shader:

- a wall is recognised from the sign of its **local** normal (`abs(aNormal.z) < 0.5`), before the
  normal matrix, so no neighbour is consulted and the object's rotation does not matter;
- it is pushed `WALL_DEPTH_BIAS = 0.25` scene units away from the eye. Only the **depth** moves:
  the shift is `uWallClipBias / gl_Position.w` added to `gl_Position.z`, which is exactly a
  distance δ behind the vertex (`z_ndc(d) = (f+n)/(f−n) − 2fn/((f−n)d)` gives
  `dz_clip = 2fnδ/(f−n) / w`). `x`, `y` and `w` are untouched, so no silhouette and no texture
  coordinate moves with it — which a constant offset in clip space could not promise (near plane
  0.3, far plane 5000: the two scales have nothing to do with each other);
- `uWallClipBias` is **0 for a mesh whose normals are not axis aligned**: `MARCHING` merges its
  tiles (REQ-E3D-520) so it has no joint to hide, and `|normal.z| < 0.5` would be meaningless on
  a smoothed surface. The flag rides on `GpuMesh`, read from `TileMesh::has_axis_aligned_normals`
  which is computed **once when the mesh is built** — so the draw path reads a bool off a handle
  it already has, and no field was added to the per-tile list.

#### The measurement, and the handheld decision

The brief asks for both sides before deciding. **No commercial ROM is available on this side**, so
the real-profile figures come from `zax-harness meshes` over the ten profiles AMD-057 §1 compares,
and the CPU figure from the post-LTO instruction count, which is immune to machine load.

**CPU — the draw list.** `DrawListBuilder::build`, with `add_instance` and the former
`sealed_sides` inlined:

| | instructions |
|---|---|
| after AMD-048 (step 4) | **1574** |
| walls kept | **1095** |

**−479 instructions, −30 %.** Two hash operations per tile disappear with the neighbour map (an
`entry` while the map is filled and a `get` while each tile is sealed), which a static count does
not even show. Wall clock, `bench-core … --3d` (982 tile records a frame), best of 5 on a quiet
machine: **0.0370 → 0.0328 ms/frame, −11 %** (the rest of the stage is per-tile work the change
does not touch). `Construction::segment` 159 → 159 and `Ppu2C02::run_dots` 2231 → 2231, the two
controls.

**GPU — the vertex count**, `zax-harness meshes`, synthetic frame 0 of each profile:

| Profile | vertices after AMD-048 | walls kept | |
|---|---|---|---|
| Super Mario Bros 3 (U) (PRG1) | 85 728 | 86 536 | +0.94 % |
| Legend Of Zelda | 50 312 | 51 136 | +1.64 % |
| Castlevania (U) (PRG 1) | 19 596 | 20 692 | **+5.59 %** |
| Mega Man 2 | 56 240 | 58 096 | +3.30 % |
| Metroid | 680 216 | 681 080 | +0.13 % |
| Contra | 61 716 | 63 492 | +2.88 % |
| Excitebike | 32 736 | 33 336 | +1.83 % |
| Ninja Gaiden III | 103 700 | 104 844 | +1.10 % |
| Zelda II | 92 512 | 93 744 | +1.33 % |
| Blaster Master (U) | 249 222 | 249 918 | +0.28 % |
| **total** | 1 431 978 | 1 442 874 | **+0.76 %** |

The synthetic scenes are the pathological limit of the same measure, and worth printing because
they show what the seal was really worth: a scene whose tiles tile the whole picture had **every**
lateral wall sealed, so `framing` goes 7704 → 21 576 triangles (+180 %) while the real profiles —
whose objects are small and whose tiles mostly have free sides — move under 1 %.

**And a third quantity the brief did not ask for, which settles it.** The Q-067 variants were
separate meshes in the GPU cache, up to sixteen per tile image. Over the eleven synthetic scenes
the resident mesh count falls **176 → 81**: nearly half the mesh cache was seal variants. That is
GPU memory and mesh uploads saved, on the platform where both are scarcest.

**Decision: the spec path everywhere, no handheld exception.** The brief's condition for keeping
Q-067 on handhelds — "if keeping the walls costs more in total than it saves" — is not met, in any
of the three quantities: the CPU draw list is 30 % cheaper, the mesh cache is less than half as
large, and the vertex count is 0.76 % higher. There is therefore **no setting** for this and no
advanced option; the walls are simply kept, as REQ-E3D-501 and AMD-057 §2 say. R0 has the numbers
above if the owner wants the trade-off revisited on a real device.

#### The images

Every synthetic capture at default settings, compared with the step-1 ones (`zax-harness compare
--tolerance 2`):

| Scene | pixels within 2 levels |
|---|---|
| framing, shadow, nudge, clip, backdrop, alpha, hud | **100.000 %** |
| heavy | **99.965 %** (108 of 307 200 pixels) |

So the walls came back and the picture did not move, which is the whole point of the bias. The
108 pixels of `heavy` are the one-pixel joint lines Q-067 reported, now erased: the distinct
colour count of that scene falls 23 → 16, and `framing`'s 4 → 3, because the joints no longer
contribute shades of their own.

**The render goldens moved on purpose and were updated.** `goldens/render-synthetic/render.json`
is a C# capture of the 0.2.0 era, i.e. of the sealed state; `triangles` and `residentMeshes`
changed in every scene and `distinctColors` in two. The **PNGs were not touched** and still match
(the comparison asks for 99.5 %, the worst scene is 99.965 %), which is the check that says the
geometry change is invisible.

#### Tests

| Criterion | Test |
|---|---|
| AMD-057 §2, first criterion on `--dump-meshes`: two full tiles of one object that touch each give 24 vertices | `zax-harness/tests/meshes_tests.rs` `synthetic_frames_export_the_counts_of_the_engine_tests` (was `[10, 10, 24]` triangles, now `[12, 12, 24]` and `[24, 24, 48]` vertices) |
| the same on the draw list, and a lone tile no longer differs from one in a surface | `zax-engine3d/.../amd057_wall_tests.rs` `amd_057_two_full_tiles_of_one_object_that_touch_each_keep_the_whole_box` |
| the mesher has no way left to leave a wall out | `req_e3d_501_a_full_tile_is_a_closed_box_whatever_its_neighbours` |
| REQ-E3D-540: the four tiles share one mesh, no variant is built | `req_e3d_540_a_repeated_tile_draws_the_very_mesh_its_object_owns` |
| AMD-057 §2, second criterion: REQ-SCN-065 passes with the walls kept, at three shadow-map sizes | `zax-render/tests/amd057_wall_tests.rs` `amd_057_the_surface_of_req_scn_065_passes_with_every_wall_kept` |
| the same surface split over two objects | `amd_057_a_surface_split_over_two_objects_passes_too` |
| the bias is not geometry: same draws, same triangles, same silhouette to the pixel | `amd_057_the_wall_depth_moves_no_vertex_and_no_silhouette` |
| a smoothed surface takes no bias: every pixel identical with and without it | `amd_057_a_smoothed_surface_takes_no_wall_bias` |
| **the measure that bites** | `req_scn_040_the_wall_depth_is_what_keeps_a_coincident_wall_out_of_the_blend` |
| the five existing REQ-SCN-065 / REQ-SCN-040 measures now assert the whole boxes first | `zax-render/tests/seam_tests.rs` (`assert_whole_boxes` added to four of them) |

**Which test bites, and the finding behind it.** The synthetic criterion of REQ-SCN-065 passes
with the walls kept **and the bias at 0**: on a surface of one object the coincident walls lose
the fill rule anyway. What does not pass is **REQ-SCN-040** — in the equal-depth colour pass of a
translucent object, a wall coincident with its neighbour's face is kept by `depth_func(EQUAL)` and
that pixel is blended twice. So the discriminating test is the translucent one, and that is the
same finding the .NET lot V2-29 recorded ("avec `WallDepthBias = 0` les deux tests de jointure
passent encore mais `REQ_SCN_040_…_shows_no_internal_face` échoue"). On real scenes, which hold
hundreds of objects at different depths, the bias additionally erases the one-pixel dark lines
Q-067 reported — the 108 pixels of `heavy` above are the synthetic trace of that.

One test of mine was written with the wrong measure and is **not** in the list: a deep-surface
REQ-SCN-065 check whose window (inset by a twelfth) takes in the slab's top face at a negative
pitch, so it reported 478 "offenders" that are a legitimately different surface. `seam_tests.rs`
already covers that case properly, with `front_face_line` and `sawtooth_jumps` rather than a
histogram, so the badly measured test was dropped rather than loosened.

`cargo test --release --workspace --features zax-app/bundled`: **2263 passed, 0 failed**. fmt and
workspace clippy clean.
