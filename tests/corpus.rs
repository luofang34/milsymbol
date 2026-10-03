//! Differential corpus: replays checked-in oracle fixtures
//! (`tests/corpus/*.jsonl.gz`, produced by `tools/oracle/fixtures.mjs` from
//! milsymbol.js 3.0.4) and requires byte-identical SVG and an identical
//! canonical semantic record for every case. No JavaScript is involved.
//!
//! Debug builds replay every 13th case; `cargo test --release --test
//! corpus` replays all of them.

mod support;

use flate2::read::GzDecoder;
use serde_json::Value;
use std::io::{BufRead, BufReader};

/// Every n-th case is replayed.
const STRIDE: usize = if cfg!(debug_assertions) { 13 } else { 1 };

fn replay(suite: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = format!(
        "{}/tests/corpus/{suite}.jsonl.gz",
        env!("CARGO_MANIFEST_DIR")
    );
    let file = std::fs::File::open(&path).map_err(|e| format!("{path}: {e}"))?;
    let reader = BufReader::new(GzDecoder::new(file));
    let (mut total, mut failures) = (0usize, Vec::new());
    for (index, line) in reader.lines().enumerate() {
        let line = line?;
        if index % STRIDE != 0 {
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }
        total += 1;
        let case: Value = serde_json::from_str(&line)?;
        let problem = match (support::render(&case), case.get("error")) {
            (Err(e), Some(u)) => {
                let upstream = u.as_str().unwrap_or_default();
                (!support::same_failure(upstream, &e))
                    .then(|| format!("rust error {e} is not upstream's {upstream}"))
            }
            (Err(e), None) => Some(format!("rust error {e}")),
            (Ok(_), Some(e)) => Some(format!("oracle throws {e}, rust renders")),
            (Ok((svg, sem)), None) => {
                let (hs, hm) = (support::fnv64(&svg), support::fnv64(&sem));
                let want = |k: &str| {
                    case.get(k)
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string()
                };
                // The fixture holds hashes only, so show what Rust produced.
                if hs != want("svg") {
                    Some(format!(
                        "svg hash {hs} != {}\n    rust svg: {svg}",
                        want("svg")
                    ))
                } else if hm != want("sem") {
                    Some(format!(
                        "semantic hash {hm} != {}\n    rust record: {sem}",
                        want("sem")
                    ))
                } else {
                    None
                }
            }
        };
        if let Some(p) = problem {
            failures.push(format!("{line}\n    {p}"));
        }
    }
    assert!(total > 0, "{suite}: empty fixture");
    assert!(
        failures.is_empty(),
        "{suite}: {} of {total} cases differ from milsymbol.js; first:\n{}\n\
         `tools/oracle/diff.sh {suite}` (needs Node) shows each difference field by field",
        failures.len(),
        failures
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    Ok(())
}

#[test]
fn corpus_invalid() -> Result<(), Box<dyn std::error::Error>> {
    replay("invalid")
}

#[test]
fn corpus_config() -> Result<(), Box<dyn std::error::Error>> {
    replay("config")
}

#[test]
fn corpus_options() -> Result<(), Box<dyn std::error::Error>> {
    replay("options")
}

#[test]
fn corpus_fuzz() -> Result<(), Box<dyn std::error::Error>> {
    replay("fuzz")
}

#[test]
fn corpus_modifiers() -> Result<(), Box<dyn std::error::Error>> {
    replay("modifiers")
}

#[test]
fn corpus_base() -> Result<(), Box<dyn std::error::Error>> {
    replay("base")
}

#[test]
fn corpus_direction() -> Result<(), Box<dyn std::error::Error>> {
    replay("direction")
}

#[test]
fn corpus_layout() -> Result<(), Box<dyn std::error::Error>> {
    replay("layout")
}

/// Every symbol of the oracle corpus draws as many items as its SVG has
/// elements, and no path is malformed.
#[test]
fn corpus_drawings_match_svg_element_counts() -> Result<(), Box<dyn std::error::Error>> {
    let mut checked = 0usize;
    for suite in ["modifiers", "options", "direction", "config", "layout"] {
        let path = format!(
            "{}/tests/corpus/{suite}.jsonl.gz",
            env!("CARGO_MANIFEST_DIR")
        );
        let reader = BufReader::new(GzDecoder::new(std::fs::File::open(path)?));
        for line in reader.lines().step_by(STRIDE) {
            let case: serde_json::Value = serde_json::from_str(&line?)?;
            if case.get("error").is_some() {
                continue;
            }
            let sidc = case
                .get("sidc")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let Ok(options) = support::options(&case) else {
                continue;
            };
            let renderer = support::renderer(&case, milsymbol::ReferencePlatform::X64);
            let Ok(symbol) = renderer.render(sidc, options) else {
                continue;
            };
            let svg = symbol.to_svg();
            let drawing = symbol.drawing();
            let paths = drawing
                .items
                .iter()
                .filter(|i| matches!(i.shape, milsymbol::drawing::Shape::Path(_)))
                .count();
            let want = svg.matches("<path").count() - svg.matches("clip-rule").count();
            assert_eq!(paths, want, "{sidc} {suite}");
            let texts = drawing
                .items
                .iter()
                .filter(|i| matches!(i.shape, milsymbol::drawing::Shape::Text(_)))
                .count();
            assert_eq!(texts, svg.matches("<text").count(), "{sidc} {suite}");
            assert_eq!(drawing.invalid_paths, 0, "{sidc} {suite}");
            checked += 1;
        }
    }
    assert!(checked > 10_000 / STRIDE, "{checked}");
    Ok(())
}
