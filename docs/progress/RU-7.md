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
| 1 | The gate: why the `wgpu` tests skip, fixed or proven; the map against `gl/`; the test plan | done — this commit |
| 2 | Curvature (AMD-043, AMD-046) | todo |
| 3 | The voxel grid and the pixel shadows | todo |
| 4 | The screen passes: Depth AO, MSVO, SSR, their targets and lifecycle, the AA resolve | todo |
| 5 | The parity measurements and the costs | todo |
| 6 | Closing: notes, `PERFORMANCE-HISTORY.md`, the Q-104 closing section, fmt, clippy, full tests, release build | todo |

The hash of a step is written in the commit of the step that follows (no amend, protocol of this
lot).

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
