//! The GLSL sources of the renderer, written against SPEC-06 §3.9 to §3.11, and how they are
//! put together.
//!
//! The original's shaders are DirectX bytecode and were not extracted; these are written from
//! the specification. Every file is written once, without a `#version` line: the program
//! builder prefixes the header of the context, `#version 330 core` on desktop GL (REQ-SCN-056)
//! or `#version 300 es` with default precisions on GLES 3.0 (REQ-ARCH-030). A program is the
//! header, its shared chunks (functions and uniforms, whose order GLSL does not care about) and
//! its own body.
//!
//! A [`Stage`] holds its chunks in fixed slots, empty ones skipped, so that a variant composes in
//! a `const fn`: the scene pass alone has eight (solid × sRGB palette × curvature).
//!
//! AMD-021 R2-009: every colour these shaders write is *linear*. The blending of REQ-SCN-040
//! therefore happens on linear colours, and the sRGB encode is the job of the resolve pass, the
//! single pass that copies the linear buffer into the host framebuffer.

use crate::sky;

/// The sRGB transfer functions of SPEC-06 §2, shared by every shader.
const TRANSFER: &str = include_str!("../shaders/transfer.glsl");

/// The animated deformation of SPEC-04B REQ-E3D-568, applied in the object's local frame before
/// the scale and the rotation (DEC-04B-Q2, REQ-SCN-043).
const DEFORMATION: &str = include_str!("../shaders/deformation.glsl");

/// The space bending of SPEC-06 REQ-SCN-067 (AMD-043).
const BEND: &str = include_str!("../shaders/bend.glsl");

/// The body of the scene pass.
const SCENE_VERT: &str = include_str!("../shaders/scene.vert");
const SCENE_FRAG: &str = include_str!("../shaders/scene.frag");

/// The backdrop lighting of AMD-049 REQ-SCN-075, the fragment body of [`backdrop`].
const BACKDROP_FRAG: &str = include_str!("../shaders/backdrop.frag");

/// The body of the shadow pass.
const SHADOW_VERT: &str = include_str!("../shaders/shadow.vert");
const SHADOW_FRAG: &str = include_str!("../shaders/shadow.frag");

/// The noise library of the procedural sky (AMD-024 REQ-PUB-003).
const SKY_NOISE: &str = include_str!("../shaders/sky_noise.glsl");

/// The five sky fields read from the cubemaps SPEC-06 names.
const SKY_TEXTURED: &str = include_str!("../shaders/sky_textured.glsl");

/// The same five fields, computed from the direction alone: no sampler, no file.
const SKY_PROCEDURAL: &str = include_str!("../shaders/sky_procedural.glsl");

/// The sun disc, the moon disc and the cloud layer of REQ-PUB-003, off by default (Q-064).
const SKY_BODIES: &str = include_str!("../shaders/sky_bodies.glsl");

/// Leaves out the `discard`s of the opaque draws that can never take them.
const SOLID_DEFINE: &str = "#define ZAX_SOLID";

/// Linearises the palette per fragment instead of reading [`super::linear_palette`].
const SRGB_PALETTE_DEFINE: &str = "#define ZAX_SRGB_PALETTE";

/// Compiles the curvature of REQ-SCN-067 in. **Its absence is the point**: at curvature 0 the
/// scene and shadow programs are the ones of before AMD-043, instruction for instruction, with
/// no uniform tested per vertex and no varying for the bent position.
const BEND_DEFINE: &str = "#define ZAX_BEND";

/// Interpolates `vUv` at the centroid of the covered samples (AMD-058 REQ-SCN-065). **Its
/// absence is the point too**: with `video.anti_alias` = 0 there is one sample per fragment, the
/// centroid *is* the pixel centre, and the programs the compiler is given are the ones of before
/// AMD-058 — byte for byte, so the on-disk program cache of a player who never turns
/// anti-aliasing on is not rebuilt.
const CENTROID_DEFINE: &str = "#define ZAX_CENTROID";

/// Modulates the albedo with the voxel grid of AMD-055 REQ-SCN-081. **Its absence is the point**,
/// as for the two above: with `spl.voxel_grid` = 0, which is the default of REQ-SCN-079 and the
/// only value Classic can have, the programs the compiler is given carry no grooves and no
/// `fwidth`.
const VOXEL_GRID_DEFINE: &str = "#define ZAX_VOXEL_GRID";

/// Evaluates the shadow of an opaque tile at the centre of the fragment's voxel (AMD-055
/// REQ-SCN-082). Independent of the grid above: REQ-SCN-079 switches the two separately, so a
/// player who turns the grid on must not pay for the snapping, and the other way round.
const PIXEL_SHADOW_DEFINE: &str = "#define ZAX_PIXEL_SHADOW";

/// Emits the pre-curvature scene normal the floor mask of AMD-055 REQ-SCN-084 judges. **Its
/// absence is the point**, as for the five above: the mask is a pass of its own, so no variant any
/// other draw uses carries this.
const FLOOR_MASK_DEFINE: &str = "#define ZAX_FLOOR_MASK";

/// The voxel grid of REQ-SCN-081 (AMD-055).
const VOXEL_GRID: &str = include_str!("../shaders/voxel_grid.glsl");

/// The frame's depth and the view-space position it reconstructs to, as the two screen-space
/// passes of AMD-055 REQ-SCN-080 read them.
const SCREEN_DEPTH: &str = include_str!("../shaders/screen_depth.glsl");

/// The pixel shadows of REQ-SCN-082 (AMD-055).
const PIXEL_SHADOW: &str = include_str!("../shaders/pixel_shadow.glsl");

/// How many chunks a stage may hold. The largest is the scene fragment with all six defines (six
/// defines, the transfer functions, the grid, the deformation and the curvature the pixel shadows
/// need in that stage too, the snapping itself and the body: twelve).
///
/// Raising it costs nothing and changes no assembled source: [`assemble`] skips an empty slot
/// without even a blank line.
const SLOTS: usize = 12;

/// One stage of a program: the chunks it is made of, in order, after the header. Empty slots are
/// skipped, so a variant can leave a define out without changing the shape of the array.
#[derive(Debug, Clone, Copy)]
pub struct Stage {
    /// The chunks; `""` for an unused slot.
    pub parts: [&'static str; SLOTS],
}

impl Stage {
    /// A stage of the chunks of `parts`, in order, the slots past them left empty.
    ///
    /// The array may be shorter than [`SLOTS`]: a stage that needs no chunk beyond its body says
    /// so in one line, and raising the ceiling does not touch every call site. Longer than
    /// [`SLOTS`] is a compile error, which is what it should be.
    const fn of<const N: usize>(parts: [&'static str; N]) -> Self {
        let mut all = [""; SLOTS];
        let mut slot = 0;
        while slot < N {
            all[slot] = parts[slot];
            slot += 1;
        }
        Self { parts: all }
    }
}

/// The vertex and fragment stages of one program.
#[derive(Debug, Clone, Copy)]
pub struct ProgramSource {
    /// Name used in error messages and in the key of the on-disk program cache.
    pub name: &'static str,
    /// Vertex stage.
    pub vertex: Stage,
    /// Fragment stage.
    pub fragment: Stage,
}

/// `""` when `enabled` is false, so that a define costs an empty slot and not a second slice.
const fn when(enabled: bool, define: &'static str) -> &'static str {
    if enabled {
        define
    } else {
        ""
    }
}

/// Which variant of the scene pass a draw asks for. One named field per axis the specification
/// names, each independent of the others.
///
/// `solid` leaves out the `discard`s for the opaque draws that can never take them (a shader that
/// may discard keeps a tiled GPU from dropping the fragments hidden behind it early).
/// `srgb_palette` linearises the palette per fragment. `bend` compiles in the curvature of
/// REQ-SCN-067, which also places the vertex and transports the normal (REQ-SCN-068). `centroid`
/// interpolates `vUv` at the centroid of the covered samples (AMD-058), which is only ever asked
/// for when the buffer is multisampled. `voxel_grid` modulates the albedo with the grooves of
/// AMD-055 REQ-SCN-081 and `pixel_shadow` evaluates the shadow at the voxel centre of
/// REQ-SCN-082; both are off in Classic and off by default in Enhanced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
// Each field is one independent variant axis the specification names; an enum per axis would say
// less, not more. The same `allow` `Passes` and `ScenePrograms` already carry.
#[allow(clippy::struct_excessive_bools)]
pub struct SceneVariant {
    /// No `discard`: the opaque draws that neither clip nor read a colour texture.
    pub solid: bool,
    /// The palette linearised per fragment instead of read from the linear copy.
    pub srgb_palette: bool,
    /// The curvature of AMD-043 REQ-SCN-067.
    pub bend: bool,
    /// `vUv` at the centroid of the covered samples (AMD-058 REQ-SCN-065).
    pub centroid: bool,
    /// The voxel grid of AMD-055 REQ-SCN-081.
    pub voxel_grid: bool,
    /// The pixel shadows of AMD-055 REQ-SCN-082.
    pub pixel_shadow: bool,
}

impl SceneVariant {
    /// The variant with every axis off: the pair the renderer is created with.
    pub const FLAT: Self = Self {
        solid: false,
        srgb_palette: false,
        bend: false,
        centroid: false,
        voxel_grid: false,
        pixel_shadow: false,
    };

    /// The same variant in its `solid` form, or its plain one.
    #[must_use]
    pub const fn with_solid(self, solid: bool) -> Self {
        Self { solid, ..self }
    }
}

/// The scene pass: albedo of REQ-SCN-034, visible-rectangle clipping of REQ-SCN-036, hard
/// shadow of REQ-SCN-030/031 and the Lambert shading of REQ-SCN-028. The scene frame is
/// left-handed (REQ-E3D-400), so the clip position is built from `(x, y, -z)` (REQ-SCN-001).
///
/// The names of the sixteen variants without `voxel_grid` and without `pixel_shadow` are
/// **unchanged**, because the name goes into the key of the on-disk program cache: a player who
/// uses neither effect keeps every binary that cache already holds, as AMD-058 kept them for a
/// player who uses no anti-aliasing.
#[must_use]
// The body is a table: sixty-four names, which have to be written out because they are the keys
// of the on-disk program cache and a `const fn` cannot build a string, plus the two stage
// arrays. There is no logic to split out of it.
#[allow(clippy::too_many_lines)]
pub const fn scene(variant: SceneVariant) -> ProgramSource {
    let SceneVariant {
        solid,
        srgb_palette,
        bend,
        centroid,
        voxel_grid,
        pixel_shadow,
    } = variant;
    ProgramSource {
        name: match (
            solid,
            srgb_palette,
            bend,
            centroid,
            voxel_grid,
            pixel_shadow,
        ) {
            (false, false, false, false, false, false) => "scene",
            (false, false, false, false, false, true) => "scene_pixel_shadow",
            (false, false, false, false, true, false) => "scene_grid",
            (false, false, false, false, true, true) => "scene_grid_pixel_shadow",
            (false, false, false, true, false, false) => "scene_centroid",
            (false, false, false, true, false, true) => "scene_centroid_pixel_shadow",
            (false, false, false, true, true, false) => "scene_centroid_grid",
            (false, false, false, true, true, true) => "scene_centroid_grid_pixel_shadow",
            (false, false, true, false, false, false) => "scene_bend",
            (false, false, true, false, false, true) => "scene_bend_pixel_shadow",
            (false, false, true, false, true, false) => "scene_bend_grid",
            (false, false, true, false, true, true) => "scene_bend_grid_pixel_shadow",
            (false, false, true, true, false, false) => "scene_bend_centroid",
            (false, false, true, true, false, true) => "scene_bend_centroid_pixel_shadow",
            (false, false, true, true, true, false) => "scene_bend_centroid_grid",
            (false, false, true, true, true, true) => "scene_bend_centroid_grid_pixel_shadow",
            (false, true, false, false, false, false) => "scene_srgb_palette",
            (false, true, false, false, false, true) => "scene_srgb_palette_pixel_shadow",
            (false, true, false, false, true, false) => "scene_srgb_palette_grid",
            (false, true, false, false, true, true) => "scene_srgb_palette_grid_pixel_shadow",
            (false, true, false, true, false, false) => "scene_srgb_palette_centroid",
            (false, true, false, true, false, true) => "scene_srgb_palette_centroid_pixel_shadow",
            (false, true, false, true, true, false) => "scene_srgb_palette_centroid_grid",
            (false, true, false, true, true, true) => {
                "scene_srgb_palette_centroid_grid_pixel_shadow"
            }
            (false, true, true, false, false, false) => "scene_srgb_palette_bend",
            (false, true, true, false, false, true) => "scene_srgb_palette_bend_pixel_shadow",
            (false, true, true, false, true, false) => "scene_srgb_palette_bend_grid",
            (false, true, true, false, true, true) => "scene_srgb_palette_bend_grid_pixel_shadow",
            (false, true, true, true, false, false) => "scene_srgb_palette_bend_centroid",
            (false, true, true, true, false, true) => {
                "scene_srgb_palette_bend_centroid_pixel_shadow"
            }
            (false, true, true, true, true, false) => "scene_srgb_palette_bend_centroid_grid",
            (false, true, true, true, true, true) => {
                "scene_srgb_palette_bend_centroid_grid_pixel_shadow"
            }
            (true, false, false, false, false, false) => "scene_solid",
            (true, false, false, false, false, true) => "scene_solid_pixel_shadow",
            (true, false, false, false, true, false) => "scene_solid_grid",
            (true, false, false, false, true, true) => "scene_solid_grid_pixel_shadow",
            (true, false, false, true, false, false) => "scene_solid_centroid",
            (true, false, false, true, false, true) => "scene_solid_centroid_pixel_shadow",
            (true, false, false, true, true, false) => "scene_solid_centroid_grid",
            (true, false, false, true, true, true) => "scene_solid_centroid_grid_pixel_shadow",
            (true, false, true, false, false, false) => "scene_solid_bend",
            (true, false, true, false, false, true) => "scene_solid_bend_pixel_shadow",
            (true, false, true, false, true, false) => "scene_solid_bend_grid",
            (true, false, true, false, true, true) => "scene_solid_bend_grid_pixel_shadow",
            (true, false, true, true, false, false) => "scene_solid_bend_centroid",
            (true, false, true, true, false, true) => "scene_solid_bend_centroid_pixel_shadow",
            (true, false, true, true, true, false) => "scene_solid_bend_centroid_grid",
            (true, false, true, true, true, true) => "scene_solid_bend_centroid_grid_pixel_shadow",
            (true, true, false, false, false, false) => "scene_solid_srgb_palette",
            (true, true, false, false, false, true) => "scene_solid_srgb_palette_pixel_shadow",
            (true, true, false, false, true, false) => "scene_solid_srgb_palette_grid",
            (true, true, false, false, true, true) => "scene_solid_srgb_palette_grid_pixel_shadow",
            (true, true, false, true, false, false) => "scene_solid_srgb_palette_centroid",
            (true, true, false, true, false, true) => {
                "scene_solid_srgb_palette_centroid_pixel_shadow"
            }
            (true, true, false, true, true, false) => "scene_solid_srgb_palette_centroid_grid",
            (true, true, false, true, true, true) => {
                "scene_solid_srgb_palette_centroid_grid_pixel_shadow"
            }
            (true, true, true, false, false, false) => "scene_solid_srgb_palette_bend",
            (true, true, true, false, false, true) => "scene_solid_srgb_palette_bend_pixel_shadow",
            (true, true, true, false, true, false) => "scene_solid_srgb_palette_bend_grid",
            (true, true, true, false, true, true) => {
                "scene_solid_srgb_palette_bend_grid_pixel_shadow"
            }
            (true, true, true, true, false, false) => "scene_solid_srgb_palette_bend_centroid",
            (true, true, true, true, false, true) => {
                "scene_solid_srgb_palette_bend_centroid_pixel_shadow"
            }
            (true, true, true, true, true, false) => "scene_solid_srgb_palette_bend_centroid_grid",
            (true, true, true, true, true, true) => {
                "scene_solid_srgb_palette_bend_centroid_grid_pixel_shadow"
            }
        },
        vertex: Stage::of([
            when(bend, BEND_DEFINE),
            when(centroid, CENTROID_DEFINE),
            when(voxel_grid, VOXEL_GRID_DEFINE),
            when(pixel_shadow, PIXEL_SHADOW_DEFINE),
            DEFORMATION,
            when(bend, BEND),
            SCENE_VERT,
        ]),
        fragment: Stage::of([
            when(solid, SOLID_DEFINE),
            when(srgb_palette, SRGB_PALETTE_DEFINE),
            when(bend, BEND_DEFINE),
            when(centroid, CENTROID_DEFINE),
            when(voxel_grid, VOXEL_GRID_DEFINE),
            when(pixel_shadow, PIXEL_SHADOW_DEFINE),
            TRANSFER,
            when(voxel_grid, VOXEL_GRID),
            // REQ-SCN-082 transforms a voxel centre **like a vertex**, so the fragment stage of
            // that variant — and of no other — carries the animated deformation and, when the
            // frame bends, the curvature as well. Both are chunks the vertex stage already had;
            // their uniforms are program-wide, so nothing new is uploaded for them.
            when(pixel_shadow, DEFORMATION),
            when(pixel_shadow && bend, BEND),
            when(pixel_shadow, PIXEL_SHADOW),
            SCENE_FRAG,
        ]),
    }
}

/// The backdrop pass of AMD-049 REQ-SCN-075: the same vertex stage as [`scene`] — the backdrop
/// is placed like any other box, and bends with the scene (REQ-SCN-068) — with a fragment stage
/// of its own that carries the almost-matt reflection model.
///
/// It is a **separate program**, not an axis of [`scene`], for three reasons: the backdrop is one
/// opaque run a frame, so the choice is made once and never per fragment; it carries a flat
/// colour, so it needs neither the palette axis nor the atlas lookups nor the `discard`s; and
/// `vUv` is unread here, so the interpolation axis of AMD-058 does not apply either.
///
/// Two axes are left. The curvature, because the backdrop bends with the scene; and the voxel
/// grid, because AMD-055 §3 point 5 makes REQ-SCN-081 apply to the faces of the box, modulating
/// the albedo `c` **before** the model of REQ-SCN-075 so that a groove does not darken the
/// specular term. The pixel shadows of REQ-SCN-082 are **not** an axis here: that requirement is
/// about tile meshes and says so ("sans objet sur le backdrop").
///
/// The program is built the first time a frame asks for it, so a player with
/// `render.backdrop_lighting` off — or with no backdrop at all, which is every profile that does
/// not enable it in Lua — never compiles it.
#[must_use]
pub const fn backdrop(bend: bool, voxel_grid: bool) -> ProgramSource {
    ProgramSource {
        name: match (bend, voxel_grid) {
            (false, false) => "backdrop",
            (true, false) => "backdrop_bend",
            (false, true) => "backdrop_grid",
            (true, true) => "backdrop_bend_grid",
        },
        vertex: Stage::of([
            when(bend, BEND_DEFINE),
            when(voxel_grid, VOXEL_GRID_DEFINE),
            DEFORMATION,
            when(bend, BEND),
            SCENE_VERT,
            "",
            "",
            "",
            "",
            "",
        ]),
        fragment: Stage::of([
            when(bend, BEND_DEFINE),
            when(voxel_grid, VOXEL_GRID_DEFINE),
            TRANSFER,
            when(voxel_grid, VOXEL_GRID),
            BACKDROP_FRAG,
            "",
            "",
            "",
            "",
            "",
        ]),
    }
}

/// The floor mask of AMD-055 REQ-SCN-084: the same vertex stage as [`scene`], so the mask is laid
/// down by exactly the geometry, the deformation, the placement, the curvature and the wall bias of
/// AMD-057 that drew the frame — and therefore cannot drift from the depth it is tested against.
/// The fragment stage is the `+Y ≥ 0,9` test of the floor definition and nothing else.
///
/// A **separate program**, not an axis of [`scene`], for the reason [`backdrop`] is one and more:
/// the mask writes one channel and reads no texture, so none of `solid`, `srgb_palette`,
/// `centroid`, `voxel_grid` and `pixel_shadow` applies to it. One axis is left, the curvature,
/// because the mask has to land where the bent frame drew.
///
/// Built the first time a frame asks for the reflections, and released with them.
#[must_use]
pub const fn floor_mask(bend: bool) -> ProgramSource {
    ProgramSource {
        name: if bend {
            "floor_mask_bend"
        } else {
            "floor_mask"
        },
        vertex: Stage::of([
            when(bend, BEND_DEFINE),
            FLOOR_MASK_DEFINE,
            DEFORMATION,
            when(bend, BEND),
            SCENE_VERT,
        ]),
        fragment: Stage::of([include_str!("../shaders/floor_mask.frag")]),
    }
}

/// The shadow pass: the same placement, depth only. `bend` bends it too, so that the shadows
/// follow the curved surface instead of sliding off it (REQ-SCN-068).
#[must_use]
pub const fn shadow(bend: bool) -> ProgramSource {
    ProgramSource {
        name: if bend { "shadow_bend" } else { "shadow" },
        vertex: Stage::of([
            when(bend, BEND_DEFINE),
            DEFORMATION,
            when(bend, BEND),
            SHADOW_VERT,
            "",
            "",
            "",
            "",
            "",
            "",
        ]),
        fragment: Stage::of([SHADOW_FRAG, "", "", "", "", "", "", "", "", ""]),
    }
}

/// The frame palette linearised entry by entry ([`super::linear_palette`]).
pub const LINEAR_PALETTE: ProgramSource = ProgramSource {
    name: "linear_palette",
    vertex: Stage::of([
        include_str!("../shaders/resolve.vert"),
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
    ]),
    fragment: Stage::of([
        TRANSFER,
        include_str!("../shaders/linear_palette.frag"),
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
    ]),
};

/// The resolve pass: one full-screen triangle and the sRGB encode of SPEC-06 §2, applied once,
/// after the blending (AMD-021 R2-009).
pub const RESOLVE: ProgramSource = ProgramSource {
    name: "resolve",
    vertex: Stage::of([
        include_str!("../shaders/resolve.vert"),
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
    ]),
    fragment: Stage::of([
        TRANSFER,
        include_str!("../shaders/resolve.frag"),
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
    ]),
};

/// The full-screen triangle every screen-space pass draws, the same vertex stage the resolve pass
/// uses: three vertices built from `gl_VertexID`, with no attribute and no buffer, and a `vUv`
/// that runs 0 to 1 over the viewport.
const SCREEN_VERTEX: Stage = Stage::of([include_str!("../shaders/resolve.vert")]);

/// The ambient occlusion of AMD-055 REQ-SCN-083, into a one-channel target (`gl/screen_effects.rs`).
///
/// **Built the first time a frame asks for the occlusion**, and released with the module when no
/// frame wants a screen effect any more: in Classic, and in Enhanced at `spl.ao` = 0, these three
/// programs do not exist (point 1 and point 2 of the lot's brief, reported as
/// `RenderStatistics::style_programs`).
pub const AO: ProgramSource = ProgramSource {
    name: "ao",
    vertex: SCREEN_VERTEX,
    fragment: Stage::of([SCREEN_DEPTH, include_str!("../shaders/ao.frag")]),
};

/// The depth-aware smoothing of the occlusion (REQ-SCN-083).
pub const AO_BLUR: ProgramSource = ProgramSource {
    name: "ao_blur",
    vertex: SCREEN_VERTEX,
    fragment: Stage::of([SCREEN_DEPTH, include_str!("../shaders/ao_blur.frag")]),
};

/// The `1 − O` multiply of REQ-SCN-083, blended into the linear buffer.
pub const AO_COMPOSITE: ProgramSource = ProgramSource {
    name: "ao_composite",
    vertex: SCREEN_VERTEX,
    fragment: Stage::of([include_str!("../shaders/ao_composite.frag")]),
};

/// The screen-space reflected ray of AMD-055 REQ-SCN-084, into an `RGBA16F` target.
///
/// **Built the first time a frame asks for the reflections**, and released when no frame wants
/// them any more: in Classic, and in Enhanced at `spl.reflections` = 0, neither this nor
/// [`SSR_COMPOSITE`] nor [`floor_mask`] exists (point 1 and point 2 of the lot's brief).
pub const SSR: ProgramSource = ProgramSource {
    name: "ssr",
    vertex: SCREEN_VERTEX,
    fragment: Stage::of([SCREEN_DEPTH, include_str!("../shaders/ssr.frag")]),
};

/// The `(1 − i) · surface + i · reflet` of REQ-SCN-084, blended into the linear buffer.
pub const SSR_COMPOSITE: ProgramSource = ProgramSource {
    name: "ssr_composite",
    vertex: SCREEN_VERTEX,
    fragment: Stage::of([include_str!("../shaders/ssr_composite.frag")]),
};

const SKY_VERTEX: Stage = Stage::of([
    include_str!("../shaders/sky.vert"),
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
]);

const GRADIENT: &str = include_str!("../shaders/sky_gradient.frag");
const FANTASY: &str = include_str!("../shaders/sky_fantasy.frag");

/// The sky fields, computed (`procedural`) or sampled from the cubemaps of an asset folder, and
/// the body of one of the two skyboxes.
const fn sky_fragment(procedural: bool, body: &'static str) -> Stage {
    Stage::of([
        TRANSFER,
        SKY_NOISE,
        if procedural {
            SKY_PROCEDURAL
        } else {
            SKY_TEXTURED
        },
        SKY_BODIES,
        body,
        "",
        "",
        "",
        "",
        "",
    ])
}

/// The Gradient sky of REQ-SCN-050, with its fields computed (`procedural`) or sampled from the
/// cubemaps of an asset folder.
#[must_use]
pub const fn gradient(procedural: bool) -> ProgramSource {
    ProgramSource {
        name: "skybox-gradient",
        vertex: SKY_VERTEX,
        fragment: sky_fragment(procedural, GRADIENT),
    }
}

/// The Fantasy nebula sky of REQ-SCN-051, with its fields computed or sampled.
#[must_use]
pub const fn fantasy(procedural: bool) -> ProgramSource {
    ProgramSource {
        name: "skybox-fantasy",
        vertex: SKY_VERTEX,
        fragment: sky_fragment(procedural, FANTASY),
    }
}

/// The full source of one stage for a desktop (`embedded == false`) or GLES context.
#[must_use]
pub fn assemble(stage: &Stage, fragment: bool, embedded: bool) -> String {
    let mut source = String::from(if embedded {
        "#version 300 es\nprecision highp float;\nprecision highp int;\n"
    } else {
        "#version 330 core\n"
    });
    if embedded && fragment {
        source.push_str(
            "precision highp sampler2D;\nprecision highp usampler2D;\nprecision highp samplerCube;\n",
        );
    }

    // The "×2" of REQ-SCN-051 as the sRGB curve gives it (AMD-019, Q-045).
    source.push_str("#define ZAX_SRGB_DOUBLE ");
    source.push_str(&sky::SRGB_DOUBLE.to_string());
    source.push('\n');
    for part in stage.parts {
        // An unused slot contributes nothing, not even a blank line: the assembled source of a
        // variant that leaves a define out has to be byte-identical to what it was before the
        // variant existed, or every program in the on-disk cache is rebuilt for nothing.
        if part.is_empty() {
            continue;
        }
        source.push('\n');
        source.push_str(part);
    }

    source
}
