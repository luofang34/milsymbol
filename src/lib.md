Native Rust port of [milsymbol.js](https://github.com/spatialillusions/milsymbol)
3.0.4: military symbols per MIL-STD-2525 (B/C/D/E) and APP-6 (B/D/E), as SVG
or as a typed drawing tree for any other renderer.

```
use milsymbol::{Renderer, options::TextField};

let renderer = Renderer::default();
let symbol = renderer
    .symbol("10031000001211000000")
    .text(TextField::UniqueDesignation, "1-66")
    .render()?;
let svg = symbol.to_svg();
assert!(svg.starts_with("<svg"));
assert!(symbol.validity().is_valid());
# Ok::<(), milsymbol::RenderError>(())
```

# Supported standards

| SIDC | Standards |
|---|---|
| Numeric (20 or 30 digits) | MIL-STD-2525D, 2525E; APP-6D, APP-6E (versions `10`–`12` → D, `13`–`14` → E) |
| Letter (10–15 characters) | MIL-STD-2525B (incl. change 2), 2525C; APP-6B |

Whether 2525 or APP-6 rules apply is a renderer setting
([`RendererBuilder::standard`]) that a symbol can override
(`style.standard`).

# Layers

- [`sidc`] and [`domain`]: parse and describe a SIDC with typed values.
- [`options`], [`Renderer`] and [`SymbolBuilder`]: what to draw and how.
- [`Symbol::drawing`] ([`drawing`]): the picture as flat typed items, the
  input for any renderer other than SVG.
- [`Symbol::to_svg`]: the SVG renderer, byte-identical to milsymbol.js.
- [`ir`] and [`compat`]: milsymbol.js's own instruction tree and string
  representations, for extensions and differential testing.

# Rendering

A [`Renderer`] holds all configuration and extensions; build one with
[`Renderer::builder`], or use `Renderer::default()`. Start a symbol with
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
[`SymbolBuilder::options`] or [`Renderer::render`]. To reuse a symbol
description across requests, [`options::SymbolOptions`] is `Clone`.
[`options::SymbolOptions::set`] accepts milsymbol.js option names and rejects
unknown names or wrongly typed values, for configuration read from text.
Numbers that cannot appear in an SVG (NaN, infinities) are rejected with
[`RenderError::InvalidOption`].

A [`Symbol`] is immutable. [`Symbol::to_svg`] returns a document identical
to milsymbol.js `asSVG()`; [`Symbol::write_svg`] appends to a reused buffer,
and [`Symbol::write_svg_with`] takes [`SvgOptions`] (an id prefix for
inlining several symbols in one page).

Rendering is deterministic: the same renderer configuration, SIDC and
options always give the same output, on every platform and thread.
`Renderer` and `Symbol` are `Send + Sync`.

# SIDCs and validity

Like milsymbol.js, rendering accepts any string and draws unknown parts as a
question mark. To validate input, parse it strictly, or ask the renderer
whether it can draw every part (including icons added by extensions):

```
use milsymbol::Renderer;
use milsymbol::domain::{Echelon, StandardIdentity};
use milsymbol::sidc::{Sidc, SidcCheckError};

let Sidc::Numeric(sidc) = Sidc::parse("10031000161211000000")? else {
    return Ok(()); // a 20-digit code is always numeric
};
assert_eq!(sidc.standard_identity(), StandardIdentity::Friend);
assert!(Sidc::parse("10091000001211000000").is_err()); // no identity 9

let renderer = Renderer::default();
assert!(renderer.check_sidc("10031000161211000000").is_ok());
assert!(matches!(
    renderer.check_sidc("10031000009999990000"), // no such entity
    Err(SidcCheckError::Unsupported { .. })
));

// A rendered symbol describes itself with typed values.
let symbol = renderer.symbol("10031000161211000000").render()?;
assert_eq!(symbol.metadata().echelon, Some(Echelon::BattalionSquadron));
assert!(symbol.sidc_validity().is_valid());
# Ok::<(), Box<dyn std::error::Error>>(())
```

[`Symbol::validity`] lists every [`ValidityIssue`] as milsymbol.js `isValid()`
judges it, quirks included; [`Symbol::sidc_validity`] judges the code alone.
[`SymbolBuilder::strict`] makes rendering fail instead of drawing a question
mark.

# Drawing

[`Symbol::drawing`] is the picture as data for renderers other than SVG:
a flat list of [`drawing::DrawItem`]s (paths, circles and text), each with
its full transform, typed paint, dash lengths, clip regions and text
attributes. A GPU, canvas or scene adapter reads it without parsing SVG or
resolving inherited styles.

```
use milsymbol::Renderer;
use milsymbol::drawing::Shape;
use milsymbol::ir::Segment;

let symbol = Renderer::default().symbol("10031000001211000000").render()?;
let drawing = symbol.drawing();
let lines: usize = drawing
    .items
    .iter()
    .map(|item| match &item.shape {
        Shape::Path(segments) => segments.iter().filter(|s| matches!(s, Segment::LineTo(_))).count(),
        _ => 0,
    })
    .sum();
assert!(lines > 0);
# Ok::<(), milsymbol::RenderError>(())
```

Coordinates are symbol units: the frame's reference box spans 0–200 on both
axes around (100, 100). [`drawing::Drawing::view_box`], [`Symbol::bounding_box`]
and [`Symbol::size`] give the layout.

[`Symbol::instructions`] is the tree exactly as milsymbol.js builds it
([`ir::Node`]: nested transforms, cascading styles, values that may be
numbers or strings), which the SVG writer and the differential tests use.

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

# Caching

With the default `std` feature, `cache::CachedRenderer` memoizes rendered
symbols by SIDC and options and shares them as `Arc<Symbol>`; a hit
allocates nothing.

# `no_std`

The crate is `no_std + alloc` without the default `std` feature, which only
adds the cache. It builds for `wasm32` and bare-metal targets; the only
dependency is `libm`. No JavaScript runs at build or run time: the upstream
icon tables are generated Rust data, and every composition rule is ported.

# Compatibility

The baseline is milsymbol.js 3.0.4 (commit `b05f2d7`), each symbol rendered as
by a freshly initialised milsymbol. SVG output is byte-identical on a
288,489-case differential corpus. Upstream's direction arrows depend on the
last bit of V8's `Math.sin`/`Math.cos`, which differs between x64 and arm64;
[`RendererConfig::reference_platform`](config::RendererConfig::reference_platform)
selects which one to reproduce. [`compat`] holds milsymbol.js's own
representations (string metadata, the canonical JSON record).

The deliberate differences are documented in
[UPSTREAM.md](https://github.com/luofang34/milsymbol/blob/main/UPSTREAM.md#known-differences).
None occurs in the corpus: they cover upstream's cross-render cache
pollution, inputs on which it throws or never finishes (typed
[`RenderError`]s here), loosely typed options, extension edge cases and
SIDCs with non-BMP characters.
