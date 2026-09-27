# Benchmarks

## Methodology

- Harness: [criterion](https://crates.io/crates/criterion) 0.8, `benches/render.rs`,
  run with `cargo bench --bench render` (release profile, default options,
  size 100).
- Machine: Apple M3 Max, macOS; rustc 1.98.1. Single run, other load minimal.
- Reported value: criterion's median estimate of the mean.
- JavaScript comparison: milsymbol.js 3.0.4 (pinned oracle checkout) on Node
  26.5.0, same SIDCs, `new ms.Symbol(sidc).asSVG()` in a loop after warm-up.
  Upstream's global icon cache stays warm across symbols, which is its normal
  operating mode (the oracle's per-symbol cache reset is *not* applied here).

| Benchmark | What it measures | Rust | milsymbol.js |
|---|---|---:|---:|
| `single/compose_infantry` | SIDC → composed `Symbol` (no SVG), friendly infantry | 1.93 µs | — |
| `single/compose_and_svg_infantry` | compose + `to_svg()` | 2.49 µs | 4.61 µs |
| `single/compose_and_svg_with_fields` | HQ battalion + 2 text fields + direction arrow | 6.44 µs | — |
| `single/compose_and_svg_letter` | letter SIDC `SFGPUCI----D` | 4.21 µs | — |
| `repeated_cached_render` | `CachedRenderer` hit (returns the cached `Symbol`, no SVG) | 0.17 µs | — |
| `bulk/all_number_icons_1431` | compose + SVG for every numeric main icon (1,431 SIDCs, 20 symbol sets) | 4.51 ms | 9.17 ms |

Bulk throughput is ≈ 317,000 symbols/s on one core. A cache hit returns
an already composed symbol; it is not comparable with SVG output times.

## Memory

`cargo run --release --example footprint` measures heap use with the `dhat`
heap profiler, one profiling session per operation:

| Operation | Peak heap | Allocated | Blocks | SVG size |
|---|---:|---:|---:|---:|
| compose: infantry | 3,562 B | 5,733 B | 57 | |
| `to_svg`: infantry | 1,056 B | 1,079 B | 7 | 343 B |
| compose: HQ battalion + text + direction | 7,936 B | 17,577 B | 96 | |
| `to_svg`: HQ battalion + text + direction | 2,120 B | 4,087 B | 65 | 1,051 B |
| compose: letter SIDC | 4,675 B | 9,155 B | 56 | |
| `to_svg`: letter SIDC | 1,096 B | 1,380 B | 45 | 595 B |

`size_of::<Symbol>()` is 2,848 B and `size_of::<ir::Node>()` 384 B (both
excluding their heap data).

Static footprint: a minimal `wasm32-unknown-unknown` module that renders a
SIDC to SVG (`opt-level = "z"`, LTO, stripped) is 2.48 MB, 362 KB
gzip-compressed. Nearly all of it is the icon tables (15,130 template nodes
and their path data). A bare-metal build carries the same tables in flash;
this has not been measured on a real target, so check the size against your
device's budget.

## Notes

- Rendering is allocation-heavy by design: the IR owns its nodes so callers
  can inspect and transform them. The largest remaining cost in the bulk case
  is attribute escaping of path data during SVG serialization.
- The icon tables are static data; there is no per-process warm-up and no
  cache to invalidate. `CachedRenderer` (std only) additionally memoizes whole
  symbols keyed by SIDC and options.
- Numbers are indicative for this machine only; rerun `cargo bench` and the
  `footprint` example to compare changes.
