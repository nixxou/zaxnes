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

/// How many cells a side of each face of the backdrop is cut into while a curvature is active
/// (AMD-043 REQ-SCN-068: "le fond DEVRAIT l'être en au moins 32 × 32 cellules par face").
///
/// At curvature 0 the box stays the six quads of [`unit_box`]: the subdivision exists only while
/// something bends, so the draw-call and vertex counts of the default settings are unchanged.
pub const BACKDROP_CELLS: u16 = 32;

/// How many cells a side of a surface the curvature bends is cut into: one — no subdivision at
/// all — when nothing bends, [`BACKDROP_CELLS`] when something does.
///
/// The single place that rule is written: the renderer reads it to pick the mesh it hands the
/// frame plan, and [`crate::scene_export`] reads it to place the same vertices on the CPU, so a
/// test of the exported geometry cannot drift from what is drawn.
#[must_use]
pub const fn subdivision_for(bend_active: bool) -> u16 {
    if bend_active {
        BACKDROP_CELLS
    } else {
        1
    }
}

/// The unit box the backdrop of SPEC-06 REQ-SCN-048 is drawn with: the box `[−0.5, 0.5]³` in
/// the scene frame, six quads with outward normals, wound as REQ-E3D-505 requires. It is a
/// [`TileMesh`] so that it goes through the same upload path and the same shaders as the tiles.
///
/// # Errors
///
/// [`TriangleCountError`] never in practice: every face adds two whole triangles.
pub fn unit_box() -> Result<TileMesh, TriangleCountError> {
    unit_box_subdivided(1)
}

/// [`unit_box`] with each face cut into `cells` × `cells` quads, so that the curvature of
/// REQ-SCN-067 — which moves each vertex on its own — bends the backdrop smoothly instead of
/// tilting six flat quads (REQ-SCN-068).
///
/// `cells` is read as at least 1, which is [`unit_box`] itself.
///
/// # Errors
///
/// [`TriangleCountError`] never in practice: every cell adds two whole triangles.
pub fn unit_box_subdivided(cells: u16) -> Result<TileMesh, TriangleCountError> {
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
    let vector = |(x, y, z): Corner, scale: f32| {
        Vec3::new(
            f32::from(x) * scale,
            f32::from(y) * scale,
            f32::from(z) * scale,
        )
    };

    // The four corners are (bottom-left, top-left, top-right, bottom-right), so the grid runs
    // along BL → BR for `column` and BL → TL for `row`; the texture coordinates of the whole
    // face stay `[0, 1]²` whatever the subdivision.
    let cells = usize::from(cells.max(1));
    let side = cells + 1;
    let mut vertices = Vec::with_capacity(6 * side * side);
    let mut triangles = Vec::with_capacity(6 * cells * cells * 6);
    for (normal, corners) in FACES {
        #[allow(clippy::cast_possible_truncation)] // 6 × 33 × 33 vertices at most.
        let first = vertices.len() as u32;
        let [bottom_left, top_left, top_right, bottom_right] =
            corners.map(|corner| vector(corner, 0.5));
        let outward = vector(normal, 1.0);
        #[allow(clippy::cast_precision_loss)] // `cells` is 1 to 32.
        let step = 1.0 / cells as f32;
        for row in 0..side {
            #[allow(clippy::cast_precision_loss)] // 0 to 32.
            let v = row as f32 * step;
            // The bottom and top edges of this row, so that a cell's corners sit exactly on the
            // bilinear patch of the face and two neighbouring faces share their boundary.
            let left = lerp(bottom_left, top_left, v);
            let right = lerp(bottom_right, top_right, v);
            for column in 0..side {
                #[allow(clippy::cast_precision_loss)] // 0 to 32.
                let u = column as f32 * step;
                vertices.push(MeshVertex::new(
                    lerp(left, right, u),
                    outward,
                    Vec2::new(u, v),
                ));
            }
        }
        for row in 0..cells {
            for column in 0..cells {
                #[allow(clippy::cast_possible_truncation)] // 6 × 33 × 33 at most.
                let corner = first + (row * side + column) as u32;
                #[allow(clippy::cast_possible_truncation)] // 33 at most.
                let up = side as u32;
                // The same winding as the undivided face: BL, TL, TR then BL, TR, BR.
                triangles.extend([
                    corner,
                    corner + up,
                    corner + up + 1,
                    corner,
                    corner + up + 1,
                    corner + 1,
                ]);
            }
        }
    }

    TileMesh::new(vertices, triangles)
}

/// `a + (b − a) · t`, component by component.
fn lerp(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    Vec3::new(
        a.x + (b.x - a.x) * t,
        a.y + (b.y - a.y) * t,
        a.z + (b.z - a.z) * t,
    )
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

/// The bits of a byte of the core's picture that carry the NES colour index (SPEC-02
/// REQ-CORE-048, 0 to 63).
///
/// The renderer does not depend on the core — it only reads the bytes the host lends it — so the
/// layout of that byte is written out here beside [`HIDDEN_PIXEL`], which is the renderer's own
/// value in the same byte. `zax_nes::ppu::PIXEL_COLOR` and `OPAQUE_PIXEL` are the same two
/// values, and a test of this crate asserts that they agree.
pub const PIXEL_COLOR: u8 = 0x3F;

/// The bit of a byte of the core's picture that carries the opacity mask of AMD-050
/// REQ-CORE-049: 1 where the PPU produced at least one opaque pixel there, 0 where the picture
/// shows the universal backdrop for want of one.
///
/// REQ-SCN-076 makes a pixel whose bit is 0 transparent in the 2D-mode rectangle. A pixel whose
/// bit is 1 is drawn even when its palette colour *is* the universal backdrop's, which is
/// exactly what the index image alone cannot express.
pub const OPAQUE_PIXEL: u8 = 0x40;

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
/// the same margin and the same subdivision; `None` for a 256-pixel picture at subdivision 1.
///
/// # Errors
///
/// [`TriangleCountError`] never in practice: every cell is two whole triangles.
pub fn wide_quad_for(
    picture: &[u8],
    height: usize,
    cached: Option<WidePictureQuad>,
) -> Result<Option<WidePictureQuad>, TriangleCountError> {
    wide_quad_for_subdivided_cached(picture, height, 1, cached)
}

/// [`wide_quad_for`] with the cells of AMD-046 REQ-SCN-074, without a cache.
///
/// # Errors
///
/// As [`wide_quad_for`].
pub fn wide_quad_for_subdivided(
    picture: &[u8],
    height: usize,
    cells: u16,
) -> Result<Option<WidePictureQuad>, TriangleCountError> {
    wide_quad_for_subdivided_cached(picture, height, cells, None)
}

/// [`wide_quad_for`] with the cells of AMD-046 REQ-SCN-074: a quad of `cells` × `cells` cells,
/// so that the curvature bends the picture smoothly. A subdivision above 1 is a quad of its own
/// even at margin 0, which is why the 256-pixel shortcut only applies at 1.
///
/// # Errors
///
/// As [`wide_quad_for`].
pub fn wide_quad_for_subdivided_cached(
    picture: &[u8],
    height: usize,
    cells: u16,
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
        Arc::new(picture_quad_subdivided(margin, margin_y, cells)?),
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
    picture_quad_subdivided(margin, margin_y, 1)
}

/// [`picture_quad_wide`] cut into `cells` × `cells` cells (AMD-046 REQ-SCN-074: the rectangle of
/// the 2D mode is subdivided into at least 32 × 32 so that the curvature bends it smoothly).
///
/// `cells` is read as at least 1, which is the single quad of [`picture_quad_wide`] — the shape
/// the rectangle keeps whenever no curvature is active, so that the draw-call and vertex counts
/// of the default settings are untouched.
///
/// # Errors
///
/// [`TriangleCountError`] never in practice: every cell is two whole triangles.
pub fn picture_quad_subdivided(
    margin: u16,
    margin_y: u16,
    cells: u16,
) -> Result<TileMesh, TriangleCountError> {
    let left = -f32::from(margin);
    let width = 256.0 + (2.0 * f32::from(margin));
    let height = 240.0 + (2.0 * f32::from(margin_y));
    let top = geometry::SCENE_HEIGHT + f32::from(margin_y);
    let bottom = top - height;
    let normal = Vec3::new(0.0, 0.0, -1.0);
    let right = left + width;

    // Same winding as the −Z face of the unit box (REQ-E3D-505): bottom-left, top-left,
    // top-right, bottom-right. The texture coordinates are in **texels** of the picture
    // (REQ-E3D-410), with v counted downwards from the top row.
    let cells = usize::from(cells.max(1));
    let side = cells + 1;
    #[allow(clippy::cast_precision_loss)] // `cells` is 1 to 32.
    let step = 1.0 / cells as f32;
    let mut vertices = Vec::with_capacity(side * side);
    let mut triangles = Vec::with_capacity(cells * cells * 6);
    for row in 0..side {
        #[allow(clippy::cast_precision_loss)] // 0 to 32.
        let up = row as f32 * step;
        let scene_y = bottom + (top - bottom) * up;
        let texel_v = height * (1.0 - up);
        for column in 0..side {
            #[allow(clippy::cast_precision_loss)] // 0 to 32.
            let across = column as f32 * step;
            let scene_x = left + (right - left) * across;
            vertices.push(MeshVertex::new(
                Vec3::new(scene_x, scene_y, 0.0),
                normal,
                Vec2::new(width * across, texel_v),
            ));
        }
    }
    for row in 0..cells {
        for column in 0..cells {
            #[allow(clippy::cast_possible_truncation)] // 33 × 33 at most.
            let corner = (row * side + column) as u32;
            #[allow(clippy::cast_possible_truncation)] // 33 at most.
            let up = side as u32;
            triangles.extend([
                corner,
                corner + up,
                corner + up + 1,
                corner,
                corner + up + 1,
                corner + 1,
            ]);
        }
    }
    TileMesh::new(vertices, triangles)
}
