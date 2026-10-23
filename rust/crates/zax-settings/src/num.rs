//! Numeric conversions of the settings. Each one reproduces the C# cast it stands for: the values
//! held here are in range for their setting, so the saturating `as` of Rust agrees with the C#.

/// The low byte of `value`, as the C# `(byte)` of a masked value.
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is masked to its low byte before the cast"
)]
pub fn low_byte(value: i32) -> u8 {
    (value & 0xFF) as u8
}

/// A real rounded to the nearest, ties away from zero, as C# `(int)Math.Round`.
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    reason = "the settings hold whole numbers far below the i32 range"
)]
pub fn round_to_i32(value: f64) -> i32 {
    value.round() as i32
}

/// A real truncated toward zero, as the C# `(int)` cast.
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    reason = "the settings hold whole numbers far below the i32 range"
)]
pub fn truncate_to_i32(value: f64) -> i32 {
    value as i32
}

/// A real narrowed to the setting's float, as the C# `(float)` cast.
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    reason = "a setting's real is a float by definition, as in the C#"
)]
pub fn to_f32(value: f64) -> f32 {
    value as f32
}

/// A whole number widened to a float, as the C# `(float)` cast.
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    reason = "the whole numbers of the settings are exact in a float"
)]
pub fn i32_to_f32(value: i32) -> f32 {
    value as f32
}

/// A unit-interval channel as a byte: clamped, scaled by 255 and rounded to even, as the C#
/// `Math.Round` of the colour text.
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the scaled channel is clamped to 0..=255 first"
)]
pub fn unit_to_byte(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round_ties_even() as u8
}
