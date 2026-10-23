// Portions adapted from 3DNes_Engine (c) Geod Studio, GPL-3.0

//! The geometry side of the engine (SPEC-04B §3.2 to §3.7) and the interface the conversion
//! pipeline drives it through.

use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use crate::geometry::greedy;
use crate::geometry::marching;
use crate::geometry::object_geometry::{BuiltObject, ObjectGeometry};
use crate::geometry::pattern_voxels::PatternVoxels;
use crate::geometry::texture::ObjectTexture;
use crate::geometry::tile_mesh::TileMesh;
use crate::geometry::tile_voxels::MAX_DEPTH;
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
pub trait PatternGeometryBuilder {
    /// Builds, or rebuilds, the geometry of every instance of pattern `id`. `global` is the
    /// global mesh algorithm an instance with `rm = 0` takes (REQ-E3D-500). The build may
    /// write the effective depth back into the instances (REQ-E3D-452).
    fn build(&mut self, id: PatternId, pattern: &mut EnginePattern, global: RenderAlgorithm);

    /// Frees the meshes and the texture space of a pattern (SPEC-04B REQ-E3D-603).
    fn release(&mut self, id: PatternId, pattern: &EnginePattern);

    /// Whether the meshes of a pattern instance can be drawn (REQ-E3D-361).
    fn is_ready(&self, instance: InstanceRef) -> bool;

    /// The mesh of `image` for `instance` with the walls of `sealed_sides` left out, the
    /// rendering variant of Q-067 (SPEC-04B REQ-E3D-640); a mask of 0 gives the plain mesh.
    /// `None` when it is not available.
    fn mesh_of(
        &mut self,
        instance: InstanceRef,
        image: &TileImage,
        sealed_sides: u8,
    ) -> Option<Arc<TileMesh>>;

    /// The lateral sides of `image` whose plane is entirely occupied: the only sides a
    /// neighbouring tile may seal (Q-067).
    fn full_sides_of(&self, instance: InstanceRef, image: &TileImage) -> u8;

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

    fn mesh_of(
        &mut self,
        _instance: InstanceRef,
        _image: &TileImage,
        _sealed_sides: u8,
    ) -> Option<Arc<TileMesh>> {
        None
    }

    fn full_sides_of(&self, _instance: InstanceRef, _image: &TileImage) -> u8 {
        0
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
    geometry: HashMap<InstanceRef, ObjectGeometry>,
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
            geometry: HashMap::new(),
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
                .map(|field| Arc::new(greedy::build(field, &texture, multicolor, 0)))
                .collect()
        };

        let object = BuiltObject {
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

    fn mesh_of(
        &mut self,
        instance: InstanceRef,
        image: &TileImage,
        sealed_sides: u8,
    ) -> Option<Arc<TileMesh>> {
        self.geometry
            .get_mut(&instance)
            .and_then(|geometry| geometry.sealed_mesh(image, sealed_sides))
    }

    fn full_sides_of(&self, instance: InstanceRef, image: &TileImage) -> u8 {
        self.geometry
            .get(&instance)
            .map_or(0, |geometry| geometry.full_sides(image))
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
