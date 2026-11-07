# Benchmarks

## Methodology

The Criterion harness in `benches/render.rs` measures the default feature build
with release optimization, default options and reference size 100. These results
are medians of eight runs on an Apple M3 Max, macOS 27.0, rustc 1.99.0. Each run
uses a 0.5-second warm-up, a 1-second measurement interval and 20 samples (10 for
the bulk group):

```sh
cargo bench --bench render -- --warm-up-time 0.5 --measurement-time 1 --sample-size 20
```

Runs used prebuilt executables with no concurrent compilation. For each benchmark,
the reported value is the median of Criterion's central estimates across the
eight runs. These are machine-specific timings, not latency guarantees.

| Benchmark | What it measures | Time |
|---|---|---:|
| `single/compose_infantry` | SIDC → composed friendly infantry, no serialization | 0.451 µs |
| `single/compose_and_svg_infantry` | Composition + `to_svg()` | 0.833 µs |
| `single/compose_and_write_svg_reused_buffer_infantry` | Composition + SVG in a reused buffer | 0.813 µs |
| `single/compose_and_data_url_infantry` | Composition + percent-encoded SVG data URL | 1.684 µs |
| `single/compose_and_write_data_url_reused_buffer_infantry` | Composition + data URL in a reused buffer | 1.673 µs |
| `single/compose_and_drawing_infantry` | Composition + flat typed drawing view | 1.057 µs |
| `single/compose_and_svg_with_fields` | HQ company + two text fields + direction | 3.645 µs |
| `single/compose_and_svg_letter` | Letter SIDC `SFGPUCI----D` + SVG | 2.350 µs |
| `repeated_cached_render` | Cached symbol hit, no serialization | 0.116 µs |
| `bulk/all_number_icons_1431` | Composition + SVG for every numeric main icon | 1.917 ms |
| `bulk/all_number_icons_1431_reused_buffer` | The same with a reused SVG buffer | 1.884 ms |
| `bulk/drawing_all_number_icons_1431` | Drawing views of already composed symbols | 2.468 ms |

Bulk SVG throughput is approximately 747,000 symbols/s on one core for this
catalogue. A cache hit and a drawing view of an already composed symbol measure
different work from composition plus serialization.

## Memory

`cargo run --release --example footprint` measures heap use with `dhat`, with
one profiling session per operation. Renderer construction and caller-owned
output-buffer reservation happen outside the corresponding measured windows.

| Operation | Peak heap | Allocated | Blocks | SVG size |
|---|---:|---:|---:|---:|
| Compose: infantry | 0 B | 0 B | 0 | |
| `to_svg`: infantry | 1,024 B | 1,024 B | 1 | 343 B |
| Compose: HQ battalion + text + direction | 6,641 B | 8,323 B | 29 | |
| `to_svg`: HQ battalion + text + direction | 2,048 B | 3,072 B | 2 | 1,051 B |
| Compose: letter SIDC `SFGPUCI----D` | 1,152 B | 1,152 B | 1 | |
| `to_svg`: letter SIDC | 1,024 B | 1,024 B | 1 | 595 B |
| Compose: HQ + text + direction + outline + stack 3 | 25,280 B | 65,560 B | 71 | |
| `to_svg`: same | 4,096 B | 7,168 B | 3 | 3,361 B |
| `validity().is_valid()`: infantry | 0 B | 0 B | 0 | |
| `write_svg` into a sufficiently sized reused buffer | 0 B | 0 B | 0 | |
| `CachedRenderer` hit | 0 B | 0 B | 0 | |
| Native and JS metadata views | 0 B | 0 B | 0 | |
| Canonical JSON: infantry, owned tree + string | 17,189 B | 22,181 B | 298 | |
| Canonical JSON: infantry, streaming + new string | 4,096 B | 4,096 B | 1 | |
| Canonical JSON: reused buffer, all four symbols above | 0 B | 0 B | 0 | |

`size_of::<Symbol>()` is 2,784 B and `size_of::<ir::Node>()` is 384 B on this
target, excluding their heap data. Inline strings and small instruction lists
avoid heap allocation for simple symbols; they still occupy memory in the
`Symbol` value. Larger lists, custom colours and nested geometry can allocate.
The allocation tests also cover rendering and cloning default numeric and letter
infantry while writing SVG and data URLs into sufficiently sized buffers.

## Storage and practical tradeoffs

- The native icon tables are static data, with no per-process table cache or
  warm-up requirement. `CachedRenderer` separately memoizes complete symbols.
- `compact-paths` reduces embedded path storage and decodes it when serializing
  or parsing. It preserves output, but can increase serialization time. Measure
  both feature builds for the workload and target you deploy.
- Static storage depends on linking, optimization, stripping, and compression.
  Measure the final firmware or WASM artifact, including both code and data;
  source-file size is not a binary-size measurement.
- Large stacks and labels consume memory in proportion to their output. Cache
  capacity counts entries, not bytes, and symbols retained by application `Arc`s
  can outlive eviction.
