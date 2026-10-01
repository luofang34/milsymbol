# milsymbol

[![CI](https://github.com/luofang34/milsymbol/actions/workflows/ci.yml/badge.svg)](https://github.com/luofang34/milsymbol/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/milsymbol.svg)](https://crates.io/crates/milsymbol)
[![docs.rs](https://docs.rs/milsymbol/badge.svg)](https://docs.rs/milsymbol)

Military unit symbols per **MIL-STD-2525** and **STANAG APP-6**, in native Rust.

This is a port of [milsymbol.js](https://github.com/spatialillusions/milsymbol)
3.0.4 by Måns Beckman. It produces the same SVG as milsymbol.js, byte for byte,
verified on 288,489 symbol/option combinations, and also exposes each symbol as
typed drawing instructions for non-SVG renderers.

![Figure 13](https://github.com/luofang34/milsymbol/raw/main/docs/images/figure13.svg)

```rust
use milsymbol::Renderer;
use milsymbol::options::TextField;

let svg = Renderer::default()
    .symbol("130315003611010300000000000000")
    .size(35.0)
    .direction(750.0 * 360.0 / 6400.0)
    .text(TextField::Quantity, "200")
    .text(TextField::StaffComments, "FOR REINFORCEMENTS")
    .text(TextField::AdditionalInformation, "ADDED SUPPORT FOR JJ")
    .text(TextField::Type, "MACHINE GUN")
    .text(TextField::Dtg, "30140000ZSEP97")
    .text(TextField::Location, "0900000.0E570306.0N")
    .render()?
    .to_svg();
# Ok::<(), milsymbol::RenderError>(())
```

This is figure 13 of MIL-STD-2525C, rendered by this crate.

## Summary

- MIL-STD-2525B/C/D/E and STANAG APP-6 B/D/E, legacy letter SIDCs and
  numeric SIDCs.
- Filled/unfilled, framed/unframed, monochrome, outlined symbols; text
  amplifiers; movement indicators and speed leaders; engagement bars;
  headquarters, task force, feint/dummy, installation, echelon and mobility
  indicators; operational condition; exercise/simulation markers.
- SVG output, plus a typed drawing IR (`Path`, `Circle`, `Text`,
  `Translate`, `Rotate`, `Scale`) with path segments for GPU, canvas and
  other renderers.
- No JavaScript at build or run time. `no_std + alloc` core; builds for
  `wasm32` and bare-metal targets. The only dependency is `libm`. The
  optional `compact-paths` feature embeds path data packed, about 195 KB less
  flash, for some CPU and heap (see the crate documentation).
- No global state: a `Renderer` holds configuration and extensions, is
  `Send + Sync`, and renders deterministically.
- About 450,000 symbols per second to SVG on one core
  ([BENCHMARKS.md](BENCHMARKS.md)).

## Getting started

```toml
[dependencies]
milsymbol = "0.2"
```

Requires Rust 1.85 or later.

To make a symbol for an infantry platoon:

```rust
use milsymbol::Renderer;

let renderer = Renderer::default();
let symbol = renderer.symbol("130310001412110000000000000000").size(35.0).render()?;
let svg: String = symbol.to_svg();
# Ok::<(), milsymbol::RenderError>(())
```

![Infantry platoon](https://github.com/luofang34/milsymbol/raw/main/docs/images/infantry-platoon.svg)

`symbol` also tells you where to place it and how big it is:

```rust
# let symbol = milsymbol::Renderer::default().symbol("130310001412110000000000000000").render()?;
let anchor = symbol.anchor();   // pixel offset of the map position (frame centre or HQ staff foot)
let size = symbol.size();       // width and height in pixels
let info = symbol.metadata();   // typed: affiliation, dimension, status, echelon, mobility, …
let ok = symbol.sidc_validity().is_valid(); // false for unknown codes, which still render with a "?" icon
# Ok::<(), milsymbol::RenderError>(())
```

`symbol.sidc_validity().issues` lists why a symbol is not valid (malformed SIDC,
unknown affiliation, dimension, icon or amplifier code, …); it judges the
code alone. `compat::validity` is milsymbol.js's `isValid()`, which also
rejects any text containing `null`.

To check a SIDC without rendering it, parse it strictly. `Sidc::parse`
checks the code against the standards' tables; `Renderer::check_sidc` also
checks that the renderer, with its extensions, has everything the code
names (such as the icon):

```rust
use milsymbol::sidc::Sidc;
use milsymbol::domain::StandardIdentity;

let Sidc::Numeric(sidc) = Sidc::parse("10031000161211000000")? else { return Ok(()) };
assert_eq!(sidc.standard_identity(), StandardIdentity::Friend);
assert_eq!(sidc.amplifier(), "16"); // battalion
assert!(Sidc::parse("10091000001211000000").is_err()); // identity 9 does not exist
# Ok::<(), milsymbol::sidc::SidcError>(())
```

## Options

Options are typed values. The builder has setters for the common ones
(`size`, `direction`, `speed_leader`, `stack`, `text`, `standard`,
`color_mode`, `mono_color`) and `with` edits any other in place; a colour is
an `options::Color` and `None` means "not set":

```rust
use milsymbol::{Renderer, options::{Color, ColorChoice, TextField}};

let symbol = Renderer::default()
    .symbol("10031000001211000000")
    .size(40.0)
    .direction(60.0)
    .text(TextField::HigherFormation, "2 BDE")
    .text(TextField::UniqueDesignation, "A/1-66")
    .with(|o| {
        o.style.outline_width = 5.0;
        o.style.mono_color = Color::rgb(20, 60, 160).into();
        o.style.info_color = Color::new("red").ok().map(ColorChoice::from);
    })
    .render()?;
# Ok::<(), milsymbol::RenderError>(())
```

Options read from text (a config file, a request) can be applied by their
milsymbol.js names with `SymbolOptions::set`, which rejects unknown names and
wrongly typed values. Complete `SymbolOptions` can be built once and passed
to `SymbolBuilder::options`. NaN and infinite numbers are rejected with
`RenderError::InvalidOption`. To fail on a malformed or unrecognised SIDC
instead of drawing a `?` icon, add `.strict()` to the builder.

### Standard identities and dimensions

| | Friend | Hostile | Neutral | Unknown |
|---|:-:|:-:|:-:|:-:|
| Air | ![](https://github.com/luofang34/milsymbol/raw/main/docs/images/friend-air.svg) | ![](https://github.com/luofang34/milsymbol/raw/main/docs/images/hostile-air.svg) | ![](https://github.com/luofang34/milsymbol/raw/main/docs/images/neutral-air.svg) | ![](https://github.com/luofang34/milsymbol/raw/main/docs/images/unknown-air.svg) |
| Land | ![](https://github.com/luofang34/milsymbol/raw/main/docs/images/friend-land.svg) | ![](https://github.com/luofang34/milsymbol/raw/main/docs/images/hostile-land.svg) | ![](https://github.com/luofang34/milsymbol/raw/main/docs/images/neutral-land.svg) | ![](https://github.com/luofang34/milsymbol/raw/main/docs/images/unknown-land.svg) |
| Sea surface | ![](https://github.com/luofang34/milsymbol/raw/main/docs/images/friend-sea.svg) | ![](https://github.com/luofang34/milsymbol/raw/main/docs/images/hostile-sea.svg) | ![](https://github.com/luofang34/milsymbol/raw/main/docs/images/neutral-sea.svg) | ![](https://github.com/luofang34/milsymbol/raw/main/docs/images/unknown-sea.svg) |
| Subsurface | ![](https://github.com/luofang34/milsymbol/raw/main/docs/images/friend-subsurface.svg) | ![](https://github.com/luofang34/milsymbol/raw/main/docs/images/hostile-subsurface.svg) | ![](https://github.com/luofang34/milsymbol/raw/main/docs/images/neutral-subsurface.svg) | ![](https://github.com/luofang34/milsymbol/raw/main/docs/images/unknown-subsurface.svg) |

The fourth digit of a numeric SIDC is the standard identity (`3` friend,
`6` hostile, `4` neutral, `1` unknown); digits 5–6 are the symbol set (`01`
air, `10` land unit, `30` sea surface, `35` subsurface).

### Amplifiers and modifiers

| | | |
|:-:|:-:|:-:|
| ![](https://github.com/luofang34/milsymbol/raw/main/docs/images/hq-brigade.svg)<br>`10031002181211000000`<br>headquarters, brigade | ![](https://github.com/luofang34/milsymbol/raw/main/docs/images/task-force-company.svg)<br>`10031004151211000000`<br>task force, company | ![](https://github.com/luofang34/milsymbol/raw/main/docs/images/feint-dummy.svg)<br>`10031001151205000000`<br>feint/dummy armour company |
| ![](https://github.com/luofang34/milsymbol/raw/main/docs/images/tracked-equipment.svg)<br>`10031500331101000000`<br>tracked equipment | ![](https://github.com/luofang34/milsymbol/raw/main/docs/images/planned.svg)<br>`10031010001211000000`<br>planned | ![](https://github.com/luofang34/milsymbol/raw/main/docs/images/damaged.svg)<br>`10031030001211000000`<br>damaged |
| ![](https://github.com/luofang34/milsymbol/raw/main/docs/images/exercise-joker.svg)<br>`10151000001211000000`<br>exercise joker | ![](https://github.com/luofang34/milsymbol/raw/main/docs/images/installation.svg)<br>`10032000001101000000`<br>installation | ![](https://github.com/luofang34/milsymbol/raw/main/docs/images/speed-leader.svg)<br>`direction: 60, speedLeader: 60`<br>speed leader |
| ![](https://github.com/luofang34/milsymbol/raw/main/docs/images/engagement-bar.svg)<br>`engagementBar: "2:10"`<br>engagement bar | | |

### Styles

| | | |
|:-:|:-:|:-:|
| ![](https://github.com/luofang34/milsymbol/raw/main/docs/images/style-unfilled.svg)<br>`fill: false` | ![](https://github.com/luofang34/milsymbol/raw/main/docs/images/style-dark.svg)<br>`colorMode: "Dark"` | ![](https://github.com/luofang34/milsymbol/raw/main/docs/images/style-mono.svg)<br>`monoColor: "rgb(20,60,160)"` |
| ![](https://github.com/luofang34/milsymbol/raw/main/docs/images/style-outline.svg)<br>`outlineWidth: 5` | ![](https://github.com/luofang34/milsymbol/raw/main/docs/images/style-unframed.svg)<br>`frame: false` | ![](https://github.com/luofang34/milsymbol/raw/main/docs/images/style-info-background.svg)<br>`infoBackground` |

Supported style options: fill/frame/icon visibility, colour modes and per-affiliation overrides, civilian colouring,
fill opacity, monochrome, outline colour and width, stroke width, padding,
size, square, information-field size/colour/background/outline, font, HQ
staff length, alternate MEDAL icons and the standard override.

`fontfamily` sets the font of generated labels and other non-icon text.
Built-in icon text keeps its milsymbol.js template font (usually Arial) unless
`iconTextUsesFontFamily: true` is set. The switch is `false` by default, so
existing SVG output remains byte-identical to milsymbol.js. Set
`o.style.icon_text_uses_font_family = true` in the typed API. The switch
does not override custom text nodes supplied by icon extensions or load fonts
for an SVG consumer.

### Letter SIDCs and APP-6

| MIL-STD-2525C `SFGPUCFRM---` | APP-6B `SFGPUCFRM---` | 2525C sea mine `SHUPWMGX----` |
|:-:|:-:|:-:|
| ![](https://github.com/luofang34/milsymbol/raw/main/docs/images/letter-2525c.svg) | ![](https://github.com/luofang34/milsymbol/raw/main/docs/images/letter-app6b.svg) | ![](https://github.com/luofang34/milsymbol/raw/main/docs/images/letter-sea-mine.svg) |

```rust
use milsymbol::{Renderer, Standard};
let app6 = Renderer::builder().standard(Standard::App6).build();
let svg = app6.symbol("SFGPUCFRM---").render()?.to_svg();
# Ok::<(), milsymbol::RenderError>(())
```

| SIDC form | Standards |
|---|---|
| Numeric (20–30 digits) | MIL-STD-2525D, 2525E; APP-6D, APP-6E (version digits `10`–`12` → D, `13`–`14` → E) |
| Letter (15 characters) | MIL-STD-2525B (incl. change 2), 2525C; APP-6B |

## Drawing

`to_svg()` is one renderer. `Symbol::drawing()` gives the same picture as
data, flattened for other backends: each item carries its full transform,
typed paint, dash lengths, clip regions and text attributes, so a GPU or
canvas adapter parses no SVG and resolves no inherited styles.

```rust
use milsymbol::{Renderer, drawing::Shape, ir::Segment};

let symbol = Renderer::default().symbol("10031000001211000000").render()?;
let drawing = symbol.drawing();
for item in &drawing.items {
    let _ = (&item.appearance, item.transform, &item.clips);
    if let Shape::Path(segments) = &item.shape {
        for seg in segments {
            if let Segment::MoveTo(p) | Segment::LineTo(p) = *seg {
                let _ = (p.x, p.y);
            }
        }
    }
}
# Ok::<(), milsymbol::RenderError>(())
```

Coordinates are in symbol units (the frame's reference box is 0–200, centred
on 100,100); `drawing.view_box`, `bounding_box()`, `size()`, `anchor()` and
`octagon_anchor()` give the layout. `Symbol::instructions()` is the tree
exactly as milsymbol.js builds it (nested groups, cascading styles), which
the SVG writer and the differential tests use.

## Extending

Configuration and extensions belong to a `Renderer`, built once through
`Renderer::builder()`; a built renderer never changes and clones cheaply, so
differently configured renderers can coexist:

| To add | Use |
|---|---|
| a drawing stage (e.g. a custom amplifier) | `RendererBuilder::symbol_part(impl SymbolPart)` |
| icons, icon parts or label placements for new SIDCs | `RendererBuilder::icons(impl IconExtension)` |
| a colour mode | `RendererBuilder::color_mode` |
| a default standard, dash arrays, HQ staff length | `standard`, `dash_arrays`, `hq_staff_length` |
| the icon octagon (debugging) | `RendererBuilder::octagon()` |
| a different set of built-in stages | `RendererBuilder::pipeline(&[..])` |

Builder calls apply in order: `pipeline` replaces the stages chosen so far,
while `symbol_part` and `octagon` append.

An `IconExtension` can replace or add named icon parts (`icon_part`) and
icons for entities, modifiers or letter SIDCs (`icon`), composing them from
the built-in parts by name (`PartLookup::part("GR.IC.FF.INFANTRY")`, listed
by `catalog::icon_parts()`). `tests/extensions.rs` has complete examples of
a custom symbol part, a new SIDC icon and a label override.

## Compatibility

The baseline is milsymbol.js 3.0.4 (commit `b05f2d7`), with each symbol
rendered as by a freshly initialised milsymbol. [UPSTREAM.md](UPSTREAM.md)
describes how the icon tables were extracted from upstream, the differential
corpus, and every deliberate difference. None occurs in the corpus: they
cover upstream's cross-render cache pollution, inputs on which it throws or
never finishes, loosely typed options, extension edge cases (clip ids,
colour objects, JavaScript object keys) and SIDCs with non-BMP characters.

Upstream's own output depends on the platform in one place. Direction
arrows and speed leaders use `Math.sin`/`Math.cos`, whose last bit differs
between V8's x64 and arm64 builds. `RendererConfig::reference_platform`
selects which one to reproduce exactly (default x64, which also matches
WebAssembly).

Coming from milsymbol.js: option names are the same (`size`,
`uniqueDesignation`, `colorMode`, …, via `SymbolOptions::set`), and the
global calls map to builder methods — `ms.addSymbolPart` →
`symbol_part`, `ms.addIcons` → `icons`, `ms.setColorMode` → `color_mode`,
`ms.setStandard`/`setDashArrays`/`setHqStaffLength` →
`standard`/`dash_arrays`/`hq_staff_length`, `ms.showOctagon` → `octagon`.
`compat::is_valid` is `isValid()` (the quirks `Symbol::sidc_validity` leaves out) and `compat::js_metadata` is
`symbol.metadata`.

## Development

```sh
cargo test                           # unit, API, property and oracle-corpus tests (no Node)
tools/ci/check.sh                    # the CI native job; tools/ci/check-linux.sh runs it on Linux
tools/ci/lint.sh stable 1.85:lib     # clippy with warnings denied on each toolchain
tools/ci/lint-config.sh              # every crate carries the workspace lint table
cargo run --example readme_images    # regenerates docs/images (or pass an output dir)
tools/oracle/diff.sh options         # live diff against milsymbol.js (needs Node)
tools/codegen/regenerate.sh          # regenerate src/generated from upstream (needs Node)
```

## License

Copyright © 2026 Fang Luo and contributors. Licensed under the
**GNU Affero General Public License v3.0 or later** ([LICENSE](LICENSE)).

This crate is a derivative of milsymbol.js © Måns Beckman
(spatialillusions.com), used under the MIT License; the upstream notice is
retained in [LICENSE-MIT-milsymbol](LICENSE-MIT-milsymbol) and applies to the
upstream-derived portions. See [NOTICE](NOTICE).
