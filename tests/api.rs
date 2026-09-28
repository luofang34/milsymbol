//! Public API behaviour: extensions, configuration, determinism, errors.

use milsymbol::ValidityIssue;
use milsymbol::domain::Affiliation;
use milsymbol::ir::{Node, Paint};
use milsymbol::labels::{Label, LabelField};
use milsymbol::options::{OptionError, OptionValue, SymbolOptions, field};
use milsymbol::{
    IconExtension, IconKey, IconPartContext, PartLookup, PartOutput, PartialBBox, RenderError,
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
    fn draw(&self, s: &SymbolState<'_>) -> Result<PartOutput, milsymbol::PartError> {
        let mut n = Node::circle(100.0, 100.0, 5.0);
        if let Some(st) = n.style_mut() {
            st.fill = Some(Paint::color("magenta"));
        }
        let bbox = PartialBBox {
            y1: Some(s.bbox().y1 - 30.0),
            ..PartialBBox::default()
        };
        Ok(PartOutput::new(vec![], vec![n], bbox))
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
    fn icon_part(&self, ctx: &IconPartContext<'_>, name: &str, _: &dyn PartLookup) -> Option<Node> {
        if name != "MY.PART" {
            return None;
        }
        let mut n = Node::path("M80,80 L120,120");
        if let Some(st) = n.style_mut() {
            let aff = ctx
                .metadata
                .affiliation
                .map_or("Friend", Affiliation::as_str);
            st.stroke = ctx.colors.icon_color.get(aff);
        }
        Some(n)
    }

    fn icon(
        &self,
        _: &IconPartContext<'_>,
        key: IconKey<'_>,
        parts: &dyn PartLookup,
    ) -> Option<Node> {
        let IconKey::Entity {
            symbol_set: "10",
            entity: "999900",
        } = key
        else {
            return None;
        };
        Some(Node::Group(vec![
            parts.part("MY.PART")?,
            parts.part("GR.IC.FF.INFANTRY")?,
        ]))
    }

    fn number_labels(&self, out: &mut BTreeMap<String, Vec<LabelField>>) {
        let mut label = Label::at(100.0, 20.0);
        label.font_size = Some(30.0);
        label.anchor = Some(Cow::Borrowed("middle"));
        label.stroke = Some(false);
        out.insert(
            "999900".into(),
            vec![LabelField::new("uniqueDesignation", vec![label])],
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
    assert_eq!(c.len(), 1, "cleared when full");
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

/// Replaces the built-in infantry part, as `tests/data/override_oracle.txt`
/// does in milsymbol.js via `ms.addIconParts`.
struct InfantryCircle;

impl IconExtension for InfantryCircle {
    fn icon_part(&self, ctx: &IconPartContext<'_>, name: &str, _: &dyn PartLookup) -> Option<Node> {
        if name != "GR.IC.FF.INFANTRY" {
            return None;
        }
        let mut n = Node::circle(100.0, 100.0, 20.0);
        if let Some(st) = n.style_mut() {
            st.fill = Some(Paint::None);
            st.stroke = ctx.colors.icon_color.get(
                ctx.js_metadata
                    .affiliation
                    .as_deref()
                    .unwrap_or("undefined"),
            );
            st.stroke_width = Some(milsymbol::ir::Num::Number(3.0));
        }
        Some(n)
    }
}

#[test]
fn extension_parts_replace_builtin_parts_like_upstream() -> TestResult {
    let expected = include_str!("data/override_oracle.txt");
    let r = Renderer::default().with_icons(InfantryCircle);
    let sidcs = [
        "10031000001211000000",
        "SFGPUCI-----",
        "10061000001211000000",
    ];
    for (sidc, want) in sidcs.iter().zip(expected.lines()) {
        assert_eq!(r.symbol(sidc).render()?.to_svg(), want, "{sidc}");
    }
    assert_eq!(expected.lines().count(), sidcs.len());
    Ok(())
}

#[derive(Debug)]
struct Broken;

impl std::fmt::Display for Broken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("broken part")
    }
}

impl std::error::Error for Broken {}

struct Failing;

impl SymbolPart for Failing {
    fn draw(&self, _: &SymbolState<'_>) -> Result<PartOutput, milsymbol::PartError> {
        Err(Box::new(Broken))
    }
}

#[test]
fn failing_part_keeps_its_position_and_source() -> TestResult {
    use std::error::Error as _;
    let err = Renderer::default()
        .with_symbol_part(Failing)
        .render(INFANTRY, SymbolOptions::default())
        .err();
    let Some(RenderError::Part { index, source }) = &err else {
        return Err(format!("expected a part error, got {err:?}").into());
    };
    assert_eq!(*index, 9, "after the nine built-in parts");
    assert!(source.is::<Broken>());
    assert!(err.as_ref().and_then(|e| e.source()).is_some());
    Ok(())
}

#[test]
fn unbounded_stack_is_rejected_instead_of_looping() -> TestResult {
    for bad in [f64::INFINITY, f64::NAN, 1e20, 1001.0] {
        let mut o = SymbolOptions::default();
        o.stack = Some(bad);
        let err = Renderer::default().render(INFANTRY, o).err();
        assert!(
            matches!(err, Some(RenderError::InvalidOption { name: "stack", .. })),
            "stack {bad}: {err:?}"
        );
    }
    for ok in [-3.0, 0.0, 2.5, 1000.0] {
        let mut o = SymbolOptions::default();
        o.stack = Some(ok);
        Renderer::default().render(INFANTRY, o)?;
    }
    Ok(())
}

#[test]
fn sidc_parse_validates_fields_and_exposes_typed_values() -> TestResult {
    use milsymbol::domain::{Context, StandardIdentity, Status};
    use milsymbol::sidc::{Sidc, SidcError};
    let Sidc::Numeric(n) = Sidc::parse("10031000161211000000")? else {
        return Err("expected a numeric SIDC".into());
    };
    assert_eq!(n.standard_identity(), StandardIdentity::Friend);
    assert_eq!(n.context(), Context::Reality);
    assert_eq!(n.symbol_set(), "10");
    assert_eq!(n.amplifier(), "16");
    assert_eq!(n.entity(), "121100");
    assert!(n.has_builtin_icon());
    let Sidc::Numeric(joker) = Sidc::parse("10151000001211000000")? else {
        return Err("expected a numeric SIDC".into());
    };
    assert_eq!(joker.standard_identity(), StandardIdentity::Joker);
    for (bad, field) in [
        ("99031000001211000000", "version"),
        ("10091000001211000000", "standard identity"),
        ("10034400001211000000", "symbol set"),
        ("10031000901211000000", "echelon/mobility"),
        ("10031090001211000000", "status"),
    ] {
        let err = Sidc::parse(bad).err();
        assert!(
            matches!(&err, Some(SidcError::InvalidField { field: f, .. }) if *f == field),
            "{bad}: {err:?}"
        );
    }
    assert!(matches!(
        Sidc::parse("1003"),
        Err(SidcError::Length { len: 4 })
    ));
    let Sidc::Letter(l) = Sidc::parse("sfgpuci----d")? else {
        return Err("expected a letter SIDC".into());
    };
    assert_eq!(
        (l.coding_scheme(), l.status(), l.function_id()),
        ('S', Status::Present, "UCI---")
    );
    assert!(l.has_builtin_icon());
    assert!(Sidc::parse("SZGPUCI-----").is_err());
    Ok(())
}

#[test]
fn typed_info_and_validity_issues() -> TestResult {
    use milsymbol::domain::{Affiliation, Dimension, Echelon};
    let s = Renderer::default()
        .symbol("10061000161211000000")
        .render()?;
    let info = s.metadata();
    assert_eq!(info.affiliation, Some(Affiliation::Hostile));
    assert_eq!(info.dimension, Some(Dimension::Ground));
    assert_eq!(info.echelon, Some(Echelon::BattalionSquadron));
    // Upstream counts text containing "null" as invalid; the SIDC is fine.
    let s = Renderer::default()
        .symbol(INFANTRY)
        .text(field::UNIQUE_DESIGNATION, "null value")
        .render()?;
    assert!(!s.is_valid());
    assert!(s.is_sidc_valid());
    assert_eq!(s.validity().issues, vec![ValidityIssue::NullInDrawing]);
    let s = Renderer::default()
        .symbol("10031000009999000000")
        .render()?;
    assert_eq!(s.validity().issues, vec![ValidityIssue::UnknownIcon]);
    assert!(!s.is_sidc_valid());
    Ok(())
}

#[test]
fn strict_parse_checks_the_standard_code_tables() -> TestResult {
    use milsymbol::sidc::{Sidc, SidcError};
    let field_of = |s: &str| match Sidc::parse(s) {
        Err(SidcError::InvalidField { field, .. }) => Some(field),
        _ => None,
    };
    assert_eq!(field_of("SFQPUCI-----"), Some("battle dimension"));
    assert_eq!(field_of("SFGPUCI---MZ"), Some("symbol modifier"));
    assert_eq!(field_of("GFQPUCI-----"), Some("battle dimension"));
    for ok in [
        "SFGPUCI---MO",
        "SFGPUCI----D",
        "SFGPUCI---AF",
        "SFSPUCI---NS",
        "GFGPGLB----K",
        "WAS-PC----P----",
        "10030000000000000000",
        "10034500000000000000",
    ] {
        Sidc::parse(ok).map_err(|e| format!("{ok}: {e}"))?;
    }
    let a = Sidc::parse(INFANTRY)?;
    let b = a;
    assert_eq!(a, b, "Sidc is Copy");
    Ok(())
}

#[test]
fn sidc_validity_does_not_depend_on_icon_visibility() -> TestResult {
    for icon in [true, false] {
        let s = Renderer::default()
            .symbol("10031000009999990000")
            .with(|o| o.style.icon = icon)
            .render()?;
        assert!(!s.is_sidc_valid(), "icon={icon}");
    }
    let hidden = Renderer::default()
        .symbol("10031000009999990000")
        .with(|o| o.style.icon = false)
        .render()?;
    assert!(hidden.is_valid(), "upstream treats hidden icons as found");
    Ok(())
}

#[test]
fn renderer_checks_support_including_extensions() -> TestResult {
    use milsymbol::sidc::SidcCheckError;
    let r = Renderer::default();
    r.check_sidc(INFANTRY)?;
    assert!(matches!(
        r.check_sidc("10031000009999000000"),
        Err(SidcCheckError::Unsupported { issues }) if issues == [ValidityIssue::UnknownIcon]
    ));
    assert!(matches!(
        r.check_sidc("SFQPUCI-----"),
        Err(SidcCheckError::Malformed(_))
    ));
    Renderer::default()
        .with_icons(Custom)
        .check_sidc("10031000009999000000")?;
    Ok(())
}

#[test]
fn strict_parse_accepts_every_icon_the_tables_define() -> TestResult {
    use milsymbol::sidc::Sidc;
    let mut checked = 0;
    for ss in catalog::number_symbol_sets() {
        for e in catalog::number_entities(ss) {
            let sidc = format!("1003{ss}0000{e}0000");
            Sidc::parse(&sidc).map_err(|err| format!("{sidc}: {err}"))?;
            checked += 1;
        }
    }
    for generic in catalog::letter_icons() {
        let sidc: String = generic
            .chars()
            .enumerate()
            .map(|(i, c)| match i {
                1 => 'F',
                3 => 'P',
                _ => c,
            })
            .collect();
        let sidc = format!("{sidc}-----");
        Sidc::parse(&sidc).map_err(|err| format!("{sidc}: {err}"))?;
        checked += 1;
    }
    assert!(checked > 3000, "{checked}");
    Ok(())
}

#[test]
fn sidc_validity_requires_a_well_formed_code() -> TestResult {
    use milsymbol::sidc::Sidc;
    // milsymbol.js accepts identity 7 and context 3; the strict parser does not.
    for sidc in ["10070100001100000000", "10300100001100000000"] {
        let s = Renderer::default().symbol(sidc).render()?;
        assert!(Sidc::parse(sidc).is_err());
        assert!(!s.is_sidc_valid(), "{sidc}");
    }
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
                if matches!(p.d.segments()?, Cow::Owned(_)) {
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
