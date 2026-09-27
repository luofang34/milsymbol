//! Differential corpus: replays checked-in oracle fixtures
//! (`tests/corpus/*.jsonl.gz`, produced by `tools/oracle/fixtures.mjs` from
//! milsymbol.js 3.0.4) and requires byte-identical SVG and an identical
//! canonical semantic record for every case. No JavaScript is involved.
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    clippy::indexing_slicing
)]

mod support;

use flate2::read::GzDecoder;
use serde_json::Value;
use std::io::{BufRead, BufReader};

fn replay(suite: &str) {
    let path = format!(
        "{}/tests/corpus/{suite}.jsonl.gz",
        env!("CARGO_MANIFEST_DIR")
    );
    let file = std::fs::File::open(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let reader = BufReader::new(GzDecoder::new(file));
    let (mut total, mut failures) = (0usize, Vec::new());
    for line in reader.lines() {
        let line = line.expect("read fixture line");
        if line.trim().is_empty() {
            continue;
        }
        total += 1;
        let case: Value = serde_json::from_str(&line).expect("fixture JSON");
        let problem = match (support::render(&case), case.get("error")) {
            (Err(_), Some(_)) => None,
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
                if hs != want("svg") {
                    Some(format!("svg hash {hs} != {}", want("svg")))
                } else if hm != want("sem") {
                    Some(format!("semantic hash {hm} != {}", want("sem")))
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
        "{suite}: {} of {total} cases differ from milsymbol.js; first:\n{}",
        failures.len(),
        failures
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn corpus_invalid() {
    replay("invalid");
}

#[test]
fn corpus_config() {
    replay("config");
}

#[test]
fn corpus_options() {
    replay("options");
}

#[test]
fn corpus_fuzz() {
    replay("fuzz");
}

#[test]
fn corpus_modifiers() {
    replay("modifiers");
}

#[test]
fn corpus_base() {
    replay("base");
}
