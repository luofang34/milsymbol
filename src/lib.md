Native Rust port of [milsymbol.js](https://github.com/spatialillusions/milsymbol)
3.0.4: military symbols per MIL-STD-2525 (B/C/D/E) and APP-6 (B/D/E), as SVG
or as a typed drawing tree for any other renderer.

```
use milsymbol::{Renderer, options::field};

let renderer = Renderer::default();
let symbol = renderer
    .symbol("10031000001211000000")
    .text(field::UNIQUE_DESIGNATION, "1-66")
    .render()?;
let svg = symbol.to_svg();
assert!(svg.starts_with("<svg"));
assert!(symbol.is_valid());
# Ok::<(), milsymbol::RenderError>(())
```

# Supported standards

| SIDC | Standards |
|---|---|
| Numeric (20 or 30 digits) | MIL-STD-2525D, 2525E; APP-6D, APP-6E (versions `10`–`12` → D, `13`–`14` → E) |
| Letter (10–15 characters) | MIL-STD-2525B (incl. change 2), 2525C; APP-6B |

Whether 2525 or APP-6 rules apply is a renderer setting
([`Renderer::with_standard`]) that a symbol can override
(`style.standard`).

# Rendering

A [`Renderer`] holds all configuration and extensions. Build a symbol with
[`Renderer::symbol`] and the [`SymbolBuilder`] it returns, or pass complete
[`options::SymbolOptions`] to [`Renderer::render`]. Options are typed fields;
[`options::SymbolOptions::set`] also accepts milsymbol.js option names and
rejects unknown names or wrongly typed values.

```
use milsymbol::Renderer;
use milsymbol::options::{SymbolOptions, field};

let mut options = SymbolOptions::default();
options.style.size = 40.0;
options.set("infoColor", "red")?;
options.set_text(field::HIGHER_FORMATION, "2 BDE");
options.direction = Some(45.0);
let symbol = Renderer::default().render("10031000161211000000", options)?;
let size = symbol.size(); // pixels
let anchor = symbol.anchor(); // where the map position goes, in pixels
assert!(size.width > 0.0 && anchor.y > 0.0);
# Ok::<(), Box<dyn std::error::Error>>(())
```

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
assert!(symbol.is_sidc_valid());
# Ok::<(), Box<dyn std::error::Error>>(())
```

[`Symbol::is_valid`] is milsymbol.js `isValid()` exactly, including its
quirks; [`Symbol::is_sidc_valid`] judges the code alone, and
[`Symbol::validity`] lists every [`ValidityIssue`].

# Drawing instructions

[`Symbol::instructions`] is a tree of [`ir::Node`]s (paths, circles, text,
and translated, rotated or scaled groups) with explicit paints, stroke
widths, dash arrays, opacity and fonts. Path geometry is available as
absolute [`ir::Segment`]s, so a canvas or GPU backend never parses SVG:

```
use milsymbol::Renderer;
use milsymbol::ir::{Node, Segment};

fn count_lines(nodes: &[Node]) -> usize {
    nodes
        .iter()
        .map(|node| match node {
            Node::Path(p) => p
                .d
                .segments()
                .map(|s| s.iter().filter(|s| matches!(s, Segment::LineTo(_))).count())
                .unwrap_or(0),
            other => count_lines(other.children().unwrap_or(&[])),
        })
        .sum()
}

let symbol = Renderer::default().symbol("10031000001211000000").render()?;
assert!(count_lines(symbol.instructions()) > 0);
# Ok::<(), milsymbol::RenderError>(())
```

Coordinates are symbol units: the frame's reference box spans 0–200 on both
axes around (100, 100). [`Symbol::bounding_box`] and [`Symbol::size`] give
the layout. [`Symbol::cache_path_segments`] parses every path once for
backends that read them repeatedly.

# Extensions

What milsymbol.js configures globally (`ms.addSymbolPart`, `ms.addIcons`,
`ms.setColorMode`, …) is renderer state here, so differently configured
renderers coexist:

- [`SymbolPart`] adds a drawing stage ([`Renderer::with_symbol_part`]).
- [`IconExtension`] adds or replaces icon parts, icons for new SIDCs and
  label placements ([`Renderer::with_icons`]). It is queried by key, and can
  compose new icons from the built-in parts through [`PartLookup`].
- [`Renderer::with_color_mode`], [`Renderer::with_dash_arrays`] and
  [`Renderer::with_hq_staff_length`] replace the other globals.

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
