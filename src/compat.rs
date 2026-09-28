//! milsymbol.js-compatible representations, for interoperating with the
//! JavaScript library and for differential testing against it.
//!
//! [`JsMetadata`] is upstream's `symbol.metadata` with its string values and
//! `"undefined"` sentinels; [`canonical_json`] is the record the oracle in
//! `tools/oracle` writes for a symbol. Rendering does not need this module.

use crate::ir::Point;
use crate::json::{self, Obj};
use crate::symbol::Symbol;
use alloc::string::String;

pub use crate::json::Json;
pub use crate::metadata::{Metadata as JsMetadata, OptionalFlags};

/// Canonical JSON of the symbol's observable state, in the same shape as
/// the oracle records: instructions, metadata, colours, bounding box, size,
/// anchors, validity and options.
pub fn canonical_json(symbol: &Symbol) -> Json {
    let point = |p: Point| {
        Obj::default()
            .put("x", Json::Num(p.x))
            .put("y", Json::Num(p.y))
            .done()
    };
    let md = symbol.js_metadata();
    let issues = symbol.validity().issues;
    let has = |i| issues.contains(&i);
    use crate::ValidityIssue::{MissingInstruction, NullInDrawing, UnknownIcon};
    let valid_ext = Obj::default()
        .opt("affiliation", md.affiliation.as_deref().map(json::s))
        .put("dimension", json::s(&md.dimension))
        .put("dimensionUnknown", Json::Bool(md.dimension_unknown))
        .put(
            "drawInstructions",
            Json::Bool(!has(MissingInstruction) && !has(NullInDrawing)),
        )
        .put("icon", Json::Bool(!has(UnknownIcon)))
        .put("mobility", Json::Bool(md.mobility.is_some()))
        .done();
    let size = symbol.size();
    Obj::default()
        .put("instructions", json::instructions(symbol.instructions()))
        .put("metadata", json::metadata(md))
        .put("colors", json::colors(symbol.colors()))
        .put("bbox", json::bbox(symbol.bounding_box()))
        .put(
            "size",
            Obj::default()
                .put("width", Json::Num(size.width))
                .put("height", Json::Num(size.height))
                .done(),
        )
        .put("anchor", point(symbol.anchor()))
        .put("octagonAnchor", point(symbol.octagon_anchor()))
        .put("valid", Json::Bool(issues.is_empty()))
        .put("validExtended", valid_ext)
        .put("options", json::options(symbol.sidc(), symbol.options()))
        .done()
}

/// [`canonical_json`] serialized with sorted keys, as the oracle writes it.
pub fn canonical_json_string(symbol: &Symbol) -> String {
    canonical_json(symbol).to_canonical_string()
}
