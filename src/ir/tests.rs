use super::*;

fn p(x: f64, y: f64) -> Point {
    Point { x, y }
}

#[test]
fn parses_relative_implicit_and_close() {
    let segs = PathData::new("M25,50 l150,0 0,100 -150,0 z")
        .segments()
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
        .unwrap_or_default();
    assert_eq!(segs.get(2), Some(&Segment::LineTo(p(100.0, 20.0))));
    assert_eq!(segs.get(3), Some(&Segment::MoveTo(p(99.5, 20.5))));
    assert_eq!(segs.get(4), Some(&Segment::LineTo(p(109.5, 18.5))));
}

#[test]
fn smooth_curves_and_arcs_become_absolute() {
    let segs = PathData::new("M0,0 c0,10 10,10 10,0 s10,-10 10,0 h5 v5 a5,5 0 1,0 10,0")
        .segments()
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
    let result = PathData::new("M0,0 L10,10 Lx").segments();
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
