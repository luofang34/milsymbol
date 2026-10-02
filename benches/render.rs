//! Rendering benchmarks: single symbols, repeated (cached) rendering and
//! bulk rendering of every built-in numeric icon.

use criterion::{Criterion, criterion_group, criterion_main};
use milsymbol::cache::CachedRenderer;
use milsymbol::options::{SymbolOptions, TextField};
use milsymbol::{Renderer, catalog};
use std::hint::black_box;

fn all_number_sidcs() -> Vec<String> {
    let mut out = Vec::new();
    for ss in catalog::number_symbol_sets() {
        for e in catalog::number_entities(ss) {
            out.push(format!("1003{ss}0000{e}0000"));
        }
    }
    out
}

fn single(c: &mut Criterion) {
    let r = Renderer::default();
    let mut g = c.benchmark_group("single");
    g.bench_function("compose_infantry", |b| {
        b.iter(|| r.render(black_box("10031000001211000000"), SymbolOptions::default()))
    });
    g.bench_function("compose_and_svg_infantry", |b| {
        b.iter(|| {
            r.render(black_box("10031000001211000000"), SymbolOptions::default())
                .map(|s| s.to_svg())
        })
    });
    let mut text = SymbolOptions::default();
    text.set_text(TextField::UniqueDesignation, "1-66")
        .set_text(TextField::HigherFormation, "2 BDE");
    text.direction = Some(45.0);
    let mut buffer = String::with_capacity(2048);
    g.bench_function("compose_and_write_svg_reused_buffer_infantry", |b| {
        b.iter(|| {
            buffer.clear();
            if let Ok(s) = r.render(black_box("10031000001211000000"), SymbolOptions::default()) {
                s.write_svg(&mut buffer);
            }
            buffer.len()
        })
    });
    g.bench_function("compose_and_drawing_infantry", |b| {
        b.iter(|| {
            r.render(black_box("10031000001211000000"), SymbolOptions::default())
                .map(|s| s.drawing().items.len())
        })
    });
    g.bench_function("compose_and_svg_with_fields", |b| {
        b.iter(|| {
            r.render(black_box("10031002151211000000"), text.clone())
                .map(|s| s.to_svg())
        })
    });
    g.bench_function("compose_and_svg_letter", |b| {
        b.iter(|| {
            r.render(black_box("SFGPUCI----D"), SymbolOptions::default())
                .map(|s| s.to_svg())
        })
    });
    g.finish();
}

fn repeated(c: &mut Criterion) {
    let cached = CachedRenderer::new(Renderer::default(), 4096);
    let opts = SymbolOptions::default();
    c.bench_function("repeated_cached_render", |b| {
        b.iter(|| {
            cached
                .render(black_box("10031000001211000000"), &opts)
                .map(|s| s.size())
        })
    });
}

fn bulk(c: &mut Criterion) {
    let r = Renderer::default();
    let sidcs = all_number_sidcs();
    let mut g = c.benchmark_group("bulk");
    g.sample_size(10);
    g.bench_function(format!("all_number_icons_{}", sidcs.len()), |b| {
        b.iter(|| {
            let mut bytes = 0usize;
            for s in &sidcs {
                if let Ok(sym) = r.render(s, SymbolOptions::default()) {
                    bytes += sym.to_svg().len();
                }
            }
            bytes
        })
    });
    g.bench_function(format!("drawing_all_number_icons_{}", sidcs.len()), |b| {
        let symbols: Vec<_> = sidcs
            .iter()
            .filter_map(|s| r.render(s, SymbolOptions::default()).ok())
            .collect();
        b.iter(|| {
            let mut items = 0usize;
            for sym in &symbols {
                items += sym.drawing().items.len();
            }
            items
        })
    });
    g.bench_function(
        format!("all_number_icons_{}_reused_buffer", sidcs.len()),
        |b| {
            let mut buffer = String::with_capacity(4096);
            b.iter(|| {
                let mut bytes = 0usize;
                for s in &sidcs {
                    if let Ok(sym) = r.render(s, SymbolOptions::default()) {
                        buffer.clear();
                        sym.write_svg(&mut buffer);
                        bytes += buffer.len();
                    }
                }
                bytes
            })
        },
    );
    g.finish();
}

criterion_group!(benches, single, repeated, bulk);
criterion_main!(benches);
