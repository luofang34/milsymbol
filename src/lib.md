Military symbols per MIL-STD-2525 (B/C/D/E) and APP-6 (B/D/E) in native
Rust, as SVG or as typed drawing data for any other renderer. A port of
[milsymbol.js](https://github.com/spatialillusions/milsymbol) 3.0.4 that
follows the current published standards by default.

This page is the API reference. The
[usage manual](https://luofang34.github.io/milsymbol/) covers application
workflows (map placement, styling, drawing adapters, WASM and embedded
targets) and has an interactive symbol lab; the
[README](https://github.com/luofang34/milsymbol#readme) has a picture
gallery.

```
use milsymbol::{Renderer, options::TextField};

let renderer = Renderer::default();
let symbol = renderer
    .symbol("10031000001211000000")
    .text(TextField::UniqueDesignation, "1-66")
    .render()?;
let svg = symbol.to_svg();
assert!(svg.starts_with("<svg"));
assert!(symbol.sidc_validity().is_valid());
# Ok::<(), milsymbol::RenderError>(())
```

# Where to start

| You want to | Use |
|---|---|
| Configure standard, colour modes and extensions once | [`Renderer::builder`], or [`Renderer::default`] |
| Render a symbol | [`Renderer::symbol`] → [`SymbolBuilder`] → [`render`](SymbolBuilder::render) |
| Set options | builder setters, [`SymbolBuilder::with`], or a reusable [`options::SymbolOptions`] |
| Get SVG or a data URL | [`Symbol::to_svg`], [`Symbol::to_data_url`], [`Symbol::write_svg_with`] |
| Place the image on a map | [`Symbol::anchor`], [`Symbol::size`] (see [`Symbol`]) |
| Draw through a GPU, canvas or scene backend | [`Symbol::drawing`] ([`drawing`]) |
| Read what a symbol is | [`Symbol::metadata`] ([`Metadata`], [`domain`]) |
| Parse, validate or edit a SIDC | [`sidc`] |
| List codes for a picker | [`catalog`] |
| Reuse rendered symbols | `cache::CachedRenderer` (`std` feature) |
| Add icons or drawing stages | [`IconExtension`], [`SymbolPart`] |
| Compare with milsymbol.js | [`compat`], [`ir`] |

# Supported standards

[`RenderMode::Standards`] is the default. The SIDC version selects code
meanings; [`Standard`] selects the current display rules independently.

| SIDC version | Code meanings |
|---|---|
| `10`–`12` | Edition D tables from milsymbol.js |
| `13`, `14` | MIL-STD-2525E / APP-6(E)(1) tables from milsymbol.js |
| `15` | MIL-STD-2525E Change 1, March 2025 |
| `16` | APP-6(E) Version 2, October 2025 |
| Letter forms | MIL-STD-2525B/C and APP-6B tables from milsymbol.js |

All edition E versions accept restricted/no-strike contexts `3`–`8`, symbol
set `64` and uppercase hexadecimal code characters. For numeric SIDCs,
`Standard::Mil2525` draws with MIL-STD-2525E Change 1 display rules;
`Standard::App6` and `Standard::App6EVersion2` draw with APP-6(E)(2) rules.
APP-6 reserves digits 23–27. MIL uses digit 23 to override the frame shape.

[`catalog::standards`] exposes the pinned editions' codes, names, source
pages and drawing scope. Single-point symbols are drawable; hierarchy rows
have no icon. Lines and areas requiring geographic control points are
explicitly classified as multipoint and are unsupported. APP-6's US-only
sets use the pinned MIL tables. Unknown entity/modifier codes draw an unknown
marker; `.strict()` rejects them.

```
use milsymbol::{Renderer, RenderMode, Standard};
let app6 = Renderer::builder().standard(Standard::App6).build();
let group = app6.symbol("16034000001310000000").strict().render()?;
assert!(group.sidc_validity().is_valid());

// Exact upstream output is an explicit renderer choice.
let upstream = Renderer::builder().mode(RenderMode::Milsymbol304).build();
assert!(upstream.check_sidc("16031000001211000000").is_err());
# Ok::<(), milsymbol::RenderError>(())
```

[`RenderMode::Milsymbol304`] preserves the pinned JavaScript oracle's SVG
and semantic output. Use [`Sidc::parse_with_mode`](sidc::Sidc::parse_with_mode)
when parsing separately for that mode. Fonts, line widths and other details
left open by the standards retain upstream behavior.

# Rendering

A [`Renderer`] holds all configuration and extensions. Start a symbol with
[`Renderer::symbol`], which takes the SIDC as text or as a parsed
[`sidc::Sidc`], and set options on the [`SymbolBuilder`] it returns.
Options are typed values: [`options::TextField`] names the text amplifiers,
[`options::Color`] holds a CSS colour, and `None` means "not set".

```
use milsymbol::Renderer;
use milsymbol::options::{Color, TextField};

let renderer = Renderer::default();
let symbol = renderer
    .symbol("10031000161211000000")
    .size(40.0)
    .text(TextField::HigherFormation, "2 BDE")
    .direction(45.0)
    .with(|o| o.style.info_color = Color::new("red").ok().map(Into::into))
    .render()?;
let size = symbol.size(); // pixels
let anchor = symbol.anchor(); // where the map position goes, in pixels
assert!(size.width > 0.0 && anchor.y > 0.0);
# Ok::<(), milsymbol::RenderError>(())
```

Complete [`options::SymbolOptions`] can be built ahead of time and passed to
[`SymbolBuilder::options`] or [`Renderer::render`].
[`options::SymbolOptions::set`] accepts milsymbol.js option names and rejects
unknown names or wrongly typed values, for configuration read from text.
Numbers that cannot appear in an SVG (NaN, infinities) are rejected with
[`RenderError::InvalidOption`].

[`SymbolBuilder::font`] draws all text in one font family. Built-in icons
otherwise keep the font of their template, as milsymbol.js draws them, and the
option's `font_family` alone only affects the information fields.

A [`Symbol`] is immutable. [`Symbol::to_svg`] returns a document identical
to milsymbol.js `asSVG()`, and [`Symbol::to_data_url`] returns a
`data:image/svg+xml;utf8,…` URL identical to `toDataURL()`.
[`Symbol::write_svg`] and [`Symbol::write_data_url`] append to a reused
buffer, and the `_with` variants take [`SvgOptions`] (an id prefix for
inlining several symbols in one page). [`Symbol::drawing`] is the same
picture as flat typed items for renderers other than SVG, and
[`Symbol::instructions`] is the tree exactly as milsymbol.js builds it
([`ir`]).

Rendering is deterministic: the same renderer configuration, SIDC and
options always give the same output, on every platform and thread.
`Renderer` and `Symbol` are `Send + Sync`.

# SIDCs and validity

Like milsymbol.js, rendering accepts any string and draws unknown parts as a
question mark. The [`sidc`] module parses, validates and edits codes.

| You want to | Call |
| --- | --- |
| Check that text is a well-formed SIDC, without a renderer | [`Sidc::parse`](sidc::Sidc::parse) |
| Check that a SIDC is well formed and that your renderer, extensions included, can draw all of it | [`Renderer::check_sidc`] |
| Draw, but fail on a malformed or unrecognised SIDC instead of drawing `?` | [`SymbolBuilder::strict`], on the cached builder too |
| Draw anything and inspect the verdict afterwards (the recommended call) | [`Symbol::sidc_validity`] |
| Compare with milsymbol.js `isValid()`, quirks included | [`compat::validity`] / [`compat::is_valid`] |

[`Symbol::sidc_validity`] lists every [`ValidityIssue`] for the SIDC alone: it
must be well formed and fully recognised, whether or not the icon is drawn.
`strict()` and `check_sidc` apply the same test before returning a symbol.
[`compat::validity`] reproduces upstream, including its rule that text
containing `null` makes a symbol invalid; it returns its own
[`compat::UpstreamIssue`] list.

# Extensions

What milsymbol.js configures globally (`ms.addSymbolPart`, `ms.addIcons`,
`ms.setColorMode`, …) is renderer state here, so differently configured
renderers coexist:

- [`SymbolPart`] adds a drawing stage ([`RendererBuilder::symbol_part`]).
- [`IconExtension`] adds or replaces icon parts, icons for new SIDCs and
  label placements ([`RendererBuilder::icons`]). It is queried by key, and can
  compose new icons from the built-in parts through [`PartLookup`].
- [`RendererBuilder::color_mode`], [`RendererBuilder::dash_arrays`] and
  [`RendererBuilder::hq_staff_length`] replace the other globals.

A built renderer never changes, and cloning it shares its configuration.

# Feature flags

| Feature | Default | Effect |
|---|---|---|
| `std` | on | Adds `cache::CachedRenderer`. Without it the crate is `no_std + alloc` and builds for `wasm32` and bare-metal targets. |
| `serde` | off | Serialization for options, configuration, metadata and the [`drawing`] view. [`options::SymbolOptions`], [`options::Style`] and [`RendererConfig`] also deserialize, with missing fields taking their defaults; [`Metadata`] is `Serialize` only. |
| `compact-paths` | off | Embeds built-in path data packed (about a third of the size) and decodes it while serializing; output is byte-identical, no public signature or auto-trait changes, and cloning a symbol allocates nothing for its paths. See [`ir::PathData::source`]. |

The only runtime dependency is `libm`. No JavaScript runs at build or run
time: the upstream icon tables are generated Rust data, and every
composition rule is ported. `compact-paths` trades static storage for
decoding work; measure the final linked artifact with both builds before
choosing it.

# Compatibility

The baseline is milsymbol.js 3.0.4 (commit `b05f2d7`), each symbol rendered as
by a freshly initialised milsymbol. In [`RenderMode::Milsymbol304`], SVG
output is byte-identical on a 295,753-case differential corpus. Upstream's
direction arrows depend on the last bit of V8's `Math.sin`/`Math.cos`, which
differs between x64 and arm64;
[`RendererConfig::reference_platform`](config::RendererConfig::reference_platform)
selects which one to reproduce. [`compat`] holds milsymbol.js's own
representations (string metadata, the canonical JSON record).

The deliberate differences are documented in
[UPSTREAM.md](https://github.com/luofang34/milsymbol/blob/main/UPSTREAM.md#known-differences).
None occurs in the corpus: they cover upstream's cross-render cache
pollution, inputs on which it throws or never finishes (typed
[`RenderError`]s here), loosely typed options, extension edge cases and
SIDCs with non-BMP characters.
