//! Text items of the drawing view against the `<text>` elements of the SVG,
//! read with an XML parser: position, transform, font, alignment, content
//! and the inherited fill and stroke.

use super::skia::{self, Rgba};
use milsymbol::drawing::{Baseline, DrawItem, FontWeight, Shape, Text, TextAnchor, Transform};
use roxmltree::Node;

/// Relative tolerance for the transform, whose rotation terms the two
/// sides compute with different sine implementations.
const TOLERANCE: f64 = 1e-12;

fn near(what: &str, a: f64, b: f64) -> Result<(), String> {
    if (a - b).abs() <= TOLERANCE * (1.0 + a.abs().max(b.abs())) {
        Ok(())
    } else {
        Err(format!("{what}: drawing {a} != svg {b}"))
    }
}

fn number(node: Node<'_, '_>, name: &str) -> Result<f64, String> {
    let v = node
        .attribute(name)
        .ok_or_else(|| format!("<text> without {name}"))?;
    v.parse().map_err(|e| format!("{name}={v:?}: {e}"))
}

/// The element's transform to symbol units: its own and its ancestors'
/// `transform` attributes, outermost applied last.
fn ctm(node: Node<'_, '_>) -> Result<Transform, String> {
    let mut m = Transform::IDENTITY;
    for n in node.ancestors().filter(Node::is_element) {
        if let Some(t) = n.attribute("transform") {
            let t: svgtypes::Transform = t.parse().map_err(|e| format!("transform {t:?}: {e}"))?;
            let own = Transform {
                a: t.a,
                b: t.b,
                c: t.c,
                d: t.d,
                e: t.e,
                f: t.f,
            };
            m = own.then(m);
        }
    }
    Ok(m)
}

/// The paint SVG gives `node` for `name`: the nearest ancestor-or-self
/// value that parses, since a value that does not parse is ignored and the
/// property inherits.
fn paint(
    node: Node<'_, '_>,
    name: &str,
    default: &str,
    opacity: f64,
) -> Result<Option<Rgba>, String> {
    let value = node
        .ancestors()
        .filter_map(|n| n.attribute(name))
        .map(svgtypes::Paint::from_str)
        .find_map(Result::ok);
    match value.unwrap_or(if default == "none" {
        svgtypes::Paint::None
    } else {
        svgtypes::Paint::Color(svgtypes::Color::black())
    }) {
        svgtypes::Paint::None => Ok(None),
        svgtypes::Paint::Color(c) => Ok(Some(Rgba {
            rgb: [c.red, c.green, c.blue],
            opacity: (f64::from(c.alpha) / 255.0 * opacity) as f32,
        })),
        other => Err(format!("{name} reads as {other:?}")),
    }
}

/// The inherited value of a presentation attribute.
fn inherited<'a>(node: Node<'a, '_>, name: &str) -> Option<&'a str> {
    node.ancestors().find_map(|n| n.attribute(name))
}

fn weight(w: FontWeight) -> Option<String> {
    Some(match w {
        FontWeight::Normal => return None,
        FontWeight::Bold => "bold".into(),
        FontWeight::Bolder => "bolder".into(),
        FontWeight::Lighter => "lighter".into(),
        FontWeight::Number(n) => n.to_string(),
        _ => "unknown".into(),
    })
}

fn baseline(b: Option<Baseline>) -> Option<String> {
    b.map(|b| {
        format!("{b:?}").chars().fold(String::new(), |mut s, c| {
            if c.is_uppercase() && !s.is_empty() {
                s.push('-');
            }
            s.push(c.to_ascii_lowercase());
            s
        })
    })
}

fn anchor(a: TextAnchor) -> &'static str {
    match a {
        TextAnchor::Middle => "middle",
        TextAnchor::End => "end",
        _ => "start",
    }
}

fn attributes(t: &Text, node: Node<'_, '_>) -> Result<(), String> {
    near("x", t.position.x, number(node, "x")?)?;
    near("y", t.position.y, number(node, "y")?)?;
    near("font-size", t.font_size, number(node, "font-size")?)?;
    let svg = (
        node.attribute("font-family"),
        node.attribute("text-anchor").unwrap_or("start"),
        node.attribute("font-weight").map(String::from),
        node.attribute("dominant-baseline").map(String::from),
        node.text().unwrap_or(""),
    );
    let ours = (
        Some(t.font_family.as_str()),
        anchor(t.anchor),
        weight(t.font_weight),
        baseline(t.baseline),
        t.content.as_str(),
    );
    if ours == svg {
        Ok(())
    } else {
        Err(format!("text attributes: drawing {ours:?} != svg {svg:?}"))
    }
}

fn item(d: &DrawItem, t: &Text, node: Node<'_, '_>) -> Result<(), String> {
    attributes(t, node)?;
    let m = ctm(node)?;
    let o = d.transform;
    for (name, a, b) in [
        ("a", o.a, m.a),
        ("b", o.b, m.b),
        ("c", o.c, m.c),
        ("d", o.d, m.d),
        ("e", o.e, m.e),
        ("f", o.f, m.f),
    ] {
        near(&format!("transform {name}"), a, b)?;
    }
    let fill_opacity = inherited(node, "fill-opacity").map_or(Ok(1.0), str::parse::<f64>);
    let fill_opacity = fill_opacity
        .map_err(|e| format!("fill-opacity: {e}"))?
        .clamp(0.0, 1.0);
    let fill = paint(node, "fill", "black", fill_opacity)?;
    let stroke = paint(node, "stroke", "none", 1.0)?;
    let ours = (
        skia::fill(&d.appearance)?,
        skia::paint(&d.appearance.stroke, 1.0)?,
    );
    if ours == (fill, stroke) {
        Ok(())
    } else {
        Err(format!(
            "text paint: drawing {ours:?} != svg {:?}",
            (fill, stroke)
        ))
    }
}

/// Compares every text item with the SVG's `<text>` elements, in order.
/// Returns how many were compared.
pub(crate) fn compare(items: &[DrawItem], svg: &str) -> Result<usize, String> {
    let doc = roxmltree::Document::parse(svg).map_err(|e| format!("XML: {e}"))?;
    let nodes: Vec<_> = doc
        .descendants()
        .filter(|n| n.has_tag_name("text"))
        .collect();
    let texts: Vec<_> = items
        .iter()
        .filter_map(|i| match &i.shape {
            Shape::Text(t) => Some((i, t)),
            _ => None,
        })
        .collect();
    if texts.len() != nodes.len() {
        return Err(format!(
            "{} text items != {} <text> elements",
            texts.len(),
            nodes.len()
        ));
    }
    for (i, ((d, t), node)) in texts.iter().zip(&nodes).enumerate() {
        item(d, t, *node).map_err(|e| format!("text item {i}: {e}"))?;
    }
    Ok(texts.len())
}
