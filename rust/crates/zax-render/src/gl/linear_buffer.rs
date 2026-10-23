//! The scene's own colour buffer, holding *linear* values, and the pass that encodes it to sRGB
//! into the framebuffer the host provided (AMD-021 R2-009).
//!
//! R2-009 requires the transparency of REQ-SCN-040 — objects, HUD and the FPV occlusion of
//! REQ-SCN-022 — to be blended on linear colours. OpenGL blends whatever the fragment shader
//! wrote, so the shaders write linear and the sRGB encode happens after the blending.
//!
//! The host framebuffer is 8-bit sRGB (REQ-SCN-057: the renderer never creates it), so the
//! renderer renders into this half-float attachment instead — 8 bits of linear would band
//! visibly in the darks — and resolves it with one full-screen triangle. The alpha channel is
//! copied through unchanged: it is never a colour (REQ-SCN-035). RGBA16F is colour-renderable
//! in every GL 3.0 and later implementation; on GLES 3.0 it needs `EXT_color_buffer_float`,
//! which every GLES 3.0 driver of the targeted platforms has.
//!
//! AMD-032 §2: when anti-aliasing is on, the scene is drawn into a *multisampled* copy of the
//! same attachments and that copy is resolved into the single-sample texture before anything
//! samples it.

use glow::{Framebuffer, Renderbuffer, Texture, VertexArray};

use super::api::{Gl, GlError};
use super::program::ShaderProgram;
use super::shaders;
use crate::geometry;
use crate::target::RenderTarget;

/// The single-sample attachments.
#[derive(Debug)]
struct Attachments {
    framebuffer: Framebuffer,
    color: Texture,
    depth: Renderbuffer,
}

/// The multisampled attachments.
#[derive(Debug)]
struct SampleAttachments {
    framebuffer: Framebuffer,
    color: Renderbuffer,
    depth: Renderbuffer,
}

/// The linear buffer, allocated on first use; released when dropped.
#[derive(Debug)]
pub struct LinearColorBuffer {
    gl: Gl,
    resolve: ShaderProgram,
    vertex_array: VertexArray,
    attachments: Option<Attachments>,
    samples_attachments: Option<SampleAttachments>,
    max_samples: i32,
    width: i32,
    height: i32,
    samples: i32,
}

impl LinearColorBuffer {
    /// Creates the buffer and its resolve program; the attachments are allocated on the first
    /// [`begin_pass`](Self::begin_pass).
    pub fn new(gl: &Gl) -> Result<Self, GlError> {
        // A driver that answers nonsense must not disable anti-aliasing silently for ever; the
        // completeness check is the real guard. OpenGL 3.3 core always answers at least 4.
        let max_samples = gl
            .get_integer(glow::MAX_SAMPLES)
            .clamp(1, geometry::MAX_ANTI_ALIAS_SAMPLES);
        Ok(Self {
            gl: gl.clone(),
            resolve: ShaderProgram::create(gl, &shaders::RESOLVE)?,
            vertex_array: gl.create_vertex_array()?,
            attachments: None,
            samples_attachments: None,
            max_samples,
            width: 0,
            height: 0,
            samples: 1,
        })
    }

    /// Samples the scene is actually drawn with: 1 when anti-aliasing is off, and otherwise what
    /// the implementation granted, which may be fewer than what was asked for (AMD-032 §2).
    pub const fn samples(&self) -> i32 {
        self.samples
    }

    /// Binds the buffer as the draw target at `width` × `height`, reallocating it when the size
    /// or the sample count changed. `samples` is clamped to what `GL_MAX_SAMPLES` allows, then
    /// lowered one step at a time until the multisampled framebuffer is complete (AMD-032 §2).
    pub fn begin_pass(&mut self, width: i32, height: i32, samples: i32) -> Result<(), GlError> {
        let (w, h) = (width.max(1), height.max(1));
        let wanted = geometry::clamp_anti_alias_samples(samples, self.max_samples);
        if self.attachments.is_none() || w != self.width || h != self.height || wanted != self.samples
        {
            self.release();
            self.attachments = Some(allocate(&self.gl, w, h)?);
            self.width = w;
            self.height = h;
            self.allocate_samples(wanted)?;
        }

        let framebuffer = match (&self.samples_attachments, &self.attachments) {
            (Some(sampled), _) => Some(sampled.framebuffer),
            (None, attachments) => attachments.as_ref().map(|a| a.framebuffer),
        };
        self.gl.bind_framebuffer(glow::FRAMEBUFFER, framebuffer);
        self.gl.viewport(self.width, self.height);
        Ok(())
    }

    /// Encodes the buffer to sRGB into `target`, which becomes the bound framebuffer.
    pub fn resolve_to(&mut self, target: &RenderTarget) {
        let Some(attachments) = &self.attachments else {
            return;
        };

        let gl = &self.gl;
        if let Some(sampled) = &self.samples_attachments {
            // Same size on both sides, so the filter is never used; NEAREST is the one a
            // multisample resolve is allowed to take.
            gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(sampled.framebuffer));
            gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, Some(attachments.framebuffer));
            gl.disable(glow::SCISSOR_TEST);
            gl.blit_colour(self.width, self.height);
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        }

        gl.bind_framebuffer(glow::FRAMEBUFFER, target.framebuffer.map(glow::NativeFramebuffer));
        gl.viewport(target.width, target.height);
        gl.disable(glow::SCISSOR_TEST);
        gl.disable(glow::BLEND);
        gl.disable(glow::CULL_FACE);
        gl.disable(glow::DEPTH_TEST);
        gl.depth_mask(false);
        gl.color_mask(true);

        self.resolve.use_program();
        self.resolve.set_i32("uLinear", 0);
        gl.active_texture(0);
        gl.bind_texture(glow::TEXTURE_2D, Some(attachments.color));
        gl.bind_vertex_array(Some(self.vertex_array));
        gl.draw_triangles(3);
        gl.bind_vertex_array(None);

        gl.enable(glow::DEPTH_TEST);
        gl.depth_mask(true);
    }

    /// Allocates the multisampled attachments, halving the sample count until the framebuffer
    /// is complete and falling back to no anti-aliasing at all (AMD-032 §2).
    fn allocate_samples(&mut self, samples: i32) -> Result<(), GlError> {
        self.samples = 1;
        let mut count = samples;
        while count > 1 {
            let (attachments, status) =
                try_allocate_samples(&self.gl, self.width, self.height, count)?;
            if status == glow::FRAMEBUFFER_COMPLETE {
                self.samples_attachments = Some(attachments);
                self.samples = count;
                return Ok(());
            }

            release_samples(&self.gl, &attachments);
            count /= 2;
        }

        Ok(())
    }

    fn release(&mut self) {
        if let Some(sampled) = self.samples_attachments.take() {
            release_samples(&self.gl, &sampled);
        }
        if let Some(attachments) = self.attachments.take() {
            self.gl.delete_renderbuffer(attachments.depth);
            self.gl.delete_texture(attachments.color);
            self.gl.delete_framebuffer(attachments.framebuffer);
        }
        self.samples = 1;
        self.width = 0;
        self.height = 0;
    }
}

impl Drop for LinearColorBuffer {
    fn drop(&mut self) {
        self.release();
        self.gl.delete_vertex_array(self.vertex_array);
    }
}

fn allocate(gl: &Gl, width: i32, height: i32) -> Result<Attachments, GlError> {
    let framebuffer = gl.create_framebuffer()?;
    gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));

    let color = gl.create_texture()?;
    gl.bind_texture(glow::TEXTURE_2D, Some(color));
    gl.tex_image_2d(
        glow::TEXTURE_2D,
        glow::RGBA16F,
        width,
        height,
        glow::RGBA,
        glow::FLOAT,
        None,
    );
    for (parameter, value) in [
        (glow::TEXTURE_MIN_FILTER, glow::NEAREST),
        (glow::TEXTURE_MAG_FILTER, glow::NEAREST),
        (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
        (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
    ] {
        gl.tex_parameter(glow::TEXTURE_2D, parameter, value);
    }
    gl.framebuffer_texture(glow::COLOR_ATTACHMENT0, color);
    gl.bind_texture(glow::TEXTURE_2D, None);

    let depth = gl.create_renderbuffer()?;
    gl.renderbuffer_storage(depth, 1, glow::DEPTH_COMPONENT24, width, height);
    gl.framebuffer_renderbuffer(glow::DEPTH_ATTACHMENT, depth);

    let status = gl.framebuffer_status();
    let attachments = Attachments {
        framebuffer,
        color,
        depth,
    };
    if status != glow::FRAMEBUFFER_COMPLETE {
        gl.delete_renderbuffer(attachments.depth);
        gl.delete_texture(attachments.color);
        gl.delete_framebuffer(attachments.framebuffer);
        return Err(GlError::Incomplete {
            what: "linear colour",
            width,
            height,
            status,
        });
    }

    Ok(attachments)
}

fn try_allocate_samples(
    gl: &Gl,
    width: i32,
    height: i32,
    samples: i32,
) -> Result<(SampleAttachments, u32), GlError> {
    let framebuffer = gl.create_framebuffer()?;
    gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));

    let color = gl.create_renderbuffer()?;
    gl.renderbuffer_storage(color, samples, glow::RGBA16F, width, height);
    gl.framebuffer_renderbuffer(glow::COLOR_ATTACHMENT0, color);

    let depth = gl.create_renderbuffer()?;
    gl.renderbuffer_storage(depth, samples, glow::DEPTH_COMPONENT24, width, height);
    gl.framebuffer_renderbuffer(glow::DEPTH_ATTACHMENT, depth);

    let status = gl.framebuffer_status();
    gl.bind_framebuffer(glow::FRAMEBUFFER, None);
    Ok((
        SampleAttachments {
            framebuffer,
            color,
            depth,
        },
        status,
    ))
}

fn release_samples(gl: &Gl, attachments: &SampleAttachments) {
    gl.delete_renderbuffer(attachments.depth);
    gl.delete_renderbuffer(attachments.color);
    gl.delete_framebuffer(attachments.framebuffer);
}
