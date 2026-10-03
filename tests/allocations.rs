//! Allocation guards for rendering, observers and buffer reuse.
//!
//! The target sets `harness = false`: the allocator counts every thread, and
//! libtest's main thread allocates bookkeeping while the test thread runs, so
//! under the harness a measured window can pick up blocks that are not ours.
//! Without it this process has one thread and the counts are exact. The
//! budgets are the exact counts of the default build.

use milsymbol::compat;
use milsymbol::domain::Affiliation;
use milsymbol::labels::{Label, LabelField};
use milsymbol::options::SymbolOptions;
use milsymbol::options::TextField;
use milsymbol::{IconExtension, Renderer, Symbol};
use std::collections::BTreeMap;
use std::hint::black_box;

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    prepared_observers_and_reused_json_buffer_do_not_allocate()?;
    unused_label_definitions_do_not_add_allocations()?;
    direction_rendering_stays_within_allocation_budgets()?;
    cloning_does_not_allocate_per_path()?;
    drawing_allocations_stay_within_budget()?;
    Ok(())
}

fn prepared_observers_and_reused_json_buffer_do_not_allocate()
-> Result<(), Box<dyn std::error::Error>> {
    let renderer = Renderer::default();
    let symbol = renderer
        .symbol("10031000001211000000")
        .text(TextField::UniqueDesignation, "A\u{1}\n\"\\😀")
        .render()?;
    let expected = compat::canonical_json_string(&symbol);
    let mut buffer = String::with_capacity(expected.len());
    let colors =
        milsymbol::color::ColorMode::new("pink", "blue", "red", "green", "yellow", "orange");
    #[cfg(feature = "std")]
    let cached = milsymbol::cache::CachedRenderer::new(renderer, 4);
    #[cfg(feature = "std")]
    let options = SymbolOptions::default();
    #[cfg(feature = "std")]
    cached.render(symbol.sidc(), &options)?;
    #[cfg(feature = "std")]
    cached.symbol(symbol.sidc()).strict().render()?;

    // Reserved up front so recording inside the measured window allocates nothing.
    let mut marks: Vec<(&str, u64)> = Vec::with_capacity(32);
    let mut mark = |what| marks.push((what, dhat::HeapStats::get().total_blocks));
    let profiler = dhat::Profiler::builder().testing().build();
    for _ in 0..4 {
        buffer.clear();
        compat::write_canonical_json(black_box(&symbol), &mut buffer);
        mark("canonical json");
        black_box(symbol.metadata());
        mark("metadata");
        black_box(milsymbol::compat::js_metadata(&symbol));
        mark("js metadata");
        black_box(colors.for_affiliation(Affiliation::Friend));
        mark("colours");
        #[cfg(feature = "std")]
        {
            black_box(cached.render(symbol.sidc(), &options)?);
            mark("cache hit");
            black_box(cached.symbol(symbol.sidc()).strict().render()?);
            mark("strict cache hit");
        }
    }
    let stats = dhat::HeapStats::get();
    drop(profiler);
    assert_eq!(
        stats.total_blocks, 0,
        "cumulative allocations after each operation: {marks:?}"
    );
    assert_eq!(buffer, expected);
    Ok(())
}

struct OwnedLabels(usize);

impl IconExtension for OwnedLabels {
    fn letter_labels(&self, out: &mut BTreeMap<String, Vec<LabelField>>) {
        let fields = (0..self.0)
            .map(|i| {
                LabelField::new(
                    format!("custom{i}"),
                    vec![Label::at(300.0, 100.0), Label::at(300.0, 120.0)],
                )
            })
            .collect();
        out.insert("S-G-UCI---".into(), fields);
    }
}

fn measured_render(
    renderer: &Renderer,
    sidc: &str,
    options: SymbolOptions,
) -> Result<(Symbol, dhat::HeapStats), milsymbol::RenderError> {
    renderer.render(sidc, options.clone())?;
    let profiler = dhat::Profiler::builder().testing().build();
    let symbol = renderer.render(sidc, options)?;
    let stats = dhat::HeapStats::get();
    drop(profiler);
    Ok((symbol, stats))
}

fn unused_label_definitions_do_not_add_allocations() -> Result<(), Box<dyn std::error::Error>> {
    let one = Renderer::builder().icons(OwnedLabels(1)).build();
    let ten = Renderer::builder().icons(OwnedLabels(10)).build();
    for text in ["", "VISIBLE"] {
        let mut options = SymbolOptions::default();
        options.set_text("custom0", text);
        let (a, a_stats) = measured_render(&one, "SFGPUCI-----", options.clone())?;
        let (b, b_stats) = measured_render(&ten, "SFGPUCI-----", options)?;
        assert_eq!(a_stats.total_blocks, b_stats.total_blocks, "{text:?}");
        assert_eq!(a_stats.total_bytes, b_stats.total_bytes, "{text:?}");
        assert_eq!(a.to_svg(), b.to_svg());
        assert_eq!(
            compat::canonical_json_string(&a),
            compat::canonical_json_string(&b)
        );
    }
    Ok(())
}

fn direction_rendering_stays_within_allocation_budgets() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = Renderer::default();
    for (sidc, budgets) in [
        ("10031000001211000000", [26, 38, 21, 28]),
        ("10031002161211000000", [38, 54, 38, 50]),
        ("10030100001100000000", [20, 30, 21, 28]),
        ("SFGPUCI----D", [28, 43, 22, 32]),
    ] {
        for ((speed, outline), budget) in [(0.0, 0.0), (0.0, 3.0), (60.0, 0.0), (60.0, 3.0)]
            .into_iter()
            .zip(budgets)
        {
            let mut options = SymbolOptions::default();
            options.direction = Some(45.0);
            options.speed_leader = Some(speed);
            options.style.outline_width = outline;
            let (symbol, stats) = measured_render(&renderer, sidc, options)?;
            assert!(symbol.sidc_validity().is_valid());
            assert!(
                stats.total_blocks <= budget,
                "{sidc} speed={speed} outline={outline}: {} allocations exceeds {budget}",
                stats.total_blocks
            );
        }
    }
    Ok(())
}

/// A packed path is a borrowed slice, so a clone costs what the same tree
/// with plain path text costs: the blocks are the tree's own vectors. The
/// bound is an upper limit so allocator differences between platforms do
/// not fail it.
fn cloning_does_not_allocate_per_path() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = Renderer::default();
    let symbol = renderer.render("10031000001211000000", SymbolOptions::default())?;
    let profiler = dhat::Profiler::builder().testing().build();
    let copy = black_box(&symbol).clone();
    let stats = dhat::HeapStats::get();
    drop(profiler);
    drop(copy);
    assert!(
        stats.total_blocks <= 5,
        "blocks of a clone: {}",
        stats.total_blocks
    );
    Ok(())
}

/// The drawing view borrows colours from the built-in tables and checks
/// them without allocating, so its blocks are its own vectors and the
/// colours of options.
fn drawing_allocations_stay_within_budget() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = Renderer::default();
    for (sidc, budget) in [
        ("10031000001211000000", BUDGET_INFANTRY),
        ("SFGPUCI----D", BUDGET_LETTER),
    ] {
        let symbol = renderer.render(sidc, SymbolOptions::default())?;
        black_box(symbol.drawing());
        let profiler = dhat::Profiler::builder().testing().build();
        let drawing = black_box(&symbol).drawing();
        let stats = dhat::HeapStats::get();
        drop(profiler);
        drop(drawing);
        assert!(
            stats.total_blocks <= budget,
            "drawing {sidc}: {} blocks exceeds {budget}",
            stats.total_blocks
        );
    }
    Ok(())
}

/// Upper bounds on the blocks one `drawing()` call allocates.
const BUDGET_INFANTRY: u64 = 4;
const BUDGET_LETTER: u64 = 5;
