//! Random symbols are valid for the renderer that asked for them.
use super::sidc;
use milsymbol::{RenderMode, Renderer, Standard};
use std::collections::BTreeSet;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn cases() -> Vec<(u8, Standard, RenderMode)> {
    let mut cases = Vec::new();
    for version in [10, 11, 12, 13, 14, 15, 16] {
        for standard in [Standard::Mil2525, Standard::App6] {
            cases.push((version, standard, RenderMode::Standards));
            if version <= 14 {
                cases.push((version, standard, RenderMode::Milsymbol304));
            }
        }
    }
    cases
}

#[test]
fn random_symbols_are_valid_and_varied() -> TestResult {
    for (version, standard, mode) in cases() {
        let renderer = Renderer::builder().standard(standard).mode(mode).build();
        let mut seen = BTreeSet::new();
        for seed in 0..40 {
            let code = sidc(version, standard, mode, seed)?;
            assert!(code.starts_with(&format!("{version:02}")), "{code}");
            let symbol = renderer.symbol(&renderer.check_sidc(&code)?).render()?;
            assert!(
                symbol.sidc_validity().is_valid(),
                "{code} {standard:?} {mode:?}: {:?}",
                symbol.sidc_validity().issues
            );
            seen.insert(code);
        }
        assert!(
            seen.len() >= 30,
            "{version} {standard:?} {mode:?}: {seen:?}"
        );
    }
    Ok(())
}

#[test]
fn random_symbols_repeat_for_a_seed() -> TestResult {
    let pick = |seed| sidc(16, Standard::App6, RenderMode::Standards, seed);
    assert_eq!(pick(7)?, pick(7)?);
    assert_ne!(pick(7)?, pick(8)?);
    Ok(())
}

#[test]
fn versions_the_mode_cannot_draw_are_refused() {
    assert!(sidc(16, Standard::App6, RenderMode::Milsymbol304, 1).is_err());
}
