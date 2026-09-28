#![allow(clippy::float_cmp)]
use super::*;

const NUMBERS: &[(f64, &str)] = &[
    (0.0, "0"),
    (-0.0, "0"),
    (1.0, "1"),
    (-1.0, "-1"),
    (1e-1, "0.1"),
    (3.0000000000000004e-1, "0.30000000000000004"),
    (3.333333333333333e-1, "0.3333333333333333"),
    (6.666666666666666e-1, "0.6666666666666666"),
    (2.2222222222222223e2, "222.22222222222223"),
    (2.2222222222222223e0, "2.2222222222222223"),
    (1.4285714285714286e0, "1.4285714285714286"),
    (6.32e1, "63.2"),
    (1e21, "1e+21"),
    (1e20, "100000000000000000000"),
    (1.2345678901234568e20, "123456789012345680000"),
    (1.5e-7, "1.5e-7"),
    (1e-6, "0.000001"),
    (1.2e-6, "0.0000012"),
    (-1e-7, "-1e-7"),
    (5e-324, "5e-324"),
    (1.7976931348623157e308, "1.7976931348623157e+308"),
    (f64::NAN, "NaN"),
    (f64::INFINITY, "Infinity"),
    (f64::NEG_INFINITY, "-Infinity"),
    (4.35e0, "4.35"),
    (9.500000000000001e1, "95.00000000000001"),
    (-2.5e-10, "-2.5e-10"),
    (1e100, "1e+100"),
    (1.23456e2, "123.456"),
    (5e-1, "0.5"),
    (2.5000000000000004e1, "25.000000000000004"),
];

const STRINGS: &[(&str, f64)] = &[
    ("", 0.0),
    (" ", 0.0),
    ("12", 12.0),
    (" 12 ", 12.0),
    ("1e3", 1000.0),
    ("0x1F", 31.0),
    ("0b101", 5.0),
    ("0o17", 15.0),
    ("1.", 1.0),
    (".5", 5e-1),
    (".", f64::NAN),
    ("abc", f64::NAN),
    ("Infinity", f64::INFINITY),
    ("-Infinity", f64::NEG_INFINITY),
    ("inf", f64::NAN),
    ("NaN", f64::NAN),
    ("1_0", f64::NAN),
    ("+5", 5.0),
    ("-5", -5.0),
    ("--", f64::NAN),
    ("1-", f64::NAN),
    (" 12 ", 12.0),
    ("00", 0.0),
    ("10031000001211000000", 1.0031000001211e19),
    ("1e", f64::NAN),
    ("0x", f64::NAN),
    ("+.5e-2", 5e-3),
];

#[test]
fn number_to_string_matches_js() {
    for &(v, want) in NUMBERS {
        assert_eq!(number_to_string(v), want, "formatting {v:e}");
    }
}

#[test]
fn string_to_number_matches_js() {
    for &(s, want) in STRINGS {
        let got = string_to_number(s);
        assert!(
            got == want || (got.is_nan() && want.is_nan()),
            "ToNumber({s:?}) = {got}, want {want}"
        );
    }
}

#[test]
fn substr_uses_utf16_units() {
    assert_eq!(substr("SFGPU", 4, 6), "U");
    assert_eq!(substr("SFG", 4, 6), "");
    assert_eq!(utf16_len("a😀"), 3);
    assert_eq!(parse_int("7"), Some(7));
    assert_eq!(parse_int("-"), None);
}

/// `bits String(x)` pairs from Node 26: every case where the shortest
/// representation is a tie between two digit strings (ECMAScript picks the
/// even one), plus a sample of ordinary values.
const NUMBER_VECTORS: &str = include_str!("../../tests/data/v8_numbers.txt");

#[test]
fn number_to_string_matches_v8_including_ties() {
    let mut checked = 0;
    for line in NUMBER_VECTORS.lines() {
        let Some((bits, want)) = line.split_once(' ') else {
            continue;
        };
        let Ok(bits) = u64::from_str_radix(bits, 16) else {
            continue;
        };
        assert_eq!(
            number_to_string(f64::from_bits(bits)),
            want,
            "bits {bits:016x}"
        );
        checked += 1;
    }
    assert!(checked > 4000, "only {checked} vectors");
}

#[test]
fn radix_literals_round_once_like_javascript() {
    // (input, bits of JavaScript Number(input)), from Node 26.
    let cases: [(&str, u64); 25] = [
        ("0x2b0350ba06c7298", 0x438581a85d036395),
        ("0x1fffffffffffff", 0x433fffffffffffff),
        ("0x20000000000001", 0x4340000000000000),
        ("0x20000000000003", 0x4340000000000002),
        ("0x20000000000002", 0x4340000000000001),
        ("0x40000000000005", 0x4350000000000001),
        ("0xfffffffffffff800", 0x43efffffffffffff),
        ("0xfffffffffffffc00", 0x43f0000000000000),
        ("0xfffffffffffff7ff", 0x43efffffffffffff),
        ("0x1000000000000081", 0x43b0000000000001),
        ("0x1000000000000080", 0x43b0000000000000),
        ("0x100000000000008000000001", 0x45b0000000000001),
        ("0o777777777777777777777", 0x43e0000000000000),
        ("0O1234567012345670123456701", 0x4474e5dc14e5dc15),
        (
            "0b11111111111111111111111111111111111111111111111111111",
            0x433fffffffffffff,
        ),
        (
            "0b100000000000000000000000000000000000000000000000000001",
            0x4340000000000000,
        ),
        (
            "0b1000000000000000000000000000000000000000000000000000011",
            0x4350000000000001,
        ),
        (
            "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            0x7ff0000000000000,
        ),
        (
            "0xfffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            0x7fb0000000000000,
        ),
        ("0x0000000000000000000abc", 0x40a5780000000000),
        ("0xABCDEF0123456789abcdef", 0x456579bde02468ad),
        ("0x1", 0x3ff0000000000000),
        ("0x0", 0x0000000000000000),
        ("0xg", 0x7ff8000000000000),
        ("0x", 0x7ff8000000000000),
    ];
    for (input, bits) in cases {
        let got = super::string_to_number(input);
        assert!(
            got.to_bits() == bits || (got.is_nan() && f64::from_bits(bits).is_nan()),
            "{input}: {got:e} != {:e}",
            f64::from_bits(bits)
        );
    }
}
