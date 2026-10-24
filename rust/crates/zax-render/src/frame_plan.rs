//! What one frame draws, worked out without any GPU call (REQ-SCN-058): the four buckets of the
//! tile list, the objects of the blended ones sorted by distance, the instanced runs and their
//! placements, the volume of the shadow map and the constants every scene pass binds. A backend
//! turns the plan into draw calls; the plan is generic over the handles of the meshes and the
//! textures the backend made resident, so that it compiles to the same code for each of them.

use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;

use zax_engine3d::draw_list::{DeformParameters, DrawTile, MaterialClass, SceneContainer};
use zax_engine3d::frame::FrameInstanceId;
use zax_engine3d::geometry::texture::ObjectTexture;
use zax_engine3d::geometry::tile_mesh::TileMesh;
use zax_engine3d::hashing::FastMap;
use zax_engine3d::vectors::{Mat4, Vec3};
use zax_profiles::enums::TextureFormat;

use crate::camera::CameraView;
use crate::color;
use crate::geometry;
use crate::interop;
use crate::numerics::{Matrix4x4, Vector2, Vector3, Vector4};
use crate::occlusion::{self, Occlusion};
use crate::scene::{SceneFrame, SceneState};
use crate::shadow_volume::ShadowVolume;

/// How a backend lays one placement out in its instance data. A type of its own per layout, so
/// that the layout code is compiled for each backend with the write inlined.
pub trait Placement: Copy {
    /// Appends the placement of an item from its model matrix and deformation.
    fn write(self, data: &mut Vec<u8>, model: &Mat4, deform: Option<&DeformParameters>);
}

/// The std140 `ZaxInstance` of the GL scene shader: the model matrix, the three columns of the
/// normal matrix and the deformation's tile position.
#[derive(Debug, Clone, Copy)]
pub struct Std140Placement;

impl Placement for Std140Placement {
    #[inline]
    fn write(self, data: &mut Vec<u8>, model: &Mat4, deform: Option<&DeformParameters>) {
        push_std140_placement(data, model, deform);
    }
}

/// The placements of the GL path.
pub const STD140_PLACEMENT: Std140Placement = Std140Placement;

/// Instances one draw call carries, the size of `uInstances` in `scene.vert`.
pub const INSTANCES_PER_DRAW: usize = 128;
/// Bytes of one placement in the instance block (std140: a `mat4` and four `vec4`).
pub const INSTANCE_BYTES: usize = 128;

/// Where a backend keeps the GPU copies of the meshes, the textures and the NES picture: the
/// plan asks for them while it collects the tiles, which uploads what is not resident yet.
pub trait Residency {
    /// A resident mesh: equal handles, same mesh.
    type Mesh: Copy + Eq + Hash + Debug;
    /// A resident texture: equal handles, same texture.
    type Texture: Copy + Eq + Hash + Debug;
    /// Why a resource could not be made resident.
    type Error;

    /// The resident copy of `mesh`, or `None` when it cannot be drawn.
    ///
    /// # Errors
    ///
    /// When the GPU object cannot be created.
    fn mesh(&mut self, mesh: &Arc<TileMesh>) -> Result<Option<Self::Mesh>, Self::Error>;

    /// The resident copy of `texture`.
    ///
    /// # Errors
    ///
    /// When the GPU object cannot be created.
    fn texture(&mut self, texture: &Arc<ObjectTexture>) -> Result<Self::Texture, Self::Error>;

    /// The NES picture of the 2D mode (256 × 240 colour indices) uploaded, or `None` when it has
    /// the wrong size.
    ///
    /// # Errors
    ///
    /// When the GPU object cannot be created.
    fn picture(
        &mut self,
        picture: &[u8],
        height: usize,
    ) -> Result<Option<Self::Texture>, Self::Error>;
}

/// One tile, backdrop or picture quad ready to draw.
#[derive(Debug, Clone, Copy)]
pub struct DrawItem<M, T> {
    pub mesh: M,
    pub model: Mat4,
    /// 0 indexed texture, 1 direct texture, 2 flat colour.
    pub color_mode: i32,
    pub texture: Option<T>,
    pub texture_size: Vector2,
    pub palette_table: i32,
    pub tex_offset: Vector2,
    pub tex_tilting: Vector2,
    pub flat_color: Vector3,
    pub alpha: f32,
    pub receive_shadow: bool,
    pub clip: bool,
    pub deform: Option<DeformParameters>,
    /// The shape instance the tile belongs to (SPEC-04B REQ-E3D-641), the object the depth pass
    /// and the sort of AMD-012 work on. `None` for the backdrop and the 2D-mode quad, which are
    /// single objects anyway.
    pub instance: Option<FrameInstanceId>,
    /// Local bounding box of the mesh (REQ-E3D-640).
    pub bounds_min: Vec3,
    pub bounds_max: Vec3,
}

impl<M, T> DrawItem<M, T> {
    /// A flat or textured item with the neutral texture parameters.
    fn plain(mesh: M, model: Mat4, bounds: (Vec3, Vec3)) -> Self {
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
pub struct DrawGroup {
    start: usize,
    end: usize,
    view_depth: f32,
    /// The instance runs of the group, a range of the bucket's runs.
    pub runs: (usize, usize),
}

/// Consecutive items of a bucket that share every setting but their placement
/// ([`DrawStateKey`]), drawn with one instanced call; `offset` is where their placements start
/// in the instance data.
#[derive(Debug, Clone, Copy)]
pub struct InstanceRun {
    /// The bucket index of the first item, whose state every item of the run shares.
    pub first: usize,
    /// How many items the run draws.
    pub count: usize,
    pub offset: usize,
}

/// The four buckets of REQ-SCN-058 and the objects of the two blended ones.
#[derive(Debug)]
pub struct DrawLists<M, T> {
    pub opaque: Vec<DrawItem<M, T>>,
    pub transparent: Vec<DrawItem<M, T>>,
    pub hud: Vec<DrawItem<M, T>>,
    pub casters: Vec<DrawItem<M, T>>,
    pub transparent_objects: Vec<DrawGroup>,
    pub hud_objects: Vec<DrawGroup>,
    pub opaque_runs: Vec<InstanceRun>,
    pub transparent_runs: Vec<InstanceRun>,
    pub hud_runs: Vec<InstanceRun>,
}

impl<M, T> Default for DrawLists<M, T> {
    fn default() -> Self {
        Self {
            opaque: Vec::new(),
            transparent: Vec::new(),
            hud: Vec::new(),
            casters: Vec::new(),
            transparent_objects: Vec::new(),
            hud_objects: Vec::new(),
            opaque_runs: Vec::new(),
            transparent_runs: Vec::new(),
            hud_runs: Vec::new(),
        }
    }
}

impl<M, T> DrawLists<M, T> {
    fn clear(&mut self) {
        self.opaque.clear();
        self.transparent.clear();
        self.hud.clear();
        self.casters.clear();
        self.transparent_objects.clear();
        self.hud_objects.clear();
        self.opaque_runs.clear();
        self.transparent_runs.clear();
        self.hud_runs.clear();
    }

    /// Whether nothing of the scene is drawn (the skybox aside).
    #[must_use]
    pub fn nothing_to_draw(&self) -> bool {
        self.opaque.is_empty() && self.transparent.is_empty() && self.hud.is_empty()
    }
}

/// The plan of one frame, reused from frame to frame.
#[derive(Debug)]
pub struct FramePlan<M, T> {
    pub lists: DrawLists<M, T>,
    /// The placements of every scene draw of the frame, in the std140 layout of `ZaxInstance`.
    pub instance_data: Vec<u8>,
    /// Opaque tiles drawn transparent this frame because they hide the target (REQ-SCN-022).
    pub occluders: u32,
    /// The casters cut into instanced runs ([`Self::lay_out_casters`]), and their placements
    /// (the wgpu backend's; the GL path draws the casters one by one).
    #[cfg_attr(not(feature = "wgpu"), allow(dead_code))]
    pub caster_runs: Vec<InstanceRun>,
    #[cfg_attr(not(feature = "wgpu"), allow(dead_code))]
    pub caster_data: Vec<u8>,
    scratch: OpaqueScratch<M, T>,
}

impl<M, T> Default for FramePlan<M, T> {
    fn default() -> Self {
        Self {
            lists: DrawLists::default(),
            instance_data: Vec::new(),
            occluders: 0,
            caster_runs: Vec::new(),
            caster_data: Vec::new(),
            scratch: OpaqueScratch::default(),
        }
    }
}

/// The fixed meshes the plan places itself: the backdrop box and the quad of the 2D mode.
#[derive(Debug, Clone, Copy)]
pub struct FixedMeshes<'a> {
    pub unit_box: &'a Arc<TileMesh>,
    pub picture_quad: &'a Arc<TileMesh>,
    /// The quad of a widescreen 2D-mode picture, made for the margin of the displayed picture
    /// (`interop::wide_quad_for`); `None` when the picture is 256 pixels wide.
    pub wide_picture_quad: Option<&'a Arc<TileMesh>>,
}

impl<M: Copy + Eq + Hash + Debug, T: Copy + Eq + Hash + Debug> FramePlan<M, T> {
    /// Turns the tile list of REQ-E3D-641 into the four buckets of REQ-SCN-058, applying the
    /// renderer-side sprite nudge of REQ-SCN-037 and the material rules of REQ-SCN-040/041.
    ///
    /// # Errors
    ///
    /// When `residency` cannot make a mesh or a texture resident.
    pub fn collect<R: Residency<Mesh = M, Texture = T>>(
        &mut self,
        residency: &mut R,
        fixed: FixedMeshes<'_>,
        scene: &SceneState,
        frame: &SceneFrame<'_>,
        view: &CameraView,
    ) -> Result<(), R::Error> {
        self.lists.clear();
        self.occluders = 0;

        // REQ-SCN-047: outside a running game the 3D scene is not displayed at all.
        if !scene.game_running {
            return Ok(());
        }

        // AMD-009 (Q-022): in 2D mode the core's pixel frame is shown on the NES plane instead
        // of a recomposition of the tile records.
        if scene.two_dimensional_mode
            && self.collect_picture_quad(
                residency,
                fixed,
                frame.nes_picture,
                frame.nes_picture_height,
            )?
        {
            return Ok(());
        }

        if let Some(list) = frame.draw_list {
            let hud_matrix = interop::hud_matrix(view);
            let occlusion = Occlusion::for_scene(scene, view);
            for tile in list.tiles() {
                self.collect_tile(residency, tile, &hud_matrix, &occlusion)?;
            }
        }

        if scene.backdrop.enabled {
            self.collect_backdrop(residency, fixed.unit_box, scene)?;
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
    fn collect_picture_quad<R: Residency<Mesh = M, Texture = T>>(
        &mut self,
        residency: &mut R,
        fixed: FixedMeshes<'_>,
        picture: &[u8],
        height: usize,
    ) -> Result<bool, R::Error> {
        let Some((width, height)) = crate::interop::picture_size(picture.len(), height) else {
            return Ok(false);
        };
        let quad = if (width, height) == (256, 240) {
            fixed.picture_quad
        } else {
            match fixed.wide_picture_quad {
                Some(quad) => quad,
                None => return Ok(false),
            }
        };
        let Some(texture) = residency.picture(picture, height)? else {
            return Ok(false);
        };
        let Some(mesh) = residency.mesh(quad)? else {
            return Ok(false);
        };

        self.lists.opaque.push(DrawItem {
            color_mode: 1,
            texture: Some(texture),
            #[allow(clippy::cast_precision_loss)] // 256 to 1280, 240 to 720.
            texture_size: Vector2::new(width as f32, height as f32),
            // Nothing else is in the scene in 2D mode, so the quad neither casts nor receives.
            receive_shadow: false,
            clip: true,
            ..DrawItem::plain(mesh, Mat4::IDENTITY, (quad.bounds_min(), quad.bounds_max()))
        });
        Ok(true)
    }

    fn collect_tile<R: Residency<Mesh = M, Texture = T>>(
        &mut self,
        residency: &mut R,
        tile: &DrawTile,
        hud_matrix: &Mat4,
        occlusion: &Occlusion,
    ) -> Result<(), R::Error> {
        let Some(mesh) = tile.mesh.as_ref().filter(|mesh| !mesh.is_empty()) else {
            return Ok(());
        };
        let Some(resident) = residency.mesh(mesh)? else {
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
            self.occluders += 1;
            MaterialClass::Transparent
        } else {
            tile.material
        };

        let texture = match &tile.texture {
            Some(texture) => Some(residency.texture(texture)?),
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

    fn collect_backdrop<R: Residency<Mesh = M, Texture = T>>(
        &mut self,
        residency: &mut R,
        unit: &Arc<TileMesh>,
        scene: &SceneState,
    ) -> Result<(), R::Error> {
        let backdrop = &scene.backdrop;
        let Some(resident) = residency.mesh(unit)? else {
            return Ok(());
        };

        let center = geometry::CENTER + backdrop.offset;
        let model = Mat4::translation(center.into()) * Mat4::scale_matrix(backdrop.size.into());
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

    /// The light's view of the casters (REQ-SCN-030), or [`ShadowVolume::EMPTY`] when the
    /// shadows are off or nothing casts.
    #[must_use]
    pub fn shadow_volume(&self, scene: &SceneState, view: &CameraView) -> ShadowVolume {
        if !scene.settings.shadows || self.lists.casters.is_empty() {
            return ShadowVolume::EMPTY;
        }

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
        if volume.has_content {
            volume
        } else {
            ShadowVolume::EMPTY
        }
    }

    /// Lays the placements of every bucket out in [`Self::instance_data`], cut into runs of
    /// consecutive items with the same draw state (never across the objects of the two blended
    /// buckets, whose two passes go object by object), each run starting at a multiple of
    /// `alignment`. With `group_opaque`, the opaque bucket and the tiles of each transparent
    /// object are reordered by draw state first ([`OpaqueScratch::order_range`]).
    pub fn lay_out_instances(
        &mut self,
        group_opaque: bool,
        alignment: usize,
        placement: impl Placement,
    ) {
        let lists = &mut self.lists;
        let scratch = &mut self.scratch;
        let data = &mut self.instance_data;
        data.clear();

        // Opaque: one object, the whole bucket.
        scratch.label(&lists.opaque);
        scratch.order_range(0, lists.opaque.len(), group_opaque);
        add_runs(
            &lists.opaque,
            scratch,
            &mut lists.opaque_runs,
            data,
            (alignment, placement),
        );

        // Blended objects: the runs never cross an object; the tiles of a transparent object
        // may be grouped, the HUD's never (its tiles share one plane, an overlap is a tie).
        scratch.label(&lists.transparent);
        for group in &mut lists.transparent_objects {
            let first = lists.transparent_runs.len();
            scratch.order_range(group.start, group.end, group_opaque);
            add_runs(
                &lists.transparent,
                scratch,
                &mut lists.transparent_runs,
                data,
                (alignment, placement),
            );
            group.runs = (first, lists.transparent_runs.len());
        }
        scratch.label(&lists.hud);
        for group in &mut lists.hud_objects {
            let first = lists.hud_runs.len();
            scratch.order_range(group.start, group.end, false);
            add_runs(
                &lists.hud,
                scratch,
                &mut lists.hud_runs,
                data,
                (alignment, placement),
            );
            group.runs = (first, lists.hud_runs.len());
        }
    }
}

impl<M: Copy + Eq, T> FramePlan<M, T> {
    /// Cuts the casters into runs of at most [`INSTANCES_PER_DRAW`] consecutive casters that
    /// share their mesh, their deformation but its tile position, and their side of the mirror,
    /// and lays their placements out in [`Self::caster_data`] (each run at a multiple of
    /// `alignment`). The casters keep their order, so the depth map is the same as drawn one by
    /// one.
    #[cfg_attr(not(feature = "wgpu"), allow(dead_code))]
    pub fn lay_out_casters(&mut self, alignment: usize, placement: impl Placement) {
        let casters = &self.lists.casters;
        let (runs, data) = (&mut self.caster_runs, &mut self.caster_data);
        runs.clear();
        data.clear();
        let key = |item: &DrawItem<M, T>| {
            (
                item.mesh,
                interop::determinant(&item.model) < 0.0,
                item.deform.as_ref().map(deform_bits),
            )
        };
        let mut first = 0;
        while first < casters.len() {
            let shared = key(&casters[first]);
            let mut last = first + 1;
            while last < casters.len()
                && last - first < INSTANCES_PER_DRAW
                && key(&casters[last]) == shared
            {
                last += 1;
            }

            let offset = data.len().next_multiple_of(alignment);
            data.resize(offset, 0);
            for item in &casters[first..last] {
                placement.write(data, &item.model, item.deform.as_ref());
            }
            runs.push(InstanceRun {
                first,
                count: last - first,
                offset,
            });
            first = last;
        }
    }
}

/// The values every scene sub-pass binds (the uniforms of `scene.vert` and `scene.frag` that do
/// not change from one draw to the next).
#[derive(Debug, Clone, Copy)]
pub struct SceneConstants {
    pub view_projection: Matrix4x4,
    pub time: f32,
    pub ambient: f32,
    pub shadow_strength: f32,
    pub light_linear: Vector3,
    pub clip_rect: Vector4,
    pub light_view_projection: Matrix4x4,
    pub shadow_bias: f32,
    pub normal_offset: f32,
    /// Whether the shadow map holds anything this frame.
    pub shadow_enabled: bool,
    /// The light direction of the scene buckets, and the one of the HUD, lit in the camera
    /// frame (REQ-SCN-029, REQ-SCN-044).
    pub light_direction: Vector3,
    pub hud_light_direction: Vector3,
}

impl SceneConstants {
    /// The constants of a frame of `scene` seen through `view`, with the shadow `volume` drawn
    /// into a map of `shadow_map_size` texels.
    #[must_use]
    pub fn new(
        scene: &SceneState,
        view: &CameraView,
        volume: &ShadowVolume,
        shadow_map_size: i32,
    ) -> Self {
        let rect = scene.visible_rectangle();
        // The depth bias of the shadow test, in scene units: the 0.02 unit of REQ-SCN-031,
        // raised to one texel of the current map when a texel is coarser than that (Q-032,
        // amended by AMD-034 §2). Q-025 fills the map with back faces only, and the wall that
        // closes a tile against its neighbour is a back face too, inside the solid: a receiver
        // one texel from a joint reads it and darkens (the seam of REQ-SCN-065). The error is
        // proportional to the texel, so the bias has to be too.
        let shadow_bias = if volume.has_content {
            geometry::shadow_bias(volume.texel_size(shadow_map_size)) / volume.depth_range
        } else {
            0.0
        };
        Self {
            view_projection: view.view_projection(),
            time: scene.emulated_time,
            ambient: color::ambient_linear(),
            shadow_strength: color::SHADOW_FACTOR,
            light_linear: color::rgb_to_linear(scene.light.color)
                * color::to_linear(scene.settings.light_intensity()),
            clip_rect: Vector4::new(rect.min_x, rect.min_y, rect.max_x, rect.max_y),
            light_view_projection: volume.view_projection,
            shadow_bias,
            normal_offset: geometry::SHADOW_NORMAL_OFFSET,
            shadow_enabled: volume.has_content,
            light_direction: scene.light.direction,
            hud_light_direction: view.hud_light_direction(scene.light.direction),
        }
    }
}

/// Cuts `items` into the objects of AMD-012 — the runs of tiles that share one shape instance,
/// which SPEC-04B REQ-E3D-641 delivers consecutively — and measures each one by the distance
/// from the eye to the centre of its transformed bounding box.
fn group_by_object<M, T>(items: &[DrawItem<M, T>], groups: &mut Vec<DrawGroup>, eye: Vector3) {
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
            runs: (0, 0),
        });
        start = end;
    }
}

/// The program state of a draw item as a hashable key: two items with equal keys can go out in
/// one instanced call (everything but the placement and the deformation's tile position, and
/// the same side of the mirror for Q-069's winding and normal sign).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct DrawStateKey<M, T> {
    mesh: M,
    texture: Option<T>,
    color_mode: i32,
    palette_table: i32,
    /// Texture size, offset, tilting, flat colour and alpha, as bits.
    floats: [u32; 10],
    receive_shadow: bool,
    clip: bool,
    mirrored: bool,
    /// The deformation but its tile position, which travels with the placement: amplitude,
    /// size, speed and phase, as bits.
    deform: Option<[u32; 38]>,
}

impl<M: Copy, T: Copy> DrawStateKey<M, T> {
    fn of(item: &DrawItem<M, T>) -> Self {
        Self {
            mesh: item.mesh,
            texture: item.texture,
            color_mode: item.color_mode,
            palette_table: item.palette_table,
            floats: [
                item.texture_size.x.to_bits(),
                item.texture_size.y.to_bits(),
                item.tex_offset.x.to_bits(),
                item.tex_offset.y.to_bits(),
                item.tex_tilting.x.to_bits(),
                item.tex_tilting.y.to_bits(),
                item.flat_color.x.to_bits(),
                item.flat_color.y.to_bits(),
                item.flat_color.z.to_bits(),
                item.alpha.to_bits(),
            ],
            receive_shadow: item.receive_shadow,
            clip: item.clip,
            mirrored: interop::normal_matrix(&item.model).1 < 0.0,
            deform: item.deform.as_ref().map(deform_bits),
        }
    }
}

/// The bits of every deformation parameter but the tile position.
fn deform_bits(deform: &DeformParameters) -> [u32; 38] {
    let mut bits = [0; 38];
    let vectors = [deform.amplitude, deform.size]
        .into_iter()
        .flat_map(|vector| [vector.x, vector.y, vector.z]);
    let cells = |matrix: Mat4| (0..16).map(move |cell| matrix[(cell / 4, cell % 4)]);
    let all = vectors
        .chain(cells(deform.speed))
        .chain(cells(deform.phase));
    for (slot, cell) in bits.iter_mut().zip(all) {
        *slot = cell.to_bits();
    }
    bits
}

/// The draw state labels of a bucket and the drawing order of one object of it, reused from
/// frame to frame: the items themselves never move.
#[derive(Debug)]
struct OpaqueScratch<M, T> {
    groups: FastMap<DrawStateKey<M, T>, u32>,
    /// The draw state label of each item of the bucket: equal labels, equal state.
    labels: Vec<u32>,
    /// The bucket indices of the object being cut into runs, in drawing order.
    order: Vec<u32>,
    sort: Vec<(u32, u32)>,
    /// The rank of each state inside the object being ordered.
    ranks: FastMap<u32, u32>,
}

impl<M, T> Default for OpaqueScratch<M, T> {
    fn default() -> Self {
        Self {
            groups: FastMap::default(),
            labels: Vec::new(),
            order: Vec::new(),
            sort: Vec::new(),
            ranks: FastMap::default(),
        }
    }
}

impl<M: Copy + Eq + Hash, T: Copy + Eq + Hash> OpaqueScratch<M, T> {
    /// Labels every item of `items` with its draw state (one key per item).
    fn label(&mut self, items: &[DrawItem<M, T>]) {
        self.groups.clear();
        self.labels.clear();
        for item in items {
            let next = u32::try_from(self.groups.len()).unwrap_or(u32::MAX);
            self.labels
                .push(*self.groups.entry(DrawStateKey::of(item)).or_insert(next));
        }
    }

    /// The drawing order of the object `start..end` of the labelled bucket: list order, or,
    /// with `group`, the items of one state together (states in the order of their first item,
    /// list order inside a state; Rust-only, draw-call batching).
    ///
    /// Opaque items are drawn with the depth test and without blending, so the picture does not
    /// depend on their order; the tiles of one blended object go through a depth-only pass, then
    /// a colour pass where the depth is equal, so inside an object the order does not matter
    /// either. Only exact depth ties between two different states could come out differently.
    fn order_range(&mut self, start: usize, end: usize, group: bool) {
        self.order.clear();
        let index = |position: usize| u32::try_from(position).unwrap_or(u32::MAX);
        if !group {
            self.order.extend((start..end).map(index));
            return;
        }

        // States numbered by first appearance inside the object, so that the order does not
        // depend on the label numbering of the whole bucket.
        self.sort.clear();
        self.ranks.clear();
        for position in start..end {
            let next = index(self.ranks.len());
            let rank = *self.ranks.entry(self.labels[position]).or_insert(next);
            self.sort.push((rank, index(position)));
        }
        self.sort.sort_unstable();
        self.order
            .extend(self.sort.iter().map(|&(_, position)| position));
    }
}

/// Cuts the object `scratch.order` of `items` into runs of at most [`INSTANCES_PER_DRAW`]
/// consecutive items (in drawing order) with the same draw state label, appends their placements
/// to `data` (each run at a multiple of `alignment`) and the runs to `runs`.
fn add_runs<M, T>(
    items: &[DrawItem<M, T>],
    scratch: &OpaqueScratch<M, T>,
    runs: &mut Vec<InstanceRun>,
    data: &mut Vec<u8>,
    (alignment, placement): (usize, impl Placement),
) {
    let order = &scratch.order;
    let mut first = 0;
    while first < order.len() {
        let label = scratch.labels[order[first] as usize];
        let mut last = first + 1;
        while last < order.len()
            && last - first < INSTANCES_PER_DRAW
            && scratch.labels[order[last] as usize] == label
        {
            last += 1;
        }

        let offset = data.len().next_multiple_of(alignment);
        data.resize(offset, 0);
        for &position in &order[first..last] {
            let item = &items[position as usize];
            placement.write(data, &item.model, item.deform.as_ref());
        }
        runs.push(InstanceRun {
            first: order[first] as usize,
            count: last - first,
            offset,
        });
        first = last;
    }
}

/// Appends one placement in the std140 layout of `ZaxInstance`: the model matrix as it was
/// uploaded to `uModel` (column by column), the three columns of the normal matrix, then the
/// deformation's tile position (zero without deformation).
fn push_std140_placement(data: &mut Vec<u8>, model: &Mat4, deform: Option<&DeformParameters>) {
    for cell in interop::to_numerics(model).to_cells() {
        data.extend_from_slice(&cell.to_ne_bytes());
    }

    let (m, _) = interop::normal_matrix(model);
    let tile = deform.map_or(Vec3::ZERO, |deform| deform.tile_relative_to_pivot);
    let rows = [
        [m[0], m[3], m[6]],
        [m[1], m[4], m[7]],
        [m[2], m[5], m[8]],
        [tile.x, tile.y, tile.z],
    ];
    for row in rows {
        for cell in row {
            data.extend_from_slice(&cell.to_ne_bytes());
        }
        data.extend_from_slice(&0.0f32.to_ne_bytes());
    }
}
