# Benchmarks

## Methodology

- Harness: [criterion](https://crates.io/crates/criterion) 0.8, `benches/render.rs`,
  run with `cargo bench --bench render -- --warm-up-time 0.5
  --measurement-time 1 --sample-size 20` (release profile, default options,
  size 100; the bulk group uses 10 samples).
- Machine: Apple M3 Max, macOS; rustc 1.98.1. Single run, other load minimal.
- Reported value: criterion's central time estimate.
- JavaScript comparison: milsymbol.js 3.0.4 (pinned oracle checkout) on Node
  26.5.0, same SIDCs, `new ms.Symbol(sidc).asSVG()` in a loop after warm-up.
  Upstream's global icon cache stays warm across symbols, which is its normal
  operating mode (the oracle's per-symbol cache reset is *not* applied here).

| Benchmark | What it measures | Rust | milsymbol.js |
|---|---|---:|---:|
| `single/compose_infantry` | SIDC → composed `Symbol` (no SVG), friendly infantry | 1.20 µs | — |
| `single/compose_and_svg_infantry` | compose + `to_svg()` | 1.62 µs | 4.61 µs |
| `single/compose_and_svg_with_fields` | HQ company (`10031002151211000000`) + 2 text fields + direction arrow | 4.52 µs | — |
| `single/compose_and_svg_letter` | letter SIDC `SFGPUCI----D` | 3.18 µs | — |
| `repeated_cached_render` | `CachedRenderer` hit (returns the cached `Symbol`, no SVG) | 0.16 µs | — |
| `bulk/all_number_icons_1431` | compose + SVG for every numeric main icon (1,431 SIDCs, 20 symbol sets) | 3.16 ms | 9.17 ms |

Bulk throughput is ≈ 450,000 symbols/s on one core. A cache hit returns
an already composed symbol; it is not comparable with SVG output times.

## Memory

`cargo run --release --example footprint` measures heap use with the `dhat`
heap profiler, one profiling session per operation:

| Operation | Peak heap | Allocated | Blocks | SVG size |
|---|---:|---:|---:|---:|
| compose: infantry | 3,502 B | 3,944 B | 13 | |
| `to_svg`: infantry | 1,024 B | 1,024 B | 1 | 343 B |
| compose: HQ battalion (`10031002161211000000`) + text + direction | 7,876 B | 14,992 B | 47 | |
| `to_svg`: HQ battalion + text + direction | 2,048 B | 3,072 B | 2 | 1,051 B |
| compose: letter SIDC | 4,242 B | 6,618 B | 14 | |
| `to_svg`: letter SIDC | 1,024 B | 1,024 B | 1 | 595 B |
| compose: HQ + text + direction + outline + stack 3 | 26,515 B | 73,381 B | 88 | |
| `to_svg`: same | 4,096 B | 7,168 B | 3 | 3,361 B |
| `is_valid()`: infantry | 0 B | 0 B | 0 | |
| `write_svg` into a reused `String` | 0 B | 0 B | 0 | |
| `CachedRenderer` hit | 0 B | 0 B | 0 | |
| native and JS metadata views | 0 B | 0 B | 0 | |
| canonical JSON: infantry, owned tree + string | 17,189 B | 22,181 B | 298 | |
| canonical JSON: infantry, streaming + new string | 4,096 B | 4,096 B | 1 | |
| canonical JSON: reused buffer, all four symbols above | 0 B | 0 B | 0 | |

`to_svg` allocates only the growing output string; `write_svg` appends to a
caller's buffer. Composition allocates mainly for the IR nodes the symbol
owns. Canonical streaming buffers only the current object's borrowed fields;
large extension option maps can spill this scratch buffer to the heap. The
zero-allocation figures use default fields plus the text shown above, with
output capacity reserved before profiling. `tests/allocations.rs` guards
buffer reuse, borrowed metadata/paint access and prepared cache hits.

`size_of::<Symbol>()` is 2,616 B and `size_of::<ir::Node>()` 384 B (both
excluding their heap data).

Static footprint: a minimal `wasm32-unknown-unknown` module that renders a
SIDC to SVG (`opt-level = "z"`, LTO, stripped) is 2.48 MB, 362 KB
gzip-compressed. Nearly all of it is the icon tables (15,130 template nodes
and their path data). A bare-metal build carries the same tables in flash;
this has not been measured on a real target, so check the size against your
device's budget.

## Notes

- Composition allocates by design: the IR owns its nodes so callers can
  inspect and transform them. Replacing `Vec<Node>` with inline small
  vectors is not worthwhile: a `Node` is 384 B, so inline capacity costs
  kilobytes of stack per level, and recursive children cannot be stored
  inline. An arena could reduce child-list allocations, but cannot shrink
  the largest leaf variant by itself. To avoid allocating both
  representations, composition would have to write directly into it, and
  extension constructors and mutable child access would need an arena
  context or node handles. The public IR keeps owned trees; an arena needs
  its own measurement of composition, traversal and outline generation
  before that API cost is justified.
- The icon tables are static data; there is no per-process warm-up and no
  cache to invalidate. `CachedRenderer` (std only) additionally memoizes whole
  symbols keyed by SIDC and options.
- Numbers are indicative for this machine only; rerun `cargo bench` and the
  `footprint` example to compare changes.
