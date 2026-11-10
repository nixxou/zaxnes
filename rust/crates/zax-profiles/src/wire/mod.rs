//! The binary profile format of SPEC-03 §3 and §4: a hand-written protobuf reader and writer
//! (REQ-PROF-010 to REQ-PROF-013 and REQ-PROF-051 ask for shared-reference envelopes,
//! group-encoded nullable cells, a field written out of order and byte-exact control of field
//! presence, which a single-pass codec does directly).

pub mod cipher;
mod decoder;
mod encoder;
pub mod hash_envelope;
mod reader;
mod schema;
mod writer;

use crate::format::{ProfileDocument, ProfileHeader};
use crate::hash::envelope::HashProfileManifest;
use crate::layout::FieldPresence;
use crate::profile::GameProfile;

/// The bytes are not a readable profile.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct ProfileFormatError {
    message: String,
    payload_refusal: bool,
}

impl ProfileFormatError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            payload_refusal: false,
        }
    }

    /// A payload of a "3dh" profile REQ-PROF-074 refuses — a tile outside the manifest, an
    /// unusable `textureKind`. The message is the log text the requirement prescribes, verbatim.
    pub(crate) fn payload_refusal(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            payload_refusal: true,
        }
    }

    /// The description of what is wrong with the bytes.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// True when the bytes decode but the payload is not an acceptable "3dh" profile
    /// (AMD-062 REQ-PROF-074), as opposed to a byte stream that does not decode at all.
    #[must_use]
    pub fn is_payload_refusal(&self) -> bool {
        self.payload_refusal
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

/// Decodes the payload of a "3dh" profile (AMD-062 REQ-PROF-074): tiles named by a manifest
/// string instead of their planes, and variant textures in fields 24 and 25 instead of field 19.
///
/// # Errors
///
/// The bytes are not a readable profile, or the payload is not an acceptable "3dh" payload — a
/// tile outside `manifest`, an unusable `textureKind` — which
/// [`ProfileFormatError::is_payload_refusal`] tells apart.
pub fn decode_hash_payload(
    bytes: &[u8],
    manifest: &HashProfileManifest,
) -> Result<ProfileDocument, ProfileFormatError> {
    decoder::Decoder::new_hash(bytes, manifest).read_document()
}

/// Encodes a profile; `presence` says whether every scalar is written or only those a decoded
/// file carried.
#[must_use]
pub fn encode(profile: &GameProfile, presence: FieldPresence) -> Vec<u8> {
    encoder::Encoder::new(profile, presence).write(profile)
}

/// Encodes a profile as the payload of a "3dh" profile (AMD-062 REQ-PROF-084): the canonical
/// writer of REQ-PROF-051 with the four deviations of REQ-PROF-074.
#[must_use]
pub fn encode_hash_payload(profile: &GameProfile, presence: FieldPresence) -> Vec<u8> {
    encoder::Encoder::new_hash(profile, presence).write(profile)
}

/// Reads the identity fields of a profile without decoding the rest.
///
/// # Errors
///
/// The bytes are not a readable profile.
pub fn decode_header(bytes: &[u8]) -> Result<ProfileHeader, ProfileFormatError> {
    decoder::read_header(bytes)
}
