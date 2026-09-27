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

Everything under `src/generated/` is emitted by `tools/codegen` and must not
be edited by hand. Regenerate with:

```sh
tools/codegen/regenerate.sh      # needs git and Node; fetches the pinned upstream
```

| File | Source (upstream) | Tool |
|---|---|---|
| `pool.rs`, `tables.rs`, `vars.rs` | `src/iconparts/*.js`, `src/numbersidc/sidc/*.js`, `src/lettersidc/sidc/*.js` | `extract.mjs` → `emit.mjs` |
| `misc.rs` (labels) | `src/lettersidc/labels/*.js`, `src/numbersidc/labels/*.js` | `extract-misc.mjs` → `emit.mjs` |
| `misc.rs` (geometries) | `src/ms/symbolgeometries.js` | `extract-misc.mjs` → `emit.mjs` |
| `misc.rs` (colour modes) | `src/colormodes.js` | `extract-misc.mjs` → `emit.mjs` |
| `misc.rs` (character widths) | `src/symbolfunctions/string-width.js` | `extract-misc.mjs` → `emit.mjs` |

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
| `base` | 130,978 | every valid numeric entity × 7 standard identities, every letter pattern × 6 affiliations (+ echelons); a strict superset of milsymbol-py's 109,216 |
| `modifiers` | 89,046 | versions, contexts, identities, status, HQ/TF/dummy, echelon/mobility codes, every modifier 1/2 code, frame shapes, letter-SIDC fields |
| `options` | 6,727 | every text field, every style option alone and in 4,000 random combinations, XSS strings, non-ASCII text |
| `fuzz` | 20,000 | random numeric and letter SIDCs, some with options |
| `config` | 123 | renderer-level standard, dash arrays, HQ staff length |
| `invalid` | 44 | empty/short/long/malformed SIDCs, unknown colour mode |

All 247,918 cases match byte-for-byte (SVG) and exactly (semantic record).
Regenerate fixtures with `node tools/oracle/fixtures.mjs <suite>…`; run a live
comparison with `tools/oracle/diff.sh <suite>`.

## Known differences

These are deliberate and do not occur in the corpus:

- **Cross-render cache pollution** is not reproduced (see determinism policy).
- **JavaScript exceptions become typed errors.** Inputs on which upstream
  throws (an unknown `colorMode`, `undefined` instructions reaching
  `ms.outline` or `_scale`) return `RenderError` instead of rendering.
- **Loosely typed options.** Upstream accepts any JavaScript value for any
  option; this port accepts the types upstream documents (`SymbolOptions::set`
  rejects others with `OptionError`).
- **Colour-slot truthiness is taken per slot, not per affiliation.** A custom
  colour object with an empty or `false` entry for only some affiliations can
  select a different icon variant than upstream would for those
  affiliations. Built-in colour modes are uniform, so this cannot happen with
  them.
- **`sanitizeId`** (only used by the extension-only `clip` instruction)
  replaces a non-BMP character with one `_` where upstream writes two.
