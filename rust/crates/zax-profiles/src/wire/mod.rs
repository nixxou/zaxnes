//! The binary profile format of SPEC-03 §3 and §4: a hand-written protobuf reader and writer
//! (REQ-PROF-010 to REQ-PROF-013 and REQ-PROF-051 ask for shared-reference envelopes,
//! group-encoded nullable cells, a field written out of order and byte-exact control of field
//! presence, which a single-pass codec does directly).

pub mod cipher;
mod decoder;
mod encoder;
mod reader;
mod schema;
mod writer;

use crate::format::{ProfileDocument, ProfileHeader};
use crate::layout::FieldPresence;
use crate::profile::GameProfile;

/// The bytes are not a readable profile.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct ProfileFormatError {
    message: String,
}

impl ProfileFormatError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// The description of what is wrong with the bytes.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// Decodes a profile file, without applying the load rules of `format::load`.
///
/// # Errors
///
/// The bytes are not a readable profile.
pub fn decode(bytes: &[u8]) -> Result<ProfileDocument, ProfileFormatError> {
    decoder::Decoder::new(bytes).read_document()
}

/// Encodes a profile; `presence` says whether every scalar is written or only those a decoded
/// file carried.
#[must_use]
pub fn encode(profile: &GameProfile, presence: FieldPresence) -> Vec<u8> {
    encoder::Encoder::new(profile, presence).write(profile)
}

/// Reads the identity fields of a profile without decoding the rest.
///
/// # Errors
///
/// The bytes are not a readable profile.
pub fn decode_header(bytes: &[u8]) -> Result<ProfileHeader, ProfileFormatError> {
    decoder::read_header(bytes)
}
