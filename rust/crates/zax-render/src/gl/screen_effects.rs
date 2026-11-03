//! The screen-space passes of the Enhanced style (AMD-055 REQ-SCN-080): the floor reflections of
//! REQ-SCN-084, the ambient occlusion of REQ-SCN-083, and everything they own.
//!
//! **The lifecycle is the requirement.** Point 1 of this lot's brief asks that Classic cost
//! exactly what it costs today — no extra pass, framebuffer, render target, texture or buffer —
//! and point 2 that each Enhanced effect pay only when it is on. So this module is an
//! [`Option`] the renderer holds, and each of the two effects inside it is an `Option` of its
//! own: a half is created the first time a frame asks for *that* effect and dropped as soon as no
//! frame does, which takes its programs and its targets with it. In Classic, and in Enhanced at
//! the defaults of REQ-SCN-079, the whole cost of AMD-055's screen side is the `is_none` test
//! that decides not to run it; with one effect on, the other half does not exist.
//!
//! The formats are the two the renderer already depends on: the occlusion is one scalar in
//! `[0, 1]` and takes `R8`, which OpenGL 3.3 core and GLES 3.0 both make colour-renderable; the
//! reflections carry a linear colour and take `RGBA16F`, the format
//! [`super::linear_buffer::LinearColorBuffer`] has always rendered into (on GLES 3.0 through
//! `EXT_color_buffer_float`, which every GLES 3.0 driver of the targeted platforms has). So no
//! capability is new here, and the depth the passes read is the frame's own.

use glow::{Framebuffer, Texture, VertexArray};

use super::api::{Gl, GlError};
use super::msvo::Msvo;
use super::program::ShaderProgram;
use super::shaders;
use crate::numerics::{Vector2, Vector4};
use crate::style::StyleSettings;

/// Bytes one texel of an occlusion target occupies: one channel of eight bits.
const OCCLUSION_BYTES_PER_TEXEL: u64 = 1;

/// Bytes one texel of the reflection target occupies: four half-float channels.
const REFLECTION_BYTES_PER_TEXEL: u64 = 8;

/// What the two passes of REQ-SCN-080 need of the frame they work on.
#[derive(Debug, Clone, Copy)]
pub struct ScreenFrame {
    /// The frame's depth, resolved (REQ-SCN-080's last bullet).
    pub depth: Texture,
    /// The frame's linear colour, resolved: what the reflections of REQ-SCN-084 reflect.
    pub color: Texture,
    /// The size of the attachments, which the targets follow.
    pub capacity: (i32, i32),
    /// The size actually drawn into, which is the viewport of every pass.
    pub drawn: (i32, i32),
    /// The part of the attachments drawn into, in texture coordinates.
    pub drawn_scale: Vector2,
    /// The projection as [`shaders::AO`] and [`shaders::SSR`] read it: `xy` the two scales, `z`
    /// the `range` term, `w` the near plane — read straight off the matrix the camera built.
    pub projection: Vector4,
}

/// One target and its framebuffer.
///
/// `pub(super)` so that [`super::msvo`], which allocates fifteen of them for its pyramid, builds
/// them the same way the two passes here do rather than with a second copy of the code.
#[derive(Debug)]
pub(super) struct Target {
    pub(super) framebuffer: Framebuffer,
    pub(super) texture: Texture,
    pub(super) size: (i32, i32),
}

/// The hemisphere estimator of REQ-SCN-083 — the mode `spl.ao` = 2, and the declared fallback
/// **(D)** of the MSVO mode: two programs and the pair of `R8` targets of the raw pass and its
/// blur.
#[derive(Debug)]
struct Depth {
    pass: ShaderProgram,
    blur: ShaderProgram,
    raw: Option<Target>,
    blurred: Option<Target>,
    /// 1 at `ao.quality` High, 2 otherwise (REQ-SCN-083: "à la moitié de chaque dimension de la
    /// cible de rendu courante sauf en High").
    divisor: i32,
}

/// Which estimator of REQ-SCN-083 is in force. **One or the other is built, never both**: a player
/// who chooses `spl.ao` = 1 compiles the five programs of the multi-scale port and allocates its
/// pyramid, and the two programs and two targets of the hemisphere estimator stay out of
/// existence — the same rule that keeps the occlusion and the reflections independent of each
/// other (the brief's point 2, applied one level down).
#[derive(Debug)]
enum Estimator {
    /// `spl.ao` = 2, and `spl.ao` = 1 while [`style::MSVO_PORTED`] is false.
    Depth(Depth),
    /// `spl.ao` = 1 with the port available (REQ-SCN-083 *Mode MSVO*).
    Msvo(Msvo),
}

/// The occlusion of REQ-SCN-083: whichever estimator the mode asks for, and the composite the two
/// of them share.
#[derive(Debug)]
struct Occlusion {
    composite: ShaderProgram,
    estimator: Estimator,
}

/// The reflections of REQ-SCN-084: the trace, its composite, the `R8` floor mask at the frame's
/// own size and the `RGBA16F` trace target at the quality's resolution.
///
/// The **mask program is not here**: its vertex stage is the scene's, so it lives with the other
/// scene variants in the renderer's `ScenePrograms` and is counted there.
#[derive(Debug)]
struct Reflections {
    trace: ShaderProgram,
    composite: ShaderProgram,
    mask: Option<Target>,
    /// The depth the mask framebuffer has attached, so that a reallocation of the frame's
    /// attachments reallocates the mask with them.
    mask_depth: Option<Texture>,
    traced: Option<Target>,
    /// 1 at `ssr.quality` Ultra, 2 otherwise (REQ-SCN-084: "sauf en Ultra").
    divisor: i32,
}

/// The screen-space passes and their resources, held only while a frame wants one.
#[derive(Debug)]
pub struct ScreenEffects {
    gl: Gl,
    vertex_array: VertexArray,
    occlusion: Option<Occlusion>,
    reflections: Option<Reflections>,
    /// Whether this implementation refused the multi-scale estimator of REQ-SCN-083's *Mode
    /// MSVO*, so that `spl.ao` = 1 renders as `spl.ao` = 2 from now on — the declared fallback
    /// **(D)**.
    ///
    /// Sticky for the life of the module, and for the reason RU-5 gave the whole module's own
    /// fallback: the one format the mode needs that nothing else here needs is an `R16F` colour
    /// target, so a refusal is a property of the implementation and not of the frame, and retrying
    /// it every frame would cost a failed allocation per frame for ever. It carries the reason, so
    /// that the renderer can write the one diagnostic line of the fallback.
    msvo_refused: Option<String>,
}

impl ScreenEffects {
    /// Creates the module with neither effect in it: the vertex array the full-screen triangles
    /// share, and nothing else. [`Self::select`] then builds the halves the frame asks for.
    ///
    /// # Errors
    ///
    /// [`GlError`] when the vertex array cannot be made.
    pub fn new(gl: &Gl) -> Result<Self, GlError> {
        Ok(Self {
            gl: gl.clone(),
            vertex_array: gl.create_vertex_array()?,
            occlusion: None,
            reflections: None,
            msvo_refused: None,
        })
    }

    /// Why the multi-scale estimator of REQ-SCN-083 was refused by this implementation, when it
    /// was, so that `spl.ao` = 1 renders as Depth — the fallback **(D)**. `None` on every machine
    /// whose driver answers normally.
    pub fn msvo_refused(&self) -> Option<&str> {
        self.msvo_refused.as_deref()
    }

    /// Makes the next frame that asks for the multi-scale estimator refuse it, so that the
    /// fallback above can be observed on a machine whose driver refuses nothing.
    ///
    /// Only a test calls this — the same kind of seam as `SceneRenderer::fail_screen_effects`, and
    /// for the same reason: no driver on this side refuses `R16F`, and an unreachable fallback is
    /// an untested one.
    pub fn refuse_msvo(&mut self, reason: &str) {
        self.msvo_refused = Some(reason.to_owned());
        self.release_occlusion();
    }

    /// Builds the halves `style` asks for and releases the others, once a frame.
    ///
    /// This is where point 2 of the brief is paid: a player who turns the reflections on compiles
    /// the two programs of REQ-SCN-084 and allocates its two targets, and the occlusion's three
    /// programs and two targets stay out of existence.
    ///
    /// # Errors
    ///
    /// [`GlError`] when a program does not compile or link. The caller then draws the frame
    /// without the effect rather than not at all.
    pub fn select(&mut self, style: &StyleSettings) -> Result<(), GlError> {
        if style.ao_active() {
            // A change of `spl.ao` between its two estimators releases the one that is leaving,
            // targets and all, and compiles the one that arrives — which is also how the mode
            // falls back on Depth when the multi-scale port is refused
            // ([`Self::refuse_msvo`]).
            let wanted_msvo = style.msvo_active() && self.msvo_refused.is_none();
            if self.occlusion.is_some() && self.occludes_multi_scale() != wanted_msvo {
                self.release_occlusion();
            }
            if self.occlusion.is_none() {
                // A refusal here is not a reason to draw the frame without any occlusion at all:
                // REQ-SCN-083 says `spl.ao` = 1 "DOIT être rendu exactement comme `spl.ao` = 2"
                // while the port is unavailable, and an implementation that will not give the
                // port its programs is exactly that case.
                let msvo = if wanted_msvo {
                    match Msvo::new(&self.gl) {
                        Ok(msvo) => Some(msvo),
                        Err(error) => {
                            self.msvo_refused = Some(error.to_string());
                            None
                        }
                    }
                } else {
                    None
                };
                let estimator = match msvo {
                    Some(msvo) => Estimator::Msvo(msvo),
                    None => Estimator::Depth(Depth {
                        pass: ShaderProgram::create(&self.gl, &shaders::AO)?,
                        blur: ShaderProgram::create(&self.gl, &shaders::AO_BLUR)?,
                        raw: None,
                        blurred: None,
                        divisor: 0,
                    }),
                };
                self.occlusion = Some(Occlusion {
                    composite: ShaderProgram::create(&self.gl, &shaders::AO_COMPOSITE)?,
                    estimator,
                });
            }
        } else {
            self.release_occlusion();
        }

        if style.reflections_active() {
            if self.reflections.is_none() {
                self.reflections = Some(Reflections {
                    trace: ShaderProgram::create(&self.gl, &shaders::SSR)?,
                    composite: ShaderProgram::create(&self.gl, &shaders::SSR_COMPOSITE)?,
                    mask: None,
                    mask_depth: None,
                    traced: None,
                    divisor: 0,
                });
            }
        } else if let Some(reflections) = self.reflections.take() {
            release(&self.gl, reflections.mask);
            release(&self.gl, reflections.traced);
        }

        Ok(())
    }

    /// Whether the occlusion of REQ-SCN-083 is in this module.
    pub const fn occludes(&self) -> bool {
        self.occlusion.is_some()
    }

    /// Whether the occlusion in this module is the **multi-scale** estimator of REQ-SCN-083's
    /// *Mode MSVO*, rather than the hemisphere one.
    pub const fn occludes_multi_scale(&self) -> bool {
        matches!(
            &self.occlusion,
            Some(occlusion) if matches!(occlusion.estimator, Estimator::Msvo(_))
        )
    }

    /// Whether the reflections of REQ-SCN-084 are.
    pub const fn reflects(&self) -> bool {
        self.reflections.is_some()
    }

    /// How many programs this module holds, for `RenderStatistics::style_programs`: the shared
    /// composite plus its estimator — two for the hemisphere one, five for the multi-scale one —
    /// and two for the reflections (the trace and its composite). A value and not a constant,
    /// because each effect, and each estimator, is switched on its own.
    pub fn programs(&self) -> u32 {
        let occlusion = self.occlusion.as_ref().map_or(0, |occlusion| {
            1 + match &occlusion.estimator {
                Estimator::Depth(_) => 2,
                Estimator::Msvo(_) => Msvo::PROGRAMS,
            }
        });
        occlusion + (if self.reflections.is_some() { 2 } else { 0 })
    }

    /// Bytes its targets occupy, for `RenderStatistics::screen_effect_bytes`; 0 until a pass has
    /// run once.
    pub fn bytes(&self) -> u64 {
        let occlusion = self
            .occlusion
            .as_ref()
            .map_or(0, |occlusion| match &occlusion.estimator {
                Estimator::Depth(depth) => {
                    bytes(depth.raw.as_ref(), OCCLUSION_BYTES_PER_TEXEL)
                        + bytes(depth.blurred.as_ref(), OCCLUSION_BYTES_PER_TEXEL)
                }
                Estimator::Msvo(msvo) => msvo.bytes(),
            });
        let reflections = self.reflections.as_ref().map_or(0, |reflections| {
            bytes(reflections.mask.as_ref(), OCCLUSION_BYTES_PER_TEXEL)
                + bytes(reflections.traced.as_ref(), REFLECTION_BYTES_PER_TEXEL)
        });
        occlusion + reflections
    }

    /// Levels the multi-scale pyramid holds, 0 when the estimator is the hemisphere one or has not
    /// run yet. Read by the tests of REQ-SCN-083's *Mode MSVO*, which have no other way to tell
    /// one estimator's resources from the other's.
    pub fn multi_scale_levels(&self) -> usize {
        match &self.occlusion {
            Some(Occlusion {
                estimator: Estimator::Msvo(msvo),
                ..
            }) => msvo.levels(),
            _ => 0,
        }
    }

    /// Releases the occlusion half, whichever estimator it holds.
    fn release_occlusion(&mut self) {
        if let Some(occlusion) = self.occlusion.take() {
            match occlusion.estimator {
                Estimator::Depth(depth) => {
                    release(&self.gl, depth.raw);
                    release(&self.gl, depth.blurred);
                }
                Estimator::Msvo(mut msvo) => msvo.release(&self.gl),
            }
        }
    }

    // ------------------------------------------------------- REQ-SCN-084, the reflections

    /// Binds the floor mask of REQ-SCN-084 as the draw target, with the frame's **own depth**
    /// attached and depth writes off, and clears it to 0. The caller then redraws the
    /// floor-eligible runs through the mask program; `false` when there are no reflections this
    /// frame, and the caller draws nothing.
    ///
    /// The depth attachment is what makes a floor hidden behind something not be marked, and
    /// keeping the frame's own — rather than a copy — is what makes the mask land exactly where
    /// the frame drew.
    ///
    /// # Errors
    ///
    /// [`GlError`] when the mask cannot be allocated. Nothing is drawn then.
    pub fn begin_mask(&mut self, frame: ScreenFrame) -> Result<bool, GlError> {
        let Some(reflections) = &mut self.reflections else {
            return Ok(false);
        };

        if reflections.mask_depth != Some(frame.depth)
            || !matches!(&reflections.mask, Some(mask) if mask.size == frame.capacity)
        {
            release(&self.gl, reflections.mask.take());
            reflections.mask = Some(allocate_mask(&self.gl, frame.capacity, Some(frame.depth))?);
            reflections.mask_depth = Some(frame.depth);
        }
        let Some(mask) = &reflections.mask else {
            return Ok(false);
        };

        let gl = &self.gl;
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(mask.framebuffer));
        gl.viewport(frame.drawn.0.max(1), frame.drawn.1.max(1));
        gl.disable(glow::SCISSOR_TEST);
        gl.disable(glow::BLEND);
        gl.color_mask(true);
        gl.clear_color(0.0, 0.0, 0.0, 1.0);
        gl.clear(glow::COLOR_BUFFER_BIT);
        // The frame's depth decides what is marked, and nothing may write to it: the mask is laid
        // on the surfaces the frame already drew.
        gl.enable(glow::DEPTH_TEST);
        gl.depth_func(glow::LEQUAL);
        gl.depth_mask(false);
        // And the geometry is pulled a hair towards the eye, or it loses that test to itself.
        // With anti-aliasing the depth the pass reads is a **resolved** multisampled depth — one
        // sample's, not the fragment centre's — so an exact `LEQUAL` marks nothing at all; a mask
        // that was empty on every multisampled frame is what found this. The bias is in depth
        // units of the last place, far too small to let a hidden floor through.
        gl.enable(glow::POLYGON_OFFSET_FILL);
        gl.polygon_offset(-1.0, -16.0);
        Ok(true)
    }

    /// Traces the reflected rays of REQ-SCN-084 into this module's own target, and says how many
    /// passes that took.
    ///
    /// # Errors
    ///
    /// [`GlError`] when the target cannot be allocated. Nothing is drawn then.
    pub fn trace(
        &mut self,
        frame: ScreenFrame,
        style: &StyleSettings,
        scene_scale: f32,
    ) -> Result<u32, GlError> {
        let divisor = if style.ssr_half_resolution() { 2 } else { 1 };
        let Some(reflections) = &mut self.reflections else {
            return Ok(0);
        };
        if reflections.divisor != divisor
            || !matches!(&reflections.traced, Some(target) if target.size == scaled(frame.capacity, divisor))
        {
            release(&self.gl, reflections.traced.take());
            reflections.traced = Some(allocate_reflection(
                &self.gl,
                scaled(frame.capacity, divisor),
            )?);
            reflections.divisor = divisor;
        }
        let (Some(traced), Some(mask)) = (&reflections.traced, &reflections.mask) else {
            return Ok(0);
        };

        let gl = &self.gl;
        screen_pass_state(gl);
        gl.bind_vertex_array(Some(self.vertex_array));
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(traced.framebuffer));
        gl.viewport(
            (frame.drawn.0 / divisor).max(1),
            (frame.drawn.1 / divisor).max(1),
        );
        // The trace writes every pixel of its target, so there is nothing to clear.

        let program = &mut reflections.trace;
        program.use_program();
        program.set_i32("uDepth", 0);
        program.set_i32("uColor", 1);
        program.set_i32("uMask", 2);
        program.set_vec2("uDrawnScale", frame.drawn_scale);
        program.set_vec4("uProjection", frame.projection);
        program.set_vec2("uTexel", texel(frame.drawn, divisor));
        program.set_f32("uIntensity", style.ssr_intensity());
        program.set_f32("uMaxLength", style.ssr_max_length(scene_scale));
        // The longest ray any setting can ask for, which the march's positions are fitted to.
        program.set_f32("uReferenceLength", StyleSettings::ssr_longest(scene_scale));
        program.set_f32("uBlur", style.ssr_blur_fraction());
        #[allow(clippy::cast_precision_loss)] // Sizes far below 2^24.
        program.set_vec2(
            "uBlurAspect",
            Vector2::new(
                frame.drawn.1.max(1) as f32 / frame.drawn.0.max(1) as f32,
                1.0,
            ),
        );
        program.set_i32("uSteps", style.ssr_steps());
        program.set_i32("uRefinements", style.ssr_refinements());
        program.set_i32("uBlurTaps", style.ssr_blur_taps());

        gl.active_texture(0);
        gl.bind_texture(glow::TEXTURE_2D, Some(frame.depth));
        gl.active_texture(1);
        gl.bind_texture(glow::TEXTURE_2D, Some(frame.color));
        gl.active_texture(2);
        gl.bind_texture(glow::TEXTURE_2D, Some(mask.texture));
        gl.draw_triangles(3);
        for unit in [1, 2] {
            gl.active_texture(unit);
            gl.bind_texture(glow::TEXTURE_2D, None);
        }
        Ok(1)
    }

    /// Blends the traced reflection into the bound framebuffer, which the caller has bound and
    /// given the frame's viewport again (REQ-SCN-084). One pass.
    pub fn composite_reflections(&mut self, frame: ScreenFrame) -> u32 {
        let Some(reflections) = &mut self.reflections else {
            return 0;
        };
        let (Some(traced), Some(mask)) = (&reflections.traced, &reflections.mask) else {
            return 0;
        };

        let gl = &self.gl;
        // The trace holds `rgb = w · c` and `a = w`, so `src + dst · (1 − src_a)` is the
        // `(1 − i) · surface + i · reflet` of the requirement, in place.
        gl.enable(glow::BLEND);
        gl.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);
        let program = &mut reflections.composite;
        program.use_program();
        program.set_i32("uReflection", 0);
        program.set_i32("uMask", 1);
        program.set_vec2("uDrawnScale", frame.drawn_scale);
        gl.active_texture(0);
        gl.bind_texture(glow::TEXTURE_2D, Some(traced.texture));
        gl.active_texture(1);
        gl.bind_texture(glow::TEXTURE_2D, Some(mask.texture));
        gl.draw_triangles(3);
        gl.active_texture(1);
        gl.bind_texture(glow::TEXTURE_2D, None);
        gl.active_texture(0);
        gl.disable(glow::BLEND);
        1
    }

    // ------------------------------------------------------- REQ-SCN-083, the occlusion

    /// Runs the occlusion of REQ-SCN-083 over `frame` into this module's own targets, and says how
    /// many passes that took.
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
        let radius = style.ao_radius_units(scene_scale);
        let ceiling = style.ao_ceiling();
        let samples = style.ao_samples();
        let Some(occlusion) = &mut self.occlusion else {
            return Ok(0);
        };

        // Every full-screen triangle of the two estimators draws from the same attribute-less
        // vertex array, so it is bound once here rather than inside each of them.
        self.gl.bind_vertex_array(Some(self.vertex_array));
        let depth = match &mut occlusion.estimator {
            Estimator::Msvo(msvo) => {
                // A pyramid the implementation refuses is REQ-SCN-083's fallback **(D)** and not
                // a reason to fail the frame: the mode is given up, this frame draws without the
                // occlusion, and the next `select` builds the Depth estimator in its place.
                return match msvo.draw(&self.gl, frame, style, scene_scale) {
                    Ok(passes) => Ok(passes),
                    Err(error) => {
                        self.refuse_msvo(&error.to_string());
                        Ok(0)
                    }
                };
            }
            Estimator::Depth(depth) => depth,
        };
        let size = scaled(frame.capacity, divisor);
        if depth.divisor != divisor || !matches!(&depth.raw, Some(target) if target.size == size) {
            release(&self.gl, depth.raw.take());
            release(&self.gl, depth.blurred.take());
            depth.raw = Some(allocate_occlusion(&self.gl, size, false)?);
            depth.blurred = Some(allocate_occlusion(&self.gl, size, true)?);
            depth.divisor = divisor;
        }
        // The handles are copied out before the programs are borrowed mutably: a target and the
        // program that draws into it live in the same `Depth` value.
        let (Some((raw, raw_texture)), Some(blurred)) = (
            depth
                .raw
                .as_ref()
                .map(|target| (target.framebuffer, target.texture)),
            depth.blurred.as_ref().map(|target| target.framebuffer),
        ) else {
            return Ok(0);
        };

        let gl = &self.gl;
        screen_pass_state(gl);
        let viewport = (
            (frame.drawn.0 / divisor).max(1),
            (frame.drawn.1 / divisor).max(1),
        );
        let texel = texel(frame.drawn, divisor);

        // 1. The occlusion itself, from the depth alone.
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(raw));
        gl.viewport(viewport.0, viewport.1);
        let program = &mut depth.pass;
        program.use_program();
        program.set_i32("uDepth", 0);
        program.set_vec2("uDrawnScale", frame.drawn_scale);
        program.set_vec4("uProjection", frame.projection);
        program.set_vec2("uTexel", texel);
        program.set_f32("uRadius", radius);
        program.set_f32("uCeiling", ceiling);
        // A hundredth of the radius, and never under a fiftieth of a voxel: enough to keep the
        // flattest directions of the kernel from occluding their own surface, far too little to
        // hide a contact — a voxel is one scene unit.
        program.set_f32("uBias", (0.01 * radius).max(0.02));
        program.set_i32("uSamples", samples);
        gl.active_texture(0);
        gl.bind_texture(glow::TEXTURE_2D, Some(frame.depth));
        gl.draw_triangles(3);

        // 2. The blur, which respects the depth discontinuities (REQ-SCN-083).
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(blurred));
        let program = &mut depth.blur;
        program.use_program();
        program.set_i32("uOcclusion", 0);
        program.set_i32("uDepth", 1);
        program.set_vec2("uDrawnScale", frame.drawn_scale);
        program.set_vec4("uProjection", frame.projection);
        program.set_vec2("uTexel", texel);
        gl.active_texture(0);
        gl.bind_texture(glow::TEXTURE_2D, Some(raw_texture));
        gl.active_texture(1);
        gl.bind_texture(glow::TEXTURE_2D, Some(frame.depth));
        gl.draw_triangles(3);
        gl.active_texture(1);
        gl.bind_texture(glow::TEXTURE_2D, None);
        gl.active_texture(0);

        Ok(2)
    }

    /// Multiplies the colour in the bound framebuffer by `1 − O` (REQ-SCN-083), which the caller
    /// has bound and given the frame's viewport again. One pass.
    pub fn composite_occlusion(&mut self, frame: ScreenFrame) -> u32 {
        let Some(occlusion) = &mut self.occlusion else {
            return 0;
        };
        // Both estimators write `O` into an `R8` target, so the composite reads whichever one is
        // in force and the shader is the same.
        let Some(result) = (match &occlusion.estimator {
            Estimator::Depth(depth) => depth.blurred.as_ref().map(|target| target.texture),
            Estimator::Msvo(msvo) => msvo.result(),
        }) else {
            return 0;
        };

        let gl = &self.gl;
        // `dst × src`, so the full-screen triangle scales the linear buffer in place. No second
        // colour target, no copy of the scene (REQ-SCN-083's "en espace linéaire, avant
        // l'encodage sRGB").
        gl.enable(glow::BLEND);
        gl.blend_func(glow::ZERO, glow::SRC_COLOR);
        let program = &mut occlusion.composite;
        program.use_program();
        program.set_i32("uOcclusion", 0);
        program.set_vec2("uDrawnScale", frame.drawn_scale);
        gl.active_texture(0);
        gl.bind_texture(glow::TEXTURE_2D, Some(result));
        gl.draw_triangles(3);

        gl.disable(glow::BLEND);
        1
    }

    /// Unbinds what the full-screen passes left bound. The caller calls it after the last of
    /// them, before the draws that follow rebind their own.
    pub fn end_passes(&self) {
        self.gl.disable(glow::POLYGON_OFFSET_FILL);
        self.gl.bind_vertex_array(None);
        self.gl.bind_texture(glow::TEXTURE_2D, None);
    }
}

impl Drop for ScreenEffects {
    fn drop(&mut self) {
        self.release_occlusion();
        if let Some(reflections) = self.reflections.take() {
            release(&self.gl, reflections.mask);
            release(&self.gl, reflections.traced);
        }
        self.gl.delete_vertex_array(self.vertex_array);
    }
}

/// The GL state every full-screen pass of REQ-SCN-080 draws under. `pub(super)` for
/// [`super::msvo`], whose chain draws under exactly the same state.
pub(super) fn screen_pass_state(gl: &Gl) {
    gl.disable(glow::POLYGON_OFFSET_FILL);
    gl.disable(glow::SCISSOR_TEST);
    gl.disable(glow::BLEND);
    gl.disable(glow::CULL_FACE);
    gl.disable(glow::DEPTH_TEST);
    gl.depth_mask(false);
    gl.color_mask(true);
}

/// `capacity` divided by `divisor`, never under one texel.
fn scaled(capacity: (i32, i32), divisor: i32) -> (i32, i32) {
    ((capacity.0 / divisor).max(1), (capacity.1 / divisor).max(1))
}

/// One texel of a target of `drawn / divisor`, in viewport coordinates.
fn texel(drawn: (i32, i32), divisor: i32) -> Vector2 {
    #[allow(clippy::cast_precision_loss)] // Sizes far below 2^24.
    Vector2::new(
        divisor as f32 / drawn.0.max(1) as f32,
        divisor as f32 / drawn.1.max(1) as f32,
    )
}

fn bytes(target: Option<&Target>, per_texel: u64) -> u64 {
    target.map_or(0, |target| {
        let texels =
            u64::try_from(target.size.0).unwrap_or(0) * u64::try_from(target.size.1).unwrap_or(0);
        texels * per_texel
    })
}

pub(super) fn release(gl: &Gl, target: Option<Target>) {
    if let Some(target) = target {
        gl.delete_texture(target.texture);
        gl.delete_framebuffer(target.framebuffer);
    }
}

/// One `R8` target of the occlusion. `filtered` gives the blurred one a linear filter, which is
/// what smooths the half-resolution occlusion back up to the frame's size; the raw one is read
/// texel by texel and takes `NEAREST`.
fn allocate_occlusion(gl: &Gl, size: (i32, i32), filtered: bool) -> Result<Target, GlError> {
    allocate(
        gl,
        size,
        glow::R8,
        glow::RED,
        glow::UNSIGNED_BYTE,
        filtered,
        None,
    )
}

/// The `R8` floor mask, at the frame's own size, with the frame's depth attached. `NEAREST`: the
/// mask is a decision and not a shade, and the composite reads it texel by texel on purpose.
fn allocate_mask(gl: &Gl, size: (i32, i32), depth: Option<Texture>) -> Result<Target, GlError> {
    allocate(
        gl,
        size,
        glow::R8,
        glow::RED,
        glow::UNSIGNED_BYTE,
        false,
        depth,
    )
}

/// The `RGBA16F` trace target. Filtered, because the composite samples it back up to the frame's
/// size — which is correct only because the trace writes a premultiplied value.
fn allocate_reflection(gl: &Gl, size: (i32, i32)) -> Result<Target, GlError> {
    allocate(gl, size, glow::RGBA16F, glow::RGBA, glow::FLOAT, true, None)
}

/// One colour target of `size` and its framebuffer, with `depth` attached when there is one.
/// `pub(super)` for [`super::msvo`], whose pyramid is fifteen of them.
#[allow(clippy::too_many_arguments)] // One per property of the allocation, all of them data.
pub(super) fn allocate(
    gl: &Gl,
    size: (i32, i32),
    internal: u32,
    format: u32,
    transfer: u32,
    filtered: bool,
    depth: Option<Texture>,
) -> Result<Target, GlError> {
    let (width, height) = size;
    let texture = gl.create_texture()?;
    gl.bind_texture(glow::TEXTURE_2D, Some(texture));
    gl.tex_image_2d(
        glow::TEXTURE_2D,
        internal,
        width,
        height,
        format,
        transfer,
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
    if let Some(depth) = depth {
        gl.framebuffer_texture(glow::DEPTH_ATTACHMENT, depth);
    }
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

    Ok(Target {
        framebuffer,
        texture,
        size,
    })
}
