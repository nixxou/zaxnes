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
| 1 | Map: the .NET lots read, the requirement map, the test plan, the "before" figures | done — `4d18aa5` |
| 2 | The resources and the lifecycle of the passes (the pyramid, the targets, the programs) | done — `3306bb1` |
| 3 | The occlusion per scale, the smoothing and the combination | done — `34c8c68` |
| 4 | Switching on: `MSVO_PORTED`, the parameters, the thickness, the qualities, the GLES fallback | done — `92af5b9` |
| 5 | The comparisons with .NET and the cost measurements | done — `bf12c5e` |
| 6 | Closing: notes, `PERFORMANCE-HISTORY.md`, the Q-100 appendix, fmt, clippy, full tests, release build | done — this commit |

**The lot is closed.** What is left is R0's and is listed under "What is left for R0" below.

The hash of a step is written in the commit of the step that follows (no amend, protocol of this
lot).

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
