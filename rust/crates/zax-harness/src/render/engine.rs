//! The renderer `render` draws with: the GL one on an offscreen context (default), or, built
//! with the `wgpu` feature, the wgpu one through Vulkan or DirectX 12 (`--backend`).

use zax_render::assets::SceneAssets;
use zax_render::camera::CameraView;
use zax_render::offscreen::{ContextApi, OffscreenContext};
use zax_render::scene::{SceneFrame, SceneState};
use zax_render::statistics::RenderStatistics;
use zax_render::SceneRenderer;

use crate::HarnessError;

/// The renderer `--backend` names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Backend {
    /// OpenGL 3.3 core, the renderer of every build.
    #[default]
    Gl,
    /// wgpu on Vulkan.
    Vulkan,
    /// wgpu on DirectX 12.
    Dx12,
}

impl Backend {
    /// Parses `gl`, `vulkan` or `dx12`.
    ///
    /// # Errors
    ///
    /// The message for any other value.
    pub fn parse(value: &str) -> Result<Self, String> {
        match value.to_ascii_lowercase().as_str() {
            "gl" | "opengl" => Ok(Self::Gl),
            "vulkan" => Ok(Self::Vulkan),
            "dx12" => Ok(Self::Dx12),
            _ => Err(format!(
                "--backend expects gl, vulkan or dx12, got '{value}'"
            )),
        }
    }
}

/// A renderer and the target it draws into.
#[allow(clippy::large_enum_variant)] // One engine per command, never moved around.
pub enum Engine {
    Gl {
        context: OffscreenContext,
        renderer: SceneRenderer,
    },
    #[cfg(feature = "wgpu")]
    Wgpu(Box<zax_render::wgpu_backend::headless::HeadlessGpu>),
}

impl Engine {
    /// Opens `backend` with a `width` × `height` target.
    ///
    /// # Errors
    ///
    /// When the context or the device cannot be created, or the backend is not built in.
    pub fn open(
        backend: Backend,
        width: i32,
        height: i32,
        assets: Option<Box<dyn SceneAssets>>,
        group_draws: bool,
    ) -> Result<Self, HarnessError> {
        if backend == Backend::Gl {
            let context = OffscreenContext::new(width, height, ContextApi::DesktopCore)?;
            let mut renderer = SceneRenderer::new(context.gl(), assets)?;
            renderer.set_group_opaque_draws(group_draws);
            return Ok(Self::Gl { context, renderer });
        }
        Self::open_wgpu(backend, width, height, assets.is_some(), group_draws)
    }

    #[cfg(feature = "wgpu")]
    fn open_wgpu(
        backend: Backend,
        width: i32,
        height: i32,
        assets: bool,
        group_draws: bool,
    ) -> Result<Self, HarnessError> {
        use zax_render::wgpu_backend::headless::{GpuApi, HeadlessGpu};

        if assets {
            return Err(HarnessError::Usage(
                "--assets (cubemap skies) is a GL-only option".to_owned(),
            ));
        }
        let api = if backend == Backend::Dx12 {
            GpuApi::Dx12
        } else {
            GpuApi::Vulkan
        };
        let size = |value: i32| u32::try_from(value).unwrap_or(1).max(1);
        let mut gpu = HeadlessGpu::new(api, size(width), size(height))
            .map_err(|error| HarnessError::Gpu(error.to_string()))?;
        gpu.renderer().set_group_opaque_draws(group_draws);
        Ok(Self::Wgpu(Box::new(gpu)))
    }

    #[cfg(not(feature = "wgpu"))]
    fn open_wgpu(backend: Backend, _: i32, _: i32, _: bool, _: bool) -> Result<Self, HarnessError> {
        Err(HarnessError::Usage(format!(
            "--backend {backend:?} needs a harness built with the wgpu feature"
        )))
    }

    /// Draws one frame.
    ///
    /// # Errors
    ///
    /// When the renderer fails.
    pub fn render(
        &mut self,
        scene: &SceneState,
        frame: &SceneFrame<'_>,
    ) -> Result<(), HarnessError> {
        match self {
            Self::Gl { context, renderer } => renderer.render(&context.target(), scene, frame)?,
            #[cfg(feature = "wgpu")]
            Self::Wgpu(gpu) => gpu
                .render(scene, frame)
                .map_err(|error| HarnessError::Gpu(error.to_string()))?,
        }
        Ok(())
    }

    /// Waits until the GPU has executed everything submitted.
    pub fn finish(&self) {
        match self {
            Self::Gl { context, .. } => context.finish(),
            #[cfg(feature = "wgpu")]
            Self::Wgpu(gpu) => gpu.finish(),
        }
    }

    /// The target as rows of RGBA bytes, the top row first.
    #[must_use]
    pub fn read_pixels(&self) -> Vec<u8> {
        match self {
            Self::Gl { context, .. } => context.read_pixels(),
            #[cfg(feature = "wgpu")]
            Self::Wgpu(gpu) => gpu.read_pixels(),
        }
    }

    /// What the last frame cost.
    #[must_use]
    pub fn statistics(&self) -> &RenderStatistics {
        match self {
            Self::Gl { renderer, .. } => renderer.statistics(),
            #[cfg(feature = "wgpu")]
            Self::Wgpu(gpu) => gpu.renderer_ref().statistics(),
        }
    }

    /// The camera the last frame used.
    #[must_use]
    pub fn last_view(&self) -> &CameraView {
        match self {
            Self::Gl { renderer, .. } => renderer.last_view(),
            #[cfg(feature = "wgpu")]
            Self::Wgpu(gpu) => gpu.renderer_ref().last_view(),
        }
    }
}
