//! The envelope of a "3dh" profile: recognition, shape, manifest, validation and the library
//! header (AMD-062 REQ-PROF-070 to REQ-PROF-073).

use zax_profiles::format::{self, ProfileLoadErrorKind};
use zax_profiles::hash::envelope::{self, messages, HashProfileEnvelope};
use zax_profiles::hash::tile_id::TileId;
use zax_profiles::wire::hash_envelope;

use super::common::neutral_assets;
use super::common::synthetic_hash_profile::EnvelopeSpec;

fn decode(bytes: &[u8]) -> HashProfileEnvelope {
    hash_envelope::decode(bytes).unwrap()
}

// REQ-PROF-070: the 101 files of profiles-2.1 are recognised as "3dh", the 101 of profiles as
// 3dn.
#[test]
#[ignore = "needs the built-in profiles: set ZAXNES_PROFILES to the folder that holds profiles/ and profiles-2.1/"]
fn recognition_separates_the_two_formats() {
    let hash_files = neutral_assets::hash_profile_files();
    assert_eq!(101, hash_files.len());
    for file in &hash_files {
        let bytes = std::fs::read(file).unwrap();
        assert!(
            envelope::is_hash_profile(&bytes),
            "{} should be a \"3dh\" profile.",
            file.display()
        );
        assert_eq!(envelope::SIGNATURE_BYTES, bytes[..5]);
    }

    let plain_files = neutral_assets::profile_files();
    assert_eq!(101, plain_files.len());
    for file in &plain_files {
        let bytes = std::fs::read(file).unwrap();
        assert!(
            !envelope::is_hash_profile(&bytes),
            "{} should be a 3dn profile.",
            file.display()
        );
    }
}

#[test]
fn recognition_needs_the_five_bytes_exactly() {
    assert!(!envelope::is_hash_profile(&[]));
    assert!(!envelope::is_hash_profile(&[0x0A, 0x03, 0x33, 0x64]));
    assert!(!envelope::is_hash_profile(&[0x0A, 0x03, 0x33, 0x64, 0x6E]));
    assert!(envelope::is_hash_profile(&[
        0x0A, 0x03, 0x33, 0x64, 0x68, 0x12
    ]));
}

// REQ-PROF-071: BalloonFight (2.1) has fields 1, 2, 3, 6, 7, 12 in that order, Version = "5",
// 330 identifiers (5 280 bytes), PayloadHash equal to the SHA-256 of Payload.
#[test]
#[ignore = "needs the built-in profiles: set ZAXNES_PROFILES to the folder that holds profiles/ and profiles-2.1/"]
fn envelope_of_balloon_fight_has_the_expected_shape() {
    let bytes = neutral_assets::read_hash_profile("BalloonFight.bytes");
    assert_eq!(
        vec![1, 2, 3, 6, 7, 12],
        hash_envelope::field_order(&bytes).unwrap()
    );

    let read = decode(&bytes);
    assert_eq!(Some("3dh"), read.signature.as_deref());
    assert_eq!(Some("5"), read.version.as_deref());
    assert_eq!(330, read.ids.len());
    assert_eq!(330 * 16, read.ids_byte_length);
    assert_eq!(
        read.payload_hash.as_deref(),
        Some(read.compute_payload_hash().as_str())
    );
    assert_eq!(0, read.hashes.len());

    let manifest = read.validate().unwrap();
    assert_eq!(330, manifest.len());
    assert!(!manifest.has_legacy_digests());
}

// REQ-PROF-072, notes: the 101 files are version "5", with an empty Hashes and no tile in
// field 4; the manifest is therefore the identifier table alone.
#[test]
#[ignore = "needs the built-in profiles: set ZAXNES_PROFILES to the folder that holds profiles/ and profiles-2.1/"]
fn every_built_in_hash_profile_is_version_5_without_legacy_digests() {
    for file in neutral_assets::hash_profile_files() {
        let bytes = std::fs::read(&file).unwrap();
        let read = decode(&bytes);
        let name = file.file_name().unwrap().to_string_lossy().into_owned();
        assert_eq!(Some("5"), read.version.as_deref(), "{name}");
        assert!(read.hashes.is_empty(), "{name}");
        assert_eq!(
            read.payload_hash.as_deref(),
            Some(read.compute_payload_hash().as_str()),
            "{name}"
        );
        assert!(read.has_ids, "{name}");
        let manifest = read
            .validate()
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(!manifest.has_legacy_digests(), "{name}");
    }
}

// REQ-PROF-072: a payload that does not match its recorded digest.
#[test]
fn a_damaged_payload_is_refused() {
    let spec = EnvelopeSpec {
        payload: Some(vec![1, 2, 3, 4]),
        payload_hash_override: Some("0".repeat(64)),
        ..EnvelopeSpec::default()
    };

    let error = decode(&spec.encode()).validate().unwrap_err();
    assert_eq!(messages::DAMAGED_PAYLOAD, error.message);
    assert_eq!(ProfileLoadErrorKind::DamagedHashPayload, error.kind);
}

#[test]
fn an_absent_payload_hash_is_not_checked() {
    let spec = EnvelopeSpec {
        payload: Some(vec![1, 2, 3, 4]),
        write_payload_hash: false,
        ..EnvelopeSpec::default()
    };

    let read = decode(&spec.encode());
    assert_eq!(None, read.payload_hash);
    assert!(read.validate().is_ok());
}

// REQ-PROF-072: version "4" has its own message, every other unsupported version shares one.
#[test]
fn envelope_version_4_is_refused_with_its_own_message() {
    let spec = EnvelopeSpec {
        version: Some("4".to_owned()),
        ..EnvelopeSpec::default()
    };

    let error = decode(&spec.encode()).validate().unwrap_err();
    assert_eq!(messages::VERSION_4, error.message);
    assert_eq!(
        "Hash profile envelope version 4 is no longer supported: its tile identifiers cannot be \
         resolved. Import the original profile again.",
        error.message
    );
    assert_eq!(ProfileLoadErrorKind::UnsupportedHashEnvelope, error.kind);
}

#[test]
fn an_unsupported_envelope_version_is_refused() {
    for version in ["0", "3", "6", "", "five"] {
        let spec = EnvelopeSpec {
            version: Some(version.to_owned()),
            ..EnvelopeSpec::default()
        };
        let error = decode(&spec.encode()).validate().unwrap_err();
        assert_eq!(messages::UNSUPPORTED_ENVELOPE, error.message, "{version}");
        assert_eq!(ProfileLoadErrorKind::UnsupportedHashEnvelope, error.kind);
    }
}

#[test]
fn an_absent_envelope_version_is_refused() {
    let spec = EnvelopeSpec {
        version: None,
        ..EnvelopeSpec::default()
    };
    let error = decode(&spec.encode()).validate().unwrap_err();
    assert_eq!(messages::UNSUPPORTED_ENVELOPE, error.message);
}

#[test]
fn versions_1_2_and_5_are_accepted() {
    for version in ["1", "2", "5"] {
        let spec = EnvelopeSpec {
            version: Some(version.to_owned()),
            ..EnvelopeSpec::default()
        };
        assert!(
            decode(&spec.encode()).validate().is_ok(),
            "version {version}"
        );
    }
}

#[test]
fn an_envelope_without_a_payload_is_refused() {
    let spec = EnvelopeSpec {
        payload: None,
        write_payload_hash: false,
        ..EnvelopeSpec::default()
    };

    let error = decode(&spec.encode()).validate().unwrap_err();
    assert_eq!(messages::UNSUPPORTED_ENVELOPE, error.message);
    assert_eq!(ProfileLoadErrorKind::UnsupportedHashEnvelope, error.kind);
}

// REQ-PROF-072: the manifest refusals.
#[test]
fn a_duplicated_identifier_makes_the_manifest_invalid() {
    let id = TileId::compute(&[0u8; 16]);
    let spec = EnvelopeSpec {
        ids: vec![id, id],
        ..EnvelopeSpec::default()
    };

    let error = decode(&spec.encode()).validate().unwrap_err();
    assert_eq!(messages::INVALID_MANIFEST, error.message);
    assert_eq!(ProfileLoadErrorKind::InvalidHashManifest, error.kind);
}

#[test]
fn an_identifier_also_listed_in_hashes_makes_the_manifest_invalid() {
    let id = TileId::compute(&[0u8; 16]);
    let spec = EnvelopeSpec {
        hashes: vec![id.to_text()],
        ids: vec![id],
        ..EnvelopeSpec::default()
    };

    let error = decode(&spec.encode()).validate().unwrap_err();
    assert_eq!(messages::INVALID_MANIFEST, error.message);
}

#[test]
fn an_ids_length_that_is_not_a_multiple_of_sixteen_makes_the_manifest_invalid() {
    let spec = EnvelopeSpec {
        raw_ids: Some(vec![0u8; 16 + 3]),
        ..EnvelopeSpec::default()
    };

    let read = decode(&spec.encode());
    assert_eq!(19, read.ids_byte_length);
    // The one whole entry is still read; the length is what refuses the file.
    assert_eq!(1, read.ids.len());
    let error = read.validate().unwrap_err();
    assert_eq!(messages::INVALID_MANIFEST, error.message);
}

#[test]
fn a_thirty_two_digit_entry_of_hashes_is_an_identifier_in_versions_2_and_5_only() {
    let text = TileId::compute(&[0u8; 16]).to_text();

    for version in ["2", "5"] {
        let spec = EnvelopeSpec {
            version: Some(version.to_owned()),
            hashes: vec![text.clone()],
            ..EnvelopeSpec::default()
        };
        let read = decode(&spec.encode());
        assert!(read.accepts_text_identifiers());
        let manifest = read.validate().unwrap();
        assert_eq!(1, manifest.len());
        // A text entry is indexed by its prefix, like an identifier of field 12, so the fast
        // filter of REQ-PROF-077 finds it.
        assert!(manifest.has_prefix(TileId::compute(&[0u8; 16]).prefix()));
        assert!(!manifest.has_legacy_digests());
    }

    let spec = EnvelopeSpec {
        version: Some("1".to_owned()),
        hashes: vec![text],
        ..EnvelopeSpec::default()
    };
    let read = decode(&spec.encode());
    assert!(!read.accepts_text_identifiers());
    assert_eq!(
        messages::INVALID_MANIFEST,
        read.validate().unwrap_err().message
    );
}

#[test]
fn a_legacy_digest_is_accepted_in_every_version() {
    let digest = zax_profiles::hash::raw_tile::legacy_digest(&[0u8; 16]);
    for version in ["1", "2", "5"] {
        let spec = EnvelopeSpec {
            version: Some(version.to_owned()),
            hashes: vec![digest.clone()],
            ..EnvelopeSpec::default()
        };
        let manifest = decode(&spec.encode()).validate().unwrap();
        assert!(manifest.has_legacy_digests(), "version {version}");
        assert!(manifest.contains(&digest));
        // A legacy digest has no identifier, so it is not in the prefix index.
        assert!(!manifest.has_prefix(TileId::compute(&[0u8; 16]).prefix()));
    }
}

#[test]
fn a_hashes_entry_that_is_not_lowercase_hexadecimal_makes_the_manifest_invalid() {
    for entry in [
        "A".repeat(64),
        "g".repeat(64),
        "a".repeat(63),
        "a".repeat(31),
        String::new(),
    ] {
        let spec = EnvelopeSpec {
            hashes: vec![entry.clone()],
            ..EnvelopeSpec::default()
        };
        assert_eq!(
            messages::INVALID_MANIFEST,
            decode(&spec.encode()).validate().unwrap_err().message,
            "{entry:?}"
        );
    }
}

#[test]
fn a_duplicated_legacy_digest_makes_the_manifest_invalid() {
    let digest = zax_profiles::hash::raw_tile::legacy_digest(&[0u8; 16]);
    let spec = EnvelopeSpec {
        hashes: vec![digest.clone(), digest],
        ..EnvelopeSpec::default()
    };
    assert_eq!(
        messages::INVALID_MANIFEST,
        decode(&spec.encode()).validate().unwrap_err().message
    );
}

// §3: every comparison is made on the manifest strings, and the prefix index of REQ-PROF-077
// groups the identifiers that share a prefix.
#[test]
fn the_manifest_holds_its_strings_in_the_order_they_were_added() {
    let first = TileId::compute(&[0u8; 16]);
    let second = TileId::compute(&[1u8; 16]);
    let digest = zax_profiles::hash::raw_tile::legacy_digest(&[2u8; 16]);
    let spec = EnvelopeSpec {
        hashes: vec![digest.clone()],
        ids: vec![first, second],
        ..EnvelopeSpec::default()
    };

    let manifest = decode(&spec.encode()).validate().unwrap();
    assert_eq!(
        vec![digest, first.to_text(), second.to_text()],
        manifest.keys()
    );
    assert_eq!(vec![first, second], manifest.identifiers());
    assert_eq!(3, manifest.len());
    assert!(!manifest.is_empty());
    assert!(!manifest.contains("absent"));
    assert_eq!(
        vec![first.to_text()],
        manifest.keys_with_prefix(first.prefix())
    );
    assert_eq!(0, manifest.keys_with_prefix(0xDEAD_BEEF).len());
}

// REQ-PROF-071: the fields may come in any order, an unknown field is skipped and an absent
// field is none (SPEC-03 §3.1). Field 5 is never written, and is skipped if met.
#[test]
fn the_envelope_reader_accepts_any_order_and_skips_what_it_does_not_know() {
    use super::common::proto_builder::ProtoBuilder;

    let id = TileId::compute(&[7u8; 16]);
    let mut writer = ProtoBuilder::new();
    writer.raw_bytes(12, &id.to_binary());
    writer.string(11, "description");
    writer.int32(5, 42);
    writer.string(2, "5");
    writer.string(30, "a field outside the schema");
    writer.raw_bytes(3, &[9, 9]);
    writer.string(1, "3dh");
    writer.string(7, "Id");
    let bytes = writer.into_bytes();

    let read = decode(&bytes);
    assert_eq!(Some("3dh"), read.signature.as_deref());
    assert_eq!(Some("5"), read.version.as_deref());
    assert_eq!(Some(&vec![9, 9]), read.payload.as_ref());
    assert_eq!(Some("Id"), read.id.as_deref());
    assert_eq!(Some("description"), read.description.as_deref());
    assert_eq!(None, read.title);
    assert_eq!(None, read.author);
    assert_eq!(None, read.publisher);
    assert_eq!(vec![id], read.ids);
    assert!(read.has_ids);
    assert_eq!(
        vec![12, 11, 5, 2, 30, 3, 1, 7],
        hash_envelope::field_order(&bytes).unwrap()
    );
}

// REQ-PROF-073: the metadata of a "3dh" are read in the envelope, fields 7 to 11, the payload
// not being decoded for that.
#[test]
fn the_library_header_comes_from_the_envelope() {
    let spec = EnvelopeSpec {
        id: Some("Jhj6E7iOysxT".to_owned()),
        title: Some("Super Mario Bros".to_owned()),
        author: Some("Maker".to_owned()),
        publisher: Some("GameDev".to_owned()),
        description: Some("Des".to_owned()),
        // A payload the reader could not decode: the header does not look at it.
        payload: Some(vec![0xFF; 7]),
        ..EnvelopeSpec::default()
    };
    let bytes = spec.encode();

    let header = format::load_header(&bytes, None, None).unwrap();
    assert!(header.is_hash_envelope());
    assert!(header.has_acceptable_signature());
    assert_eq!(Some("3dh"), header.signature.as_deref());
    assert_eq!(Some("5"), header.version.as_deref());
    assert_eq!(Some("Jhj6E7iOysxT"), header.id.as_deref());
    assert_eq!(Some("Super Mario Bros"), header.title.as_deref());
    assert_eq!(Some("Maker"), header.author.as_deref());
    assert_eq!(Some("GameDev"), header.publisher.as_deref());
    assert_eq!(Some("Des"), header.description.as_deref());
    assert!(header.has_id());
}

// REQ-PROF-073: a file whose payload is damaged, which the loader refuses, still shows up in the
// list of games with the Id and the Title of its envelope.
#[test]
fn a_refused_hash_profile_still_shows_its_header() {
    let spec = EnvelopeSpec {
        payload: Some(vec![1, 2, 3, 4]),
        payload_hash_override: Some("0".repeat(64)),
        id: Some("Jhj6E7iOysxT".to_owned()),
        title: Some("Super Mario Bros".to_owned()),
        ..EnvelopeSpec::default()
    };
    let bytes = spec.encode();

    assert!(decode(&bytes).validate().is_err());

    let header = format::load_header(&bytes, None, None).unwrap();
    assert_eq!(Some("Jhj6E7iOysxT"), header.id.as_deref());
    assert_eq!(Some("Super Mario Bros"), header.title.as_deref());
}

// REQ-PROF-070: a 3dh signature is no longer refused by REQ-PROF-020, and the version the header
// classifies is the envelope version (REQ-PROF-072).
#[test]
fn the_header_version_of_a_hash_envelope_is_the_envelope_version() {
    use zax_profiles::enums::ProfileFormatVersion;

    for version in ["1", "2", "5"] {
        let spec = EnvelopeSpec {
            version: Some(version.to_owned()),
            ..EnvelopeSpec::default()
        };
        let header = format::load_header(&spec.encode(), None, None).unwrap();
        assert_eq!(ProfileFormatVersion::Current, header.format_version());
        assert!(!header.has_unsupported_version(), "version {version}");
    }

    for version in ["3", "4"] {
        let spec = EnvelopeSpec {
            version: Some(version.to_owned()),
            ..EnvelopeSpec::default()
        };
        let header = format::load_header(&spec.encode(), None, None).unwrap();
        assert!(header.has_unsupported_version(), "version {version}");
    }
}

// REQ-PROF-073: the 101 envelopes all carry an Id; 56 of their 1.x counterparts carry one too,
// equal in every case, and the 45 others have none (AMD-037).
#[test]
#[ignore = "needs the built-in profiles: set ZAXNES_PROFILES to the folder that holds profiles/ and profiles-2.1/"]
fn every_envelope_carries_an_identifier_and_matches_its_3dn_counterpart() {
    let mut with_counterpart_id = 0;
    let mut without_counterpart_id = 0;
    for file in neutral_assets::hash_profile_files() {
        let name = file.file_name().unwrap().to_string_lossy().into_owned();
        let header = format::load_header(&std::fs::read(&file).unwrap(), None, None).unwrap();
        assert!(header.has_id(), "{name} has no envelope Id.");

        let counterpart =
            neutral_assets::profiles_directory().join(neutral_assets::counterpart_name(&name));
        let plain = format::load_header(&std::fs::read(&counterpart).unwrap(), None, None).unwrap();
        if plain.has_id() {
            assert_eq!(plain.id, header.id, "{name}");
            with_counterpart_id += 1;
        } else {
            without_counterpart_id += 1;
        }
    }

    assert_eq!(56, with_counterpart_id);
    assert_eq!(45, without_counterpart_id);
}

// REQ-PROF-084, envelope half: the canonical writer puts the fields back in the order 1, 2, 3,
// 4, 6 to 12, omits null strings and empty lists, sorts Ids and Hashes by their text form and
// recomputes PayloadHash. (The payload half lands with the payload codec.)
#[test]
fn the_canonical_envelope_writer_sorts_and_recomputes() {
    let first = TileId::compute(&[0u8; 16]);
    let second = TileId::compute(&[1u8; 16]);
    let third = TileId::compute(&[2u8; 16]);
    let digest_a = zax_profiles::hash::raw_tile::legacy_digest(&[3u8; 16]);
    let digest_b = zax_profiles::hash::raw_tile::legacy_digest(&[4u8; 16]);

    let mut sorted_ids = vec![first, second, third];
    sorted_ids.sort_by_key(|id| id.to_text());
    let mut sorted_hashes = vec![digest_a.clone(), digest_b.clone()];
    sorted_hashes.sort();

    let source = HashProfileEnvelope {
        signature: Some("3dh".to_owned()),
        version: Some("5".to_owned()),
        payload: Some(vec![1, 2, 3]),
        hashes: vec![digest_b, digest_a],
        payload_hash: Some("0".repeat(64)),
        id: Some("Id".to_owned()),
        title: None,
        author: None,
        publisher: None,
        description: Some("Des".to_owned()),
        ids: vec![third, first, second],
        ids_byte_length: 48,
        has_ids: true,
    };

    let written = hash_envelope::encode(&source);
    assert!(envelope::is_hash_profile(&written));
    assert_eq!(
        vec![1, 2, 3, 4, 4, 6, 7, 11, 12],
        hash_envelope::field_order(&written).unwrap()
    );

    let read = decode(&written);
    assert_eq!(sorted_hashes, read.hashes);
    assert_eq!(sorted_ids, read.ids);
    assert_eq!(
        read.payload_hash.as_deref(),
        Some(read.compute_payload_hash().as_str())
    );
    assert_eq!(Some(&vec![1, 2, 3]), read.payload.as_ref());
    assert_eq!(None, read.title);
    assert_eq!(Some("Des"), read.description.as_deref());

    // Writing the canonical form again changes nothing.
    assert_eq!(written, hash_envelope::encode(&read));
}

// REQ-PROF-084: field 12 is a bytes field, not a repeated one, so a profile that carries an
// empty table keeps it, which is what makes `empty.bytes` rewrite byte for byte.
#[test]
fn an_empty_identifier_table_is_written_back_when_it_was_read() {
    let with_empty_table = HashProfileEnvelope {
        signature: Some("3dh".to_owned()),
        version: Some("5".to_owned()),
        payload: Some(Vec::new()),
        has_ids: true,
        ..HashProfileEnvelope::default()
    };
    assert_eq!(
        vec![1, 2, 3, 6, 12],
        hash_envelope::field_order(&hash_envelope::encode(&with_empty_table)).unwrap()
    );

    let without_table = HashProfileEnvelope {
        has_ids: false,
        ..with_empty_table
    };
    assert_eq!(
        vec![1, 2, 3, 6],
        hash_envelope::field_order(&hash_envelope::encode(&without_table)).unwrap()
    );
}

#[test]
fn the_default_envelope_signature_and_version_are_the_canonical_ones() {
    let written = hash_envelope::encode(&HashProfileEnvelope::default());
    let read = decode(&written);
    assert_eq!(Some(envelope::SIGNATURE), read.signature.as_deref());
    assert_eq!(Some(envelope::CANONICAL_VERSION), read.version.as_deref());
    assert_eq!("3dh", envelope::SIGNATURE);
    assert_eq!("5", envelope::CANONICAL_VERSION);
}
