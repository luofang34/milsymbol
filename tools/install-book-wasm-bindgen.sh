#!/usr/bin/env bash
# Install the wasm-bindgen CLI matching the exact pin in tools/book-wasm/Cargo.toml.
set -euo pipefail
cd "$(dirname "$0")/.."
version=$(python3 -c 'import tomllib; print(tomllib.load(open("tools/book-wasm/Cargo.toml", "rb"))["dependencies"]["wasm-bindgen"].removeprefix("="))')
cargo install --locked wasm-bindgen-cli --version "$version"
