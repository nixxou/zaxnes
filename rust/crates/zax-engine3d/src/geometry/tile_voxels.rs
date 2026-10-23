// Portions adapted from 3DNes_Engine (c) Geod Studio, GPL-3.0

//! The voxel field of one distinct tile image of a 3D object (SPEC-04B REQ-E3D-420,
//! REQ-E3D-422).

/// Tile width in voxels.
pub const SIZE_X: i32 = 8;

/// Tile height in voxels.
pub const SIZE_Y: i32 = 8;

/// Largest depth a 3D object can have (`sizeZ` is 0 to 96).
pub const MAX_DEPTH: i32 = 96;

/// Slot index of the X axis.
pub const AXIS_X: usize = 0;

/// Slot index of the Y axis.
pub const AXIS_Y: usize = 1;

/// Slot index of the Z axis.
pub const AXIS_Z: usize = 2;

const EMPTY: i32 = -1;

/// The voxel field of one distinct tile image of a 3D object: `8 × 8 × D` voxels, each
/// carrying one slot per axis in the GREEDY algorithms and a single slot in MARCHING
/// (SPEC-04B REQ-E3D-420, REQ-E3D-422).
///
/// A voxel is addressed by `(i, j, k)`: `i` the column (0 to 7, rightwards), `j` the row (0 to
/// 7, *upwards*) and `k` the depth slice (0 to `D − 1`, `k = 0` on the viewer's side). A slot
/// is either empty or the coordinates `(tx, ty)` of a texel of the object's texture.
///
/// Writing a texel whose value is the transparent colour `e` leaves the slot *empty*, and
/// several writes on one slot resolve by "last written wins", including when the last one
/// empties it (REQ-E3D-423).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TileVoxels {
    depth: i32,
    slots_per_voxel: usize,
    slots: Vec<i32>,
}

impl TileVoxels {
    /// Creates an empty field of depth `depth` (clamped to 0 to [`MAX_DEPTH`]). `per_axis_slots`
    /// is true for the GREEDY algorithms, where each voxel carries three independent slots,
    /// and false for MARCHING, where it carries one.
    #[must_use]
    pub fn new(depth: i32, per_axis_slots: bool) -> Self {
        let depth = depth.clamp(0, MAX_DEPTH);
        let slots_per_voxel = if per_axis_slots { 3 } else { 1 };
        #[allow(clippy::cast_sign_loss)] // Clamped non-negative above.
        let count = (SIZE_X * SIZE_Y * depth) as usize * slots_per_voxel;
        Self {
            depth,
            slots_per_voxel,
            slots: vec![EMPTY; count],
        }
    }

    /// Depth `D` of the field in voxels.
    #[must_use]
    pub const fn depth(&self) -> i32 {
        self.depth
    }

    /// Three in the GREEDY algorithms, one in MARCHING.
    #[must_use]
    pub const fn slots_per_voxel(&self) -> usize {
        self.slots_per_voxel
    }

    /// True when the field holds no voxel at all (`D = 0`, REQ-E3D-420).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// True when `(i, j, k)` is inside `[0,8) × [0,8) × [0,D)`.
    #[must_use]
    pub const fn contains(&self, i: i32, j: i32, k: i32) -> bool {
        i >= 0 && i < SIZE_X && j >= 0 && j < SIZE_Y && k >= 0 && k < self.depth
    }

    /// Assigns the texel (`tx`, `ty`) to the slot `axis` of the voxel, or to its only slot in
    /// MARCHING. The slot is left *empty* when the texel value is the transparent colour
    /// (REQ-E3D-423). A write outside the field has no effect (REQ-E3D-421).
    #[allow(clippy::too_many_arguments)] // Voxel, slot, texel and transparency.
    pub fn write(
        &mut self,
        i: i32,
        j: i32,
        k: i32,
        axis: usize,
        tx: i32,
        ty: i32,
        transparent: bool,
    ) {
        if let Some(slot) = self.base(i, j, k) {
            let value = if transparent { EMPTY } else { pack(tx, ty) };
            let target = if self.slots_per_voxel == 1 { 0 } else { axis };
            self.slots[slot + target] = value;
        }
    }

    /// Assigns the texel to every slot of the voxel, whatever the algorithm (REQ-E3D-422).
    pub fn write_all(&mut self, i: i32, j: i32, k: i32, tx: i32, ty: i32, transparent: bool) {
        if let Some(slot) = self.base(i, j, k) {
            let value = if transparent { EMPTY } else { pack(tx, ty) };
            self.slots[slot..slot + self.slots_per_voxel].fill(value);
        }
    }

    /// True when the slot `axis` of the voxel is not empty.
    #[must_use]
    pub fn occupied(&self, i: i32, j: i32, k: i32, axis: usize) -> bool {
        self.texel(i, j, k, axis).is_some()
    }

    /// The texel of a slot, or `None` when the voxel is outside the field or the slot is
    /// empty.
    #[must_use]
    pub fn texel(&self, i: i32, j: i32, k: i32, axis: usize) -> Option<(i32, i32)> {
        let slot = self.base(i, j, k)?;
        let target = if self.slots_per_voxel == 1 { 0 } else { axis };
        let value = self.slots[slot + target];
        (value != EMPTY).then_some((value & 0xFFFF, (value >> 16) & 0xFFFF))
    }

    #[allow(clippy::cast_sign_loss)] // The voxel is checked inside the field first.
    fn base(&self, i: i32, j: i32, k: i32) -> Option<usize> {
        self.contains(i, j, k)
            .then(|| ((((k * SIZE_Y) + j) * SIZE_X + i) as usize) * self.slots_per_voxel)
    }
}

/// Packs a texel coordinate pair into one slot. Both components are clamped to `[0, 0x7FFF]`,
/// which keeps the packed value positive and so distinct from the empty slot; the
/// construction rules of §3.3 only ever write coordinates inside the pattern image, far below
/// that bound.
pub(crate) const fn pack(tx: i32, ty: i32) -> i32 {
    (clamp_coordinate(ty) << 16) | clamp_coordinate(tx)
}

const fn clamp_coordinate(value: i32) -> i32 {
    if value < 0 {
        0
    } else if value > 0x7FFF {
        0x7FFF
    } else {
        value
    }
}

/// The two coordinates of a packed texel.
pub(crate) const fn unpack(value: i32) -> (i32, i32) {
    (value & 0xFFFF, (value >> 16) & 0xFFFF)
}
