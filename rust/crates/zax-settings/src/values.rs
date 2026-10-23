//! The value of a setting, of any of the shapes of `SettingKind` (REQ-PERS-001).

use std::fmt;
use std::str::FromStr;

use crate::meta::SettingKind;
use crate::num;

/// Three reals, for `light.direction` (SPEC-09 §4.1.3).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vector3 {
    /// Creates a vector.
    #[must_use]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// Whether every component is zero, which the domain "non nul" refuses.
    #[must_use]
    pub fn is_zero(self) -> bool {
        self.x == 0.0 && self.y == 0.0 && self.z == 0.0
    }
}

/// Four whole numbers, for `ui.window_rect` (REQ-PERS-067).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    /// Creates a rectangle.
    #[must_use]
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

/// An RGBA colour of four reals in 0..1 (SPEC-09 §4.1.3, §4.1.4).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    /// Opaque black, the default of `scene.custom_bg_color`.
    pub const BLACK: Self = Self::new(0.0, 0.0, 0.0, 1.0);
    /// Opaque white, the default of `light.color`.
    pub const WHITE: Self = Self::new(1.0, 1.0, 1.0, 1.0);

    /// Creates a colour.
    #[must_use]
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// The colour of a packed 24-bit RGB value, fully opaque.
    #[must_use]
    pub fn from_rgb(packed: i32) -> Self {
        Self::new(
            f32::from(num::low_byte(packed >> 16)) / 255.0,
            f32::from(num::low_byte(packed >> 8)) / 255.0,
            f32::from(num::low_byte(packed)) / 255.0,
            1.0,
        )
    }
}

/// The `#RRGGBBAA` form the settings file uses (SPEC-09 §4.5.1).
impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "#{:02X}{:02X}{:02X}{:02X}",
            num::unit_to_byte(self.r),
            num::unit_to_byte(self.g),
            num::unit_to_byte(self.b),
            num::unit_to_byte(self.a),
        )
    }
}

/// A colour text that is neither `#RRGGBBAA` nor `#RRGGBB`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("'{0}' is not a colour of the form #RRGGBBAA or #RRGGBB")]
pub struct ParseColorError(String);

/// Parses the `#RRGGBBAA` form; `#RRGGBB` is accepted as opaque.
impl FromStr for Color {
    type Err = ParseColorError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let error = || ParseColorError(text.to_owned());
        let trimmed = text.trim();
        let digits = trimmed.strip_prefix('#').unwrap_or(trimmed);
        if !matches!(digits.len(), 6 | 8) {
            return Err(error());
        }
        let channels = (0..digits.len() / 2)
            .map(|i| {
                digits
                    .get(2 * i..2 * i + 2)
                    .and_then(|hex| u8::from_str_radix(hex, 16).ok())
                    .map(|byte| f32::from(byte) / 255.0)
            })
            .collect::<Option<Vec<f32>>>()
            .ok_or_else(error)?;
        match channels.as_slice() {
            [r, g, b] => Ok(Self::new(*r, *g, *b, 1.0)),
            [r, g, b, a] => Ok(Self::new(*r, *g, *b, *a)),
            _ => Err(error()),
        }
    }
}

/// The value of one setting. Scalars keep their own width; the compound shapes hold their data
/// directly. Equality is by value, which is what REQ-PERS-009 needs to decide whether a write
/// really changed anything.
#[derive(Debug, Clone, PartialEq)]
pub enum SettingValue {
    Bool(bool),
    Int(i32),
    Float(f32),
    /// An enumerated value, held as its number (REQ-PERS-004).
    Enum(i32),
    Color(Color),
    Vector3(Vector3),
    Text(String),
    Rect(Rect),
    /// A value this model only stores and hands back, without interpreting it (REQ-PERS-016).
    Opaque(Option<serde_json::Value>),
}

impl SettingValue {
    /// The shape of this value.
    #[must_use]
    pub fn kind(&self) -> SettingKind {
        match self {
            Self::Bool(_) => SettingKind::Bool,
            Self::Int(_) => SettingKind::Int,
            Self::Float(_) => SettingKind::Float,
            Self::Enum(_) => SettingKind::Enum,
            Self::Color(_) => SettingKind::Color,
            Self::Vector3(_) => SettingKind::Vector3,
            Self::Text(_) => SettingKind::Text,
            Self::Rect(_) => SettingKind::Rect,
            Self::Opaque(_) => SettingKind::Opaque,
        }
    }

    /// The boolean reading: any non-zero scalar is true.
    #[must_use]
    pub fn as_bool(&self) -> bool {
        match self {
            Self::Bool(value) => *value,
            Self::Int(value) | Self::Enum(value) => *value != 0,
            Self::Float(value) => *value != 0.0,
            _ => false,
        }
    }

    /// The whole reading, rounded to the nearest.
    #[must_use]
    pub fn as_int(&self) -> i32 {
        match self {
            Self::Bool(value) => i32::from(*value),
            Self::Int(value) | Self::Enum(value) => *value,
            Self::Float(value) => num::round_to_i32(f64::from(*value)),
            _ => 0,
        }
    }

    /// The real reading.
    #[must_use]
    pub fn as_float(&self) -> f32 {
        match self {
            Self::Bool(value) => f32::from(u8::from(*value)),
            Self::Int(value) | Self::Enum(value) => num::i32_to_f32(*value),
            Self::Float(value) => *value,
            _ => 0.0,
        }
    }

    /// The scalar as a double, without narrowing; zero for a compound value.
    #[must_use]
    pub fn as_double(&self) -> f64 {
        match self {
            Self::Bool(value) => f64::from(u8::from(*value)),
            Self::Int(value) | Self::Enum(value) => f64::from(*value),
            Self::Float(value) => f64::from(*value),
            _ => 0.0,
        }
    }

    /// The colour reading, black when the value is not a colour.
    #[must_use]
    pub fn as_color(&self) -> Color {
        match self {
            Self::Color(colour) => *colour,
            _ => Color::BLACK,
        }
    }

    /// The vector reading, zero when the value is not a vector.
    #[must_use]
    pub fn as_vector3(&self) -> Vector3 {
        match self {
            Self::Vector3(vector) => *vector,
            _ => Vector3::default(),
        }
    }

    /// The text reading, empty when the value is not text.
    #[must_use]
    pub fn as_text(&self) -> &str {
        match self {
            Self::Text(text) => text,
            _ => "",
        }
    }

    /// The rectangle reading, zero when the value is not a rectangle.
    #[must_use]
    pub fn as_rect(&self) -> Rect {
        match self {
            Self::Rect(rect) => *rect,
            _ => Rect::default(),
        }
    }

    /// The stored object of an opaque value, or none.
    #[must_use]
    pub fn as_opaque(&self) -> Option<&serde_json::Value> {
        match self {
            Self::Opaque(value) => value.as_ref(),
            _ => None,
        }
    }

    /// Reshapes this value to `kind`, so that a number read from a file can fill an enumerated or
    /// boolean setting (SPEC-09 §4.5.1). A compound value that has no reading in the new shape
    /// becomes that shape's empty value.
    #[must_use]
    pub fn coerce_to(self, kind: SettingKind) -> Self {
        if self.kind() == kind {
            return self;
        }
        match kind {
            SettingKind::Bool => Self::Bool(self.as_bool()),
            SettingKind::Int => Self::Int(self.as_int()),
            SettingKind::Float => Self::Float(self.as_float()),
            SettingKind::Enum => Self::Enum(self.as_int()),
            SettingKind::Color => Self::Color(self.as_color()),
            SettingKind::Vector3 => Self::Vector3(self.as_vector3()),
            SettingKind::Text => Self::Text(self.as_text().to_owned()),
            SettingKind::Rect => Self::Rect(self.as_rect()),
            SettingKind::Opaque => Self::Opaque(None),
        }
    }
}

impl From<bool> for SettingValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<f32> for SettingValue {
    fn from(value: f32) -> Self {
        Self::Float(value)
    }
}

impl From<Color> for SettingValue {
    fn from(value: Color) -> Self {
        Self::Color(value)
    }
}

impl From<Vector3> for SettingValue {
    fn from(value: Vector3) -> Self {
        Self::Vector3(value)
    }
}

impl From<Rect> for SettingValue {
    fn from(value: Rect) -> Self {
        Self::Rect(value)
    }
}

impl From<&str> for SettingValue {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}
