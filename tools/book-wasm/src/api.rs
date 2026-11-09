//! JSON boundary around the native library; JavaScript only presents results.

use crate::protocol::{Error, Fields, Request};
use milsymbol::sidc::Sidc;
use milsymbol::{
    Renderer,
    catalog::standards::{self, CodeKind},
};
use serde_json::{Value, json};
use wasm_bindgen::prelude::*;

/// Render a JSON request and return SVG, interpreted fields, diagnostics and a native Rust call.
#[wasm_bindgen]
pub fn render(request: &str) -> Result<String, JsValue> {
    response(request).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Export category, entity and modifier choices from the native catalog.
#[wasm_bindgen]
pub fn catalog(version: u8) -> String {
    crate::catalog::build(version).to_string()
}

/// A random SIDC of `version` that the given standard and mode draw as fully
/// valid. `standard` and `mode` are the JSON names used in render requests.
#[wasm_bindgen]
pub fn random_sidc(version: u8, standard: &str, mode: &str, seed: u32) -> Result<String, JsValue> {
    let pick = || -> Result<String, Error> {
        let standard = serde_json::from_value(Value::from(standard))?;
        let mode = serde_json::from_value(Value::from(mode))?;
        crate::random::sidc(version, standard, mode, u64::from(seed))
    };
    pick().map_err(|e| JsValue::from_str(&e.to_string()))
}

pub(crate) fn response(request: &str) -> Result<String, Error> {
    let request: Request = serde_json::from_str(request)?;
    let parsed = request.parse()?;
    let renderer = Renderer::builder()
        .standard(request.standard)
        .mode(request.mode)
        .build();
    let symbol = renderer
        .symbol(&parsed)
        .options(request.options.clone())
        .render()?;
    let (fields, entry) = match &parsed {
        Sidc::Numeric(n) => (
            Some(Fields::from_sidc(n)),
            standards::entry(n.version(), n.symbol_set(), CodeKind::Entity, n.entity()),
        ),
        _ => (None, None),
    };
    let validity = symbol.sidc_validity();
    let options = crate::snippet::options(&request)?;
    let (rust, rust_needs_serde) = crate::snippet::rust(&request, symbol.sidc())?;
    let source: Value = entry
        .map(|e| {
            json!({
                "name": e.name,
                "code": crate::catalog::source(e.source),
                "drawing": e.drawing_source().map(crate::catalog::source),
                "input": e.input_field(),
                "scope": format!("{:?}", e.scope),
            })
        })
        .unwrap_or(Value::Null);
    let issues: Vec<_> = validity.issues.iter().map(|i| format!("{i:?}")).collect();
    Ok(json!({
        "sidc": symbol.sidc(),
        "fields": fields,
        "svg": symbol.to_svg(),
        "valid": validity.is_valid(),
        "issues": issues,
        "metadata": symbol.metadata(),
        "bbox": symbol.bounding_box(),
        "anchor": symbol.anchor(),
        "octagon_anchor": symbol.octagon_anchor(),
        "size": symbol.size(),
        "source": source,
        "options": symbol.options(),
        "standard": request.standard,
        "mode": request.mode,
        "config": {
            "sidc": symbol.sidc(),
            "standard": request.standard,
            "mode": request.mode,
            "options": options,
        },
        "rust": rust,
        "rust_needs_serde": rust_needs_serde,
    })
    .to_string())
}

#[cfg(test)]
mod tests;
