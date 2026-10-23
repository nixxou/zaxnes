use zax_profiles::import::{
    read_text_assets, UnityBundle, UnityCompression, UnitySerializedFile, TEXT_ASSET_CLASS_ID,
};

use super::common::synthetic_bundle::{
    self as synthetic, compression_code, SyntheticBundleOptions, ASSET_BUNDLE_CLASS_ID,
};

fn payload(seed: u8, length: usize) -> Vec<u8> {
    // Compressible but not trivial: long runs with a changing pattern, which gives the LZ4
    // encoder both literals and matches.
    (0..length)
        .map(|index| {
            if index % 97 < 40 {
                seed
            } else {
                u8::try_from((usize::from(seed) + index) % 251).unwrap()
            }
        })
        .collect()
}

fn assets() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("Pinball", payload(0x11, 300)),
        ("Blaster Master (U)", payload(0x22, 70_000)),
        ("pacman", payload(0x33, 5)),
    ]
}

fn layouts() -> Vec<(&'static str, SyntheticBundleOptions)> {
    let default = SyntheticBundleOptions::default;
    vec![
        ("stored, one block, info first", default()),
        ("stored, info at the end", SyntheticBundleOptions { info_at_end: true, ..default() }),
        ("stored, 8 KiB blocks", SyntheticBundleOptions { block_size: 8 * 1024, ..default() }),
        (
            "LZ4 blocks, one block",
            SyntheticBundleOptions { block_compression: UnityCompression::Lz4, ..default() },
        ),
        (
            "LZ4 blocks of 128 KiB, as measured",
            SyntheticBundleOptions {
                block_compression: UnityCompression::Lz4,
                block_size: 128 * 1024,
                ..default()
            },
        ),
        (
            "LZ4 info and blocks, info at the end",
            SyntheticBundleOptions {
                info_compression: UnityCompression::Lz4,
                block_compression: UnityCompression::Lz4,
                block_size: 16 * 1024,
                info_at_end: true,
                ..default()
            },
        ),
        (
            "LZ4HC flagged blocks",
            SyntheticBundleOptions {
                block_compression: UnityCompression::Lz4Hc,
                block_size: 32 * 1024,
                ..default()
            },
        ),
        ("big-endian serialized file", SyntheticBundleOptions { big_endian: true, ..default() }),
        (
            "big-endian, LZ4, several blocks, info at the end",
            SyntheticBundleOptions {
                big_endian: true,
                block_compression: UnityCompression::Lz4,
                info_compression: UnityCompression::Lz4,
                block_size: 4 * 1024,
                info_at_end: true,
                ..default()
            },
        ),
        ("serialized file version 17", SyntheticBundleOptions { serialized_version: 17, ..default() }),
        ("serialized file version 19", SyntheticBundleOptions { serialized_version: 19, ..default() }),
        ("serialized file version 22", SyntheticBundleOptions { serialized_version: 22, ..default() }),
        (
            "serialized file version 22, big-endian",
            SyntheticBundleOptions { serialized_version: 22, big_endian: true, ..default() },
        ),
        ("no type tree", SyntheticBundleOptions { type_tree: false, ..default() }),
        ("bundle format version 6", SyntheticBundleOptions { format_version: 6, ..default() }),
        ("bundle format version 8", SyntheticBundleOptions { format_version: 8, ..default() }),
    ]
}

/// AMD-030 §4, §6
#[test]
fn every_layout_yields_the_same_text_assets() {
    let expected = assets();
    for (layout, options) in layouts() {
        let bundle = synthetic::build(&expected, &options);

        let read = read_text_assets(&bundle).unwrap();

        assert_eq!(expected.len(), read.len(), "{layout}");
        for ((name, script), asset) in expected.iter().zip(&read) {
            assert_eq!(*name, asset.name, "{layout}");
            assert!(*script == asset.script, "{layout}: the payload of \"{name}\" came back changed.");
        }
    }
}

/// AMD-030 §4.1
#[test]
fn the_header_fields_are_read() {
    let bundle = synthetic::build(&assets(), &SyntheticBundleOptions::default());

    let container = UnityBundle::read(&bundle).unwrap();

    assert_eq!(7, container.format_version);
    assert_eq!("5.x.x", container.player_version);
    assert_eq!(synthetic::ENGINE_VERSION, container.engine_version);
    assert_eq!(1, container.files.len());
    let node = &container.files[0];
    assert!(node.path.starts_with("CAB-"));
    assert_eq!(4, node.flags);
}

/// AMD-030 §4.1: the information block of a version 7 bundle starts on a multiple of 16.
#[test]
fn the_information_block_of_a_version_7_bundle_is_aligned_on_16_bytes() {
    let bundle = synthetic::build(&assets(), &SyntheticBundleOptions::default());

    // "UnityFS\0" + u32 + "5.x.x\0" + "2019.4.41f2\0" + i64 + 3 x u32 = 50 bytes, and the block
    // that follows starts at 64. The padding is what the bytes between show.
    assert!(50 < bundle.len());
    assert_eq!(64, 50_usize.div_ceil(16) * 16);
    assert_eq!(vec![0_u8; 14], bundle[50..64]);

    // Reading it back proves the reader skips exactly the same padding.
    assert_eq!(3, read_text_assets(&bundle).unwrap().len());
}

/// AMD-030 §4.3, §4.4
#[test]
fn the_serialized_file_exposes_its_version_order_and_classes() {
    let expected = assets();
    let bundle = synthetic::build(
        &expected,
        &SyntheticBundleOptions { big_endian: true, serialized_version: 21, ..SyntheticBundleOptions::default() },
    );
    let container = UnityBundle::read(&bundle).unwrap();

    let serialized = UnitySerializedFile::read(&container.files[0].data).unwrap();

    assert_eq!(21, serialized.version);
    assert!(serialized.is_big_endian);
    assert_eq!(synthetic::ENGINE_VERSION, serialized.unity_version);

    // The measured file holds one TextAsset per profile plus the AssetBundle object.
    assert_eq!(expected.len() + 1, serialized.objects.len());
    assert_eq!(
        expected.len(),
        serialized.objects.iter().filter(|object| object.class_id == TEXT_ASSET_CLASS_ID).count()
    );
    assert_eq!(
        1,
        serialized.objects.iter().filter(|object| object.class_id == ASSET_BUNDLE_CLASS_ID).count()
    );
}

/// AMD-030 §4.4: an empty payload is still a payload.
#[test]
fn an_empty_text_asset_is_read() {
    let bundle = synthetic::build(&[("empty", Vec::new())], &SyntheticBundleOptions::default());

    let read = read_text_assets(&bundle).unwrap();

    assert_eq!(1, read.len());
    assert_eq!("empty", read[0].name);
    assert!(read[0].script.is_empty());
}

/// AMD-030 §4.4: a name whose length is not a multiple of four exercises the alignment.
#[test]
fn names_of_every_length_modulo_four_are_read() {
    let assets: Vec<(&str, Vec<u8>)> = vec![
        ("a", vec![1]),
        ("ab", vec![2, 2]),
        ("abc", vec![3, 3, 3]),
        ("abcd", vec![4, 4, 4, 4]),
        ("accentué", vec![5]),
    ];

    let read = read_text_assets(&synthetic::build(&assets, &SyntheticBundleOptions::default())).unwrap();

    assert_eq!(assets.len(), read.len());
    for ((name, script), asset) in assets.iter().zip(&read) {
        assert_eq!(*name, asset.name);
        assert_eq!(*script, asset.script);
    }
}

/// AMD-030 §5: unknown signature.
#[test]
fn a_file_that_is_not_a_bundle_is_refused() {
    let bytes = b"UnityWeb\0and then nothing useful at all......";

    let error = read_text_assets(bytes).unwrap_err();

    assert!(error.message().contains("not a Unity asset bundle"), "{error}");
    assert!(error.message().contains("UnityWeb"), "{error}");
}

/// AMD-030 §5: a truncated file, cut at several places.
#[test]
fn a_truncated_bundle_is_refused() {
    let bundle = synthetic::build(&assets(), &SyntheticBundleOptions::default());

    for kept in [0, 4, 30, 60, 200] {
        assert!(read_text_assets(&bundle[..kept.min(bundle.len())]).is_err(), "kept {kept}");
    }
}

/// AMD-030 §5: cutting the tail off an otherwise well-formed bundle.
#[test]
fn a_bundle_cut_in_its_data_blocks_is_refused() {
    let bundle = synthetic::build(
        &assets(),
        &SyntheticBundleOptions { block_size: 8 * 1024, ..SyntheticBundleOptions::default() },
    );

    let error = read_text_assets(&bundle[..bundle.len() - 4096]).unwrap_err();

    assert!(error.message().contains("truncated"), "{error}");
}

/// AMD-030 §4.1: LZMA is refused with a message that names it.
#[test]
fn an_lzma_information_block_is_refused() {
    let bundle = synthetic::build(
        &assets(),
        &SyntheticBundleOptions {
            info_compression_code: Some(compression_code(UnityCompression::Lzma)),
            ..SyntheticBundleOptions::default()
        },
    );

    let error = read_text_assets(&bundle).unwrap_err();

    assert!(error.message().contains("LZMA"), "{error}");
    assert!(error.message().contains("information block"), "{error}");
}

/// AMD-030 §4.1
#[test]
fn lzma_data_blocks_are_refused() {
    let bundle = synthetic::build(
        &assets(),
        &SyntheticBundleOptions {
            block_compression_code: Some(u16::try_from(compression_code(UnityCompression::Lzma)).unwrap()),
            ..SyntheticBundleOptions::default()
        },
    );

    let error = read_text_assets(&bundle).unwrap_err();

    assert!(error.message().contains("LZMA"), "{error}");
}

/// AMD-030 §4.2: an unknown compression code is refused, not guessed.
#[test]
fn an_unknown_compression_code_is_refused() {
    let bundle = synthetic::build(
        &assets(),
        &SyntheticBundleOptions { block_compression_code: Some(9), ..SyntheticBundleOptions::default() },
    );

    let error = read_text_assets(&bundle).unwrap_err();

    assert!(error.message().contains("unknown compression code 9"), "{error}");
}

/// AMD-030 §5: a file table that points outside the decompressed stream.
#[test]
fn a_file_node_out_of_range_is_refused() {
    for (offset_delta, size_delta) in [(1_000_000, 0), (-8, 0), (0, 1_000_000), (0, -1_000_000)] {
        let bundle = synthetic::build(
            &assets(),
            &SyntheticBundleOptions {
                node_offset_delta: offset_delta,
                node_size_delta: size_delta,
                ..SyntheticBundleOptions::default()
            },
        );

        assert!(read_text_assets(&bundle).is_err(), "{offset_delta} {size_delta}");
    }
}

/// AMD-030 §5: a block table announcing an absurd stream is refused, not allocated.
#[test]
fn a_block_table_announcing_more_than_the_cap_is_refused() {
    let bundle = synthetic::build(
        &assets(),
        &SyntheticBundleOptions {
            block_uncompressed_size_code: Some(0xF000_0000),
            ..SyntheticBundleOptions::default()
        },
    );

    let error = read_text_assets(&bundle).unwrap_err();

    assert!(error.message().contains("once decompressed"), "{error}");
}

/// AMD-030 §4.1: a format version outside 6 to 8.
#[test]
fn an_unsupported_bundle_version_is_refused() {
    let mut bundle = synthetic::build(&assets(), &SyntheticBundleOptions::default());

    // The version is the u32 right after "UnityFS\0".
    bundle[8..12].copy_from_slice(&[0, 0, 0, 9]);

    let error = read_text_assets(&bundle).unwrap_err();

    assert!(error.message().contains("version 9 is not supported"), "{error}");
}

/// AMD-030 §4.1: bit 0x40 clear leaves nothing to extract.
#[test]
fn a_bundle_without_a_file_list_is_refused() {
    let bundle = synthetic::build(
        &assets(),
        &SyntheticBundleOptions { file_list: false, ..SyntheticBundleOptions::default() },
    );

    let error = read_text_assets(&bundle).unwrap_err();

    assert!(error.message().contains("no file list"), "{error}");
}

/// AMD-030 §4.3: versions 17 to 22 and nothing else.
#[test]
fn an_unsupported_serialized_file_version_is_refused() {
    for version in [16_u8, 23] {
        let mut serialized = synthetic::build_serialized_file(
            &assets(),
            &SyntheticBundleOptions { serialized_version: 21, ..SyntheticBundleOptions::default() },
        );

        // The version is the third u32 of the big-endian header.
        serialized[8..12].copy_from_slice(&[0, 0, 0, version]);

        let error = UnitySerializedFile::read(&serialized).unwrap_err();

        assert!(error.message().contains(&format!("version {version} is not supported")), "{error}");
    }
}

/// AMD-030 §5: an object table pointing past the end of the file.
#[test]
fn a_serialized_file_whose_object_table_is_out_of_range_is_refused() {
    let serialized = synthetic::build_serialized_file(&assets(), &SyntheticBundleOptions::default());

    // Shorten the file without touching its table: every object now ends out of range.
    let cut = &serialized[..serialized.len() / 2];

    assert!(UnitySerializedFile::read(cut).is_err());
}
