//! The light's view volume for one frame: the orthographic box that covers the shadow casters
//! within 512 units of the camera (SPEC-06 REQ-SCN-030).

use crate::camera::to_gl;
use crate::geometry;
use crate::numerics::{Matrix4x4, Vector3};

/// The light's view volume.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowVolume {
    /// Light view-projection matrix, OpenGL frame.
    pub view_projection: Matrix4x4,
    /// Depth of the box in scene units, used to scale the depth bias.
    pub depth_range: f32,
    /// Largest side of the box across the light, in scene units.
    pub extent: f32,
    /// False when nothing casts a shadow this frame.
    pub has_content: bool,
}

impl ShadowVolume {
    /// The empty volume.
    pub const EMPTY: Self = Self {
        view_projection: Matrix4x4::IDENTITY,
        depth_range: 1.0,
        extent: 0.0,
        has_content: false,
    };

    /// The side of one texel of a `map_size`-texel map, in scene units.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // Map sizes are at most 8192.
    pub fn texel_size(&self, map_size: i32) -> f32 {
        if map_size <= 0 {
            0.0
        } else {
            self.extent / map_size as f32
        }
    }

    /// Builds the volume from the scene-space bounds of the casters, the eye and the light
    /// propagation direction. The bounds are first restricted to the 512-unit box around the
    /// eye (REQ-SCN-030), then fitted so that the texels are spent on what actually casts rather
    /// than on the empty space the sphere mostly contains (Q-019).
    #[must_use]
    pub fn fit(
        bounds_min: Vector3,
        bounds_max: Vector3,
        eye: Vector3,
        light_direction: Vector3,
    ) -> Self {
        let limit_min = eye - Vector3::splat(geometry::SHADOW_DISTANCE);
        let limit_max = eye + Vector3::splat(geometry::SHADOW_DISTANCE);
        let mut min = Vector3::max(bounds_min, limit_min);
        let mut max = Vector3::min(bounds_max, limit_max);
        if max.x < min.x || max.y < min.y || max.z < min.z {
            return Self::EMPTY;
        }

        // A little slack so that the bias and the normal offset never push a caster out.
        min -= Vector3::splat(1.0);
        max += Vector3::splat(1.0);

        let forward = if light_direction.length_squared() > 0.0 {
            light_direction.normalize()
        } else {
            Vector3::new(0.0, -1.0, 0.0)
        };
        let forward_gl = to_gl(forward);
        let up = if forward_gl.y.abs() > 0.99 {
            Vector3::UNIT_Z
        } else {
            Vector3::UNIT_Y
        };

        let center_gl = to_gl((min + max) * 0.5);
        let radius = (Vector3::distance(min, max) * 0.5).max(1.0);
        let view = Matrix4x4::create_look_at(center_gl - (forward_gl * radius), center_gl, up);

        // Exact extents of the eight corners in light space, so the texels are not wasted.
        let mut low = Vector3::splat(f32::MAX);
        let mut high = Vector3::splat(f32::MIN);
        for corner in 0..8 {
            let point = Vector3::new(
                if corner & 1 == 0 { min.x } else { max.x },
                if corner & 2 == 0 { min.y } else { max.y },
                if corner & 4 == 0 { min.z } else { max.z },
            );
            let light = to_gl(point).transform(&view);
            low = Vector3::min(low, light);
            high = Vector3::max(high, light);
        }

        // Light space looks along −Z, so the box spans [−maxZ, −minZ] in front of the eye.
        let near = (-high.z).max(0.01);
        let mut far = -low.z;
        if far <= near {
            far = near + 1.0;
        }

        let projection =
            Matrix4x4::create_orthographic_off_center(low.x, high.x, low.y, high.y, near, far);
        Self {
            view_projection: Matrix4x4::multiply(&view, &projection),
            depth_range: far - near,
            extent: (high.x - low.x).max(high.y - low.y),
            has_content: true,
        }
    }
}
