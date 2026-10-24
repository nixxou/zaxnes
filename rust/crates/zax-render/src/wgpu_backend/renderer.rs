//! The wgpu renderer of SPEC-06, the twin of the GL [`crate::SceneRenderer`]: the same
//! [`FramePlan`], the same passes in the same order — shadow map, background or skybox, opaque
//! objects, sorted transparent objects, HUD — into the same linear half-float buffer, resolved
//! to sRGB into the target the host provides.
//!
//! What differs is only how the state reaches the GPU. Depth test, culling and winding are
//! baked into pipelines, so each combination the GL path switches to is a pipeline of its own.
//! Every mesh lives in one atlas, and the per-draw uniforms of the GL path become one block per
//! instanced run in a storage buffer that each instance finds through `run_of`: a pass binds
//! everything once, and a draw call is the call alone, which keeps wgpu's per-command cost low. The clip depth of GL (-1..1) is remapped to wgpu's (0..1) in the vertex shaders,
//! which keeps the stored depths, and the shadow map, rendered top down, is read at 1 - y.

use std::sync::Arc;

use zax_engine3d::draw_list::DeformParameters;
use zax_engine3d::geometry::texture::ObjectTexture;
use zax_engine3d::geometry::tile_mesh::{TileMesh, TriangleCountError};
use zax_engine3d::hashing::FastMap;
use zax_engine3d::palette::{FramePalette, Rgb24};
use zax_engine3d::vectors::Mat4;

use super::resources::{self, MeshHandle, MeshStore, TextureHandle, TextureStore, VERTEX_STRIDE};
use crate::camera::CameraView;
use crate::color;
use crate::frame_plan::{
    DrawItem, FixedMeshes, FramePlan, InstanceRun, Residency, SceneConstants, INSTANCE_BYTES,
};
use crate::geometry;
use crate::interop;
use crate::numerics::{Matrix4x4, Vector2, Vector3, Vector4};
use crate::scene::{SceneFrame, SceneState, SkyboxKind};
use crate::shadow_volume::ShadowVolume;
use crate::sky::{self, SkyBodySettings};
use crate::statistics::RenderStatistics;

/// The format of the linear scene buffer (REQ-SCN-035, AMD-021 R2-009).
const LINEAR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
/// The depth format of the scene buffer and of the shadow map: 24 bits, as the GL path's
/// `DEPTH_COMPONENT24`, so that coplanar faces tie the same way.
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;

/// Bytes of the per-draw block (`Draw` in common.wgsl).
const DRAW_BYTES: u64 = 304;
/// Bytes of the frame block (`Frame` in scene.wgsl).
const FRAME_BYTES: u64 = 224;
/// Bytes of the shadow frame block (`ShadowFrame` in shadow.wgsl).
const SHADOW_FRAME_BYTES: u64 = 80;
/// Bytes of the sky block (`Sky` in sky.wgsl).
const SKY_BYTES: u64 = 19 * 16;

/// Why the wgpu renderer could not draw a frame.
#[derive(Debug, thiserror::Error)]
pub enum WgpuRenderError {
    /// The target size is not usable.
    #[error("the target is {0} x {1} pixels")]
    Target(u32, u32),
    /// One of the renderer's fixed meshes is malformed.
    #[error("fixed mesh: {0}")]
    Mesh(#[from] TriangleCountError),
}

/// Where a frame is drawn: a texture view of the format the renderer was created for.
#[derive(Debug, Clone, Copy)]
pub struct WgpuTarget<'a> {
    pub view: &'a wgpu::TextureView,
    pub width: u32,
    pub height: u32,
}

/// The modes of the scene pipelines: the opaque pass, then the two passes of a blended object
/// (REQ-SCN-040).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum SceneMode {
    Opaque,
    DepthOnly,
    ColorEqual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct SceneKey {
    mode: SceneMode,
    cull: bool,
    mirrored: bool,
    samples: u32,
}

/// The linear buffer and its depth, multisampled when anti-aliasing is on.
#[derive(Debug)]
struct LinearTargets {
    width: u32,
    height: u32,
    samples: u32,
    resolved: wgpu::TextureView,
    multisampled: Option<wgpu::TextureView>,
    depth: wgpu::TextureView,
    resolve_group: wgpu::BindGroup,
}

#[derive(Debug)]
struct Layouts {
    frame: wgpu::BindGroupLayout,
    draw: wgpu::BindGroupLayout,
    textures: wgpu::BindGroupLayout,
    shadow_frame: wgpu::BindGroupLayout,
    sky: wgpu::BindGroupLayout,
    resolve: wgpu::BindGroupLayout,
    scene_pipeline: wgpu::PipelineLayout,
    shadow_pipeline: wgpu::PipelineLayout,
    sky_pipeline: wgpu::PipelineLayout,
    resolve_pipeline: wgpu::PipelineLayout,
}

/// A storage buffer that grows with what a frame writes into it.
#[derive(Debug)]
struct GrowingBuffer {
    buffer: wgpu::Buffer,
    size: u64,
}

impl GrowingBuffer {
    fn new(device: &wgpu::Device, label: &'static str, size: u64) -> Self {
        Self {
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            size,
        }
    }

    /// Makes room for `needed` bytes; true when the buffer was replaced.
    fn reserve(&mut self, device: &wgpu::Device, label: &'static str, needed: u64) -> bool {
        if needed <= self.size {
            return false;
        }
        *self = Self::new(device, label, needed.next_power_of_two());
        true
    }
}

/// The wgpu renderer of SPEC-06. The skies are the procedural ones of AMD-024 REQ-PUB-003: the
/// cubemaps of a user asset folder are a GL-only option.
#[derive(Debug)]
pub struct WgpuSceneRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    max_samples: u32,
    meshes: MeshStore,
    textures: TextureStore,
    unit_box: Arc<TileMesh>,
    picture_quad: Arc<TileMesh>,
    plan: FramePlan<MeshHandle, TextureHandle>,
    layouts: Layouts,
    scene_module: wgpu::ShaderModule,
    shadow_module: wgpu::ShaderModule,
    sky_module: wgpu::ShaderModule,
    scene_pipelines: FastMap<SceneKey, wgpu::RenderPipeline>,
    shadow_pipelines: FastMap<bool, wgpu::RenderPipeline>,
    sky_pipelines: FastMap<(bool, u32), wgpu::RenderPipeline>,
    resolve_pipeline: wgpu::RenderPipeline,
    frame_buffer: wgpu::Buffer,
    shadow_frame_buffer: wgpu::Buffer,
    sky_buffer: wgpu::Buffer,
    draws: GrowingBuffer,
    shadow_draws: GrowingBuffer,
    instances: GrowingBuffer,
    shadow_instances: GrowingBuffer,
    run_of: GrowingBuffer,
    shadow_run_of: GrowingBuffer,
    draw_data: Vec<u8>,
    shadow_draw_data: Vec<u8>,
    run_of_data: Vec<u8>,
    palette: wgpu::Texture,
    palette_view: wgpu::TextureView,
    palette_bytes: Vec<u8>,
    dummy_indexed: wgpu::TextureView,
    dummy_color: wgpu::TextureView,
    shadow_map: wgpu::TextureView,
    shadow_map_size: u32,
    shadow_map_requested: i32,
    linear: Option<LinearTargets>,
    frame_group: wgpu::BindGroup,
    draw_group: wgpu::BindGroup,
    shadow_frame_group: wgpu::BindGroup,
    shadow_draw_group: wgpu::BindGroup,
    sky_group: wgpu::BindGroup,
    texture_groups: FastMap<(Option<TextureHandle>, Option<TextureHandle>), wgpu::BindGroup>,
    texture_generation: u64,
    statistics: RenderStatistics,
    last_view: CameraView,
    group_opaque: bool,
}

/// The stores the plan asks for the resident meshes and textures.
struct WgpuResidency<'a> {
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    meshes: &'a mut MeshStore,
    textures: &'a mut TextureStore,
}

impl Residency for WgpuResidency<'_> {
    type Mesh = MeshHandle;
    type Texture = TextureHandle;
    type Error = std::convert::Infallible;

    fn mesh(&mut self, mesh: &Arc<TileMesh>) -> Result<Option<MeshHandle>, Self::Error> {
        Ok(self.meshes.get(self.device, self.queue, mesh))
    }

    fn texture(&mut self, texture: &Arc<ObjectTexture>) -> Result<TextureHandle, Self::Error> {
        Ok(self.textures.get(self.device, self.queue, texture))
    }

    fn picture(&mut self, picture: &[u8]) -> Result<Option<TextureHandle>, Self::Error> {
        Ok(self.textures.picture(self.device, self.queue, picture))
    }
}

type WgpuDrawItem = DrawItem<MeshHandle, TextureHandle>;

impl WgpuSceneRenderer {
    /// Creates the renderer on `device`, drawing into targets of `target_format`. `adapter` says
    /// which sample counts the linear buffer can use.
    ///
    /// # Errors
    ///
    /// [`WgpuRenderError::Mesh`] when a fixed mesh is malformed.
    #[allow(clippy::too_many_lines)] // Every GPU object of the renderer, made once.
    pub fn new(
        adapter: &wgpu::Adapter,
        device: wgpu::Device,
        queue: wgpu::Queue,
        target_format: wgpu::TextureFormat,
    ) -> Result<Self, WgpuRenderError> {
        let max_samples = max_samples(adapter, &device);
        let layouts = Layouts::new(&device);

        let module = |label: &'static str, parts: &[&str]| {
            let mut source = format!("const ZAX_SRGB_DOUBLE: f32 = {:?};\n", sky::SRGB_DOUBLE);
            source.push_str(COMMON);
            for part in parts {
                source.push('\n');
                source.push_str(part);
            }
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            })
        };
        let scene_module = module("zax scene", &[SCENE]);
        let shadow_module = module("zax shadow", &[SHADOW]);
        let sky_module = module("zax sky", &[SKY]);
        let resolve_module = module("zax resolve", &[RESOLVE]);

        let frame_buffer = uniform_buffer(&device, "zax frame", FRAME_BYTES);
        let shadow_frame_buffer = uniform_buffer(&device, "zax shadow frame", SHADOW_FRAME_BYTES);
        let sky_buffer = uniform_buffer(&device, "zax sky", SKY_BYTES);
        let draws = GrowingBuffer::new(&device, "zax draws", DRAW_BYTES * 256);
        let shadow_draws = GrowingBuffer::new(&device, "zax shadow draws", DRAW_BYTES * 256);
        let instance_bytes = INSTANCE_BYTES as u64;
        let instances = GrowingBuffer::new(&device, "zax instances", instance_bytes * 1024);
        let shadow_instances =
            GrowingBuffer::new(&device, "zax shadow instances", instance_bytes * 1024);
        let run_of = GrowingBuffer::new(&device, "zax run of", 4 * 1024);
        let shadow_run_of = GrowingBuffer::new(&device, "zax shadow run of", 4 * 1024);

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 64.
        let palette_width = FramePalette::ENTRY_COUNT as u32;
        let palette = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("zax palette"),
            size: extent(palette_width, 1),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let palette_view = palette.create_view(&wgpu::TextureViewDescriptor::default());
        let dummy_indexed =
            resources::upload(&device, &queue, wgpu::TextureFormat::R8Uint, (1, 1), &[0]);
        let dummy_color = resources::upload(
            &device,
            &queue,
            wgpu::TextureFormat::Rgba8Unorm,
            (1, 1),
            &[0, 0, 0, 0],
        );

        let requested = geometry::SHADOW_MAP_SIZE.max(geometry::MIN_SHADOW_MAP_SIZE);
        let shadow_map_size = shadow_map_side(&device, requested);
        let shadow_map = shadow_map(&device, shadow_map_size);

        let frame_group = frame_group(&device, &layouts, &frame_buffer, &palette_view, &shadow_map);
        let scene_draw_group = draw_group(&device, &layouts, [&draws, &instances, &run_of]);
        let shadow_frame_group =
            single_buffer_group(&device, &layouts.shadow_frame, &shadow_frame_buffer, None);
        let shadow_draw_group = draw_group(
            &device,
            &layouts,
            [&shadow_draws, &shadow_instances, &shadow_run_of],
        );
        let sky_group = single_buffer_group(&device, &layouts.sky, &sky_buffer, None);
        let resolve_pipeline = resolve_pipeline(&device, &layouts, &resolve_module, target_format);

        #[allow(clippy::cast_possible_wrap)] // The side is at most the device limit.
        let statistics = RenderStatistics {
            shadow_map_size: shadow_map_size as i32,
            requested_shadow_map_size: requested,
            ..RenderStatistics::default()
        };

        Ok(Self {
            max_samples,
            meshes: MeshStore::new(&device),
            textures: TextureStore::default(),
            unit_box: Arc::new(interop::unit_box()?),
            picture_quad: Arc::new(interop::picture_quad()?),
            plan: FramePlan::default(),
            scene_pipelines: FastMap::default(),
            shadow_pipelines: FastMap::default(),
            sky_pipelines: FastMap::default(),
            resolve_pipeline,
            frame_buffer,
            shadow_frame_buffer,
            sky_buffer,
            draws,
            shadow_draws,
            instances,
            shadow_instances,
            run_of,
            shadow_run_of,
            draw_data: Vec::new(),
            shadow_draw_data: Vec::new(),
            run_of_data: Vec::new(),
            palette,
            palette_view,
            palette_bytes: vec![0; FramePalette::ENTRY_COUNT * 4],
            dummy_indexed,
            dummy_color,
            shadow_map,
            shadow_map_size,
            shadow_map_requested: requested,
            linear: None,
            frame_group,
            draw_group: scene_draw_group,
            shadow_frame_group,
            shadow_draw_group,
            sky_group,
            texture_groups: FastMap::default(),
            texture_generation: 0,
            statistics,
            last_view: CameraView::default(),
            group_opaque: false,
            layouts,
            scene_module,
            shadow_module,
            sky_module,
            device,
            queue,
        })
    }

    /// The device the renderer draws with.
    #[must_use]
    pub const fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// The queue the renderer submits to.
    #[must_use]
    pub const fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Reorders the opaque tiles by draw state, as `SceneRenderer::set_group_opaque_draws`.
    pub fn set_group_opaque_draws(&mut self, group: bool) {
        self.group_opaque = group;
    }

    /// What the last rendered frame cost (REQ-SCN-064).
    #[must_use]
    pub const fn statistics(&self) -> &RenderStatistics {
        &self.statistics
    }

    /// The camera view the last rendered frame used.
    #[must_use]
    pub const fn last_view(&self) -> &CameraView {
        &self.last_view
    }

    /// Drops every resident mesh and texture, as `SceneRenderer::invalidate_caches`.
    pub fn invalidate_caches(&mut self) {
        self.meshes.clear();
        self.textures.clear();
    }

    /// Draws one frame of `scene` into `target` and submits it.
    ///
    /// # Errors
    ///
    /// [`WgpuRenderError::Target`] when the target size is zero.
    pub fn render(
        &mut self,
        target: WgpuTarget<'_>,
        scene: &SceneState,
        frame: &SceneFrame<'_>,
    ) -> Result<(), WgpuRenderError> {
        if target.width == 0 || target.height == 0 {
            return Err(WgpuRenderError::Target(target.width, target.height));
        }

        self.statistics.begin_frame();
        self.meshes.begin_frame();
        self.textures.begin_frame();

        #[allow(clippy::cast_precision_loss)] // Target sizes are far below 2^24.
        let aspect = target.width as f32 / target.height as f32;
        let view = scene.camera.build_view(scene.clipping, aspect);
        self.last_view = view;

        let started = std::time::Instant::now();
        let mut residency = WgpuResidency {
            device: &self.device,
            queue: &self.queue,
            meshes: &mut self.meshes,
            textures: &mut self.textures,
        };
        let fixed = FixedMeshes {
            unit_box: &self.unit_box,
            picture_quad: &self.picture_quad,
        };
        let Ok(()) = self
            .plan
            .collect(&mut residency, fixed, scene, frame, &view);
        self.statistics.occluders = self.plan.occluders;
        self.statistics.collect_time = started.elapsed();

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("zax frame"),
            });
        let volume = self.shadow_pass(&mut encoder, scene, &view);
        self.scene_pass(&mut encoder, target, scene, frame, &view, &volume);
        self.queue.submit([encoder.finish()]);

        let statistics = &mut self.statistics;
        statistics.resident_meshes = count(self.meshes.count());
        statistics.mesh_uploads = self.meshes.uploads();
        statistics.resident_textures = count(self.textures.count());
        self.meshes.collect();
        self.textures.collect();
        Ok(())
    }

    // ----------------------------------------------------------------- shadow pass

    #[allow(clippy::too_many_lines)] // The upload, then the pass, of REQ-SCN-030.
    fn shadow_pass(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        scene: &SceneState,
        view: &CameraView,
    ) -> ShadowVolume {
        if !scene.settings.shadows || self.plan.lists.casters.is_empty() {
            return ShadowVolume::EMPTY;
        }

        self.resize_shadow_map(scene.settings.shadow_map_size());
        #[allow(clippy::cast_possible_wrap)] // The side is at most the device limit.
        {
            self.statistics.shadow_map_size = self.shadow_map_size as i32;
        }
        self.statistics.requested_shadow_map_size = self.shadow_map_requested;

        let volume = self.plan.shadow_volume(scene, view);
        if !volume.has_content {
            return ShadowVolume::EMPTY;
        }

        let mut frame = Vec::new();
        push_matrix(&mut frame, &volume.view_projection);
        push_vec4(&mut frame, [scene.emulated_time, 0.0, 0.0, 0.0]);
        self.queue
            .write_buffer(&self.shadow_frame_buffer, 0, &frame);

        // The casters go out in instanced runs, in their order (one draw block per run).
        self.plan.lay_out_casters(INSTANCE_BYTES);
        self.shadow_draw_data.clear();
        self.run_of_data.clear();
        for (entry, run) in self.plan.caster_runs.iter().enumerate() {
            push_shadow_draw(
                &mut self.shadow_draw_data,
                &self.plan.lists.casters[run.first],
            );
            push_run_of(&mut self.run_of_data, run, entry);
        }
        let grew = [
            self.shadow_draws.reserve(
                &self.device,
                "zax shadow draws",
                self.shadow_draw_data.len() as u64,
            ),
            self.shadow_instances.reserve(
                &self.device,
                "zax shadow instances",
                self.plan.caster_data.len() as u64,
            ),
            self.shadow_run_of.reserve(
                &self.device,
                "zax shadow run of",
                self.run_of_data.len() as u64,
            ),
        ];
        if grew.contains(&true) {
            self.shadow_draw_group = draw_group(
                &self.device,
                &self.layouts,
                [
                    &self.shadow_draws,
                    &self.shadow_instances,
                    &self.shadow_run_of,
                ],
            );
        }
        self.queue
            .write_buffer(&self.shadow_draws.buffer, 0, &self.shadow_draw_data);
        self.queue
            .write_buffer(&self.shadow_instances.buffer, 0, &self.plan.caster_data);
        self.queue
            .write_buffer(&self.shadow_run_of.buffer, 0, &self.run_of_data);

        for mirrored in [false, true] {
            if !self.shadow_pipelines.contains_key(&mirrored) {
                let pipeline =
                    shadow_pipeline(&self.device, &self.layouts, &self.shadow_module, mirrored);
                self.shadow_pipelines.insert(mirrored, pipeline);
            }
        }

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("zax shadow"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.shadow_map,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_bind_group(0, &self.shadow_frame_group, &[]);
        pass.set_bind_group(1, &self.shadow_draw_group, &[]);
        let (vertices, indices) = self.meshes.buffers();
        pass.set_vertex_buffer(0, vertices.slice(..));
        pass.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint32);
        let mut current = None;
        for run in &self.plan.caster_runs {
            let caster = &self.plan.lists.casters[run.first];
            // Q-069: a mirrored caster winds the other way round.
            let mirrored = interop::determinant(&caster.model) < 0.0;
            if current != Some(mirrored) {
                pass.set_pipeline(&self.shadow_pipelines[&mirrored]);
                current = Some(mirrored);
            }
            let instances = u32::try_from(run.count).unwrap_or(u32::MAX);
            let (first_index, base_vertex) = self.meshes.range(caster.mesh);
            let base = instance_base(run);
            pass.draw_indexed(
                first_index..first_index + caster.mesh.index_count,
                base_vertex,
                base..base + instances,
            );
            self.statistics.batches += 1;
            self.statistics.draw_calls += instances;
            self.statistics.triangles +=
                u64::from(caster.mesh.index_count / 3) * u64::from(instances);
        }
        drop(pass);

        self.statistics.shadow_casters = count(self.plan.lists.casters.len());
        self.statistics.shadow_pass = true;
        volume
    }

    fn resize_shadow_map(&mut self, size: i32) {
        let requested = size.max(geometry::MIN_SHADOW_MAP_SIZE);
        if requested == self.shadow_map_requested {
            return;
        }
        self.shadow_map_requested = requested;
        let granted = shadow_map_side(&self.device, requested);
        if granted != self.shadow_map_size {
            self.shadow_map_size = granted;
            self.shadow_map = shadow_map(&self.device, granted);
            self.frame_group = frame_group(
                &self.device,
                &self.layouts,
                &self.frame_buffer,
                &self.palette_view,
                &self.shadow_map,
            );
        }
    }

    // ----------------------------------------------------------------- scene passes

    #[allow(clippy::too_many_lines)] // The passes of REQ-SCN-058 in their order.
    fn scene_pass(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: WgpuTarget<'_>,
        scene: &SceneState,
        frame: &SceneFrame<'_>,
        view: &CameraView,
        volume: &ShadowVolume,
    ) {
        #[allow(clippy::cast_sign_loss)] // anti_alias_samples() is at least 1.
        let wanted = geometry::clamp_anti_alias_samples(
            scene.settings.anti_alias_samples(),
            i32::try_from(self.max_samples).unwrap_or(1),
        ) as u32;
        self.ensure_linear(target.width, target.height, wanted);
        #[allow(clippy::cast_possible_wrap)] // 1 to 8.
        {
            self.statistics.anti_alias_samples = wanted as i32;
        }

        // The palette is refreshed once per frame (REQ-E3D-565).
        if let Some(list) = frame.draw_list {
            self.upload_palette(list.palette());
        }
        #[allow(clippy::cast_possible_wrap)] // The side is at most the device limit.
        let constants = SceneConstants::new(scene, view, volume, self.shadow_map_size as i32);
        self.queue
            .write_buffer(&self.frame_buffer, 0, &frame_bytes(&constants));

        let sky_kind = scene.settings.skybox;
        let sky = matches!(sky_kind, SkyboxKind::Gradient | SkyboxKind::Fantasy);
        if sky {
            let bytes = sky_bytes(scene, frame, view);
            self.queue.write_buffer(&self.sky_buffer, 0, &bytes);
            let key = (sky_kind == SkyboxKind::Gradient, wanted);
            if !self.sky_pipelines.contains_key(&key) {
                let pipeline = sky_pipeline(&self.device, &self.layouts, &self.sky_module, key);
                self.sky_pipelines.insert(key, pipeline);
            }
        }
        self.statistics.skybox_pass = sky;

        let drawing = !self.plan.lists.nothing_to_draw();
        if drawing {
            self.plan
                .lay_out_instances(self.group_opaque, INSTANCE_BYTES);
            self.upload_draws(&constants);
            self.prepare_pipelines(scene.settings.cull_back_faces, wanted);
            self.prepare_texture_groups();
        }

        let linear = self
            .linear
            .as_ref()
            .unwrap_or_else(|| unreachable!("ensure_linear made the buffer"));
        let background = color::rgb_to_linear(scene.background_color());
        let (attachment, resolve_target) = match &linear.multisampled {
            Some(multisampled) => (multisampled, Some(&linear.resolved)),
            None => (&linear.resolved, None),
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("zax scene"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: attachment,
                depth_slice: None,
                resolve_target,
                ops: wgpu::Operations {
                    // REQ-SCN-046: cleared with the background colour, without any lighting.
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: f64::from(background.x),
                        g: f64::from(background.y),
                        b: f64::from(background.z),
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &linear.depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        // REQ-SCN-058: the skybox replaces the clear, behind everything else.
        if sky {
            pass.set_pipeline(&self.sky_pipelines[&(sky_kind == SkyboxKind::Gradient, wanted)]);
            pass.set_bind_group(0, &self.sky_group, &[]);
            pass.draw(0..3, 0..1);
        }

        if drawing {
            pass.set_bind_group(0, &self.frame_group, &[]);
            pass.set_bind_group(1, &self.draw_group, &[]);
            let (vertices, indices) = self.meshes.buffers();
            pass.set_vertex_buffer(0, vertices.slice(..));
            pass.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint32);
            let cull = scene.settings.cull_back_faces;
            let lists = &self.plan.lists;
            let mut draw = Drawer {
                pass: &mut pass,
                pipelines: &self.scene_pipelines,
                texture_groups: &self.texture_groups,
                meshes: &self.meshes,
                statistics: &mut self.statistics,
                cull,
                samples: wanted,
                current: None,
                bound_textures: None,
            };

            // 1. Opaque objects.
            for run in &lists.opaque_runs {
                draw.run(&lists.opaque, run, SceneMode::Opaque);
            }
            // 2. Transparent objects, farthest first, depth first then colour (REQ-SCN-040).
            for group in &lists.transparent_objects {
                draw.blended(&lists.transparent, &lists.transparent_runs, group.runs);
            }
            // 3. HUD (REQ-SCN-044).
            for group in &lists.hud_objects {
                draw.blended(&lists.hud, &lists.hud_runs, group.runs);
            }

            let statistics = &mut *draw.statistics;
            statistics.opaque_tiles = count(lists.opaque.len());
            statistics.transparent_tiles = count(lists.transparent.len());
            statistics.transparent_objects = if lists.transparent.is_empty() {
                0
            } else {
                count(lists.transparent_objects.len())
            };
            statistics.hud_tiles = count(lists.hud.len());
        }
        drop(pass);

        // AMD-021 R2-009: the whole image is encoded to sRGB once, into the target.
        let mut resolve = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("zax resolve"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target.view,
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
        resolve.set_pipeline(&self.resolve_pipeline);
        resolve.set_bind_group(0, &linear.resolve_group, &[]);
        resolve.draw(0..3, 0..1);
    }

    fn ensure_linear(&mut self, width: u32, height: u32, samples: u32) {
        if self.linear.as_ref().is_some_and(|linear| {
            (linear.width, linear.height, linear.samples) == (width, height, samples)
        }) {
            return;
        }

        let texture = |label: &'static str, format, samples, usage| {
            self.device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: extent(width, height),
                    mip_level_count: 1,
                    sample_count: samples,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default())
        };
        let resolved = texture(
            "zax linear",
            LINEAR_FORMAT,
            1,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        );
        let multisampled = (samples > 1).then(|| {
            texture(
                "zax linear multisampled",
                LINEAR_FORMAT,
                samples,
                wgpu::TextureUsages::RENDER_ATTACHMENT,
            )
        });
        let depth = texture(
            "zax depth",
            DEPTH_FORMAT,
            samples,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
        );
        let resolve_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("zax resolve"),
            layout: &self.layouts.resolve,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&resolved),
            }],
        });
        self.linear = Some(LinearTargets {
            width,
            height,
            samples,
            resolved,
            multisampled,
            depth,
            resolve_group,
        });
    }

    fn upload_palette(&mut self, colors: &[Rgb24; FramePalette::ENTRY_COUNT]) {
        for (bytes, color) in self.palette_bytes.chunks_exact_mut(4).zip(colors) {
            bytes.copy_from_slice(&[color.r, color.g, color.b, 255]);
        }
        #[allow(clippy::cast_possible_truncation)] // 64.
        let width = FramePalette::ENTRY_COUNT as u32;
        resources::write(
            &self.queue,
            &self.palette,
            (width, 1),
            4,
            &self.palette_bytes,
        );
    }

    /// Uploads the placements, one draw block per instanced run (the opaque runs, the
    /// transparent runs, then the HUD runs, in the order of the plan's run lists) and the run of
    /// each instance.
    fn upload_draws(&mut self, constants: &SceneConstants) {
        let lists = &self.plan.lists;
        let (draws, run_of) = (&mut self.draw_data, &mut self.run_of_data);
        draws.clear();
        run_of.clear();
        let shadow_enabled = constants.shadow_enabled;
        let buckets = [
            (
                &lists.opaque,
                &lists.opaque_runs,
                constants.light_direction,
                true,
            ),
            (
                &lists.transparent,
                &lists.transparent_runs,
                constants.light_direction,
                false,
            ),
            (
                &lists.hud,
                &lists.hud_runs,
                constants.hud_light_direction,
                false,
            ),
        ];
        let mut entry = 0;
        for (items, runs, light, opaque) in buckets {
            for run in runs {
                push_draw(draws, &items[run.first], light, opaque, shadow_enabled);
                push_run_of(run_of, run, entry);
                entry += 1;
            }
        }

        let instances = &self.plan.instance_data;
        let grew = [
            self.draws
                .reserve(&self.device, "zax draws", self.draw_data.len() as u64),
            self.instances
                .reserve(&self.device, "zax instances", instances.len() as u64),
            self.run_of
                .reserve(&self.device, "zax run of", self.run_of_data.len() as u64),
        ];
        if grew.contains(&true) {
            self.draw_group = draw_group(
                &self.device,
                &self.layouts,
                [&self.draws, &self.instances, &self.run_of],
            );
        }
        self.queue
            .write_buffer(&self.draws.buffer, 0, &self.draw_data);
        self.queue
            .write_buffer(&self.instances.buffer, 0, instances);
        self.queue
            .write_buffer(&self.run_of.buffer, 0, &self.run_of_data);
    }

    fn prepare_pipelines(&mut self, cull: bool, samples: u32) {
        let lists = &self.plan.lists;
        let mut wanted = Vec::new();
        let mut want = |items: &[WgpuDrawItem], runs: &[InstanceRun], modes: &[SceneMode]| {
            for run in runs {
                let mirrored = interop::normal_matrix(&items[run.first].model).1 < 0.0;
                for &mode in modes {
                    wanted.push(SceneKey {
                        mode,
                        cull,
                        mirrored,
                        samples,
                    });
                }
            }
        };
        want(&lists.opaque, &lists.opaque_runs, &[SceneMode::Opaque]);
        let blended = [SceneMode::DepthOnly, SceneMode::ColorEqual];
        want(&lists.transparent, &lists.transparent_runs, &blended);
        want(&lists.hud, &lists.hud_runs, &blended);
        for key in wanted {
            if !self.scene_pipelines.contains_key(&key) {
                let pipeline = scene_pipeline(&self.device, &self.layouts, &self.scene_module, key);
                self.scene_pipelines.insert(key, pipeline);
            }
        }
    }

    fn prepare_texture_groups(&mut self) {
        if self.texture_generation != self.textures.generation() {
            self.texture_groups.clear();
            self.texture_generation = self.textures.generation();
        }
        let lists = &self.plan.lists;
        let runs = [
            (&lists.opaque, &lists.opaque_runs),
            (&lists.transparent, &lists.transparent_runs),
            (&lists.hud, &lists.hud_runs),
        ];
        for (items, runs) in runs {
            for run in runs {
                let key = texture_key(&items[run.first]);
                if self.texture_groups.contains_key(&key) {
                    continue;
                }
                let indexed = key
                    .0
                    .map_or(&self.dummy_indexed, |handle| self.textures.view(handle));
                let direct = key
                    .1
                    .map_or(&self.dummy_color, |handle| self.textures.view(handle));
                let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("zax textures"),
                    layout: &self.layouts.textures,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(indexed),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(direct),
                        },
                    ],
                });
                self.texture_groups.insert(key, group);
            }
        }
    }
}

/// The textures a draw item binds: its own on the slot its colour mode reads, the dummies
/// elsewhere (as `Passes::draw_run` of the GL path).
fn texture_key(item: &WgpuDrawItem) -> (Option<TextureHandle>, Option<TextureHandle>) {
    let on = |mode: i32| item.texture.filter(|_| item.color_mode == mode);
    (on(0), on(1))
}

/// Issues the draws of the scene buckets.
struct Drawer<'p, 'a> {
    pass: &'p mut wgpu::RenderPass<'a>,
    pipelines: &'p FastMap<SceneKey, wgpu::RenderPipeline>,
    texture_groups: &'p FastMap<(Option<TextureHandle>, Option<TextureHandle>), wgpu::BindGroup>,
    meshes: &'p MeshStore,
    statistics: &'p mut RenderStatistics,
    cull: bool,
    samples: u32,
    current: Option<SceneKey>,
    bound_textures: Option<(Option<TextureHandle>, Option<TextureHandle>)>,
}

impl Drawer<'_, '_> {
    /// Draws `run` of `items` in `mode`.
    fn run(&mut self, items: &[WgpuDrawItem], run: &InstanceRun, mode: SceneMode) {
        let item = &items[run.first];
        let key = SceneKey {
            mode,
            cull: self.cull,
            mirrored: interop::normal_matrix(&item.model).1 < 0.0,
            samples: self.samples,
        };
        if self.current != Some(key) {
            self.pass.set_pipeline(&self.pipelines[&key]);
            self.current = Some(key);
        }
        let textures = texture_key(item);
        if self.bound_textures != Some(textures) {
            self.pass
                .set_bind_group(2, &self.texture_groups[&textures], &[]);
            self.bound_textures = Some(textures);
        }
        let instances = u32::try_from(run.count).unwrap_or(u32::MAX);
        let (first_index, base_vertex) = self.meshes.range(item.mesh);
        let base = instance_base(run);
        self.pass.draw_indexed(
            first_index..first_index + item.mesh.index_count,
            base_vertex,
            base..base + instances,
        );
        self.statistics.batches += 1;
        self.statistics.draw_calls += instances;
        self.statistics.triangles += u64::from(item.mesh.index_count / 3) * u64::from(instances);
    }

    /// Draws one blended object, the runs `range` of `runs` (REQ-SCN-040, AMD-012).
    fn blended(&mut self, items: &[WgpuDrawItem], runs: &[InstanceRun], range: (usize, usize)) {
        for mode in [SceneMode::DepthOnly, SceneMode::ColorEqual] {
            for run in &runs[range.0..range.1] {
                self.run(items, run, mode);
            }
        }
    }
}

// --------------------------------------------------------------------- shaders

const COMMON: &str = include_str!("shaders/common.wgsl");
const SCENE: &str = include_str!("shaders/scene.wgsl");
const SHADOW: &str = include_str!("shaders/shadow.wgsl");
const SKY: &str = include_str!("shaders/sky.wgsl");
const RESOLVE: &str = include_str!("shaders/resolve.wgsl");

// --------------------------------------------------------------------- layouts and pipelines

impl Layouts {
    fn new(device: &wgpu::Device) -> Self {
        let uniform = |binding, visibility, dynamic: bool, size: u64| wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: dynamic,
                min_binding_size: wgpu::BufferSize::new(size),
            },
            count: None,
        };
        let texture = |binding, sample_type| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type,
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let layout = |label, entries: &[wgpu::BindGroupLayoutEntry]| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(label),
                entries,
            })
        };
        let both = wgpu::ShaderStages::VERTEX_FRAGMENT;
        let float = wgpu::TextureSampleType::Float { filterable: false };

        let frame = layout(
            "zax frame",
            &[
                uniform(0, both, false, FRAME_BYTES),
                texture(1, float),
                texture(2, wgpu::TextureSampleType::Depth),
            ],
        );
        let storage = |binding, visibility| wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let draw = layout(
            "zax draw",
            &[
                storage(0, both),
                storage(1, wgpu::ShaderStages::VERTEX),
                storage(2, wgpu::ShaderStages::VERTEX),
            ],
        );
        let textures = layout(
            "zax textures",
            &[texture(0, wgpu::TextureSampleType::Uint), texture(1, float)],
        );
        let shadow_frame = layout(
            "zax shadow frame",
            &[uniform(
                0,
                wgpu::ShaderStages::VERTEX,
                false,
                SHADOW_FRAME_BYTES,
            )],
        );
        let sky = layout("zax sky", &[uniform(0, both, false, SKY_BYTES)]);
        let resolve = layout("zax resolve", &[texture(0, float)]);

        let pipeline = |label, groups: &[Option<&wgpu::BindGroupLayout>]| {
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(label),
                bind_group_layouts: groups,
                immediate_size: 0,
            })
        };
        Self {
            scene_pipeline: pipeline("zax scene", &[Some(&frame), Some(&draw), Some(&textures)]),
            shadow_pipeline: pipeline("zax shadow", &[Some(&shadow_frame), Some(&draw)]),
            sky_pipeline: pipeline("zax sky", &[Some(&sky)]),
            resolve_pipeline: pipeline("zax resolve", &[Some(&resolve)]),
            frame,
            draw,
            textures,
            shadow_frame,
            sky,
            resolve,
        }
    }
}

const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
    0 => Float32x3,
    1 => Float32x3,
    2 => Float32x2,
];

fn vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: VERTEX_STRIDE,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &VERTEX_ATTRIBUTES,
    }
}

/// The scene frame is left-handed and the clip position mirrors z, so the faces the engine winds
/// per REQ-E3D-505 come out clockwise (REQ-SCN-001 notes); a mirrored placement reverses that
/// (Q-069).
const fn front_face(mirrored: bool) -> wgpu::FrontFace {
    if mirrored {
        wgpu::FrontFace::Ccw
    } else {
        wgpu::FrontFace::Cw
    }
}

fn scene_pipeline(
    device: &wgpu::Device,
    layouts: &Layouts,
    module: &wgpu::ShaderModule,
    key: SceneKey,
) -> wgpu::RenderPipeline {
    let blend = wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    };
    let (write_mask, blend, depth_write, compare) = match key.mode {
        SceneMode::Opaque => (
            wgpu::ColorWrites::ALL,
            None,
            true,
            wgpu::CompareFunction::Less,
        ),
        SceneMode::DepthOnly => (
            wgpu::ColorWrites::empty(),
            None,
            true,
            wgpu::CompareFunction::Less,
        ),
        SceneMode::ColorEqual => (
            wgpu::ColorWrites::ALL,
            Some(wgpu::BlendState {
                color: blend,
                alpha: blend,
            }),
            false,
            wgpu::CompareFunction::Equal,
        ),
    };
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("zax scene"),
        layout: Some(&layouts.scene_pipeline),
        vertex: wgpu::VertexState {
            module,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[vertex_layout()],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: front_face(key.mirrored),
            cull_mode: key.cull.then_some(wgpu::Face::Back),
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(depth_write),
            depth_compare: Some(compare),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: key.samples,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        fragment: Some(wgpu::FragmentState {
            module,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: LINEAR_FORMAT,
                blend,
                write_mask,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn shadow_pipeline(
    device: &wgpu::Device,
    layouts: &Layouts,
    module: &wgpu::ShaderModule,
    mirrored: bool,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("zax shadow"),
        layout: Some(&layouts.shadow_pipeline),
        vertex: wgpu::VertexState {
            module,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[vertex_layout()],
        },
        // Q-025: only the back faces of the casters fill the map.
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: front_face(mirrored),
            cull_mode: Some(wgpu::Face::Front),
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: None,
        multiview_mask: None,
        cache: None,
    })
}

fn sky_pipeline(
    device: &wgpu::Device,
    layouts: &Layouts,
    module: &wgpu::ShaderModule,
    (gradient, samples): (bool, u32),
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("zax sky"),
        layout: Some(&layouts.sky_pipeline),
        vertex: wgpu::VertexState {
            module,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        // The depth test is off, so that everything drawn afterwards covers the sky.
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(false),
            depth_compare: Some(wgpu::CompareFunction::Always),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: samples,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        fragment: Some(wgpu::FragmentState {
            module,
            entry_point: Some(if gradient {
                "fs_gradient"
            } else {
                "fs_fantasy"
            }),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: LINEAR_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn resolve_pipeline(
    device: &wgpu::Device,
    layouts: &Layouts,
    module: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("zax resolve"),
        layout: Some(&layouts.resolve_pipeline),
        vertex: wgpu::VertexState {
            module,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module,
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
    })
}

// --------------------------------------------------------------------- bind groups

fn frame_group(
    device: &wgpu::Device,
    layouts: &Layouts,
    frame: &wgpu::Buffer,
    palette: &wgpu::TextureView,
    shadow_map: &wgpu::TextureView,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("zax frame"),
        layout: &layouts.frame,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: frame.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(palette),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(shadow_map),
            },
        ],
    })
}

/// The draw blocks, the placements and the run of each instance of a pass.
fn draw_group(
    device: &wgpu::Device,
    layouts: &Layouts,
    [draws, instances, run_of]: [&GrowingBuffer; 3],
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("zax draw"),
        layout: &layouts.draw,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: draws.buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: instances.buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: run_of.buffer.as_entire_binding(),
            },
        ],
    })
}

/// A bind group of one uniform buffer, bound whole or, with `size`, as a dynamic window.
fn single_buffer_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    buffer: &wgpu::Buffer,
    size: Option<u64>,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("zax uniforms"),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer,
                offset: 0,
                size: size.and_then(wgpu::BufferSize::new),
            }),
        }],
    })
}

// --------------------------------------------------------------------- resources

fn uniform_buffer(device: &wgpu::Device, label: &'static str, size: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

const fn extent(width: u32, height: u32) -> wgpu::Extent3d {
    wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    }
}

/// The side of the shadow map for `requested` texels: the first size of AMD-034 §2's chain the
/// device can hold.
#[allow(clippy::cast_sign_loss)] // The chain only holds positive sizes.
fn shadow_map_side(device: &wgpu::Device, requested: i32) -> u32 {
    let limit = i32::try_from(device.limits().max_texture_dimension_2d).unwrap_or(i32::MAX);
    geometry::shadow_map_sizes(requested, limit)
        .next()
        .unwrap_or(geometry::MIN_SHADOW_MAP_SIZE) as u32
}

fn shadow_map(device: &wgpu::Device, side: u32) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("zax shadow map"),
            size: extent(side, side),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

/// The largest sample count both the linear and the depth format take, at most 8.
fn max_samples(adapter: &wgpu::Adapter, device: &wgpu::Device) -> u32 {
    let adapter_specific = device
        .features()
        .contains(wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES);
    [8, 4, 2]
        .into_iter()
        .find(|&count| {
            if !adapter_specific {
                return count == 4;
            }
            [LINEAR_FORMAT, DEPTH_FORMAT].into_iter().all(|format| {
                adapter
                    .get_texture_format_features(format)
                    .flags
                    .sample_count_supported(count)
            })
        })
        .unwrap_or(1)
}

/// The first instance of `run` in the placements (each placement is `INSTANCE_BYTES` long).
fn instance_base(run: &InstanceRun) -> u32 {
    u32::try_from(run.offset / INSTANCE_BYTES).unwrap_or(u32::MAX)
}

/// Appends the draw block index `entry` once per instance of `run`, at the run's place.
fn push_run_of(data: &mut Vec<u8>, run: &InstanceRun, entry: usize) {
    let base = run.offset / INSTANCE_BYTES * 4;
    if data.len() < base {
        data.resize(base, 0);
    }
    let entry = u32::try_from(entry).unwrap_or(u32::MAX).to_ne_bytes();
    for _ in 0..run.count {
        data.extend_from_slice(&entry);
    }
}

fn count(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

// --------------------------------------------------------------------- uniform blocks

fn push_vec4(data: &mut Vec<u8>, values: [f32; 4]) {
    for value in values {
        data.extend_from_slice(&value.to_ne_bytes());
    }
}

fn push_ivec4(data: &mut Vec<u8>, values: [i32; 4]) {
    for value in values {
        data.extend_from_slice(&value.to_ne_bytes());
    }
}

/// A matrix as the GL path uploads it (`uniform_mat4`, not transposed).
fn push_matrix(data: &mut Vec<u8>, matrix: &Matrix4x4) {
    for cell in matrix.to_cells() {
        data.extend_from_slice(&cell.to_ne_bytes());
    }
}

const fn vec3(value: Vector3, w: f32) -> [f32; 4] {
    [value.x, value.y, value.z, w]
}

const fn vec4(value: Vector4) -> [f32; 4] {
    [value.x, value.y, value.z, value.w]
}

fn frame_bytes(constants: &SceneConstants) -> Vec<u8> {
    let mut data = Vec::new();
    push_matrix(&mut data, &constants.view_projection);
    push_matrix(&mut data, &constants.light_view_projection);
    push_vec4(&mut data, vec4(constants.clip_rect));
    push_vec4(&mut data, vec3(constants.light_linear, constants.ambient));
    push_vec4(
        &mut data,
        [
            constants.time,
            constants.shadow_strength,
            constants.shadow_bias,
            constants.normal_offset,
        ],
    );
    data
}

/// The deformation part of a draw block: flag, amplitude, size, tile, speed, phase.
fn push_deformation(data: &mut Vec<u8>, deform: Option<&DeformParameters>) {
    let rows = |matrix: &Mat4, data: &mut Vec<u8>| {
        for row in 0..3 {
            push_vec4(
                data,
                [
                    matrix[(row, 0)],
                    matrix[(row, 1)],
                    matrix[(row, 2)],
                    matrix[(row, 3)],
                ],
            );
        }
    };
    if let Some(parameters) = deform {
        push_ivec4(data, [1, 0, 0, 0]);
        push_vec4(data, vec3(parameters.amplitude.into(), 0.0));
        push_vec4(data, vec3(parameters.size.into(), 0.0));
        push_vec4(data, vec3(parameters.tile_relative_to_pivot.into(), 0.0));
        rows(&parameters.speed, data);
        rows(&parameters.phase, data);
    } else {
        push_ivec4(data, [0; 4]);
        data.resize(data.len() + 9 * 16, 0);
    }
}

/// The draw block of one instanced run of the scene pass (the uniforms `Passes::draw_run` sets
/// on the GL path).
fn push_draw(
    data: &mut Vec<u8>,
    item: &WgpuDrawItem,
    light: Vector3,
    opaque: bool,
    shadow_enabled: bool,
) {
    let mirrored = interop::normal_matrix(&item.model).1 < 0.0;
    let alpha = if opaque { 1.0 } else { item.alpha };
    push_vec4(data, vec3(item.flat_color, alpha));
    push_vec4(
        data,
        [
            item.texture_size.x,
            item.texture_size.y,
            item.tex_offset.x,
            item.tex_offset.y,
        ],
    );
    push_vec4(
        data,
        [
            item.tex_tilting.x,
            item.tex_tilting.y,
            if mirrored { -1.0 } else { 1.0 },
            0.0,
        ],
    );
    push_vec4(data, vec3(light, 0.0));
    push_ivec4(
        data,
        [
            item.color_mode,
            item.palette_table,
            i32::from(item.clip),
            i32::from(shadow_enabled && item.receive_shadow),
        ],
    );
    push_deformation(data, item.deform.as_ref());
    push_matrix(data, &Matrix4x4::IDENTITY);
}

/// The draw block of one caster of the shadow pass: its placement and its deformation.
fn push_shadow_draw(data: &mut Vec<u8>, caster: &WgpuDrawItem) {
    // Colours, texture, tilting, light and modes: unused by the shadow pass.
    data.resize(data.len() + 5 * 16, 0);
    push_deformation(data, caster.deform.as_ref());
    push_matrix(data, &interop::to_numerics(&caster.model));
}

/// The sky block of the Gradient or the Fantasy sky (`SkyboxPass::draw` of the GL path).
fn sky_bytes(scene: &SceneState, frame: &SceneFrame<'_>, view: &CameraView) -> Vec<u8> {
    let mut data = Vec::new();
    let tan = (view.fov * std::f32::consts::PI / 360.0).tan();
    push_vec4(&mut data, vec3(view.right, 0.0));
    push_vec4(&mut data, vec3(view.up, 0.0));
    push_vec4(&mut data, vec3(view.forward, 0.0));
    push_vec4(&mut data, [tan * view.aspect, tan, 0.0, 0.0]);

    // REQ-SCN-052: the sky follows the colour of the displayed frame.
    let m = scene.nes_background_color;
    let stars = if sky::stars_visible(m) { 1.0 } else { 0.0 };
    push_vec4(&mut data, vec3(color::to_vector(m), stars));
    // REQ-SCN-050, REQ-SCN-059: the twinkle follows real time, 3° per second.
    let angle = scene.real_time * sky::STAR_DEGREES_PER_SECOND * std::f32::consts::PI / 180.0;
    push_vec4(
        &mut data,
        [angle % (2.0 * std::f32::consts::PI), 0.0, 0.0, 0.0],
    );

    let entry = |index: usize| {
        frame
            .draw_list
            .map_or(Rgb24::default(), |list| list.palette()[index])
    };
    push_vec4(&mut data, vec3(color::to_vector(m), 0.0));
    for index in 1..4 {
        push_vec4(&mut data, vec3(color::to_vector(entry(index)), 0.0));
    }
    push_vec4(&mut data, [sky::grey(m), 0.0, 0.0, 0.0]);
    push_bodies(&mut data, &scene.settings.sky_bodies);
    data
}

fn push_bodies(data: &mut Vec<u8>, bodies: &SkyBodySettings) {
    let disc = |degrees: f32| {
        let inner = degrees * std::f32::consts::PI / 180.0;
        let outer = inner * (1.0 + bodies.disc_softness.clamp(0.0, 4.0));
        Vector2::new(inner.cos(), outer.min(std::f32::consts::PI).cos())
    };
    let sun = disc(bodies.sun_radius_degrees());
    let moon = disc(bodies.moon_radius_degrees());
    push_vec4(data, vec3(bodies.sun_direction(), 0.0));
    push_vec4(
        data,
        vec3(color::rgb_to_linear(bodies.sun_color), bodies.sun_gain()),
    );
    push_vec4(data, [sun.x, sun.y, 0.0, 0.0]);
    push_vec4(data, vec3(bodies.moon_direction(), 0.0));
    push_vec4(
        data,
        vec3(color::rgb_to_linear(bodies.moon_color), bodies.moon_gain()),
    );
    push_vec4(data, [moon.x, moon.y, 0.0, 0.0]);
    push_vec4(
        data,
        vec3(
            color::rgb_to_linear(bodies.cloud_color),
            bodies.cloud_gain(),
        ),
    );
    push_vec4(
        data,
        [bodies.cloud_coverage(), bodies.cloud_scale, 0.0, 0.0],
    );
}
