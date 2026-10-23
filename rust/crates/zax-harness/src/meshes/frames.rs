//! The frames the `meshes` command feeds the engine: synthetic layouts of a profile's own
//! patterns, or a frame dump (port of `DumpFrame`, `SyntheticFrames` and `FrameDump`).

use zax_engine3d::engine::ConversionEngine;
use zax_engine3d::image::TileImageKey;
use zax_nes::{FrameBatch, TileRecord};

use crate::HarnessError;

/// One frame to feed the engine: tile records and the 32 palette bytes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DumpFrame {
    records: Vec<TileRecord>,
    palette: [u8; 32],
}

impl DumpFrame {
    /// The records, in emission order.
    #[must_use]
    pub fn records(&self) -> &[TileRecord] {
        &self.records
    }

    /// Sets one of the 32 PPU palette bytes; an index past 31 is ignored.
    pub fn set_palette(&mut self, index: usize, value: u8) {
        if let Some(entry) = self.palette.get_mut(index) {
            *entry = value;
        }
    }

    /// Adds a background tile.
    pub fn add_background(&mut self, image: TileImageKey, x: i16, y: i16, palette: u8) {
        self.add(image, x, y, palette, true, false);
    }

    /// Adds a sprite tile.
    pub fn add_sprite(&mut self, image: TileImageKey, x: i16, y: i16, palette: u8, hidden: bool) {
        self.add(image, x, y, palette, false, hidden);
    }

    /// Converts the frame with `engine`.
    pub fn convert(&self, engine: &mut ConversionEngine, counter: i32) {
        let batch = FrameBatch {
            records: &self.records,
            palette: self.palette,
            mask_x: 0,
            mask_y: 0,
        };
        engine.convert(&batch, counter);
    }

    #[allow(clippy::fn_params_excessive_bools)] // The two flags of a tile record.
    fn add(
        &mut self,
        image: TileImageKey,
        x: i16,
        y: i16,
        palette: u8,
        background: bool,
        hidden: bool,
    ) {
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // C# record index.
        let index = self.records.len() as i32;
        self.records.push(TileRecord::new(
            image.low,
            image.high,
            x,
            y,
            index,
            image.flip_x,
            image.flip_y,
            hidden,
            palette,
            background,
        ));
    }
}

/// Lays the tiles of a profile's own patterns over the background grid, so that the
/// recognition of SPEC-04 binds them and the geometry of every pattern used shows up. Builds
/// at most `count` frames from the permanent patterns of the registry, using at most
/// `max_patterns` of them (0: as many as fit).
#[must_use]
pub fn synthetic_frames(
    engine: &ConversionEngine,
    count: usize,
    max_patterns: usize,
) -> Vec<DumpFrame> {
    let permanent = engine.patterns().permanent();
    let limit = if max_patterns > 0 {
        max_patterns.min(permanent.len())
    } else {
        permanent.len()
    };

    let mut frames = Vec::new();
    let mut pattern = 0;
    for _ in 0..count {
        let mut frame = DumpFrame::default();
        for entry in 0..32u8 {
            // A readable palette: the universal background plus a spread of NES colours.
            frame.set_palette(
                usize::from(entry),
                if entry == 0 {
                    0x0F
                } else {
                    0x01 + (entry % 0x3F)
                },
            );
        }

        // The grid is 32 columns by 30 rows of tiles; a cursor advances row by row.
        let (mut column, mut row) = (0, 0);
        let mut placed = false;
        while pattern < limit && row < 30 {
            let Some(current) = engine.patterns().get(permanent[pattern]) else {
                pattern += 1;
                continue;
            };
            let width = current.width().max(1);
            let height = current.height().max(1);
            if width > 32 || height > 30 {
                pattern += 1;
                continue;
            }

            if column + width > 32 {
                column = 0;
                row += height;
                continue;
            }

            if row + height > 30 {
                break;
            }

            for j in 0..height {
                for i in 0..width {
                    if let Some(image) = current.image_at(i, j) {
                        // Inside the 32 x 30 grid, so the screen position fits an i16.
                        #[allow(clippy::cast_possible_truncation)]
                        let (x, y) = ((8 * (column + i)) as i16, (8 * (row + j)) as i16);
                        frame.add_background(image.key(), x, y, 0);
                    }
                }
            }

            column += width;
            pattern += 1;
            placed = true;
        }

        frames.push(frame);
        if !placed && pattern >= limit {
            break;
        }
    }

    frames
}

/// Reads the frame dump format documented in [`super::USAGE`].
///
/// # Errors
/// [`HarnessError::Format`] naming the first line that does not parse.
pub fn read_frame_dump(text: &str) -> Result<Vec<DumpFrame>, HarnessError> {
    let mut frames = Vec::new();
    let mut current = DumpFrame::default();
    let mut dirty = false;

    for (number, raw) in (1..).zip(text.lines()) {
        let line = raw.split_once('#').map_or(raw, |(line, _)| line).trim();
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        let error = |message: String| HarnessError::Format(format!("line {number}: {message}"));
        let expect = |count: usize| {
            if parts.len() < count {
                Err(error(format!("{} needs {} values", parts[0], count - 1)))
            } else {
                Ok(())
            }
        };
        let int = |text: &str| {
            text.parse::<i32>()
                .map_err(|_| error(format!("{text} is not a number")))
        };
        let short = |text: &str| {
            text.parse::<i16>()
                .map_err(|_| error(format!("{text} is not a position")))
        };
        let byte = |text: &str| {
            text.parse::<u8>()
                .map_err(|_| error(format!("{text} is not a palette")))
        };
        let image = |low: &str,
                     high: &str,
                     flip_x: &str,
                     flip_y: &str|
         -> Result<TileImageKey, HarnessError> {
            Ok(TileImageKey::new(
                u64::from_str_radix(low, 16).map_err(|_| error(format!("{low} is not a plane")))?,
                u64::from_str_radix(high, 16)
                    .map_err(|_| error(format!("{high} is not a plane")))?,
                int(flip_x)? != 0,
                int(flip_y)? != 0,
            ))
        };

        match parts[0].to_ascii_lowercase().as_str() {
            "end" => {
                frames.push(std::mem::take(&mut current));
                dirty = false;
            }
            "palette" => {
                expect(2)?;
                let digits = parts[1].as_bytes();
                for (entry, pair) in digits.chunks_exact(2).take(32).enumerate() {
                    let text =
                        std::str::from_utf8(pair).map_err(|_| error("bad palette".to_owned()))?;
                    let value = u8::from_str_radix(text, 16)
                        .map_err(|_| error(format!("{text} is not a palette byte")))?;
                    current.set_palette(entry, value);
                }
                dirty = true;
            }
            "bg" => {
                expect(8)?;
                let key = image(parts[4], parts[5], parts[6], parts[7])?;
                current.add_background(key, short(parts[1])?, short(parts[2])?, byte(parts[3])?);
                dirty = true;
            }
            "sp" => {
                expect(9)?;
                let key = image(parts[4], parts[5], parts[6], parts[7])?;
                let hidden = int(parts[8])? != 0;
                current.add_sprite(
                    key,
                    short(parts[1])?,
                    short(parts[2])?,
                    byte(parts[3])?,
                    hidden,
                );
                dirty = true;
            }
            other => return Err(error(format!("unknown directive {other}"))),
        }
    }

    if dirty {
        frames.push(current);
    }

    Ok(frames)
}
