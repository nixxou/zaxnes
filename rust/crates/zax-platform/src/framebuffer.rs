//! The offscreen framebuffer the scene is drawn into before it is blitted onto the window
//! (`OffscreenFramebuffer`), and the read-back of the window for captures. The application
//! forbids `unsafe`, and every GL call is unsafe in `glow`, so the calls live here.

use std::num::NonZeroU32;
use std::rc::Rc;

use glow::HasContext;

use crate::PlatformError;

/// A colour texture and a depth-stencil renderbuffer behind one framebuffer object.
pub struct OffscreenFramebuffer {
    gl: Rc<glow::Context>,
    framebuffer: Option<glow::Framebuffer>,
    color_texture: Option<glow::Texture>,
    depth_buffer: Option<glow::Renderbuffer>,
    width: i32,
    height: i32,
}

impl OffscreenFramebuffer {
    /// Allocates a framebuffer of at least one pixel in each direction.
    ///
    /// # Errors
    /// The framebuffer could not be created or is incomplete.
    pub fn new(gl: Rc<glow::Context>, width: i32, height: i32) -> Result<Self, PlatformError> {
        let mut buffer = Self {
            gl,
            framebuffer: None,
            color_texture: None,
            depth_buffer: None,
            width: 0,
            height: 0,
        };
        buffer.allocate(width.max(1), height.max(1))?;
        Ok(buffer)
    }

    /// Width in pixels.
    #[must_use]
    pub const fn width(&self) -> i32 {
        self.width
    }

    /// Height in pixels.
    #[must_use]
    pub const fn height(&self) -> i32 {
        self.height
    }

    /// The framebuffer object's name, for the renderer's target.
    #[must_use]
    pub fn handle(&self) -> Option<NonZeroU32> {
        self.framebuffer.map(|framebuffer| framebuffer.0)
    }

    /// Reallocates the buffers when the size changed.
    ///
    /// # Errors
    /// The new framebuffer could not be created or is incomplete.
    pub fn resize(&mut self, width: i32, height: i32) -> Result<(), PlatformError> {
        let (width, height) = (width.max(1), height.max(1));
        if width == self.width && height == self.height {
            return Ok(());
        }
        self.release();
        self.allocate(width, height)
    }

    /// Copies the buffer onto the default framebuffer, stretched to the window and filtered
    /// bilinearly (REQ-SCN-063).
    pub fn blit_to_default_framebuffer(&self, window_width: i32, window_height: i32) {
        // SAFETY: plain GL calls on the context this buffer was created with, which is current
        // on the calling thread (the application is single-threaded).
        unsafe {
            self.gl
                .bind_framebuffer(glow::READ_FRAMEBUFFER, self.framebuffer);
            self.gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, None);
            self.gl.blit_framebuffer(
                0,
                0,
                self.width,
                self.height,
                0,
                0,
                window_width.max(1),
                window_height.max(1),
                glow::COLOR_BUFFER_BIT,
                glow::LINEAR,
            );
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        }
    }

    /// Reads the buffer back as `width` x `height` RGB, nearest-sampled, rows top to bottom:
    /// the thumbnails of the save states and covers. `None` for an empty size.
    #[must_use]
    pub fn read_scaled_rgb(&self, width: i32, height: i32) -> Option<Vec<u8>> {
        let (target_width, target_height) =
            (usize::try_from(width).ok()?, usize::try_from(height).ok()?);
        let (source_width, source_height) = (
            usize::try_from(self.width).ok()?,
            usize::try_from(self.height).ok()?,
        );
        if source_width == 0 || source_height == 0 || target_width == 0 || target_height == 0 {
            return None;
        }
        let mut source = vec![0_u8; source_width * source_height * 4];
        // SAFETY: `source` holds `width * height * 4` bytes, the size of the RGBA read.
        unsafe {
            self.gl
                .bind_framebuffer(glow::READ_FRAMEBUFFER, self.framebuffer);
            self.gl.read_pixels(
                0,
                0,
                self.width,
                self.height,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(Some(&mut source)),
            );
            self.gl.bind_framebuffer(glow::READ_FRAMEBUFFER, None);
        }
        // GL reads rows bottom-up, so the vertical axis is flipped here.
        let mut target = vec![0_u8; target_width * target_height * 3];
        for y in 0..target_height {
            let source_y = (source_height - 1).saturating_sub(y * source_height / target_height);
            for x in 0..target_width {
                let source_x = (x * source_width / target_width).min(source_width - 1);
                let from = (source_y * source_width + source_x) * 4;
                let to = (y * target_width + x) * 3;
                target[to..to + 3].copy_from_slice(&source[from..from + 3]);
            }
        }
        Some(target)
    }

    fn allocate(&mut self, width: i32, height: i32) -> Result<(), PlatformError> {
        self.width = width;
        self.height = height;
        let gl = Rc::clone(&self.gl);
        // SAFETY: plain GL calls on the current context; the objects created are owned by
        // `self` and deleted by `release`.
        let status = unsafe {
            let framebuffer = gl.create_framebuffer().map_err(PlatformError::from)?;
            self.framebuffer = Some(framebuffer);
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
            let texture = gl.create_texture().map_err(PlatformError::from)?;
            self.color_texture = Some(texture);
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA8.cast_signed(),
                width,
                height,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(None),
            );
            for (parameter, value) in [
                (glow::TEXTURE_MIN_FILTER, glow::LINEAR),
                (glow::TEXTURE_MAG_FILTER, glow::LINEAR),
                (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
                (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
            ] {
                gl.tex_parameter_i32(glow::TEXTURE_2D, parameter, value.cast_signed());
            }
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(texture),
                0,
            );
            gl.bind_texture(glow::TEXTURE_2D, None);
            let depth = gl.create_renderbuffer().map_err(PlatformError::from)?;
            self.depth_buffer = Some(depth);
            gl.bind_renderbuffer(glow::RENDERBUFFER, Some(depth));
            gl.renderbuffer_storage(glow::RENDERBUFFER, glow::DEPTH24_STENCIL8, width, height);
            gl.framebuffer_renderbuffer(
                glow::FRAMEBUFFER,
                glow::DEPTH_STENCIL_ATTACHMENT,
                glow::RENDERBUFFER,
                Some(depth),
            );
            gl.bind_renderbuffer(glow::RENDERBUFFER, None);
            let status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            status
        };
        if status != glow::FRAMEBUFFER_COMPLETE {
            self.release();
            return Err(PlatformError::from(format!(
                "Offscreen framebuffer {width}x{height} is incomplete: {status:#x}."
            )));
        }
        Ok(())
    }

    fn release(&mut self) {
        // SAFETY: deletes the objects `allocate` created, on the same context.
        unsafe {
            if let Some(depth) = self.depth_buffer.take() {
                self.gl.delete_renderbuffer(depth);
            }
            if let Some(texture) = self.color_texture.take() {
                self.gl.delete_texture(texture);
            }
            if let Some(framebuffer) = self.framebuffer.take() {
                self.gl.delete_framebuffer(framebuffer);
            }
        }
    }
}

impl Drop for OffscreenFramebuffer {
    fn drop(&mut self) {
        self.release();
    }
}

/// Reads the window's own framebuffer as RGB, rows top to bottom, for the capture options of
/// the command line.
#[must_use]
pub fn read_window_rgb(gl: &glow::Context, width: i32, height: i32) -> Vec<u8> {
    let (columns, rows) = (
        usize::try_from(width).unwrap_or(0),
        usize::try_from(height).unwrap_or(0),
    );
    let mut pixels = vec![0_u8; columns * rows * 3];
    // SAFETY: `pixels` holds `width * height * 3` bytes, the size of the RGB read with a pack
    // alignment of 1.
    unsafe {
        gl.bind_framebuffer(glow::READ_FRAMEBUFFER, None);
        gl.pixel_store_i32(glow::PACK_ALIGNMENT, 1);
        gl.read_pixels(
            0,
            0,
            width,
            height,
            glow::RGB,
            glow::UNSIGNED_BYTE,
            glow::PixelPackData::Slice(Some(&mut pixels)),
        );
    }
    let stride = columns * 3;
    let mut flipped = Vec::with_capacity(pixels.len());
    for row in (0..rows).rev() {
        flipped.extend_from_slice(&pixels[row * stride..(row + 1) * stride]);
    }
    flipped
}

/// Binds the window's own framebuffer, sets the viewport to the whole window and clears the
/// colour, which is where the offscreen buffer is blitted and the interface drawn.
pub fn begin_window_pass(gl: &glow::Context, width: i32, height: i32) {
    // SAFETY: plain GL state calls on the current context.
    unsafe {
        gl.viewport(0, 0, width, height);
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        gl.clear(glow::COLOR_BUFFER_BIT);
    }
}

/// The renderer and version strings of the GL context, for the log and the statistics.
#[must_use]
pub fn gl_strings(gl: &glow::Context) -> (String, String) {
    // SAFETY: reads two constant strings of the current context.
    unsafe {
        (
            gl.get_parameter_string(glow::RENDERER),
            gl.get_parameter_string(glow::VERSION),
        )
    }
}
