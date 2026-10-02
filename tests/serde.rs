//! Serialization of options, configuration and the drawing view.

#![cfg(feature = "serde")]

use milsymbol::options::{Color, ColorChoice, SymbolOptions, TextField};
use milsymbol::{Renderer, RendererConfig, Standard};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn options_round_trip() -> TestResult {
    let mut o = SymbolOptions::default();
    o.style.size = 42.0;
    o.style.frame_color = Some(ColorChoice::Uniform(Color::new("red")?));
    o.style.hq_staff_length = Some(30.0);
    o.direction = Some(15.0);
    o.speed_leader = Some(10.0);
    o.set_text(TextField::UniqueDesignation, "A")
        .set_text("dtg1", "D");
    let json = serde_json::to_string(&o)?;
    let back: SymbolOptions = serde_json::from_str(&json)?;
    assert_eq!(back, o);
    let r = Renderer::default();
    let sidc = "10031000161211000000";
    assert_eq!(r.render(sidc, o)?.to_svg(), r.render(sidc, back)?.to_svg());
    Ok(())
}

#[test]
fn text_fields_and_colors_are_strings() -> TestResult {
    assert_eq!(
        serde_json::to_string(&TextField::HigherFormation)?,
        "\"higherFormation\""
    );
    assert_eq!(
        serde_json::from_str::<TextField>("\"dtg1\"")?,
        TextField::custom("dtg1")
    );
    assert_eq!(
        serde_json::from_str::<TextField>("\"type\"")?,
        TextField::Type
    );
    assert_eq!(serde_json::to_string(&Color::new("#f00")?)?, "\"#f00\"");
    assert!(
        serde_json::from_str::<Color>("\"\"").is_err(),
        "an empty colour is not a colour"
    );
    Ok(())
}

#[test]
fn configuration_round_trips() -> TestResult {
    let mut config = RendererConfig::default();
    config.standard = Standard::App6;
    config.hq_staff_length = 60.0;
    let back: RendererConfig = serde_json::from_str(&serde_json::to_string(&config)?)?;
    assert_eq!(back, config);
    Ok(())
}

#[test]
fn drawing_and_metadata_serialize() -> TestResult {
    let symbol = Renderer::default()
        .symbol("10031000161211000000")
        .render()?;
    let drawing = serde_json::to_value(symbol.drawing())?;
    let items = drawing.get("items").and_then(serde_json::Value::as_array);
    assert!(items.is_some_and(|i| !i.is_empty()));
    let width = drawing.get("view_box").and_then(|v| v.get("width"));
    assert!(width.is_some_and(serde_json::Value::is_number));
    let back: milsymbol::drawing::Drawing = serde_json::from_value(drawing)?;
    assert_eq!(back, symbol.drawing());
    let md = serde_json::to_value(symbol.metadata())?;
    assert_eq!(
        md.get("headquarters"),
        Some(&serde_json::Value::Bool(false))
    );
    assert!(md.get("geometry").is_some_and(serde_json::Value::is_array));
    Ok(())
}

#[test]
fn serialized_defaults_are_pinned() -> TestResult {
    let options = include_str!("data/serde_default_options.txt");
    let config = include_str!("data/serde_default_config.txt");
    assert_eq!(serde_json::to_string(&SymbolOptions::default())?, options);
    assert_eq!(serde_json::to_string(&RendererConfig::default())?, config);
    assert_eq!(
        serde_json::from_str::<SymbolOptions>(options)?,
        SymbolOptions::default()
    );
    assert_eq!(
        serde_json::from_str::<RendererConfig>(config)?,
        RendererConfig::default()
    );
    Ok(())
}

#[test]
fn missing_fields_take_their_defaults() -> TestResult {
    assert_eq!(
        serde_json::from_str::<SymbolOptions>("{}")?,
        SymbolOptions::default()
    );
    assert_eq!(
        serde_json::from_str::<RendererConfig>("{}")?,
        RendererConfig::default()
    );
    let partial: SymbolOptions =
        serde_json::from_str(r#"{"direction":15.0,"style":{"size":42.0}}"#)?;
    let mut expected = SymbolOptions::default();
    expected.direction = Some(15.0);
    expected.style.size = 42.0;
    assert_eq!(partial, expected);
    let config: RendererConfig = serde_json::from_str(r#"{"hq_staff_length":50.0}"#)?;
    let mut expected = RendererConfig::default();
    expected.hq_staff_length = 50.0;
    assert_eq!(config, expected);
    Ok(())
}
