//! The wgpu renderer of SPEC-06, the twin of the GL [`crate::SceneRenderer`]: the same
//! [`FramePlan`], the same passes in the same order — shadow map, background or skybox, opaque
//! objects, sorted transparent objects, HUD — into the same linear half-float buffer, resolved
//! to sRGB into the target the host provides.
//!
//! What differs is only how the state reaches the GPU. Depth test, culling and winding are
//! baked into pipelines, so each combination the GL path switches to is a pipeline of its own.
//! Every mesh lives in one atlas, and the per-draw uniforms of the GL path become one block per
//! instanced run in a storage buffer that each instance finds through `run_of`: a pass binds
//! everything once, and a draw call is the call alone, which keeps wgpu's per-command cost low.
//! When the device can, the draws are batched: every resident texture is bound at once in two
//! texture arrays, each run's draw call is a set of indirect arguments, and the runs that share a
//! pipeline go out in one multi-draw, in their order. Otherwise each run is a draw call of its
//! own, its textures bound before it. The clip depth of GL (-1..1) is remapped to wgpu's (0..1)
//! in the vertex shaders, which keeps the stored depths, and the shadow map, rendered top down,
//! is read at 1 - y.

use std::sync::Arc;

use zax_engine3d::draw_list::DeformParameters;
use zax_engine3d::geometry::texture::ObjectTexture;
use zax_engine3d::geometry::tile_mesh::{TileMesh, TriangleCountError};
use zax_engine3d::hashing::FastMap;
use zax_engine3d::palette::{FramePalette, Rgb24};
use zax_engine3d::space_bend::SpaceBend;
use zax_engine3d::vectors::{Mat4, Vec3};

use super::preprocess;
use super::resources::{self, MeshHandle, MeshStore, TextureHandle, TextureStore, VERTEX_STRIDE};
use crate::camera::CameraView;
use crate::color;
use crate::frame_plan::{
    DrawItem, FixedMeshes, FramePlan, InstanceRun, Placement, Residency, SceneConstants,
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

/// Bytes of the per-run block (`Draw` in common.wgsl).
const DRAW_BLOCK: usize = 80;
/// The same, as a buffer size.
const DRAW_BYTES: u64 = DRAW_BLOCK as u64;
/// Bytes of one placement (`Instance` in common.wgsl): the three rows of the affine placement
/// and the deformation's tile position; the shader derives the normal matrix itself.
const AFFINE_BYTES: usize = 64;

/// The placements of the wgpu backend.
#[derive(Debug, Clone, Copy)]
struct AffinePlacement;

impl Placement for AffinePlacement {
    #[inline]
    fn write(self, data: &mut Vec<u8>, model: &Mat4, deform: Option<&DeformParameters>) {
        push_affine_placement(data, model, deform);
    }
}

const AFFINE_PLACEMENT: AffinePlacement = AffinePlacement;

/// Bytes of a deformation (`Deform` in common.wgsl), which only deformed runs have.
const DEFORM_BYTES: u64 = 128;
/// Bytes of the frame block (`Frame` in scene.wgsl): two matrices and five `vec4`, then the four
/// curvature `vec4` of AMD-043 REQ-SCN-067, rounded up to the alignment.
const FRAME_BYTES: u64 = 288;
/// Bytes of the shadow frame block (`ShadowFrame` in shadow.wgsl): the matrix, the time, and the
/// same four curvature `vec4` (REQ-SCN-068 bends this pass too).
const SHADOW_FRAME_BYTES: u64 = 144;
/// Bytes of the sky block (`Sky` in sky.wgsl).
const SKY_BYTES: u64 = 19 * 16;
/// Bytes of the indirect arguments of one run (index count, instance count, first index, base
/// vertex, first instance).
const INDIRECT_BYTES: u64 = 20;
/// Textures each array of the batched draws holds at most: index 0 is the dummy, the texture of
/// slot `s` sits at `s + 1`.
const BINDLESS_TEXTURES: u32 = 16384;

/// The device features the batched draws need: indirect draws that start at any instance, and
/// arrays of textures, indexed per draw and partly filled.
pub const BATCHING_FEATURES: wgpu::Features = wgpu::Features::INDIRECT_FIRST_INSTANCE
    .union(wgpu::Features::TEXTURE_BINDING_ARRAY)
    .union(wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING)
    .union(wgpu::Features::PARTIALLY_BOUND_BINDING_ARRAY);

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
    /// The pipeline of the batched draws, which reads the texture arrays.
    batched: bool,
    /// Which WGSL variant the pipeline's module was built from.
    variant: ShaderVariant,
}

/// Which variant of the scene and shadow modules a draw asks for — the `ProgramKey` of
/// `gl/shaders.rs` for the axes this backend has, and read by [`preprocess`] as its defines.
///
/// One named field per axis the specification names. **Every field false is the module of before
/// RU-7**, built in [`WgpuSceneRenderer::new`] and the only one a Classic frame at the default
/// curvature ever asks for: the others are built the first time a frame wants them and cost a
/// frame that does not nothing at all.
// One independent axis per field, each named by the requirement that switches it — the same shape
// and the same reason as the OpenGL `ProgramKey` of `gl/shaders.rs`, which carries six: an enum
// per axis would say less, not more.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
struct ShaderVariant {
    /// The curvature of AMD-043 REQ-SCN-067, which also places the vertex and transports the
    /// normal (REQ-SCN-068), and bends the shadow pass with it.
    bend: bool,
    /// The voxel grid of AMD-055 REQ-SCN-081, which modulates the albedo.
    voxel_grid: bool,
    /// The pixel shadows of AMD-055 REQ-SCN-082, which read the shadow at the voxel's centre.
    /// Independent of the grid: REQ-SCN-079 switches the two separately, so a player who turns
    /// the grid on must not pay for the snapping, nor the other way round.
    pixel_shadow: bool,
    /// AMD-058 REQ-SCN-065: `uv` interpolated at the centroid of the covered samples, which is
    /// only ever asked for when the buffer is multisampled.
    centroid: bool,
}

impl ShaderVariant {
    /// Every field false: the modules of before RU-7.
    const PLAIN: Self = Self {
        bend: false,
        voxel_grid: false,
        pixel_shadow: false,
        centroid: false,
    };

    /// The defines [`preprocess::compose`] expands this variant's sources with.
    fn defines(self) -> Vec<&'static str> {
        let mut defines = Vec::new();
        if self.bend {
            defines.push("ZAX_BEND");
        }
        if self.voxel_grid {
            defines.push("ZAX_VOXEL_GRID");
        }
        if self.pixel_shadow {
            defines.push("ZAX_PIXEL_SHADOW");
        }
        if self.centroid {
            defines.push("ZAX_CENTROID");
        }
        defines
    }

    /// The chunks that go before a body for this variant.
    fn chunks(self) -> Vec<&'static str> {
        let mut chunks = vec![COMMON];
        if self.bend {
            chunks.push(BEND);
        }
        if self.voxel_grid {
            chunks.push(VOXEL_GRID);
        }
        if self.pixel_shadow {
            chunks.push(PIXEL_SHADOW);
        }
        chunks
    }

    /// What the module labels say, so that a `naga` message names the variant it came from.
    fn suffix(self) -> String {
        let mut suffix = String::new();
        for (on, name) in [
            (self.bend, " bend"),
            (self.voxel_grid, " grid"),
            (self.pixel_shadow, " pixel-shadow"),
            (self.centroid, " centroid"),
        ] {
            if on {
                suffix.push_str(name);
            }
        }
        suffix
    }

    /// The variant the **shadow** pass asks for: the curvature alone. REQ-SCN-081 and
    /// REQ-SCN-082 are about the albedo and about where the shadow is *read*, neither of which
    /// the depth-only pass has any part in, so a frame with the grid on does not build or switch
    /// to a second shadow module.
    const fn shadow(self) -> Self {
        Self {
            bend: self.bend,
            voxel_grid: false,
            pixel_shadow: false,
            centroid: false,
        }
    }
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
    usage: wgpu::BufferUsages,
}

impl GrowingBuffer {
    fn new(device: &wgpu::Device, label: &'static str, size: u64) -> Self {
        Self::with_usage(device, label, size, wgpu::BufferUsages::STORAGE)
    }

    fn with_usage(
        device: &wgpu::Device,
        label: &'static str,
        size: u64,
        usage: wgpu::BufferUsages,
    ) -> Self {
        Self {
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage: usage | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            size,
            usage,
        }
    }

    /// Makes room for `needed` bytes; true when the buffer was replaced.
    fn reserve(&mut self, device: &wgpu::Device, label: &'static str, needed: u64) -> bool {
        if needed <= self.size {
            return false;
        }
        *self = Self::with_usage(device, label, needed.next_power_of_two(), self.usage);
        true
    }
}

/// What the batched draws add: the scene shader that reads the texture arrays, its layouts, the
/// arrays themselves (with the texture revision they were built on) and the indirect arguments.
#[derive(Debug)]
struct Batching {
    textures: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    group: Option<(u64, wgpu::BindGroup)>,
    indirect: GrowingBuffer,
    shadow_indirect: GrowingBuffer,
    data: Vec<u8>,
}

impl Batching {
    fn new(device: &wgpu::Device, layouts: &Layouts) -> Self {
        let array = |binding, sample_type| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type,
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: std::num::NonZeroU32::new(BINDLESS_TEXTURES),
        };
        let textures = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("zax texture arrays"),
            entries: &[
                array(0, wgpu::TextureSampleType::Uint),
                array(1, wgpu::TextureSampleType::Float { filterable: false }),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("zax scene batched"),
            bind_group_layouts: &[Some(&layouts.frame), Some(&layouts.draw), Some(&textures)],
            immediate_size: 0,
        });
        let indirect = |label| {
            GrowingBuffer::with_usage(
                device,
                label,
                INDIRECT_BYTES * 256,
                wgpu::BufferUsages::INDIRECT,
            )
        };
        Self {
            textures,
            pipeline_layout,
            group: None,
            indirect: indirect("zax indirect"),
            shadow_indirect: indirect("zax shadow indirect"),
            data: Vec::new(),
        }
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
    /// REQ-SCN-068: the backdrop box with each face cut into 32 × 32 cells, built the first time
    /// something bends and kept after that.
    unit_box_bent: Option<Arc<TileMesh>>,
    /// REQ-SCN-074: the 2D-mode rectangle, subdivided the same way, built the first time a 2D
    /// frame bends.
    picture_quad_bent: Option<Arc<TileMesh>>,
    /// The quad of the last widescreen 2D-mode picture, by margin.
    wide_picture_quad: Option<interop::WidePictureQuad>,
    /// How many cells the cached widescreen quad was built with, so that the curvature going on
    /// or off rebuilds it (REQ-SCN-074).
    wide_picture_cells: u16,
    plan: FramePlan<MeshHandle, TextureHandle>,
    layouts: Layouts,
    /// The scene module of each variant asked for so far, the plain one built in [`Self::new`].
    scene_modules: FastMap<ShaderVariant, wgpu::ShaderModule>,
    /// The same for the shadow pass, whose only axis is the curvature (REQ-SCN-068).
    shadow_modules: FastMap<ShaderVariant, wgpu::ShaderModule>,
    /// And for the batched scene module, which differs only in how it reaches its textures.
    batched_modules: FastMap<ShaderVariant, wgpu::ShaderModule>,
    sky_module: wgpu::ShaderModule,
    scene_pipelines: FastMap<SceneKey, wgpu::RenderPipeline>,
    shadow_pipelines: FastMap<(bool, ShaderVariant), wgpu::RenderPipeline>,
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
    deforms: GrowingBuffer,
    shadow_deforms: GrowingBuffer,
    draw_data: Vec<u8>,
    shadow_draw_data: Vec<u8>,
    run_of_data: Vec<u8>,
    deform_data: Vec<u8>,
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
    batching: Option<Batching>,
    statistics: RenderStatistics,
    last_view: CameraView,
    group_opaque: bool,
    /// Q-104: the diagnostic lines this backend has to hand the host. Today there is exactly one
    /// subject — the Enhanced style of AMD-055, which this backend does not render — and it is
    /// written at most once per renderer.
    diagnostics: Vec<String>,
    /// Whether that line has been written.
    style_reported: bool,
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

    fn texture(&mut self, texture: &Arc<ObjectTexture>) -> Option<TextureHandle> {
        Some(self.textures.get(self.device, self.queue, texture))
    }

    fn picture(
        &mut self,
        picture: &[u8],
        height: usize,
    ) -> Result<Option<TextureHandle>, Self::Error> {
        Ok(self
            .textures
            .picture(self.device, self.queue, picture, height))
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

        let module =
            |label: &str, parts: &[&str]| build_module(&device, label, parts, ShaderVariant::PLAIN);
        let scene_module = module("zax scene", &[SCENE, TEXTURES_BOUND]);
        let shadow_module = module("zax shadow", &[SHADOW]);
        let sky_module = module("zax sky", &[SKY]);
        let resolve_module = module("zax resolve", &[RESOLVE]);

        let frame_buffer = uniform_buffer(&device, "zax frame", FRAME_BYTES);
        let shadow_frame_buffer = uniform_buffer(&device, "zax shadow frame", SHADOW_FRAME_BYTES);
        let sky_buffer = uniform_buffer(&device, "zax sky", SKY_BYTES);
        let draws = GrowingBuffer::new(&device, "zax draws", DRAW_BYTES * 256);
        let shadow_draws = GrowingBuffer::new(&device, "zax shadow draws", DRAW_BYTES * 256);
        let instance_bytes = AFFINE_BYTES as u64;
        let instances = GrowingBuffer::new(&device, "zax instances", instance_bytes * 1024);
        let shadow_instances =
            GrowingBuffer::new(&device, "zax shadow instances", instance_bytes * 1024);
        let run_of = GrowingBuffer::new(&device, "zax run of", 4 * 1024);
        let shadow_run_of = GrowingBuffer::new(&device, "zax shadow run of", 4 * 1024);
        let deforms = GrowingBuffer::new(&device, "zax deforms", DEFORM_BYTES * 16);
        let shadow_deforms = GrowingBuffer::new(&device, "zax shadow deforms", DEFORM_BYTES * 16);

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
        let scene_draw_group =
            draw_group(&device, &layouts, [&draws, &instances, &run_of, &deforms]);
        let shadow_frame_group =
            single_buffer_group(&device, &layouts.shadow_frame, &shadow_frame_buffer, None);
        let shadow_draw_group = draw_group(
            &device,
            &layouts,
            [
                &shadow_draws,
                &shadow_instances,
                &shadow_run_of,
                &shadow_deforms,
            ],
        );
        let sky_group = single_buffer_group(&device, &layouts.sky, &sky_buffer, None);
        let resolve_pipeline = resolve_pipeline(&device, &layouts, &resolve_module, target_format);
        let batches = batching_supported(adapter, &device);
        let mut batched_modules = FastMap::default();
        if batches {
            batched_modules.insert(
                ShaderVariant::PLAIN,
                module("zax scene batched", &[SCENE, &bindless_textures()]),
            );
        }
        let batching = batches.then(|| Batching::new(&device, &layouts));

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
            unit_box_bent: None,
            picture_quad_bent: None,
            wide_picture_quad: None,
            wide_picture_cells: interop::subdivision_for(false),
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
            deforms,
            shadow_deforms,
            draw_data: Vec::new(),
            shadow_draw_data: Vec::new(),
            run_of_data: Vec::new(),
            deform_data: Vec::new(),
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
            batching,
            statistics,
            last_view: CameraView::default(),
            group_opaque: false,
            diagnostics: Vec::new(),
            style_reported: false,
            layouts,
            scene_modules: [(ShaderVariant::PLAIN, scene_module)].into_iter().collect(),
            shadow_modules: [(ShaderVariant::PLAIN, shadow_module)]
                .into_iter()
                .collect(),
            batched_modules,
            sky_module,
            device,
            queue,
        })
    }

    /// The scene, shadow and batched modules of `variant`, built the first time it is asked for.
    ///
    /// Called once a frame, and at the default curvature it is one hash lookup that hits: the
    /// plain variant is in all three maps from [`Self::new`] onwards.
    fn ensure_modules(&mut self, variant: ShaderVariant) {
        if !self.scene_modules.contains_key(&variant) {
            let module = build_module(&self.device, "zax scene", &[SCENE, TEXTURES_BOUND], variant);
            self.scene_modules.insert(variant, module);
        }
        let shadow = variant.shadow();
        if !self.shadow_modules.contains_key(&shadow) {
            let module = build_module(&self.device, "zax shadow", &[SHADOW], shadow);
            self.shadow_modules.insert(shadow, module);
        }
        if self.batching.is_some() && !self.batched_modules.contains_key(&variant) {
            let module = build_module(
                &self.device,
                "zax scene batched",
                &[SCENE, &bindless_textures()],
                variant,
            );
            self.batched_modules.insert(variant, module);
        }
    }

    /// The variant this frame draws with — the curvature of AMD-043 REQ-SCN-067, the voxel grid
    /// of AMD-055 REQ-SCN-081 and the pixel shadows of REQ-SCN-082 — and the subdivided meshes
    /// REQ-SCN-068 and REQ-SCN-074 ask for while something bends.
    ///
    /// The twin of `SceneRenderer::select_bend_variant` and `select_style_variants`, and like
    /// them **three boolean tests** at the default settings: no module is built, no pipeline is
    /// added, no mesh is subdivided, and the frame block's curvature and style fields are written
    /// as the zeros they already were.
    ///
    /// Neither of the two style axes applies in **2D mode**: the frame then holds one object, the
    /// rectangle of AMD-046 §2, which REQ-SCN-081 and REQ-SCN-082 both exclude — the same reading
    /// the OpenGL backend applies, and it costs nothing.
    ///
    /// # Errors
    ///
    /// [`WgpuRenderError::Mesh`] when a subdivided mesh cannot be built. The frame then draws
    /// with the meshes it has, flat, rather than not at all.
    fn select_bend_variant(
        &mut self,
        scene: &SceneState,
    ) -> Result<ShaderVariant, WgpuRenderError> {
        let bend = !scene.effective_bend().is_identity();
        let style = &scene.settings.style;
        let flat = scene.two_dimensional_mode;
        let variant = ShaderVariant {
            bend,
            voxel_grid: style.voxel_grid_active() && !flat,
            pixel_shadow: style.pixel_shadow_active() && !flat,
            // AMD-058 REQ-SCN-065. The trigger is the **setting** of AMD-032 §2, not the sample
            // count the driver granted: a buffer that falls back to one sample interpolates a
            // centroid at the pixel centre anyway (the covered sample set is that one sample), so
            // the fallback draws the same pixels. With `video.anti_alias` = 0 nothing is built,
            // which is what keeps the handheld default on the module of before this amendment.
            centroid: scene.settings.anti_alias_samples() > 1,
        };
        self.ensure_modules(variant);
        // REQ-SCN-081: the HUD never takes the grid, and it draws with the flat variant of
        // REQ-SCN-068 — so the plain modules have to be there for it whatever this frame asks.
        if variant != ShaderVariant::PLAIN {
            self.ensure_modules(hud_variant(variant));
        }
        if bend {
            let cells = interop::subdivision_for(true);
            if self.unit_box_bent.is_none() {
                // REQ-SCN-068: a plane large enough to curve visibly is subdivided, the backdrop
                // into at least 32 × 32 cells per face.
                self.unit_box_bent = Some(Arc::new(interop::unit_box_subdivided(cells)?));
            }
            if self.picture_quad_bent.is_none() && scene.two_dimensional_mode {
                // REQ-SCN-074: at least 32 × 32 cells, so at least a 33 × 33 grid of vertices.
                self.picture_quad_bent =
                    Some(Arc::new(interop::picture_quad_subdivided(0, 0, cells)?));
            }
        }
        Ok(variant)
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

    /// Whether the draws go out batched, in multi-draws over the texture arrays, rather than
    /// one draw call per run.
    #[must_use]
    pub const fn batched_draws(&self) -> bool {
        self.batching.is_some()
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

    /// The diagnostic lines this backend has for the host, as `SceneRenderer::take_diagnostics`.
    ///
    /// Q-104: one subject, the Enhanced style of AMD-055 — selected in the settings, read back as
    /// selected, and **not rendered** here. The line is written once per renderer, the first time
    /// a frame asks for it.
    pub fn take_diagnostics(&mut self) -> Vec<String> {
        std::mem::take(&mut self.diagnostics)
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

        // REQ-SCN-067: the curvature decides which module variant draws the frame, once here,
        // never per vertex — the same choice, in the same place in the frame, as
        // `SceneRenderer::select_bend_variant` makes on the OpenGL side.
        let variant = self.select_bend_variant(scene)?;

        self.statistics.begin_frame();
        self.meshes.begin_frame();
        self.textures.begin_frame();

        #[allow(clippy::cast_precision_loss)] // Target sizes are far below 2^24.
        let aspect = target.width as f32 / target.height as f32;
        let view = if scene.two_dimensional_mode && scene.picture_head_on {
            scene.camera.picture_view(
                scene.picture_clipping(),
                aspect,
                crate::numerics::Vector3::new(scene.picture_tilt[0], scene.picture_tilt[1], 0.0),
            )
        } else {
            scene.camera.build_view(scene.view_clipping(), aspect)
        };
        self.last_view = view;

        let started = std::time::Instant::now();
        let mut residency = WgpuResidency {
            device: &self.device,
            queue: &self.queue,
            meshes: &mut self.meshes,
            textures: &mut self.textures,
        };
        // AMD-046 REQ-SCN-074: the rectangle of the 2D mode is subdivided while it bends, and
        // only then. `select_bend_variant` has already folded the setting in, so in 2D mode this
        // flag *is* "the rectangle bends".
        let cells = interop::subdivision_for(variant.bend);
        if cells != self.wide_picture_cells {
            self.wide_picture_quad = None;
            self.wide_picture_cells = cells;
        }
        self.wide_picture_quad = interop::wide_quad_for_subdivided_cached(
            frame.nes_picture,
            frame.nes_picture_height,
            cells,
            self.wide_picture_quad.take(),
        )?;
        let fixed = FixedMeshes {
            // REQ-SCN-068: the subdivided box only while something bends.
            unit_box: match &self.unit_box_bent {
                Some(subdivided) if variant.bend => subdivided,
                _ => &self.unit_box,
            },
            picture_quad: match &self.picture_quad_bent {
                Some(subdivided) if variant.bend => subdivided,
                _ => &self.picture_quad,
            },
            wide_picture_quad: self.wide_picture_quad.as_ref().map(|(_, quad)| quad),
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
        let volume = self.shadow_pass(&mut encoder, scene, &view, variant);
        self.scene_pass(&mut encoder, target, scene, frame, &view, &volume, variant);
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
        variant: ShaderVariant,
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
        // REQ-SCN-068: the shadow pass bends with the scene, so it reads the same constants.
        push_bend(&mut frame, scene.effective_bend());
        self.queue
            .write_buffer(&self.shadow_frame_buffer, 0, &frame);

        // The casters go out in instanced runs, in their order (one draw block per run).
        self.plan.lay_out_casters(AFFINE_BYTES, AFFINE_PLACEMENT);
        self.shadow_draw_data.clear();
        self.run_of_data.clear();
        self.deform_data.clear();
        for (entry, run) in self.plan.caster_runs.iter().enumerate() {
            let caster = &self.plan.lists.casters[run.first];
            push_shadow_draw(&mut self.shadow_draw_data, &mut self.deform_data, caster);
            push_run_of(&mut self.run_of_data, run, entry);
        }
        if let Some(batching) = &mut self.batching {
            batching.data.clear();
            for run in &self.plan.caster_runs {
                let caster = &self.plan.lists.casters[run.first];
                push_indirect(&mut batching.data, &self.meshes, caster.mesh, run);
            }
            batching.shadow_indirect.reserve(
                &self.device,
                "zax shadow indirect",
                batching.data.len() as u64,
            );
            self.queue
                .write_buffer(&batching.shadow_indirect.buffer, 0, &batching.data);
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
            self.shadow_deforms.reserve(
                &self.device,
                "zax shadow deforms",
                self.deform_data.len() as u64,
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
                    &self.shadow_deforms,
                ],
            );
        }
        self.queue
            .write_buffer(&self.shadow_draws.buffer, 0, &self.shadow_draw_data);
        self.queue
            .write_buffer(&self.shadow_instances.buffer, 0, &self.plan.caster_data);
        self.queue
            .write_buffer(&self.shadow_run_of.buffer, 0, &self.run_of_data);
        if !self.deform_data.is_empty() {
            self.queue
                .write_buffer(&self.shadow_deforms.buffer, 0, &self.deform_data);
        }

        // REQ-SCN-081 and REQ-SCN-082 are without object on a depth-only pass, so the shadow
        // module and its two pipelines are keyed on the curvature alone.
        let shadow_variant = variant.shadow();
        let module = &self.shadow_modules[&shadow_variant];
        for mirrored in [false, true] {
            self.shadow_pipelines
                .entry((mirrored, shadow_variant))
                .or_insert_with(|| shadow_pipeline(&self.device, &self.layouts, module, mirrored));
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
        let indirect = self
            .batching
            .as_ref()
            .map(|batching| &batching.shadow_indirect.buffer);
        let mut current = None;
        // The runs since the last pipeline change, drawn in one multi-draw when batched.
        let mut pending = 0..0;
        for (entry, run) in self.plan.caster_runs.iter().enumerate() {
            let caster = &self.plan.lists.casters[run.first];
            // Q-069: a mirrored caster winds the other way round.
            let mirrored = interop::determinant(&caster.model) < 0.0;
            if current != Some(mirrored) {
                if let Some(indirect) = indirect {
                    multi_draw(&mut pass, indirect, &pending);
                }
                pending = entry..entry;
                pass.set_pipeline(&self.shadow_pipelines[&(mirrored, shadow_variant)]);
                current = Some(mirrored);
            }
            let instances = u32::try_from(run.count).unwrap_or(u32::MAX);
            if indirect.is_some() {
                pending.end = entry + 1;
            } else {
                let (first_index, base_vertex) = self.meshes.range(caster.mesh);
                let base = instance_base(run);
                pass.draw_indexed(
                    first_index..first_index + caster.mesh.index_count,
                    base_vertex,
                    base..base + instances,
                );
            }
            self.statistics.batches += 1;
            self.statistics.draw_calls += instances;
            self.statistics.triangles +=
                u64::from(caster.mesh.index_count / 3) * u64::from(instances);
        }
        if let Some(indirect) = indirect {
            multi_draw(&mut pass, indirect, &pending);
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

    #[allow(clippy::too_many_lines)]
    // The passes of REQ-SCN-058 in their order.
    // The frame's inputs, one parameter each: what to draw into, what to draw, where the eye and
    // the light are, and which variant the curvature chose. Bundling them into a struct would
    // only move the list.
    #[allow(clippy::too_many_arguments)]
    fn scene_pass(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: WgpuTarget<'_>,
        scene: &SceneState,
        frame: &SceneFrame<'_>,
        view: &CameraView,
        volume: &ShadowVolume,
        variant: ShaderVariant,
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
        // Q-104, the same gap for AMD-055: the Enhanced style is **not** rendered here either.
        // `scene.settings.style` reaches this backend and nothing reads it — no variant axis, no
        // screen pass — so a game played on Vulkan or DirectX 12 with Enhanced selected sees the
        // Classic picture, which is this backend's own picture of before the amendment. The
        // setting is kept and read back as set (REQ-SCN-078 is a requirement on the *setting*),
        // and the gap is said out loud once, where AMD-051's diagnostics already go.
        if scene.settings.style.enhanced && !self.style_reported {
            self.style_reported = true;
            self.diagnostics.push(
                "the Enhanced visual style is not rendered by this graphics backend; the scene \
                 is drawn in the Classic style (AMD-055 REQ-SCN-078, Q-104)"
                    .to_owned(),
            );
        }
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
        let batched = drawing && self.prepare_texture_arrays();
        if drawing {
            self.plan
                .lay_out_instances(self.group_opaque, AFFINE_BYTES, AFFINE_PLACEMENT);
            self.upload_draws(&constants, batched);
            self.prepare_pipelines(scene.settings.cull_back_faces, wanted, batched, variant);
            if !batched {
                self.prepare_texture_groups();
            }
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
                    // Kept, not discarded: the depth of the scene is what post-processing tools
                    // such as ReShade read once the pass is over.
                    store: wgpu::StoreOp::Store,
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
            let indirect = match (&self.batching, batched) {
                (
                    Some(Batching {
                        group: Some((_, group)),
                        indirect,
                        ..
                    }),
                    true,
                ) => {
                    pass.set_bind_group(2, group, &[]);
                    Some(&indirect.buffer)
                }
                _ => None,
            };
            let mut draw = Drawer {
                pass: &mut pass,
                pipelines: &self.scene_pipelines,
                texture_groups: &self.texture_groups,
                meshes: &self.meshes,
                statistics: &mut self.statistics,
                cull,
                samples: wanted,
                variant,
                current: None,
                bound_textures: None,
                indirect,
                pending: None,
            };

            // The indirect arguments follow the run lists: opaque, transparent, then HUD.
            let transparent_base = lists.opaque_runs.len();
            let hud_base = transparent_base + lists.transparent_runs.len();
            // 1. Opaque objects.
            for (entry, run) in lists.opaque_runs.iter().enumerate() {
                draw.run(&lists.opaque, run, entry, SceneMode::Opaque);
            }
            // 2. Transparent objects, farthest first, depth first then colour (REQ-SCN-040).
            for group in &lists.transparent_objects {
                draw.blended(
                    &lists.transparent,
                    &lists.transparent_runs,
                    group.runs,
                    transparent_base,
                );
            }
            // 3. HUD (REQ-SCN-044), which never bends (REQ-SCN-068).
            draw.variant = hud_variant(variant);
            for group in &lists.hud_objects {
                draw.blended(&lists.hud, &lists.hud_runs, group.runs, hud_base);
            }
            draw.flush();

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
    /// transparent runs, then the HUD runs, in the order of the plan's run lists), the run of
    /// each instance and, when `batched`, the indirect arguments of each run in the same order.
    fn upload_draws(&mut self, constants: &SceneConstants, batched: bool) {
        let lists = &self.plan.lists;
        let (draws, run_of, deforms) = (
            &mut self.draw_data,
            &mut self.run_of_data,
            &mut self.deform_data,
        );
        draws.clear();
        run_of.clear();
        deforms.clear();
        let shadow_enabled = constants.shadow_enabled;
        // The light of each bucket: the scene's, the scene's, the HUD's (REQ-SCN-029/044).
        let buckets = [
            (&lists.opaque, &lists.opaque_runs, false, true),
            (&lists.transparent, &lists.transparent_runs, false, false),
            (&lists.hud, &lists.hud_runs, true, false),
        ];
        let mut entry = 0;
        for (items, runs, hud, opaque) in buckets {
            for run in runs {
                let look = DrawLook {
                    hud,
                    opaque,
                    shadow_enabled,
                };
                push_draw(draws, deforms, &items[run.first], look);
                push_run_of(run_of, run, entry);
                entry += 1;
            }
        }
        if let (Some(batching), true) = (&mut self.batching, batched) {
            batching.data.clear();
            for (items, runs) in [
                (&lists.opaque, &lists.opaque_runs),
                (&lists.transparent, &lists.transparent_runs),
                (&lists.hud, &lists.hud_runs),
            ] {
                for run in runs {
                    push_indirect(&mut batching.data, &self.meshes, items[run.first].mesh, run);
                }
            }
            batching
                .indirect
                .reserve(&self.device, "zax indirect", batching.data.len() as u64);
            self.queue
                .write_buffer(&batching.indirect.buffer, 0, &batching.data);
        }

        let instances = &self.plan.instance_data;
        let grew = [
            self.draws
                .reserve(&self.device, "zax draws", self.draw_data.len() as u64),
            self.instances
                .reserve(&self.device, "zax instances", instances.len() as u64),
            self.run_of
                .reserve(&self.device, "zax run of", self.run_of_data.len() as u64),
            self.deforms
                .reserve(&self.device, "zax deforms", self.deform_data.len() as u64),
        ];
        if grew.contains(&true) {
            self.draw_group = draw_group(
                &self.device,
                &self.layouts,
                [&self.draws, &self.instances, &self.run_of, &self.deforms],
            );
        }
        self.queue
            .write_buffer(&self.draws.buffer, 0, &self.draw_data);
        self.queue
            .write_buffer(&self.instances.buffer, 0, instances);
        self.queue
            .write_buffer(&self.run_of.buffer, 0, &self.run_of_data);
        if !self.deform_data.is_empty() {
            self.queue
                .write_buffer(&self.deforms.buffer, 0, &self.deform_data);
        }
    }

    fn prepare_pipelines(
        &mut self,
        cull: bool,
        samples: u32,
        batched: bool,
        variant: ShaderVariant,
    ) {
        let lists = &self.plan.lists;
        let mut wanted = Vec::new();
        let mut want = |items: &[WgpuDrawItem],
                        runs: &[InstanceRun],
                        modes: &[SceneMode],
                        variant: ShaderVariant| {
            for run in runs {
                let mirrored = interop::normal_matrix(&items[run.first].model).1 < 0.0;
                for &mode in modes {
                    wanted.push(SceneKey {
                        mode,
                        cull,
                        mirrored,
                        samples,
                        batched,
                        variant,
                    });
                }
            }
        };
        want(
            &lists.opaque,
            &lists.opaque_runs,
            &[SceneMode::Opaque],
            variant,
        );
        let blended = [SceneMode::DepthOnly, SceneMode::ColorEqual];
        want(
            &lists.transparent,
            &lists.transparent_runs,
            &blended,
            variant,
        );
        // REQ-SCN-068: the HUD never bends, so it asks for the plain variant — which the OpenGL
        // backend does by clearing the bit in `draw_hud`, and which is why that variant's
        // pipelines have to exist even on a frame that bends everything else.
        want(&lists.hud, &lists.hud_runs, &blended, hud_variant(variant));
        for key in wanted {
            if !self.scene_pipelines.contains_key(&key) {
                let (layout, module) = match (&self.batching, key.batched) {
                    (Some(batching), true) => (
                        &batching.pipeline_layout,
                        &self.batched_modules[&key.variant],
                    ),
                    _ => (
                        &self.layouts.scene_pipeline,
                        &self.scene_modules[&key.variant],
                    ),
                };
                let pipeline = scene_pipeline(&self.device, layout, module, key);
                self.scene_pipelines.insert(key, pipeline);
            }
        }
    }

    /// Readies the texture arrays of the batched draws for the resident textures. False when
    /// the frame goes out one draw call per run: the device cannot batch, or more textures are
    /// resident than the arrays hold.
    fn prepare_texture_arrays(&mut self) -> bool {
        let Some(batching) = &mut self.batching else {
            return false;
        };
        if self.textures.slot_count() >= BINDLESS_TEXTURES as usize {
            return false;
        }
        let revision = self.textures.revision();
        if batching
            .group
            .as_ref()
            .is_some_and(|(built, _)| *built == revision)
        {
            return true;
        }

        let mut indexed = vec![&self.dummy_indexed];
        let mut direct = vec![&self.dummy_color];
        for view in self.textures.slot_views() {
            let format = view.map(|view| view.texture().format());
            indexed.push(match view {
                Some(view) if format == Some(wgpu::TextureFormat::R8Uint) => view,
                _ => &self.dummy_indexed,
            });
            direct.push(match view {
                Some(view) if format == Some(wgpu::TextureFormat::Rgba8Unorm) => view,
                _ => &self.dummy_color,
            });
        }
        let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("zax texture arrays"),
            layout: &batching.textures,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureViewArray(&indexed),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureViewArray(&direct),
                },
            ],
        });
        batching.group = Some((revision, group));
        true
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

/// Where the texture of a draw item sits in the texture arrays of the batched draws: the slot
/// of the texture its colour mode reads plus one, or 0, the dummies.
fn texture_slot(item: &WgpuDrawItem) -> i32 {
    let (indexed, direct) = texture_key(item);
    indexed
        .or(direct)
        .map_or(0, |handle| i32::try_from(handle.0 + 1).unwrap_or(0))
}

/// Issues the draws of the scene buckets: one draw call per run, or, with indirect arguments,
/// one multi-draw per stretch of runs that share a pipeline.
struct Drawer<'p, 'a> {
    pass: &'p mut wgpu::RenderPass<'a>,
    pipelines: &'p FastMap<SceneKey, wgpu::RenderPipeline>,
    texture_groups: &'p FastMap<(Option<TextureHandle>, Option<TextureHandle>), wgpu::BindGroup>,
    meshes: &'p MeshStore,
    statistics: &'p mut RenderStatistics,
    cull: bool,
    samples: u32,
    /// The variant the runs drawn now ask for; the HUD puts the plain one back (REQ-SCN-068).
    variant: ShaderVariant,
    current: Option<SceneKey>,
    bound_textures: Option<(Option<TextureHandle>, Option<TextureHandle>)>,
    /// The indirect arguments of the runs when the draws are batched.
    indirect: Option<&'p wgpu::Buffer>,
    /// The pipeline and the runs (indirect entries) not drawn yet.
    pending: Option<(SceneKey, std::ops::Range<usize>)>,
}

impl Drawer<'_, '_> {
    /// Draws `run` of `items` in `mode`; `entry` is the run's place in the indirect arguments.
    fn run(&mut self, items: &[WgpuDrawItem], run: &InstanceRun, entry: usize, mode: SceneMode) {
        let item = &items[run.first];
        let key = SceneKey {
            mode,
            cull: self.cull,
            mirrored: interop::normal_matrix(&item.model).1 < 0.0,
            samples: self.samples,
            batched: self.indirect.is_some(),
            variant: self.variant,
        };
        let instances = u32::try_from(run.count).unwrap_or(u32::MAX);
        self.statistics.batches += 1;
        self.statistics.draw_calls += instances;
        self.statistics.triangles += u64::from(item.mesh.index_count / 3) * u64::from(instances);

        if self.indirect.is_some() {
            match &mut self.pending {
                Some((pending, runs)) if *pending == key && runs.end == entry => runs.end += 1,
                _ => {
                    self.flush();
                    self.pending = Some((key, entry..entry + 1));
                }
            }
            return;
        }

        self.bind(key);
        let textures = texture_key(item);
        if self.bound_textures != Some(textures) {
            self.pass
                .set_bind_group(2, &self.texture_groups[&textures], &[]);
            self.bound_textures = Some(textures);
        }
        let (first_index, base_vertex) = self.meshes.range(item.mesh);
        let base = instance_base(run);
        self.pass.draw_indexed(
            first_index..first_index + item.mesh.index_count,
            base_vertex,
            base..base + instances,
        );
    }

    /// Draws one blended object, the runs `range` of `runs` (REQ-SCN-040, AMD-012), whose
    /// indirect arguments start at `base`.
    fn blended(
        &mut self,
        items: &[WgpuDrawItem],
        runs: &[InstanceRun],
        range: (usize, usize),
        base: usize,
    ) {
        for mode in [SceneMode::DepthOnly, SceneMode::ColorEqual] {
            for (index, run) in runs[range.0..range.1].iter().enumerate() {
                self.run(items, run, base + range.0 + index, mode);
            }
        }
    }

    /// Draws the runs not drawn yet, when batched.
    fn flush(&mut self) {
        let (Some(indirect), Some((key, runs))) = (self.indirect, self.pending.take()) else {
            return;
        };
        self.bind(key);
        multi_draw(self.pass, indirect, &runs);
    }

    fn bind(&mut self, key: SceneKey) {
        if self.current != Some(key) {
            self.pass.set_pipeline(&self.pipelines[&key]);
            self.current = Some(key);
        }
    }
}

/// Draws the runs `entries` of `indirect` in one multi-draw.
fn multi_draw(
    pass: &mut wgpu::RenderPass<'_>,
    indirect: &wgpu::Buffer,
    entries: &std::ops::Range<usize>,
) {
    if entries.is_empty() {
        return;
    }
    pass.multi_draw_indexed_indirect(
        indirect,
        entries.start as u64 * INDIRECT_BYTES,
        u32::try_from(entries.len()).unwrap_or(u32::MAX),
    );
}

/// The bindless texture chunk with the array length of this build substituted in.
fn bindless_textures() -> String {
    TEXTURES_BINDLESS.replace("ZAX_BINDLESS_TEXTURES", &BINDLESS_TEXTURES.to_string())
}

/// Whether the batched draws can be used: the device has their features, executes indirect
/// draws and takes texture arrays as long as theirs. `ZAXNES_WGPU_BATCHING=0` turns them off.
fn batching_supported(adapter: &wgpu::Adapter, device: &wgpu::Device) -> bool {
    if std::env::var_os("ZAXNES_WGPU_BATCHING").is_some_and(|value| value == "0") {
        return false;
    }
    device.features().contains(BATCHING_FEATURES)
        && adapter
            .get_downlevel_capabilities()
            .flags
            .contains(wgpu::DownlevelFlags::INDIRECT_EXECUTION)
        && device.limits().max_binding_array_elements_per_shader_stage >= 2 * BINDLESS_TEXTURES
}

// --------------------------------------------------------------------- shaders

const COMMON: &str = include_str!("shaders/common.wgsl");
/// The space bending of AMD-043 REQ-SCN-067, prepended to the `ZAX_BEND` variants only.
const BEND: &str = include_str!("shaders/bend.wgsl");
/// The voxel grid of AMD-055 REQ-SCN-081, prepended to the `ZAX_VOXEL_GRID` variants only.
const VOXEL_GRID: &str = include_str!("shaders/voxel_grid.wgsl");
/// The pixel shadows of AMD-055 REQ-SCN-082, prepended to the `ZAX_PIXEL_SHADOW` variants only.
const PIXEL_SHADOW: &str = include_str!("shaders/pixel_shadow.wgsl");
const SCENE: &str = include_str!("shaders/scene.wgsl");
const TEXTURES_BOUND: &str = include_str!("shaders/textures_bound.wgsl");
const TEXTURES_BINDLESS: &str = include_str!("shaders/textures_bindless.wgsl");
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
                // AMD-055 REQ-SCN-082 point 3 applies the animated deformation to the snapped
                // voxel centre, and the snapping is a **fragment** computation — so the
                // deformations are visible to both stages, as the OpenGL path's deformation
                // uniforms have always been (`deformation.glsl` is a chunk of both stages). The
                // placements of binding 1 are not: only the linear part of one instance's is
                // wanted there, and it travels as flat varyings in the pixel-shadow variant.
                storage(3, both),
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

/// One shader module: the sRGB constant of the sky, the variant's chunks and `parts` as its body,
/// every source expanded with the variant's defines (`preprocess`).
///
/// A marker the expander cannot read is a fault in a source of this crate, so it is pushed into
/// the WGSL as a `const_assert` that names it: `naga` then fails module creation with the bad
/// line in its message, which is both louder and nearer the cause than a silent miss.
fn build_module(
    device: &wgpu::Device,
    label: &str,
    parts: &[&str],
    variant: ShaderVariant,
) -> wgpu::ShaderModule {
    let defines = variant.defines();
    let mut chunks = variant.chunks();
    chunks.extend_from_slice(parts);
    let composed = preprocess::compose(&chunks, &defines);
    let mut source = format!("const ZAX_SRGB_DOUBLE: f32 = {:?};\n", sky::SRGB_DOUBLE);
    source.push_str(&composed.wgsl);
    if let Some(problem) = &composed.problem {
        use std::fmt::Write as _;
        let _ = write!(
            source,
            "\n// unreadable variant marker in a WGSL source of this crate:\n\
             const_assert false; // {}\n",
            problem.replace('\n', " ")
        );
    }
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(&format!("{label}{}", variant.suffix())),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    })
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
    layout: &wgpu::PipelineLayout,
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
        layout: Some(layout),
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

/// The draw blocks, the placements, the run of each instance and the deformations of a pass.
fn draw_group(
    device: &wgpu::Device,
    layouts: &Layouts,
    [draws, instances, run_of, deforms]: [&GrowingBuffer; 4],
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
            wgpu::BindGroupEntry {
                binding: 3,
                resource: deforms.buffer.as_entire_binding(),
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

/// The first instance of `run` in the placements (each placement is `AFFINE_BYTES` long).
fn instance_base(run: &InstanceRun) -> u32 {
    u32::try_from(run.offset / AFFINE_BYTES).unwrap_or(u32::MAX)
}

/// Appends one placement: the first three rows of the model matrix (the affine part the shader
/// applies) and the deformation's tile position.
fn push_affine_placement(data: &mut Vec<u8>, model: &Mat4, deform: Option<&DeformParameters>) {
    let tile = deform.map_or(Vec3::ZERO, |deform| deform.tile_relative_to_pivot);
    let cells = [
        model[(0, 0)],
        model[(0, 1)],
        model[(0, 2)],
        model[(0, 3)],
        model[(1, 0)],
        model[(1, 1)],
        model[(1, 2)],
        model[(1, 3)],
        model[(2, 0)],
        model[(2, 1)],
        model[(2, 2)],
        model[(2, 3)],
        tile.x,
        tile.y,
        tile.z,
        0.0,
    ];
    let mut block = [0_u8; AFFINE_BYTES];
    for (slot, value) in block.chunks_exact_mut(4).zip(cells) {
        slot.copy_from_slice(&value.to_ne_bytes());
    }
    data.extend_from_slice(&block);
}

/// Appends the indirect arguments of `run`, whose instances are all of `mesh`.
fn push_indirect(data: &mut Vec<u8>, meshes: &MeshStore, mesh: MeshHandle, run: &InstanceRun) {
    let (first_index, base_vertex) = meshes.range(mesh);
    data.extend_from_slice(&mesh.index_count.to_ne_bytes());
    data.extend_from_slice(&count(run.count).to_ne_bytes());
    data.extend_from_slice(&first_index.to_ne_bytes());
    data.extend_from_slice(&base_vertex.to_ne_bytes());
    data.extend_from_slice(&instance_base(run).to_ne_bytes());
}

/// Appends the draw block index `entry` once per instance of `run`, at the run's place.
fn push_run_of(data: &mut Vec<u8>, run: &InstanceRun, entry: usize) {
    let base = run.offset / AFFINE_BYTES * 4;
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

/// The variant the HUD draws with: `variant` with every axis REQ-SCN-068 and AMD-055 exclude the
/// HUD from cleared, and the one it does **not** exclude kept.
///
/// The twin of what `draw_hud` does on the OpenGL side, which clears exactly three bits — the
/// curvature (REQ-SCN-068: the HUD draws flat), the voxel grid (REQ-SCN-081: "the HUD never takes
/// the grid") and the pixel shadows (the HUD carries no shadow to read) — and leaves the
/// interpolation alone. AMD-058's centroid is about reading the atlas at the right texel, which
/// the HUD's own tiles need as much as the scene's.
const fn hud_variant(variant: ShaderVariant) -> ShaderVariant {
    ShaderVariant {
        bend: false,
        voxel_grid: false,
        pixel_shadow: false,
        centroid: variant.centroid,
    }
}

/// The four curvature `vec4` of AMD-043 REQ-SCN-067, the arc pivot riding in the `w` of the
/// first: the layout `bend.wgsl` reads, and the same five values `bind_bend` sets as uniforms on
/// the OpenGL side.
///
/// Written on **every** frame, bending or not — the block has one layout, and at curvature 0
/// `SpaceBend::NONE` makes these four writes sixteen zeros, which is what the buffer already
/// held. No branch, and nothing for a Classic frame to pay beyond the 64 bytes of the copy it is
/// already making.
fn push_bend(data: &mut Vec<u8>, bend: SpaceBend) {
    push_vec4(data, vec3(bend.curvature().into(), bend.arc_pivot()));
    push_vec4(data, vec3(bend.recentring().into(), 0.0));
    push_vec4(data, vec3(bend.offset().into(), 0.0));
    push_vec4(data, vec3(bend.active().into(), 0.0));
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
    push_vec4(&mut data, vec3(constants.light_direction, 0.0));
    push_vec4(&mut data, vec3(constants.hud_light_direction, 0.0));
    push_bend(&mut data, constants.bend);
    // AMD-055 REQ-SCN-081: the voxel cell side. Written whatever the variant, for the reason
    // `push_bend` gives, and in Classic it is the cell of `spl.voxel_grid` = 0.
    push_vec4(&mut data, [constants.voxel_cell, 0.0, 0.0, 0.0]);
    data
}

/// Appends `deform` to the deformations and returns its index, or -1 without one.
fn push_deformation(deforms: &mut Vec<u8>, deform: Option<&DeformParameters>) -> i32 {
    let Some(parameters) = deform else {
        return -1;
    };
    let index = i32::try_from(deforms.len() as u64 / DEFORM_BYTES).unwrap_or(i32::MAX);
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
    push_vec4(deforms, vec3(parameters.amplitude.into(), 0.0));
    push_vec4(deforms, vec3(parameters.size.into(), 0.0));
    rows(&parameters.speed, deforms);
    rows(&parameters.phase, deforms);
    index
}

/// Which light and alpha a run of the scene pass takes, and whether shadows are received.
#[derive(Debug, Clone, Copy)]
struct DrawLook {
    hud: bool,
    opaque: bool,
    shadow_enabled: bool,
}

/// The draw block of one instanced run of the scene pass (the uniforms `Passes::draw_run` sets
/// on the GL path), written in one piece.
fn push_draw(data: &mut Vec<u8>, deforms: &mut Vec<u8>, item: &WgpuDrawItem, look: DrawLook) {
    let mirrored = interop::normal_matrix(&item.model).1 < 0.0;
    let alpha = if look.opaque { 1.0 } else { item.alpha };
    let floats = [
        item.flat_color.x,
        item.flat_color.y,
        item.flat_color.z,
        alpha,
        item.texture_size.x,
        item.texture_size.y,
        item.tex_offset.x,
        item.tex_offset.y,
        item.tex_tilting.x,
        item.tex_tilting.y,
        if mirrored { -1.0 } else { 1.0 },
        if look.hud { 1.0 } else { 0.0 },
    ];
    let deform = push_deformation(deforms, item.deform.as_ref());
    let ints = [
        item.color_mode,
        item.palette_table,
        i32::from(item.clip),
        i32::from(look.shadow_enabled && item.receive_shadow),
        deform,
        texture_slot(item),
        // AMD-055 REQ-SCN-082: the voxel z offset of this run's mesh, the `uVoxelZOffset` uniform
        // of the OpenGL path. It rides in this spare integer slot and `pixel_shadow.wgsl` reads it
        // back with `bitcast`, which keeps the exact value `style::voxel_z_offset` returns without
        // growing the per-run block — the block every draw of every variant uploads.
        i32::from_ne_bytes(crate::style::voxel_z_offset(item.mesh.build_depth).to_ne_bytes()),
        0,
    ];
    let mut block = [0_u8; DRAW_BLOCK];
    for (slot, value) in block.chunks_exact_mut(4).zip(floats) {
        slot.copy_from_slice(&value.to_ne_bytes());
    }
    for (slot, value) in block[48..].chunks_exact_mut(4).zip(ints) {
        slot.copy_from_slice(&value.to_ne_bytes());
    }
    data.extend_from_slice(&block);
}

/// The draw block of one run of casters of the shadow pass: only its deformation matters.
fn push_shadow_draw(data: &mut Vec<u8>, deforms: &mut Vec<u8>, caster: &WgpuDrawItem) {
    let deform = push_deformation(deforms, caster.deform.as_ref());
    let mut block = [0_u8; DRAW_BLOCK];
    block[64..68].copy_from_slice(&deform.to_ne_bytes());
    data.extend_from_slice(&block);
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
