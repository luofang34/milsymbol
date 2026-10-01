use super::KeyHasherBuilder;
use std::collections::HashSet;
use std::hash::BuildHasher;
use std::vec::Vec;

fn hash(b: &KeyHasherBuilder, bytes: &[u8]) -> u64 {
    b.hash_one(bytes)
}

#[test]
fn equal_keys_hash_equally_and_builders_are_keyed() {
    let (a, b) = (KeyHasherBuilder::default(), KeyHasherBuilder::default());
    assert_eq!(hash(&a, b"key"), hash(&a, b"key"));
    assert_ne!(
        hash(&a, b"key"),
        hash(&b, b"key"),
        "two caches share a seed"
    );
}

/// Fixed seeds keep the outcome deterministic: collisions depend on the seed,
/// and these are inputs that must not collide under any of them.
#[test]
fn distinct_keys_do_not_collide() {
    for seed in [0, 1, 0x9E37_79B9_7F4A_7C15, u64::MAX] {
        let b = KeyHasherBuilder { seed };
        let mut seen = HashSet::new();
        assert!(seen.insert(hash(&b, &[])));
        for len in 1..40usize {
            for byte in [0u8, 1, 0x80, 0xff] {
                assert!(
                    seen.insert(hash(&b, &vec![byte; len])),
                    "seed {seed} len {len} byte {byte}"
                );
            }
        }
        for i in 0..50_000u32 {
            assert!(
                seen.insert(hash(&b, format!("10031000001211000000-{i}").as_bytes())),
                "seed {seed} {i}"
            );
        }
    }
}

#[test]
fn flipping_any_input_bit_changes_about_half_the_output_bits() {
    let b = KeyHasherBuilder::default();
    let base: Vec<u8> = (0..191u32).map(|i| (i * 7 + 3) as u8).collect();
    let h0 = hash(&b, &base);
    let (mut total, mut flips) = (0u32, 0u32);
    for bit in 0..base.len() * 8 {
        let mut k = base.clone();
        if let Some(byte) = k.get_mut(bit / 8) {
            *byte ^= 1 << (bit % 8);
        }
        total += (h0 ^ hash(&b, &k)).count_ones();
        flips += 1;
    }
    let mean = f64::from(total) / f64::from(flips);
    assert!(
        (28.0..36.0).contains(&mean),
        "mean flipped output bits {mean}"
    );
}

#[test]
fn a_length_prefix_does_not_let_pieces_shift() {
    let b = KeyHasherBuilder::default();
    assert_ne!(hash(&b, b"a\0"), hash(&b, b"a"));
    assert_ne!(hash(&b, &[0u8; 8]), hash(&b, &[0u8; 9]));
}
