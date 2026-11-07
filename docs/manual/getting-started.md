# Installation and first render

The manual describes the repository API. Add the Git dependency to a Rust project:

```toml
[dependencies]
milsymbol = { git = "https://github.com/luofang34/milsymbol", branch = "main" }
```

The library requires Rust 1.85 or later. Development tools and dev-dependencies
can require a newer toolchain. To develop against a local checkout instead:

```toml
[dependencies]
milsymbol = { path = "../milsymbol" }
```

For reproducible Git dependencies, specify `git` and an exact `rev` from the
repository in place of a version requirement. Commit the application's
`Cargo.lock`.

## Render an infantry symbol

```rust
# extern crate milsymbol;
use milsymbol::{Renderer, options::TextField};

let renderer = Renderer::default();
let symbol = renderer
    .symbol("10031000141211000000")
    .size(40.0)
    .text(TextField::UniqueDesignation, "A/1-66")
    .render()?;

assert!(symbol.sidc_validity().is_valid());
let svg = symbol.to_svg();
assert!(svg.starts_with("<svg"));
assert!(symbol.size().width > 0.0);
# Ok::<(), milsymbol::RenderError>(())
```

The SIDC identifies friendly land infantry with a platoon echelon. `size` controls
the reference frame scale; it is not the final image width. Labels and other
indicators can extend beyond the frame.

To write SVG from a command-line program, propagate both rendering and file
errors:

```rust,no_run
# extern crate milsymbol;
use milsymbol::Renderer;
use std::path::Path;

fn write_symbol_blocking(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let symbol = Renderer::default()
        .symbol("10031000001211000000")
        .strict()
        .render()?;
    std::fs::write(path, symbol.to_svg())?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    write_symbol_blocking(Path::new("infantry.svg"))
}
```

Keep one renderer for repeated work. Building a renderer initializes its
configuration and registry; cloning it shares that state. Each `Symbol` is
independent and immutable.

## Feature choices

| Feature | Default | Purpose |
|---|---|---|
| `std` | Enabled | Adds the bounded `CachedRenderer`; core rendering still uses `alloc` |
| `serde` | Disabled | Serializes options, configuration, metadata, and drawing data; deserializes supported configuration types |
| `compact-paths` | Disabled | Packs built-in path data, trading serialization CPU for static storage |

```toml
[dependencies]
milsymbol = { git = "https://github.com/luofang34/milsymbol", branch = "main", features = ["serde", "compact-paths"] }
```

For an allocator-backed embedded application, disable default features; see
[platforms](platforms.md). Rendering itself does not require filesystem access,
an event loop, or async runtime.
