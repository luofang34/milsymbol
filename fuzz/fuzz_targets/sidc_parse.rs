//! Any text parses or is rejected without panicking, and a SIDC that parses
//! renders strictly or is rejected as unsupported.

#![no_main]

use libfuzzer_sys::fuzz_target;
use milsymbol::Renderer;
use milsymbol::sidc::Sidc;

fuzz_target!(|text: &str| {
    if Sidc::parse(text).is_ok() {
        Renderer::default().symbol(text).strict().render().ok();
    }
});
