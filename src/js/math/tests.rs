use super::{cos, sin};

/// `x sin(x) cos(x)` as IEEE-754 bit patterns from Node 26 (V8 14.6)
/// `Math.sin`/`Math.cos`: the 0.1° direction sweep, random magnitudes from
/// 2^-30 to 2^30, arguments near multiples of π/2 and huge arguments.
/// x64 values match Node on Linux and Windows x64; arm64 values match Node on
/// macOS and Linux arm64.
const X64: &str = include_str!("../../../tests/data/v8_trig_x64.txt");
const ARM64: &str = include_str!("../../../tests/data/v8_trig_arm64.txt");

fn parse(s: &str) -> Option<f64> {
    u64::from_str_radix(s, 16).ok().map(f64::from_bits)
}

fn check<const FUSED: bool>(vectors: &str) -> usize {
    let mut checked = 0;
    for line in vectors.lines() {
        let mut it = line.split(' ').filter_map(parse);
        let (Some(x), Some(s), Some(c)) = (it.next(), it.next(), it.next()) else {
            continue;
        };
        assert_eq!(sin::<FUSED>(x).to_bits(), s.to_bits(), "sin({x:e})");
        assert_eq!(cos::<FUSED>(x).to_bits(), c.to_bits(), "cos({x:e})");
        checked += 1;
    }
    checked
}

#[test]
fn plain_evaluation_matches_v8_x64() {
    assert!(check::<false>(X64) > 7000);
}

#[test]
fn fused_evaluation_matches_v8_arm64() {
    assert!(check::<true>(ARM64) > 7000);
}

#[test]
fn the_platforms_really_differ() {
    let x = 0.335_103_216_382_911_24;
    assert_ne!(sin::<false>(x), sin::<true>(x));
}

#[test]
fn special_values() {
    assert!(sin::<false>(f64::NAN).is_nan() && cos::<true>(f64::INFINITY).is_nan());
    assert_eq!(sin::<false>(-0.0).to_bits(), (-0.0f64).to_bits());
    assert_eq!(cos::<false>(0.0), 1.0);
}
