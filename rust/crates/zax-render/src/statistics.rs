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
    /// Texels the resident object textures occupy, the unit of REQ-E3D-564 and of the budget of
    /// AMD-051 REQ-E3D-644.
    pub texture_texels: u64,
    /// Distinct patterns whose texture could not be allocated since the last wholesale rebuild
    /// (AMD-051 REQ-E3D-644). 0 whenever everything fits, which is every ordinary frame.
    pub patterns_without_texture: u64,
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
    /// Whether the scene pass drew with the centroid variant of AMD-058, which it does exactly
    /// when `video.anti_alias` is not 0 (REQ-SCN-065). Reported so that the scope of the variant
    /// is observable from outside the renderer, rather than inferred from the picture.
    pub centroid_interpolation: bool,
    /// Opaque tiles turned transparent this frame because they hide the followed shape
    /// (REQ-SCN-022).
    pub occluders: u32,
    /// Screen-space passes of AMD-055 REQ-SCN-080 submitted for this frame — the reflections of
    /// REQ-SCN-084 and the occlusion of REQ-SCN-083, with their blurs.
    ///
    /// **0 is the requirement**: in Classic, and in Enhanced with `spl.ao` = 0 and
    /// `spl.reflections` = 0 (the defaults of REQ-SCN-079), no pass runs at all, which is what
    /// makes Enhanced cost what Classic costs. Reported so that it is observable from outside the
    /// renderer rather than inferred from a frame time.
    pub screen_effect_passes: u32,
    /// Bytes the targets of those two passes occupy — colour, depth and the blur targets of
    /// REQ-SCN-083 and REQ-SCN-084. **0 in Classic**, where none of them is allocated; back to 0
    /// when the style returns to Classic and they are released.
    pub screen_effect_bytes: u64,
    /// Shader programs AMD-055 added to the ones the renderer was created with: the variants of
    /// the voxel grid (REQ-SCN-081) and of the pixel shadows (REQ-SCN-082), and the programs of
    /// the two screen passes. **0 in Classic**, and 0 in Enhanced until an effect is turned on.
    pub style_programs: u32,
    /// Levels of the depth pyramid the **multi-scale** estimator of REQ-SCN-083's *Mode MSVO*
    /// built this frame: [`crate::style::MSVO_LEVELS`] with `spl.ao` = 1 and the port in force,
    /// and **0** with `spl.ao` = 2, with the occlusion off, in Classic, and under the fallback
    /// **(D)**.
    ///
    /// It is the one thing that tells the two estimators apart from outside the renderer: both
    /// write the same `O` through the same composite, so the picture does not say which ran, and
    /// the fallback to Depth has to be observable (REQ-SCN-083).
    pub occlusion_levels: u32,
    /// Passes the depth-detecting SMAA of AMD-060 REQ-SCN-085 submitted for this frame: 3 at
    /// `video.anti_alias` = 3, and **0 at every other level**, which is REQ-SCN-086's "aucune
    /// passe […] NE DOIT exister ni être soumise" made observable from outside the renderer.
    ///
    /// It is also how the fallback of REQ-SCN-086 is told from the mode running: a frame that gave
    /// the filter up reads 0 here while the setting still reads 3.
    pub smaa_passes: u32,
    /// Bytes the targets of those passes occupy — the edges, the blending weights and the blended
    /// colour. **0 at every level but 3**, and back to 0 when the level leaves 3 and they are
    /// released. The two precomputed tables are not counted: they do not grow with the resolution
    /// (`gl/smaa.rs::bytes` says why).
    pub smaa_bytes: u64,
    /// Shader programs the filter of REQ-SCN-085 holds: the three of its passes. **0 at every
    /// level but 3**, the same way `style_programs` is 0 in Classic.
    pub smaa_programs: u32,
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
            texture_texels: 0,
            patterns_without_texture: 0,
            shadow_pass: false,
            shadow_map_size: geometry::SHADOW_MAP_SIZE,
            requested_shadow_map_size: geometry::SHADOW_MAP_SIZE,
            skybox_pass: false,
            anti_alias_samples: 1,
            centroid_interpolation: false,
            occluders: 0,
            screen_effect_passes: 0,
            screen_effect_bytes: 0,
            style_programs: 0,
            occlusion_levels: 0,
            smaa_passes: 0,
            smaa_bytes: 0,
            smaa_programs: 0,
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
        self.centroid_interpolation = false;
        self.occluders = 0;
        self.screen_effect_passes = 0;
        self.smaa_passes = 0;
        self.gpu_time = None;
        self.collect_time = Duration::ZERO;
    }
}
