# Caching and bulk rendering

Reuse a renderer, avoid repeated conversion of unchanged symbols, and reuse
output buffers before adding a cache. `write_svg` and `write_data_url` append
into a caller-owned `String`; once capacity is sufficient, the output buffer
does not need to grow.

```rust
# extern crate milsymbol;
use milsymbol::{Renderer, options::SymbolOptions};

let renderer = Renderer::default();
let mut buffer = String::with_capacity(4096);
let mut total_bytes = 0usize;
for sidc in ["10031000001211000000", "10061000001211000000"] {
    let symbol = renderer.render(sidc, SymbolOptions::default())?;
    buffer.clear();
    symbol.write_svg(&mut buffer);
    total_bytes += buffer.len();
    // Consume or write this buffer before the next iteration clears it.
}
assert!(total_bytes > 0);
# Ok::<(), milsymbol::RenderError>(())
```

For default infantry, composition and cloning keep short strings and small
instruction lists inline and share built-in colour tables. More complex symbols,
custom colours, text, nested geometry, and large instruction lists can allocate.
`drawing()` builds an owned flat view and parses path geometry: retain it if a
backend will draw the same picture repeatedly.

## Bounded symbol caching

`CachedRenderer` is available with `std`. Its capacity counts entries, not bytes,
and must be chosen explicitly. Capacity `0` disables storage. Eviction uses a
clock algorithm approximating least-recently-used behaviour.

```rust
# extern crate milsymbol;
use milsymbol::{Renderer, cache::CachedRenderer, options::SymbolOptions};
use std::sync::Arc;

let cache = CachedRenderer::new(Renderer::default(), 256);
let options = SymbolOptions::default();
let first = cache.render("10031000001211000000", &options)?;
let second = cache.render("10031000001211000000", &options)?;
assert!(Arc::ptr_eq(&first, &second));
assert_eq!(cache.len(), 1);
# Ok::<(), milsymbol::RenderError>(())
```

The cache keys the SIDC and all options. Strict and lenient requests use separate
entries, and strict failures are not cached. Floating-point values are compared
by bit pattern, so `-0.0` and `0.0` are separate requests. A hit allocates nothing
when its encoded key fits in 1 KiB.

The return value is `Arc<Symbol>`. Evicting an entry does not invalidate an `Arc`
held by your application, so the entry limit is not a hard bound on total live
memory. Large labels and stacks can make each entry expensive. Set request
limits appropriate to the application and avoid caching every transient mouse
movement or continuously changing label.

The cache stores composed symbols, not SVG strings, PNGs, GPU meshes, or textures.
Cache those outputs at the application layer if serialization or rasterization
dominates your workload. Include output settings such as ID prefixes and pixel
density in the relevant output-cache key.

## Concurrency and measurement

On targets with pointer atomics, `Renderer` and `Symbol` can be shared between
threads. Each batch worker can reuse its own output buffer. Extensions must be
deterministic for caching to remain valid. The cache renders misses outside its
mutex; simultaneous misses may perform duplicate composition before converging
on one stored result.

Measure the operation you actually need:

```sh
cargo bench --bench render
cargo run --release --example footprint
```

The benchmarks distinguish composition, SVG, data URLs, flat drawings, cached
hits, and bulk rendering. The footprint example measures heap allocation and
reports the inline size of public objects. See the repository's `BENCHMARKS.md`
for measured methodology and results. A cached-symbol hit is not comparable to
composition plus SVG, and compact path storage is not automatically faster.
