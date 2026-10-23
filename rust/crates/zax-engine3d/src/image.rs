//! Tile images and the session's image registry (SPEC-04 §2, REQ-E3D-007, REQ-E3D-020,
//! REQ-E3D-021).

use std::collections::HashMap;
use std::fmt;

use crate::registry::PatternId;
use crate::vectors::Cell;

/// The identity of a tile image: its two bit planes and its flips (SPEC-04 §2).
///
/// Two tiles have the same image if and only if the four components are equal, so the same
/// CHR data shown flipped is *another* image. This is the unit of identity of the whole
/// recognition (REQ-E3D-007, TV-E3D-10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TileImageKey {
    /// Low bit plane, packed as in SPEC-02 REQ-CORE-034.
    pub low: u64,
    /// High bit plane.
    pub high: u64,
    /// Horizontal flip.
    pub flip_x: bool,
    /// Vertical flip.
    pub flip_y: bool,
}

impl TileImageKey {
    /// Creates a key.
    #[must_use]
    pub const fn new(low: u64, high: u64, flip_x: bool, flip_y: bool) -> Self {
        Self {
            low,
            high,
            flip_x,
            flip_y,
        }
    }

    /// The flips as the two-bit value of SPEC-03 (bit 0 horizontal, bit 1 vertical).
    #[must_use]
    pub const fn flip(&self) -> u8 {
        (self.flip_x as u8) | ((self.flip_y as u8) << 1)
    }

    /// True when both planes are zero: the image draws nothing.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        (self.low | self.high) == 0
    }

    /// Colour (0 to 3) of the pixel *displayed* at (`px`, `py`), flips applied, both 0 to 7.
    /// Colour 0 is transparent (SPEC-04 §2).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // Two bits.
    pub const fn color_at(&self, px: i32, py: i32) -> i32 {
        let sx = if self.flip_x { 7 - px } else { px };
        let sy = if self.flip_y { 7 - py } else { py };
        let bit = (sy << 3) | (7 - sx);
        (((self.low >> bit) & 1) | (((self.high >> bit) & 1) << 1)) as i32
    }
}

/// Colours used by an image; colour 0 is transparent and has no flag (bits 1 to 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ImageColors(u8);

impl ImageColors {
    /// No colour.
    pub const NONE: Self = Self(0);
    /// Colour 1 occurs.
    pub const COLOR1: Self = Self(1 << 1);
    /// Colour 2 occurs.
    pub const COLOR2: Self = Self(1 << 2);
    /// Colour 3 occurs.
    pub const COLOR3: Self = Self(1 << 3);

    /// The raw flags.
    #[must_use]
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// True when every flag of `other` is set.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Number of colours set.
    #[must_use]
    pub const fn count(self) -> u32 {
        self.0.count_ones()
    }
}

/// Density class of an image (REQ-E3D-021).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageDensity {
    /// No opaque pixel.
    Empty = 0,
    /// 1 to 15 opaque pixels.
    Low = 1,
    /// 16 to 31.
    Average = 2,
    /// 32 to 47.
    Medium = 3,
    /// 48 to 63.
    High = 4,
    /// All 64 pixels.
    Solid = 5,
}

/// 2D shape class of an image (REQ-E3D-021).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageShape2D {
    /// None of the other classes.
    Unidentified = 0,
    /// One colour and fewer than 48 opaque pixels; used only by 2D mode (REQ-E3D-381).
    Char = 1,
    /// The opaque pixels exactly fill the pixel box.
    Rect = 2,
    /// At least 60 opaque pixels.
    FullRect = 3,
}

/// An image of the session's image registry: its identity and its derived attributes
/// (REQ-E3D-020, REQ-E3D-021).
///
/// It is a small value: every tile, shape and pattern holds a copy, and two copies are the
/// same image exactly when their identities are equal. The registry patterns that contain an
/// image are kept by the [`ImageRegistry`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TileImage {
    key: TileImageKey,
    id: u32,
    pixel_count: i32,
    colors: ImageColors,
    pixel_start: Cell,
    pixel_end: Cell,
    density: ImageDensity,
    shape_2d: ImageShape2D,
}

impl TileImage {
    fn new(key: TileImageKey, id: u32) -> Self {
        let mut count = 0;
        let (mut sx, mut sy, mut ex, mut ey) = (7, 7, 0, 0);
        let mut colors = 0u8;
        for px in 0..8 {
            for py in 0..8 {
                let color = key.color_at(px, py);
                if color == 0 {
                    continue;
                }

                count += 1;
                sx = sx.min(px);
                sy = sy.min(py);
                ex = ex.max(px);
                ey = ey.max(py);
                colors |= 1 << color;
            }
        }

        let colors = ImageColors(colors);
        let pixel_start = Cell::new(sx, sy);
        let pixel_end = Cell::new(ex, ey);
        Self {
            key,
            id,
            pixel_count: count,
            colors,
            pixel_start,
            pixel_end,
            density: classify_density(count),
            shape_2d: classify_shape(count, colors, pixel_start, pixel_end),
        }
    }

    /// The empty image of identity 0, held by a pooled tile that no batch has filled yet.
    pub(crate) fn placeholder() -> Self {
        Self::new(TileImageKey::default(), 0)
    }

    /// The image identity.
    #[must_use]
    pub const fn key(&self) -> TileImageKey {
        self.key
    }

    /// Identity in the session's image registry: a small stable integer, in the order the
    /// images were first met (REQ-E3D-007).
    #[must_use]
    pub const fn id(&self) -> u32 {
        self.id
    }

    /// Number of non-transparent pixels, 0 to 64 (REQ-E3D-020).
    #[must_use]
    pub const fn pixel_count(&self) -> i32 {
        self.pixel_count
    }

    /// The colours used, a subset of {1, 2, 3} (REQ-E3D-020).
    #[must_use]
    pub const fn colors(&self) -> ImageColors {
        self.colors
    }

    /// Top-left corner of the pixel box; `(7, 7)` for an empty image (REQ-E3D-020).
    #[must_use]
    pub const fn pixel_start(&self) -> Cell {
        self.pixel_start
    }

    /// Bottom-right corner of the pixel box, inclusive; `(0, 0)` for an empty image.
    #[must_use]
    pub const fn pixel_end(&self) -> Cell {
        self.pixel_end
    }

    /// Density class (REQ-E3D-021).
    #[must_use]
    pub const fn density(&self) -> ImageDensity {
        self.density
    }

    /// 2D shape class (REQ-E3D-021).
    #[must_use]
    pub const fn shape_2d(&self) -> ImageShape2D {
        self.shape_2d
    }

    /// True when the image has no opaque pixel.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.pixel_count == 0
    }

    /// Number of distinct colours used, 0 to 3.
    #[must_use]
    pub const fn color_count(&self) -> u32 {
        self.colors.count()
    }

    /// Colour (0 to 3) of the displayed pixel at (x, y).
    #[must_use]
    pub const fn color_at(&self, px: i32, py: i32) -> i32 {
        self.key.color_at(px, py)
    }
}

impl fmt::Display for TileImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Image #{} (n={}, flip={})",
            self.id,
            self.pixel_count,
            self.key.flip()
        )
    }
}

const fn classify_density(count: i32) -> ImageDensity {
    match count {
        0 => ImageDensity::Empty,
        1..16 => ImageDensity::Low,
        16..32 => ImageDensity::Average,
        32..48 => ImageDensity::Medium,
        48..64 => ImageDensity::High,
        _ => ImageDensity::Solid,
    }
}

const fn classify_shape(count: i32, colors: ImageColors, start: Cell, end: Cell) -> ImageShape2D {
    if count >= 60 {
        return ImageShape2D::FullRect;
    }

    if colors.count() == 1 && count < 48 {
        return ImageShape2D::Char;
    }

    let area = (end.x - start.x + 1) * (end.y - start.y + 1);
    if count > 0 && count == area {
        ImageShape2D::Rect
    } else {
        ImageShape2D::Unidentified
    }
}

/// The registry patterns that contain one image, the recognition index of REQ-E3D-091.
///
/// Both lists are kept ordered, never sorted, so the result of the segmentation does not
/// depend on a hash order. AMD-021 R2-008 amends REQ-E3D-091 and closes DEC-04-Q02: when a
/// pattern is taken out of the registry and another pattern carrying the same image is
/// registered afterwards, the newcomer takes *the place of the last pattern removed* from this
/// image rather than the end of the list. One place is remembered per list, and filling it
/// uses it up, so a second newcomer goes to the end again.
#[derive(Debug, Clone, Default)]
struct Candidates {
    permanent: Vec<PatternId>,
    temporary: Vec<PatternId>,
    freed_permanent: Option<usize>,
    freed_temporary: Option<usize>,
}

impl Candidates {
    fn list(&mut self, permanent: bool) -> (&mut Vec<PatternId>, &mut Option<usize>) {
        if permanent {
            (&mut self.permanent, &mut self.freed_permanent)
        } else {
            (&mut self.temporary, &mut self.freed_temporary)
        }
    }

    fn attach(&mut self, pattern: PatternId, permanent: bool) {
        let (list, freed) = self.list(permanent);
        if list.contains(&pattern) {
            return;
        }

        // R2-008: the place of the last pattern removed from this image, if one is still
        // free. Q-059: occupying it uses it up, so a second newcomer goes to the end.
        match freed.take() {
            Some(place) if place <= list.len() => list.insert(place, pattern),
            _ => list.push(pattern),
        }
    }

    fn detach(&mut self, pattern: PatternId, permanent: bool) {
        let (list, freed) = self.list(permanent);
        if let Some(place) = list.iter().position(|&p| p == pattern) {
            list.remove(place);

            // R2-008: this is now the place the next newcomer takes.
            *freed = Some(place);
        }
    }
}

/// The session's image registry (REQ-E3D-007): every image met in a batch or in a loaded
/// pattern gets one unique, stable [`TileImage`].
///
/// Interning is a hash lookup, so getting the candidates of an image is constant time
/// amortised, as REQ-E3D-360 requires. The registry is part of the learned state a savestate
/// carries (REQ-E3D-008, DEC-R2-T5).
#[derive(Debug, Clone, Default)]
pub struct ImageRegistry {
    by_key: HashMap<TileImageKey, u32>,
    images: Vec<TileImage>,
    candidates: Vec<Candidates>,
}

impl ImageRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of distinct images met so far.
    #[must_use]
    pub fn count(&self) -> usize {
        self.images.len()
    }

    /// The images in registry order; index = [`TileImage::id`].
    #[must_use]
    pub fn images(&self) -> &[TileImage] {
        &self.images
    }

    /// The image of `key`, creating it on first sight.
    pub fn intern(&mut self, key: TileImageKey) -> TileImage {
        if let Some(&id) = self.by_key.get(&key) {
            return self.images[id as usize];
        }

        #[allow(clippy::cast_possible_truncation)] // C# int identities; far below 2^32 images.
        let id = self.images.len() as u32;
        let image = TileImage::new(key, id);
        self.by_key.insert(key, id);
        self.images.push(image);
        self.candidates.push(Candidates::default());
        image
    }

    /// The image of registry identity `id`, or `None` when no image has it.
    #[must_use]
    pub fn by_id(&self, id: u32) -> Option<TileImage> {
        self.images.get(id as usize).copied()
    }

    /// Permanent registry patterns that contain the image, in registration order: the
    /// initial candidate list of REQ-E3D-091.
    #[must_use]
    pub fn permanent_patterns(&self, image: &TileImage) -> &[PatternId] {
        self.candidates
            .get(image.id as usize)
            .map_or(&[], |candidates| &candidates.permanent)
    }

    /// Temporary registry patterns that contain the image, in registration order.
    #[must_use]
    pub fn temporary_patterns(&self, image: &TileImage) -> &[PatternId] {
        self.candidates
            .get(image.id as usize)
            .map_or(&[], |candidates| &candidates.temporary)
    }

    /// True when at least one *permanent* pattern of the registry contains the image, disabled
    /// patterns included (SPEC-04 §2, REQ-E3D-093).
    #[must_use]
    pub fn is_known(&self, image: &TileImage) -> bool {
        !self.permanent_patterns(image).is_empty()
    }

    pub(crate) fn attach(&mut self, image: &TileImage, pattern: PatternId, permanent: bool) {
        if let Some(candidates) = self.candidates.get_mut(image.id as usize) {
            candidates.attach(pattern, permanent);
        }
    }

    pub(crate) fn detach(&mut self, image: &TileImage, pattern: PatternId, permanent: bool) {
        if let Some(candidates) = self.candidates.get_mut(image.id as usize) {
            candidates.detach(pattern, permanent);
        }
    }

    /// Forgets every image. Done when the game changes (REQ-E3D-008).
    pub fn clear(&mut self) {
        self.by_key.clear();
        self.images.clear();
        self.candidates.clear();
    }

    /// Forgets the images of identity `count` and above, keeping the identities of the others.
    /// Used before a savestate's learned state is put back: the images the profile interned
    /// keep their identity, the ones met while playing are replaced by the recorded ones
    /// (REQ-E3D-008, DEC-R2-T5).
    pub fn truncate_to(&mut self, count: usize) {
        for image in self.images.drain(count.min(self.images.len())..) {
            self.by_key.remove(&image.key);
        }
        self.candidates.truncate(count);
    }
}
