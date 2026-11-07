//! Reproducible native Rust calls and compact configuration export.

use crate::protocol::{Error, Request};
use milsymbol::options::SymbolOptions;
use serde_json::Value;

fn difference(value: &Value, defaults: &Value) -> Value {
    match (value, defaults) {
        (Value::Object(value), Value::Object(defaults)) => Value::Object(
            value
                .iter()
                .filter(|(k, v)| defaults.get(*k) != Some(*v))
                .map(|(k, v)| {
                    (
                        k.clone(),
                        difference(v, defaults.get(k).unwrap_or(&Value::Null)),
                    )
                })
                .collect(),
        ),
        _ => value.clone(),
    }
}

pub(crate) fn options(request: &Request) -> Result<Value, Error> {
    Ok(difference(
        &serde_json::to_value(&request.options)?,
        &serde_json::to_value(SymbolOptions::default())?,
    ))
}

pub(crate) fn rust(request: &Request, sidc: &str) -> Result<String, Error> {
    let json = serde_json::to_string(&options(request)?)?;
    Ok(format!(
        "use milsymbol::{{Renderer, RenderMode, Standard, options::SymbolOptions}};\n\nlet options: SymbolOptions = serde_json::from_str({json:?})?;\nlet renderer = Renderer::builder()\n    .standard(Standard::{:?})\n    .mode(RenderMode::{:?})\n    .build();\nlet symbol = renderer.symbol({sidc:?}).options(options).render()?;\nlet svg = symbol.to_svg();",
        request.standard, request.mode
    ))
}
