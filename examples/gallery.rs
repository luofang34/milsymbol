//! Renders representative symbols to SVG files without any JavaScript.
//!
//! `cargo run --example gallery -- [output-dir]` (default `target/gallery`).

use milsymbol::options::{SymbolOptions, field};
use milsymbol::{Renderer, Standard};
use std::path::PathBuf;

const SYMBOLS: &[(&str, &str)] = &[
    ("friendly-infantry", "10031000001211000000"),
    ("hostile-armour-battalion", "10061000161205000000"),
    ("neutral-sea-surface", "10041000001201000000"),
    ("unknown-air-fixed-wing", "10010100001101000000"),
    ("friendly-hq-brigade", "10031020181211000000"),
    ("suspect-2525e-ground", "13051000001211000000"),
    ("mine-warfare", "10033600001101000000"),
    ("letter-friendly-infantry", "SFGPUCI----D"),
    ("letter-hostile-tank", "SHGPEVAT----"),
    ("letter-exercise-pending", "SGGAUCI---E-"),
    ("control-measure-point", "10032500001301000000"),
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::args()
        .nth(1)
        .map_or_else(|| PathBuf::from("target/gallery"), PathBuf::from);
    std::fs::create_dir_all(&dir)?;
    let renderer = Renderer::default();
    for (name, sidc) in SYMBOLS {
        let symbol = renderer.symbol(sidc).size(60.0).render()?;
        std::fs::write(dir.join(format!("{name}.svg")), symbol.to_svg())?;
        println!(
            "{name:28} {sidc:22} valid={} size={:?}",
            symbol.is_valid(),
            symbol.size()
        );
    }
    let mut o = SymbolOptions::default();
    o.style.size = 60.0;
    o.direction = Some(60.0);
    o.set_text(field::UNIQUE_DESIGNATION, "A/1-66")
        .set_text(field::HIGHER_FORMATION, "2 BDE")
        .set_text(field::STAFF_COMMENTS, "Reinforced")
        .set("outlineWidth", 3.0)?
        .set("infoBackground", "rgba(255,255,255,0.7)")?;
    let annotated = renderer.render("10031002161211000000", o)?;
    std::fs::write(dir.join("annotated.svg"), annotated.to_svg())?;
    let app6 = Renderer::default().with_standard(Standard::App6);
    std::fs::write(
        dir.join("app6-infantry.svg"),
        app6.symbol("SFGPUCI-----").size(60.0).render()?.to_svg(),
    )?;
    println!("wrote {}", dir.display());
    Ok(())
}
