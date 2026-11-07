# Compatibility and troubleshooting

The compatibility baseline is upstream milsymbol.js 3.0.4 at commit `b05f2d7`.
The pinned oracle, generated-data provenance, licences, and deliberate
differences are described in the repository's
[UPSTREAM.md](https://github.com/luofang34/milsymbol/blob/main/UPSTREAM.md).

The checked-in differential corpus covers 295,753 base-symbol, modifier, layout,
direction, option, configuration, invalid-input, and generated cases. Exact SVG
and semantic comparisons provide evidence for that coverage; they do not prove
every later revision of every military standard is implemented.

## Choose a standard deliberately

| Mode | Intended interpretation |
|---|---|
| `Standard::Mil2525` | Default upstream MIL-STD-2525 behaviour |
| `Standard::App6` | Upstream APP-6 behaviour |
| `Standard::App6EVersion2` | Opt-in, partial interpretation of version `16` numeric SIDCs |

The baseline numeric parser accepts versions `10`–`12` as edition D and
`13`–`14` as edition E, in 20- or 30-character form. Legacy letter forms cover
upstream's MIL-STD-2525B/C and APP-6B behaviour. Choosing `App6` does not imply
that all later APP-6 revisions are implemented.

`Sidc::parse_for(code, Standard::App6EVersion2)` enables the supported version
`16` syntax. Rendering/checking must use the same mode. Version 2 codes that were
removed or reassigned are filtered so the library does not silently draw their
earlier meanings. Newly defined icons are not automatically available. Restricted
target/no-strike contexts `3`–`8` and symbol set `64` remain unsupported. Version
`15` is not added by the version 2 mode. This is partial support, not a complete
APP-6(E) Version 2 conformance claim.

```rust
# extern crate milsymbol;
use milsymbol::{Renderer, Standard, sidc::Sidc};

let code = "16031000001211000000";
assert!(Sidc::parse(code).is_err());
assert!(Sidc::parse_for(code, Standard::App6EVersion2).is_ok());
let renderer = Renderer::builder().standard(Standard::App6EVersion2).build();
assert!(renderer.symbol(code).strict().render().is_ok());
```

The default lenient renderer can draw some malformed or later-version strings
using upstream fallback behaviour. Use strict validation when revision correctness
is a requirement. A standard-specific fixture or snapshot with passing tests can
record unsupported cases; always inspect its verdict, not just the test exit code.

## Common problems

| Symptom | Check or remedy |
|---|---|
| A question mark appears | Inspect `sidc_validity().issues`; check entity and modifier availability under the selected standard |
| Rendering succeeded but input was invalid | Lenient rendering is intentional; use `.strict()` or `check_sidc` |
| HQ marker sits above or below its map point | Subtract `anchor()`, which refers to the staff foot; do not centre the whole image |
| Text is missing in PNG | Load fonts and enable the rasterizer's text features |
| Font option changes labels but not icon text | Use `.font(...)` or enable `icon_text_uses_font_family` |
| Inline SVG clips incorrectly | Use unique `SvgOptions` ID prefixes within one document |
| Output differs only in a direction coordinate's last digit | Match `ReferencePlatform` to the upstream engine architecture |
| `UnknownColorMode` | Register that name or select a built-in mode |
| `InvalidOption` | Inspect its `name` and `reason`; reject NaN/infinity before rendering |
| A custom field is invisible | Register a matching label placement; the field name alone creates no layout |
| A custom shape is clipped | Supply its correct bounding box through the extension API |
| Flat drawing omits an extension | Check `omitted_raw_svg` and `invalid_paths`; prefer typed nodes |
| Memory grows despite bounded cache capacity | Capacity counts entries; external `Arc`s and large options retain additional memory |

## Error handling and upstream validity

`SidcError` describes malformed SIDCs. `SidcCheckError` separates malformed input,
unsupported content, and rendering errors. `RenderError` additionally covers
invalid options, unknown palettes, upstream-exception cases, and extension
failures. Match these non-exhaustive enums with a fallback arm and propagate
their source errors where useful.

`Symbol::sidc_validity` judges the SIDC itself. `compat::validity` and
`compat::is_valid` reproduce upstream validity quirks, including the rule that
text containing `null` makes a symbol invalid. Choose the native validity view
for input acceptance unless reproducing upstream's exact verdict is your goal.

The Rust renderer deliberately avoids upstream's mutable cross-render cache
pollution and returns typed errors for inputs on which upstream throws or cannot
finish. Fonts and rasterizers can still produce visual differences even when
SVG documents are identical. Compare instructions and SVG before using pixels
to diagnose a rendering mismatch.
