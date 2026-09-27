#!/usr/bin/env bash
# Fails if the crate's runtime dependency tree (any target) contains a
# JavaScript engine or JS-interop crate. The library must stay native Rust.
set -euo pipefail
tree=$(cargo tree -p milsymbol -e normal,build --target all --prefix none --no-dedupe)
banned='^(boa_[a-z_]+|boa|v8|rusty_v8|deno_[a-z_]+|quickjs[a-z_-]*|rquickjs[a-z_-]*|js-sys|wasm-bindgen[a-z-]*|web-sys|mozjs|javascriptcore[a-z-]*|duktape[a-z-]*|ducc|node-bindgen|neon|napi[a-z-]*) '
if echo "$tree" | grep -Eq "$banned"; then
  echo "JavaScript runtime/interop dependency found:" >&2
  echo "$tree" | grep -E "$banned" >&2
  exit 1
fi
if grep -rEn '\b(eval|Function)\s*\(' src --include=*.rs | grep -v '^src/generated/' | grep -Eiq 'js_sys|wasm_bindgen'; then
  echo "JS interop call in library source" >&2
  exit 1
fi
echo "no JavaScript runtime in dependency tree:"
echo "$tree" | sort -u
