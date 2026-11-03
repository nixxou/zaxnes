// Portions adapted from Microsoft MiniEngine (c) Microsoft, MIT License
//
// Origin: the public repository `microsoft/DirectX-Graphics-Samples`, MiniEngine, files
// `Core/SSAO.cpp` and `Core/Shaders/Ao*.hlsl(i)`, author James Stanard. A copy of those files and
// of their MIT licence is kept under `third_party/MiniEngine-SSAO` (see `third_party/README.md`);
// the licence is carried with the distribution in `THIRD-PARTY-NOTICES.md`. MIT is compatible with
// the GPL-3.0 this reimplementation is distributed under.

//! The **MSVO** mode of AMD-055 REQ-SCN-083, `spl.ao` = 1, on the `wgpu` backend: the twin of
//! [`crate::gl::msvo`], which is itself the port of the public MIT-licensed effect of Microsoft's
//! `MiniEngine` to fragment passes.
//!
//! Everything about the estimator is that module's: the pyramid of **linear** depths, the sphere
//! penetration test per level with the original's checkerboard of offsets, the `SmartBlur` over
//! every level and the bilateral upsample that carries each level into the next finer one and
//! multiplies the two. The four departures the .NET lot V2-25 found — the missing `SmartBlur`,
//! three levels instead of four, a coarsest sphere of `r` instead of `1,75 r`, and a pattern fixed
//! per pixel — were corrected on the OpenGL side from the start and are correct here by
//! construction, since this is a transcription of that chain (see `Q-100`).
//!
//! **What this side adds is nothing of the algorithm and two things of the API**, both recorded in
//! the WGSL: `msvo_downsample.wgsl` has to mirror which texel of each 2 × 2 block it keeps, and
//! `msvo_occlude.wgsl` has to mirror the pixel coordinate its per-pixel rotation hashes — because
//! both read a row index, and the rows run the other way round here.
//!
//! **The lifecycle is `screen_effects`'s**, not this module's: this estimator is built the first
//! frame `spl.ao` = 1 asks for it and dropped as soon as no frame does, and the Depth estimator's
//! own pipelines and targets do not exist while it lives. What this module owns is the pyramid, its
//! per-level targets and the bind groups that name them.

use zax_engine3d::hashing::FastMap;

use crate::geometry;
use crate::style::{self, StyleSettings};

use super::screen_effects::{
    allocate, screen_group, screen_module, screen_pipeline, ScreenBlock, ScreenFrame, Slot, Target,
    OCCLUSION_BYTES_PER_TEXEL, OCCLUSION_FORMAT,
};

/// One level of the depth pyramid: a **view distance in scene units**, which eight bits would
/// quantise to uselessness.
///
/// `R16Float` is the one format of this lot that is not already a dependency of the renderer, and
/// it is the one REQ-SCN-083's fallback **(D)** exists for: a device that refuses it as a render
/// attachment gets the Depth estimator and one diagnostic line. Every desktop adapter grants it —
/// WebGPU requires `r16float` to be colour-renderable — so the fallback is reached here only
/// through the test seam `ScreenEffects::refuse_msvo`, exactly as on the OpenGL side.
pub(super) const PYRAMID_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R16Float;

/// Bytes one texel of a pyramid level occupies: one half-float channel.
const DEPTH_BYTES_PER_TEXEL: u64 = 2;

/// A hair under the far plane, the distance at which a pixel holds no geometry: the test
/// `msvo_upsample.wgsl` keeps the skybox and the cleared background out of the effect with
/// (REQ-SCN-083's scope bullet).
const EMPTY_DISTANCE: f32 = geometry::FAR_PLANE * 0.999;

/// One level of the pyramid: the linear depth, the level's own occlusion, the horizontal half of
/// the smoothing of that occlusion, and the combination of it with everything coarser.
///
/// The **coarsest** level needs no combination target: it is combined *into* the level below it,
/// which is the shape of the original's `BLEND_WITH_HIGHER_RESOLUTION`.
#[derive(Debug)]
struct Level {
    depth: Target,
    occlusion: Target,
    smoothed: Target,
    combined: Option<Target>,
    size: (u32, u32),
}

/// The bind group of every pass of the chain, by the order [`Msvo::draw`] runs them in. Built with
/// the pyramid, because that is what decides the textures they name.
#[derive(Debug)]
struct Groups {
    /// The frame's depth into level 0, then each coarser level from the one above it.
    pyramid: Vec<wgpu::BindGroup>,
    /// One per level.
    occlude: Vec<wgpu::BindGroup>,
    /// Two per level: horizontally into `smoothed`, then back vertically into `occlusion`.
    blur: Vec<(wgpu::BindGroup, wgpu::BindGroup)>,
    /// One per level but the coarsest, from the coarsest pair down to the finest.
    upsample: Vec<wgpu::BindGroup>,
}

/// The multi-scale occlusion: the five pipelines of the chain and the pyramid they work on.
#[derive(Debug)]
pub(super) struct Msvo {
    linearize: wgpu::RenderPipeline,
    downsample: wgpu::RenderPipeline,
    occlude: wgpu::RenderPipeline,
    blur: wgpu::RenderPipeline,
    upsample: wgpu::RenderPipeline,
    levels: Vec<Level>,
    groups: Option<Groups>,
    /// 1 at `ao.quality` High, 2 otherwise — the divisor the pyramid was built for.
    divisor: u32,
    /// The frame allocation the pyramid and its groups were built against.
    generation: u64,
}

impl Msvo {
    /// How many pipelines this estimator holds, for `RenderStatistics::style_programs`. The shared
    /// composite is counted by the caller, which owns it.
    pub(super) const PIPELINES: u32 = 5;

    /// Builds the five pipelines; the pyramid is allocated on the first run.
    pub(super) fn new(
        device: &wgpu::Device,
        layout: &wgpu::PipelineLayout,
        modules: &mut FastMap<&'static str, wgpu::ShaderModule>,
    ) -> Self {
        let mut pipeline = |label, body, format| {
            let module = screen_module(device, modules, body);
            screen_pipeline(device, layout, &module, label, format, None)
        };
        Self {
            linearize: pipeline("zax msvo linearize", LINEARIZE, PYRAMID_FORMAT),
            downsample: pipeline("zax msvo downsample", DOWNSAMPLE, PYRAMID_FORMAT),
            occlude: pipeline("zax msvo occlude", OCCLUDE, OCCLUSION_FORMAT),
            blur: pipeline("zax msvo blur", BLUR, OCCLUSION_FORMAT),
            upsample: pipeline("zax msvo upsample", UPSAMPLE, OCCLUSION_FORMAT),
            levels: Vec::new(),
            groups: None,
            divisor: 0,
            generation: u64::MAX,
        }
    }

    /// Bytes the pyramid occupies, for `RenderStatistics::screen_effect_bytes`; 0 until the first
    /// run.
    pub(super) fn bytes(&self) -> u64 {
        self.levels
            .iter()
            .map(|level| {
                let texels = u64::from(level.size.0) * u64::from(level.size.1);
                let occlusions = 2 + u64::from(level.combined.is_some());
                texels * (DEPTH_BYTES_PER_TEXEL + occlusions * OCCLUSION_BYTES_PER_TEXEL)
            })
            .sum()
    }

    /// Levels the pyramid holds; 0 until the first run.
    pub(super) fn levels(&self) -> usize {
        self.levels.len()
    }

    /// The bind group the shared composite of REQ-SCN-083 reads `O` through: the finest level's
    /// combination. `None` until the first run.
    pub(super) fn result(&self) -> Option<&wgpu::TextureView> {
        self.levels
            .first()
            .and_then(|level| level.combined.as_ref().map(|target| &target.view))
    }

    /// Runs the whole chain over `frame` and says how many passes that took, leaving `O` in
    /// [`Self::result`] for the caller to composite.
    #[allow(clippy::too_many_arguments)] // The frame's inputs, one parameter each.
    pub(super) fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        bindings: Bindings<'_>,
        frame: ScreenFrame<'_>,
        style: &StyleSettings,
        scene_scale: f32,
    ) -> u32 {
        let divisor = if style.ao_half_resolution() { 2 } else { 1 };
        self.allocate(device, bindings, frame, divisor);
        if self.levels.len() < 2 {
            return 0;
        }
        self.upload(
            queue,
            bindings.uniforms,
            frame,
            style,
            (divisor, scene_scale),
        );

        let Some(groups) = &self.groups else {
            return 0;
        };
        let levels = self.levels.len();

        // 1. The pyramid: the frame's own depth linearised into the finest level, then each
        //    coarser level decimated from the one above it.
        for level in 0..levels {
            let pipeline = if level == 0 {
                &self.linearize
            } else {
                &self.downsample
            };
            pass(encoder, "zax msvo pyramid", &self.levels[level].depth.view)
                .draw_with(pipeline, &groups.pyramid[level]);
        }

        // 2. The occlusion of each level, each on its own share of the radius.
        for level in 0..levels {
            pass(
                encoder,
                "zax msvo occlude",
                &self.levels[level].occlusion.view,
            )
            .draw_with(&self.occlude, &groups.occlude[level]);
        }

        // 3. The `SmartBlur`, two separable passes per level: horizontally into the level's
        //    smoothing target and back vertically into its own occlusion target, so that step 4
        //    reads a smoothed level whichever end of the chain it is at.
        for level in 0..levels {
            let (horizontal, vertical) = &groups.blur[level];
            pass(encoder, "zax msvo blur", &self.levels[level].smoothed.view)
                .draw_with(&self.blur, horizontal);
            pass(encoder, "zax msvo blur", &self.levels[level].occlusion.view)
                .draw_with(&self.blur, vertical);
        }

        // 4. The bilateral upsample chain, from the coarsest level to the finest; the last step
        //    writes the `O` the composite reads.
        for level in (0..levels - 1).rev() {
            let Some(target) = &self.levels[level].combined else {
                continue;
            };
            pass(encoder, "zax msvo upsample", &target.view)
                .draw_with(&self.upsample, &groups.upsample[level]);
        }

        count(levels + levels + 2 * levels + (levels - 1))
    }

    /// The block of each pass of the chain. Sixteen writes, because `wgpu` applies every
    /// `write_buffer` of a submission **before** the command buffers run: a pass whose numbers are
    /// its level's cannot share a block with a pass whose numbers are another's.
    fn upload(
        &self,
        queue: &wgpu::Queue,
        uniforms: &wgpu::Buffer,
        frame: ScreenFrame<'_>,
        style: &StyleSettings,
        (divisor, scene_scale): (u32, f32),
    ) {
        let levels = self.levels.len();
        let base = ScreenBlock {
            frame: frame.size,
            projection: frame.projection,
            divisor,
            msvo_b: [
                style::MSVO_ACCESS_HEADROOM,
                style::MSVO_BLUR_SHARPNESS,
                style::MSVO_SHARE,
                style.ao_ceiling(),
            ],
            msvo_far: EMPTY_DISTANCE,
            ..ScreenBlock::default()
        };
        // The attachments of a level are allocated at exactly that level's size, so the drawn
        // scale is (1, 1) throughout — which is why `uScale` of the GLSL has no counterpart here
        // beyond the scale every block already carries. `uTexel` is one texel of the level, which
        // `ScreenBlock` computes from `target` and `divisor`: a level's step is
        // `divisor << level` pixels of the frame.
        let level_block = |level: usize| ScreenBlock {
            target: self.levels[level].size,
            divisor: divisor << u32::try_from(level).unwrap_or(0),
            ..base
        };

        // The linearize pass, which the downsample groups share: they read no field of the block.
        queue.write_buffer(uniforms, Slot::Msvo(0).offset(), &level_block(0).bytes());

        let falloff = style.msvo_falloff();
        let entries = style.msvo_pattern_entries();
        let weight_scale = style.msvo_weight_scale();
        for level in 0..levels {
            #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
            let radius = style.msvo_level_radius(scene_scale, level as i32);
            // The focal length of this level in pixels: half its height over `tan(fov / 2)`,
            // which the projection of REQ-SCN-004 carries in `m22`.
            #[allow(clippy::cast_precision_loss)] // Sizes far below 2^24.
            let focal = 0.5 * self.levels[level].size.1.max(1) as f32 * frame.projection.y;
            let block = ScreenBlock {
                msvo_a: [radius, focal, falloff, weight_scale],
                msvo_pattern: entries,
                ..level_block(level)
            };
            queue.write_buffer(uniforms, Slot::occlude(level).offset(), &block.bytes());

            let texel = block.texel();
            for (index, step) in [(0, [texel.0, 0.0]), (1, [0.0, texel.1])] {
                let axis = ScreenBlock {
                    msvo_step: step,
                    ..level_block(level)
                };
                queue.write_buffer(uniforms, Slot::blur(level, index).offset(), &axis.bytes());
            }
        }

        for level in 0..levels - 1 {
            let coarse = level + 1;
            let coarse_texel = level_block(coarse).texel();
            let block = ScreenBlock {
                msvo_coarse_texel: [coarse_texel.0, coarse_texel.1],
                msvo_final: i32::from(level == 0),
                ..level_block(level)
            };
            queue.write_buffer(uniforms, Slot::upsample(level).offset(), &block.bytes());
        }
    }

    /// Allocates the pyramid and its bind groups when they do not already fit the frame.
    fn allocate(
        &mut self,
        device: &wgpu::Device,
        bindings: Bindings<'_>,
        frame: ScreenFrame<'_>,
        divisor: u32,
    ) {
        let finest = (
            (frame.size.0 / divisor).max(1),
            (frame.size.1 / divisor).max(1),
        );
        if self.divisor == divisor
            && self.generation == frame.generation
            && matches!(self.levels.first(), Some(level) if level.size == finest)
        {
            return;
        }

        self.levels.clear();
        self.groups = None;
        let count = usize::try_from(style::MSVO_LEVELS).unwrap_or(1);
        for level in 0..count {
            let size = ((finest.0 >> level).max(1), (finest.1 >> level).max(1));
            self.levels.push(Level {
                depth: allocate(device, "zax msvo depth", size, PYRAMID_FORMAT),
                occlusion: allocate(device, "zax msvo occlusion", size, OCCLUSION_FORMAT),
                smoothed: allocate(device, "zax msvo smoothed", size, OCCLUSION_FORMAT),
                combined: (level + 1 < count)
                    .then(|| allocate(device, "zax msvo combined", size, OCCLUSION_FORMAT)),
                size,
            });
        }
        self.divisor = divisor;
        self.generation = frame.generation;
        self.groups = Some(self.build_groups(device, bindings, frame));
    }

    /// The bind group of every pass, named after the targets that now exist.
    fn build_groups(
        &self,
        device: &wgpu::Device,
        bindings: Bindings<'_>,
        frame: ScreenFrame<'_>,
    ) -> Groups {
        let dummy = bindings.dummy;
        let group = |slot: Slot, depth: &wgpu::TextureView, sources: [&wgpu::TextureView; 4]| {
            screen_group(
                device,
                bindings.layout,
                bindings.uniforms,
                slot,
                bindings.sampler,
                depth,
                sources,
            )
        };
        let levels = self.levels.len();

        let mut pyramid = Vec::with_capacity(levels);
        // The finest level reads the frame's own depth through `zax_view_sample`; every coarser one
        // decimates the level above it, which is a source and not the frame's depth.
        pyramid.push(group(
            Slot::Msvo(0),
            frame.depth,
            [dummy, dummy, dummy, dummy],
        ));
        for level in 1..levels {
            pyramid.push(group(
                Slot::Msvo(0),
                frame.depth,
                [&self.levels[level - 1].depth.view, dummy, dummy, dummy],
            ));
        }

        let mut occlude = Vec::with_capacity(levels);
        let mut blur = Vec::with_capacity(levels);
        for level in 0..levels {
            let depth = &self.levels[level].depth.view;
            occlude.push(group(
                Slot::occlude(level),
                frame.depth,
                [depth, dummy, dummy, dummy],
            ));
            blur.push((
                group(
                    Slot::blur(level, 0),
                    frame.depth,
                    [&self.levels[level].occlusion.view, depth, dummy, dummy],
                ),
                group(
                    Slot::blur(level, 1),
                    frame.depth,
                    [&self.levels[level].smoothed.view, depth, dummy, dummy],
                ),
            ));
        }

        let mut upsample = Vec::with_capacity(levels.saturating_sub(1));
        for level in 0..levels.saturating_sub(1) {
            let coarse = level + 1;
            // The coarsest level carries its own occlusion; every finer one carries the
            // combination written by the step before this one.
            let coarse_ao = match &self.levels[coarse].combined {
                Some(combined) => &combined.view,
                None => &self.levels[coarse].occlusion.view,
            };
            upsample.push(group(
                Slot::upsample(level),
                frame.depth,
                [
                    &self.levels[level].depth.view,
                    &self.levels[coarse].depth.view,
                    coarse_ao,
                    &self.levels[level].occlusion.view,
                ],
            ));
        }

        Groups {
            pyramid,
            occlude,
            blur,
            upsample,
        }
    }
}

/// What [`Msvo`] needs of the module that owns it to make a bind group. Field borrows rather than
/// `&ScreenEffects`, because that value is borrowed mutably while this estimator runs.
#[derive(Debug, Clone, Copy)]
pub(super) struct Bindings<'a> {
    pub(super) layout: &'a wgpu::BindGroupLayout,
    pub(super) uniforms: &'a wgpu::Buffer,
    pub(super) sampler: &'a wgpu::Sampler,
    pub(super) dummy: &'a wgpu::TextureView,
}

/// One full-screen pass over `target`, which every step of the chain is. Each writes every texel
/// of its target, so nothing is cleared — the `Load` is what the OpenGL path's absence of a clear
/// is.
fn pass<'a>(
    encoder: &'a mut wgpu::CommandEncoder,
    label: &'static str,
    target: &wgpu::TextureView,
) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
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
    })
}

/// The triangle, with its pipeline and its group. An extension trait so that each step of
/// [`Msvo::draw`] reads as one line.
trait DrawWith {
    fn draw_with(self, pipeline: &wgpu::RenderPipeline, group: &wgpu::BindGroup);
}

impl DrawWith for wgpu::RenderPass<'_> {
    fn draw_with(mut self, pipeline: &wgpu::RenderPipeline, group: &wgpu::BindGroup) {
        self.set_pipeline(pipeline);
        self.set_bind_group(0, group, &[]);
        self.draw(0..3, 0..1);
    }
}

/// A pass count as the statistics carry it.
fn count(passes: usize) -> u32 {
    u32::try_from(passes).unwrap_or(u32::MAX)
}

const LINEARIZE: &str = include_str!("shaders/msvo_linearize.wgsl");
const DOWNSAMPLE: &str = include_str!("shaders/msvo_downsample.wgsl");
const OCCLUDE: &str = include_str!("shaders/msvo_occlude.wgsl");
const BLUR: &str = include_str!("shaders/msvo_blur.wgsl");
const UPSAMPLE: &str = include_str!("shaders/msvo_upsample.wgsl");
