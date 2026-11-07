//! Each expected snippet is also written out below as compiled code, which
//! must render the same SVG as the browser request it was generated from.
use crate::api::response;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn generated(request: &Value) -> Result<Value, Box<dyn std::error::Error>> {
    Ok(serde_json::from_str(&response(&request.to_string())?)?)
}

#[test]
fn lab_controls_become_typed_builder_calls() -> TestResult {
    let request = json!({"sidc":"15034500001101020000","options":{
        "style":{"size":80,"fill":false},"direction":45,
        "text":{"uniqueDesignation":"","altitudeDepth":"320"}}});
    let result = generated(&request)?;
    assert_eq!(
        result.get("rust").and_then(Value::as_str),
        Some(
            "use milsymbol::{Renderer, options::TextField};\n\nlet symbol = Renderer::default()\n    .symbol(\"15034500001101020000\")\n    .size(80.0)\n    .fill(false)\n    .direction(45.0)\n    .text(TextField::AltitudeDepth, \"320\")\n    .render()?;\nlet svg = symbol.to_svg();"
        )
    );
    assert_eq!(result.get("rust_needs_serde"), Some(&Value::Bool(false)));

    use milsymbol::{Renderer, options::TextField};
    let symbol = Renderer::default()
        .symbol("15034500001101020000")
        .size(80.0)
        .fill(false)
        .direction(45.0)
        .text(TextField::AltitudeDepth, "320")
        .render()?;
    assert_eq!(
        result.get("svg").and_then(Value::as_str),
        Some(symbol.to_svg().as_str())
    );
    Ok(())
}

#[test]
fn non_default_renderer_is_built_explicitly() -> TestResult {
    let request = json!({"sidc":"10031000161211000000","standard":"App6","mode":"Milsymbol304",
        "options":{"text":{"uniqueDesignation":"A/1"}}});
    let result = generated(&request)?;
    assert_eq!(
        result.get("rust").and_then(Value::as_str),
        Some(
            "use milsymbol::{Renderer, Standard, RenderMode, options::TextField};\n\nlet symbol = Renderer::builder()\n    .standard(Standard::App6)\n    .mode(RenderMode::Milsymbol304)\n    .build()\n    .symbol(\"10031000161211000000\")\n    .text(TextField::UniqueDesignation, \"A/1\")\n    .render()?;\nlet svg = symbol.to_svg();"
        )
    );

    use milsymbol::{RenderMode, Renderer, Standard, options::TextField};
    let symbol = Renderer::builder()
        .standard(Standard::App6)
        .mode(RenderMode::Milsymbol304)
        .build()
        .symbol("10031000161211000000")
        .text(TextField::UniqueDesignation, "A/1")
        .render()?;
    assert_eq!(
        result.get("svg").and_then(Value::as_str),
        Some(symbol.to_svg().as_str())
    );
    Ok(())
}

#[test]
fn options_without_a_setter_fall_back_to_serialized_options() -> TestResult {
    let result = generated(&json!({"options":{"style":{"civilian_color":false}}}))?;
    let rust = result.get("rust").and_then(Value::as_str).ok_or("rust")?;
    assert!(rust.contains("serde_json::from_str("), "{rust}");
    assert!(rust.contains("civilian_color"), "{rust}");
    assert_eq!(result.get("rust_needs_serde"), Some(&Value::Bool(true)));
    Ok(())
}
