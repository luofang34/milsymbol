//! SVG serialization (upstream `assvg.js`), including its escaping and
//! sanitization of attribute values and trusted fragments.

use crate::ir::{Node, Num, Paint, Style, TextNode};
use crate::js::write_number;
use alloc::string::String;

mod sanitize;
use sanitize::*;

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

/// Renders a complete SVG document.
pub(crate) fn render(frame: &SvgFrame, nodes: &[Node]) -> String {
    let sw = safe(frame.stroke_width, 0.0);
    let ow = safe(frame.outline_width, 0.0);
    let x = safe(frame.bbox_x1, 0.0) - sw - ow;
    let y = safe(frame.bbox_y1, 0.0) - sw - ow;
    let width = safe(frame.width, frame.base_width);
    let height = safe(frame.height, frame.base_height);
    let bw = safe(frame.base_width, width);
    let bh = safe(frame.base_height, height);
    let mut out = String::with_capacity(1024);
    out.push_str("<svg");
    str_attr(&mut out, "xmlns", "http://www.w3.org/2000/svg");
    str_attr(&mut out, "version", "1.2");
    str_attr(&mut out, "baseProfile", "tiny");
    num_attr(&mut out, "width", width);
    num_attr(&mut out, "height", height);
    let mut vb = String::new();
    for (i, v) in [x, y, bw, bh].into_iter().enumerate() {
        if i > 0 {
            vb.push(' ');
        }
        write_number(&mut vb, v);
    }
    str_attr(&mut out, "viewBox", &vb);
    out.push('>');
    let mut w = Writer {
        out,
        clip_counter: 0,
        stroke_width: frame.stroke_width,
        style_fill: frame.style_fill,
    };
    w.list(nodes);
    w.out.push_str("</svg>");
    w.out
}

struct Writer {
    out: String,
    clip_counter: u32,
    stroke_width: f64,
    style_fill: bool,
}

impl Writer {
    fn list(&mut self, nodes: &[Node]) {
        for n in nodes {
            self.node(n);
        }
    }

    fn clip_def(&mut self, id: &str, d: &str) {
        self.out.push_str("<clipPath");
        str_attr(&mut self.out, "id", id);
        self.out.push_str("><path");
        str_attr(&mut self.out, "d", d);
        str_attr(&mut self.out, "clip-rule", "nonzero");
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
            let id = alloc::format!("clip-inline-{}", self.clip_counter);
            self.clip_counter += 1;
            self.clip_def(&id, clip);
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
            str_attr(&mut self.out, "clip-path", &alloc::format!("url(#{id})"));
        }
        self.presentation(style);
        self.out.push_str(" >");
        self.close(node);
    }

    fn open(&mut self, node: &Node) {
        let o = &mut self.out;
        match node {
            Node::Path(p) => {
                o.push_str("<path");
                str_attr(o, "d", p.d.source());
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
                let v = alloc::format!(
                    "translate({},{})",
                    js_num(safe(t.x.value(), 0.0)),
                    js_num(safe(t.y.value(), 0.0))
                );
                str_attr(o, "transform", &v);
            }
            Node::Rotate(r) => {
                o.push_str("<g");
                let v = alloc::format!(
                    "rotate({},{},{})",
                    js_num(safe(r.degree.value(), 0.0)),
                    js_num(safe(r.x.value(), 0.0)),
                    js_num(safe(r.y.value(), 0.0))
                );
                str_attr(o, "transform", &v);
            }
            Node::Scale(sc) => {
                o.push_str("<g");
                str_attr(
                    o,
                    "transform",
                    &alloc::format!("scale({})", js_num(safe(sc.factor.value(), 1.0))),
                );
            }
            Node::Clip(c) => {
                let id = c
                    .clip_id
                    .as_deref()
                    .and_then(sanitize_id)
                    .unwrap_or_else(|| {
                        let id = alloc::format!("clip-custom-{}", self.clip_counter);
                        self.clip_counter += 1;
                        id
                    });
                self.clip_def(&id, c.d.source());
                self.out.push_str("<g");
                str_attr(&mut self.out, "clip-path", &alloc::format!("url(#{id})"));
            }
            _ => {}
        }
    }

    fn presentation(&mut self, st: &Style) {
        let o = &mut self.out;
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
                escape_text(&mut self.out, &t.text);
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

fn js_num(v: f64) -> String {
    crate::js::number_to_string(v)
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
