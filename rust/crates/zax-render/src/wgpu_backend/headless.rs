//! A wgpu device and an RGBA8 target without any window, for the harness and the tests: the
//! twin of the offscreen GL context.

use super::renderer::{WgpuRenderError, WgpuSceneRenderer, WgpuTarget};

/// The format of the headless target, the RGBA8 of the offscreen GL framebuffer.
const TARGET_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// The graphics API wgpu draws through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuApi {
    Vulkan,
    Dx12,
}

impl GpuApi {
    const fn backends(self) -> wgpu::Backends {
        match self {
            Self::Vulkan => wgpu::Backends::VULKAN,
            Self::Dx12 => wgpu::Backends::DX12,
        }
    }
}

/// Why the headless device could not be made.
#[derive(Debug, thiserror::Error)]
pub enum HeadlessError {
    #[error("no {0:?} adapter: {1}")]
    Adapter(GpuApi, wgpu::RequestAdapterError),
    #[error("the device could not be created: {0}")]
    Device(#[from] wgpu::RequestDeviceError),
    #[error(transparent)]
    Render(#[from] WgpuRenderError),
}

/// A device, a renderer and a target of `width` × `height` pixels.
#[derive(Debug)]
pub struct HeadlessGpu {
    renderer: WgpuSceneRenderer,
    target: wgpu::Texture,
    view: wgpu::TextureView,
    width: u32,
    height: u32,
    adapter: wgpu::AdapterInfo,
}

impl HeadlessGpu {
    /// Opens the first adapter of `api` and makes the renderer and the target.
    ///
    /// # Errors
    ///
    /// When no adapter of `api` is there or the device cannot be created.
    pub fn new(api: GpuApi, width: u32, height: u32) -> Result<Self, HeadlessError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: api.backends(),
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
        }))
        .map_err(|error| HeadlessError::Adapter(api, error))?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("zax headless"),
                required_features: adapter.features()
                    & wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                required_limits: wgpu::Limits::default().using_resolution(adapter.limits()),
                ..wgpu::DeviceDescriptor::default()
            }))?;

        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("zax headless target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TARGET_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let info = adapter.get_info();
        let renderer = WgpuSceneRenderer::new(&adapter, device, queue, TARGET_FORMAT)?;
        Ok(Self {
            renderer,
            target,
            view,
            width,
            height,
            adapter: info,
        })
    }

    /// The renderer.
    pub const fn renderer(&mut self) -> &mut WgpuSceneRenderer {
        &mut self.renderer
    }

    /// The renderer, read only.
    #[must_use]
    pub const fn renderer_ref(&self) -> &WgpuSceneRenderer {
        &self.renderer
    }

    /// The target to render into.
    #[must_use]
    pub const fn target(&self) -> WgpuTarget<'_> {
        WgpuTarget {
            view: &self.view,
            width: self.width,
            height: self.height,
        }
    }

    /// Draws one frame of `scene` into the target.
    ///
    /// # Errors
    ///
    /// As [`WgpuSceneRenderer::render`].
    pub fn render(
        &mut self,
        scene: &crate::scene::SceneState,
        frame: &crate::scene::SceneFrame<'_>,
    ) -> Result<(), WgpuRenderError> {
        let target = WgpuTarget {
            view: &self.view,
            width: self.width,
            height: self.height,
        };
        self.renderer.render(target, scene, frame)
    }

    /// What the adapter is, for the reports.
    #[must_use]
    pub const fn adapter(&self) -> &wgpu::AdapterInfo {
        &self.adapter
    }

    /// Waits until the GPU has executed everything submitted so far.
    pub fn finish(&self) {
        let _ = self.renderer.device().poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        });
    }

    /// Reads the target back as rows of RGBA bytes, the top row first (as
    /// `OffscreenContext::read_pixels`).
    #[must_use]
    pub fn read_pixels(&self) -> Vec<u8> {
        let device = self.renderer.device();
        let row = self.width * 4;
        let padded = row.next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("zax readback"),
            size: u64::from(padded) * u64::from(self.height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("zax readback"),
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        self.renderer.queue().submit([encoder.finish()]);

        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        self.finish();
        let mapped = slice.get_mapped_range();
        let mut pixels = Vec::with_capacity((row * self.height) as usize);
        for line in mapped.chunks_exact(padded as usize) {
            pixels.extend_from_slice(&line[..row as usize]);
        }
        drop(mapped);
        buffer.unmap();
        pixels
    }
}
