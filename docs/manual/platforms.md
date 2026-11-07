# WASM and embedded targets

The core is `no_std + alloc`. The default `std` feature adds the cache; it does
not make rendering depend on JavaScript, a browser, a filesystem, or a network.
Normal library builds and tests use checked-in Rust tables and fixtures.

## WebAssembly

```sh
rustup target add wasm32-unknown-unknown
cargo build --lib --target wasm32-unknown-unknown
cargo build --lib --no-default-features --target wasm32-unknown-unknown
```

These commands check the library target. A complete application must still
choose exports, memory ownership, allocation, and host integration. A host can
consume SVG bytes or flattened drawing data; no JavaScript is imported to
implement symbol generation. Bindings used by a consuming application are
separate from the rendering core.

Use a consistent `ReferencePlatform` if comparing generated coordinates with a
JavaScript oracle. The default is `X64`, independent of the actual WASM host.

## Bare metal

```toml
[dependencies]
milsymbol = { git = "https://github.com/luofang34/milsymbol", branch = "main", default-features = false }
```

```sh
rustup target add thumbv7em-none-eabihf thumbv6m-none-eabi
cargo build --lib --no-default-features --target thumbv7em-none-eabihf
cargo build --lib --no-default-features --target thumbv6m-none-eabi
```

An embedded binary supplies its allocator, panic handler, startup code, and
linker configuration. `no_std` does not mean no allocation: complex symbols,
configuration, text, and output buffers need memory. Measure stack and heap
against the actual device budget. Inline instructions reduce heap activity but
also make each `Symbol` a larger value.

On targets with pointer-width atomics, renderer clones use shared atomic
ownership. Targets such as `thumbv6m` use non-atomic shared ownership instead,
and `Renderer` is not `Send + Sync` there. Do not assume host thread properties
apply to every embedded target.

## Compact path storage

```toml
[dependencies]
milsymbol = { git = "https://github.com/luofang34/milsymbol", branch = "main", default-features = false, features = ["compact-paths"] }
```

`compact-paths` packs the built-in path text and decodes it while writing or
parsing. It preserves SVG output and public API signatures, without adding a
per-path heap cache. The benefit is static storage; serialization can take longer.
Compression already applied to a distribution artifact can reduce the additional
saving. Measure your linked binary or WASM module rather than the source size.

Use `PathData::write_source` to serialize path text without allocating an owned
string. `source()` may allocate for packed data. The catalogue still enumerates
native built-in keys; enabling this feature does not replace composition with
pre-rendered image lookup.
