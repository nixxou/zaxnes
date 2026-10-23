//! The integer speed setting `s` of SPEC-07 REQ-LOOP-015 and the factor `k` it maps to:
//! `k = 1 + s/4` for `s >= 0` and `k = 1 / (1 - s/4)` for `s < 0`, so the setting is differential
//! rather than symmetric: -8 gives 1/3 and +8 gives 3. There is no separate slow-motion mode; the
//! negative half of the setting is it.

/// Lowest value of the setting (REQ-LOOP-015, DEC-07-Q1).
pub const MINIMUM: i32 = -8;
/// Highest value of the setting.
pub const MAXIMUM: i32 = 8;
/// Default value, which is the factor 1.
pub const DEFAULT: i32 = 0;
/// The setting fast forward applies, whatever the user's setting is (REQ-LOOP-016, DEC-07-Q2):
/// `s = 2`, so `k = 1.5`.
pub const FAST_FORWARD: i32 = 2;

/// Brings a value back into `[-8 ; 8]` (SPEC-09 REQ-PERS-003 rule 3).
#[must_use]
pub fn clamp(setting: i32) -> i32 {
    setting.clamp(MINIMUM, MAXIMUM)
}

/// The factor `k` of a setting, after clamping.
#[must_use]
pub fn factor(setting: i32) -> f64 {
    let s = f64::from(clamp(setting));
    if s >= 0.0 {
        1.0 + (s / 4.0)
    } else {
        1.0 / (1.0 - (s / 4.0))
    }
}

/// The factor fast forward uses, 1.5.
#[must_use]
pub fn fast_forward_factor() -> f64 {
    factor(FAST_FORWARD)
}
