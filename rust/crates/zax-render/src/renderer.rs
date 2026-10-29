//! The OpenGL renderer of SPEC-06: shadow map, background or skybox, opaque objects, sorted
//! transparent objects, then the HUD (REQ-SCN-058), all inside a framebuffer the host provides
//! (REQ-SCN-057). What a frame draws is worked out by [`crate::frame_plan`]; this module turns
//! the plan into GL calls.

use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use glow::Texture;
use thiserror::Error;
use zax_engine3d::draw_list::DeformParameters;
use zax_engine3d::geometry::texture::ObjectTexture;
use zax_engine3d::geometry::tile_mesh::{TileMesh, TriangleCountError};
use zax_engine3d::vectors::Mat4;

use crate::assets::SceneAssets;
use crate::camera::CameraView;
use crate::color;
use crate::frame_plan::{
    DrawItem, FixedMeshes, FramePlan, InstanceRun, Residency, SceneConstants, INSTANCE_BYTES,
    STD140_PLACEMENT,
};
use crate::geometry;
use crate::gl::api::{Gl, GlError};
use crate::gl::gpu_timer::GpuTimer;
use crate::gl::linear_buffer::LinearColorBuffer;
use crate::gl::mesh_cache::{GpuMesh, MeshCache};
use crate::gl::picture::NesPictureTexture;
use crate::gl::program::ShaderProgram;
use crate::gl::shaders;
use crate::gl::shadow_map::ShadowMap;
use crate::gl::skybox::SkyboxPass;
use crate::gl::texture_cache::TextureCache;
use crate::interop;
use crate::numerics::Vector4;
use crate::scene::{SceneFrame, SceneState};
use crate::shadow_volume::ShadowVolume;
use crate::statistics::RenderStatistics;
use crate::target::{RenderTarget, TargetError};

/// Why the renderer could not be created or could not draw a frame.
#[derive(Debug, Error)]
pub enum RenderError {
    /// The render target size is not usable.
    #[error(transparent)]
    Target(#[from] TargetError),
    /// A GL object could not be created, compiled, linked or completed.
    #[error(transparent)]
    Gl(#[from] GlError),
    /// One of the renderer's fixed meshes is malformed.
    #[error("fixed mesh: {0}")]
    Mesh(#[from] TriangleCountError),
}

/// A draw item of the GL renderer: its mesh is a vertex array, its texture a GL texture.
type GlDrawItem = DrawItem<GpuMesh, Texture>;

/// The uniform buffer binding point of the instance block.
const INSTANCE_BINDING: u32 = 0;

/// The GL caches the plan asks for the resident meshes and textures.
struct GlResidency<'a> {
    meshes: &'a mut MeshCache,
    textures: &'a mut TextureCache,
    picture: &'a mut NesPictureTexture,
}

impl Residency for GlResidency<'_> {
    type Mesh = GpuMesh;
    type Texture = Texture;
    type Error = GlError;

    fn mesh(&mut self, mesh: &Arc<TileMesh>) -> Result<Option<GpuMesh>, GlError> {
        self.meshes.get(mesh)
    }

    fn texture(&mut self, texture: &Arc<ObjectTexture>) -> Result<Texture, GlError> {
        self.textures.get(texture)
    }

    fn picture(&mut self, picture: &[u8], height: usize) -> Result<Option<Texture>, GlError> {
        self.picture.upload(picture, height)
    }
}

/// The programs and the cached GL state the draw calls go through, and what they cost.
#[derive(Debug)]
struct Passes {
    gl: Gl,
    scene: ShaderProgram,
    shadow: ShaderProgram,
    dummy_indexed: Texture,
    dummy_color: Texture,
    shadow_enabled: bool,
    mirrored_winding: bool,
    mirrored_normal: bool,
    /// The textures [`Passes::draw_run`] left on units 0 and 1, and the vertex array
    /// [`Passes::draw`] left bound, so that a draw only rebinds what changed. Forgotten at the
    /// start of each pass ([`Passes::forget_bindings`]): other code binds in between.
    bound_textures: [Option<Texture>; 2],
    bound_vertex_array: Option<glow::VertexArray>,
    /// The placements of every scene draw of the frame, uploaded once before the scene pass.
    instance_buffer: glow::Buffer,
    /// `GL_UNIFORM_BUFFER_OFFSET_ALIGNMENT`: each run starts at a multiple of it.
    instance_alignment: usize,
    statistics: RenderStatistics,
}

/// The OpenGL 3.3 core / GLES 3.0 renderer of SPEC-06.
///
/// Not thread-safe, and bound to the GL context that was current when it was created; the host
/// keeps that context current while it uses or drops the renderer. The renderer binds the
/// target framebuffer itself and leaves it bound on return; the host rebinds whatever it needs
/// afterwards. No process-wide state is kept, so several independent instances can coexist
/// (REQ-SCN-057, SPEC-01 REQ-ARCH-003). The skyboxes of REQ-SCN-050 and REQ-SCN-051 are
/// procedural (AMD-024 REQ-PUB-003) unless the host supplies scene assets.
#[derive(Debug)]
pub struct SceneRenderer {
    gl: Gl,
    meshes: MeshCache,
    textures: TextureCache,
    shadow_map: ShadowMap,
    skybox: SkyboxPass,
    picture: NesPictureTexture,
    linear: LinearColorBuffer,
    unit_box: Arc<TileMesh>,
    picture_quad: Arc<TileMesh>,
    /// The quad of the last widescreen 2D-mode picture, by margin.
    wide_picture_quad: Option<interop::WidePictureQuad>,
    plan: FramePlan<GpuMesh, Texture>,
    passes: Passes,
    last_view: CameraView,
    /// Whether the opaque bucket and the tiles of each transparent object are reordered by draw
    /// state (`FramePlan::lay_out_instances`).
    group_opaque: bool,
    /// The GPU time of the scene passes, when the context can measure it.
    timer: Option<GpuTimer>,
}

impl SceneRenderer {
    /// Creates the renderer for the host context `gl`, which must be current. `assets` are
    /// where the skybox cubemaps of a user-supplied folder come from, or `None` when the host
    /// has none (REQ-SCN-057: the renderer reads no file of its own, but the program cache
    /// of [`Self::with_program_cache`], whose folder the host names).
    ///
    /// # Errors
    ///
    /// [`RenderError::Gl`] when a program does not compile or link, or no shadow map size
    /// gives a complete framebuffer.
    pub fn new(
        gl: Rc<glow::Context>,
        assets: Option<Box<dyn SceneAssets>>,
    ) -> Result<Self, RenderError> {
        Self::with_program_cache(gl, assets, None)
    }

    /// Like [`Self::new`], keeping the binaries of the linked programs in `cache_directory` when
    /// there is one and the driver can give them: the next start loads them instead of
    /// compiling. A binary the driver refuses is compiled from source.
    ///
    /// # Errors
    ///
    /// As [`Self::new`].
    pub fn with_program_cache(
        gl: Rc<glow::Context>,
        assets: Option<Box<dyn SceneAssets>>,
        cache_directory: Option<PathBuf>,
    ) -> Result<Self, RenderError> {
        let gl = Gl::new(gl);
        let gl = match cache_directory {
            Some(directory) => gl.with_program_cache(directory),
            None => gl,
        };
        let textures = TextureCache::new(&gl)?;
        let shadow_map = ShadowMap::new(&gl, geometry::SHADOW_MAP_SIZE)?;

        // AMD-034 §2: the size the hardware granted is known from here on, whether or not a
        // shadow pass ever runs, so the host can report a fallback once.
        let statistics = RenderStatistics {
            shadow_map_size: shadow_map.size(),
            requested_shadow_map_size: shadow_map.requested(),
            ..RenderStatistics::default()
        };

        let scene_program = ShaderProgram::create(&gl, &shaders::SCENE)?;
        scene_program.bind_uniform_block("ZaxInstances", INSTANCE_BINDING);
        let instance_alignment =
            usize::try_from(gl.get_integer(glow::UNIFORM_BUFFER_OFFSET_ALIGNMENT))
                .unwrap_or(256)
                .max(1);
        let passes = Passes {
            gl: gl.clone(),
            scene: scene_program,
            shadow: ShaderProgram::create(&gl, &shaders::SHADOW)?,
            dummy_indexed: textures.dummy_indexed(),
            dummy_color: textures.dummy_color(),
            shadow_enabled: false,
            mirrored_winding: false,
            mirrored_normal: false,
            bound_textures: [None; 2],
            bound_vertex_array: None,
            instance_buffer: gl.create_buffer()?,
            instance_alignment,
            statistics,
        };

        Ok(Self {
            meshes: MeshCache::new(&gl),
            skybox: SkyboxPass::new(&gl, assets),
            picture: NesPictureTexture::new(&gl),
            linear: LinearColorBuffer::new(&gl)?,
            unit_box: Arc::new(interop::unit_box()?),
            picture_quad: Arc::new(interop::picture_quad()?),
            wide_picture_quad: None,
            plan: FramePlan::default(),
            last_view: CameraView::default(),
            group_opaque: false,
            timer: None,
            textures,
            shadow_map,
            passes,
            gl,
        })
    }

    /// Reorders the opaque tiles by draw state before drawing them (Rust-only setting, off by
    /// default): far fewer draw calls on a scene that repeats a few meshes, but where two
    /// different opaque surfaces sit at exactly the same depth, the one drawn first is no longer
    /// the same, so a few edge pixels may change.
    pub fn set_group_opaque_draws(&mut self, group: bool) {
        self.group_opaque = group;
    }

    /// Turns the GPU timing of the scene passes on or off (`RenderStatistics::gpu_time`, the
    /// measure of the dynamic resolution). Returns whether the context times GPU work.
    pub fn set_gpu_timing(&mut self, enabled: bool) -> bool {
        if enabled {
            if self.timer.is_none() {
                self.timer = GpuTimer::new(&self.gl);
            }
        } else if let Some(timer) = self.timer.take() {
            timer.delete(&self.gl);
        }
        self.timer.is_some()
    }

    /// Keeps the renderer's own buffers at least `width` × `height`, whatever size the target
    /// has: a host whose target size changes often (the automatic resolution) avoids a
    /// reallocation at each change. `(0, 0)` gives the room back.
    pub const fn reserve_target(&mut self, width: i32, height: i32) {
        self.linear.reserve(width, height);
    }

    /// What the last rendered frame cost (REQ-SCN-064).
    #[must_use]
    pub const fn statistics(&self) -> &RenderStatistics {
        &self.passes.statistics
    }

    /// The camera view the last rendered frame used.
    #[must_use]
    pub const fn last_view(&self) -> &CameraView {
        &self.last_view
    }

    /// Draws one frame of `scene` into `target`: settings, camera, light and clip margins come
    /// from `scene`, the tile list and the pixel picture of the displayed frame from `frame`
    /// (SPEC-04B REQ-E3D-641).
    ///
    /// # Errors
    ///
    /// [`RenderError::Target`] when the target size is not positive, [`RenderError::Gl`] when
    /// a GL object cannot be created.
    pub fn render(
        &mut self,
        target: &RenderTarget,
        scene: &SceneState,
        frame: &SceneFrame<'_>,
    ) -> Result<(), RenderError> {
        target.validate()?;

        self.passes.statistics.begin_frame();
        self.meshes.begin_frame();
        self.textures.begin_frame();

        #[allow(clippy::cast_precision_loss)] // Target sizes are far below 2^24.
        let aspect = target.width as f32 / target.height as f32;
        let view = if scene.two_dimensional_mode && scene.picture_head_on {
            scene.camera.picture_view(
                scene.picture_clipping(),
                aspect,
                crate::numerics::Vector3::new(scene.picture_tilt[0], scene.picture_tilt[1], 0.0),
            )
        } else {
            scene.camera.build_view(scene.view_clipping(), aspect)
        };
        self.last_view = view;

        let started = std::time::Instant::now();
        let mut residency = GlResidency {
            meshes: &mut self.meshes,
            textures: &mut self.textures,
            picture: &mut self.picture,
        };
        self.wide_picture_quad = interop::wide_quad_for(
            frame.nes_picture,
            frame.nes_picture_height,
            self.wide_picture_quad.take(),
        )?;
        let fixed = FixedMeshes {
            unit_box: &self.unit_box,
            picture_quad: &self.picture_quad,
            wide_picture_quad: self.wide_picture_quad.as_ref().map(|(_, quad)| quad),
        };
        let collected = self
            .plan
            .collect(&mut residency, fixed, scene, frame, &view);
        self.passes.statistics.occluders = self.plan.occluders;
        collected?;
        self.passes.statistics.collect_time = started.elapsed();
        if let Some(timer) = &mut self.timer {
            self.passes.statistics.gpu_time = timer.collect(&self.gl);
            timer.begin(&self.gl);
        }
        let volume = self.render_shadow_map(scene, &view)?;
        self.render_scene(target, scene, frame, &view, &volume)?;
        if let Some(timer) = &mut self.timer {
            timer.end(&self.gl);
        }

        let statistics = &mut self.passes.statistics;
        statistics.resident_meshes = count(self.meshes.count());
        statistics.mesh_uploads = self.meshes.uploads();
        statistics.resident_textures = count(self.textures.count());
        self.meshes.collect();
        self.textures.collect();
        Ok(())
    }

    /// Drops every cached GPU mesh and texture. The host calls it when the 3D engine rebuilds
    /// its geometry wholesale, for instance after a change of mesh algorithm (REQ-E3D-340).
    pub fn invalidate_caches(&mut self) {
        self.meshes.clear();
        self.textures.clear();
    }

    // ----------------------------------------------------------------- shadow pass

    fn render_shadow_map(
        &mut self,
        scene: &SceneState,
        view: &CameraView,
    ) -> Result<ShadowVolume, GlError> {
        let casters = &self.plan.lists.casters;
        if !scene.settings.shadows || casters.is_empty() {
            return Ok(ShadowVolume::EMPTY);
        }

        self.shadow_map.resize(scene.settings.shadow_map_size())?;
        let passes = &mut self.passes;
        passes.statistics.shadow_map_size = self.shadow_map.size();
        passes.statistics.requested_shadow_map_size = self.shadow_map.requested();

        let volume = self.plan.shadow_volume(scene, view);
        if !volume.has_content {
            return Ok(ShadowVolume::EMPTY);
        }

        self.shadow_map.begin_pass();
        let gl = &self.gl;
        gl.enable(glow::DEPTH_TEST);
        gl.depth_func(glow::LESS);
        gl.depth_mask(true);
        gl.disable(glow::BLEND);

        // Q-025: only the back faces of the casters fill the map. The tile meshes are closed
        // (REQ-E3D-505), so the stored depth is the far side of each object and a lit front
        // face can no longer shadow itself: over a whole scene one texel still spans more than
        // the 0.02 unit bias of REQ-SCN-031 — about 0.1 unit on the 4096-texel map of
        // AMD-034 §2 — and front faces produce visible shadow acne on every slanted surface.
        // The bias and the normal offset of REQ-SCN-031 are still applied on top.
        gl.enable(glow::CULL_FACE);
        gl.cull_face(glow::FRONT);
        passes.set_winding(false, true);

        passes.shadow.use_program();
        passes.forget_bindings();
        passes
            .shadow
            .set_matrix4("uLightViewProjection", &volume.view_projection);
        passes.shadow.set_f32("uTime", scene.emulated_time);

        for caster in casters {
            // Q-069: a mirrored caster winds the other way round, so the far side of the solid
            // is only selected once the winding is corrected.
            passes.set_winding(interop::determinant(&caster.model) < 0.0, false);
            passes
                .shadow
                .set_matrix4("uModel", &interop::to_numerics(&caster.model));
            set_deformation(&mut passes.shadow, caster.deform.as_ref());
            passes.draw(caster.mesh);
        }

        passes.set_winding(false, false);
        passes.statistics.shadow_casters = count(casters.len());
        passes.statistics.shadow_pass = true;
        Ok(volume)
    }

    // ----------------------------------------------------------------- scene passes

    fn render_scene(
        &mut self,
        target: &RenderTarget,
        scene: &SceneState,
        frame: &SceneFrame<'_>,
        view: &CameraView,
        volume: &ShadowVolume,
    ) -> Result<(), GlError> {
        // AMD-021 R2-009: the scene is drawn into the renderer's own linear buffer, so that the
        // blending of the transparent objects and of the HUD happens on linear colours, and the
        // whole image is encoded to sRGB once, on the way into the host framebuffer.
        // AMD-032 §2: that buffer is multisampled when anti-aliasing is on, and resolved before
        // the sRGB pass reads it. The shadow map keeps its own single-sample target.
        self.linear.begin_pass(
            target.width,
            target.height,
            scene.settings.anti_alias_samples(),
        )?;
        self.passes.statistics.anti_alias_samples = self.linear.samples();
        let gl = &self.gl;
        gl.disable(glow::SCISSOR_TEST);
        gl.disable(glow::BLEND);
        gl.enable(glow::DEPTH_TEST);
        gl.depth_func(glow::LESS);
        gl.depth_mask(true);
        gl.color_mask(true);

        // REQ-SCN-046: the image is cleared with the background colour, without any lighting.
        let background = color::rgb_to_linear(scene.background_color());
        gl.clear_color(background.x, background.y, background.z, 1.0);
        gl.clear_depth(1.0);
        gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

        // REQ-SCN-058: the skybox replaces the clear, behind everything else. It is drawn even
        // when the game is not running, because only the 3D scene is hidden then
        // (REQ-SCN-047).
        self.passes.statistics.skybox_pass = self.skybox.draw(scene, frame, view)?;

        if self.plan.lists.nothing_to_draw() {
            self.linear.resolve_to(target);
            return Ok(());
        }

        self.plan.lay_out_instances(
            self.group_opaque,
            self.passes.instance_alignment,
            STD140_PLACEMENT,
        );
        let passes = &mut self.passes;
        passes.upload_instances(&self.plan.instance_data);
        let lists = &self.plan.lists;
        if scene.settings.cull_back_faces {
            // The scene frame is left-handed and the clip position mirrors z, so the faces the
            // engine winds per REQ-E3D-505 come out clockwise (REQ-SCN-001 notes).
            gl.enable(glow::CULL_FACE);
            gl.cull_face(glow::BACK);
            passes.set_winding(false, true);
        } else {
            gl.disable(glow::CULL_FACE);
        }

        let mut common = CommonUniforms {
            constants: SceneConstants::new(scene, view, volume, self.shadow_map.size()),
            frame,
            textures: &mut self.textures,
            shadow_texture: self.shadow_map.depth_texture(),
        };

        passes.scene.use_program();
        common.bind(passes);

        // Q-069: the sign lives in the program, which starts at 0 — it has to be written at
        // least once per frame before anything is drawn.
        passes.set_normal_sign(false, true);

        // 1. Opaque objects.
        passes
            .scene
            .set_vec3("uLightDirection", common.constants.light_direction);
        for run in &lists.opaque_runs {
            passes.draw_run(&lists.opaque, run, true);
        }
        passes.statistics.opaque_tiles = count(lists.opaque.len());

        // The tiles may have left the winding reversed (Q-069).
        passes.set_winding(false, false);

        // 2. Transparent objects, farthest first, each with a depth-only pass first
        // (REQ-SCN-040, AMD-012).
        if !lists.transparent.is_empty() {
            passes.scene.use_program();
            common.bind(passes);
            passes
                .scene
                .set_vec3("uLightDirection", common.constants.light_direction);
            gl.enable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            for group in &lists.transparent_objects {
                passes.draw_blended(
                    &lists.transparent,
                    &lists.transparent_runs[group.runs.0..group.runs.1],
                );
            }

            gl.disable(glow::BLEND);
            passes.statistics.transparent_tiles = count(lists.transparent.len());
            passes.statistics.transparent_objects = count(lists.transparent_objects.len());
        }

        // 3. HUD, after the transparent objects, lit in the camera frame (REQ-SCN-029,
        // REQ-SCN-044).
        if !lists.hud.is_empty() {
            passes.scene.use_program();
            common.bind(passes);
            passes
                .scene
                .set_vec3("uLightDirection", common.constants.hud_light_direction);
            gl.enable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            for group in &lists.hud_objects {
                passes.draw_blended(&lists.hud, &lists.hud_runs[group.runs.0..group.runs.1]);
            }

            gl.disable(glow::BLEND);
            passes.statistics.hud_tiles = count(lists.hud.len());
        }

        gl.depth_mask(true);
        gl.depth_func(glow::LESS);
        passes.set_winding(false, false);
        self.linear.resolve_to(target);
        Ok(())
    }
}

/// The uniforms every scene sub-pass binds again after `use_program`.
struct CommonUniforms<'a> {
    constants: SceneConstants,
    frame: &'a SceneFrame<'a>,
    textures: &'a mut TextureCache,
    shadow_texture: Texture,
}

impl CommonUniforms<'_> {
    fn bind(&mut self, passes: &mut Passes) {
        let (gl, program) = (&passes.gl, &mut passes.scene);
        let constants = &self.constants;

        // The palette is refreshed first: the upload binds its own texture, so it has to happen
        // before the sampler units are set up (REQ-E3D-565).
        gl.active_texture(0);
        if let Some(list) = self.frame.draw_list {
            self.textures.upload_palette(list.palette());
        }

        program.set_matrix4("uViewProjection", &constants.view_projection);
        program.set_f32("uTime", constants.time);
        program.set_f32("uAmbient", constants.ambient);
        program.set_f32("uShadowStrength", constants.shadow_strength);
        program.set_vec3("uLightLinear", constants.light_linear);
        program.set_vec4("uClipRect", constants.clip_rect);
        program.set_matrix4("uLightViewProjection", &constants.light_view_projection);
        program.set_f32("uShadowBias", constants.shadow_bias);
        program.set_f32("uNormalOffset", constants.normal_offset);
        passes.shadow_enabled = constants.shadow_enabled;

        program.set_i32("uIndexTexture", 0);
        program.set_i32("uColorTexture", 1);
        program.set_i32("uPalette", 2);
        program.set_i32("uShadowMap", 3);

        gl.active_texture(2);
        gl.bind_texture(glow::TEXTURE_2D, Some(self.textures.palette()));
        gl.active_texture(3);
        gl.bind_texture(glow::TEXTURE_2D, Some(self.shadow_texture));
        passes.forget_bindings();
    }
}

impl Passes {
    /// Selects the winding the rasterizer takes for front faces: clockwise for an ordinary
    /// placement, counter-clockwise for a mirrored one (Q-069). `force` issues the call even
    /// when the cached state already matches, which is what the entry of a pass does: another
    /// pass of the host context may have changed it.
    ///
    /// A placement whose determinant is negative — the profile of "Zelda II" scales its ground
    /// slab by −1 on z — reverses the order of every triangle in screen space. Left alone, the
    /// cull test then keeps exactly the faces REQ-E3D-505 oriented away from the eye: the solid
    /// renders inside-out. Reversing the winding for that object restores its surface.
    fn set_winding(&mut self, mirrored: bool, force: bool) {
        if mirrored == self.mirrored_winding && !force {
            return;
        }

        self.gl
            .front_face(if mirrored { glow::CCW } else { glow::CW });
        self.mirrored_winding = mirrored;
    }

    /// Sets the sign the scene shader applies to the cofactor normal: −1 for a mirrored
    /// placement, which would otherwise point the normal inwards (Q-069).
    fn set_normal_sign(&mut self, mirrored: bool, force: bool) {
        if mirrored == self.mirrored_normal && !force {
            return;
        }

        self.scene
            .set_f32("uNormalSign", if mirrored { -1.0 } else { 1.0 });
        self.mirrored_normal = mirrored;
    }

    /// Uploads the placements the plan laid out, in one call.
    fn upload_instances(&mut self, data: &[u8]) {
        if !data.is_empty() {
            self.gl
                .bind_buffer(glow::UNIFORM_BUFFER, Some(self.instance_buffer));
            self.gl.buffer_stream_data(glow::UNIFORM_BUFFER, data);
        }
    }

    /// Draws the items of `run` (a run of `items`) with one instanced call: the state of its
    /// first item, which every item of the run shares, and the placements of the run.
    fn draw_run(&mut self, items: &[GlDrawItem], run: &InstanceRun, opaque: bool) {
        let item = &items[run.first];
        let (_, determinant) = interop::normal_matrix(&item.model);

        // Q-069: a placement with a negative determinant mirrors the object. Its triangles come
        // out the other way round and its cofactor normal points inwards, so both are corrected
        // here; otherwise the solid renders inside-out. Almost no object is mirrored, so both
        // corrections are cached and cost nothing on an ordinary scene.
        let mirrored = determinant < 0.0;
        self.set_normal_sign(mirrored, false);
        self.set_winding(mirrored, false);

        let program = &mut self.scene;
        program.set_i32("uColorMode", item.color_mode);
        program.set_i32("uPaletteTable", item.palette_table);
        program.set_vec2("uTextureSize", item.texture_size);
        program.set_vec2("uTexOffset", item.tex_offset);
        program.set_vec2("uTexTilting", item.tex_tilting);
        program.set_vec3("uFlatColor", item.flat_color);
        program.set_f32("uAlpha", if opaque { 1.0 } else { item.alpha });
        program.set_i32("uClip", i32::from(item.clip));
        program.set_i32(
            "uReceiveShadow",
            i32::from(self.shadow_enabled && item.receive_shadow),
        );
        set_deformation(program, item.deform.as_ref());

        let texture = |mode: i32, dummy: Texture| match item.texture {
            Some(texture) if item.color_mode == mode => texture,
            _ => dummy,
        };
        let textures = [texture(0, self.dummy_indexed), texture(1, self.dummy_color)];
        for (unit, texture) in (0u32..).zip(textures) {
            let slot = &mut self.bound_textures[unit as usize];
            if *slot != Some(texture) {
                self.gl.active_texture(unit);
                self.gl.bind_texture(glow::TEXTURE_2D, Some(texture));
                *slot = Some(texture);
            }
        }

        let instances = run.count;
        self.gl.bind_buffer_range(
            glow::UNIFORM_BUFFER,
            INSTANCE_BINDING,
            self.instance_buffer,
            gl_size(run.offset),
            gl_size(instances * INSTANCE_BYTES),
        );
        self.bind_mesh(item.mesh);
        self.gl
            .draw_triangles_indexed_instanced(item.mesh.index_count, gl_size(instances));
        self.statistics.batches += 1;
        self.statistics.draw_calls += count(instances);
        self.statistics.triangles +=
            u64::from(item.mesh.index_count.unsigned_abs() / 3) * instances as u64;
    }

    /// Forgets which textures and vertex array the draws left bound: the next draw binds its
    /// own again. Called at the start of each pass, after the code that binds other objects.
    fn forget_bindings(&mut self) {
        self.bound_textures = [None; 2];
        self.bound_vertex_array = None;
    }

    /// Draws one blended object with the two passes of REQ-SCN-040: depth only, then colour
    /// where the depth is equal, so that only the nearest surface of the object is visible.
    ///
    /// "An object" is a whole shape instance, every tile of it at once (AMD-012, revising
    /// Q-020): the first pass lays down the depth of all its tiles before any of them is
    /// blended, so the faces that close each tile mesh on itself (SPEC-04B REQ-E3D-505) at the
    /// joints inside the object are never drawn.
    fn draw_blended(&mut self, items: &[GlDrawItem], runs: &[InstanceRun]) {
        self.gl.color_mask(false);
        self.gl.depth_mask(true);
        self.gl.depth_func(glow::LESS);
        for run in runs {
            self.draw_run(items, run, false);
        }

        self.gl.color_mask(true);
        self.gl.depth_mask(false);
        self.gl.depth_func(glow::EQUAL);
        for run in runs {
            self.draw_run(items, run, false);
        }

        self.gl.depth_mask(true);
        self.gl.depth_func(glow::LESS);
    }

    fn bind_mesh(&mut self, mesh: GpuMesh) {
        if self.bound_vertex_array != Some(mesh.vertex_array) {
            self.gl.bind_vertex_array(Some(mesh.vertex_array));
            self.bound_vertex_array = Some(mesh.vertex_array);
        }
    }

    fn draw(&mut self, mesh: GpuMesh) {
        self.bind_mesh(mesh);
        self.gl.draw_triangles_indexed(mesh.index_count);
        self.statistics.batches += 1;
        self.statistics.draw_calls += 1;
        self.statistics.triangles += u64::from(mesh.index_count.unsigned_abs() / 3);
    }
}

fn set_deformation(program: &mut ShaderProgram, deform: Option<&DeformParameters>) {
    const SPEED: [&str; 3] = ["uDeformSpeed[0]", "uDeformSpeed[1]", "uDeformSpeed[2]"];
    const PHASE: [&str; 3] = ["uDeformPhase[0]", "uDeformPhase[1]", "uDeformPhase[2]"];

    let Some(parameters) = deform else {
        program.set_i32("uDeform", 0);
        return;
    };

    program.set_i32("uDeform", 1);
    program.set_vec3("uDeformAmplitude", parameters.amplitude.into());
    program.set_vec3("uDeformSize", parameters.size.into());
    program.set_vec3("uDeformTile", parameters.tile_relative_to_pivot.into());
    for row in 0..3 {
        program.set_vec4(SPEED[row], matrix_row(&parameters.speed, row));
        program.set_vec4(PHASE[row], matrix_row(&parameters.phase, row));
    }
}

fn matrix_row(matrix: &Mat4, row: usize) -> Vector4 {
    Vector4::new(
        matrix[(row, 0)],
        matrix[(row, 1)],
        matrix[(row, 2)],
        matrix[(row, 3)],
    )
}

/// A size or offset for a GL call (the instance buffer stays far below `i32::MAX`).
fn gl_size(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

/// A count for the statistics, saturated (no frame comes near `u32::MAX` items).
fn count(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}
