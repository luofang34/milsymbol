//! Borrowed fields of the canonical record. Object keys are sorted by the
//! writer; children are visited only when their field is emitted.

use super::view::Value;
use super::view::{Object, Value as Json};
use crate::color::{COLOR_KEYS, ColorMode, ColorSet};
use crate::ir::{Node, Num, Paint, Style};
use crate::options::{ColorChoice, ColorModeChoice, SymbolOptions};

fn s(v: &str) -> Json<'_> {
    Json::Str(v)
}
fn num(n: &Num) -> Json<'_> {
    match n {
        Num::Number(v) => Json::Num(*v),
        Num::Text(t) => s(t),
        Num::Bool(b) => Json::Bool(*b),
    }
}
fn paint(p: &Paint) -> Json<'_> {
    match p {
        Paint::None => Json::Bool(false),
        Paint::Color(c) => s(c),
    }
}
fn style_color(c: Option<&ColorChoice>) -> Json<'_> {
    match c {
        None => Json::Str(""),
        Some(ColorChoice::Uniform(c)) => Json::Str(c.as_str()),
        Some(ColorChoice::PerAffiliation(m)) => Json::ColorMode(m),
    }
}
fn mode_choice(c: &ColorModeChoice) -> Json<'_> {
    match c {
        ColorModeChoice::Named(s) => Json::Str(s),
        ColorModeChoice::Custom(m) => Json::ColorMode(m),
    }
}
fn style<'a>(o: &mut Object<'a, 24>, st: &'a Style) {
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

pub(super) fn node_value(n: &Node) -> Json<'_> {
    match n {
        Node::Group(v) => Json::Instructions(v),
        Node::Missing => Json::Null,
        Node::Scalar(v) => num(v),
        _ => Json::Node(n),
    }
}
pub(super) fn node<'a>(n: &'a Node, o: &mut Object<'a, 24>) {
    match n {
        Node::Group(_) | Node::Missing | Node::Scalar(_) => {}
        Node::TrustedSvg(svg) => {
            o.put("type", s("svg")).put("svg", s(svg));
        }
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
                .put("draw", Json::Instructions(&t.draw));
        }
        Node::Rotate(r) => {
            o.put("type", s("rotate"))
                .put("degree", num(&r.degree))
                .put("x", num(&r.x))
                .put("y", num(&r.y));
            o.put("draw", Json::Instructions(&r.draw));
        }
        Node::Scale(sc) => {
            o.put("type", s("scale"))
                .put("factor", num(&sc.factor))
                .put("draw", Json::Instructions(&sc.draw));
        }
        Node::Clip(c) => {
            o.put("type", s("clip"))
                .put("d", s(c.d.source()))
                .put("draw", Json::Instructions(&c.draw));
            o.opt("clipId", c.clip_id.as_deref().map(s));
        }
        Node::Bare(_) => {}
    }
    if let Some(st) = n.style() {
        style(o, st);
    }
}

/// JSON of a colour mode.
pub(super) fn color_mode<'a>(m: &'a ColorMode, o: &mut Object<'a, 6>) {
    for (k, v) in COLOR_KEYS.iter().zip(m.values()) {
        o.opt(k, v.as_ref().map(paint));
    }
}

/// JSON of a colour set (upstream `symbol.colors`).
pub(super) fn colors<'a>(c: &'a ColorSet, o: &mut Object<'a, 7>) {
    o.put("fillColor", Json::ColorMode(&c.fill_color))
        .put("frameColor", Json::ColorMode(&c.frame_color))
        .put("iconColor", Json::ColorMode(&c.icon_color))
        .put("iconFillColor", Json::ColorMode(&c.icon_fill_color))
        .put("none", Json::ColorMode(&c.none))
        .put("black", Json::ColorMode(&c.black))
        .put("white", Json::ColorMode(&c.white));
}

pub(super) fn metadata<'a>(md: &'a crate::metadata::Metadata, o: &mut Object<'a, 48>) {
    let md = crate::compat::JsMetadata::from(md);
    let b = Json::Bool;
    let os = |v: Option<&'a str>| v.map(s);
    o.put("activity", b(md.activity))
        .opt("affiliation", os(md.affiliation))
        .opt("baseAffilation", os(md.base_affiliation))
        .put("baseDimension", s(md.base_dimension))
        .put("civilian", b(md.civilian))
        .put("condition", s(md.condition))
        .opt("context", os(md.context))
        .put("dimension", s(md.dimension))
        .put("dimensionUnknown", b(md.dimension_unknown))
        .opt("echelon", os(md.echelon))
        .put("faker", b(md.faker))
        .put("fenintDummy", b(md.fenint_dummy))
        .put("fill", b(md.fill))
        .put("frame", b(md.frame))
        .put("functionid", s(md.function_id))
        .put("headquarters", b(md.headquarters))
        .put("installation", b(md.installation))
        .put("joker", b(md.joker))
        .opt("mobility", os(md.mobility))
        .put("notpresent", s(md.notpresent))
        .put("numberSIDC", b(md.number_sidc))
        .put("space", b(md.space))
        .put("STD2525", b(md.std2525))
        .put("taskForce", b(md.task_force))
        .put("unit", b(md.unit));
    let f = &md.flags;
    o.opt("edition", os(f.edition))
        .opt("suspect", f.suspect.map(b))
        .opt("landequipment", f.landequipment.map(b))
        .opt("controlMeasure", f.control_measure.map(b))
        .opt("cyberspace", f.cyberspace.map(b))
        .opt("dismounted", f.dismounted.map(b))
        .opt("feintDummy", f.feint_dummy.map(b))
        .opt("leadership", os(f.leadership))
        .opt("_modifier1", os(f.modifier1))
        .opt("_modifier2", os(f.modifier2));
    o.put("baseGeometry", Json::Geometry(md.geometry()));
}
pub(super) fn options<'a>(sidc: &'a str, o: &'a SymbolOptions, j: &mut Object<'a, 80>) {
    for f in crate::options::TextField::STANDARD {
        j.put(f.name(), s(o.text(f)));
    }
    j.put("sidc", s(sidc))
        .opt("direction", o.direction.map(Json::Num))
        .put("speedLeader", Json::Num(o.speed_leader_px()))
        .opt("stack", o.stack.map(Json::Num))
        .opt("country_flag", o.country_flag.as_deref().map(s))
        .opt("full_frame_flag", o.full_frame_flag.map(Json::Bool))
        .opt("signature", o.signature.as_deref().map(s));
    let st = &o.style;
    let n = Json::Num;
    let b = Json::Bool;
    j.put("alternateMedal", b(st.alternate_medal))
        .put("civilianColor", b(st.civilian_color))
        .put("colorMode", mode_choice(&st.color_mode))
        .put("fill", b(st.fill))
        .put(
            "fillColor",
            s(st.fill_color.as_ref().map_or("", |c| c.as_str())),
        )
        .put("fillOpacity", n(st.fill_opacity))
        .put("fontfamily", s(&st.font_family))
        .put("frame", b(st.frame))
        .put("frameColor", style_color(st.frame_color.as_ref()))
        .put("hqStaffLength", n(st.hq_staff_length.unwrap_or(0.0)))
        .put("icon", b(st.icon))
        .put("iconColor", style_color(st.icon_color.as_ref()))
        .put("infoBackground", style_color(st.info_background.as_ref()))
        .put(
            "infoBackgroundFrame",
            style_color(st.info_background_frame.as_ref()),
        )
        .put("infoColor", style_color(st.info_color.as_ref()))
        .put("infoFields", b(st.info_fields))
        .put(
            "infoOutlineColor",
            s(st.info_outline_color.as_ref().map_or("", |c| c.as_str())),
        )
        .put(
            "infoOutlineWidth",
            st.info_outline_width.map_or(Json::Bool(false), n),
        )
        .put("infoSize", n(st.info_size))
        .put("monoColor", s(st.mono_color_str()))
        .put("outlineColor", style_color(st.outline_color.as_ref()))
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
    if st.icon_text_uses_font_family {
        j.put("iconTextUsesFontFamily", b(true));
    }
    j.extend_missing(o.text.iter().map(|(k, v)| (k.as_str(), s(v))));
}

pub(super) fn symbol<'a>(s: &'a crate::Symbol, o: &mut Object<'a, 10>) {
    o.put("instructions", Value::Instructions(s.instructions()))
        .put("metadata", Value::Metadata(&s.metadata))
        .put("colors", Value::Colors(s.colors()))
        .put("bbox", Value::BBox(s.bounding_box()))
        .put("size", Value::Size(s.size()))
        .put("anchor", Value::Point(s.anchor()))
        .put("octagonAnchor", Value::Point(s.octagon_anchor()))
        .put("valid", Value::Bool(crate::compat::is_valid(s)))
        .put("validExtended", Value::Validity(s))
        .put("options", Value::Options(s));
}

pub(super) fn validity<'a>(s: &'a crate::Symbol, o: &mut Object<'a, 6>) {
    let md = crate::compat::js_metadata(s);
    let issues = s.upstream_issues();
    let has = |i| issues.contains(&i);
    use crate::compat::UpstreamIssue::{MissingInstruction, NullInDrawing, UnknownIcon};
    o.opt("affiliation", md.affiliation.map(Value::Str))
        .put("dimension", Value::Str(md.dimension))
        .put("dimensionUnknown", Value::Bool(md.dimension_unknown))
        .put(
            "drawInstructions",
            Value::Bool(!has(MissingInstruction) && !has(NullInDrawing)),
        )
        .put("icon", Value::Bool(!has(UnknownIcon)))
        .put("mobility", Value::Bool(md.mobility.is_some()));
}
