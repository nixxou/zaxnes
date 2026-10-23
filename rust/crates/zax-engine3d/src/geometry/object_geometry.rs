// Portions adapted from 3DNes_Engine (c) Geod Studio, GPL-3.0

//! The geometry of one 3D object (SPEC-04B §3.4, §3.7).

use std::collections::HashMap;
use std::sync::Arc;

use crate::geometry::greedy;
use crate::geometry::texture::ObjectTexture;
use crate::geometry::tile_mesh::TileMesh;
use crate::geometry::tile_sides;
use crate::geometry::tile_voxels::TileVoxels;
use crate::geometry::voxel_construction::image_index;
use crate::image::TileImage;
use crate::pattern::EnginePattern;
use crate::registry::PatternId;
use crate::settings::RenderAlgorithm;

/// What a build produced for one 3D object, before it is wrapped into an [`ObjectGeometry`].
#[derive(Debug, Clone)]
pub struct BuiltObject {
    /// The object's texture (REQ-E3D-426).
    pub texture: Arc<ObjectTexture>,
    /// The mesh algorithm the meshes were built with (REQ-E3D-500).
    pub algorithm: RenderAlgorithm,
    /// The depth the build was asked for (Q-013).
    pub requested_depth: i32,
    /// The depth `D` the voxel field used (REQ-E3D-504, REQ-E3D-452).
    pub build_depth: i32,
    /// The depth `D'` placement uses (REQ-E3D-452).
    pub effective_depth: i32,
    /// The meshes, in the order of the pattern's images.
    pub meshes: Vec<Arc<TileMesh>>,
    /// The voxel fields, in the same order.
    pub fields: Vec<TileVoxels>,
}

/// The geometry of one 3D object: its resolved texture, the mesh of each distinct tile image
/// of its pattern, and the depths the construction used and produced (SPEC-04B §3.4, §3.7).
///
/// Meshes depend only on the reference 3D object — its parameters at build time, its texture
/// and the tiles of its pattern (REQ-E3D-542). Nothing a script changes on a frame instance,
/// and no palette change, rebuilds them (REQ-E3D-565, REQ-E3D-602).
#[derive(Debug, Clone)]
pub struct ObjectGeometry {
    pattern: PatternId,
    built: BuiltObject,
    mesh_of_image: Vec<Option<usize>>,
    full_sides: Vec<u8>,
    variants: HashMap<(usize, u8), Arc<TileMesh>>,
    ready: bool,
}

impl ObjectGeometry {
    pub(crate) fn new(pattern_id: PatternId, pattern: &EnginePattern, built: BuiltObject) -> Self {
        // Q-067: only a side whose whole plane is occupied can be sealed by a neighbour.
        let full_sides = if built.algorithm == RenderAlgorithm::Marching {
            vec![0; built.fields.len()]
        } else {
            built.fields.iter().map(tile_sides::full_sides).collect()
        };

        Self {
            pattern: pattern_id,
            mesh_of_image: image_index(pattern),
            built,
            full_sides,
            variants: HashMap::new(),
            ready: false,
        }
    }

    /// The pattern the object belongs to.
    #[must_use]
    pub const fn pattern(&self) -> PatternId {
        self.pattern
    }

    /// The object's texture (REQ-E3D-426).
    #[must_use]
    pub fn texture(&self) -> &Arc<ObjectTexture> {
        &self.built.texture
    }

    /// The mesh algorithm the meshes were built with (REQ-E3D-500).
    #[must_use]
    pub const fn algorithm(&self) -> RenderAlgorithm {
        self.built.algorithm
    }

    /// The depth the build was *asked* for, before the cylinders replaced it by
    /// [`ObjectGeometry::effective_depth`]. A rebuild starts from this value again (Q-013).
    #[must_use]
    pub const fn requested_depth(&self) -> i32 {
        self.built.requested_depth
    }

    /// The depth `D` the voxel field and the local centring of the meshes use (REQ-E3D-504,
    /// REQ-E3D-452).
    #[must_use]
    pub const fn build_depth(&self) -> i32 {
        self.built.build_depth
    }

    /// The depth `D'` placement, scale correction and deformation use: `D` for the
    /// non-cylindrical types, `max(B + 1)` over the segments otherwise (REQ-E3D-452).
    #[must_use]
    pub const fn effective_depth(&self) -> i32 {
        self.built.effective_depth
    }

    /// True once the meshes have reached the renderer; an object that is not ready is not
    /// drawn (REQ-E3D-543, REQ-E3D-601).
    #[must_use]
    pub const fn ready(&self) -> bool {
        self.ready
    }

    pub(crate) fn set_ready(&mut self) {
        self.ready = true;
    }

    /// The meshes, in the order of [`EnginePattern::images`].
    #[must_use]
    pub fn meshes(&self) -> &[Arc<TileMesh>] {
        &self.built.meshes
    }

    /// Total number of triangles over every tile mesh of the object.
    #[must_use]
    pub fn triangle_count(&self) -> usize {
        self.built
            .meshes
            .iter()
            .map(|mesh| mesh.triangle_count())
            .sum()
    }

    /// True when the meshes were merged by colour (REQ-E3D-503).
    #[must_use]
    pub fn multicolor(&self) -> bool {
        self.built.algorithm == RenderAlgorithm::GreedyMulticolor
    }

    /// The mesh of `image`, or `None` when the pattern has no such tile.
    #[must_use]
    pub fn mesh(&self, image: &TileImage) -> Option<&Arc<TileMesh>> {
        self.index(image).map(|index| &self.built.meshes[index])
    }

    /// The lateral sides of `image` whose plane is entirely occupied, as a mask of
    /// [`tile_sides`]: the sides a neighbour may seal (Q-067).
    #[must_use]
    pub fn full_sides(&self, image: &TileImage) -> u8 {
        self.index(image).map_or(0, |index| self.full_sides[index])
    }

    /// The mesh of `image` with the walls of `sealed_sides` left out (Q-067). A mask of 0 —
    /// or MARCHING, which has no such walls — gives [`ObjectGeometry::mesh`] itself. A variant
    /// is built once and kept.
    pub fn sealed_mesh(&mut self, image: &TileImage, sealed_sides: u8) -> Option<Arc<TileMesh>> {
        let index = self.index(image)?;
        let sealed_sides = sealed_sides & self.full_sides[index];
        if sealed_sides == 0 || self.built.algorithm == RenderAlgorithm::Marching {
            return Some(Arc::clone(&self.built.meshes[index]));
        }

        let built = &self.built;
        let variant = self
            .variants
            .entry((index, sealed_sides))
            .or_insert_with(|| {
                Arc::new(greedy::build(
                    &built.fields[index],
                    &built.texture,
                    built.algorithm == RenderAlgorithm::GreedyMulticolor,
                    sealed_sides,
                ))
            });
        Some(Arc::clone(variant))
    }

    fn index(&self, image: &TileImage) -> Option<usize> {
        self.mesh_of_image
            .get(image.id() as usize)
            .copied()
            .flatten()
    }
}
