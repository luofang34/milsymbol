# Changelog

## 0.4.2 — 2026-10-08

Documentation only; the code and SVG output are unchanged.

- The crate documentation and README say which later standard revisions
  milsymbol.js 3.0.4, and therefore this crate, does not recognise: version
  digits `15` (MIL-STD-2525E Change 1) and `16` (APP-6(E) Version 2),
  contexts `3`–`8`, symbol set `64` and letters in numeric SIDCs. A doctest
  checks each statement, including how a version `16` SIDC is drawn.
- `TextField::Country`, `HeadquartersElement` and `InstallationComposition`
  name their APP-6(E) fields (AS, AW, AX) next to the field letters
  milsymbol.js documents (AC, AH, AI). In APP-6(E), AH and AI are the area
  of uncertainty and the dead reckoning trailer.

## 0.4.1 — 2026-10-03

SVG output is unchanged.

- `Symbol::drawing` ignores a fill or stroke that is not a CSS colour and
  keeps the inherited paint, as SVG readers do. Before, such an item
  carried the invalid colour text. Upstream's own suspect frame colour
  (`rbg(255, 188, 1)`, see UPSTREAM.md) is one such value.
- `Symbol::drawing` reports a `fill` or `stroke` of `none` as
  `drawing::Paint::None` instead of a colour named `none`.
- `Symbol::drawing` borrows colours from the built-in tables instead of
  copying each into a new `String`: an infantry symbol's drawing allocates
  4 heap blocks instead of 12, a letter SIDC's 5 instead of 26. Against
  0.4.0 in 8 interleaved rounds, building the drawing of all 1,431 numeric
  icons is 3.6% faster (minimum) and every other benchmark stays within the
  control build's spread or the 1–3% code-layout noise of the machine.
- New differential test `tests/drawing_svg.rs`: every symbol of the oracle
  corpus is read back from its SVG with usvg and compared with the drawing
  view, item by item, and a sample is painted from both and compared.
- New oracle suite `layout` (7,264 cases): information fields and style on
  every frame shape, and 40 inputs on which milsymbol.js throws. Where
  upstream throws, the port must now fail for the same reason, not merely
  fail.
- Documented known difference: a colour mode named after an
  `Object.prototype` property (`constructor`, `toString`, …) renders without
  fill colours upstream and is `RenderError::UnknownColorMode` here.

## 0.4.0 — 2026-10-02

Breaking change to one method. Rendered output is unchanged. The default
build's code is unchanged: a benchmark binary and a thumbv7em firmware image
built from 0.4.0 and from 0.3.1 with the same flags are byte-identical, because
nothing uses the new `source` and `write_source` internally.

- `PathData::source()` returns `Cow<'_, str>` instead of `&str`.
  `cargo semver-checks` does not detect a changed return type, so this is
  listed here and was found by diffing the public item signatures. It is
  always `Cow::Borrowed` in the default build and derefs to `&str`, so most
  callers compile unchanged; code that needs a `String` calls
  `.into_owned()` (`.to_owned()` on a `Cow` is another `Cow`). The new
  `PathData::write_source(&mut impl fmt::Write)` streams the text and never
  allocates; use it in writers. `source()` is `#[must_use]`.
- With the `compact-paths` feature a composed symbol now keeps each built-in
  path packed instead of decoding it into a `String` when it is composed:
  no extra heap per path, and cloning a symbol allocates nothing for its
  paths. Composing a symbol is about as fast as without the feature and
  building the drawing view of composed symbols is faster, but the SVG and
  JSON writers now decode the packed bytes as they write, so composing and
  writing SVG is still slower than without the feature (a few percent for one
  symbol, about a fifth over all built-in icons). `PathData::segments`
  decodes the packed bytes directly. A packed path's `source()` builds a new
  `String` on every call (documented). The figures, including the small gzip
  saving, are in the crate documentation ("Compact path data").
- With `compact-paths` the derived `Debug` output of `PathData` (and so of
  `PathNode`, `Node` and `Symbol`) prints a built-in path as `Packed([..])`
  and a path you built as `Text("..")`, instead of the bare text. Do not
  parse `Debug` output.
- Other public API changes made now so the 0.4 series can stay stable:
  `BuiltinPart::DEFAULT` is a `&'static [BuiltinPart]` instead of a
  `[BuiltinPart; 9]` (its length is no longer part of the type; `.iter()`
  and indexing keep working); `drawing::LineCap`, `LineJoin` and `TextAnchor`
  are `#[non_exhaustive]` like the other drawing enums (matches need a
  wildcard arm); `RequestBuilder::render` returns `Result<B::Output, _>`
  with `Output` now an associated type of the public, still sealed,
  `Backend` trait (it was an unnameable item of the private sealed trait;
  `Backend` is therefore no longer dyn-compatible, which nothing could use
  before either);
  with `serde`, `Style`, `SymbolOptions` and `RendererConfig` deserialize
  missing fields to their defaults, so stored JSON keeps loading when fields
  are added (serialized output is unchanged and pinned by a test).
- New benchmark `bulk/drawing_all_number_icons_1431`, and a performance gate
  in `RELEASING.md`.

## 0.3.1 — 2026-10-01

Additive release; output and the default build are unchanged (the compiled
library is byte-identical to 0.3.0 without the feature).

- New Cargo feature `compact-paths` (off by default, additive, no public API
  or auto-trait change): the icon tables embed their paths packed and decode
  them when a symbol is composed, trading CPU and heap for flash. Output is
  byte-identical and builds without the feature compile the same code as
  before. Figures and trade-offs are in the crate documentation ("Compact
  path data"). `xtask emit` regenerates both forms and checks every packed
  path against the library decoder.

## 0.3.0 — 2026-10-01

Breaking changes for 0.3. `cargo semver-checks --baseline-rev v0.2.1
--release-type minor` reports four failures, all listed here:
`struct_missing`, `inherent_method_missing`, `enum_variant_missing` and
`enum_no_repr_variant_discriminant_changed`.

- Cached and uncached builders are one type (`struct_missing` for
  `SymbolBuilder` and `cache::CachedSymbolBuilder`). `SymbolBuilder<'a>` is
  now `RequestBuilder<'a, Renderer>` and `CachedSymbolBuilder<'a>` is
  `RequestBuilder<'a, &'a CachedRenderer>` (new, with the sealed `Backend`
  marker trait), so every setter, `font()` included, exists on both.
  `CachedSymbolBuilder::strict()` is new and behaves as
  `SymbolBuilder::strict()`: same errors, same checks, extension icons
  included. Strict and lenient requests have separate cache entries and a
  strict failure is never cached; hits still allocate nothing for requests
  whose key fits in 1 KiB. Migration: code that names
  `SymbolBuilder`/`CachedSymbolBuilder` as types keeps compiling; code that
  was generic over the two, or matched on them as structs, must go through
  `RequestBuilder`.
- `Symbol::validity()` is removed (`inherent_method_missing`), so no call
  silently changes meaning. Migration: `Symbol::sidc_validity()`, unchanged,
  judges the SIDC (it parses, every part is recognised, the icon exists
  whether or not it is drawn; the verdict does not depend on options, the
  issue list can differ with `style.icon`); `compat::validity(&symbol)` and
  `compat::is_valid(&symbol)` are milsymbol.js `isValid()`, with its
  `null`-text and hidden-icon rules, and are what the differential corpus
  uses.
- `ValidityIssue` is the SIDC verdict only: `NullInDrawing` moves to the new
  `compat::UpstreamIssue` (`enum_variant_missing`), and `MalformedSidc`'s
  discriminant changes from 6 to 5 (`enum_no_repr_variant_discriminant_changed`;
  the enum is `non_exhaustive`, so match on variants, not numbers).
  `compat::validity` is new and returns `compat::UpstreamValidity`;
  `compat::is_valid` implements upstream's verdict itself.
- Cache hits are about 17% faster (137 ns to 113 ns median on an Apple M3
  Max, 11 of 11 paired rounds): the index uses a fast hasher seeded per cache
  instead of SipHash.
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
