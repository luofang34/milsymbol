//! Standards-mode differences must have a source rule; compatibility remains exact.

mod support;

use flate2::read::GzDecoder;
use milsymbol::{ReferencePlatform, RenderMode};
use serde_json::Value;
use std::io::{BufRead, BufReader};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn changed_paths(a: &Value, b: &Value, path: &str, out: &mut Vec<String>) {
    if a == b {
        return;
    }
    match (a, b) {
        (Value::Object(a), Value::Object(b)) => {
            let keys: std::collections::BTreeSet<_> = a.keys().chain(b.keys()).collect();
            for key in keys {
                changed_paths(
                    a.get(key).unwrap_or(&Value::Null),
                    b.get(key).unwrap_or(&Value::Null),
                    &if path.is_empty() {
                        key.to_owned()
                    } else {
                        format!("{path}.{key}")
                    },
                    out,
                );
            }
        }
        (Value::Array(a), Value::Array(b)) if a.len() == b.len() => {
            for (a, b) in a.iter().zip(b) {
                changed_paths(a, b, &format!("{path}.*"), out);
            }
        }
        _ => out.push(path.to_owned()),
    }
}

fn applies(rule: &str, sidc: &str, before: &Value, after: &Value) -> bool {
    let md = after.get("metadata").unwrap_or(&Value::Null);
    if md.get("numberSIDC") != Some(&Value::Bool(true)) {
        return false;
    }
    let mil = md.get("STD2525") == Some(&Value::Bool(true));
    let version = sidc.get(..2).unwrap_or("");
    let set = sidc.get(4..6).unwrap_or("");
    let edition_e = matches!(version, "13" | "14" | "15" | "16");
    match rule {
        "identity_colors" => true,
        "edition_tables" => matches!(version, "15" | "16") || (edition_e && set == "64"),
        "restricted_contexts" => {
            edition_e && matches!(sidc.get(2..3), Some("3" | "4" | "5" | "6" | "7" | "8"))
        }
        "metoc_unframed" => matches!(set, "45" | "46" | "47"),
        "movement_amplifiers" => {
            matches!(set, "45" | "46" | "47")
                && before
                    .pointer("/options/direction")
                    .is_some_and(|d| !d.is_null())
        }
        "frame_selection" => sidc.get(22..23).is_some_and(|v| v != "0"),
        "leadership" => {
            md.get("leadership").is_some() || before.pointer("/metadata/leadership").is_some()
        }
        "frame_proportions" => {
            let dimension = md.get("dimension").and_then(Value::as_str).unwrap_or("");
            let affiliation = md.get("affiliation").and_then(Value::as_str).unwrap_or("");
            (affiliation == "Unknown"
                && matches!(dimension, "Ground" | "Sea" | "LandDismountedIndividual"))
                || (affiliation == "Friend"
                    && ((mil && dimension == "LandDismountedIndividual")
                        || (!mil && dimension == "Sea")))
        }
        _ => false,
    }
}

fn check_case(case: &Value, rules: &[Vec<&str>]) -> TestResult {
    let sidc = case.get("sidc").and_then(Value::as_str).unwrap_or_default();
    let options = support::options(case)?;
    let render = |mode| {
        support::renderer_mode(case, ReferencePlatform::X64, mode).render(sidc, options.clone())
    };
    let (before, after) = match (
        render(RenderMode::Milsymbol304),
        render(RenderMode::Standards),
    ) {
        (Ok(before), Ok(after)) => (before, after),
        (Err(a), Err(b)) => {
            assert_eq!(a.to_string(), b.to_string(), "{sidc}");
            return Ok(());
        }
        (a, b) => return Err(format!("unexplained result change {sidc}: {a:?} -> {b:?}").into()),
    };
    let a: Value = serde_json::from_str(&milsymbol::compat::canonical_json_string(&before))?;
    let b: Value = serde_json::from_str(&milsymbol::compat::canonical_json_string(&after))?;
    let mut paths = Vec::new();
    changed_paths(&a, &b, "", &mut paths);
    let allowed: Vec<&str> = rules
        .iter()
        .filter(|r| applies(r.first().copied().unwrap_or(""), sidc, &a, &b))
        .flat_map(|r| r.last().copied().unwrap_or("").split(';'))
        .collect();
    for path in &paths {
        assert!(
            allowed.iter().any(|root| path == root
                || path.strip_prefix(root).is_some_and(|s| s.starts_with('.'))
                || root.split_once("**").is_some_and(
                    |(prefix, suffix)| path.starts_with(prefix) && path.ends_with(suffix)
                )),
            "unexplained standards difference: {sidc} {path}; rules {allowed:?}"
        );
    }
    if paths.is_empty() {
        assert_eq!(before.to_svg(), after.to_svg(), "{sidc}");
    }
    Ok(())
}

#[test]
fn every_corpus_difference_is_traced_to_a_pinned_rule() -> TestResult {
    let table = include_str!("data/standards/deviations.tsv");
    let rules: Vec<Vec<&str>> = table
        .lines()
        .skip(1)
        .map(|l| l.split('\t').collect())
        .collect();
    assert!(
        rules
            .iter()
            .all(|r| r.len() == 5 && r.iter().all(|s| !s.is_empty()))
    );
    for suite in [
        "base",
        "modifiers",
        "options",
        "config",
        "invalid",
        "fuzz",
        "direction",
        "layout",
    ] {
        let file = std::fs::File::open(format!(
            "{}/tests/corpus/{suite}.jsonl.gz",
            env!("CARGO_MANIFEST_DIR")
        ))?;
        for (i, line) in BufReader::new(GzDecoder::new(file)).lines().enumerate() {
            if cfg!(debug_assertions) && i % 13 != 0 {
                continue;
            }
            check_case(&serde_json::from_str::<Value>(&line?)?, &rules)?;
        }
    }
    Ok(())
}
