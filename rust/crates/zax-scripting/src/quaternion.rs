//! The quaternion arithmetic of `System.Numerics.Quaternion`, which the `Quaternion` type of
//! SPEC-05 §4.3 is defined with: the same formulas in the same order, single precision.

use zax_engine3d::vectors::Vec3;

/// A quaternion `(x, y, z, w)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Quat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

/// Below this `1 - |cos|` two quaternions are close enough to be mixed linearly.
const SLERP_EPSILON: f32 = 1e-6;

impl Quat {
    pub const IDENTITY: Self = Self::new(0.0, 0.0, 0.0, 1.0);

    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    pub const fn from_parts(parts: [f32; 4]) -> Self {
        Self::new(parts[0], parts[1], parts[2], parts[3])
    }

    pub const fn parts(self) -> [f32; 4] {
        [self.x, self.y, self.z, self.w]
    }

    pub fn length_squared(self) -> f32 {
        (self.x * self.x) + (self.y * self.y) + (self.z * self.z) + (self.w * self.w)
    }

    pub fn dot(self, other: Self) -> f32 {
        (self.x * other.x) + (self.y * other.y) + (self.z * other.z) + (self.w * other.w)
    }

    pub fn normalize(self) -> Self {
        let inverse = 1.0 / self.length_squared().sqrt();
        Self::new(
            self.x * inverse,
            self.y * inverse,
            self.z * inverse,
            self.w * inverse,
        )
    }

    pub fn negate(self) -> Self {
        Self::new(-self.x, -self.y, -self.z, -self.w)
    }

    /// `self * other`: applies `other` first, then `self` (§4.3).
    pub fn multiply(self, other: Self) -> Self {
        let cross_x = (self.y * other.z) - (self.z * other.y);
        let cross_y = (self.z * other.x) - (self.x * other.z);
        let cross_z = (self.x * other.y) - (self.y * other.x);
        let dot = (self.x * other.x) + (self.y * other.y) + (self.z * other.z);

        Self::new(
            (self.x * other.w) + (other.x * self.w) + cross_x,
            (self.y * other.w) + (other.y * self.w) + cross_y,
            (self.z * other.w) + (other.z * self.w) + cross_z,
            (self.w * other.w) - dot,
        )
    }

    /// Rotates a vector by this quaternion.
    pub fn rotate(self, value: Vec3) -> Vec3 {
        let x2 = self.x + self.x;
        let y2 = self.y + self.y;
        let z2 = self.z + self.z;

        let wx2 = self.w * x2;
        let wy2 = self.w * y2;
        let wz2 = self.w * z2;
        let xx2 = self.x * x2;
        let xy2 = self.x * y2;
        let xz2 = self.x * z2;
        let yy2 = self.y * y2;
        let yz2 = self.y * z2;
        let zz2 = self.z * z2;

        Vec3::new(
            (value.x * (1.0 - yy2 - zz2)) + (value.y * (xy2 - wz2)) + (value.z * (xz2 + wy2)),
            (value.x * (xy2 + wz2)) + (value.y * (1.0 - xx2 - zz2)) + (value.z * (yz2 - wx2)),
            (value.x * (xz2 - wy2)) + (value.y * (yz2 + wx2)) + (value.z * (1.0 - xx2 - yy2)),
        )
    }

    /// The rotation of `angle` radians around the unit vector `axis`.
    pub fn from_axis_angle(axis: Vec3, angle: f32) -> Self {
        let half = angle * 0.5;
        let sine = half.sin();
        let cosine = half.cos();
        Self::new(axis.x * sine, axis.y * sine, axis.z * sine, cosine)
    }

    /// Spherical interpolation, shortest way round.
    pub fn slerp(from: Self, to: Self, t: f32) -> Self {
        let mut cosine = from.dot(to);
        let mut flip = false;
        if cosine < 0.0 {
            flip = true;
            cosine = -cosine;
        }

        let (first, second) = if cosine > 1.0 - SLERP_EPSILON {
            (1.0 - t, if flip { -t } else { t })
        } else {
            let omega = cosine.acos();
            let inverse_sine = 1.0 / omega.sin();
            let first = ((1.0 - t) * omega).sin() * inverse_sine;
            let second = if flip {
                -(t * omega).sin() * inverse_sine
            } else {
                (t * omega).sin() * inverse_sine
            };
            (first, second)
        };

        Self::new(
            (first * from.x) + (second * to.x),
            (first * from.y) + (second * to.y),
            (first * from.z) + (second * to.z),
            (first * from.w) + (second * to.w),
        )
    }
}
