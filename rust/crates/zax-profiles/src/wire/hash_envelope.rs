//! Reads and writes the envelope of a "3dh" profile (AMD-062 REQ-PROF-070, 071, 084).

use super::reader::ProtoReader;
use super::schema::hash_envelope as field;
use super::writer::ProtoWriter;
use super::ProfileFormatError;
use crate::hash::envelope::{HashProfileEnvelope, CANONICAL_VERSION, SIGNATURE};
use crate::hash::tile_id::{self, TileId};

/// Decodes the envelope. Fields may come in any order, an unknown field is skipped and an absent
/// field is none (SPEC-03 §3.1).
///
/// # Errors
///
/// The bytes are not a well-formed envelope.
pub fn decode(bytes: &[u8]) -> Result<HashProfileEnvelope, ProfileFormatError> {
    let mut reader = ProtoReader::new(bytes);
    let limit = reader.end();
    let mut envelope = HashProfileEnvelope::default();
    while let Some((number, wire_type)) = reader.try_read_tag(limit)? {
        match number {
            field::TYPE => envelope.signature = Some(reader.read_string()?),
            field::VERSION => envelope.version = Some(reader.read_string()?),
            field::PAYLOAD => envelope.payload = Some(reader.read_bytes()?.to_vec()),
            field::HASHES => envelope.hashes.push(reader.read_string()?),
            field::PAYLOAD_HASH => envelope.payload_hash = Some(reader.read_string()?),
            field::ID => envelope.id = Some(reader.read_string()?),
            field::TITLE => envelope.title = Some(reader.read_string()?),
            field::MAKER => envelope.author = Some(reader.read_string()?),
            field::GAME_DEV => envelope.publisher = Some(reader.read_string()?),
            field::DES => envelope.description = Some(reader.read_string()?),
            field::IDS => read_ids(&mut reader, &mut envelope)?,
            _ => reader.skip(number, wire_type)?,
        }
    }

    Ok(envelope)
}

/// Reads the field numbers of the envelope in the order they appear, which is what the
/// acceptance criterion of REQ-PROF-071 looks at.
///
/// # Errors
///
/// The bytes are not a well-formed envelope.
pub fn field_order(bytes: &[u8]) -> Result<Vec<u32>, ProfileFormatError> {
    let mut reader = ProtoReader::new(bytes);
    let limit = reader.end();
    let mut order = Vec::new();
    while let Some((number, wire_type)) = reader.try_read_tag(limit)? {
        order.push(number);
        reader.skip(number, wire_type)?;
    }

    Ok(order)
}

/// Writes the canonical envelope of REQ-PROF-084: fields 1, 2, 3, 4, 6 to 12 in that order, null
/// strings and empty lists left out, `Version = "5"`, `Ids` and `Hashes` sorted by their text
/// form (ordinal) and `PayloadHash` recomputed.
///
/// The version is the canonical one whatever the file carried: versions `"1"` and `"2"` are
/// readable but `"5"` accepts everything they do (REQ-PROF-072), so writing it loses nothing. All
/// 101 built-in profiles of 3dSen 2.1 are already `"5"`, which is what keeps their rewrite
/// byte-identical.
#[must_use]
pub fn encode(envelope: &HashProfileEnvelope) -> Vec<u8> {
    let payload = envelope.payload.as_deref().unwrap_or_default();
    let mut writer =
        ProtoWriter::with_capacity(payload.len() + envelope.ids.len() * tile_id::BYTE_COUNT + 1024);

    writer.write_string(field::TYPE, SIGNATURE);
    writer.write_string(field::VERSION, CANONICAL_VERSION);
    writer.write_bytes(field::PAYLOAD, payload);

    let mut hashes: Vec<&str> = envelope.hashes.iter().map(String::as_str).collect();
    hashes.sort_unstable();
    for hash in hashes {
        writer.write_string(field::HASHES, hash);
    }

    writer.write_string(field::PAYLOAD_HASH, &envelope.compute_payload_hash());

    write_optional_string(&mut writer, field::ID, envelope.id.as_deref());
    write_optional_string(&mut writer, field::TITLE, envelope.title.as_deref());
    write_optional_string(&mut writer, field::MAKER, envelope.author.as_deref());
    write_optional_string(&mut writer, field::GAME_DEV, envelope.publisher.as_deref());
    write_optional_string(&mut writer, field::DES, envelope.description.as_deref());

    // Field 12 is a bytes field: an empty table is still written when the profile carries one,
    // which is what keeps `empty.bytes` identical (REQ-PROF-084).
    if !envelope.ids.is_empty() || envelope.has_ids {
        let mut ids: Vec<(String, TileId)> =
            envelope.ids.iter().map(|id| (id.to_text(), *id)).collect();
        ids.sort_unstable_by(|left, right| left.0.cmp(&right.0));
        let mut table = Vec::with_capacity(ids.len() * tile_id::BYTE_COUNT);
        for (_, id) in &ids {
            table.extend_from_slice(&id.to_binary());
        }

        writer.write_bytes(field::IDS, &table);
    }

    writer.into_bytes()
}

fn read_ids(
    reader: &mut ProtoReader<'_>,
    envelope: &mut HashProfileEnvelope,
) -> Result<(), ProfileFormatError> {
    let table = reader.read_bytes()?;
    envelope.has_ids = true;
    envelope.ids_byte_length = table.len();
    for chunk in table.chunks_exact(tile_id::BYTE_COUNT) {
        if let Some(id) = TileId::from_binary(chunk) {
            envelope.ids.push(id);
        }
    }

    Ok(())
}

fn write_optional_string(writer: &mut ProtoWriter, number: u32, value: Option<&str>) {
    if let Some(value) = value {
        writer.write_string(number, value);
    }
}
