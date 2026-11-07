#!/usr/bin/env bash
# Compile the manual against one isolated library build so rustdoc resolves
# the same feature set for every example.
set -euo pipefail
cd "$(dirname "$0")/.."
export CARGO_TARGET_DIR="$PWD/target/manual-tests"
mdbook build
python3 tools/check-book-links.py
cargo test --locked -p milsymbol --features serde --test serde --no-run
mdbook test -L "$CARGO_TARGET_DIR/debug/deps"
