//! The 256 × 240 pixel frame of the core, uploaded as an RGBA texture for the 2D mode of
//! SPEC-06 REQ-SCN-055 and AMD-009 (Q-022).
//!
//! The core hands out one NES colour index per pixel (SPEC-02 REQ-CORE-048); they are resolved
//! here through the fixed 64-colour table of SPEC-04 §4.1, which is also the table the tiles
//! use. The texture is a direct-colour one for the scene shader, sampled nearest without mipmap
//! like every other NES texture (REQ-E3D-563).

use glow::Texture;
use zax_engine3d::palette::nes_color;

use super::api::{Gl, GlError};
use super::texture_cache::set_nearest_clamp;

/// Width of the pixel frame.
pub const WIDTH: usize = 256;

/// Height of the pixel frame.
pub const HEIGHT: usize = 240;

/// The picture texture, created on first upload; released when dropped.
#[derive(Debug)]
pub struct NesPictureTexture {
    gl: Gl,
    pixels: Vec<u8>,
    handle: Option<Texture>,
    /// Size of the texture allocated: 256 x 240, or larger for a widescreen picture.
    size: (usize, usize),
}

impl NesPictureTexture {
    /// Creates the texture for the current context.
    pub fn new(gl: &Gl) -> Self {
        Self {
            gl: gl.clone(),
            pixels: vec![0; WIDTH * HEIGHT * 4],
            handle: None,
            size: (WIDTH, HEIGHT),
        }
    }

    /// Uploads `indices` (NES colour indices, row 0 first, `height` rows of 256 or of a
    /// widescreen width) and returns the texture, or `None` when the frame has no such size.
    pub fn upload(&mut self, indices: &[u8], height: usize) -> Result<Option<Texture>, GlError> {
        let Some(size) = crate::interop::picture_size(indices.len(), height) else {
            return Ok(None);
        };
        if size != self.size {
            // Another size: a new texture of the right size.
            if let Some(handle) = self.handle.take() {
                self.gl.delete_texture(handle);
            }
            self.size = size;
        }
        self.pixels.resize(size.0 * size.1 * 4, 0);

        for (pixel, &index) in self.pixels.chunks_exact_mut(4).zip(indices) {
            if index == crate::interop::HIDDEN_PIXEL {
                pixel.copy_from_slice(&[0; 4]);
                continue;
            }
            let color = nes_color(usize::from(index));
            pixel.copy_from_slice(&[color.r, color.g, color.b, 255]);
        }

        let gl = &self.gl;
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // At most 1280.
        let (width, height) = (size.0 as i32, size.1 as i32);
        let existing = self.handle;
        let handle = match existing {
            Some(handle) => handle,
            None => gl.create_texture()?,
        };
        gl.bind_texture(glow::TEXTURE_2D, Some(handle));
        gl.pixel_store(glow::UNPACK_ALIGNMENT, 1);
        if existing.is_some() {
            gl.tex_sub_image_2d(glow::TEXTURE_2D, width, height, glow::RGBA, &self.pixels);
        } else {
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                glow::RGBA8,
                width,
                height,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                Some(&self.pixels),
            );
            set_nearest_clamp(gl);
            self.handle = Some(handle);
        }

        gl.pixel_store(glow::UNPACK_ALIGNMENT, 4);
        gl.bind_texture(glow::TEXTURE_2D, None);
        Ok(Some(handle))
    }
}

impl Drop for NesPictureTexture {
    fn drop(&mut self) {
        if let Some(handle) = self.handle {
            self.gl.delete_texture(handle);
        }
    }
}
