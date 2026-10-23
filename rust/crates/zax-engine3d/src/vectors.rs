//! Vectors, cells, rectangles and matrices of the conversion engine (SPEC-04B REQ-E3D-400).

use std::fmt;
use std::ops::{Add, AddAssign, Index, IndexMut, Mul, Sub};

/// A three-component vector, in scene coordinates unless said otherwise (x right, y up, z away
/// from the viewer; SPEC-04B REQ-E3D-400).
///
/// The fields are public on purpose: scripts change one component of an instance's transform
/// in place (`shape.Offset.x = shape.Offset.x + 5`, SPEC-05 REQ-LUA-044).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec3 {
    /// X component.
    pub x: f32,
    /// Y component.
    pub y: f32,
    /// Z component.
    pub z: f32,
}

impl Vec3 {
    /// The zero vector.
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);

    /// The vector of all ones.
    pub const ONE: Self = Self::new(1.0, 1.0, 1.0);

    /// The default pivot, `(0.5, 0.5, 0.5)` (SPEC-04 §4.3).
    pub const HALF: Self = Self::new(0.5, 0.5, 0.5);

    /// Creates a vector.
    #[must_use]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// Component-by-component product (the `⊙` of SPEC-04 REQ-E3D-205).
    #[must_use]
    pub fn scale(a: Self, b: Self) -> Self {
        Self::new(a.x * b.x, a.y * b.y, a.z * b.z)
    }

    /// Euclidean distance between two vectors.
    #[must_use]
    pub fn distance(a: Self, b: Self) -> f32 {
        let dx = a.x - b.x;
        let dy = a.y - b.y;
        let dz = a.z - b.z;
        ((dx * dx) + (dy * dy) + (dz * dz)).sqrt()
    }

    /// Squared length.
    #[must_use]
    pub fn squared_length(self) -> f32 {
        (self.x * self.x) + (self.y * self.y) + (self.z * self.z)
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

    /// Dot product.
    #[must_use]
    pub fn dot(a: Self, b: Self) -> f32 {
        (a.x * b.x) + (a.y * b.y) + (a.z * b.z)
    }
}

impl Add for Vec3 {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }
}

impl AddAssign for Vec3 {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

impl Sub for Vec3 {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;

    fn mul(self, k: f32) -> Self {
        Self::new(self.x * k, self.y * k, self.z * k)
    }
}

impl fmt::Display for Vec3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

/// A two-component vector, mutable like [`Vec3`].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec2 {
    /// X component.
    pub x: f32,
    /// Y component.
    pub y: f32,
}

impl Vec2 {
    /// The zero vector.
    pub const ZERO: Self = Self::new(0.0, 0.0);

    /// The vector of all ones.
    pub const ONE: Self = Self::new(1.0, 1.0);

    /// Creates a vector.
    #[must_use]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl fmt::Display for Vec2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

/// A pair of integers: a grid cell, a tile box corner or a screen pixel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub struct Cell {
    /// First component.
    pub x: i32,
    /// Second component.
    pub y: i32,
}

impl Cell {
    /// Creates a cell.
    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

impl Add for Cell {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y)
    }
}

impl Sub for Cell {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y)
    }
}

impl fmt::Display for Cell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

/// An axis-aligned rectangle of origin `(x, y)` and size `(width, height)`, with the strict
/// overlap test of SPEC-05 §4.3 (type `Rect`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    /// Origin x.
    pub x: f32,
    /// Origin y.
    pub y: f32,
    /// Width; a negative width is normalised by [`Rect::overlaps`].
    pub width: f32,
    /// Height; a negative height is normalised by [`Rect::overlaps`].
    pub height: f32,
}

impl Rect {
    /// Creates a rectangle.
    #[must_use]
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Strict overlap: `xMax > r.x`, `x < r.xMax`, and the same on y.
    #[must_use]
    pub fn overlaps(&self, other: &Self) -> bool {
        let (x0, x1) = normalise(self.x, self.width);
        let (y0, y1) = normalise(self.y, self.height);
        let (ox0, ox1) = normalise(other.x, other.width);
        let (oy0, oy1) = normalise(other.y, other.height);
        x1 > ox0 && x0 < ox1 && y1 > oy0 && y0 < oy1
    }
}

fn normalise(origin: f32, size: f32) -> (f32, f32) {
    if size < 0.0 {
        (origin + size, origin)
    } else {
        (origin, origin + size)
    }
}

/// A 4 × 4 matrix of floats in row-major order, applied to column vectors (`P = M · (v, 1)`),
/// the convention of SPEC-04B REQ-E3D-406 and REQ-E3D-407. The default value is the zero
/// matrix.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Mat4 {
    cells: [[f32; 4]; 4],
}

impl Mat4 {
    /// The identity matrix.
    pub const IDENTITY: Self = Self {
        cells: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ],
    };

    /// Builds the translation by `t`.
    #[must_use]
    pub fn translation(t: Vec3) -> Self {
        let mut m = Self::IDENTITY;
        m.cells[0][3] = t.x;
        m.cells[1][3] = t.y;
        m.cells[2][3] = t.z;
        m
    }

    /// Builds the diagonal scale by `s`.
    #[must_use]
    pub fn scale_matrix(s: Vec3) -> Self {
        let mut m = Self::IDENTITY;
        m.cells[0][0] = s.x;
        m.cells[1][1] = s.y;
        m.cells[2][2] = s.z;
        m
    }

    /// Rotation about X by `degrees` (REQ-E3D-407).
    #[must_use]
    pub fn rotation_x(degrees: f32) -> Self {
        let (s, c) = sin_cos(degrees);
        let mut m = Self::IDENTITY;
        m.cells[1][1] = c;
        m.cells[1][2] = -s;
        m.cells[2][1] = s;
        m.cells[2][2] = c;
        m
    }

    /// Rotation about Y by `degrees` (REQ-E3D-407).
    #[must_use]
    pub fn rotation_y(degrees: f32) -> Self {
        let (s, c) = sin_cos(degrees);
        let mut m = Self::IDENTITY;
        m.cells[0][0] = c;
        m.cells[0][2] = s;
        m.cells[2][0] = -s;
        m.cells[2][2] = c;
        m
    }

    /// Rotation about Z by `degrees` (REQ-E3D-407).
    #[must_use]
    pub fn rotation_z(degrees: f32) -> Self {
        let (s, c) = sin_cos(degrees);
        let mut m = Self::IDENTITY;
        m.cells[0][0] = c;
        m.cells[0][1] = -s;
        m.cells[1][0] = s;
        m.cells[1][1] = c;
        m
    }

    /// Cell at (`row`, `column`), or `None` when a coordinate is outside 0..3.
    #[must_use]
    pub fn get(&self, row: usize, column: usize) -> Option<f32> {
        self.cells
            .get(row)
            .and_then(|cells| cells.get(column))
            .copied()
    }

    /// Applies the matrix to the point `v` (w = 1).
    #[must_use]
    pub fn transform_point(&self, v: Vec3) -> Vec3 {
        let m = &self.cells;
        Vec3::new(
            (m[0][0] * v.x) + (m[0][1] * v.y) + (m[0][2] * v.z) + m[0][3],
            (m[1][0] * v.x) + (m[1][1] * v.y) + (m[1][2] * v.z) + m[1][3],
            (m[2][0] * v.x) + (m[2][1] * v.y) + (m[2][2] * v.z) + m[2][3],
        )
    }

    /// The sixteen cells in row-major order.
    #[must_use]
    pub fn to_row_major(&self) -> [f32; 16] {
        let mut out = [0.0; 16];
        for (i, row) in self.cells.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(row);
        }
        out
    }
}

fn sin_cos(degrees: f32) -> (f32, f32) {
    let r = degrees * std::f32::consts::PI / 180.0;
    (r.sin(), r.cos())
}

impl Index<(usize, usize)> for Mat4 {
    type Output = f32;

    /// Cell at (row, column); both must be 0 to 3.
    fn index(&self, (row, column): (usize, usize)) -> &f32 {
        &self.cells[row][column]
    }
}

impl IndexMut<(usize, usize)> for Mat4 {
    fn index_mut(&mut self, (row, column): (usize, usize)) -> &mut f32 {
        &mut self.cells[row][column]
    }
}

impl Mul for Mat4 {
    type Output = Self;

    fn mul(self, b: Self) -> Self {
        let mut r = Self::default();
        for i in 0..4 {
            for j in 0..4 {
                let mut sum = 0.0;
                for k in 0..4 {
                    sum += self.cells[i][k] * b.cells[k][j];
                }
                r.cells[i][j] = sum;
            }
        }
        r
    }
}
