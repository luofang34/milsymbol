//! A standards gallery: `cargo run --example standards > standards.svg`.

use milsymbol::options::TextField;
use milsymbol::{Renderer, Standard, SvgOptions};
use std::fmt::Write as _;
use std::io::Write as _;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ANCHOR: standards_input
    let renderer = Renderer::builder().standard(Standard::Mil2525).build();
    let tropopause = renderer
        .symbol("15034500001101020000")
        .text(TextField::AltitudeDepth, "320")
        .strict()
        .render()?;
    // ANCHOR_END: standards_input
    let cases = [
        (
            "MIL combined combat",
            "15031000001221000000",
            Standard::Mil2525,
        ),
        (
            "APP combined combat",
            "16031000001221000000",
            Standard::App6,
        ),
        (
            "Restricted target",
            "15331000001211000000",
            Standard::Mil2525,
        ),
        ("No-strike exercise", "16631000001211000000", Standard::App6),
        ("Cultural property", "16032500003601000000", Standard::App6),
        (
            "Enhanced protection",
            "16032500003603000000",
            Standard::App6,
        ),
        ("Password query", "16032500004001000000", Standard::App6),
        ("Harbour entrance O", "16032500002129020000", Standard::App6),
        ("Individual", "16034000001800000000", Standard::App6),
        ("Firewall", "16036400001301000000", Standard::App6),
        ("Tented camp", "16032000001109030000", Standard::App6),
    ];
    let mut page = String::from(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"900\" height=\"700\" viewBox=\"0 0 900 700\">\
         <rect width=\"900\" height=\"700\" fill=\"white\"/>",
    );
    for (i, (name, sidc, standard)) in cases.iter().enumerate() {
        let symbol = renderer
            .symbol(*sidc)
            .standard(*standard)
            .strict()
            .render()?;
        cell(&mut page, i, name, &symbol)?;
    }
    cell(
        &mut page,
        cases.len(),
        "Tropopause: height 320",
        &tropopause,
    )?;
    page.push_str("</svg>");
    std::io::stdout().write_all(page.as_bytes())?;
    Ok(())
}

fn cell(
    page: &mut String,
    index: usize,
    label: &str,
    symbol: &milsymbol::Symbol,
) -> std::fmt::Result {
    let x = (index % 3) as f64 * 300.0;
    let y = (index / 3) as f64 * 175.0;
    let size = symbol.size();
    let scale = (240.0 / size.width).min(120.0 / size.height).min(1.0);
    let left = x + (300.0 - size.width * scale) / 2.0;
    let top = y + (140.0 - size.height * scale) / 2.0;
    write!(
        page,
        "<g transform=\"translate({left},{top}) scale({scale})\">"
    )?;
    symbol.write_svg_with(
        page,
        &SvgOptions::default().with_id_prefix(format!("s{index}-")),
    );
    write!(
        page,
        "</g><text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"sans-serif\" font-size=\"14\">{label}</text>",
        x + 150.0,
        y + 158.0
    )
}
