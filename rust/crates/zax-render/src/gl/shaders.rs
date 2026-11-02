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

/// How many chunks a stage may hold. The largest are the sky fragments (five) and the scene
/// fragment with all four defines (six).
const SLOTS: usize = 6;

/// One stage of a program: the chunks it is made of, in order, after the header. Empty slots are
/// skipped, so a variant can leave a define out without changing the shape of the array.
#[derive(Debug, Clone, Copy)]
pub struct Stage {
    /// The chunks; `""` for an unused slot.
    pub parts: [&'static str; SLOTS],
}

impl Stage {
    /// A stage of the chunks of `parts`, in order.
    const fn of(parts: [&'static str; SLOTS]) -> Self {
        Self { parts }
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

/// The scene pass: albedo of REQ-SCN-034, visible-rectangle clipping of REQ-SCN-036, hard
/// shadow of REQ-SCN-030/031 and the Lambert shading of REQ-SCN-028. The scene frame is
/// left-handed (REQ-E3D-400), so the clip position is built from `(x, y, −z)` (REQ-SCN-001).
///
/// `solid` leaves out the `discard`s for the opaque draws that can never take them (a shader that
/// may discard keeps a tiled GPU from dropping the fragments hidden behind it early).
/// `srgb_palette` linearises the palette per fragment. `bend` compiles in the curvature of
/// REQ-SCN-067, which also places the vertex and transports the normal (REQ-SCN-068).
/// `centroid` interpolates `vUv` at the centroid of the covered samples (AMD-058), which is only
/// ever asked for when the buffer is multisampled.
///
/// The names of the eight variants without `centroid` are **unchanged**, because the name is the
/// key of the on-disk program cache: a player who does not use anti-aliasing keeps every binary
/// that cache already holds.
// The four switches are the variant axes the specification names, each independent of the others;
// an enum per axis would say less, not more.
#[allow(clippy::fn_params_excessive_bools)]
#[must_use]
pub const fn scene(solid: bool, srgb_palette: bool, bend: bool, centroid: bool) -> ProgramSource {
    ProgramSource {
        name: match (solid, srgb_palette, bend, centroid) {
            (false, false, false, false) => "scene",
            (true, false, false, false) => "scene_solid",
            (false, true, false, false) => "scene_srgb_palette",
            (true, true, false, false) => "scene_solid_srgb_palette",
            (false, false, true, false) => "scene_bend",
            (true, false, true, false) => "scene_solid_bend",
            (false, true, true, false) => "scene_srgb_palette_bend",
            (true, true, true, false) => "scene_solid_srgb_palette_bend",
            (false, false, false, true) => "scene_centroid",
            (true, false, false, true) => "scene_solid_centroid",
            (false, true, false, true) => "scene_srgb_palette_centroid",
            (true, true, false, true) => "scene_solid_srgb_palette_centroid",
            (false, false, true, true) => "scene_bend_centroid",
            (true, false, true, true) => "scene_solid_bend_centroid",
            (false, true, true, true) => "scene_srgb_palette_bend_centroid",
            (true, true, true, true) => "scene_solid_srgb_palette_bend_centroid",
        },
        vertex: Stage::of([
            when(bend, BEND_DEFINE),
            when(centroid, CENTROID_DEFINE),
            DEFORMATION,
            when(bend, BEND),
            SCENE_VERT,
            "",
        ]),
        fragment: Stage::of([
            when(solid, SOLID_DEFINE),
            when(srgb_palette, SRGB_PALETTE_DEFINE),
            when(bend, BEND_DEFINE),
            when(centroid, CENTROID_DEFINE),
            TRANSFER,
            SCENE_FRAG,
        ]),
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
        ]),
        fragment: Stage::of([SHADOW_FRAG, "", "", "", "", ""]),
    }
}

/// The frame palette linearised entry by entry ([`super::linear_palette`]).
pub const LINEAR_PALETTE: ProgramSource = ProgramSource {
    name: "linear_palette",
    vertex: Stage::of([include_str!("../shaders/resolve.vert"), "", "", "", "", ""]),
    fragment: Stage::of([
        TRANSFER,
        include_str!("../shaders/linear_palette.frag"),
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
    vertex: Stage::of([include_str!("../shaders/resolve.vert"), "", "", "", "", ""]),
    fragment: Stage::of([
        TRANSFER,
        include_str!("../shaders/resolve.frag"),
        "",
        "",
        "",
        "",
    ]),
};

const SKY_VERTEX: Stage = Stage::of([include_str!("../shaders/sky.vert"), "", "", "", "", ""]);

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
