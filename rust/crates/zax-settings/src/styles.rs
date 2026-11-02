//! The visual styles and their parameters, as `Setting:Get/SetVisualStyleParameter(style, id)`
//! names them (AMD-040 REQ-LUA-114, AMD-059 REQ-LUA-116).
//!
//! A parameter identifier is a fact of interoperability: a 2.0 profile script passes it as a
//! string. Each one stands for a setting of the catalogue, so a write from a script and a write
//! from the options window reach the very same value (AMD-043 REQ-SCN-066, AMD-055
//! REQ-SCN-079), and the rounding of AMD-040 §3 happens once, in the setting's domain.

use crate::catalogue::SettingId;

/// The identifier of the Classic style (AMD-040 §3): it has no parameter.
pub const CLASSIC_STYLE: i32 = 0;

/// The identifier of the Enhanced style, the one `Setting.VisualStyle` reads back (AMD-055
/// REQ-SCN-078).
pub const ENHANCED_STYLE: i32 = 4;

/// The other identifier REQ-SCN-078 accepts for Enhanced, in a setting as in an argument of the
/// style-parameter methods. AMD-040 wrote the style 1; the 2.0 numbers it 4.
pub const ENHANCED_STYLE_ALIAS: i32 = 1;

/// The parameters of the Enhanced style: the four curvature ones of AMD-043 REQ-SCN-066 and the
/// twelve of AMD-055 REQ-SCN-079. Every other identifier of 3dSen 2.0 is unspecified and
/// therefore "unknown" in the sense of REQ-LUA-114 — reading gives 0, writing answers false.
const ENHANCED_PARAMETERS: [(&str, SettingId); 16] = [
    // AMD-043 REQ-SCN-066.
    ("spl.bend_zx", SettingId::SceneBendLr),
    ("spl.bend_zy", SettingId::SceneBendTb),
    ("spl.arc_yx", SettingId::SceneArcH),
    ("spl.arc_center", SettingId::SceneArcCenter),
    // AMD-055 REQ-SCN-079.
    ("spl.ao", SettingId::RenderStyleAo),
    ("ao.strength", SettingId::RenderStyleAoStrength),
    ("ao.radius", SettingId::RenderStyleAoRadius),
    ("ao.thickness", SettingId::RenderStyleAoThickness),
    ("ao.quality", SettingId::RenderStyleAoQuality),
    ("spl.pixel_shadow", SettingId::RenderStylePixelShadow),
    ("spl.voxel_grid", SettingId::RenderStyleVoxelGrid),
    ("spl.reflections", SettingId::RenderStyleReflections),
    ("ssr.shine", SettingId::RenderStyleSsrShine),
    ("ssr.clarity", SettingId::RenderStyleSsrClarity),
    ("ssr.length", SettingId::RenderStyleSsrLength),
    ("ssr.quality", SettingId::RenderStyleSsrQuality),
];

/// The style identifier a read gives back for `style`: the alias 1 of Enhanced becomes 4
/// (REQ-SCN-078), anything else is left alone — an unknown style stays unknown.
#[must_use]
pub const fn canonical_style(style: i32) -> i32 {
    if style == ENHANCED_STYLE_ALIAS {
        ENHANCED_STYLE
    } else {
        style
    }
}

/// Whether `style` is a style Zaxnes knows (REQ-LUA-114: the methods answer false for any
/// other).
#[must_use]
pub fn is_known_style(style: i32) -> bool {
    matches!(canonical_style(style), CLASSIC_STYLE | ENHANCED_STYLE)
}

/// The setting the parameter `id` of `style` stands for, or `None` when the style or the
/// identifier is unknown (REQ-LUA-114).
#[must_use]
pub fn parameter(style: i32, id: &str) -> Option<SettingId> {
    if canonical_style(style) != ENHANCED_STYLE {
        return None;
    }
    ENHANCED_PARAMETERS
        .iter()
        .find(|(name, _)| *name == id)
        .map(|(_, setting)| *setting)
}

/// Every parameter of `style`, in the order of the amendments, with the setting it stands for.
#[must_use]
pub fn parameters_of(style: i32) -> &'static [(&'static str, SettingId)] {
    if canonical_style(style) == ENHANCED_STYLE {
        &ENHANCED_PARAMETERS
    } else {
        &[]
    }
}

/// The styles that have parameters, for the "every parameter of every style" of REQ-LUA-116.
pub const STYLES_WITH_PARAMETERS: [i32; 1] = [ENHANCED_STYLE];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalogue;

    /// AMD-055 REQ-SCN-078: 1 and 4 are the same style, 0 is Classic, the rest is unknown.
    #[test]
    fn the_alias_of_enhanced_is_the_enhanced_style() {
        assert_eq!(ENHANCED_STYLE, canonical_style(ENHANCED_STYLE_ALIAS));
        assert_eq!(ENHANCED_STYLE, canonical_style(ENHANCED_STYLE));
        assert_eq!(CLASSIC_STYLE, canonical_style(CLASSIC_STYLE));
        assert!(is_known_style(0));
        assert!(is_known_style(1));
        assert!(is_known_style(4));
        assert!(!is_known_style(7));
        assert!(!is_known_style(2));
    }

    /// AMD-040 REQ-LUA-114: Classic has no parameter, and an unknown style none either.
    #[test]
    fn only_enhanced_has_parameters() {
        assert_eq!(Some(SettingId::SceneBendLr), parameter(1, "spl.bend_zx"));
        assert_eq!(Some(SettingId::SceneBendLr), parameter(4, "spl.bend_zx"));
        assert_eq!(None, parameter(0, "spl.bend_zx"));
        assert_eq!(None, parameter(7, "spl.bend_zx"));
        assert_eq!(None, parameter(4, "unknown"));
        assert_eq!(0, parameters_of(0).len());
        assert_eq!(16, parameters_of(4).len());
    }

    /// Every identifier names a setting of the catalogue, and no two name the same one.
    #[test]
    fn every_parameter_has_its_own_setting() {
        let mut seen = Vec::new();
        for (name, id) in ENHANCED_PARAMETERS {
            assert!(!seen.contains(&id), "{name} shares its setting");
            seen.push(id);
            let descriptor = catalogue::descriptor(id);
            assert!(
                descriptor.domain().step().is_some(),
                "{name} needs a step for the rounding of AMD-040 §3"
            );
        }
    }
}
