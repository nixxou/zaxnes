//! The domain model of a profile (SPEC-03 §4.1) and its version rules (REQ-PROF-021).

use std::collections::HashSet;
use std::sync::Arc;

use crate::enums::ProfileFormatVersion;
use crate::layout::FieldMask;
use crate::pattern::ProfilePattern;
use crate::script::ProfileScript;
use crate::script_order;
use crate::settings::{LayerSettingsSet, LightSettings, ProfileSettings};
use crate::tile::ProfileTile;
use crate::values::{ArgbColor, Vec3F, Vec4F};

/// The signature a profile file carries.
pub const REQUIRED_SIGNATURE: &str = "3dn";

/// The version text written by this program.
pub const CURRENT_VERSION_TEXT: &str = "3";

/// A game profile: scripts, patterns and embedded settings, with the identity fields of the
/// header.
#[derive(Debug, Clone, Default)]
pub struct GameProfile {
    /// Signature (`"3dn"`), or none when the field is absent.
    pub signature: Option<String>,
    /// Version text as stored.
    pub version_text: Option<String>,
    /// Scripts, in file order until [`Self::sort_scripts`] runs.
    pub scripts: Vec<ProfileScript>,
    /// Patterns of the permanent family.
    pub permanent_patterns: Vec<Arc<ProfilePattern>>,
    /// Patterns of the temporary family.
    pub temporary_patterns: Vec<Arc<ProfilePattern>>,
    /// Embedded settings.
    pub settings: Option<ProfileSettings>,
    /// Identifier.
    pub id: Option<String>,
    /// Title.
    pub title: Option<String>,
    /// Author (`maker`).
    pub author: Option<String>,
    /// Publisher (`gameDev`).
    pub publisher: Option<String>,
    /// Description (`des`).
    pub description: Option<String>,
    /// The fields the file carried, when they differ from the schema (Q-010).
    pub recorded_fields: Option<FieldMask>,
}

impl GameProfile {
    /// Format version of this profile, from its version text.
    #[must_use]
    pub fn format_version(&self) -> ProfileFormatVersion {
        version_rules::classify(self.version_text.as_deref())
    }

    /// True when the "V2 format" flag is set (REQ-PROF-021).
    #[must_use]
    pub fn is_v2_format(&self) -> bool {
        self.format_version() == ProfileFormatVersion::V2
    }

    /// Every pattern once, permanent family first; a pattern held in both lists (REQ-PROF-011)
    /// is reported once.
    pub fn all_patterns(&self) -> impl Iterator<Item = &Arc<ProfilePattern>> {
        let mut seen = HashSet::new();
        self.permanent_patterns
            .iter()
            .chain(&self.temporary_patterns)
            .filter(move |pattern| seen.insert(Arc::as_ptr(pattern)))
    }

    /// Every tile object once, in pattern then cell order; two cells that hold the same shared
    /// tile report it once, two distinct objects with equal planes report both.
    pub fn distinct_tile_objects(&self) -> impl Iterator<Item = &Arc<ProfileTile>> {
        let mut seen = HashSet::new();
        self.all_patterns()
            .filter_map(|pattern| pattern.grid.as_ref())
            .flat_map(|grid| grid.cells().iter().flatten())
            .filter(move |tile| seen.insert(Arc::as_ptr(tile)))
    }

    /// Sorts the scripts by name (REQ-PROF-024).
    pub fn sort_scripts(&mut self) {
        self.scripts
            .sort_by(|left, right| script_order::compare_names(left.name.as_deref(), right.name.as_deref()));
    }

    /// A new empty profile with the default settings.
    #[must_use]
    pub fn create_empty() -> Self {
        Self {
            signature: Some(REQUIRED_SIGNATURE.to_owned()),
            version_text: Some(CURRENT_VERSION_TEXT.to_owned()),
            settings: Some(ProfileSettings {
                light: Some(LightSettings {
                    direction: Some(Vec3F::new(1.0, -1.0, 1.0)),
                    color: Some(ArgbColor::new(-1)),
                    intensity: 1.0,
                    recorded_fields: None,
                }),
                clipping: Some(Vec4F::new(0.1, 8.1, 8.1, 0.1)),
                layers: Some(LayerSettingsSet::create_default()),
                use_nes_background_color: true,
                ..ProfileSettings::default()
            }),
            ..Self::default()
        }
    }
}

/// How the version text of a profile is read (REQ-PROF-021).
pub mod version_rules {
    use crate::enums::ProfileFormatVersion;

    /// The version text with everything but digits, `.` and `$` removed.
    #[must_use]
    pub fn sanitize(version_text: Option<&str>) -> String {
        version_text
            .unwrap_or_default()
            .chars()
            .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '$')
            .collect()
    }

    /// The format version of a version text, after filtering.
    #[must_use]
    pub fn classify(version_text: Option<&str>) -> ProfileFormatVersion {
        match sanitize(version_text).as_str() {
            "1" => ProfileFormatVersion::Unsupported,
            "2" => ProfileFormatVersion::V2,
            _ => ProfileFormatVersion::Current,
        }
    }
}
