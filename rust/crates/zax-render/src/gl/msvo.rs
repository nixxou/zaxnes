// Portions adapted from Microsoft MiniEngine (c) Microsoft, MIT License
//
// Origin: the public repository `microsoft/DirectX-Graphics-Samples`, MiniEngine, files
// `Core/SSAO.cpp` and `Core/Shaders/Ao*.hlsl(i)`, author James Stanard. A copy of those files and
// of their MIT licence is kept under `third_party/MiniEngine-SSAO` (see `third_party/README.md`);
// the licence is carried with the distribution in `THIRD-PARTY-NOTICES.md`. MIT is compatible with
// the GPL-3.0 this reimplementation is distributed under.

//! The **MSVO** mode of AMD-055 REQ-SCN-083, `spl.ao` = 1: a multi-scale screen-space ambient
//! occlusion ported from the public MIT-licensed effect of Microsoft's `MiniEngine`, as the
//! requirement names, and **adapted to OpenGL 3.3 fragment passes** — the original is Direct3D 12
//! compute shaders that cache a tile of the depth in group-shared memory and gather from it, and
//! OpenGL 3.3 has neither compute shaders nor group-shared memory (REQ-SCN-056).
//!
//! What is kept of the original:
//!
//! - the pyramid of **linear** depths, each level half the previous one;
//! - the occlusion estimated per level by the **sphere penetration** test of `AoRenderCS.hlsli`,
//!   with its table of per-offset sphere thicknesses, its per-offset weights and its rejection
//!   fade-off;
//! - the same checkerboard of sample offsets, and the same axial / diagonal / L-shaped grouping;
//! - the `SmartBlur` of `AoBlurAndUpsampleCS.hlsli` over every level, and the **bilateral
//!   upsample** that carries each level into the next finer one and multiplies the two.
//!
//! What is adapted: the original's sphere has a fixed radius of five *pixels* per level, while
//! REQ-SCN-083 binds the radius `r` in **scene units**, so the pattern is scaled per pixel by
//! `r × focal / depth`; the depth is kept in view units rather than normalised by the far plane;
//! the accentuation of the original is left at 0, since §2.4's first invariant wants a lone plane
//! untouched; and the per-pixel variation of the pattern is a rotation rather than the original's
//! deinterleaving, which a fragment pass has no way to do. The four departures lot V2-25 of the
//! .NET application found — the missing `SmartBlur`, three levels instead of four, a coarsest
//! sphere of `r` instead of `1,75 r`, and a pattern fixed per pixel — are **corrected here from
//! the start**; see [`crate::style::MSVO_LEVELS`] and its siblings, and `Q-100`.
//!
//! **The lifecycle is `screen_effects`'s**, not this module's: this estimator is built the first
//! frame `spl.ao` = 1 asks for it and dropped as soon as no frame does, and the Depth estimator's
//! own programs and targets do not exist while it lives. What this module owns is the pyramid and
//! the per-level targets, allocated on the first run.

use glow::Texture;

use super::api::{Gl, GlError};
use super::program::ShaderProgram;
use super::screen_effects::{allocate, release, screen_pass_state, ScreenFrame, Target};
use super::shaders;
use crate::geometry;
use crate::numerics::Vector2;
use crate::style::{self, StyleSettings};

/// Bytes one texel of a pyramid level occupies: one half-float channel.
const DEPTH_BYTES_PER_TEXEL: u64 = 2;

/// Bytes one texel of an occlusion level occupies: one channel of eight bits.
const OCCLUSION_BYTES_PER_TEXEL: u64 = 1;

/// A hair under the far plane, the distance at which a pixel holds no geometry: the test
/// `msvo_upsample.frag` keeps the skybox and the cleared background out of the effect with
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
    /// The size of the attachments at this level, `capacity >> level`.
    capacity: (i32, i32),
}

/// The multi-scale occlusion: the five programs of the chain and the pyramid they work on.
#[derive(Debug)]
pub struct Msvo {
    linearize: ShaderProgram,
    downsample: ShaderProgram,
    occlude: ShaderProgram,
    blur: ShaderProgram,
    upsample: ShaderProgram,
    levels: Vec<Level>,
    /// 1 at `ao.quality` High, 2 otherwise — the divisor the pyramid was built for.
    divisor: i32,
}

impl Msvo {
    /// How many programs this estimator holds, for `RenderStatistics::style_programs`. The shared
    /// composite is counted by the caller, which owns it.
    pub const PROGRAMS: u32 = 5;

    /// Compiles the five programs; the pyramid is allocated on the first run.
    ///
    /// # Errors
    ///
    /// [`GlError`] when a program does not compile or link. The caller then falls back on the
    /// Depth estimator, which is REQ-SCN-083's declared fallback **(D)**.
    pub fn new(gl: &Gl) -> Result<Self, GlError> {
        Ok(Self {
            linearize: ShaderProgram::create(gl, &shaders::MSVO_LINEARIZE)?,
            downsample: ShaderProgram::create(gl, &shaders::MSVO_DOWNSAMPLE)?,
            occlude: ShaderProgram::create(gl, &shaders::MSVO_OCCLUDE)?,
            blur: ShaderProgram::create(gl, &shaders::MSVO_BLUR)?,
            upsample: ShaderProgram::create(gl, &shaders::MSVO_UPSAMPLE)?,
            levels: Vec::new(),
            divisor: 0,
        })
    }

    /// Bytes the pyramid occupies, for `RenderStatistics::screen_effect_bytes`; 0 until the first
    /// run.
    pub fn bytes(&self) -> u64 {
        self.levels
            .iter()
            .map(|level| {
                let texels = u64::try_from(level.capacity.0).unwrap_or(0)
                    * u64::try_from(level.capacity.1).unwrap_or(0);
                let occlusions = 2 + u64::from(level.combined.is_some());
                texels * (DEPTH_BYTES_PER_TEXEL + occlusions * OCCLUSION_BYTES_PER_TEXEL)
            })
            .sum()
    }

    /// Levels the pyramid holds; 0 until the first run.
    pub fn levels(&self) -> usize {
        self.levels.len()
    }

    /// The texture the shared composite of REQ-SCN-083 multiplies the frame by: the finest level's
    /// combination, which holds `O` itself. `None` until the first run.
    pub fn result(&self) -> Option<Texture> {
        self.levels
            .first()
            .and_then(|level| level.combined.as_ref().map(|target| target.texture))
    }

    /// Runs the whole chain over `frame` and says how many passes that took, leaving `O` in
    /// [`Self::result`] for the caller to composite.
    ///
    /// # Errors
    ///
    /// [`GlError`] when the pyramid cannot be allocated. Nothing is drawn then, and the caller
    /// falls back on the Depth estimator.
    pub fn draw(
        &mut self,
        gl: &Gl,
        frame: ScreenFrame,
        style: &StyleSettings,
        scene_scale: f32,
    ) -> Result<u32, GlError> {
        let divisor = if style.ao_half_resolution() { 2 } else { 1 };
        self.allocate(gl, frame.capacity, divisor)?;
        if self.levels.len() < 2 {
            return Ok(0);
        }

        screen_pass_state(gl);
        let mut submitted = self.build_pyramid(gl, frame, divisor);
        submitted += self.build_occlusion(gl, frame, style, (divisor, scene_scale));
        submitted += self.smooth(gl, frame, divisor);
        submitted += self.combine(gl, frame, style, divisor);
        Ok(submitted)
    }

    /// Releases the pyramid. The caller does this when the estimator is dropped and when the
    /// targets no longer fit the frame.
    pub fn release(&mut self, gl: &Gl) {
        for level in self.levels.drain(..) {
            release(gl, Some(level.depth));
            release(gl, Some(level.occlusion));
            release(gl, Some(level.smoothed));
            release(gl, level.combined);
        }
        self.divisor = 0;
    }

    /// The pyramid of linear depths: the frame's own depth linearised into the finest level, then
    /// each coarser level decimated from the one above it.
    fn build_pyramid(&mut self, gl: &Gl, frame: ScreenFrame, divisor: i32) -> u32 {
        let Some(finest) = self.levels.first() else {
            return 0;
        };

        bind(
            gl,
            finest.depth.framebuffer,
            drawn_at(frame.drawn, divisor, 0),
        );
        let program = &mut self.linearize;
        program.use_program();
        program.set_i32("uDepth", 0);
        program.set_vec2("uDrawnScale", frame.drawn_scale);
        program.set_vec4("uProjection", frame.projection);
        gl.active_texture(0);
        gl.bind_texture(glow::TEXTURE_2D, Some(frame.depth));
        gl.draw_triangles(3);

        let program = &mut self.downsample;
        program.use_program();
        program.set_i32("uSource", 0);
        for level in 1..self.levels.len() {
            let source = self.levels[level - 1].depth.texture;
            bind(
                gl,
                self.levels[level].depth.framebuffer,
                drawn_at(frame.drawn, divisor, level),
            );
            gl.bind_texture(glow::TEXTURE_2D, Some(source));
            gl.draw_triangles(3);
        }

        count(self.levels.len())
    }

    /// The occlusion of each level, each on its own share of the radius: the coarsest covers the
    /// whole of what the far field of §2.4 allows and each finer one half of the one above it,
    /// which is the fine contact and the wide darkening the mode is for.
    fn build_occlusion(
        &mut self,
        gl: &Gl,
        frame: ScreenFrame,
        style: &StyleSettings,
        (divisor, scene_scale): (i32, f32),
    ) -> u32 {
        let falloff = style.msvo_falloff();
        let entries = style.msvo_pattern_entries();
        let weight_scale = style.msvo_weight_scale();
        let program = &mut self.occlude;
        program.use_program();
        program.set_i32("uDepth", 0);
        program.set_f32("uFalloff", falloff);
        program.set_i32("uPatternSize", entries);
        program.set_f32("uWeightScale", weight_scale);
        program.set_f32("uHeadroom", style::MSVO_ACCESS_HEADROOM);

        for level in 0..self.levels.len() {
            let drawn = drawn_at(frame.drawn, divisor, level);
            let capacity = self.levels[level].capacity;
            #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
            let radius = style.msvo_level_radius(scene_scale, level as i32);
            program.set_f32("uRadius", radius);
            // The focal length of this level in pixels: half its height over `tan(fov / 2)`,
            // which the projection of REQ-SCN-004 carries in `m22`.
            #[allow(clippy::cast_precision_loss)] // Sizes far below 2^24.
            program.set_f32("uFocal", 0.5 * drawn.1 as f32 * frame.projection.y);
            program.set_vec2("uScale", scale_of(drawn, capacity));
            program.set_vec2("uTexel", texel_of(drawn));
            let source = self.levels[level].depth.texture;
            bind(gl, self.levels[level].occlusion.framebuffer, drawn);
            gl.bind_texture(glow::TEXTURE_2D, Some(source));
            gl.draw_triangles(3);
        }

        count(self.levels.len())
    }

    /// The `SmartBlur` of the original, over every level: two separable passes, horizontally into
    /// the level's smoothing target and back vertically into its own occlusion target, so that
    /// [`Self::combine`] reads a smoothed level whichever end of the chain it is at.
    fn smooth(&mut self, gl: &Gl, frame: ScreenFrame, divisor: i32) -> u32 {
        let program = &mut self.blur;
        program.use_program();
        program.set_i32("uAo", 0);
        program.set_i32("uDepth", 1);
        program.set_f32("uSharpness", style::MSVO_BLUR_SHARPNESS);

        for level in 0..self.levels.len() {
            let drawn = drawn_at(frame.drawn, divisor, level);
            let capacity = self.levels[level].capacity;
            let texel = texel_of(drawn);
            program.set_vec2("uScale", scale_of(drawn, capacity));
            gl.active_texture(1);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.levels[level].depth.texture));
            gl.active_texture(0);

            program.set_vec2("uStep", Vector2::new(texel.x, 0.0));
            bind(gl, self.levels[level].smoothed.framebuffer, drawn);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.levels[level].occlusion.texture));
            gl.draw_triangles(3);

            program.set_vec2("uStep", Vector2::new(0.0, texel.y));
            bind(gl, self.levels[level].occlusion.framebuffer, drawn);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.levels[level].smoothed.texture));
            gl.draw_triangles(3);
        }

        count(2 * self.levels.len())
    }

    /// The bilateral upsample chain, from the coarsest level to the finest; the last step writes
    /// the `O` the composite reads.
    fn combine(&mut self, gl: &Gl, frame: ScreenFrame, style: &StyleSettings, divisor: i32) -> u32 {
        let ceiling = style.ao_ceiling();
        let coarsest = self.levels.len() - 1;
        let program = &mut self.upsample;
        program.use_program();
        program.set_i32("uFineDepth", 0);
        program.set_i32("uCoarseDepth", 1);
        program.set_i32("uCoarseAo", 2);
        program.set_i32("uFineAo", 3);
        program.set_f32("uShare", style::MSVO_SHARE);
        program.set_f32("uHeadroom", style::MSVO_ACCESS_HEADROOM);
        program.set_f32("uCeiling", ceiling);
        program.set_f32("uFarDistance", EMPTY_DISTANCE);

        for level in (0..coarsest).rev() {
            let coarse = level + 1;
            let fine_drawn = drawn_at(frame.drawn, divisor, level);
            let coarse_drawn = drawn_at(frame.drawn, divisor, coarse);
            program.set_vec2(
                "uFineScale",
                scale_of(fine_drawn, self.levels[level].capacity),
            );
            program.set_vec2(
                "uCoarseScale",
                scale_of(coarse_drawn, self.levels[coarse].capacity),
            );
            program.set_vec2("uCoarseTexel", texel_of(coarse_drawn));
            program.set_i32("uFinal", i32::from(level == 0));

            let Some(target) = &self.levels[level].combined else {
                continue;
            };
            let framebuffer = target.framebuffer;
            // The coarsest level carries its own occlusion; every finer one carries the
            // combination written by the step before this one.
            let coarse_ao = if coarse == coarsest {
                self.levels[coarse].occlusion.texture
            } else {
                match &self.levels[coarse].combined {
                    Some(combined) => combined.texture,
                    None => self.levels[coarse].occlusion.texture,
                }
            };
            let textures = [
                self.levels[level].depth.texture,
                self.levels[coarse].depth.texture,
                coarse_ao,
                self.levels[level].occlusion.texture,
            ];
            bind(gl, framebuffer, fine_drawn);
            for (unit, texture) in textures.into_iter().enumerate() {
                #[allow(clippy::cast_possible_truncation)] // Four units.
                gl.active_texture(unit as u32);
                gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            }
            gl.draw_triangles(3);
        }

        for unit in (0..4).rev() {
            gl.active_texture(unit);
            gl.bind_texture(glow::TEXTURE_2D, None);
        }
        count(coarsest)
    }

    /// Allocates the pyramid when it does not already fit `capacity` at `divisor`.
    ///
    /// # Errors
    ///
    /// [`GlError`] when a target or its framebuffer is refused. The pyramid is then released
    /// whole rather than left half built, and the caller falls back on the Depth estimator.
    fn allocate(&mut self, gl: &Gl, capacity: (i32, i32), divisor: i32) -> Result<(), GlError> {
        let finest = ((capacity.0 / divisor).max(1), (capacity.1 / divisor).max(1));
        if self.divisor == divisor
            && matches!(self.levels.first(), Some(level) if level.capacity == finest)
        {
            return Ok(());
        }

        self.release(gl);
        let count = usize::try_from(style::MSVO_LEVELS).unwrap_or(1);
        for level in 0..count {
            let size = ((finest.0 >> level).max(1), (finest.1 >> level).max(1));
            let combined = if level + 1 < count {
                Some(occlusion_target(gl, size)?)
            } else {
                None
            };
            let built = Level {
                depth: depth_target(gl, size)?,
                occlusion: occlusion_target(gl, size)?,
                smoothed: occlusion_target(gl, size)?,
                combined,
                capacity: size,
            };
            self.levels.push(built);
        }
        self.divisor = divisor;
        Ok(())
    }
}

/// Binds `framebuffer` and gives it the viewport of the level that owns it.
fn bind(gl: &Gl, framebuffer: glow::Framebuffer, drawn: (i32, i32)) {
    gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
    gl.viewport(drawn.0, drawn.1);
}

/// The drawn part of level `level`, in texels: the viewport every pass of that level uses.
fn drawn_at(drawn: (i32, i32), divisor: i32, level: usize) -> (i32, i32) {
    (
        ((drawn.0 / divisor) >> level).max(1),
        ((drawn.1 / divisor) >> level).max(1),
    )
}

/// One texel of a level of `drawn` texels, in viewport coordinates.
fn texel_of(drawn: (i32, i32)) -> Vector2 {
    #[allow(clippy::cast_precision_loss)] // Sizes far below 2^24.
    Vector2::new(1.0 / drawn.0 as f32, 1.0 / drawn.1 as f32)
}

/// The part of a level's attachment the frame drew into, in texture coordinates.
///
/// **Computed per level rather than taken from the frame**, which is the one place this port
/// departs from what the two passes of RU-5 do. They scale every read by the frame's own
/// `drawn_scale`, which is right to a texel at full and half resolution; at a sixteenth of the
/// frame the integer truncation of `drawn >> 4` against `capacity >> 4` is no longer negligible,
/// and a coarse level read at the wrong scale shows up as a drift of the wide darkening across
/// the image.
fn scale_of(drawn: (i32, i32), capacity: (i32, i32)) -> Vector2 {
    #[allow(clippy::cast_precision_loss)] // Sizes far below 2^24.
    Vector2::new(
        drawn.0 as f32 / capacity.0.max(1) as f32,
        drawn.1 as f32 / capacity.1.max(1) as f32,
    )
}

/// A pass count as the statistics carry it.
fn count(passes: usize) -> u32 {
    u32::try_from(passes).unwrap_or(u32::MAX)
}

/// One level of the depth pyramid: `R16F`, because the level holds a **view distance in scene
/// units** and eight bits of it would quantise the sphere test to uselessness.
///
/// `R16F` is colour-renderable in OpenGL 3.3 core, and on GLES 3.0 through the very same
/// `EXT_color_buffer_float` / `EXT_color_buffer_half_float` that already makes the `RGBA16F` of
/// [`super::linear_buffer::LinearColorBuffer`] renderable — a driver that gives one gives the
/// other, so this is not a new capability. Where a driver refuses it all the same, the caller
/// falls back on the Depth estimator. Sixteen bits are enough *with* the access headroom of
/// [`style::MSVO_ACCESS_HEADROOM`] in place: measured, `R32F` changes no figure of §2.4's
/// invariants once the smoothing is allowed to average the estimator's noise both ways.
///
/// **`NEAREST`**, which matters. A level holds a *perspective* depth, and a perspective depth is
/// convex along a screen line; interpolating it between texel centres therefore overestimates it
/// by a ripple that peaks mid-texel, and the two samples of an opposite pair pick up different
/// amounts of that ripple — which is exactly the cancellation the sphere test rests on. Point
/// sampling has no ripple, and two opposite offsets about a texel centre round to two symmetric
/// texels. Measured on the lone plane of §2.4's first invariant at full resolution, this alone took
/// the worst ratio from 0,996 to exactly 1. The bilateral upsample wants no filtering either: it
/// gathers nine *distinct* coarse texels with weights of its own.
fn depth_target(gl: &Gl, size: (i32, i32)) -> Result<Target, GlError> {
    allocate(gl, size, glow::R16F, glow::RED, glow::FLOAT, false, None)
}

/// One occlusion level: `R8`, the format the Depth estimator's own targets use, filtered for the
/// same reason.
fn occlusion_target(gl: &Gl, size: (i32, i32)) -> Result<Target, GlError> {
    allocate(
        gl,
        size,
        glow::R8,
        glow::RED,
        glow::UNSIGNED_BYTE,
        true,
        None,
    )
}
