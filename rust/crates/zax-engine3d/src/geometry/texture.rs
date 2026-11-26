// Portions adapted from 3DNes_Engine (c) Geod Studio, GPL-3.0

//! The texture of a 3D object (SPEC-04B REQ-E3D-426, §2) and the default texture of a pattern
//! (SPEC-03 REQ-PROF-035).

use zax_profiles::enums::TextureFormat;
use zax_profiles::pattern::ProfilePattern;
use zax_profiles::variant::{TexturePixels, VariantTexture};

use crate::palette::Rgb24;
use crate::pattern::EnginePattern;

/// The texture of one 3D object, resolved once at build time: the pattern image by default, or
/// the custom texture the profile carries (SPEC-04B REQ-E3D-426, §2).
///
/// Two formats exist (SPEC-03 REQ-PROF-035). An indexed texture stores a texel value of 0 to
/// 7 — two colour bits plus one extra bit; only the two colour bits reach the palette
/// (REQ-E3D-560), the extra bit serves transparency (REQ-E3D-424) and the multicolour merge
/// (REQ-E3D-503). A direct texture stores a 24-bit colour per texel, used as is, with the
/// integer value 0 standing for the universal background colour *as it was when the texture
/// was loaded* (REQ-E3D-562).
///
/// Every read is taken modulo the texture size (§2, DEC-04B-Q3): a texture smaller than the
/// pattern repeats over it, which four built-in profiles rely on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectTexture {
    width: i32,
    height: i32,
    format: TextureFormat,
    texels: Vec<i32>,
    colors: Option<Vec<Rgb24>>,
}

impl ObjectTexture {
    /// Width in texels (`tw_tex`); at least 1.
    #[must_use]
    pub const fn width(&self) -> i32 {
        self.width
    }

    /// Height in texels (`th_tex`); at least 1.
    #[must_use]
    pub const fn height(&self) -> i32 {
        self.height
    }

    /// Texel format.
    #[must_use]
    pub const fn format(&self) -> TextureFormat {
        self.format
    }

    /// True when the texture carries 24-bit colours instead of palette indices.
    #[must_use]
    pub fn is_direct(&self) -> bool {
        self.format == TextureFormat::Direct
    }

    /// The texel value `t(x, y)`, read modulo the texture size: 0 to 7 for an indexed texture,
    /// the stored integer for a direct one (§2).
    #[must_use]
    pub fn texel(&self, x: i32, y: i32) -> i32 {
        self.texels[self.wrap(x, y)]
    }

    /// The colour of a *direct* texel, the integer value 0 resolved to the universal
    /// background colour captured when the texture was built (REQ-E3D-562). Black for an
    /// indexed texture, whose colours the renderer resolves from the frame palette
    /// (REQ-E3D-560, REQ-E3D-565).
    #[must_use]
    pub fn color(&self, x: i32, y: i32) -> Rgb24 {
        self.colors
            .as_ref()
            .map_or(Rgb24::default(), |colors| colors[self.wrap(x, y)])
    }

    /// The palette column `t mod 4` an indexed texel addresses (REQ-E3D-560). The extra bit
    /// does not reach the colour.
    #[must_use]
    pub const fn palette_column(texel: i32) -> i32 {
        texel & 3
    }

    /// The transparent colour `e` of REQ-E3D-424: `emptyColor` when it is positive or zero,
    /// `emptyColor + 5` when it is negative.
    #[must_use]
    pub const fn empty_color(empty_color: i32) -> i32 {
        if empty_color >= 0 {
            empty_color
        } else {
            empty_color + 5
        }
    }

    /// The pattern image as an indexed texture: `8·tw × 8·th` texels of 0 to 3.
    #[must_use]
    pub fn from_pattern(pattern: &EnginePattern) -> Self {
        let width = (pattern.width() * 8).max(1);
        let height = (pattern.height() * 8).max(1);
        let mut texels = Vec::with_capacity(area(width, height));
        for y in 0..height {
            for x in 0..width {
                texels.push(
                    pattern
                        .image_at(x >> 3, y >> 3)
                        .map_or(0, |image| image.color_at(x & 7, y & 7)),
                );
            }
        }

        Self {
            width,
            height,
            format: TextureFormat::Indexed,
            texels,
            colors: None,
        }
    }

    /// The texture of a 3D object: its custom texture when the profile gives one of non-zero
    /// size, the pattern image otherwise (REQ-E3D-426). `background` is the universal
    /// background colour used for the value-0 texels of a direct texture, read once here
    /// (REQ-E3D-562).
    #[must_use]
    pub fn for_object(
        pattern: &EnginePattern,
        texture: Option<&VariantTexture>,
        background: Rgb24,
    ) -> Self {
        let Some((texture, pixels)) = texture
            .filter(|texture| !texture.is_null_marker() && texture.width > 0 && texture.height > 0)
            .and_then(|texture| texture.pixels.as_ref().map(|pixels| (texture, pixels)))
        else {
            return Self::from_pattern(pattern);
        };

        let count = area(texture.width, texture.height);
        if pixels.format == TextureFormat::Direct {
            let texels: Vec<i32> = (0..count)
                .map(|n| pixels.data.get(n).copied().unwrap_or(0))
                .collect();
            let colors = texels
                .iter()
                .map(|&value| {
                    if value == 0 {
                        background
                    } else {
                        Rgb24::from_packed(value & 0xFF_FFFF)
                    }
                })
                .collect();
            return Self {
                width: texture.width,
                height: texture.height,
                format: TextureFormat::Direct,
                texels,
                colors: Some(colors),
            };
        }

        Self {
            width: texture.width,
            height: texture.height,
            format: TextureFormat::Indexed,
            texels: (0..count).map(|n| read_indexed_texel(pixels, n)).collect(),
            colors: None,
        }
    }

    #[allow(clippy::cast_sign_loss)] // rem_euclid by a positive size is never negative.
    fn wrap(&self, x: i32, y: i32) -> usize {
        let tx = x.rem_euclid(self.width);
        let ty = y.rem_euclid(self.height);
        (tx + (ty * self.width)) as usize
    }
}

/// `width × height` as a length; both are positive.
#[allow(clippy::cast_sign_loss)] // Callers pass positive sizes.
const fn area(width: i32, height: i32) -> usize {
    (width * height) as usize
}

/// Builds the default texture of a pattern: one texel per pixel of the pattern image, the
/// colour index 0 to 3 of the tile of the cell, and 0 where the cell is empty
/// (SPEC-03 REQ-PROF-035, SPEC-04B REQ-E3D-426).
///
/// It belongs here because the engine owns the tile images and the palettes (DEC-04B-Q1). The
/// profile writer needs it for the deferred rule of REQ-PROF-052: a variant whose texture is
/// identical to the default texture is written as the null marker.
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // Colours are 0 to 3.
pub fn default_texture(pattern: &EnginePattern) -> Vec<u8> {
    let width = pattern.width() * 8;
    let height = pattern.height() * 8;
    let mut texels = Vec::with_capacity(area(width, height));
    for y in 0..height {
        for x in 0..width {
            texels.push(
                pattern
                    .image_at(x >> 3, y >> 3)
                    .map_or(0, |image| image.color_at(x & 7, y & 7) as u8),
            );
        }
    }

    texels
}

/// Builds the default texture of a profile pattern, without going through the registry.
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // Colours are 0 to 3.
pub fn default_profile_texture(pattern: &ProfilePattern) -> Vec<u8> {
    let (cells_x, cells_y) = profile_grid_size(pattern);
    let width = (cells_x * 8).max(0);
    let height = (cells_y * 8).max(0);
    let mut texels = vec![0; area(width, height)];
    let Some(grid) = pattern.grid.as_ref() else {
        return texels;
    };

    for y in 0..height {
        for x in 0..width {
            if let Some(color) = grid
                .get(x >> 3, y >> 3)
                .and_then(|tile| tile.color_at((x & 7) as u32, (y & 7) as u32))
            {
                texels[(x + (y * width)) as usize] = color as u8;
            }
        }
    }

    texels
}

fn profile_grid_size(pattern: &ProfilePattern) -> (i32, i32) {
    pattern.grid.as_ref().map_or_else(
        || pattern.tile_size.map_or((0, 0), |size| (size.x, size.y)),
        |grid| (grid.width(), grid.height()),
    )
}

/// Whether `texture` holds exactly the default texture of `pattern`, which is what
/// REQ-PROF-052 replaces by the null marker. An absent texture, or the null marker itself,
/// already means "use the default", so it answers true as well.
#[must_use]
pub fn is_default_texture(texture: Option<&VariantTexture>, pattern: &ProfilePattern) -> bool {
    let Some(texture) = texture.filter(|texture| !texture.is_null_marker()) else {
        return true;
    };

    let (cells_x, cells_y) = profile_grid_size(pattern);
    if texture.width != cells_x * 8 || texture.height != cells_y * 8 {
        return false;
    }

    let Some(pixels) = texture
        .pixels
        .as_ref()
        .filter(|pixels| pixels.format == TextureFormat::Indexed)
    else {
        return false;
    };

    default_profile_texture(pattern)
        .iter()
        .enumerate()
        .all(|(n, &expected)| read_indexed_texel(pixels, n) == i32::from(expected))
}

/// Reads texel `n` of an indexed payload (SPEC-03 REQ-PROF-035): bits `2(n mod 16)` and
/// `2(n mod 16) + 1` of `data[n / 16]`, plus bit `n mod 16` of `extra[n / 16]` as bit 2.
#[must_use]
pub fn read_indexed_texel(pixels: &TexturePixels, n: usize) -> i32 {
    let word = n / 16;
    let shift = 2 * (n % 16);
    let low = pixels.data.get(word).map_or(0, |&data| (data >> shift) & 3);
    let high = pixels
        .extra
        .get(word)
        .map_or(0, |&extra| (extra >> (n % 16)) & 1);
    low | (high << 2)
}
