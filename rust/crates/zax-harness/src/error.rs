use std::fmt;

use zax_nes::apu::AudioError;

/// Why a harness command failed.
#[derive(Debug)]
pub enum HarnessError {
    /// A file or directory could not be read or written.
    Io(std::io::Error),
    /// The core refused an audio read.
    Audio(AudioError),
    /// The command line is wrong; the message says how.
    Usage(String),
}

impl fmt::Display for HarnessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Audio(error) => write!(f, "{error}"),
            Self::Usage(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for HarnessError {}

impl From<std::io::Error> for HarnessError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<AudioError> for HarnessError {
    fn from(error: AudioError) -> Self {
        Self::Audio(error)
    }
}
