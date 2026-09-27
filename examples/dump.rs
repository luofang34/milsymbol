//! Renders oracle cases (JSON lines on stdin) and prints one JSON record per
//! line: `{"svg": …, "sem": …}` or `{"error": …}`. Used with
//! `cargo xtask compare` to diff against milsymbol.js. `ORACLE_ARCH=arm64`
//! reproduces V8's arm64 floating-point results instead of x64's.

#[path = "../tests/support.rs"]
mod support;

use std::io::{BufRead, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let platform = match std::env::var("ORACLE_ARCH").as_deref() {
        Ok("arm64") => milsymbol::ReferencePlatform::Arm64,
        _ => milsymbol::ReferencePlatform::X64,
    };
    let stdin = std::io::stdin();
    let mut out = std::io::BufWriter::new(std::io::stdout());
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let case: serde_json::Value = serde_json::from_str(&line)?;
        let rec = match support::render_for(&case, platform) {
            Ok((svg, sem)) => serde_json::json!({ "svg": svg, "sem": sem }),
            Err(e) => serde_json::json!({ "error": e }),
        };
        writeln!(out, "{rec}")?;
    }
    Ok(())
}
