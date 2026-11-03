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
