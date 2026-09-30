//! Font overrides for built-in icon templates and extension boundaries.

use milsymbol::ir::{Node, TextNode};
use milsymbol::options::{OptionError, SymbolOptions};
use milsymbol::{IconExtension, IconKey, IconPartContext, PartLookup, Renderer, Standard};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const CIVILIAN: &str = "10031100001100000000";

fn replace_fonts(nodes: &mut [Node]) -> usize {
    let mut count = 0;
    for node in nodes {
        if let Node::Text(text) = node {
            text.font_family = Some("Courier".into());
            count += 1;
        }
        if let Some(children) = node.children_mut() {
            count += replace_fonts(children);
        }
    }
    count
}

#[test]
fn icon_font_override_preserves_layout_and_other_instructions() -> TestResult {
    for standard in [Standard::Mil2525, Standard::App6] {
        let renderer = Renderer::builder().standard(standard).build();
        for sidc in [CIVILIAN, "SFAPC-----", "10031000001211000700"] {
            let mut options = SymbolOptions::default();
            options.style.font_family = "Courier".into();
            options.style.outline_width = 2.0;
            options.style.frame = false;
            let plain = renderer.render(sidc, options.clone())?;
            options.style.icon_text_uses_font_family = true;
            let custom = renderer.render(sidc, options)?;
            let mut expected = plain.instructions().to_vec();
            assert!(replace_fonts(&mut expected) > 0, "{sidc}");
            assert_eq!(custom.instructions(), expected, "{sidc}");
            assert_eq!(custom.bounding_box(), plain.bounding_box());
            assert_eq!(custom.size(), plain.size());
            assert_eq!(custom.anchor(), plain.anchor());
            assert_eq!(custom.octagon_anchor(), plain.octagon_anchor());
        }
    }
    Ok(())
}

struct MixedText;

impl IconExtension for MixedText {
    fn icon(
        &self,
        _: &IconPartContext<'_>,
        key: IconKey<'_>,
        parts: &dyn PartLookup,
    ) -> Option<Node> {
        if !matches!(
            key,
            IconKey::Entity {
                symbol_set: "11",
                entity: "110000"
            }
        ) {
            return None;
        }
        let mut text = TextNode::new(100.0, 100.0, "custom");
        text.font_family = Some("Georgia".into());
        Some(Node::Group(vec![
            Node::Text(text),
            parts.part("AR.I.CIVILIAN")?,
        ]))
    }
}

#[test]
fn extension_text_keeps_its_font_and_builtin_lookups_use_the_override() -> TestResult {
    let symbol = Renderer::builder()
        .icons(MixedText)
        .build()
        .symbol(CIVILIAN)
        .with(|options| {
            options.style.font_family = "Courier".into();
            options.style.icon_text_uses_font_family = true;
        })
        .render()?;
    let svg = symbol.to_svg();
    assert!(svg.contains("font-family=\"Georgia\""));
    assert!(svg.contains("font-family=\"Courier\""));
    assert!(!svg.contains("font-family=\"Arial\""));
    Ok(())
}

#[test]
fn icon_font_names_follow_svg_sanitization() -> TestResult {
    for (font, expected) in [
        (
            "\"Courier New\", monospace",
            "&quot;Courier New&quot;, monospace",
        ),
        ("Arial\" onload=\"alert(1)", "sans-serif"),
        ("<script>alert(1)</script>", "sans-serif"),
        ("", "sans-serif"),
    ] {
        let symbol = Renderer::default()
            .symbol(CIVILIAN)
            .with(|options| {
                options.style.font_family = font.into();
                options.style.icon_text_uses_font_family = true;
            })
            .render()?;
        let svg = symbol.to_svg();
        assert!(
            svg.contains(&format!("font-family=\"{expected}\"")),
            "{svg}"
        );
        assert!(!svg.contains(" onload="));
        assert!(!svg.contains("<script>"));
    }
    Ok(())
}

#[test]
fn icon_font_switch_requires_a_boolean() {
    let mut options = SymbolOptions::default();
    assert!(matches!(
        options.set("iconTextUsesFontFamily", "true"),
        Err(OptionError::Invalid { key, .. }) if key == "iconTextUsesFontFamily"
    ));
    assert!(!options.style.icon_text_uses_font_family);
}

#[cfg(feature = "std")]
#[test]
fn cache_keeps_font_override_requests_separate() -> TestResult {
    use milsymbol::cache::CachedRenderer;
    use std::sync::Arc;

    let cache = CachedRenderer::new(Renderer::default(), 4);
    let mut options = SymbolOptions::default();
    options.style.font_family = "Courier".into();
    let plain = cache.render(CIVILIAN, &options)?;
    options.style.icon_text_uses_font_family = true;
    let custom = cache.render(CIVILIAN, &options)?;
    assert!(!Arc::ptr_eq(&plain, &custom));
    assert_ne!(plain.to_svg(), custom.to_svg());
    assert!(Arc::ptr_eq(&custom, &cache.render(CIVILIAN, &options)?));
    options.style.icon_text_uses_font_family = false;
    assert!(Arc::ptr_eq(&plain, &cache.render(CIVILIAN, &options)?));
    Ok(())
}
