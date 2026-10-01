use super::*;
use alloc::borrow::ToOwned;
use alloc::vec;

fn p(x: f64, y: f64) -> Point {
    Point { x, y }
}

#[test]
fn parses_relative_implicit_and_close() {
    let segs = PathData::new("M25,50 l150,0 0,100 -150,0 z")
        .segments()
        .map(alloc::borrow::Cow::into_owned)
        .unwrap_or_default();
    assert_eq!(
        segs,
        vec![
            Segment::MoveTo(p(25.0, 50.0)),
            Segment::LineTo(p(175.0, 50.0)),
            Segment::LineTo(p(175.0, 150.0)),
            Segment::LineTo(p(25.0, 150.0)),
            Segment::Close,
        ]
    );
}

#[test]
fn implicit_lineto_after_moveto_and_compact_numbers() {
    let segs = PathData::new("M 45,150 L45,70 100,20m-.5.5 1e1-2")
        .segments()
        .map(alloc::borrow::Cow::into_owned)
        .unwrap_or_default();
    assert_eq!(segs.get(2), Some(&Segment::LineTo(p(100.0, 20.0))));
    assert_eq!(segs.get(3), Some(&Segment::MoveTo(p(99.5, 20.5))));
    assert_eq!(segs.get(4), Some(&Segment::LineTo(p(109.5, 18.5))));
}

#[test]
fn smooth_curves_and_arcs_become_absolute() {
    let segs = PathData::new("M0,0 c0,10 10,10 10,0 s10,-10 10,0 h5 v5 a5,5 0 1,0 10,0")
        .segments()
        .map(alloc::borrow::Cow::into_owned)
        .unwrap_or_default();
    assert_eq!(
        segs.get(2),
        Some(&Segment::CubicTo {
            ctrl1: p(10.0, -10.0),
            ctrl2: p(20.0, -10.0),
            to: p(20.0, 0.0)
        })
    );
    assert_eq!(segs.get(3), Some(&Segment::LineTo(p(25.0, 0.0))));
    assert_eq!(segs.get(4), Some(&Segment::LineTo(p(25.0, 5.0))));
    assert!(matches!(
        segs.get(5),
        Some(Segment::ArcTo {
            large_arc: true,
            sweep: false,
            ..
        })
    ));
}

#[test]
fn invalid_data_reports_prefix() {
    let path = PathData::new("M0,0 L10,10 Lx");
    let result = path.segments();
    assert!(
        matches!(result, Err(ref e) if e.valid_prefix.len() == 2),
        "{result:?}"
    );
    assert!(PathData::new("L0,0").segments().is_err());
}

#[test]
fn missing_detection_recurses() {
    let nested = vec![Node::translate(
        0.0,
        0.0,
        vec![Node::Group(vec![Node::Missing])],
    )];
    assert!(contains_missing(&nested));
    assert!(!contains_missing(&[Node::path("M0,0")]));
}

#[test]
fn from_segments_round_trips_and_borrows() {
    let segs = vec![
        Segment::MoveTo(p(25.0, 50.0)),
        Segment::LineTo(p(175.5, 50.0)),
        Segment::QuadTo {
            ctrl: p(1.0, 2.0),
            to: p(3.0, 4.0),
        },
        Segment::CubicTo {
            ctrl1: p(1.0, 2.0),
            ctrl2: p(3.0, 4.0),
            to: p(-5.0, 6.25),
        },
        Segment::ArcTo {
            rx: 5.0,
            ry: 5.0,
            rotation: 0.0,
            large_arc: true,
            sweep: false,
            to: p(10.0, 0.0),
        },
        Segment::Close,
    ];
    let path = PathData::from_segments(segs.clone());
    assert_eq!(
        path.source(),
        "M25,50 L175.5,50 Q1,2 3,4 C1,2 3,4 -5,6.25 A5,5 0 1 0 10,0 Z"
    );
    assert!(matches!(path.segments(), Ok(alloc::borrow::Cow::Borrowed(s)) if s == segs.as_slice()));
    let reparsed = PathData::new(path.source().to_owned());
    assert_eq!(reparsed.segments().ok().as_deref(), Some(segs.as_slice()));
}

#[test]
fn caching_keeps_failing_paths_intact() {
    let mut nodes = vec![Node::path("M0,0 L10,10"), Node::path("M0,0 Lx")];
    assert!(parse_paths(&mut nodes).is_err());
    let Some(Node::Path(bad)) = nodes.get(1) else {
        return;
    };
    assert_eq!(bad.d.source(), "M0,0 Lx");
    let Some(Node::Path(good)) = nodes.first() else {
        return;
    };
    assert!(matches!(
        good.d.segments(),
        Ok(alloc::borrow::Cow::Borrowed(_))
    ));
}

#[test]
fn caching_continues_after_bad_paths_and_clip_geometry() -> Result<(), PathParseError> {
    let mut nodes = vec![
        Node::path("M0,0 Lx"),
        Node::clip(PathData::new("M,0,0"), None, vec![Node::path("M1,1")]),
        Node::translate(1.0, 2.0, vec![Node::path("M2,2")]),
        Node::path("M3,3"),
    ];
    let expected = PathData::new("M0,0 Lx").segments().err();
    assert_eq!(parse_paths(&mut nodes).err(), expected);
    fn check(nodes: &[Node]) -> Result<usize, PathParseError> {
        let mut good = 0;
        for n in nodes {
            if let Node::Path(p) = n {
                if p.d.source() != "M0,0 Lx" {
                    assert!(matches!(p.d.segments()?, Cow::Borrowed(_)));
                    good += 1;
                }
            }
            good += check(n.children().unwrap_or(&[]))?;
        }
        Ok(good)
    }
    assert_eq!(check(&nodes)?, 3);
    Ok(())
}

#[test]
fn comma_is_a_separator_between_arguments_only() {
    let err = |d: &'static str| PathData::new(d).segments().err();
    let e = err("M,0,0 L10,10");
    assert!(
        matches!(&e, Some(e) if e.offset == 1 && e.valid_prefix.is_empty()),
        "{e:?}"
    );
    let e = err("M0,0 L,10,10");
    assert!(matches!(&e, Some(e) if e.valid_prefix.len() == 1), "{e:?}");
    assert!(err("M0,0 L1,,2").is_some());
    assert!(err("M0,0 L1,1,").is_some());
    assert!(err("M0,0 Z 1,1").is_some());
    for ok in ["M 0 0, 1 1", "M0,0L1-1", "M0 0 , 1 1 z", "m1,1\t2\n3"] {
        assert!(PathData::new(ok).segments().is_ok(), "{ok}");
    }
}

#[test]
fn every_generated_path_parses() {
    use crate::generated::pool::PATHS;
    for d in PATHS.iter() {
        let parsed = PathData::new(*d).segments().map(|s| s.len());
        assert!(parsed.is_ok(), "{d}: {parsed:?}");
    }
    assert!(PATHS.len() > 900, "{}", PATHS.len());
}

#[cfg(feature = "compact-paths")]
mod compact {
    use super::*;
    use crate::generated::pool::{PATH_BYTES, PATH_OFFSETS, PATHS};
    use crate::template::path_bytes;
    use alloc::boxed::Box;
    use alloc::format;
    use alloc::string::String;

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
            let text = PathData::new(*d);
            let (a, b) = (packed.segments()?, text.segments()?);
            assert_eq!(format!("{:?}", &*a), format!("{:?}", &*b), "path {i}");
        }
        Ok(())
    }

    proptest::proptest! {
        #[test]
        fn decoding_arbitrary_bytes_never_panics(bytes in proptest::collection::vec(0u8..=255, 0..64)) {
            let mut out = String::new();
            super::super::path::codec::write_text(&bytes, &mut out).ok();
        }
    }
}
