//! The occlusion transparency of SPEC-06 REQ-SCN-022: in FPV with a followed shape, every tile
//! of **another** shape whose geometry crosses the segment from the eye to the target is drawn
//! transparent for that frame.
//!
//! The test is done on the tile's axis-aligned bounding box in scene coordinates, which is the
//! cheapest conservative test: a tile that is near the line of sight but does not quite hide the
//! character may fade too, never the other way round.

use zax_engine3d::draw_list::DrawTile;
use zax_engine3d::frame::ShapeId;
use zax_engine3d::geometry::tile_mesh::TileMesh;
use zax_engine3d::vectors::{Mat4, Vec3};

use crate::camera::{CameraMode, CameraView};
use crate::numerics::Vector3;
use crate::scene::SceneState;

/// The occlusion test of one frame.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Occlusion {
    eye: Vector3,
    target: Vector3,
    followed: Option<ShapeId>,
}

impl Occlusion {
    /// The disabled test.
    pub const NONE: Self = Self {
        eye: Vector3::ZERO,
        target: Vector3::ZERO,
        followed: None,
    };

    /// The test for `scene`: enabled only in FPV with a followed shape (REQ-SCN-021,
    /// REQ-SCN-022).
    #[must_use]
    pub fn for_scene(scene: &SceneState, view: &CameraView) -> Self {
        match scene.followed_shape {
            Some(shape) if scene.camera.mode() == CameraMode::Fpv => Self {
                eye: view.eye,
                target: view.target,
                followed: Some(shape),
            },
            _ => Self::NONE,
        }
    }

    /// Whether any tile can become an occluder this frame.
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.followed.is_some()
    }

    /// Whether `tile` hides the followed shape this frame. `mesh` is its mesh, for the local
    /// bounding box, and `model` its placement, the sprite nudge included.
    #[must_use]
    pub fn hides(&self, tile: &DrawTile, mesh: &TileMesh, model: &Mat4) -> bool {
        match self.followed {
            Some(followed) if tile.instance.shape != followed => {
                let (min, max) = world_bounds(mesh, model);
                segment_hits_box(self.eye, self.target, min, max)
            }
            _ => false,
        }
    }
}

/// The scene-space bounding box of `mesh` placed by `model`.
#[must_use]
pub fn world_bounds(mesh: &TileMesh, model: &Mat4) -> (Vector3, Vector3) {
    box_bounds(mesh.bounds_min(), mesh.bounds_max(), model)
}

/// The scene-space bounding box of the eight corners of the local box `[lo, hi]` placed by
/// `model`.
#[must_use]
pub fn box_bounds(lo: Vec3, hi: Vec3, model: &Mat4) -> (Vector3, Vector3) {
    let mut min = Vector3::splat(f32::MAX);
    let mut max = Vector3::splat(f32::MIN);
    for corner in 0..8 {
        let point = Vec3::new(
            if corner & 1 == 0 { lo.x } else { hi.x },
            if corner & 2 == 0 { lo.y } else { hi.y },
            if corner & 4 == 0 { lo.z } else { hi.z },
        );
        let placed = Vector3::from(model.transform_point(point));
        min = Vector3::min(min, placed);
        max = Vector3::max(max, placed);
    }

    (min, max)
}

/// Whether the segment `[a, b]` meets the axis-aligned box, by the usual slab test.
#[must_use]
pub fn segment_hits_box(a: Vector3, b: Vector3, min: Vector3, max: Vector3) -> bool {
    let direction = b - a;
    let mut enter = 0.0_f32;
    let mut exit = 1.0_f32;

    let axes = [
        (a.x, direction.x, min.x, max.x),
        (a.y, direction.y, min.y, max.y),
        (a.z, direction.z, min.z, max.z),
    ];
    for (origin, delta, low, high) in axes {
        if delta.abs() < 1e-6 {
            if origin < low || origin > high {
                return false;
            }

            continue;
        }

        let t0 = (low - origin) / delta;
        let t1 = (high - origin) / delta;
        let (t0, t1) = if t0 > t1 { (t1, t0) } else { (t0, t1) };

        enter = enter.max(t0);
        exit = exit.min(t1);
        if enter > exit {
            return false;
        }
    }

    true
}
