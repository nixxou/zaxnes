//! Lot REWRITE-UI, the signal options: the television the picture is shown on, its colours and
//! its knobs, shared by the composite decoder ([`crate::composite::RowDecoder`]) and the palette
//! the NES colours are otherwise shown in (`zax_engine3d::palette`).
//!
//! A "modern screen" shows the colours as they are. A tube decodes them with its own gamma curve
//! and its own phosphors: the colour is taken to linear light by a measured CRT curve, from the
//! tube's primaries and white to sRGB's (no white adaptation: the tube's white stays its own),
//! brought into the gamut by moving it towards its luminance, then encoded for an sRGB screen.
//! The curves, the chromaticities and the gamut rule are those of `BeesNES` (L. Spiro, MIT,
//! `Src/Utilities/LSNUtilities.h` `CrtProperToLinear` / `CrtProper2ToLinear`,
//! `Src/ColorSpace/LSNColorSpace.cpp` `ConvertLinear` and the tables of measured sets);
//! recorded as FIDELITY-LEADS H4.

/// The television of `video.tv_model`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TvModel {
    /// A modern screen: the colours as they are.
    #[default]
    Modern,
    /// A US tube: an RCA `ColorTrak` E13169GM (1989), measured by patchy68k, the `CrtProper2` curve.
    NtscRca,
    /// A Japanese tube: a Toshiba CF2005, measured by patchy68k, the `CrtProper2` curve.
    JapanToshiba,
    /// A European tube: a PAL Sony Trinitron at 6900 K, measured by `DariusG`, the `CrtProper`
    /// curve (white 1, brightness 0.1448, `BeesNES`'s PAL default).
    PalTrinitron,
}

impl TvModel {
    /// The model of choice `index` of `video.tv_model` (a modern screen for any other).
    #[must_use]
    pub const fn from_index(index: i32) -> Self {
        match index {
            1 => Self::NtscRca,
            2 => Self::JapanToshiba,
            3 => Self::PalTrinitron,
            _ => Self::Modern,
        }
    }

    /// Its index in `video.tv_model`.
    #[must_use]
    pub const fn index(self) -> u8 {
        match self {
            Self::Modern => 0,
            Self::NtscRca => 1,
            Self::JapanToshiba => 2,
            Self::PalTrinitron => 3,
        }
    }

    /// Red, green, blue and white chromaticities (x, y), `None` for a modern screen.
    const fn chromaticities(self) -> Option<[f64; 8]> {
        match self {
            Self::Modern => None,
            Self::NtscRca => Some([
                0.623_492_333,
                0.343_975_233,
                0.312_266_967,
                0.590_305_267,
                0.153_586_233,
                0.064_531_667,
                0.301,
                0.308,
            ]),
            Self::JapanToshiba => Some([
                0.642_388, 0.339_424, 0.301, 0.601, 0.143_599, 0.070_975, 0.281, 0.311,
            ]),
            Self::PalTrinitron => Some([
                0.635, 0.325, 0.290, 0.585, 0.1525, 0.0575, 0.306_769, 0.322_990,
            ]),
        }
    }

    /// The tube's curve from a signal value (0 to 1) to linear light.
    fn to_linear(self, value: f64) -> f64 {
        let value = value.clamp(0.0, 1.0);
        match self {
            Self::Modern => srgb_to_linear(value),
            Self::NtscRca | Self::JapanToshiba => crt_proper2_to_linear(value),
            Self::PalTrinitron => crt_proper_to_linear(value, 1.0, 0.0181 * 8.0),
        }
    }
}

/// `CrtProper2ToLinear` (`BeesNES`): a power of 2.31 above 0.36, blended below with the Rec. 709
/// curve.
fn crt_proper2_to_linear(value: f64) -> f64 {
    const ALPHA: f64 = 0.111_572_195_921_731_26;
    const BETA: f64 = 1.111_572_195_921_731_3;
    const CUT: f64 = 0.091_286_342_117_780_11;
    if value >= 0.36 {
        return value.powf(2.31);
    }
    let fraction = value / 0.36;
    let low = if value <= CUT {
        value / 4.0
    } else {
        ((value + ALPHA) / BETA).powf(1.0 / 0.45)
    };
    low * (1.0 - fraction) + fraction * value.powf(2.31)
}

/// `CrtProperToLinear` (`BeesNES`, after ITU-R BT.1886 annex 1): white `white`, black lift
/// `lift`.
fn crt_proper_to_linear(value: f64, white: f64, lift: f64) -> f64 {
    const ALPHA1: f64 = 2.6;
    const ALPHA2: f64 = 3.0;
    const KNEE: f64 = 0.35;
    let k = white / (1.0 + lift).powf(ALPHA1);
    if value < KNEE {
        k * (KNEE + lift).powf(ALPHA1 - ALPHA2) * (value + lift).powf(ALPHA2)
    } else {
        k * (value + lift).powf(ALPHA1)
    }
}

fn srgb_to_linear(value: f64) -> f64 {
    if value <= 0.040_45 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(value: f64) -> f64 {
    let value = value.clamp(0.0, 1.0);
    if value <= 0.003_130_8 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    }
}

type Matrix = [[f64; 3]; 3];

fn multiply(a: &Matrix, b: &Matrix) -> Matrix {
    let mut out = [[0.0; 3]; 3];
    for (row, out_row) in out.iter_mut().enumerate() {
        for (column, cell) in out_row.iter_mut().enumerate() {
            *cell = (0..3).map(|k| a[row][k] * b[k][column]).sum();
        }
    }
    out
}

fn invert(m: &Matrix) -> Matrix {
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    let inv = 1.0 / det;
    [
        [
            (m[1][1] * m[2][2] - m[1][2] * m[2][1]) * inv,
            (m[0][2] * m[2][1] - m[0][1] * m[2][2]) * inv,
            (m[0][1] * m[1][2] - m[0][2] * m[1][1]) * inv,
        ],
        [
            (m[1][2] * m[2][0] - m[1][0] * m[2][2]) * inv,
            (m[0][0] * m[2][2] - m[0][2] * m[2][0]) * inv,
            (m[0][2] * m[1][0] - m[0][0] * m[1][2]) * inv,
        ],
        [
            (m[1][0] * m[2][1] - m[1][1] * m[2][0]) * inv,
            (m[0][1] * m[2][0] - m[0][0] * m[2][1]) * inv,
            (m[0][0] * m[1][1] - m[0][1] * m[1][0]) * inv,
        ],
    ]
}

/// RGB to XYZ for primaries and white `c` (x, y pairs: red, green, blue, white).
fn rgb_to_xyz(c: &[f64; 8]) -> Matrix {
    let column = |x: f64, y: f64| [x / y, 1.0, (1.0 - x - y) / y];
    let (r, g, b) = (column(c[0], c[1]), column(c[2], c[3]), column(c[4], c[5]));
    let primaries = [[r[0], g[0], b[0]], [r[1], g[1], b[1]], [r[2], g[2], b[2]]];
    let white = column(c[6], c[7]);
    let inverse = invert(&primaries);
    let scale: Vec<f64> = (0..3)
        .map(|row| (0..3).map(|k| inverse[row][k] * white[k]).sum())
        .collect();
    let mut out = primaries;
    for row in &mut out {
        for (column, cell) in row.iter_mut().enumerate() {
            *cell *= scale[column];
        }
    }
    out
}

/// sRGB's primaries and D65 white.
const SRGB: [f64; 8] = [0.64, 0.33, 0.30, 0.60, 0.15, 0.06, 0.3127, 0.3290];

/// The knobs of the set (`video.tv_*`): neutral at 0 degrees, 100 %, 100 %, 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TvKnobs {
    /// Tint, in degrees (-30 to 30).
    pub hue: i8,
    /// Colour, in percent (0 to 200).
    pub saturation: u8,
    /// Contrast (picture), in percent (50 to 150).
    pub contrast: u8,
    /// Brightness, in hundredths of the black-to-white span (-50 to 50).
    pub brightness: i8,
}

impl Default for TvKnobs {
    fn default() -> Self {
        Self::NEUTRAL
    }
}

impl TvKnobs {
    /// The knobs at rest.
    pub const NEUTRAL: Self = Self {
        hue: 0,
        saturation: 100,
        contrast: 100,
        brightness: 0,
    };

    /// Whether they change nothing.
    #[must_use]
    pub fn is_neutral(self) -> bool {
        self == Self::NEUTRAL
    }

    /// Luma and the two chroma components (I and Q, or U and V) as the knobs leave them: the tint
    /// turns the chroma, the colour scales it, the contrast scales everything, the brightness
    /// lifts the luma.
    #[must_use]
    pub fn adjust(self, y: f32, a: f32, b: f32) -> (f32, f32, f32) {
        if self.is_neutral() {
            return (y, a, b);
        }
        let (sin, cos) = f32::from(self.hue).to_radians().sin_cos();
        let colour = f32::from(self.saturation) / 100.0;
        let contrast = f32::from(self.contrast) / 100.0;
        let (a, b) = (a * cos - b * sin, a * sin + b * cos);
        (
            y * contrast + f32::from(self.brightness) / 100.0,
            a * colour * contrast,
            b * colour * contrast,
        )
    }
}

/// The television of the picture: its model and its knobs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TvLook {
    /// The set.
    pub model: TvModel,
    /// Its knobs.
    pub knobs: TvKnobs,
}

impl TvLook {
    /// A modern screen, knobs at rest: the colours as they are.
    pub const NEUTRAL: Self = Self {
        model: TvModel::Modern,
        knobs: TvKnobs::NEUTRAL,
    };

    /// Whether it changes nothing.
    #[must_use]
    pub fn is_neutral(self) -> bool {
        self == Self::NEUTRAL
    }

    /// Packed into 40 bits (model, hue, colour, contrast, brightness), for an atomic.
    #[must_use]
    #[allow(clippy::cast_sign_loss)] // the signed knobs travel as their bytes
    pub fn packed(self) -> u64 {
        u64::from(self.model.index())
            | u64::from(self.knobs.hue as u8) << 8
            | u64::from(self.knobs.saturation) << 16
            | u64::from(self.knobs.contrast) << 24
            | u64::from(self.knobs.brightness as u8) << 32
    }

    /// The colour `rgb` (0 to 1, a palette's) as this set shows it: the knobs on its luma and
    /// chroma (FCC YIQ), then the tube's colours.
    #[must_use]
    pub fn show(self, rgb: [f32; 3]) -> [f32; 3] {
        if self.is_neutral() {
            return rgb;
        }
        let [red, green, blue] = rgb;
        let luma = 0.299 * red + 0.587 * green + 0.114 * blue;
        let in_phase = 0.596 * red - 0.274 * green - 0.322 * blue;
        let quadrature = 0.211 * red - 0.523 * green + 0.312 * blue;
        let (luma, in_phase, quadrature) = self.knobs.adjust(luma, in_phase, quadrature);
        let rgb = [
            (luma + 0.956 * in_phase + 0.621 * quadrature).clamp(0.0, 1.0),
            (luma - 0.272 * in_phase - 0.647 * quadrature).clamp(0.0, 1.0),
            (luma - 1.106 * in_phase + 1.703 * quadrature).clamp(0.0, 1.0),
        ];
        TvTransform::new(self.model).map_or(rgb, |transform| transform.apply(rgb))
    }
}

/// Entries of the tube curve's table, over the signal's 0 to 1.
const CURVE_STEPS: usize = 1024;

/// Entries of the screen's encoding table, over linear light's 0 to 1.
const ENCODE_STEPS: usize = 4096;

/// A tube's colours, made fast for a whole picture: its curve and the screen's in tables.
#[derive(Debug, Clone)]
pub struct TvTransform {
    curve: Vec<f32>,
    matrix: [[f32; 3]; 3],
    luma: [f32; 3],
    encode: Vec<f32>,
}

impl TvTransform {
    /// The transform of `model`, `None` for a modern screen (nothing to do).
    #[must_use]
    pub fn new(model: TvModel) -> Option<Self> {
        let chromaticities = model.chromaticities()?;
        let to_srgb = invert(&rgb_to_xyz(&SRGB));
        let matrix = multiply(&to_srgb, &rgb_to_xyz(&chromaticities));
        let luma = rgb_to_xyz(&SRGB)[1];
        #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)] // small tables
        let curve = (0..CURVE_STEPS)
            .map(|step| model.to_linear(step as f64 / (CURVE_STEPS - 1) as f64) as f32)
            .collect();
        #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
        let encode = (0..ENCODE_STEPS)
            .map(|step| linear_to_srgb(step as f64 / (ENCODE_STEPS - 1) as f64) as f32)
            .collect();
        #[allow(clippy::cast_possible_truncation)] // matrix coefficients near 1
        let narrow = |row: [f64; 3]| row.map(|cell| cell as f32);
        Some(Self {
            curve,
            matrix: matrix.map(narrow),
            luma: narrow(luma),
            encode,
        })
    }

    /// The colour `rgb` (signal values, 0 to 1) as the tube shows it on an sRGB screen.
    #[must_use]
    pub fn apply(&self, rgb: [f32; 3]) -> [f32; 3] {
        let look_up = |table: &[f32], value: f32| -> f32 {
            #[allow(clippy::cast_precision_loss)]
            let last = (table.len() - 1) as f32;
            let at = value.clamp(0.0, 1.0) * last;
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0..len
            let index = (at as usize).min(table.len() - 2);
            #[allow(clippy::cast_precision_loss)]
            let fraction = at - index as f32;
            table[index] + (table[index + 1] - table[index]) * fraction
        };
        let linear = rgb.map(|value| look_up(&self.curve, value));
        let mut out = [0.0_f32; 3];
        for (row, value) in out.iter_mut().enumerate() {
            *value = (0..3).map(|k| self.matrix[row][k] * linear[k]).sum();
        }
        // Out of gamut: towards the luminance just far enough that every channel fits.
        let y = (0..3)
            .map(|k| self.luma[k] * out[k])
            .sum::<f32>()
            .clamp(0.0, 1.0);
        let mut t = 1.0_f32;
        for &value in &out {
            let d = value - y;
            let this = if d >= 0.0 {
                (1.0 - y) / d.max(1.0e-6)
            } else {
                y / (-d).max(1.0e-6)
            };
            t = t.min(this);
        }
        let t = t.clamp(0.0, 1.0);
        out.map(|value| look_up(&self.encode, (y + (value - y) * t).clamp(0.0, 1.0)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_neutral_look_changes_nothing() {
        let colour = [0.2, 0.5, 0.9];
        assert_eq!(colour, TvLook::NEUTRAL.show(colour));
        assert!(TvTransform::new(TvModel::Modern).is_none());
    }

    #[test]
    fn a_tube_keeps_black_and_grey_order_and_tints_white_with_its_own() {
        for model in [
            TvModel::NtscRca,
            TvModel::JapanToshiba,
            TvModel::PalTrinitron,
        ] {
            let transform = TvTransform::new(model).unwrap();
            let black = transform.apply([0.0; 3]);
            assert!(black.iter().all(|&c| c < 0.05), "{model:?} {black:?}");
            let grey = transform.apply([0.5; 3]);
            let white = transform.apply([1.0; 3]);
            assert!(grey.iter().zip(white).all(|(g, w)| *g < w), "{model:?}");
            // The US and Japanese tubes' whites are bluer than D65.
            if model != TvModel::PalTrinitron {
                assert!(white[2] >= white[0], "{model:?} {white:?}");
            }
        }
    }

    #[test]
    fn the_crt_curves_meet_their_ends() {
        assert!((crt_proper2_to_linear(1.0) - 1.0).abs() < 1e-9);
        assert!(crt_proper2_to_linear(0.0).abs() < 1e-9);
        assert!((crt_proper_to_linear(1.0, 1.0, 0.1448) - 1.0).abs() < 1e-9);
        // Continuous at the knee of CrtProper2.
        let below = crt_proper2_to_linear(0.359_999);
        let above = crt_proper2_to_linear(0.36);
        assert!((below - above).abs() < 1e-4);
    }

    #[test]
    fn the_knobs_turn_scale_and_lift() {
        let knobs = TvKnobs {
            hue: 0,
            saturation: 0,
            contrast: 100,
            brightness: 0,
        };
        let (_, i, q) = knobs.adjust(0.5, 0.2, 0.1);
        assert!(i.abs() < 1e-6 && q.abs() < 1e-6);
        let lifted = TvKnobs {
            brightness: 10,
            ..TvKnobs::NEUTRAL
        };
        assert!((lifted.adjust(0.5, 0.0, 0.0).0 - 0.6).abs() < 1e-6);
    }
}
