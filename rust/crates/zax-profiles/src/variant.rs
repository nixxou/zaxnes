//! A 3D variant of a pattern and its explicit texture (SPEC-03 §4.4, REQ-PROF-033 to
//! REQ-PROF-037).

use crate::enums::{
    MeshAlgorithm, ProfileLayer, TextureFormat, VariantGeometry, VariantRenderStyle,
};
use crate::layout::FieldMask;
use crate::tags;
use crate::values::{Mat4F, Vec3F};

/// The pixel words of an explicit variant texture (REQ-PROF-035).
#[derive(Debug, Clone)]
pub struct TexturePixels {
    /// Pixel format.
    pub format: TextureFormat,
    /// The `data` words.
    pub data: Vec<i32>,
    /// The `extra` words (the third bit of an indexed pixel).
    pub extra: Vec<i32>,
    /// The fields the file carried, when they differ from the schema (Q-010).
    pub recorded_fields: Option<FieldMask>,
}

impl TexturePixels {
    /// Creates a payload of `format` with the given words.
    #[must_use]
    pub fn new(format: TextureFormat, data: Vec<i32>, extra: Vec<i32>) -> Self {
        Self {
            format,
            data,
            extra,
            recorded_fields: None,
        }
    }

    /// True when `extra` holds no non-zero word.
    #[must_use]
    pub fn extra_is_empty(&self) -> bool {
        self.extra.iter().all(|word| *word == 0)
    }

    /// Number of `data` words a texture of `width x height` pixels needs: one per pixel for a
    /// direct texture, one per sixteen pixels (rounded up) for an indexed one.
    #[must_use]
    pub fn expected_word_count(&self, width: i32, height: i32) -> usize {
        let pixels = i64::from(width) * i64::from(height);
        if pixels <= 0 {
            return 0;
        }
        let words = if self.format == TextureFormat::Direct {
            pixels
        } else {
            (pixels + 15) / 16
        };
        usize::try_from(words).unwrap_or(usize::MAX)
    }
}

/// An explicit texture of a variant (SPEC-03 REQ-PROF-035).
#[derive(Debug, Clone)]
pub struct VariantTexture {
    /// Width in pixels.
    pub width: i32,
    /// Height in pixels.
    pub height: i32,
    /// The pixel words, or none when the payload is absent.
    pub pixels: Option<TexturePixels>,
    /// The fields the file carried, when they differ from the schema (Q-010).
    pub recorded_fields: Option<FieldMask>,
}

impl VariantTexture {
    /// Creates a texture.
    #[must_use]
    pub fn new(width: i32, height: i32, pixels: Option<TexturePixels>) -> Self {
        Self {
            width,
            height,
            pixels,
            recorded_fields: None,
        }
    }

    /// The "no texture" marker the built-in profiles use: 0 by 0 with an empty indexed payload.
    #[must_use]
    pub fn null_marker() -> Self {
        Self::new(0, 0, Some(TexturePixels::new(TextureFormat::Indexed, Vec::new(), Vec::new())))
    }

    /// True for the "no texture" marker.
    #[must_use]
    pub fn is_null_marker(&self) -> bool {
        self.width == 0
            && self.height == 0
            && self
                .pixels
                .as_ref()
                .is_none_or(|pixels| pixels.format == TextureFormat::Indexed && pixels.data.is_empty())
    }

    /// True when the payload has the length the size and format call for.
    #[must_use]
    pub fn has_consistent_length(&self) -> bool {
        if self.is_null_marker() {
            return true;
        }
        let Some(pixels) = &self.pixels else {
            return self.width == 0 && self.height == 0;
        };
        let expected = pixels.expected_word_count(self.width, self.height);
        pixels.data.len() == expected && (pixels.extra.is_empty() || pixels.extra.len() == expected)
    }
}

/// One 3D variant of a pattern (SPEC-03 §4.4).
#[derive(Debug, Clone, Default)]
#[allow(clippy::struct_excessive_bools)] // one flag per stored field of the message
pub struct PatternVariant {
    /// Depth in voxels (`sizeZ`).
    pub depth: i32,
    /// Geometry family.
    pub geometry: VariantGeometry,
    /// `antiShadow`.
    pub anti_shadow: bool,
    /// `flip`.
    pub flip: bool,
    /// `solid`.
    pub solid: bool,
    /// `fullWarp`.
    pub full_warp: bool,
    /// Depth layer.
    pub layer: ProfileLayer,
    /// Scale, when stored.
    pub scale: Option<Vec3F>,
    /// Rotation, when stored.
    pub rotation: Option<Vec3F>,
    /// Offset, when stored.
    pub offset: Option<Vec3F>,
    /// Pivot, when stored.
    pub pivot: Option<Vec3F>,
    /// Alpha.
    pub alpha: f32,
    /// Mesh algorithm override.
    pub mesh_algorithm: MeshAlgorithm,
    /// Deformation amplitude, when stored.
    pub deform_amplitude: Option<Vec3F>,
    /// Deformation pivot, when stored.
    pub deform_pivot: Option<Vec3F>,
    /// Deformation speed matrix, when stored.
    pub deform_speed: Option<Mat4F>,
    /// Deformation offset matrix, when stored.
    pub deform_offset: Option<Mat4F>,
    /// How the variant is drawn.
    pub render_style: VariantRenderStyle,
    /// Explicit texture, when stored.
    pub texture: Option<VariantTexture>,
    /// Raw tag string, or none when the field is absent.
    pub tag_string: Option<String>,
    /// The empty colour as stored (`emptyColor`, a signed byte); a negative value is brought
    /// back into 0 to 4 by [`Self::empty_color`] (REQ-PROF-037).
    pub stored_empty_color: i32,
    /// The fields the file carried, when they differ from the schema (Q-010).
    pub recorded_fields: Option<FieldMask>,
}

impl PatternVariant {
    /// The empty colour, 0 to 4: a negative stored value counts from 5 (REQ-PROF-037).
    #[must_use]
    pub fn empty_color(&self) -> i32 {
        if self.stored_empty_color < 0 {
            self.stored_empty_color + 5
        } else {
            self.stored_empty_color
        }
    }

    /// Tags of this variant, split and trimmed (REQ-PROF-036).
    #[must_use]
    pub fn tags(&self) -> Vec<&str> {
        tags::parse(self.tag_string.as_deref())
    }

    /// True when the variant carries exactly `tag` (case-sensitive).
    #[must_use]
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags().contains(&tag)
    }

    /// Custom data `index` (0 to 7, REQ-PROF-034): the last row of the deformation speed matrix
    /// for 0 to 3, of the deformation offset matrix for 4 to 7; zero when the matrix is absent.
    /// None when `index` is outside 0..7.
    #[must_use]
    pub fn custom_data(&self, index: usize) -> Option<f32> {
        if index > 7 {
            return None;
        }
        let matrix = if index < 4 {
            &self.deform_speed
        } else {
            &self.deform_offset
        };
        Some(matrix.as_ref().map_or(0.0, |matrix| {
            matrix.get(3, index & 3).unwrap_or(0.0)
        }))
    }
}
