# RU-6 — the MSVO ambient occlusion mode (REQ-SCN-083), ported from `dotnetupdated`

Branch `reworkoptions`, on top of RO-5 (`f5a5be9`). RU-5 rendered `spl.ao` = 1 as Depth, the
declared fallback **(D)** of REQ-SCN-083, on the stated ground that
`third_party/README.md` carried no MiniEngine entry. **It does** — the file is
`<cleanroom>/third_party/README.md`, not under `impl/` — and the MIT-licensed
`MiniEngine-SSAO` copy is beside it. This lot ports the multi-scale occlusion into the Rust
renderer and switches the mode on.

The .NET lots V2-20 to V2-25 on `dotnetupdated` are the reading reference, never a line-by-line
translation: the Rust architecture of RU-5 (`gl/screen_effects.rs`, the lazy lifecycle, the shared
`screen_depth.glsl`, the ceiling applied inside the occlusion pass) comes first.

Build directory: `rust/target-ro`. `CARGO_TARGET_DIR=<impl>/rust/target-ro`,
`--features zax-app/bundled` where the app is built.

## Checklist

| # | Step | Status |
|---|---|---|
| 1 | Map: the .NET lots read, the requirement map, the test plan, the "before" figures | done — `1e0d8cf` |
| 2 | The MSVO pass: the module, the resources, the lifecycle, the whole chain, the mode switched on, and the invariants of §2.4 in it | done — `d9bd07a` |
| 3 | The handheld and the GLES side: the defaults, the console page, the capability finding, Q-104 | done — `b3b3687` |
| 4 | The comparisons with the figures of the .NET lots and of Q-100 | done — this commit |
| 5 | The cost of the pass at 1080p, per quality, beside Depth's | todo |
| 6 | Closing: notes, `PERFORMANCE-HISTORY.md`, the Q-100 appendix, fmt, clippy, full tests, release build | todo |

The hash of a step is written in the commit of the step that follows (no amend, protocol of this
lot).

**The checklist is six steps, not the brief's six in the brief's order**, and the reason is written
out under "The checklist was rearranged after step 1" below.

## Notes

### State of the working copy at the start

`git status` showed exactly what RU-5's and RO-5's notes predict and nothing else:

- `rust/crates/zax-render/tests/dump_sources_tmp.rs` and `probe_tmp.rs` — the two scratch probes
  RU-5 and RO-2 deliberately left uncommitted; the brief of this lot repeats that they are never
  staged;
- `tests/` — the **.NET** test tree's `bin/`, `obj/` and `TestResults/` build output, left behind
  by an earlier checkout of `dotnetupdated` in this working copy, untracked since RU-4a.

No tracked file was modified, so there was nothing to review, keep or discard.

### What the .NET lots settled, read commit by commit

| Lot | Commit | What it established |
|---|---|---|
| V2-20 | `a2382f2` | the port itself: the pyramid of linear depths, the per-level sphere-penetration estimator of `AoRenderCS.hlsli`, the bilateral upsample chain, `ao.thickness` as the rejection fall-off |
| V2-22 | — | Q-095: the ceiling of REQ-SCN-083 is read as a **scale**, not a mere bound; the port's equivalent constant is `zaxMsvoShare` |
| V2-23 | `b88df22` | Q-097: the tolerant reading of the contact invariant; `zaxMsvoShare` = 0,12 under the old ceiling of 0,3 |
| V2-24 | `e8e0dc3` | the ceiling v1.4 (`min(0,95 ; 0,3 × ao.strength)`) recalibrated the gain to **0,75**, and the Low quality got its coarsest pyramid level back (4 levels at every quality) |
| V2-25 | `912f1c4`, `03b9606` | Q-100's four real departures from the original: the missing `SmartBlur`, 3 levels → **4**, the coarsest sphere `r` → **1,75 r**, the fixed pattern → a per-pixel rotation |

The four numbers that carry the calibration, and that this port has to reproduce:

| Quantity | Value | Where it comes from |
|---|---|---|
| `zaxMsvoShare` | **0,75** | Q-097 / V2-24, swept over 0,12 … 1,00 |
| coarsest sphere | **1,75 r** | Q-100 correction 3 (`2 r` of §2.4 less the blur's reach) |
| pyramid levels | **4** at every quality | Q-100 correction 2, and Q-097 for Low |
| blur sharpness | **32** | `SmartBlur` port, a free choice of §2.4 |

### What exists on the branch, checked piece by piece

| Piece | State |
|---|---|
| `style::MSVO_PORTED` | `false`, and `thickness_acts` is `MSVO_PORTED && ao == Msvo` — one constant two distant places agree on, exactly as this lot needs |
| `style::ao_radius_units`, `ao_ceiling`, `ao_samples`, `ao_half_resolution` | **exist**, normative, unit-tested; MSVO reuses all four unchanged (REQ-SCN-083 *Paramètres*) |
| `style.ao_thickness` | persisted and bounded (RU-2), carried on `StyleSettings`, **read by nothing** |
| `gl/screen_effects.rs` | `ScreenEffects` → `Option<Occlusion>` + `Option<Reflections>`; `Occlusion` = three programs (`AO`, `AO_BLUR`, `AO_COMPOSITE`) and two `R8` targets |
| the shared depth helpers | `shaders/screen_depth.glsl`: `zaxViewDepth`, `zaxViewSample`, `zaxProjectView`, `zaxDither`, `zaxViewNormal` — the linearise pass of the pyramid wants the first two |
| where the ceiling is applied | **inside `ao.frag`**, not in the composite: the `R8` target holds `O` already scaled, and `ao_composite.frag` is a bare `1 − O`. The .NET applies it in its shared `Apply` pass instead. **The Rust convention wins** — it keeps `ao_composite.frag` byte-identical and shared |
| the frame the passes see | `ScreenFrame { depth, color, capacity, drawn, drawn_scale, projection }`; `projection` = `(m11, m22, range, near)`, which is where the .NET's `Projection.M22` focal term comes from |
| the lifecycle | `select_screen_effects` (two boolean tests in Classic) → `prepare_screen_effects` (`#[inline(never)]`) → `build_screen_effects`; a refusal gives the effects up for the session with one diagnostic line |
| the statistics | `style_programs`, `screen_effect_bytes`, `screen_effect_passes` — the three zeros RU-5 made observable |
| the handheld default | `ui/handheld/defaults.rs` already forces `render.visual_style` = 0 and `spl.ao` = 0, and `ao.quality` = 0 — **the console default already keeps AO off**, so point 4 of the brief needs nothing there |
| the options window | `options::inert_reason` greys `ao.thickness` through `style::thickness_acts`; `EXCLUDED` holds all thirteen style keys |
| the `wgpu` backend | draws Classic, recorded in `Q-104` §5; point 5 of the brief leaves it alone |

### Requirement map

| Piece of REQ-SCN-083 *Mode MSVO* | Rust file |
|---|---|
| the pass, its resources, its lifecycle | `zax-render/src/gl/msvo.rs` (new) |
| depth preparation | `shaders/msvo_linearize.frag` (new), on top of `screen_depth.glsl` |
| the pyramid | `shaders/msvo_downsample.frag` (new) |
| occlusion per scale | `shaders/msvo_occlude.frag` (new) |
| the `SmartBlur` | `shaders/msvo_blur.frag` (new) |
| blur and upsampling, the combination, `O` | `shaders/msvo_upsample.frag` (new) |
| the program declarations | `gl/shaders.rs` (five `ProgramSource`s) |
| `ao.thickness`, the qualities | `style.rs` (`msvo_falloff`, `msvo_levels`, `msvo_pattern_entries`) |
| the choice of estimator, and `MSVO_PORTED` | `style.rs`, `gl/screen_effects.rs` |
| the GLES fallback to Depth | `gl/screen_effects.rs`, reported through `diagnostics.rs` |
| the licence in the distribution | `THIRD-PARTY-NOTICES.md` (the three packaging scripts already carry it) |

### The design this lot rests on, and why

Five decisions, each forced by the brief's "no performance regression, none at all" and by the
architecture RU-5 left.

1. **`spl.ao` chooses between two estimators, and only one is ever built.** RU-5's `Occlusion`
   holds the Depth programs unconditionally. MSVO must not compile the hemisphere pass and Depth
   must not compile the five MSVO ones, so `Occlusion` keeps the **shared** composite and holds
   one of two bodies. `style_programs` then reads 3 in Depth and 6 in MSVO, and switching the mode
   releases the other side's targets.
2. **The ceiling stays inside the occlusion pass.** The `R8` target holds `O` = `ceiling ×
   occlusion`, which is what `ao.frag` already writes and what `ao_composite.frag` already reads.
   The final upsample therefore ends with `uCeiling × min(1, (1 − access) / zaxMsvoShare)` — the
   .NET's two steps (`/ share` into `R8`, then `× ceiling` in Apply) composed into one, bit for bit
   the same value, with one fewer uniform on a shared program and `ao_composite.frag` untouched.
3. **Every level is allocated at `capacity >> level` and viewported at `drawn >> level`**, the
   convention `linear_buffer` and RU-5's two passes already use — so a host that reserves room for
   a size that changes often (`LinearColorBuffer::reserve`) does not reallocate the pyramid every
   frame. Each pass is given the **exact** scale of each level it samples (`drawn_L / cap_L`,
   computed on the CPU once per pass), rather than the frame's, because at a sixteenth of the frame
   integer truncation is no longer negligible.
4. **`R16F` is not a new capability.** The pyramid holds a view distance in scene units, which
   wants more than 8 bits. `R16F` is colour-renderable on GL 3.3 core, and on GLES 3.0 through the
   same `EXT_color_buffer_float` / `EXT_color_buffer_half_float` that already makes the
   `RGBA16F` of `LinearColorBuffer` renderable — a driver that gives one gives the other. So the
   probe RU-5 declined to add is still not needed; what the lot adds instead is the **fallback**
   (point 4 of the brief): a pyramid that cannot be allocated falls back to the **Depth**
   estimator for that frame and every later one, rather than giving the whole module up.
5. **No `unsafe`, no new per-frame work outside the mode.** Everything this lot adds lives inside
   `run_screen_effects`, which RU-5 already took out of line precisely so that the Enhanced path
   does not sit in `render_scene`'s instruction cache. Classic and Enhanced-without-AO execute
   not one new instruction.

### The test plan

Ported from `Amd055OcclusionTests.cs` and `Amd055StyleStrengthTests.cs`, each criterion kept with
its `REQ-...` identifier. The RU-5 tests that **pin the fallback** are the ones to rewrite:

| Criterion | Where it goes |
|---|---|
| §2.4 invariant 1: a plane alone is 1,00 ± 0,02 — in **MSVO** | `amd055_msvo_tests.rs` (new) |
| §2.4 invariant 2: the contact `< 1` and `≥ 0,10`; the plane at `2 r` = 1,00 ± 0,02 | the same |
| §2.4 invariant 3: `ao.strength` 1 → ≥ 0,70, 5 → ≥ 0,05, non-increasing, strictly 1 → 4 | the same |
| §2.4 invariant 4 **with the port**: `ao.thickness` 1 → 5 moves at least one pixel by > 2 sRGB levels, and the contact ratio does not **grow** | the same |
| REQ-SCN-083: the darkening never decreases as `ao.thickness` grows | the same |
| §2.4 invariant 5: `spl.ao` = 0 is the Classic picture; the A-h variant leaves the skybox and the HUD alone | the same |
| the three qualities give the same general aspect, Low noisier; only High is full resolution | the same |
| Q-100: the multi-scale mode is **wider** than the single-scale one (the ratio the reference gives as 1,38) | the same |
| the fallback: a refused pyramid draws **Depth**, says so once, and is not retried | the same |
| RU-5's `req_scn_083_msvo_is_rendered_as_depth_and_the_thickness_does_nothing` | **rewritten**: MSVO is no longer Depth, and the thickness acts |
| RU-5's `req_scn_079_the_thickness_is_greyed_while_msvo_is_rendered_as_depth` | **rewritten**: greyed at `spl.ao` 0 and 2, live at 1 |

### The "before" figures (step 1)

Taken on this machine, in release, nothing else running, at `f5a5be9`. **No commercial ROM is
available on this side** (`ZAXNES_ROMS` is unset and the original program's folders are out of
bounds), so the ROM comparisons of the brief's "Checks" section are measured on the **synthetic
fixtures** of §2.4 — the close-up A and a floor-and-kerb scene, which is what Q-100's own
measurement table uses — and the Punch-Out!! / SMB frames stay R0's.

**Post-LTO instruction counts** (`cargo rustc -p zax-harness --release --bin zax-harness --
--emit=asm`, the method of RU-3a-fix through RU-5):

| Symbol | instructions |
|---|---|
| `Ppu2C02::run_dots` | **2231** |
| `DrawListBuilder::build` | **1095** |
| `SceneRenderer::render_scene` | **3411** |
| `Construction::segment` | **159** |

All four agree with the figures RU-5 recorded at its close, which is the check that this lot starts
where that one ended. (RU-5 step 7c read `DrawListBuilder::build` as 1187 with a differently
written counting script; the same script runs over both columns of every comparison in this lot.)

`cargo test --release -p zax-render`: **287 passed, 0 failed**, 1 ignored.

### The checklist was rearranged after step 1

The brief suggests six steps, the second being "resources and the lifecycle of the passes" and the
third "occlusion per scale and the combination". Read against the code, those two cannot be
separated into two **green** commits, which is what this lot's protocol asks of every step:

- the resources are per-level targets of a pyramid, and a pyramid with no estimator over it is
  observable only as a memory count — a step whose whole deliverable is an allocation;
- and the estimator cannot be tested at all until the mode is selectable, because
  `style::MSVO_PORTED` gates it: with the gate shut the chain is dead code, with it open the chain
  must already be right (REQ-SCN-083's invariants hold "aussi pour ce mode").

So step 2 lands the pass whole and opens the gate, which is the brief's steps 2, 3 and most of 4.
What is left of its step 4 — the handheld defaults, the console page and the capability finding —
is a subject of its own and became step 3; the brief's steps 5 and 6 split into the comparisons
(step 4), the cost measurements (step 5) and the close (step 6). Nothing of the brief is dropped;
what moved is which commit carries it.

### Step 2 — the pass, the lifecycle, and the mode switched on

| Piece | What landed |
|---|---|
| `gl/msvo.rs` (new) | `Msvo`: the five programs, the four-level pyramid, its allocation and release, and the chain pyramid, occlusion, smoothing, combination |
| `shaders/msvo_linearize.frag` (new) | the frame's window depth as a view distance in scene units (`LinearizeZ`) |
| `shaders/msvo_downsample.frag` (new) | one coarser level, point-decimated (`AoPrepareDepthBuffers2CS`) |
| `shaders/msvo_occlude.frag` (new) | the sphere-penetration test of `AoRenderCS.hlsli`, its checkerboard and its weights |
| `shaders/msvo_blur.frag` (new) | the `SmartBlur` of `AoBlurAndUpsampleCS.hlsli`, separable, over every level |
| `shaders/msvo_upsample.frag` (new) | the bilateral upsample, and in its last step the `O` of REQ-SCN-083 |
| `gl/screen_effects.rs` | `Occlusion` = the shared composite plus one `Estimator` of two; `release_occlusion`; the runtime fallback (`msvo_refused`, `refuse_msvo`) |
| `gl/shaders.rs` | the five `ProgramSource`s; `AO_COMPOSITE`'s doc now says it is shared |
| `style.rs` | `MSVO_PORTED` **true**; `MSVO_LEVELS`, `MSVO_COARSEST_RADIUS`, `MSVO_SHARE`, `MSVO_ACCESS_HEADROOM`, `MSVO_BLUR_SHARPNESS`, `MSVO_PATTERN` and its two entry counts; `msvo_active`, `msvo_falloff`, `msvo_pattern_entries`, `msvo_weight_scale`, `msvo_level_radius` |
| `statistics.rs` | `occlusion_levels`, the one thing that tells the two estimators apart from outside |
| `diagnostics.rs` | `msvo_unavailable`, the one line of the fallback **(D)** |
| `renderer.rs` | the `occlusion_levels` reading, the fallback's line, `refuse_msvo` and `msvo_refused` |
| `THIRD-PARTY-NOTICES.md` | the MiniEngine entry with the MIT licence in full — which is how the three packaging scripts already carry every third-party licence |

Each of the six new files carries the attribution header `third_party/README.md` prescribes, with
the origin, the files it adapts and the note that the MIT licence is compatible with the GPL-3.0.

#### The four departures Q-100 found are in from the start

The .NET port needed lots V2-20 to V2-25 to arrive at them; this one is written with all four:
the `SmartBlur` over **every** level, **four** pyramid levels at every quality, a coarsest sphere
of **1,75 r**, and a pattern turned by a **per-pixel** hash. Each is a named constant in `style.rs`
with the Q-ticket that settled it in its doc comment, so the calibration lives in one place and the
shader reads it as a uniform.

#### Two decisions of this port the .NET one did not have to make

1. **The ceiling stays inside the estimator, and `ao_composite.frag` is untouched.** The .NET has
   a separate `Apply` pass with a `uCeiling` uniform; the Rust convention since RU-5 is that the
   `R8` target holds `O` already scaled and the composite is a bare `1 - O`. The last upsample step
   therefore ends with `uCeiling x clamp((1 - access) / share, 0, 1)` — the .NET's two steps
   composed into one, the same value, and the composite shared by the two estimators byte for byte.
2. **Each level is given the exact scale of each level it samples.** RU-5's two passes scale every
   read by the frame's own `drawn_scale`, which is right to a texel at full and half resolution; at
   a sixteenth of the frame the integer truncation of `drawn >> 4` against `capacity >> 4` is no
   longer negligible, so `scale_of(drawn_L, capacity_L)` is computed per level, on the CPU, once
   per pass.

#### Three readings that were wrong, and the invariant that found all three

§2.4's **first** invariant — "un plan seul … le rapport vaut 1,00 ± 0,02 en tout pixel du sol" — is
the one that pays for itself. The first run of the chain measured **0,949** at its worst, and
chasing that number is what produced three real corrections. Each is recorded where it acts, with
the measurement:

1. **A sample outside the drawn rectangle must not occlude.** The original gathers through a
   *border* sampler; a port that keeps the drawn rectangle of a reserved attachment cannot, and
   `CLAMP_TO_EDGE` repeats the edge texel instead — which breaks the cancellation the opposite
   pairs rest on. `msvo_occlude.frag` answers 1 ("no occlusion") for such a sample, the same arm
   `ao.frag` carries. (On its own it changed nothing on this fixture, where the coarsest sphere
   still reaches a quarter of the image but the plane fills it; it is kept because it is a
   correctness arm and not a tuning one.)
2. **The pyramid is point-sampled, not filtered.** A level holds a *perspective* depth, convex
   along a screen line, so interpolating it between texel centres adds a ripple that peaks
   mid-texel — and the two samples of a pair pick up different amounts of it. `NEAREST` took the
   worst ratio at full resolution from **0,996 to exactly 1**.
3. **The access is stored with headroom** (`MSVO_ACCESS_HEADROOM` = 2). This is the big one. The
   access is 1 on a lone plane and rises **above** 1 where a pair is less than half occluded; the
   original writes it to a float, and clamping it at 1 before the smoothing turns the estimator's
   **zero-mean** sampling noise into a one-sided bias that four levels then multiply. Dividing by
   two before the `R8` store keeps the sign of the noise and lets the `SmartBlur` average it away.

With all three in, the invariant reads **exactly 1,00000** over all 144 094 pixels of the plane, at
**each of the nine** combinations of `ao.quality` and `ao.radius` the test walks — which is why it
walks them: all three faults bite harder as the sphere covers fewer texels, and the default
(Medium, `r` = 20) is not the worst case. Before the headroom, `ao.radius` = 1 at half resolution
left **41 569** pixels of the plane below 0,98.

Worth stating plainly, because it cuts the other way too: the noise the clamp was rectifying was
*also* inflating the effect. Before the fix the mode moved 3,55 % of the frame and darkened it by
0,89 sRGB levels on average; after, 1,58 % and 0,34. The earlier figures were an artefact, and the
later ones are the estimator.

#### The invariants of §2.4, measured in the mode

Close-up A as the fixture reaches it (640 × 480, albedo 0,503, `r` = 20, `ao.strength` = 3):

| Invariant | Required | Measured |
|---|---|---|
| 1 — a plane alone | 1,00 ± 0,02 at every pixel | **1,00000**, all 144 094, at all nine quality × radius |
| 2 — the contact | `< 1` and `>= 0,10` | **0,11785** |
| 2 — the plane at `2 r` | 1,00 ± 0,02 | 1,00000 at each visible far point |
| 3 — `ao.strength` 1 to 5, worst | >= 0,70 / 0,40 / 0,10 / 0,05 / 0,05 | **0,70745 / 0,41001 / 0,11785 / 0,06920 / 0,06920** |
| 3 — the contact in `ao.strength` | non-increasing, strict 1 to 4 | 0,707 > 0,410 > 0,118 > 0,069 = 0,069 |
| 4 — `ao.thickness` 1 to 5 | one pixel by more than 2 sRGB levels | **86 levels** |
| 4 — the contact in `ao.thickness` | does not grow | 0,536 >= 0,211 >= 0,118 >= 0,101 >= 0,101 |
| 5 — `spl.ao` = 0 | the Classic capture | identical, tolerance 0 |
| the qualities | the same general aspect; only High full-resolution | contact 0,127 / 0,118 / 0,101; bytes x1 / x1 / x4 |

The `ao.strength` floors are read with the same one-sRGB-level slack the Depth file uses, which is
the tolerance §2.4 states for the whole section since Q-097.

#### The fallback **(D)** is still a real path, and is tested

`MSVO_PORTED` is now true, so the fallback is no longer a compile-time state — it is a **runtime**
one, and better for it: an implementation that refuses the pyramid (the one format the mode needs
that nothing else here does, an `R16F` colour target) falls back on the Depth estimator, writes one
diagnostic line naming REQ-SCN-083, and is not retried. The refusal is injected by
`SceneRenderer::refuse_msvo`, the same kind of seam `fail_screen_effects` is.

**`R16F` is not a new capability**, which is the finding RU-5's step 7c would have recorded: it is
colour-renderable in OpenGL 3.3 core, and on GLES 3.0 through the very
`EXT_color_buffer_float` / `EXT_color_buffer_half_float` that already makes the `RGBA16F` of
`LinearColorBuffer` renderable on every frame — a driver that gives one gives the other. Sixteen
bits are enough *with* the headroom in place: measured, `R32F` changes no figure of the table above.

#### The brief's "defaults unchanged", measured

- **The twelve files of the synthetic render are byte identical to `f5a5be9`'s**, at
  `--anti-alias 0` *and* at the default anti-aliasing — the eleven captures and `render.json` with
  them (draw calls, triangles, resident meshes, colours). Compared with `cmp` against a throwaway
  `git worktree`, the method RU-5 established.
- **Post-LTO instruction counts**, the same script over both columns:

| Symbol | `f5a5be9` | step 2 |
|---|---|---|
| `Ppu2C02::run_dots` | 2231 | **2231** |
| `DrawListBuilder::build` | 1095 | **1095** |
| `SceneRenderer::render_scene` | 3411 | **3401** (−10, −0,29 %) |
| `Construction::segment` | 159 | **159** |

  Three are untouched, as they must be. `render_scene` is ten instructions **smaller**: everything
  this step adds lives behind the `is_some` that calls `run_screen_effects`, which RU-5 already took
  out of line, and the estimator choice is made inside `ScreenEffects::occlude` — so the only change
  `render_scene` sees is the one statistics line, and the ten are code layout. Nothing new runs in a
  Classic frame.

#### Tests

| Criterion | Test |
|---|---|
| §2.4 invariant 1 in the mode, over nine quality × radius combinations | `amd055_msvo_tests.rs` `req_scn_083_msvo_leaves_a_plane_alone_undarkened` |
| invariant 2, the contact and the plane at `2 r` | `req_scn_083_msvo_darkens_the_foot_of_the_cube_and_not_the_far_plane` |
| invariant 3, the five floors and the two monotonicities | `req_scn_083_msvo_keeps_the_strength_as_the_darkening_ceiling` |
| invariant 4 **with the port**, and REQ-SCN-083's "l'assombrissement NE DOIT PAS décroître quand `ao.thickness` croît" over all five rungs | `req_scn_083_the_thickness_acts_in_the_msvo_mode_and_never_lightens` |
| invariant 5 and the three zeros, released again when the style goes back | `req_scn_083_msvo_is_off_at_zero_and_in_classic` |
| the brief's point 2 one level down: one estimator or the other, never both | `req_scn_083_each_mode_builds_only_its_own_estimator` |
| the quality rungs: four scales at each, the pattern prefix at Low, only High full-resolution | `req_scn_083_the_msvo_qualities_agree_and_only_high_is_full_resolution` |
| the fallback **(D)**: the Depth picture, one line, not retried | `req_scn_083_an_implementation_that_refuses_the_pyramid_renders_msvo_as_depth` |
| Q-100: the multi-scale mode reaches further than the single-scale one | `req_scn_083_the_multi_scale_mode_reaches_further_than_the_single_scale_one` |
| REQ-SCN-079: `ao.thickness` does nothing to the **Depth** estimator (the half of the clause that file owns) | `amd055_occlusion_tests.rs` `req_scn_079_the_thickness_does_nothing_to_the_depth_occlusion` (rewritten) |
| REQ-SCN-079's greying: live at `spl.ao` = 1, greyed at 0 and 2 | `zax-app` `amd055_style_options_tests.rs` `req_scn_079_the_thickness_is_greyed_outside_the_msvo_mode` (rewritten) |
| the pattern table against the square root of `1 - u² - v²`, the prefix, and the weights summing to 1 | 4 unit tests in `style.rs` |
| the five rungs of the rejection fall-off; the four level radii; the mode's gate | the same |

The two tests RU-5 wrote to pin the fallback are the two marked "rewritten": the occlusion file's
`req_scn_083_msvo_is_rendered_as_depth_and_the_thickness_does_nothing` and the options file's
`req_scn_079_the_thickness_is_greyed_while_msvo_is_rendered_as_depth`. Neither was deleted — each
kept the half of its criterion that survives the port and handed the other half to its new home,
and both now assert `MSVO_PORTED` in a `const` block so that whoever flips the constant back is
sent straight to them.

`cargo test --release -p zax-render`: **300 passed, 0 failed**, 1 ignored (287 at the start of the
lot). `-p zax-app --test amd055_style_options_tests --features bundled`: 10 passed.
`cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings` clean.

### Step 3 — the handheld and the GLES side

Point 4 of the brief is **almost entirely already true**, and what this step adds is the finding,
the two comments that record it, and the one line of Q-104's test that keeps the gap pinned.

| Clause of point 4 | State |
|---|---|
| "the console default keeps AO off" | **already true** — `ui/handheld/defaults.rs` sets nothing for `spl.ao`, because REQ-SCN-079's own default is 0 and the four effect switches need no handheld default of their own |
| "the handheld options page only offers it as an advanced option if useful" | **already true** — RO-3 put `ao.thickness` under *Advanced options › Light and shadows › Ambient occlusion* (`handheld/options.rs` `LIGHT_SHADOWS_ROWS`), and the port is exactly what makes it useful |
| "if MSVO cannot run on GLES 3.0 … it falls back to Depth there" | **landed in step 2**, and tested there: `req_scn_083_an_implementation_that_refuses_the_pyramid_renders_msvo_as_depth` |
| "says so in the console page as RU-5 does for unsupported effects" | **RU-5 does nothing in the console page**, and for the same reason this lot does nothing: see below |

#### What the brief's "as RU-5 does" actually resolves to

RU-5's step 7c answered the same conditional and found that no capability was new, so there was
nothing to grey out and nothing to say: its fallback is a **render diagnostic** line, not a UI
state, and `options::inert_reason` — the greying rule RO-1 established — is read only by the
desktop window (`ui/zax_ui/options.rs`), never by the console tree. Greying a console row would be
a change to RO-3's tree and to the greying rule's single call site, which is beyond this lot and
would be the wrong shape anyway: the refusal is discovered by the **renderer**, on the first frame
that asks for the mode, and the settings model has no way to know of it.

So the fallback says so where RU-5's says so: one line through AMD-051's bounded diagnostics
channel, named after REQ-SCN-083 (`diagnostics::msvo_unavailable`), once per game. The brief's
condition — "**if** MSVO cannot run on GLES 3.0 with the capability probing of RU-5" — does not
hold in any case:

| Format the mode needs | Required renderable where |
|---|---|
| `R8` (the four occlusion targets per level) | colour-renderable in OpenGL 3.3 core, **required** renderable in GLES 3.0 — RU-5's own occlusion already uses it |
| `R16F` (the four pyramid levels) | colour-renderable in OpenGL 3.3 core; on GLES 3.0 through `EXT_color_buffer_float` or `EXT_color_buffer_half_float`, the very extensions that already make the `RGBA16F` of `LinearColorBuffer` renderable on **every** frame — a driver that gives one gives the other |
| the sampled `DEPTH_COMPONENT24` | the frame's own, allocated by RU-5 |

And there is no probe at context creation, deliberately, for the reason RU-5 recorded: a probe has
to allocate something to be worth anything, and the brief forbids Classic allocating anything the
amendment needs. The first frame that asks for the mode *is* the probe, and a refusal is handled
rather than predicted.

#### What landed

| Piece | What changed |
|---|---|
| `ui/handheld/defaults.rs` | the comment on `ao.quality` now covers **both** estimators — at `spl.ao` = 1 the rung chooses the sample pattern (four of the seven checkerboard pairs at Low) rather than a sample count, the half-resolution clause is the same, and the pyramid keeps its four levels at every rung (Q-097) so that a handheld gets a *cheaper* effect and not a lighter one |
| `ui/handheld/defaults.rs` test | three assertions: `spl.ao` is 0 on a handheld, and `ao.thickness` keeps the amendment's default on both platforms — it is one uniform of the MSVO shader and costs nothing at any rung, so there is no economical value to prefer |
| `amd055_style_tests.rs` | `msvo` joined the five names Q-104's known-gap test forbids in the `wgpu` WGSL, so the gap stays pinned one effect wider |

Nothing else of point 5 of the brief was needed: the `wgpu` backend still draws Classic, Q-104 §5
already describes that gap for the whole of AMD-055, and the port added no WGSL.

`cargo test --release -p zax-app --lib --features bundled`: 131 passed. The six option suites of
`zax-app` (`amd055_style_options`, `app_options`, `desktop_options_golden`, `handheld_tree`,
`handheld_ui`, `options_window`): 70 passed, 0 failed, 1 ignored.
`cargo test --release -p zax-render --features wgpu --test amd055_style_tests`: 5 passed.

### Step 4 — the comparison with the .NET figures

#### No commercial ROM is on this side, and V2-25 was in the same position

The brief asks for "the same profiles as the V2-24/V2-25 comparisons (Punch-Out!! 1499, SMB 419)".
`ZAXNES_ROMS` is unset, `test-roms/` holds only the public test ROMs, and a search of the whole
clean-room tree for `*.nes` finds no commercial title. **V2-25 itself hit this exactly**: its notes
open with "the Punch-Out!! ROM is gone … It cannot be the R5 scene for this lot, and the
reference's own figures for it cannot be reproduced here", and it measured instead on the two
**normative, third-party-reproducible** configurations of §2.4 plus one synthetic scene of its own
making. This lot measures on the same three, which is what makes the two columns comparable at all:

- the **close-up A** of §2.4 — the floor and the 8-unit cube;
- the **kerb scene** V2-25 added for the large-scale question — the same floor with a laterally
  wide occluder of 64 × 8 × 8 units, because the pattern of each level is a seven-entry
  checkerboard scaled by that level's sphere radius, and an 8-unit cube falls between the samples
  of a coarse one whatever the pyramid does;
- and the two modes read off the **same** capture of each, so that the figure that matters — the
  mode against the Depth mode — is free of every fixture and framing difference.

The Punch-Out!! and SMB figures stay R0's, and `Q-100`'s R1 answer already records them for the
.NET port (MSVO zone 29,7 % against the reference's 32,2 %).

#### Why the absolute columns are not comparable, and what is

The two fixtures differ in three ways the progress file of RU-5 already records for this side: the
Rust floor is the frame's **own plane** (normal (0, 0, −1)) rather than the criterion's floor turned
by `rot = (90, 0, 0)`, the albedo is reached through NES colour `0x10` rather than a custom direct
texture, and the capture is 640 × 480 unless stated. The base brightness of the floor therefore
differs, and a darkening read in **sRGB levels** scales with it: the Rust columns below run about a
third of the .NET's on the same bands. What no such difference can disturb is the **ratio of one
mode to the other on one capture**, which is the figure both Q-095's answer and Q-100 are written
in, and which is why V2-25 printed its Depth column "for scale".

#### Close-up A — the mean darkening of the floor by 4-unit bands of distance to the cube

At **1920 × 1080**, the resolution the criterion and the .NET lots measure at:

| | 0-4 | 4-8 | 8-12 | 12-16 | 16-20 | wide/contact | zone | mean |
|---|---|---|---|---|---|---|---|---|
| Depth, .NET V2-25 | 30,90 | 21,13 | 13,01 | 6,49 | 2,63 | 0,239 | — | — |
| MSVO, .NET V2-25 | 30,20 | 14,27 | 7,59 | 3,87 | 1,77 | **0,146** | — | — |
| Depth, **Rust RU-6** | 12,07 | 5,57 | 3,20 | 1,48 | 0,58 | 0,145 | 1,378 % | 0,220 |
| MSVO, **Rust RU-6** | 22,88 | 10,64 | 5,80 | 3,74 | 0,96 | **0,153** | 1,452 % | 0,377 |

("wide/contact" is V2-25's own summary: the mean of the `0,5 r` to `r` bands over the contact band.
"zone" is the share of the frame whose RGB-summed change exceeds 12 and "mean" the mean darkening
over the whole frame — the two figures of the Q-095 answer.)

Read as ratios of one mode to the other on the same capture:

| figure | .NET | **Rust** | the reference (Q-095 R1, the ring) |
|---|---|---|---|
| MSVO wide/contact over Depth's | 0,61 | **1,06** | — |
| MSVO zone over Depth's | — | **1,05** | **1,38** |
| MSVO mean over Depth's | — | **1,71** | **1,37** |

The .NET port's MSVO was **narrower** than its own Depth mode (0,61), which is the whole subject of
Q-100; this one is **wider** on both figures, and its mean ratio of 1,71 brackets the reference's
1,37 from above rather than from a third of it.

#### The kerb scene — the laterally wide occluder

At **1920 × 1080**, on the **near side of the kerb** (see `kerb_profile`'s own comment for why the
side has to be chosen):

| | 0-4 | 4-8 | 8-12 | 12-16 | mid/contact | zone | mean |
|---|---|---|---|---|---|---|---|
| Depth, .NET V2-25 | 90,38 | 48,62 | 25,49 | 7,25 | 0,410 | — | — |
| MSVO, .NET V2-25 | 67,07 | 20,18 | 5,47 | 0,57 | **0,191** | — | — |
| Depth, **Rust RU-6** | 17,74 | 8,94 | 4,34 | 1,85 | 0,374 | 4,685 % | 0,657 |
| MSVO, **Rust RU-6** | 34,19 | 16,82 | 5,12 | 0,14 | **0,321** | 3,668 % | 0,952 |

| figure | .NET | **Rust** |
|---|---|---|
| MSVO mid/contact over Depth's — the bound V2-25's regression test holds at > 0,30 | **0,466** | **0,858** |
| MSVO contact band over Depth's | 0,742 | **1,927** |
| MSVO zone over Depth's | — | 0,78 |
| MSVO mean over Depth's | — | 1,45 |

And at 640 × 480, which is where the committed regression test runs: Depth 17,20 / 9,24 / 4,42 /
1,82 (mid/contact 0,397), MSVO 23,41 / 16,17 / 4,86 / 0,11 (mid/contact **0,449**), the ratio
**1,131**.

#### The three figures Q-100 is written in, side by side

| Q-100's figure | the reference | .NET at V2-25 | .NET after its R1 re-measure | **Rust RU-6** |
|---|---|---|---|---|
| darkening at the mat centre (Punch-Out!!) | 15,7 | 11,2 | 18,0 | **R0's** — no ROM here |
| zone, MSVO (Punch-Out!!) | 32,2 % | 23 % | 29,7 % | **R0's** |
| **MSVO zone over Depth zone** | **1,38** | 0,98 (V2-24) | — | **1,05** close-up A, 0,78 kerb |
| **MSVO mid/contact over Depth's**, kerb | — | 0,466 | — | **0,858** |
| **MSVO wide/contact over Depth's**, close-up A | — | 0,61 | — | **1,06** |

What this says, stated plainly:

1. **The ordering the reference shows is now the port's ordering.** MSVO stronger than Depth in
   mean (1,71 and 1,45 against the reference's 1,37) and wider in zone on the close-up A (1,05).
   The .NET port at V2-25 had it the other way on every figure, which is what Q-100 opened for.
2. **On a laterally wide, low occluder the zone ratio is still under 1** (0,78 on the kerb), and
   that is the residual of exactly the structural cause Q-100 names: the MiniEngine estimator
   compares a sample's depth to the **centre pixel's**, so an occluder counts by its departure from
   the surface's own depth gradient and not by its rise above the tangent plane. A low kerb seen on
   a receding floor departs from that gradient very little. No calibration constant moves it, and
   `MSVO_SHARE` is **not** retouched — for the reason V2-25 gave and this lot inherits: the scene
   the reference's own figures come from is not on this machine, and retuning a constant against a
   different scene is tuning blind.
3. The gap is therefore **narrower than the .NET port's by a factor of 1,7 to 1,8** on both
   synthetic scenes, and Q-100's hypothesis 1 — the port stays the MiniEngine's and the residual
   width is a declared deviation — still holds, with a smaller residual. The Rust appendix of
   Q-100 (step 6) records this without touching the ticket's earlier text.

Three of the four corrections of step 2 contribute to the improvement, and the access **headroom**
is the one that does most of it: before it, the clamp at 1 rectified the estimator's noise into a
spurious darkening spread over the whole frame, which *inflated* the zone and the mean while
breaking §2.4's first invariant. Removing it both fixed the invariant and let the real mid-field
through.

#### What landed

`req_scn_083_the_msvo_mode_finds_a_laterally_wide_occluder` in `amd055_msvo_tests.rs`: the kerb
scene, both modes read off two captures of it, and the **four bounds of V2-25's own regression
test** — the contact band over 20 levels (23,4), mid/contact over 0,12 (0,449), that ratio over the
Depth mode's over 0,30 (1,131), and nothing beyond `2 r` (0,000). Keeping the .NET's bounds rather
than tightening them to this port's figures is deliberate: a regression in either direction is then
caught by a test whose numbers are already public in V2-25.

`cargo test --release -p zax-render --test amd055_msvo_tests`: 10 passed. `cargo fmt --all --check`
and `cargo clippy -p zax-render --all-targets -- -D warnings` clean.
