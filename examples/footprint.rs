//! Measures heap use of rendering with the `dhat` heap profiler: peak heap
//! and total allocated bytes per symbol, for composing and SVG serialization.
//!
//! `cargo run --release --example footprint`

use milsymbol::compat;
use milsymbol::ir::Node;
use milsymbol::options::{SymbolOptions, TextField};
use milsymbol::{Renderer, Symbol};
use std::io::{self, Write};

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
    rich.set_text(TextField::UniqueDesignation, "1-66")
        .set_text(TextField::HigherFormation, "2 BDE");
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
        profile_symbol(&r, name, sidc, options)?;
    }
    let infantry = r.render("10031000001211000000", SymbolOptions::default())?;
    measure("is_valid: infantry", || infantry.validity().is_valid());
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

fn profile_symbol(
    r: &Renderer,
    name: &str,
    sidc: &str,
    options: SymbolOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let symbol = measure(&format!("compose: {name}"), || r.render(sidc, options))?;
    let svg = measure(&format!("to_svg:  {name}"), || symbol.to_svg());
    let (nodes, lists) = ir_storage(symbol.instructions());
    writeln!(
        io::stdout(),
        "{name}: {} SVG bytes, {nodes} IR nodes in {lists} nonempty lists",
        svg.len()
    )?;
    let owned = measure("canonical JSON: owned tree + string", || {
        compat::canonical_json(&symbol).to_canonical_string()
    });
    let streamed = measure("canonical JSON: stream to new string", || {
        compat::canonical_json_string(&symbol)
    });
    let mut buffer = String::with_capacity(owned.len());
    measure("canonical JSON: stream to reused buffer", || {
        compat::write_canonical_json(&symbol, &mut buffer)
    });
    if buffer != owned || streamed != owned {
        return Err("canonical JSON writers disagree".into());
    }
    measure("native + JS metadata views", || {
        (symbol.metadata(), milsymbol::compat::js_metadata(&symbol))
    });
    Ok(())
}

fn ir_storage(nodes: &[Node]) -> (usize, usize) {
    let mut count = nodes.len();
    let mut lists = usize::from(!nodes.is_empty());
    for node in nodes {
        if let Some(children) = node.children() {
            let (child_nodes, child_lists) = ir_storage(children);
            count += child_nodes;
            lists += child_lists;
        }
    }
    (count, lists)
}
