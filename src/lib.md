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

[`SymbolBuilder::font`] draws all text in one font family. Built-in icons
otherwise keep the font of their template, as milsymbol.js draws them, and the
option's `font_family` alone only affects the information fields.

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
assert!(symbol.validity().is_valid());
# Ok::<(), Box<dyn std::error::Error>>(())
```

Which call to use:

| You want to | Call |
| --- | --- |
| Check that text is a well-formed SIDC, without a renderer | [`Sidc::parse`](sidc::Sidc::parse) |
| Check that a SIDC is well formed and that your renderer, extensions included, can draw all of it | [`Renderer::check_sidc`] |
| Draw, but fail on a malformed or unrecognised SIDC instead of drawing `?` | [`SymbolBuilder::strict`], on the cached builder too |
| Draw anything and inspect the verdict afterwards | [`Symbol::validity`] |
| Compare with milsymbol.js `isValid()`, quirks included | [`compat::validity`] / [`compat::is_valid`] |

[`Symbol::validity`] lists every [`ValidityIssue`] for the SIDC alone: it
must be well formed and fully recognised, whether or not the icon is drawn.
`strict()` and `check_sidc` apply the same test before returning a symbol.
[`compat::validity`] reproduces upstream, including its rule that text
containing `null` makes a symbol invalid.

# Editing SIDCs

A parsed [`sidc::Sidc`] changes status, affiliation, identity and context
without string surgery, handling the differences between numeric and letter
codes. The result is always a valid SIDC whose identity, status and context
getters return what was set:

```
use milsymbol::Renderer;
use milsymbol::domain::{Affiliation, Context, Status};
use milsymbol::sidc::Sidc;

let friendly = Sidc::parse("10031000001211000000")?;
let hostile_damaged = friendly
    .with_affiliation(Affiliation::Hostile)
    .with_status(Status::Damaged);
assert_eq!(hostile_damaged.as_str(), "10061030001211000000");

// In an exercise, Hostile is written as Faker; letter codes carry the
// context in the identity letter.
let exercise = friendly.with_context(Context::Exercise)?.with_affiliation(Affiliation::Hostile);
assert_eq!(exercise.as_str(), "10161000001211000000");
let letter = Sidc::parse("SFGPUCI----D")?.with_context(Context::Exercise)?;
assert_eq!(letter.as_str(), "SDGPUCI----D");

let symbol = Renderer::default().symbol(&hostile_damaged).render()?;
assert_eq!(symbol.metadata().affiliation, Some(Affiliation::Hostile));
# Ok::<(), Box<dyn std::error::Error>>(())
```

An exercise Joker or Faker stands for a friendly force playing the suspect or
hostile role, so it is drawn with a friendly frame and a marker:
`metadata().affiliation` is `Friend` and `base_affiliation` is the role played.
[`sidc::Sidc::with_standard_identity`] sets the exact identity and fails with
[`sidc::SidcModifyError`] where the scheme has no code for it (Joker outside an
exercise, or Hostile inside one). Letter codes have no simulation context.

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

# Placing symbols on a map

Pixel values come from symbol units (the 0–200 reference box) scaled by
`size / 100`, so at the default size one unit is one pixel and every pixel
value scales linearly with [`SymbolBuilder::size`]. The image is larger than
the frame: [`Symbol::size`] covers the bounding box (including padding, text
fields, the direction arrow and a headquarters staff) plus the stroke and
outline width on every side. Do not assume the frame fills the image.

[`Symbol::anchor`] is the pixel offset, from the image's top-left, to put on
the map position: the frame centre, or the foot of a headquarters staff.
[`Symbol::octagon_anchor`] is always the frame centre, for attaching labels.
`style.square` makes the image a square centred on the anchor.

```
use milsymbol::Renderer;

let renderer = Renderer::default();
let render = |sidc: &str, size: f64| renderer.symbol(sidc).size(size).render();

// Image size: bounding box plus stroke and outline on both sides.
let infantry = render("10031000001211000000", 100.0)?;
let (stroke, outline) = (infantry.options().style.stroke_width, infantry.options().style.outline_width);
let bbox = infantry.bounding_box();
assert_eq!(infantry.size().width, bbox.width() + 2.0 * (stroke + outline));

// High-density screens: render at size * pixel_ratio and divide by the
// ratio when laying out; every pixel value scales exactly.
let dense = render("10031000001211000000", 200.0)?;
assert_eq!(dense.size().width, 2.0 * infantry.size().width);
assert_eq!(dense.anchor().x, 2.0 * infantry.anchor().x);

// Placing a headquarters symbol so its staff foot is on the map position.
let hq = render("10031002001211000000", 100.0)?;
let map = (400.0, 300.0);
let (left, top) = (map.0 - hq.anchor().x, map.1 - hq.anchor().y);
assert!(top < map.1 - hq.octagon_anchor().y); // the frame is above the staff foot
assert!(left < map.0);
# Ok::<(), milsymbol::RenderError>(())
```

[`Symbol::to_svg`] and [`Symbol::drawing`] describe the same region: the
`viewBox` starts at the bounding box corner minus the stroke and outline
width. When inlining several SVG symbols in one page, give each a distinct
[`SvgOptions`] id prefix. Font loading, texture atlases and pixel-density
caches belong in the adapter that draws the symbols.

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
allocates nothing for requests whose key fits in 1 KiB. Its builder is the same type as [`Renderer::symbol`]'s,
so it has every setter and `strict()`. A strict request is cached apart from
a lenient one, and a strict failure is never cached.

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
