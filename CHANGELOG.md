# Changelog

## 0.1.0 — unreleased

First release: native Rust port of milsymbol.js 3.0.4.

- Numeric (2525D/E, APP-6D/E) and letter (2525B/C, APP-6B) SIDCs.
- All symbol parts, text fields and style options of 3.0.4.
- SVG output byte-identical to milsymbol.js on a 288,489-case corpus.
- Typed drawing IR with typed path segments.
- Renderer-scoped configuration and extensions (symbol parts, icons, labels,
  colour modes).
- `no_std + alloc`; `wasm32` and bare-metal builds.
- Strict typed SIDC parsing (`sidc::Sidc`), typed symbol info
  (`Symbol::metadata`, `domain`) and typed validity issues.
- `ReferencePlatform`: reproduces V8's x64 or arm64 `Math.sin`/`Math.cos`
  bit for bit (upstream output differs between them).
- Typed path construction (`PathData::from_segments`) and cached segments.
- Typed `Symbol::metadata()`; milsymbol.js representations (`JsMetadata`,
  canonical JSON) live in `compat`.
- `IconExtension` is queried by key (`icon_part`, `icon(IconKey)`,
  `icon_bbox`); public data structs are `#[non_exhaustive]` with
  constructors.
- `Sidc::parse` checks the standards' code tables; `Renderer::check_sidc`
  also checks renderer support; `is_sidc_valid` ignores icon visibility.
- `CachedRenderer::with_prepared_paths`; unique clip-path ids and
  `SvgOptions::id_prefix`; SVG path grammar enforced by the path parser.
- `Symbol::write_svg` appends to a caller's buffer; SVG serialization,
  `is_valid()` and cache hits make no temporary allocations.
- Enum-backed internal metadata and borrowed `JsMetadata` views;
  `ColorMode::for_affiliation` borrows a paint using a typed affiliation.
- Streaming `compat::write_canonical_json`, shared field definitions with
  the owned JSON view, UTF-16 key ordering and any number of extension
  option keys.
- `NumericSidc::modifier_codes` returns the complete three-digit modifier
  codes that extensions receive; `0x`/`0o`/`0b` numeric strings round once,
  as JavaScript `Number` does.
- Oracle comparison reports lone UTF-16 surrogates (non-BMP SIDCs) as a
  known difference.
- Oracle comparison rejects malformed records; path preparation caches all
  valid paths even when another path or clip geometry fails to parse.
- SIDC support checks include icons when a custom pipeline omits the icon
  stage. Canonical options have unique keys and follow JavaScript property
  ordering, including numeric extension keys. Oracle JSON-lines input
  preserves Unicode line separators inside strings.
