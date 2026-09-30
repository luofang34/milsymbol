//! Property tests: arbitrary input never panics and rendering is deterministic.

use milsymbol::Renderer;
use milsymbol::options::{Color, OptionValue, SymbolOptions, TextField};
use proptest::prelude::*;
use proptest::test_runner::TestCaseError;

fn fail(e: milsymbol::RenderError) -> TestCaseError {
    TestCaseError::fail(e.to_string())
}

fn numeric_sidc() -> impl Strategy<Value = String> {
    (
        prop::sample::select(vec!["10", "11", "12", "13", "14", "15", "99"]),
        "[0-3]",
        "[0-8]",
        prop::sample::select(vec![
            "01", "02", "05", "06", "10", "11", "15", "20", "25", "27", "30", "35", "36", "40",
            "50", "51", "52", "53", "54", "60", "00", "77",
        ]),
        "[0-9]{2}",
        "[0-9]{2}",
        "[0-9]{10}",
        "[0-9A]{0,4}",
    )
        .prop_map(|(v, c, si, ss, st, em, ent, tail)| format!("{v}{c}{si}{ss}{st}{em}{ent}{tail}"))
}

fn letter_sidc() -> impl Strategy<Value = String> {
    "[SGWIOE][A-Z*-][A-Z-][A-Z-][A-Z0-9-]{0,11}"
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    #[test]
    fn numeric_sidcs_never_panic(sidc in numeric_sidc()) {
        let r = Renderer::default();
        let a = r.symbol(&sidc).render().map_err(fail)?;
        let b = r.symbol(&sidc).render().map_err(fail)?;
        prop_assert_eq!(a.to_svg(), b.to_svg());
        prop_assert!(a.size().width.is_finite());
    }

    #[test]
    fn letter_sidcs_never_panic(sidc in letter_sidc()) {
        let s = Renderer::default().symbol(&sidc).render().map_err(fail)?;
        s.validity().is_valid();
        prop_assert!(s.to_svg().ends_with("</svg>"));
    }

    #[test]
    fn strict_parse_never_panics(sidc in "\\PC{0,40}") {
        milsymbol::sidc::Sidc::parse(&sidc).ok();
    }

    #[test]
    fn arbitrary_strings_never_panic(sidc in "\\PC{0,40}") {
        Renderer::default().symbol(&sidc).render().map(|s| s.to_svg()).ok();
    }

    #[test]
    fn drawing_agrees_with_svg(
        sidc in numeric_sidc(),
        size in 1.0f64..500.0,
        direction in prop::option::of(0.0f64..360.0),
        speed in 0.0f64..200.0,
        text in "\\PC{0,12}",
    ) {
        let mut b = Renderer::default().symbol(&sidc).size(size).speed_leader(speed).text(TextField::UniqueDesignation, text);
        if let Some(d) = direction {
            b = b.direction(d);
        }
        let s = b.render().map_err(fail)?;
        let svg = s.to_svg();
        let drawing = s.drawing();
        let circles = drawing.items.iter().filter(|i| matches!(i.shape, milsymbol::drawing::Shape::Circle { .. })).count();
        prop_assert_eq!(circles, svg.matches("<circle").count());
        prop_assert_eq!(drawing.invalid_paths, 0);
    }

    #[test]
    fn option_names_and_values_never_panic(
        name in "[a-zA-Z_]{0,24}",
        text in "\\PC{0,12}",
        number in prop::num::f64::ANY,
        flag in any::<bool>(),
    ) {
        for value in [OptionValue::Str(text.clone()), OptionValue::Num(number), OptionValue::Bool(flag)] {
            let mut o = SymbolOptions::default();
            if o.set(&name, value).is_ok() {
                Renderer::default().render("10031000161211000000", o).map(|s| s.to_svg()).ok();
            }
        }
    }

    #[test]
    fn arbitrary_options_never_panic(
        sidc in numeric_sidc(),
        size in prop::num::f64::ANY,
        stroke in -5.0f64..20.0,
        outline in 0.0f64..5.0,
        direction in prop::option::of(prop::num::f64::ANY),
        speed in prop::num::f64::ANY,
        stack in prop::option::of(prop_oneof![-2.0f64..5.0, prop::num::f64::ANY]),
        text in "\\PC{0,12}",
        flags in prop::array::uniform6(any::<bool>()),
    ) {
        let mut o = SymbolOptions::default();
        o.style.size = size;
        o.style.stroke_width = stroke;
        o.style.outline_width = outline;
        let [fill, frame, icon, square, info_fields, mono] = flags;
        o.style.fill = fill;
        o.style.frame = frame;
        o.style.icon = icon;
        o.style.square = square;
        o.style.info_fields = info_fields;
        if mono { o.style.mono_color = Color::new("black").ok(); }
        o.direction = direction;
        o.speed_leader = Some(speed);
        o.stack = stack;
        for k in [TextField::UniqueDesignation, TextField::Quantity, TextField::EngagementBar, TextField::StaffComments] {
            o.set_text(k, text.clone());
        }
        Renderer::default().render(&sidc, o).map(|s| (s.to_svg(), milsymbol::compat::canonical_json_string(&s))).ok();
    }
}
