# milsymbol

A native Rust port of [milsymbol.js](https://github.com/spatialillusions/milsymbol)
3.0.4: military unit symbols per **MIL-STD-2525** and **NATO APP-6**, as SVG
or as a typed drawing IR.

- No JavaScript at build or run time; `no_std + alloc` core, builds for
  `wasm32` and bare-metal targets. Runtime dependency: `libm`.
- Output is byte-identical to milsymbol.js 3.0.4 on a 247,918-case
  differential corpus (see [UPSTREAM.md](UPSTREAM.md)).
- Explicit renderer objects instead of global state; safe to share across
  threads; deterministic.

```rust
use milsymbol::{Renderer, options::field};

let renderer = Renderer::default();
let symbol = renderer
    .symbol("10031000161211000000") // friendly infantry battalion
    .text(field::UNIQUE_DESIGNATION, "1-66")
    .size(60.0)
    .render()?;

let svg = symbol.to_svg();                 // SVG document string
let nodes = symbol.instructions();         // typed draw instructions
let anchor = symbol.anchor();              // pixel offset of the map position
let valid = symbol.is_valid();
# Ok::<(), milsymbol::RenderError>(())
```

## Supported standards

Everything upstream 3.0.4 implements:

| SIDC form | Standards |
|---|---|
| Numeric (20–30 digits) | MIL-STD-2525D, 2525E; APP-6D, APP-6E (edition from the version digits, `10`–`12` → D, `13`–`14` → E) |
| Letter (15 characters) | MIL-STD-2525B (incl. change 2), 2525C; APP-6B |

MIL-STD-2525 vs APP-6 rendering is chosen by `RendererConfig::standard`
(upstream `ms.setStandard`) or per symbol with the `standard` style option.
FM 1-02.2 behaviour is covered to the extent upstream encodes it in the
2525 tables.

## SIDC API

`Renderer::symbol(sidc)` accepts either form. Parsing never fails: like
upstream, unknown codes render (often with the "?" icon) and are reported by
`Symbol::is_valid()` / `Symbol::validity()`. `Symbol::metadata()` exposes the
interpreted fields (affiliation, dimension, context, status, echelon,
mobility, HQ/task force/feint-dummy flags, edition, …) with upstream's names.
`milsymbol::catalog` enumerates the built-in icon codes.

## Options

Options follow upstream's names. Set them typed:

```rust
use milsymbol::options::SymbolOptions;
let mut o = SymbolOptions::default();
o.style.size = 40.0;
o.style.mono_color = "black".into();
o.direction = Some(90.0);
```

or by upstream name, which rejects unsupported values with a typed error:

```rust
# use milsymbol::options::SymbolOptions;
let mut o = SymbolOptions::default();
o.set("outlineWidth", 2.0)?.set("uniqueDesignation", "A")?.set("colorMode", "Dark")?;
# Ok::<(), milsymbol::options::OptionError>(())
```

All text amplifiers, direction/speed leader, stack, engagement bar, and every
style option of 3.0.4 (fill/frame/icon visibility, colour modes and
overrides, civilian colouring, fill opacity, monochrome, outline, stroke
width, padding, size, square, information-field size/colour/background/
outline, font, HQ staff length, alternate MEDAL, standard) are supported.

## Rendering API and IR

`Symbol::instructions()` returns `ir::Node`s: `Path`, `Circle`, `Text`,
`Translate`, `Rotate`, `Scale` (plus `Clip`/`TrustedSvg` for extensions and
`Group`/`Missing` for upstream's nested/undefined entries). Paints, stroke
widths, dash arrays, opacity, text anchoring and fonts are explicit fields;
`PathData::segments()` yields absolute `MoveTo`/`LineTo`/`CubicTo`/`QuadTo`/
`ArcTo`/`Close` segments for non-SVG renderers. Coordinates are in symbol
units (frame reference box 0–200); `bounding_box()`, `size()`, `anchor()` and
`octagon_anchor()` give the layout.

`Symbol::to_svg()` is the bundled serializer, reproducing upstream 3.0.4's
escaping and sanitization (attribute/text escaping, colour/dash/font/anchor
whitelists, `url(`/`javascript:`/`data:` blocking, raw-SVG blocklist).

## Extension model

Upstream's global extension points become renderer builder methods:

| milsymbol.js | Rust |
|---|---|
| `ms.addSymbolPart(fn)` | `Renderer::with_symbol_part(impl SymbolPart)` |
| `ms.addIcons({iconParts, icons, labels})` | `Renderer::with_icons(impl IconExtension)` |
| `ms.addIconParts` / `addSIDCicons` / `addLabelOverrides` | `IconExtension::{icon_parts, number_icons, letter_icons, number_labels, letter_labels}` |
| `ms.setColorMode` | `Renderer::with_color_mode` |
| `ms.setStandard`, `setDashArrays`, `setHqStaffLength` | `with_standard`, `with_dash_arrays`, `with_hq_staff_length` |
| `ms.showOctagon()` | `Renderer::with_octagon()` |

Extensions can build icons from the built-in parts by name
(`PartLookup::part("GR.IC.FF.INFANTRY")`; list them with
`catalog::icon_parts()`). See `tests/api.rs` for complete examples.

## Determinism and concurrency

Output depends only on the renderer configuration, SIDC and options.
`Renderer` is `Send + Sync` and immutable while rendering. With `std`,
`cache::CachedRenderer` memoizes rendered symbols.

## Differences from upstream

See [UPSTREAM.md § Known differences](UPSTREAM.md#known-differences). In
short: upstream's cross-render cache pollution is not reproduced, inputs on
which upstream throws return `RenderError`, and options are typed.

## Performance

See [BENCHMARKS.md](BENCHMARKS.md).

## Development

```sh
cargo test                           # unit, API, property and oracle-corpus tests (no Node)
cargo run --example gallery          # writes representative SVGs to target/gallery
tools/oracle/diff.sh options         # live diff against milsymbol.js (needs Node)
tools/codegen/regenerate.sh          # regenerate src/generated from upstream (needs Node)
```

## License

Copyright © 2026 sokoly and contributors. Licensed under the
**GNU Affero General Public License v3.0 or later** (`LICENSE`).

This is a derivative of milsymbol.js © Måns Beckman, used under the MIT
License; the upstream notice is retained in `LICENSE-MIT-milsymbol` and
applies to the upstream-derived portions. See `NOTICE`.
