//! Item-by-item comparison of `Symbol::drawing` with the SVG read by usvg.

use super::model::{self, SvgItem};
use super::skia::{self, Rgba, StrokeStyle};
use milsymbol::Symbol;
use milsymbol::drawing::{DrawItem, Drawing, Shape};

/// Relative tolerance for `f32` values that both sides compute from the
/// same `f64` numbers in a different order.
const TOLERANCE: f32 = 1e-4;

/// What one comparison covered.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct Coverage {
    pub(crate) paths: usize,
    pub(crate) points: usize,
    pub(crate) clipped: usize,
    pub(crate) dashed: usize,
    /// Largest relative difference seen in any compared number.
    pub(crate) max_deviation: f32,
}

impl Coverage {
    pub(crate) fn add(&mut self, other: Coverage) {
        self.paths += other.paths;
        self.points += other.points;
        self.clipped += other.clipped;
        self.dashed += other.dashed;
        self.max_deviation = self.max_deviation.max(other.max_deviation);
    }

    fn near(&mut self, what: &str, a: f32, b: f32) -> Result<(), String> {
        let deviation = (a - b).abs() / (1.0 + a.abs().max(b.abs()));
        if deviation.is_nan() || deviation > TOLERANCE {
            return Err(format!("{what}: drawing {a} != svg {b}"));
        }
        self.max_deviation = self.max_deviation.max(deviation);
        Ok(())
    }
}

/// The drawing item as the SVG should show it; `None` when SVG draws
/// nothing for it (no geometry, or neither fill nor stroke).
fn expected(d: &Drawing, item: &DrawItem) -> Result<Option<SvgItem>, String> {
    let Some(path) = skia::shape(&item.shape) else {
        return Ok(None);
    };
    let fill = skia::fill(&item.appearance)?;
    let stroke = skia::stroke(&item.appearance)?;
    if path.len() < 2 || (fill.is_none() && stroke.is_none()) {
        return Ok(None);
    }
    let view = skia::viewport(d);
    let to_pixels = skia::transform(&view);
    let clips = item
        .clips
        .iter()
        .map(|c| skia::path(&c.segments).and_then(|p| p.transform(to_pixels)))
        .collect::<Option<Vec<_>>>()
        .ok_or("a clip region has no geometry")?;
    Ok(Some(SvgItem {
        path,
        transform: skia::transform(&view.then(item.transform)),
        fill,
        stroke,
        clips,
    }))
}

fn transform(
    c: &mut Coverage,
    a: tiny_skia::Transform,
    b: tiny_skia::Transform,
) -> Result<(), String> {
    for (name, x, y) in [
        ("sx", a.sx, b.sx),
        ("ky", a.ky, b.ky),
        ("kx", a.kx, b.kx),
        ("sy", a.sy, b.sy),
        ("tx", a.tx, b.tx),
        ("ty", a.ty, b.ty),
    ] {
        c.near(&format!("transform {name}"), x, y)?;
    }
    Ok(())
}

fn path(
    c: &mut Coverage,
    what: &str,
    a: &tiny_skia::Path,
    b: &tiny_skia::Path,
) -> Result<(), String> {
    if a.verbs() != b.verbs() {
        return Err(format!(
            "{what}: verbs {:?} != svg {:?}",
            a.verbs(),
            b.verbs()
        ));
    }
    for (i, (p, q)) in a.points().iter().zip(b.points()).enumerate() {
        c.near(&format!("{what} point {i} x"), p.x, q.x)?;
        c.near(&format!("{what} point {i} y"), p.y, q.y)?;
    }
    c.points += a.points().len();
    Ok(())
}

fn color(c: &mut Coverage, what: &str, a: Option<Rgba>, b: Option<Rgba>) -> Result<(), String> {
    match (a, b) {
        (None, None) => Ok(()),
        (Some(a), Some(b)) if a.rgb == b.rgb => {
            c.near(&format!("{what} opacity"), a.opacity, b.opacity)
        }
        (a, b) => Err(format!("{what}: drawing {a:?} != svg {b:?}")),
    }
}

fn stroke(
    c: &mut Coverage,
    a: Option<&StrokeStyle>,
    b: Option<&StrokeStyle>,
) -> Result<(), String> {
    match (a, b) {
        (None, None) => Ok(()),
        (Some(a), Some(b)) => {
            color(c, "stroke", Some(a.color), Some(b.color))?;
            c.near("stroke width", a.width, b.width)?;
            if (a.cap, a.join) != (b.cap, b.join) || a.dashes.len() != b.dashes.len() {
                return Err(format!("stroke: drawing {a:?} != svg {b:?}"));
            }
            for (x, y) in a.dashes.iter().zip(&b.dashes) {
                c.near("dash", *x, *y)?;
            }
            c.dashed += usize::from(!a.dashes.is_empty());
            Ok(())
        }
        (a, b) => Err(format!("stroke: drawing {a:?} != svg {b:?}")),
    }
}

fn item(c: &mut Coverage, a: &SvgItem, b: &SvgItem) -> Result<(), String> {
    path(c, "path", &a.path, &b.path)?;
    transform(c, a.transform, b.transform)?;
    color(c, "fill", a.fill, b.fill)?;
    stroke(c, a.stroke.as_ref(), b.stroke.as_ref())?;
    if a.clips.len() != b.clips.len() {
        return Err(format!(
            "{} clip regions != svg {}",
            a.clips.len(),
            b.clips.len()
        ));
    }
    for (x, y) in a.clips.iter().zip(&b.clips) {
        path(c, "clip", x, y)?;
    }
    c.clipped += usize::from(!a.clips.is_empty());
    c.paths += 1;
    Ok(())
}

/// Compares every visible path of `svg`, the symbol's SVG text, and the
/// document size; text is left to [`super::text`]. Returns what was
/// compared, or the first difference.
pub(crate) fn symbol(symbol: &Symbol, svg: &str) -> Result<(Coverage, model::SvgModel), String> {
    let drawing = symbol.drawing();
    let svg = model::parse(svg)?;
    let mut c = Coverage::default();
    let size = svg.tree.size();
    c.near("width", drawing.size.width as f32, size.width())?;
    c.near("height", drawing.size.height as f32, size.height())?;
    let mut ours = Vec::new();
    for d in drawing
        .items
        .iter()
        .filter(|i| !matches!(i.shape, Shape::Text(_)))
    {
        ours.extend(expected(&drawing, d)?);
    }
    if ours.len() != svg.items.len() {
        return Err(format!(
            "{} visible drawing paths != {} visible SVG paths",
            ours.len(),
            svg.items.len()
        ));
    }
    for (i, (a, b)) in ours.iter().zip(&svg.items).enumerate() {
        item(&mut c, a, b).map_err(|e| format!("path item {i}: {e}"))?;
    }
    Ok((c, svg))
}
