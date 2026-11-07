//! The browser protocol executes the same renderer as native callers.
use super::response;
use crate::protocol::{Fields, Request};
use milsymbol::{Renderer, Standard, sidc::Sidc};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn browser_svg_is_the_native_svg() -> TestResult {
    for (sidc, standard) in [
        ("15031000141211000000", Standard::Mil2525),
        ("16031000001221000000", Standard::App6),
        ("16631000001211000000", Standard::App6),
        ("15034500001101020000", Standard::Mil2525),
    ] {
        let request =
            json!({"sidc":sidc,"standard":standard,"options":{"text":{"altitudeDepth":"320"}}});
        let parsed: Request = serde_json::from_value(request.clone())?;
        let actual: Value = serde_json::from_str(&response(&request.to_string())?)?;
        let native = Renderer::builder()
            .standard(standard)
            .build()
            .symbol(sidc)
            .options(parsed.options)
            .render()?;
        assert_eq!(
            actual.get("svg").and_then(Value::as_str),
            Some(native.to_svg().as_str())
        );
        assert_eq!(actual.get("valid"), Some(&Value::Bool(true)));
    }
    Ok(())
}

#[test]
fn controls_round_trip_extended_codes_and_keep_meanings() -> TestResult {
    for code in [
        "1663100014121100A0000000000000",
        "150364000013010000000000000000",
        "150310000012110007001000000000",
    ] {
        let parsed = Sidc::parse(code)?;
        let Sidc::Numeric(n) = parsed else {
            return Err("numeric fixture".into());
        };
        assert_eq!(Fields::from_sidc(&n).encode()?, code);
        let result: Value = serde_json::from_str(&response(
            &json!({"fields":Fields::from_sidc(&n)}).to_string(),
        )?)?;
        assert_eq!(result.get("sidc").and_then(Value::as_str), Some(code));
    }
    Ok(())
}

#[test]
fn malformed_input_is_reported_and_unknown_codes_remain_unknown() -> TestResult {
    assert!(response(r#"{"sidc":"not a SIDC"}"#).is_err());
    let result: Value = serde_json::from_str(&response(r#"{"sidc":"15031000009999990000"}"#)?)?;
    assert_eq!(result.get("valid"), Some(&Value::Bool(false)));
    assert!(
        !result
            .get("issues")
            .and_then(Value::as_array)
            .ok_or("issues")?
            .is_empty()
    );
    Ok(())
}

#[test]
fn exported_configuration_recreates_the_same_svg() -> TestResult {
    let result: Value = serde_json::from_str(&response(
        r#"{"options":{"style":{"size":40,"outline_width":2},"text":{"uniqueDesignation":"<&Unit"}}}"#,
    )?)?;
    let config = result.get("config").ok_or("config")?;
    let again: Value = serde_json::from_str(&response(&config.to_string())?)?;
    assert_eq!(result.get("svg"), again.get("svg"));
    Ok(())
}
