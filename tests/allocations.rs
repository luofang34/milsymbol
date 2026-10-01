//! Allocation guards for rendering, observers and buffer reuse. This integration test
//! has its own process so other tests cannot contaminate allocator counts.

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

#[test]
fn allocation_budgets() -> Result<(), Box<dyn std::error::Error>> {
    prepared_observers_and_reused_json_buffer_do_not_allocate()?;
    unused_label_definitions_do_not_add_allocations()?;
    direction_rendering_stays_within_allocation_budgets()?;
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

    let cpu = std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|t| t.lines().find(|l| l.starts_with("model name")).map(String::from))
        .unwrap_or_default();
    let mut marks: Vec<u64> = Vec::with_capacity(64);
    let profiler = dhat::Profiler::builder().testing().build();
    for _ in 0..4 {
        buffer.clear();
        marks.push(dhat::HeapStats::get().total_blocks);
        compat::write_canonical_json(black_box(&symbol), &mut buffer);
        marks.push(dhat::HeapStats::get().total_blocks);
        black_box(symbol.metadata());
        marks.push(dhat::HeapStats::get().total_blocks);
        black_box(milsymbol::compat::js_metadata(&symbol));
        marks.push(dhat::HeapStats::get().total_blocks);
        black_box(colors.for_affiliation(Affiliation::Friend));
        marks.push(dhat::HeapStats::get().total_blocks);
        #[cfg(feature = "std")]
        black_box(cached.render(symbol.sidc(), &options)?);
        marks.push(dhat::HeapStats::get().total_blocks);
    }
    let stats = dhat::HeapStats::get();
    drop(profiler);
    assert_eq!(
        stats.total_blocks, 0,
        "DIAG {cpu} marks[start,json,metadata,js_metadata,colors,cache]x4={marks:?}"
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
            assert!(symbol.validity().is_valid());
            assert!(
                stats.total_blocks <= budget,
                "{sidc} speed={speed} outline={outline}: {} allocations exceeds {budget}",
                stats.total_blocks
            );
        }
    }
    Ok(())
}
