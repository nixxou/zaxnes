//! The root message of a "3dh" profile (AMD-062 REQ-PROF-071, `HashEnvelope`): the profile
//! itself as an opaque payload, the table of tile identifiers, the library header and the
//! integrity fields, with the validation of REQ-PROF-072 and the manifest of §3.
//!
//! `HashEnvelope` and the field names are the spec's own names of convenience; only the field
//! numbers and the wire types travel. Field 5 is never written.

use std::collections::{HashMap, HashSet};

use sha2::{Digest, Sha256};

use crate::format::{ProfileHeader, ProfileLoadErrorKind};
use crate::hash::raw_tile;
use crate::hash::tile_id::{self, TileId};

/// The value of field 1 of a "3dh" profile.
pub const SIGNATURE: &str = "3dh";

/// The envelope version the canonical writer produces (REQ-PROF-084).
pub const CANONICAL_VERSION: &str = "5";

/// The five bytes a "3dh" profile starts with: field 1, length 3, the ASCII of `"3dh"`
/// (REQ-PROF-070).
pub const SIGNATURE_BYTES: [u8; 5] = [0x0A, 0x03, 0x33, 0x64, 0x68];

/// True when these bytes are a "3dh" profile (REQ-PROF-070). Anything else is read as a 3dn
/// profile, which is what SPEC-03 already does.
#[must_use]
pub fn is_hash_profile(bytes: &[u8]) -> bool {
    bytes.starts_with(&SIGNATURE_BYTES)
}

/// The log messages REQ-PROF-072 and REQ-PROF-074 prescribe. They are log texts, in English,
/// never translated.
pub mod messages {
    /// Envelope version, or payload absent.
    pub const UNSUPPORTED_ENVELOPE: &str = "Unsupported hash profile envelope";

    /// Envelope version `"4"`, whose identifiers can no longer be resolved.
    pub const VERSION_4: &str = "Hash profile envelope version 4 is no longer supported: its tile identifiers cannot be resolved. Import the original profile again.";

    /// The payload does not match its recorded digest.
    pub const DAMAGED_PAYLOAD: &str = "Damaged hash profile payload";

    /// The manifest is malformed or holds the same string twice.
    pub const INVALID_MANIFEST: &str = "Invalid hash profile manifest";

    /// A variant's `textureKind` or XOR texture is not usable (REQ-PROF-074).
    pub const INVALID_XOR_TEXTURE: &str = "Invalid profile XOR texture";

    /// A tile refers to a string the manifest does not hold (REQ-PROF-074).
    #[must_use]
    pub fn tile_hash_absent(key: &str) -> String {
        format!("Tile hash absent from profile manifest: {key}")
    }
}

/// Why a "3dh" profile was refused, with the log text REQ-PROF-072 prescribes.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct HashProfileError {
    /// The reason, as the loader reports it.
    pub kind: ProfileLoadErrorKind,
    /// The prescribed log text, verbatim.
    pub message: String,
}

impl HashProfileError {
    /// The error REQ-PROF-072 attaches to `message`.
    #[must_use]
    pub fn of_message(message: impl Into<String>) -> Self {
        let message = message.into();
        let kind = match message.as_str() {
            messages::DAMAGED_PAYLOAD => ProfileLoadErrorKind::DamagedHashPayload,
            messages::INVALID_MANIFEST => ProfileLoadErrorKind::InvalidHashManifest,
            _ => ProfileLoadErrorKind::UnsupportedHashEnvelope,
        };
        Self { kind, message }
    }

    /// A malformed payload (REQ-PROF-074): a tile outside the manifest, an unusable
    /// `textureKind`.
    #[must_use]
    pub fn invalid_payload(message: impl Into<String>) -> Self {
        Self {
            kind: ProfileLoadErrorKind::InvalidHashPayload,
            message: message.into(),
        }
    }
}

/// The envelope of a "3dh" profile, as read (REQ-PROF-071).
#[derive(Debug, Clone, Default)]
pub struct HashProfileEnvelope {
    /// Field 1; `"3dh"` for a recognised file.
    pub signature: Option<String>,
    /// Field 2: envelope version, `"1"`, `"2"` or `"5"`.
    pub version: Option<String>,
    /// Field 3: the profile proper (REQ-PROF-074), or none when absent.
    pub payload: Option<Vec<u8>>,
    /// Field 4: inherited digests and text identifiers (REQ-PROF-076).
    pub hashes: Vec<String>,
    /// Field 6: SHA-256 of [`Self::payload`], 64 lowercase hexadecimal digits.
    pub payload_hash: Option<String>,
    /// Field 7: profile identifier.
    pub id: Option<String>,
    /// Field 8: title.
    pub title: Option<String>,
    /// Field 9: author of the profile.
    pub author: Option<String>,
    /// Field 10: publisher of the game.
    pub publisher: Option<String>,
    /// Field 11: description.
    pub description: Option<String>,
    /// Field 12: tile identifiers, sixteen bytes each, concatenated.
    pub ids: Vec<TileId>,
    /// Length in bytes of field 12 as it was read, so that a length that is not a multiple of
    /// sixteen can be refused (REQ-PROF-072) rather than silently rounded.
    pub ids_byte_length: usize,
    /// Whether field 12 was present at all. It is a **bytes** field, not a repeated one, so an
    /// empty table is still a field the writer puts back: `empty.bytes`, the one profile with no
    /// tile, carries field 12 with a length of zero, and REQ-PROF-084 asks for its bytes
    /// unchanged.
    pub has_ids: bool,
}

impl HashProfileEnvelope {
    /// True when the version lets a 32-digit entry of [`Self::hashes`] stand for a tile
    /// identifier in text form (versions `"2"` and `"5"`, REQ-PROF-072).
    #[must_use]
    pub fn accepts_text_identifiers(&self) -> bool {
        matches!(self.version.as_deref(), Some("2" | "5"))
    }

    /// The library header of this envelope (REQ-PROF-073).
    #[must_use]
    pub fn to_header(&self) -> ProfileHeader {
        ProfileHeader {
            signature: self.signature.clone(),
            version: self.version.clone(),
            id: self.id.clone(),
            title: self.title.clone(),
            author: self.author.clone(),
            publisher: self.publisher.clone(),
            description: self.description.clone(),
        }
    }

    /// The SHA-256 of the payload, 64 lowercase hexadecimal digits.
    #[must_use]
    pub fn compute_payload_hash(&self) -> String {
        let payload = self.payload.as_deref().unwrap_or_default();
        raw_tile::hex_lowercase(Sha256::digest(payload).as_slice())
    }

    /// Builds the manifest, the set of strings this profile's tiles may refer to: the entries of
    /// [`Self::hashes`] and the text form of every identifier of [`Self::ids`] (§3).
    ///
    /// # Errors
    ///
    /// `Invalid hash profile manifest` when the length of field 12 is not a multiple of sixteen,
    /// when an entry of field 4 is not a legacy digest (nor, in versions `"2"` and `"5"`, an
    /// identifier in text form), or when the same string appears twice (REQ-PROF-072).
    pub fn build_manifest(&self) -> Result<HashProfileManifest, HashProfileError> {
        if self.ids_byte_length % tile_id::BYTE_COUNT != 0 {
            return Err(HashProfileError::of_message(messages::INVALID_MANIFEST));
        }

        let mut manifest = HashProfileManifest::default();
        let accepts_text = self.accepts_text_identifiers();
        for hash in &self.hashes {
            let acceptable = tile_id::is_legacy_digest(hash)
                || (accepts_text && tile_id::is_identifier_text(hash));
            if !acceptable || !manifest.add_hash(hash.clone()) {
                return Err(HashProfileError::of_message(messages::INVALID_MANIFEST));
            }
        }

        for id in &self.ids {
            if !manifest.add_identifier(*id) {
                return Err(HashProfileError::of_message(messages::INVALID_MANIFEST));
            }
        }

        Ok(manifest)
    }

    /// Validates the envelope in the order of REQ-PROF-072 and returns the manifest.
    ///
    /// # Errors
    ///
    /// The refusal of REQ-PROF-072, with its prescribed log text.
    pub fn validate(&self) -> Result<HashProfileManifest, HashProfileError> {
        if self.version.as_deref() == Some("4") {
            return Err(HashProfileError::of_message(messages::VERSION_4));
        }

        if !matches!(self.version.as_deref(), Some("1" | "2" | "5")) {
            return Err(HashProfileError::of_message(messages::UNSUPPORTED_ENVELOPE));
        }

        if self.payload.is_none() {
            return Err(HashProfileError::of_message(messages::UNSUPPORTED_ENVELOPE));
        }

        if let Some(recorded) = &self.payload_hash {
            if *recorded != self.compute_payload_hash() {
                return Err(HashProfileError::of_message(messages::DAMAGED_PAYLOAD));
            }
        }

        self.build_manifest()
    }
}

/// The manifest of a "3dh" profile (AMD-062 §3): the set of strings its tiles refer to, each
/// either a 32-digit tile identifier or a 64-digit legacy digest.
///
/// Every comparison of the format — duplicates, a tile's reference, the equality of two pending
/// tiles, resolution from a ROM or from the running game — is made on these strings, which is
/// what keeps an identifier and a legacy digest of the same tile distinct while they wait.
#[derive(Debug, Clone, Default)]
pub struct HashProfileManifest {
    keys: Vec<String>,
    key_set: HashSet<String>,
    identifiers: Vec<TileId>,
    by_prefix: HashMap<u32, Vec<String>>,
    legacy_digests: HashSet<String>,
}

impl HashProfileManifest {
    /// The manifest strings, in the order they were added.
    #[must_use]
    pub fn keys(&self) -> &[String] {
        &self.keys
    }

    /// The identifiers of field 12, in file order.
    #[must_use]
    pub fn identifiers(&self) -> &[TileId] {
        &self.identifiers
    }

    /// Number of strings.
    #[must_use]
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// True when the manifest holds no string.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// True when at least one legacy digest is in the manifest.
    #[must_use]
    pub fn has_legacy_digests(&self) -> bool {
        !self.legacy_digests.is_empty()
    }

    /// True when `key` is a manifest string.
    #[must_use]
    pub fn contains(&self, key: &str) -> bool {
        self.key_set.contains(key)
    }

    /// Adds an entry of field 4. Returns false when the string is already in the manifest, which
    /// REQ-PROF-072 refuses.
    pub fn add_hash(&mut self, hash: String) -> bool {
        if self.key_set.contains(&hash) {
            return false;
        }

        self.key_set.insert(hash.clone());
        self.keys.push(hash.clone());
        if let Some(id) = TileId::parse_text(&hash) {
            self.index(id.prefix(), hash);
        } else {
            self.legacy_digests.insert(hash);
        }

        true
    }

    /// Adds an identifier of field 12. Returns false when its text form is already in the
    /// manifest.
    pub fn add_identifier(&mut self, id: TileId) -> bool {
        let key = id.to_text();
        if self.key_set.contains(&key) {
            return false;
        }

        self.key_set.insert(key.clone());
        self.keys.push(key.clone());
        self.identifiers.push(id);
        self.index(id.prefix(), key);
        true
    }

    /// The manifest strings whose identifier has this prefix, or an empty slice. This is the
    /// fast filter of REQ-PROF-077: a scan hashes a window only when its prefix is here.
    #[must_use]
    pub fn keys_with_prefix(&self, prefix: u32) -> &[String] {
        self.by_prefix
            .get(&prefix)
            .map_or(&[] as &[String], Vec::as_slice)
    }

    /// True when `prefix` is the prefix of at least one identifier.
    #[must_use]
    pub fn has_prefix(&self, prefix: u32) -> bool {
        self.by_prefix.contains_key(&prefix)
    }

    fn index(&mut self, prefix: u32, key: String) {
        self.by_prefix.entry(prefix).or_default().push(key);
    }
}
