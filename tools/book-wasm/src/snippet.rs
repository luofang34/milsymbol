//! Reproducible native Rust calls and compact configuration export.

use crate::protocol::{Error, Request};
use milsymbol::options::{SymbolOptions, TextField};
use milsymbol::{RenderMode, Standard};
use serde_json::{Map, Value};

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

/// Builder calls for the options that have a typed setter; `None` when the
/// request sets anything else, so the snippet never silently drops an option.
fn builder_calls(options: &Value) -> Option<(Vec<String>, bool)> {
    let mut options = options.as_object()?.clone();
    let mut calls = Vec::new();
    let mut uses_text = false;
    let mut style = take_object(&mut options, "style")?;
    for (key, setter) in [("size", "size"), ("outline_width", "outline_width")] {
        if let Some(value) = style.remove(key) {
            calls.push(format!(".{setter}({:?})", value.as_f64()?));
        }
    }
    for key in ["fill", "frame", "icon"] {
        if let Some(value) = style.remove(key) {
            calls.push(format!(".{key}({})", value.as_bool()?));
        }
    }
    if let Some(value) = options.remove("direction") {
        calls.push(format!(".direction({:?})", value.as_f64()?));
    }
    for (name, value) in take_object(&mut options, "text")? {
        let value = value.as_str()?;
        if value.is_empty() {
            continue;
        }
        let field = match TextField::from_name(&name) {
            TextField::Custom(_) => format!("TextField::custom({name:?})"),
            field => format!("TextField::{field:?}"),
        };
        calls.push(format!(".text({field}, {value:?})"));
        uses_text = true;
    }
    (style.is_empty() && options.is_empty()).then_some((calls, uses_text))
}

fn take_object(options: &mut Map<String, Value>, key: &str) -> Option<Map<String, Value>> {
    match options.remove(key) {
        None => Some(Map::new()),
        Some(Value::Object(map)) => Some(map),
        Some(_) => None,
    }
}

fn renderer(request: &Request, imports: &mut Vec<&str>) -> String {
    if request.standard == Standard::default() && request.mode == RenderMode::default() {
        return "Renderer::default()".into();
    }
    let mut builder = String::from("Renderer::builder()");
    if request.standard != Standard::default() {
        imports.push("Standard");
        builder += &format!("\n    .standard(Standard::{:?})", request.standard);
    }
    if request.mode != RenderMode::default() {
        imports.push("RenderMode");
        builder += &format!("\n    .mode(RenderMode::{:?})", request.mode);
    }
    builder + "\n    .build()"
}

/// The native call that reproduces this request, and whether it needs the
/// `serde` feature and `serde_json` because an option has no typed setter.
pub(crate) fn rust(request: &Request, sidc: &str) -> Result<(String, bool), Error> {
    let options = options(request)?;
    let mut imports = vec!["Renderer"];
    let renderer = renderer(request, &mut imports);
    let Some((calls, uses_text)) = builder_calls(&options) else {
        imports.push("options::SymbolOptions");
        let json = serde_json::to_string(&options)?;
        return Ok((
            format!(
                "use milsymbol::{{{}}};\n\nlet options: SymbolOptions = serde_json::from_str({json:?})?;\nlet symbol = {renderer}\n    .symbol({sidc:?})\n    .options(options)\n    .render()?;\nlet svg = symbol.to_svg();",
                imports.join(", ")
            ),
            true,
        ));
    };
    if uses_text {
        imports.push("options::TextField");
    }
    let calls: String = calls.iter().map(|call| format!("\n    {call}")).collect();
    Ok((
        format!(
            "use milsymbol::{{{}}};\n\nlet symbol = {renderer}\n    .symbol({sidc:?}){calls}\n    .render()?;\nlet svg = symbol.to_svg();",
            imports.join(", ")
        ),
        false,
    ))
}

#[cfg(test)]
mod tests;
