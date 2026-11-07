# SIDCs and validation

A SIDC is a string, not a number. Preserve leading zeroes and the exact field
widths. Numeric and legacy letter forms use different layouts.

## Numeric fields

Strict parsing accepts 20 or 30 characters. Positions below are one-based.

| Positions | Meaning | Example |
|---|---|---|
| 1–2 | Version | `10` |
| 3 | Context | `0`: reality |
| 4 | Standard identity | `3`: friend |
| 5–6 | Symbol set | `10`: land unit |
| 7 | Status | `0`: present |
| 8 | HQ, task force, feint/dummy flags | `0`: none |
| 9–10 | Echelon, mobility, or leadership amplifier | `14`: platoon |
| 11–16 | Entity, type, subtype | `121100`: infantry |
| 17–18 | Sector 1 modifier | `00`: none |
| 19–20 | Sector 2 modifier | `00`: none |
| 21–22 | Modifier-set selectors in the extended form | `00`: symbol set's own modifiers |
| 23–30 | Edition-dependent extension fields | Interpret through the selected standard |

For example, `10031000141211000000` separates as
`10 | 0 | 3 | 10 | 0 | 0 | 14 | 121100 | 00 | 00`.
Do not concatenate extended fields from a different edition: some are reserved
or interpreted differently. The parser's accessors expose fields without
requiring application code to slice strings.

Legacy letter SIDCs use scheme, identity, dimension, status, function, modifier,
country, and order-of-battle characters. The supported parser accepts 10–15
characters, uppercases letters, and validates the defined header fields.
Rendering also accepts shorter and malformed input for upstream compatibility.

## Parsing and support are separate checks

`Sidc::parse` validates the supported syntax and header codes. It does not prove
that an entity or modifier has an icon. `Renderer::check_sidc` also checks the
renderer and its extensions. `.strict()` applies the same validation when
rendering your actual options.

```rust
# extern crate milsymbol;
use milsymbol::{Renderer, sidc::{Sidc, SidcCheckError}};

let unknown_entity = "10031000009999990000";
assert!(Sidc::parse(unknown_entity).is_ok());
let renderer = Renderer::default();
assert!(matches!(
    renderer.check_sidc(unknown_entity),
    Err(SidcCheckError::Unsupported { .. })
));
assert!(renderer.symbol(unknown_entity).strict().render().is_err());

let fallback = renderer.symbol(unknown_entity).render()?;
assert!(!fallback.sidc_validity().is_valid());
assert!(!fallback.sidc_validity().issues.is_empty());
# Ok::<(), milsymbol::RenderError>(())
```

Lenient rendering is useful for a display that must show a fallback symbol.
Strict rendering is useful at an import boundary that must reject unsupported
input. Choose explicitly; an `Ok(Symbol)` from lenient rendering is not a validity
verdict. Turning off icons or information fields does not turn an unsupported
SIDC into a supported one.

## Edit typed fields

```rust
# extern crate milsymbol;
use milsymbol::domain::{Affiliation, Context, Status};
use milsymbol::sidc::Sidc;

let original = Sidc::parse("10031000001211000000")?;
let changed = original
    .with_affiliation(Affiliation::Hostile)
    .with_status(Status::Damaged);
assert_eq!(changed.as_str(), "10061030001211000000");

let exercise = original.with_context(Context::Exercise)?;
assert_eq!(exercise.as_str(), "10131000001211000000");
# Ok::<(), Box<dyn std::error::Error>>(())
```

`with_standard_identity` sets the exact identity and can fail if the scheme or
context cannot represent it. `with_affiliation` selects the identity appropriate
to the context. Exercise hostile and suspect roles become Faker and Joker;
their drawn affiliation can differ from their base role. Letter SIDCs cannot
represent simulation context.

## Build a symbol picker

`catalog::number_symbol_sets`, `number_entities`, `number_modifier1`,
`number_modifier2`, and `letter_icons` enumerate built-in table keys. Use the
selected renderer to validate candidates, because availability depends on
standard, edition, and extensions. Extension-defined keys are not enumerated
by the built-in catalogue.

```rust
# extern crate milsymbol;
use milsymbol::{Renderer, catalog};

let renderer = Renderer::default();
let supported: Vec<String> = catalog::number_entities("10")
    .map(|entity| format!("1003100000{entity}0000"))
    .filter(|sidc| renderer.check_sidc(sidc).is_ok())
    .collect();
assert!(supported.iter().any(|s| s == "10031000001211000000"));
```

See [compatibility](compatibility.md) for revision selection and its limits.
