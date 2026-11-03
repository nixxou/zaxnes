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
| 2 | The MSVO pass: the module, the resources, the lifecycle, the whole chain, the mode switched on, and the invariants of §2.4 in it | done — this commit |
| 3 | The handheld and the GLES side: the defaults, the console page, the capability finding, Q-104 | todo |
| 4 | The comparisons with the figures of the .NET lots and of Q-100 | todo |
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
