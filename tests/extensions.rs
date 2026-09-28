//! Extension API: custom symbol parts, icon extensions, label overrides and
//! clip instructions.

use milsymbol::ValidityIssue;
use milsymbol::domain::Affiliation;
use milsymbol::ir::{Node, Paint};
use milsymbol::labels::{Label, LabelField};
use milsymbol::options::SymbolOptions;
use milsymbol::{
    IconExtension, IconKey, IconPartContext, PartLookup, PartOutput, PartialBBox, RenderError,
    Renderer, SymbolPart, SymbolState,
};
use std::borrow::Cow;
use std::collections::BTreeMap;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const INFANTRY: &str = "10031000001211000000";

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
            let aff = ctx.metadata.affiliation.unwrap_or(Affiliation::Friend);
            st.stroke = ctx.colors.icon_color.for_affiliation(aff).cloned();
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
fn extension_icons_are_recognised_without_an_icon_stage() -> TestResult {
    let r = Renderer::default()
        .with_icons(Custom)
        .with_builtin_parts(&[milsymbol::BuiltinPart::BaseGeometry]);
    let sidc = "10031000009999000000";
    r.check_sidc(sidc)?;
    for icon in [true, false] {
        assert!(
            r.symbol(sidc)
                .with(|o| o.style.icon = icon)
                .render()?
                .is_sidc_valid()
        );
    }
    Ok(())
}

struct IncompleteIcon;

impl IconExtension for IncompleteIcon {
    fn icon(&self, _: &IconPartContext<'_>, key: IconKey<'_>, _: &dyn PartLookup) -> Option<Node> {
        matches!(
            key,
            IconKey::Entity {
                symbol_set: "10",
                entity: "999900"
            }
        )
        .then(|| Node::Group(vec![Node::Missing]))
    }
}

#[test]
fn incomplete_extension_icons_fail_sidc_validation_even_when_not_drawn() -> TestResult {
    use milsymbol::BuiltinPart;
    for parts in [&BuiltinPart::DEFAULT[..], &[BuiltinPart::BaseGeometry][..]] {
        let renderer = Renderer::default()
            .with_icons(IncompleteIcon)
            .with_builtin_parts(parts);
        let sidc = "10031000009999000000";
        assert!(renderer.check_sidc(sidc).is_err());
        for icon in [true, false] {
            let symbol = renderer
                .symbol(sidc)
                .with(|o| o.style.icon = icon)
                .render()?;
            assert!(!symbol.is_sidc_valid(), "icon={icon}");
        }
    }
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
            st.stroke = ctx
                .metadata
                .affiliation
                .and_then(|a| ctx.colors.icon_color.for_affiliation(a))
                .cloned();
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

/// Draws clipped groups with the given requested ids.
struct Clips(Vec<Option<&'static str>>);

impl SymbolPart for Clips {
    fn draw(&self, _: &SymbolState<'_>) -> Result<PartOutput, milsymbol::PartError> {
        let post = self
            .0
            .iter()
            .enumerate()
            .map(|(i, id)| {
                let d = milsymbol::ir::PathData::new(format!("M0,0 L{},0 L0,10 Z", i + 1));
                Node::clip(
                    d,
                    id.map(Cow::Borrowed),
                    vec![Node::circle(100.0, 100.0, 5.0)],
                )
            })
            .collect();
        Ok(PartOutput::new(vec![], post, PartialBBox::default()))
    }
}

fn clip_ids(svg: &str) -> Vec<String> {
    svg.split("<clipPath id=\"")
        .skip(1)
        .filter_map(|s| s.split('"').next())
        .map(String::from)
        .collect()
}

#[test]
fn clip_ids_are_unique_and_prefixable() -> TestResult {
    let r = Renderer::default().with_symbol_part(Clips(vec![
        Some("clip-custom-0"),
        None,
        Some("a b"),
        Some("a_b"),
    ]));
    let s = r.symbol(INFANTRY).render()?;
    let ids = clip_ids(&s.to_svg());
    assert_eq!(ids, ["clip-custom-0", "clip-custom-1", "a_b", "a_b-1"]);
    let mut prefixed = String::new();
    s.write_svg_with(
        &mut prefixed,
        &milsymbol::SvgOptions::default().with_id_prefix("sym7-"),
    );
    let ids = clip_ids(&prefixed);
    assert_eq!(
        ids,
        [
            "sym7-clip-custom-0",
            "sym7-clip-custom-1",
            "sym7-a_b",
            "sym7-a_b-1"
        ]
    );
    for id in &ids {
        assert!(prefixed.contains(&format!("url(#{id})")), "{id}");
    }
    Ok(())
}

#[cfg(feature = "std")]
#[test]
fn cached_preparation_keeps_going_after_an_extension_path_error() -> TestResult {
    struct MixedPaths;
    impl SymbolPart for MixedPaths {
        fn draw(&self, _: &SymbolState<'_>) -> Result<PartOutput, milsymbol::PartError> {
            Ok(PartOutput::new(
                vec![],
                vec![Node::path("M,0,0"), Node::path("M101,101 L102,102")],
                PartialBBox::default(),
            ))
        }
    }
    let cache =
        milsymbol::cache::CachedRenderer::new(Renderer::default().with_symbol_part(MixedPaths), 4)
            .with_prepared_paths();
    let options = SymbolOptions::default();
    let symbol = cache.render(INFANTRY, &options)?;
    let hit = cache.render(INFANTRY, &options)?;
    assert!(std::sync::Arc::ptr_eq(&symbol, &hit));
    let Some(Node::Path(path)) = hit.instructions().last() else {
        return Err("expected final extension path".into());
    };
    assert_eq!(path.d.source(), "M101,101 L102,102");
    assert!(matches!(path.d.segments()?, Cow::Borrowed(_)));
    Ok(())
}

/// Records the modifier keys it is asked for.
#[derive(Default)]
struct ModifierKeys(std::sync::Mutex<Vec<String>>);

impl IconExtension for ModifierKeys {
    fn icon(&self, _: &IconPartContext<'_>, key: IconKey<'_>, _: &dyn PartLookup) -> Option<Node> {
        if let IconKey::Modifier1 { code, .. } = key {
            if let Ok(mut keys) = self.0.lock() {
                keys.push(code.to_string());
            }
        }
        None
    }
}

#[test]
fn modifier_keys_match_the_complete_modifier_codes() -> TestResult {
    use milsymbol::sidc::Sidc;
    for (sidc, key, complete) in [
        ("10031000001211000700", "07", "007"),
        ("100310000012110007001000000000", "107", "107"),
    ] {
        let Sidc::Numeric(n) = Sidc::parse(sidc)? else {
            return Err("expected a numeric SIDC".into());
        };
        assert_eq!(n.modifiers().0, "07");
        assert_eq!(n.modifier_codes().0.as_str(), complete);
        let keys = std::sync::Arc::new(ModifierKeys::default());
        let r = Renderer::default().with_icons(SharedKeys(std::sync::Arc::clone(&keys)));
        r.symbol(sidc).render()?;
        let seen = keys.0.lock().map_err(|e| e.to_string())?.clone();
        assert_eq!(seen, [key], "{sidc}");
    }
    Ok(())
}

/// Lets the test read what a registered extension recorded.
struct SharedKeys(std::sync::Arc<ModifierKeys>);

impl IconExtension for SharedKeys {
    fn icon(
        &self,
        ctx: &IconPartContext<'_>,
        key: IconKey<'_>,
        parts: &dyn PartLookup,
    ) -> Option<Node> {
        self.0.icon(ctx, key, parts)
    }
}
