//! The screen-space passes of the Enhanced style (AMD-055 REQ-SCN-080) on the `wgpu` backend: the
//! twin of [`crate::gl::screen_effects`], with the occlusion of REQ-SCN-083 and, from step 4c, the
//! reflections of REQ-SCN-084.
//!
//! **The lifecycle is the requirement**, and it is the OpenGL module's, for the same reasons: this
//! is an [`Option`] the renderer holds, and each effect inside it is an `Option` of its own, built
//! the first time a frame asks for *that* effect and dropped as soon as no frame does — which
//! takes its pipelines and its targets with it. In Classic the whole cost of AMD-055's screen side
//! is the `is_none` test that decides not to run it.
//!
//! The formats carry over as they are: `R8Unorm` for the occlusion, which is one scalar in
//! `[0, 1]`, and `Rgba16Float` for the reflections' trace, which is the format the linear buffer
//! has always been. Neither is a new capability on either `wgpu` backend.
//!
//! What this module has that the OpenGL one does not is [`ScreenEffects::resolve_depth`]. OpenGL
//! resolves a multisampled depth with one `glBlitFramebuffer`; `wgpu` has no depth resolve and no
//! depth blit, so the resolve is a pass of this module's own — see `shaders/depth_resolve.wgsl`,
//! which carries the whole argument.

use zax_engine3d::hashing::FastMap;

use crate::numerics::Vector4;
use crate::style::StyleSettings;

use super::preprocess;

/// The format of the occlusion targets and of the floor mask: one channel of eight bits.
const OCCLUSION_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R8Unorm;

/// Bytes one texel of an occlusion target or of the floor mask occupies.
const OCCLUSION_BYTES_PER_TEXEL: u64 = 1;

/// The format of the reflections' trace target: a linear colour, as the linear buffer's own
/// (REQ-SCN-084).
const REFLECTION_FORMAT: wgpu::TextureFormat = super::renderer::LINEAR_FORMAT;

/// Bytes one texel of it occupies: four half-float channels.
const REFLECTION_BYTES_PER_TEXEL: u64 = 8;

/// Bytes of the [`Screen`](shaders/screen_depth.wgsl) block: seven `vec4`.
const SCREEN_BYTES: usize = 7 * 16;

/// How far apart two blocks of [`ScreenEffects::uniforms`] sit. Each pass group writes its own,
/// and they have to be separate because `wgpu` applies every `write_buffer` of a submission
/// **before** the command buffers run: one block written twice would hand both passes the second
/// value.
const SCREEN_SLOT: u64 = 256;

/// Which block of [`ScreenEffects::uniforms`] a pass group reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Slot {
    /// The occlusion of REQ-SCN-083 and its blur, at the estimator's own resolution.
    Occlusion,
    /// The composite of REQ-SCN-083, at the frame's resolution.
    OcclusionComposite,
    /// The trace of REQ-SCN-084, at the quality's resolution.
    Reflections,
    /// The composite of REQ-SCN-084, at the frame's resolution.
    ReflectionsComposite,
}

impl Slot {
    const COUNT: u64 = 4;

    const fn offset(self) -> u64 {
        SCREEN_SLOT
            * match self {
                Self::Occlusion => 0,
                Self::OcclusionComposite => 1,
                Self::Reflections => 2,
                Self::ReflectionsComposite => 3,
            }
    }
}

/// What the passes of REQ-SCN-080 need of the frame they work on.
///
/// There is no `capacity` beside `size`: the OpenGL backend lets the host reserve room for a size
/// that changes often, and this one allocates its attachments at the frame's exact size, so the
/// `uDrawnScale` of the GLSL is (1, 1) here. It is still written into the block, because the WGSL
/// is the twin of a chunk both backends share and a scale that is always one costs one multiply.
#[derive(Debug, Clone, Copy)]
pub struct ScreenFrame<'a> {
    /// The frame's depth, single-sample (REQ-SCN-080's last bullet): the frame's own texture
    /// without anti-aliasing, and what [`ScreenEffects::resolve_depth`] wrote with it.
    pub depth: &'a wgpu::TextureView,
    /// The frame's linear colour, resolved: what the reflections of REQ-SCN-084 reflect.
    pub color: &'a wgpu::TextureView,
    /// The size of the attachments, which the targets follow.
    pub size: (u32, u32),
    /// Which allocation of the frame's attachments these views belong to, so that a resize
    /// rebuilds the bind groups that name them. `wgpu::TextureView` is a handle with no identity
    /// to compare, so the renderer counts its allocations instead.
    pub generation: u64,
    /// The projection as the WGSL reads it: `xy` the two scales, `z` the `range` term, `w` the
    /// near plane — read straight off the matrix the camera built.
    pub projection: Vector4,
}

/// One colour target and the size it was allocated at.
#[derive(Debug)]
struct Target {
    view: wgpu::TextureView,
    size: (u32, u32),
}

/// The hemisphere estimator of REQ-SCN-083 — the mode `spl.ao` = 2, and the declared fallback
/// **(D)** of the MSVO mode: two pipelines and the pair of `R8` targets of the raw pass and its
/// blur.
#[derive(Debug)]
struct Depth {
    pass: wgpu::RenderPipeline,
    blur: wgpu::RenderPipeline,
    raw: Option<Target>,
    blurred: Option<Target>,
    /// The bind groups of the raw pass, the blur and the composite, rebuilt with the targets.
    groups: Option<[wgpu::BindGroup; 3]>,
    /// 1 at `ao.quality` High, 2 otherwise (REQ-SCN-083: "à la moitié de chaque dimension de la
    /// cible de rendu courante sauf en High").
    divisor: u32,
    /// The frame allocation [`Self::groups`] was built against.
    generation: u64,
}

/// Which estimator of REQ-SCN-083 is in force. **One or the other is built, never both**, which is
/// the rule that keeps the two effects independent of each other, applied one level down.
#[derive(Debug)]
enum Estimator {
    /// `spl.ao` = 2, and `spl.ao` = 1 once the multi-scale estimator has been refused.
    Depth(Depth),
}

/// The occlusion of REQ-SCN-083: whichever estimator the mode asks for, and the composite the two
/// of them share.
#[derive(Debug)]
struct Occlusion {
    composite: wgpu::RenderPipeline,
    estimator: Estimator,
}

/// The reflections of REQ-SCN-084: the trace, its composite, the `R8` floor mask at the frame's
/// own size and the `Rgba16Float` trace target at the quality's resolution.
///
/// The **mask pipeline is not here**: its vertex stage is the scene's, so it lives with the other
/// scene pipelines in the renderer and is counted there — the same split the OpenGL module makes
/// for the same reason.
#[derive(Debug)]
struct Reflections {
    trace: wgpu::RenderPipeline,
    composite: wgpu::RenderPipeline,
    mask: Option<Target>,
    traced: Option<Target>,
    /// The bind groups of the trace and of the composite, rebuilt with the targets.
    groups: Option<[wgpu::BindGroup; 2]>,
    /// 1 at `ssr.quality` Ultra, 2 otherwise (REQ-SCN-084: "sauf en Ultra").
    divisor: u32,
    /// The frame allocation the mask and [`Self::groups`] were built against.
    generation: u64,
}

/// The depth resolve of REQ-SCN-080's last bullet, which only a multisampled frame needs: built
/// the first time one runs a screen pass, and kept after that.
#[derive(Debug)]
struct DepthResolve {
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    /// The group that names the multisampled depth, by frame allocation.
    group: Option<(u64, wgpu::BindGroup)>,
}

/// The screen-space passes and their resources, held only while a frame wants one.
#[derive(Debug)]
pub struct ScreenEffects {
    layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    /// The filtered sampler of the two composites, the only reads that are an upsample.
    sampler: wgpu::Sampler,
    /// What a source slot a pass does not read is bound to.
    dummy: wgpu::TextureView,
    uniforms: wgpu::Buffer,
    /// One module per source set asked for so far, so that a switch of effect does not recompile
    /// what it already has.
    modules: FastMap<&'static str, wgpu::ShaderModule>,
    occlusion: Option<Occlusion>,
    reflections: Option<Reflections>,
    depth_resolve: Option<DepthResolve>,
}

impl ScreenEffects {
    /// Creates the module with neither effect in it: the layout every screen pass takes, the
    /// sampler, the dummy source and the uniform blocks. [`Self::select`] then builds the halves
    /// the frame asks for.
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let source = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                // Filterable: the two composites sample their source up to the frame's size
                // through `zax_smooth`, and both formats (`R8Unorm`, `Rgba16Float`) are.
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("zax screen"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(SCREEN_BYTES as u64),
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
            label: Some("zax screen"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        Self {
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("zax screen"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::MipmapFilterMode::Nearest,
                ..wgpu::SamplerDescriptor::default()
            }),
            dummy: super::resources::upload(device, queue, OCCLUSION_FORMAT, (1, 1), &[0]),
            uniforms: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("zax screen"),
                size: SCREEN_SLOT * Slot::COUNT,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            modules: FastMap::default(),
            occlusion: None,
            reflections: None,
            depth_resolve: None,
            layout,
            pipeline_layout,
        }
    }

    /// Builds the halves `style` asks for and releases the others, once a frame.
    ///
    /// This is where the brief's "each effect pays only when it is on" is paid: a player who turns
    /// the occlusion on compiles its three pipelines and allocates its two targets, and nothing of
    /// the reflections exists.
    pub fn select(&mut self, device: &wgpu::Device, style: &StyleSettings) {
        if style.ao_active() {
            if self.occlusion.is_none() {
                let estimator = Estimator::Depth(Depth {
                    pass: self.screen_pipeline(device, "zax ao", AO, OCCLUSION_FORMAT, None),
                    blur: self.screen_pipeline(
                        device,
                        "zax ao blur",
                        AO_BLUR,
                        OCCLUSION_FORMAT,
                        None,
                    ),
                    raw: None,
                    blurred: None,
                    groups: None,
                    divisor: 0,
                    generation: u64::MAX,
                });
                // REQ-SCN-083: `dst x src`, so the full-screen triangle scales the linear buffer
                // in place — no second colour target and no copy of the scene. The OpenGL path's
                // `glBlendFunc(GL_ZERO, GL_SRC_COLOR)`, spelled for this API.
                let multiply = wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::Zero,
                    dst_factor: wgpu::BlendFactor::Src,
                    operation: wgpu::BlendOperation::Add,
                };
                let composite = self.screen_pipeline(
                    device,
                    "zax ao composite",
                    AO_COMPOSITE,
                    super::renderer::LINEAR_FORMAT,
                    Some(wgpu::BlendState {
                        color: multiply,
                        alpha: multiply,
                    }),
                );
                self.occlusion = Some(Occlusion {
                    composite,
                    estimator,
                });
            }
        } else {
            self.occlusion = None;
        }

        if style.reflections_active() {
            if self.reflections.is_none() {
                let trace = self.screen_pipeline(device, "zax ssr", SSR, REFLECTION_FORMAT, None);
                // REQ-SCN-084: the trace holds `rgb = w * c` and `a = w`, so `src + dst * (1 - a)`
                // is the `(1 - i) * surface + i * reflet` of the requirement, in place. The OpenGL
                // path's `glBlendFunc(GL_ONE, GL_ONE_MINUS_SRC_ALPHA)`, spelled for this API.
                let over = wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    operation: wgpu::BlendOperation::Add,
                };
                let composite = self.screen_pipeline(
                    device,
                    "zax ssr composite",
                    SSR_COMPOSITE,
                    super::renderer::LINEAR_FORMAT,
                    Some(wgpu::BlendState {
                        color: over,
                        alpha: over,
                    }),
                );
                self.reflections = Some(Reflections {
                    trace,
                    composite,
                    mask: None,
                    traced: None,
                    groups: None,
                    divisor: 0,
                    generation: u64::MAX,
                });
            }
        } else {
            self.reflections = None;
        }
    }

    /// Whether the occlusion of REQ-SCN-083 is in this module.
    pub const fn occludes(&self) -> bool {
        self.occlusion.is_some()
    }

    /// Whether the reflections of REQ-SCN-084 are.
    pub const fn reflects(&self) -> bool {
        self.reflections.is_some()
    }

    /// Whether anything in this module has a pass to run this frame.
    pub const fn active(&self) -> bool {
        self.occludes() || self.reflects()
    }

    /// How many pipelines this module holds, for `RenderStatistics::style_programs`: the shared
    /// composite plus its estimator's two, and two for the reflections (the trace and its
    /// composite) — as the OpenGL module counts its programs, and with the floor mask counted by
    /// the renderer there as here.
    pub fn pipelines(&self) -> u32 {
        let occlusion = self.occlusion.as_ref().map_or(0, |occlusion| {
            1 + match &occlusion.estimator {
                Estimator::Depth(_) => 2,
            }
        });
        occlusion + if self.reflections.is_some() { 2 } else { 0 }
    }

    /// Bytes its targets occupy, for `RenderStatistics::screen_effect_bytes`; 0 until a pass has
    /// run once.
    pub fn bytes(&self) -> u64 {
        let occlusion = self
            .occlusion
            .as_ref()
            .map_or(0, |occlusion| match &occlusion.estimator {
                Estimator::Depth(depth) => {
                    bytes(depth.raw.as_ref(), OCCLUSION_BYTES_PER_TEXEL)
                        + bytes(depth.blurred.as_ref(), OCCLUSION_BYTES_PER_TEXEL)
                }
            });
        let reflections = self.reflections.as_ref().map_or(0, |reflections| {
            bytes(reflections.mask.as_ref(), OCCLUSION_BYTES_PER_TEXEL)
                + bytes(reflections.traced.as_ref(), REFLECTION_BYTES_PER_TEXEL)
        });
        occlusion + reflections
    }

    // ------------------------------------------------- REQ-SCN-080, the resolved depth

    /// Writes sample 0 of a multisampled depth into a single-sample one, so that the passes below
    /// read the **resolved** depth REQ-SCN-080's last bullet names. One pass, and only on a
    /// multisampled frame: `shaders/depth_resolve.wgsl` carries the argument.
    pub fn resolve_depth(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        multisampled: &wgpu::TextureView,
        single: &wgpu::TextureView,
        generation: u64,
    ) -> u32 {
        // Cloned so that the map it lives in is no longer borrowed when `depth_resolve` is: both
        // are fields of `self` and both are reached through a method.
        let module = self.module(device, DEPTH_RESOLVE, &[]).clone();
        let resolve = self.depth_resolve.get_or_insert_with(|| {
            let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("zax depth resolve"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: true,
                    },
                    count: None,
                }],
            });
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("zax depth resolve"),
                bind_group_layouts: &[Some(&layout)],
                immediate_size: 0,
            });
            DepthResolve {
                pipeline: device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("zax depth resolve"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &module,
                        entry_point: Some("vs_main"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        buffers: &[],
                    },
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: super::renderer::DEPTH_FORMAT,
                        depth_write_enabled: Some(true),
                        // The pass writes `frag_depth` itself, so nothing is compared.
                        depth_compare: Some(wgpu::CompareFunction::Always),
                        stencil: wgpu::StencilState::default(),
                        bias: wgpu::DepthBiasState::default(),
                    }),
                    multisample: wgpu::MultisampleState::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: &module,
                        entry_point: Some("fs_main"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        targets: &[],
                    }),
                    multiview_mask: None,
                    cache: None,
                }),
                layout,
                group: None,
            }
        });

        if !matches!(&resolve.group, Some((built, _)) if *built == generation) {
            resolve.group = Some((
                generation,
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("zax depth resolve"),
                    layout: &resolve.layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(multisampled),
                    }],
                }),
            ));
        }
        let Some((_, group)) = &resolve.group else {
            return 0;
        };

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("zax depth resolve"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: single,
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
        pass.set_pipeline(&resolve.pipeline);
        pass.set_bind_group(0, group, &[]);
        pass.draw(0..3, 0..1);
        1
    }

    // ------------------------------------------------------- REQ-SCN-084, the reflections

    /// The floor mask of REQ-SCN-084 as a draw target, allocated if need be. The caller then opens
    /// a pass over it with the frame's **own depth** attached and depth writes off, clears it to 0
    /// and redraws the floor-eligible runs through the mask pipeline; `None` when there are no
    /// reflections this frame, and the caller draws nothing.
    ///
    /// The depth attachment is what makes a floor hidden behind something not be marked, and
    /// keeping the frame's own — rather than a copy — is what makes the mask land exactly where
    /// the frame drew. The view is handed back by value because the caller needs it while it also
    /// borrows this module to run the trace.
    pub fn mask_target(
        &mut self,
        device: &wgpu::Device,
        frame: ScreenFrame<'_>,
    ) -> Option<wgpu::TextureView> {
        let reflections = self.reflections.as_mut()?;
        if reflections.generation != frame.generation
            || !matches!(&reflections.mask, Some(mask) if mask.size == frame.size)
        {
            reflections.mask = Some(allocate(
                device,
                "zax floor mask",
                frame.size,
                OCCLUSION_FORMAT,
            ));
            reflections.generation = frame.generation;
            reflections.groups = None;
        }
        reflections.mask.as_ref().map(|mask| mask.view.clone())
    }

    /// Traces the reflected rays of REQ-SCN-084 into this module's own target, and says how many
    /// passes that took.
    pub fn trace(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        frame: ScreenFrame<'_>,
        style: &StyleSettings,
        scene_scale: f32,
    ) -> u32 {
        if self.reflections.is_none() {
            return 0;
        }
        let divisor = if style.ssr_half_resolution() { 2 } else { 1 };
        let size = scaled(frame.size, divisor);
        #[allow(clippy::cast_precision_loss)] // Sizes far below 2^24.
        let block = ScreenBlock {
            frame: frame.size,
            target: size,
            divisor,
            projection: frame.projection,
            ssr: [
                style.ssr_intensity(),
                style.ssr_max_length(scene_scale),
                // The longest ray any setting can ask for, which the march's positions are fitted
                // to.
                StyleSettings::ssr_longest(scene_scale),
                style.ssr_blur_fraction(),
            ],
            // (height / width, 1): turns a radius given as a fraction of the screen height into
            // one in viewport coordinates, which are normalised per axis.
            blur_aspect: [frame.size.1.max(1) as f32 / frame.size.0.max(1) as f32, 1.0],
            steps: style.ssr_steps(),
            refinements: style.ssr_refinements(),
            blur_taps: style.ssr_blur_taps(),
            ..ScreenBlock::default()
        };
        queue.write_buffer(&self.uniforms, Slot::Reflections.offset(), &block.bytes());
        queue.write_buffer(
            &self.uniforms,
            Slot::ReflectionsComposite.offset(),
            &ScreenBlock {
                target: frame.size,
                divisor: 1,
                ..block
            }
            .bytes(),
        );

        let (layout, sampler, dummy, uniforms) =
            (&self.layout, &self.sampler, &self.dummy, &self.uniforms);
        let Some(reflections) = &mut self.reflections else {
            return 0;
        };
        if reflections.divisor != divisor
            || !matches!(&reflections.traced, Some(target) if target.size == size)
        {
            reflections.traced = Some(allocate(device, "zax ssr", size, REFLECTION_FORMAT));
            reflections.divisor = divisor;
            reflections.groups = None;
        }
        if reflections.groups.is_none() {
            let (Some(mask), Some(traced)) = (&reflections.mask, &reflections.traced) else {
                return 0;
            };
            let group = |slot: Slot, sources: [&wgpu::TextureView; 3]| {
                screen_group(
                    device,
                    layout,
                    uniforms,
                    slot,
                    sampler,
                    frame.depth,
                    sources,
                )
            };
            let groups = [
                // The trace reads the frame's colour and the mask, both texel by texel.
                group(Slot::Reflections, [frame.color, &mask.view, dummy]),
                // The composite samples the trace back up to the frame's size through the filtered
                // sampler, and reads the mask texel by texel at that size.
                group(
                    Slot::ReflectionsComposite,
                    [&traced.view, &mask.view, dummy],
                ),
            ];
            reflections.groups = Some(groups);
        }
        let (Some(traced), Some(groups)) = (&reflections.traced, &reflections.groups) else {
            return 0;
        };

        // The trace writes every texel of its target, so there is nothing to clear.
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("zax ssr"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &traced.view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&reflections.trace);
        pass.set_bind_group(0, &groups[0], &[]);
        pass.draw(0..3, 0..1);
        1
    }

    /// Blends the traced reflection into `target` (REQ-SCN-084). One pass.
    pub fn composite_reflections(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
    ) -> u32 {
        let Some(reflections) = &self.reflections else {
            return 0;
        };
        let Some(groups) = &reflections.groups else {
            return 0;
        };

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("zax ssr composite"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&reflections.composite);
        pass.set_bind_group(0, &groups[1], &[]);
        pass.draw(0..3, 0..1);
        1
    }

    // ------------------------------------------------------- REQ-SCN-083, the occlusion

    /// Runs the occlusion of REQ-SCN-083 over `frame` into this module's own targets, and says how
    /// many passes that took.
    pub fn occlude(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        frame: ScreenFrame<'_>,
        style: &StyleSettings,
        scene_scale: f32,
    ) -> u32 {
        if self.occlusion.is_none() {
            return 0;
        }
        let divisor = if style.ao_half_resolution() { 2 } else { 1 };
        let radius = style.ao_radius_units(scene_scale);
        let size = scaled(frame.size, divisor);
        let block = ScreenBlock {
            frame: frame.size,
            target: size,
            divisor,
            projection: frame.projection,
            // A hundredth of the radius, and never under a fiftieth of a voxel: enough to keep
            // the flattest directions of the kernel from occluding their own surface, far too
            // little to hide a contact — a voxel is one scene unit.
            ao: [radius, style.ao_ceiling(), (0.01 * radius).max(0.02), 0.0],
            samples: style.ao_samples(),
            ..ScreenBlock::default()
        };
        queue.write_buffer(&self.uniforms, Slot::Occlusion.offset(), &block.bytes());
        // The composite runs at the frame's own resolution, so its target size is the frame's and
        // its step is one pixel. It needs a block of its own and not a second write of the same
        // one: `wgpu` applies every `write_buffer` of a submission before the command buffers.
        queue.write_buffer(
            &self.uniforms,
            Slot::OcclusionComposite.offset(),
            &ScreenBlock {
                target: frame.size,
                divisor: 1,
                ..block
            }
            .bytes(),
        );

        let (layout, sampler, dummy, uniforms) =
            (&self.layout, &self.sampler, &self.dummy, &self.uniforms);
        let Some(occlusion) = &mut self.occlusion else {
            return 0;
        };
        // One arm today; `Estimator::Msvo` joins it in step 4d.
        let Estimator::Depth(depth) = &mut occlusion.estimator;
        if depth.divisor != divisor
            || depth.generation != frame.generation
            || !matches!(&depth.raw, Some(target) if target.size == size)
        {
            depth.raw = Some(allocate(device, "zax occlusion", size, OCCLUSION_FORMAT));
            depth.blurred = Some(allocate(
                device,
                "zax occlusion blur",
                size,
                OCCLUSION_FORMAT,
            ));
            depth.divisor = divisor;
            depth.generation = frame.generation;
            depth.groups = None;
        }
        if depth.groups.is_none() {
            let (Some(raw), Some(blurred)) = (&depth.raw, &depth.blurred) else {
                return 0;
            };
            // The raw target is read texel by texel (`zax_load0`) and the blurred one through the
            // filtered sampler, which is what the OpenGL path's NEAREST and LINEAR are.
            let group = |slot: Slot, sources: [&wgpu::TextureView; 3]| {
                screen_group(
                    device,
                    layout,
                    uniforms,
                    slot,
                    sampler,
                    frame.depth,
                    sources,
                )
            };
            let groups = [
                group(Slot::Occlusion, [dummy, dummy, dummy]),
                group(Slot::Occlusion, [&raw.view, dummy, dummy]),
                group(Slot::OcclusionComposite, [&blurred.view, dummy, dummy]),
            ];
            depth.groups = Some(groups);
        }
        let (Some(raw), Some(blurred), Some(groups)) = (&depth.raw, &depth.blurred, &depth.groups)
        else {
            return 0;
        };

        // 1. The occlusion itself, from the depth alone. 2. The blur, which respects the depth
        // discontinuities (REQ-SCN-083). Each writes every texel of its target, so nothing is
        // cleared — the `Load` is what the OpenGL path's absence of a clear is.
        for (pipeline, target, group) in [
            (&depth.pass, &raw.view, &groups[0]),
            (&depth.blur, &blurred.view, &groups[1]),
        ] {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("zax occlusion"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
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
        2
    }

    /// Multiplies the colour of `target` by `1 - O` (REQ-SCN-083). One pass.
    pub fn composite_occlusion(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
    ) -> u32 {
        let Some(occlusion) = &self.occlusion else {
            return 0;
        };
        // Both estimators write `O` into an `R8` target, so the composite reads whichever one is
        // in force and the pipeline is the same.
        let Some(group) = (match &occlusion.estimator {
            Estimator::Depth(depth) => depth.groups.as_ref().map(|groups| &groups[2]),
        }) else {
            return 0;
        };

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("zax occlusion composite"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&occlusion.composite);
        pass.set_bind_group(0, group, &[]);
        pass.draw(0..3, 0..1);
        1
    }

    // ------------------------------------------------------------------- modules and pipelines

    /// The module of `body`, built the first time it is asked for. Keyed on the body because one
    /// body is one pass here: these modules have no variant axis, unlike the scene's.
    fn module(
        &mut self,
        device: &wgpu::Device,
        body: &'static str,
        chunks: &[&'static str],
    ) -> &wgpu::ShaderModule {
        self.modules.entry(body).or_insert_with(|| {
            let mut parts = chunks.to_vec();
            parts.push(body);
            let composed = preprocess::compose(&parts, &[]);
            super::renderer::shader_module(
                device,
                "zax screen",
                &composed.wgsl,
                composed.problem.as_deref(),
            )
        })
    }

    /// One full-screen pass of REQ-SCN-080: the triangle of `screen_depth.wgsl`, `body`'s
    /// fragment stage, and no depth attachment — every pass of this module reads the frame's
    /// depth as a texture and writes none.
    fn screen_pipeline(
        &mut self,
        device: &wgpu::Device,
        label: &'static str,
        body: &'static str,
        format: wgpu::TextureFormat,
        blend: Option<wgpu::BlendState>,
    ) -> wgpu::RenderPipeline {
        let module = self.module(device, body, &[SCREEN_DEPTH]).clone();
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(&self.pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_screen"),
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
                    blend,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        })
    }
}

/// The [`Screen`](shaders/screen_depth.wgsl) block, as the passes of REQ-SCN-080 set it.
#[derive(Debug, Clone, Copy, Default)]
struct ScreenBlock {
    /// The size in pixels the frame drew into, which `uTexel` is relative to.
    frame: (u32, u32),
    /// The size in pixels of the target this pass draws into.
    target: (u32, u32),
    /// 1, or 2 for a pass at half of each dimension (REQ-SCN-083, REQ-SCN-084).
    divisor: u32,
    projection: Vector4,
    /// `x` the radius, `y` the ceiling, `z` the bias.
    ao: [f32; 4],
    samples: i32,
    /// `x` the intensity, `y` the longest ray this setting asks for, `z` the longest any setting
    /// can ask for, `w` the blur as a fraction of the ray (REQ-SCN-084).
    ssr: [f32; 4],
    /// The aspect the blur spiral is stretched by.
    blur_aspect: [f32; 2],
    steps: i32,
    refinements: i32,
    blur_taps: i32,
}

impl ScreenBlock {
    /// The block's bytes, in the order `screen_depth.wgsl` declares them.
    #[allow(clippy::cast_precision_loss)] // Sizes far below 2^24.
    fn bytes(self) -> Vec<u8> {
        let frame = (self.frame.0.max(1) as f32, self.frame.1.max(1) as f32);
        let target = (self.target.0.max(1) as f32, self.target.1.max(1) as f32);
        let divisor = self.divisor.max(1) as f32;
        let mut data = Vec::with_capacity(SCREEN_BYTES);
        // The attachments are allocated at the frame's exact size, so the drawn scale is (1, 1).
        push(&mut data, [1.0, 1.0, target.0, target.1]);
        // `uTexel`: one texel of this pass's target in **viewport** coordinates, which is
        // `divisor / drawn` on the OpenGL side and the same quotient here.
        push(&mut data, [divisor / frame.0, divisor / frame.1, 0.0, 0.0]);
        push(
            &mut data,
            [
                self.projection.x,
                self.projection.y,
                self.projection.z,
                self.projection.w,
            ],
        );
        push(&mut data, self.ao);
        push(&mut data, self.ssr);
        push(
            &mut data,
            [self.blur_aspect[0], self.blur_aspect[1], 0.0, 0.0],
        );
        for value in [self.samples, self.steps, self.refinements, self.blur_taps] {
            data.extend_from_slice(&value.to_ne_bytes());
        }
        data
    }
}

fn push(data: &mut Vec<u8>, values: [f32; 4]) {
    for value in values {
        data.extend_from_slice(&value.to_ne_bytes());
    }
}

/// `size` divided by `divisor`, never under one texel.
fn scaled(size: (u32, u32), divisor: u32) -> (u32, u32) {
    ((size.0 / divisor).max(1), (size.1 / divisor).max(1))
}

fn bytes(target: Option<&Target>, per_texel: u64) -> u64 {
    target.map_or(0, |target| {
        u64::from(target.size.0) * u64::from(target.size.1) * per_texel
    })
}

/// One colour target of `size` and its view.
fn allocate(
    device: &wgpu::Device,
    label: &'static str,
    size: (u32, u32),
    format: wgpu::TextureFormat,
) -> Target {
    let view = device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default());
    Target { view, size }
}

/// The bind group of one screen pass: its block of the uniforms, the frame's depth, the shared
/// sampler and the three source slots — a dummy in each the pass does not read.
fn screen_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    uniforms: &wgpu::Buffer,
    slot: Slot,
    sampler: &wgpu::Sampler,
    depth: &wgpu::TextureView,
    sources: [&wgpu::TextureView; 3],
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("zax screen"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: uniforms,
                    offset: slot.offset(),
                    size: wgpu::BufferSize::new(SCREEN_BYTES as u64),
                }),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(depth),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(sampler),
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
}

// --------------------------------------------------------------------- shaders

/// The chunk every screen pass shares: the block, the bindings, the full-screen triangle and the
/// depth reconstruction (`shaders/screen_depth.glsl` on the other side).
const SCREEN_DEPTH: &str = include_str!("shaders/screen_depth.wgsl");
const AO: &str = include_str!("shaders/ao.wgsl");
const AO_BLUR: &str = include_str!("shaders/ao_blur.wgsl");
const AO_COMPOSITE: &str = include_str!("shaders/ao_composite.wgsl");
const SSR: &str = include_str!("shaders/ssr.wgsl");
const SSR_COMPOSITE: &str = include_str!("shaders/ssr_composite.wgsl");
const DEPTH_RESOLVE: &str = include_str!("shaders/depth_resolve.wgsl");

#[cfg(test)]
mod tests {
    use super::{ScreenBlock, SCREEN_BYTES};
    use crate::numerics::Vector4;

    /// The block the WGSL declares is seven `vec4`, and what is written has to be exactly that
    /// long or `min_binding_size` refuses the bind group — a mismatch that no render would reach.
    #[test]
    fn the_screen_block_is_as_long_as_the_wgsl_declares() {
        let block = ScreenBlock {
            frame: (640, 480),
            target: (640, 480),
            divisor: 1,
            projection: Vector4::new(1.0, 2.0, 3.0, 4.0),
            ao: [0.5, 0.25, 0.01, 0.0],
            samples: 10,
            ssr: [0.3, 90.0, 360.0, 0.05],
            blur_aspect: [0.75, 1.0],
            steps: 24,
            refinements: 5,
            blur_taps: 8,
        };
        assert_eq!(block.bytes().len(), SCREEN_BYTES);
    }

    /// `uTexel` is one texel of the target **in viewport coordinates**: at half resolution a step
    /// of one texel is two pixels of the frame, which is what the OpenGL `texel` helper computes
    /// as `divisor / drawn`.
    #[test]
    fn one_texel_of_a_half_resolution_target_is_two_pixels_of_the_frame() {
        let block = ScreenBlock {
            frame: (640, 480),
            target: (320, 240),
            divisor: 2,
            ..ScreenBlock::default()
        };
        let bytes = block.bytes();
        let texel = |at: usize| f32::from_ne_bytes(bytes[at..at + 4].try_into().unwrap());
        assert!((texel(16) - 2.0 / 640.0).abs() < 1e-9);
        assert!((texel(20) - 2.0 / 480.0).abs() < 1e-9);
    }
}
