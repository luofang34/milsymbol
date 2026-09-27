//! A rendered symbol.

use crate::bbox::BBox;
use crate::color::ColorSet;
use crate::compose::Composition;
use crate::ir::{Node, Num, Paint, Point, Style};
use crate::json::{self, Json, Obj};
use crate::metadata::Metadata;
use crate::options::SymbolOptions;
use crate::svg::{self, SvgFrame};
use alloc::string::String;
use alloc::vec::Vec;

/// Pixel size of a rendered symbol.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    /// Width in pixels.
    pub width: f64,
    /// Height in pixels.
    pub height: f64,
}

/// Detailed validity (upstream `isValid(true)`).
#[derive(Debug, Clone, PartialEq)]
pub struct Validity {
    /// Affiliation the symbol was drawn with.
    pub affiliation: Option<String>,
    /// Dimension the symbol was drawn with.
    pub dimension: String,
    /// The battle dimension is unknown.
    pub dimension_unknown: bool,
    /// No draw instruction is missing.
    pub draw_instructions: bool,
    /// The icon was found.
    pub icon: bool,
    /// The mobility code was recognised.
    pub mobility: bool,
}

/// A composed symbol: draw instructions plus layout and metadata.
#[derive(Debug, Clone)]
pub struct Symbol {
    pub(crate) sidc: String,
    pub(crate) options: SymbolOptions,
    pub(crate) metadata: Metadata,
    pub(crate) colors: ColorSet,
    pub(crate) instructions: Vec<Node>,
    pub(crate) bbox: BBox,
    pub(crate) base_width: f64,
    pub(crate) base_height: f64,
    pub(crate) size: Size,
    pub(crate) anchor: Point,
    pub(crate) octagon_anchor: Point,
    pub(crate) valid_icon: bool,
}

impl Symbol {
    pub(crate) fn from_composition(c: Composition, options: SymbolOptions) -> Self {
        Symbol {
            sidc: c.sidc,
            options,
            metadata: c.metadata,
            colors: c.colors,
            instructions: c.instructions,
            bbox: c.bbox,
            base_width: c.base_width,
            base_height: c.base_height,
            size: Size {
                width: c.width,
                height: c.height,
            },
            anchor: c.anchor,
            octagon_anchor: c.octagon_anchor,
            valid_icon: c.valid_icon,
        }
    }

    /// The SIDC as interpreted (spaces removed, `*` → `-`, letter codes upper-cased).
    pub fn sidc(&self) -> &str {
        &self.sidc
    }

    /// Draw instructions in symbol units (the frame's reference box is
    /// 0–200 on both axes, centred on 100,100).
    pub fn instructions(&self) -> &[Node] {
        &self.instructions
    }

    /// Metadata derived from the SIDC and options.
    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    /// Resolved colours.
    pub fn colors(&self) -> &ColorSet {
        &self.colors
    }

    /// Options and style used.
    pub fn options(&self) -> &SymbolOptions {
        &self.options
    }

    /// Bounding box in symbol units, excluding stroke and outline margins.
    pub fn bounding_box(&self) -> BBox {
        self.bbox
    }

    /// Size in pixels.
    pub fn size(&self) -> Size {
        self.size
    }

    /// Point (in pixels, from the top-left) to place on the map position:
    /// the frame centre, or the foot of the headquarters staff.
    pub fn anchor(&self) -> Point {
        self.anchor
    }

    /// Centre of the icon octagon, in pixels from the top-left.
    pub fn octagon_anchor(&self) -> Point {
        self.octagon_anchor
    }

    /// Detailed validity.
    pub fn validity(&self) -> Validity {
        let md = &self.metadata;
        Validity {
            affiliation: md.affiliation.clone(),
            dimension: md.dimension.clone(),
            dimension_unknown: md.dimension_unknown,
            draw_instructions: !contains_null(&self.instructions),
            icon: self.valid_icon,
            mobility: md.mobility.is_some(),
        }
    }

    /// Whether upstream considers the symbol valid.
    pub fn is_valid(&self) -> bool {
        let v = self.validity();
        let md = &self.metadata;
        let bad_frame = md.affiliation.as_deref() == Some("undefined")
            || (md.dimension == "undefined" && !md.control_measure());
        !bad_frame && v.draw_instructions && v.icon && v.mobility
    }

    /// Renders the symbol as an SVG document identical to milsymbol.js `asSVG()`.
    pub fn to_svg(&self) -> String {
        let st = &self.options.style;
        let frame = SvgFrame {
            stroke_width: st.stroke_width,
            outline_width: st.outline_width,
            style_fill: st.style_fill,
            bbox_x1: self.bbox.x1,
            bbox_y1: self.bbox.y1,
            width: self.size.width,
            height: self.size.height,
            base_width: self.base_width,
            base_height: self.base_height,
        };
        svg::render(&frame, &self.instructions)
    }

    /// Canonical JSON of the symbol's observable state, in the same shape as
    /// the oracle records (`tools/oracle`): instructions, metadata, colours,
    /// bounding box, size, anchors, validity and options.
    pub fn to_canonical_json(&self) -> Json {
        let point = |p: Point| {
            Obj::default()
                .put("x", Json::Num(p.x))
                .put("y", Json::Num(p.y))
                .done()
        };
        let v = self.validity();
        let valid_ext = Obj::default()
            .opt("affiliation", v.affiliation.as_deref().map(json::s))
            .put("dimension", json::s(&v.dimension))
            .put("dimensionUnknown", Json::Bool(v.dimension_unknown))
            .put("drawInstructions", Json::Bool(v.draw_instructions))
            .put("icon", Json::Bool(v.icon))
            .put("mobility", Json::Bool(v.mobility))
            .done();
        Obj::default()
            .put("instructions", json::instructions(&self.instructions))
            .put("metadata", json::metadata(&self.metadata))
            .put("colors", json::colors(&self.colors))
            .put("bbox", json::bbox(self.bbox))
            .put(
                "size",
                Obj::default()
                    .put("width", Json::Num(self.size.width))
                    .put("height", Json::Num(self.size.height))
                    .done(),
            )
            .put("anchor", point(self.anchor))
            .put("octagonAnchor", point(self.octagon_anchor))
            .put("valid", Json::Bool(self.is_valid()))
            .put("validExtended", valid_ext)
            .put("options", json::options(&self.sidc, &self.options))
            .done()
    }
}

/// Whether `JSON.stringify(nodes)` would contain `"null"`: a missing
/// instruction, a non-finite number, or a string containing `null`.
fn contains_null(nodes: &[Node]) -> bool {
    nodes.iter().any(node_null)
}

fn num_null(n: &Num) -> bool {
    match n {
        Num::Number(v) => !v.is_finite(),
        Num::Text(t) => t.contains("null"),
        Num::Bool(_) => false,
    }
}

fn style_null(st: &Style) -> bool {
    let paint = |p: &Option<Paint>| matches!(p, Some(Paint::Color(c)) if c.contains("null"));
    let text = |t: &Option<crate::ir::Str>| t.as_deref().is_some_and(|t| t.contains("null"));
    paint(&st.fill)
        || paint(&st.stroke)
        || st.fill_opacity.as_ref().is_some_and(num_null)
        || st.stroke_width.as_ref().is_some_and(num_null)
        || text(&st.stroke_dasharray)
        || text(&st.line_cap)
        || text(&st.clip_path)
        || st.non_scaling_stroke.is_some_and(|v| !v.is_finite())
}

fn node_null(n: &Node) -> bool {
    let text = |t: &Option<crate::ir::Str>| t.as_deref().is_some_and(|t| t.contains("null"));
    let own = match n {
        Node::Missing => true,
        Node::Scalar(v) => num_null(v),
        Node::TrustedSvg(s) => s.contains("null"),
        Node::Group(v) => return contains_null(v),
        Node::Path(p) => p.d.source().contains("null"),
        Node::Circle(c) => num_null(&c.cx) || num_null(&c.cy) || num_null(&c.r),
        Node::Text(t) => {
            num_null(&t.x)
                || num_null(&t.y)
                || t.text.contains("null")
                || t.font_size.as_ref().is_some_and(num_null)
                || text(&t.font_family)
                || text(&t.font_weight)
                || text(&t.text_anchor)
                || text(&t.alignment_baseline)
        }
        Node::Translate(t) => num_null(&t.x) || num_null(&t.y),
        Node::Rotate(r) => num_null(&r.degree) || num_null(&r.x) || num_null(&r.y),
        Node::Scale(s) => num_null(&s.factor),
        Node::Clip(c) => c.d.source().contains("null") || text(&c.clip_id),
        Node::Bare(_) => false,
    };
    own || n.style().is_some_and(style_null) || n.children().is_some_and(contains_null)
}
