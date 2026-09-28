//! Allocation guards for observers and buffer reuse. This integration test
//! has its own process so other tests cannot contaminate allocator counts.

use milsymbol::Renderer;
use milsymbol::compat;
use milsymbol::domain::Affiliation;
#[cfg(feature = "std")]
use milsymbol::options::SymbolOptions;
use milsymbol::options::field;
use std::hint::black_box;

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

#[test]
fn prepared_observers_and_reused_json_buffer_do_not_allocate()
-> Result<(), Box<dyn std::error::Error>> {
    let renderer = Renderer::default();
    let symbol = renderer
        .symbol("10031000001211000000")
        .text(field::UNIQUE_DESIGNATION, "A\u{1}\n\"\\😀")
        .render()?;
    let expected = compat::canonical_json_string(&symbol);
    let mut buffer = String::with_capacity(expected.len());
    let colors =
        milsymbol::color::ColorMode::new("pink", "blue", "red", "green", "yellow", "orange");
    #[cfg(feature = "std")]
    let cached = milsymbol::cache::CachedRenderer::new(renderer, 4).with_prepared_paths();
    #[cfg(feature = "std")]
    let options = SymbolOptions::default();
    #[cfg(feature = "std")]
    cached.render(symbol.sidc(), &options)?;

    let profiler = dhat::Profiler::builder().testing().build();
    for _ in 0..4 {
        buffer.clear();
        compat::write_canonical_json(black_box(&symbol), &mut buffer);
        black_box((symbol.metadata(), symbol.js_metadata()));
        black_box(colors.for_affiliation(Affiliation::Friend));
        #[cfg(feature = "std")]
        black_box(cached.render(symbol.sidc(), &options)?);
    }
    let stats = dhat::HeapStats::get();
    drop(profiler);
    assert_eq!(stats.total_blocks, 0);
    assert_eq!(buffer, expected);
    Ok(())
}
