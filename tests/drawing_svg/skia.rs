//! A renderer other than SVG, written only against [`milsymbol::drawing`]:
//! each item becomes tiny-skia geometry, paint and stroke.
//!
//! Paths are built with the calls usvg makes when it reads the SVG text
//! (`f32` coordinates, elliptical arcs as kurbo cubics with tolerance 0.1,
//! circles as four arcs), so a comparison with usvg sees the same
//! normalization on both sides and any difference is in the drawing view.

use milsymbol::drawing::{Appearance, Drawing, LineCap, LineJoin, Paint, Shape, Transform};
use milsymbol::ir::{Point, Segment};
use tiny_skia::PathBuilder;

/// A colour with the opacity it is painted with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Rgba {
    pub(crate) rgb: [u8; 3],
    pub(crate) opacity: f32,
}

/// Stroke settings as a renderer applies them.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StrokeStyle {
    pub(crate) color: Rgba,
    pub(crate) width: f32,
    /// Dash lengths, repeated to an even count as SVG requires.
    pub(crate) dashes: Vec<f32>,
    pub(crate) cap: tiny_skia::LineCap,
    pub(crate) join: tiny_skia::LineJoin,
}

/// The drawing transform as tiny-skia's.
pub(crate) fn transform(t: &Transform) -> tiny_skia::Transform {
    tiny_skia::Transform::from_row(
        t.a as f32, t.b as f32, t.c as f32, t.d as f32, t.e as f32, t.f as f32,
    )
}

/// Symbol units to pixels: the SVG viewport mapping of `view_box` onto
/// `size` (`preserveAspectRatio="xMidYMid meet"`, the SVG default).
pub(crate) fn viewport(d: &Drawing) -> Transform {
    let vb = &d.view_box;
    let s = (d.size.width / vb.width).min(d.size.height / vb.height);
    Transform {
        a: s,
        b: 0.0,
        c: 0.0,
        d: s,
        e: (d.size.width - vb.width * s) / 2.0 - vb.x * s,
        f: (d.size.height - vb.height * s) / 2.0 - vb.y * s,
    }
}

/// svgtypes' arc conversion (kurbo in `f64`, tolerance 0.1, a line when
/// kurbo finds no arc).
fn arc_to(
    b: &mut PathBuilder,
    from: (f64, f64),
    radii: (f64, f64),
    rotation_degrees: f64,
    large_arc: bool,
    sweep: bool,
    to: (f64, f64),
) {
    let arc = kurbo::SvgArc {
        from: kurbo::Point::new(from.0, from.1),
        to: kurbo::Point::new(to.0, to.1),
        radii: kurbo::Vec2::new(radii.0, radii.1),
        x_rotation: rotation_degrees.to_radians(),
        large_arc,
        sweep,
    };
    match kurbo::Arc::from_svg_arc(&arc) {
        Some(arc) => arc.to_cubic_beziers(0.1, |p1, p2, p| {
            b.cubic_to(
                p1.x as f32,
                p1.y as f32,
                p2.x as f32,
                p2.y as f32,
                p.x as f32,
                p.y as f32,
            );
        }),
        None => b.line_to(to.0 as f32, to.1 as f32),
    }
}

/// A path's segments as tiny-skia builds them from the SVG text; `None`
/// when the path is empty, as usvg then draws nothing.
pub(crate) fn path(segments: &[Segment]) -> Option<tiny_skia::Path> {
    let mut b = PathBuilder::new();
    let origin = Point { x: 0.0, y: 0.0 };
    let (mut current, mut start) = (origin, origin);
    for segment in segments {
        match *segment {
            Segment::MoveTo(p) => {
                b.move_to(p.x as f32, p.y as f32);
                (current, start) = (p, p);
            }
            Segment::LineTo(p) => {
                b.line_to(p.x as f32, p.y as f32);
                current = p;
            }
            Segment::QuadTo { ctrl, to } => {
                b.quad_to(ctrl.x as f32, ctrl.y as f32, to.x as f32, to.y as f32);
                current = to;
            }
            Segment::CubicTo { ctrl1, ctrl2, to } => {
                b.cubic_to(
                    ctrl1.x as f32,
                    ctrl1.y as f32,
                    ctrl2.x as f32,
                    ctrl2.y as f32,
                    to.x as f32,
                    to.y as f32,
                );
                current = to;
            }
            Segment::ArcTo {
                rx,
                ry,
                rotation,
                large_arc,
                sweep,
                to,
            } => {
                let from = (current.x, current.y);
                arc_to(
                    &mut b,
                    from,
                    (rx, ry),
                    rotation,
                    large_arc,
                    sweep,
                    (to.x, to.y),
                );
                current = to;
            }
            Segment::Close => {
                b.close();
                current = start;
            }
            // A segment kind this consumer does not know: draw nothing, so
            // the comparison reports the item.
            _ => return None,
        }
    }
    b.finish()
}

/// A circle as usvg converts `<circle>`: four arcs from the rightmost point,
/// in `f32`; `None` for a radius SVG does not draw.
pub(crate) fn circle(center: Point, radius: f64) -> Option<tiny_skia::Path> {
    let (cx, cy, r) = (center.x as f32, center.y as f32, radius as f32);
    if !(r.is_finite() && r > 0.0) {
        return None;
    }
    let mut b = PathBuilder::new();
    b.move_to(cx + r, cy);
    for (x, y) in [(cx, cy + r), (cx - r, cy), (cx, cy - r), (cx + r, cy)] {
        let from = b.last_point()?;
        let from = (f64::from(from.x), f64::from(from.y));
        let radii = (f64::from(r), f64::from(r));
        arc_to(
            &mut b,
            from,
            radii,
            0.0,
            false,
            true,
            (f64::from(x), f64::from(y)),
        );
    }
    b.close();
    b.finish()
}

/// The geometry of a non-text shape.
pub(crate) fn shape(shape: &Shape) -> Option<tiny_skia::Path> {
    match shape {
        Shape::Path(segments) => path(segments),
        Shape::Circle { center, radius } => circle(*center, *radius),
        _ => None,
    }
}

/// A paint's colour and opacity; `Err` holds a colour no SVG reader parses.
pub(crate) fn paint(p: &Paint, opacity: f64) -> Result<Option<Rgba>, String> {
    let Paint::Solid(c) = p else { return Ok(None) };
    let parsed = c
        .as_str()
        .parse::<svgtypes::Color>()
        .map_err(|e| format!("colour {:?} does not parse: {e}", c.as_str()))?;
    Ok(Some(Rgba {
        rgb: [parsed.red, parsed.green, parsed.blue],
        opacity: (f64::from(parsed.alpha) / 255.0 * opacity) as f32,
    }))
}

/// The fill, as SVG paints it.
pub(crate) fn fill(a: &Appearance) -> Result<Option<Rgba>, String> {
    paint(&a.fill, a.fill_opacity)
}

/// The stroke, as SVG paints it: none without paint or with a width that is
/// not positive.
pub(crate) fn stroke(a: &Appearance) -> Result<Option<StrokeStyle>, String> {
    let Some(color) = paint(&a.stroke, 1.0)? else {
        return Ok(None);
    };
    let width = a.stroke_width as f32;
    if !(width.is_finite() && width > 0.0) {
        return Ok(None);
    }
    let mut dashes: Vec<f32> = a.dashes.iter().map(|&d| d as f32).collect();
    if dashes.len() % 2 == 1 {
        dashes.extend_from_within(..);
    }
    Ok(Some(StrokeStyle {
        color,
        width,
        dashes,
        cap: match a.line_cap {
            LineCap::Round => tiny_skia::LineCap::Round,
            LineCap::Square => tiny_skia::LineCap::Square,
            _ => tiny_skia::LineCap::Butt,
        },
        join: match a.line_join {
            LineJoin::Round => tiny_skia::LineJoin::Round,
            _ => tiny_skia::LineJoin::Miter,
        },
    }))
}
