//! Canonical JSON view of a rendered symbol, matching how milsymbol.js
//! serializes the same objects (`JSON.stringify` with keys sorted).
//!
//! Used for differential testing against the JavaScript oracle and for
//! diagnostics; not needed for rendering.

use crate::color::{COLOR_KEYS, ColorMode, ColorSet};
use crate::geometry::GeomShape;
use crate::ir::{Node, Num, Paint, Style};
use crate::js::write_number;
use crate::metadata::Metadata;
use crate::options::{StyleColor, SymbolOptions};
use alloc::string::String;
use alloc::vec::Vec;

/// A JSON value.
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    /// `null`.
    Null,
    /// A boolean.
    Bool(bool),
    /// A number (non-finite values serialize as `null`).
    Num(f64),
    /// A string.
    Str(String),
    /// An array.
    Arr(Vec<Json>),
    /// An object; keys are sorted on output.
    Obj(Vec<(String, Json)>),
}

impl Json {
    /// Serializes with keys sorted by UTF-16 code units, as the oracle does.
    pub fn to_canonical_string(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }

    fn write(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Num(n) if n.is_finite() => write_number(out, *n),
            Json::Num(_) => out.push_str("null"),
            Json::Str(s) => write_str(out, s),
            Json::Arr(v) => {
                out.push('[');
                for (i, e) in v.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    e.write(out);
                }
                out.push(']');
            }
            Json::Obj(fields) => {
                let mut sorted: Vec<&(String, Json)> = fields.iter().collect();
                sorted.sort_by(|a, b| a.0.encode_utf16().cmp(b.0.encode_utf16()));
                out.push('{');
                for (i, (k, v)) in sorted.into_iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_str(out, k);
                    out.push(':');
                    v.write(out);
                }
                out.push('}');
            }
        }
    }
}

fn write_str(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&alloc::format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Object builder that skips `undefined` (`None`) values.
#[derive(Default)]
pub(crate) struct Obj(Vec<(String, Json)>);

impl Obj {
    pub(crate) fn put(&mut self, k: &str, v: Json) -> &mut Self {
        self.0.push((String::from(k), v));
        self
    }

    pub(crate) fn opt(&mut self, k: &str, v: Option<Json>) -> &mut Self {
        if let Some(v) = v {
            self.put(k, v);
        }
        self
    }

    pub(crate) fn done(&mut self) -> Json {
        Json::Obj(core::mem::take(&mut self.0))
    }
}

pub(crate) fn s(v: &str) -> Json {
    Json::Str(String::from(v))
}

fn num(n: &Num) -> Json {
    match n {
        Num::Number(v) => Json::Num(*v),
        Num::Text(t) => s(t),
        Num::Bool(b) => Json::Bool(*b),
    }
}

fn paint(p: &Paint) -> Json {
    match p {
        Paint::None => Json::Bool(false),
        Paint::Color(c) => s(c),
    }
}

fn style(o: &mut Obj, st: &Style) {
    o.opt("fill", st.fill.as_ref().map(paint))
        .opt("fillopacity", st.fill_opacity.as_ref().map(num))
        .opt("stroke", st.stroke.as_ref().map(paint))
        .opt("strokewidth", st.stroke_width.as_ref().map(num))
        .opt("strokedasharray", st.stroke_dasharray.as_deref().map(s))
        .opt("linecap", st.line_cap.as_deref().map(s))
        .opt("non_scaling_stroke", st.non_scaling_stroke.map(Json::Num))
        .opt("styleFill", st.style_fill.map(Json::Bool))
        .opt("icon", st.icon.map(Json::Bool))
        .opt("clipPath", st.clip_path.as_deref().map(s));
}

/// JSON of a draw instruction list.
pub fn instructions(nodes: &[Node]) -> Json {
    Json::Arr(nodes.iter().map(node).collect())
}

fn node(n: &Node) -> Json {
    let mut o = Obj::default();
    match n {
        Node::Group(v) => return instructions(v),
        Node::Missing => return Json::Null,
        Node::Scalar(v) => return num(v),
        Node::TrustedSvg(svg) => return o.put("type", s("svg")).put("svg", s(svg)).done(),
        Node::Path(p) => {
            o.put("type", s("path")).put("d", s(p.d.source()));
        }
        Node::Circle(c) => {
            o.put("type", s("circle"))
                .put("cx", num(&c.cx))
                .put("cy", num(&c.cy))
                .put("r", num(&c.r));
        }
        Node::Text(t) => {
            o.put("type", s("text"))
                .put("x", num(&t.x))
                .put("y", num(&t.y))
                .put("text", s(&t.text));
            o.opt("fontsize", t.font_size.as_ref().map(num))
                .opt("fontfamily", t.font_family.as_deref().map(s))
                .opt("fontweight", t.font_weight.as_deref().map(s))
                .opt("textanchor", t.text_anchor.as_deref().map(s))
                .opt("alignmentBaseline", t.alignment_baseline.as_deref().map(s));
        }
        Node::Translate(t) => {
            o.put("type", s("translate"))
                .put("x", num(&t.x))
                .put("y", num(&t.y))
                .put("draw", instructions(&t.draw));
        }
        Node::Rotate(r) => {
            o.put("type", s("rotate"))
                .put("degree", num(&r.degree))
                .put("x", num(&r.x))
                .put("y", num(&r.y));
            o.put("draw", instructions(&r.draw));
        }
        Node::Scale(sc) => {
            o.put("type", s("scale"))
                .put("factor", num(&sc.factor))
                .put("draw", instructions(&sc.draw));
        }
        Node::Clip(c) => {
            o.put("type", s("clip"))
                .put("d", s(c.d.source()))
                .put("draw", instructions(&c.draw));
            o.opt("clipId", c.clip_id.as_deref().map(s));
        }
        Node::Bare(_) => {}
    }
    if let Some(st) = n.style() {
        style(&mut o, st);
    }
    o.done()
}

/// JSON of a colour mode.
pub fn color_mode(m: &ColorMode) -> Json {
    let mut o = Obj::default();
    for (k, v) in COLOR_KEYS.iter().zip(m.values()) {
        o.opt(k, v.as_ref().map(paint));
    }
    o.done()
}

/// JSON of a colour set (upstream `symbol.colors`).
pub fn colors(c: &ColorSet) -> Json {
    Obj::default()
        .put("fillColor", color_mode(&c.fill_color))
        .put("frameColor", color_mode(&c.frame_color))
        .put("iconColor", color_mode(&c.icon_color))
        .put("iconFillColor", color_mode(&c.icon_fill_color))
        .put("none", color_mode(&c.none))
        .put("black", color_mode(&c.black))
        .put("white", color_mode(&c.white))
        .done()
}

fn bbox_json(b: crate::BBox) -> Json {
    Obj::default()
        .put("x1", Json::Num(b.x1))
        .put("y1", Json::Num(b.y1))
        .put("x2", Json::Num(b.x2))
        .put("y2", Json::Num(b.y2))
        .done()
}

pub(crate) fn bbox(b: crate::BBox) -> Json {
    bbox_json(b)
}

/// JSON of metadata (upstream `symbol.metadata`).
pub fn metadata(md: &Metadata) -> Json {
    let mut o = Obj::default();
    let b = Json::Bool;
    let os = |v: &Option<String>| v.as_deref().map(s);
    o.put("activity", b(md.activity))
        .opt("affiliation", os(&md.affiliation))
        .opt("baseAffilation", os(&md.base_affiliation))
        .put("baseDimension", s(&md.base_dimension))
        .put("civilian", b(md.civilian))
        .put("condition", s(&md.condition))
        .opt("context", os(&md.context))
        .put("dimension", s(&md.dimension))
        .put("dimensionUnknown", b(md.dimension_unknown))
        .opt("echelon", os(&md.echelon))
        .put("faker", b(md.faker))
        .put("fenintDummy", b(md.fenint_dummy))
        .put("fill", b(md.fill))
        .put("frame", b(md.frame))
        .put("functionid", s(&md.function_id))
        .put("headquarters", b(md.headquarters))
        .put("installation", b(md.installation))
        .put("joker", b(md.joker))
        .opt("mobility", os(&md.mobility))
        .put("notpresent", s(&md.notpresent))
        .put("numberSIDC", b(md.number_sidc))
        .put("space", b(md.space))
        .put("STD2525", b(md.std2525))
        .put("taskForce", b(md.task_force))
        .put("unit", b(md.unit));
    let f = &md.flags;
    o.opt("edition", os(&f.edition))
        .opt("suspect", f.suspect.map(b))
        .opt("landequipment", f.landequipment.map(b))
        .opt("controlMeasure", f.control_measure.map(b))
        .opt("cyberspace", f.cyberspace.map(b))
        .opt("dismounted", f.dismounted.map(b))
        .opt("feintDummy", f.feint_dummy.map(b))
        .opt("leadership", os(&f.leadership))
        .opt("_modifier1", os(&f.modifier1))
        .opt("_modifier2", os(&f.modifier2));
    let (g, bb) = match md.geometry() {
        Some(geom) => {
            let g = match geom.shape {
                GeomShape::Path(d) => Obj::default().put("type", s("path")).put("d", s(d)).done(),
                GeomShape::Circle { cx, cy, r } => Obj::default()
                    .put("type", s("circle"))
                    .put("cx", Json::Num(cx))
                    .put("cy", Json::Num(cy))
                    .put("r", Json::Num(r))
                    .done(),
            };
            (g, geom.bbox())
        }
        None => (s(""), crate::BBox::default()),
    };
    o.put(
        "baseGeometry",
        Obj::default().put("g", g).put("bbox", bbox_json(bb)).done(),
    );
    o.done()
}

fn style_color(c: &StyleColor) -> Json {
    match c {
        StyleColor::Str(v) => s(v),
        StyleColor::PerAffiliation(m) => color_mode(m),
    }
}

/// JSON of options and style (upstream `symbol.getOptions()`).
pub fn options(sidc: &str, o: &SymbolOptions) -> Json {
    let mut j = Obj::default();
    for k in crate::options::field::DEFAULTS {
        if !o.text.contains_key(k) {
            j.put(k, s(""));
        }
    }
    for (k, v) in &o.text {
        j.put(k, s(v));
    }
    j.put("sidc", s(sidc))
        .opt("direction", o.direction.map(Json::Num))
        .put("speedLeader", Json::Num(o.speed_leader))
        .opt("stack", o.stack.map(Json::Num))
        .opt("country_flag", o.country_flag.as_deref().map(s))
        .opt("full_frame_flag", o.full_frame_flag.map(Json::Bool))
        .opt("signature", o.signature.as_deref().map(s));
    let st = &o.style;
    let n = Json::Num;
    let b = Json::Bool;
    j.put("alternateMedal", b(st.alternate_medal))
        .put("civilianColor", b(st.civilian_color))
        .put("colorMode", style_color(&st.color_mode))
        .put("fill", b(st.fill))
        .put("fillColor", s(&st.fill_color))
        .put("fillOpacity", n(st.fill_opacity))
        .put("fontfamily", s(&st.font_family))
        .put("frame", b(st.frame))
        .put("frameColor", style_color(&st.frame_color))
        .put("hqStaffLength", n(st.hq_staff_length))
        .put("icon", b(st.icon))
        .put("iconColor", style_color(&st.icon_color))
        .put("infoBackground", style_color(&st.info_background))
        .put(
            "infoBackgroundFrame",
            style_color(&st.info_background_frame),
        )
        .put("infoColor", style_color(&st.info_color))
        .put("infoFields", b(st.info_fields))
        .put("infoOutlineColor", s(&st.info_outline_color))
        .put(
            "infoOutlineWidth",
            st.info_outline_width.map_or(Json::Bool(false), n),
        )
        .put("infoSize", n(st.info_size))
        .put("monoColor", s(&st.mono_color))
        .put("outlineColor", style_color(&st.outline_color))
        .put("outlineWidth", n(st.outline_width))
        .put("padding", n(st.padding))
        .put("simpleStatusModifier", b(st.simple_status_modifier))
        .put("size", n(st.size))
        .put("square", b(st.square))
        .put(
            "standard",
            s(match st.standard {
                None => "",
                Some(crate::Standard::Mil2525) => "2525",
                Some(crate::Standard::App6) => "APP6",
            }),
        )
        .put("strokeWidth", n(st.stroke_width))
        .put("styleFill", b(st.style_fill));
    j.done()
}
