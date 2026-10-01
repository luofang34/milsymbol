//! The cached and uncached builders have the same setters and the same
//! validation, `strict` included.

#![cfg(feature = "std")]

use milsymbol::cache::CachedRenderer;
use milsymbol::ir::Node;
use milsymbol::options::TextField;
use milsymbol::sidc::SidcCheckError;
use milsymbol::{
    IconExtension, IconKey, IconPartContext, PartLookup, RenderError, Renderer, ValidityIssue,
};
use proptest::prelude::*;
use std::sync::Arc;

const INFANTRY: &str = "10031000161211000000";
const MALFORMED: &str = "10091000001211000000";
const NO_ENTITY: &str = "10031000009999990000";
const EXTENDED: &str = "10031000009999000000";

struct Extra;

impl IconExtension for Extra {
    fn icon(&self, _: &IconPartContext<'_>, key: IconKey<'_>, p: &dyn PartLookup) -> Option<Node> {
        let IconKey::Entity {
            symbol_set: "10",
            entity: "999900",
        } = key
        else {
            return None;
        };
        p.part("GR.IC.FF.INFANTRY")
    }
}

fn kind(r: &Result<impl Sized, RenderError>) -> &'static str {
    match r {
        Ok(_) => "ok",
        Err(RenderError::MalformedSidc(_)) => "malformed",
        Err(RenderError::UnsupportedSidc { .. }) => "unsupported",
        Err(_) => "other",
    }
}

#[test]
fn strict_on_the_cache_matches_the_uncached_builder() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = Renderer::default();
    let cache = CachedRenderer::new(renderer.clone(), 8);
    for (sidc, expected) in [
        (INFANTRY, "ok"),
        (MALFORMED, "malformed"),
        (NO_ENTITY, "unsupported"),
    ] {
        let plain = renderer.symbol(sidc).strict().render();
        let cached = cache.symbol(sidc).strict().render();
        assert_eq!(kind(&plain), expected, "{sidc}");
        assert_eq!(kind(&cached), expected, "{sidc}");
        if let (
            Err(RenderError::UnsupportedSidc { issues: a }),
            Err(RenderError::UnsupportedSidc { issues: b }),
        ) = (&plain, &cached)
        {
            assert_eq!(a, b);
            assert_eq!(a, &[ValidityIssue::UnknownIcon]);
        }
    }
    Ok(())
}

#[test]
fn strict_failures_are_never_cached_and_lenient_entries_are_not_reused_by_strict()
-> Result<(), Box<dyn std::error::Error>> {
    let cache = CachedRenderer::new(Renderer::default(), 8);
    let lenient = cache.symbol(NO_ENTITY).render()?;
    assert_eq!(cache.len(), 1);
    assert!(cache.symbol(NO_ENTITY).strict().render().is_err());
    assert!(cache.symbol(NO_ENTITY).strict().render().is_err());
    assert_eq!(cache.len(), 1, "a strict failure stores nothing");
    assert!(Arc::ptr_eq(&lenient, &cache.symbol(NO_ENTITY).render()?));

    let strict = cache.symbol(INFANTRY).strict().render()?;
    assert!(Arc::ptr_eq(
        &strict,
        &cache.symbol(INFANTRY).strict().render()?
    ));
    assert_eq!(strict.to_svg(), cache.symbol(INFANTRY).render()?.to_svg());
    Ok(())
}

#[test]
fn strict_on_the_cache_sees_extension_icons() -> Result<(), Box<dyn std::error::Error>> {
    let extended = Renderer::builder().icons(Extra).build();
    let cache = CachedRenderer::new(extended.clone(), 8);
    assert!(cache.symbol(EXTENDED).strict().render().is_ok());
    assert!(extended.check_sidc(EXTENDED).is_ok());
    let plain = CachedRenderer::new(Renderer::default(), 8);
    assert!(plain.symbol(EXTENDED).strict().render().is_err());
    assert!(matches!(
        Renderer::default().check_sidc(EXTENDED),
        Err(SidcCheckError::Unsupported { .. })
    ));
    Ok(())
}

fn sidc() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(INFANTRY.to_owned()),
        Just(MALFORMED.to_owned()),
        Just(NO_ENTITY.to_owned()),
        Just(EXTENDED.to_owned()),
        "[0-9]{20}",
        "[SGWIOE][A-Z*-][A-Z-][A-Z-][A-Z0-9-]{0,11}",
        "\\PC{0,24}",
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1500))]

    #[test]
    fn cached_and_uncached_builders_agree(
        sidc in sidc(),
        strict in any::<bool>(),
        extended in any::<bool>(),
        size in prop::sample::select(vec![20.0, 35.0, 100.0]),
        text in "[ -~]{0,6}",
        icon in any::<bool>(),
        font in prop::sample::select(vec!["", "Courier New"]),
        capacity in prop::sample::select(vec![0usize, 1, 16]),
    ) {
        let renderer = if extended {
            Renderer::builder().icons(Extra).build()
        } else {
            Renderer::default()
        };
        let cache = CachedRenderer::new(renderer.clone(), capacity);
        for _ in 0..2 {
            let mut plain = renderer.symbol(&sidc).with(|o| o.style.icon = icon).size(size).text(TextField::UniqueDesignation, text.clone());
            let mut cached = cache.symbol(&sidc).with(|o| o.style.icon = icon).size(size).text(TextField::UniqueDesignation, text.clone());
            if !font.is_empty() {
                plain = plain.font(font);
                cached = cached.font(font);
            }
            if strict {
                plain = plain.strict();
                cached = cached.strict();
            }
            let (plain, cached) = (plain.render(), cached.render());
            prop_assert_eq!(kind(&plain), kind(&cached));
            if let (Ok(a), Ok(b)) = (&plain, &cached) {
                prop_assert_eq!(a.to_svg(), b.to_svg());
            }
            if let (Err(a), Err(b)) = (&plain, &cached) {
                prop_assert_eq!(a.to_string(), b.to_string());
            }
        }
    }
}

#[test]
fn concurrent_strict_and_lenient_requests_match_uncached() -> Result<(), Box<dyn std::error::Error>>
{
    let renderer = Renderer::builder().icons(Extra).build();
    let cache = Arc::new(CachedRenderer::new(renderer.clone(), 3));
    let sidcs = [INFANTRY, MALFORMED, NO_ENTITY, EXTENDED];
    let handles: Vec<_> = (0..8usize)
        .map(|t| {
            let (cache, renderer) = (Arc::clone(&cache), renderer.clone());
            std::thread::spawn(move || -> Result<(), String> {
                for i in 0..200usize {
                    let sidc = sidcs
                        .get((t + i) % sidcs.len())
                        .copied()
                        .unwrap_or(INFANTRY);
                    let strict = (t + i / 2) % 2 == 0;
                    let (mut plain, mut cached) = (renderer.symbol(sidc), cache.symbol(sidc));
                    if strict {
                        (plain, cached) = (plain.strict(), cached.strict());
                    }
                    match (plain.render(), cached.render()) {
                        (Ok(a), Ok(b)) if a.to_svg() == b.to_svg() => {}
                        (Err(a), Err(b)) if a.to_string() == b.to_string() => {}
                        _ => return Err(format!("{sidc} strict={strict} differs")),
                    }
                }
                Ok(())
            })
        })
        .collect();
    for h in handles {
        h.join().map_err(|_| "thread panicked")??;
    }
    assert!(cache.len() <= 3);
    Ok(())
}

#[test]
fn a_strict_and_a_lenient_request_occupy_two_entries() -> Result<(), Box<dyn std::error::Error>> {
    let cache = CachedRenderer::new(Renderer::default(), 8);
    cache.symbol(INFANTRY).render()?;
    assert_eq!(cache.len(), 1);
    cache.symbol(INFANTRY).strict().render()?;
    assert_eq!(cache.len(), 2);
    cache.symbol(INFANTRY).render()?;
    cache.symbol(INFANTRY).strict().render()?;
    assert_eq!(cache.len(), 2);
    Ok(())
}
