//! Public API behaviour: extensions, configuration, determinism, errors.

use milsymbol::ValidityIssue;
use milsymbol::domain::Affiliation;
use milsymbol::ir::Node;
use milsymbol::options::{OptionError, OptionValue, SymbolOptions, field};
use milsymbol::{RenderError, Renderer, Standard, catalog};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const INFANTRY: &str = "10031000001211000000";

#[test]
fn renders_valid_infantry_with_expected_frame() -> TestResult {
    let s = Renderer::default().symbol(INFANTRY).render()?;
    assert!(s.is_valid());
    let svg = s.to_svg();
    assert!(svg.starts_with(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" version=\"1.2\" baseProfile=\"tiny\""
    ));
    assert!(svg.contains("d=\"M25,50 l150,0 0,100 -150,0 z\""), "{svg}");
    assert_eq!(s.metadata().affiliation, Some(Affiliation::Friend));
    assert_eq!(s.js_metadata().dimension, "Ground");
    let a = s.anchor();
    assert_eq!((a.x, a.y), (79.0, 54.0));
    Ok(())
}

#[test]
fn rendering_is_deterministic_and_thread_safe() -> TestResult {
    let r = std::sync::Arc::new(Renderer::default());
    let expected = r.symbol("SHGPUCIZ---K").render()?.to_svg();
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let r = std::sync::Arc::clone(&r);
            std::thread::spawn(move || r.symbol("SHGPUCIZ---K").render().map(|s| s.to_svg()))
        })
        .collect();
    for h in handles {
        let svg = h.join().map_err(|_| "render thread panicked")??;
        assert_eq!(svg, expected);
    }
    Ok(())
}

#[test]
fn unknown_color_mode_is_a_typed_error() -> TestResult {
    let mut o = SymbolOptions::default();
    o.set("colorMode", "NoSuchMode")?;
    let err = Renderer::default().render(INFANTRY, o).err();
    assert!(
        matches!(&err, Some(RenderError::UnknownColorMode { name }) if name == "NoSuchMode"),
        "{err:?}"
    );
    Ok(())
}

#[test]
fn wrongly_typed_option_is_a_typed_error() -> TestResult {
    let mut o = SymbolOptions::default();
    let err = o.set("size", "big").err();
    assert!(
        matches!(&err, Some(OptionError::Invalid { key, .. }) if key == "size"),
        "{err:?}"
    );
    let err = o.set("uniqueDesignaton", "A").err();
    assert!(
        matches!(&err, Some(OptionError::Unknown { key }) if key == "uniqueDesignaton"),
        "{err:?}"
    );
    let err = o.set("standard", "APP-6").err();
    assert!(
        matches!(&err, Some(OptionError::Invalid { key, .. }) if key == "standard"),
        "{err:?}"
    );
    o.set("standard", "APP6")?;
    assert_eq!(o.style.standard, Some(Standard::App6));
    o.set_text("dtg1", "D1");
    assert_eq!(o.text("dtg1"), "D1");
    assert!(o.set("size", OptionValue::Num(50.0)).is_ok());
    assert!(o.set("sidc", "SFG").is_err());
    Ok(())
}

#[test]
fn text_is_escaped_in_svg() -> TestResult {
    let s = Renderer::default()
        .symbol(INFANTRY)
        .text(field::UNIQUE_DESIGNATION, "<script>&")
        .render()?;
    let svg = s.to_svg();
    assert!(svg.contains("&lt;script&gt;&amp;</text>"), "{svg}");
    assert!(!svg.contains("<script>"));
    Ok(())
}

#[test]
fn built_in_icon_text_uses_font_family_only_when_requested() -> TestResult {
    const CIVILIAN: &str = "10031100001100000000";
    fn text_family<'a>(nodes: &'a [Node], text: &str) -> Option<&'a str> {
        nodes.iter().find_map(|node| match node {
            Node::Text(t) if t.text == text => t.font_family.as_deref(),
            other => other
                .children()
                .and_then(|children| text_family(children, text)),
        })
    }

    let renderer = Renderer::default();
    let mut legacy = SymbolOptions::default();
    legacy.set("fontfamily", "Courier")?;
    legacy.set("uniqueDesignation", "A")?;
    let default = renderer.render(CIVILIAN, legacy.clone())?;
    assert_eq!(text_family(default.instructions(), "CIV"), Some("Arial"));
    assert_eq!(text_family(default.instructions(), "A"), Some("Courier"));

    legacy.set("iconTextUsesFontFamily", false)?;
    assert_eq!(
        renderer.render(CIVILIAN, legacy.clone())?.to_svg(),
        default.to_svg()
    );
    legacy.set("iconTextUsesFontFamily", true)?;
    let enabled = renderer.render(CIVILIAN, legacy)?;
    assert_eq!(text_family(enabled.instructions(), "CIV"), Some("Courier"));
    assert_eq!(text_family(enabled.instructions(), "A"), Some("Courier"));
    assert_eq!(
        enabled.to_svg(),
        default
            .to_svg()
            .replace("font-family=\"Arial\"", "font-family=\"Courier\"")
    );
    Ok(())
}

#[test]
fn standard_is_renderer_configuration() -> TestResult {
    let app6 = Renderer::default().with_standard(Standard::App6);
    assert!(!app6.symbol(INFANTRY).render()?.metadata().std2525);
    assert!(
        Renderer::default()
            .symbol(INFANTRY)
            .render()?
            .metadata()
            .std2525
    );
    Ok(())
}

#[test]
fn catalog_lists_renderable_icons() -> TestResult {
    let r = Renderer::default();
    let sets: Vec<_> = catalog::number_symbol_sets().collect();
    assert!(sets.contains(&"10") && sets.contains(&"25"));
    let entities: Vec<_> = catalog::number_entities("10").collect();
    assert!(entities.len() > 150, "{}", entities.len());
    for e in entities.iter().take(50) {
        let s = r.render(&format!("1003100000{e}0000"), SymbolOptions::default())?;
        assert!(
            !s.validity().issues.contains(&ValidityIssue::UnknownIcon),
            "{e}"
        );
    }
    assert!(catalog::letter_icons().any(|c| c == "S-G-UCI---"));
    assert!(catalog::icon_parts().any(|p| p == "GR.IC.FF.INFANTRY"));
    Ok(())
}

#[test]
fn instructions_expose_typed_path_segments() -> TestResult {
    let s = Renderer::default().symbol(INFANTRY).render()?;
    let Some(Node::Path(frame)) = s.instructions().first() else {
        return Err("the frame is not the first instruction".into());
    };
    let segs = frame.d.segments()?;
    assert_eq!(segs.len(), 5);
    Ok(())
}

#[cfg(feature = "std")]
#[test]
fn cached_renderer_reuses_identical_requests_only() -> TestResult {
    use milsymbol::cache::CachedRenderer;
    let c = CachedRenderer::new(Renderer::default(), 2);
    let o = SymbolOptions::default();
    let a = c.render(INFANTRY, &o)?;
    let b = c.render(INFANTRY, &o)?;
    assert!(std::sync::Arc::ptr_eq(&a, &b));
    let mut other = o.clone();
    other.style.size = 50.0;
    let d = c.render(INFANTRY, &other)?;
    assert!(!std::sync::Arc::ptr_eq(&a, &d));
    assert_eq!(c.len(), 2);
    c.render("SFGPUCI-----", &o)?;
    assert_eq!(c.len(), 2, "one entry replaced when full");
    Ok(())
}

#[cfg(feature = "std")]
#[test]
fn cached_renderer_keys_long_requests_exactly() -> TestResult {
    use milsymbol::cache::CachedRenderer;
    let c = CachedRenderer::new(Renderer::default(), 8);
    // Text long enough that the lookup key no longer fits on the stack.
    let long = "x".repeat(4096);
    let mut a = SymbolOptions::default();
    a.set_text(field::ADDITIONAL_INFORMATION, &long);
    let mut b = SymbolOptions::default();
    b.set_text(
        field::ADDITIONAL_INFORMATION,
        format!("{}y", "x".repeat(4095)),
    );
    let first = c.render(INFANTRY, &a)?;
    assert!(std::sync::Arc::ptr_eq(&first, &c.render(INFANTRY, &a)?));
    assert!(!std::sync::Arc::ptr_eq(&first, &c.render(INFANTRY, &b)?));
    assert_eq!(c.len(), 2);
    Ok(())
}

#[test]
fn write_svg_appends_the_same_document() -> TestResult {
    let symbol = Renderer::default().symbol(INFANTRY).render()?;
    let mut buf = String::from("prefix");
    symbol.write_svg(&mut buf);
    assert_eq!(buf.strip_prefix("prefix"), Some(symbol.to_svg().as_str()));
    Ok(())
}

#[cfg(feature = "std")]
#[test]
fn cached_symbols_can_carry_parsed_paths() -> TestResult {
    use milsymbol::cache::CachedRenderer;
    use milsymbol::ir::Node;
    fn all_borrowed(nodes: &[Node]) -> Result<bool, milsymbol::ir::PathParseError> {
        for n in nodes {
            if let Node::Path(p) = n {
                if matches!(p.d.segments()?, std::borrow::Cow::Owned(_)) {
                    return Ok(false);
                }
            }
            if !all_borrowed(n.children().unwrap_or(&[]))? {
                return Ok(false);
            }
        }
        Ok(true)
    }
    let plain = CachedRenderer::new(Renderer::default(), 4);
    assert!(!all_borrowed(
        plain
            .render(INFANTRY, &SymbolOptions::default())?
            .instructions()
    )?);
    let c = CachedRenderer::new(Renderer::default(), 4).with_prepared_paths();
    let first = c.render(INFANTRY, &SymbolOptions::default())?;
    let hit = c.render(INFANTRY, &SymbolOptions::default())?;
    assert!(std::sync::Arc::ptr_eq(&first, &hit));
    assert!(all_borrowed(first.instructions())?);
    Ok(())
}

#[test]
fn every_catalog_symbol_has_parseable_paths() -> TestResult {
    let r = Renderer::default();
    let mut sidcs: Vec<String> = catalog::number_symbol_sets()
        .flat_map(|ss| catalog::number_entities(ss).map(move |e| format!("1003{ss}0016{e}0000")))
        .collect();
    sidcs.extend(catalog::letter_icons().map(|g| {
        g.chars()
            .enumerate()
            .map(|(i, c)| match i {
                1 => 'H',
                3 => 'A',
                _ => c,
            })
            .collect::<String>()
    }));
    for sidc in &sidcs {
        let mut s = r
            .symbol(sidc)
            .text(field::UNIQUE_DESIGNATION, "A")
            .render()?;
        s.cache_path_segments()
            .map_err(|e| format!("{sidc}: {e}"))?;
    }
    Ok(())
}

#[cfg(feature = "std")]
#[test]
fn enabling_prepared_paths_invalidates_unprepared_cache() -> TestResult {
    use milsymbol::cache::CachedRenderer;
    use std::{borrow::Cow, sync::Arc};

    let cache = CachedRenderer::new(Renderer::default(), 4);
    let options = SymbolOptions::default();
    let unprepared = cache.render(INFANTRY, &options)?;
    let Some(Node::Path(original_frame)) = unprepared.instructions().first() else {
        return Err("the frame is not the first instruction".into());
    };
    assert!(matches!(original_frame.d.segments()?, Cow::Owned(_)));

    let cache = cache.with_prepared_paths();
    let prepared = cache.render(INFANTRY, &options)?;
    let Some(Node::Path(frame)) = prepared.instructions().first() else {
        return Err("the frame is not the first instruction".into());
    };
    assert!(matches!(frame.d.segments()?, Cow::Borrowed(_)));
    assert!(!Arc::ptr_eq(&unprepared, &prepared));
    assert_eq!(unprepared.to_svg(), prepared.to_svg());
    assert!(matches!(original_frame.d.segments()?, Cow::Owned(_)));
    assert!(Arc::ptr_eq(&prepared, &cache.render(INFANTRY, &options)?));

    let cache = cache.with_prepared_paths();
    assert!(Arc::ptr_eq(&prepared, &cache.render(INFANTRY, &options)?));
    Ok(())
}

#[test]
fn javascript_object_keys_are_ordinary_text_fields() -> TestResult {
    // milsymbol.js drops `__proto__` (the prototype setter swallows it) and
    // throws for `hasOwnProperty` (it shadows the method upstream calls).
    // Here both are plain custom text fields (UPSTREAM.md).
    for key in ["__proto__", "hasOwnProperty", "constructor"] {
        let mut o = SymbolOptions::default();
        o.set_text(key, "x");
        let s = Renderer::default().render(INFANTRY, o)?;
        assert_eq!(s.options().text(key), "x", "{key}");
        let json = milsymbol::compat::canonical_json_string(&s);
        assert!(json.contains(&format!("\"{key}\":\"x\"")), "{key}");
    }
    Ok(())
}
