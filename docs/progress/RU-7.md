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
| 4a | The anti-aliasing levels and AMD-058's centroid interpolation | done — `d678e46` |
| 4b | The screen-pass architecture, the frame order of REQ-SCN-080, and the Depth occlusion | done — `46f3174` (its clippy fix and its design: `9ef1447`) |
| 4c | The reflections of REQ-SCN-084 and the floor mask | done — this commit |
| 4d | The MSVO estimator and its runtime fallback | todo |
| 4e | The backdrop lighting of AMD-049 (REQ-SCN-075), added to the lot by the owner | todo |
| 5 | The parity measurements and the costs | todo |
| 6 | Closing: notes, `PERFORMANCE-HISTORY.md`, the Q-104 closing section, fmt, clippy, full tests, release build | todo |

The hash of a step is written in the commit of the step that follows (no amend, protocol of this
lot).

**Step 4e is an addition the owner decided between the two sessions of this lot**, and it is in
the checklist rather than in a lot of its own because it is the same subject: step 1 measured that
`src/shaders/backdrop.frag` and the `backdrop` axis of the OpenGL `ProgramKey` have **no WGSL
counterpart at all**, and that with the backdrop lighting on — the desktop default since AMD-049 —
the two backends agree on only 58.9 % of the `backdrop` scene. Step 1 recorded it as out of scope
("Implement only what is listed here"); the owner put it in. It goes **before** the parity
measurements, so that step 5's table is the whole picture rather than a picture with a known hole
in it, and the remaining steps are renumbered accordingly (the brief's step 5 is now 5 of 10
rather than 5 of 9).

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

### Step 4b — the screen-pass architecture, the frame order, and the Depth occlusion

The architecture the three effects live in, and the first of them. Everything the previous
session's design section settled was built as it stands there; what follows is what the code says
and what the device said back.

#### What landed

| Piece | What landed |
|---|---|
| `wgpu_backend/screen_effects.rs` (new, 700 lines) | the twin of `gl/screen_effects.rs`: the lazy lifecycle, the `R8Unorm` targets of the raw pass and its blur, the one bind-group layout every screen pass takes, the uniform blocks, the three pipelines of REQ-SCN-083 and the depth resolve |
| `shaders/screen_depth.wgsl` (new) | the twin of `shaders/screen_depth.glsl`, plus the full-screen triangle of `resolve.vert` and the bindings — `zax_view_depth`, `zax_view_sample`, `zax_view_normal`, `zax_project_view`, `zax_dither`, `zax_neighbour` |
| `shaders/ao.wgsl`, `ao_blur.wgsl`, `ao_composite.wgsl` (new) | the hemisphere estimator of REQ-SCN-083, its depth-aware blur, and the `1 - O` factor the multiply blend applies in place |
| `shaders/depth_resolve.wgsl` (new) | the pass with no OpenGL counterpart, below |
| `wgpu_backend/renderer.rs` | `screen_effects`, `linear_generation`, `style_modules`; `select_screen_effects` / `prepare_screen_effects`; `ensure_linear` grows the readable-depth flag and the single-sample depth; `scene_pass` splits into one or two render passes; `Buckets`, `BucketScene`, `draw_buckets` and `run_screen_effects`; `shader_module` factored out of `build_module`; the Q-104 diagnostic line narrowed to what is still missing |
| `renderer.rs` (OpenGL) | one word: `projection_terms` is `pub(crate)`. See below — it is the whole of this lot's diff in that file, and the four instruction counts are unchanged |
| `zax-goldens/src/lib.rs` | `RenderScene` reads the four keys AMD-055 writes, as `Option`s that are `None` on every capture at the default settings, so that a test can compare the passes and the bytes of REQ-SCN-080 on each backend |
| `zax-render/tests/amd055_style_tests.rs` | the two-sided Q-104 test moves one effect across: the occlusion must now **be** in the WGSL, the reflections and the MSVO estimator must not be yet |
| `zax-harness/tests/parity_wgpu_tests.rs` | five more tests |

#### The scene pass had to become two, and that is the whole of the architecture

REQ-SCN-080 puts the screen passes between the opaque objects and the transparent ones, and has
them read the frame's rendered colour and depth. On OpenGL that is a sequence of draws into
framebuffers bound and unbound inside one logical pass; in `wgpu` reading an attachment means the
render pass that wrote it has ended. So a frame with a screen effect submits:

1. **`zax scene`** — the sky and the opaque objects, resolving colour into the single-sample target;
2. **`zax depth resolve`** — only when multisampled (below);
3. the effect's own passes — two for the occlusion, then its composite;
4. **`zax scene blended`** — the transparent objects and the HUD, into the **single-sample** target
   with the single-sample depth, loading both.

A frame without one submits exactly what it submitted before this step: one `zax scene` carrying
all three buckets, then the resolve. That is what `Buckets::All` is, and it is why the bucket loop
was lifted into a free function (`draw_buckets`) taking field borrows rather than a `&mut self`
method — the render pass already borrows `self.linear`, so a method could not be used.

Pass 4 is REQ-SCN-080's own stated consequence, made concrete: **with anti-aliasing *and* a screen
effect, the transparent objects and the HUD are not multisampled**, because from the resolve
onwards the frame draws into the single-sample target. The practical cost is one extra line in
`prepare_pipelines` — those two buckets ask for the one-sample pipelines as well, and
`SceneKey.samples` already keyed on it.

#### The depth resolve, and why sample 0 is the faithful answer and not an approximation

The one piece of this step with no OpenGL counterpart at all.
`LinearColorBuffer::resolve_samples` is one `glBlitFramebuffer` with
`COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT`. On `wgpu` the colour half is free — the scene pass already
names the single-sample target as its `resolve_target` — but there is **no depth resolve**:
`RenderPassDepthStencilAttachment` has no `resolve_target` field, and there is no
texture-to-texture blit for a depth format.

So `depth_resolve.wgsl`: one full-screen triangle reading `texture_depth_multisampled_2d` and
writing `textureLoad(depth, coord, 0)` into `@builtin(frag_depth)`, through a depth-only pipeline
with `depth_compare: Always`. Sample 0 is what the OpenGL blit gives, and `gl/screen_effects.rs`
already says so where it matters — "the depth the pass reads is a **resolved** multisampled depth
— one sample's, not the fragment centre's" — which is also why the floor mask of REQ-SCN-084
carries its polygon-offset bias. With `video.anti_alias` = 0, the handheld default and the one the
goldens are taken at, the pass is **not submitted** and `depth_single` *is* the frame's own depth
texture.

It is **counted** in `screen_effect_passes`, unlike the OpenGL blit, which is not a pass. So that
counter reads one higher on `wgpu` under anti-aliasing, and
`req_scn_080_the_occlusion_costs_the_same_passes_and_bytes_on_each_backend` asserts exactly that
difference rather than relaxing to an inequality: 3 and 3 at `video.anti_alias` = 0, **4 against
3** at level 1, and 153 600 bytes of targets on both at both levels.

#### Four things the port had to get right that a text substitution would not

1. **`uTexel` and the target size are different numbers, and one block cannot serve two
   resolutions.** `wgpu` applies every `queue.write_buffer` of a submission **before** the command
   buffers run, so a block written twice hands both passes the second value. The estimator runs at
   half of each dimension (except at `ao.quality` High) and the composite at the frame's own, so
   there are two blocks 256 bytes apart in one buffer and a bind group per block.
2. **The depth is read with `textureLoad` and the clamp written out.** GL's `NEAREST` with
   `CLAMP_TO_EDGE` is `clamp(floor(uv * size), 0, size - 1)`; a `textureLoad` outside the texture
   hands back **0** here, and 0 is the near plane — the one value that would make every sample
   occlude. The composite's read is the one that keeps a sampler, because its upsample is the
   `LINEAR` filter of `allocate_occlusion(filtered = true)`.
3. **`out` parameters again.** `zaxProjectView` and `zaxViewNormal` each return a value and a
   boolean, so each returns a struct (`ZaxProjected`, `ZaxNormal`).
4. **The composite is a blend state, not a copy.** `glBlendFunc(GL_ZERO, GL_SRC_COLOR)` — the
   `dst = dst * src` that lets one triangle scale the linear buffer in place — is
   `BlendComponent { src_factor: Zero, dst_factor: Src, operation: Add }`. So the occlusion needs
   no second colour target and no copy of the scene here either, which is REQ-SCN-083's "en espace
   linéaire, avant l'encodage sRGB" satisfied the same way.

#### The y mirror: measured, right, and nearly invisible — which is the finding

The previous session's design section called the `uv`-to-NDC mapping "the real coordinate trap"
and predicted that getting it wrong "would **not** fail obviously". Both halves were tested by
building the wrong version and measuring it. The scene is `shadow`, the only one of the four this
file reads where the occlusion does anything (see below); release, 640 × 480, anti-aliasing off,
Vulkan and DX12 identical in every row.

| `screen_depth.wgsl` as built | `shadow`, within 2 levels | pixels differing |
|---|---|---|
| **the port as it stands** (y mirrored in the NDC map *and* in the dither's pixel) | **99.8594 %** | 1.4782 % |
| the dither left in `wgpu`'s row order | 99.8232 % | 1.5726 % |
| the GLSL's `ndc = uv * 2 - 1` kept — no y mirror at all | 99.7790 % | 1.5684 % |

So the prediction holds **exactly**: the wrong NDC mapping costs 0.08 of a percentage point and
would have passed every bar in this file. The reason is the one the design gave —
`zax_view_normal` fixes the normal's sign from `dot(normal, p)` rather than from the winding, so
the occlusion is nearly invariant under a mirror of view space. It is the reflections of step 4c,
whose rays march in a direction that has to agree with the eye, where it would bite; the figure
above is the warning written down with a number on it.

The dither's own mirror is the smaller half and it is a deliberate choice rather than a
requirement: `zax_pixel` returns `(x, height - y)` so that `zax_dither` hands each pixel the same
kernel rotation it gets on OpenGL. REQ-SCN-083 leaves the per-pixel variation free, so either
would be faithful — but mirroring it means the two backends' occlusion of a pixel differs by the
compilers' rounding rather than by the kernel's variance, and it is worth 0.036 of a point.

#### Parity, measured

Same method as steps 2 and 3 — worst of four scenes, release, 640 × 480. Vulkan and DX12 agree to
the fourth decimal in every row.

| Configuration | max channel | pixels differing | within 2 levels |
|---|---|---|---|
| Classic, the control | 223 | 0.3577 % | 99.6423 % |
| `spl.ao` = 2 (Depth, `ao.quality` Medium — half resolution) | 223 | 0.3577 % | **99.6423 %** |
| `spl.ao` = 2, `ao.quality` = 2 (High — full resolution) | 223 | 0.3577 % | **99.6423 %** |
| `spl.ao` = 2 under MSAA 4x | 223 | 0.8919 % | **99.1084 %** |

The worst scene of each row is `heavy`, and on `heavy` the figure is **the control's, to the
fourth decimal** — for the reason the next section gives. The scene where the occlusion actually
acts is `shadow`, and there the rows read:

| Configuration | `shadow`, within 2 | pixels differing | (the Classic control) |
|---|---|---|---|
| `spl.ao` = 2, Medium | 99.8594 % | 1.4782 % | 99.9990 %, 0.0010 % |
| `spl.ao` = 2, High | 99.9342 % | 1.2692 % | — |
| `spl.ao` = 2, MSAA 4x | 99.7044 % | 1.5990 % | — |

The occlusion itself moves 1.76 % of `shadow` on OpenGL, so of the pixels it touches, **84 % agree
exactly and 92 % agree to within 2 levels**. The residual is the kernel's boundary cases: ten
samples, each worth up to `ceiling / 10` = 0.09 of `O`, and each decided by a `>` on floats the
two compilers may round the other way — one such flip is up to about 23 sRGB levels on a bright
surface, which is where the max channel of 107 on that scene comes from. High (16 samples, full
resolution, no upsample) reads better than Medium, which is the same explanation from the other
side.

The MSAA row is held to 98.5 % for the reason step 4a established and measured: MSAA sample
positions are implementation-defined in all three APIs. It reads 99.1084 %, which is 4a's own
`heavy` figure to the fourth decimal — the occlusion adds nothing to it.

#### The occlusion moves nothing at all of `framing` or `heavy`, and that is REQ-SCN-083 working

The one thing in this step that looked like a defect and is not. The first version of
`req_scn_083_the_occlusion_darkens_as_much_of_the_frame_on_each_backend` asked `framing`, and
failed with "the **OpenGL** occlusion darkened only 0.00 %". Measured across the whole set, on the
OpenGL backend, `spl.ao` = 2 against Classic:

| scene | pixels the occlusion changes |
|---|---|
| `framing`, `heavy`, `clip`, `nudge`, `alpha`, `sky`, `hud` | **0.000 %** — byte-identical |
| `shadow`, `no-shadow`, `layer-shadow-off` | 1.76 % |
| `backdrop` | 2.54 % |

That is the requirement as written: `O` is null "là où aucune géométrie occultante ne se trouve à
moins de 2r", and the dense tile-box scenes of the synthetic set are flat facades with no
concavity inside the radius, while the three shadow scenes and the backdrop have a box standing on
a floor. So the agreement rows above sitting exactly on the Classic control for `framing` and
`heavy` is not the port hiding — on those two the pass runs (3 submitted passes, 153 600 bytes
allocated, both backends) and changes nothing, on both backends alike. The test now reads `shadow`
and **asserts that `framing` and `heavy` are untouched**, so that a future fixture change which
makes them occlude is a failure rather than a silent weakening.

#### The one word of the OpenGL path, and the counts that say it cost nothing

`crate::renderer::projection_terms` — the four numbers these passes read off the projection matrix
— went from `fn` to `pub(crate) fn`, and the `wgpu` screen passes call it. That is the whole diff
in an OpenGL file for this step, and it was preferred to a second copy of the same four matrix
reads because a numerical rule in two places is how the two backends drift.

- `git diff --stat` over `gl/`, `src/shaders/`, `zax-engine3d/` and `zax-app/` is **empty**;
  `renderer.rs` is +5/-1, four lines of which are the comment explaining the fifth.
- **The four instruction counts are unchanged**, re-measured with a script written fresh for this
  lot (post-LTO assembly of the harness binary, default features, so the column is the OpenGL
  path):

| Symbol | `faced2a` | step 4b |
|---|---|---|
| `Ppu2C02::run_dots` | 2231 | **2231** |
| `DrawListBuilder::build` | 1095 | **1095** |
| `SceneRenderer::render_scene` | 3401 | **3401** |
| `Construction::segment` | 159 | **159** |

- `render_tests` (the OpenGL goldens): 3 passed. `render_wgpu_tests`: 2 passed.

#### What a Classic `wgpu` frame pays for this step

Nothing it can measure, and the list is short enough to state in full:

- `select_screen_effects` is one `screen_effects_active()` on the style and one assignment of
  `None` to an `Option` that is already `None`;
- `scene_pass` gains one `is_some_and` on that same `Option`, whose result (`false`) selects
  `Buckets::All` and the single render pass of before this step;
- `ensure_linear` takes a fourth comparison in its "same as last frame" test, and allocates
  exactly what it allocated before (`readable_depth` false adds no usage flag and no texture);
- `style_programs` and `screen_effect_bytes` are assigned 0 once a frame.

No pass is submitted, no pipeline is switched to, no target is allocated and no module is built —
which is the brief's "wgpu Classic unchanged in cost" check, item by item.

#### Tests

| Criterion | Test |
|---|---|
| the block the WGSL declares and the block the renderer writes are the same length; `uTexel` at half resolution is two pixels of the frame | 2 unit tests in `screen_effects.rs` |
| REQ-SCN-080's depth reconstruction and the y mirror, REQ-SCN-083's kernel, ceiling, blur and factor, and the multisampled-depth pass are in the WGSL; **nothing of REQ-SCN-084 or of the MSVO estimator is** | `amd055_style_tests.rs` `q_104_the_wgpu_backend_carries_the_occlusion_and_not_yet_the_reflections` (rewritten) |
| REQ-SCN-083 at the default quality | `parity_wgpu_tests.rs` `req_scn_083_the_depth_occlusion_agrees_between_the_backends` |
| REQ-SCN-083 at High, where the divisor is 1 — the one number a port can get right at one quality and wrong at the other | `req_scn_083_the_full_resolution_occlusion_agrees_between_the_backends` |
| the occlusion darkens as much of `shadow` on each backend, leaves `framing` and `heavy` alone, and never touches the HUD | `req_scn_083_the_occlusion_darkens_as_much_of_the_frame_on_each_backend` |
| REQ-SCN-080 under MSAA 4x — the depth-resolve path | `req_scn_080_the_occlusion_agrees_under_anti_aliasing` |
| the passes and the target bytes each backend spent, off `render.json`, with the one pass this API needs more | `req_scn_080_the_occlusion_costs_the_same_passes_and_bytes_on_each_backend` |
| Vulkan against DX12 at every anti-aliasing level, **with the occlusion on** | `ru_7_vulkan_and_dx12_agree_with_each_other_at_every_level` (extended) — 100.0000 %, max channel 0, at all three levels |

`cargo test --release -p zax-render --features wgpu`: 33 suites, **314 passed, 0 failed**, 1
ignored (312 at step 4a, plus this step's two unit tests). `-p zax-harness --features wgpu`: 13
suites, **68 passed, 0 failed** (17 of them `parity_wgpu_tests`). `-p zax-goldens`: 3 passed.
`cargo fmt --all --check` and `cargo clippy --workspace --all-targets --features zax-app/wgpu --
-D warnings` clean.

### Step 4c — the reflections of REQ-SCN-084 and the floor mask

The second screen effect, and the one with the most moving parts: a scene-pipeline variant that
lays a mask down, a march that reads the frame's own colour, and a blend that puts it back.

#### What landed

| Piece | What landed |
|---|---|
| `shaders/ssr.wgsl` (new) | the twin of `shaders/ssr.frag`: the quadratic march on positions fitted to the longest ray, the bisection, the thickness test, the spiral blur and the two attenuations |
| `shaders/ssr_composite.wgsl` (new) | the twin of `ssr_composite.frag`: the trace sampled back up through the filtered sampler, multiplied by the mask at full resolution |
| `shaders/scene.wgsl` | the `ZAX_FLOOR_MASK` variant — the `scene_normal` varying (the normal **before** the curvature) and an `fs_main` that is REQ-SCN-084's `+Y ≥ 0,9` test, with the ordinary one moved into the `//#else` branch |
| `shaders/screen_depth.wgsl` | `zax_load1`, the second source slot |
| `wgpu_backend/screen_effects.rs` | the `Reflections` half: the trace and composite pipelines, the `R8Unorm` mask at the frame's size, the `Rgba16Float` trace at the quality's, two more uniform blocks, `mask_target`, `trace`, `composite_reflections` |
| `wgpu_backend/renderer.rs` | `ShaderVariant.floor_mask` and `ShaderVariant::mask()`; `mask_modules`, `mask_pipelines`, `Layouts::mask_pipeline`, `mask_pipeline()`; `prepare_mask`; `mark_floors`; the floor runs in `upload_draws`; `draw_floor_mask`; the mask pass and the reflections in `run_screen_effects`; the floor vertical in the frame block; **the Q-104 diagnostic line and `style_reported` are gone** |
| `zax-render/tests/common/mod.rs` | `WgpuFixture` — a headless `wgpu` device with a target of the same shape as `GlFixture`'s, so that one `Built` scene can be drawn through both backends; and `close_up_r`/`is_cube`, moved here from `amd055_reflection_tests.rs` because two binaries now want them |
| `zax-render/src/wgpu_backend/headless.rs` | `HeadlessGpu::size`, which a reader of `read_pixels` needs to index rows |
| `zax-render/tests/amd055_parity_tests.rs` (new) | seven tests — the file the reflections are measured in, for the reason below |
| `zax-render/tests/amd055_style_tests.rs` | the two-sided Q-104 test now reads "every effect but the multi-scale estimator" |
| `zax-harness/tests/parity_wgpu_tests.rs` | three more tests |
| `zax-app/src/graphics.rs`, `zax-harness/src/render/engine.rs` | one comment each: Q-104's subject is answered, so the two `// Q-104` notes about the unrendered style go. **Comment-only**, inside the `#[cfg(feature = "wgpu")]` arm |

#### The synthetic set has no floor at all, and that is why this step has a test file of its own

The first thing this step measured, and it changes where the evidence lives. On the **OpenGL**
backend, `spl.reflections` = 1 against Classic leaves **all eleven synthetic captures
byte-identical**. Not one instance of the set is a floor surface, and that is REQ-SCN-084 as
written: a floor needs a bounding box of at least `6 × échelle de scène` in both dimensions of the
floor plane **and** a face whose scene normal is `+Y ≥ 0,9`. The widest flat thing in the set is
the backdrop, 256 × 240 × **1** — which Q-093 (a) already records as not a floor.

So the harness, whose parity file is this lot's, cannot measure what the reflections *do*. The
frame that can is the **close-up R of §2.5**, which `amd055_reflection_tests.rs` builds in-process
for the OpenGL invariants. Hence two changes:

- `common::WgpuFixture`, a headless `wgpu` device beside `common::GlFixture`, so that one `Built`
  scene is drawn through both backends and compared. It is this lot's most reusable piece of test
  support: step 5's §2.4 invariants on the `wgpu` side need exactly the same thing.
- `close_up_r` moved from the one test binary into `common`, so the fixture is written once.

The harness file keeps three tests of the reflections all the same, and they are **not**
redundant: they exercise the plumbing at the frame's own scale — a mask pass opened over a frame of
1 926 opaque runs, a trace reading 307 200 zero texels, the three passes submitted for it — and
one of them is a **negative** parity test that asserts the frame is untouched where there is no
floor. That test is what would have caught the bug below.

#### The bug: `glPolygonOffset`'s two arguments cross over into `DepthBiasState`

The one real defect of this step, and the parity file found it.

The floor mask is drawn with the frame's own depth attached, `LEQUAL`, depth writes off, and a bias
that pulls the geometry a hair towards the eye — without it the pass loses the depth test to
itself, and with a *resolved multisampled* depth an exact `LEQUAL` marks nothing at all (the
OpenGL side recorded that: "a mask that was empty on every multisampled frame is what found
this"). The OpenGL call is `polygon_offset(-1.0, -16.0)`.

`glPolygonOffset(factor, units)` offsets by `m × factor + r × units`, where `m` is the depth slope
of the primitive and `r` the smallest resolvable difference. So GL's **first** argument is wgpu's
`slope_scale` and its **second** is wgpu's `constant`. Written the obvious way round —
`DepthBiasState { constant: -1, slope_scale: -16.0 }` — the slope term is sixteen times too large,
and a steeply-seen face then wins `LessEqual` against surfaces in front of it. Measured on the
close-up R:

| | pixels the mask let through that should not have been there |
|---|---|
| `constant: -1, slope_scale: -16.0` (wrong) | the top of the red cube marked where OpenGL leaves it alone; 1 709 pixels differ from OpenGL, box reaching 54 rows above OpenGL's |
| `constant: -16, slope_scale: -1.0` (right) | 1 350, then 1 236 once the figures settled; the box agrees |

Worth stating plainly: this is the **second** defect of this lot that only a device could find, and
like `fwidth` in step 3 it was invisible in both shaders — the bug is in a pipeline field, not in
any WGSL. It is also the second one where the two APIs agreed with each other and both differed
from OpenGL, which is why the OpenGL-reference comparison and not the Vulkan-against-DX12 one is
the assertion that caught it.

#### Three places where the port could not be a transcription

1. **The mask's module carries no texture chunk, and its pipeline layout has two groups.** The
   mask's fragment stage reads no texture, so the module is built from `[SCENE]` alone rather than
   `[SCENE, TEXTURES_BOUND]`, and `Layouts::mask_pipeline` binds `[frame, draw]` and not
   `[frame, draw, textures]` — otherwise the pass would have to bind a group of textures it never
   reads. That is the shape the OpenGL side has for free (a program simply has no such uniforms)
   and that `wgpu` makes explicit.
2. **The floor vertical rides in the frame block's spare slots.** The GLSL has `uFloorUp` as a
   `vec3` uniform of its own. Here it goes into `style.yzw`, the three slots the voxel cell left
   spare in a `vec4` the block already had — so `FRAME_BYTES` stays at the 288 of step 2, and a
   Classic frame copies no more bytes for the reflections than it already did.
3. **The floor runs are a fourth bucket of the draw blocks.** `lay_out_floor` appends their
   placements at offsets of their own (shared code, with this backend's 64-byte alignment instead
   of the OpenGL one's 256). What this side has to add is a draw block and a `run_of` entry per
   floor run, after the opaque, transparent and HUD runs — and an indirect-arguments entry in the
   same order, so the batched path multi-draws them. Empty on every frame that does not ask for the
   reflections, so the loop is one `is_empty` in Classic.

#### Parity, measured — and a different kind of residual

The reflections are the first effect of this lot whose per-pixel result is **discontinuous in the
depth buffer**. The curvature, the grid, the pixel shadows and the occlusion are all smooth
functions of the frame, so two correct implementations differ by their compilers' rounding and land
on the Classic control's own figure. A screen-space march is not: the hit is decided by `>`
comparisons on reconstructed view depths, and one step of difference in where the bisection lands
makes the pixel reflect a **different texel of the frame**. On the close-up R that texel is often
across the boundary between the red cube and the grey floor, so the colour jump is large even where
the march agreed.

So the whole-frame share is the wrong instrument, and the right one is the **reflected pixel set**.
Release, 640 × 480, anti-aliasing off, `ssr.quality` High:

| | pixels |
|---|---|
| reflected by **both** backends | **5 259** (DX12: 5 261) |
| reflected by OpenGL alone | 204 (DX12 column: 202) |
| reflected by Vulkan alone | 241 (DX12: 241) |
| of the common set, differing by more than 2 levels | 740, **worst 35** |
| outside every reflection yet differing by more than 2 | 44 — the control's own silhouette noise |

**The fringe is symmetric** — 3.88 % of the common set on OpenGL's side against 4.58 % on
Vulkan's — and that is the figure that says it is the march's hit/miss boundary rather than one
backend reflecting something the other cannot see. A mask that marked an extra face, or a ray sent
the wrong way by the `y` mirror of `screen_depth.wgsl`, would put the whole of that face on one
side of the table; the wrong depth bias above did exactly that, and the fringe was what showed it.
The worst difference **inside** the common set, 35 levels, is far under the **118** the unreflected
control already carries on this scene.

The whole-frame shares, for the record, and the control they are read against:

| Configuration | max channel | pixels differing | within 2 levels |
|---|---|---|---|
| the close-up R, reflections **off** (the control) | 118 | 0.0479 % | **99.9521 %** |
| `spl.reflections` = 1, `ssr.quality` High (half resolution) | 118 | 0.4395 % | **99.5999 %** |
| the same at `ssr.quality` Ultra (full resolution) | 118 | 0.1507 % | **99.8493 %** |
| with the occlusion of REQ-SCN-083 as well | 108 | 20.9860 % | **99.1045 %** |
| **Vulkan against DX12**, reflections on | **38** | — | **99.9993 %** |

Ultra reads better than High by exactly the half-resolution trace's upsample, which is the
expected ordering. The last row is the two `wgpu` APIs against each other: not byte-identical any
more — the first configuration of this lot where they are not — and the reason is the same
discontinuity, on **two or three pixels** (5 259 against 5 261 reflected). The occlusion row's
20.99 % of differing pixels is the occlusion doing its job on a frame that is almost entirely
floor; its `within 2` is what matters and it is the occlusion's own figure.

And the effect is real and the same size on each backend: the reflections move **1.7783 %** of the
close-up R on OpenGL, **1.7904 %** on Vulkan and **1.7910 %** on DX12 — 5 463 pixels of the floor
at the foot of the cube and of the cube's own top face, which the extent test also admits (8 units
in both dimensions of the floor plane, over the 6 the requirement asks).

#### What a Classic `wgpu` frame pays for this step

- `scene_pass` gains one `is_some_and` on `self.screen_effects` (the `reflects` test), whose
  result selects neither `mark_floors` nor `prepare_mask`;
- `upload_draws` gains one loop over `self.plan.floor_runs`, which is empty;
- `frame_bytes` writes three more floats into a `vec4` it was already writing.

No module is built, no pipeline is added, no target is allocated and no pass is submitted. The four
instruction counts are still **2231 / 1095 / 3401 / 159**, and the only OpenGL-side diff of this
step is the two comments listed above.

#### Tests

| Criterion | Test |
|---|---|
| REQ-SCN-084's four constants, the march, the premultiplied result, the composite's mask and the floor test are in the WGSL; **nothing of the MSVO estimator is** | `amd055_style_tests.rs` `q_104_the_wgpu_backend_carries_every_effect_but_the_multi_scale_estimator` (rewritten) |
| the close-up R with the reflections off — the control | `amd055_parity_tests.rs` `ru_7_the_close_up_r_agrees_between_the_backends_with_the_reflections_off` |
| REQ-SCN-084 at the default quality, and at Ultra where the divisor is 1 | `req_scn_084_the_reflections_agree_between_the_backends`, `..._the_full_resolution_trace_...` |
| **the reflected pixel set**: the common set, the two fringes, and their symmetry | `req_scn_084_the_two_backends_reflect_the_same_pixels` — the assertion that constrains the port |
| the reflections move as much of the frame on each backend | `req_scn_084_the_reflections_change_as_much_of_the_frame_on_each_backend` |
| REQ-SCN-080's order: the reflections before the occlusion, on one frame that has both | `req_scn_080_the_reflections_run_before_the_occlusion_on_both_backends` |
| Vulkan against DX12 with the reflections on | `ru_7_vulkan_and_dx12_agree_on_the_reflections` |
| the synthetic set agrees with the reflections on, **and nothing of it is a floor** — the negative test the depth-bias bug would have failed | `parity_wgpu_tests.rs` `req_scn_084_the_reflections_agree_between_the_backends`, `req_scn_084_nothing_of_the_synthetic_set_is_a_floor` |
| three passes and 921 600 bytes on each backend, plus the one this API needs under anti-aliasing | `req_scn_084_the_reflections_cost_the_same_passes_and_bytes_on_each_backend` |

`cargo test --release -p zax-render --features wgpu`: 34 suites, **321 passed, 0 failed**, 1
ignored (314 at step 4b, plus this step's seven). `-p zax-harness --features wgpu`: 13 suites,
**71 passed, 0 failed** (20 of them `parity_wgpu_tests`). `cargo fmt --all --check` and
`cargo clippy --workspace --all-targets --features zax-app/wgpu -- -D warnings` clean.

### The design 4b was built from, and what is left after it

Written at the end of the lot's **first** session, when steps 1, 2, 3 and 4a were landed, so that
the second session would not have to repeat the design work. **Step 4b was built exactly as this
section says**, including the two things it predicted would be awkward (the resolved depth and the
`y` sign), and its own section above records what the device said back — so this is kept as it was
written rather than updated, and the table at the end of it is the one that still has work in it.
The two measured predictions are worth reading together with it: the resolved depth became
`depth_resolve.wgsl`, and the `y` mirror turned out to cost 0.08 of a percentage point in the
occlusion and to be the reflections' problem instead.

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
| ~~4b~~ | ~~`wgpu_backend/screen_effects.rs`, `screen_depth.wgsl`, `ao.wgsl`, `ao_blur.wgsl`, `ao_composite.wgsl`, the depth-resolve pass, the frame order, the lazy lifecycle and its statistics~~ — **landed**, see its own section above |
| ~~4c~~ | ~~`ssr.wgsl`, `ssr_composite.wgsl`, `floor_mask.wgsl`, the mask's scene-pipeline variant (`ZAX_FLOOR_MASK`) and its depth attachment~~ — **landed**, see its own section above |
| 4d | `msvo.wgsl` x5, the `R16F` pyramid, the runtime fallback **(D)** and its one diagnostic line |
| 4e | `backdrop.wgsl` and the `backdrop` axis of AMD-049 REQ-SCN-075, the step the owner added between the two sessions (see the checklist) |
| — | then the diagnostic line of `wgpu_backend/renderer.rs` — narrowed by 4b to the reflections alone — `style_reported`, and the remaining `// Q-104` comments in `zax-app/src/graphics.rs` and `zax-harness/src/render/engine.rs` all go, and the second list of `amd055_style_tests.rs`'s two-sided test empties |
| 5 | the parity table and the GPU costs of each effect at 1080p, beside RU-5's and RU-6's OpenGL figures |
| 6 | the close: `PERFORMANCE-HISTORY.md`, the Q-104 closing section answering its questions 1, 3 and 4, the full suite, the release build with `zax-app/bundled,zax-app/wgpu` |

Two notes 4b leaves for the steps after it:

- **`DepthBiasState` is the right equivalent for the floor mask's bias**, but the mask's depth
  attachment is `depth_single`, the single-sample depth 4b allocates and (under anti-aliasing)
  writes — not the frame's multisampled one. 4c binds that, and its pipeline is a one-sample
  variant of the scene pipeline.
- **The `Screen` uniform block already has the reflections' five fields** (`ssr`, `blur_aspect`,
  `counts.yzw`), written as zeros today, and a `Slot` enum to put a third and fourth block in.
  4d's MSVO pyramid is the one that will need the block per level.

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
