//! Compares oracle and Rust record files produced from the same case list.
//!
//! Records are JSON lines `{"svg": …, "sem": …}` or `{"error": …}`; `sem` is
//! the canonical JSON string of the semantic record. Files are streamed:
//! record files for the base suite exceed a gigabyte.

use crate::Error;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Lines};
use std::path::Path;

fn open(path: &Path) -> Result<Lines<BufReader<File>>, Error> {
    let f = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(BufReader::new(f).lines())
}

/// Next non-empty line, parsed (`{}` when the file ends early).
fn next_record(lines: &mut Lines<BufReader<File>>) -> Result<Value, Error> {
    for line in lines.by_ref() {
        let line = line?;
        if !line.trim().is_empty() {
            return Ok(serde_json::from_str(&line)?);
        }
    }
    Ok(Value::Object(serde_json::Map::new()))
}

fn clip(v: &Value) -> String {
    v.to_string().chars().take(300).collect()
}

/// First differing path between two JSON values, with both values.
fn first_diff(x: &Value, y: &Value, path: &str) -> Option<String> {
    match (x, y) {
        (Value::Object(a), Value::Object(b)) => a
            .keys()
            .chain(b.keys().filter(|k| !a.contains_key(*k)))
            .find_map(|k| {
                let null = Value::Null;
                first_diff(
                    a.get(k).unwrap_or(&null),
                    b.get(k).unwrap_or(&null),
                    &format!("{path}.{k}"),
                )
            }),
        (Value::Array(a), Value::Array(b)) => {
            let null = Value::Null;
            (0..a.len().max(b.len())).find_map(|i| {
                first_diff(
                    a.get(i).unwrap_or(&null),
                    b.get(i).unwrap_or(&null),
                    &format!("{path}.{i}"),
                )
            })
        }
        _ if x == y => None,
        _ => Some(format!("{path}: {} != {}", clip(x), clip(y))),
    }
}

fn svg_diff(s: &str, t: &str) -> String {
    let at = s.chars().zip(t.chars()).take_while(|(a, b)| a == b).count();
    let window = |v: &str| {
        v.chars()
            .skip(at.saturating_sub(80))
            .take(200)
            .collect::<String>()
    };
    format!(
        "svg differs at {at}:\n     oracle …{}\n     rust   …{}",
        window(s),
        window(t)
    )
}

/// Groups a problem by its path with array indices removed.
fn bucket(problem: &str) -> String {
    let path = problem.split(':').next().unwrap_or(problem);
    path.split('.')
        .map(|seg| {
            if seg.chars().all(|c| c.is_ascii_digit()) {
                "[]"
            } else {
                seg
            }
        })
        .collect::<Vec<_>>()
        .join(".")
}

#[derive(Default)]
struct Tally {
    cases: usize,
    svg: usize,
    sem: usize,
    errors: usize,
    reported: usize,
    by_path: BTreeMap<String, usize>,
}

impl Tally {
    /// Records one case and returns the problem to report, if any.
    fn case(&mut self, x: &Value, y: &Value) -> Result<Option<String>, Error> {
        self.cases += 1;
        let (xe, ye) = (x.get("error"), y.get("error"));
        if xe.is_some() != ye.is_some() {
            self.errors += 1;
            return Ok(Some(format!(
                "error mismatch: oracle={} rust={}",
                clip(xe.unwrap_or(&Value::Null)),
                clip(ye.unwrap_or(&Value::Null))
            )));
        }
        if xe.is_some() {
            return Ok(None);
        }
        let text = |v: &Value, k: &str| {
            v.get(k)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string()
        };
        let (xs, ys) = (text(x, "sem"), text(y, "sem"));
        let sem = if xs == ys {
            None
        } else {
            first_diff(
                &serde_json::from_str(&xs)?,
                &serde_json::from_str(&ys)?,
                "sem",
            )
        };
        let (svg_x, svg_y) = (text(x, "svg"), text(y, "svg"));
        if sem.is_some() {
            self.sem += 1;
        }
        if svg_x != svg_y {
            self.svg += 1;
        }
        let problem = sem.or_else(|| (svg_x != svg_y).then(|| svg_diff(&svg_x, &svg_y)));
        if let Some(p) = &problem {
            let key = if p.starts_with("svg differs") {
                String::from("svg")
            } else {
                bucket(p)
            };
            *self.by_path.entry(key).or_default() += 1;
        }
        Ok(problem)
    }
}

/// Compares the record files; returns whether they agree.
pub fn run(cases: &Path, oracle: &Path, rust: &Path, max: usize) -> Result<bool, Error> {
    let (mut a, mut b) = (open(oracle)?, open(rust)?);
    let mut tally = Tally::default();
    for (i, line) in open(cases)?.enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let (x, y) = (next_record(&mut a)?, next_record(&mut b)?);
        if let Some(problem) = tally.case(&x, &y)? {
            if tally.reported < max {
                println!("#{i} {line}\n   {problem}");
            }
            tally.reported += 1;
        }
    }
    println!(
        "cases {}: svg mismatches {}, semantic mismatches {}, error mismatches {}",
        tally.cases, tally.svg, tally.sem, tally.errors
    );
    let mut paths: Vec<(&String, &usize)> = tally.by_path.iter().collect();
    paths.sort_by(|p, q| q.1.cmp(p.1));
    for (path, n) in paths.into_iter().take(15) {
        println!("  {n}\t{path}");
    }
    Ok(tally.svg + tally.sem + tally.errors == 0)
}
