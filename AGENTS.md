Implement a complete, native Rust implementation of `milsymbol`.

The repository and public library are to be called **`milsymbol`**, not `milsymbol-rs`.

## Mission

Reimplement the behavior of `spatialillusions/milsymbol` in native Rust, using upstream `milsymbol` v3.0.4, commit `b05f2d7`, as the compatibility oracle for everything that upstream implements.

From 0.5.x the single source of truth for symbol content is the current published standards: MIL-STD-2525E with Change 1 and NATO APP-6(E) Version 2 (see "Standards source of truth"). Upstream remains the reference for the standard revisions without a pinned document, for presentation details the standards leave open, and, byte for byte, for an explicit milsymbol 3.0.4 mode.

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

## Standards source of truth

The authoritative documents are:

- MIL-STD-2525E with Change 1, 2 March 2025 (SIDC version `15`), Distribution A, from ASSIST/QuickSearch.
- NATO APP-6(E) Version 2, October 2025 (SIDC version `16`), promulgated 16 October 2025.

Pin both in machine-readable repository metadata (`tools/standards/pin.json`): title, edition/version, date, SIDC version code, source, page count and SHA-256 of the PDF. Do not commit the PDFs.

Rendering has two modes, chosen by explicit renderer configuration:

- **Standards** (the default): the rules below.
- **Milsymbol 3.0.4**: byte-identical to upstream for every input. The existing oracle corpus runs in this mode with no exceptions.

In standards mode, decoding and display are separate:

- **Decoding** (what a code means) follows the SIDC version digits, which name the edition the code was written against (MIL-STD-2525E w/Change 1, A.5.3.1): `13` MIL-STD-2525E, `14` APP-6(E) Version 1, `15` MIL-STD-2525E w/Change 1, `16` APP-6(E) Version 2. Versions `15` and `16` use their pinned standard's tables. Versions `13` and `14` keep upstream's edition E tables, which are what those editions mean, except where upstream contradicts a pinned standard on a point that no change record (APP-6(E) Version 2 NSCP list, the 2525E change) explains; such a fix applies to `13`/`14` as well. Codes added by a later version stay unknown under an earlier one. Contexts `3`–`8` (restricted target, no strike entity), symbol set `64` and hexadecimal characters are decoded for every edition E version. Unrecognised codes are handled as MIL-STD-2525E w/Change 1, 4.7.3 allows: draw what is understood (frame from Set A) and mark the rest unknown.
- **Display** (frames, amplifier and indicator drawing, digit 23) follows the renderer's `standard` setting, using the current text of that document: MIL-STD-2525E w/Change 1 for 2525, APP-6(E) Version 2 for APP-6. The two disagree (e.g. land equipment friend frame 1.2L against 1.0L), and version `10` is shared by 2525D and APP-6(D)(1), so the version digits cannot carry presentation. Display rules apply to every numeric SIDC; the frame shapes are the same table for edition D.

Versions `10`–`12` and letter SIDCs keep upstream code meanings; no pinned document covers them.

Where a standard is silent (line widths, text placement, colours not specified), keep upstream 3.0.4 behavior.

In standards mode, every output that differs from upstream must be traceable to a row, figure or paragraph of a pinned standard, and is listed by rule in a reviewed, checked-in table that the corpus test applies; never a tolerance.

Extract the standards' tables mechanically from the pinned PDFs with explicit developer tooling (`tools/standards/`) into checked-in, reviewable fixtures under `tests/data/standards/`, with the printed page and table of every row. Hand-entered values are allowed only for figures that carry no extractable text (e.g. frame proportions), each with its page reference. Fixtures derived from APP-6 acknowledge NATO in `NOTICE`.

Icon geometry that upstream does not provide must come from the standards' own drawings by a reproducible conversion, not from hand-drawn approximations; record the page each icon came from.

Multipoint control measures (lines and areas) stay out of scope; the fixtures mark them, and conformance tests skip them explicitly, never silently.


## Compatibility target

Implement the symbol behavior exposed by upstream `milsymbol` 3.0.4 for the standard revisions it implements.

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

When exact upstream behavior appears erroneous, retain compatibility by default and document the issue. Improvements may be offered only behind an explicit behavior/version mode. From 0.5.x the standards mode is the default and the milsymbol 3.0.4 mode is the explicit one: in standards mode the pinned standard decides, and upstream behavior that contradicts it is not retained.

Compare standards-mode output against the standards' fixtures semantically (code recognized, icon present and the one the standard names, frame proportions, required amplifiers drawn), with tolerances only where the standard states one or a figure must be measured; record each tolerance and its source.

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

For 0.5.x, additionally: every in-scope code of both pinned standards (entities, sector 1 and 2 modifiers, common modifiers, header fields) parses as valid for its version and draws the icon the standard names; restricted target and no strike entity contexts draw their amplifiers; frame proportions match the standards' frame size tables. The conformance suite reports per-code status from the fixtures; any code it still skips must be listed with a reason.

0.5.x is developed and tested on its own branch and is not released until the user explicitly authorizes publication.

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
