//! Synthetic iNES files and the "3dh" profiles that go with them, for the resolution of
//! AMD-062 REQ-PROF-078 and the cache of REQ-PROF-082.

use zax_profiles::format::ProfileDocument;
use zax_profiles::hash::profile;
use zax_profiles::hash::raw_tile::{self, RawTile};
use zax_profiles::hash::tile_id::TileId;

use super::synthetic_hash_profile::{EnvelopeSpec, PatternSpec, PayloadSpec, TileSpec};

/// Length of an iNES header.
const HEADER_SIZE: usize = 16;

/// Length of a trainer.
const TRAINER_SIZE: usize = 512;

const PRG_UNIT: usize = 16 * 1024;
const CHR_UNIT: usize = 8 * 1024;

/// An iNES file of `prg_units` PRG banks and `chr_units` CHR banks, the CHR zone starting with
/// `chr` and padded with zeroes.
pub fn ines(prg_units: u8, chr_units: u8, chr: &[u8]) -> Vec<u8> {
    let mut file = vec![0_u8; HEADER_SIZE];
    file[..4].copy_from_slice(&[b'N', b'E', b'S', 0x1A]);
    file[4] = prg_units;
    file[5] = chr_units;
    file.resize(HEADER_SIZE + usize::from(prg_units) * PRG_UNIT, 0);
    file.extend_from_slice(chr);
    file.resize(
        HEADER_SIZE + usize::from(prg_units) * PRG_UNIT + usize::from(chr_units) * CHR_UNIT,
        0,
    );
    file
}

/// The same file with a trainer, declared by bit 2 of byte 6.
pub fn with_trainer(file: &[u8]) -> Vec<u8> {
    let mut trained = file[..HEADER_SIZE].to_vec();
    trained[6] |= 0x04;
    trained.extend(std::iter::repeat_n(0xAA, TRAINER_SIZE));
    trained.extend_from_slice(&file[HEADER_SIZE..]);
    trained
}

/// An iNES file whose CHR zone holds the sixteen bytes of each tile of `tiles`, each at a
/// position unaligned on sixteen (REQ-PROF-078's acceptance criterion).
pub fn chr_rom_with_tiles(tiles: &[RawTile]) -> Vec<u8> {
    let mut chr = vec![0x5A_u8];
    for tile in tiles {
        chr.extend_from_slice(tile);
        // One filler byte between two tiles, so that every one of them sits at an offset of
        // 1 modulo 17 and none is aligned on sixteen.
        chr.push(0x5A);
    }

    let units = u8::try_from(chr.len().div_ceil(CHR_UNIT).max(1)).unwrap();
    ines(1, units, &chr)
}

/// The bytes of a "3dh" profile of one pattern of `tiles.len()` by 1 cells, each cell naming one
/// of `tiles` by its identifier (field 5).
pub fn hash_profile_bytes(tiles: &[RawTile]) -> Vec<u8> {
    let ids: Vec<TileId> = tiles.iter().map(TileId::compute).collect();
    let cells = ids.iter().map(|id| Some(TileSpec::named(*id))).collect();
    EnvelopeSpec {
        ids,
        payload: Some(PayloadSpec::of(vec![PatternSpec::row(cells)]).encode()),
        ..EnvelopeSpec::default()
    }
    .encode()
}

/// The same profile, read.
pub fn hash_profile_of(tiles: &[RawTile]) -> ProfileDocument {
    let bytes = hash_profile_bytes(tiles);
    profile::read(&bytes).unwrap_or_else(|error| panic!("{error}"))
}

/// A "3dh" profile whose cells name their tiles by their legacy digest (field 4).
pub fn hash_profile_named_by_digest(tiles: &[RawTile]) -> ProfileDocument {
    let digests: Vec<String> = tiles.iter().map(raw_tile::legacy_digest).collect();
    let cells = digests
        .iter()
        .map(|digest| Some(TileSpec::text(digest)))
        .collect();
    let bytes = EnvelopeSpec {
        hashes: digests,
        payload: Some(PayloadSpec::of(vec![PatternSpec::row(cells)]).encode()),
        ..EnvelopeSpec::default()
    }
    .encode();

    profile::read(&bytes).unwrap_or_else(|error| panic!("{error}"))
}
