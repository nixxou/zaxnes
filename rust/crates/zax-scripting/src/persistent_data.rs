//! The persistent-data block a savestate carries: 64 little-endian IEEE-754 single-precision
//! reals, `PData[1]` to `PData[64]` (SPEC-05 REQ-LUA-073, §4.6).

/// How many entries are saved, the value of the global `pdata_count`.
pub const ENTRY_COUNT: usize = 64;

/// [`ENTRY_COUNT`] as a Lua number.
pub const ENTRY_COUNT_REAL: f64 = 64.0;

/// Size of the block in bytes, always the same.
pub const BLOCK_SIZE: usize = ENTRY_COUNT * 4;

/// The 1-based `PData` index `GetPData(i)` and `SetPData(i, v)` address: `(i mod 64) + 1`, the
/// modulo giving a positive result (REQ-LUA-070).
#[must_use]
pub fn wrap_index(index: i32) -> i64 {
    const COUNT: i32 = 64;
    i64::from(index.rem_euclid(COUNT)) + 1
}

/// Writes the 64 values as the 256-byte block of §4.6. Entries missing from `values` are
/// written as 0.
#[must_use]
pub fn write(values: &[f32]) -> [u8; BLOCK_SIZE] {
    let mut block = [0_u8; BLOCK_SIZE];
    for (chunk, value) in block.chunks_exact_mut(4).zip(values) {
        chunk.copy_from_slice(&value.to_le_bytes());
    }

    block
}

/// Reads the 64 values from a block. A short or absent block gives zeros for the entries it
/// does not cover, so a truncated savestate never fails (REQ-ACC-041).
#[must_use]
pub fn read(block: &[u8]) -> [f32; ENTRY_COUNT] {
    let mut values = [0.0_f32; ENTRY_COUNT];
    for (value, chunk) in values.iter_mut().zip(block.chunks_exact(4)) {
        *value = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
    }

    values
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_real_count_is_the_count() {
        assert_eq!(
            ENTRY_COUNT_REAL,
            f64::from(u8::try_from(ENTRY_COUNT).unwrap())
        );
    }

    #[test]
    fn index_wraps_to_one_based_positive() {
        assert_eq!(wrap_index(0), 1);
        assert_eq!(wrap_index(63), 64);
        assert_eq!(wrap_index(64), 1);
        assert_eq!(wrap_index(-1), 64);
    }

    #[test]
    fn a_short_block_gives_zeros() {
        let values = read(&1.5_f32.to_le_bytes());
        assert_eq!(values[0], 1.5);
        assert_eq!(values[1], 0.0);
    }
}
