//! Compiled examples embedded in the usage manual; stdout is their SVGs as JSON.
use milsymbol::Symbol;
use std::io::Write;

type ExampleResult = Result<Symbol, Box<dyn std::error::Error>>;

fn first_render() -> ExampleResult {
    // ANCHOR: first_render
    use milsymbol::{Renderer, options::TextField};
    let symbol = Renderer::default()
        .symbol("15031000141211000000")
        .size(60.0)
        .text(TextField::UniqueDesignation, "A/1-66")
        .strict()
        .render()?;
    assert!(symbol.sidc_validity().is_valid());
    let svg = symbol.to_svg();
    assert!(svg.starts_with("<svg"));
    // ANCHOR_END: first_render
    Ok(symbol)
}

fn restricted_target() -> ExampleResult {
    // ANCHOR: restricted_target
    use milsymbol::{Renderer, domain::Context, sidc::Sidc};
    let sidc = Sidc::parse("15031000001211000000")?.with_context(Context::RestrictedReality)?;
    let symbol = Renderer::default()
        .symbol(&sidc)
        .size(60.0)
        .strict()
        .render()?;
    assert_eq!(symbol.metadata().context, Some(Context::RestrictedReality));
    // ANCHOR_END: restricted_target
    Ok(symbol)
}

fn weather_input() -> ExampleResult {
    // ANCHOR: weather_input
    use milsymbol::{Renderer, options::TextField};
    let symbol = Renderer::default()
        .symbol("15034500001101020000")
        .text(TextField::AltitudeDepth, "320")
        .size(80.0)
        .strict()
        .render()?;
    assert!(symbol.to_svg().contains(">320</text>"));
    // ANCHOR_END: weather_input
    Ok(symbol)
}

fn app_combined() -> ExampleResult {
    // ANCHOR: app_combined
    use milsymbol::{Renderer, Standard};
    let renderer = Renderer::builder().standard(Standard::App6).build();
    let symbol = renderer
        .symbol("16031000001221000000")
        .size(60.0)
        .strict()
        .render()?;
    assert!(symbol.sidc_validity().is_valid());
    // ANCHOR_END: app_combined
    Ok(symbol)
}

fn direction_and_text() -> ExampleResult {
    // ANCHOR: direction_and_text
    use milsymbol::{Renderer, options::TextField};
    let symbol = Renderer::default()
        .symbol("15031002161211000000")
        .size(45.0)
        .direction(60.0)
        .text(TextField::UniqueDesignation, "HQ / 2 BDE")
        .text(TextField::HigherFormation, "1 DIV")
        .render()?;
    let anchor = symbol.anchor();
    assert!(anchor.y > symbol.octagon_anchor().y);
    // ANCHOR_END: direction_and_text
    Ok(symbol)
}

fn style() -> ExampleResult {
    // ANCHOR: style
    use milsymbol::{Renderer, options::Color};
    let symbol = Renderer::default()
        .symbol("15031500331101000000")
        .size(60.0)
        .fill(false)
        .outline_width(3.0)
        .outline_color(Color::WHITE)
        .render()?;
    assert!(!symbol.options().style.fill);
    // ANCHOR_END: style
    Ok(symbol)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let examples = [
        ("first", first_render()?),
        ("restricted", restricted_target()?),
        ("weather", weather_input()?),
        ("app", app_combined()?),
        ("hq", direction_and_text()?),
        ("style", style()?),
    ];
    let output: std::collections::BTreeMap<_, _> = examples
        .into_iter()
        .map(|(name, symbol)| (name, symbol.to_svg()))
        .collect();
    std::io::stdout().write_all(serde_json::to_string(&output)?.as_bytes())?;
    Ok(())
}
