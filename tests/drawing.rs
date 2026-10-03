//! The flat drawing view matches the SVG output and resolves transforms,
//! inherited paint and clips.

use milsymbol::drawing::{
    Baseline, FontWeight, LineCap, LineJoin, Paint, Shape, TextAnchor, Transform,
};
use milsymbol::ir::{Node, Paint as IrPaint, Point, Segment};
use milsymbol::{PartOutput, PartialBBox, Renderer, SymbolPart, SymbolState};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const INFANTRY: &str = "10031000161211000000";

struct Nodes(Vec<Node>);

impl SymbolPart for Nodes {
    fn draw(&self, _: &SymbolState<'_>) -> Result<PartOutput, milsymbol::PartError> {
        Ok(PartOutput::new(
            vec![],
            self.0.clone(),
            PartialBBox::default(),
        ))
    }
}

/// Draws only `nodes`, so the drawing holds exactly their items.
fn drawing_of(nodes: Vec<Node>) -> Result<milsymbol::drawing::Drawing, Box<dyn std::error::Error>> {
    let renderer = Renderer::builder()
        .pipeline(&[])
        .symbol_part(Nodes(nodes))
        .build();
    Ok(renderer.symbol(INFANTRY).render()?.drawing())
}

fn close(a: Point, b: Point) -> bool {
    (a.x - b.x).abs() < 1e-9 && (a.y - b.y).abs() < 1e-9
}

fn first_point(segments: &[Segment]) -> Option<Point> {
    match segments.first() {
        Some(Segment::MoveTo(p) | Segment::LineTo(p)) => Some(*p),
        _ => None,
    }
}

#[test]
fn item_counts_match_the_svg_elements() -> TestResult {
    for sidc in [
        INFANTRY,
        "10061500331101000000",
        "SFGPUCFRM---",
        "10031002181211000000",
    ] {
        let symbol = Renderer::builder()
            .build()
            .symbol(sidc)
            .direction(45.0)
            .text(milsymbol::options::TextField::UniqueDesignation, "A")
            .render()?;
        let svg = symbol.to_svg();
        let drawing = symbol.drawing();
        let count = |f: fn(&Shape) -> bool| drawing.items.iter().filter(|i| f(&i.shape)).count();
        let clip_paths = svg.matches("clip-rule").count();
        assert_eq!(
            count(|s| matches!(s, Shape::Path(_))),
            svg.matches("<path").count() - clip_paths,
            "{sidc}"
        );
        assert_eq!(
            count(|s| matches!(s, Shape::Circle { .. })),
            svg.matches("<circle").count(),
            "{sidc}"
        );
        assert_eq!(
            count(|s| matches!(s, Shape::Text(_))),
            svg.matches("<text").count(),
            "{sidc}"
        );
        assert_eq!(drawing.invalid_paths, 0);
        assert_eq!(drawing.omitted_raw_svg, 0);
    }
    Ok(())
}

#[test]
fn view_box_and_size_are_the_svg_header() -> TestResult {
    let symbol = Renderer::default().symbol(INFANTRY).size(60.0).render()?;
    let d = symbol.drawing();
    let svg = symbol.to_svg();
    let vb = format!(
        "viewBox=\"{} {} {} {}\"",
        d.view_box.x, d.view_box.y, d.view_box.width, d.view_box.height
    );
    assert!(svg.contains(&vb), "{vb} in {svg}");
    assert_eq!(d.size, symbol.size());
    assert_eq!(d.anchor, symbol.anchor());
    Ok(())
}

#[test]
fn transforms_compose_outer_first() -> TestResult {
    let path = Node::path("M10,0 L20,0");
    let nodes = vec![Node::translate(
        100.0,
        50.0,
        vec![Node::rotate(
            90.0,
            0.0,
            0.0,
            vec![Node::scale(2.0, vec![path])],
        )],
    )];
    let d = drawing_of(nodes)?;
    let item = d.items.first().ok_or("no item")?;
    let Shape::Path(segments) = &item.shape else {
        return Err("not a path".into());
    };
    // (10,0) scaled to (20,0), rotated a quarter turn to (0,20), moved to (100,70).
    let p = item.transform.apply(first_point(segments).ok_or("empty")?);
    assert!(close(p, Point { x: 100.0, y: 70.0 }), "{p:?}");
    Ok(())
}

#[test]
fn rotation_about_a_point_fixes_that_point() {
    let t = Transform::rotate(37.0, 30.0, 40.0);
    assert!(close(
        t.apply(Point { x: 30.0, y: 40.0 }),
        Point { x: 30.0, y: 40.0 }
    ));
    let q = Transform::translate(5.0, 6.0).then(Transform::scale(2.0));
    assert!(close(
        q.apply(Point { x: 1.0, y: 1.0 }),
        Point { x: 7.0, y: 8.0 }
    ));
}

fn styled(mut n: Node, f: impl FnOnce(&mut milsymbol::ir::Style)) -> Node {
    if let Some(st) = n.style_mut() {
        f(st);
    }
    n
}

#[test]
fn group_paint_is_inherited_and_overridden() -> TestResult {
    let group = styled(
        Node::translate(
            0.0,
            0.0,
            vec![
                Node::path("M0,0 L1,1"),
                styled(Node::path("M0,0 L2,2"), |st| st.fill = Some(IrPaint::None)),
            ],
        ),
        |st| {
            st.stroke = Some(IrPaint::color("blue"));
            st.stroke_width = Some(3.0.into());
            st.stroke_dasharray = Some("4,2".into());
            st.line_cap = Some("round".into());
            st.fill = Some(IrPaint::color("red"));
            st.fill_opacity = Some(0.5.into());
        },
    );
    let d = drawing_of(vec![group])?;
    let [a, b] = d.items.as_slice() else {
        return Err(format!("{} items", d.items.len()).into());
    };
    let color = |p: &Paint| match p {
        Paint::Solid(c) => Some(c.as_str().to_owned()),
        _ => None,
    };
    assert_eq!(color(&a.appearance.fill).as_deref(), Some("red"));
    assert_eq!(color(&a.appearance.stroke).as_deref(), Some("blue"));
    assert_eq!(a.appearance.stroke_width, 3.0);
    assert_eq!(a.appearance.dashes, vec![4.0, 2.0]);
    assert_eq!(a.appearance.line_cap, LineCap::Round);
    assert_eq!(a.appearance.line_join, LineJoin::Round);
    assert_eq!(a.appearance.fill_opacity, 0.5);
    assert_eq!(b.appearance.fill, Paint::None);
    assert_eq!(color(&b.appearance.stroke).as_deref(), Some("blue"));
    Ok(())
}

#[test]
fn unsafe_and_unset_paint_resolves_as_svg_does() -> TestResult {
    let bad = styled(Node::path("M0,0 L1,1"), |st| {
        st.fill = Some(IrPaint::color("url(#x)"));
        st.stroke = Some(IrPaint::color("red"));
        st.stroke_dasharray = Some("4;evil".into());
        st.line_cap = Some("sharp".into());
    });
    let d = drawing_of(vec![bad, Node::path("M0,0 L1,1")])?;
    let [a, b] = d.items.as_slice() else {
        return Err("items".into());
    };
    assert_eq!(a.appearance.fill, Paint::None);
    assert!(a.appearance.dashes.is_empty());
    assert_eq!(a.appearance.line_cap, LineCap::Butt);
    // SVG's initial values apply to a path that sets nothing.
    assert!(matches!(&b.appearance.fill, Paint::Solid(c) if c.as_str() == "black"));
    assert_eq!(b.appearance.stroke, Paint::None);
    assert_eq!(b.appearance.fill_opacity, 1.0);
    Ok(())
}

#[test]
fn text_attributes_are_typed() -> TestResult {
    let mut t = Node::text(10.0, 20.0, "hi");
    if let Node::Text(n) = &mut t {
        n.font_size = Some(30.0.into());
        n.font_weight = Some("bold".into());
        n.text_anchor = Some("END".into());
        n.alignment_baseline = Some("central".into());
        n.font_family = Some("bad;family".into());
    }
    let d = drawing_of(vec![t])?;
    let item = d.items.first().ok_or("none")?;
    let Shape::Text(t) = &item.shape else {
        return Err("not text".into());
    };
    assert_eq!(
        (t.position, t.font_size),
        (Point { x: 10.0, y: 20.0 }, 30.0)
    );
    assert_eq!(t.font_weight, FontWeight::Bold);
    assert_eq!(t.anchor, TextAnchor::End);
    assert_eq!(t.baseline, Some(Baseline::Central));
    assert_eq!(t.font_family, "sans-serif");
    Ok(())
}

#[test]
fn clips_are_mapped_to_symbol_units_and_nest() -> TestResult {
    let inner = Node::clip(
        milsymbol::ir::PathData::new("M0,0 L10,0 L10,10 Z"),
        None,
        vec![Node::path("M0,0 L1,1")],
    );
    let outer = Node::translate(
        100.0,
        100.0,
        vec![Node::clip(
            milsymbol::ir::PathData::new("M0,0 L50,0 L50,50 Z"),
            None,
            vec![inner],
        )],
    );
    let d = drawing_of(vec![outer])?;
    let item = d.items.first().ok_or("none")?;
    assert_eq!(item.clips.len(), 2);
    let start = item.clips.first().and_then(|c| first_point(&c.segments));
    assert!(close(start.ok_or("empty")?, Point { x: 100.0, y: 100.0 }));
    Ok(())
}

#[test]
fn malformed_paths_and_raw_svg_are_reported() -> TestResult {
    let d = drawing_of(vec![
        Node::path("M0,0 L10,10 L bogus"),
        Node::TrustedSvg("<g/>".into()),
    ])?;
    assert_eq!(d.invalid_paths, 1);
    assert_eq!(d.omitted_raw_svg, 1);
    let Some(Shape::Path(segments)) = d.items.first().map(|i| &i.shape) else {
        return Err("no path".into());
    };
    assert_eq!(segments.len(), 2, "the valid prefix is drawn");
    Ok(())
}

/// SVG readers ignore a fill or stroke that is not a CSS colour, so the
/// item keeps the paint its group gives it; `none` is no paint.
#[test]
fn paint_that_is_not_a_colour_is_inherited() -> TestResult {
    let mut group = Node::translate(0.0, 0.0, Vec::new());
    if let Some(style) = group.style_mut() {
        style.fill = Some(IrPaint::color("blue"));
        style.stroke = Some(IrPaint::color("red"));
    }
    let mut children = Vec::new();
    for (fill, stroke) in [
        ("rbg(255, 188, 1)", "\u{85}red"),
        ("none", "NONE"),
        ("#0f0", "rgb(1 2 3 / 50%)"),
    ] {
        let mut path = Node::path("M 0,0 L 10,10");
        if let Some(style) = path.style_mut() {
            style.fill = Some(IrPaint::color(fill));
            style.stroke = Some(IrPaint::color(stroke));
        }
        children.push(path);
    }
    if let Some(draw) = group.children_mut() {
        *draw = children;
    }
    let drawing = drawing_of(vec![group])?;
    fn color(p: &Paint) -> Option<&str> {
        match p {
            Paint::Solid(c) => Some(c.as_str()),
            _ => None,
        }
    }
    let paints: Vec<_> = drawing
        .items
        .iter()
        .map(|i| (color(&i.appearance.fill), color(&i.appearance.stroke)))
        .collect();
    assert_eq!(
        paints,
        [
            (Some("blue"), Some("red")),
            (None, None),
            (Some("#0f0"), Some("rgb(1 2 3 / 50%)")),
        ]
    );
    Ok(())
}
