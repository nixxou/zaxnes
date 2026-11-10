//! The lossless rewrite of a "3dh" profile (AMD-062 REQ-PROF-084): the canonical envelope, the
//! canonical payload with the four deviations of REQ-PROF-074, and the route through
//! `format::load` that makes a profile keep its own format.

use zax_profiles::format::{self, ProfileDocument, ProfileLoadErrorKind};
use zax_profiles::hash::envelope::{self, messages};
use zax_profiles::hash::profile;
use zax_profiles::hash::raw_tile::{self, RawTile};
use zax_profiles::hash::tile_id::TileId;
use zax_profiles::layout::FieldMask;
use zax_profiles::tile::HashTileField;
use zax_profiles::wire::hash_envelope;

use super::common::hash_state;
use super::common::neutral_assets;
use super::common::recording_log::RecordingProfileLog;
use super::common::synthetic_hash_profile::{
    EnvelopeSpec, PatternSpec, PayloadSpec, TextureSpec, TileSpec, VariantSpec,
};
use super::common::synthetic_rom;

/// The two profiles of the 2.1 bundle that gained patterns in 2.0, whose 1.x counterpart
/// therefore cannot give every tile (AMD-062 §1, REQ-PROF-084).
const WITHOUT_A_FULL_COUNTERPART: [(&str, usize); 2] =
    [("DQ3_DW3.bytes", 173), ("Final_Fantasy_USA.bytes", 66)];

/// A raw tile whose bytes are `seed`, `seed + 1`, ...
fn raw(seed: u8) -> RawTile {
    let mut bytes = [0_u8; 16];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = seed.wrapping_add(u8::try_from(index).unwrap());
    }
    bytes
}

fn read(bytes: &[u8]) -> ProfileDocument {
    profile::read(bytes).unwrap_or_else(|error| panic!("{error}"))
}

/// The field numbers of the rewritten payload's root message, each once, in the order they are
/// written: a repeated field (the scripts, the two pattern lists) appears at its first place.
fn payload_field_order(document: &ProfileDocument) -> Vec<u32> {
    let rewritten = document.save();
    let payload = hash_envelope::decode(&rewritten)
        .unwrap()
        .payload
        .expect("the canonical envelope always writes field 3");
    let mut order = hash_envelope::field_order(&payload).unwrap();
    order.dedup();
    order
}

fn describe_first_difference(expected: &[u8], actual: &[u8]) -> String {
    for (index, (expected_byte, actual_byte)) in expected.iter().zip(actual).enumerate() {
        if expected_byte != actual_byte {
            return format!(
                "byte {index} is 0x{actual_byte:02X}, expected 0x{expected_byte:02X} (lengths {} and {})",
                actual.len(),
                expected.len()
            );
        }
    }
    format!("length {}, expected {}", actual.len(), expected.len())
}

// REQ-PROF-084: decoding then rewriting each of the 101 files of profiles-2.1 gives the same
// bytes back, with no ROM and so with every tile still waiting.
#[test]
#[ignore = "needs the built-in profiles: set ZAXNES_PROFILES to the folder that holds profiles/ and profiles-2.1/"]
fn req_prof_084_every_hash_profile_is_rewritten_byte_for_byte() {
    let mut failures = Vec::new();
    let mut identical = 0;
    for path in neutral_assets::hash_profile_files() {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let original = std::fs::read(&path).unwrap();
        let document = match format::load(&original, None, Some(&name)) {
            Ok(document) => document,
            Err(error) => {
                failures.push(format!("{name}: refused: {error}"));
                continue;
            }
        };

        // The loader recognised a 3dh and so will rewrite one (AMD-062 §2: never converted).
        assert!(document.is_hash_profile(), "{name}");
        assert_eq!(0, hash_state(&document).tiles.resolved_count(), "{name}");

        let rewritten = document.save();
        if rewritten == original {
            identical += 1;
        } else {
            failures.push(format!(
                "{name}: {}",
                describe_first_difference(&original, &rewritten)
            ));
        }
    }

    assert!(failures.is_empty(), "{failures:?}");
    assert_eq!(101, identical);
}

// REQ-PROF-084: the same 99 pairable profiles, rewritten after every tile has been resolved from
// a ROM. A resolved tile goes back as its identifier, never as its planes.
#[test]
#[ignore = "needs the built-in profiles: set ZAXNES_PROFILES to the folder that holds profiles/ and profiles-2.1/"]
fn req_prof_084_a_fully_resolved_hash_profile_is_rewritten_byte_for_byte() {
    let unpairable: Vec<&str> = WITHOUT_A_FULL_COUNTERPART
        .iter()
        .map(|(name, _)| *name)
        .collect();
    let mut failures = Vec::new();
    let mut identical = 0;
    for path in neutral_assets::hash_profile_files() {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if unpairable.contains(&name.as_str()) {
            continue;
        }

        let original = std::fs::read(&path).unwrap();
        let mut document = format::load(&original, None, Some(&name)).unwrap();
        document.resolve_from_rom(&synthetic_rom::chr_rom_with_tiles(&counterpart_tiles(
            &name,
        )));
        assert!(hash_state(&document).tiles.is_complete(), "{name}");
        assert_eq!(0, document.waiting_tile_count(), "{name}");

        let rewritten = document.save();
        if rewritten == original {
            identical += 1;
        } else {
            failures.push(format!(
                "{name}: {}",
                describe_first_difference(&original, &rewritten)
            ));
        }
    }

    assert!(failures.is_empty(), "{failures:?}");
    assert_eq!(99, identical);
}

// REQ-PROF-084 and Q-178 §3: DQ3_DW3 and Final_Fantasy_USA gained patterns in 2.0, so their 1.x
// counterpart cannot give every tile — 173 and 66 manifest strings stay waiting, the 67th of
// Final Fantasy being the mirror of a tile the four flips of REQ-PROF-079 do resolve — and the
// rewrite is identical with those tiles half resolved.
#[test]
#[ignore = "needs the built-in profiles: set ZAXNES_PROFILES to the folder that holds profiles/ and profiles-2.1/"]
fn req_prof_084_the_profiles_without_a_full_counterpart_keep_their_waiting_strings() {
    for (name, waiting) in WITHOUT_A_FULL_COUNTERPART {
        let original = neutral_assets::read_hash_profile(name);
        let mut document = format::load(&original, None, Some(name)).unwrap();
        document.resolve_from_rom(&synthetic_rom::chr_rom_with_tiles(&counterpart_tiles(name)));

        assert_eq!(
            waiting,
            hash_state(&document).tiles.waiting_count(),
            "{name}"
        );
        assert!(document.waiting_tile_count() > 0, "{name}");
        let rewritten = document.save();
        assert!(
            rewritten == original,
            "{name}: {}",
            describe_first_difference(&original, &rewritten)
        );
    }
}

// REQ-PROF-084: field 12 (`skipGeo`) is written for every pattern of the root lists, where the
// patterns of the memorised slots keep the absence read — 480 of the 1 010 slots have none.
#[test]
#[ignore = "needs the built-in profiles: set ZAXNES_PROFILES to the folder that holds profiles/ and profiles-2.1/"]
fn req_prof_084_the_root_patterns_carry_skip_geo_and_the_memorised_slots_keep_what_they_had() {
    let mut root_patterns = 0;
    let mut root_without_skip_geo = 0;
    let mut slot_patterns = 0;
    let mut slots_without_skip_geo = 0;
    for path in neutral_assets::hash_profile_files() {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let document = format::load(&std::fs::read(&path).unwrap(), None, Some(&name)).unwrap();
        for pattern in document.profile.all_patterns() {
            root_patterns += 1;
            if pattern
                .recorded_fields
                .is_some_and(|mask| !mask.contains(12))
            {
                root_without_skip_geo += 1;
            }
        }

        let settings = document.profile.settings.as_ref().unwrap();
        for slot in settings
            .saved_patterns
            .iter()
            .chain(&settings.saved_anim_patterns)
        {
            let Some(pattern) = slot.pattern.as_ref() else {
                continue;
            };
            slot_patterns += 1;
            if pattern
                .recorded_fields
                .is_some_and(|mask| !mask.contains(12))
            {
                slots_without_skip_geo += 1;
            }
        }
    }

    assert_eq!(67_924, root_patterns);
    assert_eq!(0, root_without_skip_geo);
    assert_eq!(1_010, slot_patterns);
    assert_eq!(480, slots_without_skip_geo);
}

// REQ-PROF-074 point 3: the rewritten payload writes fields 3, 4, 5, 6 first, ascending, then the
// others — 1, 2, 7 to 11 — ascending. `empty.bytes`, the one profile with no tile, gives
// 3, 6, 1, 2.
#[test]
#[ignore = "needs the built-in profiles: set ZAXNES_PROFILES to the folder that holds profiles/ and profiles-2.1/"]
fn req_prof_074_the_rewritten_payload_keeps_the_root_field_order_of_the_format() {
    let empty = read(&neutral_assets::read_hash_profile("empty.bytes"));
    assert_eq!(vec![3, 6, 1, 2], payload_field_order(&empty));

    let mega_man = read(&neutral_assets::read_hash_profile("Mega Man 2.bytes"));
    assert_eq!(vec![3, 4, 5, 6, 1, 2], payload_field_order(&mega_man));
}

// The same order from a payload whose root fields were read in the ascending order of a 3dn: the
// reader accepts any order, the writer puts back the one REQ-PROF-074 prescribes.
#[test]
fn the_rewritten_payload_reorders_a_payload_read_in_another_order() {
    let id = TileId::compute(&raw(1));
    let payload = PayloadSpec::of(vec![PatternSpec::row(vec![Some(TileSpec::named(id))])]);
    let document = read(
        &EnvelopeSpec {
            ids: vec![id],
            payload: Some(payload.encode()),
            ..EnvelopeSpec::default()
        }
        .encode(),
    );

    // The synthetic payload writes 1, 2 then 4; the rewrite writes 4 then 1, 2.
    assert_eq!(
        vec![1, 2, 4],
        hash_envelope::field_order(&payload.encode()).unwrap()
    );
    assert_eq!(vec![4, 1, 2], payload_field_order(&document));
}

// REQ-PROF-084: a tile goes back in the field that named it — field 5 for an identifier, field 4
// for a string — and never by its planes.
#[test]
fn req_prof_084_a_tile_goes_back_in_the_field_that_named_it() {
    let tiles = [raw(2), raw(3)];
    let digests: Vec<String> = tiles.iter().map(raw_tile::legacy_digest).collect();

    for (expected, document) in [
        (
            HashTileField::Identifier,
            synthetic_rom::hash_profile_of(&tiles),
        ),
        (
            HashTileField::Text,
            synthetic_rom::hash_profile_named_by_digest(&tiles),
        ),
    ] {
        let again = read(&document.save());
        let mut count = 0;
        for tile in again.profile.distinct_tile_objects() {
            count += 1;
            let identity = tile.hash_identity.as_ref().unwrap();
            assert_eq!(expected, identity.field);
            assert_eq!(0, tile.low_plane);
            assert_eq!(0, tile.high_plane);
            // The recorded set is kept only when it differs from the schema of a "3dh" tile,
            // {3, 5}; a tile named in field 4 carries {3, 4} instead.
            match expected {
                HashTileField::Identifier => assert_eq!(None, tile.recorded_fields),
                HashTileField::Text => {
                    assert_eq!(vec![3, 4], tile.recorded_fields.unwrap().fields());
                }
            }
        }
        assert_eq!(2, count);
        assert!(digests.iter().all(|digest| digest.len() == 64));
    }
}

// REQ-PROF-084: a tile resolved from a ROM is rewritten by its identifier, never by its planes —
// the bytes are the same before and after the resolution.
#[test]
fn req_prof_084_resolving_the_tiles_does_not_change_the_bytes_written() {
    let tiles = [raw(4), raw(5), raw(6)];
    let mut document = synthetic_rom::hash_profile_of(&tiles);
    let waiting = document.save();

    assert_eq!(
        3,
        document.resolve_from_rom(&synthetic_rom::chr_rom_with_tiles(&tiles))
    );
    assert_eq!(0, document.waiting_tile_count());
    assert_eq!(waiting, document.save());

    // And the planes really are absent from what was written.
    let again = read(&waiting);
    for tile in again.profile.distinct_tile_objects() {
        assert_eq!(
            None, tile.recorded_fields,
            "the fields {{3, 5}} and no other"
        );
        assert!(tile.is_waiting());
    }
}

// REQ-PROF-084: a variant's texture is written according to its kind — field 25 as read, and
// field 24 only for a kind of 2.
#[test]
fn req_prof_084_the_texture_kind_decides_which_fields_a_variant_writes() {
    let id = TileId::compute(&raw(7));
    let texture = TextureSpec::indexed(8, 8, vec![1, 2, 3, 4]);
    let variants = vec![
        VariantSpec::of_kind(0),
        VariantSpec::of_kind(1),
        VariantSpec::of_kind(2).with_xor_texture(texture),
        // No field 25 and no texture at all: a memorised slot's variant, which writes neither.
        VariantSpec::default(),
    ];
    let payload = PayloadSpec::of(vec![
        PatternSpec::row(vec![Some(TileSpec::named(id))]).with_variants(variants)
    ]);
    let document = read(
        &EnvelopeSpec {
            ids: vec![id],
            payload: Some(payload.encode()),
            ..EnvelopeSpec::default()
        }
        .encode(),
    );

    let again = read(&document.save());
    let patterns: Vec<_> = again.profile.all_patterns().collect();
    let written: Vec<Vec<u32>> = patterns[0]
        .variants
        .iter()
        .map(|variant| {
            variant
                .recorded_fields
                .map_or_else(Vec::new, FieldMask::fields)
        })
        .collect();

    assert_eq!(vec![1, 25], written[0]);
    assert_eq!(vec![1, 25], written[1]);
    assert_eq!(vec![1, 24, 25], written[2]);
    assert_eq!(vec![1], written[3]);
    for (index, kind) in [Some(0), Some(1), Some(2), None].into_iter().enumerate() {
        assert_eq!(
            kind,
            patterns[0].variants[index]
                .texture_kind
                .map(zax_profiles::variant::HashTextureKind::to_raw),
            "variant {index}"
        );
    }
}

// REQ-PROF-084: `Hashes` keeps the entries a tile still names and leaves the others out; `Ids`
// goes back whole, sorted by the text form of each identifier.
#[test]
fn req_prof_084_the_envelope_keeps_the_strings_that_are_still_named() {
    let named = raw_tile::legacy_digest(&raw(8));
    let orphan = raw_tile::legacy_digest(&raw(9));
    let ids = [TileId::compute(&raw(10)), TileId::compute(&raw(11))];
    let payload = PayloadSpec::of(vec![PatternSpec::row(vec![
        Some(TileSpec::text(&named)),
        Some(TileSpec::named(ids[0])),
        Some(TileSpec::named(ids[1])),
    ])]);
    let document = read(
        &EnvelopeSpec {
            hashes: vec![named.clone(), orphan],
            ids: ids.to_vec(),
            payload: Some(payload.encode()),
            ..EnvelopeSpec::default()
        }
        .encode(),
    );

    let rewritten = hash_envelope::decode(&document.save()).unwrap();
    assert_eq!(vec![named], rewritten.hashes);

    let mut expected: Vec<String> = ids.iter().map(|id| id.to_text()).collect();
    expected.sort();
    assert_eq!(
        expected,
        rewritten
            .ids
            .iter()
            .map(|id| id.to_text())
            .collect::<Vec<String>>()
    );
}

// REQ-PROF-084: the envelope the rewrite writes is the canonical one — the signature, version
// `"5"` whatever the file carried, and `PayloadHash` recomputed.
#[test]
fn req_prof_084_the_rewritten_envelope_is_the_canonical_one() {
    let id = TileId::compute(&raw(12));
    let payload = PayloadSpec::of(vec![PatternSpec::row(vec![Some(TileSpec::named(id))])]);
    let bytes = EnvelopeSpec {
        version: Some("1".to_owned()),
        ids: vec![id],
        payload: Some(payload.encode()),
        payload_hash_override: None,
        ..EnvelopeSpec::default()
    }
    .encode();

    let rewritten = read(&bytes).save();
    assert!(envelope::is_hash_profile(&rewritten));
    assert_eq!(
        vec![1, 2, 3, 6, 7, 8, 12],
        hash_envelope::field_order(&rewritten).unwrap()
    );

    let read_back = hash_envelope::decode(&rewritten).unwrap();
    assert_eq!(Some(envelope::SIGNATURE), read_back.signature.as_deref());
    assert_eq!(
        Some(envelope::CANONICAL_VERSION),
        read_back.version.as_deref()
    );
    assert_eq!(
        read_back.payload_hash.as_deref(),
        Some(read_back.compute_payload_hash().as_str())
    );
}

// A rewrite is a fixed point: the second one gives the first one's bytes.
#[test]
fn req_prof_084_the_rewrite_is_idempotent() {
    let document = synthetic_rom::hash_profile_of(&[raw(13), raw(14)]);
    let once = document.save();
    let twice = read(&once).save();
    assert_eq!(once, twice);
    assert!(envelope::is_hash_profile(&twice));
}

// REQ-PROF-084, notes: a 3dn is rewritten as a 3dn. The loader recognises the format by the
// content, so a profile never changes format on its own (AMD-062 §2).
#[test]
#[ignore = "needs the built-in profiles: set ZAXNES_PROFILES to the folder that holds profiles/ and profiles-2.1/"]
fn req_prof_084_a_3dn_profile_is_not_rewritten_as_a_3dh() {
    let original = neutral_assets::read_profile("Mega Man 2.bytes");
    let document = format::load(&original, None, None).unwrap();
    assert!(!document.is_hash_profile());
    assert_eq!(None, document.hash_state().map(|_| ()));

    let rewritten = document.save();
    assert!(!envelope::is_hash_profile(&rewritten));
    assert_eq!(original, rewritten);
    assert_eq!(None, document.tile_cache());
    assert_eq!(None, document.tile_cache_file_name());
    assert_eq!(0, document.waiting_tile_count());
}

// REQ-PROF-072 through the loader: `format::load` now routes a 3dh, so its refusals carry the
// prescribed log text and the kind the user interface reads.
#[test]
fn req_prof_072_the_loader_refuses_a_damaged_hash_profile_with_the_prescribed_text() {
    let id = TileId::compute(&raw(15));
    let payload = PayloadSpec::of(vec![PatternSpec::row(vec![Some(TileSpec::named(id))])]);
    let bytes = EnvelopeSpec {
        ids: vec![id],
        payload: Some(payload.encode()),
        payload_hash_override: Some("0".repeat(64)),
        ..EnvelopeSpec::default()
    }
    .encode();

    let log = RecordingProfileLog::new();
    let error = format::load(&bytes, Some(&log), Some("Damaged.3dh")).unwrap_err();
    assert_eq!(ProfileLoadErrorKind::DamagedHashPayload, error.kind);
    assert_eq!(messages::DAMAGED_PAYLOAD, error.message);
    assert_eq!(
        vec![format!("Damaged.3dh: {}", messages::DAMAGED_PAYLOAD)],
        *log.errors.borrow()
    );
}

/// The sixteen raw bytes of every distinct tile of the 1.x counterpart of `name`.
fn counterpart_tiles(name: &str) -> Vec<RawTile> {
    let plain = neutral_assets::load_profile(&neutral_assets::counterpart_name(name));
    let mut seen = std::collections::HashSet::new();
    plain
        .profile
        .distinct_tile_objects()
        .map(|tile| raw_tile::from_planes(tile.low_plane, tile.high_plane))
        .filter(|raw| seen.insert(*raw))
        .collect()
}
