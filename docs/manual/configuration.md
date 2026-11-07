# Configuration and serialization

Renderer-wide settings replace the global configuration used by milsymbol.js.
Choose a standard, colour modes, dash arrays, staff length, and reference
platform when constructing a renderer.

```rust
# extern crate milsymbol;
use milsymbol::{Renderer, Standard, ReferencePlatform};
use milsymbol::config::DashArrays;

let renderer = Renderer::builder()
    .standard(Standard::App6)
    .reference_platform(ReferencePlatform::X64)
    .dash_arrays(DashArrays::new("4,4", "8,12", "8,8"))
    .hq_staff_length(100.0)
    .build();
let symbol = renderer.symbol("SFGPUCI-----").render()?;
assert!(!symbol.metadata().std2525);
# Ok::<(), milsymbol::RenderError>(())
```

`Renderer::new(RendererConfig)` uses a complete configuration with the built-in
pipeline. To combine it with extensions, use
`Renderer::builder().config(config).icons(extension).build()`.

`ReferencePlatform::X64` is the default. `Arm64` reproduces the last-bit
trigonometric results used by V8 on that architecture. This selects an upstream
reference; it does not detect or change the machine on which Rust is running.
Use the same setting across clients when byte-identical output matters.

## Options from configuration text

`SymbolOptions::set` accepts upstream option names and typed `OptionValue`s.
Strings, `f64`, and booleans convert directly. Unknown names and incorrect types
return `OptionError`; non-finite numeric values are rejected during rendering.

```rust
# extern crate milsymbol;
use milsymbol::{Renderer, options::SymbolOptions};

let mut options = SymbolOptions::default();
options.set("size", 40.0)?;
options.set("fill", false)?;
options.set("uniqueDesignation", "A/1-66")?;
assert!(options.set("unknownOption", true).is_err());
let symbol = Renderer::default().render("10031000001211000000", options)?;
assert!(!symbol.options().style.fill);
# Ok::<(), Box<dyn std::error::Error>>(())
```

Custom text fields use `set_text(TextField::custom(...), value)` rather than an
unknown `set` name. The Rust API deliberately avoids JavaScript's arbitrary
type coercions. For owned font names and other `ir::Str` values, pass a `String`
or static string; clone a borrowed `String` when an owned value is needed.

## Serde

Enable `serde` on `milsymbol` and add your chosen format crate, such as
`serde_json`, in the application. `SymbolOptions`, `Style`, and `RendererConfig`
support serialization and deserialization. Missing fields use defaults.
Metadata is serialize-only; drawing data also supports its documented serde
representations.

```rust
# extern crate milsymbol;
# extern crate serde_json;
use milsymbol::{Renderer, options::{SymbolOptions, TextField}};

let mut options = SymbolOptions::default();
options.style.size = 40.0;
options.set_text(TextField::UniqueDesignation, "1-66");
let json = serde_json::to_string(&options)?;
let decoded: SymbolOptions = serde_json::from_str(&json)?;
assert_eq!(decoded, options);

let defaults: SymbolOptions = serde_json::from_str("{}")?;
assert_eq!(defaults, SymbolOptions::default());
let symbol = Renderer::default().render("10031000001211000000", decoded)?;
assert!(symbol.sidc_validity().is_valid());
# Ok::<(), Box<dyn std::error::Error>>(())
```

The typed serde representation is distinct from milsymbol.js's flat option
object. For example, style belongs under `style`, and text is a map under `text`.
Use `SymbolOptions::set` to import individual upstream option names, and `compat`
for canonical oracle records. Do not assume one format can be deserialized as
the other without translation.

Extensions are executable Rust registrations and are not serialized in
`RendererConfig`. Restore them explicitly when constructing a renderer from
saved configuration. Store the application's revision and extension identity
alongside exported configuration when that matters for reproducibility.
