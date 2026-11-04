//! The wgpu backend of the renderer (feature `wgpu`, desktop only): the same frame as the GL
//! [`crate::SceneRenderer`], drawn through Vulkan or DirectX 12. [`headless`] makes a device
//! and a target without any window, for the harness and the tests.

pub mod headless;
mod msvo;
mod preprocess;
mod renderer;
mod resources;
mod screen_effects;

pub use renderer::{WgpuRenderError, WgpuSceneRenderer, WgpuTarget, BATCHING_FEATURES};

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
