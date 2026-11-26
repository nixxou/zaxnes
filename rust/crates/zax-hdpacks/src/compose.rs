//! Drawing one HD frame, as Mesen2's `HdNesPack` does.
//!
//! Which tiles make each pixel comes from the core's tile records: the background tile covering
//! it and up to four sprites, in OAM order, plus the `<addition>` tiles the pack adds. That model
//! is checked against the real picture: where it does not give the colour the PPU shows (the
//! sprite limit, a palette or scroll changed in the middle of the frame), the original pixel is
//! kept, enlarged. Mesen records the tiles of each pixel inside its PPU; this is the
//! approximation of it the records allow.
//!
//! Each block is drawn from the back: the universal colour, backgrounds of priority 0-9, the
//! sprites behind the background, backgrounds 10-19, the background tile, backgrounds 20-29, the
//! other sprites, backgrounds 30-39. A tile without an HD picture is drawn in its own colours.
//!
//! The rule of a tile is chosen once per tile on screen, its conditions evaluated at the tile's
//! top-left pixel (Mesen evaluates per pixel; the result is the same for tile-aligned offsets).

// Screen geometry: positions of a few hundred pixels, checked before each cast.
#![allow(
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation
)]

use std::collections::HashMap;

use zax_core_api::{PaletteSnapshot, TileRecord};

use crate::pack::{Blend, Condition, HdPack, HdTile, Rule, Term, TileData, TileKey, Wanted};
use crate::NesRgb;

/// The byte of a picture pixel left out by the overscan (the renderer's convention).
pub const HIDDEN_PIXEL: u8 = 0xFF;
/// The colour bits of a picture pixel.
pub const PIXEL_COLOR: u8 = 0x3F;
/// The opacity bit of a picture pixel: an opaque pixel of the PPU drew it.
pub const OPAQUE_PIXEL: u8 = 0x40;

/// What a frame lends the compositor.
pub struct FrameInput<'a> {
    /// One byte a pixel, row 0 first (colour index, opacity bit), `width` x `height`.
    pub picture: &'a [u8],
    pub width: usize,
    pub height: usize,
    /// The columns and rows of the picture before the screen's (widescreen margins).
    pub margin: i32,
    pub margin_y: i32,
    pub records: &'a [TileRecord],
    pub palette: PaletteSnapshot,
    pub frame: u64,
    /// The colours of the 64 NES colour indices.
    pub rgb: &'a NesRgb,
    /// Reads a byte of CPU memory (memory conditions).
    pub memory: &'a dyn Fn(u16) -> u8,
}

/// The CHR ROM tile indices of each pattern (empty for a CHR-RAM game).
#[derive(Debug, Clone, Default)]
pub struct ChrIndex {
    indices: HashMap<[u8; 16], Vec<u32>>,
}

impl ChrIndex {
    /// The index of `chr_rom`, 16 bytes a tile.
    #[must_use]
    pub fn new(chr_rom: &[u8]) -> Self {
        let mut indices: HashMap<[u8; 16], Vec<u32>> = HashMap::new();
        for (at, tile) in chr_rom.chunks_exact(16).enumerate() {
            let mut bytes = [0u8; 16];
            bytes.copy_from_slice(tile);
            indices.entry(bytes).or_default().push(at as u32);
        }
        Self { indices }
    }

    /// The ways a pack may name the pattern `bytes`: its CHR ROM indices, then its bytes.
    fn names(&self, bytes: &[u8; 16]) -> Vec<TileData> {
        let mut names: Vec<TileData> = self
            .indices
            .get(bytes)
            .map(|list| list.iter().map(|&index| TileData::Index(index)).collect())
            .unwrap_or_default();
        names.push(TileData::Bytes(*bytes));
        names
    }
}

/// The 16 CHR bytes of a record (low plane rows then high plane rows).
#[must_use]
pub fn chr_bytes(record: &TileRecord) -> [u8; 16] {
    let mut chr = [0u8; 16];
    for row in 0..8 {
        chr[row] = (record.plane0 >> (8 * row)).to_le_bytes()[0];
        chr[8 + row] = (record.plane1 >> (8 * row)).to_le_bytes()[0];
    }
    chr
}

/// The palette of a record as a pack names it: the backdrop then the sub-palette for a
/// background tile, `FF` then the sub-palette for a sprite.
#[must_use]
pub fn palette_of(record: &TileRecord, palette: &PaletteSnapshot) -> [u8; 4] {
    let base = usize::from(record.palette & 3) * 4 + if record.background { 0 } else { 16 };
    [
        if record.background {
            palette[0] & 0x3F
        } else {
            0xFF
        },
        palette[base + 1] & 0x3F,
        palette[base + 2] & 0x3F,
        palette[base + 3] & 0x3F,
    ]
}

/// The widescreen margins of the picture: its columns and rows before the screen's.
#[derive(Debug, Clone, Copy)]
struct Margins(i32, i32);

/// Runs `draw` on each `chunk`-long piece of `out` with its number, over the cores (rayon), in
/// order on a target without threads.
fn for_each_chunk<T: Send>(out: &mut [T], chunk: usize, draw: &(dyn Fn(usize, &mut [T]) + Sync)) {
    let chunk = chunk.max(1);
    #[cfg(not(target_arch = "wasm32"))]
    {
        use rayon::prelude::*;
        out.par_chunks_mut(chunk)
            .enumerate()
            .for_each(|(at, piece)| draw(at, piece));
    }
    #[cfg(target_arch = "wasm32")]
    for (at, piece) in out.chunks_mut(chunk).enumerate() {
        draw(at, piece);
    }
}

/// One tile of the frame: a record of the core, or a tile an `<addition>` puts beside one.
struct Entry {
    record: TileRecord,
    bytes: [u8; 16],
    palette: [u8; 4],
    /// An addition: no colours of its own, only its HD picture.
    added: bool,
}

impl Entry {
    /// The colour index (0 to 3) shown at screen pixel (`x`, `y`) of the picture, flips
    /// applied; 0 for an addition.
    fn index_at(&self, margins: Margins, x: usize, y: usize) -> u8 {
        if self.added {
            return 0;
        }
        let px = x as i32 - (i32::from(self.record.x) + margins.0);
        let py = y as i32 - (i32::from(self.record.y) + margins.1);
        if !(0..8).contains(&px) || !(0..8).contains(&py) {
            return 0;
        }
        let column = if self.record.mirror_h { 7 - px } else { px };
        let row = if self.record.mirror_v { 7 - py } else { py };
        self.record
            .source_pixel(column as usize, row as usize)
            .unwrap_or(0)
    }

    fn is(&self, wanted: &Wanted, chr: &ChrIndex) -> bool {
        if !wanted.ignore_palette && self.palette != wanted.key.palette {
            return false;
        }
        match wanted.key.data {
            TileData::Bytes(bytes) => bytes == self.bytes,
            TileData::Index(index) => chr
                .indices
                .get(&self.bytes)
                .is_some_and(|list| list.contains(&index)),
        }
    }
}

const NONE: u32 = u32::MAX;

/// The tiles of each pixel of the picture: the background tile and up to four sprites.
struct Owners {
    width: usize,
    height: usize,
    background: Vec<u32>,
    sprites: Vec<[u32; 4]>,
}

impl Owners {
    fn at(&self, input: &FrameInput<'_>, x: i32, y: i32) -> Option<usize> {
        let (px, py) = (x + input.margin, y + input.margin_y);
        (px >= 0 && py >= 0 && (px as usize) < self.width && (py as usize) < self.height)
            .then(|| py as usize * self.width + px as usize)
    }

    /// Calls `mark` for each picture pixel of the 8 x 8 at screen (`left`, `top`).
    fn cover(
        &mut self,
        input: &FrameInput<'_>,
        left: i32,
        top: i32,
        mut mark: impl FnMut(&mut Self, usize),
    ) {
        for py in 0..8 {
            for px in 0..8 {
                if let Some(at) = self.at(input, left + px, top + py) {
                    mark(self, at);
                }
            }
        }
    }

    fn add_sprite(&mut self, input: &FrameInput<'_>, entry: u32, left: i32, top: i32) {
        self.cover(input, left, top, |owners, pixel| {
            if let Some(slot) = owners.sprites[pixel].iter_mut().find(|s| **s == NONE) {
                *slot = entry;
            }
        });
    }
}

/// The entries and owners of a frame, additions included.
fn build(pack: &HdPack, chr: &ChrIndex, input: &FrameInput<'_>) -> (Vec<Entry>, Owners) {
    let mut entries: Vec<Entry> = input
        .records
        .iter()
        .map(|record| Entry {
            record: *record,
            bytes: chr_bytes(record),
            palette: palette_of(record, &input.palette),
            added: false,
        })
        .collect();
    let mut owners = Owners {
        width: input.width,
        height: input.height,
        background: vec![NONE; input.width * input.height],
        sprites: vec![[NONE; 4]; input.width * input.height],
    };
    let mut sprites: Vec<(i32, u32)> = Vec::new();
    for (at, entry) in entries.iter().enumerate() {
        let at = at as u32;
        if entry.record.background {
            let (x, y) = (i32::from(entry.record.x), i32::from(entry.record.y));
            owners.cover(input, x, y, |owners, pixel| owners.background[pixel] = at);
        } else {
            sprites.push((entry.record.index, at));
        }
    }
    // OAM order: the first sprite is in front.
    sprites.sort_unstable();
    for &(_, at) in &sprites {
        let record = entries[at as usize].record;
        owners.add_sprite(input, at, i32::from(record.x), i32::from(record.y));
    }
    if !pack.additions.is_empty() {
        // Sprites first, then the background tiles, as Mesen inserts them.
        let sources: Vec<u32> = sprites
            .iter()
            .map(|&(_, at)| at)
            .chain((0..entries.len() as u32).filter(|&at| entries[at as usize].record.background))
            .collect();
        let mut added = Vec::new();
        for at in sources {
            let source = &entries[at as usize];
            for data in chr.names(&source.bytes) {
                for addition in pack.additions.iter().filter(|a| a.original.data == data) {
                    if !addition.ignore_palette && addition.original.palette != source.palette {
                        continue;
                    }
                    let TileData::Bytes(bytes) = addition.added.data else {
                        continue;
                    };
                    let mut record = source.record;
                    let dx = if record.mirror_h {
                        -addition.x
                    } else {
                        addition.x
                    };
                    let dy = if record.mirror_v {
                        -addition.y
                    } else {
                        addition.y
                    };
                    record.x = (i32::from(record.x) + dx) as i16;
                    record.y = (i32::from(record.y) + dy) as i16;
                    record.background = false;
                    added.push(Entry {
                        record,
                        bytes,
                        palette: addition.added.palette,
                        added: true,
                    });
                }
            }
        }
        for entry in added {
            let at = entries.len() as u32;
            let (x, y) = (i32::from(entry.record.x), i32::from(entry.record.y));
            entries.push(entry);
            owners.add_sprite(input, at, x, y);
        }
    }
    (entries, owners)
}

struct Context<'a, 'b> {
    pack: &'a HdPack,
    input: &'a FrameInput<'b>,
    owners: &'a Owners,
    entries: &'a [Entry],
    chr: &'a ChrIndex,
    /// The conditions true of the whole frame, once evaluated: 0 not yet, 1 false, 2 true.
    global: std::cell::RefCell<Vec<u8>>,
}

impl Context<'_, '_> {
    fn sprite_is(&self, x: i32, y: i32, wanted: &Wanted) -> bool {
        self.owners.at(self.input, x, y).is_some_and(|pixel| {
            self.owners.sprites[pixel]
                .iter()
                .take_while(|&&s| s != NONE)
                .any(|&s| self.entries[s as usize].is(wanted, self.chr))
        })
    }

    fn tile_is(&self, x: i32, y: i32, wanted: &Wanted) -> bool {
        self.owners.at(self.input, x, y).is_some_and(|pixel| {
            let at = self.owners.background[pixel];
            at != NONE && self.entries[at as usize].is(wanted, self.chr)
        })
    }

    /// Whether `term` holds for `entry` (none: a background's term).
    fn holds(&self, term: Term, entry: Option<&Entry>) -> bool {
        let condition = &self.pack.conditions[term.condition];
        let value = if condition.is_global() {
            let known = self.global.borrow()[term.condition];
            if known == 0 {
                let value = self.evaluate(condition, entry);
                self.global.borrow_mut()[term.condition] = 1 + u8::from(value);
                value
            } else {
                known == 2
            }
        } else {
            self.evaluate(condition, entry)
        };
        value != term.negated
    }

    fn evaluate(&self, condition: &Condition, entry: Option<&Entry>) -> bool {
        let record = entry.map(|e| e.record);
        let sprite = record.filter(|r| !r.background);
        let (x, y) = record.map_or((0, 0), |r| (i32::from(r.x), i32::from(r.y)));
        let memory = |address: u16| (self.input.memory)(address);
        match condition {
            Condition::HMirror => sprite.is_some_and(|r| r.mirror_h),
            Condition::VMirror => sprite.is_some_and(|r| r.mirror_v),
            Condition::BgPriority => sprite.is_some_and(|r| r.hidden),
            Condition::SpritePalette(palette) => sprite.is_some_and(|r| r.palette & 3 == *palette),
            Condition::TileAtPosition { x, y, wanted } => self.tile_is(*x, *y, wanted),
            Condition::SpriteAtPosition { x, y, wanted } => self.sprite_is(*x, *y, wanted),
            Condition::TileNearby {
                x: dx,
                y: dy,
                wanted,
            } => self.tile_is(x + dx, y + dy, wanted),
            Condition::SpriteNearby {
                x: dx,
                y: dy,
                wanted,
            } => {
                let sx = if record.is_some_and(|r| r.mirror_h) {
                    -dx
                } else {
                    *dx
                };
                let sy = if record.is_some_and(|r| r.mirror_v) {
                    -dy
                } else {
                    *dy
                };
                self.sprite_is(x + sx, y + sy, wanted)
            }
            Condition::MemoryCheck {
                left,
                operator,
                right,
                mask,
            } => operator.holds(
                i32::from(memory(*left) & mask),
                i32::from(memory(*right) & mask),
            ),
            Condition::MemoryConstant {
                address,
                operator,
                value,
                mask,
            } => operator.holds(i32::from(memory(*address) & mask), i32::from(*value)),
            Condition::FrameRange { divisor, compare } => {
                self.input.frame % u64::from(*divisor) >= u64::from(*compare)
            }
            Condition::Position {
                vertical,
                operator,
                value,
            } => operator.holds(if *vertical { y } else { x }, *value),
            // At the tile's top-left pixel, its column (row) is 0, or 7 mirrored.
            Condition::OriginPosition {
                vertical,
                operator,
                value,
            } => {
                let mirrored =
                    record.is_some_and(|r| if *vertical { r.mirror_v } else { r.mirror_h });
                operator.holds(if mirrored { 7 } else { 0 }, *value)
            }
            Condition::Unsupported => false,
        }
    }

    fn first<'r>(&self, rules: Option<&'r Vec<Rule>>, entry: &Entry) -> Option<&'r HdTile> {
        rules?
            .iter()
            .find(|rule| rule.terms.iter().all(|&term| self.holds(term, Some(entry))))
            .map(|rule| &rule.tile)
    }

    /// The HD tile the pack gives entry `at`, if any: its own key, then its default tile.
    fn tile_of(&self, at: usize) -> Option<HdTile> {
        let entry = &self.entries[at];
        let names = self.chr.names(&entry.bytes);
        for &data in &names {
            let key = TileKey {
                data,
                palette: entry.palette,
            };
            if let Some(tile) = self.first(self.pack.rules.get(&key), entry) {
                return Some(*tile);
            }
        }
        names
            .iter()
            .find_map(|data| self.first(self.pack.defaults.get(data), entry))
            .copied()
    }
}

/// A colour with the brightness of a line applied, as Mesen does.
fn lit(value: u8, brightness: f32) -> u8 {
    if (brightness - 1.0).abs() < f32::EPSILON {
        return value;
    }
    let level = (brightness * 255.0) as i32;
    ((level * (i32::from(value) + 1)) >> 8).clamp(0, 255) as u8
}

/// `colour` over `under`, its brightness applied.
fn blend(under: &mut [u8; 4], colour: [u8; 4], brightness: f32, mode: Blend) {
    let c = |at: usize| lit(colour[at], brightness);
    match mode {
        Blend::Alpha => {
            if colour[3] == 0 {
                return;
            }
            let alpha = u16::from(colour[3]);
            for (at, channel) in under.iter_mut().take(3).enumerate() {
                *channel =
                    ((u16::from(c(at)) * alpha + u16::from(*channel) * (255 - alpha)) / 255) as u8;
            }
        }
        Blend::Add => {
            for (at, channel) in under.iter_mut().take(3).enumerate() {
                *channel = channel.saturating_add(c(at));
            }
        }
        Blend::Subtract => {
            for (at, channel) in under.iter_mut().take(3).enumerate() {
                *channel = channel.saturating_sub(c(at));
            }
        }
    }
    under[3] = 255;
}

/// One block being drawn: its pixels and which were covered by something of the pack.
struct Block {
    scale: usize,
    pixels: Vec<[u8; 4]>,
    covered: Vec<bool>,
}

impl Block {
    fn fill(&mut self, colour: [u8; 4]) {
        self.pixels.fill(colour);
        self.covered.fill(true);
    }

    /// The part of HD `tile` of `entry` covering picture pixel (`x`, `y`).
    fn tile(
        &mut self,
        pack: &HdPack,
        tile: &HdTile,
        entry: &Entry,
        margins: Margins,
        x: usize,
        y: usize,
    ) {
        let Some(picture) = pack.pictures.get(tile.image) else {
            return;
        };
        let picture = picture.get();
        let scale = self.scale;
        let px = (x as i32 - (i32::from(entry.record.x) + margins.0)) as usize;
        let py = (y as i32 - (i32::from(entry.record.y) + margins.1)) as usize;
        let side = 8 * scale;
        for j in 0..scale {
            for i in 0..scale {
                let mut hx = px * scale + i;
                let mut hy = py * scale + j;
                if entry.record.mirror_h {
                    hx = side - 1 - hx;
                }
                if entry.record.mirror_v {
                    hy = side - 1 - hy;
                }
                let colour = picture.get(tile.x + hx as u32, tile.y + hy as u32);
                blend(
                    &mut self.pixels[j * scale + i],
                    colour,
                    tile.brightness,
                    Blend::Alpha,
                );
                self.covered[j * scale + i] |= colour[3] > 0;
            }
        }
    }
}

/// [`Composer::compose`] with buffers of its own (tests, one-off pictures).
pub fn compose(pack: &HdPack, chr: &ChrIndex, input: &FrameInput<'_>, out: &mut Vec<u8>) {
    Composer::default().compose(pack, chr, input, out);
}

/// What drawing a frame keeps from one frame to the next: the backgrounds of each layer drawn
/// over the whole frame.
#[derive(Debug, Default)]
pub struct Composer {
    layers: [Vec<[u8; 4]>; 4],
}

/// The backgrounds `layer` (ascending priority, all alpha-blended) over the whole HD frame
/// (`hd_width` x `hd_height`), premultiplied: each pixel looks from the front-most background
/// back and stops at the first opaque one.
fn composite_layer(
    pack: &HdPack,
    layer: &[usize],
    margins: Margins,
    scale: i32,
    (hd_width, hd_height): (usize, usize),
    buffer: &mut Vec<[u8; 4]>,
) {
    buffer.clear();
    buffer.resize(hd_width * hd_height, [0; 4]);
    let front_first: Vec<_> = layer
        .iter()
        .rev()
        .map(|&at| {
            let background = &pack.backgrounds[at];
            (background, pack.pictures[background.image].get())
        })
        .collect();
    let (left, top) = (margins.0 * scale, margins.1 * scale);
    let _ = hd_height;
    for_each_chunk(buffer, hd_width, &|hy, row| {
        let sy = hy as i32 - top;
        for (hx, pixel) in row.iter_mut().enumerate() {
            let sx = hx as i32 - left;
            let (mut r, mut g, mut b, mut a) = (0u32, 0u32, 0u32, 0u32);
            for (background, picture) in &front_first {
                let py = background.top * scale + sy;
                let px = background.left * scale + sx;
                if px < 0 || py < 0 || px as u32 >= picture.width || py as u32 >= picture.height {
                    continue;
                }
                let at = ((py as u32 * picture.width + px as u32) * 4) as usize;
                let alpha = u32::from(picture.data[at + 3]);
                if alpha == 0 {
                    continue;
                }
                let weight = alpha * (255 - a) / 255;
                let lit_channel = |c: u8| u32::from(lit(c, background.brightness));
                r += lit_channel(picture.data[at]) * weight / 255;
                g += lit_channel(picture.data[at + 1]) * weight / 255;
                b += lit_channel(picture.data[at + 2]) * weight / 255;
                a += weight;
                if a >= 255 {
                    break;
                }
            }
            *pixel = [
                r.min(255) as u8,
                g.min(255) as u8,
                b.min(255) as u8,
                a.min(255) as u8,
            ];
        }
    });
}

impl Composer {
    /// The HD picture of a frame: `width * scale` x `height * scale`, 4 bytes a pixel. A pixel
    /// the overscan leaves out is transparent; a pixel no opaque PPU pixel drew (the backdrop)
    /// is transparent unless something of the pack covers it, as the plain picture is.
    #[allow(clippy::too_many_lines)] // one pass over the picture, layer by layer
    pub fn compose(
        &mut self,
        pack: &HdPack,
        chr: &ChrIndex,
        input: &FrameInput<'_>,
        out: &mut Vec<u8>,
    ) {
        let scale = pack.scale.max(1) as usize;
        let (width, height) = (input.width, input.height);
        let row_bytes = width * scale * 4;
        out.clear();
        out.resize(row_bytes * height * scale, 0);
        if input.picture.len() < width * height {
            return;
        }
        let (entries, owners) = build(pack, chr, input);
        let context = Context {
            pack,
            input,
            owners: &owners,
            entries: &entries,
            chr,
            global: std::cell::RefCell::new(vec![0; pack.conditions.len()]),
        };
        let tiles: Vec<Option<HdTile>> = (0..entries.len()).map(|at| context.tile_of(at)).collect();
        // The background of each priority: the first, in file order, whose conditions hold.
        let mut chosen: [Option<usize>; 40] = [None; 40];
        for (at, background) in pack.backgrounds.iter().enumerate() {
            let slot = usize::from(background.priority.min(39));
            if chosen[slot].is_none() && background.terms.iter().all(|&t| context.holds(t, None)) {
                chosen[slot] = Some(at);
            }
        }
        let layers: [Vec<usize>; 4] = std::array::from_fn(|layer| {
            chosen[layer * 10..layer * 10 + 10]
                .iter()
                .flatten()
                .copied()
                .collect()
        });
        let any_background = layers.iter().any(|layer| !layer.is_empty());
        // A layer with an additive or subtractive background is drawn pixel by pixel over what is
        // under it; the others are drawn once over the whole frame.
        let direct: [bool; 4] = std::array::from_fn(|layer| {
            layers[layer]
                .iter()
                .any(|&at| pack.backgrounds[at].blend != Blend::Alpha)
        });
        let hd_size = (width * scale, height * scale);
        let margins = Margins(input.margin, input.margin_y);
        let (picture, palette, nes_rgb) = (input.picture, input.palette, input.rgb);
        for (layer, buffer) in self.layers.iter_mut().enumerate() {
            if !direct[layer] && !layers[layer].is_empty() {
                composite_layer(pack, &layers[layer], margins, scale as i32, hd_size, buffer);
            }
        }
        let composites = &self.layers;
        let rgb = |index: u8| {
            let [r, g, b] = nes_rgb[usize::from(index & PIXEL_COLOR)];
            [r, g, b, 255]
        };
        let backdrop = rgb(palette[0]);
        let draw_backgrounds = |block: &mut Block, layer: &[usize], x: usize, y: usize| {
            let (sx, sy) = (x as i32 - margins.0, y as i32 - margins.1);
            for &at in layer {
                let background = &pack.backgrounds[at];
                let picture = pack.pictures[background.image].get();
                for j in 0..scale {
                    for i in 0..scale {
                        let hx = (background.left + sx) * scale as i32 + i as i32;
                        let hy = (background.top + sy) * scale as i32 + j as i32;
                        if hx < 0 || hy < 0 {
                            continue;
                        }
                        let colour = picture.get(hx as u32, hy as u32);
                        blend(
                            &mut block.pixels[j * scale + i],
                            colour,
                            background.brightness,
                            background.blend,
                        );
                        block.covered[j * scale + i] |= colour[3] > 0;
                    }
                }
            }
        };
        let apply = |block: &mut Block, layer: usize, x: usize, y: usize| {
            if layers[layer].is_empty() {
                return;
            }
            if direct[layer] {
                draw_backgrounds(block, &layers[layer], x, y);
                return;
            }
            let composite = &composites[layer];
            for j in 0..scale {
                let row = (y * scale + j) * hd_size.0 + x * scale;
                for i in 0..scale {
                    let over = composite[row + i];
                    if over[3] == 0 {
                        continue;
                    }
                    let keep = 255 - u16::from(over[3]);
                    let under = &mut block.pixels[j * scale + i];
                    for (channel, &top) in under.iter_mut().zip(&over).take(3) {
                        *channel =
                            (u16::from(top) + u16::from(*channel) * keep / 255).min(255) as u8;
                    }
                    block.covered[j * scale + i] = true;
                }
            }
        };
        let entries = &entries;
        let owners = &owners;
        let tiles = &tiles;
        // One NES row: `scale` rows of the HD picture.
        let draw_row = |y: usize, out: &mut [u8]| {
            let mut block = Block {
                scale,
                pixels: vec![[0u8; 4]; scale * scale],
                covered: vec![false; scale * scale],
            };
            for x in 0..width {
                let pixel = picture[y * width + x];
                let at = y * width + x;
                if pixel == HIDDEN_PIXEL {
                    continue;
                }
                let actual = pixel & PIXEL_COLOR;
                let opaque = pixel & OPAQUE_PIXEL != 0;
                let background = owners.background[at];
                let all = &owners.sprites[at];
                let sprites = &all[..all.iter().position(|&s| s == NONE).unwrap_or(4)];
                let bg_index = if background == NONE {
                    0
                } else {
                    entries[background as usize].index_at(margins, x, y)
                };
                // The colour the model gives, checked against the picture.
                let front = sprites
                    .iter()
                    .copied()
                    .find(|&s| entries[s as usize].index_at(margins, x, y) != 0);
                let predicted = match front {
                    Some(s) if !entries[s as usize].record.hidden || bg_index == 0 => {
                        let entry = &entries[s as usize];
                        entry.palette[usize::from(entry.index_at(margins, x, y))]
                    }
                    _ if bg_index != 0 => {
                        entries[background as usize].palette[usize::from(bg_index)]
                    }
                    _ => palette[0] & PIXEL_COLOR,
                };
                let involved = any_background
                    || sprites.iter().any(|&s| tiles[s as usize].is_some())
                    || (background != NONE && tiles[background as usize].is_some());
                if predicted != actual || !involved {
                    let colour = rgb(actual);
                    let alpha = if opaque { 255 } else { 0 };
                    fill(
                        out,
                        row_bytes,
                        x,
                        0,
                        scale,
                        [colour[0], colour[1], colour[2], alpha],
                    );
                    continue;
                }
                block.pixels.fill(backdrop);
                block.covered.fill(false);
                apply(&mut block, 0, x, y);
                // A sprite: its HD tile, else its own colour where it has one.
                let draw_sprite = |block: &mut Block, slot: u32| {
                    let entry = &entries[slot as usize];
                    if let Some(tile) = &tiles[slot as usize] {
                        block.tile(pack, tile, entry, margins, x, y);
                    } else {
                        let index = entry.index_at(margins, x, y);
                        if index != 0 {
                            block.fill(rgb(entry.palette[usize::from(index)]));
                        }
                    }
                };
                // The sprites behind the background, the last first; the front-most opaque one
                // hides the other sprites behind it.
                let mut lowest_behind = sprites.len();
                for (k, &slot) in sprites.iter().enumerate().rev() {
                    let entry = &entries[slot as usize];
                    if entry.record.hidden {
                        draw_sprite(&mut block, slot);
                        if entry.index_at(margins, x, y) != 0 {
                            lowest_behind = k;
                        }
                    }
                }
                apply(&mut block, 1, x, y);
                if background != NONE {
                    let entry = &entries[background as usize];
                    match &tiles[background as usize] {
                        Some(tile) => block.tile(pack, tile, entry, margins, x, y),
                        None if bg_index != 0 && !pack.options.disable_original_tiles => {
                            block.fill(rgb(entry.palette[usize::from(bg_index)]));
                        }
                        None => {}
                    }
                }
                apply(&mut block, 2, x, y);
                for (k, &slot) in sprites.iter().enumerate().rev() {
                    if !entries[slot as usize].record.hidden && k < lowest_behind {
                        draw_sprite(&mut block, slot);
                    }
                }
                apply(&mut block, 3, x, y);
                for j in 0..scale {
                    let row = j * row_bytes + x * scale * 4;
                    for i in 0..scale {
                        let mut colour = block.pixels[j * scale + i];
                        colour[3] = if opaque || block.covered[j * scale + i] {
                            255
                        } else {
                            0
                        };
                        out[row + i * 4..row + i * 4 + 4].copy_from_slice(&colour);
                    }
                }
            }
        };
        for_each_chunk(out, row_bytes * scale, &draw_row);
    }
}

/// Fills the `scale` x `scale` block of picture pixel (`x`, `y`).
fn fill(out: &mut [u8], row_bytes: usize, x: usize, y: usize, scale: usize, colour: [u8; 4]) {
    for j in 0..scale {
        let row = (y * scale + j) * row_bytes + x * scale * 4;
        for i in 0..scale {
            out[row + i * 4..row + i * 4 + 4].copy_from_slice(&colour);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::Rgba;
    use crate::pack::Picture;

    fn grey() -> NesRgb {
        let mut rgb = [[0u8; 3]; 64];
        for (at, colour) in rgb.iter_mut().enumerate() {
            let v = (at * 4) as u8;
            *colour = [v, v, v];
        }
        rgb
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().fold(String::new(), |mut hex, b| {
            use std::fmt::Write as _;
            let _ = write!(hex, "{b:02X}");
            hex
        })
    }

    /// A background tile of colour 1 everywhere at (0, 0), and its 8 x 8 picture.
    fn frame() -> (Vec<u8>, Vec<TileRecord>, PaletteSnapshot) {
        let record = TileRecord::new(u64::MAX, 0, 0, 0, 0, false, false, false, 0, true);
        let mut palette = [0u8; 32];
        palette[0] = 0x0F;
        palette[1] = 0x16;
        let picture = vec![0x16 | OPAQUE_PIXEL; 64];
        (picture, vec![record], palette)
    }

    fn run(
        text: &str,
        picture: &[u8],
        records: &[TileRecord],
        palette: PaletteSnapshot,
    ) -> Vec<u8> {
        let pack = crate::pack::parse(text, |name| {
            Ok(Picture::ready(if name.starts_with("bg") {
                Rgba::new(64, 64, [200, 0, 0, 255])
            } else {
                Rgba::new(16, 16, [10, 20, 30, 255])
            }))
        })
        .unwrap_or_default();
        let rgb = grey();
        let memory = |address: u16| if address == 0x10 { 7 } else { 0 };
        let input = FrameInput {
            picture,
            width: 8,
            height: 8,
            margin: 0,
            margin_y: 0,
            records,
            palette,
            frame: 0,
            rgb: &rgb,
            memory: &memory,
        };
        let mut out = Vec::new();
        compose(&pack, &ChrIndex::default(), &input, &mut out);
        out
    }

    #[test]
    fn a_replaced_tile_is_drawn_bigger_and_a_disagreeing_pixel_kept() {
        let (picture, records, palette) = frame();
        let chr = hex(&chr_bytes(&records[0]));
        let text = format!("<ver>105\n<scale>2\n<img>a.png\n<tile>0,{chr},0F160000,0,0\n");
        let out = run(&text, &picture, &records, palette);
        assert_eq!(16 * 16 * 4, out.len());
        assert_eq!(&[10, 20, 30, 255], &out[0..4]);
        let mut other = picture.clone();
        other[0] = 0x20 | OPAQUE_PIXEL;
        let out = run(&text, &other, &records, palette);
        assert_eq!(&[0x80, 0x80, 0x80, 255], &out[0..4]);
        assert_eq!(&[10, 20, 30, 255], &out[8..12]);
    }

    #[test]
    fn a_background_behind_the_tiles_shows_where_the_tile_is_transparent() {
        // A tile of colour 0 (transparent) everywhere: the background shows.
        let (_, mut records, palette) = frame();
        records[0].plane0 = 0;
        let picture = vec![0x0F; 64];
        let text = "<ver>107\n<scale>1\n<condition>on,memoryCheckConstant,10,==,7\n\
                    <condition>off,memoryCheckConstant,10,==,8\n\
                    [off]<background>bg_a.png,1,0,0,5\n[on]<background>bg_b.png,1,0,0,5\n";
        let out = run(text, &picture, &records, palette);
        assert_eq!(&[200, 0, 0, 255], &out[0..4]);
    }
}
