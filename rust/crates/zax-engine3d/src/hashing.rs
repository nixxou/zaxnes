//! A fast hasher for the engine's and the renderer's per-frame maps (Rust-only performance
//! work): their keys are small integers and ids hashed thousands of times per frame, where
//! the standard `SipHash` costs more than the lookup itself. No map relies on an iteration
//! order (the standard one is random per process anyway), so the output does not change.

use std::collections::{HashMap, HashSet};
use std::hash::{BuildHasherDefault, Hasher};

/// A `HashMap` with [`FastHasher`].
pub type FastMap<K, V> = HashMap<K, V, BuildHasherDefault<FastHasher>>;
/// A `HashSet` with [`FastHasher`].
pub type FastSet<T> = HashSet<T, BuildHasherDefault<FastHasher>>;

/// The multiply-and-rotate hash of rustc's `FxHasher`: not resistant to chosen keys, which
/// none of these maps receive.
#[derive(Debug, Default, Clone, Copy)]
pub struct FastHasher(u64);

const MULTIPLIER: u64 = 0x517c_c1b7_2722_0a95;

impl FastHasher {
    fn add(&mut self, word: u64) {
        self.0 = (self.0.rotate_left(5) ^ word).wrapping_mul(MULTIPLIER);
    }
}

impl Hasher for FastHasher {
    fn write(&mut self, bytes: &[u8]) {
        for chunk in bytes.chunks(8) {
            self.add(
                chunk
                    .iter()
                    .fold(0, |word, byte| (word << 8) | u64::from(*byte)),
            );
        }
    }

    fn write_u8(&mut self, value: u8) {
        self.add(u64::from(value));
    }

    fn write_u16(&mut self, value: u16) {
        self.add(u64::from(value));
    }

    fn write_u32(&mut self, value: u32) {
        self.add(u64::from(value));
    }

    fn write_u64(&mut self, value: u64) {
        self.add(value);
    }

    fn write_usize(&mut self, value: usize) {
        self.add(value as u64);
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_map_finds_what_it_holds() {
        let mut map: FastMap<(u32, u8), u32> = FastMap::default();
        for index in 0..1000_u32 {
            map.insert((index, (index % 7).to_le_bytes()[0]), index);
        }
        for index in 0..1000_u32 {
            assert_eq!(
                Some(&index),
                map.get(&(index, (index % 7).to_le_bytes()[0]))
            );
        }
        assert_eq!(1000, map.len());
    }
}
