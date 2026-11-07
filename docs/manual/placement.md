# Map placement and scaling

Position an image by its anchor, not by half its width and height. Ordinary
symbols anchor at the frame centre; headquarters symbols anchor at the staff
foot. Text and movement indicators make the image bounds asymmetric.

```rust
# extern crate milsymbol;
use milsymbol::Renderer;

let symbol = Renderer::default()
    .symbol("10031002181211000000")
    .size(40.0)
    .render()?;
let map_point = (400.0, 300.0);
let anchor = symbol.anchor();
let image_top_left = (map_point.0 - anchor.x, map_point.1 - anchor.y);
assert_eq!(image_top_left.0 + anchor.x, map_point.0);
assert_eq!(image_top_left.1 + anchor.y, map_point.1);
# Ok::<(), milsymbol::RenderError>(())
```

Keep the image dimensions and anchor with the image in a texture atlas or marker
cache. Recompute them when changing the SIDC, text, direction, outline, padding,
square option, or size.

## Coordinate spaces

| Value | Space and purpose |
|---|---|
| `bounding_box()` | Bounds in reference symbol units |
| `drawing().view_box` | SVG view box including stroke/outline allowance |
| `size()` | Final image width and height in pixels |
| `anchor()` | Pixel offset from image top-left to geographic placement point |
| `octagon_anchor()` | Pixel offset to the frame centre, including for HQ symbols |
| `DrawItem::transform` | Item coordinates to symbol units |
| `Drawing::viewport_transform()` | Symbol units to image pixels |

Reference geometry is centred at `(100, 100)` in a nominal `0–200` frame box.
Pixel scaling is `style.size / 100`. The final image is the composed bounds,
including indicators and padding, with stroke and outline allowance. A symbol
rendered at size `40` does not necessarily produce a `40 × 40` image.

`style.square = true` produces a square image centred on the placement anchor.
It is useful for APIs that require square marker textures, but can leave
substantial empty space around a headquarters staff or long label.

## High-density screens

Render at `logical_size * pixel_ratio`. Divide image dimensions and anchors by
that ratio when positioning in logical screen coordinates:

```rust
# extern crate milsymbol;
use milsymbol::Renderer;

let renderer = Renderer::default();
let ratio = 2.0;
let normal = renderer.symbol("10031000001211000000").size(40.0).render()?;
let dense = renderer.symbol("10031000001211000000").size(40.0 * ratio).render()?;
assert_eq!(dense.size().width / ratio, normal.size().width);
assert_eq!(dense.anchor().x / ratio, normal.anchor().x);
# Ok::<(), milsymbol::RenderError>(())
```

For a map with rotation, distinguish north-referenced movement direction from
screen rotation. The library draws in its local image axes. Your map adapter
decides whether to rotate the whole symbol or convert bearings to a screen-relative
direction while keeping labels upright. It also owns projection, geographic
distance-to-pixel conversion, hit testing, and collision avoidance.
