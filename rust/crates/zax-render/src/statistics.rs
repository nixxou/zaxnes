//! What the last rendered frame cost, for the performance report of SPEC-06 REQ-SCN-064 and the
//! debug overlay.

use std::time::Duration;

use crate::geometry;

/// What the last rendered frame cost (REQ-SCN-064).
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // Two independent pass flags of the report.
pub struct RenderStatistics {
    /// Tile draws of the scene passes, shadow pass included (the draw calls of REQ-SCN-064
    /// before batching).
    pub draw_calls: u32,
    /// GL draw calls actually issued: consecutive tiles that share every setting but their
    /// placement go out as one instanced call (Rust-only).
    pub batches: u32,
    /// Triangles submitted, counted once per draw call.
    pub triangles: u64,
    /// Opaque tiles drawn.
    pub opaque_tiles: u32,
    /// Transparent tiles drawn.
    pub transparent_tiles: u32,
    /// Transparent objects drawn: the shape instances those tiles are grouped into for the
    /// depth pass and the distance sort (AMD-012).
    pub transparent_objects: u32,
    /// HUD tiles drawn.
    pub hud_tiles: u32,
    /// Tiles that went into the shadow map.
    pub shadow_casters: u32,
    /// Meshes resident on the GPU.
    pub resident_meshes: u32,
    /// Meshes uploaded since the renderer was created.
    pub mesh_uploads: u32,
    /// Object textures resident on the GPU.
    pub resident_textures: u32,
    /// Whether a shadow map was produced for this frame.
    pub shadow_pass: bool,
    /// Side of the shadow map in texels the hardware granted (AMD-034 §2): the requested size,
    /// or the largest smaller one whose depth framebuffer came out complete.
    pub shadow_map_size: i32,
    /// Side of the shadow map in texels that was asked for (AMD-034 §2).
    pub requested_shadow_map_size: i32,
    /// Whether a skybox was drawn for this frame (REQ-SCN-049).
    pub skybox_pass: bool,
    /// Samples per pixel the scene pass was granted (AMD-032 §2): 1 when anti-aliasing is off
    /// or when the implementation refused what was asked for.
    pub anti_alias_samples: i32,
    /// Opaque tiles turned transparent this frame because they hide the followed shape
    /// (REQ-SCN-022).
    pub occluders: u32,
    /// CPU time of the collection of the tiles into the buckets (Rust-only, for the frame
    /// profiler).
    pub collect_time: Duration,
    /// GPU time of the scene passes of an earlier frame, when a measure arrived during this one
    /// (Rust-only, for the dynamic resolution); `None` without one or without timer queries.
    pub gpu_time: Option<Duration>,
}

impl Default for RenderStatistics {
    fn default() -> Self {
        Self {
            draw_calls: 0,
            batches: 0,
            triangles: 0,
            opaque_tiles: 0,
            transparent_tiles: 0,
            transparent_objects: 0,
            hud_tiles: 0,
            shadow_casters: 0,
            resident_meshes: 0,
            mesh_uploads: 0,
            resident_textures: 0,
            shadow_pass: false,
            shadow_map_size: geometry::SHADOW_MAP_SIZE,
            requested_shadow_map_size: geometry::SHADOW_MAP_SIZE,
            skybox_pass: false,
            anti_alias_samples: 1,
            occluders: 0,
            collect_time: Duration::ZERO,
            gpu_time: None,
        }
    }
}

impl RenderStatistics {
    /// Clears the per-frame counters; the resident counts and the shadow map sizes stay.
    pub(crate) fn begin_frame(&mut self) {
        self.draw_calls = 0;
        self.batches = 0;
        self.triangles = 0;
        self.opaque_tiles = 0;
        self.transparent_tiles = 0;
        self.transparent_objects = 0;
        self.hud_tiles = 0;
        self.shadow_casters = 0;
        self.shadow_pass = false;
        self.skybox_pass = false;
        self.anti_alias_samples = 1;
        self.occluders = 0;
        self.gpu_time = None;
        self.collect_time = Duration::ZERO;
    }
}
