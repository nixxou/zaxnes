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

/// Every parameter of every style, in one table: the style, the identifier and the setting. This
/// is the order the write log of `Setting.VisualStyleParameters` names a parameter by
/// (REQ-LUA-116).
#[must_use]
pub fn all_parameters() -> &'static [(&'static str, SettingId)] {
    &ENHANCED_PARAMETERS
}

/// The position of the parameter `id` in [`all_parameters`], or `None` when the style or the
/// identifier is unknown.
#[must_use]
pub fn parameter_position(style: i32, id: &str) -> Option<usize> {
    if canonical_style(style) != ENHANCED_STYLE {
        return None;
    }
    ENHANCED_PARAMETERS.iter().position(|(name, _)| *name == id)
}

/// The style, the identifier and the setting of the parameter at `position`.
#[must_use]
pub fn parameter_of_position(position: usize) -> Option<(i32, &'static str, SettingId)> {
    ENHANCED_PARAMETERS
        .get(position)
        .map(|(name, id)| (ENHANCED_STYLE, *name, *id))
}

/// `value` normalised for a write from a **script** (REQ-LUA-114): the domain's own rounding, for
/// every style parameter, which is the one place AMD-040 §3 lives.
///
/// RO-1 point 7 made this function an exception for the three curvature angles — their domain had
/// grown to 23 rungs (5° between −30 and 30), and a Lua write was rounded onto the 13-rung grid
/// of 30° first, so that a script kept behaving as it did against 3dSen. **RO-2 part 3 (owner
/// decision) removes that exception**: a Lua write snaps on the same grid as the menu, to the
/// nearest rung, ties away from zero — which is, letter for letter, what `Domain::grid` does. So
/// 45 → 60, 75 → **90**, −45 → −60, 27.5 → 30 and 12.5 → 15, and a value already on the grid is
/// left alone, which is why a profile written for 3dSen out of multiples of 30 behaves exactly as
/// it always did.
///
/// The function stays, rather than its callers going straight to the domain, because it is the
/// one place a *script* write is normalised: the next divergence between a script and the
/// interface, if the owner ever wants one, lands here and nowhere else.
// Q-107 §3
#[must_use]
pub fn snap_write(id: SettingId, value: f64) -> f64 {
    crate::catalogue::descriptor(id).domain().snap(value)
}

/// The rung of `value` among the values the domain of `id` publishes, and the value of a rung.
/// A style parameter only ever holds one of its rungs, the domain snapping every write
/// (AMD-040 §3), so the two are a faithful round trip.
#[must_use]
pub fn rung_of(id: SettingId, value: f64) -> usize {
    let domain = crate::catalogue::descriptor(id).domain();
    let snapped = domain.snap(value);
    domain
        .options()
        .and_then(|options| {
            options
                .iter()
                .position(|option| (option.value - snapped).abs() < 1e-9)
        })
        .unwrap_or(0)
}

/// The value of rung `rung` of `id`, or its default when the rung is past the end.
#[must_use]
pub fn value_of_rung(id: SettingId, rung: usize) -> f64 {
    let descriptor = crate::catalogue::descriptor(id);
    descriptor
        .domain()
        .options()
        .and_then(|options| options.get(rung))
        .map_or_else(
            || f64::from(descriptor.default_value().as_int()),
            |option| option.value,
        )
}

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
            // A step, or — since RO-1 point 7 gave the three curvature angles an uneven grid —
            // the published rungs the grid snaps to. Either way AMD-040 §3 has something to
            // round onto.
            let domain = descriptor.domain();
            assert!(
                domain.step().is_some() || domain.options().is_some_and(|rungs| rungs.len() > 1),
                "{name} needs a step or a grid for the rounding of AMD-040 §3"
            );
        }
    }

    /// The write log of `Setting.VisualStyleParameters` names a parameter and its value by one
    /// byte each (`zax_scripting::binding::payload::StyleParameterLog`): the table must stay at
    /// or under 256 parameters and every domain at or under 256 rungs. A lot that passes either
    /// bound has to widen the encoding, and this is where it finds out.
    #[test]
    fn a_parameter_and_a_rung_both_fit_in_a_byte() {
        assert!(ENHANCED_PARAMETERS.len() <= 256);
        for (name, id) in ENHANCED_PARAMETERS {
            let rungs = catalogue::descriptor(id)
                .domain()
                .options()
                .map_or(0, <[crate::domain::SettingOption]>::len);
            assert!((1..=256).contains(&rungs), "{name} publishes {rungs} rungs");
        }
    }

    /// A value and its rung are a faithful round trip, which is what the write log rests on.
    #[test]
    fn a_rung_and_its_value_round_trip() {
        for (position, (name, id)) in ENHANCED_PARAMETERS.iter().enumerate() {
            assert_eq!(
                Some((ENHANCED_STYLE, *name, *id)),
                parameter_of_position(position)
            );
            assert_eq!(Some(position), parameter_position(ENHANCED_STYLE, name));
            let rungs = catalogue::descriptor(*id)
                .domain()
                .options()
                .map_or(0, <[crate::domain::SettingOption]>::len);
            for rung in 0..rungs {
                let value = value_of_rung(*id, rung);
                assert_eq!(rung, rung_of(*id, value), "{name} rung {rung}");
            }
        }
    }
}
