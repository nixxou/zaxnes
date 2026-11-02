# RU-4a — curvature, curvature in 2D mode, negative clipping margins, transparent 2D backdrop

Branch `rustupdated`, on top of RU-3b-fix (`553af8b`). Amendments AMD-043 (curvature of space),
AMD-046 (curvature in 2D mode), AMD-047 (negative clipping margins) and AMD-050 (2D mode:
universal-backdrop pixels are transparent) ported to the **Rust** application. The .NET lots
V2-4 (`b78e44c`), V2-6 (`7aeadf8`), V2-7 (`7983a3a`) and V2-12 (`5caf498`) of `dotnetupdated`
are a reading reference only.

## Checklist

| # | Step | Status |
|---|---|---|
| 1 | Map: what exists, where each requirement goes, the spec copies | done — `<pending>` |
| 2 | AMD-047: the four margins accept any finite real (REQ-PERS-087), Q-080's grid guard | todo |
| 3 | AMD-043 §3: `SpaceBend` — B, its inverse, its Jacobian, the 13 vectors | todo |
| 4 | AMD-043 §3: the shader **variants**, the renderer wiring, the subdivisions | todo |
| 5 | AMD-046: the 2D rectangle bends, 32 × 32 only when it must, `scene.bend_2d_mode` | todo |
| 6 | AMD-043 §4: the picking inverse (REQ-INP-124), on input events only | todo |
| 7 | AMD-050: REQ-CORE-048 checked, the opacity mask measured and decided, REQ-SCN-076 | todo |
| 8 | The settings: desktop options, handheld defaults and page, harness options | todo |
| 9 | Closing: draw/vertex counts at default, `PERFORMANCE-HISTORY.md`, fmt, clippy, tests | todo |

The hash of a step is written in the commit of the step that follows (no amend, protocol of this
lot).

## Notes

### State of the working copy at the start

`git status` showed one untracked folder, `tests/`: the **.NET** test tree's `bin/` and `obj/`
build output, left behind by an earlier checkout of `dotnetupdated` in this working copy. It is
not this lot's work and not the Rust application's; it is left untracked and uncommitted.

### What already exists on the branch, checked key by key

| Piece | State |
|---|---|
| `scene.bend_lr`, `scene.bend_tb`, `scene.arc_h`, `scene.arc_center` | **exist** (RU-2): `catalogue.rs`, `table.rs` rows with `Domain::stepped(−180, 180, 30)` and `(0, 1, 1)`, the style parameters `spl.bend_zx` / `spl.bend_zy` / `spl.arc_yx` / `spl.arc_center` of REQ-LUA-114 (`styles.rs`), all four in `ui/options.rs` `EXCLUDED` |
| `scene.bend_2d_mode` | **does not exist.** The lot's brief says RU-2 created it "as Lua surface only", but AMD-046 §2 gives the setting **no Lua access** ("le programme d'origine n'en a pas"), so RU-2 — which added the keys the 2.0 `Setting` members stand for — had no reason to. It is created here, in step 5. |
| the curvature itself | nothing: no `SpaceBend`, no bend uniform, no bend shader chunk |
| the four clip margins | `Domain::range(0, 256)` / `(0, 248)` in `table.rs` `clipping()`; `ConversionSettings::DEFAULT_CLIPPING = (0.1, 8.1, 8.1, 0.1)` |
| the picking | `zax-app/src/input/zapper.rs`: `ray`, `try_intersect`, `to_nes_pixel`, `try_aim`, `ZapperState::update` |
| the opacity mask | nothing; `Ppu2C02::pixels` holds one 6-bit NES colour index per pixel |

### Requirement map

#### AMD-047 — negative clipping margins

| Requirement | Rust crate / file |
|---|---|
| REQ-PERS-087 domain of the four margins | `zax-settings/src/table.rs` (`clipping()`), `zax-settings/src/domain.rs` (a finite-real rule) |
| the formulas unchanged for negative margins | `zax-render/src/geometry.rs` (`VisibleRect`), `camera.rs` (T′, P\*) — read-only check |
| the tile filter of REQ-E3D-002 and the grid margin of Q-080 | `zax-engine3d/src/grid.rs` / `segmenter.rs` |

#### AMD-043 — curvature of space

| Requirement | Rust crate / file |
|---|---|
| REQ-SCN-066 the four settings | already there (RU-2); out of `EXCLUDED` in step 8 |
| REQ-SCN-067 the transform B | `zax-engine3d/src/space_bend.rs` (new): B, B⁻¹, the Jacobian, the transported normal |
| REQ-SCN-068 scope, shadow pass, HUD/skybox flat, subdivision, order with the clipping | `zax-render/src/shaders/bend.glsl` (new), `scene.vert`, `shadow.vert`, `gl/shaders.rs` (the variants), `renderer.rs`, `scene.rs` (`SceneState::bend`) |
| REQ-SCN-028 amended (the normal transported) | `bend.glsl` (the cofactor of the Jacobian) |
| REQ-INP-124 the picking | `zax-app/src/input/zapper.rs` |

#### AMD-046 — curvature in 2D mode

| Requirement | Rust crate / file |
|---|---|
| REQ-SCN-074 the rectangle subdivided and bent, the setting | `zax-render/src/interop.rs` (`picture_quad_wide`), `scene.rs`, `renderer.rs`; `zax-settings` for `scene.bend_2d_mode` |
| REQ-INP-124 amended (2D mode, setting off → REQ-INP-063) | `zax-app/src/input/zapper.rs` |

#### AMD-050 — the 2D rectangle's universal-backdrop pixels

| Requirement | Rust crate / file |
|---|---|
| REQ-CORE-048 amended (P1, the index image) | `zax-nes/src/ppu/render.rs` — already held, checked in step 7 |
| REQ-CORE-049 the opacity mask (P2) | `zax-nes/src/ppu/render.rs` (`span_pixel`, `render_pixel`), `ppu/mod.rs` (the palette) |
| REQ-SCN-076 the fragment rejected | `zax-render/src/gl/picture.rs` (the alpha channel), `scene.frag` — **already discards on alpha 0** |

### The plan for REQ-CORE-049, and why it can be free (step 7's hypothesis, measured there)

The brief asks for a cheap encoding before anything else. Reading the two pixel functions gives
one, and it rests on an identity:

In `span_pixel` and `render_pixel` the palette entry is

    entry = if sprite opaque and (background transparent or sprite in front) { 0x10 | (sprite & 0x0F) }
            else if background opaque { background }
            else { 0 }

- the sprite branch yields `entry ≥ 0x10`, so non-zero;
- the background branch yields `background` whose low two bits are non-zero, so non-zero;
- the third branch is the only one that yields 0.

And `background` is held at 0 unless the background layer is on and, for `x < 8`, unmasked;
likewise `sprite` (`SpanPixels::background`/`background_left`/`sprites`/`sprites_left`, the four
PPUMASK switches of REQ-CORE-049). `SPRITE_OPAQUE` is set by `paint_sprite` only where the
sprite's pixel value is non-zero, and `paint_sprite` looks at no "hidden" flag — which is what
REQ-CORE-049 asks ("quel que soit son drapeau « caché »").

So **`entry != 0` is exactly the opacity bit of REQ-CORE-049**, and `entry & 0x1F == 0` iff
`entry == 0` (entry never exceeds `0x1F`). The index into the palette therefore already carries
the bit, which gives an encoding that adds *no instruction at all* to either pixel function:

1. keep a derived `palette_marked: [u8; 32]` beside `palette`, with
   `palette_marked[0] = palette[0]` and `palette_marked[i] = palette[i] | 0x40` for `i ≥ 1`;
2. make the span's `color_mask` `0x7F` / `0x70` instead of `0x3F` / `0x30` (built once per span),
   and `render_pixel`'s greyscale mask `0x70`;
3. the two functions then read `self.palette_marked[entry & 0x1F] & color_mask` — the same
   indexed load and the same `AND` as today.

`palette_marked` is maintained at the three sites that write `palette` (the one register write,
the power-up fill, the state restore), never per pixel; it is derived, so it is **not** in the
state blob (`ppu/state.rs` writes `palette` only, and `pixels` is not in the blob at all, so no
savestate moves).

The consequence to settle in step 7 is the **interface**: `Ppu2C02::pixels` then holds
`index | opacity << 6`, and `pixel_frame()` no longer hands out 0..63. The PPU goldens hash
`pixel_frame()` (`zax-nes/tests/ppu_golden_tests.rs`, `zax-harness/src/golden_core.rs`), so the
accessor is **renamed** rather than redefined, which makes the compiler visit every call site
instead of letting one silently hash the wrong bytes. The index image of REQ-CORE-048 stays
available, and the goldens keep their hashes.

On the renderer's side nothing new runs: `gl/picture.rs` already resolves each index to RGBA once
per frame **in 2D mode only**, already writes `alpha = 0` for a pixel it leaves out
(`interop::HIDDEN_PIXEL`), and `scene.frag` already discards on `direct.a == 0.0`, in the opaque
pass, with no sorting and no blending — which is REQ-SCN-076's "rejet du fragment" word for word.

### The plan for the shader variants (step 4), and why not the .NET shape

The .NET lot tests a `uBend` uniform per vertex. This lot's point 1 forbids that: the vertex
shaders at curvature 0 must be today's, instruction for instruction. So `bend.glsl` is guarded by
`#define ZAX_BEND` and the renderer holds **two** scene programs and **two** shadow programs,
picking the pair when a curvature setting changes — never per vertex, never per frame.

### Open questions

None yet. Next free Q number is **Q-104**.
