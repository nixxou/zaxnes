//! Conversions between the matrix conventions of the 3D engine and of [`Matrix4x4`], and the
//! two fixed meshes the renderer draws besides the tiles.
//!
//! [`Mat4`] holds its cells row by row and is applied to column vectors (`P = M · (v, 1)`,
//! SPEC-04B REQ-E3D-406). [`Matrix4x4`] holds `m11..m14` as its first row and is applied to row
//! vectors, so the same transform is the transpose. GLSL reads a `mat4` uniform column by
//! column, so uploading the fields of a [`Matrix4x4`] in field order hands GLSL its transpose,
//! that is the column-vector matrix the shaders use — which is why the conversion transposes.

use std::sync::Arc;

use zax_engine3d::geometry::tile_mesh::{MeshVertex, TileMesh, TriangleCountError};
use zax_engine3d::vectors::{Mat4, Vec2, Vec3};

use crate::camera::CameraView;
use crate::geometry;
use crate::numerics::Matrix4x4;

/// The [`Matrix4x4`] that uploads `matrix` unchanged.
#[must_use]
pub fn to_numerics(matrix: &Mat4) -> Matrix4x4 {
    let m = |row: usize, column: usize| matrix[(row, column)];
    Matrix4x4::from_rows([
        [m(0, 0), m(1, 0), m(2, 0), m(3, 0)],
        [m(0, 1), m(1, 1), m(2, 1), m(3, 1)],
        [m(0, 2), m(1, 2), m(2, 2), m(3, 2)],
        [m(0, 3), m(1, 3), m(2, 3), m(3, 3)],
    ])
}

/// The cofactor matrix of the upper-left 3 × 3 block, row by row, and the determinant of that
/// block.
///
/// The cofactor matrix is `det(M) · M⁻ᵗ`, so once the shader normalises the result it is the
/// normal matrix of REQ-SCN-028 for every transform of positive determinant (the inverse
/// transpose is the provisional choice of Q-021). The determinant is negative exactly when the
/// placement mirrors the object, the case of Q-069.
#[must_use]
pub fn normal_matrix(matrix: &Mat4) -> ([f32; 9], f32) {
    let m = |row: usize, column: usize| matrix[(row, column)];
    let (m00, m01, m02) = (m(0, 0), m(0, 1), m(0, 2));
    let (m10, m11, m12) = (m(1, 0), m(1, 1), m(1, 2));
    let (m20, m21, m22) = (m(2, 0), m(2, 1), m(2, 2));

    let cells = [
        (m11 * m22) - (m12 * m21),
        (m12 * m20) - (m10 * m22),
        (m10 * m21) - (m11 * m20),
        (m02 * m21) - (m01 * m22),
        (m00 * m22) - (m02 * m20),
        (m01 * m20) - (m00 * m21),
        (m01 * m12) - (m02 * m11),
        (m02 * m10) - (m00 * m12),
        (m00 * m11) - (m01 * m10),
    ];
    let determinant = (m00 * cells[0]) + (m01 * cells[1]) + (m02 * cells[2]);
    (cells, determinant)
}

/// Determinant of the upper-left 3 × 3 block of a placement matrix.
#[must_use]
pub fn determinant(m: &Mat4) -> f32 {
    (m[(0, 0)] * ((m[(1, 1)] * m[(2, 2)]) - (m[(1, 2)] * m[(2, 1)])))
        - (m[(0, 1)] * ((m[(1, 0)] * m[(2, 2)]) - (m[(1, 2)] * m[(2, 0)])))
        + (m[(0, 2)] * ((m[(1, 0)] * m[(2, 1)]) - (m[(1, 1)] * m[(2, 0)])))
}

/// The affine map of REQ-SCN-045 that takes a point of the reference frame to the plane
/// attached to the camera, as a [`Mat4`] so it composes with the engine's transforms.
#[must_use]
pub fn hud_matrix(view: &CameraView) -> Mat4 {
    let right = view.right * geometry::HUD_SCALE;
    let up = view.up * geometry::HUD_SCALE;
    let forward = view.forward * geometry::HUD_SCALE;
    let origin = view.eye + (view.forward * view.hud_distance)
        - (right * geometry::CENTER.x)
        - (up * geometry::CENTER.y)
        - (forward * geometry::CENTER.z);

    let mut m = Mat4::IDENTITY;
    for (column, axis) in [right, up, forward, origin].into_iter().enumerate() {
        m[(0, column)] = axis.x;
        m[(1, column)] = axis.y;
        m[(2, column)] = axis.z;
    }
    m
}

/// The unit box the backdrop of SPEC-06 REQ-SCN-048 is drawn with: the box `[−0.5, 0.5]³` in
/// the scene frame, six quads with outward normals, wound as REQ-E3D-505 requires. It is a
/// [`TileMesh`] so that it goes through the same upload path and the same shaders as the tiles.
///
/// # Errors
///
/// [`TriangleCountError`] never in practice: every face adds two whole triangles.
pub fn unit_box() -> Result<TileMesh, TriangleCountError> {
    type Corner = (i8, i8, i8);
    // Faces given as (normal, bottom-left, top-left, top-right, bottom-right) with
    // cross(TL − BL, TR − BL) along the normal.
    const FACES: [(Corner, [Corner; 4]); 6] = [
        (
            (0, 0, -1),
            [(-1, -1, -1), (-1, 1, -1), (1, 1, -1), (1, -1, -1)],
        ),
        ((0, 0, 1), [(1, -1, 1), (1, 1, 1), (-1, 1, 1), (-1, -1, 1)]),
        (
            (-1, 0, 0),
            [(-1, -1, 1), (-1, 1, 1), (-1, 1, -1), (-1, -1, -1)],
        ),
        ((1, 0, 0), [(1, -1, -1), (1, 1, -1), (1, 1, 1), (1, -1, 1)]),
        ((0, 1, 0), [(-1, 1, -1), (-1, 1, 1), (1, 1, 1), (1, 1, -1)]),
        (
            (0, -1, 0),
            [(-1, -1, 1), (-1, -1, -1), (1, -1, -1), (1, -1, 1)],
        ),
    ];
    const UVS: [Vec2; 4] = [
        Vec2::new(0.0, 0.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(1.0, 1.0),
        Vec2::new(1.0, 0.0),
    ];
    let vector = |(x, y, z): Corner, scale: f32| {
        Vec3::new(
            f32::from(x) * scale,
            f32::from(y) * scale,
            f32::from(z) * scale,
        )
    };

    let mut vertices = Vec::with_capacity(24);
    let mut triangles = Vec::with_capacity(36);
    for (normal, corners) in FACES {
        #[allow(clippy::cast_possible_truncation)] // 24 vertices at most.
        let first = vertices.len() as u32;
        for (corner, uv) in corners.into_iter().zip(UVS) {
            vertices.push(MeshVertex::new(
                vector(corner, 0.5),
                vector(normal, 1.0),
                uv,
            ));
        }
        triangles.extend([first, first + 1, first + 2, first, first + 2, first + 3]);
    }

    TileMesh::new(vertices, triangles)
}

/// The quad of the NES plane for the 2D mode (AMD-009): `x ∈ [0, 256]`, `y ∈ [8, 248]`,
/// `z = 0`, facing the default camera, with texture coordinates in texels so that NES pixel
/// `(px, py)` lands on `[px, px + 1] × [247 − py, 248 − py]` (REQ-E3D-410).
///
/// # Errors
///
/// [`TriangleCountError`] never in practice: the quad is two whole triangles.
pub fn picture_quad() -> Result<TileMesh, TriangleCountError> {
    picture_quad_wide(0, 0)
}

/// The colour index of a pixel of the 2D picture that is not drawn (Rust-only, owner request:
/// the overscan's cut sides seen through the camera), transparent in the picture's texture.
pub const HIDDEN_PIXEL: u8 = 0xFF;

/// The width and height in pixels of a 2D-mode picture of `len` colour indices in `height` rows
/// (0 reads as 240): 256 x 240, or wider and taller by widescreen margins on each side; `None`
/// for any other size.
#[must_use]
pub const fn picture_size(len: usize, height: usize) -> Option<(usize, usize)> {
    let height = if height == 0 { 240 } else { height };
    if len == 0 || height < 240 || (height - 240) % 2 != 0 || len % height != 0 {
        return None;
    }
    let width = len / height;
    if width < 256 || (width - 256) % 2 != 0 {
        return None;
    }
    Some((width, height))
}

/// A widescreen 2D-mode quad and the margins (sides, top and bottom) it was made for.
pub type WidePictureQuad = ((u16, u16), Arc<TileMesh>);

/// The quad for the widescreen 2D-mode picture `picture`, reusing `cached` when it was made for
/// the same margin; `None` for a 256-pixel picture.
///
/// # Errors
///
/// [`TriangleCountError`] never in practice: the quad is two whole triangles.
pub fn wide_quad_for(
    picture: &[u8],
    height: usize,
    cached: Option<WidePictureQuad>,
) -> Result<Option<WidePictureQuad>, TriangleCountError> {
    let Some((width, height)) = picture_size(picture.len(), height) else {
        return Ok(None);
    };
    let (Ok(margin), Ok(margin_y)) = (
        u16::try_from((width - 256) / 2),
        u16::try_from((height - 240) / 2),
    ) else {
        return Ok(None);
    };
    if margin == 0 && margin_y == 0 {
        return Ok(None);
    }
    let margins = (margin, margin_y);
    if let Some(cached) = cached.filter(|(cached, _)| *cached == margins) {
        return Ok(Some(cached));
    }
    Ok(Some((
        margins,
        Arc::new(picture_quad_wide(margin, margin_y)?),
    )))
}

/// [`picture_quad`] for a widescreen picture: `margin` more pixels on each side, x from
/// -margin to 256 + margin, and `margin_y` more above and below, the texture coordinates still in
/// texels of the larger picture.
///
/// # Errors
///
/// [`TriangleCountError`] never in practice: the quad is two whole triangles.
pub fn picture_quad_wide(margin: u16, margin_y: u16) -> Result<TileMesh, TriangleCountError> {
    let left = -f32::from(margin);
    let width = 256.0 + (2.0 * f32::from(margin));
    let height = 240.0 + (2.0 * f32::from(margin_y));
    let top = geometry::SCENE_HEIGHT + f32::from(margin_y);
    let bottom = top - height;
    let normal = Vec3::new(0.0, 0.0, -1.0);

    // Same winding as the −Z face of the unit box (REQ-E3D-505): bottom-left, top-left,
    // top-right, bottom-right.
    let right = left + width;
    let vertices = vec![
        MeshVertex::new(Vec3::new(left, bottom, 0.0), normal, Vec2::new(0.0, height)),
        MeshVertex::new(Vec3::new(left, top, 0.0), normal, Vec2::new(0.0, 0.0)),
        MeshVertex::new(Vec3::new(right, top, 0.0), normal, Vec2::new(width, 0.0)),
        MeshVertex::new(
            Vec3::new(right, bottom, 0.0),
            normal,
            Vec2::new(width, height),
        ),
    ];
    TileMesh::new(vertices, vec![0, 1, 2, 0, 2, 3])
}
