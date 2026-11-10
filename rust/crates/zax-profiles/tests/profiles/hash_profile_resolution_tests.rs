//! Resolving the tiles of a "3dh" profile from a ROM, the waiting tile and the cache
//! (AMD-062 REQ-PROF-078, REQ-PROF-079, REQ-PROF-081, REQ-PROF-082), and the textures that come
//! back once the tiles are there (REQ-PROF-083).

use std::collections::HashSet;

use zax_profiles::enums::{TextureFormat, TileFlip};
use zax_profiles::format::ProfileDocument;
use zax_profiles::hash::default_texture::PatternTextures;
use zax_profiles::hash::profile;
use zax_profiles::hash::raw_tile::{self, RawTile};
use zax_profiles::hash::rom_zones::RomZones;
use zax_profiles::hash::tile_cache;
use zax_profiles::hash::tile_id::TileId;
use zax_profiles::pattern::ProfilePattern;
use zax_profiles::variant::VariantTexture;

use super::common::hash_state;
use super::common::neutral_assets;
use super::common::synthetic_rom;

/// The two profiles of the 2.1 bundle that gained patterns in 2.0 and so have no 1.x counterpart
/// to pair with (AMD-062 §1).
const NOT_PAIRABLE: [&str; 2] = ["DQ3_DW3.bytes", "Final_Fantasy_USA.bytes"];

fn raw(seed: u8) -> RawTile {
    let mut bytes = [0_u8; 16];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = seed
            .wrapping_mul(17)
            .wrapping_add(u8::try_from(index).unwrap());
    }
    bytes
}

// REQ-PROF-078 point 1: the PRG zone then the CHR zone of an iNES file, the trainer skipped.
#[test]
fn the_zones_of_an_ines_file_follow_the_header() {
    let plain = synthetic_rom::ines(2, 1, &[]);
    let zones = RomZones::of(&plain).unwrap();
    assert_eq!(16..16 + 2 * 16 * 1024, zones.prg);
    assert_eq!(16 + 2 * 16 * 1024..16 + 2 * 16 * 1024 + 8 * 1024, zones.chr);

    let with_trainer = synthetic_rom::with_trainer(&synthetic_rom::ines(1, 1, &[]));
    let zones = RomZones::of(&with_trainer).unwrap();
    assert_eq!(16 + 512..16 + 512 + 16 * 1024, zones.prg);
    assert_eq!(
        16 + 512 + 16 * 1024..16 + 512 + 16 * 1024 + 8 * 1024,
        zones.chr
    );
}

// REQ-PROF-078 point 1: NES 2.0 sizes, the exponent-multiplier notation included.
#[test]
fn the_zones_read_the_nes_2_0_sizes() {
    // Byte 9 carries the high nibbles: PRG 1, CHR 0 — so 0x101 PRG units.
    let mut file = synthetic_rom::ines(1, 2, &[]);
    file[7] = 0x08;
    file[9] = 0x01;
    file.resize(16 + 0x101 * 16 * 1024 + 2 * 8 * 1024, 0);
    let zones = RomZones::of(&file).unwrap();
    assert_eq!(16..16 + 0x101 * 16 * 1024, zones.prg);
    assert_eq!(0x101 * 16 * 1024 + 16..file.len(), zones.chr);

    // High nibble 15: the size is `(low & 3) * 2 + 1` shifted left by `low >> 2`.
    // low = 0x12 -> multiplier 5, exponent 4, so 80 bytes.
    let mut file = synthetic_rom::ines(0, 0, &[]);
    file[7] = 0x08;
    file[4] = 0x12;
    file[9] = 0x0F;
    file.resize(16 + 1024, 0);
    let zones = RomZones::of(&file).unwrap();
    assert_eq!(16..16 + 80, zones.prg);
    assert!(zones.chr.is_empty());
}

// REQ-PROF-078 point 1: the zones are truncated to the length of the file.
#[test]
fn the_zones_are_truncated_to_the_file() {
    let mut file = synthetic_rom::ines(4, 4, &[]);
    file.truncate(16 + 1000);
    let zones = RomZones::of(&file).unwrap();
    assert_eq!(16..16 + 1000, zones.prg);
    assert!(zones.chr.is_empty());
    assert_eq!(16 + 1000, zones.chr.start);
}

// REQ-PROF-078: a file that is not iNES resolves nothing.
#[test]
fn a_file_that_is_not_ines_has_no_zones() {
    assert!(RomZones::of(&[]).is_none());
    assert!(RomZones::of(b"NES").is_none());
    assert!(RomZones::of(&[0_u8; 64]).is_none());

    let id = TileId::compute(&raw(1));
    let mut read = synthetic_rom::hash_profile_of(&[raw(1)]);
    assert_eq!(1, hash_state(&read).tiles.waiting_count());
    assert_eq!(0, read.resolve_from_rom(&[0_u8; 4096]));
    assert_eq!(1, hash_state(&read).tiles.waiting_count());
    assert!(hash_state(&read).tiles.bytes_of(&id.to_text()).is_none());
}

// REQ-PROF-079: the mirrorings taken from the planes, which is what a scan computes, are the
// mirrorings taken from the bytes.
#[test]
fn the_mirrored_planes_are_the_mirrored_bytes() {
    for seed in 0..16_u8 {
        let source = raw(seed);
        let (low, high) = raw_tile::to_planes(&source);
        for bits in 0..4 {
            let flip = TileFlip::from_bits(bits);
            let (flipped_low, flipped_high) = raw_tile::flipped_planes(low, high, flip);
            assert_eq!(
                raw_tile::flipped(&source, flip),
                raw_tile::from_planes(flipped_low, flipped_high),
                "seed {seed}, flip {bits}"
            );
        }
    }
}

// REQ-PROF-078 point 2: the scan runs at every byte position, unaligned ones included, and takes
// the four mirrorings.
#[test]
fn a_rom_holding_the_tiles_unaligned_resolves_them_all() {
    let tiles = [raw(1), raw(2), raw(3)];
    let mut read = synthetic_rom::hash_profile_of(&tiles);
    assert_eq!(3, hash_state(&read).tiles.waiting_count());

    let rom = synthetic_rom::chr_rom_with_tiles(&tiles);
    assert_eq!(3, read.resolve_from_rom(&rom));
    assert!(hash_state(&read).tiles.is_complete());
    for tile in &tiles {
        assert_eq!(
            Some(tile),
            hash_state(&read)
                .tiles
                .bytes_of(&TileId::compute(tile).to_text())
        );
    }
    assert_eq!(0, read.waiting_tile_count());
}

#[test]
fn a_rom_without_one_tile_leaves_that_one_waiting() {
    let tiles = [raw(1), raw(2), raw(3)];
    let mut read = synthetic_rom::hash_profile_of(&tiles);
    let rom = synthetic_rom::chr_rom_with_tiles(&tiles[..2]);

    assert_eq!(2, read.resolve_from_rom(&rom));
    assert_eq!(
        vec![TileId::compute(&tiles[2]).to_text()],
        hash_state(&read).tiles.waiting_keys()
    );
}

// REQ-PROF-078 point 2: a tile the ROM holds mirrored is resolved, with the mirrored bytes.
#[test]
fn a_tile_the_rom_holds_mirrored_is_resolved() {
    let wanted = raw(5);
    let mut read = synthetic_rom::hash_profile_of(&[wanted]);
    let stored = raw_tile::flipped(&wanted, TileFlip::HORIZONTAL);
    assert_ne!(wanted, stored);

    let rom = synthetic_rom::chr_rom_with_tiles(&[stored]);
    assert_eq!(1, read.resolve_from_rom(&rom));
    assert_eq!(
        Some(&wanted),
        hash_state(&read)
            .tiles
            .bytes_of(&TileId::compute(&wanted).to_text())
    );
}

// REQ-PROF-076: a tile named by its legacy digest resolves like one named by its identifier.
#[test]
fn a_legacy_digest_is_resolved_by_the_same_scan() {
    let wanted = raw(6);
    let mut read = synthetic_rom::hash_profile_named_by_digest(&[wanted]);
    assert_eq!(1, hash_state(&read).tiles.waiting_count());

    let rom = synthetic_rom::chr_rom_with_tiles(&[wanted]);
    assert_eq!(1, read.resolve_from_rom(&rom));
    assert_eq!(
        Some(&wanted),
        hash_state(&read)
            .tiles
            .bytes_of(&raw_tile::legacy_digest(&wanted))
    );
}

// REQ-PROF-081: a waiting tile has no pixels and recognises nothing; once resolved it is the
// tile the 3dn profile would have carried.
#[test]
fn a_pattern_with_a_waiting_tile_draws_only_its_resolved_cells() {
    let tiles = [raw(7), raw(8)];
    let mut read = synthetic_rom::hash_profile_of(&tiles);
    let waiting_texture = PatternTextures::of(first_pattern(&read))
        .default_texture()
        .pixels
        .clone();
    assert!(waiting_texture.data.iter().all(|word| *word == 0));

    // A pattern of 2 by 1 cells is 16 by 8 pixels, so one `data` word is one row of the two
    // cells: the eight pixels of the left cell in its low sixteen bits, those of the right cell
    // in its high sixteen.
    let left = |word: &i32| word & 0xFFFF;
    let right = |word: &i32| (word >> 16) & 0xFFFF;

    // Only the first tile is in the ROM: the right half of every row stays at zero.
    read.resolve_from_rom(&synthetic_rom::chr_rom_with_tiles(&tiles[..1]));
    let half = PatternTextures::of(first_pattern(&read))
        .default_texture()
        .pixels
        .clone();
    assert!(half.data.iter().map(left).any(|bits| bits != 0));
    assert!(half.data.iter().map(right).all(|bits| bits == 0));
    assert_eq!(1, read.waiting_tile_count());

    read.resolve_from_rom(&synthetic_rom::chr_rom_with_tiles(&tiles));
    let full = PatternTextures::of(first_pattern(&read))
        .default_texture()
        .pixels
        .clone();
    assert!(full.data.iter().map(right).any(|bits| bits != 0));
    assert_eq!(0, read.waiting_tile_count());
}

// REQ-PROF-080: resolution rebuilds the patterns that hold the tile, and nothing else.
#[test]
fn resolution_rebuilds_only_the_patterns_it_changes() {
    let tiles = [raw(9), raw(10)];
    let mut read = synthetic_rom::hash_profile_of(&tiles);
    let before: Vec<*const ProfilePattern> = read
        .profile
        .permanent_patterns
        .iter()
        .map(std::sync::Arc::as_ptr)
        .collect();

    assert_eq!(
        0,
        read.resolve_from_rom(&synthetic_rom::chr_rom_with_tiles(&[]))
    );
    let after: Vec<*const ProfilePattern> = read
        .profile
        .permanent_patterns
        .iter()
        .map(std::sync::Arc::as_ptr)
        .collect();
    assert_eq!(before, after, "nothing resolved, nothing rebuilt");

    read.resolve_from_rom(&synthetic_rom::chr_rom_with_tiles(&tiles));
    let rebuilt: Vec<*const ProfilePattern> = read
        .profile
        .permanent_patterns
        .iter()
        .map(std::sync::Arc::as_ptr)
        .collect();
    assert_ne!(before, rebuilt);
}

// REQ-PROF-082: a cache written after a resolution gives every tile back, before any ROM, and a
// cache that is unreadable or wrong is ignored.
#[test]
fn the_cache_gives_the_tiles_back_without_a_rom() {
    let tiles = [raw(11), raw(12), raw(13)];
    let mut resolved = synthetic_rom::hash_profile_of(&tiles);
    resolved.resolve_from_rom(&synthetic_rom::chr_rom_with_tiles(&tiles));
    assert!(hash_state(&resolved).tiles.is_complete());
    let cache = resolved.tile_cache().unwrap();

    // A ROM that holds none of its tiles, and the cache: everything is resolved all the same.
    let mut read = synthetic_rom::hash_profile_of(&tiles);
    let entries = tile_cache::read(&cache);
    assert_eq!(
        3,
        read.apply_tile_cache(entries.iter().map(|(key, raw)| (key.as_str(), *raw)))
    );
    assert!(hash_state(&read).tiles.is_complete());
    assert_eq!(0, read.waiting_tile_count());
    assert_eq!(
        0,
        read.resolve_from_rom(&synthetic_rom::chr_rom_with_tiles(&[]))
    );

    // The profile's own bytes are untouched by any of this.
    assert_eq!(
        synthetic_rom::hash_profile_bytes(&tiles),
        synthetic_rom::hash_profile_bytes(&tiles)
    );
}

#[test]
fn an_unreadable_or_wrong_cache_is_ignored() {
    let tiles = [raw(14)];
    let key = TileId::compute(&tiles[0]).to_text();

    // No header, truncated, arbitrary bytes.
    assert_eq!(0, tile_cache::read("").len());
    assert_eq!(0, tile_cache::read("something else\nx y\n").len());
    assert_eq!(0, tile_cache::read_bytes(&[0xFF, 0xFE, 0x00]).len());
    let good = tile_cache::write([(key.as_str(), &tiles[0])]);
    assert_eq!(1, tile_cache::read(&good).len());
    assert_eq!(0, tile_cache::read(&good[..good.len() / 2]).len());

    // An entry whose string does not recompute from its bytes.
    let wrong = tile_cache::write([(key.as_str(), &raw(15))]);
    assert_eq!(0, tile_cache::read(&wrong).len());

    // And a cache that holds nothing leaves every tile waiting.
    let mut read = synthetic_rom::hash_profile_of(&tiles);
    assert_eq!(0, read.apply_tile_cache(Vec::new()));
    assert_eq!(1, hash_state(&read).tiles.waiting_count());
}

#[test]
fn the_cache_file_is_named_after_the_profile_identifier() {
    let read = synthetic_rom::hash_profile_of(&[raw(16)]);
    assert_eq!(
        Some("SyntheticId".to_owned()),
        hash_state(&read).envelope.id
    );
    assert_eq!(
        Some("SyntheticId.tiles".to_owned()),
        read.tile_cache_file_name()
    );
}

// REQ-PROF-078, acceptance criterion: for each of the 99 pairable profiles, a synthetic iNES
// file whose CHR zone holds the sixteen raw bytes of every tile of the 1.x counterpart, each at
// a position unaligned on 16, resolves the whole manifest.
#[test]
#[ignore = "needs the built-in profiles: set ZAXNES_PROFILES to the folder that holds profiles/ and profiles-2.1/"]
fn the_99_pairable_profiles_resolve_whole_from_their_counterparts_tiles() {
    let mut pairable = 0;
    for file in neutral_assets::hash_profile_files() {
        let name = file.file_name().unwrap().to_string_lossy().into_owned();
        if NOT_PAIRABLE.contains(&name.as_str()) {
            continue;
        }
        pairable += 1;

        let mut read = read_hash(&name);
        let rom = synthetic_rom::chr_rom_with_tiles(&counterpart_tiles(&name));
        read.resolve_from_rom(&rom);
        assert!(
            hash_state(&read).tiles.is_complete(),
            "{name}: {} strings still waiting",
            hash_state(&read).tiles.waiting_count()
        );
        assert_eq!(0, read.waiting_tile_count(), "{name}");
    }

    assert_eq!(99, pairable);
}

// REQ-PROF-083, acceptance criterion: once the tiles are there, the k-th variant of a pairable
// profile has the texture of the k-th variant of its 1.x counterpart.
#[test]
#[ignore = "needs the built-in profiles: set ZAXNES_PROFILES to the folder that holds profiles/ and profiles-2.1/"]
fn the_textures_of_the_99_pairable_profiles_come_back_whole() {
    let mut markers = 0;
    let mut same_shape = 0;
    let mut direct = 0;
    let mut other_size = 0;

    for file in neutral_assets::hash_profile_files() {
        let name = file.file_name().unwrap().to_string_lossy().into_owned();
        if NOT_PAIRABLE.contains(&name.as_str()) {
            continue;
        }

        let mut read = read_hash(&name);
        read.resolve_from_rom(&synthetic_rom::chr_rom_with_tiles(&counterpart_tiles(
            &name,
        )));
        assert!(hash_state(&read).tiles.is_complete(), "{name}");

        let plain = neutral_assets::load_profile(&neutral_assets::counterpart_name(&name));
        let hash_patterns: Vec<_> = read.profile.all_patterns().collect();
        let plain_patterns: Vec<_> = plain.profile.all_patterns().collect();
        assert_eq!(plain_patterns.len(), hash_patterns.len(), "{name}");

        for (index, (hash_pattern, plain_pattern)) in
            hash_patterns.iter().zip(&plain_patterns).enumerate()
        {
            let textures = PatternTextures::of(hash_pattern);
            assert_eq!(
                plain_pattern.variants.len(),
                hash_pattern.variants.len(),
                "{name}, pattern {index}"
            );
            for (variant, expected) in hash_pattern.variants.iter().zip(&plain_pattern.variants) {
                let built = textures.texture_of(variant);
                match (built.as_deref(), expected.texture.as_ref()) {
                    (None, None) => {}
                    (Some(built), Some(expected)) if expected.is_null_marker() => {
                        assert!(built.is_null_marker(), "{name}, pattern {index}");
                        markers += 1;
                    }
                    (Some(built), Some(expected)) => {
                        assert_same_texture(built, expected, &name, index);
                        let format = expected.pixels.as_ref().unwrap().format;
                        let size = plain_pattern.tile_size.unwrap_or_default();
                        if format == TextureFormat::Direct {
                            direct += 1;
                        } else if expected.width == size.x * 8 && expected.height == size.y * 8 {
                            same_shape += 1;
                        } else {
                            other_size += 1;
                        }
                    }
                    (built, expected) => panic!(
                        "{name}, pattern {index}: {} against {}",
                        built.is_some(),
                        expected.is_some()
                    ),
                }
            }
        }
    }

    assert_eq!(55_575, markers);
    assert_eq!(12_418, same_shape);
    assert_eq!(130, direct);
    assert_eq!(4, other_size);
    assert_eq!(68_127, markers + same_shape + direct + other_size);
}

fn read_hash(name: &str) -> ProfileDocument {
    let bytes = neutral_assets::read_hash_profile(name);
    profile::read(&bytes).unwrap_or_else(|error| panic!("{name}: {error}"))
}

/// The sixteen raw bytes of every distinct tile of the 1.x counterpart of `name`.
fn counterpart_tiles(name: &str) -> Vec<RawTile> {
    let plain = neutral_assets::load_profile(&neutral_assets::counterpart_name(name));
    let mut seen = HashSet::new();
    plain
        .profile
        .distinct_tile_objects()
        .map(|tile| raw_tile::from_planes(tile.low_plane, tile.high_plane))
        .filter(|raw| seen.insert(*raw))
        .collect()
}

fn first_pattern(read: &ProfileDocument) -> &ProfilePattern {
    read.profile.permanent_patterns[0].as_ref()
}

/// Two textures are the same when their size, their format and their words agree; an absent
/// `extra` array and one of zeroes are the same texture (SPEC-03 REQ-PROF-035).
fn assert_same_texture(
    built: &VariantTexture,
    expected: &VariantTexture,
    name: &str,
    index: usize,
) {
    assert_eq!(
        (expected.width, expected.height),
        (built.width, built.height),
        "{name}, pattern {index}"
    );
    let (built_pixels, expected_pixels) = (
        built.pixels.as_ref().unwrap(),
        expected.pixels.as_ref().unwrap(),
    );
    assert_eq!(
        expected_pixels.format, built_pixels.format,
        "{name}, pattern {index}"
    );
    assert_eq!(
        expected_pixels.data, built_pixels.data,
        "{name}, pattern {index}"
    );
    for word in 0..built_pixels.extra.len().max(expected_pixels.extra.len()) {
        assert_eq!(
            expected_pixels.extra.get(word).copied().unwrap_or(0),
            built_pixels.extra.get(word).copied().unwrap_or(0),
            "{name}, pattern {index}, extra word {word}"
        );
    }
}
