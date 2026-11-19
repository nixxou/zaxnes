//! The picture settings the core options choose (the application's Signal tab and its colour
//! emphasis in 3D): the television and its knobs, the composite signal decoder, and the scene's
//! emphasis. `tv_look`, `composite_options` and `decode_composite` are copies of
//! `zax-app/src/application.rs`'s (the application crate is out of a core's reach): keep them
//! the same.

use zax_nes::composite::{
    DecoderOptions, Neighbours, RowDecoder, Sharpness, Signal, PAL_VIDEO, PHASES,
};
use zax_nes::tv::{TvKnobs, TvLook, TvModel};
use zax_settings::catalogue::SettingId;

/// The picture settings, each as the catalogue's value of its setting.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Picture {
    pub values: Vec<(SettingId, i32)>,
}

/// The settings the core exposes in its Image submenu, then the 3D one.
pub const SETTINGS: [SettingId; 13] = [
    SettingId::VideoTvModel,
    SettingId::VideoTvHue,
    SettingId::VideoTvSaturation,
    SettingId::VideoTvContrast,
    SettingId::VideoTvBrightness,
    SettingId::VideoCompositeSignal,
    SettingId::VideoSignalKind,
    SettingId::VideoDecoderSharpness,
    SettingId::VideoCombFilter,
    SettingId::VideoFrozenDotCrawl,
    SettingId::VideoRfNoise,
    SettingId::VideoRfGhost,
    SettingId::VideoEmphasis3d,
];

impl Picture {
    /// The value of `id`, or its catalogue default.
    fn value(&self, id: SettingId) -> i32 {
        self.values
            .iter()
            .find(|(setting, _)| *setting == id)
            .map_or_else(|| crate::machine::default_value(id), |(_, value)| *value)
    }

    /// `video.emphasis_3d`: the scene takes the picture's most common colour emphasis.
    #[must_use]
    pub fn emphasis_in_three_d(&self) -> bool {
        self.value(SettingId::VideoEmphasis3d) != 0
    }

    /// The television of the signal options: `video.tv_model` and its knobs.
    #[must_use]
    pub fn tv_look(&self) -> TvLook {
        let knob = |id: SettingId, low: i32, high: i32| self.value(id).clamp(low, high);
        TvLook {
            model: TvModel::from_index(self.value(SettingId::VideoTvModel)),
            knobs: TvKnobs {
                hue: i8::try_from(knob(SettingId::VideoTvHue, -30, 30)).unwrap_or(0),
                saturation: u8::try_from(knob(SettingId::VideoTvSaturation, 0, 200)).unwrap_or(100),
                contrast: u8::try_from(knob(SettingId::VideoTvContrast, 50, 150)).unwrap_or(100),
                brightness: i8::try_from(knob(SettingId::VideoTvBrightness, -50, 50)).unwrap_or(0),
            },
        }
    }

    /// What the composite decoder is set to (`video.composite_signal` on), and whether the dot
    /// crawl is frozen; `None` with the signal off.
    #[must_use]
    pub fn composite(&self) -> Option<(DecoderOptions, bool)> {
        if self.value(SettingId::VideoCompositeSignal) == 0 {
            return None;
        }
        #[allow(clippy::cast_precision_loss)] // 0 to 100
        let fraction = |id: SettingId| self.value(id).clamp(0, 100) as f32 / 100.0;
        let options = DecoderOptions {
            signal: match self.value(SettingId::VideoSignalKind) {
                1 => Signal::SVideo,
                2 => Signal::Rf,
                _ => Signal::Composite,
            },
            sharpness: match self.value(SettingId::VideoDecoderSharpness) {
                0 => Sharpness::Soft,
                2 => Sharpness::Sharp,
                _ => Sharpness::Normal,
            },
            comb: self.value(SettingId::VideoCombFilter) != 0,
            rf_noise: fraction(SettingId::VideoRfNoise),
            rf_ghost: fraction(SettingId::VideoRfGhost),
            look: self.tv_look(),
        };
        Some((options, self.value(SettingId::VideoFrozenDotCrawl) != 0))
    }
}

/// The colour emphasis most of `emphasis` carries (an empty plane: none; ties to the lower).
#[must_use]
pub fn common_emphasis(emphasis: &[u8]) -> u8 {
    let mut counts = [0usize; 8];
    for &bits in emphasis {
        counts[usize::from(bits & 7)] += 1;
    }
    (0..8u8)
        .rev()
        .max_by_key(|&bits| counts[usize::from(bits)])
        .filter(|_| !emphasis.is_empty())
        .unwrap_or(0)
}

/// The picture `picture` (`width` × `rows` NES colour indices, its emphasis plane beside it)
/// decoded from the composite signal, row by row, into `out` (RGB, 3 bytes a pixel). Row `r`
/// starts its subcarrier 4 half-clocks after row `r - 1`; a widescreen margin column `c` before
/// the screen is `8 c` half-clocks earlier.
pub fn decode_composite(
    decoder: &mut RowDecoder,
    picture: &[u8],
    emphasis: &[u8],
    (width, rows): (usize, usize),
    video_phase: usize,
    out: &mut Vec<u8>,
) {
    out.clear();
    if width == 0 || rows == 0 || picture.len() < width * rows {
        return;
    }
    let margin = width.saturating_sub(256) / 2;
    let margin_y = rows.saturating_sub(240) / 2;
    out.resize(width * rows * 3, 0);
    let emphasis_ok = emphasis.len() == picture.len();
    for row in 0..rows {
        // Phases mod 12, the margins counted backwards from the screen's first row and column.
        let line = (row + PHASES * 60 - margin_y % PHASES) % PHASES;
        let phase = (video_phase + 4 * line + PHASES * 1000 - (margin * 8) % PHASES) % PHASES;
        let line_of = |row: usize| {
            let pixels = &picture[row * width..(row + 1) * width];
            let emphasis: &[u8] = if emphasis_ok {
                &emphasis[row * width..(row + 1) * width]
            } else {
                &[]
            };
            (pixels, emphasis)
        };
        let (pixels, row_emphasis) = line_of(row);
        // The comb filter's rows around (none at the picture's top and bottom).
        let neighbours = (row > 0 && row + 1 < rows).then(|| Neighbours {
            above: line_of(row - 1),
            below: line_of(row + 1),
        });
        let target = &mut out[row * width * 3..(row + 1) * width * 3];
        if video_phase & PAL_VIDEO != 0 {
            // The 2C07: the phase follows the picture line and column.
            #[allow(clippy::cast_possible_wrap)] // a picture is far below i64::MAX rows
            let (line, first_x) = (row as i64 - margin_y as i64, -(margin as i64));
            decoder.decode_row_pal(pixels, row_emphasis, line, first_x, target);
        } else {
            decoder.decode_row_with(pixels, row_emphasis, phase, neighbours, target);
        }
    }
}
