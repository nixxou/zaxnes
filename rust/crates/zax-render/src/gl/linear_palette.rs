//! The frame palette converted to linear once per frame (Rust-only, a handheld GPU cost).
//!
//! The scene pass reads its indexed albedo from the 64-colour palette and linearises it per
//! fragment, three `pow` calls that a small GPU feels. Here the 64 entries go through the same
//! transfer function in a 64 × 1 pass of the same GPU, into an `RGBA32F` texture the scene pass
//! reads instead: the values are the ones it computed itself.
//!
//! Rendering into `RGBA32F` needs `EXT_color_buffer_float` on GLES 3.0. Without a complete
//! framebuffer the entries are converted on the CPU ([`color::to_linear`]), which may differ
//! from the GPU's `pow` in the last bit.

use glow::{Framebuffer, Texture, VertexArray};
use zax_engine3d::palette::{FramePalette, Rgb24};

use super::api::{Gl, GlError};
use super::program::ShaderProgram;
use super::shaders;
use crate::color;

/// One texel per palette entry.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // 64 entries.
const WIDTH: i32 = FramePalette::ENTRY_COUNT as i32;

/// The GPU side of the conversion, when the float target is complete.
#[derive(Debug)]
struct Converter {
    program: ShaderProgram,
    framebuffer: Framebuffer,
    vertex_array: VertexArray,
}

/// The linear palette texture and what fills it.
#[derive(Debug)]
pub struct LinearPalette {
    gl: Gl,
    texture: Texture,
    converter: Option<Converter>,
    /// The CPU fallback's texels, as bytes of `f32`.
    texels: Vec<u8>,
}

impl LinearPalette {
    /// Creates the texture and, when the GPU can render into it, the conversion pass.
    pub fn new(gl: &Gl) -> Result<Self, GlError> {
        let texture = gl.create_texture()?;
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            glow::RGBA32F,
            WIDTH,
            1,
            glow::RGBA,
            glow::FLOAT,
            None,
        );
        super::texture_cache::set_nearest_clamp(gl);
        gl.bind_texture(glow::TEXTURE_2D, None);

        let framebuffer = gl.create_framebuffer()?;
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
        gl.framebuffer_texture(glow::COLOR_ATTACHMENT0, texture);
        let complete = gl.framebuffer_status() == glow::FRAMEBUFFER_COMPLETE;
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        let converter = if complete {
            Some(Converter {
                program: ShaderProgram::create(gl, &shaders::LINEAR_PALETTE)?,
                framebuffer,
                vertex_array: gl.create_vertex_array()?,
            })
        } else {
            gl.delete_framebuffer(framebuffer);
            None
        };

        Ok(Self {
            gl: gl.clone(),
            texture,
            converter,
            texels: Vec::new(),
        })
    }

    /// The 64 × 1 linear palette.
    pub const fn texture(&self) -> Texture {
        self.texture
    }

    /// Converts the sRGB palette texture `palette`, which holds `colors`. The GPU pass leaves
    /// the default framebuffer bound, no texture bound on the active unit, and the depth test,
    /// blending, culling and scissor test disabled; the caller binds its own target afterwards.
    pub fn convert(&mut self, palette: Texture, colors: &[Rgb24; FramePalette::ENTRY_COUNT]) {
        let gl = &self.gl;
        let Some(converter) = &mut self.converter else {
            self.texels.clear();
            for color in colors {
                let linear = color::rgb_to_linear(*color);
                for value in [linear.x, linear.y, linear.z, 1.0] {
                    self.texels.extend_from_slice(&value.to_ne_bytes());
                }
            }
            gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                glow::RGBA32F,
                WIDTH,
                1,
                glow::RGBA,
                glow::FLOAT,
                Some(&self.texels),
            );
            gl.bind_texture(glow::TEXTURE_2D, None);
            return;
        };

        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(converter.framebuffer));
        gl.viewport(WIDTH, 1);
        gl.disable(glow::SCISSOR_TEST);
        gl.disable(glow::BLEND);
        gl.disable(glow::CULL_FACE);
        gl.disable(glow::DEPTH_TEST);
        gl.color_mask(true);
        converter.program.use_program();
        converter.program.set_i32("uPalette", 0);
        gl.bind_texture(glow::TEXTURE_2D, Some(palette));
        gl.bind_vertex_array(Some(converter.vertex_array));
        gl.draw_triangles(3);
        gl.bind_vertex_array(None);
        gl.bind_texture(glow::TEXTURE_2D, None);
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);
    }
}

impl Drop for LinearPalette {
    fn drop(&mut self) {
        if let Some(converter) = self.converter.take() {
            self.gl.delete_framebuffer(converter.framebuffer);
            self.gl.delete_vertex_array(converter.vertex_array);
        }
        self.gl.delete_texture(self.texture);
    }
}
