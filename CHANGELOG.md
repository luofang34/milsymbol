# Changelog

## 0.1.0 — unreleased

First release: native Rust port of milsymbol.js 3.0.4.

- Numeric (2525D/E, APP-6D/E) and letter (2525B/C, APP-6B) SIDCs.
- All symbol parts, text fields and style options of 3.0.4.
- SVG output byte-identical to milsymbol.js on a 247,918-case corpus.
- Typed drawing IR with typed path segments.
- Renderer-scoped configuration and extensions (symbol parts, icons, labels,
  colour modes).
- `no_std + alloc`; `wasm32` and bare-metal builds.
- Strict typed SIDC parsing (`sidc::Sidc`), typed symbol info
  (`Symbol::info`, `domain`) and typed validity issues.
- `ReferencePlatform`: reproduces V8's x64 or arm64 `Math.sin`/`Math.cos`
  bit for bit (upstream output differs between them).
- Typed path construction (`PathData::from_segments`) and cached segments.
