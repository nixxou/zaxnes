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
//! AMD-021 R2-009: every colour these shaders write is *linear*. The blending of REQ-SCN-040
//! therefore happens on linear colours, and the sRGB encode is the job of the resolve pass, the
//! single pass that copies the linear buffer into the host framebuffer.

use crate::sky;

/// The sRGB transfer functions of SPEC-06 §2, shared by every shader.
const TRANSFER: &str = include_str!("../shaders/transfer.glsl");

/// The animated deformation of SPEC-04B REQ-E3D-568, applied in the object's local frame before
/// the scale and the rotation (DEC-04B-Q2, REQ-SCN-043).
const DEFORMATION: &str = include_str!("../shaders/deformation.glsl");

/// The noise library of the procedural sky (AMD-024 REQ-PUB-003).
const SKY_NOISE: &str = include_str!("../shaders/sky_noise.glsl");

/// The five sky fields read from the cubemaps SPEC-06 names.
const SKY_TEXTURED: &str = include_str!("../shaders/sky_textured.glsl");

/// The same five fields, computed from the direction alone: no sampler, no file.
const SKY_PROCEDURAL: &str = include_str!("../shaders/sky_procedural.glsl");

/// The sun disc, the moon disc and the cloud layer of REQ-PUB-003, off by default (Q-064).
const SKY_BODIES: &str = include_str!("../shaders/sky_bodies.glsl");

/// One stage of a program: the chunks it is made of, in order, after the header.
#[derive(Debug, Clone, Copy)]
pub struct Stage {
    /// The chunks.
    pub parts: &'static [&'static str],
}

/// The vertex and fragment stages of one program.
#[derive(Debug, Clone, Copy)]
pub struct ProgramSource {
    /// Name used in error messages.
    pub name: &'static str,
    /// Vertex stage.
    pub vertex: Stage,
    /// Fragment stage.
    pub fragment: Stage,
}

/// The scene pass: albedo of REQ-SCN-034, visible-rectangle clipping of REQ-SCN-036, hard
/// shadow of REQ-SCN-030/031 and the Lambert shading of REQ-SCN-028. The scene frame is
/// left-handed (REQ-E3D-400), so the clip position is built from `(x, y, −z)` (REQ-SCN-001).
pub const SCENE: ProgramSource = ProgramSource {
    name: "scene",
    vertex: Stage {
        parts: &[DEFORMATION, include_str!("../shaders/scene.vert")],
    },
    fragment: Stage {
        parts: &[TRANSFER, include_str!("../shaders/scene.frag")],
    },
};

/// The shadow pass: the same placement, depth only.
pub const SHADOW: ProgramSource = ProgramSource {
    name: "shadow",
    vertex: Stage {
        parts: &[DEFORMATION, include_str!("../shaders/shadow.vert")],
    },
    fragment: Stage {
        parts: &[include_str!("../shaders/shadow.frag")],
    },
};

/// The resolve pass: one full-screen triangle and the sRGB encode of SPEC-06 §2, applied once,
/// after the blending (AMD-021 R2-009).
pub const RESOLVE: ProgramSource = ProgramSource {
    name: "resolve",
    vertex: Stage {
        parts: &[include_str!("../shaders/resolve.vert")],
    },
    fragment: Stage {
        parts: &[TRANSFER, include_str!("../shaders/resolve.frag")],
    },
};

const SKY_VERTEX: Stage = Stage {
    parts: &[include_str!("../shaders/sky.vert")],
};

const GRADIENT: &str = include_str!("../shaders/sky_gradient.frag");
const FANTASY: &str = include_str!("../shaders/sky_fantasy.frag");

/// The Gradient sky of REQ-SCN-050, with its fields computed (`procedural`) or sampled from the
/// cubemaps of an asset folder.
pub const fn gradient(procedural: bool) -> ProgramSource {
    ProgramSource {
        name: "skybox-gradient",
        vertex: SKY_VERTEX,
        fragment: Stage {
            parts: if procedural {
                &[TRANSFER, SKY_NOISE, SKY_PROCEDURAL, SKY_BODIES, GRADIENT]
            } else {
                &[TRANSFER, SKY_NOISE, SKY_TEXTURED, SKY_BODIES, GRADIENT]
            },
        },
    }
}

/// The Fantasy nebula sky of REQ-SCN-051, with its fields computed or sampled.
pub const fn fantasy(procedural: bool) -> ProgramSource {
    ProgramSource {
        name: "skybox-fantasy",
        vertex: SKY_VERTEX,
        fragment: Stage {
            parts: if procedural {
                &[TRANSFER, SKY_NOISE, SKY_PROCEDURAL, SKY_BODIES, FANTASY]
            } else {
                &[TRANSFER, SKY_NOISE, SKY_TEXTURED, SKY_BODIES, FANTASY]
            },
        },
    }
}

/// The full source of one stage for a desktop (`embedded == false`) or GLES context.
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
        source.push('\n');
        source.push_str(part);
    }

    source
}
