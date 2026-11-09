//! The textures of a "3dh" variant (AMD-062 REQ-PROF-083): the default texture of a pattern in
//! the packed form the exclusive-or works against, and what each `textureKind` gives.

use std::sync::Arc;

use zax_profiles::enums::{TextureFormat, TileFlip};
use zax_profiles::hash::default_texture::{self, DefaultTexture, PatternTextures};
use zax_profiles::hash::raw_tile;
use zax_profiles::hash::tile_id::TileId;
use zax_profiles::pattern::ProfilePattern;
use zax_profiles::tile::{HashTileField, PatternGrid, ProfileTile};
use zax_profiles::values::Vec2I;
use zax_profiles::variant::{HashTextureKind, PatternVariant, TexturePixels, VariantTexture};

/// A tile whose every pixel has the colour `color`, 0 to 3.
fn solid(color: u64) -> Arc<ProfileTile> {
    Arc::new(ProfileTile::new(
        if color & 1 == 1 { u64::MAX } else { 0 },
        if color & 2 == 2 { u64::MAX } else { 0 },
        TileFlip::NONE,
    ))
}

fn waiting(seed: u8) -> Arc<ProfileTile> {
    let mut bytes = [0_u8; 16];
    bytes[0] = seed;
    Arc::new(ProfileTile::waiting(
        Arc::from(TileId::compute(&bytes).to_text().as_str()),
        HashTileField::Identifier,
        TileFlip::NONE,
    ))
}

/// A pattern of `cells.len()` by 1 cells with one variant.
fn pattern(cells: Vec<Option<Arc<ProfileTile>>>, variant: PatternVariant) -> ProfilePattern {
    let width = i32::try_from(cells.len()).unwrap();
    ProfilePattern {
        grid: Some(PatternGrid::new(width, 1, cells)),
        tile_size: Some(Vec2I::new(width, 1)),
        variants: vec![Arc::new(variant)],
        ..ProfilePattern::default()
    }
}

/// Reads texel `n` of an indexed payload (SPEC-03 REQ-PROF-035).
fn texel(pixels: &TexturePixels, n: usize) -> i32 {
    let low = pixels
        .data
        .get(n / 16)
        .map_or(0, |word| (word >> (2 * (n % 16))) & 3);
    let high = pixels
        .extra
        .get(n / 16)
        .map_or(0, |word| (word >> (n % 16)) & 1);
    low | (high << 2)
}

// REQ-PROF-083: the default texture is tSize x 8 pixels, of format 0, with `extra` as long as
// `data` — the length the exclusive-or relies on — and all its texels are the colour indices of
// the tiles.
#[test]
fn the_default_texture_has_the_shape_the_exclusive_or_needs() {
    let built = DefaultTexture::of(&pattern(
        vec![Some(solid(1)), Some(solid(2))],
        PatternVariant::default(),
    ));

    assert_eq!((16, 8), (built.width, built.height));
    assert_eq!(TextureFormat::Indexed, built.pixels.format);
    assert_eq!(16 * 8 / 16, built.pixels.data.len());
    assert_eq!(built.pixels.data.len(), built.pixels.extra.len());
    assert!(built.pixels.extra.iter().all(|word| *word == 0));

    // Row 0: eight texels of colour 1 then eight of colour 2.
    for x in 0..8 {
        assert_eq!(1, texel(&built.pixels, x), "x = {x}");
    }
    for x in 8..16 {
        assert_eq!(2, texel(&built.pixels, x), "x = {x}");
    }
    // Last row, same thing.
    for x in 0..8 {
        assert_eq!(1, texel(&built.pixels, 7 * 16 + x), "x = {x}");
    }
}

// REQ-PROF-081: a tile still waiting leaves its 8 by 8 pixels at 0, as does an empty cell.
#[test]
fn a_waiting_tile_leaves_its_pixels_at_zero_in_the_default_texture() {
    let built = DefaultTexture::of(&pattern(
        vec![Some(solid(3)), Some(waiting(1)), None],
        PatternVariant::default(),
    ));

    assert_eq!((24, 8), (built.width, built.height));
    for x in 0..8 {
        assert_eq!(3, texel(&built.pixels, x), "x = {x}");
    }
    for x in 8..24 {
        assert_eq!(0, texel(&built.pixels, x), "x = {x}");
    }
}

#[test]
fn a_pattern_without_a_grid_has_an_empty_default_texture() {
    let built = DefaultTexture::of(&ProfilePattern::default());
    assert_eq!((0, 0), (built.width, built.height));
    assert_eq!(0, built.pixels.data.len());
    assert_eq!(0, built.pixels.extra.len());
}

// REQ-PROF-083: the exclusive-or is its own inverse, which is what makes the stored texture of a
// variant come back exactly.
#[test]
fn the_exclusive_or_undoes_itself() {
    let base = DefaultTexture::of(&pattern(
        vec![Some(solid(1)), Some(solid(3))],
        PatternVariant::default(),
    ));
    let wanted = VariantTexture::new(
        16,
        8,
        Some(TexturePixels::new(
            TextureFormat::Indexed,
            (0..8).map(|word| 0x1234_5678 ^ word).collect(),
            (0..8).map(|word| word - 4).collect(),
        )),
    );

    let stored = default_texture::exclusive_or(&wanted, &base);
    let back = default_texture::exclusive_or(&stored, &base);
    let (wanted_pixels, back_pixels) = (
        wanted.pixels.as_ref().unwrap(),
        back.pixels.as_ref().unwrap(),
    );
    assert_eq!((wanted.width, wanted.height), (back.width, back.height));
    assert_eq!(wanted_pixels.format, back_pixels.format);
    assert_eq!(wanted_pixels.data, back_pixels.data);
    assert_eq!(wanted_pixels.extra, back_pixels.extra);
}

// REQ-PROF-083: the result has the dimensions and the format of field 24, and the exclusive-or
// is word against word whatever the formats and the sizes.
#[test]
fn the_exclusive_or_keeps_the_shape_of_the_stored_texture() {
    let base = DefaultTexture::of(&pattern(
        vec![Some(solid(3)), Some(solid(3))],
        PatternVariant::default(),
    ));
    assert_eq!(8, base.pixels.data.len());
    assert!(base.pixels.data.iter().all(|word| *word == -1));

    // Format 1 and four words only: four words of the default texture are used and no more.
    let stored = VariantTexture::new(
        2,
        2,
        Some(TexturePixels::new(
            TextureFormat::Direct,
            vec![0, 1, 2, 3],
            Vec::new(),
        )),
    );
    let built = default_texture::exclusive_or(&stored, &base);

    assert_eq!((2, 2), (built.width, built.height));
    let pixels = built.pixels.as_ref().unwrap();
    assert_eq!(TextureFormat::Direct, pixels.format);
    assert_eq!(vec![-1, -2, -3, -4], pixels.data);
    // `extra` is as long as `data`; a missing element of field 24 counts as zero.
    assert_eq!(4, pixels.extra.len());
    assert!(pixels.extra.iter().all(|word| *word == 0));
}

// REQ-PROF-083: `extra` is exclusive-ored on 16 bits, and only where the default texture has a
// `data` word.
#[test]
fn the_extra_words_are_exclusive_ored_on_sixteen_bits() {
    let base = DefaultTexture::of(&pattern(vec![Some(solid(1))], PatternVariant::default()));
    assert_eq!(4, base.pixels.data.len());

    let stored = VariantTexture::new(
        8,
        8,
        Some(TexturePixels::new(
            TextureFormat::Indexed,
            vec![0; 6],
            vec![-1, 0x7FFF, -32_768, 1, 2, 3],
        )),
    );
    let built = default_texture::exclusive_or(&stored, &base);

    // The default texture's `extra` is zero, so each word comes back as the signed 16-bit value
    // it was, including the ones past its end.
    assert_eq!(
        vec![-1, 0x7FFF, -32_768, 1, 2, 3],
        built.pixels.as_ref().unwrap().extra
    );
}

#[test]
fn a_variant_of_kind_absent_has_no_texture() {
    let variant = PatternVariant {
        texture_kind: Some(HashTextureKind::Absent),
        ..PatternVariant::default()
    };
    let shape = pattern(vec![Some(solid(1))], variant.clone());
    assert!(PatternTextures::of(&shape).texture_of(&variant).is_none());
}

// REQ-PROF-083: `textureKind = 1` is the pattern's default texture, which SPEC-03 REQ-PROF-035
// writes as the null marker.
#[test]
fn a_variant_of_kind_default_uses_the_null_marker() {
    let variant = PatternVariant {
        texture_kind: Some(HashTextureKind::Default),
        ..PatternVariant::default()
    };
    let shape = pattern(vec![Some(solid(1))], variant.clone());
    let texture = PatternTextures::of(&shape).texture_of(&variant).unwrap();
    assert!(texture.is_null_marker());
}

// REQ-PROF-083: field 24 of the shape of the default texture is rebuilt even while a tile waits
// — that is where the pattern's own pixels have to come back as the tiles arrive.
#[test]
fn a_variant_of_kind_xor_of_the_default_shape_is_rebuilt_while_a_tile_waits() {
    let wanted: Vec<i32> = (0..8).map(|word| 0x0F0F_0F0F ^ word).collect();
    let resolved_pattern = pattern(
        vec![Some(solid(1)), Some(solid(2))],
        PatternVariant::default(),
    );
    let base = DefaultTexture::of(&resolved_pattern);
    let stored = default_texture::exclusive_or(
        &VariantTexture::new(
            16,
            8,
            Some(TexturePixels::new(
                TextureFormat::Indexed,
                wanted.clone(),
                vec![0; 8],
            )),
        ),
        &base,
    );

    let variant = PatternVariant {
        texture_kind: Some(HashTextureKind::Xor),
        xor_texture: Some(stored),
        ..PatternVariant::default()
    };

    // While a tile waits the default texture is not the one the file was written against, so the
    // rebuilt texture is not yet the wanted one — but it is the exclusive-or, not a fallback.
    let waiting_pattern = pattern(vec![Some(solid(1)), Some(waiting(2))], variant.clone());
    let waiting_textures = PatternTextures::of(&waiting_pattern);
    assert!(waiting_textures.has_waiting_tile());
    let built = waiting_textures.texture_of(&variant).unwrap();
    assert_eq!((16, 8), (built.width, built.height));
    assert_ne!(wanted, built.pixels.as_ref().unwrap().data);

    // Once the tiles are there, it is exactly the texture the profile meant.
    let full_pattern = pattern(vec![Some(solid(1)), Some(solid(2))], variant.clone());
    let full_textures = PatternTextures::of(&full_pattern);
    assert!(!full_textures.has_waiting_tile());
    let built = full_textures.texture_of(&variant).unwrap();
    assert_eq!(wanted, built.pixels.as_ref().unwrap().data);
}

// REQ-PROF-083: while a tile waits and field 24 has other dimensions or another format, the
// variant uses the default texture itself (the 146 variants of the 101 profiles).
#[test]
fn a_variant_of_kind_xor_of_another_shape_falls_back_while_a_tile_waits() {
    let stored = VariantTexture::new(
        4,
        4,
        Some(TexturePixels::new(
            TextureFormat::Direct,
            vec![7; 16],
            Vec::new(),
        )),
    );
    let variant = PatternVariant {
        texture_kind: Some(HashTextureKind::Xor),
        xor_texture: Some(stored.clone()),
        ..PatternVariant::default()
    };

    let waiting_pattern = pattern(vec![Some(solid(1)), Some(waiting(3))], variant.clone());
    let fallback = PatternTextures::of(&waiting_pattern)
        .texture_of(&variant)
        .unwrap();
    let base = DefaultTexture::of(&waiting_pattern);
    assert_eq!((base.width, base.height), (fallback.width, fallback.height));
    assert_eq!(
        base.pixels.data,
        fallback.pixels.as_ref().unwrap().data,
        "the fallback is the default texture itself"
    );

    // Resolved, the exclusive-or runs whatever the shapes.
    let full_pattern = pattern(vec![Some(solid(1)), Some(solid(2))], variant.clone());
    let built = PatternTextures::of(&full_pattern)
        .texture_of(&variant)
        .unwrap();
    assert_eq!((4, 4), (built.width, built.height));
    assert_eq!(TextureFormat::Direct, built.pixels.as_ref().unwrap().format);
}

// A 3dn variant is untouched: it uses the texture it carries, and the default texture is never
// built for it.
#[test]
fn a_3dn_variant_keeps_the_texture_it_carries() {
    let explicit = VariantTexture::new(
        1,
        1,
        Some(TexturePixels::new(
            TextureFormat::Direct,
            vec![0x00FF_00FF],
            Vec::new(),
        )),
    );
    let variant = PatternVariant {
        texture: Some(explicit),
        ..PatternVariant::default()
    };
    let shape = pattern(vec![Some(solid(1))], variant.clone());

    let texture = PatternTextures::of(&shape).texture_of(&variant).unwrap();
    assert_eq!((1, 1), (texture.width, texture.height));
    assert_eq!(vec![0x00FF_00FF], texture.pixels.as_ref().unwrap().data);

    let without = PatternVariant::default();
    assert!(PatternTextures::of(&shape).texture_of(&without).is_none());
}

// The default texture of a pattern whose tiles are flipped follows the displayed pixels
// (REQ-PROF-031), as SPEC-03's own default texture does.
#[test]
fn the_default_texture_applies_the_mirroring_of_its_tiles() {
    let raw_tile = {
        let mut bytes = [0_u8; 16];
        bytes[0] = 0b1000_0000;
        bytes
    };
    let (low, high) = raw_tile::to_planes(&raw_tile);
    let plain = Arc::new(ProfileTile::new(low, high, TileFlip::NONE));
    let mirrored = Arc::new(ProfileTile::new(low, high, TileFlip::HORIZONTAL));

    let plain_texture = DefaultTexture::of(&pattern(vec![Some(plain)], PatternVariant::default()));
    let mirrored_texture =
        DefaultTexture::of(&pattern(vec![Some(mirrored)], PatternVariant::default()));

    assert_eq!(1, texel(&plain_texture.pixels, 0));
    assert_eq!(0, texel(&plain_texture.pixels, 7));
    assert_eq!(0, texel(&mirrored_texture.pixels, 0));
    assert_eq!(1, texel(&mirrored_texture.pixels, 7));
}
