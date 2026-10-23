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

        Ok(Self::with_bounds(vertices, triangles))
    }

    /// A mesh over a triangle list known to hold whole triangles, with its bounding box.
    fn with_bounds(vertices: Vec<MeshVertex>, triangles: Vec<u32>) -> Self {
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

        Self {
            vertices,
            triangles,
            bounds_min,
            bounds_max,
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

    /// Freezes the accumulated data into a mesh.
    pub fn build(self) -> TileMesh {
        if self.triangles.is_empty() {
            return TileMesh::empty();
        }

        // Every push adds whole triangles.
        TileMesh::with_bounds(self.vertices, self.triangles)
    }

    #[allow(clippy::cast_possible_truncation)] // A tile mesh holds far fewer than 2^32 vertices.
    fn next_index(&self) -> u32 {
        self.vertices.len() as u32
    }

    fn push_triangle(&mut self, a: u32, b: u32, c: u32) {
        self.triangles.extend([a, b, c]);
    }
}
