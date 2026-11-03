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
use zax_engine3d::registry::PatternId;
use zax_engine3d::space_bend::SpaceBend;
use zax_engine3d::vectors::Mat4;

use crate::assets::SceneAssets;
use crate::camera::CameraView;
use crate::color;
use crate::frame_plan::{
    DrawItem, DrawLists, FixedMeshes, FramePlan, InstanceRun, Residency, SceneConstants,
    INSTANCE_BYTES, STD140_PLACEMENT,
};
use crate::geometry;
use crate::gl::api::{Gl, GlError};
use crate::gl::gpu_timer::GpuTimer;
use crate::gl::linear_buffer::LinearColorBuffer;
use crate::gl::linear_palette::LinearPalette;
use crate::gl::mesh_cache::{GpuMesh, MeshCache};
use crate::gl::picture::NesPictureTexture;
use crate::gl::program::ShaderProgram;
use crate::gl::screen_effects::{ScreenEffects, ScreenFrame};
use crate::gl::shaders::{self, SceneVariant};
use crate::gl::shadow_map::ShadowMap;
use crate::gl::skybox::SkyboxPass;
use crate::gl::texture_cache::TextureCache;
use crate::interop;
use crate::numerics::{Vector3, Vector4};
use crate::scene::{SceneFrame, SceneState};
use crate::shadow_volume::ShadowVolume;
use crate::statistics::RenderStatistics;
use crate::style::StyleSettings;
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

    fn texture(&mut self, texture: &Arc<ObjectTexture>) -> Option<Texture> {
        self.textures.get(texture)
    }

    fn picture(&mut self, picture: &[u8], height: usize) -> Result<Option<Texture>, GlError> {
        self.picture.upload(picture, height)
    }
}

/// The scene programs of [`shaders::scene`] and which one is in use.
///
/// Three of the four variant axes are chosen per draw or per frame and so need their programs
/// side by side: `solid` changes within the opaque bucket (several times a frame), `bend` changes
/// when a curvature setting changes and when the HUD is drawn (REQ-SCN-068 keeps it flat), and
/// `centroid` changes when `video.anti_alias` changes (AMD-058). The fourth, the per-fragment
/// palette, is fixed when the renderer is created.
///
/// The backdrop lighting of AMD-049 is a third such axis, chosen for the one opaque run that
/// draws the backdrop (REQ-SCN-075).
///
/// Only the flat, pixel-centre scene pair is built when the renderer is created. Every other key
/// is built the first time it is asked for, and a key that has none falls back to that pair: at
/// curvature 0 (REQ-SCN-066's default), with anti-aliasing off and the backdrop lighting off (the
/// handheld defaults) **no second program is ever compiled**, which on a handheld is seconds of
/// start-up. The fallback is also what a failed compile leaves behind — the frame is then drawn
/// with the model of before the amendment rather than not at all.
#[derive(Debug)]
// Each bool is one independent variant axis the specification names, plus the frame's answer for
// the backdrop; an enum per axis would say less, not more.
#[allow(clippy::struct_excessive_bools)]
struct ScenePrograms {
    /// Every program built so far; `built[0]` and `built[1]` are the flat, pixel-centre
    /// `[plain, solid]` pair the renderer is created with.
    built: Vec<ShaderProgram>,
    /// For each `(bend, centroid, voxel_grid, pixel_shadow, backdrop, floor_mask)` key, the indices in
    /// `built` of its plain and solid programs — `[0, 1]`, the flat scene pair, until that
    /// variant has been built. A backdrop key points **both** entries at one program: the
    /// backdrop never clips and never reads a colour texture, so only the solid form is ever
    /// asked for, and compiling the same source twice to fill a pair would be a program for
    /// nothing.
    slots: [[usize; 2]; Self::KEYS],
    /// `slots[key(...)]`, kept in step by the setters and by [`Self::install`], so that
    /// [`Self::current`] — which every draw run calls — is two indexings and no branch.
    slot: [usize; 2],
    solid: bool,
    bend: bool,
    centroid: bool,
    /// The voxel grid of AMD-055 REQ-SCN-081.
    voxel_grid: bool,
    /// The pixel shadows of AMD-055 REQ-SCN-082. Not an axis of the **backdrop** key: that
    /// requirement is about tile meshes and is without object on the backdrop.
    pixel_shadow: bool,
    backdrop: bool,
    /// Where in `built` the floor-mask program of AMD-055 REQ-SCN-084 is, and the curvature it
    /// was built for; `None` until a frame asks for the reflections.
    ///
    /// **Not an axis of [`ProgramKey`]**, and that is the brief's point 1 measured: widening the
    /// key put a branch — or an extra shift and add — inside [`Self::current_key`], which every
    /// draw run of every frame refreshes the cached slot through, and cost `render_scene` about
    /// 120 instructions. A field of its own costs Classic nothing: [`Self::set_floor_mask`]
    /// writes the cached slot directly, twice on a frame that lays a mask down and never
    /// otherwise.
    mask: Option<usize>,
    mask_bend: bool,
    /// Whether this frame wants the backdrop model at all (`render.backdrop_lighting` and a
    /// backdrop to draw), decided once a frame by [`SceneRenderer::select_backdrop_variant`].
    ///
    /// Kept apart from whether the program *exists*: a program built for one frame outlives the
    /// setting that asked for it, and turning the setting off again has to go back to the diffuse
    /// model of REQ-SCN-048 without throwing the program away.
    backdrop_wanted: bool,
    /// How many of `built` were compiled for AMD-055 — a key with the voxel grid or the pixel
    /// shadows set. **0 in Classic, and 0 in Enhanced until one of the two is turned on**, which
    /// is the brief's point 1 and point 2 as a counter (`RenderStatistics::style_programs`). The
    /// curvature's and the centroid's own programs are not counted here: they are AMD-043's and
    /// AMD-058's, and they exist in Classic too.
    style_programs: u32,
}

/// One key of [`ScenePrograms::slots`]: which variant a draw of this frame asks for, less the
/// `solid` form, which the pair itself holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// One bool per variant axis, which is what a key is. The same `allow` `ScenePrograms` carries.
#[allow(clippy::struct_excessive_bools)]
struct ProgramKey {
    bend: bool,
    centroid: bool,
    voxel_grid: bool,
    pixel_shadow: bool,
    backdrop: bool,
}

impl ScenePrograms {
    /// `bend` × `centroid` × `voxel_grid` × `pixel_shadow` × `backdrop`. The floor mask of
    /// AMD-055 REQ-SCN-084 is **not** an axis of this table: see [`Self::mask`].
    const KEYS: usize = 32;

    /// The slot of `slots` a variant is kept at.
    const fn key(key: ProgramKey) -> usize {
        (key.bend as usize) * 16
            + (key.centroid as usize) * 8
            + (key.voxel_grid as usize) * 4
            + (key.pixel_shadow as usize) * 2
            + key.backdrop as usize
    }

    /// The key of the variant currently in use.
    ///
    /// The pixel-shadow bit is **cleared for a backdrop key**, because REQ-SCN-082 is without
    /// object on the backdrop and [`shaders::backdrop`] therefore has no such axis: normalising
    /// it here is what keeps the slot a backdrop program was installed at and the slot the
    /// backdrop run then looks it up at the same one.
    ///
    /// The mask of REQ-SCN-084 is **not** in this key, and that is a measurement: a `floor_mask`
    /// axis here cost `render_scene` about 120 instructions, because every `set_*` of the opaque
    /// loop refreshes the cached slot through this function. [`Self::mask`] holds it instead, and
    /// Classic therefore pays nothing at all for it.
    const fn current_key(&self) -> ProgramKey {
        ProgramKey {
            bend: self.bend,
            centroid: self.centroid,
            voxel_grid: self.voxel_grid,
            pixel_shadow: self.pixel_shadow && !self.backdrop,
            backdrop: self.backdrop,
        }
    }

    /// The program in use, for a uniform or a draw.
    fn current(&mut self) -> &mut ShaderProgram {
        &mut self.built[self.slot[usize::from(self.solid)]]
    }

    /// Whether this key has a program of its own (the flat scene pair always has).
    const fn has(&self, key: ProgramKey) -> bool {
        let slot = Self::key(key);
        slot == 0 || self.slots[slot][1] != 1
    }

    /// Takes a freshly built `[plain, solid]` pair into service for its key.
    fn install(&mut self, key: ProgramKey, pair: [ShaderProgram; 2]) {
        let first = self.built.len();
        self.built.extend(pair);
        self.slots[Self::key(ProgramKey {
            backdrop: false,
            ..key
        })] = [first, first + 1];
        if key.voxel_grid || key.pixel_shadow {
            self.style_programs += 2;
        }
        self.refresh();
    }

    /// Takes a freshly built backdrop program into service: one program for both forms.
    fn install_backdrop(&mut self, key: ProgramKey, program: ShaderProgram) {
        self.built.push(program);
        let only = self.built.len() - 1;
        self.slots[Self::key(ProgramKey {
            backdrop: true,
            ..key
        })] = [only, only];
        if key.voxel_grid {
            self.style_programs += 1;
        }
        self.refresh();
    }

    /// Takes a freshly built floor-mask program of AMD-055 REQ-SCN-084 into service. One program
    /// for both forms, as the backdrop's is: the mask never clips and reads no colour texture, so
    /// the solid form is never asked for.
    ///
    /// Kept in [`Self::mask`] rather than in the slot table, for the reason that field states.
    fn install_mask(&mut self, bend: bool, program: ShaderProgram) {
        self.built.push(program);
        self.mask = Some(self.built.len() - 1);
        self.mask_bend = bend;
        self.style_programs += 1;
    }

    /// Puts the floor-mask program of AMD-055 REQ-SCN-084 in or out of use, for the one pass that
    /// lays the mask down. Writes the cached slot straight, so the frame's own variant comes back
    /// with one [`Self::refresh`] and nothing of this reaches the draw runs.
    fn set_floor_mask(&mut self, floor_mask: bool) {
        match (floor_mask, self.mask) {
            (true, Some(index)) => self.slot = [index, index],
            _ => self.refresh(),
        }
    }

    fn set_bend(&mut self, bend: bool) {
        self.bend = bend;
        self.refresh();
    }

    fn set_centroid(&mut self, centroid: bool) {
        self.centroid = centroid;
        self.refresh();
    }

    /// Puts the voxel-grid variant of AMD-055 REQ-SCN-081 in or out of use. The HUD pass calls it
    /// to go back to the plain variant for its own draws (REQ-SCN-081: the grid never applies to
    /// the HUD), exactly as it does for the curvature.
    fn set_voxel_grid(&mut self, voxel_grid: bool) {
        self.voxel_grid = voxel_grid;
        self.refresh();
    }

    /// Puts the pixel-shadow variant of AMD-055 REQ-SCN-082 in or out of use.
    fn set_pixel_shadow(&mut self, pixel_shadow: bool) {
        self.pixel_shadow = pixel_shadow;
        self.refresh();
    }

    /// Puts the backdrop variant of AMD-049 in use, and says whether that changed the program —
    /// it does not when this frame does not want the model, nor when the variant has not been
    /// built, which are both the case whenever `render.backdrop_lighting` is off.
    fn set_backdrop(&mut self, backdrop: bool) -> bool {
        let backdrop = backdrop && self.backdrop_wanted;
        if backdrop == self.backdrop {
            return false;
        }
        let before = self.slot;
        self.backdrop = backdrop;
        self.refresh();
        before != self.slot
    }

    fn refresh(&mut self) {
        self.slot = self.slots[Self::key(self.current_key())];
    }
}

/// The programs and the cached GL state the draw calls go through, and what they cost.
#[derive(Debug)]
#[allow(clippy::struct_excessive_bools)] // Independent pieces of cached GL state.
struct Passes {
    gl: Gl,
    /// The four scene programs and the one in use.
    scene: ScenePrograms,
    shadow: ShaderProgram,
    /// The shadow program with the curvature compiled in (REQ-SCN-068: the shadow pass bends
    /// too), built beside the scene pair the first time a curvature is asked for.
    shadow_bent: Option<ShaderProgram>,
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
    /// The clip-space bias of AMD-057 the draws apply to the lateral walls of a tile box
    /// (`geometry::wall_clip_bias(WALL_DEPTH_BIAS)`). A test lowers it to 0 to show that it is
    /// what holds REQ-SCN-065 now that the walls are kept.
    wall_bias: f32,
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
// Each boolean is an independent property of the session — whether the palette is converted,
// whether the screen effects were given up, the test seam — and folding them into an enum would
// hide that. The same `allow` `Passes`, `ScenePrograms`, `SceneVariant` and `ProgramKey` carry.
#[allow(clippy::struct_excessive_bools)]
pub struct SceneRenderer {
    gl: Gl,
    meshes: MeshCache,
    textures: TextureCache,
    shadow_map: ShadowMap,
    skybox: SkyboxPass,
    picture: NesPictureTexture,
    linear: LinearColorBuffer,
    /// The screen-space passes of AMD-055 REQ-SCN-080 and everything they own, created the first
    /// time a frame asks for one and dropped as soon as none does.
    ///
    /// **`None` is the state of every Classic frame**, and of every Enhanced frame at the
    /// defaults of REQ-SCN-079: the whole cost of AMD-055's screen side is then the `is_none`
    /// test that decides not to run it (point 1 and point 2 of this lot's brief).
    screen_effects: Option<ScreenEffects>,
    /// Whether this session has given the screen-space passes up because the implementation
    /// refused them (point 3 of the brief). Tested only on a frame that wants one, and out of
    /// line, so a Classic frame never reads it.
    screen_effects_given_up: bool,
    /// The test seam of [`Self::fail_screen_effects`]: no code path sets it.
    fail_screen_effects: bool,
    linear_palette: LinearPalette,
    /// Whether the scene programs read [`LinearPalette`] (else they linearise per fragment).
    convert_palette: bool,
    unit_box: Arc<TileMesh>,
    /// The backdrop box with each face cut into 32 × 32 cells (REQ-SCN-068), built the first
    /// time a curvature is active and used only then: at curvature 0 the box stays the six quads
    /// of `interop::unit_box`, so the vertex count of the default settings is unchanged.
    unit_box_bent: Option<Arc<TileMesh>>,
    picture_quad: Arc<TileMesh>,
    /// The rectangle of the 2D mode cut into 32 × 32 cells (AMD-046 REQ-SCN-074), built the
    /// first time it has to bend. At curvature 0, or with "Bend 2D Mode" off, the rectangle
    /// stays the single quad of `interop::picture_quad`.
    picture_quad_bent: Option<Arc<TileMesh>>,
    /// The quad of the last widescreen 2D-mode picture, by margin.
    wide_picture_quad: Option<interop::WidePictureQuad>,
    /// The subdivision `wide_picture_quad` was built for, so a change of the curvature rebuilds
    /// it (the margin alone no longer tells the two apart).
    wide_picture_cells: u16,
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

        // `ZAXNES_SRGB_PALETTE` keeps the per-fragment conversion of the palette, to compare
        // the two on a device (same pictures).
        let convert_palette = std::env::var_os("ZAXNES_SRGB_PALETTE").is_none();
        let flat = scene_pair(
            &gl,
            !convert_palette,
            ProgramKey {
                bend: false,
                centroid: false,
                voxel_grid: false,
                pixel_shadow: false,
                backdrop: false,
            },
        )?;
        let instance_alignment =
            usize::try_from(gl.get_integer(glow::UNIFORM_BUFFER_OFFSET_ALIGNMENT))
                .unwrap_or(256)
                .max(1);
        let passes = Passes {
            gl: gl.clone(),
            scene: ScenePrograms {
                built: flat.into(),
                slots: [[0, 1]; ScenePrograms::KEYS],
                slot: [0, 1],
                solid: false,
                bend: false,
                centroid: false,
                voxel_grid: false,
                pixel_shadow: false,
                backdrop: false,
                mask: None,
                mask_bend: false,
                backdrop_wanted: false,
                style_programs: 0,
            },
            shadow: ShaderProgram::create(&gl, &shaders::shadow(false))?,
            shadow_bent: None,
            dummy_indexed: textures.dummy_indexed(),
            dummy_color: textures.dummy_color(),
            shadow_enabled: false,
            mirrored_winding: false,
            mirrored_normal: false,
            bound_textures: [None; 2],
            bound_vertex_array: None,
            instance_buffer: gl.create_buffer()?,
            instance_alignment,
            wall_bias: geometry::wall_clip_bias(geometry::WALL_DEPTH_BIAS),
            statistics,
        };

        Ok(Self {
            meshes: MeshCache::new(&gl),
            skybox: SkyboxPass::new(&gl, assets),
            picture: NesPictureTexture::new(&gl),
            linear: LinearColorBuffer::new(&gl)?,
            screen_effects: None,
            screen_effects_given_up: false,
            fail_screen_effects: false,
            linear_palette: LinearPalette::new(&gl)?,
            convert_palette,
            unit_box: Arc::new(interop::unit_box()?),
            unit_box_bent: None,
            picture_quad: Arc::new(interop::picture_quad()?),
            picture_quad_bent: None,
            wide_picture_quad: None,
            wide_picture_cells: 1,
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

        // AMD-058 REQ-SCN-065: the anti-aliasing setting decides how `vUv` is interpolated, once
        // here, never per fragment. Chosen **before** the curvature so that the two pairs the
        // curvature then builds are the ones the frame and its HUD actually use (the HUD draws
        // flat, REQ-SCN-068). At the handheld default — anti-aliasing off — this is one integer
        // comparison and nothing else.
        self.select_centroid_variant(scene)?;
        // REQ-SCN-067: the curvature decides which shader variant draws the frame, once here,
        // never per vertex. At the default settings this is one `is_identity` test and nothing
        // else: no program is compiled, none is swapped, and no uniform is uploaded.
        self.select_bend_variant(scene)?;
        // AMD-055 REQ-SCN-081 and REQ-SCN-082: the voxel grid and the pixel shadows, two
        // independent axes of the same key. In Classic, and in Enhanced with both at their
        // defaults, two boolean tests and two stores.
        self.select_style_variants(scene)?;
        // AMD-049 REQ-SCN-075: and which key the backdrop program belongs to, once the three
        // above have settled. One boolean test when the setting is off.
        self.select_backdrop_variant(scene)?;
        // AMD-055 REQ-SCN-080 and REQ-SCN-083: whether this frame runs a screen-space pass, which
        // is what decides whether the module and its targets exist at all. In Classic, and in
        // Enhanced at the defaults of REQ-SCN-079, one boolean test and one `is_none`.
        self.select_screen_effects(scene);

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
        // AMD-046 REQ-SCN-074: the rectangle of the 2D mode is subdivided while it bends, and
        // only then. `select_bend_variant` has already folded the setting in, so in 2D mode this
        // flag *is* "the rectangle bends".
        let bend_rectangle = self.passes.scene.bend;
        let cells = interop::subdivision_for(bend_rectangle);
        if cells != self.wide_picture_cells {
            self.wide_picture_quad = None;
            self.wide_picture_cells = cells;
        }
        self.wide_picture_quad = interop::wide_quad_for_subdivided_cached(
            frame.nes_picture,
            frame.nes_picture_height,
            cells,
            self.wide_picture_quad.take(),
        )?;
        let fixed = FixedMeshes {
            // REQ-SCN-068: the subdivided box only while something bends.
            unit_box: match &self.unit_box_bent {
                Some(subdivided) if self.passes.scene.bend => subdivided,
                _ => &self.unit_box,
            },
            picture_quad: match &self.picture_quad_bent {
                Some(subdivided) if bend_rectangle => subdivided,
                _ => &self.picture_quad,
            },
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

        let effects = self.screen_effects.as_ref();
        let statistics = &mut self.passes.statistics;
        // AMD-055, point 1 of the lot's brief: the variants of REQ-SCN-081 and REQ-SCN-082 and the
        // programs of the two screen passes, counted together. 0 in Classic, and 0 in Enhanced
        // until an effect is turned on.
        statistics.style_programs =
            self.passes.scene.style_programs + effects.map_or(0, ScreenEffects::programs);
        statistics.screen_effect_bytes = effects.map_or(0, ScreenEffects::bytes);
        statistics.resident_meshes = count(self.meshes.count());
        statistics.mesh_uploads = self.meshes.uploads();
        statistics.resident_textures = count(self.textures.count());
        statistics.texture_texels = self.textures.texels();
        statistics.patterns_without_texture = self.plan.diagnostics.patterns_without_texture();
        self.meshes.collect();
        self.textures.collect();
        Ok(())
    }

    /// Picks the shader variant the frame's curvature asks for (REQ-SCN-067), building the
    /// curvature programs and the subdivided meshes the first time one is needed.
    ///
    /// At curvature 0 — the default of all four settings of REQ-SCN-066 — this leaves the
    /// renderer exactly as AMD-043 found it: the flat pair stays in use, the curvature programs
    /// are never compiled and the two fixed meshes keep their shape.
    ///
    /// In **2D mode** the frame holds one object, the rectangle of AMD-046 §2 (the backdrop of
    /// REQ-SCN-048 is not collected there), so "Bend 2D Mode" turned off is simply the flat
    /// variant for the whole frame — which is REQ-SCN-074's "la position de chaque sommet du
    /// rectangle DOIT être sa position plate, quels que soient les réglages de REQ-SCN-066". In
    /// 3D mode the setting has no effect at all, as the same requirement asks.
    ///
    /// # Errors
    ///
    /// [`RenderError::Gl`] when a curvature program does not compile or link, or
    /// [`RenderError::Mesh`] when a subdivided mesh is malformed. The flat programs stay in use,
    /// so the frame is drawn without the curvature rather than not at all.
    fn select_bend_variant(&mut self, scene: &SceneState) -> Result<(), RenderError> {
        let wanted = !scene.effective_bend().is_identity();
        if !wanted {
            self.passes.scene.set_bend(false);
            return Ok(());
        }

        let key = ProgramKey {
            bend: true,
            backdrop: false,
            ..self.passes.scene.current_key()
        };
        if !self.passes.scene.has(key) {
            let pair = scene_pair(&self.gl, !self.convert_palette, key)?;
            self.passes.scene.install(key, pair);
        }
        if self.passes.shadow_bent.is_none() {
            self.passes.shadow_bent =
                Some(ShaderProgram::create(&self.gl, &shaders::shadow(true))?);
        }
        let cells = interop::subdivision_for(true);
        if self.unit_box_bent.is_none() {
            // REQ-SCN-068: a plane large enough to curve visibly is subdivided, the backdrop
            // into at least 32 × 32 cells per face.
            self.unit_box_bent = Some(Arc::new(interop::unit_box_subdivided(cells)?));
        }
        if self.picture_quad_bent.is_none() && scene.two_dimensional_mode {
            // REQ-SCN-074: at least 32 × 32 cells, so at least a 33 × 33 grid of vertices.
            self.picture_quad_bent = Some(Arc::new(interop::picture_quad_subdivided(0, 0, cells)?));
        }
        self.passes.scene.set_bend(true);
        Ok(())
    }

    /// Picks the interpolation variant the frame's anti-aliasing asks for (AMD-058 REQ-SCN-065),
    /// building its pair the first time it is needed.
    ///
    /// The trigger is the **setting** of AMD-032 §2, not the sample count the driver granted: a
    /// buffer that falls back to one sample interpolates a centroid at the pixel centre anyway
    /// (the covered sample set is that one sample), so the fallback draws the same pixels. With
    /// `video.anti_alias` = 0 nothing is built and nothing is compiled, which is what keeps the
    /// handheld default at the programs of before this amendment.
    ///
    /// # Errors
    ///
    /// [`RenderError::Gl`] when the variant does not compile or link. The pixel-centre programs
    /// stay in use, so the frame is drawn with the joints of before rather than not at all.
    fn select_centroid_variant(&mut self, scene: &SceneState) -> Result<(), RenderError> {
        let wanted = scene.settings.anti_alias_samples() > 1;
        if !wanted {
            self.passes.scene.set_centroid(false);
            return Ok(());
        }

        let key = ProgramKey {
            centroid: true,
            backdrop: false,
            ..self.passes.scene.current_key()
        };
        if !self.passes.scene.has(key) {
            let pair = scene_pair(&self.gl, !self.convert_palette, key)?;
            self.passes.scene.install(key, pair);
        }
        self.passes.scene.set_centroid(true);
        Ok(())
    }

    /// Picks the voxel-grid variant of AMD-055 REQ-SCN-081 and the pixel-shadow variant of
    /// REQ-SCN-082, building each pair the first time it is needed.
    ///
    /// Two **independent** axes, because REQ-SCN-079 switches `spl.voxel_grid` and
    /// `spl.pixel_shadow` separately: a player who turns the grid on does not pay for the voxel
    /// snapping, nor the other way round. In Classic, and in Enhanced with both at their
    /// defaults, this is two boolean tests and two stores — no program is compiled, none is
    /// swapped and no uniform is uploaded.
    ///
    /// Neither applies in **2D mode**: the frame then holds one object, the rectangle of
    /// AMD-046 §2 (REQ-SCN-081 and REQ-SCN-082 both exclude it), so the plain variant draws the
    /// whole frame and the exclusion costs nothing.
    ///
    /// # Errors
    ///
    /// [`RenderError::Gl`] when a variant does not compile or link. The plain programs stay in
    /// use, so the frame is drawn without the effect rather than not at all.
    fn select_style_variants(&mut self, scene: &SceneState) -> Result<(), RenderError> {
        let style = &scene.settings.style;
        let flat = scene.two_dimensional_mode;
        let grid = style.voxel_grid_active() && !flat;
        let shadow = style.pixel_shadow_active() && !flat;
        self.passes.scene.set_voxel_grid(grid);
        self.passes.scene.set_pixel_shadow(shadow);
        if !grid && !shadow {
            return Ok(());
        }

        let key = ProgramKey {
            backdrop: false,
            ..self.passes.scene.current_key()
        };
        if !self.passes.scene.has(key) {
            let pair = scene_pair(&self.gl, !self.convert_palette, key)?;
            self.passes.scene.install(key, pair);
        }
        // REQ-SCN-081: the HUD never takes the grid, and it draws with the flat variant of
        // REQ-SCN-068 — which, now that the grid is an axis of the same key, is also the variant
        // without the grid. The pair the HUD then asks for has to exist.
        let hud = ProgramKey {
            bend: false,
            voxel_grid: false,
            pixel_shadow: false,
            backdrop: false,
            ..key
        };
        if !self.passes.scene.has(hud) {
            let pair = scene_pair(&self.gl, !self.convert_palette, hud)?;
            self.passes.scene.install(hud, pair);
        }
        Ok(())
    }

    /// Builds the backdrop program of AMD-049 REQ-SCN-075 when the frame needs it, which is when
    /// `render.backdrop_lighting` is on **and** the profile's Lua enabled the backdrop
    /// (REQ-SCN-048). Both are false by default on a handheld, and the second is false for every
    /// profile that does not ask for a backdrop, so the usual frame compiles nothing here.
    ///
    /// Called once a frame, after the two variant choices above, because the curvature and the
    /// interpolation decide which key the backdrop program belongs to.
    ///
    /// # Errors
    ///
    /// [`RenderError::Gl`] when the program does not compile or link. The scene programs stay in
    /// use for the backdrop run, so it is drawn with the diffuse model of REQ-SCN-028 — the
    /// behaviour of before the amendment — rather than not at all.
    fn select_backdrop_variant(&mut self, scene: &SceneState) -> Result<(), RenderError> {
        let wanted = scene.settings.backdrop_lighting && scene.backdrop.enabled;
        self.passes.scene.backdrop_wanted = wanted;
        if !wanted {
            return Ok(());
        }

        // REQ-SCN-081 applies to the faces of the box (AMD-055 §3 point 5), so the grid is an
        // axis of the backdrop program too; the pixel shadows are not (REQ-SCN-082: "sans objet
        // sur le backdrop"), so that bit is cleared in the key.
        let key = ProgramKey {
            pixel_shadow: false,
            backdrop: true,
            ..self.passes.scene.current_key()
        };
        if !self.passes.scene.has(key) {
            let program =
                ShaderProgram::create(&self.gl, &shaders::backdrop(key.bend, key.voxel_grid))?;
            program.bind_uniform_block("ZaxInstances", INSTANCE_BINDING);
            self.passes.scene.install_backdrop(key, program);
        }
        Ok(())
    }

    /// Creates or releases the screen-space passes of AMD-055 REQ-SCN-080, once a frame.
    ///
    /// **What it does in the usual case is nothing.** In Classic the style gates every effect
    /// off, and in Enhanced `spl.ao` and `spl.reflections` are 0 by default (REQ-SCN-079), so this
    /// is one boolean test and an `Option` that is already `None`. The first frame that asks for
    /// an effect compiles its programs; the first frame that asks for none again drops the module,
    /// which releases its targets with it — the lifecycle point 1 of this lot's brief requires.
    ///
    /// **Not in 2D mode.** The rectangle of AMD-046 §2 is the only object of a 2D-mode frame, and
    /// REQ-SCN-083 excludes it from being modified while letting it occlude; not running the pass
    /// satisfies both clauses at once and costs nothing — the same reading
    /// [`Self::select_style_variants`] already applies to the voxel grid.
    ///
    /// **Each effect on its own.** The module holds the occlusion of REQ-SCN-083 and the
    /// reflections of REQ-SCN-084 as two independent halves, so a player who turns one on compiles
    /// and allocates only that one — point 2 of the brief, for the pair that are passes rather
    /// than variants.
    ///
    /// **A failure is not fatal** (point 3 of this lot's brief): an implementation that cannot
    /// give the passes their programs or their targets draws the frame **without** them — the
    /// Classic picture, which is cheaper and never more expensive — and says so once
    /// ([`RenderDiagnostics::screen_effects_unavailable`]). The effects are then given up for the
    /// session: the two formats they need are the two the renderer already depends on, so a
    /// refusal is a property of the implementation and not of the frame.
    fn select_screen_effects(&mut self, scene: &SceneState) {
        let style = &scene.settings.style;
        if !style.screen_effects_active() || scene.two_dimensional_mode {
            self.screen_effects = None;
            return;
        }
        self.prepare_screen_effects(scene);
    }

    /// Builds what the two screen passes need, and gives them up for the session if the
    /// implementation refuses. Out of line because none of it belongs in a Classic frame's
    /// instruction cache — the reason [`Self::run_screen_effects`] and
    /// [`LinearColorBuffer::allocate`] are out of line too.
    #[inline(never)]
    fn prepare_screen_effects(&mut self, scene: &SceneState) {
        if self.screen_effects_given_up {
            return;
        }
        if let Err(error) = self.build_screen_effects(&scene.settings.style) {
            self.screen_effects = None;
            self.screen_effects_given_up = true;
            let frame = self.plan.frame_count();
            self.plan
                .diagnostics
                .screen_effects_unavailable(&error.to_string(), frame);
        }
    }

    /// The fallible half of [`Self::prepare_screen_effects`].
    fn build_screen_effects(&mut self, style: &StyleSettings) -> Result<(), RenderError> {
        // The seam the fallback is observed through: no code path sets it, only a test
        // ([`Self::fail_screen_effects`]), because no driver this side refuses `R8`.
        if self.fail_screen_effects {
            return Err(RenderError::Gl(GlError::Unsupported {
                what: "the screen-space effects, refused on request of a test",
            }));
        }
        let effects = match &mut self.screen_effects {
            Some(effects) => effects,
            none => none.insert(ScreenEffects::new(&self.gl)?),
        };
        effects.select(style)?;

        // REQ-SCN-084's mask is laid down by the scene's own vertex stage, so its program belongs
        // with the other scene variants — and like them it is built the first time a frame asks
        // for it, for the curvature the frame draws with and no other.
        if style.reflections_active() {
            let bend = self.passes.scene.bend;
            let scene = &self.passes.scene;
            if scene.mask.is_none() || scene.mask_bend != bend {
                let program = ShaderProgram::create(&self.gl, &shaders::floor_mask(bend))?;
                program.bind_uniform_block("ZaxInstances", INSTANCE_BINDING);
                self.passes.scene.install_mask(bend, program);
            }
        }
        Ok(())
    }

    /// Whether the screen-space effects of REQ-SCN-080 have been given up for this session
    /// because the implementation refused them (point 3 of the brief). False on every machine
    /// whose driver answers normally, which is every machine the two formats are required on.
    #[must_use]
    pub const fn screen_effects_given_up(&self) -> bool {
        self.screen_effects_given_up
    }

    /// Makes the next frame that wants a screen-space effect fail to prepare it, so that the
    /// fallback above can be observed on a machine whose driver refuses nothing.
    ///
    /// Only a test calls this — the same kind of seam as [`Self::set_wall_depth_bias`], and for
    /// the same reason: the behaviour it exercises is otherwise unreachable here.
    pub const fn fail_screen_effects(&mut self, fail: bool) {
        self.fail_screen_effects = fail;
    }

    /// Drops every cached GPU mesh and texture. The host calls it when the 3D engine rebuilds
    /// its geometry wholesale, for instance after a change of mesh algorithm (REQ-E3D-340).
    pub fn invalidate_caches(&mut self) {
        self.meshes.clear();
        self.textures.clear();
        // AMD-051 REQ-E3D-644 counts its lines "par partie": a wholesale rebuild is the start of
        // a new one, and the patterns it reported no longer exist.
        self.plan.diagnostics.reset();
    }

    /// The scene distance a lateral wall of a tile box is pushed away from the eye
    /// (AMD-057, REQ-SCN-065); [`geometry::WALL_DEPTH_BIAS`] is what every frame uses.
    ///
    /// Settable so that a test can put it back to 0 and show that this — and not the geometry,
    /// which AMD-057 forbids changing — is what keeps the joints of a flat surface invisible now
    /// that the walls are kept.
    pub fn set_wall_depth_bias(&mut self, delta: f32) {
        self.passes.wall_bias = geometry::wall_clip_bias(delta);
    }

    /// Caps the texels the resident object textures may occupy (AMD-051 REQ-E3D-644,
    /// REQ-E3D-564); `None` — the default — leaves the driver as the only limit.
    ///
    /// Beyond the cap an object's texture is simply not made resident: its tiles are left out of
    /// the frame, every other pattern is drawn, the next frame is produced, and one diagnostic
    /// per pattern reaches [`Self::take_diagnostics`]. Lowering it is how the harness
    /// (`--atlas-budget`) and the tests reach a path a desktop GPU never takes on its own.
    pub fn set_texture_budget(&mut self, texels: Option<u64>) {
        self.textures.set_budget(texels);
    }

    /// The diagnostic lines AMD-051 asks for, and an empty vector on a frame where nothing went
    /// wrong (REQ-E3D-644, REQ-E3D-646). The host logs them; the renderer writes no file.
    pub fn take_diagnostics(&mut self) -> Vec<String> {
        self.plan.diagnostics.take()
    }

    /// Reports that the 3D object `index` of `pattern` has been waiting for its meshes for
    /// `frames` emulated frames (AMD-051 REQ-E3D-646). The conversion side finds them
    /// (`DrawListBuilder::take_late_objects`); the renderer only holds the one place that bounds
    /// how often a line is written.
    pub fn report_late_object(&mut self, pattern: PatternId, index: usize, frames: u32) {
        let frame = self.plan.frame_count();
        self.plan
            .diagnostics
            .object_late(pattern, index, frames, frame);
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

        // REQ-SCN-068: the shadow pass takes the same curvature as the scene, so the shadows
        // follow the curved surface.
        passes.shadow_program().use_program();
        passes.forget_bindings();
        passes
            .shadow_program()
            .set_matrix4("uLightViewProjection", &volume.view_projection);
        passes
            .shadow_program()
            .set_f32("uTime", scene.emulated_time);
        if !scene.bend.is_identity() {
            bind_bend(passes.shadow_program(), scene.bend);
        }

        for caster in casters {
            // Q-069: a mirrored caster winds the other way round, so the far side of the solid
            // is only selected once the winding is corrected.
            passes.set_winding(interop::determinant(&caster.model) < 0.0, false);
            passes
                .shadow_program()
                .set_matrix4("uModel", &interop::to_numerics(&caster.model));
            set_deformation(passes.shadow_program(), caster.deform.as_ref());
            passes.draw(caster.mesh);
        }

        passes.set_winding(false, false);
        passes.statistics.shadow_casters = count(casters.len());
        passes.statistics.shadow_pass = true;
        Ok(volume)
    }

    // ----------------------------------------------------------------- scene passes

    /// REQ-E3D-565: the palette of the frame, refreshed once, and linearised for the scene pass
    /// before its target is bound (the conversion draws into its own).
    fn refresh_palette(&mut self, frame: &SceneFrame<'_>) {
        if let Some(list) = frame.draw_list {
            self.gl.active_texture(0);
            self.textures.upload_palette(list.palette());
            if self.convert_palette {
                self.linear_palette
                    .convert(self.textures.palette(), list.palette());
            }
        }
    }

    fn render_scene(
        &mut self,
        target: &RenderTarget,
        scene: &SceneState,
        frame: &SceneFrame<'_>,
        view: &CameraView,
        volume: &ShadowVolume,
    ) -> Result<(), GlError> {
        self.refresh_palette(frame);

        // AMD-021 R2-009: the scene is drawn into the renderer's own linear buffer, so that the
        // blending of the transparent objects and of the HUD happens on linear colours, and the
        // whole image is encoded to sRGB once, on the way into the host framebuffer.
        // AMD-032 §2: that buffer is multisampled when anti-aliasing is on, and resolved before
        // the sRGB pass reads it. The shadow map keeps its own single-sample target.
        // AMD-055 REQ-SCN-080: a frame that runs a screen-space pass needs a depth a shader can
        // read, which is the one allocation Classic cannot share. `false` is the Classic path,
        // and with it `begin_pass` allocates exactly what it allocated before the amendment.
        self.linear.begin_pass(
            target.width,
            target.height,
            scene.settings.anti_alias_samples(),
            self.screen_effects.is_some(),
        )?;
        self.passes.statistics.anti_alias_samples = self.linear.samples();
        self.passes.statistics.centroid_interpolation = self.passes.scene.centroid;
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
        // AMD-055 REQ-SCN-084: which instances are floors, and the runs the mask pass redraws.
        // Both only on a frame whose Enhanced style asks for the reflections — in Classic this is
        // one `is_some_and` on an `Option` that is already `None`. After `lay_out_instances`,
        // which fixes the order of the opaque bucket the marks are indexed by, and before the
        // instance data is uploaded.
        if self
            .screen_effects
            .as_ref()
            .is_some_and(ScreenEffects::reflects)
        {
            mark_floors(
                &mut self.plan,
                self.passes.instance_alignment,
                scene.camera.local_camera,
            );
        }
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
            textures: &mut self.textures,
            linear_palette: self.linear_palette.texture(),
            shadow_texture: self.shadow_map.depth_texture(),
        };

        bind_frame_uniforms(passes, &mut common);
        // AMD-055 REQ-SCN-084: and on the mask program, when this frame has one. One
        // `has` on a slot table in Classic.
        bind_mask_uniforms(passes, &mut common, scene.camera.local_camera);

        // 1. Opaque objects.
        for run in &lists.opaque_runs {
            let item = &lists.opaque[run.first];
            // REQ-SCN-075: the backdrop is colour mode 2, and the only run that is. The test
            // rides on the `color_mode` this line already reads for `use_solid`, and
            // `set_backdrop` reports that nothing changed — so costs nothing at all — whenever
            // the variant was not built.
            if passes.scene.set_backdrop(item.color_mode == 2) {
                passes.scene.current().use_program();
                passes.set_normal_sign(passes.mirrored_normal, true);
            }
            passes.use_solid(!item.clip && item.color_mode != 1);
            passes.draw_run(&lists.opaque, run, true);
        }
        if passes.scene.set_backdrop(false) {
            passes.scene.current().use_program();
        }
        passes.use_solid(false);
        passes.statistics.opaque_tiles = count(lists.opaque.len());

        // The tiles may have left the winding reversed (Q-069).
        passes.set_winding(false, false);

        // 2. The screen-space passes of AMD-055 REQ-SCN-080, in the order that requirement fixes:
        // after the opaque objects — so the tiles, the backdrop and the 2D-mode rectangle occlude,
        // are darkened and are reflected — and before the transparent ones, which therefore
        // neither occlude nor reflect nor are darkened, nor is the HUD drawn after them. The floor
        // reflections of REQ-SCN-084 go first, then the occlusion of REQ-SCN-083.
        if let Some(effects) = &mut self.screen_effects {
            run_screen_effects(
                effects,
                &mut self.linear,
                passes,
                lists,
                &self.plan.floor_runs,
                (scene, view),
            )?;
        }

        // 3. Transparent objects, farthest first, each with a depth-only pass first
        // (REQ-SCN-040, AMD-012).
        if !lists.transparent.is_empty() {
            draw_transparent(passes, &mut common, lists);
        }

        // 4. HUD, after the transparent objects, lit in the camera frame (REQ-SCN-029,
        // REQ-SCN-044).
        if !lists.hud.is_empty() {
            draw_hud(passes, &mut common, lists);
        }

        gl.depth_mask(true);
        gl.depth_func(glow::LESS);
        passes.set_winding(false, false);
        self.linear.resolve_to(target);
        Ok(())
    }
}

/// The HUD pass of REQ-SCN-044, after the transparent objects and lit in the camera frame
/// (REQ-SCN-029).
///
/// It draws with the **plain** variant whatever the frame holds: the HUD is the plane attached to
/// the camera (REQ-SCN-045), so it stays flat under the curvature of REQ-SCN-068 — which is also
/// what keeps the score of a bent game readable — and AMD-055 excludes it from the voxel grid of
/// REQ-SCN-081 and from the pixel shadows of REQ-SCN-082. One switch therefore drops all three,
/// and `select_style_variants` has already built the pair this asks for.
/// Runs the screen-space passes of AMD-055 REQ-SCN-080 over the frame drawn so far, and leaves the
/// state the draws that follow expect.
///
/// Extracted from `render_scene` for the reason `draw_hud` was: the body of that function is at
/// clippy's line ceiling, and the pass block is one self-contained step of REQ-SCN-080's order.
///
/// **`#[inline(never)]` is the brief's point 1, measured.** Inlined, this body put 973 instructions
/// — 27 % — into `render_scene`, a function R0 counts: none of them runs in Classic, where the
/// branch is not taken, but a hot once-a-frame function should not carry the Enhanced path through
/// its instruction cache to prove it. Out of line, `render_scene` costs the one `is_some` test that
/// decides not to call this, and the call itself is paid once a frame by a frame that wants it.
///
/// # Errors
///
/// [`GlError`] when a target cannot be allocated. The frame is then drawn without the effect.
#[inline(never)]
fn run_screen_effects(
    effects: &mut ScreenEffects,
    linear: &mut LinearColorBuffer,
    passes: &mut Passes,
    lists: &DrawLists<GpuMesh, Texture>,
    floor_runs: &[InstanceRun],
    (scene, view): (&SceneState, &CameraView),
) -> Result<(), GlError> {
    // REQ-SCN-080's last bullet: the two passes work on the **resolved** colour and depth. The
    // consequence, stated where it happens: from here on the frame draws into the single-sample
    // target, so with anti-aliasing *and* a screen effect the transparent objects and the HUD are
    // not multisampled.
    linear.resolve_samples();
    if let (Some(depth), Some(color)) = (linear.depth_texture(), linear.color_texture()) {
        let frame = ScreenFrame {
            depth,
            color,
            capacity: linear.capacity(),
            drawn: linear.drawn(),
            drawn_scale: linear.drawn_scale(),
            projection: projection_terms(view),
        };
        // The scene scale of REQ-SCN-002 is 1 outside virtual reality, which Zaxnes has none of,
        // so a length in scene units is a length in view units
        // (`frame_plan::SceneConstants::voxel_cell` reads the same scale the same way).
        let style = &scene.settings.style;
        let mut submitted = 0;

        // 2a. The reflections of REQ-SCN-084, **first** (REQ-SCN-058 as the amendment orders it),
        // so that what they reflect is the scene the occlusion has not yet darkened.
        if effects.reflects() {
            if effects.begin_mask(frame)? {
                draw_floor_mask(passes, lists, floor_runs);
                submitted += 1;
            }
            submitted += effects.trace(frame, style, 1.0)?;
            linear.bind_for_draw();
            submitted += effects.composite_reflections(frame);
        }

        // 2b. The occlusion of REQ-SCN-083.
        if effects.occludes() {
            submitted += effects.occlude(frame, style, 1.0)?;
            linear.bind_for_draw();
            submitted += effects.composite_occlusion(frame);
        }

        effects.end_passes();
        passes.statistics.screen_effect_passes = submitted;
    }

    // The passes bound targets and a vertex array of their own, and left the depth test off; the
    // draws that follow rebind what they need and expect the frame's state.
    passes.forget_bindings();
    let gl = &passes.gl.clone();
    gl.enable(glow::DEPTH_TEST);
    gl.depth_func(glow::LESS);
    gl.depth_mask(true);
    if scene.settings.cull_back_faces {
        gl.enable(glow::CULL_FACE);
        gl.cull_face(glow::BACK);
    }
    Ok(())
}

/// The two steps AMD-055 REQ-SCN-084 asks of the draw list: which instances are floors, and the
/// runs the mask pass redraws.
///
/// **`#[inline(never)]` is the brief's point 1, measured**, the same way `run_screen_effects`
/// carries it: inlined, the pair put 126 instructions into `render_scene`, a function R0 counts,
/// and none of them runs on a frame that does not ask for the reflections. Out of line,
/// `render_scene` pays the one `is_some_and` that decides not to call this.
///
/// The scene scale of REQ-SCN-002 is 1 outside virtual reality, which Zaxnes has none of
/// (`SceneConstants::voxel_cell` and the occlusion's radius read it the same way).
#[inline(never)]
fn mark_floors(plan: &mut FramePlan<GpuMesh, Texture>, alignment: usize, local_camera: bool) {
    plan.mark_floor_surfaces(local_camera, 1.0);
    plan.lay_out_floor(alignment, STD140_PLACEMENT);
}

/// The floor-mask pass of AMD-055 REQ-SCN-084: the floor-eligible opaque runs redrawn into the
/// mask target the caller has bound, through the program whose fragment stage is the `+Y ≥ 0,9`
/// test of the floor definition.
///
/// It draws with `Passes::draw_run`, exactly as the opaque loop does, so the mask carries the
/// animated deformation, the placement, the curvature and the wall bias of AMD-057 of the frame it
/// masks — the mask cannot land anywhere the frame did not draw. The uniforms the mask program
/// does not have (the palette, the clip rectangle, the shadow map, the grid cell) are writes that
/// find nothing, the same way the five curvature uniforms are on a flat program.
fn draw_floor_mask(
    passes: &mut Passes,
    lists: &DrawLists<GpuMesh, Texture>,
    floor_runs: &[InstanceRun],
) {
    passes.scene.set_floor_mask(true);
    passes.scene.current().use_program();
    for run in floor_runs {
        passes.draw_run(&lists.opaque, run, true);
    }
    passes.scene.set_floor_mask(false);
}

/// The transparent objects of REQ-SCN-040 and AMD-012, farthest first, each with its own
/// depth-only pass.
///
/// Extracted from `render_scene` for the reason `draw_hud` was: that function is at clippy's line
/// ceiling, and this block is one self-contained step of REQ-SCN-058's order.
fn draw_transparent(
    passes: &mut Passes,
    common: &mut CommonUniforms<'_>,
    lists: &DrawLists<GpuMesh, Texture>,
) {
    passes.scene.current().use_program();
    common.bind(passes);
    let light = common.constants.light_direction;
    passes.scene.current().set_vec3("uLightDirection", light);
    passes.gl.enable(glow::BLEND);
    passes
        .gl
        .blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
    for group in &lists.transparent_objects {
        passes.draw_blended(
            &lists.transparent,
            &lists.transparent_runs[group.runs.0..group.runs.1],
        );
    }

    passes.gl.disable(glow::BLEND);
    passes.statistics.transparent_tiles = count(lists.transparent.len());
    passes.statistics.transparent_objects = count(lists.transparent_objects.len());
}

fn draw_hud(
    passes: &mut Passes,
    common: &mut CommonUniforms<'_>,
    lists: &DrawLists<GpuMesh, Texture>,
) {
    let (bent, gridded, snapped) = (
        passes.scene.bend,
        passes.scene.voxel_grid,
        passes.scene.pixel_shadow,
    );
    passes.scene.set_bend(false);
    passes.scene.set_voxel_grid(false);
    passes.scene.set_pixel_shadow(false);
    passes.scene.current().use_program();
    common.bind(passes);
    let light = common.constants.hud_light_direction;
    passes.scene.current().set_vec3("uLightDirection", light);
    passes.set_normal_sign(passes.mirrored_normal, true);
    passes.gl.enable(glow::BLEND);
    passes
        .gl
        .blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
    for group in &lists.hud_objects {
        passes.draw_blended(&lists.hud, &lists.hud_runs[group.runs.0..group.runs.1]);
    }

    passes.gl.disable(glow::BLEND);
    passes.statistics.hud_tiles = count(lists.hud.len());
    passes.scene.set_bend(bent);
    passes.scene.set_voxel_grid(gridded);
    passes.scene.set_pixel_shadow(snapped);
}

/// The uniforms every scene sub-pass binds again after `use_program`.
/// Gives every program that will draw this frame its frame uniforms, before any draw.
///
/// Both scene programs get them — the solid one is left in use, so the order of the draws is
/// kept — and so does the backdrop program of AMD-049 REQ-SCN-075 when this frame has one, which
/// additionally reads the eye. On a frame without a backdrop the second half is one boolean test.
fn bind_frame_uniforms(passes: &mut Passes, common: &mut CommonUniforms<'_>) {
    for solid in [false, true] {
        passes.use_solid(solid);
        passes.scene.current().use_program();
        common.bind(passes);

        // Q-069: the sign lives in the program, which starts at 0 — it has to be written at
        // least once per frame before anything is drawn.
        passes.set_normal_sign(false, true);
        let light = common.constants.light_direction;
        passes.scene.current().set_vec3("uLightDirection", light);
    }

    let backdrop_key = ProgramKey {
        pixel_shadow: false,
        backdrop: true,
        ..passes.scene.current_key()
    };
    let backdrop_program = passes.scene.backdrop_wanted && passes.scene.has(backdrop_key);
    if !backdrop_program {
        return;
    }

    passes.scene.set_backdrop(true);
    passes.scene.current().use_program();
    common.bind(passes);
    passes.set_normal_sign(false, true);
    let light = common.constants.light_direction;
    let eye = common.constants.eye;
    let program = passes.scene.current();
    program.set_vec3("uLightDirection", light);
    program.set_vec3("uEye", eye);

    // Back to the scene programs, so the opaque loop starts where it always did.
    passes.scene.set_backdrop(false);
    passes.scene.current().use_program();
}

/// Binds the frame's uniforms on the floor-mask program of AMD-055 REQ-SCN-084, when this frame
/// has one — which is only when `spl.reflections` = 1.
///
/// **Here and not in the pass**, for the reason the backdrop's are here: the mask program needs
/// the same once-a-frame values every scene program needs (the view projection, the curvature's
/// five constants, the deformation's), and binding them where [`CommonUniforms`] is already open
/// keeps that value out of the screen-pass block — passing it there cost `render_scene` 126
/// instructions of spilled matrices, which is the brief's point 1 measured.
///
/// `#[inline(never)]` for the same reason: inlined, this body is 116 instructions of
/// `render_scene` that only a frame with the reflections on ever runs.
#[inline(never)]
fn bind_mask_uniforms(passes: &mut Passes, common: &mut CommonUniforms<'_>, local_camera: bool) {
    if passes.scene.mask.is_none() {
        return;
    }

    passes.scene.set_floor_mask(true);
    passes.scene.current().use_program();
    common.bind(passes);
    passes.set_normal_sign(false, true);
    let up = crate::style::floor_vertical(local_camera);
    passes
        .scene
        .current()
        .set_vec3("uFloorUp", Vector3::new(up[0], up[1], up[2]));

    passes.scene.set_floor_mask(false);
    passes.scene.current().use_program();
}

struct CommonUniforms<'a> {
    constants: SceneConstants,
    textures: &'a mut TextureCache,
    linear_palette: Texture,
    shadow_texture: Texture,
}

impl CommonUniforms<'_> {
    fn bind(&mut self, passes: &mut Passes) {
        let gridded = passes.scene.voxel_grid;
        let (gl, program) = (&passes.gl, passes.scene.current());
        let constants = &self.constants;

        // REQ-SCN-067: the per-axis constants, which the CPU computed when the setting changed.
        // The flat variant has no such uniform, so on a program that does not bend these are
        // five lookups that find nothing — and they only run at all when a curvature is set.
        if !constants.bend.is_identity() {
            bind_bend(program, constants.bend);
        }

        // AMD-055 REQ-SCN-081: the cell side, uploaded only while the grid variant is in use —
        // the programs without it have no such uniform, and a frame at `spl.voxel_grid` = 0
        // never reaches this line.
        if gridded {
            program.set_f32("uVoxelCell", constants.voxel_cell);
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
        program.set_i32("uLinearPalette", 4);

        gl.active_texture(2);
        gl.bind_texture(glow::TEXTURE_2D, Some(self.textures.palette()));
        gl.active_texture(3);
        gl.bind_texture(glow::TEXTURE_2D, Some(self.shadow_texture));
        gl.active_texture(4);
        gl.bind_texture(glow::TEXTURE_2D, Some(self.linear_palette));
        gl.active_texture(0);
        passes.forget_bindings();
    }
}

impl Passes {
    /// Puts the solid variant of [`shaders::scene`] in use when `solid`, the plain one
    /// otherwise. Both draw the same pixels where the solid one may be used: it only leaves out
    /// the `discard`s a draw that neither clips nor reads a colour texture never reaches. The
    /// normal sign is written again, the two programs may hold different ones.
    fn use_solid(&mut self, solid: bool) {
        if solid == self.scene.solid {
            return;
        }
        self.scene.solid = solid;
        self.scene.current().use_program();
        self.set_normal_sign(self.mirrored_normal, true);
    }

    /// The shadow program of the curvature in use (REQ-SCN-068).
    fn shadow_program(&mut self) -> &mut ShaderProgram {
        match (self.scene.bend, &mut self.shadow_bent) {
            (true, Some(bent)) => bent,
            _ => &mut self.shadow,
        }
    }

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
            .current()
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

        // AMD-057 / REQ-SCN-065: read before the program is borrowed.
        let wall_bias = self.wall_clip_bias(item.mesh);
        // AMD-055 REQ-SCN-082: whether the voxel-grid offset of `z` has to be uploaded for this
        // run. Read from the cached flag of the variant in use — before the program is borrowed —
        // so Classic pays one boolean test per **run** and no upload.
        let snapping = self.scene.pixel_shadow;
        let program = self.scene.current();
        program.set_i32("uColorMode", item.color_mode);
        program.set_i32("uPaletteTable", item.palette_table);
        program.set_vec2("uTextureSize", item.texture_size);
        program.set_vec2("uTexOffset", item.tex_offset);
        program.set_vec2("uTexTilting", item.tex_tilting);
        program.set_vec3("uFlatColor", item.flat_color);
        program.set_f32("uAlpha", if opaque { 1.0 } else { item.alpha });
        program.set_i32("uClip", i32::from(item.clip));
        // AMD-057 / REQ-SCN-065: the inner walls of a tile box are pushed behind the faces they
        // meet. 0 for a mesh whose normals are not axis aligned (MARCHING merges its tiles, so
        // it has no joint to hide and the `|normal.z|` test would be meaningless there).
        program.set_f32("uWallClipBias", wall_bias);
        if snapping {
            program.set_f32(
                "uVoxelZOffset",
                crate::style::voxel_z_offset(item.mesh.build_depth),
            );
        }
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

    /// The clip-space wall bias a draw of `mesh` takes (AMD-057): the bias of
    /// [`geometry::WALL_DEPTH_BIAS`] for a box of axis-aligned faces, 0 otherwise.
    ///
    /// `wall_bias` is the scene distance in force; a test sets it to 0 to show that the bias is
    /// what holds REQ-SCN-065 with the walls kept.
    fn wall_clip_bias(&self, mesh: GpuMesh) -> f32 {
        if mesh.axis_aligned_normals {
            self.wall_bias
        } else {
            0.0
        }
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

/// Uploads the five per-axis constants of the curvature (REQ-SCN-067 step 1) to `program`.
///
/// They are computed on the CPU when a setting changes — `SpaceBend::new` — so a vertex pays
/// only for the wrapping itself. On a program without the curvature compiled in the five
/// uniforms are not active and the writes find nothing; the caller only runs this when the
/// curvature bends something at all.
fn bind_bend(program: &mut ShaderProgram, bend: SpaceBend) {
    program.set_vec3("uBendK", bend.curvature().into());
    program.set_vec3("uBendF", bend.recentring().into());
    program.set_vec3("uBendH", bend.offset().into());
    program.set_vec3("uBendOn", bend.active().into());
    program.set_f32("uBendPivot", bend.arc_pivot());
}

/// The `[plain, solid]` pair of scene programs of one variant, each with its instance block
/// bound (the block is the placements of a batched draw, SPEC-06 §3.9).
fn scene_pair(
    gl: &Gl,
    srgb_palette: bool,
    key: ProgramKey,
) -> Result<[ShaderProgram; 2], RenderError> {
    let variant = SceneVariant {
        solid: false,
        srgb_palette,
        bend: key.bend,
        centroid: key.centroid,
        voxel_grid: key.voxel_grid,
        pixel_shadow: key.pixel_shadow,
    };
    let one = |solid: bool| -> Result<ShaderProgram, RenderError> {
        let program = ShaderProgram::create(gl, &shaders::scene(variant.with_solid(solid)))?;
        program.bind_uniform_block("ZaxInstances", INSTANCE_BINDING);
        Ok(program)
    };
    Ok([one(false)?, one(true)?])
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

/// The four terms of the projection the screen-space passes of AMD-055 REQ-SCN-080 read: the two
/// scales, the `range` term and the near plane.
///
/// Four floats rather than the inverse matrix, because the camera of SPEC-06 §3.3 builds one
/// symmetric perspective and nothing else (`Matrix4x4::create_perspective_field_of_view`): `m11`
/// and `m22` *are* its two scales, `m33` is `range = far/(near − far)` and `m43` is `range × near`,
/// so the near plane is their quotient. Reading them off the matrix rather than plumbing the
/// camera's field of view through keeps this right if the camera ever changes its planes.
fn projection_terms(view: &CameraView) -> Vector4 {
    let projection = &view.projection;
    let range = projection.m33;
    let near = if range.abs() > f32::EPSILON {
        projection.m43 / range
    } else {
        0.0
    };
    Vector4::new(projection.m11, projection.m22, range, near)
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
