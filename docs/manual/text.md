# Text and fonts

Text amplifiers use standard placements around a symbol. Set them through
`TextField`, or through the corresponding milsymbol.js option names when reading
configuration. Empty or unset text generally contributes no visible label.
Layout depends on the symbol set, edition, and registered label overrides.

```rust
# extern crate milsymbol;
use milsymbol::{Renderer, options::TextField};

let symbol = Renderer::default()
    .symbol("10031000161211000000")
    .text(TextField::UniqueDesignation, "1-66")
    .text(TextField::HigherFormation, "2 BDE")
    .text(TextField::Dtg, "071200Z NOV")
    .font("Arial")
    .render()?;
assert_eq!(symbol.options().text(&TextField::UniqueDesignation), "1-66");
# Ok::<(), milsymbol::RenderError>(())
```

## Field names

| `TextField` | Configuration name | Purpose |
|---|---|---|
| `Quantity` | `quantity` | Quantity (C) |
| `ReinforcedReduced` | `reinforcedReduced` | Reinforced/reduced indicator (F) |
| `StaffComments` | `staffComments` | Staff comments (G) |
| `AdditionalInformation` | `additionalInformation` | Additional information (H) |
| `EvaluationRating` | `evaluationRating` | Evaluation (J) |
| `CombatEffectiveness` | `combatEffectiveness` | Effectiveness (K) |
| `SignatureEquipment` | `signatureEquipment` | Signature equipment (L) |
| `HigherFormation` | `higherFormation` | Parent formation (M) |
| `Hostile` | `hostile` | Enemy indicator (N) |
| `IffSif` | `iffSif` | IFF/SIF (P) |
| `Sigint` | `sigint` | Signals intelligence (R2) |
| `UniqueDesignation` | `uniqueDesignation` | Unit or object designation (T) |
| `Type` | `type` | Type (V) |
| `Dtg` | `dtg` | Date-time group (W) |
| `AltitudeDepth` | `altitudeDepth` | Altitude/depth (X) |
| `Location` | `location` | Location (Y) |
| `Speed` | `speed` | Speed label (Z) |
| `SpecialHeadquarters` | `specialHeadquarters` | Special HQ (AA) |
| `Country` | `country` | Geographical entity (APP-6E AS) |
| `PlatformType` | `platformType` | Platform (AD) |
| `EquipmentTeardownTime` | `equipmentTeardownTime` | Teardown time (AE) |
| `CommonIdentifier` | `commonIdentifier` | Common identifier (AF) |
| `AuxiliaryEquipmentIndicator` | `auxiliaryEquipmentIndicator` | Auxiliary equipment (AG) |
| `HeadquartersElement` | `headquartersElement` | Headquarters element (APP-6E AW) |
| `InstallationComposition` | `installationComposition` | Installation composition (APP-6E AX) |
| `EngagementBar` | `engagementBar` | Engagement bar (AO) |
| `EngagementType` | `engagementType` | `TARGET`, `NON-TARGET`, or `EXPIRED` |
| `GuardedUnit` | `guardedUnit` | Guarded unit (AQ) |
| `SpecialDesignator` | `specialDesignator` | Special designator (AR) |

`Speed` is only a text label; it does not calculate a speed leader.
`Country` is text. The separate `country_flag` option reserves flag space and
does not download or embed a flag image. `full_frame_flag` controls that space
for applicable full-frame friendly ground symbols. `signature` participates in
upstream layout, with `"!"` reserving extra space.

`TextField::custom("dtg1")` supports fields used by label overrides. It does not
automatically create a new visible placement. Register the corresponding label
layout through an [extension](extensions.md).

## Font selection and measurement

`.font("Your Font")` sets the information-field font and enables its use in
built-in icon text. Setting only `style.font_family` preserves the built-in
icons' template fonts unless `icon_text_uses_font_family` is enabled. Custom
extension text retains the attributes supplied by the extension.

The library names fonts; it does not install, download, or shape them. SVG
viewers, PNG rasterizers, and custom backends must load suitable fonts. Font
fallback can change glyph appearance or cause text to disappear. The layout
follows the upstream metrics rather than measuring an arbitrary installed font.

`info_size` changes information-field text size. `info_background`,
`info_background_frame`, and information outlines improve legibility over a
map. `info_fields = false` hides those fields while retaining their option
values. Prefer native text shaping in a custom backend and preserve the supplied
anchor, weight, baseline, transform, and clipping information.

Text is escaped by the SVG writer. Pass ordinary unescaped text such as
`"A & B"`; do not pre-escape it into `"A &amp; B"`, which would be escaped again.
