# RU-5 — the Enhanced visual style (AMD-055)

Branch `rustupdated`, on top of RU-4b (`9cb306c`). Amendment AMD-055 (REQ-SCN-078 to REQ-SCN-084:
style selection, parameters, base lighting and the order of effects, voxel grid and pixel
shadows, ambient occlusion, floor reflections) ported to the **Rust** application. The .NET lot of
AMD-055 on `dotnetupdated` is a reading reference only.

## Checklist

| # | Step | Status |
|---|---|---|
| 1 | Map: what exists, where each requirement goes, the "before" measurements | in progress |
| 2 | REQ-SCN-078: the style reaches the renderer, the lazy resource lifecycle, the zero assertions | todo |
| 3 | REQ-SCN-080: base lighting and the order of passes | todo |
| 4 | REQ-SCN-081: the voxel grid | todo |
| 5 | REQ-SCN-082: the pixel shadows | todo |
| 6 | REQ-SCN-083: ambient occlusion (Depth, MSVO under its declared fallback D) | todo |
| 7 | REQ-SCN-084: the floor reflections | todo |
| 8 | GLES capability probing, handheld page, desktop options group, harness options | todo |
| 9 | The per-effect measurements | todo |
| 10 | Closing: `PERFORMANCE-HISTORY.md`, intended image changes, fmt, clippy, full tests, goldens | todo |

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
