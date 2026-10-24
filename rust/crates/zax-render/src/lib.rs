//! zax-render: the OpenGL renderer of SPEC-06 (port of `src/Zax.Render`).
//!
//! It draws one frame of the 3D scene — shadow map, background or skybox, opaque objects,
//! sorted transparent objects, then the HUD — into a framebuffer the host provides, on an
//! OpenGL 3.3 core or OpenGL ES 3.0 context the host owns and hands over as a
//! `glow::Context` (REQ-SCN-056, REQ-SCN-057, REQ-ARCH-030). Everything that needs no GPU —
//! camera, controls, colour science, scene state — is plain safe code; the GL calls are
//! confined to [`gl::api`].

pub mod assets;
pub mod camera;
pub mod color;
pub mod controls;
mod frame_plan;
pub mod geometry;
mod gl;
pub mod interop;
pub mod numerics;
pub mod occlusion;
#[cfg(feature = "offscreen")]
pub mod offscreen;
mod renderer;
pub mod scene;
pub mod shadow_volume;
pub mod sky;
pub mod smoothing;
pub mod statistics;
pub mod target;

pub use gl::api::GlError;
pub use renderer::{RenderError, SceneRenderer};

/// Stable identifier of this module.
pub const NAME: &str = "Zax.Render";

/// Short human-readable role of this module.
pub const ROLE: &str = "OpenGL 3.3 core rendering into a host-provided target";

/// Major version of the OpenGL core profile this renderer targets.
pub const GL_MAJOR_VERSION: i32 = 3;

/// Minor version of the OpenGL core profile this renderer targets.
pub const GL_MINOR_VERSION: i32 = 3;
