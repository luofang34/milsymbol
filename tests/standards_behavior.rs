//! Observable composition rules of the pinned standards.
use milsymbol::catalog::standards::{self, CodeKind};
use milsymbol::options::{ColorModeChoice, SymbolOptions};
use milsymbol::{RenderMode, Renderer, Standard};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn leadership_is_drawn_for_every_identity_and_numeric_edition() -> TestResult {
    for standard in [Standard::Mil2525, Standard::App6] {
        let r = Renderer::builder().standard(standard).build();
        for version in 10..=16 {
            for identity in [1, 3, 4, 6] {
                let leader = r
                    .symbol(format!("{version}0{identity}2700710000000000"))
                    .render()?;
                let bare = r
                    .symbol(format!("{version}0{identity}2700000000000000"))
                    .render()?;
                assert!(leader.bounding_box().y1 < bare.bounding_box().y1);
                assert!(leader.drawing().items.len() > bare.drawing().items.len());
                assert_eq!(leader.drawing().invalid_paths, 0);
            }
        }
    }
    Ok(())
}

#[test]
fn display_standard_selects_suspect_and_default_fill_colors() -> TestResult {
    for version in 10..=16 {
        let mil = Renderer::default();
        let app = Renderer::builder().standard(Standard::App6).build();
        let sidc = format!("{version}051000001211000000");
        assert!(
            mil.symbol(sidc.as_str())
                .render()?
                .to_svg()
                .contains("rgb(255, 229, 153)")
        );
        assert!(
            app.symbol(sidc.as_str())
                .render()?
                .to_svg()
                .contains("rgb(255,128,128)")
        );
        let friend = format!("{version}031000001211000000");
        assert!(
            mil.symbol(friend.as_str())
                .render()?
                .to_svg()
                .contains("rgb(128,225,255)")
        );
        assert!(
            app.symbol(friend.as_str())
                .render()?
                .to_svg()
                .contains("rgb(128,224,255)")
        );
        let neutral = format!("{version}041000001211000000");
        let symbol = mil
            .symbol(neutral)
            .with(|o| o.style.color_mode = ColorModeChoice::Named("Medium".into()))
            .render()?;
        assert!(symbol.to_svg().contains("rgb(0,226,0)"));
    }
    Ok(())
}

#[test]
fn standard_graphic_inputs_are_live_escaped_and_optional() -> TestResult {
    let r = Renderer::default();
    let mut count = 0;
    for version in [15, 16] {
        for entry in standards::entries(version).filter(|e| e.kind == CodeKind::Entity) {
            let Some(field) = entry.input_field() else {
                continue;
            };
            count += 1;
            let sidc = format!("{version}03{}0000{}0000", entry.symbol_set, entry.code);
            let mut options = SymbolOptions::default();
            options.set_text(field, "<&320");
            let svg = r
                .symbol(sidc.as_str())
                .options(options.clone())
                .render()?
                .to_svg();
            assert!(svg.contains("&lt;&amp;320"), "{sidc}: {field}");
            assert!(!svg.contains("<&320"));
            options.style.info_fields = false;
            assert!(
                !r.symbol(sidc.as_str())
                    .options(options)
                    .render()?
                    .to_svg()
                    .contains("&lt;&amp;320")
            );
        }
    }
    assert!(count >= 40);
    Ok(())
}

#[test]
fn app_us_only_sets_delegate_to_the_pinned_mil_codes() -> TestResult {
    let r = Renderer::builder().standard(Standard::App6).build();
    for entry in standards::entries(15).filter(|e| {
        matches!(
            e.symbol_set,
            "06" | "45" | "46" | "47" | "50" | "51" | "52" | "53" | "54"
        )
    }) {
        let alias = standards::entry(16, entry.symbol_set, entry.kind, entry.code)
            .ok_or("missing US-only alias")?;
        assert_eq!(entry.name, alias.name);
        assert_eq!(entry.scope, alias.scope);
    }
    assert!(r.check_sidc("16034500001101020000").is_ok());
    Ok(())
}

#[test]
fn explicit_compatibility_mode_keeps_upstream_frames_and_colors() -> TestResult {
    let r = Renderer::builder().mode(RenderMode::Milsymbol304).build();
    let symbol = r.symbol("10031000001211000000").render()?;
    assert!(symbol.to_svg().contains("rgb(128,224,255)"));
    assert!(r.check_sidc("16031000001211000000").is_err());
    Ok(())
}

#[test]
fn exercise_jokers_are_hostile_red_in_every_exercise_context() -> TestResult {
    for version in 13..=16 {
        for context in [1, 5, 6] {
            let sidc = format!("{version}{context}51000001211000000");
            let symbol = Renderer::default().symbol(sidc).render()?;
            assert!(symbol.to_svg().contains("rgb(255,128,128)"));
            assert!(!symbol.to_svg().contains("fill=\"rgb(255, 229, 153)\""));
        }
    }
    Ok(())
}
