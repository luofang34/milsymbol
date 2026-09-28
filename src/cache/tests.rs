use super::CachedRenderer;
use crate::options::SymbolOptions;
use crate::renderer::Renderer;
use std::sync::Arc;

#[test]
fn cached_renderer_can_be_shared_across_threads() {
    fn shareable<T: Send + Sync>() {}
    shareable::<CachedRenderer>();
    shareable::<Renderer>();
    shareable::<crate::Symbol>();
}

#[test]
fn clock_eviction_keeps_recently_used_symbols() -> Result<(), crate::RenderError> {
    let cache = CachedRenderer::new(Renderer::default(), 2);
    let o = SymbolOptions::default();
    let (a, b, c) = (
        "10031000001211000000",
        "10061000001211000000",
        "10041000001211000000",
    );
    let first_a = cache.render(a, &o)?;
    let first_b = cache.render(b, &o)?;
    // Using `a` again protects it from the next eviction.
    assert!(Arc::ptr_eq(&first_a, &cache.render(a, &o)?));
    cache.render(c, &o)?;
    assert_eq!(cache.len(), 2);
    assert!(Arc::ptr_eq(&first_a, &cache.render(a, &o)?), "a kept");
    assert!(!Arc::ptr_eq(&first_b, &cache.render(b, &o)?), "b evicted");
    assert_eq!(cache.len(), 2);
    Ok(())
}
