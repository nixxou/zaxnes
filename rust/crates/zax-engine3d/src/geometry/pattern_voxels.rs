// Portions adapted from 3DNes_Engine (c) Geod Studio, GPL-3.0

//! The voxel fields of one 3D object with the pattern-level neighbourhood MARCHING needs
//! (SPEC-04B REQ-E3D-425, REQ-E3D-520).

use crate::geometry::texture::ObjectTexture;
use crate::geometry::tile_voxels::{TileVoxels, SIZE_X, SIZE_Y};
use crate::geometry::voxel_construction::image_index;
use crate::image::TileImage;
use crate::pattern::EnginePattern;

/// The voxel fields of one 3D object: one per distinct tile image of its pattern, plus the
/// pattern-level neighbourhood MARCHING needs (SPEC-04B REQ-E3D-425, REQ-E3D-520).
///
/// MARCHING reads corners outside the tile from the neighbouring tile of the pattern. When a
/// tile image occupies several cells there is no single neighbourhood, so the occurrence
/// closest to the side being looked at is used (REQ-E3D-520). The nine positions kept per
/// image are exactly that: for each of the nine (left, middle, right) x (top, middle, bottom)
/// directions, the extreme occurrence in that direction.
#[derive(Debug, Clone)]
pub struct PatternVoxels<'a> {
    pattern: &'a EnginePattern,
    texture: &'a ObjectTexture,
    fields: &'a [TileVoxels],
    field_of_image: Vec<Option<usize>>,
    corners: Vec<Option<(i32, i32)>>,
    pattern_width: i32,
    pattern_height: i32,
}

/// Which way an occurrence is preferred along one axis: the smaller coordinate, the larger
/// one, or any.
#[derive(Debug, Clone, Copy)]
enum Prefer {
    Smaller,
    Larger,
    Any,
}

impl Prefer {
    const fn accepts(self, kept: i32, candidate: i32) -> bool {
        match self {
            Self::Smaller => kept >= candidate,
            Self::Larger => kept <= candidate,
            Self::Any => true,
        }
    }
}

impl<'a> PatternVoxels<'a> {
    /// Groups `fields` with their neighbourhood tables.
    #[must_use]
    pub fn new(
        pattern: &'a EnginePattern,
        texture: &'a ObjectTexture,
        fields: &'a [TileVoxels],
    ) -> Self {
        let field_of_image = image_index(pattern);
        let mut corners = vec![None; pattern.images().len() * 9];
        for j in 0..pattern.height() {
            for i in 0..pattern.width() {
                if let Some(at) = pattern
                    .image_at(i, j)
                    .and_then(|image| field_of_image[image.id() as usize])
                {
                    set_occurrence(&mut corners[at * 9..at * 9 + 9], i * 8, j * 8);
                }
            }
        }

        Self {
            pattern,
            texture,
            fields,
            field_of_image,
            corners,
            pattern_width: pattern.width() * 8,
            pattern_height: pattern.height() * 8,
        }
    }

    /// The pattern the fields belong to.
    #[must_use]
    pub const fn pattern(&self) -> &EnginePattern {
        self.pattern
    }

    /// The object's texture.
    #[must_use]
    pub const fn texture(&self) -> &ObjectTexture {
        self.texture
    }

    /// Number of distinct tile images, hence of fields.
    #[must_use]
    pub const fn count(&self) -> usize {
        self.fields.len()
    }

    /// The field of the image of index `image_index`.
    #[must_use]
    pub fn field(&self, image_index: usize) -> &TileVoxels {
        &self.fields[image_index]
    }

    /// The image of index `image_index`.
    #[must_use]
    pub fn image(&self, image_index: usize) -> &TileImage {
        &self.pattern.images()[image_index]
    }

    /// True when the cells at `i = −1` must be meshed for this image, that is when its
    /// leftmost occurrence sits in the leftmost column of the pattern (REQ-E3D-520).
    /// `cell_row` is the row of voxels the cell starts at, which selects the occurrence used.
    #[must_use]
    pub fn meshes_left_edge(&self, image_index: usize, cell_row: i32) -> bool {
        matches!(
            self.corners[image_index * 9 + direction_index(-1, cell_row)],
            Some((0, _))
        )
    }

    /// Reads the single slot of a corner of the MARCHING grid, in the tile frame of the image
    /// `image_index`. Coordinates outside `[0,8)` on x or y are resolved in the neighbouring
    /// tile of the pattern; outside the pattern image, outside `[0, D)` in depth or on an
    /// empty cell, the corner is empty (REQ-E3D-520).
    #[must_use]
    pub fn corner(&self, image_index: usize, i: i32, j: i32, k: i32) -> Option<(i32, i32)> {
        let (origin_x, origin_y) = self.corners[image_index * 9 + direction_index(i, j)]?;
        let px = i + origin_x;
        let py = (7 - j) + origin_y;
        if px < 0 || px >= self.pattern_width || py < 0 || py >= self.pattern_height {
            return None;
        }

        let image = self.pattern.image_at(px >> 3, py >> 3)?;
        let index = (*self.field_of_image.get(image.id() as usize)?)?;
        self.fields[index].texel(px & 7, 7 - (py & 7), k, 0)
    }
}

/// The direction slot of a tile coordinate: 0 to 2 for x (left, inside, right) plus 0, 3 or 6
/// for y (above, inside, below), the layout of the nine occurrences.
const fn direction_index(i: i32, j: i32) -> usize {
    let mut index = if i == -1 {
        0
    } else if i == SIZE_X {
        2
    } else {
        1
    };
    if j == -1 {
        index += 6;
    } else if j != SIZE_Y {
        index += 3;
    }

    index
}

fn set_occurrence(slots: &mut [Option<(i32, i32)>], x: i32, y: i32) {
    // Slot 4 is "any occurrence"; the eight others keep the extreme one in their direction,
    // x towards the left or the right and y towards the top or the bottom of the pattern
    // image.
    const PREFERENCES: [(Prefer, Prefer); 9] = [
        (Prefer::Smaller, Prefer::Smaller),
        (Prefer::Any, Prefer::Smaller),
        (Prefer::Larger, Prefer::Smaller),
        (Prefer::Smaller, Prefer::Any),
        (Prefer::Any, Prefer::Any),
        (Prefer::Larger, Prefer::Any),
        (Prefer::Smaller, Prefer::Larger),
        (Prefer::Any, Prefer::Larger),
        (Prefer::Larger, Prefer::Larger),
    ];

    for (slot, (prefer_x, prefer_y)) in slots.iter_mut().zip(PREFERENCES) {
        let keep = match *slot {
            None => true,
            Some((kept_x, kept_y)) => prefer_x.accepts(kept_x, x) && prefer_y.accepts(kept_y, y),
        };
        if keep {
            *slot = Some((x, y));
        }
    }
}
