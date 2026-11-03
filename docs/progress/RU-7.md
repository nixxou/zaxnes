# RU-7 — the parity of the `wgpu` backend (Q-104)

Branch `reworkoptions`, on top of RU-6 (`faced2a`). Q-104 recorded that the `wgpu` backend draws
the scene flat and in the Classic style, and gave as its reason that WGSL could not be *verified*
on the machines of RU-4a and RU-5: `crates/zax-harness/tests/render_wgpu_tests.rs` skips without a
Vulkan driver. The owner's answer to the ticket is **port it**. Step 1 of this lot is a gate on
that premise.

Build directory: `rust/target-ro`. `CARGO_TARGET_DIR=<impl>/rust/target-ro`, and the release
executable is built with `--features zax-app/bundled,zax-app/wgpu`.

## Checklist

| # | Step | Status |
|---|---|---|
| 1 | The gate: why the `wgpu` tests skip, fixed or proven; the map against `gl/`; the test plan | done — `f46feea` |
| 2 | Curvature (AMD-043, AMD-046) | done — `9ffb3b7` |
| 3 | The voxel grid and the pixel shadows | done — `6872645` |
| 4a | The anti-aliasing levels and AMD-058's centroid interpolation | done — this commit |
| 4b | The screen-pass architecture, the frame order of REQ-SCN-080, and the Depth occlusion | todo |
| 4c | The reflections of REQ-SCN-084 and the floor mask | todo |
| 4d | The MSVO estimator and its runtime fallback | todo |
| 5 | The parity measurements and the costs | todo |
| 6 | Closing: notes, `PERFORMANCE-HISTORY.md`, the Q-104 closing section, fmt, clippy, full tests, release build | todo |

The hash of a step is written in the commit of the step that follows (no amend, protocol of this
lot).

**The brief's step 4 is split into four**, and the reason is this lot's own protocol: every step
must be a commit that builds and whose tests pass. The brief's step 4 is "the screen passes: Depth
AO, MSVO, SSR, their targets and lifecycle, AA resolve" — which against the code is four
independent deliverables and about 1 400 lines of the OpenGL backend (`gl/screen_effects.rs` 789,
`gl/msvo.rs` 472, plus eleven GLSL sources). They cannot be one green commit, and three of the
four can each be one:

- the **anti-aliasing levels** (the brief's point 5) turned out to be a subject of its own and the
  smallest: the MSAA machinery was already there, and what was missing was AMD-058's centroid
  interpolation. It is also a *prerequisite* of the other three, because REQ-SCN-080's last bullet
  has the screen passes read the **resolved** colour and depth — so the AA behaviour has to be
  settled and measured before a pass is laid on top of it. Hence 4a rather than last.
- the **module, the frame order and the Depth estimator** (4b) is the architecture: the lazy
  lifecycle, the targets, the blend states, the composite, `REQ-SCN-080`'s order. The Depth
  estimator rides with it because an architecture with no pass over it is observable only as an
  allocation — the same argument RU-6 made for its own steps 2 and 3.
- the **reflections** (4c) add a pass of a different shape: a geometric mask drawn with a scene
  pipeline variant, and a trace that reads the frame's colour.
- the **MSVO estimator** (4d) is RU-6's port: a four-level pyramid, five programs, twenty passes
  and a runtime fallback.

Nothing of the brief is dropped; what moves is which commit carries it.

## Notes

### State of the working copy at the start

`git status` showed exactly what RU-6's notes predict and nothing else: the three
`crates/zax-render/tests/*_tmp.rs` scratch probes RU-5, RO-2 and RU-6 deliberately left
uncommitted, and the untracked `tests/` directory — the **.NET** test tree's `bin/`, `obj/` and
`TestResults/` build output left behind by an earlier checkout of `dotnetupdated`. No tracked file
was modified, so there was nothing to review, keep or discard. A fourth probe,
`ru7_adapters_tmp.rs`, is added by this step and is never staged either.

### The gate — Q-104's "it cannot be verified" premise is **false on this machine**

The question the step asks is "what does the adapter request return for Vulkan and for DX12, with
which backend flags, in a headless test process". The answer, measured by
`HeadlessGpu::new` on each API with the backend mask and the instance flags the backend itself
uses (`ru7_adapters_tmp.rs`):

| API | Result | `AdapterInfo` |
|---|---|---|
| **Vulkan** | **OK** | `NVIDIA GeForce RTX 3050`, `DiscreteGpu`, driver `NVIDIA` 581.08 |
| **DirectX 12** | **OK** | `NVIDIA GeForce RTX 3050`, `DiscreteGpu`, driver 32.0.15.8108 |

Both adapters answer. The gate is open, and point 2 of the brief applies in full.

Worth recording because it looks like the opposite at first: `HKLM\SOFTWARE\Khronos\Vulkan\Drivers`,
the registry key the Vulkan loader has historically read its ICDs from, **does not exist** on this
installation (only `ImplicitLayers` does, with six overlay layers), and there is no `nv-vk64.json`
or `nvoglv64.dll` in `System32` — only `nv-vk32.json`, `nv-vk64.json` and `nv-vksc64.json` inside
the driver store. That is not a missing driver: since Vulkan 1.2 the loader also enumerates ICDs
through the PnP device registry of the display adapter, which is how this one is found. **A
registry probe is not evidence; the adapter request is.**

### What `render_wgpu_tests.rs` actually did here — it did not skip, it **failed**

Run unchanged on this machine, both tests ran to the end and failed:

```
vulkan heavy    93.125 % of the pixels within 2 levels      (threshold 99.5 %)
dx12   heavy    93.125 % of the pixels within 2 levels
```

Identical to the thousandth on two different APIs, which already says the cause is not the driver.
It is the **test**, and the two columns below are the proof: rendered with the test's own options
and compared against `goldens/render-synthetic`, the **OpenGL** backend fails the same scene by
the same margin.

| scene | GL, the test's options | Vulkan, the test's options | GL, `render_tests.rs`' options | Vulkan, the same |
|---|---|---|---|---|
| `heavy` | 93.203 % | 93.125 % | 99.684 % | **99.571 %** |
| `backdrop` | 58.486 % | 99.786 % | 100.000 % | **99.999 %** |
| `framing` | 99.059 % | 99.822 % | 99.987 % | **99.971 %** |

`render_tests.rs`, the OpenGL twin of this test, passes `--anti-alias 0` and
`--no-backdrop-lighting`, and says in a comment of eight lines why: the goldens were captured
before AMD-052 `REQ-PERS-089` took the anti-aliasing default to 1 and before AMD-049
`REQ-SCN-075` took the backdrop to the almost-matt model. `render_wgpu_tests.rs` passes
**neither**, so it was comparing a MSAA 4x capture of the new backdrop model against a no-AA
capture of the old one. No backend can match that.

The fault went unnoticed for exactly the reason Q-104 §2 gives for the gap itself: the test skips
where no adapter answers, so on the machines of RU-4a, RU-5 and RU-6 it never got as far as the
comparison. **The skip hid a defect in the skip's own test.** Fixed by passing the two options
the OpenGL twin passes, with the measurement above written into the comment. Both tests now pass,
worst scene `heavy` at 99.571 % (Vulkan) and 99.571 % (DX12).

This is the first thing the lot establishes and it is worth stating plainly: the `wgpu` backend's
golden check had **never once been executed** before this step.

### A gap outside this lot's list, found by the same measurement

The `backdrop` column above is not just a test artefact. Rendered with the backdrop lighting
**on** — the desktop default since AMD-049 — the two backends disagree:

```
gl/backdrop.png vs vk/backdrop.png, anti-aliasing off:  58.930 % of pixels within 2 levels
```

`src/shaders/backdrop.frag` and the `backdrop` axis of the OpenGL `ProgramKey` have **no WGSL
counterpart at all**: `grep -rn "REQ-SCN-075\|backdrop" src/wgpu_backend/` finds one unrelated
comment. So AMD-049's almost-matt reflection model is a **third** effect the `wgpu` backend lacks,
beside the curvature and the Enhanced style that Q-104 records — and unlike those two it is on by
default on the desktop, which is where this backend ships.

It is **not** in this lot's list ("Implement only what is listed here"), so this step records it
rather than fixing it. It is the sharpest available answer to Q-104's question 3 (is the parity of
the second backend a requirement of every lot, or a catch-up lot of its own?), and the closing
section of step 6 will carry it there. No Q ticket: nothing is ambiguous, and Q-104 is already the
ticket this belongs to.

### The map — what the `wgpu` backend lacks against `gl/`

The OpenGL scene program is selected by a six-axis `ProgramKey`
(`bend × centroid × voxel_grid × pixel_shadow × backdrop × floor_mask`, `gl/shaders.rs`), and the
screen-space work by a lazily built `ScreenEffects` module (`gl/screen_effects.rs`, `gl/msvo.rs`).
The `wgpu` scene pipeline is selected by `SceneKey { mode, cull, mirrored, samples, batched }` —
**no style axis whatever**.

| Piece of the OpenGL backend | `wgpu` today | This lot |
|---|---|---|
| `bend` axis + `shaders/bend.glsl` (REQ-SCN-067, 068, 074) | absent; `constants.bend` reaches `scene_pass` and nothing reads it | step 2 |
| the curvature of the **shadow** pass (`shadow.vert`, REQ-SCN-068) | absent | step 2 |
| the 32 × 32 subdivided backdrop box of REQ-SCN-068 | absent (no backdrop at all) | out of scope, see above |
| `centroid` axis (AMD-058 REQ-SCN-065) | absent; WGSL has `@interpolate(perspective, centroid)` (naga 29 `Sampling::Centroid`), so it is portable | step 4 (AA) |
| `voxel_grid` axis + `shaders/voxel_grid.glsl` (REQ-SCN-081) | absent | step 3 |
| `pixel_shadow` axis + `shaders/pixel_shadow.glsl` (REQ-SCN-082) | absent | step 3 |
| `floor_mask` axis + `shaders/floor_mask.frag` (REQ-SCN-084) | absent | step 4 |
| Depth AO: `ao.frag`, `ao_blur.frag`, `ao_composite.frag` (REQ-SCN-083) | absent | step 4 |
| MSVO AO: `msvo_linearize/downsample/occlude/blur/upsample.frag` (REQ-SCN-083, RU-6) | absent | step 4 |
| the `R16F` pyramid and its runtime fallback **(D)** to Depth, one diagnostic line | absent | step 4 |
| SSR: `ssr.frag`, `ssr_composite.frag` (REQ-SCN-084) | absent | step 4 |
| `shaders/screen_depth.glsl`, shared by every screen pass | absent | step 4 |
| the frame order of REQ-SCN-080 (resolve, then reflections, then occlusion) | absent | step 4 |
| the lifecycle: lazy build, release on return to Classic, one refusal per session | absent | step 4 |
| `backdrop` axis + `shaders/backdrop.frag` (AMD-049 REQ-SCN-075) | **absent** | **out of scope** — recorded above |
| anti-aliasing 0 / 1 / 2 clamped to the device | **present** (`clamp_anti_alias_samples`, `ensure_linear`, multisampled attachment with a resolve target) — to be verified against GL, and the screen passes must resolve as REQ-SCN-080 orders | step 4 |
| `video.anti_alias` reaching the statistics | present | — |

Twelve GLSL sources to port against seven WGSL ones that exist today
(`common`, `scene`, `shadow`, `sky`, `resolve`, `textures_bound`, `textures_bindless`).

#### Two things the architecture forces, noted before any code is written

1. **The bend must be a module variant, not a uniform branch.** The brief's check is that "with
   the Classic style and no curvature, the `wgpu` frame does no new work (no new pass submitted, no
   new pipeline switched to)". The OpenGL backend gets that from the `ZAX_BEND` compile-time
   variant — at curvature 0 "there is no uniform tested per vertex" (`bend.glsl`'s own header).
   The `wgpu` renderer already composes its modules by string concatenation
   (`COMMON` + `SCENE` + one of the two texture chunks), so a second module built lazily with a
   `BEND` chunk prepended is the same shape, and the bend becomes an axis of `SceneKey`. A
   `if (frame.bend_on.x != 0.0)` in one module would cost the Classic vertex a test it does not
   pay today.
2. **`FRAME_BYTES` has to grow.** `frame_bytes` writes two matrices and five `vec4` = 208 bytes
   into a 256-byte block. The bend needs k, f, h, `on` and the pivot = 13 floats, which is four
   `vec4` with the pivot in a `w` slot: 272 bytes, so the block goes to 288. That is a `wgpu`-only
   constant; the OpenGL uniforms are untouched.

### The test plan

Three kinds of test, and the first two are what the brief calls the known-gap machinery's
replacement.

| # | What it checks | Where |
|---|---|---|
| 1 | **Parity, measured.** The RU-4a/RU-5/RU-6 synthetic scenes rendered through *both* backends and compared: curvature, voxel-grid close-up, pixel shadows, AO Depth, AO MSVO, SSR, and each AA level. Per-effect max channel difference and share of differing pixels. | `zax-harness/tests/parity_wgpu_tests.rs` (new) — beside `render_wgpu_tests.rs`, which is where an adapter is already required and skipped for |
| 2 | **The §2.4 invariants on the `wgpu` side**: the isolated plane at 1.00, the contact ratio `< 1` and `>= 0.10`, the five `ao.strength` floors — the same assertions `amd055_msvo_tests.rs` and `amd055_occlusion_tests.rs` make on the GL side, read off a `wgpu` capture | the same file |
| 3 | **The WGSL exists and compiles.** Module creation is where naga validates, so a test that builds every variant of every new module on a live device is the only thing that pins the WGSL. The known-gap test of `amd055_style_tests.rs` is rewritten into its opposite: the five effect names must now **be** in the WGSL sources | `amd055_style_tests.rs` (rewritten), plus a module-build test in the parity file |

Each parity test skips with a clear message where no adapter answers, the way `render_wgpu_tests.rs`
does — `HarnessError::Gpu` whose message starts with `no `, which is what `HeadlessError::Adapter`
formats.

What goes, per point 6 of the brief: `q_104_the_wgpu_backend_carries_nothing_of_the_enhanced_style_yet`,
the "the Enhanced visual style is not rendered by this graphics backend…" diagnostic line and
`style_reported` with it, and the `// Q-104` comments in `wgpu_backend/renderer.rs`,
`gl/shaders.rs`, `zax-app/src/graphics.rs` and `zax-harness/src/render/engine.rs`.

### The "before" figures (step 1)

Taken on this machine, in release, at `faced2a`, so that step 6 can show the OpenGL backend did
not move.

`cargo test --release -p zax-render`: **300 passed**, 0 failed, 1 ignored (RU-6's close).
`cargo test --release -p zax-harness --features wgpu --test render_wgpu_tests`: **2 passed** after
the fix of this step, 2 **failed** before it.

Post-LTO instruction counts (`cargo rustc -p zax-harness --release --bin zax-harness --
--emit=asm`, the method of RU-3a-fix through RU-6, default features so that the column is the
OpenGL path):

| Symbol | at `faced2a`, measured here | RU-6 recorded |
|---|---|---|
| `Ppu2C02::run_dots` | **2231** | 2231 |
| `DrawListBuilder::build` | **1095** | 1095 |
| `SceneRenderer::render_scene` | **3401** | 3401 |
| `Construction::segment` | **159** | 159 |

All four re-measured rather than inherited, and all four agree to the instruction, which is the
check that this lot starts where RU-6 ended. The counting script is written fresh each lot; this
one's is kept out of the tree with the other scratch probes, and the same script runs over both
columns of every comparison in this lot.

### Step 1 — what landed

| Piece | What changed |
|---|---|
| `zax-harness/tests/render_wgpu_tests.rs` | the two options the OpenGL twin passes, with the measurement that found the fault in the comment |
| `docs/progress/RU-7.md` | this file |

Nothing else. The gate is a finding, and the finding is that the premise of Q-104 §2 does not hold
here, so the lot continues at step 2.

### Step 2 — the curvature

#### WGSL has no preprocessor, and six axes to mirror

The one real design decision of this step, made before a line of shader was written. The OpenGL
backend's `ZAX_BEND` is a **compile-time** variant and `bend.glsl`'s own header says why: at
curvature 0 "there is no uniform tested per vertex", and no varying for the bent position. The
brief asks the same of this side ("with the Classic style and no curvature, the `wgpu` frame does
no new work"). A `if (frame.bend_on.x != 0.0)` in one module would fail that.

WGSL has no `#ifdef`, so three shapes were available:

1. a run-time branch on a frame field — rejected, it makes Classic pay;
2. one chunk per variant defining `VertexOut` and a few hook functions — works for one axis, but
   the map of step 1 lists **six** (`bend`, `centroid`, `voxel_grid`, `pixel_shadow`, `floor_mask`,
   and the backdrop that is out of scope), and a chunk that has to define the varyings struct for
   each combination is 2⁵ chunks;
3. a preprocessor.

So `wgpu_backend/preprocess.rs`: `//#ifdef`, `//#ifndef`, `//#if A || B`, `//#else`, `//#endif`,
nesting, 11 unit tests. The markers are **WGSL line comments**, so every source stays a valid
WGSL file that an editor and `naga` can still read whole, and the three condition forms are
exactly the three the GLSL sources use between them. A marker it cannot read is a fault in a
source of this crate, not input, so it is reported and turned into a `const_assert false` carrying
the bad line — module creation then fails with it in the message, rather than silently dropping
code. What the expander deliberately does **not** do is substitute anything inside a line:
everything the two APIs disagree about (0..1 clip depth, top-left texture origin, the binding
model) stays hand-written, which is what the brief asks.

#### What landed

| Piece | What landed |
|---|---|
| `wgpu_backend/preprocess.rs` (new) | the variant mechanism above, with its 11 unit tests |
| `wgpu_backend/shaders/bend.wgsl` (new) | `zax_wrap` (E of REQ-SCN-067 step 2), `zax_bend` (B, the three axes in their order, the Jacobians composed), `zax_bend_normal` (the cofactor transport of REQ-SCN-068) — the twin of `shaders/bend.glsl`, read through four accessor functions so the chunk serves the scene *and* the shadow module, whose frame blocks are different structs |
| `shaders/scene.wgsl` | `Frame` gains the four curvature `vec4`; `VertexOut` gains `bent` **in the variant only**; `vs_main` places the vertex and transports the normal; `fs_main` starts the shadow lookup from the bent position and keeps judging REQ-SCN-036 on the unbent one |
| `shaders/shadow.wgsl` | `ShadowFrame` gains the same four, and the pass bends (REQ-SCN-068) |
| `wgpu_backend/renderer.rs` | `ShaderVariant` (one named field per axis, `PLAIN` the module of before RU-7); `build_module`; `scene_modules` / `shadow_modules` / `batched_modules` keyed by variant and built on demand; `SceneKey.variant`; the shadow pipeline key; `select_bend_variant`; `push_bend`; `FRAME_BYTES` 256 → 288 and `SHADOW_FRAME_BYTES` 80 → 144; the subdivided box and rectangle of REQ-SCN-068/074 with the widescreen quad's cell count; `hud_variant`; **the `// Q-104` curvature comment is gone** |
| `zax-render/tests/bend_tests.rs` | `q_104_the_wgpu_backend_carries_nothing_of_the_curvature_yet` → `q_104_the_wgpu_backend_carries_the_curvature`, its own opposite, as its message instructed |
| `zax-harness/tests/parity_wgpu_tests.rs` (new) | the parity file of the test plan, five tests |

`Batching` lost its `module` field — the batched module is now one per variant in the renderer,
beside the other two.

#### The things the port had to get right that a text substitution would not

1. **`mat3x3<f32>(a, b, c)` takes three columns**, and GLSL's `mat3(...)` takes three columns too,
   so each of the three Jacobian factors is written with the same nine numbers in the same nine
   places. Had either been row-major the normals would be wrong only where two axes bend at once
   — which is why `req_scn_068_the_three_axes_together_agree_between_the_backends` exists.
2. **The arc pivot rides in `bend_k.w`.** The GLSL has it as a `float` uniform of its own; the
   frame block wants `vec4`s, and 13 floats fit in four of them with one `w` spare. A port that
   dropped it would pass the left-right test and fail the arc one, so there is a test per axis.
3. **`out` parameters do not exist in WGSL.** `zaxWrap` and `zaxBend` each return two things, so
   each returns a struct (`ZaxWrap`, `ZaxBend`) instead.
4. **`select(a, b, c)` is `c ? b : a`** — the operand order is the reverse of a ternary's, and it
   is used three times here (the depth bound's two arms and the degenerate-Jacobian fallback).
5. **The HUD never bends** (REQ-SCN-068). The OpenGL backend clears the bit in `draw_hud`; here
   `hud_variant` puts `ShaderVariant::PLAIN` back before the HUD runs, which also means the plain
   pipelines have to be prepared on a frame that bends everything else. Measured below: the HUD
   moves by **0 levels** under a 90° curvature on both backends.

#### The frame block grew, and a Classic frame pays 64 bytes of memcpy for it

`frame_bytes` wrote 208 bytes into a 256-byte block; the curvature needs four more `vec4`, so the
block is 288 and the shadow block 144. `push_bend` writes them on **every** frame, bending or not,
because the block has one layout — and at curvature 0 `SpaceBend::NONE` makes those four writes
sixteen zeros, which is what the buffer already held. That is the one new cost a Classic `wgpu`
frame carries from this step: 64 bytes added to a `write_buffer` it was already making, once a
frame, no branch. No new pass is submitted and no new pipeline is switched to, which is what the
brief's check names.

#### Parity, measured

`parity_wgpu_tests.rs`, release, this machine, 640 × 480, anti-aliasing off. Each row is the worst
of four scenes (`framing`, `shadow`, `heavy`, `hud`). Vulkan and DX12 returned **identical figures
to the fourth decimal** in every row, so they share one column.

| Configuration | max channel | pixels differing | within 2 levels |
|---|---|---|---|
| **Classic, the control** | 223 | 0.3577 % | **99.6423 %** |
| `--bend 90,0,0` | 250 | 0.1299 % | **99.8701 %** |
| `--bend 0,0,60,1` (the arc, centre moved) | 237 | 0.4062 % | **99.5938 %** |
| `--bend 45,30,20` (all three axes) | 254 | 0.1953 % | **99.8047 %** |

**The control is the figure to read the others against, and it is the point of having one.** The
two backends already disagreed on 0.36 % of `heavy` with a max channel difference of 223 levels
*before* this step — that is silhouette rasterisation on a frame of 1926 overlapping tile boxes,
where a pixel either takes a box's face or the one behind it. The curvature's rows sit on either
side of the control (0.13 % at 90°, 0.41 % on the arc) rather than above it, and the max channel
difference is the same 220-to-250 all along, including in Classic. So the residual is not the
curvature's: **the curvature adds no measurable disagreement at all.** On `framing` and `shadow`,
where silhouettes do not pile up, every configuration is at 99.97 % or better and the bent shadow
pass reads 99.9967 %.

Two figures carry the other half of the check — that the shader is not quietly compiling to
nothing:

- the curvature moves **62.24 %** of `framing` on OpenGL, **62.24 %** on Vulkan and **62.24 %** on
  DX12 (the test holds them to within 1 point of each other);
- the HUD moves by **0 levels** on both backends (REQ-SCN-068).

And read scene by scene, bent against flat, the two backends agree on how much they move to within
0.02 of a percentage point: `framing` 40.061 % (GL) against 40.059 % (Vulkan) of pixels still
within 2 levels, `heavy` 44.950 % against 44.938 %.

#### The OpenGL backend did not move

- `git diff --name-only faced2a` over `gl/`, `src/shaders/`, `renderer.rs`, `zax-engine3d/` and
  `zax-app/` is **empty**: not one file of the OpenGL path is touched by this step. The five
  changed files are the two WGSL sources, `wgpu_backend/renderer.rs`, `wgpu_backend/mod.rs` and
  the harness test.
- **The four instruction counts are unchanged**, re-measured with the same script:

| Symbol | `faced2a` | step 2 |
|---|---|---|
| `Ppu2C02::run_dots` | 2231 | **2231** |
| `DrawListBuilder::build` | 1095 | **1095** |
| `SceneRenderer::render_scene` | 3401 | **3401** |
| `Construction::segment` | 159 | **159** |

  The byte-identical synthetic-render check against a `faced2a` worktree is step 6's, where one
  throwaway build pays for the whole lot.

#### Tests

| Criterion | Test |
|---|---|
| the variant mechanism: the five markers, nesting, a malformed marker reported and kept | 11 unit tests in `preprocess.rs` |
| REQ-SCN-067/068 exist in WGSL, in both stages, behind the marker; the shadow lookup reads the bent position; REQ-SCN-036 still reads the unbent one | `bend_tests.rs` `q_104_the_wgpu_backend_carries_the_curvature` (rewritten) |
| the Classic control | `parity_wgpu_tests.rs` `ru_7_the_classic_frame_agrees_between_the_backends` |
| REQ-SCN-067/068, left-right at 90° | `req_scn_067_the_curvature_agrees_between_the_backends` |
| REQ-SCN-066's arc and its centre — the axis that reads the pivot | `req_scn_066_the_arc_and_its_centre_agree_between_the_backends` |
| REQ-SCN-068's composed Jacobians, three axes at once | `req_scn_068_the_three_axes_together_agree_between_the_backends` |
| the effect is real and the same size on each backend; the HUD is not bent | `req_scn_067_the_curvature_moves_as_much_of_the_frame_on_each_backend` |

`cargo test --release -p zax-render --features wgpu`: 33 suites, **312 passed, 0 failed**, 1
ignored (the committed suite, the four scratch probes moved out for the run and put back; RU-6
closed at 300 without `--features wgpu`, and the 11 new ones of `preprocess.rs` plus the `wgpu`
suites make up the difference).
`-p zax-harness --features wgpu --test parity_wgpu_tests`: 5 passed.
`--test render_wgpu_tests`: 2 passed. `cargo fmt --all --check` and
`cargo clippy --workspace --all-targets --features zax-app/wgpu -- -D warnings` clean.

#### What is left of the curvature for step 3 and after

REQ-SCN-074's 2D mode is **ported** — `picture_quad_bent`, the widescreen quad's cell count and
`--no-bend-2d` all reach the `wgpu` path — but it is not *measured* above, because no scene of the
synthetic set is a 2D-mode frame. The 2D rectangle is read in step 5, with a frame of its own.

### Step 3 — the voxel grid and the pixel shadows

The two shader effects of AMD-055, each its own axis of `ShaderVariant` because REQ-SCN-079
switches `spl.voxel_grid` and `spl.pixel_shadow` separately — "a player who turns the grid on does
not pay for the voxel snapping, nor the other way round", which is the OpenGL backend's own
reasoning and now this one's.

#### What landed

| Piece | What landed |
|---|---|
| `shaders/voxel_grid.wgsl` (new) | `zax_voxel_grid`, the twin of `voxel_grid.glsl`: the alignment factor, the axis weights, the groove and bright bands, the corner product |
| `shaders/pixel_shadow.wgsl` (new) | `zax_voxel_center` (the axis left alone, the `z` offset), `zax_place_linear`, `zax_voxel_shadow_point` (the deformation then the placement then the curvature, in REQ-E3D-406's order) |
| `shaders/scene.wgsl` | `Frame.style.x` the voxel cell; `local` and `local_normal` under `ZAX_VOXEL_GRID \|\| ZAX_PIXEL_SHADOW`; three flat placement rows and the deform tile under `ZAX_PIXEL_SHADOW`; the grid on the **linear** albedo; the snapped shadow origin |
| `shaders/common.wgsl` | the `Draw.modes2` comment: `z` now carries the voxel `z` offset |
| `wgpu_backend/renderer.rs` | two more fields on `ShaderVariant` with their defines and chunks; `ShaderVariant::shadow()`; `select_bend_variant` grown into the twin of `select_style_variants` too; the voxel cell in `frame_bytes`; the per-run `z` offset in `push_draw`; `storage(3)` visible to both stages |
| `wgpu_backend/resources.rs` | `MeshHandle.build_depth`, as `GpuMesh` carries it |
| `zax-render/tests/amd055_style_tests.rs` | the Q-104 known-gap test is now **two-sided**: the two shader effects must be in the WGSL, the two screen passes must not be yet |
| `zax-harness/tests/parity_wgpu_tests.rs` | four more tests |

#### Four places where the WGSL could not be the GLSL

1. **The grid multiplies the *linear* albedo.** `scene.frag` keeps `albedoLinear` throughout and
   the OpenGL indexed path reads an already-linear palette; `scene.wgsl` kept the albedo in sRGB
   until the lighting line and linearised there (its equivalent of the `ZAX_SRGB_PALETTE`
   variant). REQ-SCN-081 modulates the albedo "before any lighting calculation", and the
   reference does it in linear space — so the fragment now computes `albedo_linear` first and the
   grid multiplies that. Applied to the sRGB value instead, the grooves would have come out the
   wrong depth.
2. **The `z` offset rides in a spare integer slot.** The GLSL has `uVoxelZOffset` as a uniform of
   its own, set per run. The `wgpu` per-run block had two unused `i32` slots, so the offset goes
   into `modes2.z` as its bits and `pixel_shadow.wgsl` reads it back with `bitcast<f32>`. The
   exact value `style::voxel_z_offset` returns, no rule duplicated in the shader, and the block
   does not grow by 16 bytes for **every** draw of **every** variant (on `heavy`, 1926 runs).
3. **`MeshHandle` had to grow a `build_depth`.** It is where that offset comes from, and the
   OpenGL `GpuMesh` already carries it for the same reason — so the draw path reads it from the
   handle it has. The depth itself and not the `D/2` the shader wants: the handle is a key of the
   draw list (`Eq + Hash`) and a float is neither.
4. **The deformations had to become fragment-visible.** REQ-SCN-082 point 3 applies the animated
   deformation to the snapped voxel centre, and the snapping is a *fragment* computation — so
   `storage(3)` went from `VERTEX` to `VERTEX_FRAGMENT`. The OpenGL path's `deformation.glsl` is a
   chunk of both stages already, so this is the faithful shape; it is a static layout property, so
   no Classic frame does any new work for it. The **placements** of binding 1 stayed vertex-only:
   only the linear part of one instance's is wanted, and nine floats of flat varyings in the
   pixel-shadow variant are cheaper than making the whole buffer visible — which is the same
   choice, and the same nine floats, the GLSL makes.

#### `fwidth` is not `fwidth`, and the measurement that found it

The one real bug this step turned up, and the parity test is what caught it. With a plain
`fwidth(d)` in the grid — a literal transcription of the GLSL — the figures came out:

| | `framing` | `framing` + curvature |
|---|---|---|
| Vulkan | 99.9834 % | 99.9795 % |
| **DX12** | **99.8675 %** | **99.0124 %** |

DX12 alone, and worse the more the geometry was foreshortened. The cause is that the precision of
a plain `fwidth` is implementation-chosen in all three languages: naga emits SPIR-V `OpFwidth` for
Vulkan and HLSL `fwidth` for DirectX 12, and on this NVIDIA driver the second is quad-**coarse**.
The grid uses the derivative only to set the smoothing width of the 3.5 % band, so a coarse
derivative widens every groove edge by about a pixel — many pixels off by a little, concentrated
on the grooves, and the max channel difference large wherever an edge moved a whole step.

Asking for the fine derivative by name (`fwidthFine`) takes **both** backends to 99.983 %, which
also says what the OpenGL driver's GLSL `fwidth` is. REQ-SCN-081 fixes the smoothing only as
"about one pixel", so naming the variant changes nothing the requirement states. The finding is
written out in `voxel_grid.wgsl`'s header with both columns.

Two things worth drawing from it. First, **the DX12 column earns its place**: Vulkan alone would
have passed this step and shipped the defect to every DirectX 12 player. Second, this is precisely
the class of fault Q-104 §2 said could not be found without a device — and it could not have been
found by reading either shader.

#### Parity, measured

Same method as step 2 — worst of four scenes, release, 640 × 480, anti-aliasing off. Vulkan and
DX12 agree to the fourth decimal in every row once the derivative is named.

| Configuration | max channel | pixels differing | within 2 levels |
|---|---|---|---|
| Classic, the control | 223 | 0.3577 % | 99.6423 % |
| `spl.voxel_grid` = 3 | 229 | 0.3597 % | **99.6423 %** |
| `spl.pixel_shadow` = 1 | 223 | 0.3577 % | **99.6423 %** |
| both, with `--bend 60,0,0` | 255 | 0.1689 % | **99.8324 %** |

**All three sit on the control, to the fourth decimal.** The two effects add no disagreement that
the flat Classic frame did not already have, and the combined row with the curvature is *better*
than the control for the reason step 2 gave — bent geometry pulls silhouettes apart.

That each effect does something, and the same something:

| Effect | scene | OpenGL moves | Vulkan | DX12 |
|---|---|---|---|---|
| `spl.voxel_grid` = 3 | `framing` | 32.2497 % | 32.2565 % | 32.2565 % |
| `spl.pixel_shadow` = 1 | `shadow` | 0.0091 % | 0.0091 % | 0.0091 % |

And the HUD moves by **0 levels** under either effect, on both backends (REQ-SCN-081,
REQ-SCN-082).

#### The pixel shadows are weakly exercised by this fixture set, and that is stated

0.0091 % of `shadow` is about **28 pixels** of 307 200, and the effect moves **nothing at all** of
`framing`, which has no cast shadow for it to move. That is the synthetic set's doing and not the
port's — REQ-SCN-082 moves the point a shadow is *read* at, and these scenes' shadows cover a
sliver of the frame. So the honest reading is that this step's evidence for REQ-SCN-082 is the
agreement figure plus the OpenGL unit tests, not the "it moves something" figure; the test says so
in its own doc comment, with the number. A fixture with a large cast shadow is what would
strengthen it, and it is listed for R0 at the end of this file.

#### The OpenGL backend still did not move

`git diff --name-only faced2a` over `gl/`, `src/shaders/`, `renderer.rs`, `zax-engine3d/` and
`zax-app/` is **still empty**, and the four instruction counts are still 2231 / 1095 / 3401 / 159.

#### Tests

| Criterion | Test |
|---|---|
| REQ-SCN-081's five constants and REQ-SCN-082's two functions are in the WGSL, each behind its own marker; **and nothing of REQ-SCN-083 or REQ-SCN-084 is yet** | `amd055_style_tests.rs` `q_104_the_wgpu_backend_carries_the_two_shader_effects_and_not_yet_the_two_passes` (rewritten) |
| REQ-SCN-081 on its own | `parity_wgpu_tests.rs` `req_scn_081_the_voxel_grid_agrees_between_the_backends` |
| REQ-SCN-082 on its own | `req_scn_082_the_pixel_shadows_agree_between_the_backends` |
| the two together **and** under curvature — the ordering of REQ-SCN-082 point 3 with REQ-SCN-068 | `req_scn_082_the_two_effects_and_the_curvature_agree_between_the_backends` |
| each effect moves as much of the frame on each backend, and neither touches the HUD | `req_scn_079_each_shader_effect_moves_as_much_of_the_frame_on_each_backend` |

`cargo test --release -p zax-render --features wgpu`: 33 suites, **312 passed, 0 failed**, 1
ignored. `-p zax-harness --features wgpu`: 13 suites, **60 passed, 0 failed** (9 of them this
file's). `cargo fmt --all --check` and `cargo clippy --workspace --all-targets --features
zax-app/wgpu -- -D warnings` clean.

#### A bug in this file's own tests, fixed

The nine parity tests run in parallel and name their temporary folders after the label, the
backend and the process — so `req_scn_081`'s `voxel-grid` folder and the one
`req_scn_079` renders under the same label were **the same path**, and whichever finished first
deleted the other's captures. Found by the suite failing only when run without
`--test-threads=1`. The folder name now carries a counter as well.

### Step 4a — the anti-aliasing levels and the centroid

Point 5 of the brief: "check the `wgpu` backend honours them as the OpenGL one does, clamped to
what the adapter supports, with the screen passes resolving correctly under MSAA as in the OpenGL
backend." Checked, and one thing was missing.

#### What was already right

The MSAA machinery of the `wgpu` backend predates this lot and is correct:
`geometry::clamp_anti_alias_samples` is the shared helper both backends call, `ensure_linear`
allocates a multisampled colour target with the resolved one as its `resolve_target`, and
`max_samples` clamps to what the adapter reports. Measured, every backend grants the same count:

| `video.anti_alias` | means | OpenGL | Vulkan | DX12 |
|---|---|---|---|---|
| 0 | no anti-aliasing | 1 | **1** | **1** |
| 1 | MSAA 4x | 4 | **4** | **4** |
| 2 | MSAA 8x | 8 | **8** | **8** |

#### What was missing: AMD-058's centroid interpolation

`gl/shaders.rs` has a `centroid` axis of its own (REQ-SCN-065, AMD-058): under MSAA, `vUv` is
interpolated at the centroid of the samples the fragment covers. The defect AMD-058 §1 measured is
that a fragment on the edge of a wall whose pixel *centre* falls outside the triangle gets an
**extrapolated** `uv`, `floor()` then lands one texel past the face, and the atlas returns a
neighbour's texel — a light line one pixel wide along every joint. The `wgpu` backend had no such
axis, so it had the defect at both multisampled levels.

WGSL can express it: `@interpolate(perspective, centroid)` (naga 29 `Sampling::Centroid`). So
`ShaderVariant` gained a fourth field and `scene.wgsl` a `//#ifdef ZAX_CENTROID` on that one
varying. The trigger is the **setting** and not the granted count, for the reason the OpenGL side
states: a buffer that falls back to one sample interpolates a centroid at the pixel centre anyway.

What it bought, worst scene against the OpenGL reference:

| | before | after |
|---|---|---|
| `framing`, MSAA 4x | 99.118 % | **99.528 %** |
| `heavy`, MSAA 4x | 99.023 % | **99.108 %** |
| `hud`, MSAA 4x | 99.990 % | **99.997 %** |
| the other eight scenes | ≈ 99.855 % | ≈ **99.919 %** |

Every scene improved, and `framing` crossed the 99.5 % bar.

**The HUD keeps the centroid.** `hud_variant` clears three bits and not this one, because
`draw_hud` on the OpenGL side clears exactly the curvature, the grid and the pixel shadows and
leaves the interpolation alone — AMD-058 is about reading the atlas at the right texel, which the
HUD's own tiles need as much as the scene's. That is why `hud_variant` kept its parameter in step
2 instead of being a constant.

#### The residual is MSAA sample positions, and the proof is in the measurement

`heavy` stays at 99.108 % (4x) and 98.949 % (8x). That is not a port fault, and the figure that
settles it is this:

| MSAA 4x, `heavy` | within 2 levels | max channel |
|---|---|---|
| **Vulkan against DX12** | **100.0000 %** | **0** |
| Vulkan against OpenGL | 99.108 % | 223 |
| DX12 against OpenGL | 99.108 % | 223 |

The two `wgpu` APIs are **byte-identical** — at every level, on every scene — and both differ from
OpenGL by exactly the same amount. MSAA sample positions are implementation-defined in all three
APIs: none of OpenGL, Vulkan or DirectX 12 fixes where inside a pixel the 4 or 8 samples sit, so
two correct implementations resolve an edge pixel to different colours, and `heavy` is 1926
overlapping boxes, i.e. silhouette edge everywhere. There is nothing in the renderer to choose
between the two patterns, and more samples let them diverge further — which is why 8x reads lower
than 4x.

So the multisampled rows are held to **98.5 %** against OpenGL (just under the worse of the two
measured values) and to **99.9 %** against each other. The second is the stronger assertion and
the one a port can actually break — it is what caught `fwidth` in step 3, and it now reads
100.0000 % with max channel 0 at all three levels.

#### What landed

| Piece | What changed |
|---|---|
| `shaders/scene.wgsl` | `uv` under `//#ifdef ZAX_CENTROID`, with REQ-SCN-065's defect written out |
| `wgpu_backend/renderer.rs` | `ShaderVariant.centroid`, its define and its label; the trigger in `select_bend_variant`; `hud_variant` keeps it |
| `zax-harness/tests/parity_wgpu_tests.rs` | three tests: the granted counts, the three levels' pictures, and the two `wgpu` backends against each other |

#### The OpenGL backend still did not move

`git diff --name-only faced2a` over the OpenGL path is **still empty**; the four instruction counts
are still 2231 / 1095 / 3401 / 159. `zax-render --test amd058_centroid_tests`: 10 passed, which is
the OpenGL side of the same requirement unchanged. `render_tests` (the OpenGL goldens) and
`render_wgpu_tests`: 3 and 2 passed.

`parity_wgpu_tests`: **12 passed**. `cargo fmt --all --check` clean.

#### A clippy failure went into `d678e46`, and the fix is the next commit

Worth recording rather than hiding, because the protocol of this lot says a step's commit is one
whose tests pass. The verification chain used `grep -c` to count clippy's complaints, and `grep -c`
**exits 0 when it finds matches** — so a `&&` chain read "2 problems" as success and committed.
`ShaderVariant`'s fourth `bool` had tripped `clippy::struct_excessive_bools`.

The fix is the `#[allow]` the OpenGL `ProgramKey` already carries for the same reason (it has six
such fields: "an enum per axis would say less, not more"), and it rides in step 4b's commit rather
than an amend, which this lot's protocol forbids. The check itself now tests the count against
zero instead of trusting an exit status.

### Where the lot stands, and what step 4b has to solve

Steps 1, 2, 3 and 4a are landed and green. What follows is the state 4b starts from, written out
because this lot's protocol says the session may be cut and resumed by someone else, and because
the design work below is already done and should not be repeated.

#### The three effects still to port, against `gl/`

| Piece | OpenGL source | Lines |
|---|---|---|
| the module, its lifecycle, its targets, the two composites | `gl/screen_effects.rs` | 789 |
| the shared depth helpers | `shaders/screen_depth.glsl` | 122 |
| the Depth estimator (REQ-SCN-083) | `ao.frag`, `ao_blur.frag`, `ao_composite.frag` | 290 |
| the reflections (REQ-SCN-084) | `ssr.frag`, `ssr_composite.frag`, `floor_mask.frag` | 400 |
| the MSVO estimator (REQ-SCN-083 *Mode MSVO*) | `gl/msvo.rs` + five `msvo_*.frag` | 472 + 640 |

#### What is settled, and should be built this way

1. **The stored depth values are already the same on both backends**, so `zaxViewDepth` ports
   unchanged. `zax_clip` maps GL's `-1..1` clip depth to wgpu's `0..1` as `(z + w) * 0.5`
   precisely so that "the same depth values end up in the depth buffer" (`common.wgsl`), and
   `Depth24Plus` matches `DEPTH_COMPONENT24`. So the two conventions `screen_depth.glsl` undoes —
   the Direct3D-style projection to `[0, 1]` and then OpenGL's NDC-to-window mapping — are undone
   by the same two lines here. **This is the one place where the obvious worry is unfounded, and
   it was checked rather than assumed.**

2. **The `uv`-to-NDC mapping is *not* the same, and this is the real coordinate trap.** A wgpu
   framebuffer's origin is top-left and its NDC `y` points up, so for a full-screen pass
   `ndc = vec2(uv.x * 2 - 1, 1 - uv.y * 2)` — the `y` **negated** against the GLSL's
   `ndc = uv * 2 - 1`. `zaxViewSample` must use that, and `zaxProjectView` must invert it
   (`uv = vec2(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5)`). Sampling the depth and colour textures
   themselves needs no flip: they were rendered in wgpu's own convention, which is also what the
   existing resolve pass relies on (`textureLoad(linear, position.xy)`).

   A port that left `ndc = uv * 2 - 1` would **not** fail obviously: the occlusion is close to
   invariant under a mirror of view space, because `zaxViewNormal` fixes the normal's sign from
   `dot(normal, p)` rather than from the winding. It would show up in the reflections, whose rays
   march in a direction that has to agree with the eye.

3. **The formats carry over as they are.** `R8` for the occlusion and the floor mask (`R8Unorm`),
   `RGBA16F` for the trace (`Rgba16Float`, which `LINEAR_FORMAT` already is), `R16F` for the MSVO
   pyramid (`R16Float`). None is a new capability on either `wgpu` backend, which is the same
   finding RU-5 and RU-6 recorded for OpenGL and GLES.

4. **The composites are blend states, not copies.** `ao_composite.frag` relies on
   `glBlendFunc(GL_ZERO, GL_SRC_COLOR)` — `dst = dst * src` — so one triangle multiplies the
   linear buffer in place with no second target. In wgpu that is
   `BlendComponent { src_factor: Zero, dst_factor: Src, operation: Add }`, and the reflections'
   `glBlendFunc(GL_ONE, GL_ONE_MINUS_SRC_ALPHA)` is `BlendComponent { src_factor: One,
   dst_factor: OneMinusSrcAlpha, operation: Add }`. Both are expressible, so neither composite
   needs a copy of the scene.

5. **The frame order of REQ-SCN-080 is the reflections first, then the occlusion**, and both after
   the resolve and before the transparent objects and the HUD (`run_screen_effects`). Its stated
   consequence has to be carried over too: with anti-aliasing *and* a screen effect, the
   transparent objects and the HUD are **not** multisampled, because the frame draws into the
   single-sample target from the resolve onwards.

#### The one problem that has no OpenGL counterpart: resolving the depth

This is what 4b has to solve first, and it is worth writing down because it is not visible from
either shader.

REQ-SCN-080's last bullet has the passes read the **resolved** colour and depth. On OpenGL that is
one call — `glBlitFramebuffer` with `COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT`
(`LinearColorBuffer::resolve_samples`). On `wgpu` the colour side is free (the scene pass already
names the single-sample target as its `resolve_target`), but **there is no depth resolve**:
`RenderPassDepthStencilAttachment` has no `resolve_target` field, and there is no
texture-to-texture blit for a depth format.

So a multisampled frame that runs a screen pass needs a pass of its own that reads
`texture_depth_multisampled_2d` and writes a single-sample depth — `textureLoad(depth, coord, 0)`
into `@builtin(frag_depth)`, a depth-only pipeline with `depth_compare: Always`. Sample 0 is the
faithful choice: it is what the OpenGL blit gives, and `screen_effects.rs` already says so where
it matters ("the depth the pass reads is a **resolved** multisampled depth — one sample's, not the
fragment centre's"), which is also why the floor mask carries its `polygon_offset(-1.0, -16.0)`
bias. That bias has a wgpu equivalent in `DepthBiasState { constant: -1, slope_scale: -16.0, .. }`.

With `video.anti_alias` = 0 — the handheld default and the one the goldens are taken at — no such
pass is needed, and it must not be submitted: the depth texture is already single-sample.

#### What is left, in order

| Step | What |
|---|---|
| 4b | `wgpu_backend/screen_effects.rs`, `screen_depth.wgsl`, `ao.wgsl`, `ao_blur.wgsl`, `ao_composite.wgsl`, the depth-resolve pass, the frame order, the lazy lifecycle and its statistics |
| 4c | `ssr.wgsl`, `ssr_composite.wgsl`, `floor_mask.wgsl`, the mask's scene-pipeline variant (`ZAX_FLOOR_MASK`) and its depth attachment |
| 4d | `msvo.wgsl` x5, the `R16F` pyramid, the runtime fallback **(D)** and its one diagnostic line |
| — | then the diagnostic line "the Enhanced visual style is not rendered by this graphics backend…", `style_reported`, and the remaining `// Q-104` comments in `wgpu_backend/renderer.rs`, `zax-app/src/graphics.rs` and `zax-harness/src/render/engine.rs` all go, and the second list of `amd055_style_tests.rs`'s two-sided test empties |
| 5 | the parity table and the GPU costs of each effect at 1080p, beside RU-5's and RU-6's OpenGL figures |
| 6 | the close: `PERFORMANCE-HISTORY.md`, the Q-104 closing section answering its questions 1, 3 and 4, the full suite, the release build with `zax-app/bundled,zax-app/wgpu` |

#### What the lot has established so far, for Q-104's closing section

Three findings the closing section should carry, all of them already measured:

1. **Q-104 §2's premise is false on hardware that has a driver**, and the skip it rested on was
   hiding a defect in its own test (step 1).
2. **The parity is reachable and it is cheap**: the curvature, the voxel grid and the pixel
   shadows all agree with OpenGL to within the Classic control's own figure, and the two `wgpu`
   APIs are byte-identical to each other at every anti-aliasing level.
3. **A device finds what reading cannot** — the `fwidth` of step 3 was a real defect on DirectX 12
   only, invisible in both shaders, and it is the strongest available answer to Q-104's question 3
   (whether parity is a per-lot requirement or a catch-up lot): a backend that is never executed
   accumulates faults that no review finds. The backdrop model of AMD-049 (step 1's second
   finding) is the other half of that answer.
