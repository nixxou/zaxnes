//! Small value types of the profile model: integer and float vectors, the 4x4 matrix and the
//! packed colour. Float equality is by bit pattern, so that a value read from a file and written
//! back compares equal to itself (NaN included).

/// A pair of integers (a tile size, a constraint size).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Vec2I {
    /// Horizontal component.
    pub x: i32,
    /// Vertical component.
    pub y: i32,
}

impl Vec2I {
    /// A pair from its components.
    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// The "no constraint" size, 99 by 99.
    pub const UNCONSTRAINED: Self = Self::new(99, 99);
}

/// Three floats.
#[derive(Debug, Clone, Copy, Default)]
pub struct Vec3F {
    /// First component.
    pub x: f32,
    /// Second component.
    pub y: f32,
    /// Third component.
    pub z: f32,
}

impl Vec3F {
    /// The zero vector.
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);

    /// A vector from its components.
    #[must_use]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    fn bits(self) -> [u32; 3] {
        [self.x.to_bits(), self.y.to_bits(), self.z.to_bits()]
    }
}

impl PartialEq for Vec3F {
    fn eq(&self, other: &Self) -> bool {
        self.bits() == other.bits()
    }
}

impl Eq for Vec3F {}

/// Four floats (the clipping box).
#[derive(Debug, Clone, Copy, Default)]
pub struct Vec4F {
    /// First component.
    pub x: f32,
    /// Second component.
    pub y: f32,
    /// Third component.
    pub z: f32,
    /// Fourth component.
    pub w: f32,
}

impl Vec4F {
    /// A vector from its components.
    #[must_use]
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    /// The component at `index` (0 to 3), or none outside that range.
    #[must_use]
    pub const fn get(self, index: usize) -> Option<f32> {
        match index {
            0 => Some(self.x),
            1 => Some(self.y),
            2 => Some(self.z),
            3 => Some(self.w),
            _ => None,
        }
    }

    fn bits(self) -> [u32; 4] {
        [
            self.x.to_bits(),
            self.y.to_bits(),
            self.z.to_bits(),
            self.w.to_bits(),
        ]
    }
}

impl PartialEq for Vec4F {
    fn eq(&self, other: &Self) -> bool {
        self.bits() == other.bits()
    }
}

impl Eq for Vec4F {}

/// A 4x4 float matrix, row-major.
#[derive(Debug, Clone, Copy, Default)]
pub struct Mat4F([f32; Mat4F::CELL_COUNT]);

impl Mat4F {
    /// Number of cells.
    pub const CELL_COUNT: usize = 16;

    /// A matrix from its sixteen cells, row by row.
    #[must_use]
    pub const fn new(row_major_cells: [f32; Self::CELL_COUNT]) -> Self {
        Self(row_major_cells)
    }

    /// The cells, row by row.
    #[must_use]
    pub const fn cells(&self) -> &[f32; Self::CELL_COUNT] {
        &self.0
    }

    /// The cell at (`row`, `column`), or none when either is outside 0 to 3.
    #[must_use]
    pub fn get(&self, row: usize, column: usize) -> Option<f32> {
        if row < 4 && column < 4 {
            self.0.get(row * 4 + column).copied()
        } else {
            None
        }
    }
}

impl PartialEq for Mat4F {
    fn eq(&self, other: &Self) -> bool {
        self.0
            .iter()
            .zip(&other.0)
            .all(|(left, right)| left.to_bits() == right.to_bits())
    }
}

impl Eq for Mat4F {}

/// A colour packed as `0xAARRGGBB` in a signed integer, as the files store it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ArgbColor {
    /// The packed value.
    pub packed: i32,
}

impl ArgbColor {
    /// A colour from its packed value.
    #[must_use]
    pub const fn new(packed: i32) -> Self {
        Self { packed }
    }

    /// Red channel.
    #[must_use]
    pub const fn r(self) -> u8 {
        self.packed.to_le_bytes()[2]
    }

    /// Green channel.
    #[must_use]
    pub const fn g(self) -> u8 {
        self.packed.to_le_bytes()[1]
    }

    /// Blue channel.
    #[must_use]
    pub const fn b(self) -> u8 {
        self.packed.to_le_bytes()[0]
    }

    /// Alpha as stored; the profiles ignore it.
    #[must_use]
    pub const fn stored_alpha(self) -> u8 {
        self.packed.to_le_bytes()[3]
    }
}
