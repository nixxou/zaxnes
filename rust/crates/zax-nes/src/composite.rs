//! Lot FIDPLUS-D step D2a (`docs/design/ANALOG-OUTPUT.md`): the 2C02's composite video signal,
//! generated per pixel, and a reference decoder.
//!
//! The 2C02 makes its picture as a square wave: during each pixel it outputs, at each of the
//! 12 phases of the colour subcarrier (3.579545 MHz, the master clock / 6, sampled on both master
//! clock edges), either the "high" or the "low" level of the pixel's brightness row. A pixel lasts
//! 4 master clocks, 8 of those half-clock samples, so the phase advances by 8 (mod 12) per pixel.
//! The emphasis bits attenuate the signal during the phases of their colour. Reference: nesdev,
//! "NTSC video" (the voltage levels, the colour-phase rule and the emphasis rule below); recorded
//! as FIDELITY-LEADS H2.
//!
//! Nothing here runs unless a host asks for it: the core's picture is unchanged.

/// Half-clock samples in one colour subcarrier cycle.
pub const PHASES: usize = 12;

/// Half-clock samples a pixel lasts.
pub const SAMPLES_PER_PIXEL: usize = 8;

/// The signal's levels in volts (nesdev "NTSC video", terminated measurement): the low level of
/// rows 0-3, then the high level of rows 0-3.
const LEVELS: [f32; 8] = [0.228, 0.312, 0.552, 0.880, 0.616, 0.840, 1.100, 1.100];

/// The same levels with an emphasis bit attenuating them.
const ATTENUATED: [f32; 8] = [0.192, 0.256, 0.448, 0.712, 0.484, 0.668, 0.896, 0.896];

/// Black: the low level of row 1, also what colours `$xE` / `$xF` output.
pub const BLACK: f32 = LEVELS[1];

/// White: the high level of row 3.
pub const WHITE: f32 = LEVELS[7];

/// Whether colour phase `hue` (1 to 12) is in its high half at subcarrier phase `phase`.
const fn in_colour_phase(hue: usize, phase: usize) -> bool {
    (hue + phase) % PHASES < 6
}

/// The signal, in volts, of palette entry `pixel` (6 bits: hue in bits 0-3, row in bits 4-5)
/// with the emphasis bits `emphasis` (`$2001` bits 5-7 as 0-7: red, green, blue on NTSC) at
/// subcarrier phase `phase` (0 to 11).
#[must_use]
pub fn signal(pixel: u8, emphasis: u8, phase: usize) -> f32 {
    let hue = usize::from(pixel & 0x0F);
    if hue > 13 {
        // `$xE` / `$xF` select the blanking level, which no emphasis bit attenuates (the 2C02
        // netlist, `ext-tools/visual2c07-tools/benchvid.cpp` on VisualNes's 2C02: the whole
        // table matches this generator only so; lot FIDPLUS-D2c).
        return BLACK;
    }
    let row = usize::from((pixel >> 4) & 3);
    let phase = phase % PHASES;
    let high = if hue > 12 { row } else { row + 4 };
    let low = if hue == 0 { row + 4 } else { row };
    let index = if in_colour_phase(hue, phase) {
        high
    } else {
        low
    };
    let attenuated = (emphasis & 1 != 0 && in_colour_phase(0, phase))
        || (emphasis & 2 != 0 && in_colour_phase(4, phase))
        || (emphasis & 4 != 0 && in_colour_phase(8, phase));
    if attenuated {
        ATTENUATED[index]
    } else {
        LEVELS[index]
    }
}

/// The 8 samples of one pixel starting at subcarrier phase `phase`, into `out`.
pub fn pixel_samples(pixel: u8, emphasis: u8, phase: usize, out: &mut [f32; SAMPLES_PER_PIXEL]) {
    for (offset, sample) in out.iter_mut().enumerate() {
        *sample = signal(pixel, emphasis, phase + offset);
    }
}

/// The phase a pixel starts at: 8 half-clocks per dot, from the line's starting phase.
#[must_use]
pub const fn pixel_phase(line_phase: usize, dot: usize) -> usize {
    (line_phase + dot * SAMPLES_PER_PIXEL) % PHASES
}

/// The phase the next line starts at: a line of 341 dots is 2728 half-clocks, 4 more than a
/// whole number of subcarrier cycles; a short line (the odd frame's skipped dot) is 8 fewer.
#[must_use]
pub const fn next_line_phase(line_phase: usize, short_line: bool) -> usize {
    let dots = if short_line { 340 } else { 341 };
    (line_phase + dots * SAMPLES_PER_PIXEL) % PHASES
}

/// A reference decoder for one colour, as a television sees a field of it: the signal over a
/// whole subcarrier cycle demodulated into Y, I and Q (luma the mean; chroma against the
/// subcarrier, `hue_degrees` the set's tint), then the FCC YIQ matrix to RGB, 0 to 1, clamped.
/// Black is 0, white 1 (the blanking-to-white span of the levels above).
#[must_use]
pub fn decode_uniform(pixel: u8, emphasis: u8, hue_degrees: f32) -> [f32; 3] {
    let span = WHITE - BLACK;
    let (mut y, mut i, mut q) = (0.0_f32, 0.0_f32, 0.0_f32);
    for phase in 0..PHASES {
        let level = (signal(pixel, emphasis, phase) - BLACK) / span;
        #[allow(clippy::cast_precision_loss)] // phase < 12
        let angle = std::f32::consts::PI * ((phase as f32 - 0.5) / 6.0 + 0.5)
            + (HUE_OFFSET + hue_degrees).to_radians();
        y += level;
        i += level * angle.cos();
        q += level * angle.sin();
    }
    #[allow(clippy::cast_precision_loss)]
    let count = PHASES as f32;
    let (y, i, q) = (y / count, 2.0 * i / count, 2.0 * q / count);
    let red = y + 0.956 * i + 0.621 * q;
    let green = y - 0.272 * i - 0.647 * q;
    let blue = y - 1.106 * i + 1.703 * q;
    [
        red.clamp(0.0, 1.0),
        green.clamp(0.0, 1.0),
        blue.clamp(0.0, 1.0),
    ]
}

/// Samples a television's luma filter averages: one subcarrier cycle, which removes the chroma
/// of a flat colour entirely and leaves the colour artefacts of its edges.
const LUMA_WINDOW: usize = PHASES;

/// Samples its chroma filter averages: two cycles, the narrower chroma band of a set.
const CHROMA_WINDOW: usize = 2 * PHASES;

/// The decoder's hue reference against the subcarrier phase, in degrees: fitted (1-degree steps)
/// so that the 64 colours decoded from a flat field come closest to `TriCNES`'s palette, the error
/// over the 52 coloured entries going from 627 to 165 (lot FIDPLUS-D2b; at 0 the sky of Super Mario
/// Bros. `$22` decoded magenta). A television's tint knob turns the same angle.
const HUE_OFFSET: f32 = 45.0;

/// The demodulation angle of subcarrier phase `phase`, the one [`decode_uniform`] uses.
fn phase_angle(phase: usize) -> f32 {
    #[allow(clippy::cast_precision_loss)] // phase < 12
    let phase = phase as f32;
    std::f32::consts::PI * ((phase - 0.5) / 6.0 + 0.5) + HUE_OFFSET.to_radians()
}

/// What [`RowDecoder::decode_row`] reads per sample, computed once: the signal's level (0 black,
/// 1 white) of every colour, emphasis and subcarrier phase, and the demodulation's cosine and
/// sine of every phase.
struct DecodeTables {
    levels: Vec<f32>,
    cos: [f32; PHASES],
    sin: [f32; PHASES],
}

fn decode_tables() -> &'static DecodeTables {
    static TABLES: std::sync::OnceLock<DecodeTables> = std::sync::OnceLock::new();
    TABLES.get_or_init(|| {
        let span = WHITE - BLACK;
        let mut levels = Vec::with_capacity(64 * 8 * PHASES);
        for pixel in 0..64u8 {
            for emphasis in 0..8u8 {
                for phase in 0..PHASES {
                    levels.push((signal(pixel, emphasis, phase) - BLACK) / span);
                }
            }
        }
        let mut cos = [0.0; PHASES];
        let mut sin = [0.0; PHASES];
        for phase in 0..PHASES {
            let angle = phase_angle(phase);
            cos[phase] = angle.cos();
            sin[phase] = angle.sin();
        }
        DecodeTables { levels, cos, sin }
    })
}

/// Working buffers of [`decode_row`], kept by the caller from row to row.
#[derive(Debug, Default)]
pub struct RowDecoder {
    luma: Vec<f32>,
    in_phase: Vec<f32>,
    quadrature: Vec<f32>,
    /// PAL: the previous line's U and V per pixel and its number, for the delay line.
    previous_u: Vec<f32>,
    previous_v: Vec<f32>,
    previous_line: Option<i64>,
}

impl RowDecoder {
    /// Decodes one row of the picture as a PAL television sees the 2C07's signal (lot
    /// FIDPLUS-D2c): `line` the picture line (0 the first; the rows of a widescreen margin above
    /// are negative), `first_x` the screen column of `pixels[0]` (negative in a left margin).
    /// Luma over one subcarrier cycle, U and V over two against the burst's axis, V's sign
    /// switched every other line, then the set's delay line: U and V averaged with the line above
    /// when it was decoded just before (PAL-D). Writes R, G, B (0 to 255) per pixel.
    pub fn decode_row_pal(
        &mut self,
        pixels: &[u8],
        emphasis: &[u8],
        line: i64,
        first_x: i64,
        out: &mut [u8],
    ) {
        let samples = pixels.len() * PAL_SAMPLES_PER_PIXEL;
        for sums in [&mut self.luma, &mut self.in_phase, &mut self.quadrature] {
            sums.clear();
            sums.resize(samples + 1, 0.0);
        }
        let tables = pal_decode_tables();
        let odd = line.rem_euclid(2) == 1;
        let sign = pal_v_sign(odd);
        let (mut y, mut u, mut v) = (0.0_f32, 0.0_f32, 0.0_f32);
        let mut at = pal_pixel_phase(line, first_x);
        let mut index = 1;
        for (x, &pixel) in pixels.iter().enumerate() {
            let bits = emphasis.get(x).copied().unwrap_or(0) & 7;
            let base = ((usize::from(pixel & 0x3F) << 3 | usize::from(bits)) * 2
                + usize::from(odd))
                * PHASES;
            let levels = &tables.levels[base..base + PHASES];
            for _ in 0..PAL_SAMPLES_PER_PIXEL {
                let level = levels[at];
                y += level;
                u += level * tables.cos[at];
                v += sign * level * tables.sin[at];
                self.luma[index] = y;
                self.in_phase[index] = u;
                self.quadrature[index] = v;
                index += 1;
                at += 1;
                if at == PHASES {
                    at = 0;
                }
            }
        }
        let window = |sums: &[f32], centre: usize, width: usize| -> f32 {
            let from = centre.saturating_sub(width / 2);
            let to = (from + width).min(samples);
            let from = to.saturating_sub(width);
            #[allow(clippy::cast_precision_loss)] // at most 24
            let count = (to - from) as f32;
            (sums[to] - sums[from]) / count
        };
        let delayed = self.previous_line == Some(line - 1) && self.previous_u.len() == pixels.len();
        self.previous_u.resize(pixels.len(), 0.0);
        self.previous_v.resize(pixels.len(), 0.0);
        for (x, rgb) in out.chunks_exact_mut(3).take(pixels.len()).enumerate() {
            let centre = x * PAL_SAMPLES_PER_PIXEL + PAL_SAMPLES_PER_PIXEL / 2;
            let y = window(&self.luma, centre, LUMA_WINDOW);
            let mut u = 2.0 * window(&self.in_phase, centre, CHROMA_WINDOW);
            let mut v = 2.0 * window(&self.quadrature, centre, CHROMA_WINDOW);
            let (here_u, here_v) = (u, v);
            if delayed {
                u = f32::midpoint(u, self.previous_u[x]);
                v = f32::midpoint(v, self.previous_v[x]);
            }
            self.previous_u[x] = here_u;
            self.previous_v[x] = here_v;
            for (byte, value) in rgb.iter_mut().zip(yuv_to_rgb(y, u, v)) {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0 to 255
                let value = (value * 255.0).round() as u8;
                *byte = value;
            }
        }
        self.previous_line = Some(line);
    }

    /// Decodes one row of the picture as a television would see the 2C02's signal (lot
    /// FIDPLUS-D2b step 2): `pixels` colour indices (6 bits used), `emphasis` the emphasis of
    /// each (0 to 7; empty for none), `phase` the subcarrier phase of the row's first sample.
    /// Writes R, G, B (0 to 255) per pixel into `out` (three bytes a pixel).
    ///
    /// The signal is generated 8 samples a pixel ([`pixel_samples`]); luma is its mean over one
    /// subcarrier cycle around each pixel's centre, I and Q its products with the subcarrier over
    /// two cycles, then the matrix of [`decode_uniform`]. A flat colour decodes exactly as
    /// `decode_uniform` does; where colours meet, luma and chroma leak into each other as on a
    /// set (the colour fringes), and since the phase moves by 4 a line and by 4 or 8 a frame the
    /// fringes move with it (the dot crawl).
    pub fn decode_row(&mut self, pixels: &[u8], emphasis: &[u8], phase: usize, out: &mut [u8]) {
        let samples = pixels.len() * SAMPLES_PER_PIXEL;
        // Running sums: entry k is the sum of the first k samples.
        for sums in [&mut self.luma, &mut self.in_phase, &mut self.quadrature] {
            sums.clear();
            sums.reserve(samples + 1);
            sums.push(0.0);
        }
        let tables = decode_tables();
        self.luma.resize(samples + 1, 0.0);
        self.in_phase.resize(samples + 1, 0.0);
        self.quadrature.resize(samples + 1, 0.0);
        let (mut y, mut i, mut q) = (0.0_f32, 0.0_f32, 0.0_f32);
        let mut at = phase % PHASES;
        let mut index = 1;
        for (x, &pixel) in pixels.iter().enumerate() {
            let bits = emphasis.get(x).copied().unwrap_or(0) & 7;
            let levels = &tables.levels
                [(usize::from(pixel & 0x3F) << 3 | usize::from(bits)) * PHASES..][..PHASES];
            for _ in 0..SAMPLES_PER_PIXEL {
                let level = levels[at];
                y += level;
                i += level * tables.cos[at];
                q += level * tables.sin[at];
                self.luma[index] = y;
                self.in_phase[index] = i;
                self.quadrature[index] = q;
                index += 1;
                at += 1;
                if at == PHASES {
                    at = 0;
                }
            }
        }
        let window = |sums: &[f32], centre: usize, width: usize| -> f32 {
            let from = centre.saturating_sub(width / 2);
            let to = (from + width).min(samples);
            let from = to.saturating_sub(width);
            #[allow(clippy::cast_precision_loss)] // at most 24
            let count = (to - from) as f32;
            (sums[to] - sums[from]) / count
        };
        for (x, rgb) in out.chunks_exact_mut(3).take(pixels.len()).enumerate() {
            let centre = x * SAMPLES_PER_PIXEL + SAMPLES_PER_PIXEL / 2;
            let y = window(&self.luma, centre, LUMA_WINDOW);
            let i = 2.0 * window(&self.in_phase, centre, CHROMA_WINDOW);
            let q = 2.0 * window(&self.quadrature, centre, CHROMA_WINDOW);
            let channels = [
                y + 0.956 * i + 0.621 * q,
                y - 0.272 * i - 0.647 * q,
                y - 1.106 * i + 1.703 * q,
            ];
            for (byte, value) in rgb.iter_mut().zip(channels) {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0 to 255
                let value = (value.clamp(0.0, 1.0) * 255.0).round() as u8;
                *byte = value;
            }
        }
    }
}

/// The phase the next frame's first line starts at: a frame of 262 lines of 341 dots advances
/// the subcarrier by 4 half-clocks (mod 12), 8 fewer when the odd frame skips its first dot.
#[must_use]
pub const fn next_frame_phase(frame_phase: usize, skipped_dot: bool) -> usize {
    let advance = if skipped_dot { PHASES - 4 } else { 4 };
    (frame_phase + advance) % PHASES
}

// ----- Lot FIDPLUS-D2c: the PAL 2C07 -----------------------------------------------------------
//
// Measured on the Visual 2C07 netlist (`ext-tools/visual2c07-tools/benchvid.cpp`, every colour and
// emphasis on two consecutive lines, the DAC's level select read every master half-clock;
// `vidmodel.py` reproduces all 1 024 records). The 2C07's subcarrier (4.43 MHz) is also the
// master clock / 6, 12 half-clock phases a cycle; a pixel lasts 5 master clocks, 10 phases; a line
// of 341 dots is 3 410 half-clocks, 2 more than a whole number of cycles; a frame of 312 lines is
// a whole number of cycles, so the pattern does not crawl from frame to frame. The phases below
// count from the netlist bench's half-clock counter; only their differences reach a television,
// which locks on the burst.
//
// * colour `h` (1 to 12) is at its row's high level for six phases from `h` on odd lines and from
//   `5 - h` on even lines: the 2C07 mirrors the phase every other line, as PAL's V switch wants;
// * the burst is high for six phases from 10 on odd lines, 7 on even ones (90 degrees apart);
// * emphasis bit 0 attenuates six phases from 5 on odd lines, 1 on even; bit 1 from 1 on odd, 5 on
//   even (the two bits trade places with the line); bit 2 from 9 on both;
// * colours `$xE` / `$xF` output the blanking level and no emphasis touches them;
// * the first pixel of a picture line starts at phase `4 + 2 line` (pixel `x` at `+ 10 x`).
//
// The levels are the 2C02's (`LEVELS`, `ATTENUATED`): the netlist gives the select, not the
// resistor ladder's voltages (FIDELITY-LEADS H2).

/// Half-clock samples a 2C07 pixel lasts.
pub const PAL_SAMPLES_PER_PIXEL: usize = 10;

/// What [`crate::NesCore::video_phase`] returns on the 2C07: the PAL decoder's marker (its phases
/// depend on the line alone).
pub const PAL_VIDEO: usize = 1 << 8;

/// Whether `phase` falls in the six phases from `start`.
const fn in_half_cycle(start: usize, phase: usize) -> bool {
    (phase + PHASES - start % PHASES) % PHASES < 6
}

/// The 2C07's emphasis windows: per bit, the first phase on even and on odd lines.
const PAL_EMPHASIS_START: [[usize; 2]; 3] = [[1, 5], [5, 1], [9, 9]];

/// The 2C07's signal, in volts, of palette entry `pixel` with emphasis `emphasis` (`$2001` bits
/// 5-7 as 0-7) at phase `phase` of a line of parity `odd`.
#[must_use]
pub fn pal_signal(pixel: u8, emphasis: u8, phase: usize, odd: bool) -> f32 {
    let hue = usize::from(pixel & 0x0F);
    if hue >= 14 {
        return BLACK;
    }
    let row = usize::from((pixel >> 4) & 3);
    let phase = phase % PHASES;
    let index = match hue {
        0 => row + 4,
        13 => row,
        _ => {
            let start = if odd { hue } else { (PHASES + 5 - hue) % PHASES };
            if in_half_cycle(start, phase) {
                row + 4
            } else {
                row
            }
        }
    };
    let attenuated = (0..3).any(|bit| {
        emphasis >> bit & 1 != 0 && in_half_cycle(PAL_EMPHASIS_START[bit][usize::from(odd)], phase)
    });
    if attenuated {
        ATTENUATED[index]
    } else {
        LEVELS[index]
    }
}

/// The phase of the first sample of pixel `x` of picture line `line` (negative values: the
/// widescreen margins before the picture).
#[must_use]
pub const fn pal_pixel_phase(line: i64, x: i64) -> usize {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0..12
    let phase = (4 + 2 * line + 10 * x).rem_euclid(12) as usize;
    phase
}

/// The burst's first high phase on a line of parity `odd`.
#[must_use]
pub const fn pal_burst_start(odd: bool) -> usize {
    if odd {
        10
    } else {
        7
    }
}

/// The real subcarrier angle of phase `phase`, in degrees: the subcarrier runs backwards against
/// the phase count (on the 2C02 the same bench puts colour `$x8` in phase with the burst and the
/// hues then go round as a television decodes them only this way).
fn pal_phase_angle(phase: f32) -> f32 {
    -30.0 * (phase + 0.5)
}

/// The U axis, in degrees: opposite the mean of the two lines' burst angles, as a PAL set locks.
fn pal_u_axis() -> f32 {
    // A six-phase square wave's fundamental points at its centre, 2.5 phases past its start.
    let odd = pal_phase_angle(f32::from(u8::try_from(pal_burst_start(true)).unwrap_or(0)) + 2.5);
    let even = pal_phase_angle(f32::from(u8::try_from(pal_burst_start(false)).unwrap_or(0)) + 2.5);
    let (sin, cos) = (
        odd.to_radians().sin() + even.to_radians().sin(),
        odd.to_radians().cos() + even.to_radians().cos(),
    );
    sin.atan2(cos).to_degrees() + 180.0
}

/// The sign of V on a line of parity `odd`. The set reads the V switch from the burst's swing;
/// with this chip's two bursts, taking the line whose burst lies 45 degrees after the U axis' back
/// as the inverted one gives the hues of the 2C02 turned by -15 degrees (the PAL console's known
/// tint), the other choice every hue mirrored about the U axis.
const fn pal_v_sign(odd: bool) -> f32 {
    if odd {
        -1.0
    } else {
        1.0
    }
}

/// What [`RowDecoder::decode_row_pal`] reads per sample: the level (0 black, 1 white) of every
/// colour, emphasis, line parity and phase, and the demodulation's cosine and sine against the U
/// axis.
struct PalDecodeTables {
    levels: Vec<f32>,
    cos: [f32; PHASES],
    sin: [f32; PHASES],
}

fn pal_decode_tables() -> &'static PalDecodeTables {
    static TABLES: std::sync::OnceLock<PalDecodeTables> = std::sync::OnceLock::new();
    TABLES.get_or_init(|| {
        let span = WHITE - BLACK;
        let mut levels = Vec::with_capacity(64 * 8 * 2 * PHASES);
        for pixel in 0..64u8 {
            for emphasis in 0..8u8 {
                for odd in [false, true] {
                    for phase in 0..PHASES {
                        levels.push((pal_signal(pixel, emphasis, phase, odd) - BLACK) / span);
                    }
                }
            }
        }
        let axis = pal_u_axis();
        let mut cos = [0.0; PHASES];
        let mut sin = [0.0; PHASES];
        for phase in 0..PHASES {
            #[allow(clippy::cast_precision_loss)] // phase < 12
            let angle = (pal_phase_angle(phase as f32) - axis).to_radians();
            cos[phase] = angle.cos();
            sin[phase] = angle.sin();
        }
        PalDecodeTables { levels, cos, sin }
    })
}

/// A flat field of one colour on a PAL set: Y, U and V over a whole subcarrier cycle of each line
/// parity, the two lines averaged by the set's delay line, then the PAL YUV matrix. 0 to 1.
#[must_use]
pub fn pal_decode_uniform(pixel: u8, emphasis: u8) -> [f32; 3] {
    let tables = pal_decode_tables();
    let (mut y, mut u, mut v) = (0.0_f32, 0.0_f32, 0.0_f32);
    for odd in [false, true] {
        let base = ((usize::from(pixel & 0x3F) << 3 | usize::from(emphasis & 7)) * 2
            + usize::from(odd))
            * PHASES;
        for phase in 0..PHASES {
            let level = tables.levels[base + phase];
            y += level;
            u += level * tables.cos[phase];
            v += pal_v_sign(odd) * level * tables.sin[phase];
        }
    }
    #[allow(clippy::cast_precision_loss)]
    let count = (2 * PHASES) as f32;
    yuv_to_rgb(y / count, 2.0 * u / count, 2.0 * v / count)
}

/// The PAL YUV matrix, clamped to 0..1.
fn yuv_to_rgb(y: f32, u: f32, v: f32) -> [f32; 3] {
    [
        (y + 1.140 * v).clamp(0.0, 1.0),
        (y - 0.395 * u - 0.581 * v).clamp(0.0, 1.0),
        (y + 2.032 * u).clamp(0.0, 1.0),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fidplus_d2b_a_flat_row_decodes_as_its_uniform_colour() {
        let mut decoder = RowDecoder::default();
        for pixel in [0x16_u8, 0x2A, 0x12, 0x30, 0x0F] {
            for phase in [0, 4, 8] {
                let row = [pixel; 64];
                let mut out = [0u8; 64 * 3];
                decoder.decode_row(&row, &[], phase, &mut out);
                let expected = decode_uniform(pixel, 0, 0.0);
                for (channel, &value) in expected.iter().enumerate() {
                    let got = f32::from(out[32 * 3 + channel]) / 255.0;
                    assert!(
                        (got - value).abs() < 0.01,
                        "{pixel:#x} phase {phase}: {got} {value}"
                    );
                }
            }
        }
    }

    #[test]
    fn fidplus_d2b_an_edge_fringes_and_the_fringe_moves_with_the_phase() {
        let mut decoder = RowDecoder::default();
        let mut row = [0x0F_u8; 64];
        row[32..].fill(0x30);
        let decode = |decoder: &mut RowDecoder, phase| {
            let mut out = [0u8; 64 * 3];
            decoder.decode_row(&row, &[], phase, &mut out);
            out
        };
        let a = decode(&mut decoder, 0);
        let b = decode(&mut decoder, 4);
        // Black to white: the pixels at the edge are coloured (not grey), and differently on two
        // phases; far from it, black and white.
        let edge = &a[32 * 3..33 * 3];
        assert!(
            edge.iter().max().unwrap() - edge.iter().min().unwrap() > 8,
            "{edge:?}"
        );
        assert_ne!(&a[31 * 3..34 * 3], &b[31 * 3..34 * 3]);
        assert!(a[..3].iter().all(|&c| c < 3) && a[60 * 3..61 * 3].iter().all(|&c| c > 240));
    }

    #[test]
    fn grey_rows_have_no_chroma() {
        for row in 0..4u8 {
            let pixel = row << 4;
            let first = signal(pixel, 0, 0);
            assert!((0..PHASES).all(|phase| (signal(pixel, 0, phase) - first).abs() < 1e-6));
        }
        // $0D is blacker than black; $xE and $xF are black.
        assert!(signal(0x0D, 0, 0) < BLACK);
        assert!((signal(0x0F, 0, 3) - BLACK).abs() < 1e-6);
        assert!((signal(0x3E, 0, 7) - BLACK).abs() < 1e-6);
    }

    #[test]
    fn a_hue_is_high_for_half_a_cycle() {
        for hue in 1..=12u8 {
            let high = (0..PHASES)
                .filter(|&phase| signal(0x10 | hue, 0, phase) > 0.7)
                .count();
            assert_eq!(6, high, "hue {hue}");
        }
    }

    #[test]
    fn decoded_colours_look_like_their_names() {
        let white = decode_uniform(0x20, 0, 0.0);
        assert!(white.iter().all(|&c| c > 0.95), "{white:?}");
        let black = decode_uniform(0x0F, 0, 0.0);
        assert!(black.iter().all(|&c| c < 0.01), "{black:?}");
        let [r, g, b] = decode_uniform(0x16, 0, 0.0);
        assert!(r > g && r > b, "red {r} {g} {b}");
        let [r, g, b] = decode_uniform(0x1A, 0, 0.0);
        assert!(g > r && g > b, "green {r} {g} {b}");
        let [r, g, b] = decode_uniform(0x12, 0, 0.0);
        assert!(b > r && b > g, "blue {r} {g} {b}");
    }

    #[test]
    #[ignore = "timing, run with --release --ignored --nocapture"]
    fn fidplus_d2b_cost_of_one_frame() {
        let mut decoder = RowDecoder::default();
        let row: Vec<u8> = (0..=255u8).map(|x| (x / 3) & 0x3F).collect();
        let mut out = vec![0u8; 256 * 3];
        let frames: u32 = 200;
        let start = std::time::Instant::now();
        for frame in 0..frames as usize {
            for line in 0..240 {
                decoder.decode_row(&row, &[], (frame * 4 + line * 4) % PHASES, &mut out);
            }
        }
        let per_frame = start.elapsed().as_secs_f64() * 1000.0 / f64::from(frames);
        println!("composite decode: {per_frame:.3} ms a frame");
    }

    #[test]
    fn fidplus_d2b_key_colours_decode_near_tricnes() {
        // TriCNES's plain colours (MIT): Super Mario Bros.'s sky $22, an orange $27, the cream
        // $37 of Super Mario Bros. 3, a green $2A. Each channel within 40 of 255.
        for (pixel, expected) in [
            (0x22_u8, [147.0_f32, 144.0, 255.0]),
            (0x27, [230.0, 158.0, 37.0]),
            (0x37, [245.0, 215.0, 165.0]),
            (0x2A, [94.0, 225.0, 50.0]),
        ] {
            let got = decode_uniform(pixel, 0, 0.0).map(|c| c * 255.0);
            for (g, e) in got.iter().zip(expected) {
                assert!(
                    (g - e).abs() < 40.0,
                    "{pixel:#x}: {got:?} against {expected:?}"
                );
            }
        }
    }

    #[test]
    fn emphasis_darkens() {
        let plain = decode_uniform(0x30, 0, 0.0);
        let red = decode_uniform(0x30, 1, 0.0);
        assert!(red.iter().sum::<f32>() < plain.iter().sum::<f32>());
        // Red emphasis keeps red the strongest of the three.
        assert!(red[0] >= red[1] && red[0] >= red[2], "{red:?}");
    }

    #[test]
    fn fidplus_d2c_pal_hues_are_the_ntsc_ones_turned_and_flat_rows_decode_uniformly() {
        // The PAL console's colours: red still red, green green, blue blue, white and black.
        let [r, g, b] = pal_decode_uniform(0x16, 0);
        assert!(r > g && r > b, "red {r} {g} {b}");
        let [r, g, b] = pal_decode_uniform(0x1A, 0);
        assert!(g > r && g > b, "green {r} {g} {b}");
        let [r, g, b] = pal_decode_uniform(0x12, 0);
        assert!(b > r && b > g, "blue {r} {g} {b}");
        assert!(pal_decode_uniform(0x30, 0).iter().all(|&c| c > 0.95));
        assert!(pal_decode_uniform(0x0F, 0).iter().all(|&c| c < 0.01));
        // A flat field: a row after its predecessor decodes as the uniform colour.
        let mut decoder = RowDecoder::default();
        let mut out = [0u8; 64 * 3];
        for pixel in [0x16_u8, 0x2A, 0x21, 0x37] {
            for line in 10..12 {
                decoder.decode_row_pal(&[pixel; 64], &[], line, 0, &mut out);
            }
            let expected = pal_decode_uniform(pixel, 0);
            for (channel, &value) in expected.iter().enumerate() {
                let got = f32::from(out[32 * 3 + channel]) / 255.0;
                assert!((got - value).abs() < 0.02, "{pixel:#x}: {got} {value}");
            }
        }
    }

    #[test]
    fn fidplus_d2c_pal_signal_follows_the_netlist() {
        // Records of `benchvid` (Visual 2C07): $16 high for six phases from 6 on odd lines and
        // from 11 on even ones; red emphasis on white attenuates phases 5-10 on odd lines; $0E
        // stays at the blanking level whatever the emphasis.
        for phase in 0..PHASES {
            let odd = signal_high(0x16, phase, true);
            let even = signal_high(0x16, phase, false);
            assert_eq!((6..12).contains(&phase), odd, "odd {phase}");
            assert_eq!(phase == 11 || phase < 5, even, "even {phase}");
            let attenuated = pal_signal(0x30, 1, phase, true) < LEVELS[7];
            assert_eq!((5..11).contains(&phase), attenuated, "emphasis {phase}");
            assert!((pal_signal(0x0E, 7, phase, true) - BLACK).abs() < 1e-6);
        }
        // Line 21 of the bench started its first pixel at phase 10; a pixel later, 8.
        assert_eq!((10, 8), (pal_pixel_phase(21, 0), pal_pixel_phase(21, 1)));
    }

    fn signal_high(pixel: u8, phase: usize, odd: bool) -> bool {
        pal_signal(pixel, 0, phase, odd) > LEVELS[usize::from(pixel >> 4)] + 0.01
    }

    #[test]
    fn the_phase_advances_four_per_line() {
        assert_eq!(4, next_line_phase(0, false));
        assert_eq!(8, next_line_phase(0, true));
        assert_eq!(8, pixel_phase(0, 1));
        assert_eq!(4, pixel_phase(0, 2));
    }
}
