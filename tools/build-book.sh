#!/usr/bin/env bash
# Build the book, its native example images and the matching Rust/WASM renderer.
set -euo pipefail
cd "$(dirname "$0")/.."
task_target="${CARGO_TARGET_DIR:-$PWD/target}"
mkdir -p target/book-assets
cargo run --quiet --locked -p milsymbol --features serde --example book > target/book-assets/examples.json
cargo run --quiet --locked -p milsymbol --features serde --example map_marker > target/book-assets/book-map.svg
cargo run --quiet --locked -p milsymbol --features serde --example standards > target/book-assets/book-standards.svg
cargo build --locked -p milsymbol-book --target wasm32-unknown-unknown --release
wasm-bindgen --target web --no-typescript --out-dir target/book-wasm \
  "$task_target/wasm32-unknown-unknown/release/milsymbol_book.wasm"
mdbook build
mkdir -p target/book/wasm
cp target/book-wasm/milsymbol_book.js target/book-wasm/milsymbol_book_bg.wasm target/book/wasm/
python3 - <<'PY'
import json
import shutil
from pathlib import Path
Path('target/book/wasm/package.json').write_text('{"type":"module"}\n')
images = Path('target/book/images')
images.mkdir(exist_ok=True)
for name,svg in json.loads(Path('target/book-assets/examples.json').read_text()).items():
    (images / f'book-{name}.svg').write_text(svg)
for path in Path('target/book-assets').glob('*.svg'):
    shutil.copyfile(path, images / path.name)
PY
