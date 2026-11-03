//! The platform-independent mesh of one tile (SPEC-04B REQ-E3D-640, §4.2).

use std::fmt;

use crate::vectors::{Vec2, Vec3};

/// One vertex of a tile mesh (SPEC-04B §4.2). Colour is *not* carried by the vertices: it is
/// resolved at render time from the frame palette (REQ-E3D-560, REQ-E3D-565, REQ-E3D-640).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MeshVertex {
    /// Position in the tile's local frame (REQ-E3D-504, REQ-E3D-522).
    pub position: Vec3,
    /// Unit normal (REQ-E3D-505); `(0,0,0)` on a zero-area triangle.
    pub normal: Vec3,
    /// Texture coordinates in texels of the object's texture (REQ-E3D-506).
    pub uv: Vec2,
}

impl MeshVertex {
    /// Creates a vertex.
    #[must_use]
    pub const fn new(position: Vec3, normal: Vec3, uv: Vec2) -> Self {
        Self {
            position,
            normal,
            uv,
        }
    }
}

/// A triangle list whose length is not a multiple of three.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("a triangle list needs three indices per triangle, got {0}")]
pub struct TriangleCountError(pub usize);

/// The platform-independent description of the mesh of one tile (SPEC-04B REQ-E3D-640, §4.2):
/// a vertex list, a triangle index list, one single submesh, triangle topology and a local
/// bounding box.
///
/// A mesh is immutable once built, which is what lets the same instance be shared by every
/// occurrence of a tile image in a shape (REQ-E3D-540) and reused across frames
/// (REQ-E3D-602).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TileMesh {
    vertices: Vec<MeshVertex>,
    triangles: Vec<u32>,
    bounds_min: Vec3,
    bounds_max: Vec3,
    /// Whether every normal of the mesh points along one axis of the local frame: true of the
    /// boxes `GREEDY` builds (REQ-E3D-505), false of the smoothed surfaces of `MARCHING`
    /// (REQ-E3D-521). Computed once here, never per draw; the renderer reads it to tell a
    /// lateral wall from a front face (AMD-057, `geometry::WALL_DEPTH_BIAS`).
    axis_aligned_normals: bool,
    /// The depth `D` the voxel field was built with (REQ-E3D-504), which is what centres the
    /// local frame on `z`: the corner of voxel `(i, j, k)` is at `(i − 4, j − 4, k − D/2)`. For a
    /// cylinder it stays the **initial** depth, not the effective `D'` of REQ-E3D-452, because
    /// that requirement leaves the field and the local centring on the initial one.
    ///
    /// 0 for a mesh that is not a tile's — the backdrop box, the rectangle of the 2D mode, an OBJ
    /// model — where it is read by nothing.
    build_depth: i32,
}

impl TileMesh {
    /// Number of submeshes; always one (REQ-E3D-640).
    pub const SUBMESH_COUNT: usize = 1;

    /// Creates a mesh from its vertices and triangle indices.
    ///
    /// # Errors
    ///
    /// [`TriangleCountError`] when the index count is not a multiple of three.
    pub fn new(vertices: Vec<MeshVertex>, triangles: Vec<u32>) -> Result<Self, TriangleCountError> {
        if triangles.len() % 3 != 0 {
            return Err(TriangleCountError(triangles.len()));
        }

        Ok(Self::with_bounds(vertices, triangles, 0))
    }

    /// A mesh over a triangle list known to hold whole triangles, with its bounding box and the
    /// construction depth of its local frame (REQ-E3D-504).
    fn with_bounds(vertices: Vec<MeshVertex>, triangles: Vec<u32>, build_depth: i32) -> Self {
        let (bounds_min, bounds_max) = match vertices.first() {
            None => (Vec3::ZERO, Vec3::ZERO),
            Some(first) => {
                vertices
                    .iter()
                    .fold((first.position, first.position), |(min, max), vertex| {
                        let p = vertex.position;
                        (
                            Vec3::new(min.x.min(p.x), min.y.min(p.y), min.z.min(p.z)),
                            Vec3::new(max.x.max(p.x), max.y.max(p.y), max.z.max(p.z)),
                        )
                    })
            }
        };

        let axis_aligned_normals = vertices.iter().all(|vertex| {
            let n = vertex.normal;
            // Two of the three components zero: a face normal of REQ-E3D-505. A zero-area
            // triangle has `(0, 0, 0)` and passes, which draws nothing either way.
            let zeros = usize::from(n.x == 0.0) + usize::from(n.y == 0.0) + usize::from(n.z == 0.0);
            zeros >= 2
        });

        Self {
            vertices,
            triangles,
            bounds_min,
            bounds_max,
            axis_aligned_normals,
            build_depth,
        }
    }

    /// The mesh of a tile with no geometry (`sizeZ = 0`, REQ-E3D-420).
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// The vertices.
    #[must_use]
    pub fn vertices(&self) -> &[MeshVertex] {
        &self.vertices
    }

    /// Triangle indices, three per triangle, oriented per REQ-E3D-505.
    #[must_use]
    pub fn triangles(&self) -> &[u32] {
        &self.triangles
    }

    /// Number of vertices.
    #[must_use]
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Number of triangles.
    #[must_use]
    pub fn triangle_count(&self) -> usize {
        self.triangles.len() / 3
    }

    /// True when the mesh draws nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.triangles.is_empty()
    }

    /// True when every normal of the mesh points along one axis of the local frame: the boxes
    /// of `GREEDY` (REQ-E3D-505), not the smoothed surfaces of `MARCHING` (REQ-E3D-521).
    ///
    /// The renderer uses it as the licence to read a normal of `|z| < 0.5` as a **lateral wall**
    /// of a tile box, the faces AMD-057 keeps and REQ-SCN-065 must not let show. On a smoothed
    /// surface that reading would be wrong, and there is no joint there to hide anyway
    /// (REQ-E3D-520 merges the tiles).
    #[must_use]
    pub const fn has_axis_aligned_normals(&self) -> bool {
        self.axis_aligned_normals
    }

    /// The depth `D` of the voxel field the mesh was built from (REQ-E3D-504), 0 for a mesh that
    /// is not a tile's.
    ///
    /// The one place it is read is the pixel shadows of AMD-055 REQ-SCN-082, which snap a point to
    /// the centre of its voxel: on `z` the voxel edges of the local frame are offset by `D/2`, so
    /// the snapping needs `D` and the extent of the mesh will not do (a `sizeZ` = 1 plate does not
    /// span the box of a deeper build).
    #[must_use]
    pub const fn build_depth(&self) -> i32 {
        self.build_depth
    }

    /// Lowest corner of the local bounding box.
    #[must_use]
    pub const fn bounds_min(&self) -> Vec3 {
        self.bounds_min
    }

    /// Highest corner of the local bounding box.
    #[must_use]
    pub const fn bounds_max(&self) -> Vec3 {
        self.bounds_max
    }
}

impl fmt::Display for TileMesh {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "mesh {} vertices, {} triangles",
            self.vertex_count(),
            self.triangle_count()
        )
    }
}

/// Accumulates the vertices and triangles of a tile mesh while it is being built, with the
/// orientation rule of REQ-E3D-505 applied once per face.
#[derive(Debug, Default)]
pub(crate) struct TileMeshBuilder {
    vertices: Vec<MeshVertex>,
    triangles: Vec<u32>,
}

/// The four corners of a rectangle, bottom-left, top-left, top-right, bottom-right.
pub(crate) type Quad<T> = [T; 4];

impl TileMeshBuilder {
    /// Pushes a rectangle given by its four corners in the order bottom-left, top-left,
    /// top-right, bottom-right, as two triangles oriented along `normal`.
    pub fn add_quad(&mut self, corners: Quad<Vec3>, uvs: Quad<Vec2>, normal: Vec3) {
        let first = self.next_index();
        for (position, uv) in corners.into_iter().zip(uvs) {
            self.vertices.push(MeshVertex::new(position, normal, uv));
        }

        // (B − A) x (C − A) must point the way the normal does (REQ-E3D-505).
        let [bottom_left, top_left, top_right, _] = corners;
        let forward = Vec3::dot(
            Vec3::cross(top_left - bottom_left, top_right - bottom_left),
            normal,
        ) >= 0.0;
        if forward {
            self.push_triangle(first, first + 1, first + 2);
            self.push_triangle(first, first + 2, first + 3);
        } else {
            self.push_triangle(first, first + 2, first + 1);
            self.push_triangle(first, first + 3, first + 2);
        }
    }

    /// Pushes a triangle with three vertices of its own, carrying the flat normal of
    /// REQ-E3D-522, oriented so that the normal points along `outward`. A zero-area triangle
    /// gets the normal `(0, 0, 0)`.
    pub fn add_triangle(&mut self, a: Vec3, mut b: Vec3, mut c: Vec3, uv: Vec2, outward: Vec3) {
        let mut cross = Vec3::cross(b - a, c - a);
        if Vec3::dot(cross, outward) < 0.0 {
            std::mem::swap(&mut b, &mut c);
            cross = cross * -1.0;
        }

        let length = cross.squared_length().sqrt();
        let normal = if length > 0.0 {
            cross * (1.0 / length)
        } else {
            Vec3::ZERO
        };

        let first = self.next_index();
        self.vertices.push(MeshVertex::new(a, normal, uv));
        self.vertices.push(MeshVertex::new(b, normal, uv));
        self.vertices.push(MeshVertex::new(c, normal, uv));
        self.push_triangle(first, first + 1, first + 2);
    }

    /// Freezes the accumulated data into a mesh of a field of depth `build_depth`, which is what
    /// centres its local frame on `z` (REQ-E3D-504).
    pub fn build(self, build_depth: i32) -> TileMesh {
        if self.triangles.is_empty() {
            return TileMesh::empty();
        }

        // Every push adds whole triangles.
        TileMesh::with_bounds(self.vertices, self.triangles, build_depth)
    }

    #[allow(clippy::cast_possible_truncation)] // A tile mesh holds far fewer than 2^32 vertices.
    fn next_index(&self) -> u32 {
        self.vertices.len() as u32
    }

    fn push_triangle(&mut self, a: u32, b: u32, c: u32) {
        self.triangles.extend([a, b, c]);
    }
}
