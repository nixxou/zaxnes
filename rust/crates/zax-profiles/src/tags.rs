//! Tag strings of variants and scripts (SPEC-03 REQ-PROF-036, SPEC-05 REQ-LUA-005/006).

/// The wildcard tag: a script that holds it is bound to every variant.
pub const WILDCARD: &str = "*";

/// Variant tag: the variant casts a shadow.
pub const CAST_SHADOW_TAG: &str = "!cast";

/// Variant tag: the variant receives shadows.
pub const RECEIVE_SHADOW_TAG: &str = "!receiv";

/// Short form of [`RECEIVE_SHADOW_TAG`].
pub const RECEIVE_SHADOW_SHORT_TAG: &str = "!recv";

/// Variant tag: the variant is static.
pub const STATIC_TAG: &str = "!static";

/// Variant tag: the variant is a ground axis.
pub const GROUND_AXIS_TAG: &str = "_gaxis";

/// The five special variant tags of REQ-PROF-036.
pub const SPECIAL_VARIANT_TAGS: [&str; 5] = [
    CAST_SHADOW_TAG,
    RECEIVE_SHADOW_TAG,
    RECEIVE_SHADOW_SHORT_TAG,
    STATIC_TAG,
    GROUND_AXIS_TAG,
];

/// The tags of a tag string: split on commas, trimmed, blanks dropped.
#[must_use]
pub fn parse(tag_string: Option<&str>) -> Vec<&str> {
    tag_string
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .collect()
}

/// True when the tag string holds a `*` anywhere: the script is bound to every variant.
#[must_use]
pub fn is_universal(tag_string: Option<&str>) -> bool {
    tag_string.is_some_and(|tags| tags.contains('*'))
}

/// True when a script with `script_tags` is bound to a variant with `variant_tags`
/// (REQ-LUA-006): the script is universal, or they share a tag.
#[must_use]
pub fn binds(script_tags: Option<&str>, variant_tags: Option<&str>) -> bool {
    if is_universal(script_tags) {
        return true;
    }
    let script = parse(script_tags);
    if script.is_empty() {
        return false;
    }
    let variant = parse(variant_tags);
    script.iter().any(|tag| variant.contains(tag))
}
