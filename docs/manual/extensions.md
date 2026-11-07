# Renderer extensions

Extensions belong to a renderer, so applications can use different registries
concurrently. Built renderers are immutable; clone them to share their
configuration. Callbacks must return the same result for the same context and
must not depend on mutable global rendering state.

## Replace a built-in icon part

`IconExtension::icon_part` receives the part name, resolved symbol context, and a
lookup for earlier registrations and built-ins. Return `None` for keys your
extension does not handle.

```rust
# extern crate milsymbol;
use milsymbol::{IconExtension, IconPartContext, PartLookup, Renderer};
use milsymbol::ir::{Node, Paint};

struct RoundInfantry;

impl IconExtension for RoundInfantry {
    fn icon_part(&self, _: &IconPartContext<'_>, name: &str,
                 _: &dyn PartLookup) -> Option<Node> {
        (name == "GR.IC.FF.INFANTRY").then(|| {
            let mut node = Node::circle(100.0, 100.0, 25.0);
            if let Some(style) = node.style_mut() {
                style.fill = Some(Paint::None);
                style.stroke = Some(Paint::color("black"));
            }
            node
        })
    }
}

let renderer = Renderer::builder().icons(RoundInfantry).build();
let symbol = renderer.symbol("10031000001211000000").strict().render()?;
assert!(symbol.to_svg().contains("<circle"));
# Ok::<(), milsymbol::RenderError>(())
```

`catalog::icon_parts()` lists available built-in part names. Later registrations
take precedence over earlier ones. An `icon_part` callback sees earlier
definitions through `PartLookup`; an `icon` callback resolves parts with all
extensions applied.

## Add an entity

Use `IconKey::Entity` for numeric entity keys and `IconKey::Letter` for generic
letter SIDCs. Modifier keys identify their sector and can contain the extended
three-character modifier-set code.

```rust
# extern crate milsymbol;
use milsymbol::{IconExtension, IconKey, IconPartContext, PartLookup, Renderer};
use milsymbol::ir::Node;

struct CustomUnit;

impl IconExtension for CustomUnit {
    fn icon(&self, _: &IconPartContext<'_>, key: IconKey<'_>,
            parts: &dyn PartLookup) -> Option<Node> {
        match key {
            IconKey::Entity { symbol_set: "10", entity: "999900" } => {
                parts.part("GR.IC.FF.INFANTRY")
            }
            _ => None,
        }
    }
}

let renderer = Renderer::builder().icons(CustomUnit).build();
assert!(renderer.check_sidc("10031000009999000000").is_ok());
assert!(Renderer::default().check_sidc("10031000009999000000").is_err());
```

Use custom codes consistently within your application's interchange contract;
registering an icon does not assign an official standards meaning to its code.
`icon_bbox` supplies bounds for entity or letter icons extending beyond the
default octagon. Returning a missing node leaves the icon unsupported, including
when the symbol's icon visibility is disabled.

## Add a composition stage

`SymbolPart::draw` receives the current symbol state and returns instructions
to draw behind (`pre`) or in front (`post`) of the accumulated symbol, plus bounds
to merge. Propagate failures through `PartError`; the resulting `RenderError::Part`
retains the error source.

```rust
# extern crate milsymbol;
use milsymbol::{PartError, PartOutput, PartialBBox, Renderer, SymbolPart, SymbolState};
use milsymbol::ir::Node;

struct FrameCentreDot;

impl SymbolPart for FrameCentreDot {
    fn draw(&self, _: &SymbolState<'_>) -> Result<PartOutput, PartError> {
        Ok(PartOutput::new(
            vec![], vec![Node::circle(100.0, 100.0, 4.0)], PartialBBox::default(),
        ))
    }
}

let renderer = Renderer::builder().symbol_part(FrameCentreDot).build();
assert!(renderer.symbol("10031000001211000000").render()?.to_svg().contains("<circle"));
# Ok::<(), milsymbol::RenderError>(())
```

Bounds are part of the extension contract. Supply actual bounds when drawing
outside existing geometry, or the output can clip your contribution. The frame
centre is not the placement anchor of a headquarters symbol.

`RendererBuilder::pipeline` replaces the chosen built-in stages. Later
`symbol_part` calls append custom stages. Omitting `TextFields`, `Modifier`, or
other stages deliberately changes rendering behaviour; start with the default
pipeline unless your application needs a restricted composition.

## Label overrides

Implement `number_labels` or `letter_labels` to register `LabelField` collections.
Each `LabelField` names an option such as `uniqueDesignation` or a custom field,
and supplies one or more `Label` placements. `Label::at(x, y)` starts a placement;
set font size, anchor, and other attributes as needed. Omitted values receive
the library's label defaults.

Numeric label overrides apply to control measures and are keyed by six-character
entity code. Other numeric sets use the standard information-field layout.
Letter overrides use generic SIDCs. These registration callbacks run when the
extension is added, rather than building every label definition for each symbol.

Prefer typed geometry and text. Raw SVG is a trusted extension escape hatch and
is omitted by the flat drawing view; it limits portability to non-SVG backends.
