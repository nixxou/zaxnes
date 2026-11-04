//! The depth-detecting SMAA of AMD-060 REQ-SCN-085 on the `wgpu` backend: its three passes, its
//! targets and the two precomputed tables of the reference — the twin of [`crate::gl::smaa`].
//!
//! **The lifecycle is REQ-SCN-086**, and it is the OpenGL module's: an [`Option`] the renderer
//! holds, built the first frame `video.anti_alias` is 3 and dropped the first frame it is not. At
//! every other level "aucune passe, aucune cible et aucune texture du SMAA NE DOIT exister ni être
//! soumise", and the whole cost of this file to such a frame is the `is_none` test that decides
//! not to run it.
//!
//! What this module has that the OpenGL one does not, and what it does not have:
//!
//! - **one uniform block, not one per pass.** [`super::screen_effects`] needs a block per pass
//!   group because `wgpu` applies every `write_buffer` of a submission before the command buffers
//!   run, so two passes of different target sizes would both read the second value. The three
//!   passes of this filter read *the same* two `vec4` — the metrics and the drawn scale of one
//!   frame — so one block written once a frame is not a shortcut but the whole of what they need;
//! - **no depth resolve.** REQ-SCN-085's first clause renders the scene without multisampling, so
//!   the frame's depth already is the single-sample one and RU-7's resolve pass is not involved;
//! - **three render passes** rather than three framebuffer binds, which is why `smaa_passes` reads
//!   the same 3 on both backends.
//!
//! The formats are the OpenGL module's, and all three are colour-renderable on every adapter
//! WebGPU admits: `Rg8Unorm` for the two edge channels, `Rgba8Unorm` for the four weights, and the
//! `Rgba16Float` of the linear buffer for the filtered colour.
//!
//! The two tables come from `third_party/smaa` (`iryoku/smaa`, MIT; notice in
//! `THIRD-PARTY-NOTICES.md`) and are the same bytes the OpenGL path uploads — `src/assets/smaa/`,
//! shared by both backends, hashed by `tests/amd060_smaa_tests.rs`.

use crate::numerics::Vector4;

use super::screen_effects::{allocate, Target};

/// The edges of the first pass: two channels in `[0, 1]`.
const EDGE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rg8Unorm;

/// The blending weights of the second: four.
const WEIGHT_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// The filtered colour, which is the linear buffer's own format (AMD-021 R2-009).
const BLEND_FORMAT: wgpu::TextureFormat = super::renderer::LINEAR_FORMAT;

/// Bytes one texel of each of the three targets occupies, as `smaa_bytes` counts them.
const EDGE_BYTES_PER_TEXEL: u64 = 2;
const WEIGHT_BYTES_PER_TEXEL: u64 = 4;
const BLEND_BYTES_PER_TEXEL: u64 = 8;

/// The area table of the reference, 160 × 560 `R8G8` (`AREATEX_PITCH` = 320).
const AREA_SIZE: (u32, u32) = (160, 560);

/// The search table, 64 × 16 `R8` — the cropped shape its own `SMAA_SEARCHTEX_PACKED_SIZE` names.
const SEARCH_SIZE: (u32, u32) = (64, 16);

/// Bytes of the [`Smaa`](shaders/smaa.wgsl) block: two `vec4`.
const SMAA_BYTES: usize = 2 * 16;

/// The two tables, which live with the other shader data in [`crate::gl::shaders`] — shared by
/// both backends, because the bytes of a precomputed table are not an API's.
use crate::gl::shaders::{SMAA_AREA_TABLE as AREA_BYTES, SMAA_SEARCH_TABLE as SEARCH_BYTES};

/// What the three passes need of the frame they filter.
///
/// There is no `capacity` beside `size`, for the reason [`super::screen_effects::ScreenFrame`]
/// has none: this backend allocates its targets at the frame's exact size, so the drawn scale is
/// (1, 1).
#[derive(Debug, Clone, Copy)]
pub struct SmaaFrame<'a> {
    /// The frame's depth, single-sample — what the first pass detects contours in. REQ-SCN-085
    /// renders the scene without multisampling, so this is the frame's own depth texture.
    pub depth: &'a wgpu::TextureView,
    /// The frame's linear colour, which the third pass mixes two texels of.
    pub color: &'a wgpu::TextureView,
    /// The size of the attachments, which the three targets follow.
    pub size: (u32, u32),
    /// Which allocation of the frame's attachments these views belong to, so that a resize
    /// rebuilds the bind groups that name them — the counter [`super::screen_effects`] uses, and
    /// for the same reason: a `wgpu::TextureView` is a handle with no identity to compare.
    pub generation: u64,
}

/// The three passes of REQ-SCN-085 and everything they own, held only while the level is 3.
#[derive(Debug)]
pub struct Smaa {
    layout: wgpu::BindGroupLayout,
    /// The filtered sampler of every read but the depth's and the frame colour's: the reference's
    /// own `LinearSampler`, which the searches of the second pass and both area lookups depend on.
    sampler: wgpu::Sampler,
    /// What a source slot a pass does not read is bound to.
    dummy: wgpu::TextureView,
    /// The one block the three passes share, written once a frame.
    uniforms: wgpu::Buffer,
    /// The two tables of the reference, uploaded once with the module.
    area: wgpu::TextureView,
    search: wgpu::TextureView,
    /// The depth edge detection of the reference's `SMAADepthEdgeDetectionPS`.
    edges: wgpu::RenderPipeline,
    /// The reference's `SMAABlendingWeightCalculationPS`.
    weights: wgpu::RenderPipeline,
    /// The reference's `SMAANeighborhoodBlendingPS`.
    blend: wgpu::RenderPipeline,
    /// `Rg8Unorm`, the west and north edge of every pixel; cleared every frame by its pass's own
    /// `LoadOp::Clear`, because the first pass `discard`s where there is no contour.
    edge_target: Option<Target>,
    /// `Rgba8Unorm`, the blending weight of every pixel. Loaded rather than cleared: the second
    /// pass writes every pixel, zero included.
    weight_target: Option<Target>,
    /// `Rgba16Float`, the filtered colour — the one target of the three the filter cannot do
    /// without, because the neighbourhood blending writes a mix of two texels of the texture it
    /// reads and so cannot work in place the way REQ-SCN-083's composite does.
    blend_target: Option<Target>,
    /// The bind groups of the three passes, rebuilt with the targets.
    groups: Option<[wgpu::BindGroup; 3]>,
    /// The group the sRGB encode reads the filtered colour through, built with the blend target
    /// against the renderer's own resolve layout. This is this backend's spelling of the OpenGL
    /// path's `LinearColorBuffer::redirect_color`.
    resolve: Option<wgpu::BindGroup>,
    /// The frame allocation the groups were built against.
    generation: u64,
}

impl Smaa {
    /// How many pipelines this module holds, for `RenderStatistics::smaa_programs` — the same 3
    /// the OpenGL module counts, since one program there is one pipeline here.
    pub const PROGRAMS: u32 = 3;

    /// Builds the module: the layout the three passes share, the sampler, the block, the two
    /// tables uploaded to the device and the three pipelines.
    ///
    /// Nothing here is fallible, and that is a property of the API rather than a shortcut — the
    /// same one [`super::renderer::WgpuSceneRenderer::prepare_screen_effects`] records: where the
    /// OpenGL module returns a `GlError` when a program will not link or a target will not
    /// allocate, `wgpu` reports both through the device's error scope and hands back a usable
    /// handle. REQ-SCN-086's fallback is therefore *asked of the adapter* before this is called.
    // Every GPU object of the filter, made once: the layout's six entries, the sampler, the block,
    // the two tables and the three pipelines. Splitting it would move the list rather than shorten
    // it — the shape `ScreenEffects::new` has for the same reason.
    #[allow(clippy::too_many_lines)]
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let source = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                // Filterable, and it has to be: the searches of the second pass fetch four edges
                // at a time through the filter (`@PSEUDO_GATHER4`) and both area lookups
                // interpolate between two distances. All four formats that reach these slots
                // (`Rg8Unorm`, `Rgba8Unorm`, `R8Unorm`, `Rgba16Float`) are.
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("zax smaa"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(SMAA_BYTES as u64),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
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
                source(3),
                source(4),
                source(5),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("zax smaa"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = |label: &'static str, body: &'static str, format| {
            let composed = super::preprocess::compose(&[SMAA, body], &[]);
            let module = super::renderer::shader_module(
                device,
                label,
                &composed.wgsl,
                composed.problem.as_deref(),
            );
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs_smaa"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState::default(),
                // No depth attachment on any of the three: they read the frame's depth as a
                // texture and write none. The HUD that follows the filter attaches it again.
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
            })
        };

        Self {
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("zax smaa"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::MipmapFilterMode::Nearest,
                ..wgpu::SamplerDescriptor::default()
            }),
            dummy: super::resources::upload(
                device,
                queue,
                wgpu::TextureFormat::R8Unorm,
                (1, 1),
                &[0],
            ),
            uniforms: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("zax smaa"),
                size: SMAA_BYTES as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            // Both tables are read with bilinear filtering, as the reference's own `LinearSampler`
            // does, and in their own coordinate space: a data texture's row 0 sits at v = 0 under
            // both APIs, so the bytes and the reads are the OpenGL path's unchanged.
            area: super::resources::upload(
                device,
                queue,
                wgpu::TextureFormat::Rg8Unorm,
                AREA_SIZE,
                AREA_BYTES,
            ),
            search: super::resources::upload(
                device,
                queue,
                wgpu::TextureFormat::R8Unorm,
                SEARCH_SIZE,
                SEARCH_BYTES,
            ),
            edges: pipeline("zax smaa edges", SMAA_EDGES, EDGE_FORMAT),
            weights: pipeline("zax smaa weights", SMAA_WEIGHTS, WEIGHT_FORMAT),
            blend: pipeline("zax smaa blend", SMAA_BLEND, BLEND_FORMAT),
            edge_target: None,
            weight_target: None,
            blend_target: None,
            groups: None,
            resolve: None,
            generation: u64::MAX,
            layout,
        }
    }

    /// Bytes its targets occupy, for `RenderStatistics::smaa_bytes`; 0 until a pass has run once.
    ///
    /// The two tables are **not** counted, for the reason the OpenGL module gives: they do not
    /// grow with the resolution, and what the statistic is read for is the cost the mode adds per
    /// pixel. Their 180 224 bytes are in `src/assets/smaa/README.md` instead.
    pub fn bytes(&self) -> u64 {
        bytes(self.edge_target.as_ref(), EDGE_BYTES_PER_TEXEL)
            + bytes(self.weight_target.as_ref(), WEIGHT_BYTES_PER_TEXEL)
            + bytes(self.blend_target.as_ref(), BLEND_BYTES_PER_TEXEL)
    }

    /// The filtered colour the frame carries on in: the view the HUD of REQ-SCN-044 draws into,
    /// and the group the sRGB encode reads instead of the frame's own colour. `None` before the
    /// first filtered frame.
    pub fn blended(&self) -> Option<(&wgpu::TextureView, &wgpu::BindGroup)> {
        match (&self.blend_target, &self.resolve) {
            (Some(target), Some(group)) => Some((&target.view, group)),
            _ => None,
        }
    }

    /// Runs the three passes of REQ-SCN-085 over `frame` and says how many that took.
    ///
    /// `resolve_layout` is the renderer's own resolve layout, so that the group the sRGB encode
    /// will read the filtered colour through is built with the target it names.
    pub fn filter(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        frame: SmaaFrame<'_>,
        resolve_layout: &wgpu::BindGroupLayout,
    ) -> u32 {
        let metrics = rt_metrics(frame.size);
        let block = {
            let mut data = Vec::with_capacity(SMAA_BYTES);
            // The attachments are allocated at the frame's exact size, so the drawn scale is
            // (1, 1); `zw` is `SMAA_RT_METRICS.zw`, the size in pixels.
            push(&mut data, [1.0, 1.0, metrics.z, metrics.w]);
            push(&mut data, [metrics.x, metrics.y, metrics.z, metrics.w]);
            data
        };
        queue.write_buffer(&self.uniforms, 0, &block);

        self.allocate(device, frame, resolve_layout);
        let (Some(edges), Some(weights), Some(blended), Some(groups)) = (
            &self.edge_target,
            &self.weight_target,
            &self.blend_target,
            &self.groups,
        ) else {
            return 0;
        };

        // 1. The depth edge detection, into a target **cleared** every frame: the pass `discard`s
        //    where there is no contour, so what it leaves behind is what was there before it.
        // 2. The blending weights, through the two tables. Loaded, not cleared: this pass writes
        //    every pixel, zero included — which is what the OpenGL path's absence of a clear is.
        // 3. The neighbourhood blending, into the target the frame carries on with.
        for (label, pipeline, target, group, clear) in [
            ("zax smaa edges", &self.edges, &edges.view, &groups[0], true),
            (
                "zax smaa weights",
                &self.weights,
                &weights.view,
                &groups[1],
                false,
            ),
            (
                "zax smaa blend",
                &self.blend,
                &blended.view,
                &groups[2],
                false,
            ),
        ] {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(label),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: if clear {
                            wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.0,
                                g: 0.0,
                                b: 0.0,
                                a: 1.0,
                            })
                        } else {
                            wgpu::LoadOp::Load
                        },
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, group, &[]);
            pass.draw(0..3, 0..1);
        }
        3
    }

    /// Allocates the three targets at the frame's size, or reallocates them when that size or the
    /// frame's allocation changes, and builds the four bind groups with them.
    ///
    /// Out of line on purpose, as the OpenGL twin is: it runs on the first frame at level 3 and
    /// then never again until the view is resized.
    #[inline(never)]
    fn allocate(
        &mut self,
        device: &wgpu::Device,
        frame: SmaaFrame<'_>,
        resolve_layout: &wgpu::BindGroupLayout,
    ) {
        let sized =
            |target: &Option<Target>| matches!(target, Some(target) if target.size == frame.size);
        if !sized(&self.edge_target) {
            self.edge_target = Some(allocate(device, "zax smaa edges", frame.size, EDGE_FORMAT));
            self.groups = None;
        }
        if !sized(&self.weight_target) {
            self.weight_target = Some(allocate(
                device,
                "zax smaa weights",
                frame.size,
                WEIGHT_FORMAT,
            ));
            self.groups = None;
        }
        if !sized(&self.blend_target) {
            let target = allocate(device, "zax smaa blend", frame.size, BLEND_FORMAT);
            self.resolve = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("zax smaa resolve"),
                layout: resolve_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&target.view),
                }],
            }));
            self.blend_target = Some(target);
            self.groups = None;
        }
        // The frame's own depth and colour are named by two of the groups, so a reallocation of
        // the attachments rebuilds them even when every target of this module kept its size.
        if self.generation != frame.generation {
            self.generation = frame.generation;
            self.groups = None;
        }
        if self.groups.is_some() {
            return;
        }
        let (Some(edges), Some(weights)) = (&self.edge_target, &self.weight_target) else {
            return;
        };
        let group = |sources: [&wgpu::TextureView; 3]| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("zax smaa"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &self.uniforms,
                            offset: 0,
                            size: wgpu::BufferSize::new(SMAA_BYTES as u64),
                        }),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(frame.depth),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(sources[0]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(sources[1]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::TextureView(sources[2]),
                    },
                ],
            })
        };
        let dummy = &self.dummy;
        let groups = [
            // 1. The detector reads the frame's depth and nothing else, which is REQ-SCN-085's
            //    "la détection par luminance ou par couleur NE DOIT PAS être utilisée" made
            //    structural: the frame's colour is not even bound to this pass.
            group([dummy, dummy, dummy]),
            // 2. The edges, then the two tables.
            group([&edges.view, &self.area, &self.search]),
            // 3. The frame's colour, then the weights.
            group([frame.color, &weights.view, dummy]),
        ];
        self.groups = Some(groups);
    }
}

/// The reference's `SMAA_RT_METRICS` for a frame of `size` pixels: `(1/w, +1/h, w, h)`.
///
/// **The `y` is positive here, and that is the one number the two backends disagree about.** A
/// wgpu framebuffer's origin is top-left, which is the convention the reference is written in, so
/// its own `± offset · SMAA_RT_METRICS.y` arithmetic means on screen what it means in Direct3D.
/// The OpenGL path passes `−1/h` to reach the same orientation in a texture whose rows run upward
/// (`gl/smaa.rs::rt_metrics`, `shaders/smaa.glsl` point 1).
fn rt_metrics(size: (u32, u32)) -> Vector4 {
    #[allow(clippy::cast_precision_loss)] // Sizes far below 2^24.
    let (width, height) = (size.0.max(1) as f32, size.1.max(1) as f32);
    Vector4::new(1.0 / width, 1.0 / height, width, height)
}

fn push(data: &mut Vec<u8>, values: [f32; 4]) {
    for value in values {
        data.extend_from_slice(&value.to_ne_bytes());
    }
}

fn bytes(target: Option<&Target>, per_texel: u64) -> u64 {
    target.map_or(0, |target| {
        u64::from(target.size.0) * u64::from(target.size.1) * per_texel
    })
}

// --------------------------------------------------------------------- shaders

/// What the three passes share: the block, the bindings, the triangle and the reads
/// (`shaders/smaa.wgsl`, the twin of `shaders/smaa.glsl`).
const SMAA: &str = include_str!("shaders/smaa.wgsl");
const SMAA_EDGES: &str = include_str!("shaders/smaa_edges.wgsl");
const SMAA_WEIGHTS: &str = include_str!("shaders/smaa_weights.wgsl");
const SMAA_BLEND: &str = include_str!("shaders/smaa_blend.wgsl");

/// The four sources, for the test that reads what this backend's passes compute (AMD-060 §4).
#[must_use]
pub fn sources() -> [&'static str; 4] {
    [SMAA, SMAA_EDGES, SMAA_WEIGHTS, SMAA_BLEND]
}

#[cfg(test)]
mod tests {
    use super::{rt_metrics, AREA_BYTES, AREA_SIZE, SEARCH_BYTES, SEARCH_SIZE, SMAA_BYTES};

    /// AMD-060 §4: the tables are the reference's own, and the *shape* the metrics of
    /// `smaa_weights.wgsl` are built from is the one the reference declares. Their bytes are
    /// pinned by SHA-256 in `tests/amd060_smaa_tests.rs`, once for both backends.
    #[test]
    fn req_scn_085_the_tables_have_the_shape_the_reference_declares() {
        assert_eq!((160, 560), AREA_SIZE);
        assert_eq!(160 * 560 * 2, AREA_BYTES.len());
        assert_eq!((64, 16), SEARCH_SIZE);
        assert_eq!(64 * 16, SEARCH_BYTES.len());
    }

    /// The `y` of `SMAA_RT_METRICS` is **positive** on this backend, where the OpenGL one needs it
    /// negative: both mean "north is up on screen", in texture spaces whose rows run opposite
    /// ways. The algorithm is not symmetric under a mirror of it, so this is the one number that
    /// stands between the port and a result displaced along every vertical edge.
    #[test]
    fn req_scn_085_the_render_target_metrics_follow_the_top_left_origin() {
        let metrics = rt_metrics((640, 480));
        assert!((metrics.x - 1.0 / 640.0).abs() < f32::EPSILON);
        assert!((metrics.y - 1.0 / 480.0).abs() < f32::EPSILON);
        assert!(
            metrics.y > 0.0,
            "a wgpu framebuffer's rows run downward, as Direct3D's do"
        );
        assert!((metrics.z - 640.0).abs() < f32::EPSILON);
        assert!((metrics.w - 480.0).abs() < f32::EPSILON);
    }

    /// The block the WGSL declares is two `vec4`, and what is written has to be exactly that long
    /// or `min_binding_size` refuses the bind group — a mismatch no render would reach.
    #[test]
    fn the_block_is_as_long_as_the_wgsl_declares() {
        let mut data = Vec::new();
        super::push(&mut data, [1.0, 1.0, 640.0, 480.0]);
        super::push(&mut data, [1.0 / 640.0, 1.0 / 480.0, 640.0, 480.0]);
        assert_eq!(SMAA_BYTES, data.len());
    }
}
