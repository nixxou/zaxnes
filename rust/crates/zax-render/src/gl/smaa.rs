//! The depth-detecting SMAA of AMD-060 REQ-SCN-085: its three passes, its targets and the two
//! precomputed tables of the reference.
//!
//! **The lifecycle is REQ-SCN-086**, and the module is shaped the way `super::screen_effects` is
//! shaped for the same clause of AMD-055: an [`Option`] the renderer holds, built the first frame
//! `video.anti_alias` is 3 and dropped the first frame it is not. At every other level "aucune
//! passe, aucune cible et aucune texture du SMAA NE DOIT exister ni être soumise", and the whole
//! cost of this file to such a frame is the `is_none` test that decides not to run it.
//!
//! The formats are ones both backends of REQ-SCN-056 already make colour-renderable without an
//! extension: the edges are two channels in `[0, 1]` and take `RG8`, the blending weights four and
//! take `RGBA8`, and the blended colour takes the `RGBA16F` the linear buffer of AMD-021 R2-009 has
//! always rendered into. The depth the first pass reads is the `DEPTH_COMPONENT24` **texture**
//! `LinearColorBuffer::begin_pass` already allocates for AMD-055 REQ-SCN-080, so this mode asks
//! the driver for nothing the Enhanced style does not already ask it for.
//!
//! The two tables come from `third_party/smaa` (`iryoku/smaa`, MIT; notice in
//! `THIRD-PARTY-NOTICES.md`) and are taken to the byte — see `src/assets/smaa/README.md` for the
//! shapes, the hashes and why they are committed as binary rather than generated.

use glow::{Texture, VertexArray};

use super::api::{Gl, GlError};
use super::program::ShaderProgram;
use super::screen_effects::{release, ScreenFrame, Target};
use super::shaders;
use crate::numerics::Vector4;

/// Bytes one texel of the edge target occupies: two channels of eight bits.
const EDGE_BYTES_PER_TEXEL: u64 = 2;

/// The area table of the reference, 160 × 560 `R8G8` (`AREATEX_PITCH` = 320).
const AREA_SIZE: (i32, i32) = (160, 560);

/// The search table of the reference, 64 × 16 `R8` — the cropped shape its own
/// `SMAA_SEARCHTEX_PACKED_SIZE` names.
const SEARCH_SIZE: (i32, i32) = (64, 16);

/// The two tables, which live with the other shader data in [`shaders`] — and are public there so
/// that a test can hash them (AMD-060 §4).
use shaders::{SMAA_AREA_TABLE as AREA_BYTES, SMAA_SEARCH_TABLE as SEARCH_BYTES};

/// The three passes of REQ-SCN-085 and everything they own, held only while the level is 3.
#[derive(Debug)]
pub struct Smaa {
    gl: Gl,
    vertex_array: VertexArray,
    /// The depth edge detection of the reference's `SMAADepthEdgeDetectionPS`.
    edges: ShaderProgram,
    /// The two tables, uploaded once with the module.
    area: Texture,
    search: Texture,
    /// `RG8`, the west and north edge of every pixel; cleared each frame, because the first pass
    /// `discard`s where there is no contour.
    edge_target: Option<Target>,
}

impl Smaa {
    /// How many programs this module holds, for `RenderStatistics::style_programs`' sibling
    /// `smaa_programs`.
    pub const PROGRAMS: u32 = 1;

    /// Builds the module: the full-screen triangle's vertex array, the programs of the passes, and
    /// the two tables of the reference uploaded to the device.
    ///
    /// # Errors
    ///
    /// [`GlError`] when the vertex array cannot be made, a program does not compile or link, or a
    /// table cannot be uploaded. The caller then draws the frame without anti-aliasing and writes
    /// the one diagnostic line of REQ-SCN-086's fallback.
    pub fn new(gl: &Gl) -> Result<Self, GlError> {
        let vertex_array = gl.create_vertex_array()?;
        let edges = ShaderProgram::create(gl, &shaders::SMAA_EDGES)?;
        // Both tables are read with bilinear filtering, as the reference's own `LinearSampler`
        // does: `SMAAArea` and `SMAAAreaDiag` interpolate between the areas of two distances, and
        // `SMAASearchLength`'s coordinates are built to land on texel centres ("Scale and bias to
        // access texel centers"), where a linear read gives the texel itself.
        let area = match upload_table(gl, AREA_SIZE, glow::RG8, glow::RG, AREA_BYTES) {
            Ok(texture) => texture,
            Err(error) => {
                gl.delete_vertex_array(vertex_array);
                return Err(error);
            }
        };
        let search = match upload_table(gl, SEARCH_SIZE, glow::R8, glow::RED, SEARCH_BYTES) {
            Ok(texture) => texture,
            Err(error) => {
                gl.delete_texture(area);
                gl.delete_vertex_array(vertex_array);
                return Err(error);
            }
        };

        Ok(Self {
            gl: gl.clone(),
            vertex_array,
            edges,
            area,
            search,
            edge_target: None,
        })
    }

    /// Bytes its targets occupy, for `RenderStatistics::smaa_bytes`; 0 until a pass has run once.
    ///
    /// The two tables are **not** counted, for the reason the shadow map is not counted in
    /// `screen_effect_bytes`: they do not grow with the resolution, and what the statistic is read
    /// for is the cost the mode adds per pixel. Their 180 224 bytes are stated in
    /// `src/assets/smaa/README.md` and in the lot's notes instead.
    pub fn bytes(&self) -> u64 {
        bytes(self.edge_target.as_ref(), EDGE_BYTES_PER_TEXEL)
    }

    /// Finds the edges of `frame` in its depth (REQ-SCN-085, first pass) and says how many passes
    /// that took.
    ///
    /// # Errors
    ///
    /// [`GlError`] when the edge target cannot be allocated. Nothing is drawn then, and the caller
    /// gives the mode up.
    pub fn detect_edges(&mut self, frame: ScreenFrame) -> Result<u32, GlError> {
        if !matches!(&self.edge_target, Some(target) if target.size == frame.capacity) {
            release(&self.gl, self.edge_target.take());
            self.edge_target = Some(allocate_edges(&self.gl, frame.capacity)?);
        }
        let Some(target) = &self.edge_target else {
            return Ok(0);
        };

        let gl = &self.gl;
        super::screen_effects::screen_pass_state(gl);
        gl.bind_vertex_array(Some(self.vertex_array));
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(target.framebuffer));
        gl.viewport(frame.drawn.0.max(1), frame.drawn.1.max(1));
        // The first pass `discard`s where there is no contour, so what it leaves behind is what
        // was here before it — hence the clear, every frame.
        gl.clear_color(0.0, 0.0, 0.0, 1.0);
        gl.clear(glow::COLOR_BUFFER_BIT);

        let program = &mut self.edges;
        program.use_program();
        program.set_i32("uDepth", 0);
        program.set_vec2("uDrawnScale", frame.drawn_scale);
        program.set_vec4("uRtMetrics", rt_metrics(frame.drawn));
        gl.active_texture(0);
        gl.bind_texture(glow::TEXTURE_2D, Some(frame.depth));
        gl.draw_triangles(3);

        Ok(1)
    }
}

impl Drop for Smaa {
    fn drop(&mut self) {
        release(&self.gl, self.edge_target.take());
        self.gl.delete_texture(self.search);
        self.gl.delete_texture(self.area);
        self.gl.delete_vertex_array(self.vertex_array);
    }
}

/// The reference's `SMAA_RT_METRICS` for a frame of `drawn` pixels: `(1/w, −1/h, w, h)`.
///
/// **The negative `y` is the whole of the coordinate convention**, and `shaders/smaa.glsl` point 1
/// says why: the reference is written for a texture coordinate whose `y` grows downward, the
/// algorithm is not symmetric under a mirror of it, and handing it a negative `y` makes "north"
/// mean up on screen in an OpenGL texture. The `wgpu` port passes `+1/h` here and changes nothing
/// else.
fn rt_metrics(drawn: (i32, i32)) -> Vector4 {
    #[allow(clippy::cast_precision_loss)] // Sizes far below 2^24.
    let (width, height) = (drawn.0.max(1) as f32, drawn.1.max(1) as f32);
    Vector4::new(1.0 / width, -1.0 / height, width, height)
}

fn bytes(target: Option<&Target>, per_texel: u64) -> u64 {
    target.map_or(0, |target| {
        let texels =
            u64::try_from(target.size.0).unwrap_or(0) * u64::try_from(target.size.1).unwrap_or(0);
        texels * per_texel
    })
}

/// The `RG8` edge target, at the attachments' capacity. **Filtered**, because the second pass's
/// horizontal and vertical searches read four edges at a time through bilinear filtering — the
/// reference's `@PSEUDO_GATHER4`, which a `NEAREST` edge texture would silently break.
fn allocate_edges(gl: &Gl, size: (i32, i32)) -> Result<Target, GlError> {
    super::screen_effects::allocate(
        gl,
        size,
        glow::RG8,
        glow::RG,
        glow::UNSIGNED_BYTE,
        true,
        None,
    )
}

/// One of the reference's two tables, uploaded as it stands.
fn upload_table(
    gl: &Gl,
    size: (i32, i32),
    internal: u32,
    format: u32,
    bytes: &[u8],
) -> Result<Texture, GlError> {
    let (width, height) = size;
    let texture = gl.create_texture()?;
    gl.bind_texture(glow::TEXTURE_2D, Some(texture));
    gl.tex_image_2d(
        glow::TEXTURE_2D,
        internal,
        width,
        height,
        format,
        glow::UNSIGNED_BYTE,
        Some(bytes),
    );
    for (parameter, value) in [
        (glow::TEXTURE_MIN_FILTER, glow::LINEAR),
        (glow::TEXTURE_MAG_FILTER, glow::LINEAR),
        (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
        (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
    ] {
        gl.tex_parameter(glow::TEXTURE_2D, parameter, value);
    }
    gl.bind_texture(glow::TEXTURE_2D, None);
    Ok(texture)
}

#[cfg(test)]
mod tests {
    use super::{AREA_BYTES, AREA_SIZE, SEARCH_BYTES, SEARCH_SIZE};

    /// AMD-060 §4: "les tables sont reprises à l'octet". Their *shape* is checked here, where the
    /// constants the shader's metrics are built from live; their bytes are pinned by SHA-256 in
    /// `tests/amd060_smaa_tests.rs`.
    #[test]
    fn req_scn_085_the_tables_have_the_shape_the_reference_declares() {
        // AREATEX_WIDTH 160, AREATEX_HEIGHT 560, AREATEX_PITCH (AREATEX_WIDTH * 2).
        assert_eq!((160, 560), AREA_SIZE);
        assert_eq!(160 * 560 * 2, AREA_BYTES.len());
        // SEARCHTEX_WIDTH 64, SEARCHTEX_HEIGHT 16, SEARCHTEX_PITCH SEARCHTEX_WIDTH.
        assert_eq!((64, 16), SEARCH_SIZE);
        assert_eq!(64 * 16, SEARCH_BYTES.len());
    }

    /// The `y` of `SMAA_RT_METRICS` is **negative** here, which is the one thing the port's
    /// correctness along every vertical edge rests on (`shaders/smaa.glsl` point 1).
    #[test]
    fn req_scn_085_the_render_target_metrics_mirror_y() {
        let metrics = super::rt_metrics((640, 480));
        assert!((metrics.x - 1.0 / 640.0).abs() < f32::EPSILON);
        assert!((metrics.y + 1.0 / 480.0).abs() < f32::EPSILON);
        assert!(metrics.y < 0.0, "north must mean +y in an OpenGL texture");
        assert!((metrics.z - 640.0).abs() < f32::EPSILON);
        assert!((metrics.w - 480.0).abs() < f32::EPSILON);
    }
}
