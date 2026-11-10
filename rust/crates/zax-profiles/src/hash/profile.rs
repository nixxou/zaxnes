//! Reading a "3dh" profile whole (AMD-062 REQ-PROF-070 to REQ-PROF-074): the envelope, its
//! manifest, and the payload decoded against that manifest.
//!
//! The three travel together for the rest of the profile's life: the manifest is what the
//! resolution of REQ-PROF-078 and REQ-PROF-080 fills, and the envelope is half of what the
//! rewrite of REQ-PROF-084 writes back.

use std::collections::BTreeSet;

use crate::format::{ProfileDocument, ProfileLoadErrorKind};
use crate::hash::envelope::{HashProfileEnvelope, HashProfileError, HashProfileManifest};
use crate::hash::raw_tile::RawTile;
use crate::hash::tile_cache;
use crate::hash::tiles::{self, TileResolver};
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
    /// Which of the manifest's strings have their sixteen bytes (REQ-PROF-078 to 082). Session
    /// knowledge: no savestate and no rollback snapshot carries it.
    pub tiles: TileResolver,
}

impl HashProfile {
    /// Resolves the tiles from the cache of REQ-PROF-082, then rebuilds the patterns it filled.
    /// Returns how many manifest strings the cache resolved.
    pub fn apply_tile_cache<'a>(
        &mut self,
        entries: impl IntoIterator<Item = (&'a str, RawTile)>,
    ) -> usize {
        let resolved = self.tiles.apply_cache(entries);
        if resolved > 0 {
            tiles::apply(&mut self.document.profile, &self.tiles);
        }

        resolved
    }

    /// Resolves the tiles from the bytes of an iNES ROM file and rebuilds the patterns it filled,
    /// which is what REQ-PROF-078 asks for before the first converted frame. Returns how many
    /// manifest strings the ROM resolved.
    pub fn resolve_from_rom(&mut self, file: &[u8]) -> usize {
        let resolved = self.tiles.resolve_from_rom(file);
        if resolved > 0 {
            tiles::apply(&mut self.document.profile, &self.tiles);
        }

        resolved
    }

    /// The cache of REQ-PROF-082 for this profile, as a file's contents.
    #[must_use]
    pub fn tile_cache(&self) -> String {
        tile_cache::write(self.tiles.entries())
    }

    /// The name of this profile's cache file, from the `Id` of its envelope; none when the
    /// envelope carries no identifier, which has no cache of its own.
    #[must_use]
    pub fn tile_cache_file_name(&self) -> Option<String> {
        self.envelope
            .id
            .as_deref()
            .filter(|id| !id.is_empty())
            .map(tile_cache::file_name)
    }

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

    let tiles = TileResolver::of(&manifest);
    Ok(HashProfile {
        envelope,
        manifest,
        document,
        tiles,
    })
}

fn malformed(error: &wire::ProfileFormatError) -> HashProfileError {
    HashProfileError {
        kind: ProfileLoadErrorKind::Malformed,
        message: format!("not a readable profile ({})", error.message()),
    }
}
