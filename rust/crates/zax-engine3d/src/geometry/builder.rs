// Portions adapted from 3DNes_Engine (c) Geod Studio, GPL-3.0

//! The geometry side of the engine (SPEC-04B §3.2 to §3.7) and the interface the conversion
//! pipeline drives it through.

use crate::hashing::FastMap;
use std::any::Any;
use std::sync::Arc;

use crate::geometry::greedy;
use crate::geometry::marching;
use crate::geometry::object_geometry::{BuiltObject, ObjectGeometry};
use crate::geometry::pattern_voxels::PatternVoxels;
use crate::geometry::texture::ObjectTexture;
use crate::geometry::tile_mesh::TileMesh;
use crate::geometry::tile_sides::TileSides;
use crate::geometry::tile_voxels::{self, TileVoxels, MAX_DEPTH};
use crate::geometry::voxel_construction;
use crate::image::TileImage;
use crate::instance::{InstanceRef, PatternInstance};
use crate::palette::{nes_color, Rgb24};
use crate::pattern::EnginePattern;
use crate::registry::PatternId;
use crate::settings::RenderAlgorithm;

/// How far a mass build of geometry has got (REQ-E3D-362, SPEC-04B REQ-E3D-600).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GeometryProgress {
    /// 3D objects whose voxel field and meshes are computed.
    pub built: usize,
    /// 3D objects whose meshes have reached the renderer.
    pub transferred: usize,
    /// 3D objects the current build has to produce; 0 outside a build.
    pub to_build: usize,
}

impl GeometryProgress {
    /// Creates a progress report.
    #[must_use]
    pub const fn new(built: usize, transferred: usize, to_build: usize) -> Self {
        Self {
            built,
            transferred,
            to_build,
        }
    }

    /// The loading progress `(built + transferred) / (2 × toBuild)`, and 1 outside a build or
    /// when there is nothing to build (REQ-E3D-362).
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // C# int-to-float conversions of object counts.
    pub fn value(&self) -> f32 {
        if self.to_build == 0 {
            1.0
        } else {
            ((self.built + self.transferred) as f32 / (2.0 * self.to_build as f32)).clamp(0.0, 1.0)
        }
    }
}

/// The geometry side of the engine, implemented against SPEC-04B: the conversion pipeline
/// only decides *when* a pattern's geometry is (re)built and reads back whether it is ready.
///
/// The pipeline calls [`PatternGeometryBuilder::build`] at profile load, when a pattern is
/// learned (REQ-E3D-161) and for every pattern when the global mesh algorithm changes
/// (REQ-E3D-340), always *after* the `BuildPattern` hooks (REQ-E3D-322). A shape whose
/// geometry is not ready stays in the frame and is seen by scripts but is not drawn
/// (REQ-E3D-361, SPEC-04B REQ-E3D-601).
///
/// `Send`: the engine, builder included, may be handed to another thread for the length of a
/// frame (the host's pipelined frames).
pub trait PatternGeometryBuilder: Send {
    /// Builds, or rebuilds, the geometry of every instance of pattern `id`. `global` is the
    /// global mesh algorithm an instance with `rm = 0` takes (REQ-E3D-500). The build may
    /// write the effective depth back into the instances (REQ-E3D-452).
    fn build(&mut self, id: PatternId, pattern: &mut EnginePattern, global: RenderAlgorithm);

    /// Frees the meshes and the texture space of a pattern (SPEC-04B REQ-E3D-603).
    fn release(&mut self, id: PatternId, pattern: &EnginePattern);

    /// Whether the meshes of a pattern instance can be drawn (REQ-E3D-361).
    fn is_ready(&self, instance: InstanceRef) -> bool;

    /// The mesh of `image` for `instance` (SPEC-04B REQ-E3D-640), or `None` when it is not
    /// available. AMD-057: this is the mesh of REQ-E3D-501, every wall included — the rendering
    /// variants of Q-067 are withdrawn.
    fn mesh_of(&self, instance: InstanceRef, image: &TileImage) -> Option<Arc<TileMesh>>;

    /// The mesh of `image` for `instance` with the lateral faces AMD-063 REQ-SCN-087 removes
    /// taken off. A builder that knows no full tile answers the whole mesh, as AMD-057 draws it.
    fn mesh_of_without(
        &mut self,
        instance: InstanceRef,
        image: &TileImage,
        _removed: TileSides,
    ) -> Option<Arc<TileMesh>> {
        self.mesh_of(instance, image)
    }

    /// Whether the tile `image` of `instance` is **full** in the sense of AMD-063 REQ-SCN-087.
    /// False for a builder with no voxel field to answer from, which then removes nothing.
    fn is_full_tile(&self, _instance: InstanceRef, _image: &TileImage) -> bool {
        false
    }

    /// The resolved texture of a pattern instance (SPEC-04B REQ-E3D-426, REQ-E3D-641), or
    /// `None` when its geometry has not been built.
    fn texture_of(&self, instance: InstanceRef) -> Option<Arc<ObjectTexture>>;

    /// How far the current mass build has got (REQ-E3D-362).
    fn progress(&self) -> GeometryProgress;

    /// Announces a mass build of `object_count` 3D objects, so that the loading progress of
    /// REQ-E3D-600 can be reported. A builder that reports no progress ignores it.
    fn begin_mass_build(&mut self, _object_count: usize) {}

    /// The builder as [`Any`], so that a host can reach its concrete type.
    fn as_any(&self) -> &dyn Any;

    /// The builder as [`Any`], for writing.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// A geometry builder that builds nothing and declares everything ready. It lets the
/// conversion pipeline run and be tested on its own.
#[derive(Debug, Clone, Copy, Default)]
pub struct NullPatternGeometryBuilder;

impl PatternGeometryBuilder for NullPatternGeometryBuilder {
    fn build(&mut self, _id: PatternId, _pattern: &mut EnginePattern, _global: RenderAlgorithm) {}

    fn release(&mut self, _id: PatternId, _pattern: &EnginePattern) {}

    fn is_ready(&self, _instance: InstanceRef) -> bool {
        true
    }

    fn mesh_of(&self, _instance: InstanceRef, _image: &TileImage) -> Option<Arc<TileMesh>> {
        None
    }

    fn texture_of(&self, _instance: InstanceRef) -> Option<Arc<ObjectTexture>> {
        None
    }

    fn progress(&self) -> GeometryProgress {
        GeometryProgress::default()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// The geometry side of the engine (SPEC-04B §3.2 to §3.7): it voxelises every 3D object of a
/// pattern, meshes each of its distinct tile images and keeps the result until the pattern is
/// released.
///
/// Pure data: nothing here knows about OpenGL (REQ-ARCH-002). "Transferring" a mesh to the
/// renderer is therefore just marking it ready; `defer_transfer` lets a host — or a test —
/// keep the two phases of REQ-E3D-600 apart, which is what the loading progress measures.
///
/// Building is the only thing that touches a mesh. A change of palette, position, transform,
/// alpha, layer, shadow tag or deformation never rebuilds anything (REQ-E3D-602);
/// [`GeometryBuilder::build_count`] is there so a test can prove it.
#[derive(Debug, Clone)]
pub struct GeometryBuilder {
    geometry: FastMap<InstanceRef, ObjectGeometry>,
    pending: Vec<InstanceRef>,
    built: usize,
    transferred: usize,
    to_build: usize,
    build_count: usize,
    /// The universal background colour a direct texture gives to its value-0 texels. It is
    /// read when the texture is built and does not follow later palette changes
    /// (REQ-E3D-562, Q-017).
    pub background_color: Rgb24,
    /// When true, a build only computes the meshes and [`GeometryBuilder::commit`] hands them
    /// over; when false (the default) a build does both at once.
    pub defer_transfer: bool,
}

impl Default for GeometryBuilder {
    fn default() -> Self {
        Self {
            geometry: FastMap::default(),
            pending: Vec::new(),
            built: 0,
            transferred: 0,
            to_build: 0,
            build_count: 0,
            background_color: nes_color(0x0F),
            defer_transfer: false,
        }
    }
}

impl GeometryBuilder {
    /// Creates an empty builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of 3D objects whose geometry was built since this builder was created.
    #[must_use]
    pub const fn build_count(&self) -> usize {
        self.build_count
    }

    /// The 3D objects that currently hold geometry.
    #[must_use]
    pub fn object_count(&self) -> usize {
        self.geometry.len()
    }

    /// Hands the meshes built since the last call over to the renderer and returns how many
    /// objects that was (REQ-E3D-600, second half of the progress).
    pub fn commit(&mut self) -> usize {
        let mut count = 0;
        for instance in self.pending.drain(..) {
            if let Some(geometry) = self.geometry.get_mut(&instance) {
                if !geometry.ready() {
                    geometry.set_ready();
                    self.transferred += 1;
                    count += 1;
                }
            }
        }

        count
    }

    /// Frees every mesh and texture (profile unloaded).
    pub fn clear(&mut self) {
        self.geometry.clear();
        self.pending.clear();
        self.built = 0;
        self.transferred = 0;
        self.to_build = 0;
    }

    /// The geometry of a 3D object, or `None` when it has none yet.
    #[must_use]
    pub fn of(&self, instance: InstanceRef) -> Option<&ObjectGeometry> {
        self.geometry.get(&instance)
    }

    /// The depth a build of `reference` starts from (Q-013): the depth the previous build was
    /// asked for, unless something (a `BuildPattern` hook, REQ-E3D-322) has moved sizeZ since.
    /// Without this a cylinder would shrink a little at every rebuild, because REQ-E3D-452
    /// writes D' back into sizeZ.
    fn requested_depth(&self, reference: InstanceRef, depth: i32) -> i32 {
        let depth = match self.geometry.get(&reference) {
            Some(previous) if depth == previous.effective_depth() => previous.requested_depth(),
            _ => depth,
        };
        depth.clamp(0, MAX_DEPTH)
    }

    /// Builds the geometry of instance `index`, whose depth is already the requested one, and
    /// returns its effective depth `D'` (REQ-E3D-452).
    fn build_object(
        &mut self,
        id: PatternId,
        pattern: &EnginePattern,
        index: usize,
        global: RenderAlgorithm,
    ) -> i32 {
        let reference = InstanceRef { pattern: id, index };
        let parameters: &PatternInstance = &pattern.instances()[index];
        let algorithm = parameters.effective_algorithm(global);
        let requested = parameters.depth;

        let texture = ObjectTexture::for_object(
            pattern,
            parameters.texture.as_deref(),
            self.background_color,
        );
        let built = voxel_construction::build(pattern, &texture, parameters, algorithm);
        // AMD-063 REQ-SCN-087: a variant whose effective algorithm is not MARCHING has
        // removable inner faces, whatever its opacity and its render style — the opacity is read
        // on the drawn instance and nowhere else.
        let mut full_tiles = vec![false; built.fields.len()];
        let meshes = if algorithm == RenderAlgorithm::Marching {
            let voxels = PatternVoxels::new(pattern, &texture, &built.fields);
            let empty = ObjectTexture::empty_color(parameters.empty_color);
            (0..built.fields.len())
                .map(|n| Arc::new(marching::build(&voxels, n, empty)))
                .collect()
        } else {
            let multicolor = algorithm == RenderAlgorithm::GreedyMulticolor;
            built
                .fields
                .iter()
                .enumerate()
                .map(|(n, field)| {
                    full_tiles[n] = is_full_tile(field);
                    Arc::new(greedy::build(field, &texture, multicolor))
                })
                .collect()
        };

        let object = BuiltObject {
            full_tiles,
            texture: Arc::new(texture),
            algorithm,
            requested_depth: requested,
            build_depth: requested,
            effective_depth: built.effective_depth,
            meshes,
            fields: built.fields,
        };
        self.geometry
            .insert(reference, ObjectGeometry::new(id, pattern, object));
        self.pending.push(reference);
        self.built += 1;
        self.build_count += 1;
        built.effective_depth
    }
}

impl PatternGeometryBuilder for GeometryBuilder {
    fn build(&mut self, id: PatternId, pattern: &mut EnginePattern, global: RenderAlgorithm) {
        for index in 0..pattern.instances().len() {
            let reference = InstanceRef { pattern: id, index };
            let requested = self.requested_depth(reference, pattern.instances()[index].depth);
            pattern.instances_mut()[index].depth = requested;
            let effective = self.build_object(id, pattern, index, global);

            // REQ-E3D-452: from now on placement, the MARCHING scale correction, the
            // deformation and the scripts see D'.
            pattern.instances_mut()[index].depth = effective;
        }

        if !self.defer_transfer {
            self.commit();
        }
    }

    fn release(&mut self, id: PatternId, pattern: &EnginePattern) {
        for index in 0..pattern.instances().len() {
            let instance = InstanceRef { pattern: id, index };
            self.geometry.remove(&instance);
            self.pending.retain(|&pending| pending != instance);
        }
    }

    fn is_ready(&self, instance: InstanceRef) -> bool {
        self.geometry
            .get(&instance)
            .is_some_and(ObjectGeometry::ready)
    }

    fn mesh_of(&self, instance: InstanceRef, image: &TileImage) -> Option<Arc<TileMesh>> {
        self.geometry
            .get(&instance)
            .and_then(|geometry| geometry.drawn_mesh(image))
    }

    fn mesh_of_without(
        &mut self,
        instance: InstanceRef,
        image: &TileImage,
        removed: TileSides,
    ) -> Option<Arc<TileMesh>> {
        self.geometry
            .get_mut(&instance)
            .and_then(|geometry| geometry.drawn_mesh_without(image, removed))
    }

    fn is_full_tile(&self, instance: InstanceRef, image: &TileImage) -> bool {
        self.geometry
            .get(&instance)
            .is_some_and(|geometry| geometry.is_full_tile(image))
    }

    fn texture_of(&self, instance: InstanceRef) -> Option<Arc<ObjectTexture>> {
        self.geometry
            .get(&instance)
            .map(|geometry| Arc::clone(geometry.texture()))
    }

    fn progress(&self) -> GeometryProgress {
        GeometryProgress::new(self.built, self.transferred, self.to_build)
    }

    fn begin_mass_build(&mut self, object_count: usize) {
        self.to_build = object_count;
        self.built = 0;
        self.transferred = 0;
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Whether `field` is a **full** tile in the sense of AMD-063 REQ-SCN-087: `D > 0` and all three
/// slots of each of its `8 × 8 × D` voxels occupied (REQ-E3D-422), `emptyColor` included
/// (REQ-E3D-424).
///
/// A hollow geometry — `CUBE`, the cylinders — leaves the inside of the field empty and so yields
/// no full tile, which the amendment wants.
fn is_full_tile(field: &TileVoxels) -> bool {
    if field.depth() <= 0 {
        return false;
    }

    for k in 0..field.depth() {
        for j in 0..tile_voxels::SIZE_Y {
            for i in 0..tile_voxels::SIZE_X {
                if !field.occupied(i, j, k, tile_voxels::AXIS_X)
                    || !field.occupied(i, j, k, tile_voxels::AXIS_Y)
                    || !field.occupied(i, j, k, tile_voxels::AXIS_Z)
                {
                    return false;
                }
            }
        }
    }

    true
}
