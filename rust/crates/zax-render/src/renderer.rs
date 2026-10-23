//! The OpenGL renderer of SPEC-06: shadow map, background or skybox, opaque objects, sorted
//! transparent objects, then the HUD (REQ-SCN-058), all inside a framebuffer the host provides
//! (REQ-SCN-057).

use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use glow::Texture;
use thiserror::Error;
use zax_engine3d::draw_list::{DeformParameters, DrawTile, MaterialClass, SceneContainer};
use zax_engine3d::frame::FrameInstanceId;
use zax_engine3d::geometry::tile_mesh::{TileMesh, TriangleCountError};
use zax_engine3d::vectors::{Mat4, Vec3};
use zax_profiles::enums::TextureFormat;

use crate::assets::SceneAssets;
use crate::camera::CameraView;
use crate::color;
use crate::geometry;
use crate::gl::api::{Gl, GlError};
use crate::gl::linear_buffer::LinearColorBuffer;
use crate::gl::mesh_cache::{GpuMesh, MeshCache};
use crate::gl::picture::{self, NesPictureTexture};
use crate::gl::program::ShaderProgram;
use crate::gl::shaders;
use crate::gl::shadow_map::ShadowMap;
use crate::gl::skybox::SkyboxPass;
use crate::gl::texture_cache::TextureCache;
use crate::interop;
use crate::numerics::{Vector2, Vector3, Vector4};
use crate::occlusion::{self, Occlusion};
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

/// One tile, backdrop or picture quad ready to draw.
#[derive(Debug, Clone, Copy)]
struct DrawItem {
    mesh: GpuMesh,
    model: Mat4,
    /// 0 indexed texture, 1 direct texture, 2 flat colour.
    color_mode: i32,
    texture: Option<Texture>,
    texture_size: Vector2,
    palette_table: i32,
    tex_offset: Vector2,
    tex_tilting: Vector2,
    flat_color: Vector3,
    alpha: f32,
    receive_shadow: bool,
    clip: bool,
    deform: Option<DeformParameters>,
    /// The shape instance the tile belongs to (SPEC-04B REQ-E3D-641), the object the depth pass
    /// and the sort of AMD-012 work on. `None` for the backdrop and the 2D-mode quad, which are
    /// single objects anyway.
    instance: Option<FrameInstanceId>,
    /// Local bounding box of the mesh (REQ-E3D-640).
    bounds_min: Vec3,
    bounds_max: Vec3,
}

impl DrawItem {
    /// A flat or textured item with the neutral texture parameters.
    fn plain(mesh: GpuMesh, model: Mat4, bounds: (Vec3, Vec3)) -> Self {
        Self {
            mesh,
            model,
            color_mode: 0,
            texture: None,
            texture_size: Vector2::ONE,
            palette_table: 0,
            tex_offset: Vector2::default(),
            tex_tilting: Vector2::ONE,
            flat_color: Vector3::ZERO,
            alpha: 1.0,
            receive_shadow: false,
            clip: false,
            deform: None,
            instance: None,
            bounds_min: bounds.0,
            bounds_max: bounds.1,
        }
    }
}

/// One blended object of AMD-012: a run of a bucket and the distance from the eye to the centre
/// of its transformed bounding box.
#[derive(Debug, Clone, Copy)]
struct DrawGroup {
    start: usize,
    end: usize,
    view_depth: f32,
}

/// The four buckets of REQ-SCN-058 and the objects of the two blended ones.
#[derive(Debug, Default)]
struct DrawLists {
    opaque: Vec<DrawItem>,
    transparent: Vec<DrawItem>,
    hud: Vec<DrawItem>,
    casters: Vec<DrawItem>,
    transparent_objects: Vec<DrawGroup>,
    hud_objects: Vec<DrawGroup>,
}

impl DrawLists {
    fn clear(&mut self) {
        self.opaque.clear();
        self.transparent.clear();
        self.hud.clear();
        self.casters.clear();
        self.transparent_objects.clear();
        self.hud_objects.clear();
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
    /// The textures [`Passes::draw_item`] left on units 0 and 1, and the vertex array
    /// [`Passes::draw`] left bound, so that a draw only rebinds what changed. Forgotten at the
    /// start of each pass ([`Passes::forget_bindings`]): other code binds in between.
    bound_textures: [Option<Texture>; 2],
    bound_vertex_array: Option<glow::VertexArray>,
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
    lists: DrawLists,
    passes: Passes,
    last_view: CameraView,
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

        let passes = Passes {
            gl: gl.clone(),
            scene: ShaderProgram::create(&gl, &shaders::SCENE)?,
            shadow: ShaderProgram::create(&gl, &shaders::SHADOW)?,
            dummy_indexed: textures.dummy_indexed(),
            dummy_color: textures.dummy_color(),
            shadow_enabled: false,
            mirrored_winding: false,
            mirrored_normal: false,
            bound_textures: [None; 2],
            bound_vertex_array: None,
            statistics,
        };

        Ok(Self {
            meshes: MeshCache::new(&gl),
            skybox: SkyboxPass::new(&gl, assets),
            picture: NesPictureTexture::new(&gl),
            linear: LinearColorBuffer::new(&gl)?,
            unit_box: Arc::new(interop::unit_box()?),
            picture_quad: Arc::new(interop::picture_quad()?),
            lists: DrawLists::default(),
            last_view: CameraView::default(),
            textures,
            shadow_map,
            passes,
            gl,
        })
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
        let view = scene.camera.build_view(scene.clipping, aspect);
        self.last_view = view;

        self.collect(scene, frame, &view)?;
        let volume = self.render_shadow_map(scene, &view)?;
        self.render_scene(target, scene, frame, &view, &volume)?;

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

    // ----------------------------------------------------------------- collection

    /// Turns the tile list of REQ-E3D-641 into the four buckets of REQ-SCN-058, applying the
    /// renderer-side sprite nudge of REQ-SCN-037 and the material rules of REQ-SCN-040/041.
    fn collect(
        &mut self,
        scene: &SceneState,
        frame: &SceneFrame<'_>,
        view: &CameraView,
    ) -> Result<(), GlError> {
        self.lists.clear();

        // REQ-SCN-047: outside a running game the 3D scene is not displayed at all.
        if !scene.game_running {
            return Ok(());
        }

        // AMD-009 (Q-022): in 2D mode the core's pixel frame is shown on the NES plane instead
        // of a recomposition of the tile records.
        if scene.two_dimensional_mode && self.collect_picture_quad(frame.nes_picture)? {
            return Ok(());
        }

        if let Some(list) = frame.draw_list {
            let hud_matrix = interop::hud_matrix(view);
            let occlusion = Occlusion::for_scene(scene, view);
            for tile in list.tiles() {
                self.collect_tile(tile, &hud_matrix, &occlusion)?;
            }
        }

        if scene.backdrop.enabled {
            self.collect_backdrop(scene)?;
        }

        // AMD-012 (Q-020 revised): the depth pass and the sort of the transparent objects are
        // per whole object, so the faces between the tiles of one translucent object stay
        // invisible. REQ-SCN-040: they are drawn after the opaque ones, farthest first.
        group_by_object(
            &self.lists.transparent,
            &mut self.lists.transparent_objects,
            view.eye,
        );
        self.lists
            .transparent_objects
            .sort_by(|a, b| b.view_depth.total_cmp(&a.view_depth));

        // REQ-SCN-044 blends the HUD "as in REQ-SCN-040", so its objects get the same
        // treatment; their order stays the list order, since the HUD is not sorted by distance.
        group_by_object(&self.lists.hud, &mut self.lists.hud_objects, view.eye);
        Ok(())
    }

    /// The textured quad of the 2D mode (AMD-009): the core's 256 × 240 picture mapped on the
    /// NES plane of REQ-E3D-410, shaded like any other front-facing tile. False when the
    /// picture has the wrong size.
    fn collect_picture_quad(&mut self, picture: &[u8]) -> Result<bool, GlError> {
        let Some(texture) = self.picture.upload(picture)? else {
            return Ok(false);
        };
        let Some(mesh) = self.meshes.get(&self.picture_quad)? else {
            return Ok(false);
        };

        #[allow(clippy::cast_precision_loss)] // 256 and 240.
        let size = Vector2::new(picture::WIDTH as f32, picture::HEIGHT as f32);
        let quad = &self.picture_quad;
        self.lists.opaque.push(DrawItem {
            color_mode: 1,
            texture: Some(texture),
            texture_size: size,
            // Nothing else is in the scene in 2D mode, so the quad neither casts nor receives.
            receive_shadow: false,
            clip: true,
            ..DrawItem::plain(mesh, Mat4::IDENTITY, (quad.bounds_min(), quad.bounds_max()))
        });
        Ok(true)
    }

    fn collect_tile(
        &mut self,
        tile: &DrawTile,
        hud_matrix: &Mat4,
        occlusion: &Occlusion,
    ) -> Result<(), GlError> {
        let Some(mesh) = tile.mesh.as_ref().filter(|mesh| !mesh.is_empty()) else {
            return Ok(());
        };
        let Some(resident) = self.meshes.get(mesh)? else {
            return Ok(());
        };

        // REQ-SCN-037: the renderer is the sole owner of the ±0.1 sprite depth nudge, applied
        // in the object's local frame, before the scale and the rotation, exactly once.
        let nudge = match (tile.sprite, tile.behind_background) {
            (false, _) => 0.0,
            (true, true) => geometry::SPRITE_NUDGE,
            (true, false) => -geometry::SPRITE_NUDGE,
        };
        let mut model = tile.transform;
        if nudge != 0.0 {
            model = model * Mat4::translation(Vec3::new(0.0, 0.0, nudge));
        }

        let hud = tile.container == SceneContainer::Interface;
        if hud {
            // REQ-SCN-045: the HUD lives in a plane attached to the camera.
            model = *hud_matrix * model;
        }

        // REQ-SCN-022: in FPV, an opaque tile of another shape that crosses the eye-to-target
        // segment is drawn transparent at 0.5 for this frame only.
        let occluder =
            tile.material == MaterialClass::Opaque && !hud && occlusion.hides(tile, mesh, &model);
        let material = if occluder {
            self.passes.statistics.occluders += 1;
            MaterialClass::Transparent
        } else {
            tile.material
        };

        let texture = match &tile.texture {
            Some(texture) => Some(self.textures.get(texture)?),
            None => None,
        };
        #[allow(clippy::cast_precision_loss)] // Texture sizes are small.
        let texture_size = tile.texture.as_ref().map_or(Vector2::ONE, |texture| {
            Vector2::new(texture.width() as f32, texture.height() as f32)
        });

        let item = DrawItem {
            mesh: resident,
            model,
            color_mode: i32::from(tile.texture_format == TextureFormat::Direct),
            texture,
            texture_size,
            palette_table: tile.palette_table,
            tex_offset: tile.tex_offset.into(),
            tex_tilting: tile.tex_tilting.into(),
            flat_color: Vector3::ZERO,
            alpha: if occluder {
                geometry::OCCLUDER_ALPHA
            } else if hud || material == MaterialClass::Transparent {
                tile.alpha
            } else {
                1.0
            },
            // AMD-021 R2-010 (REQ-SCN-032 "should" becomes "must not"): a tile rendered in
            // transparency — the FPV occluder of REQ-SCN-022 included — and a HUD tile receive
            // no shadow at all, whatever the flags of their motif and of their layer.
            receive_shadow: tile.receive_shadow && !hud && material != MaterialClass::Transparent,
            clip: !hud,
            deform: tile.deform,
            instance: Some(tile.instance),
            bounds_min: mesh.bounds_min(),
            bounds_max: mesh.bounds_max(),
        };

        // A tile only casts while it is rendered opaque (REQ-SCN-032, REQ-SCN-040).
        if tile.cast_shadow && !occluder {
            self.lists.casters.push(item);
        }

        match material {
            // REQ-SCN-041 style 2: not drawn, but always casts its shadow.
            MaterialClass::ShadowOnly => {}
            MaterialClass::Transparent => self.lists.transparent.push(item),
            MaterialClass::Interface => self.lists.hud.push(item),
            MaterialClass::Opaque => self.lists.opaque.push(item),
        }

        Ok(())
    }

    fn collect_backdrop(&mut self, scene: &SceneState) -> Result<(), GlError> {
        let backdrop = &scene.backdrop;
        let Some(resident) = self.meshes.get(&self.unit_box)? else {
            return Ok(());
        };

        let center = geometry::CENTER + backdrop.offset;
        let model = Mat4::translation(center.into()) * Mat4::scale_matrix(backdrop.size.into());
        let unit = &self.unit_box;
        self.lists.opaque.push(DrawItem {
            color_mode: 2,
            flat_color: color::to_vector(backdrop.color),
            // REQ-SCN-048: never casts, receives when asked, never clipped by R.
            receive_shadow: scene.settings.shadows && backdrop.receive_shadow,
            clip: false,
            ..DrawItem::plain(resident, model, (unit.bounds_min(), unit.bounds_max()))
        });
        Ok(())
    }

    // ----------------------------------------------------------------- shadow pass

    fn render_shadow_map(
        &mut self,
        scene: &SceneState,
        view: &CameraView,
    ) -> Result<ShadowVolume, GlError> {
        if !scene.settings.shadows || self.lists.casters.is_empty() {
            return Ok(ShadowVolume::EMPTY);
        }

        self.shadow_map.resize(scene.settings.shadow_map_size())?;
        let passes = &mut self.passes;
        passes.statistics.shadow_map_size = self.shadow_map.size();
        passes.statistics.requested_shadow_map_size = self.shadow_map.requested();

        let mut min = Vector3::splat(f32::MAX);
        let mut max = Vector3::splat(f32::MIN);
        for caster in &self.lists.casters {
            let point = Vector3::from(caster.model.transform_point(Vec3::ZERO));
            min = Vector3::min(min, point);
            max = Vector3::max(max, point);
        }

        // The tiles are at most 48 units across in their local frame (REQ-E3D-523), so padding
        // the anchors is enough and costs one matrix instead of one per vertex.
        min -= Vector3::splat(64.0);
        max += Vector3::splat(64.0);

        let volume = ShadowVolume::fit(min, max, view.eye, scene.light.direction);
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

        for caster in &self.lists.casters {
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
        passes.statistics.shadow_casters = count(self.lists.casters.len());
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

        let lists = &self.lists;
        if lists.opaque.is_empty() && lists.transparent.is_empty() && lists.hud.is_empty() {
            self.linear.resolve_to(target);
            return Ok(());
        }

        let passes = &mut self.passes;
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
            scene,
            frame,
            view,
            volume,
            textures: &mut self.textures,
            shadow_map_size: self.shadow_map.size(),
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
            .set_vec3("uLightDirection", scene.light.direction);
        for item in &lists.opaque {
            passes.draw_item(item, true);
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
                .set_vec3("uLightDirection", scene.light.direction);
            gl.enable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            for group in &lists.transparent_objects {
                passes.draw_blended(&lists.transparent[group.start..group.end]);
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
            passes.scene.set_vec3(
                "uLightDirection",
                view.hud_light_direction(scene.light.direction),
            );
            gl.enable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            for group in &lists.hud_objects {
                passes.draw_blended(&lists.hud[group.start..group.end]);
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
    scene: &'a SceneState,
    frame: &'a SceneFrame<'a>,
    view: &'a CameraView,
    volume: &'a ShadowVolume,
    textures: &'a mut TextureCache,
    shadow_map_size: i32,
    shadow_texture: Texture,
}

impl CommonUniforms<'_> {
    fn bind(&mut self, passes: &mut Passes) {
        let (gl, program) = (&passes.gl, &mut passes.scene);

        // The palette is refreshed first: the upload binds its own texture, so it has to happen
        // before the sampler units are set up (REQ-E3D-565).
        gl.active_texture(0);
        if let Some(list) = self.frame.draw_list {
            self.textures.upload_palette(list.palette());
        }

        program.set_matrix4("uViewProjection", &self.view.view_projection());
        program.set_f32("uTime", self.scene.emulated_time);
        program.set_f32("uAmbient", color::ambient_linear());
        program.set_f32("uShadowStrength", color::SHADOW_FACTOR);

        let light = color::rgb_to_linear(self.scene.light.color)
            * color::to_linear(self.scene.settings.light_intensity());
        program.set_vec3("uLightLinear", light);

        let rect = self.scene.visible_rectangle();
        program.set_vec4(
            "uClipRect",
            Vector4::new(rect.min_x, rect.min_y, rect.max_x, rect.max_y),
        );

        let volume = self.volume;
        program.set_matrix4("uLightViewProjection", &volume.view_projection);
        // The depth bias of the shadow test, in scene units: the 0.02 unit of REQ-SCN-031,
        // raised to one texel of the current map when a texel is coarser than that (Q-032,
        // amended by AMD-034 §2). Q-025 fills the map with back faces only, and the wall that
        // closes a tile against its neighbour is a back face too, inside the solid: a receiver
        // one texel from a joint reads it and darkens (the seam of REQ-SCN-065). The error is
        // proportional to the texel, so the bias has to be too.
        let bias = if volume.has_content {
            geometry::shadow_bias(volume.texel_size(self.shadow_map_size)) / volume.depth_range
        } else {
            0.0
        };
        program.set_f32("uShadowBias", bias);
        program.set_f32("uNormalOffset", geometry::SHADOW_NORMAL_OFFSET);
        passes.shadow_enabled = volume.has_content;

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

    fn draw_item(&mut self, item: &DrawItem, opaque: bool) {
        self.scene
            .set_matrix4("uModel", &interop::to_numerics(&item.model));
        let (normal, determinant) = interop::normal_matrix(&item.model);
        self.scene.set_matrix3("uNormalMatrix", &normal);

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

        self.draw(item.mesh);
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
    fn draw_blended(&mut self, items: &[DrawItem]) {
        self.gl.color_mask(false);
        self.gl.depth_mask(true);
        self.gl.depth_func(glow::LESS);
        for item in items {
            self.draw_item(item, false);
        }

        self.gl.color_mask(true);
        self.gl.depth_mask(false);
        self.gl.depth_func(glow::EQUAL);
        for item in items {
            self.draw_item(item, false);
        }

        self.gl.depth_mask(true);
        self.gl.depth_func(glow::LESS);
    }

    fn draw(&mut self, mesh: GpuMesh) {
        if self.bound_vertex_array != Some(mesh.vertex_array) {
            self.gl.bind_vertex_array(Some(mesh.vertex_array));
            self.bound_vertex_array = Some(mesh.vertex_array);
        }
        self.gl.draw_triangles_indexed(mesh.index_count);
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

/// Cuts `items` into the objects of AMD-012 — the runs of tiles that share one shape instance,
/// which SPEC-04B REQ-E3D-641 delivers consecutively — and measures each one by the distance
/// from the eye to the centre of its transformed bounding box.
fn group_by_object(items: &[DrawItem], groups: &mut Vec<DrawGroup>, eye: Vector3) {
    groups.clear();
    let mut start = 0;
    while start < items.len() {
        let instance = items[start].instance;
        let length = items[start + 1..]
            .iter()
            .take_while(|item| instance.is_some() && item.instance == instance)
            .count();
        let end = start + 1 + length;

        let mut min = Vector3::splat(f32::MAX);
        let mut max = Vector3::splat(f32::MIN);
        for item in &items[start..end] {
            let (low, high) = occlusion::box_bounds(item.bounds_min, item.bounds_max, &item.model);
            min = Vector3::min(min, low);
            max = Vector3::max(max, high);
        }

        groups.push(DrawGroup {
            start,
            end,
            view_depth: Vector3::distance((min + max) * 0.5, eye),
        });
        start = end;
    }
}

/// A count for the statistics, saturated (no frame comes near `u32::MAX` items).
fn count(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}
