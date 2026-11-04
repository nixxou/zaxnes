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
//!
//! AMD-055 REQ-SCN-080: the two screen-space passes of the Enhanced style read the **depth** of
//! the frame, which a renderbuffer cannot give them. That is the one allocation Classic cannot
//! share, so it is asked for by the frame ([`LinearColorBuffer::begin_pass`]'s `sample_depth`):
//! with the wish false the attachments are the ones of before the amendment, down to the
//! renderbuffer.

use glow::{Framebuffer, Renderbuffer, Texture, VertexArray};

use super::api::{Gl, GlError};
use super::program::ShaderProgram;
use super::shaders;
use crate::geometry;
use crate::numerics::Vector2;
use crate::target::RenderTarget;

/// The single-sample attachments.
#[derive(Debug)]
struct Attachments {
    framebuffer: Framebuffer,
    color: Texture,
    depth: DepthAttachment,
}

/// What holds the depth of the single-sample attachments.
///
/// **The default case is the first one**, and that is the point: a frame that samples no depth
/// allocates exactly what it allocated before AMD-055 (REQ-SCN-080, and point 1 of the lot's
/// brief: "no extra framebuffer, render target, texture or buffer allocated" in Classic).
#[derive(Debug, Clone, Copy)]
enum DepthAttachment {
    /// A renderbuffer: cheaper to allocate on some drivers, and it cannot be sampled.
    Plain(Renderbuffer),
    /// A `DEPTH_COMPONENT24` texture, which a screen-space pass of REQ-SCN-080 reads. The same
    /// format and the same transfer type the shadow map of REQ-SCN-030 already uses, so the pair
    /// is proven on both a desktop core context and GLES 3.0.
    Sampled(Texture),
}

impl DepthAttachment {
    /// The texture a screen pass samples, or `None` for a plain renderbuffer.
    const fn texture(self) -> Option<Texture> {
        match self {
            Self::Plain(_) => None,
            Self::Sampled(texture) => Some(texture),
        }
    }
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
    /// The size drawn into, at the bottom left of the attachments.
    width: i32,
    height: i32,
    /// The size of the attachments: the size drawn into, or more when the host reserved room
    /// for a size that changes often ([`Self::reserve`]).
    capacity: (i32, i32),
    /// The room the host asks to keep.
    reserve: (i32, i32),
    samples: i32,
    /// Whether the depth is a texture a screen pass can read (AMD-055 REQ-SCN-080). False for
    /// every Classic frame, and a change of it reallocates, as a change of size does.
    sample_depth: bool,
    /// Whether the multisampled copy has already been resolved into the single-sample
    /// attachments this frame, which is what the screen-space passes of REQ-SCN-080 need before
    /// they read the colour and the depth. [`Self::resolve_to`] then has nothing to blit.
    resolved: bool,
    /// The texture [`Self::resolve_to`] encodes instead of the frame's own colour, when something
    /// has filtered the frame into a target of its own: the depth-detecting SMAA of AMD-060
    /// REQ-SCN-085, whose neighbourhood blending cannot work in place.
    ///
    /// **`None` on every frame at any other level of `video.anti_alias`**: every
    /// [`Self::begin_pass`] clears it, so the invariant is "a frame starts with no redirection"
    /// and there is one place to read it off. The cost to a frame that does not filter is that
    /// store and the one `Option` test [`Self::resolve_to`] makes, both once a frame.
    redirect: Option<Texture>,
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
            capacity: (0, 0),
            reserve: (0, 0),
            samples: 1,
            sample_depth: false,
            resolved: false,
            redirect: None,
        })
    }

    /// Samples the scene is actually drawn with: 1 when anti-aliasing is off, and otherwise what
    /// the implementation granted, which may be fewer than what was asked for (AMD-032 §2).
    pub const fn samples(&self) -> i32 {
        self.samples
    }

    /// Keeps attachments of at least `width` × `height`, whatever size is drawn into: a host
    /// whose size changes often (the automatic resolution) then draws into part of them instead
    /// of reallocating them. `(0, 0)` gives the room back.
    pub const fn reserve(&mut self, width: i32, height: i32) {
        self.reserve = (width, height);
    }

    /// Binds the buffer as the draw target at `width` × `height`, reallocating it when the size
    /// of the attachments, the sample count or the depth wish changes. `samples` is clamped to
    /// what `GL_MAX_SAMPLES` allows, then lowered one step at a time until the multisampled
    /// framebuffer is complete (AMD-032 §2).
    ///
    /// `sample_depth` is the wish of AMD-055 REQ-SCN-080: a frame whose Enhanced style runs a
    /// screen-space pass needs a depth a shader can read. **False is the Classic path**, and with
    /// it this function allocates exactly what it allocated before the amendment.
    pub fn begin_pass(
        &mut self,
        width: i32,
        height: i32,
        samples: i32,
        sample_depth: bool,
    ) -> Result<(), GlError> {
        let (w, h) = (width.max(1), height.max(1));
        let capacity = (w.max(self.reserve.0), h.max(self.reserve.1));
        let wanted = geometry::clamp_anti_alias_samples(samples, self.max_samples);
        if self.attachments.is_none()
            || capacity != self.capacity
            || wanted != self.samples
            || sample_depth != self.sample_depth
        {
            self.release();
            self.attachments = Some(allocate(&self.gl, capacity.0, capacity.1, sample_depth)?);
            self.capacity = capacity;
            self.sample_depth = sample_depth;
            self.allocate_samples(wanted)?;
        }
        self.width = w;
        self.height = h;
        self.resolved = false;
        self.redirect = None;

        self.bind_for_draw();
        Ok(())
    }

    /// Binds the attachments the scene draws into — the multisampled copy while there is one and
    /// it has not been resolved yet — and sets the viewport to the drawn size.
    ///
    /// The renderer calls it again after a screen-space pass of REQ-SCN-080 has drawn into
    /// targets of its own, so that the transparent objects and the HUD go back to the frame.
    pub fn bind_for_draw(&self) {
        let framebuffer = match (&self.samples_attachments, &self.attachments) {
            (Some(multisampled), _) if !self.resolved => Some(multisampled.framebuffer),
            (_, single) => single.as_ref().map(|a| a.framebuffer),
        };
        self.gl.bind_framebuffer(glow::FRAMEBUFFER, framebuffer);
        self.gl.viewport(self.width, self.height);
    }

    /// The size drawn into this frame.
    pub const fn drawn(&self) -> (i32, i32) {
        (self.width, self.height)
    }

    /// The size of the attachments, which is the drawn size unless the host reserved room.
    pub const fn capacity(&self) -> (i32, i32) {
        self.capacity
    }

    /// The part of the attachments the frame drew into, in texture coordinates: `(1, 1)` unless
    /// the host reserved room. Every screen-space pass of REQ-SCN-080 scales its reads by it.
    pub fn drawn_scale(&self) -> Vector2 {
        #[allow(clippy::cast_precision_loss)] // Sizes far below 2^24.
        Vector2::new(
            self.width as f32 / self.capacity.0.max(1) as f32,
            self.height as f32 / self.capacity.1.max(1) as f32,
        )
    }

    /// The depth texture a screen-space pass of REQ-SCN-080 reads; `None` whenever the frame did
    /// not ask for one, which is every Classic frame.
    pub fn depth_texture(&self) -> Option<Texture> {
        self.attachments.as_ref().and_then(|a| a.depth.texture())
    }

    /// The linear colour of the frame, which the reflections of REQ-SCN-084 sample ("seule la
    /// scène déjà rendue peut être reflétée"). Always a texture — it is the one the resolve pass
    /// has always read — so this allocates nothing new; `None` before the first
    /// [`begin_pass`](Self::begin_pass).
    pub fn color_texture(&self) -> Option<Texture> {
        self.attachments.as_ref().map(|a| a.color)
    }

    /// Resolves the multisampled copy — colour **and** depth — into the single-sample
    /// attachments and binds those for the draws that follow, so that the screen-space passes of
    /// REQ-SCN-080 read a resolved frame ("après résolution du MSAA éventuel").
    ///
    /// The real consequence, stated where it happens: from here on the frame draws into the
    /// **single-sample** target, so the transparent objects and the HUD of a frame with
    /// anti-aliasing *and* a screen effect are not multisampled. That follows from the order
    /// REQ-SCN-080 fixes.
    ///
    /// Without a multisampled copy this only binds, which is what a frame with anti-aliasing off
    /// needs: the scene is already in the single-sample attachments.
    pub fn resolve_samples(&mut self) {
        if let (Some(sampled), Some(attachments)) = (&self.samples_attachments, &self.attachments) {
            let gl = &self.gl;
            gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(sampled.framebuffer));
            gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, Some(attachments.framebuffer));
            gl.disable(glow::SCISSOR_TEST);
            gl.blit_colour_and_depth(self.width, self.height);
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        }
        self.resolved = true;
        self.bind_for_draw();
    }

    /// Has the sRGB encode read `texture` instead of the frame's own colour, for the rest of this
    /// frame (AMD-060 REQ-SCN-085).
    ///
    /// The depth-detecting SMAA writes its filtered colour into a target of its own — it cannot
    /// work in place, unlike REQ-SCN-083's composite — and leaves that target bound, so the draws
    /// that follow need no change at all and this is the only thing the rest of the renderer has to
    /// be told. `None` puts the frame's own colour back.
    pub const fn redirect_color(&mut self, texture: Option<Texture>) {
        self.redirect = texture;
    }

    /// Encodes the buffer to sRGB into `target`, which becomes the bound framebuffer.
    pub fn resolve_to(&mut self, target: &RenderTarget) {
        let Some(attachments) = &self.attachments else {
            return;
        };

        let gl = &self.gl;
        if let (Some(sampled), false) = (&self.samples_attachments, self.resolved) {
            // Same size on both sides, so the filter is never used; NEAREST is the one a
            // multisample resolve is allowed to take.
            gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(sampled.framebuffer));
            gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, Some(attachments.framebuffer));
            gl.disable(glow::SCISSOR_TEST);
            gl.blit_colour(self.width, self.height);
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        }

        gl.bind_framebuffer(
            glow::FRAMEBUFFER,
            target.framebuffer.map(glow::NativeFramebuffer),
        );
        gl.viewport(target.width, target.height);
        gl.disable(glow::SCISSOR_TEST);
        gl.disable(glow::BLEND);
        gl.disable(glow::CULL_FACE);
        gl.disable(glow::DEPTH_TEST);
        gl.depth_mask(false);
        gl.color_mask(true);

        self.resolve.use_program();
        self.resolve.set_i32("uLinear", 0);
        // The part drawn into, in texture coordinates: the whole texture unless the host
        // reserved room.
        #[allow(clippy::cast_precision_loss)] // Sizes far below 2^24.
        self.resolve.set_vec2(
            "uUvScale",
            Vector2::new(
                self.width as f32 / self.capacity.0.max(1) as f32,
                self.height as f32 / self.capacity.1.max(1) as f32,
            ),
        );
        gl.active_texture(0);
        gl.bind_texture(
            glow::TEXTURE_2D,
            Some(self.redirect.unwrap_or(attachments.color)),
        );
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
                try_allocate_samples(&self.gl, self.capacity.0, self.capacity.1, count)?;
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
            release_depth(&self.gl, attachments.depth);
            self.gl.delete_texture(attachments.color);
            self.gl.delete_framebuffer(attachments.framebuffer);
        }
        self.samples = 1;
        self.width = 0;
        self.height = 0;
        self.capacity = (0, 0);
        self.sample_depth = false;
        self.resolved = false;
        self.redirect = None;
    }
}

impl Drop for LinearColorBuffer {
    fn drop(&mut self) {
        self.release();
        self.gl.delete_vertex_array(self.vertex_array);
    }
}

/// Allocates the single-sample attachments. Out of line on purpose: it runs only when the size,
/// the sample count or the depth wish of AMD-055 REQ-SCN-080 changes, and inlined it carried that
/// cold code into `begin_pass` and so into `render_scene`, which R0 counts.
#[inline(never)]
fn allocate(gl: &Gl, width: i32, height: i32, sample_depth: bool) -> Result<Attachments, GlError> {
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

    let depth = allocate_depth(gl, width, height, sample_depth)?;

    let status = gl.framebuffer_status();
    let attachments = Attachments {
        framebuffer,
        color,
        depth,
    };
    if status != glow::FRAMEBUFFER_COMPLETE {
        release_depth(gl, attachments.depth);
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

/// Attaches the depth of the single-sample framebuffer, which is bound.
///
/// With `sampled` false this is the renderbuffer of before AMD-055, call for call. With it true
/// the depth is a `DEPTH_COMPONENT24` **texture** instead, the one allocation the screen-space
/// passes of REQ-SCN-080 cannot do without — the same format and transfer type the shadow map of
/// REQ-SCN-030 has always used, so no capability is new here.
fn allocate_depth(
    gl: &Gl,
    width: i32,
    height: i32,
    sampled: bool,
) -> Result<DepthAttachment, GlError> {
    if !sampled {
        let depth = gl.create_renderbuffer()?;
        gl.renderbuffer_storage(depth, 1, glow::DEPTH_COMPONENT24, width, height);
        gl.framebuffer_renderbuffer(glow::DEPTH_ATTACHMENT, depth);
        return Ok(DepthAttachment::Plain(depth));
    }

    let depth = gl.create_texture()?;
    gl.bind_texture(glow::TEXTURE_2D, Some(depth));
    gl.tex_image_2d(
        glow::TEXTURE_2D,
        glow::DEPTH_COMPONENT24,
        width,
        height,
        glow::DEPTH_COMPONENT,
        glow::UNSIGNED_INT,
        None,
    );
    // The passes read single texels and reconstruct a position from them; a filtered depth is
    // meaningless, and GLES 3.0 does not make a 24-bit depth texture filterable anyway.
    for (parameter, value) in [
        (glow::TEXTURE_MIN_FILTER, glow::NEAREST),
        (glow::TEXTURE_MAG_FILTER, glow::NEAREST),
        (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
        (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
    ] {
        gl.tex_parameter(glow::TEXTURE_2D, parameter, value);
    }
    gl.bind_texture(glow::TEXTURE_2D, None);
    gl.framebuffer_texture(glow::DEPTH_ATTACHMENT, depth);
    Ok(DepthAttachment::Sampled(depth))
}

fn release_depth(gl: &Gl, depth: DepthAttachment) {
    match depth {
        DepthAttachment::Plain(renderbuffer) => gl.delete_renderbuffer(renderbuffer),
        DepthAttachment::Sampled(texture) => gl.delete_texture(texture),
    }
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
