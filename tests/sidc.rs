//! SIDC parsing, typed metadata and validity.

use milsymbol::ValidityIssue;
use milsymbol::options::TextField;
use milsymbol::{Renderer, catalog};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const INFANTRY: &str = "10031000001211000000";

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
        .text(TextField::UniqueDesignation, "null value")
        .render()?;
    assert!(!milsymbol::compat::is_valid(&s));
    assert!(s.validity().is_valid());
    assert_eq!(
        milsymbol::compat::validity(&s).issues,
        vec![ValidityIssue::NullInDrawing]
    );
    let s = Renderer::default()
        .symbol("10031000009999000000")
        .render()?;
    assert_eq!(s.validity().issues, vec![ValidityIssue::UnknownIcon]);
    assert!(!s.validity().is_valid());
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
    assert_eq!(field_of("SFGPUCI---Z"), Some("symbol modifier"));
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
        assert!(!s.validity().is_valid(), "icon={icon}");
    }
    let hidden = Renderer::default()
        .symbol("10031000009999990000")
        .with(|o| o.style.icon = false)
        .render()?;
    assert!(
        milsymbol::compat::is_valid(&hidden),
        "upstream treats hidden icons as found"
    );
    assert!(!hidden.validity().is_valid());
    Ok(())
}

#[test]
fn sidc_validation_checks_icons_without_an_icon_stage() -> TestResult {
    use milsymbol::{BuiltinPart, sidc::SidcCheckError};
    for parts in [&[][..], &[BuiltinPart::BaseGeometry][..]] {
        let renderer = Renderer::builder().pipeline(parts).build();
        assert!(renderer.check_sidc(INFANTRY).is_ok());
        for sidc in [
            "10031000009999990000",
            "100310000012110099009000000000",
            "SFGPZZZZZZ--",
        ] {
            for icon in [true, false] {
                let s = renderer
                    .symbol(sidc)
                    .with(|o| o.style.icon = icon)
                    .render()?;
                assert!(!s.validity().is_valid(), "{sidc}, icon={icon}");
            }
            assert!(matches!(renderer.check_sidc(sidc),
                Err(SidcCheckError::Unsupported { issues })
                    if issues.contains(&ValidityIssue::UnknownIcon)));
        }
    }
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
        assert!(!s.validity().is_valid(), "{sidc}");
        assert!(s.validity().issues.contains(&ValidityIssue::MalformedSidc));
        assert!(
            !milsymbol::compat::validity(&s)
                .issues
                .contains(&ValidityIssue::MalformedSidc)
        );
    }
    Ok(())
}

#[test]
#[allow(deprecated)]
fn sidc_validity_is_the_same_as_validity() -> TestResult {
    for sidc in [INFANTRY, "10031000009999990000", "10070100001100000000"] {
        let s = Renderer::default().symbol(sidc).render()?;
        assert_eq!(s.sidc_validity(), s.validity(), "{sidc}");
    }
    Ok(())
}

#[test]
fn native_metadata_preserves_compatibility_sentinels() -> TestResult {
    let r = Renderer::default();
    for (sidc, affiliation, dimension) in [
        ("SZGPUCI-----", Some("undefined"), "Ground"),
        ("10091000001211000000", None, "Ground"),
        ("10130000000000000000", Some(""), "Sea"),
        ("SDZPUCI-----", Some("none"), "Ground"),
        ("SFQPUCI-----", Some("Friend"), "undefined"),
        ("10034400000000000000", Some("Friend"), ""),
    ] {
        let s = r.symbol(sidc).render()?;
        let js = milsymbol::compat::js_metadata(&s);
        assert_eq!(js.affiliation, affiliation, "{sidc}");
        assert_eq!(js.dimension, dimension, "{sidc}");
        // The typed view names a known affiliation exactly as the JS view.
        let typed = s.metadata().affiliation.map(|a| a.as_str());
        assert_eq!(
            typed,
            affiliation.filter(|a| !["", "undefined", "none"].contains(a)),
            "{sidc}"
        );
    }
    Ok(())
}

#[test]
fn non_bmp_sidcs_render_like_upstream_with_lossy_metadata() -> TestResult {
    // milsymbol.js 3.0.4 output for these inputs (tools/oracle/render.mjs).
    let expected = include_str!("data/unicode_oracle.txt");
    let face = '\u{1F600}';
    let sidcs = [
        format!("10031000001211000000{face}"),
        format!("100310000012110000001{face}"),
        format!("S{face}GPUCI----"),
    ];
    assert_eq!(expected.lines().count(), sidcs.len());
    for (sidc, want) in sidcs.iter().zip(expected.lines()) {
        let s = Renderer::default().symbol(sidc).render()?;
        assert_eq!(s.to_svg(), want, "{sidc}");
    }
    // Upstream splits the surrogate pair into its halves; Rust strings
    // cannot hold one, so each half reads as U+FFFD (UPSTREAM.md).
    let s = Renderer::default().symbol(&sidcs[0]).render()?;
    let js = milsymbol::compat::js_metadata(&s);
    assert_eq!(js.flags.modifier1, Some("\u{FFFD}00"));
    assert_eq!(js.flags.modifier2, Some("\u{FFFD}00"));
    Ok(())
}
