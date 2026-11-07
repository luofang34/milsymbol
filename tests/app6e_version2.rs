//! APP-6(E) Version 2: version `16` SIDCs under `Standard::App6EVersion2`.

use milsymbol::domain::Edition;
use milsymbol::options::SymbolOptions;
use milsymbol::sidc::{Sidc, SidcCheckError, SidcError};
use milsymbol::{Renderer, Standard, ValidityIssue};

fn v2() -> Renderer {
    Renderer::builder()
        .standard(Standard::App6EVersion2)
        .build()
}

fn app6() -> Renderer {
    Renderer::builder().standard(Standard::App6).build()
}

fn svg(r: &Renderer, sidc: &str) -> Result<String, milsymbol::RenderError> {
    r.symbol(sidc).render().map(|s| s.to_svg())
}

fn field_error(sidc: &str) -> Option<(&'static str, usize)> {
    match Sidc::parse_for(sidc, Standard::App6EVersion2) {
        Err(SidcError::InvalidField {
            field, position, ..
        }) => Some((field, position)),
        _ => None,
    }
}

#[test]
fn code_edition_is_independent_of_display_standard() {
    for version in ["13", "14", "15", "16"] {
        for standard in [Standard::Mil2525, Standard::App6, Standard::App6EVersion2] {
            for rest in [
                "031000001211000000",
                "03100000121100A000",
                "336400001300000000",
            ] {
                let sidc = format!("{version}{rest}");
                assert!(
                    Sidc::parse_for(&sidc, standard).is_ok(),
                    "{sidc} {standard:?}"
                );
            }
        }
    }
    for version in ["15", "16"] {
        let sidc = format!("{version}031000001211000000");
        assert!(
            Sidc::parse_with_mode(
                &sidc,
                Standard::Mil2525,
                milsymbol::RenderMode::Milsymbol304
            )
            .is_err()
        );
    }
}

#[test]
fn version_2_fields_reject_codes_it_does_not_define() {
    // Digits 23–27 are reserved.
    assert_eq!(
        field_error("160310000012110000000010000000"),
        Some(("reserved", 23))
    );
    assert_eq!(
        field_error("160310000012110000000000100000"),
        Some(("reserved", 23))
    );
    // Hexadecimal digits are still not codes of these fields.
    assert_eq!(field_error("16A31000001211000000"), Some(("context", 3)));
    assert_eq!(field_error("1603A000001211000000"), Some(("symbol set", 5)));
    // Lower case is not hexadecimal in a SIDC.
    assert_eq!(field_error("1603100000121100a000"), Some(("character", 17)));
    // Neither current code table assigns symbol sets 12 and 39.
    for (sidc, field) in [
        ("16031200001100000000", "symbol set"),
        ("16033900001100000000", "symbol set"),
    ] {
        assert_eq!(field_error(sidc).map(|(f, _)| f), Some(field), "{sidc}");
    }
}

#[test]
fn renderer_checks_version_16_by_its_standard() {
    assert!(v2().check_sidc("16031000001211000000").is_ok());
    assert!(
        Renderer::default()
            .check_sidc("16031000001211000000")
            .is_ok()
    );
    // Well formed, but Version 2 has no special entity subtype 95.
    assert!(matches!(
        v2().check_sidc("16031000001211950000"),
        Err(SidcCheckError::Unsupported { .. })
    ));
    let strict = v2().symbol("16031000001211000000").strict().render();
    assert!(strict.is_ok());
}

#[test]
fn version_16_is_edition_e_and_ignores_digit_23() -> Result<(), milsymbol::RenderError> {
    let r = v2();
    let symbol = r.symbol("16031000001211000000").render()?;
    assert_eq!(symbol.metadata().edition, Some(Edition::E));
    assert!(symbol.sidc_validity().is_valid());
    let default = Renderer::default()
        .symbol("16031000001211000000")
        .render()?;
    assert_eq!(default.metadata().edition, Some(Edition::E));
    // APP-6 display reserves digit 23 for every numeric edition.
    let shaped = "140310000012110000000010000000";
    assert_eq!(svg(&r, shaped)?, svg(&app6(), shaped)?);
    assert_eq!(svg(&r, shaped)?, svg(&r, "140310000012110000000000000000")?);
    let reserved = r.symbol("160310000012110000000010000000").render()?;
    assert_eq!(
        reserved.to_svg(),
        svg(&r, "160310000012110000000000000000")?
    );
    assert_eq!(
        reserved.sidc_validity().issues,
        [ValidityIssue::MalformedSidc]
    );
    Ok(())
}

#[test]
fn reassigned_codes_use_the_selected_edition() -> Result<(), milsymbol::RenderError> {
    for (new, old) in [
        ("16031000001211007100", "14031000001211007100"),
        ("16034000001310000000", "14034000001310000000"),
        ("16032000000000001500", "14032000000000001500"),
        ("16036000001201000000", "14036000001201000000"),
    ] {
        let modern = v2().symbol(new).render()?;
        assert!(modern.sidc_validity().is_valid(), "{new}");
        assert_ne!(modern.to_svg(), svg(&app6(), old)?, "{new}");
    }
    // Common modifier 155 is not assigned by the current APP-6 table.
    assert_eq!(
        v2().symbol("160330000012010055001000000000")
            .render()?
            .sidc_validity()
            .issues,
        [ValidityIssue::UnknownIcon]
    );
    assert!(
        v2().symbol("1603100000121100A000")
            .render()?
            .sidc_validity()
            .is_valid()
    );
    Ok(())
}

#[test]
fn a_symbol_can_choose_version_2() -> Result<(), Box<dyn std::error::Error>> {
    let sidc = "16034000001310000000";
    let chosen = Renderer::default()
        .symbol(sidc)
        .standard(Standard::App6EVersion2)
        .render()?;
    assert_eq!(chosen.to_svg(), svg(&v2(), sidc)?);
    let mut o = SymbolOptions::default();
    o.set("standard", "APP6E2")?;
    assert_eq!(o.style.standard, Some(Standard::App6EVersion2));
    let by_name = Renderer::default().render(sidc, o)?;
    assert_eq!(by_name.to_svg(), chosen.to_svg());
    assert!(milsymbol::compat::canonical_json_string(&by_name).contains(r#""standard":"APP6E2""#));
    Ok(())
}

#[test]
fn version_2_amplifiers_match_table_a8() {
    let defined: Vec<_> = include_str!("data/app6e2_amplifiers.txt")
        .lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| line.split_once('\t').map(|(code, _)| code))
        .collect();
    assert_eq!(defined.len(), 29);
    for value in 0..=255u16 {
        let code = format!("{value:02X}");
        let sidc = format!("16031000{code}1211000000");
        assert_eq!(
            Sidc::parse_for(&sidc, Standard::App6EVersion2).is_ok(),
            defined.contains(&code.as_str()),
            "amplifier {code} must follow APP-6(E) Version 2 Table A-8"
        );
    }
}

#[test]
fn undefined_version_2_amplifier_is_rejected_by_every_validation_entry_point()
-> Result<(), milsymbol::RenderError> {
    let sidc = "16031000721211000000";
    assert_eq!(field_error(sidc), Some(("echelon/mobility", 9)));
    let renderer = v2();
    assert!(matches!(
        renderer.check_sidc(sidc),
        Err(SidcCheckError::Malformed(_))
    ));
    assert!(matches!(
        renderer.symbol(sidc).strict().render(),
        Err(milsymbol::RenderError::MalformedSidc(_))
    ));
    assert_eq!(
        renderer.symbol(sidc).render()?.sidc_validity().issues,
        [ValidityIssue::MalformedSidc]
    );
    let overridden = Renderer::default()
        .symbol(sidc)
        .standard(Standard::App6EVersion2)
        .strict()
        .render();
    assert!(matches!(
        overridden,
        Err(milsymbol::RenderError::MalformedSidc(_))
    ));
    assert!(Sidc::parse("14031000721211000000").is_ok());
    assert!(Sidc::parse_for("14031000721211000000", Standard::App6EVersion2).is_ok());
    #[cfg(feature = "std")]
    {
        let cache = milsymbol::cache::CachedRenderer::new(renderer, 4);
        assert!(cache.symbol(sidc).strict().render().is_err());
        assert!(cache.is_empty());
    }
    Ok(())
}

#[test]
fn extensions_can_supply_unassigned_version_2_icons() -> Result<(), milsymbol::RenderError> {
    struct Replacement;
    impl milsymbol::IconExtension for Replacement {
        fn icon(
            &self,
            _: &milsymbol::IconPartContext<'_>,
            key: milsymbol::IconKey<'_>,
            _: &dyn milsymbol::PartLookup,
        ) -> Option<milsymbol::ir::Node> {
            matches!(
                key,
                milsymbol::IconKey::Entity {
                    symbol_set: "40",
                    entity: "999991"
                }
            )
            .then(|| milsymbol::ir::Node::circle(100.0, 100.0, 20.0))
        }
    }
    let renderer = Renderer::builder()
        .standard(Standard::App6EVersion2)
        .icons(Replacement)
        .build();
    let sidc = "16034000009999910000";
    assert!(v2().check_sidc(sidc).is_err());
    assert!(renderer.check_sidc(sidc).is_ok());
    for visible in [true, false] {
        let symbol = renderer.symbol(sidc).icon(visible).strict().render()?;
        assert!(symbol.sidc_validity().is_valid());
        assert_eq!(symbol.to_svg().contains("<circle"), visible);
    }
    Ok(())
}
