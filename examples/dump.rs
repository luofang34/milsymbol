//! Renders oracle cases (JSON lines on stdin) and prints one JSON record per
//! line: `{"svg": …, "sem": …}` or `{"error": …}`. Used with
//! `tools/oracle/compare.mjs` to diff against milsymbol.js.

#[path = "../tests/support.rs"]
mod support;

use std::io::{BufRead, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = std::io::stdin();
    let mut out = std::io::BufWriter::new(std::io::stdout());
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let case: serde_json::Value = serde_json::from_str(&line)?;
        let rec = match support::render(&case) {
            Ok((svg, sem)) => serde_json::json!({ "svg": svg, "sem": sem }),
            Err(e) => serde_json::json!({ "error": e }),
        };
        writeln!(out, "{rec}")?;
    }
    Ok(())
}
