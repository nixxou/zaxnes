//! An RGBA picture read from a PNG.

use crate::{HdPackError, Result};

/// A picture, 4 bytes a pixel (R, G, B, A), row 0 first.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Rgba {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl Rgba {
    /// A picture of one colour.
    #[must_use]
    pub fn new(width: u32, height: u32, colour: [u8; 4]) -> Self {
        Self {
            width,
            height,
            data: colour.repeat((width * height) as usize),
        }
    }

    /// The pixel at (`x`, `y`), transparent outside the picture.
    #[must_use]
    pub fn get(&self, x: u32, y: u32) -> [u8; 4] {
        if x >= self.width || y >= self.height {
            return [0; 4];
        }
        let at = ((y * self.width + x) * 4) as usize;
        [
            self.data[at],
            self.data[at + 1],
            self.data[at + 2],
            self.data[at + 3],
        ]
    }

    /// Decodes a PNG of any 8- or 16-bit colour type into RGBA.
    ///
    /// # Errors
    ///
    /// When the bytes are not a PNG the decoder reads.
    pub fn from_png(bytes: &[u8]) -> Result<Self> {
        let failed = |error: png::DecodingError| HdPackError::Png(error.to_string());
        let mut decoder = png::Decoder::new(bytes);
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder.read_info().map_err(failed)?;
        let mut buffer = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buffer).map_err(failed)?;
        buffer.truncate(info.buffer_size());
        let pixels = (info.width * info.height) as usize;
        let data = match info.color_type {
            png::ColorType::Rgba => buffer,
            png::ColorType::Rgb => buffer
                .chunks_exact(3)
                .flat_map(|p| [p[0], p[1], p[2], 255])
                .collect(),
            png::ColorType::GrayscaleAlpha => buffer
                .chunks_exact(2)
                .flat_map(|p| [p[0], p[0], p[0], p[1]])
                .collect(),
            png::ColorType::Grayscale => buffer.iter().flat_map(|&g| [g, g, g, 255]).collect(),
            png::ColorType::Indexed => {
                return Err(HdPackError::Png("indexed picture not expanded".into()))
            }
        };
        if data.len() != pixels * 4 {
            return Err(HdPackError::Png("unexpected picture size".into()));
        }
        Ok(Self {
            width: info.width,
            height: info.height,
            data,
        })
    }

    /// Encodes the picture as a PNG (tests and diagnostics).
    #[must_use]
    pub fn to_png(&self) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut out, self.width, self.height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            if let Ok(mut writer) = encoder.write_header() {
                let _ = writer.write_image_data(&self.data);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_png_round_trips() {
        let mut picture = Rgba::new(3, 2, [1, 2, 3, 255]);
        picture.data[4..8].copy_from_slice(&[9, 8, 7, 0]);
        let back = Rgba::from_png(&picture.to_png());
        assert_eq!(Ok(picture), back);
    }
}
