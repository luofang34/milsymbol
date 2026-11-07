#!/usr/bin/env bash
# Compile the manual against one isolated library build so rustdoc resolves
# the same feature set for every example.
set -euo pipefail
cd "$(dirname "$0")/.."
export CARGO_TARGET_DIR="$PWD/target/manual-tests"
CARGO_TARGET_DIR="$PWD/target/book-tests" tools/build-book.sh
python3 tools/check-book-links.py
node tools/book-wasm.test.mjs
CARGO_TARGET_DIR="$PWD/target/book-tests" cargo test --locked -p milsymbol-book
cargo clean -p milsymbol -p serde_json
cargo test --locked -p milsymbol --features serde --test serde --no-run
mdbook test -L "$CARGO_TARGET_DIR/debug/deps"
