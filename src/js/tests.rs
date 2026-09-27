#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::float_cmp
)]
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
