//! Concurrent cache misses preserve already cached requests.

#![cfg(feature = "std")]

use milsymbol::cache::CachedRenderer;
use milsymbol::options::{SymbolOptions, field};
use milsymbol::{PartError, PartOutput, Renderer, SymbolPart, SymbolState};
use std::sync::{Arc, Barrier};

struct ConcurrentMisses(Arc<Barrier>);

impl SymbolPart for ConcurrentMisses {
    fn draw(&self, state: &SymbolState<'_>) -> Result<PartOutput, PartError> {
        if state.options().text(field::UNIQUE_DESIGNATION) == "concurrent" {
            self.0.wait();
        }
        Ok(PartOutput::default())
    }
}

#[test]
fn same_key_misses_reuse_the_winner_without_evicting_other_keys()
-> Result<(), Box<dyn std::error::Error>> {
    const SIDC: &str = "10031000001211000000";
    let renderer =
        Renderer::default().with_symbol_part(ConcurrentMisses(Arc::new(Barrier::new(2))));
    let cache = Arc::new(CachedRenderer::new(renderer, 2));
    let warm = cache.render(SIDC, &SymbolOptions::default())?;
    let mut options = SymbolOptions::default();
    options.set_text(field::UNIQUE_DESIGNATION, "concurrent");
    let spawn = || {
        let cache = Arc::clone(&cache);
        let options = options.clone();
        std::thread::spawn(move || cache.render(SIDC, &options))
    };
    let first = spawn();
    let second = spawn();
    let first = first.join().map_err(|_| "first render thread panicked")??;
    let second = second
        .join()
        .map_err(|_| "second render thread panicked")??;
    assert_eq!(cache.len(), 2);
    assert!(Arc::ptr_eq(&first, &second));
    assert!(Arc::ptr_eq(
        &warm,
        &cache.render(SIDC, &SymbolOptions::default())?
    ));
    assert!(Arc::ptr_eq(&first, &cache.render(SIDC, &options)?));
    Ok(())
}
