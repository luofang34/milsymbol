//! Public API behaviour: extensions, configuration, determinism, errors.

use milsymbol::ir::{Node, Paint};
use milsymbol::labels::{Label, LabelField};
use milsymbol::options::{OptionValue, SymbolOptions, field};
use milsymbol::{
    IconExtension, IconPartContext, IconTable, PartLookup, PartOutput, PartialBBox, RenderError,
    Renderer, Standard, SymbolPart, SymbolState, catalog,
};
use std::borrow::Cow;
use std::collections::BTreeMap;

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
    assert_eq!(s.metadata().affiliation.as_deref(), Some("Friend"));
    assert_eq!(s.metadata().dimension, "Ground");
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
    assert_eq!(
        err,
        Some(RenderError::UnknownColorMode {
            name: "NoSuchMode".into()
        })
    );
    Ok(())
}

#[test]
fn wrongly_typed_option_is_a_typed_error() -> TestResult {
    let mut o = SymbolOptions::default();
    let err = o.set("size", "big").err().map(|e| e.key);
    assert_eq!(err.as_deref(), Some("size"));
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

struct Marker;

impl SymbolPart for Marker {
    fn draw(&self, s: &SymbolState<'_>) -> Result<PartOutput, RenderError> {
        let mut n = Node::circle(100.0, 100.0, 5.0);
        if let Some(st) = n.style_mut() {
            st.fill = Some(Paint::color("magenta"));
        }
        let bbox = PartialBBox {
            y1: Some(s.bbox.y1 - 30.0),
            ..PartialBBox::default()
        };
        Ok(PartOutput {
            pre: vec![],
            post: vec![n],
            bbox,
            invalid_icon: false,
        })
    }
}

#[test]
fn custom_symbol_part_extends_pipeline_and_bbox() -> TestResult {
    let plain = Renderer::default().symbol(INFANTRY).render()?;
    let r = Renderer::default().with_symbol_part(Marker);
    let s = r.symbol(INFANTRY).render()?;
    assert!(s.to_svg().contains("fill=\"magenta\""));
    assert_eq!(s.bounding_box().y1, plain.bounding_box().y1 - 30.0);
    Ok(())
}

struct Custom;

impl IconExtension for Custom {
    fn icon_parts(&self, ctx: &IconPartContext<'_>, parts: &mut BTreeMap<String, Node>) {
        let mut n = Node::path("M80,80 L120,120");
        if let Some(st) = n.style_mut() {
            st.stroke = ctx
                .colors
                .icon_color
                .get(ctx.metadata.affiliation.as_deref().unwrap_or("Friend"));
        }
        parts.insert("MY.PART".into(), n);
    }

    fn number_icons(
        &self,
        ss: &str,
        parts: &dyn PartLookup,
        _: bool,
        _: Option<&str>,
        out: &mut IconTable,
    ) {
        if ss == "10" {
            if let (Some(mine), Some(infantry)) =
                (parts.part("MY.PART"), parts.part("GR.IC.FF.INFANTRY"))
            {
                out.icons
                    .insert("999900".into(), Node::Group(vec![mine, infantry]));
            }
        }
    }

    fn number_labels(&self, out: &mut BTreeMap<String, Vec<LabelField>>) {
        let label = Label {
            x: Some(100.0),
            y: Some(20.0),
            font_size: Some(30.0),
            anchor: Some(Cow::Borrowed("middle")),
            weight: None,
            baseline: None,
            fill: None,
            stroke: Some(false),
        };
        out.insert(
            "999900".into(),
            vec![LabelField {
                field: Cow::Borrowed("uniqueDesignation"),
                is_array: false,
                labels: Cow::Owned(vec![label]),
            }],
        );
    }
}

#[test]
fn icon_extension_adds_sidc_with_builtin_parts() -> TestResult {
    let r = Renderer::default().with_icons(Custom);
    let s = r.symbol("10031000009999000000").render()?;
    assert!(s.is_valid());
    let svg = s.to_svg();
    assert!(svg.contains("d=\"M80,80 L120,120\""), "{svg}");
    assert!(
        svg.contains("M25,50 L175,150"),
        "infantry cross from built-in part: {svg}"
    );
    assert!(
        !Renderer::default()
            .symbol("10031000009999000000")
            .render()?
            .is_valid()
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
        assert!(s.validity().icon, "{e}");
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
    assert_eq!(c.len(), 1, "cleared when full");
    Ok(())
}
