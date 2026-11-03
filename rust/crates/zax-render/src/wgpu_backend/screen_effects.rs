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

use super::msvo::{Bindings, Msvo};
use super::preprocess;

/// The format of the occlusion targets and of the floor mask: one channel of eight bits.
pub(super) const OCCLUSION_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R8Unorm;

/// Bytes one texel of an occlusion target or of the floor mask occupies.
pub(super) const OCCLUSION_BYTES_PER_TEXEL: u64 = 1;

/// The format of the reflections' trace target: a linear colour, as the linear buffer's own
/// (REQ-SCN-084).
const REFLECTION_FORMAT: wgpu::TextureFormat = super::renderer::LINEAR_FORMAT;

/// Bytes one texel of it occupies: four half-float channels.
const REFLECTION_BYTES_PER_TEXEL: u64 = 8;

/// Bytes of the [`Screen`](shaders/screen_depth.wgsl) block: twelve `vec4`.
pub(super) const SCREEN_BYTES: usize = 12 * 16;

/// How far apart two blocks of [`ScreenEffects::uniforms`] sit. Each pass group writes its own,
/// and they have to be separate because `wgpu` applies every `write_buffer` of a submission
/// **before** the command buffers run: one block written twice would hand both passes the second
/// value.
const SCREEN_SLOT: u64 = 256;

/// Which block of [`ScreenEffects::uniforms`] a pass group reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Slot {
    /// The occlusion of REQ-SCN-083 and its blur, at the estimator's own resolution.
    Occlusion,
    /// The composite of REQ-SCN-083, at the frame's resolution.
    OcclusionComposite,
    /// The trace of REQ-SCN-084, at the quality's resolution.
    Reflections,
    /// The composite of REQ-SCN-084, at the frame's resolution.
    ReflectionsComposite,
    /// One of the nineteen passes of the multi-scale chain of REQ-SCN-083's *Mode MSVO*, which
    /// needs a block per level and per axis: [`Slot::occlude`], [`Slot::blur`] and
    /// [`Slot::upsample`] name them, and index 0 is the pair of depth-pyramid passes, which read
    /// only the finest level's numbers (the decimation reads no field at all).
    Msvo(u32),
}

impl Slot {
    /// The blocks that are not the multi-scale chain's.
    const FIXED: u64 = 4;

    /// The chain's own: 1 for the pyramid, 4 for the occlusion of each level, 8 for the two axes
    /// of its blur, and 3 for the upsample chain — `style::MSVO_LEVELS` being 4.
    const MSVO: u64 = 16;

    const COUNT: u64 = Self::FIXED + Self::MSVO;

    /// The occlusion of level `level` of the multi-scale chain.
    #[allow(clippy::cast_possible_truncation)] // `style::MSVO_LEVELS` is 4.
    pub(super) const fn occlude(level: usize) -> Self {
        Self::Msvo(1 + level as u32)
    }

    /// One axis of the blur of level `level`: 0 horizontally, 1 vertically.
    #[allow(clippy::cast_possible_truncation)] // `style::MSVO_LEVELS` is 4.
    pub(super) const fn blur(level: usize, axis: u32) -> Self {
        Self::Msvo(5 + 2 * level as u32 + axis)
    }

    /// The upsample step that folds everything coarser into level `level`.
    #[allow(clippy::cast_possible_truncation)] // `style::MSVO_LEVELS` is 4.
    pub(super) const fn upsample(level: usize) -> Self {
        Self::Msvo(13 + level as u32)
    }

    pub(super) const fn offset(self) -> u64 {
        SCREEN_SLOT
            * match self {
                Self::Occlusion => 0,
                Self::OcclusionComposite => 1,
                Self::Reflections => 2,
                Self::ReflectionsComposite => 3,
                Self::Msvo(index) => Self::FIXED + index as u64,
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
pub(super) struct Target {
    pub(super) view: wgpu::TextureView,
    pub(super) size: (u32, u32),
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

/// Which estimator of REQ-SCN-083 is in force. **One or the other is built, never both**: a player
/// who chooses `spl.ao` = 1 builds the five pipelines of the multi-scale port and allocates its
/// pyramid, and the two pipelines and two targets of the hemisphere estimator stay out of
/// existence — the same rule that keeps the occlusion and the reflections independent of each
/// other, applied one level down.
#[derive(Debug)]
enum Estimator {
    /// `spl.ao` = 2, and `spl.ao` = 1 once the multi-scale estimator has been refused.
    Depth(Depth),
    /// `spl.ao` = 1 with the port available (REQ-SCN-083 *Mode MSVO*).
    Msvo(Msvo),
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
    /// Whether this implementation refused the multi-scale estimator of REQ-SCN-083's *Mode MSVO*,
    /// so that `spl.ao` = 1 renders as `spl.ao` = 2 from now on — the declared fallback **(D)**.
    ///
    /// Sticky for the life of the module, and for the reason RU-5 gave the whole module's own
    /// fallback: the one format the mode needs that nothing else here needs is the `R16Float`
    /// pyramid, so a refusal is a property of the implementation and not of the frame. It carries
    /// the reason, so that the renderer can write the one diagnostic line of the fallback.
    msvo_refused: Option<String>,
    /// The group the shared composite reads the multi-scale estimator's `O` through, and the frame
    /// allocation it was built against.
    msvo_composite: Option<wgpu::BindGroup>,
    msvo_composite_generation: u64,
}

impl ScreenEffects {
    /// Creates the module with neither effect in it: the layout every screen pass takes, the
    /// sampler, the dummy source and the uniform blocks. [`Self::select`] then builds the halves
    /// the frame asks for.
    /// `multi_scale` says whether the adapter renders into the `R16Float` pyramid of REQ-SCN-083's
    /// *Mode MSVO*; `false` makes that mode fall back on Depth from the first frame, with one
    /// diagnostic line (the declared fallback **(D)**).
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, multi_scale: bool) -> Self {
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
                source(6),
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
            msvo_composite: None,
            msvo_composite_generation: u64::MAX,
            msvo_refused: (!multi_scale).then(|| {
                format!(
                    "this adapter does not render into {:?}",
                    super::msvo::PYRAMID_FORMAT
                )
            }),
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
            // A change of `spl.ao` between its two estimators releases the one that is leaving,
            // targets and all, and builds the one that arrives — which is also how the mode falls
            // back on Depth when the multi-scale port is refused ([`Self::refuse_msvo`]).
            let wanted_msvo = style.msvo_active() && self.msvo_refused.is_none();
            if self.occlusion.is_some() && self.occludes_multi_scale() != wanted_msvo {
                self.occlusion = None;
            }
            if self.occlusion.is_none() && wanted_msvo {
                let estimator =
                    Estimator::Msvo(Msvo::new(device, &self.pipeline_layout, &mut self.modules));
                let composite = self.occlusion_composite(device);
                self.occlusion = Some(Occlusion {
                    composite,
                    estimator,
                });
            }
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
                let composite = self.occlusion_composite(device);
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

    /// The composite the two estimators share (REQ-SCN-083): `dst x src`, so the full-screen
    /// triangle scales the linear buffer in place — no second colour target and no copy of the
    /// scene. The OpenGL path's `glBlendFunc(GL_ZERO, GL_SRC_COLOR)`, spelled for this API.
    fn occlusion_composite(&mut self, device: &wgpu::Device) -> wgpu::RenderPipeline {
        let multiply = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::Zero,
            dst_factor: wgpu::BlendFactor::Src,
            operation: wgpu::BlendOperation::Add,
        };
        self.screen_pipeline(
            device,
            "zax ao composite",
            AO_COMPOSITE,
            super::renderer::LINEAR_FORMAT,
            Some(wgpu::BlendState {
                color: multiply,
                alpha: multiply,
            }),
        )
    }

    /// Whether the occlusion of REQ-SCN-083 is in this module.
    pub const fn occludes(&self) -> bool {
        self.occlusion.is_some()
    }

    /// Whether the occlusion in this module is the **multi-scale** estimator of REQ-SCN-083's
    /// *Mode MSVO*, rather than the hemisphere one.
    pub const fn occludes_multi_scale(&self) -> bool {
        matches!(
            &self.occlusion,
            Some(occlusion) if matches!(occlusion.estimator, Estimator::Msvo(_))
        )
    }

    /// Levels the multi-scale pyramid holds, 0 when the estimator is the hemisphere one or has not
    /// run yet. Read by the tests of REQ-SCN-083's *Mode MSVO*, which have no other way to tell
    /// one estimator's resources from the other's.
    pub fn multi_scale_levels(&self) -> usize {
        match &self.occlusion {
            Some(Occlusion {
                estimator: Estimator::Msvo(msvo),
                ..
            }) => msvo.levels(),
            _ => 0,
        }
    }

    /// Why the multi-scale estimator of REQ-SCN-083 was refused by this implementation, when it
    /// was, so that `spl.ao` = 1 renders as Depth — the fallback **(D)**. `None` on every machine
    /// whose adapter answers normally, which is every desktop one.
    pub fn msvo_refused(&self) -> Option<&str> {
        self.msvo_refused.as_deref()
    }

    /// Makes the frames that follow refuse the multi-scale estimator, so that the fallback above
    /// can be observed on a machine that refuses nothing.
    ///
    /// Only a test calls this — the same kind of seam the OpenGL module carries, and for the same
    /// reason: `R16Float` is colour-renderable on every adapter WebGPU admits, and an unreachable
    /// fallback is an untested one.
    pub fn refuse_msvo(&mut self, reason: &str) {
        self.msvo_refused = Some(reason.to_owned());
        self.occlusion = None;
        self.msvo_composite = None;
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
                Estimator::Msvo(_) => Msvo::PIPELINES,
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
                Estimator::Msvo(msvo) => msvo.bytes(),
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
        // **Without** the shared chunk, unlike every other pass of this module: that chunk
        // declares group 0 as the uniform block, the frame's depth, the sampler and the four
        // sources, and this pass's layout is one multisampled depth texture at binding 0. Its own
        // triangle is three lines, so there is nothing to share.
        //
        // Cloned so that the map it lives in is no longer borrowed when `depth_resolve` is: both
        // are fields of `self` and both are reached through a method.
        let module = self
            .modules
            .entry(DEPTH_RESOLVE)
            .or_insert_with(|| {
                super::renderer::shader_module(device, "zax depth resolve", DEPTH_RESOLVE, None)
            })
            .clone();
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
            let group = |slot: Slot, sources: [&wgpu::TextureView; 4]| {
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
                group(Slot::Reflections, [frame.color, &mask.view, dummy, dummy]),
                // The composite samples the trace back up to the frame's size through the filtered
                // sampler, and reads the mask texel by texel at that size.
                group(
                    Slot::ReflectionsComposite,
                    [&traced.view, &mask.view, dummy, dummy],
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
    // The two estimators' preambles and the hemisphere one's own two passes, which is the shape
    // the OpenGL twin has for the same reason: both estimators have to write the shared
    // composite's block, and splitting on that would move the list rather than shorten it.
    #[allow(clippy::too_many_lines)]
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
        if self.occludes_multi_scale() {
            // The shared composite's own block, which the chain does not touch: it reads the drawn
            // scale and the frame's size, and a slot left at zero would make it sample texel
            // (0, 0) everywhere.
            queue.write_buffer(
                &self.uniforms,
                Slot::OcclusionComposite.offset(),
                &ScreenBlock {
                    frame: frame.size,
                    target: frame.size,
                    divisor: 1,
                    projection: frame.projection,
                    ..ScreenBlock::default()
                }
                .bytes(),
            );
            let bindings = Bindings {
                layout: &self.layout,
                uniforms: &self.uniforms,
                sampler: &self.sampler,
                dummy: &self.dummy,
            };
            let Some(Occlusion {
                estimator: Estimator::Msvo(msvo),
                ..
            }) = &mut self.occlusion
            else {
                return 0;
            };
            return msvo.draw(device, queue, encoder, bindings, frame, style, scene_scale);
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
        let Estimator::Depth(depth) = &mut occlusion.estimator else {
            return 0;
        };
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
            let group = |slot: Slot, sources: [&wgpu::TextureView; 4]| {
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
                group(Slot::Occlusion, [dummy, dummy, dummy, dummy]),
                group(Slot::Occlusion, [&raw.view, dummy, dummy, dummy]),
                group(
                    Slot::OcclusionComposite,
                    [&blurred.view, dummy, dummy, dummy],
                ),
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
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        frame: ScreenFrame<'_>,
        target: &wgpu::TextureView,
    ) -> u32 {
        // The multi-scale estimator's result is a level of its own pyramid, so the group that
        // reads it is rebuilt whenever that pyramid is — which the generation says.
        if let Some(Occlusion {
            estimator: Estimator::Msvo(msvo),
            ..
        }) = &self.occlusion
        {
            if self.msvo_composite_generation != frame.generation || self.msvo_composite.is_none() {
                let result = msvo.result().cloned();
                self.msvo_composite = result.map(|view| {
                    screen_group(
                        device,
                        &self.layout,
                        &self.uniforms,
                        Slot::OcclusionComposite,
                        &self.sampler,
                        frame.depth,
                        [&view, &self.dummy, &self.dummy, &self.dummy],
                    )
                });
                self.msvo_composite_generation = frame.generation;
            }
        }
        let Some(occlusion) = &self.occlusion else {
            return 0;
        };
        // Both estimators write `O` into an `R8` target, so the composite reads whichever one is
        // in force and the pipeline is the same. The hemisphere one keeps its group with its
        // targets; the multi-scale one hands back the view of its finest combination, and the
        // group that names it is built here and kept beside it.
        let Some(group) = (match &occlusion.estimator {
            Estimator::Depth(depth) => depth.groups.as_ref().map(|groups| &groups[2]),
            Estimator::Msvo(_) => self.msvo_composite.as_ref(),
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

    /// The module of `body`, built the first time it is asked for.
    fn module(&mut self, device: &wgpu::Device, body: &'static str) -> wgpu::ShaderModule {
        screen_module(device, &mut self.modules, body)
    }

    /// One full-screen pass of REQ-SCN-080 through this module's layout.
    fn screen_pipeline(
        &mut self,
        device: &wgpu::Device,
        label: &'static str,
        body: &'static str,
        format: wgpu::TextureFormat,
        blend: Option<wgpu::BlendState>,
    ) -> wgpu::RenderPipeline {
        let module = self.module(device, body);
        screen_pipeline(device, &self.pipeline_layout, &module, label, format, blend)
    }
}

/// The module of `body`, built the first time it is asked for and handed back by value. Keyed on
/// the body because one body is one pass here: these modules have no variant axis, unlike the
/// scene's.
///
/// `pub(super)` for [`super::msvo`], whose five pipelines are built the same way.
pub(super) fn screen_module(
    device: &wgpu::Device,
    modules: &mut FastMap<&'static str, wgpu::ShaderModule>,
    body: &'static str,
) -> wgpu::ShaderModule {
    modules
        .entry(body)
        .or_insert_with(|| {
            let composed = preprocess::compose(&[SCREEN_DEPTH, body], &[]);
            super::renderer::shader_module(
                device,
                "zax screen",
                &composed.wgsl,
                composed.problem.as_deref(),
            )
        })
        .clone()
}

/// One full-screen pass of REQ-SCN-080: the triangle of `screen_depth.wgsl`, `body`'s fragment
/// stage, and no depth attachment — every pass of this module reads the frame's depth as a
/// texture and writes none, so every one of them is single-sample whatever the frame is.
pub(super) fn screen_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    module: &wgpu::ShaderModule,
    label: &'static str,
    format: wgpu::TextureFormat,
    blend: Option<wgpu::BlendState>,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module,
            entry_point: Some("vs_screen"),
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
                blend,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

/// The [`Screen`](shaders/screen_depth.wgsl) block, as the passes of REQ-SCN-080 set it.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct ScreenBlock {
    /// The size in pixels the frame drew into, which `uTexel` is relative to.
    pub(super) frame: (u32, u32),
    /// The size in pixels of the target this pass draws into.
    pub(super) target: (u32, u32),
    /// 1, or 2 for a pass at half of each dimension, or `divisor << level` for a level of the
    /// multi-scale pyramid (REQ-SCN-083, REQ-SCN-084).
    pub(super) divisor: u32,
    pub(super) projection: Vector4,
    /// `x` the radius, `y` the ceiling, `z` the bias.
    pub(super) ao: [f32; 4],
    pub(super) samples: i32,
    /// `x` the intensity, `y` the longest ray this setting asks for, `z` the longest any setting
    /// can ask for, `w` the blur as a fraction of the ray (REQ-SCN-084).
    pub(super) ssr: [f32; 4],
    /// The aspect the blur spiral is stretched by.
    pub(super) blur_aspect: [f32; 2],
    pub(super) steps: i32,
    pub(super) refinements: i32,
    pub(super) blur_taps: i32,
    /// REQ-SCN-083 *Mode MSVO*: the level's radius, its focal length, the rejection falloff and
    /// the weight normalisation.
    pub(super) msvo_a: [f32; 4],
    /// The headroom, the blur's sharpness, the share and the ceiling.
    pub(super) msvo_b: [f32; 4],
    /// One texel of this level along the axis the blur runs on.
    pub(super) msvo_step: [f32; 2],
    /// One texel of the coarse level the upsample gathers from.
    pub(super) msvo_coarse_texel: [f32; 2],
    /// The distance a pixel with no geometry carries.
    pub(super) msvo_far: f32,
    /// How many entries of the pattern this quality takes.
    pub(super) msvo_pattern: i32,
    /// Whether this is the last upsample step, the one that writes `O`.
    pub(super) msvo_final: i32,
}

impl ScreenBlock {
    /// `uTexel`: one texel of this pass's target in **viewport** coordinates, which is
    /// `divisor / drawn` on the OpenGL side and the same quotient here.
    #[allow(clippy::cast_precision_loss)] // Sizes far below 2^24.
    pub(super) fn texel(self) -> (f32, f32) {
        let frame = (self.frame.0.max(1) as f32, self.frame.1.max(1) as f32);
        let divisor = self.divisor.max(1) as f32;
        (divisor / frame.0, divisor / frame.1)
    }

    /// The block's bytes, in the order `screen_depth.wgsl` declares them.
    #[allow(clippy::cast_precision_loss)] // Sizes far below 2^24.
    pub(super) fn bytes(self) -> Vec<u8> {
        let target = (self.target.0.max(1) as f32, self.target.1.max(1) as f32);
        let mut data = Vec::with_capacity(SCREEN_BYTES);
        // The attachments are allocated at the frame's exact size, so the drawn scale is (1, 1).
        push(&mut data, [1.0, 1.0, target.0, target.1]);
        let texel = self.texel();
        push(&mut data, [texel.0, texel.1, 0.0, 0.0]);
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
        push(&mut data, self.msvo_a);
        push(&mut data, self.msvo_b);
        push(
            &mut data,
            [
                self.msvo_step[0],
                self.msvo_step[1],
                self.msvo_coarse_texel[0],
                self.msvo_coarse_texel[1],
            ],
        );
        push(&mut data, [self.msvo_far, 0.0, 0.0, 0.0]);
        for value in [self.msvo_pattern, self.msvo_final, 0, 0] {
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
pub(super) fn allocate(
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
pub(super) fn screen_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    uniforms: &wgpu::Buffer,
    slot: Slot,
    sampler: &wgpu::Sampler,
    depth: &wgpu::TextureView,
    sources: [&wgpu::TextureView; 4],
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
            wgpu::BindGroupEntry {
                binding: 6,
                resource: wgpu::BindingResource::TextureView(sources[3]),
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
            ..ScreenBlock::default()
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
