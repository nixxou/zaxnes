//! The screen-space passes of the Enhanced style (AMD-055 REQ-SCN-080): the ambient occlusion of
//! REQ-SCN-083 today, the floor reflections of REQ-SCN-084 beside it, and everything they own.
//!
//! **The lifecycle is the requirement.** Point 1 of this lot's brief asks that Classic cost
//! exactly what it costs today — no extra pass, framebuffer, render target, texture or buffer —
//! and point 2 that each Enhanced effect pay only when it is on. So this module is an
//! [`Option`] the renderer holds: it is created the first time a frame asks for a screen effect
//! and dropped as soon as none does, which takes its programs and its targets with it. In Classic,
//! and in Enhanced at the defaults of REQ-SCN-079, the whole cost of AMD-055's screen side is the
//! `is_none` test that decides not to run it.
//!
//! The two targets it allocates are one-channel `R8`, a format OpenGL 3.3 core and GLES 3.0 both
//! make colour-renderable, so no capability is probed here; the depth the passes read is the
//! frame's own ([`super::linear_buffer::LinearColorBuffer`]).

use glow::{Framebuffer, Texture, VertexArray};

use super::api::{Gl, GlError};
use super::program::ShaderProgram;
use super::shaders;
use crate::numerics::{Vector2, Vector4};
use crate::style::StyleSettings;

/// Bytes one texel of a target occupies: one channel of eight bits.
const BYTES_PER_TEXEL: u64 = 1;

/// What the occlusion pass of REQ-SCN-083 needs of the frame it darkens.
#[derive(Debug, Clone, Copy)]
pub struct ScreenFrame {
    /// The frame's depth, resolved (REQ-SCN-080's last bullet).
    pub depth: Texture,
    /// The size of the attachments, which the targets follow.
    pub capacity: (i32, i32),
    /// The size actually drawn into, which is the viewport of every pass.
    pub drawn: (i32, i32),
    /// The part of the attachments drawn into, in texture coordinates.
    pub drawn_scale: Vector2,
    /// The projection as [`shaders::AO`] reads it: `xy` the two scales, `z` the `range` term,
    /// `w` the near plane — read straight off the matrix the camera built.
    pub projection: Vector4,
}

/// The targets of the occlusion: the raw pass and its blur, both one channel.
#[derive(Debug)]
struct OcclusionTargets {
    raw_framebuffer: Framebuffer,
    raw: Texture,
    blurred_framebuffer: Framebuffer,
    blurred: Texture,
    /// The size of the pair, which is the attachments' capacity divided by [`Self::divisor`].
    size: (i32, i32),
    /// 1 at `ao.quality` High, 2 otherwise (REQ-SCN-083: "à la moitié de chaque dimension de la
    /// cible de rendu courante sauf en High").
    divisor: i32,
}

impl OcclusionTargets {
    fn bytes(&self) -> u64 {
        let texels =
            u64::try_from(self.size.0).unwrap_or(0) * u64::try_from(self.size.1).unwrap_or(0);
        2 * texels * BYTES_PER_TEXEL
    }
}

/// The screen-space passes and their resources, held only while a frame wants one.
#[derive(Debug)]
pub struct ScreenEffects {
    gl: Gl,
    vertex_array: VertexArray,
    occlusion: ShaderProgram,
    blur: ShaderProgram,
    composite: ShaderProgram,
    targets: Option<OcclusionTargets>,
}

impl ScreenEffects {
    /// Compiles the programs of the occlusion. Called the first time a frame asks for it.
    ///
    /// # Errors
    ///
    /// [`GlError`] when a program does not compile or link, or the vertex array cannot be made.
    /// The caller then draws the frame without the effect rather than not at all.
    pub fn new(gl: &Gl) -> Result<Self, GlError> {
        Ok(Self {
            gl: gl.clone(),
            vertex_array: gl.create_vertex_array()?,
            occlusion: ShaderProgram::create(gl, &shaders::AO)?,
            blur: ShaderProgram::create(gl, &shaders::AO_BLUR)?,
            composite: ShaderProgram::create(gl, &shaders::AO_COMPOSITE)?,
            targets: None,
        })
    }

    /// How many programs this module holds, for `RenderStatistics::style_programs`: the occlusion,
    /// its blur and the composite.
    ///
    /// A constant of the type rather than of the value, because [`Self::new`] compiles all three or
    /// none — the module exists exactly when a frame wants a screen effect. The floor reflections
    /// of REQ-SCN-084 will add theirs, and the count becomes a property of the value the day one
    /// effect can be on without the other.
    pub const PROGRAMS: u32 = 3;

    /// Bytes its targets occupy, for `RenderStatistics::screen_effect_bytes`; 0 until a pass has
    /// run once.
    pub fn bytes(&self) -> u64 {
        self.targets.as_ref().map_or(0, OcclusionTargets::bytes)
    }

    /// Runs the occlusion of REQ-SCN-083 over `frame` and multiplies the colour already in the
    /// bound framebuffer by `1 − O`, and says how many passes that took.
    ///
    /// The caller has bound the frame's own framebuffer and resolved any multisampling; on return
    /// it is bound again, with the blend disabled and the depth test off — the state the caller
    /// restores for the transparent objects.
    ///
    /// # Errors
    ///
    /// [`GlError`] when a target cannot be allocated. Nothing is drawn then.
    pub fn occlude(
        &mut self,
        frame: ScreenFrame,
        style: &StyleSettings,
        scene_scale: f32,
    ) -> Result<u32, GlError> {
        let divisor = if style.ao_half_resolution() { 2 } else { 1 };
        self.allocate(frame.capacity, divisor)?;
        let Some(targets) = &self.targets else {
            return Ok(0);
        };

        let gl = &self.gl;
        gl.disable(glow::SCISSOR_TEST);
        gl.disable(glow::BLEND);
        gl.disable(glow::CULL_FACE);
        gl.disable(glow::DEPTH_TEST);
        gl.depth_mask(false);
        gl.color_mask(true);
        gl.bind_vertex_array(Some(self.vertex_array));

        let viewport = (frame.drawn.0 / divisor, frame.drawn.1 / divisor);
        #[allow(clippy::cast_precision_loss)] // Sizes far below 2^24.
        let texel = Vector2::new(
            divisor as f32 / frame.drawn.0.max(1) as f32,
            divisor as f32 / frame.drawn.1.max(1) as f32,
        );
        let radius = style.ao_radius_units(scene_scale);

        // 1. The occlusion itself, from the depth alone.
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(targets.raw_framebuffer));
        gl.viewport(viewport.0.max(1), viewport.1.max(1));
        self.occlusion.use_program();
        self.occlusion.set_i32("uDepth", 0);
        self.occlusion.set_vec2("uDrawnScale", frame.drawn_scale);
        self.occlusion.set_vec4("uProjection", frame.projection);
        self.occlusion.set_vec2("uTexel", texel);
        self.occlusion.set_f32("uRadius", radius);
        self.occlusion.set_f32("uCeiling", style.ao_ceiling());
        // A hundredth of the radius, and never under a fiftieth of a voxel: enough to keep the
        // flattest directions of the kernel from occluding their own surface, far too little to
        // hide a contact — a voxel is one scene unit.
        self.occlusion.set_f32("uBias", (0.01 * radius).max(0.02));
        self.occlusion.set_i32("uSamples", style.ao_samples());
        gl.active_texture(0);
        gl.bind_texture(glow::TEXTURE_2D, Some(frame.depth));
        gl.draw_triangles(3);

        // 2. The blur, which respects the depth discontinuities (REQ-SCN-083).
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(targets.blurred_framebuffer));
        self.blur.use_program();
        self.blur.set_i32("uOcclusion", 0);
        self.blur.set_i32("uDepth", 1);
        self.blur.set_vec2("uDrawnScale", frame.drawn_scale);
        self.blur.set_vec4("uProjection", frame.projection);
        self.blur.set_vec2("uTexel", texel);
        gl.active_texture(0);
        gl.bind_texture(glow::TEXTURE_2D, Some(targets.raw));
        gl.active_texture(1);
        gl.bind_texture(glow::TEXTURE_2D, Some(frame.depth));
        gl.draw_triangles(3);
        gl.active_texture(1);
        gl.bind_texture(glow::TEXTURE_2D, None);

        Ok(2)
    }

    /// Multiplies the colour in the bound framebuffer by `1 − O` (REQ-SCN-083), which the caller
    /// has bound and given the frame's viewport again. One pass.
    pub fn composite(&mut self, frame: ScreenFrame) -> u32 {
        let Some(targets) = &self.targets else {
            return 0;
        };

        let gl = &self.gl;
        // `dst × src`, so the full-screen triangle scales the linear buffer in place. No second
        // colour target, no copy of the scene (REQ-SCN-083's "en espace linéaire, avant
        // l'encodage sRGB").
        gl.enable(glow::BLEND);
        gl.blend_func(glow::ZERO, glow::SRC_COLOR);
        self.composite.use_program();
        self.composite.set_i32("uOcclusion", 0);
        self.composite.set_vec2("uDrawnScale", frame.drawn_scale);
        gl.active_texture(0);
        gl.bind_texture(glow::TEXTURE_2D, Some(targets.blurred));
        gl.draw_triangles(3);

        gl.disable(glow::BLEND);
        gl.bind_vertex_array(None);
        gl.bind_texture(glow::TEXTURE_2D, None);
        1
    }

    /// Keeps a pair of targets of `capacity / divisor`, reallocating when either changes.
    fn allocate(&mut self, capacity: (i32, i32), divisor: i32) -> Result<(), GlError> {
        let size = ((capacity.0 / divisor).max(1), (capacity.1 / divisor).max(1));
        if matches!(&self.targets, Some(targets) if targets.size == size && targets.divisor == divisor)
        {
            return Ok(());
        }

        self.release_targets();
        let (raw_framebuffer, raw) = allocate_target(&self.gl, size.0, size.1, false)?;
        let (blurred_framebuffer, blurred) = match allocate_target(&self.gl, size.0, size.1, true) {
            Ok(pair) => pair,
            Err(error) => {
                self.gl.delete_texture(raw);
                self.gl.delete_framebuffer(raw_framebuffer);
                return Err(error);
            }
        };
        self.targets = Some(OcclusionTargets {
            raw_framebuffer,
            raw,
            blurred_framebuffer,
            blurred,
            size,
            divisor,
        });
        Ok(())
    }

    fn release_targets(&mut self) {
        if let Some(targets) = self.targets.take() {
            self.gl.delete_texture(targets.blurred);
            self.gl.delete_framebuffer(targets.blurred_framebuffer);
            self.gl.delete_texture(targets.raw);
            self.gl.delete_framebuffer(targets.raw_framebuffer);
        }
    }
}

impl Drop for ScreenEffects {
    fn drop(&mut self) {
        self.release_targets();
        self.gl.delete_vertex_array(self.vertex_array);
    }
}

/// One `R8` target and its framebuffer. `filtered` gives the blurred one a linear filter, which
/// is what smooths the half-resolution occlusion back up to the frame's size; the raw one is read
/// texel by texel and takes `NEAREST`.
fn allocate_target(
    gl: &Gl,
    width: i32,
    height: i32,
    filtered: bool,
) -> Result<(Framebuffer, Texture), GlError> {
    let texture = gl.create_texture()?;
    gl.bind_texture(glow::TEXTURE_2D, Some(texture));
    gl.tex_image_2d(
        glow::TEXTURE_2D,
        glow::R8,
        width,
        height,
        glow::RED,
        glow::UNSIGNED_BYTE,
        None,
    );
    let filter = if filtered {
        glow::LINEAR
    } else {
        glow::NEAREST
    };
    for (parameter, value) in [
        (glow::TEXTURE_MIN_FILTER, filter),
        (glow::TEXTURE_MAG_FILTER, filter),
        (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
        (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
    ] {
        gl.tex_parameter(glow::TEXTURE_2D, parameter, value);
    }
    gl.bind_texture(glow::TEXTURE_2D, None);

    let framebuffer = gl.create_framebuffer()?;
    gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
    gl.framebuffer_texture(glow::COLOR_ATTACHMENT0, texture);
    let status = gl.framebuffer_status();
    gl.bind_framebuffer(glow::FRAMEBUFFER, None);
    if status != glow::FRAMEBUFFER_COMPLETE {
        gl.delete_texture(texture);
        gl.delete_framebuffer(framebuffer);
        return Err(GlError::Incomplete {
            what: "screen effect",
            width,
            height,
            status,
        });
    }

    Ok((framebuffer, texture))
}
