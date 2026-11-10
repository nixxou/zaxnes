//! Reading and rewriting a "3dh" profile whole (AMD-062 REQ-PROF-070 to REQ-PROF-074 and
//! REQ-PROF-084): the envelope, its manifest, the payload decoded against that manifest, and the
//! canonical writer that puts the three back.
//!
//! What a 3dh carries beyond the model of SPEC-03 travels in [`HashProfileState`], which a
//! [`ProfileDocument`] holds for the rest of the profile's life: the manifest is what the
//! resolution of REQ-PROF-078 and REQ-PROF-080 fills, and the envelope is half of what the
//! rewrite of REQ-PROF-084 writes back.

use std::collections::{BTreeSet, HashSet};
use std::sync::Arc;

use crate::format::{ProfileDocument, ProfileLoadErrorKind};
use crate::hash::envelope::{HashProfileEnvelope, HashProfileError, HashProfileManifest};
use crate::hash::raw_tile::RawTile;
use crate::hash::tile_cache;
use crate::hash::tiles::{self, TileResolver};
use crate::layout::FieldPresence;
use crate::pattern::ProfilePattern;
use crate::profile::GameProfile;
use crate::tile::ProfileTile;
use crate::wire;

/// What a profile read from a "3dh" file carries besides the model of SPEC-03.
#[derive(Debug, Clone)]
pub struct HashProfileState {
    /// The envelope (REQ-PROF-071), **without** its payload: the rewrite of REQ-PROF-084 encodes
    /// a fresh one from the model, so the bytes read are of no further use and are not kept.
    pub envelope: HashProfileEnvelope,
    /// The strings its tiles may name (AMD-062 §3).
    pub manifest: HashProfileManifest,
    /// Which of the manifest's strings have their sixteen bytes (REQ-PROF-078 to 082). Session
    /// knowledge: no savestate and no rollback snapshot carries it.
    pub tiles: TileResolver,
}

impl HashProfileState {
    /// Writes `profile` back as a "3dh" profile (REQ-PROF-084): the canonical payload of
    /// REQ-PROF-074 inside the canonical envelope, with `PayloadHash` recomputed and the entries
    /// of field 4 no tile names any more left out.
    #[must_use]
    pub fn write(&self, profile: &GameProfile) -> Vec<u8> {
        let envelope = HashProfileEnvelope {
            payload: Some(wire::encode_hash_payload(
                profile,
                FieldPresence::AsRecorded,
            )),
            hashes: self.referenced_hashes(profile),
            ..self.envelope.clone()
        };

        wire::hash_envelope::encode(&envelope)
    }

    /// The entries of field 4 a tile of `profile` still names (REQ-PROF-084). Field 12 goes back
    /// whole: the requirement filters the inherited digests, not the table of identifiers.
    fn referenced_hashes(&self, profile: &GameProfile) -> Vec<String> {
        if self.envelope.hashes.is_empty() {
            return Vec::new();
        }

        let referenced: HashSet<&str> = named_keys(profile).collect();
        self.envelope
            .hashes
            .iter()
            .filter(|hash| referenced.contains(hash.as_str()))
            .cloned()
            .collect()
    }
}

impl ProfileDocument {
    /// The "3dh" state of this profile, or none for a 3dn (AMD-062 REQ-PROF-070).
    #[must_use]
    pub fn hash_state(&self) -> Option<&HashProfileState> {
        self.hash.as_ref()
    }

    /// True when this profile was read from a "3dh" file, which is also what says that
    /// [`Self::save`] writes a 3dh (REQ-PROF-084).
    #[must_use]
    pub fn is_hash_profile(&self) -> bool {
        self.hash.is_some()
    }

    /// Resolves the tiles from the cache of REQ-PROF-082, then rebuilds the patterns it filled.
    /// Returns how many manifest strings the cache resolved; a 3dn profile resolves nothing.
    pub fn apply_tile_cache<'a>(
        &mut self,
        entries: impl IntoIterator<Item = (&'a str, RawTile)>,
    ) -> usize {
        let Some(state) = self.hash.as_mut() else {
            return 0;
        };
        let resolved = state.tiles.apply_cache(entries);
        if resolved > 0 {
            tiles::apply(&mut self.profile, &state.tiles);
        }

        resolved
    }

    /// Resolves the tiles from the bytes of an iNES ROM file and rebuilds the patterns it filled,
    /// which is what REQ-PROF-078 asks for before the first converted frame. Returns how many
    /// manifest strings the ROM resolved; a 3dn profile resolves nothing.
    pub fn resolve_from_rom(&mut self, file: &[u8]) -> usize {
        let Some(state) = self.hash.as_mut() else {
            return 0;
        };
        let resolved = state.tiles.resolve_from_rom(file);
        if resolved > 0 {
            tiles::apply(&mut self.profile, &state.tiles);
        }

        resolved
    }

    /// The cache of REQ-PROF-082 for this profile, as a file's contents; none for a 3dn, which
    /// has no tile to resolve.
    #[must_use]
    pub fn tile_cache(&self) -> Option<String> {
        self.hash
            .as_ref()
            .map(|state| tile_cache::write(state.tiles.entries()))
    }

    /// The name of this profile's cache file, from the `Id` of its envelope; none for a 3dn and
    /// none when the envelope carries no identifier, which has no cache of its own.
    #[must_use]
    pub fn tile_cache_file_name(&self) -> Option<String> {
        self.hash
            .as_ref()?
            .envelope
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
        self.profile
            .distinct_tile_objects()
            .filter(|tile| tile.is_waiting())
            .filter_map(|tile| tile.hash_key())
            .collect()
    }

    /// Number of tile objects still waiting for their planes.
    #[must_use]
    pub fn waiting_tile_count(&self) -> usize {
        self.profile
            .distinct_tile_objects()
            .filter(|tile| tile.is_waiting())
            .count()
    }
}

/// Reads a "3dh" profile: recognition is the caller's (`envelope::is_hash_profile`), validation
/// is REQ-PROF-072's and the payload is REQ-PROF-074's.
///
/// No tile is resolved here: every tile the payload names comes out waiting, and the resolution
/// from the ROM (REQ-PROF-078) or from the running game (REQ-PROF-080) comes after. The load
/// rules of SPEC-03 are not applied either — [`crate::format::load`] does that, for both formats.
///
/// # Errors
///
/// The refusal of REQ-PROF-072 or REQ-PROF-074, with the log text the requirement prescribes, or
/// a payload that is not a readable profile at all (REQ-PROF-033).
pub fn read(bytes: &[u8]) -> Result<ProfileDocument, HashProfileError> {
    let mut envelope = wire::hash_envelope::decode(bytes).map_err(|error| malformed(&error))?;
    let manifest = envelope.validate()?;
    // `validate` has already refused an absent payload.
    let payload = envelope.payload.take().unwrap_or_default();
    let mut document = wire::decode_hash_payload(&payload, &manifest).map_err(|error| {
        if error.is_payload_refusal() {
            HashProfileError::invalid_payload(error.message())
        } else {
            malformed(&error)
        }
    })?;

    let tiles = TileResolver::of(&manifest);
    document.hash = Some(HashProfileState {
        envelope,
        manifest,
        tiles,
    });

    Ok(document)
}

/// Every manifest string a tile of `profile` names, the patterns of the memorised slots included:
/// those hold their patterns by value, outside the shared graph, and a string one of their tiles
/// names is still a referenced string.
fn named_keys(profile: &GameProfile) -> impl Iterator<Item = &str> {
    profile
        .distinct_tile_objects()
        .filter_map(|tile| tile.hash_key())
        .chain(
            slot_patterns(profile)
                .flat_map(slot_tiles)
                .filter_map(|tile| tile.hash_key()),
        )
}

fn slot_patterns(profile: &GameProfile) -> impl Iterator<Item = &ProfilePattern> {
    profile
        .settings
        .iter()
        .flat_map(|settings| {
            settings
                .saved_patterns
                .iter()
                .chain(&settings.saved_anim_patterns)
        })
        .filter_map(|slot| slot.pattern.as_ref())
}

fn slot_tiles(pattern: &ProfilePattern) -> impl Iterator<Item = &Arc<ProfileTile>> {
    pattern
        .grid
        .iter()
        .flat_map(|grid| grid.cells().iter().flatten())
}

fn malformed(error: &wire::ProfileFormatError) -> HashProfileError {
    HashProfileError {
        kind: ProfileLoadErrorKind::Malformed,
        message: format!("not a readable profile ({})", error.message()),
    }
}
