//! A pattern of a profile and the memorised pattern slots of the embedded settings
//! (SPEC-03 §4.3, REQ-PROF-044).

use std::sync::Arc;

use crate::enums::{PatternColorFlags, PatternDensity};
use crate::layout::FieldMask;
use crate::tile::PatternGrid;
use crate::values::Vec2I;
use crate::variant::PatternVariant;

/// One pattern of a profile (SPEC-03 §4.3).
#[derive(Debug, Clone)]
pub struct ProfilePattern {
    /// The tile grid, or none when the pattern has no cells.
    pub grid: Option<PatternGrid>,
    /// Constraint size (`cSize`); 99 by 99 means unconstrained.
    pub constraint_size: Vec2I,
    /// Size in tiles (`tSize`), when stored.
    pub tile_size: Option<Vec2I>,
    /// `permanent`: whether the pattern belongs to the permanent family.
    pub permanent: bool,
    /// Pixel count as stored.
    pub pixel_count: i32,
    /// Colours present, as stored.
    pub color_flags: PatternColorFlags,
    /// Density in tiles, as stored.
    pub tile_density: PatternDensity,
    /// Density in pixels, as stored.
    pub pixel_density: PatternDensity,
    /// The variants; a variant may be shared between patterns (REQ-PROF-011).
    pub variants: Vec<Arc<PatternVariant>>,
    /// `selfConnected`.
    pub self_connected: bool,
    /// `skipGeo`.
    pub skip_geometry: bool,
    /// The fields the file carried, when they differ from the schema (Q-010).
    pub recorded_fields: Option<FieldMask>,
}

impl Default for ProfilePattern {
    fn default() -> Self {
        Self {
            grid: None,
            constraint_size: Vec2I::UNCONSTRAINED,
            tile_size: None,
            permanent: false,
            pixel_count: 0,
            color_flags: PatternColorFlags::NONE,
            tile_density: PatternDensity::Empty,
            pixel_density: PatternDensity::Empty,
            variants: Vec::new(),
            self_connected: true,
            skip_geometry: false,
            recorded_fields: None,
        }
    }
}

impl ProfilePattern {
    /// The first variant, which carries the default 3D look, or none.
    #[must_use]
    pub fn main_variant(&self) -> Option<&Arc<PatternVariant>> {
        self.variants.first()
    }
}

/// A memorised pattern slot of the editor (`SavedPattern3D`, `SaveAnimPattern3D`): a name and a
/// pattern held directly, without a reference envelope.
#[derive(Debug, Clone, Default)]
pub struct RecordedPatternSlot {
    /// Slot name.
    pub name: Option<String>,
    /// The memorised pattern, or none.
    pub pattern: Option<ProfilePattern>,
    /// The fields the file carried, when they differ from the schema (Q-010).
    pub recorded_fields: Option<FieldMask>,
}

impl RecordedPatternSlot {
    /// Creates a slot.
    #[must_use]
    pub fn new(name: Option<&str>, pattern: Option<ProfilePattern>) -> Self {
        Self {
            name: name.map(str::to_owned),
            pattern,
            recorded_fields: None,
        }
    }

    /// The five default shape slots `Config1` to `Config5` (DEC-03-Q03).
    #[must_use]
    pub fn default_shape_slots() -> Vec<Self> {
        Self::defaults("Config")
    }

    /// The five default deformation slots `Deform1` to `Deform5` (DEC-03-Q03).
    #[must_use]
    pub fn default_deform_slots() -> Vec<Self> {
        Self::defaults("Deform")
    }

    fn defaults(prefix: &str) -> Vec<Self> {
        (1..=5)
            .map(|number| {
                let pattern = ProfilePattern {
                    tile_size: Some(Vec2I::new(0, 0)),
                    variants: vec![Arc::new(PatternVariant::default())],
                    ..ProfilePattern::default()
                };
                Self::new(Some(&format!("{prefix}{number}")), Some(pattern))
            })
            .collect()
    }
}
