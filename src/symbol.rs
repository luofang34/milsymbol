//! A rendered symbol.

use crate::bbox::BBox;
use crate::color::ColorSet;
use crate::compose::Composition;
use crate::ir::{Node, Point};
use crate::metadata::Metadata as JsMetadata;
use crate::options::SymbolOptions;
use crate::svg::{self, SvgFrame};
use alloc::string::String;
use alloc::vec::Vec;

mod validity;
use validity::IconCheck;
pub use validity::{Validity, ValidityIssue};

/// Pixel size of a rendered symbol.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    /// Width in pixels.
    pub width: f64,
    /// Height in pixels.
    pub height: f64,
}

/// A composed symbol: draw instructions plus layout and metadata.
#[derive(Debug, Clone)]
pub struct Symbol {
    pub(crate) sidc: String,
    pub(crate) options: SymbolOptions,
    pub(crate) metadata: JsMetadata,
    pub(crate) colors: ColorSet,
    pub(crate) instructions: Vec<Node>,
    pub(crate) bbox: BBox,
    pub(crate) base_width: f64,
    pub(crate) base_height: f64,
    pub(crate) size: Size,
    pub(crate) anchor: Point,
    pub(crate) octagon_anchor: Point,
    pub(crate) valid_icon: bool,
    pub(crate) icon_known: bool,
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
            icon_known: c.icon_known,
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

    /// Typed description: affiliation, dimension, status, amplifiers.
    pub fn metadata(&self) -> crate::domain::Metadata {
        crate::domain::Metadata::from_js(&self.metadata)
    }

    /// Parses every path once and caches the segments, for renderers that
    /// read [`PathData::segments`](crate::ir::PathData::segments) repeatedly.
    pub fn cache_path_segments(&mut self) -> Result<(), crate::ir::PathParseError> {
        crate::ir::parse_paths(&mut self.instructions)
    }

    /// Metadata in milsymbol.js's representation (string values, including
    /// its `"undefined"` sentinels); see [`Symbol::metadata`] for typed values.
    pub fn js_metadata(&self) -> &JsMetadata {
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

    /// Every reason milsymbol.js `isValid()` would reject the symbol.
    pub fn validity(&self) -> Validity {
        Validity {
            issues: validity::issues(self, IconCheck::Drawn),
        }
    }

    /// Whether milsymbol.js considers the symbol valid (`isValid()`).
    ///
    /// This mirrors upstream exactly, including its heuristic that treats
    /// any text containing `null` (e.g. a unique designation `"null value"`)
    /// or a non-finite number as invalid, and its treatment of hidden icons
    /// as found. To check the SIDC itself, use [`Symbol::is_sidc_valid`] or
    /// [`Renderer::check_sidc`](crate::Renderer::check_sidc).
    pub fn is_valid(&self) -> bool {
        validity::issues(self, IconCheck::Drawn).is_empty()
    }

    /// Whether the SIDC was fully recognised: every code is known and the
    /// icon exists, whether or not icons are drawn, and without upstream's
    /// `null`-text heuristic.
    pub fn is_sidc_valid(&self) -> bool {
        validity::issues(self, IconCheck::Sidc)
            .iter()
            .all(|i| *i == ValidityIssue::NullInDrawing)
    }

    /// Renders the symbol as an SVG document identical to milsymbol.js `asSVG()`.
    pub fn to_svg(&self) -> String {
        let mut out = String::with_capacity(1024);
        self.write_svg(&mut out);
        out
    }

    /// Appends the SVG document of [`Symbol::to_svg`] to `out`, so bulk
    /// rendering can reuse one buffer.
    pub fn write_svg(&self, out: &mut String) {
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
        svg::render_into(&frame, &self.instructions, out);
    }
}
