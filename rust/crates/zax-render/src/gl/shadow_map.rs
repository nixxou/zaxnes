//! The single shadow map of SPEC-06 REQ-SCN-030: hard shadows, no cascade, 4096 texels of side
//! by default (AMD-034 §2, which replaces the 1024 of DEC-R2-D4).
//!
//! OpenGL 3.3 only guarantees `GL_MAX_TEXTURE_SIZE` ≥ 1024, so 4096 is not a size every
//! implementation has to accept. AMD-034 §2 asks for the largest one it does accept: the
//! requested size is clamped to what the driver advertises, then halved until the depth
//! framebuffer comes out complete — 4096, 2048, 1024. [`ShadowMap::requested`] and
//! [`ShadowMap::size`] differ exactly when that happened, which is what the host reports once.

use glow::{Framebuffer, Texture};

use super::api::{Gl, GlError};
use crate::geometry;

/// The depth texture and its framebuffer; released when dropped.
#[derive(Debug)]
pub struct ShadowMap {
    gl: Gl,
    framebuffer: Framebuffer,
    depth_texture: Texture,
    max_texture_size: i32,
    size: i32,
    requested: i32,
}

impl ShadowMap {
    /// Creates the map at `size` texels of side, or the largest smaller size the hardware
    /// accepts.
    pub fn new(gl: &Gl, size: i32) -> Result<Self, GlError> {
        // A driver that answers nonsense must not shrink the map for ever; the completeness
        // check is the real guard.
        let max_texture_size = gl
            .get_integer(glow::MAX_TEXTURE_SIZE)
            .max(geometry::MIN_SHADOW_MAP_SIZE);
        let requested = size.max(geometry::MIN_SHADOW_MAP_SIZE);
        let (framebuffer, depth_texture, granted) = allocate(gl, requested, max_texture_size)?;
        Ok(Self {
            gl: gl.clone(),
            framebuffer,
            depth_texture,
            max_texture_size,
            size: granted,
            requested,
        })
    }

    /// Side of the map in texels, as the hardware granted it.
    pub const fn size(&self) -> i32 {
        self.size
    }

    /// Side of the map in texels, as it was asked for.
    pub const fn requested(&self) -> i32 {
        self.requested
    }

    /// The depth texture.
    pub const fn depth_texture(&self) -> Texture {
        self.depth_texture
    }

    /// Reallocates the map when the setting changes.
    pub fn resize(&mut self, size: i32) -> Result<(), GlError> {
        let requested = size.max(geometry::MIN_SHADOW_MAP_SIZE);
        if requested == self.requested {
            return Ok(());
        }

        let (framebuffer, depth_texture, granted) =
            allocate(&self.gl, requested, self.max_texture_size)?;
        self.release();
        self.framebuffer = framebuffer;
        self.depth_texture = depth_texture;
        self.size = granted;
        self.requested = requested;
        Ok(())
    }

    /// Binds the map as the draw target and clears its depth.
    pub fn begin_pass(&self) {
        let gl = &self.gl;
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.framebuffer));
        gl.viewport(self.size, self.size);
        gl.depth_mask(true);
        gl.clear_depth(1.0);
        gl.clear(glow::DEPTH_BUFFER_BIT);
    }

    fn release(&self) {
        self.gl.delete_framebuffer(self.framebuffer);
        self.gl.delete_texture(self.depth_texture);
    }
}

impl Drop for ShadowMap {
    fn drop(&mut self) {
        self.release();
    }
}

/// Allocates the largest map the hardware accepts, starting at `requested` and halving on
/// refusal (AMD-034 §2).
fn allocate(
    gl: &Gl,
    requested: i32,
    max_texture_size: i32,
) -> Result<(Framebuffer, Texture, i32), GlError> {
    let mut last_status = 0;
    for side in geometry::shadow_map_sizes(requested, max_texture_size) {
        let (framebuffer, texture, status) = try_allocate(gl, side)?;
        if status == glow::FRAMEBUFFER_COMPLETE {
            return Ok((framebuffer, texture, side));
        }

        gl.delete_framebuffer(framebuffer);
        gl.delete_texture(texture);
        last_status = status;
    }

    Err(GlError::Incomplete {
        what: "shadow map",
        width: geometry::MIN_SHADOW_MAP_SIZE,
        height: geometry::MIN_SHADOW_MAP_SIZE,
        status: last_status,
    })
}

/// Builds the depth texture and its framebuffer, and says how complete the pair is.
fn try_allocate(gl: &Gl, size: i32) -> Result<(Framebuffer, Texture, u32), GlError> {
    let texture = gl.create_texture()?;
    gl.bind_texture(glow::TEXTURE_2D, Some(texture));
    // No data is uploaded; UNSIGNED_INT is the transfer type both GL 3.3 and GLES 3.0 accept
    // for a 24-bit depth texture.
    gl.tex_image_2d(
        glow::TEXTURE_2D,
        glow::DEPTH_COMPONENT24,
        size,
        size,
        glow::DEPTH_COMPONENT,
        glow::UNSIGNED_INT,
        None,
    );

    // Hard shadows: nearest sampling, no soft filtering (REQ-SCN-030). Outside the map the
    // depth reads as the far plane; GLES 3.0 has no border colour, and the scene shader drops
    // every sample outside [0, 1] anyway.
    let wrap = if gl.embedded() {
        glow::CLAMP_TO_EDGE
    } else {
        glow::CLAMP_TO_BORDER
    };
    for (parameter, value) in [
        (glow::TEXTURE_MIN_FILTER, glow::NEAREST),
        (glow::TEXTURE_MAG_FILTER, glow::NEAREST),
        (glow::TEXTURE_WRAP_S, wrap),
        (glow::TEXTURE_WRAP_T, wrap),
    ] {
        gl.tex_parameter(glow::TEXTURE_2D, parameter, value);
    }
    if !gl.embedded() {
        gl.tex_border_color(glow::TEXTURE_2D, &[1.0; 4]);
    }
    gl.bind_texture(glow::TEXTURE_2D, None);

    let framebuffer = gl.create_framebuffer()?;
    gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
    gl.framebuffer_texture(glow::DEPTH_ATTACHMENT, texture);
    gl.draw_no_colour();
    let status = gl.framebuffer_status();
    gl.bind_framebuffer(glow::FRAMEBUFFER, None);
    Ok((framebuffer, texture, status))
}
