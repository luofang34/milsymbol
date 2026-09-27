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
    let cases: [(&str, &str, SymbolOptions); 3] = [
        ("infantry", "10031000001211000000", SymbolOptions::default()),
        (
            "HQ battalion + text + direction",
            "10031002161211000000",
            rich,
        ),
        ("letter SIDC", "SFGPUCI----D", SymbolOptions::default()),
    ];
    for (name, sidc, options) in cases {
        let symbol = measure(&format!("compose: {name}"), || r.render(sidc, options))?;
        let svg = measure(&format!("to_svg:  {name}"), || symbol.to_svg());
        println!("{:40} {} B of SVG", "", svg.len());
    }
    Ok(())
}
