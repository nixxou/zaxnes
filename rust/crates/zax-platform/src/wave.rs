//! RIFF/WAVE reader for the files of `assets-neutral/audio/` (SPEC-11 §2): three menu tracks and
//! four interface sounds, all PCM at 44 100 Hz.
//!
//! Uncompressed PCM (8, 16, 24 and 32-bit integer) and IEEE float are accepted, which covers
//! everything the asset folder holds; any other codec is refused. Nothing here talks to the
//! audio device, so the decoder is testable without one.

use std::path::Path;

const FORMAT_PCM: u16 = 1;
const FORMAT_IEEE_FLOAT: u16 = 3;
const FORMAT_EXTENSIBLE: u16 = 0xFFFE;

/// Decoded PCM audio: interleaved samples in `[-1 ; 1]`.
#[derive(Debug, Clone, PartialEq)]
pub struct WaveAudio {
    /// Interleaved frames, `channels` values per frame.
    pub samples: Vec<f32>,
    /// 1 for mono, 2 for stereo.
    pub channels: u16,
    /// Frames per second as the file declares them.
    pub sample_rate: u32,
}

impl WaveAudio {
    /// Frames in the clip.
    #[must_use]
    pub fn frame_count(&self) -> usize {
        if self.channels == 0 {
            0
        } else {
            self.samples.len() / usize::from(self.channels)
        }
    }

    /// Duration in seconds.
    #[must_use]
    pub fn duration(&self) -> f64 {
        if self.sample_rate == 0 {
            0.0
        } else {
            // Frame counts of an audio clip are far below 2^52, so the conversion is exact.
            #[allow(clippy::cast_precision_loss)]
            let frames = self.frame_count() as f64;
            frames / f64::from(self.sample_rate)
        }
    }
}

/// Why a WAVE file was refused.
#[derive(Debug, thiserror::Error)]
pub enum WaveError {
    /// The file could not be read.
    #[error("cannot read the WAVE file: {0}")]
    Io(#[from] std::io::Error),
    /// The bytes are not a RIFF/WAVE container.
    #[error("Not a RIFF/WAVE file.")]
    NotWave,
    /// The container has no usable `fmt ` or `data` chunk.
    #[error("The WAVE file has no usable fmt or data chunk.")]
    NoUsableChunk,
    /// The codec is neither integer PCM nor 32-bit float.
    #[error("Unsupported WAVE encoding: format {format}, {bits_per_sample} bits per sample.")]
    UnsupportedEncoding {
        /// The `wFormatTag` of the file.
        format: u16,
        /// The declared sample width.
        bits_per_sample: u16,
    },
}

/// Reads a file from disk.
///
/// # Errors
/// [`WaveError::Io`] if the file cannot be read, otherwise as [`decode`].
pub fn load(path: &Path) -> Result<WaveAudio, WaveError> {
    decode(&std::fs::read(path)?)
}

fn u16_at(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

/// Reads a file already in memory.
///
/// # Errors
/// [`WaveError::NotWave`], [`WaveError::NoUsableChunk`] or [`WaveError::UnsupportedEncoding`].
pub fn decode(file: &[u8]) -> Result<WaveAudio, WaveError> {
    if file.len() < 12 || &file[..4] != b"RIFF" || &file[8..12] != b"WAVE" {
        return Err(WaveError::NotWave);
    }

    let mut format = 0u16;
    let mut channels = 0u16;
    let mut sample_rate = 0u32;
    let mut bits_per_sample = 0u16;
    let mut data: &[u8] = &[];
    let mut saw_data = false;

    let mut offset = 12usize;
    while offset + 8 <= file.len() {
        let id = &file[offset..offset + 4];
        let size = u32_at(file, offset + 4) as usize;
        let body = offset + 8;
        let available = file.len() - body;
        let length = size.min(available);

        if id == b"fmt " && length >= 16 {
            let chunk = &file[body..body + length];
            format = u16_at(chunk, 0);
            channels = u16_at(chunk, 2);
            sample_rate = u32_at(chunk, 4);
            bits_per_sample = u16_at(chunk, 14);

            if format == FORMAT_EXTENSIBLE && length >= 26 {
                // The real format sits in the first two bytes of the GUID.
                format = u16_at(chunk, 24);
            }
        } else if id == b"data" {
            data = &file[body..body + length];
            saw_data = true;
        }

        // Chunks are padded to even sizes.
        offset = body + length + (length & 1);
    }

    if !saw_data || !(1..=2).contains(&channels) || sample_rate == 0 {
        return Err(WaveError::NoUsableChunk);
    }

    Ok(WaveAudio {
        samples: convert(data, format, bits_per_sample)?,
        channels,
        sample_rate,
    })
}

fn convert(data: &[u8], format: u16, bits_per_sample: u16) -> Result<Vec<f32>, WaveError> {
    let samples = match (format, bits_per_sample) {
        // 8-bit PCM is unsigned, centred on 128.
        (FORMAT_PCM, 8) => data
            .iter()
            .map(|&byte| (f32::from(byte) - 128.0) / 128.0)
            .collect(),
        (FORMAT_PCM, 16) => data
            .chunks_exact(2)
            .map(|b| f32::from(i16::from_le_bytes([b[0], b[1]])) / 32768.0)
            .collect(),
        (FORMAT_PCM, 24) => data
            .chunks_exact(3)
            .map(|b| {
                // Place the 24 bits at the top of an i32, then shift back: sign extension.
                let value = i32::from_le_bytes([0, b[0], b[1], b[2]]) >> 8;
                // `value` is below 2^23, so the conversion is exact, as in the C#.
                #[allow(clippy::cast_precision_loss)]
                let value = value as f32;
                value / 8_388_608.0
            })
            .collect(),
        (FORMAT_PCM, 32) => data
            .chunks_exact(4)
            .map(|b| {
                // C# divides the int converted to float: the rounding is part of the semantics.
                #[allow(clippy::cast_precision_loss)]
                let value = i32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f32;
                value / 2_147_483_648.0
            })
            .collect(),
        (FORMAT_IEEE_FLOAT, 32) => data
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect(),
        _ => {
            return Err(WaveError::UnsupportedEncoding {
                format,
                bits_per_sample,
            })
        }
    };
    Ok(samples)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wave(format: u16, channels: u16, rate: u32, bits: u16, data: &[u8]) -> Vec<u8> {
        let mut out = b"RIFF".to_vec();
        out.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&format.to_le_bytes());
        out.extend_from_slice(&channels.to_le_bytes());
        out.extend_from_slice(&rate.to_le_bytes());
        out.extend_from_slice(&(rate * u32::from(channels) * u32::from(bits) / 8).to_le_bytes());
        out.extend_from_slice(&(channels * bits / 8).to_le_bytes());
        out.extend_from_slice(&bits.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(data);
        out
    }

    #[test]
    fn pcm16_stereo_decodes_to_normalised_floats() {
        let data: Vec<u8> = [0i16, 16384, -32768, 32767]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        let audio = decode(&wave(FORMAT_PCM, 2, 44_100, 16, &data)).unwrap();
        assert_eq!(audio.channels, 2);
        assert_eq!(audio.sample_rate, 44_100);
        assert_eq!(audio.frame_count(), 2);
        assert_eq!(audio.samples[..3], [0.0, 0.5, -1.0]);
        assert!((audio.duration() - 2.0 / 44_100.0).abs() < 1e-12);
    }

    #[test]
    fn pcm8_24_32_and_float_decode() {
        let a = decode(&wave(FORMAT_PCM, 1, 8000, 8, &[128, 255, 0])).unwrap();
        assert_eq!(a.samples, [0.0, 127.0 / 128.0, -1.0]);
        let a = decode(&wave(FORMAT_PCM, 1, 8000, 24, &[0, 0, 0x80, 0xFF, 0xFF, 0x7F])).unwrap();
        assert_eq!(a.samples, [-1.0, 8_388_607.0 / 8_388_608.0]);
        let a = decode(&wave(FORMAT_PCM, 1, 8000, 32, &i32::MIN.to_le_bytes())).unwrap();
        assert_eq!(a.samples, [-1.0]);
        let a = decode(&wave(FORMAT_IEEE_FLOAT, 1, 8000, 32, &0.25f32.to_le_bytes())).unwrap();
        assert_eq!(a.samples, [0.25]);
    }

    #[test]
    fn a_truncated_data_chunk_is_clamped_and_an_odd_chunk_is_padded() {
        let mut file = wave(FORMAT_PCM, 1, 8000, 16, &[1, 0, 2, 0]);
        // Claim more data than the file holds.
        let size_at = file.len() - 8;
        file[size_at..size_at + 4].copy_from_slice(&100u32.to_le_bytes());
        assert_eq!(decode(&file).unwrap().samples.len(), 2);
    }

    #[test]
    fn refused_files() {
        assert!(matches!(decode(b"nope"), Err(WaveError::NotWave)));
        assert!(matches!(
            decode(&wave(FORMAT_PCM, 3, 8000, 16, &[0, 0])),
            Err(WaveError::NoUsableChunk)
        ));
        assert!(matches!(
            decode(&wave(2, 1, 8000, 4, &[0, 0])),
            Err(WaveError::UnsupportedEncoding { format: 2, bits_per_sample: 4 })
        ));
    }
}
