// Portions adapted from 3DNes_Engine (c) Geod Studio, GPL-3.0

//! Builds the voxel fields of a 3D object, one per distinct tile image of its pattern, for
//! every geometry type of SPEC-04B §3.3 (REQ-E3D-420 to 452).
//!
//! The construction rules write on *pattern coordinates* `(x, y, k)`, `y = 0` at the top of
//! the pattern image; each write goes to the field of the tile of cell `(⌊x/8⌋, ⌊y/8⌋)` at
//! the voxel `(x mod 8, 7 − (y mod 8))`, and is dropped when the cell is empty or outside the
//! grid (REQ-E3D-421). Cells that carry the same tile image share one field, so all their
//! writes pile up on it (REQ-E3D-425) — that is where the inner walls of the original come
//! from.
//!
//! All reads of the texture go through [`ObjectTexture::texel`], hence modulo the texture size
//! (REQ-E3D-426).

use crate::geometry::texture::ObjectTexture;
use crate::geometry::tile_voxels::{TileVoxels, AXIS_X, AXIS_Y, AXIS_Z, MAX_DEPTH};
use crate::image::TileImage;
use crate::instance::PatternInstance;
use crate::pattern::EnginePattern;
use crate::settings::{Geo3DType, RenderAlgorithm};

/// The position of each image of `pattern` in its image list, by image identity; `None` for
/// an identity the pattern does not hold.
pub(crate) fn image_index(pattern: &EnginePattern) -> Vec<Option<usize>> {
    let highest = pattern
        .images()
        .iter()
        .map(TileImage::id)
        .max()
        .unwrap_or(0);
    let mut index = vec![None; highest as usize + 1];
    for (n, image) in pattern.images().iter().enumerate() {
        index[image.id() as usize] = Some(n);
    }
    index
}

/// The fields of a 3D object, in the order of [`EnginePattern::images`], and its effective
/// depth `D'`: `D` for the non-cylindrical types, `max(B + 1)` over the segments built
/// otherwise (REQ-E3D-452).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoxelFields {
    /// One field per distinct image.
    pub fields: Vec<TileVoxels>,
    /// The depth placement and deformation must use.
    pub effective_depth: i32,
}

/// Builds one voxel field per distinct image of `pattern` (REQ-E3D-420 to 452).
///
/// `parameters` are the 3D parameters as the build hook left them (REQ-E3D-455): geometry,
/// depth, options, transparent colour and `fillrate`; `algorithm` is the effective mesh
/// algorithm (REQ-E3D-500).
#[must_use]
pub fn build(
    pattern: &EnginePattern,
    texture: &ObjectTexture,
    parameters: &PatternInstance,
    algorithm: RenderAlgorithm,
) -> VoxelFields {
    let depth = parameters.depth.clamp(0, MAX_DEPTH);
    let per_axis_slots = algorithm != RenderAlgorithm::Marching;
    let fields = (0..pattern.images().len())
        .map(|_| TileVoxels::new(depth, per_axis_slots))
        .collect();

    // D = 0: no voxel is built and the tile has no geometry (REQ-E3D-420); a cylinder without
    // a segment ends up with D' = 0 as well (REQ-E3D-452).
    if depth == 0 {
        return VoxelFields {
            fields,
            effective_depth: 0,
        };
    }

    let mut construction = Construction {
        pattern,
        texture,
        fields,
        field_of_image: image_index(pattern),
        pattern_width: pattern.width() * 8,
        pattern_height: pattern.height() * 8,
        depth,
        empty_color: ObjectTexture::empty_color(parameters.empty_color),
        solid: parameters.solid,
        full_warp: parameters.full_warp,
        flip: parameters.flip,
        anti_shadow: parameters.anti_shadow,
        fill_rate: parameters.fill_rate,
        geometry: parameters.geometry,
        marching: algorithm == RenderAlgorithm::Marching,
        effective_depth: depth,
    };
    construction.run();
    VoxelFields {
        fields: construction.fields,
        effective_depth: construction.effective_depth,
    }
}

/// The quadrant flags and texel lines of one contour point (REQ-E3D-446, 447).
#[derive(Debug, Clone, Copy)]
struct Quadrants {
    active: [bool; 4],
    texel: [i32; 4],
}

#[allow(clippy::struct_excessive_bools)] // The options of SPEC-04B §3.3.
struct Construction<'a> {
    pattern: &'a EnginePattern,
    texture: &'a ObjectTexture,
    fields: Vec<TileVoxels>,
    field_of_image: Vec<Option<usize>>,
    pattern_width: i32,
    pattern_height: i32,
    depth: i32,
    empty_color: i32,
    solid: bool,
    full_warp: bool,
    flip: bool,
    anti_shadow: bool,
    fill_rate: f32,
    geometry: Geo3DType,
    marching: bool,
    effective_depth: i32,
}

impl Construction<'_> {
    fn run(&mut self) {
        match self.geometry {
            Geo3DType::Cube => self.build_cube(),
            Geo3DType::VCylinder
            | Geo3DType::HCylinder
            | Geo3DType::HalfVCylinder
            | Geo3DType::HalfHCylinder => self.build_cylinder(),
            Geo3DType::Default => self.build_default(),
        }
    }

    // writes

    /// Assigns the texel (`tx`, `ty`) to the slot `axis` (or every slot when `None`) of the
    /// pattern coordinate (`x`, `y`, `k`) (REQ-E3D-421, 422, 423).
    #[allow(clippy::too_many_arguments)] // Pattern coordinate, slot and texel.
    fn write(&mut self, x: i32, y: i32, k: i32, axis: Option<usize>, tx: i32, ty: i32) {
        if x < 0 || x >= self.pattern_width || y < 0 || y >= self.pattern_height {
            return;
        }

        let transparent = self.is_transparent(tx, ty);
        let Some(field) = self.field_at(x >> 3, y >> 3) else {
            return;
        };

        match axis {
            Some(axis) => field.write(x & 7, 7 - (y & 7), k, axis, tx, ty, transparent),
            None => field.write_all(x & 7, 7 - (y & 7), k, tx, ty, transparent),
        }
    }

    /// Writes every slot of the voxel (REQ-E3D-422, "sur tous les axes").
    fn write_all(&mut self, x: i32, y: i32, k: i32, tx: i32, ty: i32) {
        self.write(x, y, k, None, tx, ty);
    }

    fn field_at(&mut self, cell_x: i32, cell_y: i32) -> Option<&mut TileVoxels> {
        let image = self.pattern.image_at(cell_x, cell_y)?;
        let index = (*self.field_of_image.get(image.id() as usize)?)?;
        self.fields.get_mut(index)
    }

    /// True when the texel is the transparent colour `e` (REQ-E3D-423, 424).
    fn is_transparent(&self, tx: i32, ty: i32) -> bool {
        self.texture.texel(tx, ty) == self.empty_color
    }

    // Default (REQ-E3D-440, 441, 442)

    fn build_default(&mut self) {
        // Canonical order: x ascending, then y, then k (REQ-E3D-440).
        for x in 0..self.pattern_width {
            for y in 0..self.pattern_height {
                if self.is_transparent(x, y) {
                    continue;
                }

                for k in 0..self.depth {
                    if !self.full_warp {
                        // Extrusion: every opaque pixel becomes a 1 x 1 x D prism.
                        self.write_all(x, y, k, x, y);
                    } else if self.marching {
                        // REQ-E3D-442: the slice k takes the lowest opaque texel of the
                        // interval [y, y + k], which always exists since t(x, y) is opaque.
                        let mut wrapped = y + k;
                        while self.is_transparent(x, wrapped) {
                            wrapped -= 1;
                        }

                        self.write_all(x, y, k, x, wrapped);
                    } else {
                        // REQ-E3D-441, wrapped extrusion, in this order: Y, then X, then Z.
                        self.write(x, y, k, Some(AXIS_Y), x, self.depth - 1 - k);
                        self.write(x, y, k, Some(AXIS_X), self.depth - 1 - k, y);
                        self.write(x, y, k, Some(AXIS_Z), x, y);
                    }
                }
            }
        }
    }

    // CUBE (REQ-E3D-443)

    fn build_cube(&mut self) {
        // The box covers the whole pattern image, empty cells included; writes that land in
        // an empty cell have no effect, so the box is open there (REQ-E3D-443 notes).
        let (w, h, d) = (self.pattern_width, self.pattern_height, self.depth);
        for x in 0..w {
            for k in 0..d {
                self.write(x, h - 1, k, Some(AXIS_Y), x, k);
                self.write(x, 0, k, Some(AXIS_Y), x, d - 1 - k);
            }
        }

        for y in 0..h {
            for k in 0..d {
                self.write(0, y, k, Some(AXIS_X), d - 1 - k, y);
                self.write(w - 1, y, k, Some(AXIS_X), d - 1 - k, y);
            }
        }

        for x in 0..w {
            for y in 0..h {
                self.write(x, y, d - 1, Some(AXIS_Z), x, y);
                self.write(x, y, 0, Some(AXIS_Z), x, y);
            }
        }
    }

    // cylinders (REQ-E3D-444 to 452)

    fn vertical(&self) -> bool {
        matches!(
            self.geometry,
            Geo3DType::VCylinder | Geo3DType::HalfVCylinder
        )
    }

    fn half(&self) -> bool {
        matches!(
            self.geometry,
            Geo3DType::HalfVCylinder | Geo3DType::HalfHCylinder
        )
    }

    /// True when the half section keeps its plus side: the flat face sits at the end of the
    /// segment that carries the centre.
    fn centre_at_first(&self) -> bool {
        (self.geometry == Geo3DType::HalfVCylinder && !self.flip)
            || (self.geometry == Geo3DType::HalfHCylinder && self.flip)
    }

    fn build_cylinder(&mut self) {
        // Every segment contributes max(B + 1) to the effective depth; without a segment the
        // object ends up with D' = 0 (REQ-E3D-452).
        self.effective_depth = 0;

        if self.vertical() {
            // V types: each row, x ascending.
            for y in 0..self.pattern_height {
                let mut x = 0;
                while x < self.pattern_width {
                    if !self.is_transparent(x, y) {
                        let mut last = x;
                        while last + 1 < self.pattern_width && !self.is_transparent(last + 1, y) {
                            last += 1;
                        }

                        self.segment(y, x, last);
                        x = last;
                    }

                    x += 1;
                }
            }

            return;
        }

        // H types: each column, y ascending.
        for x in 0..self.pattern_width {
            let mut y = 0;
            while y < self.pattern_height {
                if !self.is_transparent(x, y) {
                    let mut last = y;
                    while last + 1 < self.pattern_height && !self.is_transparent(x, last + 1) {
                        last += 1;
                    }

                    self.segment(x, y, last);
                    y = last;
                }

                y += 1;
            }
        }
    }

    /// Replaces one maximal segment of opaque pixels by an elliptic section in the plane
    /// (line, depth) (REQ-E3D-444 to 451). Doubled coordinates, in half pixels (REQ-E3D-445).
    /// `line` is the row (V types) or the column (H types) the segment lies on; `first` and
    /// `last` are its first and last pixels `s` and `f` along the line.
    fn segment(&mut self, line: i32, first: i32, last: i32) {
        let length = last - first;
        let (centre, major, minor) = if self.half() {
            let centre = if self.centre_at_first() {
                2 * first
            } else {
                2 * last
            };
            (centre, 2 * length, (self.depth - 1).min(2 * length))
        } else {
            (first + last, length, (self.depth - 1).min(length))
        };

        let centre_z = self.depth - 1;
        self.effective_depth = self.effective_depth.max(minor + 1);

        if major == 0 || minor == 0 {
            // Degenerate section: a single voxel at the centre (REQ-E3D-451).
            let u = centre.div_euclid(2);
            let z = centre_z.div_euclid(2);
            self.write_section(line, u, z, u);
            return;
        }

        let a2 = i64::from(major) * i64::from(major);
        let b2 = i64::from(minor) * i64::from(minor);
        let limit = a2 * b2;

        // Region 1, from (0, B): midpoint ellipse tracing in doubled coordinates.
        let (mut p, mut q) = (0, minor);
        while b2 * i64::from(p) <= a2 * i64::from(q) {
            self.quadrants(line, centre, centre_z, p, q, false);
            let next = i64::from(p) + 1;
            if (b2 * next * (next - 1)) + (a2 * i64::from(q) * i64::from(q - 1)) >= limit {
                q -= 1;
            }

            p += 1;
        }

        // Region 2, from (A, 0).
        let (mut p, mut q) = (major, 0);
        while a2 * i64::from(q) <= b2 * i64::from(p) {
            self.quadrants(line, centre, centre_z, p, q, true);
            let next = i64::from(q) + 1;
            if (a2 * next * (next - 1)) + (b2 * i64::from(p) * i64::from(p - 1)) > limit {
                p -= 1;
            }

            q += 1;
        }
    }

    /// Turns one contour point into its four quadrant points and writes them, in the order
    /// Q0, Q1, Q2, Q3 (REQ-E3D-446, 447, 448, 449, 450).
    fn quadrants(&mut self, line: i32, centre: i32, centre_z: i32, p: i32, q: i32, region2: bool) {
        let up = (centre + p).div_euclid(2);
        let um = centre - up;
        let zp = if region2 {
            (centre_z + q - 1).div_euclid(2)
        } else {
            (centre_z + q).div_euclid(2)
        };
        let zm = centre_z - zp;

        // Texel of each quadrant along the line (REQ-E3D-447). The full types mirror the image
        // on the back half; the half types take the quadrant's own position. The rule is
        // applied as written, region 2 included — which is what TV-Y1, TV-Y3, TV-Y6 and TV-Y7
        // show, and what the parenthetical of §5.3 contradicts for one case (Q-014).
        let (mut tex_plus, mut tex_minus) = (up, um);
        if self.anti_shadow && region2 {
            // antiShadow only moves the texel choice of the region 2 points, never the voxels.
            tex_plus = (centre + q).div_euclid(2);
            tex_minus = (centre - q + 1).div_euclid(2);
        }

        let half = self.half();
        let texel = if half {
            [tex_plus, tex_plus, tex_minus, tex_minus]
        } else {
            [tex_minus, tex_plus, tex_plus, tex_minus]
        };

        let plus_side = !half || self.centre_at_first();
        let minus_side = !half || !plus_side;
        let quadrants = Quadrants {
            active: [plus_side, plus_side, minus_side, minus_side],
            texel,
        };

        if !self.solid {
            // Contour only: a quadrant point whose texel value is zero is skipped
            // (REQ-E3D-448), whatever the transparent colour is.
            let points = [(up, zp), (up, zm), (um, zp), (um, zm)];
            for (n, (u, z)) in points.into_iter().enumerate() {
                self.write_contour(line, quadrants.active[n], u, z, texel[n]);
            }

            return;
        }

        if self.full_warp {
            // REQ-E3D-450: fill each column of the section over the depth span of the point,
            // with the texel of the opposite quadrant and no zero test. The span runs between
            // the two bounds whichever way round they are, as TV-Y4 requires (Q-015).
            for z in zm.min(zp)..=zm.max(zp) {
                if quadrants.active[0] && quadrants.active[1] {
                    self.write_section(line, up, z, texel[2]);
                }

                if quadrants.active[2] && quadrants.active[3] {
                    self.write_section(line, um, z, texel[0]);
                }
            }

            return;
        }

        // REQ-E3D-449: Bresenham from the centre to Q0, mirrored into the active quadrants.
        self.fill_to_centre(line, centre, centre_z, up, zp, quadrants);
    }

    fn write_contour(&mut self, line: i32, active: bool, u: i32, z: i32, texel_line: i32) {
        if !active || self.texel_value(line, texel_line) == 0 {
            return;
        }

        self.write_section(line, u, z, texel_line);
    }

    /// Writes one voxel of a section: pattern coordinate `(u, line, z)` for the V types,
    /// `(line, u, z)` for the H types, on all axes (REQ-E3D-448).
    fn write_section(&mut self, line: i32, u: i32, z: i32, texel_line: i32) {
        if self.vertical() {
            self.write_all(u, line, z, texel_line, line);
        } else {
            self.write_all(line, u, z, line, texel_line);
        }
    }

    /// The texel value at the position `texel_line` along the line.
    fn texel_value(&self, line: i32, texel_line: i32) -> i32 {
        if self.vertical() {
            self.texture.texel(texel_line, line)
        } else {
            self.texture.texel(line, texel_line)
        }
    }

    /// Traces the discrete segment from the section centre `M` to `Q0` by the classic
    /// Bresenham algorithm and mirrors each traced point into the active quadrants
    /// (REQ-E3D-449).
    #[allow(clippy::cast_precision_loss)] // C# int-to-float conversions of small values.
    fn fill_to_centre(
        &mut self,
        line: i32,
        centre: i32,
        centre_z: i32,
        target_u: i32,
        target_z: i32,
        quadrants: Quadrants,
    ) {
        let centre_u = centre.div_euclid(2);
        let mid_z = centre_z.div_euclid(2);

        let delta_u = (target_u - centre_u).abs();
        let delta_z = (target_z - mid_z).abs();
        let major_span = delta_u.max(delta_z);
        if major_span == 0 {
            return;
        }

        let major_is_u = delta_u >= delta_z;
        let minor_span = if major_is_u { delta_z } else { delta_u };

        // Walked from the end of smallest major coordinate to the other one.
        let (major_start, minor_start, minor_step) = if major_is_u {
            let from_centre = centre_u <= target_u;
            let minor_start = if from_centre { mid_z } else { target_z };
            let minor_end = if from_centre { target_z } else { mid_z };
            (
                centre_u.min(target_u),
                minor_start,
                (minor_end - minor_start).signum(),
            )
        } else {
            let from_centre = mid_z <= target_z;
            let minor_start = if from_centre { centre_u } else { target_u };
            let minor_end = if from_centre { target_u } else { centre_u };
            (
                mid_z.min(target_z),
                minor_start,
                (minor_end - minor_start).signum(),
            )
        };

        let major_target = if major_is_u { target_u } else { target_z };
        let mut error = major_span / 2;
        let mut minor = minor_start;

        for step in 0..=major_span {
            let major = major_start + step;
            let (u, z) = if major_is_u {
                (major, minor)
            } else {
                (minor, major)
            };

            // Only the points close enough to Q0 are drawn; with fillrate = 1 that excludes
            // the centre alone.
            if ((major - major_target).abs() as f32 / major_span as f32) < self.fill_rate {
                self.write_mirrored(line, centre, centre_z, u, z, quadrants);
            }

            error -= minor_span;
            if error < 0 {
                minor += minor_step;
                error += major_span;
            }
        }
    }

    fn write_mirrored(
        &mut self,
        line: i32,
        centre: i32,
        centre_z: i32,
        u: i32,
        z: i32,
        quadrants: Quadrants,
    ) {
        let points = [
            (u, z),
            (u, centre_z - z),
            (centre - u, z),
            (centre - u, centre_z - z),
        ];
        for (n, (pu, pz)) in points.into_iter().enumerate() {
            let texel = quadrants.texel[n];
            if quadrants.active[n] && self.texel_value(line, texel) != 0 {
                self.write_section(line, pu, pz, texel);
            }
        }
    }
}
