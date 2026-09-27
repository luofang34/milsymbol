//! Development tasks for the milsymbol crate.
//!
//! - `cargo xtask emit`: writes `src/generated/*.rs` from the tables that
//!   `tools/codegen/extract*.mjs` extracted from upstream milsymbol.js.
//! - `cargo xtask compare CASES ORACLE RUST [MAX]`: compares oracle and Rust
//!   record files produced from the same cases (see `tools/oracle/diff.sh`).
//!
//! Only the steps that must execute upstream JavaScript remain in Node.

use std::path::PathBuf;
use std::process::ExitCode;

mod compare;
mod emit;
mod jsnum;

/// Error type of the tasks; messages carry the context.
type Error = Box<dyn std::error::Error>;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn usage() -> ExitCode {
    tracing::error!("usage: cargo xtask emit | compare CASES ORACLE RUST [MAX_REPORTS]");
    ExitCode::from(2)
}

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_target(false)
        .without_time()
        .with_writer(std::io::stderr)
        .init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("emit") => emit::run(&repo_root()).map(|()| true),
        Some("compare") => match args.get(1..4) {
            Some([cases, oracle, rust]) => {
                let max = args.get(4).and_then(|m| m.parse().ok()).unwrap_or(10);
                compare::run(cases.as_ref(), oracle.as_ref(), rust.as_ref(), max)
            }
            _ => return usage(),
        },
        _ => return usage(),
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            tracing::error!("{e}");
            ExitCode::FAILURE
        }
    }
}
