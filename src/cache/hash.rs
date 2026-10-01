//! A fast, randomly keyed hasher for cache keys.
//!
//! Keys are a few hundred bytes the caller does not control, and SipHash
//! spends most of a cache hit on them. This folds 8 bytes per multiply and is
//! randomly keyed per cache, so keys cannot be chosen to collide without
//! knowing the seed.

use std::hash::{BuildHasher, Hasher, RandomState};

const K1: u64 = 0x9E37_79B9_7F4A_7C15;
const K2: u64 = 0xD6E8_FEB8_6659_FD93;

/// 128-bit product folded to 64 bits: every input bit reaches every output bit
/// range after two rounds.
fn fold(a: u64, b: u64) -> u64 {
    let r = u128::from(a) * u128::from(b);
    (r as u64) ^ ((r >> 64) as u64)
}

/// Builds [`KeyHasher`]s sharing one random seed.
#[derive(Clone, Debug)]
pub(super) struct KeyHasherBuilder {
    seed: u64,
}

impl Default for KeyHasherBuilder {
    fn default() -> Self {
        KeyHasherBuilder {
            seed: RandomState::new().hash_one(0u64),
        }
    }
}

impl BuildHasher for KeyHasherBuilder {
    type Hasher = KeyHasher;

    fn build_hasher(&self) -> KeyHasher {
        KeyHasher { state: self.seed }
    }
}

pub(super) struct KeyHasher {
    state: u64,
}

impl KeyHasher {
    fn absorb(&mut self, word: u64) {
        self.state = fold(self.state ^ word, K1);
    }
}

impl Hasher for KeyHasher {
    fn write(&mut self, bytes: &[u8]) {
        let mut words = bytes.chunks_exact(8);
        for chunk in &mut words {
            if let Ok(w) = <[u8; 8]>::try_from(chunk) {
                self.absorb(u64::from_le_bytes(w));
            }
        }
        let tail = words
            .remainder()
            .iter()
            .enumerate()
            .fold(0u64, |acc, (i, &b)| acc | (u64::from(b) << (8 * i)));
        // The length keeps "ab" + "c" distinct from "a" + "bc" and zero padding
        // distinct from a real zero byte.
        self.absorb(tail ^ ((bytes.len() as u64) << 56));
    }

    fn finish(&self) -> u64 {
        fold(self.state, K2)
    }
}

#[cfg(test)]
mod tests;
