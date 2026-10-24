//! The window drawn through wgpu (feature `wgpu`, desktop only), the twin of the GL default
//! framebuffer: a surface on the window, the picture of the scene stretched under the menu bar
//! with bilinear filtering (REQ-SCN-063, as the GL blit), the Dear `ImGui` interface over it,
//! and the present that the GL swap was. The surface is non-sRGB, as the GL window: the scene
//! picture already holds sRGB values and the interface colours are written as they are.

use std::mem::size_of;

use imgui::{DrawCmd, DrawCmdParams, DrawData, DrawVert, FontAtlas, TextureId};

use crate::PlatformError;

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

/// The id the font atlas carries; no other texture is ever drawn by the interface.
const FONT_TEXTURE_ID: usize = 1;

/// The scene picture and the height of the window it fills, from the bottom up (the menu bar
/// keeps the rest).
#[derive(Debug, Clone, Copy)]
pub struct SceneLayer<'a> {
    pub view: &'a wgpu::TextureView,
    pub height: u32,
}

/// The surface, the device and what draws the window.
pub struct WgpuDisplay {
    // Field order is drop order: everything made on the device goes before the surface.
    compositor: Compositor,
    interface: InterfaceRenderer,
    pending: Option<wgpu::SurfaceTexture>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    present_modes: Vec<wgpu::PresentMode>,
    can_copy: bool,
    /// The names of the adapters that can draw on the window, for the log.
    adapters: Vec<String>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    adapter: wgpu::Adapter,
}

impl WgpuDisplay {
    /// Makes the surface on `window` and the device for `api`.
    ///
    /// # Safety
    ///
    /// The display must be dropped before `window` (the surface holds its handle).
    ///
    /// # Errors
    ///
    /// When the surface, the adapter or the device cannot be made.
    pub(crate) unsafe fn new(
        window: &sdl2::video::Window,
        api: GpuApi,
        vsync: bool,
    ) -> Result<Self, PlatformError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: api.backends(),
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        // SAFETY: the handles are read from a live window; the caller keeps it alive for as
        // long as the display lives.
        let target = unsafe { wgpu::SurfaceTargetUnsafe::from_display_and_window(window, window) }
            .map_err(|error| PlatformError::new(format!("no window handle: {error}")))?;
        // SAFETY: the caller keeps the window alive for as long as the display (and so the
        // surface) lives.
        let surface = unsafe { instance.create_surface_unsafe(target) }
            .map_err(|error| PlatformError::new(format!("no {api:?} surface: {error}")))?;
        // The adapter the system prefers for a high-performance program, unless `ZAXNES_GPU`
        // names another (a part of its name, as the log lists them).
        let candidates: Vec<wgpu::Adapter> =
            pollster::block_on(instance.enumerate_adapters(api.backends()))
                .into_iter()
                .filter(|adapter| adapter.is_surface_supported(&surface))
                .collect();
        let mut names: Vec<String> = Vec::new();
        for adapter in &candidates {
            // DirectX 12 lists an adapter once per output: each name is said once.
            let name = adapter.get_info().name;
            if !names.contains(&name) {
                names.push(name);
            }
        }
        let wanted = std::env::var("ZAXNES_GPU")
            .ok()
            .map(|name| name.to_lowercase())
            .filter(|name| !name.is_empty());
        let chosen = wanted.as_deref().and_then(|wanted| {
            candidates
                .into_iter()
                .find(|adapter| adapter.get_info().name.to_lowercase().contains(wanted))
        });
        let adapter = match chosen {
            Some(adapter) => adapter,
            None => pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            }))
            .map_err(|error| PlatformError::new(format!("no {api:?} adapter: {error}")))?,
        };
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("zax display"),
            required_features: adapter.features()
                & wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
            required_limits: wgpu::Limits::default().using_resolution(adapter.limits()),
            ..wgpu::DeviceDescriptor::default()
        }))
        .map_err(|error| PlatformError::new(format!("no {api:?} device: {error}")))?;

        let capabilities = surface.get_capabilities(&adapter);
        let format = [
            wgpu::TextureFormat::Bgra8Unorm,
            wgpu::TextureFormat::Rgba8Unorm,
        ]
        .into_iter()
        .find(|format| capabilities.formats.contains(format))
        .or_else(|| capabilities.formats.first().copied())
        .ok_or_else(|| PlatformError::new("the surface offers no format"))?;
        let can_copy = capabilities.usages.contains(wgpu::TextureUsages::COPY_SRC);
        let (width, height) = window.drawable_size();
        let mut config = wgpu::SurfaceConfiguration {
            usage: if can_copy {
                wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC
            } else {
                wgpu::TextureUsages::RENDER_ATTACHMENT
            },
            format,
            width: width.max(1),
            height: height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: Vec::new(),
        };
        config.present_mode = present_mode(&capabilities.present_modes, vsync);
        surface.configure(&device, &config);

        Ok(Self {
            compositor: Compositor::new(&device, format),
            interface: InterfaceRenderer::new(&device, format),
            pending: None,
            surface,
            config,
            present_modes: capabilities.present_modes,
            can_copy,
            adapters: names,
            device,
            queue,
            adapter,
        })
    }

    /// The device the scene renderer draws with.
    #[must_use]
    pub const fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// The queue the scene renderer submits to.
    #[must_use]
    pub const fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// The adapter, for the sample counts the scene renderer can use.
    #[must_use]
    pub const fn adapter(&self) -> &wgpu::Adapter {
        &self.adapter
    }

    /// One line for the start-up log: the API, the adapter, the driver, the surface, and the
    /// adapters `ZAXNES_GPU` can pick from.
    #[must_use]
    pub fn report(&self) -> String {
        let info = self.adapter.get_info();
        format!(
            "wgpu {:?} on '{}' ({:?}, driver '{} {}'), surface {:?} {:?}; adapters: {} \
             (ZAXNES_GPU=<part of a name> picks one)",
            info.backend,
            info.name,
            info.device_type,
            info.driver,
            info.driver_info,
            self.config.format,
            self.config.present_mode,
            self.adapters.join(", ")
        )
    }

    /// The renderer name the statistics show.
    #[must_use]
    pub fn adapter_name(&self) -> String {
        let info = self.adapter.get_info();
        format!("{} ({:?})", info.name, info.backend)
    }

    /// Presents with vsync or without (the swap interval of the GL window).
    pub(crate) fn set_vsync(&mut self, vsync: bool) {
        let mode = present_mode(&self.present_modes, vsync);
        if mode != self.config.present_mode {
            self.config.present_mode = mode;
            self.surface.configure(&self.device, &self.config);
        }
    }

    /// Uploads the font atlas of the interface and gives it its texture id.
    pub fn upload_fonts(&mut self, fonts: &mut FontAtlas) {
        self.interface
            .upload_fonts(&self.device, &self.queue, fonts);
    }

    /// Draws a frame of `width` x `height` pixels: black, the scene layer if there is one, then
    /// the interface. The frame is presented by [`Self::present`].
    pub fn draw(
        &mut self,
        (width, height): (u32, u32),
        scene: Option<SceneLayer<'_>>,
        interface: Option<&DrawData>,
    ) {
        let (width, height) = (width.max(1), height.max(1));
        if (width, height) != (self.config.width, self.config.height) {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
        }
        let Some(frame) = self.acquire() else {
            return;
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("zax window"),
            });
        if let Some(interface) = interface {
            self.interface.prepare(&self.device, &self.queue, interface);
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("zax window"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if let Some(scene) = scene {
                self.compositor
                    .draw(&self.device, &mut pass, scene, (width, height));
            }
            if let Some(interface) = interface {
                self.interface.draw(&mut pass, interface, (width, height));
            }
        }
        self.queue.submit([encoder.finish()]);
        self.pending = Some(frame);
    }

    /// Presents the frame [`Self::draw`] drew.
    pub fn present(&mut self) {
        if let Some(frame) = self.pending.take() {
            frame.present();
        }
    }

    /// The frame [`Self::draw`] drew, as RGB rows top to bottom (the command-line captures), or
    /// `None` when the surface cannot be read.
    #[must_use]
    pub fn read_rgb(&self) -> Option<Vec<u8>> {
        let frame = self.pending.as_ref().filter(|_| self.can_copy)?;
        let (width, height) = (self.config.width, self.config.height);
        let rgba = read_rgba(&self.device, &self.queue, &frame.texture, width, height);
        let bgra = matches!(
            self.config.format,
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
        );
        Some(
            rgba.chunks_exact(4)
                .flat_map(|pixel| {
                    if bgra {
                        [pixel[2], pixel[1], pixel[0]]
                    } else {
                        [pixel[0], pixel[1], pixel[2]]
                    }
                })
                .collect(),
        )
    }

    /// The next surface texture, configured again once if the surface went out of date; `None`
    /// skips the frame.
    fn acquire(&mut self) -> Option<wgpu::SurfaceTexture> {
        self.pending = None;
        for _ in 0..2 {
            match self.surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(frame)
                | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => return Some(frame),
                wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                    self.surface.configure(&self.device, &self.config);
                }
                _ => return None,
            }
        }
        None
    }
}

/// Fifo with vsync; without, the first of Immediate and Mailbox the surface offers.
fn present_mode(available: &[wgpu::PresentMode], vsync: bool) -> wgpu::PresentMode {
    if vsync {
        return wgpu::PresentMode::Fifo;
    }
    [wgpu::PresentMode::Immediate, wgpu::PresentMode::Mailbox]
        .into_iter()
        .find(|mode| available.contains(mode))
        .unwrap_or(wgpu::PresentMode::Fifo)
}

/// Reads the whole of `texture` (4 bytes a texel) back as rows top to bottom.
#[must_use]
pub fn read_rgba(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let row = width * 4;
    let padded = row.next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("zax readback"),
        size: u64::from(padded) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("zax readback"),
    });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([encoder.finish()]);
    let slice = buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    let _ = device.poll(wgpu::PollType::Wait {
        submission_index: None,
        timeout: None,
    });
    let mapped = slice.get_mapped_range();
    let mut pixels = Vec::with_capacity((row * height) as usize);
    for line in mapped.chunks_exact(padded as usize) {
        pixels.extend_from_slice(&line[..row as usize]);
    }
    drop(mapped);
    buffer.unmap();
    pixels
}

// --------------------------------------------------------------------- compositor

const COMPOSITE: &str = "
@group(0) @binding(0) var picture: texture_2d<f32>;
@group(0) @binding(1) var picture_sampler: sampler;

struct Out {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> Out {
    let corner = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    var out: Out;
    out.position = vec4<f32>(corner * 2.0 - 1.0, 0.0, 1.0);
    // The picture's first row is its top.
    out.uv = vec2<f32>(corner.x, 1.0 - corner.y);
    return out;
}

@fragment
fn fs_main(in: Out) -> @location(0) vec4<f32> {
    return textureSample(picture, picture_sampler, in.uv);
}
";

/// Stretches the scene picture over its part of the window, filtered bilinearly.
struct Compositor {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}

impl Compositor {
    fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("zax composite"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("zax composite"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("zax composite"),
            source: wgpu::ShaderSource::Wgsl(COMPOSITE.into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("zax composite"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("zax composite"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..wgpu::SamplerDescriptor::default()
        });
        Self {
            pipeline,
            layout,
            sampler,
        }
    }

    #[allow(clippy::cast_precision_loss)] // Window sizes are far below 2^24.
    fn draw(
        &self,
        device: &wgpu::Device,
        pass: &mut wgpu::RenderPass<'_>,
        scene: SceneLayer<'_>,
        (width, height): (u32, u32),
    ) {
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("zax composite"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(scene.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        let layer = scene.height.clamp(1, height);
        pass.set_viewport(
            0.0,
            (height - layer) as f32,
            width as f32,
            layer as f32,
            0.0,
            1.0,
        );
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.draw(0..3, 0..1);
        pass.set_viewport(0.0, 0.0, width as f32, height as f32, 0.0, 1.0);
    }
}

// --------------------------------------------------------------------- interface

const INTERFACE: &str = "
struct Projection {
    matrix: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> projection: Projection;
@group(0) @binding(1) var atlas: texture_2d<f32>;
@group(0) @binding(2) var atlas_sampler: sampler;

struct Out {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
}

@vertex
fn vs_main(
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
) -> Out {
    var out: Out;
    out.position = projection.matrix * vec4<f32>(position, 0.0, 1.0);
    out.uv = uv;
    out.color = color;
    return out;
}

@fragment
fn fs_main(in: Out) -> @location(0) vec4<f32> {
    return in.color * textureSample(atlas, atlas_sampler, in.uv);
}
";

/// Draws the Dear `ImGui` draw data (the GL `ImguiRenderer` of this crate, through wgpu).
struct InterfaceRenderer {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    projection: wgpu::Buffer,
    sampler: wgpu::Sampler,
    group: Option<wgpu::BindGroup>,
    vertices: Option<wgpu::Buffer>,
    indices: Option<wgpu::Buffer>,
    vertex_data: Vec<u8>,
    index_data: Vec<u8>,
}

impl InterfaceRenderer {
    #[allow(clippy::too_many_lines)] // The layout, the shader and the pipeline, made once.
    fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("zax interface"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(64),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("zax interface"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("zax interface"),
            source: wgpu::ShaderSource::Wgsl(INTERFACE.into()),
        });
        let attributes = wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Unorm8x4];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("zax interface"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: size_of::<DrawVert>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &attributes,
                }],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    // As the GL renderer: alpha blending on the colour, the separate factors
                    // on the alpha.
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        Self {
            pipeline,
            layout,
            projection: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("zax interface projection"),
                size: 64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("zax interface"),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..wgpu::SamplerDescriptor::default()
            }),
            group: None,
            vertices: None,
            indices: None,
            vertex_data: Vec::new(),
            index_data: Vec::new(),
        }
    }

    fn upload_fonts(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, fonts: &mut FontAtlas) {
        let atlas = fonts.build_rgba32_texture();
        let size = wgpu::Extent3d {
            width: atlas.width,
            height: atlas.height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("zax interface font"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            atlas.data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(atlas.width * 4),
                rows_per_image: Some(atlas.height),
            },
            size,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("zax interface"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.projection.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        }));
        fonts.tex_id = TextureId::new(FONT_TEXTURE_ID);
    }

    /// Uploads the projection and every list's vertices and indices, one after the other.
    fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, data: &DrawData) {
        let [left, top] = data.display_pos;
        let [right, bottom] = [left + data.display_size[0], top + data.display_size[1]];
        let projection: [f32; 16] = [
            2.0 / (right - left),
            0.0,
            0.0,
            0.0,
            0.0,
            2.0 / (top - bottom),
            0.0,
            0.0,
            0.0,
            0.0,
            -1.0,
            0.0,
            (right + left) / (left - right),
            (top + bottom) / (bottom - top),
            0.0,
            1.0,
        ];
        let bytes: Vec<u8> = projection
            .iter()
            .flat_map(|value| value.to_ne_bytes())
            .collect();
        queue.write_buffer(&self.projection, 0, &bytes);

        self.vertex_data.clear();
        self.index_data.clear();
        for list in data.draw_lists() {
            self.vertex_data
                .extend_from_slice(as_bytes(list.vtx_buffer()));
            self.index_data
                .extend_from_slice(as_bytes(list.idx_buffer()));
        }
        // Buffer writes go by multiples of 4 bytes.
        self.vertex_data
            .resize(self.vertex_data.len().next_multiple_of(4).max(4), 0);
        self.index_data
            .resize(self.index_data.len().next_multiple_of(4).max(4), 0);
        upload(
            device,
            queue,
            &mut self.vertices,
            wgpu::BufferUsages::VERTEX,
            &self.vertex_data,
        );
        upload(
            device,
            queue,
            &mut self.indices,
            wgpu::BufferUsages::INDEX,
            &self.index_data,
        );
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )] // Pixel rectangles of a window.
    fn draw(&self, pass: &mut wgpu::RenderPass<'_>, data: &DrawData, (width, height): (u32, u32)) {
        let (Some(group), Some(vertices), Some(indices)) =
            (&self.group, &self.vertices, &self.indices)
        else {
            return;
        };
        let [scale_x, scale_y] = data.framebuffer_scale;
        let clip_offset = data.display_pos;
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, group, &[]);
        pass.set_vertex_buffer(0, vertices.slice(..));
        pass.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint16);
        let (mut vertex_base, mut index_base) = (0_i32, 0_u32);
        for list in data.draw_lists() {
            for command in list.commands() {
                let DrawCmd::Elements {
                    count,
                    cmd_params:
                        DrawCmdParams {
                            clip_rect,
                            idx_offset,
                            vtx_offset,
                            ..
                        },
                } = command
                else {
                    continue;
                };
                let left = ((clip_rect[0] - clip_offset[0]) * scale_x).max(0.0);
                let top = ((clip_rect[1] - clip_offset[1]) * scale_y).max(0.0);
                let right = ((clip_rect[2] - clip_offset[0]) * scale_x).min(width as f32);
                let bottom = ((clip_rect[3] - clip_offset[1]) * scale_y).min(height as f32);
                if right <= left || bottom <= top {
                    continue;
                }
                pass.set_scissor_rect(
                    left as u32,
                    top as u32,
                    (right - left) as u32,
                    (bottom - top) as u32,
                );
                let first = index_base + idx_offset as u32;
                pass.draw_indexed(
                    first..first + count as u32,
                    vertex_base + i32::try_from(vtx_offset).unwrap_or(i32::MAX),
                    0..1,
                );
            }
            vertex_base += i32::try_from(list.vtx_buffer().len()).unwrap_or(i32::MAX);
            index_base += list.idx_buffer().len() as u32;
        }
        pass.set_scissor_rect(0, 0, width, height);
    }
}

/// Writes `data` into `slot`, made again when too small.
fn upload(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    slot: &mut Option<wgpu::Buffer>,
    usage: wgpu::BufferUsages,
    data: &[u8],
) {
    let needed = data.len() as u64;
    if slot.as_ref().is_none_or(|buffer| buffer.size() < needed) {
        *slot = Some(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("zax interface data"),
            size: needed.next_power_of_two(),
            usage: usage | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
    }
    if let Some(buffer) = slot {
        queue.write_buffer(buffer, 0, data);
    }
}

/// The bytes of a draw list's vertex or index buffer.
fn as_bytes<T>(slice: &[T]) -> &[u8] {
    // SAFETY: the vertex and index types are plain data (`#[repr(C)]` floats and bytes, and
    // `u16`), and the byte view has the same lifetime as the slice.
    unsafe { std::slice::from_raw_parts(slice.as_ptr().cast(), std::mem::size_of_val(slice)) }
}
