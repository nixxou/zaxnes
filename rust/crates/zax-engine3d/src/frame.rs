// Portions adapted from 3DNes_Engine (c) Geod Studio, GPL-3.0

//! The frame model built from one batch of the core: tiles, shapes, frame instances
//! (REQ-E3D-040 to 048, REQ-E3D-190).

use std::fmt;
use std::ops::{Deref, DerefMut};

use crate::grid;
use crate::image::TileImage;
use crate::instance::{InstanceRef, PatternInstance, ScriptHandleSlot};
use crate::palette::FramePalette;
use crate::pattern::{EnginePattern, ImageSetKey};
use crate::registry::PatternId;
use crate::settings::{ConversionSettings, ZLayer};
use crate::vectors::{Cell, Rect, Vec2, Vec3};

/// Where a tile of a frame lives: its background cell, its sprite slot, or the pool of the
/// tiles no batch produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TileId {
    /// The background tile of a grid slot (REQ-E3D-005).
    Background(usize),
    /// The sprite of emission slot `n` (REQ-E3D-006).
    Sprite(usize),
    /// A tile no batch produced: one of a shape a script injects (REQ-E3D-230 to 232) or of a
    /// shape restored from the pinned frame (REQ-E3D-233).
    Extra(usize),
}

/// The handle of a shape inside its frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ShapeId(usize);

/// The handle of a frame instance inside its frame: its shape and its index there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FrameInstanceId {
    /// The shape.
    pub shape: ShapeId,
    /// The index of the instance in the shape's list, which is the index of its pattern
    /// instance (REQ-E3D-190).
    pub index: usize,
}

/// One tile kept in a frame: its image, where it is on screen, its palette attribute and the
/// shape it ended up in (REQ-E3D-005, REQ-E3D-042).
///
/// Tiles are pooled: a frame holds one per background cell and one per sprite slot, reused
/// from frame to frame, so a full frame costs no allocation (REQ-E3D-360).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameTile {
    image: TileImage,
    x: i32,
    y: i32,
    palette: i32,
    hidden: bool,
    ord: i32,
    position: Cell,
    grid_index: i32,
    background: bool,
    present: bool,
    shape: Option<ShapeId>,
}

impl FrameTile {
    fn new(position: Cell, grid_index: i32, background: bool) -> Self {
        Self {
            image: TileImage::placeholder(),
            x: 0,
            y: 0,
            palette: 0,
            hidden: false,
            ord: -1,
            position,
            grid_index,
            background,
            present: false,
            shape: None,
        }
    }

    /// The image of the tile.
    #[must_use]
    pub const fn image(&self) -> &TileImage {
        &self.image
    }

    /// Screen x of the top-left pixel.
    #[must_use]
    pub const fn x(&self) -> i32 {
        self.x
    }

    /// Screen y of the top-left pixel.
    #[must_use]
    pub const fn y(&self) -> i32 {
        self.y
    }

    /// Palette attribute, 0 to 3.
    #[must_use]
    pub const fn palette(&self) -> i32 {
        self.palette
    }

    /// Sprite priority bit "behind the background"; always false for background.
    #[must_use]
    pub const fn hidden(&self) -> bool {
        self.hidden
    }

    /// Sprite slot, 0 to 127 (REQ-CORE-043); −1 for a background tile.
    #[must_use]
    pub const fn ord(&self) -> i32 {
        self.ord
    }

    /// The cell of the tile in the grid of its layer.
    #[must_use]
    pub const fn position(&self) -> Cell {
        self.position
    }

    /// Index of `position` in a grid array.
    #[must_use]
    pub const fn grid_index(&self) -> i32 {
        self.grid_index
    }

    /// True for a background tile, false for a sprite.
    #[must_use]
    pub const fn background(&self) -> bool {
        self.background
    }

    /// True when the tile was kept in the batch of the current frame.
    #[must_use]
    pub const fn present(&self) -> bool {
        self.present
    }

    /// The shape the tile belongs to, or `None` while it is still free.
    #[must_use]
    pub const fn shape(&self) -> Option<ShapeId> {
        self.shape
    }

    /// True when the tile is not in a shape yet (REQ-E3D-090 point 3).
    #[must_use]
    pub const fn is_free(&self) -> bool {
        self.shape.is_none()
    }

    /// Colour (0 to 3) of the displayed pixel at (x, y) of the tile.
    #[must_use]
    pub const fn color_at(&self, px: i32, py: i32) -> i32 {
        self.image.color_at(px, py)
    }

    fn fill(&mut self, image: TileImage, x: i32, y: i32, palette: i32, hidden: bool, ord: i32) {
        self.image = image;
        self.x = x;
        self.y = y;
        self.palette = palette;
        self.hidden = hidden;
        self.ord = ord;
        self.present = true;
        self.shape = None;
    }

    fn reset(&mut self) {
        self.present = false;
        self.shape = None;
    }
}

impl fmt::Display for FrameTile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} tile ({}, {}) cell {} pal {}",
            if self.background { "bg" } else { "sp" },
            self.x,
            self.y,
            self.position,
            self.palette
        )
    }
}

/// The part of a shape a frame instance reads: its pixel box, its layer and its palette. They
/// are fixed once the shape is bound (REQ-E3D-044, REQ-E3D-047).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct ShapeBox {
    pixel_start: Cell,
    pixel_end: Cell,
    background: bool,
    palette: i32,
}

/// A shape: a group of adjacent tiles of one frame and one layer, bound to exactly one
/// reference pattern (SPEC-04 §2, REQ-E3D-044 to 048).
///
/// A shape also plays the part of the *frame pattern* of SPEC-05 (type `Pattern`): its grid is
/// its tile box, its instances are the frame instances, and the recognition parameters a
/// script reads through it (`constraint_size`, `self_connected`, `skip_geometry`) are
/// per-frame copies of the reference pattern's, so writing them affects this frame only.
///
/// Shapes are pooled by the frame, like tiles; the tile list holds handles into the frame.
#[derive(Debug, Clone, Default)]
#[allow(clippy::struct_excessive_bools)] // The flags of REQ-E3D-045 to 048 and REQ-E3D-220.
pub struct FrameShape {
    tiles: Vec<TileId>,
    images: Vec<TileImage>,
    instances: Vec<FrameInstance>,
    background: bool,
    tile_start: Cell,
    tile_end: Cell,
    pixel_start: Cell,
    pixel_end: Cell,
    pixel_count: i32,
    palette: i32,
    hidden: bool,
    center: Vec3,
    reference_pattern: Option<PatternId>,
    reference_image_count: usize,
    scriptable: bool,
    restored_from_pinned_frame: bool,
    restored_from: Option<ShapeId>,
    mask: Cell,
    /// The script layer's handle cache (SPEC-05 REQ-LUA-043).
    pub script_handle: ScriptHandleSlot,
    /// Size constraint of the frame pattern, copied from the reference pattern.
    pub constraint_size: Cell,
    /// Connectivity option of the frame pattern, copied from the reference pattern.
    pub self_connected: bool,
    /// Geometry-skip option of the frame pattern, copied from the reference pattern.
    pub skip_geometry: bool,
}

impl FrameShape {
    /// The age of a shape, which is always 1 (REQ-E3D-207).
    pub const AGE: i32 = 1;

    /// The tiles, in the order they were added; the first one is the germ.
    #[must_use]
    pub fn tiles(&self) -> &[TileId] {
        &self.tiles
    }

    /// The distinct images of the shape, `Img(S)`, in the order first met.
    #[must_use]
    pub fn images(&self) -> &[TileImage] {
        &self.images
    }

    /// The frame instances, one per instance of the reference pattern (REQ-E3D-190).
    #[must_use]
    pub fn instances(&self) -> &[FrameInstance] {
        &self.instances
    }

    /// The frame instances, for writing.
    pub fn instances_mut(&mut self) -> &mut [FrameInstance] {
        &mut self.instances
    }

    /// True for a background shape, false for a sprite shape.
    #[must_use]
    pub const fn background(&self) -> bool {
        self.background
    }

    /// Top-left corner of the tile box, in cells of the shape's grid.
    #[must_use]
    pub const fn tile_start(&self) -> Cell {
        self.tile_start
    }

    /// Bottom-right corner of the tile box, inclusive.
    #[must_use]
    pub const fn tile_end(&self) -> Cell {
        self.tile_end
    }

    /// Size of the tile box, `tEnd − tStart + (1, 1)`.
    #[must_use]
    pub const fn tile_size(&self) -> Cell {
        Cell::new(
            self.tile_end.x - self.tile_start.x + 1,
            self.tile_end.y - self.tile_start.y + 1,
        )
    }

    /// Top-left corner of the pixel box, in screen pixels (REQ-E3D-044).
    #[must_use]
    pub const fn pixel_start(&self) -> Cell {
        self.pixel_start
    }

    /// Bottom-right corner of the pixel box, inclusive (REQ-E3D-044).
    #[must_use]
    pub const fn pixel_end(&self) -> Cell {
        self.pixel_end
    }

    /// Size of the pixel box, `pEnd − pStart + (1, 1)` (SPEC-04 §4.4).
    #[must_use]
    pub const fn pixel_size(&self) -> Cell {
        Cell::new(
            self.pixel_end.x - self.pixel_start.x + 1,
            self.pixel_end.y - self.pixel_start.y + 1,
        )
    }

    /// Number of tiles.
    #[must_use]
    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }

    /// Total number of non-transparent pixels of the shape's tiles.
    #[must_use]
    pub const fn pixel_count(&self) -> i32 {
        self.pixel_count
    }

    /// Palette of the shape: the germ's attribute, plus 4 for a sprite shape (REQ-E3D-047).
    #[must_use]
    pub const fn palette(&self) -> i32 {
        self.palette
    }

    /// Or of the `hidden` flags of the sprites; always false for a background shape
    /// (REQ-E3D-048).
    #[must_use]
    pub const fn hidden(&self) -> bool {
        self.hidden
    }

    /// Screen centre `(pStart + pEnd + (1, 1)) / 2` (REQ-E3D-044), read by the tracker.
    #[must_use]
    pub const fn center(&self) -> Vec3 {
        self.center
    }

    /// The reference pattern the shape is bound to, or `None` before binding.
    #[must_use]
    pub const fn reference_pattern(&self) -> Option<PatternId> {
        self.reference_pattern
    }

    /// Whether the shape's instances are scriptable (REQ-E3D-220): true for a shape produced
    /// by the segmentation, false for one injected by a script or restored from the pinned
    /// frame.
    #[must_use]
    pub const fn scriptable(&self) -> bool {
        self.scriptable
    }

    /// True when the shape was restored from the pinned frame: its tiles are drawn only where
    /// they overlap one of the frame's rectangles (REQ-E3D-233).
    #[must_use]
    pub const fn restored_from_pinned_frame(&self) -> bool {
        self.restored_from_pinned_frame
    }

    /// True when the shape is fully away from the unreliable border bands (REQ-E3D-045):
    /// `pStart.x > mX`, `pEnd.x + mX < 255`, `pStart.y > mY` and `pEnd.y + mY < 239`.
    #[must_use]
    pub const fn inside(&self) -> bool {
        self.pixel_start.x > self.mask.x
            && self.pixel_end.x + self.mask.x < 255
            && self.pixel_start.y > self.mask.y
            && self.pixel_end.y + self.mask.y < 239
    }

    /// The negation of [`FrameShape::inside`] (REQ-E3D-045).
    #[must_use]
    pub const fn at_border(&self) -> bool {
        !self.inside()
    }

    /// True when the tile box is at least 28 cells wide or 26 cells high (REQ-E3D-045).
    #[must_use]
    pub const fn is_big(&self) -> bool {
        let size = self.tile_size();
        size.x >= grid::BIG_WIDTH || size.y >= grid::BIG_HEIGHT
    }

    /// True when the shape is inside or big (REQ-E3D-045).
    #[must_use]
    pub const fn is_finished(&self) -> bool {
        self.inside() || self.is_big()
    }

    /// True when the shape shows every image of its reference pattern, `|Img(S)| = |Img(P)|`
    /// (REQ-E3D-046).
    #[must_use]
    pub fn completed(&self) -> bool {
        self.reference_pattern.is_some() && self.images.len() == self.reference_image_count
    }

    /// `BottomLeft = (pStart.x, Hs − pEnd.y − 1)` (SPEC-04 §4.4).
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // C# int-to-float conversions of small values.
    pub fn bottom_left(&self) -> Vec2 {
        Vec2::new(
            self.pixel_start.x as f32,
            (grid::SCENE_HEIGHT - self.pixel_end.y - 1) as f32,
        )
    }

    /// `TopRight = (pEnd.x, Hs − pStart.y − 1)` (SPEC-04 §4.4).
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // C# int-to-float conversions of small values.
    pub fn top_right(&self) -> Vec2 {
        Vec2::new(
            self.pixel_end.x as f32,
            (grid::SCENE_HEIGHT - self.pixel_start.y - 1) as f32,
        )
    }

    /// The key of the shape in the pattern registry: its set of images (REQ-E3D-063).
    #[must_use]
    pub fn key(&self) -> ImageSetKey {
        ImageSetKey::new(&self.images)
    }

    /// The tile box the shape would have if a tile at `position` joined it.
    #[must_use]
    pub fn size_with(&self, position: Cell) -> Cell {
        if self.tiles.is_empty() {
            return Cell::new(1, 1);
        }

        let sx = self.tile_start.x.min(position.x);
        let sy = self.tile_start.y.min(position.y);
        let ex = self.tile_end.x.max(position.x);
        let ey = self.tile_end.y.max(position.y);
        Cell::new(ex - sx + 1, ey - sy + 1)
    }

    fn shape_box(&self) -> ShapeBox {
        ShapeBox {
            pixel_start: self.pixel_start,
            pixel_end: self.pixel_end,
            background: self.background,
            palette: self.palette,
        }
    }

    fn reset(&mut self, background: bool, mask: Cell) {
        self.background = background;
        self.mask = mask;
        self.tiles.clear();
        self.images.clear();
        self.instances.clear();
        self.pixel_count = 0;
        self.palette = 0;
        self.hidden = false;
        self.tile_start = Cell::default();
        self.tile_end = Cell::default();
        self.pixel_start = Cell::default();
        self.pixel_end = Cell::default();
        self.center = Vec3::default();
        self.reference_pattern = None;
        self.reference_image_count = 0;
        self.scriptable = false;
        self.restored_from_pinned_frame = false;
        self.restored_from = None;
        self.constraint_size = Cell::new(99, 99);
        self.self_connected = true;
        self.skip_geometry = false;
    }

    /// Records a tile that joined the shape: box, pixel count, palette, images.
    fn push_tile(&mut self, id: TileId, tile: &FrameTile) {
        self.tiles.push(id);
        self.pixel_count += tile.image.pixel_count();
        if !self.background {
            self.hidden |= tile.hidden;
        }

        let image = tile.image;
        if !self.images.contains(&image) {
            self.images.push(image);
        }

        let start = Cell::new(
            tile.x + image.pixel_start().x,
            tile.y + image.pixel_start().y,
        );
        let end = Cell::new(tile.x + image.pixel_end().x, tile.y + image.pixel_end().y);
        if self.tiles.len() == 1 {
            self.palette = tile.palette + if self.background { 0 } else { 4 };
            self.tile_start = tile.position;
            self.tile_end = tile.position;
            self.pixel_start = start;
            self.pixel_end = end;
            return;
        }

        self.tile_start = Cell::new(
            self.tile_start.x.min(tile.position.x),
            self.tile_start.y.min(tile.position.y),
        );
        self.tile_end = Cell::new(
            self.tile_end.x.max(tile.position.x),
            self.tile_end.y.max(tile.position.y),
        );
        self.pixel_start = Cell::new(
            self.pixel_start.x.min(start.x),
            self.pixel_start.y.min(start.y),
        );
        self.pixel_end = Cell::new(self.pixel_end.x.max(end.x), self.pixel_end.y.max(end.y));
    }
}

impl fmt::Display for FrameShape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} shape {} tiles, box {}-{}",
            if self.background { "bg" } else { "sp" },
            self.tiles.len(),
            self.tile_start,
            self.tile_end
        )
    }
}

/// A frame instance: the copy, valid for one frame, of one instance of a shape's reference
/// pattern (REQ-E3D-190). It is the object scripts receive as `shape` (SPEC-05).
///
/// It carries the same 3D parameters as a pattern instance — it dereferences to them — plus
/// the quantities derived from the shape it belongs to: the placement box, the anchor `pos`
/// and the values of SPEC-04 §4.4.
#[derive(Debug, Clone)]
pub struct FrameInstance {
    parameters: PatternInstance,
    reference: InstanceRef,
    shape: ShapeId,
    shape_box: ShapeBox,
    /// The anchor point, computed after the Lua hooks (REQ-E3D-205). Its value before the
    /// first computation of the frame is not specified (SPEC-05 REQ-LUA-083).
    pub pos: Vec3,
    /// The same point computed with the centred pivot (REQ-E3D-205).
    pub center: Vec3,
}

impl Deref for FrameInstance {
    type Target = PatternInstance;

    fn deref(&self) -> &PatternInstance {
        &self.parameters
    }
}

impl DerefMut for FrameInstance {
    fn deref_mut(&mut self) -> &mut PatternInstance {
        &mut self.parameters
    }
}

#[allow(clippy::cast_precision_loss)] // C# int-to-float conversions of small values.
impl FrameInstance {
    /// The shape this instance belongs to, exposed to scripts as `shape2D`.
    #[must_use]
    pub const fn shape(&self) -> ShapeId {
        self.shape
    }

    /// The pattern instance this one copies (SPEC-05 `refPattern3D`).
    #[must_use]
    pub const fn reference(&self) -> InstanceRef {
        self.reference
    }

    /// The pattern instance that supplies the texture: `texture_source` when a script
    /// redirected it, otherwise the reference (SPEC-05 REQ-LUA-097).
    #[must_use]
    pub fn texture_owner(&self) -> InstanceRef {
        self.texture_source.unwrap_or(self.reference)
    }

    /// The palette table used to colour the tiles: the custom palette when it is ≥ 0,
    /// otherwise the shape's palette (REQ-E3D-192).
    #[must_use]
    pub fn palette_table(&self) -> i32 {
        if self.custom_palette >= 0 {
            self.custom_palette
        } else {
            self.shape_box.palette
        }
    }

    /// True when the shape is a sprite shape.
    #[must_use]
    pub const fn sprite(&self) -> bool {
        !self.shape_box.background
    }

    /// True when the shape is a background shape.
    #[must_use]
    pub const fn background(&self) -> bool {
        self.shape_box.background
    }

    /// The effective layer (REQ-E3D-205): the instance's layer, or, when it is Unidentified,
    /// L1 for a sprite shape and the default layer of the settings for a background shape.
    #[must_use]
    pub fn effective_layer(&self, settings: &ConversionSettings) -> ZLayer {
        if self.layer != ZLayer::Unidentified {
            return self.layer;
        }

        if self.shape_box.background {
            settings.default_layer
        } else {
            ZLayer::L1
        }
    }

    /// The depth interval `[z0, z1]` of the instance (REQ-E3D-205): `z0 = O + (s/2)(a − 1)`,
    /// `z1 = O + (s/2)(a + 1)`.
    #[must_use]
    pub fn depth_interval(&self, settings: &ConversionSettings) -> (f32, f32) {
        let layer = settings.layer(self.effective_layer(settings));
        let half = self.depth as f32 / 2.0;
        let coefficient = layer.coefficient();
        (
            layer.offset + (half * (coefficient - 1.0)),
            layer.offset + (half * (coefficient + 1.0)),
        )
    }

    /// The placement box `Bmin` of REQ-E3D-205 (SPEC-04B REQ-E3D-404): the corner
    /// `(pStart.x, Hs − pEnd.y − 1, z0)`.
    #[must_use]
    pub fn box_min(&self, settings: &ConversionSettings) -> Vec3 {
        Vec3::new(
            self.shape_box.pixel_start.x as f32,
            (grid::SCENE_HEIGHT - self.shape_box.pixel_end.y - 1) as f32,
            self.depth_interval(settings).0,
        )
    }

    /// The placement box `Bmax` of REQ-E3D-205: `(pEnd.x + 1, Hs − pStart.y, z1)`.
    #[must_use]
    pub fn box_max(&self, settings: &ConversionSettings) -> Vec3 {
        Vec3::new(
            (self.shape_box.pixel_end.x + 1) as f32,
            (grid::SCENE_HEIGHT - self.shape_box.pixel_start.y) as f32,
            self.depth_interval(settings).1,
        )
    }

    /// The anchor point for an arbitrary pivot: `Bmin ⊙ (1 − pivot) + Bmax ⊙ pivot + offset`
    /// (REQ-E3D-205, SPEC-05 REQ-LUA-083).
    #[must_use]
    pub fn get_pos_with(&self, settings: &ConversionSettings, pivot: Vec3) -> Vec3 {
        let (front, back) = self.depth_interval(settings);
        let min = Vec3::new(
            self.shape_box.pixel_start.x as f32,
            (grid::SCENE_HEIGHT - self.shape_box.pixel_end.y - 1) as f32,
            front,
        );
        let max = Vec3::new(
            (self.shape_box.pixel_end.x + 1) as f32,
            (grid::SCENE_HEIGHT - self.shape_box.pixel_start.y) as f32,
            back,
        );
        Vec3::scale(min, Vec3::ONE - pivot) + Vec3::scale(max, pivot) + self.offset
    }

    /// The anchor point for the instance's own pivot.
    #[must_use]
    pub fn get_pos(&self, settings: &ConversionSettings) -> Vec3 {
        self.get_pos_with(settings, self.pivot)
    }

    /// Recomputes `pos` and `center` (REQ-E3D-205).
    pub fn calculate_pos(&mut self, settings: &ConversionSettings) {
        self.pos = self.get_pos_with(settings, self.pivot);
        self.center = self.get_pos_with(settings, Vec3::HALF);
    }

    /// `BottomLeft = (pStart.x, Hs − pEnd.y, −s/2)` (SPEC-04 §4.4). Not to be confused with
    /// `Bmin`.
    #[must_use]
    pub fn bottom_left(&self) -> Vec3 {
        Vec3::new(
            self.shape_box.pixel_start.x as f32,
            (grid::SCENE_HEIGHT - self.shape_box.pixel_end.y) as f32,
            -self.depth as f32 / 2.0,
        )
    }

    /// `TopRight = (pEnd.x, Hs − pStart.y, s/2)` (SPEC-04 §4.4).
    #[must_use]
    pub fn top_right(&self) -> Vec3 {
        Vec3::new(
            self.shape_box.pixel_end.x as f32,
            (grid::SCENE_HEIGHT - self.shape_box.pixel_start.y) as f32,
            self.depth as f32 / 2.0,
        )
    }

    /// `BL = (pStart.x, Hs − pEnd.y − 1, −s/2)` (SPEC-04 §4.4).
    #[must_use]
    pub fn bl(&self) -> Vec3 {
        Vec3::new(
            self.shape_box.pixel_start.x as f32,
            (grid::SCENE_HEIGHT - self.shape_box.pixel_end.y - 1) as f32,
            -self.depth as f32 / 2.0,
        )
    }

    /// `TR = (pEnd.x, Hs − pStart.y − 1, s/2)` (SPEC-04 §4.4).
    #[must_use]
    pub fn tr(&self) -> Vec3 {
        Vec3::new(
            self.shape_box.pixel_end.x as f32,
            (grid::SCENE_HEIGHT - self.shape_box.pixel_start.y - 1) as f32,
            self.depth as f32 / 2.0,
        )
    }

    /// `size = (pEnd.x − pStart.x + 1, pEnd.y − pStart.y + 1, s)` (SPEC-04 §4.4).
    #[must_use]
    pub fn size(&self) -> Vec3 {
        Vec3::new(
            (self.shape_box.pixel_end.x - self.shape_box.pixel_start.x + 1) as f32,
            (self.shape_box.pixel_end.y - self.shape_box.pixel_start.y + 1) as f32,
            self.depth as f32,
        )
    }

    /// `OriPos = BottomLeft ⊙ (1 − q) + TopRight ⊙ q` with `q = deform_pivot` (SPEC-05
    /// REQ-LUA-083 point 2).
    #[must_use]
    pub fn ori_pos(&self) -> Vec3 {
        self.get_original_pos(self.deform_pivot)
    }

    /// `GetOriginalPos(q) = BottomLeft ⊙ (1 − q) + TopRight ⊙ q`.
    #[must_use]
    pub fn get_original_pos(&self, pivot: Vec3) -> Vec3 {
        Vec3::scale(self.bottom_left(), Vec3::ONE - pivot) + Vec3::scale(self.top_right(), pivot)
    }

    /// `GetPosWoLayer(q) = BL ⊙ (1 − q) + TR ⊙ q`.
    #[must_use]
    pub fn get_pos_without_layer(&self, pivot: Vec3) -> Vec3 {
        Vec3::scale(self.bl(), Vec3::ONE - pivot) + Vec3::scale(self.tr(), pivot)
    }

    /// Strict overlap of the pixel boxes of the two shapes, with the slack `delta` (SPEC-05
    /// REQ-LUA-083 point 4).
    #[must_use]
    pub const fn overlap(&self, other: &Self, delta: i32) -> bool {
        let (a_start, a_end) = (self.shape_box.pixel_start, self.shape_box.pixel_end);
        let (b_start, b_end) = (other.shape_box.pixel_start, other.shape_box.pixel_end);
        a_start.x < b_end.x + delta
            && a_end.x + delta > b_start.x
            && a_start.y < b_end.y + delta
            && a_end.y + delta > b_start.y
    }

    /// Strict overlap with a rectangle placed on the instance's own box, origin `BottomLeft`
    /// (SPEC-05 REQ-LUA-083 point 4).
    #[must_use]
    pub fn overlap_with_rect(&self, rectangle: &Rect) -> bool {
        let origin = self.bottom_left();
        let size = self.size();
        Rect::new(origin.x, origin.y, size.x, size.y).overlaps(rectangle)
    }

    /// The same test with the origin `BL`, one unit lower. The one-pixel difference with
    /// [`FrameInstance::overlap_with_rect`] is the original's and is kept (SPEC-05
    /// REQ-LUA-083).
    #[must_use]
    pub fn overlap_rect(&self, rectangle: &Rect) -> bool {
        let origin = self.bl();
        let size = self.size();
        Rect::new(origin.x, origin.y, size.x, size.y).overlaps(rectangle)
    }
}

/// The frame model built from one batch of the core: its tiles, its ordered shapes, its
/// instances, its palette, its mask and its counter (REQ-E3D-040).
///
/// A frame owns pools for its tiles and its shapes and is reused from frame to frame, so a
/// full frame of 1 023 background tiles and 128 sprites costs no allocation once warm
/// (REQ-E3D-360).
#[derive(Debug, Clone)]
pub struct ConversionFrame {
    background_cells: Vec<FrameTile>,
    sprite_grid: Vec<Option<TileId>>,
    sprite_pool: Vec<FrameTile>,
    sprite_count: usize,
    extra_pool: Vec<FrameTile>,
    extra_count: usize,
    shape_store: Vec<FrameShape>,
    shape_pool: Vec<ShapeId>,
    shapes: Vec<ShapeId>,
    instances: Vec<FrameInstanceId>,
    scriptable_instances: Vec<FrameInstanceId>,
    pinned_rects: Vec<Rect>,
    palette: FramePalette,
    mask: Cell,
    counter: i32,
    background_shape_count: usize,
    background_tile_count: usize,
    background_pixel_count: i32,
    script_error_count: usize,
}

impl Default for ConversionFrame {
    fn default() -> Self {
        Self::new()
    }
}

impl ConversionFrame {
    /// Creates an empty frame.
    #[must_use]
    pub fn new() -> Self {
        let mut background_cells = Vec::with_capacity(grid::CELL_COUNT);
        for y in 0..grid::HEIGHT {
            for x in 0..grid::WIDTH {
                background_cells.push(FrameTile::new(Cell::new(x, y), grid::index(x, y), true));
            }
        }

        Self {
            background_cells,
            sprite_grid: vec![None; grid::CELL_COUNT],
            sprite_pool: Vec::new(),
            sprite_count: 0,
            extra_pool: Vec::new(),
            extra_count: 0,
            shape_store: Vec::new(),
            shape_pool: Vec::new(),
            shapes: Vec::new(),
            instances: Vec::new(),
            scriptable_instances: Vec::new(),
            pinned_rects: Vec::new(),
            palette: FramePalette::default(),
            mask: Cell::default(),
            counter: 0,
            background_shape_count: 0,
            background_tile_count: 0,
            background_pixel_count: 0,
            script_error_count: 0,
        }
    }

    /// The background tiles kept for this frame, in grid order (REQ-E3D-005, REQ-E3D-041).
    pub fn background_tiles(&self) -> impl Iterator<Item = &FrameTile> {
        self.background_cells.iter().filter(|tile| tile.present)
    }

    /// The sprites kept, in the emission order of the core (REQ-E3D-006).
    #[must_use]
    pub fn sprites(&self) -> &[FrameTile] {
        &self.sprite_pool[..self.sprite_count]
    }

    /// The ordered shapes: every background shape first, then every sprite shape
    /// (REQ-E3D-040).
    #[must_use]
    pub fn shapes(&self) -> &[ShapeId] {
        &self.shapes
    }

    /// The shape of a handle.
    #[must_use]
    pub fn shape(&self, id: ShapeId) -> &FrameShape {
        &self.shape_store[id.0]
    }

    /// The shape of a handle, for writing.
    pub fn shape_mut(&mut self, id: ShapeId) -> &mut FrameShape {
        &mut self.shape_store[id.0]
    }

    /// The ordered shapes themselves.
    pub fn shape_list(&self) -> impl Iterator<Item = &FrameShape> {
        self.shapes.iter().map(|&id| self.shape(id))
    }

    /// How many of the first shapes are background shapes (REQ-E3D-040).
    #[must_use]
    pub const fn background_shape_count(&self) -> usize {
        self.background_shape_count
    }

    /// The frame instances, in shape order then by instance index (REQ-E3D-220).
    #[must_use]
    pub fn instances(&self) -> &[FrameInstanceId] {
        &self.instances
    }

    /// The scriptable instances: those of the shapes the segmentation produced, in the same
    /// order (REQ-E3D-220). Injected and restored shapes are absent.
    #[must_use]
    pub fn scriptable_instances(&self) -> &[FrameInstanceId] {
        &self.scriptable_instances
    }

    /// The frame instance of a handle.
    #[must_use]
    pub fn instance(&self, id: FrameInstanceId) -> &FrameInstance {
        &self.shape_store[id.shape.0].instances[id.index]
    }

    /// The frame instance of a handle, for writing.
    pub fn instance_mut(&mut self, id: FrameInstanceId) -> &mut FrameInstance {
        &mut self.shape_store[id.shape.0].instances[id.index]
    }

    /// Recomputes `pos` and `center` of every instance of the frame (REQ-E3D-205).
    pub fn calculate_positions(&mut self, settings: &ConversionSettings) {
        for &shape in &self.shapes {
            for instance in &mut self.shape_store[shape.0].instances {
                instance.calculate_pos(settings);
            }
        }
    }

    /// The tile of a handle.
    #[must_use]
    pub fn tile(&self, id: TileId) -> &FrameTile {
        match id {
            TileId::Background(slot) => &self.background_cells[slot],
            TileId::Sprite(slot) => &self.sprite_pool[slot],
            TileId::Extra(slot) => &self.extra_pool[slot],
        }
    }

    fn tile_mut(&mut self, id: TileId) -> &mut FrameTile {
        match id {
            TileId::Background(slot) => &mut self.background_cells[slot],
            TileId::Sprite(slot) => &mut self.sprite_pool[slot],
            TileId::Extra(slot) => &mut self.extra_pool[slot],
        }
    }

    /// The tiles of a shape, in the order they were added.
    pub fn shape_tiles(&self, shape: ShapeId) -> impl Iterator<Item = &FrameTile> {
        self.shape(shape).tiles.iter().map(|&id| self.tile(id))
    }

    /// The palette of the frame, 64 entries (REQ-E3D-250).
    #[must_use]
    pub const fn palette(&self) -> &FramePalette {
        &self.palette
    }

    /// The palette of the frame, for writing (REQ-E3D-252).
    pub fn palette_mut(&mut self) -> &mut FramePalette {
        &mut self.palette
    }

    /// The unreliable border widths of the batch (SPEC-02 REQ-CORE-046).
    #[must_use]
    pub const fn mask(&self) -> Cell {
        self.mask
    }

    /// Number of emulated frames since the game started (REQ-E3D-043).
    #[must_use]
    pub const fn counter(&self) -> i32 {
        self.counter
    }

    /// The frame time, `counter / 60` seconds whatever the region (REQ-E3D-043).
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // C# int-to-float conversion.
    pub fn time(&self) -> f32 {
        self.counter as f32 / 60.0
    }

    /// Number of background tiles kept.
    #[must_use]
    pub const fn background_tile_count(&self) -> usize {
        self.background_tile_count
    }

    /// Total number of non-transparent background pixels kept.
    #[must_use]
    pub const fn background_pixel_count(&self) -> i32 {
        self.background_pixel_count
    }

    /// How many Lua errors were swallowed while converting this frame (REQ-E3D-324).
    #[must_use]
    pub const fn script_error_count(&self) -> usize {
        self.script_error_count
    }

    pub(crate) fn count_script_error(&mut self) {
        self.script_error_count += 1;
    }

    /// The rectangles added by `Frame:AddRectFromPinnedFrame`, in scene coordinates
    /// (REQ-E3D-233).
    #[must_use]
    pub fn pinned_rects(&self) -> &[Rect] {
        &self.pinned_rects
    }

    /// The background tile of a cell, or `None` when the cell holds none.
    #[must_use]
    pub fn background_at_cell(&self, cell: Cell) -> Option<&FrameTile> {
        if grid::contains(cell) {
            self.background_at(grid::index_of(cell))
        } else {
            None
        }
    }

    /// The background tile of a grid index, or `None`.
    #[must_use]
    pub fn background_at(&self, index: i32) -> Option<&FrameTile> {
        grid::slot(index)
            .map(|slot| &self.background_cells[slot])
            .filter(|tile| tile.present)
    }

    /// The handle of the background tile of a grid index, or `None`.
    pub(crate) fn background_id(&self, index: i32) -> Option<TileId> {
        grid::slot(index)
            .filter(|&slot| self.background_cells[slot].present)
            .map(TileId::Background)
    }

    /// The sprite that occupies a cell of the sprite grid, or `None` (REQ-E3D-132).
    #[must_use]
    pub fn sprite_occupant(&self, index: i32) -> Option<&FrameTile> {
        grid::slot(index)
            .and_then(|slot| self.sprite_grid[slot])
            .map(|id| self.tile(id))
    }

    /// Empties the frame: no shape, no tile, counter 0 (REQ-E3D-281).
    pub fn clear(&mut self) {
        self.begin_batch(0, Cell::default());
        self.palette.clear();
    }

    /// Starts a new batch: same pools, no shape, no tile.
    pub(crate) fn begin_batch(&mut self, counter: i32, mask: Cell) {
        self.shape_pool.extend_from_slice(&self.shapes);
        self.shapes.clear();
        self.extra_count = 0;
        self.instances.clear();
        self.scriptable_instances.clear();
        self.pinned_rects.clear();
        self.sprite_count = 0;
        self.sprite_grid.fill(None);
        self.background_shape_count = 0;
        self.background_tile_count = 0;
        self.background_pixel_count = 0;
        self.script_error_count = 0;
        self.counter = counter;
        self.mask = mask;
        for tile in self
            .background_cells
            .iter_mut()
            .chain(&mut self.sprite_pool)
        {
            tile.reset();
        }
    }

    pub(crate) fn set_background_shape_count(&mut self) {
        self.background_shape_count = self.shapes.len();
    }

    /// Places a background tile in its cell (REQ-E3D-005). When two tiles of the batch land
    /// in the same cell — which a mid-frame scroll change allows (SPEC-02 REQ-CORE-040) — the
    /// later one wins and the cell germinates once (Q-012). The cell must be inside the grid.
    pub(crate) fn place_background(
        &mut self,
        image: TileImage,
        x: i32,
        y: i32,
        palette: i32,
        hidden: bool,
    ) -> Option<TileId> {
        let slot = grid::slot(grid::index_of(grid::background_cell(x, y)))?;
        let tile = &mut self.background_cells[slot];
        if tile.present {
            self.background_pixel_count -= tile.image.pixel_count();
        } else {
            self.background_tile_count += 1;
        }

        tile.fill(image, x, y, palette, hidden, -1);
        self.background_pixel_count += image.pixel_count();
        Some(TileId::Background(slot))
    }

    /// Appends a sprite tile in emission order (REQ-E3D-006).
    pub(crate) fn add_sprite(
        &mut self,
        image: TileImage,
        x: i32,
        y: i32,
        palette: i32,
        hidden: bool,
        ord: i32,
    ) -> TileId {
        let slot = self.sprite_count;
        if self.sprite_pool.len() <= slot {
            self.sprite_pool
                .push(FrameTile::new(Cell::default(), 0, false));
        }

        let tile = &mut self.sprite_pool[slot];
        tile.fill(image, x, y, palette, hidden, ord);
        tile.position = grid::sprite_cell(x, y);
        tile.grid_index = grid::index_of(tile.position);
        self.sprite_count += 1;
        TileId::Sprite(slot)
    }

    /// Takes a shape from the pool, bound to the grid of its layer. Its mask is the frame's,
    /// which [`FrameShape::inside`] needs (REQ-E3D-045).
    pub(crate) fn take_shape(&mut self, background: bool) -> ShapeId {
        let id = self.shape_pool.pop().unwrap_or_else(|| {
            self.shape_store.push(FrameShape::default());
            ShapeId(self.shape_store.len() - 1)
        });
        let mask = self.mask;
        self.shape_mut(id).reset(background, mask);
        id
    }

    /// Appends a tile that no batch produced: the tiles of a shape a script injects
    /// (REQ-E3D-230 to 232). They never take part in a segmentation.
    pub(crate) fn add_injected_tile(
        &mut self,
        image: TileImage,
        x: i32,
        y: i32,
        palette: i32,
    ) -> TileId {
        let id = self.take_extra_tile();
        let tile = self.tile_mut(id);
        tile.fill(image, x, y, palette, false, -1);
        tile.position = grid::sprite_cell(x, y);
        tile.grid_index = grid::index_of(tile.position);
        id
    }

    fn take_extra_tile(&mut self) -> TileId {
        let slot = self.extra_count;
        if self.extra_pool.len() <= slot {
            self.extra_pool
                .push(FrameTile::new(Cell::default(), 0, false));
        }

        self.extra_count += 1;
        TileId::Extra(slot)
    }

    /// Adds a copy of a shape of another frame — one restored from the pinned frame — as a
    /// non-scriptable shape of this frame, without duplicates (REQ-E3D-233). The copy keeps
    /// its tiles but takes no part in this frame's grids.
    pub(crate) fn add_foreign_shape(&mut self, source: &Self, source_shape: ShapeId) -> bool {
        if self
            .shapes
            .iter()
            .any(|&id| self.shape(id).restored_from == Some(source_shape))
        {
            return false;
        }

        let original = source.shape(source_shape);
        let id = self.take_shape(original.background);
        let mut copy = std::mem::take(&mut self.shape_store[id.0]);
        copy.clone_from(original);
        copy.tiles.clear();
        for &tile_id in &original.tiles {
            let extra = self.take_extra_tile();
            let tile = self.tile_mut(extra);
            *tile = *source.tile(tile_id);
            tile.shape = Some(id);
            copy.tiles.push(extra);
        }

        for instance in &mut copy.instances {
            instance.shape = id;
        }

        copy.scriptable = false;
        copy.restored_from_pinned_frame = true;
        copy.restored_from = Some(source_shape);
        self.shape_store[id.0] = copy;
        self.add_shape(id);
        true
    }

    /// Gives a shape back to the pool without adding it to the frame.
    pub(crate) fn return_shape(&mut self, shape: ShapeId) {
        self.release_tiles(shape);
        self.shape_pool.push(shape);
    }

    /// Adds a shape and its instances to the frame, in retention order (REQ-E3D-092,
    /// REQ-E3D-220).
    pub(crate) fn add_shape(&mut self, shape: ShapeId) {
        self.shapes.push(shape);
        let data = &self.shape_store[shape.0];
        for index in 0..data.instances.len() {
            let id = FrameInstanceId { shape, index };
            self.instances.push(id);
            if data.scriptable {
                self.scriptable_instances.push(id);
            }
        }
    }

    /// Adds a rectangle to the pinned-frame list (REQ-E3D-233).
    pub(crate) fn add_pinned_rect(&mut self, rectangle: Rect) {
        self.pinned_rects.push(rectangle);
    }

    /// True when the 8 × 8 cell of `tile` overlaps one of the frame's rectangles; the test a
    /// restored shape's tiles must pass to be drawn (REQ-E3D-233).
    ///
    /// The tile box is in screen pixels and the rectangles are in scene coordinates, so the y
    /// axis of the two is reversed. The mismatch is the original's and is kept as is
    /// (DEC-04-Q05, Q-05).
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // C# int-to-float conversions of small values.
    pub fn tile_overlaps_pinned_rects(&self, tile: &FrameTile) -> bool {
        let tile_box = Rect::new(tile.x as f32, tile.y as f32, 8.0, 8.0);
        self.pinned_rects
            .iter()
            .any(|rectangle| tile_box.overlaps(rectangle))
    }

    pub(crate) fn set_scriptable(&mut self, shape: ShapeId, scriptable: bool) {
        self.shape_mut(shape).scriptable = scriptable;
    }

    /// Adds `tile` to `shape` (REQ-E3D-090): the tile now belongs to the shape and, for a
    /// sprite shape, occupies its cell of the sprite grid for the rest of the frame
    /// (REQ-E3D-132).
    pub(crate) fn add_tile(&mut self, shape: ShapeId, tile_id: TileId) {
        let tile = {
            let tile = self.tile_mut(tile_id);
            tile.shape = Some(shape);
            *tile
        };

        let data = &mut self.shape_store[shape.0];
        if !data.background {
            if let Some(slot) = grid::slot(tile.grid_index) {
                self.sprite_grid[slot] = Some(tile_id);
            }
        }

        data.push_tile(tile_id, &tile);
    }

    /// The tile of `shape` at grid index `index`, or `None` when the cell holds no tile of
    /// this shape.
    #[must_use]
    pub fn shape_tile_at(&self, shape: ShapeId, index: i32) -> Option<&FrameTile> {
        let slot = grid::slot(index)?;
        let tile = if self.shape(shape).background {
            Some(&self.background_cells[slot]).filter(|tile| tile.present)
        } else {
            self.sprite_grid[slot].map(|id| self.tile(id))
        }?;
        (tile.shape == Some(shape)).then_some(tile)
    }

    /// The handle of the tile of `shape` at grid index `index`, or `None`.
    pub(crate) fn shape_tile_id_at(&self, shape: ShapeId, index: i32) -> Option<TileId> {
        let slot = grid::slot(index)?;
        let id = if self.shape(shape).background {
            Some(TileId::Background(slot))
        } else {
            self.sprite_grid[slot]
        }?;
        let tile = self.tile(id);
        (tile.present && tile.shape == Some(shape)).then_some(id)
    }

    /// True when no cell strictly inside the tile box and outside the shape is *surrounded*,
    /// that is, has a tile of the shape somewhere to its left, right, above and below inside
    /// the box (REQ-E3D-094).
    #[must_use]
    pub fn is_full(&self, shape: ShapeId) -> bool {
        let data = self.shape(shape);
        for x in data.tile_start.x + 1..data.tile_end.x {
            for y in data.tile_start.y + 1..data.tile_end.y {
                if self.shape_tile_at(shape, grid::index(x, y)).is_none()
                    && self.is_surrounded(shape, x, y)
                {
                    return false;
                }
            }
        }

        true
    }

    fn is_surrounded(&self, shape: ShapeId, x: i32, y: i32) -> bool {
        let data = self.shape(shape);
        let has = |i: i32, j: i32| self.shape_tile_at(shape, grid::index(i, j)).is_some();
        (data.tile_start.x..x).any(|i| has(i, y))
            && (x + 1..=data.tile_end.x).any(|i| has(i, y))
            && (data.tile_start.y..y).any(|j| has(x, j))
            && (y + 1..=data.tile_end.y).any(|j| has(x, j))
    }

    /// Binds `shape` to the pattern `id`: copies its recognition parameters into the frame
    /// pattern, creates one frame instance per pattern instance (REQ-E3D-190) and computes the
    /// screen centre.
    #[allow(clippy::cast_precision_loss)] // C# int-to-float conversions of small values.
    pub(crate) fn bind(&mut self, shape: ShapeId, id: PatternId, pattern: &EnginePattern) {
        let data = &mut self.shape_store[shape.0];
        data.reference_pattern = Some(id);
        data.reference_image_count = pattern.images().len();
        data.constraint_size = pattern.constraint_size;
        data.self_connected = pattern.self_connected();
        data.skip_geometry = pattern.skip_geometry;

        let shape_box = data.shape_box();
        data.instances.clear();
        for (index, source) in pattern.instances().iter().enumerate() {
            // The copy of REQ-E3D-190: every parameter, the tags shared, the custom palette
            // reset. Cloning shares the reference-counted parts, so binding allocates nothing
            // once the instance list is warm (REQ-E3D-360).
            let mut parameters = source.clone();
            parameters.custom_palette = -1;
            parameters.set_index(index);
            data.instances.push(FrameInstance {
                parameters,
                reference: InstanceRef { pattern: id, index },
                shape,
                shape_box,
                pos: Vec3::default(),
                center: Vec3::default(),
            });
        }

        data.center = Vec3::new(
            (data.pixel_start.x + data.pixel_end.x + 1) as f32 / 2.0,
            (data.pixel_start.y + data.pixel_end.y + 1) as f32 / 2.0,
            0.0,
        );
    }

    /// Releases the shape's tiles so that another germ can take them: the tiles of a shape
    /// rejected by pass 1 (REQ-E3D-092) or dropped by the band extraction of 2D mode
    /// (REQ-E3D-381), both of which are background passes.
    ///
    /// The grid entry is left alone on purpose. For a background shape the grid *is* the list
    /// of tiles kept in the batch, which a release must not empty; for a sprite shape the
    /// entry is the cell occupancy of REQ-E3D-132, which lasts for the rest of the frame.
    pub(crate) fn release_tiles(&mut self, shape: ShapeId) {
        let mut tiles = std::mem::take(&mut self.shape_store[shape.0].tiles);
        for &id in &tiles {
            let tile = self.tile_mut(id);
            if tile.shape == Some(shape) {
                tile.shape = None;
            }
        }

        tiles.clear();
        let data = &mut self.shape_store[shape.0];
        data.tiles = tiles;
        data.images.clear();
        data.pixel_count = 0;
    }
}
