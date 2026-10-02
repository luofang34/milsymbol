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
