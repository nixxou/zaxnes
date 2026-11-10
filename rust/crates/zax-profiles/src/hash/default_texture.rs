//! The textures of a "3dh" variant (AMD-062 REQ-PROF-083): the default texture of a pattern in
//! the packed form the format stores, and the exclusive-or that turns field 24 back into the
//! texture the variant uses.
//!
//! A 3dn variant keeps its texture as read; a 3dh variant has none, and the texture it uses is
//! **derived** from `textureKind`, field 24 and the pattern's tiles. Nothing caches that result
//! inside the model: the derivation is made where the texture is used, so a tile resolved while
//! the game runs (REQ-PROF-080) changes the texture of its pattern by itself, with no stale copy
//! to rebuild and no reference identity to break (which is what REQ-PROF-084's rewrite needs).

use std::borrow::Cow;
use std::cell::OnceCell;

use crate::enums::TextureFormat;
use crate::pattern::ProfilePattern;
use crate::variant::{HashTextureKind, PatternVariant, TexturePixels, VariantTexture};

/// Pixels per tile, each way.
const TILE_SIZE: i32 = 8;

/// Texels one `data` word of an indexed payload carries (SPEC-03 REQ-PROF-035).
const TEXELS_PER_WORD: usize = 16;

/// The default texture of a pattern, packed as SPEC-03 REQ-PROF-035 stores an indexed texture:
/// `tSize.x * 8` by `tSize.y * 8` pixels, format 0, `extra` exactly as long as `data`.
///
/// This is the `B` of REQ-PROF-083, against which field 24 is exclusive-ored word by word. The
/// colour index of a tile pixel is 0 to 3, so bit 2 of a texel is never set and `extra` stays
/// zero — it is still as long as `data`, which is the length REQ-PROF-083's rule relies on.
#[derive(Debug, Clone)]
pub struct DefaultTexture {
    /// Width in pixels.
    pub width: i32,
    /// Height in pixels.
    pub height: i32,
    /// The packed payload, of format 0.
    pub pixels: TexturePixels,
}

impl DefaultTexture {
    /// Builds the default texture of `pattern`. A cell that is empty, or that holds a tile still
    /// waiting for its planes, leaves its 8 by 8 pixels at 0 (REQ-PROF-081).
    #[must_use]
    pub fn of(pattern: &ProfilePattern) -> Self {
        let (cells_x, cells_y) = grid_size(pattern);
        let width = cells_x.saturating_mul(TILE_SIZE).max(0);
        let height = cells_y.saturating_mul(TILE_SIZE).max(0);
        let texels =
            usize::try_from(u64::try_from(width).unwrap_or(0) * u64::try_from(height).unwrap_or(0))
                .unwrap_or(usize::MAX);
        let mut data = vec![0_i32; texels.div_ceil(TEXELS_PER_WORD)];

        if let Some(grid) = pattern.grid.as_ref() {
            let mut index = 0_usize;
            for y in 0..height {
                for x in 0..width {
                    let color = grid
                        .get(x / TILE_SIZE, y / TILE_SIZE)
                        .filter(|tile| !tile.is_waiting())
                        .and_then(|tile| {
                            tile.color_at(
                                u32::try_from(x % TILE_SIZE).unwrap_or(0),
                                u32::try_from(y % TILE_SIZE).unwrap_or(0),
                            )
                        })
                        .unwrap_or(0);
                    let shift = 2 * (index % TEXELS_PER_WORD);
                    data[index / TEXELS_PER_WORD] |= i32::from(color.to_le_bytes()[0] & 3) << shift;
                    index += 1;
                }
            }
        }

        let words = data.len();
        Self {
            width,
            height,
            pixels: TexturePixels::new(TextureFormat::Indexed, data, vec![0_i32; words]),
        }
    }

    /// The same texture as a variant texture, which is what a variant whose `textureKind` is 2
    /// falls back on while a tile is waiting and field 24 has another shape (REQ-PROF-083).
    #[must_use]
    pub fn to_variant_texture(&self) -> VariantTexture {
        VariantTexture::new(self.width, self.height, Some(self.pixels.clone()))
    }

    /// True when `stored` has the dimensions and the format of this default texture, the
    /// condition REQ-PROF-083 puts on rebuilding field 24 while a tile is still waiting.
    #[must_use]
    pub fn has_shape_of(&self, stored: &VariantTexture) -> bool {
        stored.width == self.width
            && stored.height == self.height
            && stored
                .pixels
                .as_ref()
                .is_some_and(|pixels| pixels.format == self.pixels.format)
    }
}

/// Undoes the exclusive-or of field 24 (REQ-PROF-083, `textureKind = 2`): the result has the
/// dimensions and the format of `stored`, and for every index of `stored.data` the word of the
/// default texture of the same index — or zero past its end — is exclusive-ored in, on 32 bits
/// for `data` and on 16 bits for `extra`. A missing element of `stored.extra` counts as zero.
///
/// The exclusive-or is word against word whatever the formats and the sizes, which is what makes
/// the texture of a variant whose field 24 is of format 1, or of another size, come back exactly.
#[must_use]
pub fn exclusive_or(stored: &VariantTexture, default: &DefaultTexture) -> VariantTexture {
    let Some(source) = stored.pixels.as_ref() else {
        return stored.clone();
    };

    let base = &default.pixels;
    let mut data = Vec::with_capacity(source.data.len());
    let mut extra = Vec::with_capacity(source.data.len());
    for index in 0..source.data.len() {
        let base_data = base.data.get(index).copied().unwrap_or(0);
        data.push(source.data[index] ^ base_data);

        let source_extra = source.extra.get(index).copied().unwrap_or(0);
        // `extra` is a signed 16-bit word, and the base's is only taken where the base has a
        // `data` word; a format-0 default texture has as many of one as of the other.
        let base_extra = if index < base.data.len() {
            base.extra.get(index).copied().unwrap_or(0)
        } else {
            0
        };
        let [low, high, ..] = source_extra.to_le_bytes();
        let [base_low, base_high, ..] = base_extra.to_le_bytes();
        extra.push(i32::from(i16::from_le_bytes([
            low ^ base_low,
            high ^ base_high,
        ])));
    }

    VariantTexture::new(
        stored.width,
        stored.height,
        Some(TexturePixels::new(source.format, data, extra)),
    )
}

/// The textures of one pattern's variants (REQ-PROF-083).
///
/// The default texture is built on first use, so a 3dn pattern — and a 3dh pattern whose
/// variants all use the default texture or none — never pays for it.
#[derive(Debug)]
pub struct PatternTextures<'a> {
    pattern: &'a ProfilePattern,
    default: OnceCell<DefaultTexture>,
    waiting: OnceCell<bool>,
}

impl<'a> PatternTextures<'a> {
    /// The textures of `pattern`.
    #[must_use]
    pub fn of(pattern: &'a ProfilePattern) -> Self {
        Self {
            pattern,
            default: OnceCell::new(),
            waiting: OnceCell::new(),
        }
    }

    /// The default texture of the pattern, built once.
    pub fn default_texture(&self) -> &DefaultTexture {
        self.default
            .get_or_init(|| DefaultTexture::of(self.pattern))
    }

    /// True when at least one cell of the pattern holds a tile still waiting for its planes
    /// (REQ-PROF-081).
    pub fn has_waiting_tile(&self) -> bool {
        *self
            .waiting
            .get_or_init(|| crate::hash::tiles::has_waiting_tile(self.pattern))
    }

    /// The texture `variant` actually uses (REQ-PROF-083). A 3dn variant uses the texture it
    /// carries; a 3dh variant uses no texture, the pattern's default texture — written as the
    /// null marker, which is how SPEC-03 REQ-PROF-035 says "use the default" — or field 24
    /// exclusive-ored with the default texture.
    ///
    /// While a tile of the pattern is waiting and field 24 has other dimensions or another
    /// format than the default texture, the variant uses the default texture itself: the
    /// exclusive-or would be against a texture that is not yet the right one, and the result
    /// would be neither the stored texture nor the default one.
    #[must_use]
    pub fn texture_of<'v>(&self, variant: &'v PatternVariant) -> Option<Cow<'v, VariantTexture>> {
        let Some(kind) = variant.texture_kind else {
            return variant.texture.as_ref().map(Cow::Borrowed);
        };

        match kind {
            HashTextureKind::Absent => None,
            HashTextureKind::Default => Some(Cow::Owned(VariantTexture::null_marker())),
            HashTextureKind::Xor => {
                // A `textureKind` of 2 without field 24 is refused at load (REQ-PROF-074); a
                // variant built in memory without one falls back on the default texture.
                let stored = variant.xor_texture.as_ref()?;
                let default = self.default_texture();
                if self.has_waiting_tile() && !default.has_shape_of(stored) {
                    return Some(Cow::Owned(default.to_variant_texture()));
                }
                Some(Cow::Owned(exclusive_or(stored, default)))
            }
        }
    }
}

fn grid_size(pattern: &ProfilePattern) -> (i32, i32) {
    pattern.grid.as_ref().map_or_else(
        || pattern.tile_size.map_or((0, 0), |size| (size.x, size.y)),
        |grid| (grid.width(), grid.height()),
    )
}
