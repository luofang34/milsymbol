# Changelog

## 0.1.0 — unreleased

First release: a native Rust port of milsymbol.js 3.0.4.

### Symbology

- Numeric (MIL-STD-2525D/E, APP-6D/E) and letter (MIL-STD-2525B/C, APP-6B)
  SIDCs, with all symbol parts, text fields and style options of 3.0.4.
- SVG output byte-identical to milsymbol.js on a 288,489-case corpus.
  `RendererConfig::reference_platform` reproduces either V8's x64 or arm64
  `Math.sin`/`Math.cos`, whose last bit differs between the two.

### API

- `Renderer` holds configuration and extensions; no global state.
  `CachedRenderer` (std) memoizes symbols, optionally with parsed paths.
- Typed drawing IR (`ir::Node`, typed path segments) for non-SVG renderers;
  `Symbol::write_svg` appends to a reused buffer, `SvgOptions::id_prefix`
  keeps ids unique when several symbols share a page.
- Typed metadata (`Symbol::metadata`, `domain`) and validity issues;
  milsymbol.js representations (`compat::JsMetadata`, canonical JSON) in
  `compat`.
- Strict SIDC parsing (`sidc::Sidc`) and `Renderer::check_sidc`, which also
  checks what the renderer and its extensions can draw.
- Extensions: custom symbol parts (`SymbolPart`), icons, icon parts and label
  overrides (`IconExtension`, queried by key), colour modes.

### Platforms

- No JavaScript at build or run time; the only dependency is `libm`.
- `no_std + alloc` core; builds for `wasm32` and bare-metal targets.

Deliberate differences from milsymbol.js are listed in `UPSTREAM.md`.
