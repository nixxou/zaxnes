//! The plain values the script API hands over and the number dialect of the original
//! (SPEC-05 §4.3, REQ-LUA-040, REQ-LUA-052).

use zax_engine3d::palette::Rgb24;
use zax_engine3d::vectors::Vec3;

/// A colour with four components in `[0, 1]` (SPEC-05 §4.3, type `Color`). Equality is exact
/// on all four components, as the type table requires.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LuaColor {
    /// Red.
    pub r: f32,
    /// Green.
    pub g: f32,
    /// Blue.
    pub b: f32,
    /// Alpha.
    pub a: f32,
}

impl LuaColor {
    /// Opaque black.
    pub const BLACK: Self = Self::new(0.0, 0.0, 0.0, 1.0);

    /// Opaque white.
    pub const WHITE: Self = Self::new(1.0, 1.0, 1.0, 1.0);

    /// Creates a colour.
    #[must_use]
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// The opaque colour of an 8-bit sRGB triple.
    #[must_use]
    pub fn from_bytes(r: u8, g: u8, b: u8) -> Self {
        Self::new(
            f32::from(r) / 255.0,
            f32::from(g) / 255.0,
            f32::from(b) / 255.0,
            1.0,
        )
    }

    /// The 8-bit sRGB triple of this colour, each component clamped and rounded.
    #[must_use]
    pub fn to_rgb24(self) -> Rgb24 {
        Rgb24::new(
            Self::component(self.r),
            Self::component(self.g),
            Self::component(self.b),
        )
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // C# (int) then (byte) after the clamp.
    fn component(value: f32) -> u8 {
        // MathF.Round rounds half to even, as `round_ties_even` does.
        ((value * 255.0).round_ties_even() as i32).clamp(0, 255) as u8
    }
}

/// A three-component integer vector (SPEC-05 §4.3, type `IntVector3`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LuaIntVector3 {
    /// First component.
    pub x: i32,
    /// Second component.
    pub y: i32,
    /// Third component.
    pub z: i32,
}

/// The state of one tracked VR device (SPEC-05 §4.2, type `XRState`). Every field is zero on
/// desktop (REQ-LUA-091).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LuaXrState {
    /// Position.
    pub position: Vec3,
    /// Velocity.
    pub velocity: Vec3,
    /// Acceleration.
    pub acceleration: Vec3,
    /// Rotation in degrees.
    pub rotation: Vec3,
}

/// How the runtime turns numbers into text, which has to match the dialect of the original: a
/// float whose value is integral is written without a decimal part, anything else with
/// `%.14g` (REQ-LUA-052, REQ-LUA-101 point 1).
pub mod number_format {
    /// 2^63 as a double: the first magnitude a 64-bit integer cannot hold.
    const TWO_POW_63: f64 = 9_223_372_036_854_775_808.0;

    /// Digits `%.14g` keeps.
    const SIGNIFICANT_DIGITS: usize = 14;

    /// The text `print` and `tostring` produce for a number.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // The magnitude was checked against 2^63.
    pub fn to_text(value: f64) -> String {
        if value.is_nan() {
            return if value.is_sign_negative() {
                "-nan"
            } else {
                "nan"
            }
            .to_owned();
        }

        if value.is_infinite() {
            return if value > 0.0 { "inf" } else { "-inf" }.to_owned();
        }

        // An integral value that a 64-bit integer can hold is written without a decimal part,
        // as the single numeric type of the original did.
        if value.to_bits() == value.trunc().to_bits() && value.abs() < TWO_POW_63 {
            return (value as i64).to_string();
        }

        general(value)
    }

    /// C's `%.14g`: scientific notation when the decimal exponent is below -4 or at least 14,
    /// fixed notation otherwise, trailing zeros removed.
    fn general(value: f64) -> String {
        let scientific = format!("{:.*e}", SIGNIFICANT_DIGITS - 1, value);
        let (mantissa, exponent) = scientific.split_once('e').unwrap_or((&scientific, "0"));
        let exponent: i32 = exponent.parse().unwrap_or(0);

        let digits = i32::try_from(SIGNIFICANT_DIGITS).unwrap_or(14);
        if exponent < -4 || exponent >= digits {
            let mantissa = trim_zeros(mantissa);
            let sign = if exponent < 0 { '-' } else { '+' };
            return format!("{mantissa}e{sign}{:02}", exponent.abs());
        }

        let decimals = usize::try_from(digits - 1 - exponent).unwrap_or(0);
        trim_zeros(&format!("{value:.decimals$}")).to_owned()
    }

    fn trim_zeros(text: &str) -> &str {
        if text.contains('.') {
            text.trim_end_matches('0').trim_end_matches('.')
        } else {
            text
        }
    }

    /// Truncation towards zero, the conversion a numeric Lua value undergoes on its way to an
    /// integer parameter (REQ-LUA-040).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // Saturating float-to-integer cast, as the C# clamps.
    pub fn truncate(value: f64) -> i64 {
        if value.is_nan() {
            return 0;
        }

        // `as` saturates at the bounds of i64, which is the clamp the C# performs by hand.
        value.trunc() as i64
    }

    /// Truncation towards zero, then reduction modulo 256 (REQ-LUA-040).
    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // Modulo 256 is the point.
    pub fn to_byte(value: f64) -> u8 {
        (truncate(value) & 0xFF) as u8
    }

    /// Truncation towards zero, then reduction modulo 65 536 (REQ-LUA-040).
    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // Modulo 65 536 is the point.
    pub fn to_address(value: f64) -> u16 {
        (truncate(value) & 0xFFFF) as u16
    }

    /// Truncation towards zero, saturated to the range of `i32`.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // Clamped to the i32 range first.
    pub fn to_i32(value: f64) -> i32 {
        truncate(value).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::number_format::to_text;

    #[test]
    fn integral_floats_have_no_decimal_part() {
        assert_eq!(to_text(120.0), "120");
        assert_eq!(to_text(-3.0), "-3");
        assert_eq!(to_text(0.0), "0");
    }

    #[test]
    fn other_numbers_use_14_significant_digits() {
        assert_eq!(to_text(0.4), "0.4");
        assert_eq!(to_text(0.100_000_001_490_116), "0.10000000149012");
        assert_eq!(to_text(1.0 / 3.0), "0.33333333333333");
        assert_eq!(to_text(1.5e-7), "1.5e-07");
        assert_eq!(to_text(1e20), "1e+20");
        assert_eq!(to_text(1.5e20), "1.5e+20");
        assert_eq!(to_text(f64::NAN), "nan");
        assert_eq!(to_text(f64::NEG_INFINITY), "-inf");
    }
}
