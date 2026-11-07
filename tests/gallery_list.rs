//! Symbols shown in README.md. Shared by `examples/readme_images.rs`, which
//! writes `docs/images/*.svg`, and `tests/readme_images.rs`, which checks the
//! committed images still match the renderer.
#![allow(dead_code)]

use milsymbol::options::SymbolOptions;
use milsymbol::{Renderer, Standard, Symbol};

/// One gallery image: file stem, SIDC, standard and `(option, value)` pairs.
pub(crate) struct Item {
    /// File stem under `docs/images`.
    pub name: &'static str,
    /// The SIDC.
    pub sidc: &'static str,
    /// The renderer's standard.
    pub standard: Standard,
    /// Options by upstream name.
    pub options: &'static [(&'static str, Value)],
}

/// Option value in the gallery table.
#[derive(Clone, Copy)]
pub(crate) enum Value {
    /// String.
    S(&'static str),
    /// Number.
    N(f64),
    /// Boolean.
    B(bool),
}

use Value::{B, N, S};

const SIZE: (&str, Value) = ("size", N(40.0));

/// The gallery.
pub(crate) const ITEMS: &[Item] = &[
    // Upstream README hero: MIL-STD-2525C figure 13, as a 2525E SIDC.
    Item {
        name: "figure13",
        sidc: "130315003611010300000000000000",
        standard: Standard::Mil2525,
        options: &[
            ("size", N(35.0)),
            ("quantity", S("200")),
            ("staffComments", S("FOR REINFORCEMENTS")),
            ("additionalInformation", S("ADDED SUPPORT FOR JJ")),
            ("direction", N(750.0 * 360.0 / 6400.0)),
            ("type", S("MACHINE GUN")),
            ("dtg", S("30140000ZSEP97")),
            ("location", S("0900000.0E570306.0N")),
        ],
    },
    Item {
        name: "infantry-platoon",
        sidc: "130310001412110000000000000000",
        standard: Standard::Mil2525,
        options: &[("size", N(35.0))],
    },
    // Standard identities across dimensions.
    Item {
        name: "friend-air",
        sidc: "10030100001101000000",
        standard: Standard::Mil2525,
        options: &[SIZE],
    },
    Item {
        name: "hostile-air",
        sidc: "10060100001101000000",
        standard: Standard::Mil2525,
        options: &[SIZE],
    },
    Item {
        name: "neutral-air",
        sidc: "10040100001101000000",
        standard: Standard::Mil2525,
        options: &[SIZE],
    },
    Item {
        name: "unknown-air",
        sidc: "10010100001101000000",
        standard: Standard::Mil2525,
        options: &[SIZE],
    },
    Item {
        name: "friend-land",
        sidc: "10031000001211000000",
        standard: Standard::Mil2525,
        options: &[SIZE],
    },
    Item {
        name: "hostile-land",
        sidc: "10061000001211000000",
        standard: Standard::Mil2525,
        options: &[SIZE],
    },
    Item {
        name: "neutral-land",
        sidc: "10041000001211000000",
        standard: Standard::Mil2525,
        options: &[SIZE],
    },
    Item {
        name: "unknown-land",
        sidc: "10011000001211000000",
        standard: Standard::Mil2525,
        options: &[SIZE],
    },
    Item {
        name: "friend-sea",
        sidc: "10033000001201000000",
        standard: Standard::Mil2525,
        options: &[SIZE],
    },
    Item {
        name: "hostile-sea",
        sidc: "10063000001201000000",
        standard: Standard::Mil2525,
        options: &[SIZE],
    },
    Item {
        name: "neutral-sea",
        sidc: "10043000001201000000",
        standard: Standard::Mil2525,
        options: &[SIZE],
    },
    Item {
        name: "unknown-sea",
        sidc: "10013000001201000000",
        standard: Standard::Mil2525,
        options: &[SIZE],
    },
    Item {
        name: "friend-subsurface",
        sidc: "10033500001101000000",
        standard: Standard::Mil2525,
        options: &[SIZE],
    },
    Item {
        name: "hostile-subsurface",
        sidc: "10063500001101000000",
        standard: Standard::Mil2525,
        options: &[SIZE],
    },
    Item {
        name: "neutral-subsurface",
        sidc: "10043500001101000000",
        standard: Standard::Mil2525,
        options: &[SIZE],
    },
    Item {
        name: "unknown-subsurface",
        sidc: "10013500001101000000",
        standard: Standard::Mil2525,
        options: &[SIZE],
    },
    // Amplifiers and modifiers.
    Item {
        name: "hq-brigade",
        sidc: "10031002181211000000",
        standard: Standard::Mil2525,
        options: &[SIZE],
    },
    Item {
        name: "task-force-company",
        sidc: "10031004151211000000",
        standard: Standard::Mil2525,
        options: &[SIZE],
    },
    Item {
        name: "feint-dummy",
        sidc: "10031001151205000000",
        standard: Standard::Mil2525,
        options: &[SIZE],
    },
    Item {
        name: "tracked-equipment",
        sidc: "10031500331101000000",
        standard: Standard::Mil2525,
        options: &[SIZE],
    },
    Item {
        name: "planned",
        sidc: "10031010001211000000",
        standard: Standard::Mil2525,
        options: &[SIZE],
    },
    Item {
        name: "damaged",
        sidc: "10031030001211000000",
        standard: Standard::Mil2525,
        options: &[SIZE],
    },
    Item {
        name: "exercise-joker",
        sidc: "10151000001211000000",
        standard: Standard::Mil2525,
        options: &[SIZE],
    },
    Item {
        name: "installation",
        sidc: "10032000001101000000",
        standard: Standard::Mil2525,
        options: &[SIZE],
    },
    Item {
        name: "speed-leader",
        sidc: "10030100001101000000",
        standard: Standard::Mil2525,
        options: &[SIZE, ("direction", N(60.0)), ("speedLeader", N(60.0))],
    },
    Item {
        name: "engagement-bar",
        sidc: "10061000001211000000",
        standard: Standard::Mil2525,
        options: &[
            SIZE,
            ("engagementBar", S("2:10")),
            ("engagementType", S("TARGET")),
        ],
    },
    // Styles.
    Item {
        name: "style-unfilled",
        sidc: "10031000001211000000",
        standard: Standard::Mil2525,
        options: &[SIZE, ("fill", B(false))],
    },
    Item {
        name: "style-dark",
        sidc: "10031000001211000000",
        standard: Standard::Mil2525,
        options: &[SIZE, ("colorMode", S("Dark"))],
    },
    Item {
        name: "style-mono",
        sidc: "10061000001211000000",
        standard: Standard::Mil2525,
        options: &[SIZE, ("monoColor", S("rgb(20,60,160)"))],
    },
    Item {
        name: "style-outline",
        sidc: "10031000001211000000",
        standard: Standard::Mil2525,
        options: &[
            SIZE,
            ("outlineWidth", N(5.0)),
            ("outlineColor", S("rgb(40,40,40)")),
        ],
    },
    Item {
        name: "style-unframed",
        sidc: "10031000001211000000",
        standard: Standard::Mil2525,
        options: &[SIZE, ("frame", B(false))],
    },
    Item {
        name: "style-info-background",
        sidc: "10031000001211000000",
        standard: Standard::Mil2525,
        options: &[
            SIZE,
            ("uniqueDesignation", S("A/1-66")),
            ("higherFormation", S("2 BDE")),
            ("infoBackground", S("rgb(255,240,170)")),
        ],
    },
    // APP-6(E) Version 2.
    Item {
        name: "app6e2-infantry",
        sidc: "16031000161211020000",
        standard: Standard::App6EVersion2,
        options: &[SIZE],
    },
    Item {
        name: "app6e2-armour",
        sidc: "16061000151205000000",
        standard: Standard::App6EVersion2,
        options: &[SIZE],
    },
    Item {
        name: "app6e2-uas",
        sidc: "16030100001103000000",
        standard: Standard::App6EVersion2,
        options: &[SIZE],
    },
    // Letter SIDCs.
    Item {
        name: "letter-2525c",
        sidc: "SFGPUCFRM---",
        standard: Standard::Mil2525,
        options: &[SIZE],
    },
    Item {
        name: "letter-sea-mine",
        sidc: "SHUPWMGX----",
        standard: Standard::Mil2525,
        options: &[SIZE],
    },
];

/// Renders a gallery item.
pub(crate) fn render(item: &Item) -> Result<Symbol, Box<dyn std::error::Error>> {
    let renderer = Renderer::builder().standard(item.standard).build();
    let mut o = SymbolOptions::default();
    for (k, v) in item.options {
        match *v {
            S(s) => o.set(k, s)?,
            N(n) => o.set(k, n)?,
            B(b) => o.set(k, b)?,
        };
    }
    Ok(renderer.render(item.sidc, o)?)
}

/// The README background: a white rounded rectangle covering the viewBox.
pub(crate) fn background(svg: &str) -> Option<String> {
    let vb = svg.split("viewBox=\"").nth(1)?.split('"').next()?;
    let mut v = vb.split(' ');
    let (x, y, w, h) = (v.next()?, v.next()?, v.next()?, v.next()?);
    Some(format!(
        "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"8\" fill=\"#ffffff\" />"
    ))
}

/// `svg` with [`background`] inserted as the first child of `<svg>`.
pub(crate) fn with_background(svg: &str) -> Option<String> {
    let (start_tag, rest) = svg.split_once('>')?;
    Some(format!("{start_tag}>{}{rest}", background(svg)?))
}
