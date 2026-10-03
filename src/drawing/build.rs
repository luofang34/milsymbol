//! Flattening the instruction tree into [`Drawing`] items.

use super::{
    Appearance, Baseline, ClipRegion, DrawItem, Drawing, FontWeight, LineCap, LineJoin, Paint,
    Shape, Text, TextAnchor, Transform, ViewBox,
};
use crate::ir::{Node, Num, Point, Segment, Style};
use crate::options::Color;
use crate::svg::sanitize;
use crate::symbol::Symbol;
use alloc::string::String;
use alloc::vec::Vec;

/// SVG's initial values, and what a group's presentation attributes hand to
/// its children.
#[derive(Clone)]
struct Inherited {
    appearance: Appearance,
    transform: Transform,
    clips: Vec<ClipRegion>,
}

impl Inherited {
    fn initial() -> Self {
        Inherited {
            appearance: Appearance {
                fill: Paint::Solid(Color::from_static("black")),
                fill_opacity: 1.0,
                stroke: Paint::None,
                stroke_width: 1.0,
                dashes: Vec::new(),
                line_cap: LineCap::Butt,
                line_join: LineJoin::Miter,
            },
            transform: Transform::IDENTITY,
            clips: Vec::new(),
        }
    }
}

struct Builder {
    /// The symbol's stroke width, used where an item sets none.
    stroke_width: f64,
    style_fill: bool,
    items: Vec<DrawItem>,
    omitted_raw_svg: usize,
    invalid_paths: usize,
}

fn finite(v: f64, fallback: f64) -> f64 {
    if v.is_finite() { v } else { fallback }
}

/// The paint an SVG reader takes from the `fill` or `stroke` attribute the
/// SVG writer emits for `p`. `None` when the written value is not a CSS
/// colour: readers ignore such a value, so the inherited paint stays.
fn paint(p: &crate::ir::Paint) -> Option<Paint> {
    let crate::ir::Paint::Color(c) = p else {
        return Some(Paint::None);
    };
    let Some(written) = sanitize::sanitize_color(c) else {
        return Some(Paint::None);
    };
    if written.eq_ignore_ascii_case("none") {
        return Some(Paint::None);
    }
    if !super::css::is_color(written) {
        return None;
    }
    Color::new(String::from(written)).ok().map(Paint::Solid)
}

/// Dash lengths as SVG reads them: an invalid list draws a solid line.
fn dashes(text: &str) -> Vec<f64> {
    let mut out = Vec::new();
    let tokens = text.split(|c: char| c == ',' || c.is_whitespace());
    for t in tokens.filter(|t| !t.is_empty()) {
        match t.parse::<f64>() {
            Ok(v) if v.is_finite() && v >= 0.0 => out.push(v),
            _ => return Vec::new(),
        }
    }
    if out.iter().sum::<f64>() > 0.0 {
        out
    } else {
        Vec::new()
    }
}

impl Builder {
    fn style(&self, ctx: &mut Inherited, st: &Style) {
        let a = &mut ctx.appearance;
        if let Some(stroke) = &st.stroke {
            let scale = finite(st.non_scaling_stroke.unwrap_or(1.0), 1.0);
            let setting = st
                .stroke_width
                .as_ref()
                .map_or(self.stroke_width, |w| finite(w.value(), self.stroke_width));
            a.stroke_width = finite(scale * setting, self.stroke_width);
            if let Some(d) = st
                .stroke_dasharray
                .as_deref()
                .and_then(sanitize::sanitize_dash_array)
            {
                a.dashes = dashes(d);
            }
            if let Some(cap) = st.line_cap.as_deref().and_then(sanitize::sanitize_line_cap) {
                a.line_cap = match cap {
                    "round" => LineCap::Round,
                    "square" => LineCap::Square,
                    _ => LineCap::Butt,
                };
                a.line_join = if cap == "round" {
                    LineJoin::Round
                } else {
                    LineJoin::Miter
                };
            }
            if let Some(p) = paint(stroke) {
                a.stroke = p;
            }
        }
        if let Some(fill) = &st.fill {
            let resolved = if st.style_fill == Some(true) && self.style_fill {
                Some(Paint::Solid(Color::from_static("rgba(255,255,255,0.4)")))
            } else {
                paint(fill)
            };
            if let Some(p) = resolved {
                a.fill = p;
            }
        }
        if let Some(op) = &st.fill_opacity {
            a.fill_opacity = finite(op.value(), 1.0).clamp(0.0, 1.0);
        }
    }

    fn segments(&mut self, d: &crate::ir::PathData) -> Vec<Segment> {
        match d.segments() {
            Ok(s) => s.into_owned(),
            Err(e) => {
                self.invalid_paths += 1;
                e.valid_prefix
            }
        }
    }

    fn clip(&mut self, ctx: &mut Inherited, d: &crate::ir::PathData) {
        let segments = self
            .segments(d)
            .iter()
            .map(|s| ctx.transform.map_segment(s))
            .collect();
        ctx.clips.push(ClipRegion { segments });
    }

    fn push(&mut self, ctx: &Inherited, shape: Shape) {
        self.items.push(DrawItem {
            shape,
            appearance: ctx.appearance.clone(),
            transform: ctx.transform,
            clips: ctx.clips.clone(),
        });
    }

    fn list(&mut self, nodes: &[Node], ctx: &Inherited) {
        for n in nodes {
            self.node(n, ctx);
        }
    }

    fn node(&mut self, node: &Node, parent: &Inherited) {
        let style = match node {
            Node::Group(list) => return self.list(list, parent),
            Node::Missing | Node::Scalar(_) | Node::Bare(_) => return,
            Node::TrustedSvg(_) => {
                self.omitted_raw_svg += 1;
                return;
            }
            other => other.style(),
        };
        let Some(style) = style else { return };
        let mut ctx = parent.clone();
        match node {
            Node::Translate(t) => {
                let (x, y) = (finite(t.x.value(), 0.0), finite(t.y.value(), 0.0));
                ctx.transform = ctx.transform.then(Transform::translate(x, y));
            }
            Node::Rotate(r) => {
                let deg = finite(r.degree.value(), 0.0);
                let (x, y) = (finite(r.x.value(), 0.0), finite(r.y.value(), 0.0));
                ctx.transform = ctx.transform.then(Transform::rotate(deg, x, y));
            }
            Node::Scale(s) => {
                ctx.transform = ctx
                    .transform
                    .then(Transform::scale(finite(s.factor.value(), 1.0)));
            }
            _ => {}
        }
        if let (Some(clip), false) = (&style.clip_path, matches!(node, Node::Clip(_))) {
            if let Ok(d) = crate::ir::PathData::parse(clip.clone()) {
                self.clip(&mut ctx, &d);
            } else {
                self.invalid_paths += 1;
            }
        }
        if let Node::Clip(c) = node {
            self.clip(&mut ctx, &c.d);
        }
        self.style(&mut ctx, style);
        self.leaf_or_group(node, &ctx);
    }

    fn leaf_or_group(&mut self, node: &Node, ctx: &Inherited) {
        match node {
            Node::Path(p) => {
                let segments = self.segments(&p.d);
                self.push(ctx, Shape::Path(segments));
            }
            Node::Circle(c) => {
                let center = Point {
                    x: finite(c.cx.value(), 0.0),
                    y: finite(c.cy.value(), 0.0),
                };
                self.push(
                    ctx,
                    Shape::Circle {
                        center,
                        radius: finite(c.r.value(), 0.0),
                    },
                );
            }
            Node::Text(t) => self.push(ctx, Shape::Text(text(t))),
            Node::Translate(n) => self.list(&n.draw, ctx),
            Node::Rotate(n) => self.list(&n.draw, ctx),
            Node::Scale(n) => self.list(&n.draw, ctx),
            Node::Clip(n) => self.list(&n.draw, ctx),
            _ => {}
        }
    }
}

fn text(t: &crate::ir::TextNode) -> Text {
    Text {
        position: Point {
            x: finite(t.x.value(), 0.0),
            y: finite(t.y.value(), 0.0),
        },
        content: String::from(&*t.text),
        font_size: finite(t.font_size.as_ref().map_or(f64::NAN, Num::value), 12.0),
        font_family: String::from(sanitize::sanitize_font_family(t.font_family.as_deref())),
        font_weight: t
            .font_weight
            .as_deref()
            .and_then(sanitize::sanitize_font_weight)
            .map_or(FontWeight::Normal, |w| weight(&w)),
        anchor: match t
            .text_anchor
            .as_deref()
            .and_then(sanitize::sanitize_text_anchor)
        {
            Some("middle") => TextAnchor::Middle,
            Some("end") => TextAnchor::End,
            _ => TextAnchor::Start,
        },
        baseline: t
            .alignment_baseline
            .as_deref()
            .and_then(sanitize::sanitize_baseline)
            .and_then(baseline),
    }
}

fn weight(w: &str) -> FontWeight {
    match w {
        "bold" => FontWeight::Bold,
        "bolder" => FontWeight::Bolder,
        "lighter" => FontWeight::Lighter,
        "normal" => FontWeight::Normal,
        n => n.parse().map_or(FontWeight::Normal, FontWeight::Number),
    }
}

fn baseline(b: &str) -> Option<Baseline> {
    Some(match b {
        "auto" => Baseline::Auto,
        "text-bottom" => Baseline::TextBottom,
        "alphabetic" => Baseline::Alphabetic,
        "ideographic" => Baseline::Ideographic,
        "middle" => Baseline::Middle,
        "central" => Baseline::Central,
        "mathematical" => Baseline::Mathematical,
        "hanging" => Baseline::Hanging,
        "text-top" => Baseline::TextTop,
        "text-before-edge" => Baseline::TextBeforeEdge,
        "text-after-edge" => Baseline::TextAfterEdge,
        _ => return None,
    })
}

impl Symbol {
    /// The symbol as a flat, typed [`Drawing`] for renderers other than SVG.
    ///
    /// Parses every path, so call it once per symbol and keep the result
    /// (a `CachedRenderer` shares symbols, not drawings).
    pub fn drawing(&self) -> Drawing {
        let frame = self.svg_frame();
        let mut b = Builder {
            stroke_width: frame.stroke_width,
            style_fill: frame.style_fill,
            items: Vec::new(),
            omitted_raw_svg: 0,
            invalid_paths: 0,
        };
        b.list(&self.instructions, &Inherited::initial());
        let (x, y, width, height) = frame.view_box();
        Drawing {
            items: b.items,
            view_box: ViewBox {
                x,
                y,
                width,
                height,
            },
            size: self.size,
            anchor: self.anchor,
            omitted_raw_svg: b.omitted_raw_svg,
            invalid_paths: b.invalid_paths,
        }
    }
}
