//! Measures heap use of rendering with the `dhat` heap profiler: peak heap
//! and total allocated bytes per symbol, for composing and SVG serialization.
//!
//! `cargo run --release --example footprint`

use milsymbol::options::{SymbolOptions, field};
use milsymbol::{Renderer, Symbol};

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

/// Runs `f` in a fresh profiling session and reports its heap use.
fn measure<T>(label: &str, f: impl FnOnce() -> T) -> T {
    let profiler = dhat::Profiler::builder().testing().build();
    let out = f();
    let stats = dhat::HeapStats::get();
    println!(
        "{label:40} peak {:>7} B   allocated {:>7} B in {:>4} blocks",
        stats.max_bytes, stats.total_bytes, stats.total_blocks
    );
    drop(profiler);
    out
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let r = Renderer::default();
    println!(
        "size_of::<Symbol>() = {} B, size_of::<Node>() = {} B",
        size_of::<Symbol>(),
        size_of::<milsymbol::ir::Node>()
    );
    let mut rich = SymbolOptions::default();
    rich.set_text(field::UNIQUE_DESIGNATION, "1-66")
        .set_text(field::HIGHER_FORMATION, "2 BDE");
    rich.direction = Some(45.0);
    let mut outlined = rich.clone();
    outlined.style.outline_width = 4.0;
    outlined.stack = Some(3.0);
    let cases: [(&str, &str, SymbolOptions); 4] = [
        ("infantry", "10031000001211000000", SymbolOptions::default()),
        (
            "HQ battalion + text + direction",
            "10031002161211000000",
            rich,
        ),
        ("letter SIDC", "SFGPUCI----D", SymbolOptions::default()),
        ("HQ + outline + stack 3", "10031002161211000000", outlined),
    ];
    for (name, sidc, options) in cases {
        let symbol = measure(&format!("compose: {name}"), || r.render(sidc, options))?;
        let svg = measure(&format!("to_svg:  {name}"), || symbol.to_svg());
        println!("{:40} {} B of SVG", "", svg.len());
    }
    let infantry = r.render("10031000001211000000", SymbolOptions::default())?;
    measure("is_valid: infantry", || infantry.is_valid());
    let mut buf = String::with_capacity(4096);
    measure("write_svg into a reused buffer", || {
        infantry.write_svg(&mut buf);
    });
    let cache = milsymbol::cache::CachedRenderer::new(r, 16);
    let options = SymbolOptions::default();
    cache.render("10031000001211000000", &options)?;
    measure("cache hit: infantry", || {
        cache.render("10031000001211000000", &options)
    })?;
    Ok(())
}
