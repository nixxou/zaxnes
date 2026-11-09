//! Reading a "3dh" profile whole (AMD-062 REQ-PROF-070 to REQ-PROF-074): the envelope, its
//! manifest, and the payload decoded against that manifest.
//!
//! The three travel together for the rest of the profile's life: the manifest is what the
//! resolution of REQ-PROF-078 and REQ-PROF-080 fills, and the envelope is half of what the
//! rewrite of REQ-PROF-084 writes back.

use std::collections::BTreeSet;

use crate::format::{ProfileDocument, ProfileLoadErrorKind};
use crate::hash::envelope::{HashProfileEnvelope, HashProfileError, HashProfileManifest};
use crate::wire;

/// A "3dh" profile as read.
#[derive(Debug, Clone)]
pub struct HashProfile {
    /// The envelope (REQ-PROF-071).
    pub envelope: HashProfileEnvelope,
    /// The strings its tiles may name (AMD-062 §3).
    pub manifest: HashProfileManifest,
    /// The decoded payload (REQ-PROF-074).
    pub document: ProfileDocument,
}

impl HashProfile {
    /// The manifest strings whose tiles are still waiting for their planes, in ordinal order
    /// (REQ-PROF-081). A string of the manifest that no tile names is not of this set: there is
    /// nothing waiting for it.
    #[must_use]
    pub fn waiting_keys(&self) -> BTreeSet<&str> {
        self.document
            .profile
            .distinct_tile_objects()
            .filter(|tile| tile.is_waiting())
            .filter_map(|tile| tile.hash_key())
            .collect()
    }

    /// Number of tile objects still waiting for their planes.
    #[must_use]
    pub fn waiting_tile_count(&self) -> usize {
        self.document
            .profile
            .distinct_tile_objects()
            .filter(|tile| tile.is_waiting())
            .count()
    }
}

/// Reads a "3dh" profile: recognition is the caller's (`envelope::is_hash_profile`), validation
/// is REQ-PROF-072's and the payload is REQ-PROF-074's.
///
/// No tile is resolved here: every tile the payload names comes out waiting, and the resolution
/// from the ROM (REQ-PROF-078) or from the running game (REQ-PROF-080) comes after.
///
/// # Errors
///
/// The refusal of REQ-PROF-072 or REQ-PROF-074, with the log text the requirement prescribes, or
/// a payload that is not a readable profile at all (REQ-PROF-033).
pub fn read(bytes: &[u8]) -> Result<HashProfile, HashProfileError> {
    let envelope = wire::hash_envelope::decode(bytes).map_err(|error| malformed(&error))?;
    let manifest = envelope.validate()?;
    // `validate` has already refused an absent payload.
    let payload = envelope.payload.clone().unwrap_or_default();
    let document = wire::decode_hash_payload(&payload, &manifest).map_err(|error| {
        if error.is_payload_refusal() {
            HashProfileError::invalid_payload(error.message())
        } else {
            malformed(&error)
        }
    })?;

    Ok(HashProfile {
        envelope,
        manifest,
        document,
    })
}

fn malformed(error: &wire::ProfileFormatError) -> HashProfileError {
    HashProfileError {
        kind: ProfileLoadErrorKind::Malformed,
        message: format!("not a readable profile ({})", error.message()),
    }
}
