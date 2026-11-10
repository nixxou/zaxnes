// Portions adapted from 3DNes_Engine (c) Geod Studio, GPL-3.0

//! The geometry of one 3D object (SPEC-04B §3.4, §3.7).

use std::cell::RefCell;
use std::sync::Arc;

use crate::geometry::texture::ObjectTexture;
use crate::geometry::tile_mesh::TileMesh;
use crate::geometry::tile_sides::TileSides;
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
    ready: bool,
    /// The mesh variants REQ-SCN-087 draws, built on first need: sixteen slots per mesh, one per
    /// set of removed lateral sides, slot 0 being the mesh itself. Only a full tile ever reaches
    /// them, and a variant keeps its identity from frame to frame, which is what lets the
    /// renderer keep its buffers (REQ-E3D-602).
    ///
    /// **Empty until a face is actually removed**, and behind a [`RefCell`] so that the lookup
    /// takes `&self`: the draw list asks for a mesh once per drawn tile, and a `&mut` borrow
    /// there costs every picture — transparent or not — the reloads a unique reborrow forces on
    /// the whole of the tile loop. Measured at step 12: +0.26 % of the whole process on a ROM
    /// with no profile at all. The cell is borrowed only on the removal path, after the two
    /// tests that send an ordinary tile straight back with its mesh.
    variants: RefCell<Vec<Option<Arc<TileMesh>>>>,
    /// Which meshes are those of a **full** tile (AMD-063 REQ-SCN-087), answered on demand: the
    /// test walks the `8 × 8 × D` voxels of a field, and a profile that never shows a see-through
    /// 3D object never asks. `None` is "not yet looked at".
    ///
    /// **Empty until the first question**, like [`ObjectGeometry::variants`]. Step 12 measured
    /// the scan done eagerly at build time at +0.43 % of a 600-frame Mega Man 2 run on its own
    /// 3dn pack, a profile with nothing transparent in it.
    full_tiles: RefCell<Vec<Option<bool>>>,
}

impl ObjectGeometry {
    pub(crate) fn new(pattern_id: PatternId, pattern: &EnginePattern, built: BuiltObject) -> Self {
        Self {
            pattern: pattern_id,
            mesh_of_image: image_index(pattern),
            built,
            ready: false,
            // Nothing is reserved: an object no transparent instance ever covers — every object
            // of a picture without a see-through 3D object — keeps no variant storage at all,
            // and is never asked which of its tiles are full.
            variants: RefCell::new(Vec::new()),
            full_tiles: RefCell::new(Vec::new()),
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

    /// The mesh of `image` as the draw list asks for it. AMD-057 withdrew the variants of
    /// Q-067, so **this is [`ObjectGeometry::mesh`]**: the drawn mesh of a tile is its mesh of
    /// REQ-E3D-501, every wall included — unless AMD-063 REQ-SCN-087 takes lateral faces off it,
    /// which is [`ObjectGeometry::drawn_mesh_without`].
    #[must_use]
    pub fn drawn_mesh(&self, image: &TileImage) -> Option<Arc<TileMesh>> {
        self.mesh(image).map(Arc::clone)
    }

    /// Whether `image` is a **full** tile of this 3D object (AMD-063 REQ-SCN-087): its effective
    /// mesh algorithm is not MARCHING, its depth `D` is above 0, and all three slots of every one
    /// of its `8 × 8 × D` voxels are occupied, `emptyColor` included. Only such a tile can lose a
    /// lateral face.
    #[must_use]
    pub fn is_full_tile(&self, image: &TileImage) -> bool {
        self.index(image)
            .is_some_and(|index| self.full_tile_at(index))
    }

    /// Whether the mesh at `index` is a full tile, scanning its voxel field the first time it is
    /// asked and remembering the answer (see [`ObjectGeometry::full_tiles`]).
    fn full_tile_at(&self, index: usize) -> bool {
        // A MARCHING object has one slot per voxel and no removable face, and `TileVoxels::is_full`
        // says so by itself; the check is here so such an object never allocates the cache.
        if self.built.algorithm == RenderAlgorithm::Marching {
            return false;
        }

        let mut known = self.full_tiles.borrow_mut();
        if known.is_empty() {
            known.resize(self.built.fields.len(), None);
        }
        let Some(slot) = known.get_mut(index) else {
            return false;
        };
        if let Some(answer) = *slot {
            return answer;
        }

        let answer = self.built.fields[index].is_full();
        *slot = Some(answer);
        answer
    }

    /// The mesh REQ-SCN-087 draws for `image` with `sides` taken off. The sides are honoured on
    /// a full tile only; any other tile keeps every face it was built with, as AMD-057 leaves it.
    pub fn drawn_mesh_without(&self, image: &TileImage, sides: TileSides) -> Option<Arc<TileMesh>> {
        let index = self.index(image)?;
        if sides.is_none() || !self.full_tile_at(index) {
            return self.built.meshes.get(index).map(Arc::clone);
        }

        let slot = (index * TileSides::COMBINATIONS) + usize::from(sides.bits());
        let mut variants = self.variants.borrow_mut();
        if variants.is_empty() {
            // First face removed on this object: the sixteen slots per mesh come into being now.
            variants.resize(self.built.meshes.len() * TileSides::COMBINATIONS, None);
        }
        if let Some(mesh) = variants.get(slot).and_then(Option::as_ref) {
            return Some(Arc::clone(mesh));
        }

        let built = Arc::new(self.built.meshes.get(index)?.without_sides(sides));
        *variants.get_mut(slot)? = Some(Arc::clone(&built));
        Some(built)
    }

    fn index(&self, image: &TileImage) -> Option<usize> {
        self.mesh_of_image
            .get(image.id() as usize)
            .copied()
            .flatten()
    }
}
