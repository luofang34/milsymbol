//! SVG serialization (upstream `assvg.js`), including its escaping and
//! sanitization of attribute values and trusted fragments.

use crate::ir::{Node, Num, Paint, PathData, Style, TextNode};
use crate::js::write_number;
use alloc::string::String;

mod ids;
pub(crate) mod sanitize;
use ids::ClipIds;
use sanitize::{
    escape_attr, escape_text, sanitize_baseline, sanitize_color, sanitize_dash_array,
    sanitize_font_family, sanitize_font_weight, sanitize_line_cap, sanitize_text_anchor,
    svg_fragment_blocked,
};

/// SVG output settings.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct SvgOptions {
    /// Prefix for every id in the document (clip paths), so several symbols
    /// can be inlined in one page. Characters invalid in an id are dropped.
    pub id_prefix: Option<crate::ir::Str>,
}

impl SvgOptions {
    /// Sets [`SvgOptions::id_prefix`].
    pub fn with_id_prefix(mut self, prefix: impl Into<crate::ir::Str>) -> Self {
        self.id_prefix = Some(prefix.into());
        self
    }
}

/// Inputs of the serializer besides the instructions.
pub(crate) struct SvgFrame {
    pub stroke_width: f64,
    pub outline_width: f64,
    pub style_fill: bool,
    pub bbox_x1: f64,
    pub bbox_y1: f64,
    pub width: f64,
    pub height: f64,
    pub base_width: f64,
    pub base_height: f64,
}

impl SvgFrame {
    /// The SVG `viewBox` as `(x, y, width, height)`.
    pub(crate) fn view_box(&self) -> (f64, f64, f64, f64) {
        let sw = safe(self.stroke_width, 0.0);
        let ow = safe(self.outline_width, 0.0);
        let x = safe(self.bbox_x1, 0.0) - sw - ow;
        let y = safe(self.bbox_y1, 0.0) - sw - ow;
        let width = safe(self.width, self.base_width);
        let height = safe(self.height, self.base_height);
        (
            x,
            y,
            safe(self.base_width, width),
            safe(self.base_height, height),
        )
    }
}

fn safe(v: f64, fallback: f64) -> f64 {
    if v.is_finite() { v } else { fallback }
}

fn num_attr(out: &mut String, name: &str, v: f64) {
    out.push(' ');
    out.push_str(name);
    out.push_str("=\"");
    write_number(out, v);
    out.push('"');
}

fn str_attr(out: &mut String, name: &str, v: &str) {
    out.push(' ');
    out.push_str(name);
    out.push_str("=\"");
    escape_attr(out, v);
    out.push('"');
}

/// ` name="path data"`. A packed path is streamed unescaped: see
/// `PathData::as_text`.
fn path_attr(out: &mut String, name: &str, d: &PathData) {
    match d.as_text() {
        Some(text) => str_attr(out, name, text),
        None => packed_attr(out, name, d),
    }
}

#[cfg(feature = "compact-paths")]
fn packed_attr(out: &mut String, name: &str, d: &PathData) {
    out.push(' ');
    out.push_str(name);
    out.push_str("=\"");
    d.write_source(out).ok();
    out.push('"');
}

/// Without `compact-paths` every path is text.
#[cfg(not(feature = "compact-paths"))]
fn packed_attr(_: &mut String, _: &str, _: &PathData) {}

/// ` transform="name(a,b,…)"`, non-finite arguments replaced by `fallback`.
/// Numbers contain no characters that attribute escaping would change.
fn transform_attr(out: &mut String, name: &str, args: &[f64], fallback: f64) {
    out.push_str(" transform=\"");
    out.push_str(name);
    out.push('(');
    for (i, v) in args.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        write_number(out, safe(*v, fallback));
    }
    out.push_str(")\"");
}

/// ` clip-path="url(#id)"`. Clip ids are generated or pass `sanitize_id`,
/// so they contain only ASCII alphanumerics and `.`, `_`, `:`, `-`.
fn clip_path_attr(out: &mut String, id: &str) {
    out.push_str(" clip-path=\"url(#");
    escape_attr(out, id);
    out.push_str(")\"");
}

/// Appends a complete SVG document to `out`.
pub(crate) fn render_into(
    frame: &SvgFrame,
    nodes: &[Node],
    options: &SvgOptions,
    out: &mut String,
) {
    let sw = safe(frame.stroke_width, 0.0);
    let ow = safe(frame.outline_width, 0.0);
    let x = safe(frame.bbox_x1, 0.0) - sw - ow;
    let y = safe(frame.bbox_y1, 0.0) - sw - ow;
    let width = safe(frame.width, frame.base_width);
    let height = safe(frame.height, frame.base_height);
    let bw = safe(frame.base_width, width);
    let bh = safe(frame.base_height, height);
    out.push_str("<svg");
    str_attr(out, "xmlns", "http://www.w3.org/2000/svg");
    str_attr(out, "version", "1.2");
    str_attr(out, "baseProfile", "tiny");
    num_attr(out, "width", width);
    num_attr(out, "height", height);
    // Numbers never contain characters that attribute escaping changes.
    out.push_str(" viewBox=\"");
    for (i, v) in [x, y, bw, bh].into_iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        write_number(out, v);
    }
    out.push_str("\">");
    let prefix = options
        .id_prefix
        .as_deref()
        .map(ids::sanitize_prefix)
        .unwrap_or_default();
    let mut w = Writer {
        out,
        clip_ids: ClipIds::new(&prefix, nodes),
        stroke_width: frame.stroke_width,
        style_fill: frame.style_fill,
    };
    w.list(nodes);
    w.out.push_str("</svg>");
}

struct Writer<'o> {
    out: &'o mut String,
    clip_ids: ClipIds<'o>,
    stroke_width: f64,
    style_fill: bool,
}

impl Writer<'_> {
    fn list(&mut self, nodes: &[Node]) {
        for n in nodes {
            self.node(n);
        }
    }

    fn clip_def(&mut self, id: &str, d: impl FnOnce(&mut String)) {
        self.out.push_str("<clipPath");
        str_attr(self.out, "id", id);
        self.out.push_str("><path");
        d(self.out);
        str_attr(self.out, "clip-rule", "nonzero");
        self.out.push_str(" /></clipPath>");
    }

    fn node(&mut self, node: &Node) {
        let style = match node {
            Node::Group(list) => return self.list(list),
            Node::Missing | Node::Scalar(_) => return,
            Node::TrustedSvg(svg) => {
                if !svg_fragment_blocked(svg) {
                    self.out.push_str(svg);
                }
                return;
            }
            other => other.style(),
        };
        let Some(style) = style else { return };
        let mut inline_clip = None;
        if let (Some(clip), false) = (&style.clip_path, matches!(node, Node::Clip(_))) {
            let id = self.clip_ids.generate("inline");
            self.clip_def(&id, |o| str_attr(o, "d", clip));
            inline_clip = Some(id);
        }
        self.open(node);
        let typed = matches!(
            node,
            Node::Path(_)
                | Node::Circle(_)
                | Node::Text(_)
                | Node::Translate(_)
                | Node::Rotate(_)
                | Node::Scale(_)
        );
        if let (Some(id), true) = (&inline_clip, typed) {
            clip_path_attr(self.out, id);
        }
        self.presentation(style);
        self.out.push_str(" >");
        self.close(node);
    }

    fn open(&mut self, node: &Node) {
        let o = &mut *self.out;
        match node {
            Node::Path(p) => {
                o.push_str("<path");
                path_attr(o, "d", &p.d);
            }
            Node::Circle(c) => {
                o.push_str("<circle");
                num_attr(o, "cx", safe(c.cx.value(), 0.0));
                num_attr(o, "cy", safe(c.cy.value(), 0.0));
                num_attr(o, "r", safe(c.r.value(), 0.0));
            }
            Node::Text(t) => open_text(o, t),
            Node::Translate(t) => {
                o.push_str("<g");
                transform_attr(o, "translate", &[t.x.value(), t.y.value()], 0.0);
            }
            Node::Rotate(r) => {
                o.push_str("<g");
                let args = [r.degree.value(), r.x.value(), r.y.value()];
                transform_attr(o, "rotate", &args, 0.0);
            }
            Node::Scale(sc) => {
                o.push_str("<g");
                transform_attr(o, "scale", &[sc.factor.value()], 1.0);
            }
            Node::Clip(c) => {
                let id = c
                    .clip_id
                    .as_deref()
                    .and_then(|r| self.clip_ids.request(r))
                    .unwrap_or_else(|| self.clip_ids.generate("custom"));
                self.clip_def(&id, |o| path_attr(o, "d", &c.d));
                self.out.push_str("<g");
                clip_path_attr(self.out, &id);
            }
            _ => {}
        }
    }

    fn presentation(&mut self, st: &Style) {
        let o = &mut *self.out;
        if let Some(stroke) = &st.stroke {
            let nss = safe(st.non_scaling_stroke.unwrap_or(1.0), 1.0);
            let setting = st
                .stroke_width
                .as_ref()
                .map_or(self.stroke_width, |w| safe(w.value(), self.stroke_width));
            num_attr(o, "stroke-width", safe(nss * setting, self.stroke_width));
            if let Some(dash) = st.stroke_dasharray.as_deref().and_then(sanitize_dash_array) {
                str_attr(o, "stroke-dasharray", dash);
            }
            if let Some(cap) = st.line_cap.as_deref().and_then(sanitize_line_cap) {
                str_attr(o, "stroke-linecap", cap);
                str_attr(o, "stroke-linejoin", cap);
            }
            str_attr(o, "stroke", paint_value(stroke));
        }
        if let Some(fill) = &st.fill {
            let v = if st.style_fill == Some(true) && self.style_fill {
                "rgba(255,255,255,0.4)"
            } else {
                paint_value(fill)
            };
            str_attr(o, "fill", v);
        }
        if let Some(op) = &st.fill_opacity {
            num_attr(o, "fill-opacity", safe(op.value(), 1.0).clamp(0.0, 1.0));
        }
    }

    fn close(&mut self, node: &Node) {
        match node {
            Node::Path(_) => self.out.push_str("</path>"),
            Node::Circle(_) => self.out.push_str("</circle>"),
            Node::Text(t) => {
                escape_text(self.out, &t.text);
                self.out.push_str("</text>");
            }
            Node::Translate(n) => self.group_end(&n.draw),
            Node::Rotate(n) => self.group_end(&n.draw),
            Node::Scale(n) => self.group_end(&n.draw),
            Node::Clip(n) => self.group_end(&n.draw),
            _ => {}
        }
    }

    fn group_end(&mut self, draw: &[Node]) {
        self.list(draw);
        self.out.push_str("</g>");
    }
}

fn paint_value(p: &Paint) -> &str {
    match p {
        Paint::Color(c) if !c.is_empty() => sanitize_color(c).unwrap_or("none"),
        _ => "none",
    }
}

fn open_text(o: &mut String, t: &TextNode) {
    o.push_str("<text");
    num_attr(o, "x", safe(t.x.value(), 0.0));
    num_attr(o, "y", safe(t.y.value(), 0.0));
    str_attr(
        o,
        "text-anchor",
        t.text_anchor
            .as_deref()
            .and_then(sanitize_text_anchor)
            .unwrap_or("start"),
    );
    num_attr(
        o,
        "font-size",
        safe(t.font_size.as_ref().map_or(f64::NAN, Num::value), 12.0),
    );
    str_attr(
        o,
        "font-family",
        sanitize_font_family(t.font_family.as_deref()),
    );
    if let Some(w) = t.font_weight.as_deref().and_then(sanitize_font_weight) {
        str_attr(o, "font-weight", &w);
    }
    if let Some(b) = t.alignment_baseline.as_deref().and_then(sanitize_baseline) {
        str_attr(o, "dominant-baseline", b);
    }
}

#[cfg(test)]
mod tests;
