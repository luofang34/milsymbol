# Changelog

## 0.1.0 — 2026-09-28

First release: a native Rust port of milsymbol.js 3.0.4.

- Numeric (MIL-STD-2525D/E, APP-6D/E) and letter (MIL-STD-2525B/C, APP-6B)
  SIDCs, with all symbol parts, text fields and style options of 3.0.4.
- SVG output byte-identical to milsymbol.js on a 288,489-case corpus.
  Configuration selects the x64 or arm64 V8 reference platform.
- Typed drawing instructions, metadata, strict SIDC validation and layout
  information for native, map and non-SVG renderers.
- Explicit renderer configuration and extensions for icons, symbol parts,
  labels and colour modes; no mutable global rendering state.
- Reusable output buffers, streaming canonical JSON and an optional bounded
  symbol cache (`std`).
- `no_std + alloc` core, WASM and bare-metal builds, Rust 1.85 or later.
  No JavaScript at build or run time; the only runtime dependency is `libm`.

See [UPSTREAM.md](UPSTREAM.md#known-differences) for deliberate compatibility
differences. Large corpus fixtures remain in the Git repository; the
published package includes the other tests and their reference data.
