//! Uploads the object textures of SPEC-04B REQ-E3D-426 and the 64-colour frame palette of
//! REQ-E3D-561 to the GPU.
//!
//! An indexed texture becomes an `R8UI` texture whose texels keep their 0..7 value: the colour
//! is resolved at render time from the palette (REQ-E3D-565, REQ-SCN-034). A direct texture
//! becomes an `RGBA8` texture holding the sRGB colours. Sampling is nearest neighbour without
//! mipmap in both cases (REQ-E3D-563).

use std::sync::Arc;
use zax_engine3d::hashing::FastMap;

use glow::Texture;
use zax_engine3d::geometry::texture::ObjectTexture;
use zax_engine3d::palette::{FramePalette, Rgb24};

use super::api::{Gl, GlError};

/// Rendered frames an unused texture is kept for.
pub const RETENTION_FRAMES: u64 = 600;

/// Width of the palette texture, one texel per entry.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // 64 entries.
const PALETTE_WIDTH: i32 = FramePalette::ENTRY_COUNT as i32;

#[derive(Debug)]
struct Entry {
    /// Held only so that the keyed address stays unique while the entry lives.
    _texture: Arc<ObjectTexture>,
    handle: Texture,
    last_used: u64,
}

/// The resident object textures, the palette texture and the two dummies bound to the unused
/// samplers; everything is released when dropped.
#[derive(Debug)]
pub struct TextureCache {
    gl: Gl,
    entries: FastMap<*const ObjectTexture, Entry>,
    palette_bytes: [u8; FramePalette::ENTRY_COUNT * 3],
    palette: Texture,
    dummy_indexed: Texture,
    dummy_color: Texture,
    frame: u64,
}

impl TextureCache {
    /// Creates the cache, its palette texture and its two dummies.
    pub fn new(gl: &Gl) -> Result<Self, GlError> {
        let palette = create(
            gl,
            glow::RGB8,
            PALETTE_WIDTH,
            glow::RGB,
            glow::UNSIGNED_BYTE,
        )?;
        let dummy_indexed = create(gl, glow::R8UI, 1, glow::RED_INTEGER, glow::UNSIGNED_BYTE)?;
        let dummy_color = create(gl, glow::RGBA8, 1, glow::RGBA, glow::UNSIGNED_BYTE)?;
        Ok(Self {
            gl: gl.clone(),
            entries: FastMap::default(),
            palette_bytes: [0; FramePalette::ENTRY_COUNT * 3],
            palette,
            dummy_indexed,
            dummy_color,
            frame: 0,
        })
    }

    /// Number of object textures resident on the GPU.
    pub fn count(&self) -> usize {
        self.entries.len()
    }

    /// The 64 × 1 palette texture of the current frame.
    pub const fn palette(&self) -> Texture {
        self.palette
    }

    /// A 1 × 1 integer texture, bound to the unused indexed sampler.
    pub const fn dummy_indexed(&self) -> Texture {
        self.dummy_indexed
    }

    /// A 1 × 1 colour texture, bound to the unused colour sampler.
    pub const fn dummy_color(&self) -> Texture {
        self.dummy_color
    }

    /// Starts a rendered frame, for the retention bookkeeping.
    pub fn begin_frame(&mut self) {
        self.frame += 1;
    }

    /// Uploads the 64 resolved colours of the displayed frame (REQ-E3D-641). Called once per
    /// rendered frame: a palette change shows on the next frame without any mesh rebuild
    /// (REQ-E3D-565). It binds and unbinds on the active texture unit, so the caller sets up its
    /// sampler units afterwards.
    pub fn upload_palette(&mut self, colors: &[Rgb24; FramePalette::ENTRY_COUNT]) {
        for (bytes, color) in self.palette_bytes.chunks_exact_mut(3).zip(colors) {
            bytes.copy_from_slice(&[color.r, color.g, color.b]);
        }

        self.gl.bind_texture(glow::TEXTURE_2D, Some(self.palette));
        self.gl.pixel_store(glow::UNPACK_ALIGNMENT, 1);
        self.gl.tex_sub_image_2d(
            glow::TEXTURE_2D,
            PALETTE_WIDTH,
            1,
            glow::RGB,
            &self.palette_bytes,
        );
        self.gl.pixel_store(glow::UNPACK_ALIGNMENT, 4);
        self.gl.bind_texture(glow::TEXTURE_2D, None);
    }

    /// The GPU texture of `texture`, uploading it on first use.
    pub fn get(&mut self, texture: &Arc<ObjectTexture>) -> Result<Texture, GlError> {
        let key = Arc::as_ptr(texture);
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.last_used = self.frame;
            return Ok(entry.handle);
        }

        let handle = self.upload(texture)?;
        self.entries.insert(
            key,
            Entry {
                _texture: Arc::clone(texture),
                handle,
                last_used: self.frame,
            },
        );
        Ok(handle)
    }

    /// Releases every texture whose last use is older than the retention window.
    pub fn collect(&mut self) {
        let (gl, frame) = (&self.gl, self.frame);
        self.entries.retain(|_, entry| {
            let keep = frame - entry.last_used <= RETENTION_FRAMES;
            if !keep {
                gl.delete_texture(entry.handle);
            }
            keep
        });
    }

    /// Releases every object texture, keeping the palette.
    pub fn clear(&mut self) {
        for (_, entry) in self.entries.drain() {
            self.gl.delete_texture(entry.handle);
        }
    }

    #[allow(clippy::cast_sign_loss)] // Texture sizes are at least 1.
    fn upload(&self, texture: &ObjectTexture) -> Result<Texture, GlError> {
        let gl = &self.gl;
        let handle = gl.create_texture()?;
        gl.bind_texture(glow::TEXTURE_2D, Some(handle));

        let (width, height) = (texture.width(), texture.height());
        let texels = (0..height).flat_map(|y| (0..width).map(move |x| (x, y)));
        if texture.is_direct() {
            let pixels: Vec<u8> = texels
                .flat_map(|(x, y)| {
                    let color = texture.color(x, y);
                    [color.r, color.g, color.b, 255]
                })
                .collect();
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                glow::RGBA8,
                width,
                height,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                Some(&pixels),
            );
        } else {
            #[allow(clippy::cast_possible_truncation)] // The low byte, as `& 0xFF` in C#.
            let pixels: Vec<u8> = texels
                .map(|(x, y)| (texture.texel(x, y) & 0xFF) as u8)
                .collect();
            gl.pixel_store(glow::UNPACK_ALIGNMENT, 1);
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                glow::R8UI,
                width,
                height,
                glow::RED_INTEGER,
                glow::UNSIGNED_BYTE,
                Some(&pixels),
            );
            gl.pixel_store(glow::UNPACK_ALIGNMENT, 4);
        }

        set_nearest_clamp(gl);
        gl.bind_texture(glow::TEXTURE_2D, None);
        Ok(handle)
    }
}

impl Drop for TextureCache {
    fn drop(&mut self) {
        self.clear();
        self.gl.delete_texture(self.palette);
        self.gl.delete_texture(self.dummy_indexed);
        self.gl.delete_texture(self.dummy_color);
    }
}

/// A `width × 1` texture with undefined content.
fn create(gl: &Gl, internal: u32, width: i32, format: u32, ty: u32) -> Result<Texture, GlError> {
    let handle = gl.create_texture()?;
    gl.bind_texture(glow::TEXTURE_2D, Some(handle));
    gl.pixel_store(glow::UNPACK_ALIGNMENT, 1);
    gl.tex_image_2d(glow::TEXTURE_2D, internal, width, 1, format, ty, None);
    gl.pixel_store(glow::UNPACK_ALIGNMENT, 4);
    set_nearest_clamp(gl);
    gl.bind_texture(glow::TEXTURE_2D, None);
    Ok(handle)
}

/// Nearest sampling, no mipmap, clamped addressing on the bound 2D texture (REQ-E3D-563).
pub fn set_nearest_clamp(gl: &Gl) {
    for (parameter, value) in [
        (glow::TEXTURE_MIN_FILTER, glow::NEAREST),
        (glow::TEXTURE_MAG_FILTER, glow::NEAREST),
        (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
        (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
        (glow::TEXTURE_BASE_LEVEL, 0),
        (glow::TEXTURE_MAX_LEVEL, 0),
    ] {
        gl.tex_parameter(glow::TEXTURE_2D, parameter, value);
    }
}
