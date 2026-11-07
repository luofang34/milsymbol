# Drawing adapters

`Symbol::drawing()` produces a flat `Drawing` for a native canvas, GPU, scene,
or other graphics backend. Each item has its shape, resolved appearance, full
transform, and clip regions. No SVG parsing or inherited-style resolution is
required in the adapter.

```rust
# extern crate milsymbol;
use milsymbol::{Renderer, drawing::Shape};

let symbol = Renderer::default().symbol("10031000001211000000").render()?;
let drawing = symbol.drawing();
let viewport = drawing.viewport_transform();
let mut paths = 0usize;
for item in &drawing.items {
    let pixels = viewport.then(item.transform);
    match &item.shape {
        Shape::Path(segments) => {
            paths += usize::from(!segments.is_empty());
            let _geometry_and_style = (segments, pixels, &item.appearance);
        }
        Shape::Circle { center, radius } => {
            let _circle = (center, radius, pixels, &item.appearance);
        }
        Shape::Text(text) => {
            let _text = (text, pixels, &item.appearance);
        }
        _ => {}
    }
}
assert!(paths > 0);
assert_eq!(drawing.invalid_paths, 0);
# Ok::<(), milsymbol::RenderError>(())
```

`viewport.then(item.transform)` applies the item's transform followed by the
viewport mapping. Clip paths are already in symbol units: apply only the
viewport to them. Intersect all clip regions in their supplied order. Applying
the item transform to a clip a second time moves it incorrectly.

Draw items in order. Honour fill, fill opacity, stroke, width, dash lengths,
caps/joins, and text attributes. Handle all supported `ir::Segment` variants;
curves and arcs cannot be replaced by straight lines without changing the
picture. Circles can become ellipses under nonuniform transforms.

Text items retain strings, family, size, weight, anchor, and baseline information.
The backend must load fonts and shape text; the drawing view does not turn text
into glyph outlines. Preserve the transform when drawing both fill and stroke.

## Diagnostics and fallbacks

Inspect `invalid_paths` and `omitted_raw_svg` before declaring an adapter result complete.
Invalid extension path data and trusted raw-SVG nodes cannot always be represented
as ordinary drawing items. Built-in symbols should not depend on such fallbacks.
The drawing API resolves supported CSS colours into typed paints; unknown paint
syntax follows its documented SVG-compatible inheritance behaviour.

`Drawing::viewport_transform()` follows SVG's default centred aspect-ratio fit.
Degenerate or non-finite view boxes return the identity transform, and an empty
viewport can yield zero scale. Applications should reject unusable output sizes
before raster allocation.

## The instruction tree

`Symbol::instructions()` exposes the lower-level `ir::Node` tree used by the SVG
writer and extensions: paths, circles, text, groups, and transforms with cascading
attributes. Use it when writing extensions or matching upstream instructions.
Use the flat drawing view for a backend that wants resolved data.

`ir::Paint` stores upstream colour strings; `drawing::Paint` is the resolved
paint type. Likewise, `ir::Style` describes node presentation, while
`options::Style` configures a whole symbol. These types serve different stages.

For `PathData`, use `segments()` when consuming geometry and `write_source()`
when writing path text. `source()` returns `Cow<str>` and may allocate with
`compact-paths`; call `.into_owned()` only when an owned `String` is needed.
