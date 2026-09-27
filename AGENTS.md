Implement a complete, native Rust implementation of `milsymbol`.

The repository and public library are to be called **`milsymbol`**, not `milsymbol-rs`.

## Mission

Reimplement the behavior of `spatialillusions/milsymbol` in native Rust, using upstream `milsymbol` v3.0.4, commit `b05f2d7`, as the primary compatibility oracle.

The finished library must generate military symbology without requiring JavaScript at runtime, build time, or in downstream applications.

This is a real port, not a Rust wrapper around the JavaScript implementation and not merely a frozen lookup table of pre-rendered symbols.

The intended consumers include native applications, WASM applications, map renderers, avionics/HWD displays, simulation and wargame engines, headless servers, and future GPU renderers.

## Non-negotiable runtime constraints

The public Rust library must contain no embedded or external JavaScript runtime.

Do not use V8, QuickJS, Boa, Node, Deno, browser DOM execution, `eval`, or imported JavaScript through `wasm-bindgen` to implement symbol generation.

JavaScript is permitted only inside explicit development/oracle tooling such as `tools/oracle/`. No downstream consumer of the Rust crate may need Node or JavaScript.

Normal `cargo build`, `cargo test`, native execution, and `wasm32` builds must work without Node.

Aim for a `no_std + alloc` capable core where practical. Features that genuinely require `std` may be layered above it.

Do not use mutable process-global rendering state. Configuration must be explicit, deterministic, and suitable for concurrent use.

Do not introduce network access at runtime.

## Compatibility target

Implement the symbol behavior exposed by upstream `milsymbol` 3.0.4.

Support the standards and compatibility behavior implemented upstream, including:

MIL-STD-2525C, 2525D, and 2525E; APP-6B, APP-6D, and APP-6E; FM 1-02.2 behavior represented by upstream; legacy letter SIDCs and numeric SIDCs.

Port both parsing and composition logic.

The runtime implementation must handle valid symbol combinations algorithmically. A corpus of precomputed symbols may be used for testing, bootstrap work, or generated source data, but it must not be the implementation of symbol composition.

Large upstream geometry/icon tables should be mechanically converted into native Rust data rather than manually rewritten where possible.

## Required semantics

Port base geometries, frames, affiliation and dimension handling, entities/icons, echelon, mobility, headquarters indicators, task forces, installations, feint/dummy indicators, status modifiers, stack extensions, engagement bars, affiliation/dimension modifiers, direction indicators, movement/speed leaders, and the complete modifier/text-field layout implemented upstream.

Support upstream style and information-field behavior, including fill/frame/icon visibility, color modes and overrides, civilian coloring, fill opacity, icon/frame/fill colors, monochrome mode, outline color and width, stroke width, padding, sizing, square mode, information-field sizing/background/outline, font selection, HQ staff length, alternate MEDAL behavior, standard selection, and other public `SymbolOptions` behavior present in 3.0.4.

Expose native equivalents for metadata, colors, validity, bounding box, symbol anchor, octagon anchor, size, normalized options, and style.

Preserve the SVG escaping/security behavior of upstream 3.0.4, including its SVG text/XSS fixes.

Support an idiomatic Rust equivalent of upstream extensibility (`addSymbolPart`, `addIconParts`, label overrides, icon/SIDC additions) through explicit renderer/registry objects rather than mutable global state.

## Native intermediate representation

Do not make SVG strings the internal representation.

Define a typed native drawing IR corresponding to the semantic drawing instructions used by upstream. It must at minimum represent:

`Path`, `Circle`, `Text`, `Translate`, `Rotate`, and `Scale`.

The path representation must support the geometry required by upstream, including move, line, cubic Bézier, and close operations; add quadratic/arcs only where useful or required.

Represent fills, strokes, opacity, dash patterns, widths, text anchoring, font information, transforms, and bounding boxes explicitly.

If compatibility requires a raw-SVG extension escape hatch, isolate it as an explicitly trusted extension mechanism; ordinary built-in symbols must not depend on arbitrary raw SVG strings.

The first renderer must be an SVG renderer whose output can be compared deterministically against the JavaScript oracle.

Design the IR so a future adapter can feed a generic Rust `scene` library and GPU renderer without changing the symbol parser/compositor.

Do not make this initial implementation depend on the future generic `scene` repository.

## Public API

Prefer idiomatic Rust semantics over mechanically reproducing the JavaScript singleton API.

A likely shape is:

```rust
let renderer = Renderer::default();

let symbol = renderer
    .symbol("...")
    .options(options)
    .render()?;

let svg = symbol.to_svg();
let instructions = symbol.instructions();
let metadata = symbol.metadata();
let bbox = symbol.bounding_box();
let anchor = symbol.anchor();
```

Exact naming may differ if a cleaner Rust API emerges.

Parsing failures and unsupported options must be typed errors. User input must not cause panics.

Configuration that is global in JavaScript, such as standard selection, color modes, dash arrays, or HQ staff length, must instead belong to explicit configuration/renderer state.

The library should be deterministic for identical renderer configuration, SIDC, and options.

## Oracle infrastructure

Pin the authoritative oracle to upstream `spatialillusions/milsymbol` v3.0.4 / commit `b05f2d7`.

Record that pin in machine-readable repository metadata.

Implement explicit development tooling that runs the JavaScript oracle and emits canonical machine-readable results for requested SIDCs and option sets.

The oracle output should capture enough information to isolate errors, including:

SVG output; canonical draw instructions; metadata; colors; bounding box; size; anchor; octagon anchor; validity; normalized options/style where observable.

Do not use only screenshots or raster comparison when exact semantic comparison is available.

## Differential corpus

Build a broad checked-in or reproducibly generated compatibility corpus.

As a minimum target, cover the complete 109,216-base-symbol space demonstrated by `milsymbol-py`, generated independently from the pinned JavaScript oracle where possible.

Then add systematic coverage of modifiers and option combinations rather than assuming base-symbol parity proves complete compatibility.

Include representative and boundary combinations for every symbol set, affiliation, status, echelon, mobility type, headquarters/task-force/feint combinations, modifiers 1 and 2, direction indicators, text fields, color/style options, legacy SIDCs, invalid SIDCs, and empty/default fields.

Use property-based or generated testing to explore valid and invalid SIDC combinations beyond the fixed corpus.

The corpus is an oracle and regression suite, not the runtime implementation.

## Comparison requirements

Prefer comparisons in this order:

1. canonical native draw-instruction equivalence;
2. exact SVG string equivalence where serialization can be made deterministic;
3. parsed SVG structural equivalence where insignificant serialization differences exist;
4. raster/pixel comparison as a diagnostic fallback.

Do not declare a mismatch acceptable merely because the result “looks similar.”

When exact upstream behavior appears erroneous, retain compatibility by default and document the issue. Improvements may be offered only behind an explicit behavior/version mode.

## Implementation strategy

Proceed continuously through the following phases rather than stopping after scaffolding:

1. Pin upstream, build oracle tooling, and establish the corpus format.
2. Define Rust SIDC types, parser, validation, metadata, options, colors, bounding boxes, and drawing IR.
3. Mechanically convert large upstream symbol/icon/path datasets to generated or checked-in native Rust tables with provenance.
4. Port numeric and legacy letter SIDC interpretation.
5. Port base geometry and icon composition.
6. Port modifier, status, echelon, mobility, HQ/task-force/feint, stack, engagement, affiliation/dimension, and direction logic.
7. Port text fields and all relevant public styling/options.
8. Implement deterministic SVG rendering and escaping.
9. Implement extension/registry APIs.
10. Drive differential failures to parity against the oracle.
11. Validate native, `wasm32`, and `no_std + alloc` configurations where applicable.
12. Benchmark and document the resulting implementation.

Do not stop merely because the base-symbol corpus passes.

## Generated data and provenance

Generated geometry/data is acceptable and preferred over hand transcription.

The repository must retain enough provenance to answer which upstream file/revision produced each generated dataset and how to regenerate it.

Generation may depend on Node inside an explicit developer command, but the generated source/data consumed by the Rust library must not require JavaScript.

Keep upstream licensing and attribution intact. Add appropriate `LICENSE`, attribution/NOTICE documentation, and an `UPSTREAM.md` describing the compatibility baseline and generated-data provenance.

## Quality requirements

Use current stable Rust unless the repository has a documented reason otherwise.

Avoid `unsafe` unless it provides a demonstrated benefit and is isolated, documented, and tested. Prefer entirely safe Rust.

Use `cargo fmt`, strict Clippy, rustdoc, unit tests, property tests, and corpus/differential tests.

CI should include native builds/tests and at least a `wasm32` compile check.

Normal CI should be capable of running from checked-in canonical fixtures without Node. A separate oracle/regeneration job may install Node and compare against pinned upstream.

Add a CI guard that makes accidental introduction of a JavaScript runtime dependency visible.

Do not fetch external resources during ordinary library use or tests.

## Acceptance criteria

The implementation is complete only when the public runtime is fully native Rust and the primary behavior exercised by upstream 3.0.4 is reproduced.

The complete base-symbol oracle corpus must pass.

Modifier, status, text-field, styling, and representative combinatorial corpora must pass. Any remaining mismatch must be individually understood and documented; do not hide mismatches behind broad tolerances.

A native example must render representative symbols to SVG without JavaScript.

A WASM-target build must compile without imported JavaScript implementing symbol generation.

The runtime dependency tree must contain no JavaScript engine.

Library documentation must explain the SIDC API, rendering API, compatibility baseline, supported standards, extension model, deterministic behavior, and current known differences.

Benchmark representative single-symbol rendering, repeated cached rendering, and bulk rendering. Record methodology and results rather than making unsupported performance claims.

The implementation must support symbol combinations produced at runtime; it must not merely retrieve members of the regression corpus.

## Repository/package naming

The repository is `milsymbol`.

The intended Rust package name is also `milsymbol`.

Before crates.io publication, verify that the package name is available and that publication is legally and operationally appropriate.

If the exact crates.io name is unavailable, do not silently rename the project and do not compromise the implementation. Complete the repository first and report the naming conflict separately.

Do not publish a crate or release without explicit authorization.

## Scope boundary

Do not build a game engine, map renderer, or generalized scene graph as part of this task.

The native drawing IR should be clean enough to adapt to those systems later, but the immediate goal is a complete, testable, native-Rust `milsymbol` implementation with exceptionally strong compatibility evidence against the pinned JavaScript oracle.

Continue implementing and testing until these acceptance criteria are met; do not return after producing only an architecture document, scaffold, partial parser, or proof of concept.
