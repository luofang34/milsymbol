//! The unified font entry point.

use milsymbol::Renderer;
use milsymbol::options::TextField;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A symbol whose icon draws text with a template font.
const TEXT_ICON: &str = "10031000001100000000";

fn families(svg: &str) -> Vec<&str> {
    svg.split("font-family=\"")
        .skip(1)
        .filter_map(|rest| rest.split('"').next())
        .collect()
}

#[test]
fn icons_keep_their_template_font_by_default() -> TestResult {
    let svg = Renderer::default().symbol(TEXT_ICON).render()?.to_svg();
    assert!(families(&svg).contains(&"Arial"), "{svg}");
    Ok(())
}

#[test]
fn font_applies_to_icon_text_and_information_fields() -> TestResult {
    let svg = Renderer::default()
        .symbol(TEXT_ICON)
        .text(TextField::UniqueDesignation, "A1")
        .font("Courier New")
        .render()?
        .to_svg();
    let found = families(&svg);
    assert!(found.len() >= 2, "{svg}");
    assert!(found.iter().all(|f| *f == "Courier New"), "{found:?}");
    Ok(())
}

#[cfg(feature = "std")]
#[test]
fn cached_and_plain_builders_agree() -> TestResult {
    use milsymbol::cache::CachedRenderer;
    use milsymbol::options::SymbolOptions;
    let renderer = Renderer::default();
    let plain = renderer
        .symbol(TEXT_ICON)
        .font("Courier New")
        .render()?
        .to_svg();
    let cache = CachedRenderer::new(renderer.clone(), 4);
    let cached = cache
        .symbol(TEXT_ICON)
        .font("Courier New")
        .render()?
        .to_svg();
    assert_eq!(plain, cached);
    let mut options = SymbolOptions::default();
    options.set_font("Courier New");
    assert_eq!(renderer.render(TEXT_ICON, options)?.to_svg(), plain);
    Ok(())
}
