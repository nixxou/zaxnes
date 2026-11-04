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

/// Bytes one texel of the weight target occupies: four channels of eight bits.
const WEIGHT_BYTES_PER_TEXEL: u64 = 4;

/// Bytes one texel of the blended colour occupies: four half-float channels, the format the linear
/// buffer of AMD-021 R2-009 has always used.
const BLEND_BYTES_PER_TEXEL: u64 = 8;

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
    /// The reference's `SMAABlendingWeightCalculationPS`.
    weights: ShaderProgram,
    /// The reference's `SMAANeighborhoodBlendingPS`.
    blend: ShaderProgram,
    /// The two tables, uploaded once with the module.
    area: Texture,
    search: Texture,
    /// `RG8`, the west and north edge of every pixel; cleared each frame, because the first pass
    /// `discard`s where there is no contour.
    edge_target: Option<Target>,
    /// `RGBA8`, the blending weight of every pixel. **Not** cleared: the second pass writes every
    /// pixel, zero included.
    weight_target: Option<Target>,
    /// `RGBA16F`, the filtered colour — the one target of the three the filter cannot do without,
    /// because the neighbourhood blending writes a mix of two texels of the texture it reads and so
    /// cannot work in place the way REQ-SCN-083's composite does.
    ///
    /// The frame's **own depth** is attached to it, so that the HUD drawn after the filter
    /// depth-tests against the scene exactly as it did before (REQ-SCN-044).
    blend_target: Option<Target>,
    /// The depth the blend framebuffer has attached, so that a reallocation of the frame's
    /// attachments reallocates it with them — the same guard the floor mask of REQ-SCN-084 carries.
    blend_depth: Option<Texture>,
}

impl Smaa {
    /// How many programs this module holds, for `RenderStatistics::style_programs`' sibling
    /// `smaa_programs`.
    pub const PROGRAMS: u32 = 3;

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
        let programs = (
            ShaderProgram::create(gl, &shaders::SMAA_EDGES),
            ShaderProgram::create(gl, &shaders::SMAA_WEIGHTS),
            ShaderProgram::create(gl, &shaders::SMAA_BLEND),
        );
        let (edges, weights, blend) = match programs {
            (Ok(edges), Ok(weights), Ok(blend)) => (edges, weights, blend),
            (edges, weights, blend) => {
                gl.delete_vertex_array(vertex_array);
                // Whichever of the three failed is the error the fallback of REQ-SCN-086 reports;
                // the ones that linked are dropped with the tuple.
                return Err(edges.err().or(weights.err()).or(blend.err()).unwrap_or(
                    GlError::Unsupported {
                        what: "the SMAA programs",
                    },
                ));
            }
        };
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
            weights,
            blend,
            area,
            search,
            edge_target: None,
            weight_target: None,
            blend_target: None,
            blend_depth: None,
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
            + bytes(self.weight_target.as_ref(), WEIGHT_BYTES_PER_TEXEL)
            + bytes(self.blend_target.as_ref(), BLEND_BYTES_PER_TEXEL)
    }

    /// The filtered colour the sRGB encode must read instead of the frame's own, once this frame's
    /// three passes have run; `None` before the first one.
    pub fn blended_texture(&self) -> Option<Texture> {
        self.blend_target.as_ref().map(|target| target.texture)
    }

    /// Runs the three passes of REQ-SCN-085 over `frame` and says how many that took.
    ///
    /// The filtered colour is left in this module's own target, which the caller leaves **bound**:
    /// the HUD of REQ-SCN-044 then draws into it with no change to any draw path, and the sRGB
    /// encode reads [`Self::blended_texture`] instead of the frame's own colour.
    ///
    /// # Errors
    ///
    /// [`GlError`] when a target cannot be allocated. Nothing is drawn then, and the caller gives
    /// the mode up for the session (REQ-SCN-086's fallback).
    pub fn filter(&mut self, frame: ScreenFrame) -> Result<u32, GlError> {
        self.allocate(frame)?;
        let (Some(edges), Some(weights), Some(blended)) = (
            self.edge_target.as_ref(),
            self.weight_target.as_ref(),
            self.blend_target.as_ref(),
        ) else {
            return Ok(0);
        };
        let (edge_texture, weight_texture) = (edges.texture, weights.texture);
        let (edge_buffer, weight_buffer, blend_buffer) =
            (edges.framebuffer, weights.framebuffer, blended.framebuffer);

        let gl = &self.gl;
        super::screen_effects::screen_pass_state(gl);
        gl.bind_vertex_array(Some(self.vertex_array));
        let viewport = (frame.drawn.0.max(1), frame.drawn.1.max(1));
        let metrics = rt_metrics(frame.drawn);

        // 1. The depth edge detection. The pass `discard`s where there is no contour, so what it
        //    leaves behind is what was there before it — hence the clear, every frame.
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(edge_buffer));
        gl.viewport(viewport.0, viewport.1);
        gl.clear_color(0.0, 0.0, 0.0, 1.0);
        gl.clear(glow::COLOR_BUFFER_BIT);
        let program = &mut self.edges;
        program.use_program();
        program.set_i32("uDepth", 0);
        program.set_vec2("uDrawnScale", frame.drawn_scale);
        program.set_vec4("uRtMetrics", metrics);
        gl.active_texture(0);
        gl.bind_texture(glow::TEXTURE_2D, Some(frame.depth));
        gl.draw_triangles(3);

        // 2. The blending weights, through the two tables. No clear: this pass writes every pixel,
        //    zero included.
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(weight_buffer));
        let program = &mut self.weights;
        program.use_program();
        program.set_i32("uEdges", 0);
        program.set_i32("uArea", 1);
        program.set_i32("uSearch", 2);
        program.set_vec2("uDrawnScale", frame.drawn_scale);
        program.set_vec4("uRtMetrics", metrics);
        gl.active_texture(0);
        gl.bind_texture(glow::TEXTURE_2D, Some(edge_texture));
        gl.active_texture(1);
        gl.bind_texture(glow::TEXTURE_2D, Some(self.area));
        gl.active_texture(2);
        gl.bind_texture(glow::TEXTURE_2D, Some(self.search));
        gl.draw_triangles(3);

        // 3. The neighbourhood blending, into the target the frame carries on with. Depth writes
        //    stay off and the depth test off: this pass covers every pixel and must not disturb the
        //    depth the HUD is about to test against.
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(blend_buffer));
        let program = &mut self.blend;
        program.use_program();
        program.set_i32("uColor", 0);
        program.set_i32("uWeights", 1);
        program.set_vec2("uDrawnScale", frame.drawn_scale);
        program.set_vec4("uRtMetrics", metrics);
        gl.active_texture(0);
        gl.bind_texture(glow::TEXTURE_2D, Some(frame.color));
        gl.active_texture(1);
        gl.bind_texture(glow::TEXTURE_2D, Some(weight_texture));
        gl.draw_triangles(3);

        for unit in [2, 1] {
            gl.active_texture(unit);
            gl.bind_texture(glow::TEXTURE_2D, None);
        }
        gl.active_texture(0);
        gl.bind_texture(glow::TEXTURE_2D, None);
        gl.bind_vertex_array(None);
        // The blend target stays bound, with the frame's own depth attached: the HUD draws into it.
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(blend_buffer));
        gl.viewport(viewport.0, viewport.1);

        Ok(3)
    }

    /// Allocates the three targets at the attachments' capacity, or reallocates them when the
    /// capacity or the frame's depth texture changes.
    ///
    /// Out of line on purpose, the way `LinearColorBuffer::allocate` is: it runs on the first frame
    /// at level 3 and then never again until the view is resized.
    #[inline(never)]
    fn allocate(&mut self, frame: ScreenFrame) -> Result<(), GlError> {
        let sized = |target: &Option<Target>| matches!(target, Some(target) if target.size == frame.capacity);
        if !sized(&self.edge_target) {
            release(&self.gl, self.edge_target.take());
            self.edge_target = Some(allocate_edges(&self.gl, frame.capacity)?);
        }
        if !sized(&self.weight_target) {
            release(&self.gl, self.weight_target.take());
            self.weight_target = Some(allocate_weights(&self.gl, frame.capacity)?);
        }
        if !sized(&self.blend_target) || self.blend_depth != Some(frame.depth) {
            release(&self.gl, self.blend_target.take());
            self.blend_target = Some(allocate_blend(&self.gl, frame.capacity, frame.depth)?);
            self.blend_depth = Some(frame.depth);
        }
        Ok(())
    }
}

impl Drop for Smaa {
    fn drop(&mut self) {
        release(&self.gl, self.blend_target.take());
        release(&self.gl, self.weight_target.take());
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

/// The `RGBA8` weight target. **Filtered** too: the third pass reads three of its texels through
/// the filter, which is how `SMAASample(blendTex, …)` gets the neighbours' weights.
fn allocate_weights(gl: &Gl, size: (i32, i32)) -> Result<Target, GlError> {
    super::screen_effects::allocate(
        gl,
        size,
        glow::RGBA8,
        glow::RGBA,
        glow::UNSIGNED_BYTE,
        true,
        None,
    )
}

/// The `RGBA16F` target the filtered frame carries on in, with the frame's **own depth** attached
/// so that the HUD drawn after the filter depth-tests against the scene (REQ-SCN-044). `NEAREST`:
/// the sRGB encode reads it texel by texel, exactly as it reads the frame's own colour.
fn allocate_blend(gl: &Gl, size: (i32, i32), depth: Texture) -> Result<Target, GlError> {
    super::screen_effects::allocate(
        gl,
        size,
        glow::RGBA16F,
        glow::RGBA,
        glow::FLOAT,
        false,
        Some(depth),
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
