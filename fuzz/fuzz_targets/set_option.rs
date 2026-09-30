//! Options applied by name: unknown names and wrong types are errors, and
//! whatever is accepted renders or fails with a typed error.

#![no_main]

use libfuzzer_sys::fuzz_target;
use milsymbol::Renderer;
use milsymbol::options::{OptionValue, SymbolOptions};

fuzz_target!(|input: (&str, u8, &str, f64)| {
    let (name, kind, text, number) = input;
    let value = match kind % 3 {
        0 => OptionValue::Str(text.into()),
        1 => OptionValue::Num(number),
        _ => OptionValue::Bool(number > 0.0),
    };
    let mut options = SymbolOptions::default();
    if options.set(name, value).is_ok() {
        Renderer::default()
            .render("10031000161211000000", options)
            .ok();
    }
});
