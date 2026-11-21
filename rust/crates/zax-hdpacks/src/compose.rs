//! Drawing one HD frame.
//!
//! Which tile drew each pixel comes from the core's tile records: the background tile covering
//! it and up to four sprites, by OAM order. That model is checked against the real picture: where
//! it does not give the colour the PPU shows (the sprite limit, a palette or scroll changed in the
//! middle of the frame), the original pixel is kept, enlarged. Mesen records the owner of each
//! pixel inside its PPU; this is the approximation of it the records allow.
//!
//! The rule of a tile is chosen once per record (Mesen evaluates conditions per pixel; the result
//! is the same for the usual tile-aligned positions).

// Screen geometry: positions of a few hundred pixels, checked before each cast.
#![allow(
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation
)]

use std::collections::HashMap;

use zax_core_api::{PaletteSnapshot, TileRecord};

use crate::pack::{Condition, HdPack, HdTile, Rule, Term, TileData, TileKey};
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
            #[allow(clippy::cast_possible_truncation)] // a CHR ROM is at most a few MB
            indices.entry(bytes).or_default().push(at as u32);
        }
        Self { indices }
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

/// The colour index (0 to 3) a record shows at (`px`, `py`) of its 8 x 8, flips applied.
fn shown(record: &TileRecord, px: usize, py: usize) -> u8 {
    let column = if record.mirror_h { 7 - px } else { px };
    let row = if record.mirror_v { 7 - py } else { py };
    record.source_pixel(column, row).unwrap_or(0)
}

/// What each pixel of the screen area is made of: the background record covering it and up to
/// four sprite records (OAM order).
struct Owners {
    width: usize,
    height: usize,
    background: Vec<u32>,
    sprites: Vec<[u32; 4]>,
}

const NONE: u32 = u32::MAX;

impl Owners {
    fn new(input: &FrameInput<'_>) -> Self {
        let (width, height) = (input.width, input.height);
        let mut owners = Self {
            width,
            height,
            background: vec![NONE; width * height],
            sprites: vec![[NONE; 4]; width * height],
        };
        let mut sprites: Vec<(i32, u32)> = Vec::new();
        for (at, record) in input.records.iter().enumerate() {
            #[allow(clippy::cast_possible_truncation)] // a frame has a few thousand records
            let at = at as u32;
            if record.background {
                owners.cover(input, record, |owners, pixel| owners.background[pixel] = at);
            } else {
                sprites.push((record.index, at));
            }
        }
        // OAM order: the first sprite is in front.
        sprites.sort_by_key(|&(index, at)| (index, at));
        for (_, at) in sprites {
            let record = &input.records[at as usize];
            owners.cover(input, record, |owners, pixel| {
                if let Some(slot) = owners.sprites[pixel].iter_mut().find(|s| **s == NONE) {
                    *slot = at;
                }
            });
        }
        owners
    }

    /// Calls `mark` for each picture pixel of `record`'s 8 x 8.
    fn cover(
        &mut self,
        input: &FrameInput<'_>,
        record: &TileRecord,
        mut mark: impl FnMut(&mut Self, usize),
    ) {
        let left = i32::from(record.x) + input.margin;
        let top = i32::from(record.y) + input.margin_y;
        for py in 0..8 {
            let y = top + py;
            if y < 0 || y as usize >= self.height {
                continue;
            }
            for px in 0..8 {
                let x = left + px;
                if x < 0 || x as usize >= self.width {
                    continue;
                }
                #[allow(clippy::cast_sign_loss)] // checked above
                mark(self, y as usize * self.width + x as usize);
            }
        }
    }

    /// The record covering screen pixel (`x`, `y`) on the background, if any.
    fn background_at(&self, input: &FrameInput<'_>, x: i32, y: i32) -> Option<u32> {
        let (px, py) = (x + input.margin, y + input.margin_y);
        if px < 0 || py < 0 || px as usize >= self.width || py as usize >= self.height {
            return None;
        }
        #[allow(clippy::cast_sign_loss)] // checked above
        let at = self.background[py as usize * self.width + px as usize];
        (at != NONE).then_some(at)
    }

    /// The sprites covering screen pixel (`x`, `y`).
    fn sprites_at(&self, input: &FrameInput<'_>, x: i32, y: i32) -> [u32; 4] {
        let (px, py) = (x + input.margin, y + input.margin_y);
        if px < 0 || py < 0 || px as usize >= self.width || py as usize >= self.height {
            return [NONE; 4];
        }
        #[allow(clippy::cast_sign_loss)] // checked above
        self.sprites[py as usize * self.width + px as usize]
    }
}

/// What a record is, as a pack looks it up.
struct Keyed {
    bytes: [u8; 16],
    palette: [u8; 4],
}

/// True when the record `keyed` is the tile `key` names.
fn same_tile(keyed: &Keyed, key: &TileKey, chr: &ChrIndex) -> bool {
    if keyed.palette != key.palette {
        return false;
    }
    match key.data {
        TileData::Bytes(bytes) => bytes == keyed.bytes,
        TileData::Index(index) => chr
            .indices
            .get(&keyed.bytes)
            .is_some_and(|list| list.contains(&index)),
    }
}

struct Context<'a, 'b> {
    pack: &'a HdPack,
    input: &'a FrameInput<'b>,
    owners: &'a Owners,
    keyed: &'a [Keyed],
    chr: &'a ChrIndex,
}

impl Context<'_, '_> {
    fn holds(&self, term: Term, record: &TileRecord) -> bool {
        let value = match &self.pack.conditions[term.condition] {
            Condition::HMirror => !record.background && record.mirror_h,
            Condition::VMirror => !record.background && record.mirror_v,
            Condition::BgPriority => !record.background && record.hidden,
            Condition::TileAtPosition { x, y, key } => self
                .owners
                .background_at(self.input, *x, *y)
                .is_some_and(|at| same_tile(&self.keyed[at as usize], key, self.chr)),
            Condition::TileNearby { x, y, key } => self
                .owners
                .background_at(self.input, i32::from(record.x) + x, i32::from(record.y) + y)
                .is_some_and(|at| same_tile(&self.keyed[at as usize], key, self.chr)),
            Condition::SpriteAtPosition { x, y, key } => self.sprite_is(*x, *y, key),
            Condition::SpriteNearby { x, y, key } => {
                self.sprite_is(i32::from(record.x) + x, i32::from(record.y) + y, key)
            }
            Condition::MemoryCheck {
                left,
                operator,
                right,
                mask,
            } => operator.holds(
                (self.input.memory)(*left) & mask,
                (self.input.memory)(*right) & mask,
            ),
            Condition::MemoryConstant {
                address,
                operator,
                value,
                mask,
            } => operator.holds((self.input.memory)(*address) & mask, *value),
            Condition::FrameRange { divisor, compare } => {
                self.input.frame % u64::from(*divisor) >= u64::from(*compare)
            }
            Condition::Unsupported => false,
        };
        value != term.negated
    }

    fn sprite_is(&self, x: i32, y: i32, key: &TileKey) -> bool {
        self.owners
            .sprites_at(self.input, x, y)
            .iter()
            .filter(|&&at| at != NONE)
            .any(|&at| same_tile(&self.keyed[at as usize], key, self.chr))
    }

    fn first<'r>(&self, rules: Option<&'r Vec<Rule>>, record: &TileRecord) -> Option<&'r HdTile> {
        rules?
            .iter()
            .find(|rule| rule.terms.iter().all(|&term| self.holds(term, record)))
            .map(|rule| &rule.tile)
    }

    /// The HD tile the pack gives `record`, if any.
    fn tile_of(&self, at: usize) -> Option<HdTile> {
        let record = &self.input.records[at];
        let keyed = &self.keyed[at];
        let mut candidates: Vec<TileData> = Vec::with_capacity(2);
        if let Some(list) = self.chr.indices.get(&keyed.bytes) {
            candidates.extend(list.iter().map(|&index| TileData::Index(index)));
        }
        candidates.push(TileData::Bytes(keyed.bytes));
        for &data in &candidates {
            let key = TileKey {
                data,
                palette: keyed.palette,
            };
            if let Some(tile) = self.first(self.pack.rules.get(&key), record) {
                return Some(*tile);
            }
        }
        candidates
            .iter()
            .find_map(|data| self.first(self.pack.defaults.get(data), record))
            .copied()
    }
}

/// The HD picture of a frame: `width * scale` x `height * scale`, 4 bytes a pixel. A pixel the
/// overscan leaves out is transparent; a pixel no opaque PPU pixel drew (the backdrop) is
/// transparent unless something of the pack covers it, as the plain picture is.
#[allow(clippy::too_many_lines)] // one pass over the picture, layer by layer
pub fn compose(pack: &HdPack, chr: &ChrIndex, input: &FrameInput<'_>, out: &mut Vec<u8>) {
    let scale = pack.scale.max(1) as usize;
    let (width, height) = (input.width, input.height);
    let row_bytes = width * scale * 4;
    out.clear();
    out.resize(row_bytes * height * scale, 0);
    if input.picture.len() < width * height {
        return;
    }
    let owners = Owners::new(input);
    let keyed: Vec<Keyed> = input
        .records
        .iter()
        .map(|record| Keyed {
            bytes: chr_bytes(record),
            palette: palette_of(record, &input.palette),
        })
        .collect();
    let context = Context {
        pack,
        input,
        owners: &owners,
        keyed: &keyed,
        chr,
    };
    let tiles: Vec<Option<HdTile>> = (0..input.records.len())
        .map(|at| context.tile_of(at))
        .collect();
    let backgrounds: Vec<usize> = pack
        .backgrounds
        .iter()
        .enumerate()
        .filter(|(_, background)| {
            // A background's conditions on a record make no sense; only the global ones count.
            let none = TileRecord::default();
            background
                .terms
                .iter()
                .all(|&term| context.holds(term, &none))
        })
        .map(|(at, _)| at)
        .take(1)
        .collect();
    let rgb = |index: u8| {
        let [r, g, b] = input.rgb[usize::from(index & PIXEL_COLOR)];
        [r, g, b, 255]
    };
    let backdrop = rgb(input.palette[0]);
    let mut block = vec![[0u8; 4]; scale * scale];
    let mut covered = vec![false; scale * scale];
    for y in 0..height {
        for x in 0..width {
            let pixel = input.picture[y * width + x];
            let at = y * width + x;
            if pixel == HIDDEN_PIXEL {
                continue;
            }
            let actual = pixel & PIXEL_COLOR;
            let opaque = pixel & OPAQUE_PIXEL != 0;
            let background = owners.background[at];
            let sprites = owners.sprites[at];
            // The colour index the model gives, to check it against the picture.
            let bg_index = (background != NONE).then(|| {
                let record = &input.records[background as usize];
                let left = i32::from(record.x) + input.margin;
                let top = i32::from(record.y) + input.margin_y;
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // in the tile
                shown(
                    record,
                    (x as i32 - left) as usize,
                    (y as i32 - top) as usize,
                )
            });
            let sprite_index = |slot: u32| {
                let record = &input.records[slot as usize];
                let left = i32::from(record.x) + input.margin;
                let top = i32::from(record.y) + input.margin_y;
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // in the tile
                shown(
                    record,
                    (x as i32 - left) as usize,
                    (y as i32 - top) as usize,
                )
            };
            let front = sprites
                .iter()
                .copied()
                .take_while(|&s| s != NONE)
                .find(|&s| sprite_index(s) != 0);
            let bg_opaque = bg_index.is_some_and(|i| i != 0);
            let predicted = match front {
                Some(s) if !input.records[s as usize].hidden || !bg_opaque => {
                    keyed[s as usize].palette[usize::from(sprite_index(s))]
                }
                _ if bg_opaque => {
                    keyed[background as usize].palette[usize::from(bg_index.unwrap_or(0))]
                }
                _ => input.palette[0] & 0x3F,
            };
            let trusted = predicted == actual
                && (sprites
                    .iter()
                    .any(|&s| s != NONE && tiles[s as usize].is_some())
                    || (background != NONE && tiles[background as usize].is_some())
                    || !backgrounds.is_empty());
            if !trusted {
                let colour = rgb(actual);
                let alpha = if opaque { 255 } else { 0 };
                fill(
                    out,
                    row_bytes,
                    x,
                    y,
                    scale,
                    [colour[0], colour[1], colour[2], alpha],
                );
                continue;
            }
            // The block, from the back.
            block.fill(backdrop);
            covered.fill(false);
            for &index in &backgrounds {
                let background = &pack.backgrounds[index];
                let picture = &pack.images[background.image];
                for j in 0..scale {
                    for i in 0..scale {
                        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
                        let (px, py) = (
                            ((x as i32 - input.margin) * scale as i32 + i as i32 + background.left)
                                .max(0),
                            ((y as i32 - input.margin_y) * scale as i32
                                + j as i32
                                + background.top)
                                .max(0),
                        );
                        #[allow(clippy::cast_sign_loss)] // clamped
                        let colour = picture.get(px as u32, py as u32);
                        blend(&mut block[j * scale + i], colour, background.brightness);
                        covered[j * scale + i] |= colour[3] > 0;
                    }
                }
            }
            let draw_sprite = |slot: u32, block: &mut [[u8; 4]], covered: &mut [bool]| {
                let record = &input.records[slot as usize];
                let index = sprite_index(slot);
                match tiles[slot as usize] {
                    Some(tile) => {
                        draw_hd(pack, &tile, record, input, x, y, scale, block, covered);
                    }
                    None if index != 0 && !pack.options.disable_original_tiles => {
                        let colour = rgb(keyed[slot as usize].palette[usize::from(index)]);
                        block.fill(colour);
                        covered.fill(true);
                    }
                    None => {}
                }
            };
            let live: Vec<u32> = sprites.iter().copied().take_while(|&s| s != NONE).collect();
            // Sprites behind the background, the last in OAM order first.
            for &slot in live.iter().rev() {
                if input.records[slot as usize].hidden {
                    draw_sprite(slot, &mut block, &mut covered);
                }
            }
            if background != NONE {
                let record = &input.records[background as usize];
                match tiles[background as usize] {
                    Some(tile) => {
                        draw_hd(
                            pack,
                            &tile,
                            record,
                            input,
                            x,
                            y,
                            scale,
                            &mut block,
                            &mut covered,
                        );
                    }
                    None if bg_opaque && !pack.options.disable_original_tiles => {
                        let index = bg_index.unwrap_or(0);
                        block.fill(rgb(keyed[background as usize].palette[usize::from(index)]));
                        covered.fill(true);
                    }
                    None => {}
                }
            }
            for &slot in live.iter().rev() {
                if !input.records[slot as usize].hidden {
                    draw_sprite(slot, &mut block, &mut covered);
                }
            }
            for j in 0..scale {
                let row = (y * scale + j) * row_bytes + x * scale * 4;
                for i in 0..scale {
                    let mut colour = block[j * scale + i];
                    colour[3] = if opaque || covered[j * scale + i] {
                        255
                    } else {
                        0
                    };
                    out[row + i * 4..row + i * 4 + 4].copy_from_slice(&colour);
                }
            }
        }
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

/// `colour` over `under`, its brightness applied.
fn blend(under: &mut [u8; 4], colour: [u8; 4], brightness: f32) {
    if colour[3] == 0 {
        return;
    }
    let light = |value: u8| {
        if (brightness - 1.0).abs() < f32::EPSILON {
            value
        } else {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // clamped
            let lit = (f32::from(value) * brightness).clamp(0.0, 255.0) as u8;
            lit
        }
    };
    let alpha = u16::from(colour[3]);
    for c in 0..3 {
        #[allow(clippy::cast_possible_truncation)] // a weighted mean of bytes
        let mixed = ((u16::from(light(colour[c])) * alpha + u16::from(under[c]) * (255 - alpha))
            / 255) as u8;
        under[c] = mixed;
    }
    under[3] = 255;
}

/// Draws the part of HD `tile` (of `record`) that covers picture pixel (`x`, `y`) into `block`.
#[allow(clippy::too_many_arguments)] // the tile, where it is and the block
fn draw_hd(
    pack: &HdPack,
    tile: &HdTile,
    record: &TileRecord,
    input: &FrameInput<'_>,
    x: usize,
    y: usize,
    scale: usize,
    block: &mut [[u8; 4]],
    covered: &mut [bool],
) {
    let Some(picture) = pack.images.get(tile.image) else {
        return;
    };
    let left = i32::from(record.x) + input.margin;
    let top = i32::from(record.y) + input.margin_y;
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss
    )]
    let (px, py) = ((x as i32 - left) as usize, (y as i32 - top) as usize);
    let side = 8 * scale;
    for j in 0..scale {
        for i in 0..scale {
            let mut hx = px * scale + i;
            let mut hy = py * scale + j;
            if record.mirror_h {
                hx = side - 1 - hx;
            }
            if record.mirror_v {
                hy = side - 1 - hy;
            }
            #[allow(clippy::cast_possible_truncation)] // within a picture
            let colour = picture.get(tile.x + hx as u32, tile.y + hy as u32);
            blend(&mut block[j * scale + i], colour, tile.brightness);
            covered[j * scale + i] |= colour[3] > 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::Rgba;

    fn grey() -> NesRgb {
        let mut rgb = [[0u8; 3]; 64];
        for (at, colour) in rgb.iter_mut().enumerate() {
            #[allow(clippy::cast_possible_truncation)]
            let v = (at * 4) as u8;
            *colour = [v, v, v];
        }
        rgb
    }

    /// A background tile of colour 1 everywhere at (0, 0), and a picture of 8 x 8.
    fn frame() -> (Vec<u8>, Vec<TileRecord>, PaletteSnapshot) {
        let record = TileRecord::new(u64::MAX, 0, 0, 0, 0, false, false, false, 0, true);
        let mut palette = [0u8; 32];
        palette[0] = 0x0F;
        palette[1] = 0x16;
        let picture = vec![0x16 | OPAQUE_PIXEL; 64];
        (picture, vec![record], palette)
    }

    #[test]
    fn a_replaced_tile_is_drawn_bigger_and_the_rest_kept() {
        let (picture, records, palette) = frame();
        let chr = [
            0xFFu8, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let hex = chr.iter().fold(String::new(), |mut hex, b| {
            use std::fmt::Write as _;
            let _ = write!(hex, "{b:02X}");
            hex
        });
        let text = format!("<ver>105\n<scale>2\n<img>a.png\n<tile>0,{hex},0F160000,0,0\n");
        let pack = crate::pack::parse(&text, |_| Ok(Rgba::new(16, 16, [10, 20, 30, 255])))
            .unwrap_or_default();
        let rgb = grey();
        let memory = |_: u16| 0u8;
        let input = FrameInput {
            picture: &picture,
            width: 8,
            height: 8,
            margin: 0,
            margin_y: 0,
            records: &records,
            palette,
            frame: 0,
            rgb: &rgb,
            memory: &memory,
        };
        let mut out = Vec::new();
        compose(&pack, &ChrIndex::default(), &input, &mut out);
        assert_eq!(16 * 16 * 4, out.len());
        assert_eq!(&[10, 20, 30, 255], &out[0..4]);
        // A picture that disagrees with the records keeps its own pixel.
        let mut other = picture.clone();
        other[0] = 0x20 | OPAQUE_PIXEL;
        let input = FrameInput {
            picture: &other,
            ..input
        };
        compose(&pack, &ChrIndex::default(), &input, &mut out);
        assert_eq!(&[0x80, 0x80, 0x80, 255], &out[0..4]);
        assert_eq!(&[10, 20, 30, 255], &out[8..12]);
    }
}
