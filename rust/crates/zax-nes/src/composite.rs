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
    let row = if hue > 13 {
        1
    } else {
        usize::from((pixel >> 4) & 3)
    };
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
        let angle =
            std::f32::consts::PI * ((phase as f32 - 0.5) / 6.0 + 0.5) + hue_degrees.to_radians();
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn emphasis_darkens() {
        let plain = decode_uniform(0x30, 0, 0.0);
        let red = decode_uniform(0x30, 1, 0.0);
        assert!(red.iter().sum::<f32>() < plain.iter().sum::<f32>());
        // Red emphasis keeps red the strongest of the three.
        assert!(red[0] >= red[1] && red[0] >= red[2], "{red:?}");
    }

    #[test]
    fn the_phase_advances_four_per_line() {
        assert_eq!(4, next_line_phase(0, false));
        assert_eq!(8, next_line_phase(0, true));
        assert_eq!(8, pixel_phase(0, 1));
        assert_eq!(4, pixel_phase(0, 2));
    }
}
