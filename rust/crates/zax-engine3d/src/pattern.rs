//! Patterns of the registry and their keys (SPEC-04 §2, REQ-E3D-060 to 065).

use std::borrow::Borrow;
use std::collections::HashSet;
use std::fmt;

use zax_profiles::pattern::ProfilePattern;
use zax_profiles::tile::PatternGrid;

use crate::error::EngineError;
use crate::image::{ImageRegistry, TileImage, TileImageKey};
use crate::instance::{PatternInstance, ScriptHandleSlot};
use crate::vectors::{Cell, Vec2};

/// Direction of a neighbour relative to a tile, in the examination order of REQ-E3D-090
/// (right, down, up, left). `3 − d` is the opposite direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GrowDirection {
    /// The neighbour is to the right.
    Right = 0,
    /// The neighbour is below (screen y grows downwards).
    Down = 1,
    /// The neighbour is above.
    Up = 2,
    /// The neighbour is to the left.
    Left = 3,
}

impl GrowDirection {
    /// The four directions in the examination order of REQ-E3D-090.
    pub const ORDER: [Self; 4] = [Self::Right, Self::Down, Self::Up, Self::Left];
}

/// The key of a pattern in the registry: its set of distinct images (REQ-E3D-063). Layout,
/// grid size, number of occurrences and parameters are ignored.
///
/// It holds the image identities sorted ascending and borrows as that slice, so the registry
/// can be searched with a slice built in a reused buffer.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct ImageSetKey(Box<[u32]>);

impl ImageSetKey {
    /// Builds the key of a set of images, in any order.
    pub fn new<'a>(images: impl IntoIterator<Item = &'a TileImage>) -> Self {
        let mut ids = Vec::new();
        Self::collect_ids(images, &mut ids);
        Self(ids.into_boxed_slice())
    }

    /// Writes the sorted distinct identities of `images` into `ids`, the slice form of a key.
    pub fn collect_ids<'a>(images: impl IntoIterator<Item = &'a TileImage>, ids: &mut Vec<u32>) {
        ids.clear();
        ids.extend(images.into_iter().map(TileImage::id));
        ids.sort_unstable();
        ids.dedup();
    }

    /// Number of distinct images, `|Img(P)|`.
    #[must_use]
    pub fn count(&self) -> usize {
        self.0.len()
    }

    /// The image identities, ascending.
    #[must_use]
    pub fn ids(&self) -> &[u32] {
        &self.0
    }

    /// True when this set is a subset of `other` (REQ-E3D-065).
    #[must_use]
    pub fn is_subset_of(&self, other: &Self) -> bool {
        is_subset(&self.0, &other.0)
    }
}

impl Borrow<[u32]> for ImageSetKey {
    fn borrow(&self) -> &[u32] {
        &self.0
    }
}

/// True when the sorted set `mine` is a subset of the sorted set `theirs`.
#[must_use]
pub fn is_subset(mine: &[u32], theirs: &[u32]) -> bool {
    let mut j = 0;
    for &id in mine {
        while j < theirs.len() && theirs[j] < id {
            j += 1;
        }

        if j >= theirs.len() || theirs[j] != id {
            return false;
        }

        j += 1;
    }

    true
}

/// A pattern of the registry: a grid of images, its recognition parameters and its ordered
/// instances (SPEC-04 §2, REQ-E3D-060 to 065).
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)] // The recognition flags of REQ-E3D-061 to 093.
pub struct EnginePattern {
    width: i32,
    height: i32,
    permanent: bool,
    cells: Vec<Option<TileImage>>,
    images: Vec<TileImage>,
    horizontal: HashSet<u64>,
    vertical: HashSet<u64>,
    instances: Vec<PatternInstance>,
    key: ImageSetKey,
    self_connected: bool,
    pixel_start: Cell,
    pixel_end: Cell,
    registration_index: i32,
    /// Skip the geometric adjacency test (`skipGeo`, REQ-E3D-062).
    pub skip_geometry: bool,
    /// Size constraint; `(99, 99)` means unconstrained (REQ-E3D-062).
    pub constraint_size: Cell,
    /// Script flag that takes the pattern out of the candidates (REQ-E3D-093). Never saved.
    pub disabled: bool,
    /// The script layer's handle cache (SPEC-05 REQ-LUA-043).
    pub script_handle: ScriptHandleSlot,
    /// Stored pixel count of the profile, kept as read (REQ-E3D-341).
    pub stored_pixel_count: i32,
    /// Stored colour flags of the profile, kept as read (REQ-E3D-341).
    pub stored_color_flags: i32,
    /// Stored tile density of the profile, kept as read (REQ-E3D-341).
    pub stored_tile_density: i32,
    /// Stored pixel density of the profile, kept as read (REQ-E3D-341).
    pub stored_pixel_density: i32,
}

impl EnginePattern {
    /// Builds a pattern over a grid of images given row by row; a `None` entry is an empty
    /// cell. Missing cells are empty and extra ones are ignored.
    pub fn new(
        width: u16,
        height: u16,
        cells: impl IntoIterator<Item = Option<TileImage>>,
        permanent: bool,
    ) -> Self {
        let count = usize::from(width) * usize::from(height);
        let mut grid: Vec<Option<TileImage>> = cells.into_iter().take(count).collect();
        grid.resize(count, None);

        let mut images: Vec<TileImage> = Vec::new();
        for cell in grid.iter().flatten() {
            if !images.contains(cell) {
                images.push(*cell);
            }
        }

        let mut pattern = Self {
            width: i32::from(width),
            height: i32::from(height),
            permanent,
            key: ImageSetKey::new(&images),
            cells: grid,
            images,
            horizontal: HashSet::new(),
            vertical: HashSet::new(),
            instances: Vec::new(),
            self_connected: true,
            pixel_start: Cell::default(),
            pixel_end: Cell::default(),
            registration_index: 0,
            skip_geometry: false,
            constraint_size: Cell::new(99, 99),
            disabled: false,
            script_handle: ScriptHandleSlot::default(),
            stored_pixel_count: 0,
            stored_color_flags: 0,
            stored_tile_density: 0,
            stored_pixel_density: 0,
        };
        pattern.build_relations();
        pattern.build_pixel_box();
        pattern
    }

    /// Grid width in cells (`W`).
    #[must_use]
    pub const fn width(&self) -> i32 {
        self.width
    }

    /// Grid height in cells (`H`).
    #[must_use]
    pub const fn height(&self) -> i32 {
        self.height
    }

    /// Whether the pattern is permanent (as opposed to temporary).
    #[must_use]
    pub const fn permanent(&self) -> bool {
        self.permanent
    }

    /// The registry key: the set of distinct images (REQ-E3D-063).
    #[must_use]
    pub const fn key(&self) -> &ImageSetKey {
        &self.key
    }

    /// The distinct images of the grid, in first-occurrence order; `Img(P)`. Its length is
    /// `|Img(P)|`, the tie-break of REQ-E3D-090.
    #[must_use]
    pub fn images(&self) -> &[TileImage] {
        &self.images
    }

    /// The ordered instances; a pattern of a file always has at least one.
    #[must_use]
    pub fn instances(&self) -> &[PatternInstance] {
        &self.instances
    }

    /// The ordered instances, for writing (the `BuildPattern` hook, REQ-E3D-322, and the
    /// scripts change them in place).
    pub fn instances_mut(&mut self) -> &mut [PatternInstance] {
        &mut self.instances
    }

    /// Internal connectivity option (`selfConnected`, REQ-E3D-061).
    #[must_use]
    pub const fn self_connected(&self) -> bool {
        self.self_connected
    }

    /// Order of registration in the registry, which is the candidate order of REQ-E3D-091.
    #[must_use]
    pub const fn registration_index(&self) -> i32 {
        self.registration_index
    }

    pub(crate) fn set_registration_index(&mut self, index: i32) {
        self.registration_index = index;
    }

    /// Top-left corner of the pattern pixel box, in grid pixels (REQ-E3D-064).
    #[must_use]
    pub const fn pixel_start(&self) -> Cell {
        self.pixel_start
    }

    /// Bottom-right corner of the pattern pixel box, inclusive (REQ-E3D-064).
    #[must_use]
    pub const fn pixel_end(&self) -> Cell {
        self.pixel_end
    }

    /// `BottomLeft(P) = (pStart.x, 8H − pEnd.y − 1)` (REQ-E3D-064), used by
    /// `Frame:InsertShape` (REQ-E3D-232).
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // C# int-to-float conversions of small values.
    pub fn bottom_left(&self) -> Vec2 {
        Vec2::new(
            self.pixel_start.x as f32,
            ((8 * self.height) - self.pixel_end.y - 1) as f32,
        )
    }

    /// `TopRight(P) = (pEnd.x, 8W − pStart.y − 1)` (REQ-E3D-064). The y component uses the
    /// grid *width*: a quirk of the original, kept as is (DEC-04-Q04, Q-04).
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // C# int-to-float conversions of small values.
    pub fn top_right(&self) -> Vec2 {
        Vec2::new(
            self.pixel_end.x as f32,
            ((8 * self.width) - self.pixel_start.y - 1) as f32,
        )
    }

    /// The image of cell (`i`, `j`), or `None`.
    #[must_use]
    pub fn image_at(&self, i: i32, j: i32) -> Option<&TileImage> {
        if i < 0 || i >= self.width || j < 0 || j >= self.height {
            return None;
        }

        #[allow(clippy::cast_sign_loss)] // Both coordinates checked non-negative above.
        let index = (i + (j * self.width)) as usize;
        self.cells[index].as_ref()
    }

    /// Adds an instance at the end of the list.
    pub fn add_instance(&mut self, mut instance: PatternInstance) {
        instance.set_index(self.instances.len());
        self.instances.push(instance);
    }

    /// True when the pattern may be a candidate at all (REQ-E3D-093).
    #[must_use]
    pub const fn is_candidate(&self) -> bool {
        self.permanent && !self.disabled
    }

    /// Whether the pattern *allows* adding a neighbour of image `target` in direction
    /// `direction` from a tile of image `source` (REQ-E3D-062, conditions 1 and 2). The size
    /// constraint of condition 3 is [`EnginePattern::fits_constraint`].
    #[must_use]
    pub fn allows_direction(
        &self,
        source: &TileImage,
        target: &TileImage,
        direction: GrowDirection,
    ) -> bool {
        if self.skip_geometry {
            return self.images.contains(target);
        }

        match direction {
            GrowDirection::Right => self.horizontal.contains(&pair(source.id(), target.id())),
            GrowDirection::Left => self.horizontal.contains(&pair(target.id(), source.id())),
            GrowDirection::Down => self.vertical.contains(&pair(source.id(), target.id())),
            GrowDirection::Up => self.vertical.contains(&pair(target.id(), source.id())),
        }
    }

    /// The size constraint of REQ-E3D-062 condition 3.
    #[must_use]
    pub const fn fits_constraint(&self, width: i32, height: i32) -> bool {
        width <= self.constraint_size.x && height <= self.constraint_size.y
    }

    /// True when `(source, target)` is in the horizontal relation R(P).
    #[must_use]
    pub fn has_horizontal_pair(&self, source: &TileImage, target: &TileImage) -> bool {
        self.horizontal.contains(&pair(source.id(), target.id()))
    }

    /// True when `(source, target)` is in the vertical relation D(P).
    #[must_use]
    pub fn has_vertical_pair(&self, source: &TileImage, target: &TileImage) -> bool {
        self.vertical.contains(&pair(source.id(), target.id()))
    }

    /// Number of pairs in the horizontal relation R(P).
    #[must_use]
    pub fn horizontal_pair_count(&self) -> usize {
        self.horizontal.len()
    }

    /// Number of pairs in the vertical relation D(P).
    #[must_use]
    pub fn vertical_pair_count(&self) -> usize {
        self.vertical.len()
    }

    /// Reads the recognition parameters of a profile pattern (SPEC-03 §4.3) and builds the
    /// registry pattern, interning its images on the way (REQ-E3D-060).
    ///
    /// # Errors
    ///
    /// [`EngineError::PatternSize`] when the grid size is negative or too large.
    pub fn from_profile(
        pattern: &ProfilePattern,
        images: &mut ImageRegistry,
        permanent: bool,
    ) -> Result<Self, EngineError> {
        let grid = pattern.grid.as_ref();
        let width = grid.map_or_else(
            || pattern.tile_size.map_or(0, |size| size.x),
            PatternGrid::width,
        );
        let height = grid.map_or_else(
            || pattern.tile_size.map_or(0, |size| size.y),
            PatternGrid::height,
        );
        let size_error = || EngineError::PatternSize { width, height };
        let grid_width = u16::try_from(width).map_err(|_| size_error())?;
        let grid_height = u16::try_from(height).map_err(|_| size_error())?;

        let mut cells = Vec::with_capacity(usize::from(grid_width) * usize::from(grid_height));
        for j in 0..height {
            for i in 0..width {
                let tile = grid.and_then(|grid| grid.get(i, j));
                cells.push(tile.map(|tile| {
                    images.intern(TileImageKey::new(
                        tile.low_plane,
                        tile.high_plane,
                        tile.flip_x(),
                        tile.flip_y(),
                    ))
                }));
            }
        }

        let mut result = Self::new(grid_width, grid_height, cells, permanent);
        result.skip_geometry = pattern.skip_geometry;
        result.constraint_size = Cell::new(pattern.constraint_size.x, pattern.constraint_size.y);
        result.stored_pixel_count = pattern.pixel_count;
        result.stored_color_flags = pattern.color_flags.bits();
        result.stored_tile_density = pattern.tile_density.raw();
        result.stored_pixel_density = pattern.pixel_density.raw();
        result.set_self_connected(pattern.self_connected);

        for variant in &pattern.variants {
            result.add_instance(PatternInstance::from_variant(variant));
        }

        Ok(result)
    }

    /// Sets `selfConnected` and rebuilds the adjacency relations, which depend on it
    /// (REQ-E3D-061).
    pub fn set_self_connected(&mut self, value: bool) {
        if self.self_connected == value {
            return;
        }

        self.self_connected = value;
        self.horizontal.clear();
        self.vertical.clear();
        self.build_relations();
    }

    fn build_relations(&mut self) {
        for j in 0..self.height {
            for i in 0..self.width {
                let Some(&here) = self.image_at(i, j) else {
                    continue;
                };

                let right = self.image_at(i + 1, j).copied();
                let below = self.image_at(i, j + 1).copied();
                add_pair(&mut self.horizontal, &here, right);
                add_pair(&mut self.vertical, &here, below);
                if self.self_connected {
                    let wrap_right = self.image_at((i + 1) % self.width, j).copied();
                    let wrap_below = self.image_at(i, (j + 1) % self.height).copied();
                    add_pair(&mut self.horizontal, &here, wrap_right);
                    add_pair(&mut self.vertical, &here, wrap_below);
                }
            }
        }
    }

    fn build_pixel_box(&mut self) {
        let mut start: Option<Cell> = None;
        let mut end = Cell::new(i32::MIN, i32::MIN);
        for j in 0..self.height {
            for i in 0..self.width {
                let Some(image) = self.image_at(i, j).filter(|image| !image.is_empty()) else {
                    continue;
                };

                let s = start.get_or_insert(Cell::new(i32::MAX, i32::MAX));
                s.x = s.x.min((8 * i) + image.pixel_start().x);
                s.y = s.y.min((8 * j) + image.pixel_start().y);
                end.x = end.x.max((8 * i) + image.pixel_end().x);
                end.y = end.y.max((8 * j) + image.pixel_end().y);
            }
        }

        if let Some(start) = start {
            self.pixel_start = start;
            self.pixel_end = end;
        } else {
            self.pixel_start = Cell::new(0, 0);
            self.pixel_end = Cell::new(0, 0);
        }
    }
}

impl fmt::Display for EnginePattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} #{} {}x{}, |Img|={}",
            if self.permanent { "pattern" } else { "temp" },
            self.registration_index,
            self.width,
            self.height,
            self.key.count()
        )
    }
}

const fn pair(a: u32, b: u32) -> u64 {
    ((a as u64) << 32) | b as u64
}

fn add_pair(relation: &mut HashSet<u64>, source: &TileImage, target: Option<TileImage>) {
    if let Some(target) = target {
        relation.insert(pair(source.id(), target.id()));
    }
}
