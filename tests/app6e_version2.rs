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
fn version_16_parses_only_under_version_2_rules() {
    for sidc in [
        "16031000001211000000",
        "1603100000121100A000",
        "160310000012110000001000000123",
        "160325000013030000000000000000",
    ] {
        assert!(Sidc::parse(sidc).is_err(), "{sidc}");
        assert!(Sidc::parse_for(sidc, Standard::App6).is_err(), "{sidc}");
        assert!(
            Sidc::parse_for(sidc, Standard::App6EVersion2).is_ok(),
            "{sidc}"
        );
    }
    // Other versions parse as before.
    for sidc in ["14031000001211000000", "140310000012110000000010000000"] {
        assert_eq!(
            Sidc::parse(sidc).ok(),
            Sidc::parse_for(sidc, Standard::App6EVersion2).ok()
        );
    }
    assert!(Sidc::parse_for("1403100000121100A000", Standard::App6EVersion2).is_err());
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
    // No symbol sets 12 and 39; no icons for 64; no restricted-target context.
    for (sidc, field) in [
        ("16031200001100000000", "symbol set"),
        ("16033900001100000000", "symbol set"),
        ("16036400001300000000", "symbol set"),
        ("16331000001211000000", "context"),
    ] {
        assert_eq!(field_error(sidc).map(|(f, _)| f), Some(field), "{sidc}");
    }
}

#[test]
fn renderer_checks_version_16_by_its_standard() {
    assert!(v2().check_sidc("16031000001211000000").is_ok());
    assert!(matches!(
        Renderer::default().check_sidc("16031000001211000000"),
        Err(SidcCheckError::Malformed(_))
    ));
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
    assert_eq!(default.metadata().edition, None);
    // Digit 23 is a frame shape in version 14, reserved in version 16.
    let shaped = "140310000012110000000010000000";
    assert_eq!(svg(&r, shaped)?, svg(&app6(), shaped)?);
    assert_ne!(svg(&r, shaped)?, svg(&r, "140310000012110000000000000000")?);
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
fn codes_version_2_changed_draw_as_unknown() -> Result<(), milsymbol::RenderError> {
    let r = v2();
    for (v16, v14) in [
        // Land unit sector 1 modifier 71: accident in APP-6(D).
        ("16031000001211007100", "14031000001211007100"),
        // Activities 131000: a meeting before Version 2.
        ("16034000001310000000", "14034000001310000000"),
        // Common sector 1 modifier 155, removed by Version 2.
        (
            "160330000012010055001000000000",
            "140330000012010055001000000000",
        ),
    ] {
        let issues = r.symbol(v16).render()?.sidc_validity().issues;
        assert_eq!(issues, [ValidityIssue::UnknownIcon], "{v16}");
        assert!(
            app6().symbol(v14).render()?.sidc_validity().is_valid(),
            "{v14}"
        );
    }
    // A hexadecimal sector 1 modifier this crate has no icon for.
    let hex = r.symbol("1603100000121100A000").render()?;
    assert_eq!(hex.sidc_validity().issues, [ValidityIssue::UnknownIcon]);
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
