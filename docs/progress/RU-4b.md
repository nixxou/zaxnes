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
| 6 | AMD-057 sprite depth: `0.001 × (i mod 64) ∓ 0.1` | done |
| 7 | AMD-058: the joints under MSAA, a shader variant for `anti_alias` ≥ 1 only | done |
| 8 | AMD-049: REQ-SCN-075 as a shader variant, the setting, the vectors | done |
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

### Step 6 — AMD-057: the sprite depth by index

**The brief's point 6 held exactly**: "the offset formula replaces a constant; computed where the
constant was, no new per-frame pass." The index was already in hand — `DrawTile.tile` carries
`FrameTile::ord()`, the sprite slot of REQ-CORE-043, which step 1 had already checked — so the
`match` on `(sprite, behind_background)` in `frame_plan::collect_tile` became a call to
`geometry::sprite_nudge(tile.tile.ord(), tile.behind_background)` and nothing else moved:

```rust
0.001 * (index.max(0) % 64) as f32  ±  0.1
```

One `imul`, one `and`, one `cvtsi2ss`, one `mulss` on a path that already did a two-armed branch
and a float select. **Nothing was added to any per-tile record**, no setting is consulted, and the
`if nudge != 0.0` guard that skips the matrix multiply for a background tile is untouched — a
background tile still costs one float compare.

#### Why `index.max(0)`, and not a `u8`

`FrameTile::ord()` is an `i32` on this branch (the draw list carries the core's own ordinal). A
negative value is not reachable for a sprite — REQ-CORE-043 numbers them 0..127 — but `%` in Rust
keeps the sign of the dividend, so an unchecked `index % 64` on a hypothetical −1 would give −1 and
a nudge of −0.101, i.e. a sprite *in front of* index 0 instead of at it. `max(0)` makes the
degenerate case read as index 0 rather than invent a new depth; it compiles to one `cmovns`.

#### The sign of the index term on the far side

AMD-057 §2 writes **`+`** for both priorities: `0,001 × (i mod 64) − 0,1` in front, and
`0,001 × (i mod 64) + 0,1` behind. So a "behind the background" sprite of a *higher* index is
pushed *further behind*, which keeps the priority order of REQ-CORE-043 the same on both sides of
the background. That is what the amendment says and it is also the sensible reading — the
alternative (mirroring the term) would put the lowest-priority sprite nearest the eye among the
"behind" ones — so no Q ticket was opened. `req_scn_037_…` asserts both sides explicitly.

#### The frailty worth recording

The measured span of the index term is **0.063 scene unit** over the 64 indices, i.e. 0.001 unit
between two neighbours. At the default camera setpoint (about 148 units out, near 0.3 / far 5000
of REQ-SCN-004) that gap is **1.6 least significant bits of a 24-bit depth buffer** — it resolves,
but barely, and a 16-bit depth buffer would not resolve it at all. This is a property of the
formula AMD-057 measured on the original, not of this port; it is recorded here, and the GPU half
of the overlap test brings the camera in to 32 units so that it measures the *rule* rather than the
depth buffer's luck. R0 should expect the overlap criterion to be resolution-dependent on a
handheld whose depth buffer is 16 bits (none of the current targets is: `gl/context.rs` asks for
24).

#### The images

Measured rather than assumed: the step-5 look was reproduced by zeroing the index term in
`sprite_nudge` (a one-line local change, reverted), both sets captured with
`zax-harness render --anti-alias 0`, and compared.

| Scene | step-5 look vs step 6, tolerance 2 |
|---|---|
| framing, sky, shadow, no-shadow, layer-shadow-off, **nudge**, alpha, hud, clip, backdrop | **100.000 %** |
| **heavy** | **99.734 %** (0.266 % of 307 200 pixels, about 820; identical at tolerance 0 too, so these pixels change colour outright rather than shade) |

**One scene moves, and it is the right one.** `nudge` — the scene named after this very offset —
does *not* move: its sprites take the index term, but they do not overlap each other, and the term
is far too small to reorder a sprite against the background. `heavy` is the scene of REQ-SCN-064,
**64 sprites of indices 0 to 63** laid out at `x = 3i mod 248`, `y = 8 + (5i mod 224)` so that many
of them do overlap — and where they do, the lower index is now in front. Those ~820 pixels are
exactly the behaviour AMD-057 §2 asks for, on exactly the scene that can show it.

**No setting reproduces the old look**, because AMD-057 amends the rule rather than adding a mode;
the method above (zero the index term) is what R0 can use to separate this change from a
regression. The render goldens were **not** touched: `heavy` is 99.684 % against the C# goldens,
inside the 99.5 % the golden test asks for, and no statistic of `render.json` depends on the
offset — `synthetic_captures_match_the_csharp_goldens` passes unchanged.

#### Tests (3 in `zax-render/tests/amd057_sprite_depth_tests.rs`)

| Criterion | Test |
|---|---|
| the formula, the amended TV-W2's three cases, the eight Contra tiles of AMD-057 §1, and both priorities | `req_scn_037_the_offset_is_a_thousandth_of_the_index_either_side_of_the_tenth` |
| the nudge reaches the drawn geometry (TV-W2 restated), and a background tile is still not nudged | `tv_w2_the_front_face_of_a_sprite_sits_at_the_offset_of_its_index` |
| AMD-057 §2 second criterion: indices 3 and 10 overlap, 3 in front, a fixed 0.007 apart, and the overlap is 3's colour on the GPU | `amd_057_of_two_overlapping_sprites_the_lower_index_is_in_front` |

`SceneFixture` gained `sprite_indexed` / `push_sprite_at` so a criterion can choose the index;
`push_sprite` keeps its running order, and `sprite_order` now advances past an explicit index so
the two can be mixed in one fixture.

The engine-side `tv_w2_a_sprite_tile_gets_the_same_transform_and_the_nudge_flags` is **unchanged**
and that is the check that the ownership of DEC-R2-T4 survived the amendment: the engine still
delivers the same transform for a sprite tile as for a background tile, index or no index.

`cargo test --release -p zax-render`: all suites pass, the 3 new ones included. fmt and workspace
clippy clean.

#### SPEC chapter edits

Step 5 implemented the wall half of AMD-057 but recorded no chapter edit, so this step carries
both halves:

- `SPEC-06` REQ-SCN-037: the amended text, the amended acceptance criterion, the second criterion
  of AMD-057 §2 and the note on the consequence.
- `SPEC-06` §4.3, the constants table: the « Décalage des sprites » row becomes
  `0,001 × (i mod 64) ∓ 0,1`.
- `SPEC-04B` REQ-E3D-405: the formula in the renvoi, and the sprite index added to what the engine
  delivers for the renderer to choose `n`.
- `SPEC-04B` TV-W2: indices 0, 5 and 69 written out.
- `SPEC-04B` REQ-E3D-501: the note that the **drawn** mesh is the whole mesh, that AMD-057 §2
  withdraws AMD-027 §1-2, and that REQ-SCN-065 stays required by other means (the wall half).
- `SPEC-v1.2-INDEX`: the REQ-SCN-037 row, and the REQ-E3D-450 row that step 4 left pointing at the
  superseded AMD-008 Q-015 answer.

### Step 7 — AMD-058: the joints under anti-aliasing

The cause AMD-058 §1 isolated is the documented one: with a multisampled buffer, a fragment that
covers *some* samples but whose **pixel centre falls outside the primitive** gets an
**extrapolated** `vUv`, so the `floor()` of `scene.frag` lands one texel past the face and the
atlas lookup of REQ-E3D-563 returns a texel the face does not hold. With one sample per pixel no
such fragment exists, which is exactly why §1 found the picture clean at `video.anti_alias` = 0 and
cracked at 1, same binary, same scene.

The fix is the standard remedy and the one the amendment's "traitement des échantillons" names
first: `centroid` interpolation of `vUv`. The centroid is always inside the covered samples, so the
coordinate stays within the face.

| Piece | What landed |
|---|---|
| `shaders/scene.vert`, `scene.frag` | the `vUv` declaration under `#ifdef ZAX_CENTROID`; `centroid out` / `centroid in` with it, the plain declaration without. The qualifier must agree between the two stages, so both carry the guard |
| `gl/shaders.rs` | `CENTROID_DEFINE`, a **fourth** variant axis on `shaders::scene(solid, srgb_palette, bend, centroid)`, and the sixteen names |
| `renderer.rs` | `ScenePrograms` rebuilt around a slot table; `select_centroid_variant`, called once a frame **before** `select_bend_variant` |
| `statistics.rs` | `centroid_interpolation`, so the scope of the variant is observable from outside |

#### `ScenePrograms`: four pairs, and a lookup that got cheaper

`current()` runs **once per draw run** (`draw_run` reads it for the per-item uniforms), so adding a
fourth axis had to not cost anything there. The old shape was a `match` on `(bend, &mut bent)`;
the new one is a slot table:

- `built: Vec<[ShaderProgram; 2]>`, `built[0]` the flat pixel-centre pair the renderer is created
  with;
- `slots: [usize; 4]`, one entry per `(bend, centroid)` key, **0** until that variant is built, so
  a key with no program falls back to the flat pair by construction and not by a branch;
- `slot`, refreshed by `set_bend` / `set_centroid` / `install` — the only places the two switches
  move, a few times a frame at most.

`current()` is therefore `&mut self.built[self.slot][usize::from(self.solid)]`: **two indexings and
no branch**, where it used to be a two-arm match plus an `Option` test. The HUD's flat-variant
toggle (REQ-SCN-068) and the curvature selection now go through the two setters.

**Nothing is compiled at the handheld default.** Both lazy axes stay unbuilt: curvature 0
(REQ-SCN-066) and `video.anti_alias` = 0 mean the renderer holds exactly the one pair it always
held. `select_centroid_variant` at that setting is one integer comparison and a store.

**The trigger is the setting, not the granted sample count.** A context that refuses a
multisampled buffer falls back to one sample, whose single sample *is* the pixel centre, so the
centroid variant draws the same pixels there — the fallback is correct without a second decision.
Chosen before the curvature so the pairs the curvature then builds are the ones the frame and its
flat HUD actually use (two pairs, not three).

#### Point 3 of the brief: what "byte for byte" could and could not be delivered

Measured, not asserted. The 40 assembled sources (8 scene variants × 2 stages × 2 contexts, plus
the 2 shadow ones) were dumped before the change and after it, and compared:

- **the code the GLSL compiler is given is identical, line for line, for all 40.** Checked by
  running the `ZAX_CENTROID`-undefined branch of the preprocessor over both dumps and diffing.
- **the source text is not**, and cannot be: the `#ifdef ZAX_CENTROID` lines live in
  `scene.vert` / `scene.frag`, and a preprocessor directive is text. This is the same situation
  AMD-043 left for the curvature — `bend_tests::without_bend` says so in as many words ("the
  requirement is about what the compiler is given, not about what the file says") — and the brief
  points at that precedent for how to assert it.

**The one consequence, and it is a one-time one.** `gl/program_cache.rs` keys a cached binary on a
hash of the driver, the program name and both sources, so the eight pixel-centre programs get new
cache keys and are compiled once more on the first start after this update. The **names** are
unchanged (`amd_058_the_names_of_the_pixel_centre_variants_are_unchanged`), so nothing else about
the cache moves, and no frame is affected. Reported rather than hidden: literal byte identity of
the assembled text was not attainable without splitting the two shaders into three chunks each,
which costs more in readability than one rebuild costs a player.

#### What bites, and what does not — the honest part of this step

**Both automatic criteria of AMD-058 §2 pass with *and* without the centroid variant** on this
side, and so does the "differs only at the edges" measure. Verified by forcing
`select_centroid_variant` to refuse, rebuilding and re-running. Two reasons, both about the
fixtures available here rather than about the fix:

1. **A plain surface cannot show it.** REQ-SCN-065's surface is 4 × 4 copies of a *uniform* tile
   image, so a texel fetched one step outside the face is the **same colour** as the right one.
   The two scenes AMD-058 §1 measured — the Mega Man 2 tower façade, the Excitebike blue wall —
   are textured, which is what makes the wrong texel a *light* line.
2. **Inside the silhouette the extrapolation stays within the face** at the magnifications a
   synthetic scene reaches here: half a pixel of extrapolation is well under one texel when a
   texel spans several screen pixels.

What **is** proved automatically:

- the variant is selected exactly when `video.anti_alias` is not 0, and never otherwise
  (`centroid_interpolation`);
- it is **in effect on the GPU**, not merely selected: on a striped 4 × 4 surface at four samples
  the capture holds **12** distinct colours with the variant and **8** without it;
- the pixel-centre programs carry nothing of it, in all 16 source combinations.

The discriminating check stays AMD-058's own review criterion, which needs the ROM scenes at
2560 × 1440: **R0 runs it.** Everything it needs is in place (`zax-harness render --rom ...
--press 200:start,300:start --run 599 --anti-alias 1` against `--anti-alias 0`).

**One measure I wrote and then corrected rather than loosened.** The "differs only at the edges"
test first ran on a *textured* surface, and failed — 109 interior pixels on a magnified 4 × 4
striped surface, 2100 on a minified 28 × 26 one. Both are right and the test was wrong: a textured
tile joint carries a real colour discontinuity (one tile image's right edge against the next one's
left edge), so multisampling blends it, and AMD-058's own exclusion *requires* it to keep doing
that. The criterion means what it says only of a plain surface, which is how the test now reads.

#### Tests (10 in `zax-render/tests/amd058_centroid_tests.rs`)

| Criterion | Test |
|---|---|
| AMD-058 §2 first criterion: the 4 × 4 lit, shadowed surface of whole boxes at `anti_alias` 0, 1, 2 | `req_scn_065_the_surface_shows_no_seam_at_any_anti_alias_setting` |
| second criterion: the same surface over two objects, three values | `req_scn_065_a_surface_of_two_objects_shows_no_seam_at_any_anti_alias_setting` |
| the exclusion: the setting still softens the object edges | `amd_058_the_setting_still_softens_the_edges_of_the_objects` |
| third criterion, automated for a plain surface | `amd_058_an_anti_aliased_capture_differs_from_a_plain_one_only_at_the_edges` |
| the statement the light line breaks, as a regression guard | `amd_058_no_interior_pixel_takes_a_colour_the_surface_cannot_produce` |
| **the scope** (point 3): the variant draws exactly when anti-aliasing is on, and back | `amd_058_the_centroid_variant_draws_exactly_when_anti_aliasing_is_on` |
| **the variant is in effect on the GPU**, 12 colours against 8 | `amd_058_the_variant_changes_what_the_gpu_draws` |
| point 3: the pixel-centre variant carries nothing of the centroid, 16 combinations × 2 contexts | `amd_058_the_pixel_centre_variant_carries_nothing_of_the_centroid` |
| the two variants differ **only** by the qualifier on `vUv` | `amd_058_the_two_variants_differ_only_by_the_qualifier_on_v_uv` |
| the eight pixel-centre names are unchanged (the cache key) | `amd_058_the_names_of_the_pixel_centre_variants_are_unchanged` |

`bend_tests` follows the new signature: `scene_source` pins the pixel-centre variant (the one the
default settings use), and `req_scn_067_each_variant_has_its_own_name` now walks all sixteen.

#### The images

All eleven synthetic captures at default settings (`--anti-alias 0`) are **identical to step 6's at
tolerance 0**, which is the point of the whole design: with anti-aliasing off nothing of AMD-058
runs. The render goldens did not move.

`cargo test --release -p zax-render`: all suites pass (10 new). `-p zax-harness`: the render and
core goldens pass. fmt and workspace clippy clean.

*(One build hiccup worth recording for whoever resumes: a `LNK1285 fichier PDB endommagé` on
`target/release/deps/amd057_sprite_depth_tests-*.pdb`, the machine's instability showing through.
Deleting the `.pdb` and re-running was enough.)*

#### SPEC chapter edits

- `SPEC-06`: **REQ-SCN-065 written out** after REQ-SCN-040 — it had been created by AMD-012 and
  only ever indexed, never carried into the chapter — with AMD-058's completion (the three values
  of `video.anti_alias`, the two-object clause), its two exclusions, the Q-032 residue, the three
  acceptance criteria and the notes of AMD-058 §1 and §3.
- `SPEC-v1.2-INDEX`: the REQ-SCN-065 row.
- `impl/docs/spec/changes/`: `AMD-058-jointures-avec-anticrenelage.md` copied.

### Step 8 — AMD-049: the backdrop lighting

REQ-SCN-075 replaces the diffuse term of REQ-SCN-048 on the backdrop alone. The brief's point 2
asks for a **shader variant** chosen when the setting or the style changes, never a per-fragment
branch on a uniform — and the .NET lot's `uColorMode == 2` test per fragment is exactly what it
forbids.

| Piece | What landed |
|---|---|
| `color.rs` | `backdrop_linear`, the platform-free CPU twin of the model, plus `BACKDROP_ROUGHNESS` and `BACKDROP_REFLECTANCE` |
| `shaders/backdrop.frag` (new) | the model on the GPU, a fragment body of its own |
| `gl/shaders.rs` | `shaders::backdrop(bend)`, a **separate program**, not a fifth axis of `scene` |
| `renderer.rs` | a third axis of the slot table; `select_backdrop_variant`; `bind_frame_uniforms` |
| `frame_plan.rs` | `SceneConstants::eye`, read by the backdrop program alone |
| `scene.rs` | `RenderSettings::backdrop_lighting`, true (the spec default) |
| `zax-harness render` | `--no-backdrop-lighting`, the flag that reproduces the look of before |

#### Why a separate program and not a fifth variant axis

Three reasons, and they also make it the cheapest shape:

1. the backdrop is **one opaque run a frame**, so the choice is made once per run and never per
   fragment;
2. it carries a flat colour, so it needs **neither the palette axis nor the atlas lookups nor the
   fragment rejections** — `backdrop.frag` holds no `uColorMode`, no `uIndexTexture`, no
   `uPaletteTable`, which a test asserts;
3. it does not read the texel coordinate at all, so the interpolation axis of AMD-058 does not
   apply to it either.

Only the curvature axis is left (REQ-SCN-068: the backdrop bends with the scene), so the
amendment costs **at most two programs**, built lazily. A fifth axis on `scene` would have cost
32 names and put the model's identifiers into every tile shader's source.

#### Where the per-frame cost went, and what it is

`ScenePrograms` gained a third key bit and, with it, a different pair lookup: `built` is now a
flat `Vec<ShaderProgram>` and `slots[key]` holds **two** indices, the plain and the solid one.
A backdrop key points **both** at one program — the backdrop never clips and never reads a colour
texture, so only the solid form is ever asked for, and compiling the same source twice to fill a
pair would be a program for nothing. `current()` is still two indexings and no branch.

On the per-frame path, exactly two things are new:

- `select_backdrop_variant`, once a frame: one `&&` of two booleans, and a return. Nothing else
  unless both the setting and `Backdrop.Enable` are on.
- in the opaque loop, `set_backdrop(item.color_mode == 2)` per **run**. The `color_mode` is the
  field the next line already reads for `use_solid`, and `set_backdrop` returns whether the
  program actually changed — which it does not when the variant was not built. So with the
  setting off the whole frame costs one comparison per run against a `false`, and no GL call.

**The setting is honoured per frame, not per program.** A first version read "is the backdrop
program built?" as "is the setting on", and a test caught it: a program built for one frame
outlives the setting that asked for it, so turning the setting off again kept drawing the model.
`backdrop_wanted` is now set every frame and `set_backdrop` ands with it — turning the setting off
goes back to REQ-SCN-048's diffuse term without throwing the program away.

#### The model, and every published figure

The CPU twin was written first and checked against the amendment before any GPU code existed.
**Every figure AMD-049 publishes reproduces, first run**, including the two R2 recomputed:

| Vector | Published | Found |
|---|---|---|
| head on, albedo c | `c · 0,96 · 1,1812 + 0,01041` | exact, for c = 0, 0.2, 0.5, 1 |
| head on, c = 0.2, out of shadow | 0,2372 | ✓ |
| head on, c = 0.2, in shadow | 0,0753 | ✓ |
| albedo 0, out of shadow | 0,01041 → sRGB 26 | ✓ |
| albedo 0, in shadow | 0,00208 → sRGB 7 | ✓ |
| the black panel of REQ-SCN-048, default light (R2 §1.4) | 0,007534 → sRGB ≈ 21 / ≈ 5 | ✓ |
| **Punch-Out!! front face** (`[ROM]`) | sRGB (26 ; 100 ; 209) ±3 | ✓ **at 0°, 15°, 30° and 45°** |

The `[ROM]` one is the interesting one: R2's point 49-2 says the published figure cannot be
recovered from the amendment's wording alone, because the eye position is missing. It can be
recovered from the *pitch sweep* the criterion itself names — the value holds across 0° to 45°,
which is why the criterion is stated that way, and the test pins the whole range rather than one
angle. R0 still runs the capture; this says the model is the right one.

Two things the model does that are easy to get wrong and are pinned in place:

- the **specular term is not tinted by the albedo** (it is outside the `c · 0,96 · (…)`
  parenthesis), so a zero-albedo backdrop takes the light's own colour — which is the dominant
  visible change, not the 4 % darkening (R2's point 49-7);
- `H = N` when `L + V = 0` (R2's point 49-8), the one real hole in the formula; without it the
  model is `NaN` for an eye exactly opposite the light.

#### The image change, and how to get the old look back

Measured over the eleven synthetic scenes, `--anti-alias 0`, default settings:

| Scene | step 7 vs step 8, tolerance 2 |
|---|---|
| framing, sky, shadow, no-shadow, layer-shadow-off, nudge, alpha, hud, clip, heavy | **100.000 %** |
| **backdrop** | **58.931 %** — the black box becomes sRGB 26, over 41 % of the picture |

One scene, and it is the only synthetic scene with `Backdrop.Enable` set. Its distinct colour
count goes 6 → 10.

**`--no-backdrop-lighting` reproduces the old look exactly**: all eleven scenes compare
**100.000 % at tolerance 0** against the step-7 captures.

**The render goldens were not touched**, and `backdrop.png` is still the C# reference it has
always been. `render_tests.rs` now passes `--no-backdrop-lighting` beside the `--anti-alias 0` it
already passed, with the same reasoning written out: the comparison asks for the state the goldens
were made in. Overwriting a C# reference capture with our own output would have cost the only
independent reference this scene has; the new model is covered by `amd049_backdrop_tests.rs`
instead.

#### What is left for step 9

This step wired the **renderer** setting (`RenderSettings::backdrop_lighting`, the spec default on
desktop) and the harness flag, because the goldens could not be kept verifiable without them.
Still to do in step 9, and listed here in case the session is cut:

- `zax-settings`: the persisted `render.backdrop_lighting`, its catalogue descriptor and its
  libretro option;
- `publish_settings_to_scene` in `zax-app/src/application.rs`, so the setting reaches the scene;
- `ui/handheld/defaults.rs`: **off**, with the advanced option on the handheld page;
- the desktop options window and the `EXCLUDED` list;
- `ui/keys.rs` and the English and French strings.

#### Tests (13 in `zax-render/tests/amd049_backdrop_tests.rs`, 1 in `zax-harness`)

| Criterion | Test |
|---|---|
| AMD-049 first automatic criterion, the whole chain of products | `req_scn_075_the_head_on_vector_of_amd_049` |
| second automatic criterion: a zero albedo is no longer black, and is black under the diffuse model | `req_scn_075_a_zero_albedo_is_no_longer_black` |
| the amended criterion of REQ-SCN-048: sRGB ≈ 21 / ≈ 5, and the shadow becomes observable | `req_scn_048_the_black_panel_of_the_amended_criterion` |
| R2's point 49-8: `H = N` when `L + V = 0` | `req_scn_075_the_half_vector_falls_back_to_the_normal` |
| R2's point 49-7: the specular term carries the light's colour, not the albedo | `req_scn_075_the_specular_term_carries_the_light_colour_not_the_albedo` |
| the `[ROM]` criterion over the pitch sweep it names | `req_scn_075_the_punch_out_review_vector_over_the_pitch_sweep` |
| **the GPU draws what the twin says**, four albedos | `req_scn_075_the_gpu_draws_what_the_cpu_twin_says` |
| on the GPU: sRGB 26 with the setting, black without it | `req_scn_075_a_black_backdrop_is_drawn_srgb_twenty_six_and_not_black` |
| the scope: the setting changes the backdrop and no other pixel | `req_scn_075_the_setting_changes_the_backdrop_and_nothing_else` |
| **the brief's point 2**: no tile fragment carries the model, and the backdrop fragment carries no atlas lookup | `req_scn_075_no_tile_fragment_carries_the_model` |
| the program names are all distinct (the cache key) | `req_scn_075_the_backdrop_variants_have_names_of_their_own` |
| REQ-SCN-068: the model bends with the scene | `req_scn_075_the_backdrop_model_bends_with_the_scene` |
| what the head-on fixture is worth, stated rather than assumed | `the_head_on_fixture_really_looks_down_the_axis` |
| the harness flag, and that it reaches the picture | `zax-harness` `backdrop_lighting_is_on_unless_the_flag_turns_it_off` |

One fixture assertion was wrong and is now right rather than loosened: the head-on camera is
**not** on the axis through `C = (128, 120, 0)` but through the centre of the *visible rectangle*,
which the default clip margins put at (124, 124) — 1.2° off at 190 units. That is why the GPU
comparison is to ±2 sRGB levels and not exact, and the test now says so with the angle.

`cargo test --release -p zax-render` and `-p zax-harness`: all suites pass. fmt and workspace
clippy clean. Two pre-existing clippy ceilings were hit by the additions and are worth recording:
`ScenePrograms` passed the four-bool limit (the same `allow` `Passes` already carries, each bool
being an independent variant axis), and `render_scene` passed the 100-line ceiling, so the
frame-uniform binding moved out into `bind_frame_uniforms`.

#### SPEC chapter edits

- `SPEC-06` REQ-SCN-048: lit according to REQ-SCN-075 (AMD-049 §3 point 1), and the acceptance
  criterion's "panneau noir" becomes "presque noir (sRGB ≈ 21 hors ombre, ≈ 5 dans l'ombre)"
  (§3 point 2).
- `SPEC-06`: **REQ-SCN-075 written out** after REQ-SCN-048, with the model, the priority clause
  T-1, the three acceptance criteria and the notes of R2's points 49-7 and 49-8.
- `SPEC-v1.2-INDEX` (§3 point 3): REQ-SCN-075 added, and the REQ-SCN-065 row of the same table
  corrected — step 7 wrote that requirement into SPEC-06, so it is no longer one of the
  requirements that "appear in no SPEC file".
- `impl/docs/spec/changes/`: `AMD-049-eclairage-backdrop.md` and `R2-review-AMD-049.md` copied.
