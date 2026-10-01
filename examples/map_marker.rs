//! Places symbols on a map-like page: each symbol's anchor lands on its map
//! position, whatever the symbol's shape. Run it and redirect stdout to an
//! `.svg` file. `PIXEL_RATIO` shows the high-density convention: render at
//! `size * ratio` and lay out in logical pixels by dividing by the ratio.

use milsymbol::options::TextField;
use milsymbol::sidc::Sidc;
use milsymbol::{Renderer, SvgOptions};
use std::fmt::Write as _;
use std::io::Write as _;

const SIZE: f64 = 40.0;
const PIXEL_RATIO: f64 = 2.0;

struct Marker {
    sidc: &'static str,
    at: (f64, f64),
    label: &'static str,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let markers = [
        Marker {
            sidc: "10031000001211000000",
            at: (120.0, 90.0),
            label: "infantry",
        },
        Marker {
            sidc: "10031002001211000000",
            at: (300.0, 160.0),
            label: "HQ, anchored at the staff foot",
        },
        Marker {
            sidc: "10030100001101000000",
            at: (200.0, 260.0),
            label: "air",
        },
    ];
    let renderer = Renderer::default();
    let mut page = String::from(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"480\" height=\"360\" viewBox=\"0 0 480 360\">\n\
         <rect width=\"480\" height=\"360\" fill=\"#e8eef2\"/>\n",
    );
    for (i, m) in markers.iter().enumerate() {
        let sidc = Sidc::parse(m.sidc)?;
        let symbol = renderer
            .symbol(&sidc)
            .size(SIZE * PIXEL_RATIO)
            .text(TextField::UniqueDesignation, format!("U{i}"))
            .render()?;
        // The anchor and size are in rendered pixels; the page is in logical ones.
        let anchor = symbol.anchor();
        let (left, top) = (
            m.at.0 - anchor.x / PIXEL_RATIO,
            m.at.1 - anchor.y / PIXEL_RATIO,
        );
        let mut svg = String::new();
        symbol.write_svg_with(
            &mut svg,
            &SvgOptions::default().with_id_prefix(format!("m{i}-")),
        );
        writeln!(
            page,
            "<g transform=\"translate({left} {top}) scale({})\">{svg}</g>\n\
             <circle cx=\"{}\" cy=\"{}\" r=\"2\" fill=\"red\"/>\n\
             <text x=\"{}\" y=\"{}\" font-size=\"9\" font-family=\"Arial\" text-anchor=\"middle\">{}</text>",
            1.0 / PIXEL_RATIO,
            m.at.0,
            m.at.1,
            m.at.0,
            m.at.1 + 36.0,
            m.label,
        )?;
    }
    page.push_str("</svg>\n");
    std::io::stdout().write_all(page.as_bytes())?;
    Ok(())
}
