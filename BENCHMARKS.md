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
| `single/compose_infantry` | SIDC → composed `Symbol` (no SVG), friendly infantry | 2.16 µs | — |
| `single/compose_and_svg_infantry` | compose + `to_svg()` | 2.72 µs | 4.61 µs |
| `single/compose_and_svg_with_fields` | HQ battalion + 2 text fields + direction arrow | 6.55 µs | — |
| `single/compose_and_svg_letter` | letter SIDC `SFGPUCI----D` | 4.10 µs | — |
| `repeated_cached_render` | `CachedRenderer` hit for the same SIDC/options | 0.35 µs | — |
| `bulk/all_number_icons_1431` | compose + SVG for every numeric main icon (1,431 SIDCs, 20 symbol sets) | 4.97 ms | 9.17 ms |

Bulk throughput is ≈ 288,000 symbols/s on one core.

## Notes

- Rendering is allocation-heavy by design: the IR owns its nodes so callers
  can inspect and transform them. The largest remaining cost in the bulk case
  is attribute escaping of path data during SVG serialization.
- The icon tables are static data; there is no per-process warm-up and no
  cache to invalidate. `CachedRenderer` (std only) additionally memoizes whole
  symbols keyed by SIDC and options.
- Numbers are indicative for this machine only; rerun `cargo bench` to
  compare changes.
