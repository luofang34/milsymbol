//! `Symbol::drawing` is the input for renderers other than SVG, so it must
//! describe the same picture as `Symbol::to_svg`, which the oracle corpus
//! proves byte for byte. This test reads every corpus symbol's SVG with an
//! independent SVG implementation (usvg) and compares it, item by item, with
//! the drawing view: geometry, transforms, fill, stroke, dashes, caps,
//! joins, clips and text attributes. A sample is also painted twice, once
//! from the drawing view by a small tiny-skia consumer and once from the
//! SVG by resvg, and the pixels compared.
//!
//! Debug builds check a stride of the corpus; `cargo test --release --test
//! drawing_svg` checks all of it.

mod support;

#[path = "drawing_svg/compare.rs"]
mod compare;
#[path = "drawing_svg/css_view.rs"]
mod css_view;
#[path = "drawing_svg/model.rs"]
mod model;
#[path = "drawing_svg/raster.rs"]
mod raster;
#[path = "drawing_svg/skia.rs"]
mod skia;
#[path = "drawing_svg/text.rs"]
mod text;

use flate2::read::GzDecoder;
use milsymbol::ir::{Node, Paint, PathData};
use milsymbol::{PartOutput, PartialBBox, Renderer, Symbol, SymbolPart, SymbolState};
use serde_json::Value;
use std::io::{BufRead, BufReader};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const SUITES: [&str; 8] = [
    "invalid",
    "config",
    "options",
    "layout",
    "fuzz",
    "modifiers",
    "direction",
    "base",
];

/// Every n-th case is compared; every m-th compared case is also painted.
const STRIDE: usize = if cfg!(debug_assertions) { 37 } else { 1 };
const RASTER_STRIDE: usize = if cfg!(debug_assertions) { 20 } else { 25 };

/// Renderings larger than this are not painted (`size: 250` with long text).
const MAX_PIXELS: f64 = 4.0e6;

/// The raster difference accepted. tiny-skia antialiases with 4×4
/// supersampling, so one sample is worth 255/16 of a channel. The drawing
/// view composes each transform in `f64` and rounds it once; usvg composes
/// `f32` matrices group by group. The results differ in the last bits,
/// which flips single samples on a few edge pixels: for rotated direction
/// arrows, painting the drawing with usvg's transforms reproduces resvg
/// exactly. A clip under a scale also leaves a few pixels within this bound
/// (scale alone and clip alone paint identically); that case was narrowed
/// down, not isolated. Beyond that, 8-bit blending rounds by up to 2.
const MAX_CHANNEL: u8 = 2 * 16 + 2;

/// How many pixels may differ by more than rounding: a few, or a small
/// share of a large symbol.
fn max_differing(pixels: usize) -> usize {
    (pixels / 500).max(8)
}

#[derive(Debug)]
struct Totals {
    symbols: usize,
    texts: usize,
    /// `fill` and `stroke` declarations CSS ignores (see `css_view`).
    ignored_paints: usize,
    painted: usize,
    pixels: usize,
    coverage: compare::Coverage,
    raster: raster::Difference,
    failures: Vec<String>,
}

impl Totals {
    fn new() -> Self {
        Totals {
            symbols: 0,
            texts: 0,
            ignored_paints: 0,
            painted: 0,
            pixels: 0,
            coverage: compare::Coverage::default(),
            raster: raster::Difference::default(),
            failures: Vec::new(),
        }
    }

    fn check(&mut self, label: &str, symbol: &Symbol, paint: bool) {
        if let Err(e) = self.try_check(symbol, paint) {
            self.failures
                .push(format!("{label}\n    {e}\n    svg: {}", symbol.to_svg()));
        }
    }

    fn try_check(&mut self, symbol: &Symbol, paint: bool) -> Result<(), String> {
        let (svg, ignored) = css_view::css_view(&symbol.to_svg());
        self.ignored_paints += ignored;
        let (coverage, model) = compare::symbol(symbol, &svg)?;
        let drawing = symbol.drawing();
        self.texts += text::compare(&drawing.items, &svg)?;
        self.coverage.add(coverage);
        self.symbols += 1;
        if paint && drawing.size.width * drawing.size.height <= MAX_PIXELS {
            let d = raster::difference(
                &raster::drawing(&drawing)?,
                &raster::svg(&model.tree, &drawing)?,
            )?;
            if d.differing > max_differing(d.pixels) || d.max_channel > MAX_CHANNEL {
                return Err(format!(
                    "{} of {} pixels differ (max channel {})",
                    d.differing, d.pixels, d.max_channel
                ));
            }
            self.painted += 1;
            self.pixels += d.pixels;
            self.raster.max_channel = self.raster.max_channel.max(d.max_channel);
            self.raster.differing += d.differing;
        }
        Ok(())
    }

    fn finish(&self, what: &str) {
        let mut causes = std::collections::BTreeMap::<String, usize>::new();
        for f in &self.failures {
            let cause = f.lines().nth(1).unwrap_or_default().trim().to_string();
            *causes.entry(cause).or_default() += 1;
        }
        for (cause, n) in &causes {
            eprintln!("{what}: {n} x {cause}");
        }
        eprintln!(
            "{what}: {} symbols, {} paths ({} points, {} dashed, {} clipped), {} texts, \
             max relative deviation {:e}; {} painted ({} pixels, max channel difference {}); \
             {} ignored paint declarations",
            self.symbols,
            self.coverage.paths,
            self.coverage.points,
            self.coverage.dashed,
            self.coverage.clipped,
            self.texts,
            self.coverage.max_deviation,
            self.painted,
            self.pixels,
            self.raster.max_channel,
            self.ignored_paints,
        );
        // `DRAWING_SVG_FAILURES=<file>` keeps every failure with its SVG.
        if let Ok(path) = std::env::var("DRAWING_SVG_FAILURES") {
            std::fs::write(path, self.failures.join("\n")).ok();
        }
        assert!(
            self.failures.is_empty(),
            "{what}: {} symbols whose drawing view differs from their SVG; first:\n{}",
            self.failures.len(),
            self.failures
                .iter()
                .take(5)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

fn corpus(suite: &str, totals: &mut Totals) -> TestResult {
    let path = format!(
        "{}/tests/corpus/{suite}.jsonl.gz",
        env!("CARGO_MANIFEST_DIR")
    );
    let reader = BufReader::new(GzDecoder::new(std::fs::File::open(&path)?));
    for (index, line) in reader.lines().enumerate().filter(|(i, _)| i % STRIDE == 0) {
        let line = line?;
        let case: Value = serde_json::from_str(&line)?;
        let Ok(options) = support::options(&case) else {
            continue;
        };
        let sidc = case.get("sidc").and_then(Value::as_str).unwrap_or_default();
        let renderer = support::renderer(&case, milsymbol::ReferencePlatform::X64);
        if let Ok(symbol) = renderer.render(sidc, options) {
            let paint = (index / STRIDE) % RASTER_STRIDE == 0;
            totals.check(&format!("{suite}: {line}"), &symbol, paint);
        }
    }
    Ok(())
}

#[test]
fn corpus_drawings_match_their_svg() -> TestResult {
    let mut totals = Totals::new();
    for suite in SUITES {
        corpus(suite, &mut totals)?;
    }
    totals.finish("corpus");
    assert!(totals.symbols > 295_000 / STRIDE, "{}", totals.symbols);
    Ok(())
}

/// Adds fixed instructions to every symbol.
struct Extra(Vec<Node>);

impl SymbolPart for Extra {
    fn draw(&self, _: &SymbolState<'_>) -> Result<PartOutput, milsymbol::PartError> {
        Ok(PartOutput::new(
            vec![],
            self.0.clone(),
            PartialBBox::default(),
        ))
    }
}

fn stroked(mut node: Node, color: &'static str) -> Node {
    if let Some(style) = node.style_mut() {
        style.fill = Some(Paint::color("rgba(0,128,0,0.5)"));
        style.stroke = Some(Paint::color(color));
        style.stroke_dasharray = Some("6,3,2".into());
    }
    node
}

/// An extension clip group nesting a rotated group, one of whose paths has
/// an inline `clip_path` style.
fn nested_clips() -> Vec<Node> {
    let mut inline = stroked(Node::path("M 60,60 L 140,60 L 140,140 Z"), "red");
    if let Some(style) = inline.style_mut() {
        style.clip_path = Some("M 50,50 h 100 v 60 h -100 z".into());
    }
    let circle = stroked(Node::circle(100.0, 100.0, 30.0), "blue");
    let rotated = Node::rotate(30.0, 100.0, 100.0, vec![inline, circle]);
    let outer = PathData::new("M 40,40 L 160,40 L 160,130 L 40,130 Z");
    vec![Node::clip(outer, Some("c".into()), vec![rotated])]
}

/// A clip group under a scale, around an elliptical arc.
fn scaled_clip() -> Vec<Node> {
    let arc = stroked(Node::path("M 70,110 a 60,30 20 1 1 120,0"), "purple");
    let clip = PathData::new("M 0,0 h 200 v 150 h -200 z");
    vec![Node::scale(0.5, vec![Node::clip(clip, None, vec![arc])])]
}

fn clip_case(nodes: Vec<Node>, what: &str) -> TestResult {
    let renderer = Renderer::builder().symbol_part(Extra(nodes)).build();
    let mut totals = Totals::new();
    for sidc in [
        "10031000161211000000",
        "SHGPUCI-----",
        "10061500331101000000",
    ] {
        for size in [30.0, 100.0] {
            let symbol = renderer.symbol(sidc).size(size).direction(45.0).render()?;
            totals.check(sidc, &symbol, true);
        }
    }
    totals.finish(what);
    assert_eq!((totals.symbols, totals.painted), (6, 6), "{what}");
    assert!(
        totals.coverage.clipped >= 6,
        "{what}: {:?}",
        totals.coverage
    );
    Ok(())
}

/// The corpus has no clips (only extensions create them), so these cases
/// exercise the clip paths of both readers.
#[test]
fn clipped_extension_drawings_match_their_svg() -> TestResult {
    clip_case(nested_clips(), "nested clips")?;
    clip_case(scaled_clip(), "scaled clip")
}
