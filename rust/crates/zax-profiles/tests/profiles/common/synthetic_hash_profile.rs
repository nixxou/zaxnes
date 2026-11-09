//! Builds synthetic "3dh" envelopes, for the branches AMD-062 cannot test on the 101 built-in
//! files: envelope versions 1 and 2, text entries of field 4, legacy digests and malformed
//! manifests (REQ-PROF-072, notes).

use zax_profiles::hash::tile_id::{self, TileId};

use super::proto_builder::ProtoBuilder;

/// Every value of an envelope under the test's control, so that a refusal of REQ-PROF-072 can be
/// provoked (REQ-PROF-071 writes the fields in the order 1, 2, 3, 4, 6 to 12).
#[derive(Debug, Clone)]
pub struct EnvelopeSpec {
    pub signature: Option<String>,
    pub version: Option<String>,
    pub payload: Option<Vec<u8>>,
    pub hashes: Vec<String>,
    pub ids: Vec<TileId>,
    /// Field 12 written verbatim, for a table whose length is not a multiple of sixteen.
    pub raw_ids: Option<Vec<u8>>,
    pub write_payload_hash: bool,
    pub payload_hash_override: Option<String>,
    pub id: Option<String>,
    pub title: Option<String>,
    pub author: Option<String>,
    pub publisher: Option<String>,
    pub description: Option<String>,
}

impl Default for EnvelopeSpec {
    fn default() -> Self {
        Self {
            signature: Some("3dh".to_owned()),
            version: Some("5".to_owned()),
            payload: Some(Vec::new()),
            hashes: Vec::new(),
            ids: Vec::new(),
            raw_ids: None,
            write_payload_hash: true,
            payload_hash_override: None,
            id: Some("SyntheticId".to_owned()),
            title: Some("Synthetic".to_owned()),
            author: None,
            publisher: None,
            description: None,
        }
    }
}

impl EnvelopeSpec {
    /// The bytes of this envelope.
    pub fn encode(&self) -> Vec<u8> {
        let mut writer = ProtoBuilder::new();
        if let Some(signature) = &self.signature {
            writer.string(1, signature);
        }
        if let Some(version) = &self.version {
            writer.string(2, version);
        }
        if let Some(payload) = &self.payload {
            writer.raw_bytes(3, payload);
        }
        for hash in &self.hashes {
            writer.string(4, hash);
        }
        if self.write_payload_hash {
            let digest = self.payload_hash_override.clone().unwrap_or_else(|| {
                use sha2::{Digest, Sha256};
                let payload = self.payload.as_deref().unwrap_or_default();
                zax_profiles::hash::raw_tile::hex_lowercase(Sha256::digest(payload).as_slice())
            });
            writer.string(6, &digest);
        }
        if let Some(id) = &self.id {
            writer.string(7, id);
        }
        if let Some(title) = &self.title {
            writer.string(8, title);
        }
        if let Some(author) = &self.author {
            writer.string(9, author);
        }
        if let Some(publisher) = &self.publisher {
            writer.string(10, publisher);
        }
        if let Some(description) = &self.description {
            writer.string(11, description);
        }

        let table = self.raw_ids.clone().unwrap_or_else(|| {
            let mut table = Vec::with_capacity(self.ids.len() * tile_id::BYTE_COUNT);
            for id in &self.ids {
                table.extend_from_slice(&id.to_binary());
            }
            table
        });
        if !table.is_empty() {
            writer.raw_bytes(12, &table);
        }

        writer.into_bytes()
    }
}
