//! Typed options, the builder entry points and strict SIDC handling.

use milsymbol::options::{
    Color, ColorChoice, ColorError, ColorModeChoice, OptionValue, SymbolOptions, TextField,
};
use milsymbol::sidc::Sidc;
use milsymbol::{RenderError, Renderer, Standard};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const INFANTRY: &str = "10031000161211000000";

#[test]
fn text_fields_round_trip_their_names() {
    for f in TextField::STANDARD {
        assert_eq!(&TextField::from_name(f.name()), f, "{}", f.name());
    }
    assert_eq!(TextField::STANDARD.len(), 29);
    let custom = TextField::custom("dtg1");
    assert_eq!(custom.name(), "dtg1");
    assert!(matches!(custom, TextField::Custom(_)));
    // A standard name never becomes a custom field.
    assert_eq!(TextField::custom("quantity"), TextField::Quantity);
}

#[test]
fn text_values_are_set_and_read_by_field() {
    let mut o = SymbolOptions::default();
    assert_eq!(o.text(&TextField::Quantity), "");
    o.set_text(TextField::Quantity, "200")
        .set_text("dtg1", "D1")
        .set_text("type", "T");
    assert_eq!(o.text(&TextField::Quantity), "200");
    assert_eq!(o.text_named("dtg1"), "D1");
    assert_eq!(o.text(&TextField::Type), "T");
    let names: Vec<String> = o.text_fields().map(|(f, _)| f.name().to_string()).collect();
    assert_eq!(names, ["dtg1", "quantity", "type"]);
}

#[test]
fn colors_reject_only_the_empty_string() {
    assert_eq!(Color::new(""), Err(ColorError::Empty));
    assert_eq!(Color::rgb(1, 2, 3).as_str(), "rgb(1,2,3)");
    // The text is kept as written.
    assert_eq!(
        Color::new("rgb(20, 60, 160)").map(|c| c.to_string()),
        Ok("rgb(20, 60, 160)".into())
    );
    assert!(Color::try_from("#f00").is_ok());
}

#[test]
fn set_maps_empty_strings_to_not_set() -> TestResult {
    let mut o = SymbolOptions::default();
    o.set("frameColor", "red")?
        .set("monoColor", "blue")?
        .set("fillColor", "pink")?;
    assert!(matches!(o.style.frame_color, Some(ColorChoice::Uniform(_))));
    assert!(o.style.mono_color.is_some());
    o.set("frameColor", "")?
        .set("monoColor", "")?
        .set("fillColor", "")?;
    assert!(
        o.style.frame_color.is_none()
            && o.style.mono_color.is_none()
            && o.style.fill_color.is_none()
    );
    o.set("hqStaffLength", 30.0)?.set("speedLeader", 20.0)?;
    assert_eq!(
        (o.style.hq_staff_length, o.speed_leader),
        (Some(30.0), Some(20.0))
    );
    assert!(o.set("colorMode", OptionValue::Bool(true)).is_err());
    o.set("colorMode", "Dark")?;
    assert_eq!(o.style.color_mode, ColorModeChoice::named("Dark"));
    Ok(())
}

#[test]
fn non_finite_numbers_are_typed_errors() -> TestResult {
    let r = Renderer::default();
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        type Setter = fn(milsymbol::SymbolBuilder<'_>, f64) -> milsymbol::SymbolBuilder<'_>;
        let setters: [Setter; 4] = [
            |b, v| b.size(v),
            |b, v| b.direction(v),
            |b, v| b.speed_leader(v),
            |b, v| b.stack(v),
        ];
        for setter in setters {
            let err = setter(r.symbol(INFANTRY), bad).render();
            assert!(
                matches!(err, Err(RenderError::InvalidOption { .. })),
                "{bad}: {err:?}"
            );
        }
        let err = r
            .symbol(INFANTRY)
            .with(|o| o.style.stroke_width = bad)
            .render();
        assert!(matches!(
            err,
            Err(RenderError::InvalidOption {
                name: "strokeWidth",
                ..
            })
        ));
    }
    Ok(())
}

#[test]
fn builder_setters_equal_direct_options() -> TestResult {
    let r = Renderer::default();
    let built = r
        .symbol(INFANTRY)
        .size(50.0)
        .direction(30.0)
        .speed_leader(40.0)
        .stack(2.0)
        .text(TextField::UniqueDesignation, "A")
        .color_mode(ColorModeChoice::named("Dark"))
        .mono_color(Color::new("blue")?)
        .standard(Standard::App6)
        .render()?;
    let mut o = SymbolOptions::default();
    o.style.size = 50.0;
    o.direction = Some(30.0);
    o.speed_leader = Some(40.0);
    o.stack = Some(2.0);
    o.set_text(TextField::UniqueDesignation, "A");
    o.style.color_mode = ColorModeChoice::named("Dark");
    o.style.mono_color = Some(Color::new("blue")?);
    o.style.standard = Some(Standard::App6);
    let direct = r.render(INFANTRY, o.clone())?;
    assert_eq!(built.to_svg(), direct.to_svg());
    let via_options = r.symbol(INFANTRY).options(o).render()?;
    assert_eq!(built.to_svg(), via_options.to_svg());
    Ok(())
}

#[test]
fn a_symbol_can_start_from_text_or_a_parsed_sidc() -> TestResult {
    let r = Renderer::default();
    let text = r.symbol(INFANTRY).render()?.to_svg();
    let owned = r.symbol(String::from(INFANTRY)).render()?.to_svg();
    let borrowed = String::from(INFANTRY);
    let by_ref = r.symbol(&borrowed).render()?.to_svg();
    let parsed = Sidc::parse(INFANTRY)?;
    let by_sidc = r.symbol(&parsed).render()?.to_svg();
    assert!([owned, by_ref, by_sidc].iter().all(|s| *s == text));
    Ok(())
}

#[test]
fn strict_rejects_what_lenient_draws() -> TestResult {
    let r = Renderer::default();
    for bad in ["10091000001211000000", "not a sidc", ""] {
        assert!(
            r.symbol(bad).render().is_ok(),
            "{bad}: lenient rendering draws a ? icon"
        );
        assert!(
            matches!(
                r.symbol(bad).strict().render(),
                Err(RenderError::MalformedSidc(_))
            ),
            "{bad}"
        );
    }
    assert!(matches!(
        r.symbol("10031000009999990000").strict().render(),
        Err(RenderError::UnsupportedSidc { .. })
    ));
    let strict = r.symbol(INFANTRY).strict().render()?;
    assert_eq!(strict.to_svg(), r.symbol(INFANTRY).render()?.to_svg());
    Ok(())
}

#[test]
fn renderer_clones_share_configuration() -> TestResult {
    let app6 = Renderer::builder().standard(Standard::App6).build();
    let copy = app6.clone();
    assert_eq!(copy.config().standard, Standard::App6);
    assert_eq!(
        app6.symbol("SFGPUCI-----").render()?.to_svg(),
        copy.symbol("SFGPUCI-----").render()?.to_svg()
    );
    fn assert_send_sync<T: Send + Sync + Clone>() {}
    assert_send_sync::<Renderer>();
    assert_send_sync::<milsymbol::Symbol>();
    Ok(())
}

#[test]
fn builder_calls_apply_in_order() -> TestResult {
    use milsymbol::BuiltinPart;
    let frame_only = Renderer::builder()
        .pipeline(&[BuiltinPart::BaseGeometry])
        .build();
    let with_octagon = Renderer::builder()
        .pipeline(&[BuiltinPart::BaseGeometry])
        .octagon()
        .build();
    let replaced = Renderer::builder()
        .octagon()
        .pipeline(&[BuiltinPart::BaseGeometry])
        .build();
    let plain = frame_only.symbol(INFANTRY).render()?.to_svg();
    assert_ne!(plain, with_octagon.symbol(INFANTRY).render()?.to_svg());
    assert_eq!(plain, replaced.symbol(INFANTRY).render()?.to_svg());
    Ok(())
}

#[cfg(feature = "std")]
#[test]
fn cached_builder_shares_symbols() -> TestResult {
    use milsymbol::cache::CachedRenderer;
    let cache = CachedRenderer::new(Renderer::default(), 8);
    let a = cache.symbol(INFANTRY).size(40.0).render()?;
    let b = cache.symbol(INFANTRY).size(40.0).render()?;
    let c = cache.symbol(INFANTRY).size(41.0).render()?;
    assert!(std::sync::Arc::ptr_eq(&a, &b));
    assert!(!std::sync::Arc::ptr_eq(&a, &c));
    Ok(())
}
