//! Arbitrary SIDC text, numbers and label text: rendering returns a typed
//! error or a symbol whose SVG, drawing and canonical JSON all build.

#![no_main]

use libfuzzer_sys::fuzz_target;
use milsymbol::Renderer;
use milsymbol::options::TextField;

fuzz_target!(|input: (&str, f64, f64, f64, &str)| {
    let (sidc, size, direction, speed, label) = input;
    let result = Renderer::default()
        .symbol(sidc)
        .size(size)
        .direction(direction)
        .speed_leader(speed)
        .text(TextField::UniqueDesignation, label)
        .render();
    if let Ok(symbol) = result {
        let svg = symbol.to_svg();
        assert!(svg.starts_with("<svg") && svg.ends_with("</svg>"));
        symbol.drawing();
        milsymbol::compat::canonical_json_string(&symbol);
    }
});
