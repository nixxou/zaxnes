//! The vector and matrix types of `System.Numerics` the renderer is written against.
//!
//! [`Matrix4x4`] keeps the .NET convention on purpose: its cells `m11..m44` are stored row by
//! row and it is applied to *row* vectors, so a transform is the transpose of the column-vector
//! [`Mat4`](zax_engine3d::vectors::Mat4) of the 3D engine. Uploading its cells in field order
//! hands GLSL the column-vector matrix the shaders use (see `interop`). Keeping the same
//! convention, constructors and operation order as the C# reference keeps the captures
//! comparable with the goldens.

use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

use zax_engine3d::vectors::{Vec2, Vec3};

/// A two-component vector (`System.Numerics.Vector2`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vector2 {
    /// X component.
    pub x: f32,
    /// Y component.
    pub y: f32,
}

impl Vector2 {
    /// `(1, 1)`.
    pub const ONE: Self = Self::new(1.0, 1.0);

    /// Creates a vector.
    #[must_use]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl From<Vec2> for Vector2 {
    fn from(v: Vec2) -> Self {
        Self::new(v.x, v.y)
    }
}

/// A three-component vector (`System.Numerics.Vector3`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vector3 {
    /// X component.
    pub x: f32,
    /// Y component.
    pub y: f32,
    /// Z component.
    pub z: f32,
}

impl Vector3 {
    /// The zero vector.
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);
    /// The X axis.
    pub const UNIT_X: Self = Self::new(1.0, 0.0, 0.0);
    /// The Y axis.
    pub const UNIT_Y: Self = Self::new(0.0, 1.0, 0.0);
    /// The Z axis.
    pub const UNIT_Z: Self = Self::new(0.0, 0.0, 1.0);

    /// Creates a vector.
    #[must_use]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// The vector whose three components are `value`.
    #[must_use]
    pub const fn splat(value: f32) -> Self {
        Self::new(value, value, value)
    }

    /// Squared length.
    #[must_use]
    pub fn length_squared(self) -> f32 {
        Self::dot(self, self)
    }

    /// Length.
    #[must_use]
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    /// The vector divided by its length (NaN for the zero vector, as in .NET).
    #[must_use]
    pub fn normalize(self) -> Self {
        self / self.length()
    }

    /// Dot product.
    #[must_use]
    pub fn dot(a: Self, b: Self) -> f32 {
        (a.x * b.x) + (a.y * b.y) + (a.z * b.z)
    }

    /// Cross product.
    #[must_use]
    pub fn cross(a: Self, b: Self) -> Self {
        Self::new(
            (a.y * b.z) - (a.z * b.y),
            (a.z * b.x) - (a.x * b.z),
            (a.x * b.y) - (a.y * b.x),
        )
    }

    /// Distance between two points.
    #[must_use]
    pub fn distance(a: Self, b: Self) -> f32 {
        (a - b).length()
    }

    /// Component-wise minimum.
    #[must_use]
    pub fn min(a: Self, b: Self) -> Self {
        Self::new(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z))
    }

    /// Component-wise maximum.
    #[must_use]
    pub fn max(a: Self, b: Self) -> Self {
        Self::new(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z))
    }

    /// The point transformed by a row-vector matrix (`Vector3.Transform`).
    #[must_use]
    pub fn transform(self, m: &Matrix4x4) -> Self {
        Self::new(
            (self.x * m.m11) + (self.y * m.m21) + (self.z * m.m31) + m.m41,
            (self.x * m.m12) + (self.y * m.m22) + (self.z * m.m32) + m.m42,
            (self.x * m.m13) + (self.y * m.m23) + (self.z * m.m33) + m.m43,
        )
    }
}

impl From<Vec3> for Vector3 {
    fn from(v: Vec3) -> Self {
        Self::new(v.x, v.y, v.z)
    }
}

impl From<Vector3> for Vec3 {
    fn from(v: Vector3) -> Self {
        Self::new(v.x, v.y, v.z)
    }
}

impl Add for Vector3 {
    type Output = Self;

    fn add(self, b: Self) -> Self {
        Self::new(self.x + b.x, self.y + b.y, self.z + b.z)
    }
}

impl AddAssign for Vector3 {
    fn add_assign(&mut self, b: Self) {
        *self = *self + b;
    }
}

impl Sub for Vector3 {
    type Output = Self;

    fn sub(self, b: Self) -> Self {
        Self::new(self.x - b.x, self.y - b.y, self.z - b.z)
    }
}

impl SubAssign for Vector3 {
    fn sub_assign(&mut self, b: Self) {
        *self = *self - b;
    }
}

impl Neg for Vector3 {
    type Output = Self;

    fn neg(self) -> Self {
        Self::new(-self.x, -self.y, -self.z)
    }
}

impl Mul<f32> for Vector3 {
    type Output = Self;

    fn mul(self, s: f32) -> Self {
        Self::new(self.x * s, self.y * s, self.z * s)
    }
}

impl Mul<Vector3> for f32 {
    type Output = Vector3;

    fn mul(self, v: Vector3) -> Vector3 {
        v * self
    }
}

impl Mul for Vector3 {
    type Output = Self;

    fn mul(self, b: Self) -> Self {
        Self::new(self.x * b.x, self.y * b.y, self.z * b.z)
    }
}

impl Div<f32> for Vector3 {
    type Output = Self;

    fn div(self, s: f32) -> Self {
        Self::new(self.x / s, self.y / s, self.z / s)
    }
}

/// A four-component vector (`System.Numerics.Vector4`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vector4 {
    /// X component.
    pub x: f32,
    /// Y component.
    pub y: f32,
    /// Z component.
    pub z: f32,
    /// W component.
    pub w: f32,
}

impl Vector4 {
    /// Creates a vector.
    #[must_use]
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }
}

/// A 4 × 4 matrix stored row by row and applied to row vectors (`System.Numerics.Matrix4x4`).
///
/// The sixteen cells are named after their row and column.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix4x4 {
    pub m11: f32,
    pub m12: f32,
    pub m13: f32,
    pub m14: f32,
    pub m21: f32,
    pub m22: f32,
    pub m23: f32,
    pub m24: f32,
    pub m31: f32,
    pub m32: f32,
    pub m33: f32,
    pub m34: f32,
    pub m41: f32,
    pub m42: f32,
    pub m43: f32,
    pub m44: f32,
}

impl Default for Matrix4x4 {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Matrix4x4 {
    /// The identity.
    pub const IDENTITY: Self = Self::from_rows([
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]);

    /// The matrix whose rows are `rows`.
    #[must_use]
    pub const fn from_rows(rows: [[f32; 4]; 4]) -> Self {
        let [[m11, m12, m13, m14], [m21, m22, m23, m24], [m31, m32, m33, m34], [m41, m42, m43, m44]] =
            rows;
        Self {
            m11,
            m12,
            m13,
            m14,
            m21,
            m22,
            m23,
            m24,
            m31,
            m32,
            m33,
            m34,
            m41,
            m42,
            m43,
            m44,
        }
    }

    /// The four rows.
    #[must_use]
    pub const fn rows(&self) -> [[f32; 4]; 4] {
        [
            [self.m11, self.m12, self.m13, self.m14],
            [self.m21, self.m22, self.m23, self.m24],
            [self.m31, self.m32, self.m33, self.m34],
            [self.m41, self.m42, self.m43, self.m44],
        ]
    }

    /// The sixteen cells in field order, `m11, m12, …, m44`.
    #[must_use]
    pub fn to_cells(&self) -> [f32; 16] {
        let mut cells = [0.0; 16];
        for (row, values) in self.rows().iter().enumerate() {
            cells[row * 4..row * 4 + 4].copy_from_slice(values);
        }
        cells
    }

    /// The product `a · b` of the stored cells (`Matrix4x4.Multiply`).
    #[must_use]
    pub fn multiply(a: &Self, b: &Self) -> Self {
        let (a, b) = (a.rows(), b.rows());
        let mut rows = [[0.0; 4]; 4];
        for (i, row) in rows.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                *cell = (a[i][0] * b[0][j])
                    + (a[i][1] * b[1][j])
                    + (a[i][2] * b[2][j])
                    + (a[i][3] * b[3][j]);
            }
        }
        Self::from_rows(rows)
    }

    /// The right-handed view matrix of a camera at `position` looking at `target`
    /// (`Matrix4x4.CreateLookAt`).
    #[must_use]
    pub fn create_look_at(position: Vector3, target: Vector3, up: Vector3) -> Self {
        let z = (position - target).normalize();
        let x = Vector3::cross(up, z).normalize();
        let y = Vector3::cross(z, x);
        Self::from_rows([
            [x.x, y.x, z.x, 0.0],
            [x.y, y.y, z.y, 0.0],
            [x.z, y.z, z.z, 0.0],
            [
                -Vector3::dot(x, position),
                -Vector3::dot(y, position),
                -Vector3::dot(z, position),
                1.0,
            ],
        ])
    }

    /// The right-handed perspective projection with a `[0, 1]` depth range
    /// (`Matrix4x4.CreatePerspectiveFieldOfView`).
    #[must_use]
    pub fn create_perspective_field_of_view(fov: f32, aspect: f32, near: f32, far: f32) -> Self {
        let y_scale = 1.0 / (fov * 0.5).tan();
        let x_scale = y_scale / aspect;
        let range = far / (near - far);
        Self::from_rows([
            [x_scale, 0.0, 0.0, 0.0],
            [0.0, y_scale, 0.0, 0.0],
            [0.0, 0.0, range, -1.0],
            [0.0, 0.0, range * near, 0.0],
        ])
    }

    /// The right-handed off-centre orthographic projection with a `[0, 1]` depth range
    /// (`Matrix4x4.CreateOrthographicOffCenter`).
    #[must_use]
    pub fn create_orthographic_off_center(
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
        near: f32,
        far: f32,
    ) -> Self {
        let range = 1.0 / (near - far);
        Self::from_rows([
            [2.0 / (right - left), 0.0, 0.0, 0.0],
            [0.0, 2.0 / (top - bottom), 0.0, 0.0],
            [0.0, 0.0, range, 0.0],
            [
                (left + right) / (left - right),
                (top + bottom) / (bottom - top),
                range * near,
                1.0,
            ],
        ])
    }
}
