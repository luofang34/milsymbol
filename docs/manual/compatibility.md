# Compatibility and troubleshooting

The compatibility baseline is upstream milsymbol.js 3.0.4 at commit `b05f2d7`.
The pinned oracle, generated-data provenance, licences, and deliberate
differences are described in the repository's
[UPSTREAM.md](https://github.com/luofang34/milsymbol/blob/main/UPSTREAM.md).

The checked-in differential corpus covers 295,753 base-symbol, modifier, layout,
direction, option, configuration, invalid-input, and generated cases. Exact SVG
and semantic comparisons provide evidence for that coverage; they do not prove
every later revision of every military standard is implemented.

## Code edition, display standard, and compatibility

| Setting | Meaning |
|---|---|
| `RenderMode::Standards` | Default; pinned standards supply current numeric display rules |
| `RenderMode::Milsymbol304` | Exact upstream SVG and semantic behavior |
| `Standard::Mil2525` | MIL-STD-2525E Change 1 display |
| `Standard::App6` / `App6EVersion2` | APP-6(E) Version 2 display |

SIDC version digits select code meanings independently: `10`–`12` use upstream
edition D tables, `13`–`14` use its edition E tables, `15` uses MIL-STD-2525E
Change 1, and `16` uses APP-6(E) Version 2. Letter SIDCs retain upstream's
MIL-STD-2525B/C and APP-6B meanings. In standards mode all edition E versions
accept contexts `3`–`8`, symbol set `64`, and uppercase hexadecimal codes.

```rust
# extern crate milsymbol;
use milsymbol::{Renderer, RenderMode, Standard, sidc::Sidc};

let code = "16031000001211000000";
assert!(Sidc::parse(code).is_ok());
let renderer = Renderer::builder().standard(Standard::App6).build();
assert!(renderer.symbol(code).strict().render().is_ok());
let upstream = Renderer::builder().mode(RenderMode::Milsymbol304).build();
assert!(upstream.check_sidc(code).is_err());
```

`catalog::standards` supplies names, source pages, scopes and variable graphic
inputs. Hierarchy headings have no graphic. Single-point symbols are supported;
geographic lines and areas needing multiple control points are explicitly
classified as multipoint and are not rendered. APP-6's US-only sets delegate
to the pinned MIL code tables. Strict validation distinguishes unavailable
content from malformed syntax; lenient rendering draws the understood frame
and an unknown marker for unavailable icon content.

The checked-in standards deviation table links each changed output category
to a source rule. It is applied by the differential tests; it does not relax
the byte-for-byte oracle checks in compatibility mode.

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
