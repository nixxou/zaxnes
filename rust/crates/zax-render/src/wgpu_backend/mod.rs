//! The wgpu backend of the renderer (feature `wgpu`, desktop only): the same frame as the GL
//! [`crate::SceneRenderer`], drawn through Vulkan or DirectX 12. [`headless`] makes a device
//! and a target without any window, for the harness and the tests.

pub mod headless;
mod msvo;
mod preprocess;
mod renderer;
mod resources;
mod screen_effects;
mod smaa;

pub use renderer::{WgpuRenderError, WgpuSceneRenderer, WgpuTarget, BATCHING_FEATURES};

/// The four WGSL sources of the depth-detecting SMAA of AMD-060 REQ-SCN-085: the shared chunk,
/// then the three passes.
///
/// Public for the reason [`expand_wgsl`] is: AMD-060 §4 asks that the three passes be translated
/// from the reference by reading what they compute, and what a port of this shape can get wrong —
/// the sign of `SMAA_RT_METRICS.y`, an integer texel offset, a point sampler where the reference
/// wants a filtered one — is read on the source. `tests/amd060_smaa_tests.rs` reads them.
#[must_use]
pub fn smaa_sources() -> [&'static str; 4] {
    smaa::sources()
}

/// `source` with the variant blocks `defines` does not select removed, as a module of this backend
/// is composed (`preprocess`).
///
/// Public so that a test can assert what a variant does **not** carry — which for AMD-049
/// REQ-SCN-075 and AMD-055 REQ-SCN-081 is as much the requirement as what it does: no tile
/// fragment may see the backdrop's reflection model, and the OpenGL side has the same assertion
/// over `shaders::assemble`.
#[must_use]
pub fn expand_wgsl(source: &str, defines: &[&str]) -> String {
    preprocess::expand(source, defines).wgsl
}
