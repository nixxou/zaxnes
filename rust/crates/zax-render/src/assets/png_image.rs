//! The PNG reader and writer of the asset folder and of the savestate thumbnails.
//!
//! The decoder returns every supported colour type as 8-bit RGBA, 16-bit samples keeping their
//! high byte. Like the C# reader it does not verify the chunk checksums.

use std::io::Cursor;
use std::path::Path;

use png::{BitDepth, ColorType, Transformations};
use thiserror::Error;

use super::ImageData;

/// Why a PNG could not be read or written.
#[derive(Debug, Error)]
pub enum PngError {
    /// The file could not be read or written.
    #[error("{0}")]
    Io(#[from] std::io::Error),
    /// The data is not a PNG this decoder supports.
    #[error("invalid PNG data: {0}")]
    Decode(#[from] png::DecodingError),
    /// The image could not be encoded.
    #[error("cannot encode the PNG: {0}")]
    Encode(#[from] png::EncodingError),
    /// The pixel buffer does not match the declared image.
    #[error("the pixel buffer holds {actual} bytes, the {width}x{height} image needs {expected}")]
    Size {
        /// Declared width.
        width: u32,
        /// Declared height.
        height: u32,
        /// Bytes needed.
        expected: usize,
        /// Bytes given.
        actual: usize,
    },
}

/// Decodes the PNG file `path`.
///
/// # Errors
///
/// [`PngError`] when the file cannot be read or is not a PNG this decoder supports.
pub fn load(path: &Path) -> Result<ImageData, PngError> {
    decode(&std::fs::read(path)?)
}

/// Decodes a PNG file held in memory.
///
/// # Errors
///
/// [`PngError::Decode`] when the data is not a PNG this decoder supports.
pub fn decode(file: &[u8]) -> Result<ImageData, PngError> {
    let mut decoder = png::Decoder::new(Cursor::new(file));
    decoder.ignore_checksums(true);
    decoder.set_transformations(Transformations::EXPAND | Transformations::STRIP_16);
    let mut reader = decoder.read_info()?;
    let mut buffer = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buffer)?;
    let (width, height) = (info.width as usize, info.height as usize);
    let samples = &buffer[..info.buffer_size()];

    let stride = info.line_size;
    let mut pixels = Vec::with_capacity(width * height * 4);
    for row in samples.chunks(stride).take(height) {
        match info.color_type {
            ColorType::Grayscale => {
                for &v in &row[..width] {
                    pixels.extend_from_slice(&[v, v, v, 255]);
                }
            }
            ColorType::GrayscaleAlpha => {
                for pixel in row[..width * 2].chunks_exact(2) {
                    pixels.extend_from_slice(&[pixel[0], pixel[0], pixel[0], pixel[1]]);
                }
            }
            // EXPAND turns an indexed image into RGB, or RGBA when it carries a tRNS chunk.
            ColorType::Rgb | ColorType::Indexed => {
                for pixel in row[..width * 3].chunks_exact(3) {
                    pixels.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]);
                }
            }
            ColorType::Rgba => pixels.extend_from_slice(&row[..width * 4]),
        }
    }

    Ok(ImageData {
        width,
        height,
        pixels,
    })
}

/// Encodes an 8-bit RGB PNG into memory: the savestate thumbnails of SPEC-09 REQ-PERS-051 and
/// the cover art of REQ-PERS-036, which are both stored without an alpha channel. `pixels`
/// holds the rows top to bottom, three bytes per pixel.
///
/// # Errors
///
/// [`PngError::Size`] when the buffer is not `width × height × 3` bytes.
pub fn encode_rgb(width: u32, height: u32, pixels: &[u8]) -> Result<Vec<u8>, PngError> {
    encode(width, height, ColorType::Rgb, 3, pixels)
}

/// Encodes an 8-bit RGBA PNG into memory, rows top to bottom, four bytes per pixel.
///
/// # Errors
///
/// [`PngError::Size`] when the buffer is not `width × height × 4` bytes.
pub fn encode_rgba(width: u32, height: u32, pixels: &[u8]) -> Result<Vec<u8>, PngError> {
    encode(width, height, ColorType::Rgba, 4, pixels)
}

fn encode(
    width: u32,
    height: u32,
    color: ColorType,
    channels: usize,
    pixels: &[u8],
) -> Result<Vec<u8>, PngError> {
    let expected = width as usize * height as usize * channels;
    if width == 0 || height == 0 || pixels.len() != expected {
        return Err(PngError::Size {
            width,
            height,
            expected,
            actual: pixels.len(),
        });
    }

    let mut file = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut file, width, height);
        encoder.set_color(color);
        encoder.set_depth(BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(pixels)?;
        writer.finish()?;
    }
    Ok(file)
}
