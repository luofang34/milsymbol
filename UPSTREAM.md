# Upstream baseline and provenance

## Compatibility baseline

| | |
|---|---|
| Upstream | [spatialillusions/milsymbol](https://github.com/spatialillusions/milsymbol) |
| Version | 3.0.4 (tag `v3.0.4`) |
| Commit | `b05f2d7cbadb0845ca637d346a2d7d8163ab59cb` |
| License | MIT — see `LICENSE-MIT-milsymbol` |
| Machine-readable pin | `tools/oracle/pin.json` |

The baseline is upstream's full distribution as assembled by its `index.js`:
`ms.addIcons` of APP-6B, 2525B, 2525C, APP-6D, 2525D and 2525E, in that order,
with the default symbol parts.

**Determinism policy.** Upstream keeps a process-global icon cache, and
`ms._scale(…, true)` mutates cached icon parts in place, so a symbol's output
can depend on which symbols were rendered earlier in the same process. The
oracle clears `ms._iconCache` and `ms._labelCache` before every case. The
compatibility baseline is therefore *each symbol rendered by a freshly
initialised milsymbol*. In-place mutations that happen *within* one render
(e.g. the land-equipment `_scale` calls in the symbol-set 15 mapping, which
change the stroke of `GR.IC.ENGINEER` for every symbol-set 15 icon that uses
it) are reproduced.

## What is generated, and how

Everything under `src/generated/` is produced by `tools/codegen` (extraction,
which runs upstream JavaScript) and `cargo xtask emit` (Rust), and must not
be edited by hand. Regenerate with:

```sh
tools/codegen/regenerate.sh      # needs git and Node; fetches the pinned upstream
```

| File | Source (upstream) | Tool |
|---|---|---|
| `pool.rs`, `tables.rs`, `vars.rs` | `src/iconparts/*.js`, `src/numbersidc/sidc/*.js`, `src/lettersidc/sidc/*.js` | `extract.mjs` → `cargo xtask emit` |
| `misc.rs` (labels) | `src/lettersidc/labels/*.js`, `src/numbersidc/labels/*.js` | `extract-misc.mjs` → `cargo xtask emit` |
| `misc.rs` (geometries) | `src/ms/symbolgeometries.js` | `extract-misc.mjs` → `cargo xtask emit` |
| `misc.rs` (colour modes) | `src/colormodes.js` | `extract-misc.mjs` → `cargo xtask emit` |
| `misc.rs` (character widths) | `src/symbolfunctions/string-width.js` | `extract-misc.mjs` → `cargo xtask emit` |

### Icon tables: evaluate-and-extract

Upstream's icon parts and SIDC→icon mappings are JavaScript functions of a
small rendering context, not static data. Translating that code would have to
re-implement its object aliasing, in-place mutation, key overriding across
files and `defaultProperties` pass. `extract.mjs` instead **runs the upstream
functions** and records what they produce:

1. Colours and dash arrays are replaced by symbolic slot tokens
   (`iconColor` of the symbol's affiliation, `fillColor.Unknown`, …) so an
   entry is stored once per structural variant, not per colour.
2. The context is 15 variables: standard (2525/APP-6), frame, numeric SIDC,
   alternate MEDAL, monochrome, edition, not-present dash, affiliation, base
   geometry, and the truthiness of six colour slots.
3. For every table entry (2,108 named parts; every numeric symbol-set
   icon/modifier-1/modifier-2/bbox entry; every letter-SIDC icon/bbox entry)
   the variables it depends on are discovered by single-variable perturbation
   from 200 random baseline contexts, then the entry is evaluated for every
   combination of those variables.
4. Mapping functions are evaluated with reference tokens in place of parts,
   so mappings are stored as structure plus part references (they depend only
   on standard and edition). Parts that a mapping mutates in place are stored
   as per-mapping overrides (20 such part/mapping pairs).
5. **Round-trip check:** 800 fresh random contexts are evaluated by upstream
   with real parts, and every table lookup is compared with the fully inlined
   upstream result (≈1.8 million comparisons). Any mismatch aborts
   generation.

Composition — which entry to look up, scaling for sector modifiers, frames,
modifiers, text fields, layout, colours, bounding boxes, SVG — is hand-ported
Rust in `src/compose/`, `src/sidc/`, `src/color.rs` and `src/svg.rs`. The
tables are keyed by entity/modifier code and context, never by complete SIDC.

## Differential corpus

`tools/oracle` runs the pinned JavaScript as an oracle. `tests/corpus/*.jsonl.gz`
holds FNV-1a hashes of the oracle's SVG and of a canonical JSON record
(draw instructions, metadata, colours, bounding box, size, anchors, validity,
options) for each case; `cargo test --test corpus` replays them without Node.

| Suite | Cases | Content |
|---|---:|---|
| `base` | 131,881 | every valid numeric entity × 7 standard identities for every symbol set upstream has icons for (discovered, not listed; the generator fails if a set has no cases), frame-only SIDCs for all 100 symbol sets, every letter pattern × 6 affiliations (+ echelons). Checked to be a strict superset of milsymbol-py's 109,216 SIDCs |
| `modifiers` | 93,660 | versions, contexts, identities, status, HQ/TF/dummy, echelon/mobility codes, every modifier 1/2 code, frame shapes, letter-SIDC fields |
| `direction` | 36,000 | direction of movement and speed leader over 0–359.9° in 0.1° steps, for five SIDCs (exercises `Math.sin`/`Math.cos`) |
| `options` | 6,781 | every text field, every style option alone and in 4,000 random combinations, XSS strings, non-ASCII text and JavaScript-vs-Unicode whitespace |
| `fuzz` | 20,000 | random numeric and letter SIDCs, some with options |
| `config` | 123 | renderer-level standard, dash arrays, HQ staff length |
| `invalid` | 44 | empty/short/long/malformed SIDCs, unknown colour mode |

All 288,489 cases match byte-for-byte (SVG) and exactly (semantic record).
Regenerate fixtures with `node tools/oracle/fixtures.mjs <suite>…` (x64 Node,
see below); run a live comparison with `tools/oracle/diff.sh <suite>`.

## Platform-dependent upstream output

milsymbol.js computes direction-arrow and speed-leader coordinates with
`Math.sin`/`Math.cos`, and V8's results for these differ by platform. On
Node 26 (V8 14.6), `Math.sin`/`Math.cos` are V8's own fdlibm port. The x64
builds evaluate it in plain IEEE arithmetic, while the arm64 builds are
compiled with floating-point contraction (fused multiply-adds). Over 7,136
arguments, Node on Linux x64 and Windows x64 agree with each other, and
Node on macOS arm64 and Linux arm64 agree with each other, but the two
groups differ in the last bit of 66 of those 14,272 values. Over the `direction`
suite this changes 215 of 36,000 SVGs.

`src/js/math.rs` ports V8's fdlibm exactly, including argument reduction
(with the Sun and V8 notices; see `NOTICE` and `LICENSE-BSD-V8`). Plain
evaluation reproduces the x64 group bit for bit, and fused evaluation
reproduces the arm64 group bit for bit. Other implementations do not: the
`libm` crate differs on 110 of the 7,136 reference arguments, and a
correctly rounded reduction of large arguments differs because fdlibm's
reduced value is not always correctly rounded. `tests/data/v8_trig_*.txt` holds both
reference sets, and `RendererConfig::reference_platform` selects the one to
match (default x64, which is also what WebAssembly engines compute). The
committed fixtures are generated with x64 Node; on arm64 hosts run
`fixtures.mjs` with an x64 Node (e.g. under Rosetta). `diff.sh` passes the
oracle Node's architecture to the Rust side, so live comparisons are exact
on either architecture.

Number formatting follows ECMAScript `Number::toString`, including its
tie-breaking rule. When a double lies exactly halfway between two shortest
decimal representations, it picks the even digit, which Rust's shortest
formatter does not (`tests/data/v8_numbers.txt`).

## Upstream issues retained

The SVG output keeps these upstream bytes; the drawing view
(`Symbol::drawing`) describes what SVG renderers draw from them.

- **Invalid suspect frame colour.** milsymbol.js's `FrameColor` colour mode
  sets `Suspect: "rbg(255, 188, 1)"` (`src/colormodes.js`). The frame of a
  2525E/APP-6E mine-warfare symbol (symbol set 36) with standard identity
  suspect is stroked with it. No CSS reader accepts the value, so browsers
  ignore the stroke and the frame outline is not drawn. In the corpus this
  affects 318 symbols.
- **Colours with control characters.** A colour option is written after
  removing JavaScript whitespace only, so a value such as `"\u0085red"`
  reaches the SVG unchanged. CSS strips only space, tab, LF, CR and FF, so
  browsers ignore it.

## Known differences

These are deliberate and do not occur in the corpus. The `known` oracle
suite and native regression tests check the observable differences.

- **Cross-render cache pollution** is not reproduced (see determinism policy).
- **Opt-in icon font override.** `iconTextUsesFontFamily: true` uses
  `fontfamily` for built-in icon text, including sector modifiers. Upstream
  keeps template fonts for these nodes. The option defaults to `false` and
  appears in canonical options only when enabled. Custom text nodes returned
  by icon extensions keep their own fonts. Font metrics and icon bounds are
  not recalculated, so wider fonts can extend beyond the template bounds.
- **JavaScript exceptions become typed errors.** Inputs on which upstream
  throws (an unknown `colorMode`, `undefined` instructions reaching
  `ms.outline` or `_scale`) return `RenderError` instead of rendering.
- **Loosely typed options.** Upstream accepts any JavaScript value for any
  option; this port accepts the types upstream documents. `SymbolOptions::set`
  rejects unknown names and wrongly typed or unsupported values (e.g.
  `standard: "APP-6"`, which upstream silently treats as 2525) with
  `OptionError`; custom text fields go through `set_text`.
- **Finite numbers only.** A NaN or infinite numeric option (`size`,
  `strokeWidth`, `direction`, `speedLeader`, `hqStaffLength`, …) makes upstream
  write `NaN` into its SVG; rendering rejects it with
  `RenderError::InvalidOption`. `SymbolOptions::set` still accepts the number
  and rendering reports it.
- **Typed colours.** A style colour is `options::Color`, which cannot be the
  empty string; upstream's empty string ("not set") is `None`, and
  `SymbolOptions::set` maps `""` to `None`. Other text, including control
  characters, is kept and written escaped, as upstream does.
- **Bounded `stack`.** Upstream's `for (i = stack; i >= 1; i--)` never ends
  for non-finite or huge values (`i - 1 == i`). Rendering rejects a `stack`
  that is not finite or exceeds 1000 with `RenderError::InvalidOption`.
- **Extension parts override built-in parts everywhere**, as in upstream,
  except for the in-place `_scale(…, true)` mutations some upstream mappings
  apply to built-in parts: those are not re-applied to a replacement part.
- **Custom label defaults.** Omitted coordinates, font size and anchor in
  registered label overrides use SVG defaults for both layout and output.
  Built-in label data retains upstream's missing-property behaviour.
- **Colour-slot truthiness is taken per slot, not per affiliation.** A custom
  colour object with an empty or `false` entry for only some affiliations can
  select a different icon variant than upstream would for those
  affiliations. Built-in colour modes are uniform, so this cannot happen with
  them.
- **`sanitizeId`** (only used by the extension-only `clip` instruction)
  replaces a non-BMP character with one `_` where upstream writes two.
- **Clip-path ids are unique.** Upstream numbers generated ids without
  checking requested ones and keeps repeated requested ids, so one document
  can define the same id twice. Here generated ids skip requested ones and a
  repeated requested id gets `-1`, `-2`, … appended; `SvgOptions::id_prefix`
  prefixes every id. Only the extension-only `clip` instruction and
  extension styles with `clip_path` create clip paths.
- **Lone UTF-16 surrogates in metadata.** Upstream reads SIDC fields as
  UTF-16 substrings, so a SIDC containing a non-BMP character (e.g. an
  emoji) can put half a surrogate pair into `_modifier1`, `_modifier2` or
  `functionid`, which `JSON.stringify` writes as `\udXXX`. Rust strings
  cannot hold a lone surrogate; each half becomes U+FFFD. The SVG is
  unaffected (`tests/data/unicode_oracle.txt`).
- **JavaScript object keys as option names.** Upstream assigns options onto
  a plain object: a `__proto__` key is swallowed by the prototype setter,
  and a `hasOwnProperty` key shadows the method upstream later calls, so it
  throws. Here every name set with `SymbolOptions::set_text` is an ordinary
  text field, kept in `options` and rendered normally.
