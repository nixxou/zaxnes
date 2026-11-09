//! The payload of a "3dh" profile: the three differences from the schema of SPEC-03, the
//! refusals they bring, and the identity of a tile that is still waiting for its planes
//! (AMD-062 REQ-PROF-074, REQ-PROF-081).

use std::collections::BTreeSet;
use std::sync::Arc;

use zax_profiles::enums::{TextureFormat, TileFlip};
use zax_profiles::format::ProfileLoadErrorKind;
use zax_profiles::hash::profile::{self, HashProfile};
use zax_profiles::hash::raw_tile::{self, RawTile};
use zax_profiles::hash::tile_id::TileId;
use zax_profiles::tile::{HashTileField, ProfileTile};
use zax_profiles::variant::HashTextureKind;
use zax_profiles::wire::hash_envelope;

use super::common::neutral_assets;
use super::common::synthetic_hash_profile::{
    EnvelopeSpec, PatternSpec, PayloadSpec, TextureSpec, TileName, TileSpec, VariantSpec,
};

/// A raw tile whose bytes are `seed`, `seed + 1`, ...
fn raw(seed: u8) -> RawTile {
    let mut bytes = [0_u8; 16];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = seed.wrapping_add(u8::try_from(index).unwrap());
    }
    bytes
}

/// A "3dh" file whose manifest holds `hashes` and `ids` and whose payload is `payload`.
fn hash_profile(hashes: Vec<String>, ids: Vec<TileId>, payload: &PayloadSpec) -> Vec<u8> {
    EnvelopeSpec {
        hashes,
        ids,
        payload: Some(payload.encode()),
        ..EnvelopeSpec::default()
    }
    .encode()
}

fn read(bytes: &[u8]) -> HashProfile {
    profile::read(bytes).unwrap_or_else(|error| panic!("{error}"))
}

/// The single tile of the single cell of the single pattern.
fn only_tile(read: &HashProfile) -> Arc<ProfileTile> {
    let patterns: Vec<_> = read.document.profile.all_patterns().collect();
    assert_eq!(1, patterns.len());
    let cells = patterns[0].grid.as_ref().unwrap().cells();
    assert_eq!(1, cells.len());
    Arc::clone(cells[0].as_ref().unwrap())
}

// REQ-PROF-074: in Mega Man 2 (2.1) every tile has the fields {3, 5} and none has 1 or 2; 1 809
// variants carry field 25 (1 663 of value 1, 146 of value 2), 146 carry field 24, none field 19;
// the payload has Type = "3dn" and Ver = "2".
#[test]
#[ignore = "needs the built-in profiles: set ZAXNES_PROFILES to the folder that holds profiles/ and profiles-2.1/"]
fn mega_man_2_has_the_partition_of_req_prof_074() {
    let read = read(&neutral_assets::read_hash_profile("Mega Man 2.bytes"));
    let model = &read.document.profile;
    assert_eq!(Some("3dn"), model.signature.as_deref());
    assert_eq!(Some("2"), model.version_text.as_deref());

    for tile in model.distinct_tile_objects() {
        // The recorded set is kept only when it differs from the schema of a "3dh" tile, {3, 5}.
        assert_eq!(None, tile.recorded_fields);
        assert_eq!(
            Some(HashTileField::Identifier),
            tile.hash_identity.as_ref().map(|identity| identity.field)
        );
        assert!(tile.is_waiting());
    }

    let mut with_kind = 0;
    let mut kinds = (0, 0, 0);
    let mut with_field_24 = 0;
    let mut with_field_19 = 0;
    for variant in model.all_patterns().flat_map(|pattern| &pattern.variants) {
        match variant.texture_kind {
            None => {}
            Some(HashTextureKind::Absent) => {
                with_kind += 1;
                kinds.0 += 1;
            }
            Some(HashTextureKind::Default) => {
                with_kind += 1;
                kinds.1 += 1;
            }
            Some(HashTextureKind::Xor) => {
                with_kind += 1;
                kinds.2 += 1;
            }
        }
        with_field_24 += usize::from(variant.xor_texture.is_some());
        with_field_19 += usize::from(variant.texture.is_some());
    }

    assert_eq!(1809, with_kind);
    assert_eq!((0, 1663, 146), kinds);
    assert_eq!(146, with_field_24);
    assert_eq!(0, with_field_19);
}

// REQ-PROF-074, on the 101 files: 225 843 tiles, all of fields {3, 5}; 74 482 variants outside
// the memorised slots, 60 777 of textureKind 1 and 13 705 of textureKind 2 — 13 559 with a field
// 24 of format 0 and of size tSize x 8, 4 of format 0 and another size, 142 of format 1; the
// 1 010 variants of the memorised slots carry no field 25.
#[test]
#[ignore = "needs the built-in profiles: set ZAXNES_PROFILES to the folder that holds profiles/ and profiles-2.1/"]
fn the_101_payloads_have_the_measured_partition() {
    let mut tiles = 0;
    let mut variants = 0;
    let mut kind_default = 0;
    let mut kind_xor = 0;
    let mut xor_same_shape = 0;
    let mut xor_other_size = 0;
    let mut xor_direct = 0;
    let mut slot_variants = 0;

    for file in neutral_assets::hash_profile_files() {
        let bytes = std::fs::read(&file).unwrap();
        let name = file.file_name().unwrap().to_string_lossy().into_owned();
        let read = profile::read(&bytes).unwrap_or_else(|error| panic!("{name}: {error}"));
        let model = &read.document.profile;

        tiles += model.distinct_tile_objects().count();
        for tile in model.distinct_tile_objects() {
            assert_eq!(None, tile.recorded_fields, "{name}");
            assert_eq!(
                Some(HashTileField::Identifier),
                tile.hash_identity.as_ref().map(|identity| identity.field),
                "{name}"
            );
        }

        for pattern in model.all_patterns() {
            let size = pattern.tile_size.unwrap_or_default();
            for variant in &pattern.variants {
                variants += 1;
                assert!(variant.texture.is_none(), "{name}");
                match variant.texture_kind {
                    Some(HashTextureKind::Default) => kind_default += 1,
                    Some(HashTextureKind::Xor) => {
                        kind_xor += 1;
                        let stored = variant.xor_texture.as_ref().unwrap();
                        let format = stored.pixels.as_ref().unwrap().format;
                        if format == TextureFormat::Direct {
                            xor_direct += 1;
                        } else if stored.width == size.x * 8 && stored.height == size.y * 8 {
                            xor_same_shape += 1;
                        } else {
                            xor_other_size += 1;
                        }
                    }
                    other => panic!("{name}: a variant has textureKind {other:?}"),
                }
            }
        }

        if let Some(settings) = model.settings.as_ref() {
            for slot in settings
                .saved_patterns
                .iter()
                .chain(&settings.saved_anim_patterns)
            {
                for variant in slot.pattern.iter().flat_map(|pattern| &pattern.variants) {
                    slot_variants += 1;
                    assert_eq!(None, variant.texture_kind, "{name}");
                    assert!(variant.texture.is_none(), "{name}");
                }
            }
        }
    }

    assert_eq!(225_843, tiles);
    assert_eq!(74_482, variants);
    assert_eq!(60_777, kind_default);
    assert_eq!(13_705, kind_xor);
    assert_eq!(13_559, xor_same_shape);
    assert_eq!(4, xor_other_size);
    assert_eq!(142, xor_direct);
    assert_eq!(1_010, slot_variants);
}

// REQ-PROF-074 point 3: the root fields of the payload are 3, 4, 5, 6 first, in ascending order,
// then the others ascending — eleven combinations over the 101 files, empty.bytes giving
// 3, 6, 1, 2.
#[test]
#[ignore = "needs the built-in profiles: set ZAXNES_PROFILES to the folder that holds profiles/ and profiles-2.1/"]
fn the_root_field_order_of_the_101_payloads_puts_3_to_6_first() {
    let mut orders: BTreeSet<Vec<u32>> = BTreeSet::new();
    for file in neutral_assets::hash_profile_files() {
        let bytes = std::fs::read(&file).unwrap();
        let name = file.file_name().unwrap().to_string_lossy().into_owned();
        let read = profile::read(&bytes).unwrap_or_else(|error| panic!("{name}: {error}"));
        let payload = read.envelope.payload.as_deref().unwrap();

        // A repeated field carries one tag per element; the order of the field *numbers* is what
        // the requirement describes.
        let mut order = hash_envelope::field_order(payload).unwrap();
        order.dedup();

        let first: Vec<u32> = order
            .iter()
            .copied()
            .take_while(|field| *field >= 3)
            .collect();
        let rest: Vec<u32> = order.iter().copied().skip(first.len()).collect();
        assert!(first.windows(2).all(|pair| pair[0] < pair[1]), "{name}");
        assert!(first.iter().all(|field| (3..=6).contains(field)), "{name}");
        assert_eq!(Some(6), first.last().copied(), "{name}");
        assert!(rest.windows(2).all(|pair| pair[0] < pair[1]), "{name}");
        assert_eq!(Some(&1), rest.first(), "{name}");

        orders.insert(order);
    }

    assert_eq!(11, orders.len());
    assert!(orders.contains(&vec![3, 6, 1, 2]));
    assert!(orders.contains(&vec![3, 4, 5, 6, 1, 2, 7, 8, 9, 10]));

    let empty = read(&neutral_assets::read_hash_profile("empty.bytes"));
    let mut order = hash_envelope::field_order(empty.envelope.payload.as_deref().unwrap()).unwrap();
    order.dedup();
    assert_eq!(vec![3, 6, 1, 2], order);
}

// REQ-PROF-074: every string a tile names is a manifest string; with no ROM, every tile waits.
#[test]
#[ignore = "needs the built-in profiles: set ZAXNES_PROFILES to the folder that holds profiles/ and profiles-2.1/"]
fn every_built_in_payload_names_only_manifest_strings() {
    for file in neutral_assets::hash_profile_files() {
        let bytes = std::fs::read(&file).unwrap();
        let name = file.file_name().unwrap().to_string_lossy().into_owned();
        let read = profile::read(&bytes).unwrap_or_else(|error| panic!("{name}: {error}"));

        for tile in read.document.profile.distinct_tile_objects() {
            let key = tile.hash_key().unwrap_or_else(|| panic!("{name}"));
            assert!(read.manifest.contains(key), "{name}: {key}");
            assert!(tile.is_waiting(), "{name}: {key}");
        }

        let waiting = read.waiting_keys();
        assert!(waiting.len() <= read.manifest.len(), "{name}");
        assert!(
            waiting.iter().all(|key| read.manifest.contains(key)),
            "{name}"
        );
    }
}

#[test]
fn a_tile_named_by_its_identifier_waits_for_its_planes() {
    let id = TileId::compute(&raw(1));
    let payload = PayloadSpec::of(vec![PatternSpec::row(vec![Some(TileSpec::named(id))])]);
    let read = read(&hash_profile(Vec::new(), vec![id], &payload));

    let tile = only_tile(&read);
    assert!(tile.is_waiting());
    assert_eq!(Some(id.to_text().as_str()), tile.hash_key());
    assert_eq!(0, tile.low_plane);
    assert_eq!(0, tile.high_plane);
    assert_eq!(
        Some(HashTileField::Identifier),
        tile.hash_identity.as_ref().map(|identity| identity.field)
    );
}

// REQ-PROF-076: a tile may name its legacy digest, in field 4.
#[test]
fn a_tile_named_by_its_legacy_digest_waits_for_its_planes() {
    let digest = raw_tile::legacy_digest(&raw(2));
    let payload = PayloadSpec::of(vec![PatternSpec::row(vec![Some(TileSpec::text(&digest))])]);
    let read = read(&hash_profile(vec![digest.clone()], Vec::new(), &payload));

    let tile = only_tile(&read);
    assert!(tile.is_waiting());
    assert_eq!(Some(digest.as_str()), tile.hash_key());
    assert_eq!(
        Some(HashTileField::Text),
        tile.hash_identity.as_ref().map(|identity| identity.field)
    );
    // Field 4 instead of field 5: the recorded set differs from the schema and is kept.
    assert_eq!(
        Some(vec![3, 4]),
        tile.recorded_fields
            .map(zax_profiles::layout::FieldMask::fields)
    );
}

// REQ-PROF-074: a string the manifest does not hold is refused, the message naming it.
#[test]
fn a_tile_outside_the_manifest_is_refused() {
    let known = TileId::compute(&raw(3));
    let unknown = TileId::compute(&raw(4));
    let payload = PayloadSpec::of(vec![PatternSpec::row(vec![Some(TileSpec::named(unknown))])]);

    let error = profile::read(&hash_profile(Vec::new(), vec![known], &payload)).unwrap_err();
    assert_eq!(
        format!(
            "Tile hash absent from profile manifest: {}",
            unknown.to_text()
        ),
        error.message
    );
    assert_eq!(ProfileLoadErrorKind::InvalidHashPayload, error.kind);
}

#[test]
fn a_field_5_of_another_length_is_refused_naming_its_bytes() {
    let id = TileId::compute(&raw(5));
    let payload = PayloadSpec::of(vec![PatternSpec::row(vec![Some(TileSpec {
        name: TileName::RawIdentifier(vec![0xAB, 0xCD]),
        ..TileSpec::default()
    })])]);

    let error = profile::read(&hash_profile(Vec::new(), vec![id], &payload)).unwrap_err();
    assert_eq!(
        "Tile hash absent from profile manifest: abcd",
        error.message
    );
    assert_eq!(ProfileLoadErrorKind::InvalidHashPayload, error.kind);
}

// REQ-PROF-074 point 1: a tile that carries fields 1 and 2 anyway is an ordinary tile — it keeps
// the string it names, which the rewrite writes back, but it does not wait.
#[test]
fn a_tile_that_carries_its_planes_is_an_ordinary_tile() {
    let tile_raw = raw(6);
    let id = TileId::compute(&tile_raw);
    let (low, high) = raw_tile::to_planes(&tile_raw);
    let payload = PayloadSpec::of(vec![PatternSpec::row(vec![Some(
        TileSpec::named(id).with_stored_planes(
            zax_profiles::wire::cipher::encode(low),
            zax_profiles::wire::cipher::encode(high),
        ),
    )])]);

    let read = read(&hash_profile(Vec::new(), vec![id], &payload));
    let tile = only_tile(&read);
    assert!(!tile.is_waiting());
    assert_eq!(low, tile.low_plane);
    assert_eq!(high, tile.high_plane);
    assert_eq!(Some(id.to_text().as_str()), tile.hash_key());
    assert_eq!(0, read.waiting_tile_count());
}

// REQ-PROF-074 point 2: any textureKind but 0, 1 and 2 is refused.
#[test]
fn an_unusable_texture_kind_is_refused() {
    for kind in [3, 4, -1, 100] {
        let payload = PayloadSpec::of(vec![
            PatternSpec::row(vec![None]).with_variants(vec![VariantSpec::of_kind(kind)])
        ]);
        let error = profile::read(&hash_profile(Vec::new(), Vec::new(), &payload)).unwrap_err();
        assert_eq!("Invalid profile XOR texture", error.message, "kind {kind}");
        assert_eq!(ProfileLoadErrorKind::InvalidHashPayload, error.kind);
    }
}

#[test]
fn a_texture_kind_of_2_without_field_24_is_refused() {
    let payload = PayloadSpec::of(vec![
        PatternSpec::row(vec![None]).with_variants(vec![VariantSpec::of_kind(2)])
    ]);

    let error = profile::read(&hash_profile(Vec::new(), Vec::new(), &payload)).unwrap_err();
    assert_eq!("Invalid profile XOR texture", error.message);
    assert_eq!(ProfileLoadErrorKind::InvalidHashPayload, error.kind);
}

#[test]
fn the_three_texture_kinds_are_read() {
    let payload = PayloadSpec::of(vec![PatternSpec::row(vec![None]).with_variants(vec![
        VariantSpec::of_kind(0),
        VariantSpec::of_kind(1),
        VariantSpec::of_kind(2).with_xor_texture(TextureSpec::indexed(8, 8, vec![1, 2, 3, 4])),
    ])]);

    let read = read(&hash_profile(Vec::new(), Vec::new(), &payload));
    let patterns: Vec<_> = read.document.profile.all_patterns().collect();
    let variants = &patterns[0].variants;
    assert_eq!(
        vec![
            Some(HashTextureKind::Absent),
            Some(HashTextureKind::Default),
            Some(HashTextureKind::Xor),
        ],
        variants
            .iter()
            .map(|variant| variant.texture_kind)
            .collect::<Vec<_>>()
    );
    assert!(variants[0].xor_texture.is_none());
    assert!(variants[1].xor_texture.is_none());
    let stored = variants[2].xor_texture.as_ref().unwrap();
    assert_eq!((8, 8), (stored.width, stored.height));
    assert_eq!(vec![1, 2, 3, 4], stored.pixels.as_ref().unwrap().data);
}

// AMD-062 §3: every comparison is made on the manifest strings, so one string is interned once
// and the tiles that name it share it.
#[test]
fn the_tiles_of_a_payload_share_one_string_per_manifest_entry() {
    let id = TileId::compute(&raw(7));
    let payload = PayloadSpec::of(vec![PatternSpec::row(vec![
        Some(TileSpec::named(id)),
        Some(TileSpec::named(id).with_flip(1)),
    ])]);

    let read = read(&hash_profile(Vec::new(), vec![id], &payload));
    let patterns: Vec<_> = read.document.profile.all_patterns().collect();
    let cells = patterns[0].grid.as_ref().unwrap().cells();
    let left = cells[0].as_ref().unwrap().hash_identity.as_ref().unwrap();
    let right = cells[1].as_ref().unwrap().hash_identity.as_ref().unwrap();
    assert!(Arc::ptr_eq(&left.key, &right.key));
}

// The 3dn reader is untouched: fields 4, 5, 24 and 25 are unknown fields there, and nothing of
// the "3dh" model is set.
#[test]
#[ignore = "needs the built-in profiles: set ZAXNES_PROFILES to the folder that holds profiles/ and profiles-2.1/"]
fn a_3dn_profile_carries_no_hash_identity() {
    let document = neutral_assets::load_profile("Super Mario Bros.bytes");
    for tile in document.profile.distinct_tile_objects() {
        assert_eq!(None, tile.hash_identity);
        assert!(!tile.is_waiting());
    }
    for variant in document
        .profile
        .all_patterns()
        .flat_map(|pattern| &pattern.variants)
    {
        assert_eq!(None, variant.texture_kind);
        assert!(variant.xor_texture.is_none());
    }
}

// REQ-PROF-081: a waiting tile is equal to no tile whose planes are known, and to another
// waiting tile only when both name the same manifest string and share the same flip.
#[test]
fn a_waiting_tile_is_equal_to_no_tile_of_the_screen() {
    let key: Arc<str> = Arc::from(TileId::compute(&raw(8)).to_text().as_str());
    let waiting = ProfileTile::waiting(Arc::clone(&key), HashTileField::Identifier, TileFlip::NONE);
    let blank = ProfileTile::new(0, 0, TileFlip::NONE);
    let (low, high) = raw_tile::to_planes(&raw(8));
    let screen = ProfileTile::new(low, high, TileFlip::NONE);

    assert_ne!(waiting, blank);
    assert_ne!(waiting, screen);
    assert_ne!(blank, waiting);
}

#[test]
fn two_waiting_tiles_are_equal_on_the_same_string_and_flip() {
    let key: Arc<str> = Arc::from(TileId::compute(&raw(9)).to_text().as_str());
    let other: Arc<str> = Arc::from(raw_tile::legacy_digest(&raw(9)).as_str());
    let left = ProfileTile::waiting(Arc::clone(&key), HashTileField::Identifier, TileFlip::NONE);
    let same = ProfileTile::waiting(Arc::from(&*key), HashTileField::Text, TileFlip::NONE);
    let flipped = ProfileTile::waiting(
        Arc::clone(&key),
        HashTileField::Identifier,
        TileFlip::HORIZONTAL,
    );
    let by_digest = ProfileTile::waiting(other, HashTileField::Text, TileFlip::NONE);

    // The string decides, not the field that carried it nor the `Arc`.
    assert_eq!(left, same);
    assert_ne!(left, flipped);
    assert_ne!(left, by_digest);
}

// REQ-PROF-081: a tile named by its legacy digest and the same tile named by its identifier are
// distinct while they wait, and equal once resolved — their planes are then the same.
#[test]
fn a_digest_and_an_identifier_of_one_tile_meet_once_resolved() {
    let tile_raw = raw(10);
    let (low, high) = raw_tile::to_planes(&tile_raw);
    let by_id = ProfileTile::waiting(
        Arc::from(TileId::compute(&tile_raw).to_text().as_str()),
        HashTileField::Identifier,
        TileFlip::NONE,
    );
    let by_digest = ProfileTile::waiting(
        Arc::from(raw_tile::legacy_digest(&tile_raw).as_str()),
        HashTileField::Text,
        TileFlip::NONE,
    );

    assert_ne!(by_id, by_digest);

    let resolved_id = by_id.with_planes(low, high);
    let resolved_digest = by_digest.with_planes(low, high);
    assert_eq!(resolved_id, resolved_digest);
    assert!(!resolved_id.is_waiting());
    assert_eq!(ProfileTile::new(low, high, TileFlip::NONE), resolved_id);
    // The identity is kept, so the rewrite still knows which string to write (REQ-PROF-084).
    assert_eq!(
        Some(TileId::compute(&tile_raw).to_text().as_str()),
        resolved_id.hash_key()
    );
}
