use super::*;
use crate::generated::pool::{PATH_BYTES, PATH_OFFSETS, PATHS};
use crate::ir::path::{codec, packed};
use crate::template::path_bytes;
use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;

fn leak(bytes: &[u8]) -> &'static [u8] {
    Box::leak(bytes.to_vec().into_boxed_slice())
}

#[test]
fn offsets_cover_the_blob() {
    assert_eq!(PATH_OFFSETS.len(), PATHS.len() + 1);
    assert_eq!(
        PATH_OFFSETS.last().map(|&o| o as usize),
        Some(PATH_BYTES.len())
    );
    assert!(PATH_OFFSETS.is_sorted());
}

#[test]
fn every_packed_path_decodes_to_its_text() -> Result<(), Box<dyn core::error::Error>> {
    for (i, d) in PATHS.iter().enumerate() {
        let bytes = path_bytes(i as u32).ok_or("missing path")?;
        let packed = PathData::from_packed(bytes);
        assert_eq!(packed.source(), *d, "path {i}");
        let mut streamed = String::new();
        packed.write_source(&mut streamed)?;
        assert_eq!(streamed, *d, "path {i}");
        assert_eq!(packed, PathData::new(*d), "path {i}");
        assert_eq!(PathData::new(*d), packed, "path {i}");
    }
    Ok(())
}

#[test]
fn every_packed_path_parses_to_the_text_segments_without_the_text()
-> Result<(), Box<dyn core::error::Error>> {
    for (i, d) in PATHS.iter().enumerate() {
        let bytes = path_bytes(i as u32).ok_or("missing path")?;
        let direct = packed::segments(bytes);
        let text = PathData::new(*d).segments()?.into_owned();
        let from_bytes = PathData::from_packed(bytes).segments()?.into_owned();
        assert_eq!(format!("{from_bytes:?}"), format!("{text:?}"), "path {i}");
        if codec::raw_text(bytes).is_none() {
            let direct = direct.ok_or("declined")?;
            assert_eq!(format!("{direct:?}"), format!("{text:?}"), "path {i}");
        }
    }
    Ok(())
}

#[test]
fn packed_text_needs_no_escaping() -> Result<(), Box<dyn core::error::Error>> {
    for i in 0..PATHS.len() {
        let bytes = path_bytes(i as u32).ok_or("missing path")?;
        let path = PathData::from_packed(bytes);
        if path.as_text().is_some() {
            continue;
        }
        let ok = |b: u8| b.is_ascii_alphanumeric() || b" ,.+-".contains(&b);
        assert!(path.source().bytes().all(ok), "path {i}");
        assert!(!path.contains_null(), "path {i}");
    }
    Ok(())
}

#[test]
fn raw_text_and_raw_number_entries_decode_verbatim() -> Result<(), Box<dyn core::error::Error>> {
    let text = "M0,0 L1e1,2.50 z";
    let mut raw = alloc::vec![0x80];
    raw.extend(text.bytes());
    let path = PathData::from_packed(leak(&raw));
    assert_eq!(path.source(), text);
    assert_eq!(path.as_text(), Some(text));
    assert_eq!(path.segments()?.len(), 3);
    // Header 0 (no decimals); `m`, then a raw number `-1e-5`, then `2`.
    let packed = [0x00, 0xC1, 0xD5, 5, b'-', b'1', b'e', b'-', b'5', 4];
    let path = PathData::from_packed(leak(&packed));
    assert_eq!(path.source(), "m -1e-5,2");
    assert_eq!(path, PathData::new("m -1e-5,2"));
    assert_eq!(path.segments()?, PathData::new("m -1e-5,2").segments()?);
    Ok(())
}

#[test]
fn packed_errors_match_the_text_parser() {
    // `m` followed by a comma-separated first argument is invalid text.
    let bytes = leak(&[0x00, 0xC1, 0xDA, 4, 6, 0xD6]);
    let path = PathData::from_packed(bytes);
    assert_eq!(path.source(), "m,2,3");
    assert_eq!(path.segments(), PathData::new("m,2,3").segments());
    assert!(path.segments().is_err());
}

fn debug_segments(r: Result<alloc::vec::Vec<Segment>, PathParseError>) -> String {
    format!("{r:?}")
}

proptest::proptest! {
    #[test]
    fn decoding_arbitrary_bytes_never_panics(bytes in proptest::collection::vec(0u8..=255, 0..64)) {
        let mut out = String::new();
        codec::write_text(&bytes, &mut out).ok();
    }

    #[test]
    fn packed_segments_equal_text_segments(
        header in 0u8..0x80,
        ops in proptest::collection::vec(
            proptest::sample::select(&[
                0x00u8, 0x01, 0x02, 0x03, 0x14, 0x15, 0x40, 0x7F, 0x81, 0x80,
                0xC0, 0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7, 0xC8, 0xC9, 0xCA, 0xCB,
                0xCC, 0xCD, 0xCE, 0xCF, 0xD0, 0xD1, 0xD2, 0xD3, 0xD4, 0xD6,
                0xD8, 0xD9, 0xDA, 0xDB, 0xDC,
            ][..]),
            0..40,
        ),
    ) {
        let mut bytes = alloc::vec![header];
        bytes.extend(ops);
        if let Some(direct) = packed::segments(&bytes) {
            let mut text = String::new();
            codec::write_text(&bytes, &mut text).ok();
            let expected = PathData::new(text).segments().map(|s| s.into_owned());
            proptest::prop_assert_eq!(debug_segments(Ok(direct)), debug_segments(expected));
        }
    }
}

fn text_of(bytes: &[u8]) -> String {
    let mut text = String::new();
    codec::write_text(bytes, &mut text).ok();
    text
}

fn text_segments(bytes: &[u8]) -> Result<Vec<Segment>, PathParseError> {
    PathData::new(text_of(bytes))
        .segments()
        .map(|s| s.into_owned())
}

/// The packed lexer must decline `bytes`, and the text must fail to parse.
fn declined(bytes: &[u8]) {
    assert!(packed::segments(bytes).is_none(), "{bytes:?}");
    assert!(text_segments(bytes).is_err(), "{:?}", text_of(bytes));
}

/// The packed lexer must accept `bytes` and agree with the text parse bit for
/// bit.
fn accepted(bytes: &[u8]) {
    let direct = packed::segments(bytes);
    let text = text_segments(bytes).ok();
    assert!(direct.is_some(), "{:?}", text_of(bytes));
    assert_eq!(
        direct.map(|s| format!("{s:?}")),
        text.map(|s| format!("{s:?}")),
        "{:?}",
        text_of(bytes)
    );
}

const M: u8 = 0xC7;
const L: u8 = 0xC6;
const ARC: u8 = 0xCD;
const TAIL: u8 = 0xD6;
const NO_SEP: u8 = 0xD8;
const COMMA: u8 = 0xDA;
const WIDE: u8 = 0xDB;

#[test]
fn numbers_that_would_merge_in_text_are_declined() {
    // `M 1` then `2` with no separator reads as the single number 12.
    declined(&[0x00, M, 0x02, NO_SEP, 0x04, TAIL]);
    // The same for `0`: `10`.
    declined(&[0x00, M, 0x02, NO_SEP, 0x00, TAIL]);
}

#[test]
fn a_leading_minus_needs_no_separator() {
    // `M 1-1`, `M 1-0` (negative zero) and a raw `-2`.
    accepted(&[0x00, M, 0x02, NO_SEP, 0x01, TAIL]);
    accepted(&[0x00, M, 0x02, NO_SEP, 0xD4, TAIL]);
    accepted(&[0x00, M, 0x02, NO_SEP, 0xD5, 2, b'-', b'2', TAIL]);
    let bytes = [0x00, M, 0x02, NO_SEP, 0xD4, TAIL];
    let segments = packed::segments(&bytes);
    let y = segments.as_deref().and_then(<[Segment]>::first);
    assert!(matches!(y, Some(Segment::MoveTo(p)) if p.y.is_sign_negative() && p.y == 0.0));
}

#[test]
fn arc_flags_are_zero_or_one_only() {
    // `M 0 0 A 1 1 0 1 0 5 5`
    let arc = [
        0x00, M, 0x00, 0x00, ARC, 0x02, 0x02, 0x00, 0x02, 0x00, 0x0A, 0x0A, TAIL,
    ];
    accepted(&arc);
    // Raw flag texts `1` and `0`.
    accepted(&[
        0x00, M, 0x00, 0x00, ARC, 0x02, 0x02, 0x00, 0xD5, 1, b'1', 0xD5, 1, b'0', 0x0A, 0x0A, TAIL,
    ]);
    // A flag of 2 (also at one decimal place, where `1` is stored as 10).
    declined(&[
        0x00, M, 0x00, 0x00, ARC, 0x02, 0x02, 0x00, 0x04, 0x00, 0x0A, 0x0A, TAIL,
    ]);
    declined(&[
        0x10, M, 0x00, 0x00, ARC, 0x14, 0x14, 0x00, 0x28, 0x00, 0x14, 0x14, TAIL,
    ]);
    // Negative zero is not a flag.
    declined(&[
        0x00, M, 0x00, 0x00, ARC, 0x02, 0x02, 0x00, 0xD4, 0x00, 0x0A, 0x0A, TAIL,
    ]);
    // Raw `2` is not a flag.
    declined(&[
        0x00, M, 0x00, 0x00, ARC, 0x02, 0x02, 0x00, 0xD5, 1, b'2', 0x00, 0x0A, 0x0A, TAIL,
    ]);
}

#[test]
fn only_blank_separators_may_precede_commands_and_the_end() {
    declined(&[0x00, M, 0x00, 0x00, COMMA, L, 0x02, 0x02, TAIL]);
    declined(&[0x00, M, 0x00, 0x00, COMMA, TAIL]);
    accepted(&[0x00, M, 0x00, 0x00, WIDE, L, 0x02, 0x02, TAIL]);
    accepted(&[0x00, M, 0x00, 0x00, WIDE, TAIL]);
}

#[test]
fn a_path_starts_with_a_move() {
    declined(&[0x00, L, 0x02, 0x02, TAIL]);
    accepted(&[0x00, 0xC1, 0x02, 0x02, TAIL]);
    accepted(&[0x00, M, 0x02, 0x02, TAIL]);
}

#[test]
fn text_after_an_invalid_opcode_is_not_part_of_the_path() {
    // `0xF0` is invalid: the text ends after `M 0 0`.
    let bytes = [0x00, 0xC1, 0x00, 0x00, 0xF0, 0xC3, 0x00, 0x00, 0x00];
    assert_eq!(text_of(&bytes), "m 0,0");
    let text = text_segments(&bytes).map(|s| format!("{s:?}"));
    assert_eq!(
        packed::segments(&bytes).map(|s| format!("{s:?}")).map(Ok),
        Some(text)
    );
    let leaked = PathData::from_packed(leak(&bytes));
    assert_eq!(leaked.segments().map(|s| s.len()), Ok(1));
}

#[test]
fn equality_compares_the_whole_text() {
    let zero = PathData::from_packed(leak(&[0x00, M, 0x00, 0x00, TAIL]));
    let one = PathData::from_packed(leak(&[0x00, M, 0x02, 0x00, TAIL]));
    assert_eq!(zero.source(), "M 0,0");
    assert_ne!(zero, one);
    assert_eq!(
        zero,
        PathData::from_packed(leak(&[0x00, M, 0x00, 0x00, TAIL]))
    );
    assert_ne!(zero, PathData::new("M 1,0"));
    assert_ne!(PathData::new("M 1,0"), zero);
    for other in ["M 0", "M 0,0 ", "M 0,0 L", "M 0,00", "M 0,", ""] {
        assert_ne!(zero, PathData::new(other), "{other:?}");
        assert_ne!(PathData::new(other), zero, "{other:?}");
    }
    assert_eq!(zero, PathData::new("M 0,0"));
}

#[test]
fn packed_and_text_paths_agree_on_arbitrary_streams() {
    use proptest::test_runner::{Config, TestRunner};
    let mut runner = TestRunner::new(Config::with_cases(20_000));
    let result = runner.run(&stream(), |bytes| check(leak(&bytes)));
    assert_eq!(result, Ok(()));
}

const RAWS: &[&str] = &[
    "1e5", "1E+2", "-1e-3", ".5", "5.", "+3", "-.5", "00", "01", "0", "1", "-0", "1e400", "1.e5",
    "-1.5E3", "1e", "1-2", "1.5.5", "0.1", "-0.0", "007.50", "1 2", "", "e5", "+", "-", ".",
    "0x10", "inf", "NaN", "1,2", "<",
];

fn op() -> impl proptest::strategy::Strategy<Value = Vec<u8>> {
    use proptest::prelude::*;
    prop_oneof![
        3 => (0u8..0x80).prop_map(|b| vec![b]),
        1 => (0x80u8..0xC0, any::<u8>()).prop_map(|(a, b)| vec![a, b]),
        6 => (0xC0u8..0xD4).prop_map(|b| vec![b]),
        1 => Just(vec![0xD4]),
        2 => proptest::sample::select(RAWS).prop_map(|t| {
            let mut v = vec![0xD5, t.len() as u8];
            v.extend(t.bytes());
            v
        }),
        1 => Just(vec![0xD6]),
        2 => (0xD8u8..0xDD).prop_map(|b| vec![b]),
        1 => (0xE0u8..0xF0).prop_map(|b| vec![b, 0, 0]),
    ]
}

fn stream() -> impl proptest::strategy::Strategy<Value = Vec<u8>> {
    use proptest::prelude::*;
    (0u8..0x80, proptest::collection::vec(op(), 0..30)).prop_map(|(header, ops)| {
        let mut bytes = alloc::vec![header];
        bytes.extend(ops.into_iter().flatten());
        bytes
    })
}

fn check(bytes: &'static [u8]) -> Result<(), proptest::test_runner::TestCaseError> {
    use proptest::prelude::*;
    let text = text_of(bytes);
    let packed_path = PathData::from_packed(bytes);
    let text_path = PathData::new(text.clone());
    prop_assert_eq!(&*packed_path.source(), text.as_str());
    prop_assert_eq!(&packed_path, &text_path);
    prop_assert_eq!(&text_path, &packed_path);
    prop_assert_eq!(
        format!("{:?}", packed_path.segments()),
        format!("{:?}", text_path.segments())
    );
    prop_assert_eq!(packed_path.contains_null(), text_path.contains_null());
    if codec::raw_text(bytes).is_none() {
        if let Some(direct) = packed::segments(bytes) {
            let expected = text_path
                .segments()
                .map(|s| format!("{:?}", s.into_owned()));
            prop_assert_eq!(Ok(format!("{direct:?}")), expected.map_err(|_| ()));
        }
    }
    Ok(())
}

#[test]
fn the_shared_grammar_matches_the_text_only_parser() {
    for d in PATHS.iter() {
        let shared = PathData::new(*d).segments().map(|s| s.into_owned());
        assert_eq!(
            format!("{shared:?}"),
            format!("{:?}", crate::ir::path::reference_segments(d)),
            "{d}"
        );
    }
    use proptest::prelude::*;
    let mut runner =
        proptest::test_runner::TestRunner::new(proptest::test_runner::Config::with_cases(20_000));
    let text = proptest::collection::vec(
        proptest::sample::select(b"MmLlHhVvCcSsQqTtAaZz0123456789.,- eE+".as_slice()),
        0..60,
    )
    .prop_map(|b| String::from_utf8_lossy(&b).into_owned());
    let result = runner.run(&text, |t| {
        let shared = PathData::new(t.clone()).segments().map(|s| s.into_owned());
        let reference = crate::ir::path::reference_segments(&t);
        prop_assert_eq!(format!("{shared:?}"), format!("{reference:?}"));
        Ok(())
    });
    assert_eq!(result, Ok(()));
}

/// Packs `text` (integers, the separators ` ` and `,`, command letters) with
/// every separator forced, so the packed text equals `text`.
fn pack(text: &str) -> alloc::vec::Vec<u8> {
    let mut out = alloc::vec![0x00];
    let mut sep = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == ' ' || c == ',' {
            sep.push(c);
            continue;
        }
        let forced = codec::SEPS.iter().position(|s| *s == sep);
        out.push(codec::OP_SEP + forced.map_or(0, |i| i as u8));
        sep.clear();
        if let Some(i) = codec::LETTERS.iter().position(|&l| char::from(l) == c) {
            out.push(codec::OP_LETTER + i as u8);
            continue;
        }
        let mut digits = String::from(c);
        while let Some(d) = chars.next_if(char::is_ascii_digit) {
            digits.push(d);
        }
        let n: i32 = digits.parse().unwrap_or(0);
        let z = if n < 0 { -2 * n - 1 } else { 2 * n } as u32;
        if z < 0x80 {
            out.push(z as u8);
        } else {
            out.extend([0x80 | (z >> 8) as u8, z as u8]);
        }
    }
    out.push(codec::OP_TAIL);
    out
}

#[test]
fn smooth_curves_reflect_the_matching_command_only() {
    for text in [
        "M0,0 Q1,1 2,2 S3,3 4,4",
        "M0,0 C1,1 2,2 3,3 T4,4",
        "M0,0 L1,1 S2,2 3,3",
        "M0,0 L1,1 T2,2",
        "M0,0 C1,1 2,2 3,3 S4,4 5,5",
        "M0,0 Q1,1 2,2 T3,3",
        "M0,0 Q1,1 2,2 T3,3 T4,4",
        "M0,0 C1,1 2,2 3,3 S4,4 5,5 T6,6",
        "M0,0 C1,1 2,2 3,3 s1,1 2,2",
        "M0,0 Q1,1 2,2 t1,1",
        "M0,0 C1,1 2,2 3,3 t1,1",
        "M0,0 Q1,1 2,2 s1,1 2,2",
        "M5,5 S1,1 2,2",
        "M5,5 T1,1",
    ] {
        let bytes = pack(text);
        assert_eq!(text_of(&bytes), text);
        let reference = crate::ir::path::reference_segments(text);
        let shared = PathData::new(text).segments().map(|s| s.into_owned());
        assert_eq!(format!("{shared:?}"), format!("{reference:?}"), "{text}");
        let direct = packed::segments(&bytes);
        assert_eq!(
            direct.map(|s| format!("{s:?}")),
            reference.map(|s| format!("{s:?}")).ok(),
            "{text}"
        );
    }
}
