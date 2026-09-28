//! Compares oracle and Rust record files produced from the same case list.
//!
//! Records are JSON lines `{"svg": …, "sem": …}` or `{"error": …}`; `sem` is
//! the canonical JSON string of the semantic record. Files are streamed:
//! record files for the base suite exceed a gigabyte. An oracle record can
//! carry an `expected` control record for a declared option-key difference.

use crate::Error;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Lines, Write};
use std::path::Path;

fn open(path: &Path) -> Result<BufReader<File>, Error> {
    let f = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(BufReader::new(f))
}

/// Next non-empty line, parsed; `None` at the end of the file.
fn next_record<R: BufRead>(lines: &mut Lines<R>) -> Result<Option<Value>, Error> {
    for line in lines.by_ref() {
        let line = line?;
        if !line.trim().is_empty() {
            return Ok(Some(serde_json::from_str(&line)?));
        }
    }
    Ok(None)
}

fn clip(v: &Value) -> String {
    clip_str(&v.to_string())
}

fn clip_str(s: &str) -> String {
    s.chars().take(300).collect()
}

/// A member or element that exists on one side only.
fn absent(path: &str, x: Option<&Value>, y: Option<&Value>) -> String {
    let show = |v: Option<&Value>| v.map_or_else(|| String::from("<absent>"), clip);
    format!("{path}: {} != {}", show(x), show(y))
}

/// First differing path between two JSON values, with both values. A
/// member missing on one side differs from a member that is `null`, and
/// arrays of different lengths differ.
fn first_diff(x: &Value, y: &Value, path: &str) -> Option<String> {
    match (x, y) {
        (Value::Object(a), Value::Object(b)) => a
            .keys()
            .chain(b.keys().filter(|k| !a.contains_key(*k)))
            .find_map(|k| {
                let at = format!("{path}.{k}");
                match (a.get(k), b.get(k)) {
                    (Some(p), Some(q)) => first_diff(p, q, &at),
                    (p, q) => Some(absent(&at, p, q)),
                }
            }),
        (Value::Array(a), Value::Array(b)) => (0..a.len().max(b.len())).find_map(|i| {
            let at = format!("{path}.{i}");
            match (a.get(i), b.get(i)) {
                (Some(p), Some(q)) => first_diff(p, q, &at),
                (p, q) => Some(absent(&at, p, q)),
            }
        }),
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
    /// Cases that differ exactly as they declare (see `known.rs`).
    known: usize,
    /// Cases that declare a difference which did not occur as documented.
    undocumented: usize,
    by_path: BTreeMap<String, usize>,
}

enum Record<'a> {
    Rendered {
        svg: &'a str,
        sem: &'a str,
        value: Value,
        /// `sem` held lone surrogates, parsed as U+FFFD.
        lone_surrogates: bool,
    },
    Error(&'a str),
}

impl<'a> Record<'a> {
    fn read(v: &'a Value) -> Result<Self, Error> {
        let object = v.as_object().ok_or("record must be an object")?;
        let text = |key: &str| {
            object
                .get(key)
                .and_then(Value::as_str)
                .ok_or_else(|| format!("record field {key:?} must be a string"))
        };
        if object.contains_key("error") {
            if object.contains_key("svg") || object.contains_key("sem") {
                return Err("record cannot contain both error and rendered output".into());
            }
            return Ok(Self::Error(text("error")?));
        }
        let (svg, sem) = (text("svg")?, text("sem")?);
        let invalid = |e: serde_json::Error| format!("record field \"sem\" is not valid JSON: {e}");
        let (value, lone_surrogates) = match serde_json::from_str(sem) {
            Ok(v) => (v, false),
            Err(e) => match surrogates::replace_lone(sem) {
                Some(replaced) => (serde_json::from_str(&replaced).map_err(invalid)?, true),
                None => return Err(invalid(e).into()),
            },
        };
        Ok(Self::Rendered {
            svg,
            sem,
            value,
            lone_surrogates,
        })
    }
}

impl Tally {
    /// Records one case and returns the problem to report, if any.
    fn case(&mut self, x: Record<'_>, y: Record<'_>) -> Option<String> {
        self.cases += 1;
        let (svg_x, xs, xv, svg_y, ys, yv, lone) = match (x, y) {
            (
                Record::Rendered {
                    svg: a,
                    sem: b,
                    value: c,
                    lone_surrogates: l1,
                },
                Record::Rendered {
                    svg: d,
                    sem: e,
                    value: f,
                    lone_surrogates: l2,
                },
            ) => (a, b, c, d, e, f, l1 || l2),
            // Oracle and Rust error messages need not use the same wording.
            (Record::Error(_), Record::Error(_)) => return None,
            (Record::Error(e), _) => {
                self.errors += 1;
                return Some(format!(
                    "error mismatch: oracle={} rust renders",
                    clip_str(e)
                ));
            }
            (_, Record::Error(e)) => {
                self.errors += 1;
                return Some(format!(
                    "error mismatch: oracle renders rust={}",
                    clip_str(e)
                ));
            }
        };
        let sem = if xs == ys {
            None
        } else {
            // Equal parsed values with different text (e.g. `1` and `1.0`)
            // are still a serialization difference.
            match first_diff(&xv, &yv, "sem") {
                Some(d) => Some(d),
                // Equal once lone surrogates read as U+FFFD: a documented
                // difference, but only accepted where a case declares it.
                None if lone => Some(String::from(
                    "sem differs only by lone UTF-16 surrogates; declare \"known\": \"lone-surrogate\" if intended",
                )),
                None => Some(format!(
                    "sem text differs: {} != {}",
                    clip_str(xs),
                    clip_str(ys)
                )),
            }
        };
        if sem.is_some() {
            self.sem += 1;
        }
        if svg_x != svg_y {
            self.svg += 1;
        }
        let problem = sem.or_else(|| (svg_x != svg_y).then(|| svg_diff(svg_x, svg_y)));
        if let Some(p) = &problem {
            let key = if p.starts_with("svg differs") {
                String::from("svg")
            } else {
                bucket(p)
            };
            *self.by_path.entry(key).or_default() += 1;
        }
        problem
    }
}

/// Compares the record files; returns whether they agree.
pub fn run(cases: &Path, oracle: &Path, rust: &Path, max: usize) -> Result<bool, Error> {
    let mut out = std::io::stdout().lock();
    compare(open(cases)?, open(oracle)?, open(rust)?, max, &mut out)
}

/// Compares record streams, reporting to `out`. Every non-empty case line
/// must have exactly one record in each stream.
fn compare(
    cases: impl BufRead,
    oracle: impl BufRead,
    rust: impl BufRead,
    max: usize,
    out: &mut impl Write,
) -> Result<bool, Error> {
    let (mut a, mut b) = (oracle.lines(), rust.lines());
    let mut tally = Tally::default();
    for (i, line) in cases.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let (Some(x), Some(y)) = (next_record(&mut a)?, next_record(&mut b)?) else {
            return Err(format!("record files end before case #{i}").into());
        };
        let expected = x
            .get("expected")
            .map(Record::read)
            .transpose()
            .map_err(|e| format!("oracle control for case #{i}: {e}"))?;
        let x = Record::read(&x).map_err(|e| format!("oracle record for case #{i}: {e}"))?;
        let y = Record::read(&y).map_err(|e| format!("rust record for case #{i}: {e}"))?;
        let declared = known::declared(&line).map_err(|e| format!("case #{i}: {e}"))?;
        let problem = match declared {
            Some(kind) => {
                tally.cases += 1;
                match known::check(&kind, &x, &y, expected.as_ref()) {
                    Ok(()) => {
                        tally.known += 1;
                        None
                    }
                    Err(why) => {
                        tally.undocumented += 1;
                        Some(why)
                    }
                }
            }
            None => tally.case(x, y),
        };
        if let Some(problem) = problem {
            if tally.reported < max {
                writeln!(out, "#{i} {line}\n   {problem}")?;
            }
            tally.reported += 1;
        }
    }
    if next_record(&mut a)?.is_some() || next_record(&mut b)?.is_some() {
        return Err(format!("record files hold more than {} records", tally.cases).into());
    }
    if tally.cases == 0 {
        return Err("case list is empty".into());
    }
    writeln!(
        out,
        "cases {}: svg mismatches {}, semantic mismatches {}, error mismatches {}",
        tally.cases, tally.svg, tally.sem, tally.errors
    )?;
    if tally.known + tally.undocumented > 0 {
        writeln!(
            out,
            "declared known differences (UPSTREAM.md): {} as documented, {} not",
            tally.known, tally.undocumented
        )?;
    }
    let mut paths: Vec<(&String, &usize)> = tally.by_path.iter().collect();
    paths.sort_by(|p, q| q.1.cmp(p.1));
    for (path, n) in paths.into_iter().take(15) {
        writeln!(out, "  {n}\t{path}")?;
    }
    Ok(tally.svg + tally.sem + tally.errors + tally.undocumented == 0)
}

mod known;
mod surrogates;

#[cfg(test)]
mod tests;
