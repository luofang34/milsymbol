# SVG, data URLs, and PNG

## SVG documents

`to_svg()` returns a complete SVG document. `write_svg(&mut String)` appends the
same document to an existing buffer. Clear the buffer when replacing its
contents; append is intentional.

```rust
# extern crate milsymbol;
use milsymbol::{Renderer, SvgOptions};

let symbol = Renderer::default()
    .symbol("10033000001202040000")
    .render()?;
let mut svg = String::with_capacity(4096);
symbol.write_svg(&mut svg);
assert_eq!(svg, symbol.to_svg());

svg.clear();
let output = SvgOptions::default().with_id_prefix("map-symbol-42-");
symbol.write_svg_with(&mut svg, &output);
assert!(svg.starts_with("<svg"));
# Ok::<(), milsymbol::RenderError>(())
```

Use a distinct ID prefix for each SVG embedded inline in the same document.
Some symbols use clip paths; identical IDs across inline images can make one
symbol refer to another's definition. Separate image documents do not share
that ID namespace.

For HTTP output, use `image/svg+xml` as the media type. Applications decide
where output is stored or served; rendering itself performs no I/O.

## Data URLs

`to_data_url()` returns `data:image/svg+xml;utf8,` followed by the SVG encoded
with ECMAScript `encodeURIComponent` rules. It percent-encodes UTF-8 bytes; it
does not use Base64. This matches upstream `toDataURL()`.

```rust
# extern crate milsymbol;
use milsymbol::Renderer;

let symbol = Renderer::default().symbol("10031000001211000000").render()?;
let url = symbol.to_data_url();
assert!(url.starts_with("data:image/svg+xml;utf8,%3Csvg"));
let mut buffer = String::with_capacity(url.len());
symbol.write_data_url(&mut buffer);
assert_eq!(buffer, url);
# Ok::<(), milsymbol::RenderError>(())
```

`write_data_url_with` accepts the same output settings as `write_svg_with`.
Do not encode the whole data URL a second time before using it as an image
source. A web application's content security policy must allow its chosen image
source scheme; a regular SVG resource is another option.

## PNG rasterization

PNG conversion belongs in the consuming application. The repository example
uses the native Rust `resvg` stack:

```sh
cargo run --release --example png -- infantry.png 10031000001211000000
```

The example renders an outlined symbol, parses its SVG, rasterizes it with
`tiny-skia`, and writes PNG bytes. These are development dependencies, not
dependencies of the public rendering core. The example's build disables text
support: it is appropriate for geometric symbols. For labels or text-bearing
icons, enable the rasterizer's text/font features and provide fonts explicitly.

For a production adapter, derive the raster dimensions from the rendered
symbol, account for pixel density, and preserve transparency where needed.
Reject zero or impractically large output sizes at the application boundary.
See [placement](placement.md) to keep the raster's anchor aligned with its
location.

## Escaping and trusted extensions

Built-in SVG text and attributes follow the upstream escaping behaviour.
An extension's raw SVG escape hatch is explicitly trusted content: never put
untrusted input into it. Prefer typed `Node` constructors for extension geometry.
The library does not fetch external resources to resolve fonts or images.
