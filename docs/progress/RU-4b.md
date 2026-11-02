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
| 1 | Map: what exists, where each requirement goes, the "before" measurements | done |
| 2 | AMD-051: robustness (REQ-E3D-644, -646, REQ-LOOP-073, REQ-PERS-088, REQ-E3D-524) | todo |
| 3 | AMD-048: the empty depth interval of REQ-E3D-450, TV-Y9 and TV-Y10 | todo |
| 4 | AMD-057 walls: the Q-067 seal removed, the wall depth, the measurement, the handheld decision | todo |
| 5 | AMD-057 sprite depth: `0.001 × (i mod 64) ∓ 0.1` | todo |
| 6 | AMD-058: the joints under MSAA, a shader variant for `anti_alias` ≥ 1 only | todo |
| 7 | AMD-049: REQ-SCN-075 as a shader variant, the setting, the vectors | todo |
| 8 | The settings: desktop options, handheld defaults and page, libretro, harness options | todo |
| 9 | Closing: measurements, intended image changes, `PERFORMANCE-HISTORY.md`, fmt, clippy, tests, goldens | todo |

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
