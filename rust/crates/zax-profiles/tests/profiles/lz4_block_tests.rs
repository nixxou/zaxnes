use zax_profiles::import::decode_lz4_block;

use super::common::lz4_encoder;

fn pattern(length: usize) -> Vec<u8> {
    (0..length)
        .map(|index| {
            if index % 11 == 0 {
                u8::try_from(index % 251).unwrap()
            } else {
                0x7E
            }
        })
        .collect()
}

/// AMD-030 §4.1
#[test]
fn every_block_the_encoder_produces_decodes_back() {
    for length in [0, 1, 5, 15, 16, 270, 65_536, 131_072] {
        let plain = pattern(length);
        let compressed = lz4_encoder::compress(&plain);
        let mut decoded = vec![0; plain.len()];

        let produced = decode_lz4_block(&compressed, &mut decoded).unwrap();

        assert_eq!(plain.len(), produced, "length {length}");
        assert_eq!(plain, decoded, "length {length}");
    }
}

/// AMD-030 §4.1: the whole point of LZ4 is that it is shorter than the plain bytes.
#[test]
fn a_repetitive_block_compresses() {
    let plain = vec![0x5A; 100_000];

    let compressed = lz4_encoder::compress(&plain);

    assert!(
        compressed.len() < plain.len() / 10,
        "{} bytes for {}: the block is not compressed.",
        compressed.len(),
        plain.len()
    );

    let mut decoded = vec![0; plain.len()];
    assert_eq!(plain.len(), decode_lz4_block(&compressed, &mut decoded).unwrap());
    assert_eq!(plain, decoded);
}

/// AMD-030 §4.1: a match may overlap itself, which is how a run is encoded.
#[test]
fn an_overlapping_match_repeats_the_bytes_it_covers() {
    // Token 0x1F: one literal, match length 15 + extra; literal 'A'; offset 1; extra 0 -> a
    // match of 19 bytes one byte back, i.e. 'A' twenty times.
    let block = [0x1F, b'A', 0x01, 0x00, 0x00];
    let mut decoded = [0; 20];

    let produced = decode_lz4_block(&block, &mut decoded).unwrap();

    assert_eq!(20, produced);
    assert_eq!("A".repeat(20), String::from_utf8(decoded.to_vec()).unwrap());
}

/// AMD-030 §4.1: literal lengths above 14 are continued with 255 bytes.
#[test]
fn a_long_literal_run_reads_its_continuation_bytes() {
    let literals = pattern(300);
    let mut block = vec![0xF0, 255, 30]; // 15 + 255 + 30 = 300 literals, no match.
    block.extend_from_slice(&literals);
    let mut decoded = vec![0; 300];

    let produced = decode_lz4_block(&block, &mut decoded).unwrap();

    assert_eq!(300, produced);
    assert_eq!(literals, decoded);
}

/// AMD-030 §5: a back-reference before the start of the output.
#[test]
fn a_match_pointing_before_the_output_is_refused() {
    let block = [0x10, b'A', 0x05, 0x00]; // One literal, offset 5: nothing is there.
    let mut decoded = [0; 16];

    let error = decode_lz4_block(&block, &mut decoded).unwrap_err();

    assert!(error.message().contains("refers 5 bytes back"), "{error}");
}

/// AMD-030 §5: a literal run that runs past the end of the block.
#[test]
fn a_truncated_literal_run_is_refused() {
    let block = [0x50, b'A', b'B']; // Five literals announced, two given.
    let mut decoded = [0; 16];

    let error = decode_lz4_block(&block, &mut decoded).unwrap_err();

    assert!(error.message().contains("ends early"), "{error}");
}

/// AMD-030 §5: a continuation that never ends.
#[test]
fn a_length_that_runs_off_the_block_is_refused() {
    let block = [0xF0, 255, 255]; // Continuation bytes with nothing after them.
    let mut decoded = [0; 16];

    assert!(decode_lz4_block(&block, &mut decoded).is_err());
}

/// AMD-030 §5: more output than the block table announced.
#[test]
fn a_block_that_decodes_to_more_than_announced_is_refused() {
    let block = [0x80, 1, 2, 3, 4, 5, 6, 7, 8]; // Eight literals.
    let mut decoded = [0; 4];

    let error = decode_lz4_block(&block, &mut decoded).unwrap_err();

    assert!(error.message().contains("more than the announced 4 bytes"), "{error}");
}

/// AMD-030 §5: a match longer than the room left.
#[test]
fn a_match_longer_than_the_output_buffer_is_refused() {
    let block = [0x1F, b'A', 0x01, 0x00, 0xFF, 0xFF, 0x00];
    let mut decoded = [0; 32];

    assert!(decode_lz4_block(&block, &mut decoded).is_err());
}
