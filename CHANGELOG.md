# Changelog

## Unreleased

Breaking changes for 0.3.

- Cached and uncached builders are one type. `SymbolBuilder<'a>` is now
  `RequestBuilder<'a, Renderer>` and `CachedSymbolBuilder<'a>` is
  `RequestBuilder<'a, &'a CachedRenderer>` (new, with the sealed `Backend`
  trait), so every setter, `font()` included, exists on both.
  `CachedSymbolBuilder::strict()` is new and behaves as
  `SymbolBuilder::strict()`: same errors, same checks, extension icons
  included. Strict and lenient requests have separate cache entries and a
  strict failure is never cached; hits still allocate nothing. Migration:
  code that names `SymbolBuilder`/`CachedSymbolBuilder` as types keeps
  compiling; code that was generic over the two, or matched on them as
  structs, must go through `RequestBuilder`. `cargo semver-checks` reports
  this as `struct_missing` for both names; it is the only failure it finds.
- `Symbol::validity()` now judges the SIDC alone (it is the old
  `sidc_validity()`): the code must parse, every part must be recognised and
  the icon must exist whether or not it is drawn. Text containing `null` no
  longer makes a symbol invalid, and `ValidityIssue::MalformedSidc` is
  reported by it. The verdict does not depend on options; the issue list can
  differ with `style.icon`. `cargo semver-checks` does not model this change
  of meaning. Migration: for milsymbol.js `isValid()` use
  `compat::validity(&symbol)` or `compat::is_valid(&symbol)`, which keep the
  `null` rule and the hidden-icon rule and are what the differential corpus
  uses.
- `Symbol::sidc_validity()` is deprecated: it is identical to `validity()`.
- `compat::validity` is new; `compat::is_valid` now implements upstream's
  verdict itself instead of calling `Symbol::validity`.
- `Renderer::check_sidc` shares the strict implementation with `strict()`;
  its results are unchanged.

## 0.2.1 — 2026-10-01

Additive release; SVG output is unchanged.

- `Sidc`, `NumericSidc` and `LetterSidc` change status, affiliation, standard
  identity and context with `with_status`, `with_affiliation`,
  `with_standard_identity` and `with_context`, covering the differences
  between numeric and letter codes (Hostile is Faker in an exercise; letter
  codes have no simulation context). Results are valid by construction;
  identities a scheme cannot encode in the SIDC's context return the new
  `SidcModifyError`.
- `SymbolBuilder::font`, `CachedSymbolBuilder::font` and
  `SymbolOptions::set_font` draw all text, including the text inside built-in
  icons, in one font family. The default still preserves milsymbol.js output.
- Documentation of the size, padding, anchor and pixel-density conventions for
  map use, checked by doctests, and a `map_marker` example that places symbols
  by their anchors.
- Smaller static footprint: the generated icon tables use packed node records,
  interned paths, deduplicated text payloads and bit-packed row tables.
  `.rodata` of a Cortex-M4F build shrinks from 2.29 MB to 0.95 MB. Output is
  unchanged (full differential corpus) and render times are within
  measurement noise.

## 0.2.0 — 2026-09-30

Breaking API changes toward typed, single-path use; SVG output is unchanged.

Migrating from 0.1: `Renderer::default().with_x(..)` becomes
`Renderer::builder().x(..).build()`, `field::X` becomes `TextField::X`,
colour strings become `Color::new(..)` and `symbol.is_valid()` becomes
`symbol.validity().is_valid()`.

- Options are typed: `options::TextField` replaces the `field` name
  constants, `options::Color` and `ColorChoice` replace colour strings, and
  "not set" is `None` (`speed_leader`, `hq_staff_length` and every optional
  colour are `Option`). `SymbolOptions::set` keeps the milsymbol.js names.
  NaN and infinite numbers are rejected with `RenderError::InvalidOption`.
- `Renderer::symbol` is the entry point. It takes text or a parsed `Sidc`,
  has setters for the common options, and `strict()` fails on malformed or
  unrecognised SIDCs. `CachedRenderer::symbol` has the same setters.
- `Renderer` is built with `Renderer::builder()`, is immutable and `Clone`;
  the `with_*` methods and `config_mut` are gone.
- `Symbol::drawing()` returns a flat, typed `Drawing` (resolved transforms,
  paint, dashes, clips, text attributes). `cache_path_segments` and
  `CachedRenderer::with_prepared_paths` are removed.
- `Symbol::is_valid`, `is_sidc_valid` and `js_metadata` are replaced by
  `Symbol::validity`, `Symbol::sidc_validity`, `compat::is_valid` and
  `compat::js_metadata`.
- `IconPartContext::mono_color` is `Option<&str>`. `SymbolOptions::text` is
  no longer a public field; use `text`, `text_named`, `text_fields` and
  `set_text`. `LabelField::for_field` takes a `TextField`.
- `Renderer::symbol` builders own a handle on the renderer, so they can be
  stored; `Renderer` is `Rc`-shared (not `Send`) on targets without atomic
  pointers such as thumbv6m.
- New `serde` feature for options, configuration, metadata and the drawing
  view.
- Stricter lints (`dead_code`, `unused`, `unreachable_pub`, `clippy::all`)
  for every crate, enforced on stable, beta and the minimum Rust version;
  fuzz targets for the SIDC parser, rendering and option assignment;
  semver, feature-combination and lint-configuration checks in CI.

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
differences.
