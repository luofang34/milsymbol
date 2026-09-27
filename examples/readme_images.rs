//! Renders the README gallery with this crate: `cargo run --example
//! readme_images [output-dir]` (default `docs/images`).
//!
//! Each image is exactly `Symbol::to_svg()` plus one white background
//! rectangle, so the symbols stay legible on dark pages. `--cases` prints the
//! gallery as oracle cases for `tools/oracle/render.mjs`.

#[path = "../tests/gallery_list.rs"]
mod gallery_list;

use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arg = std::env::args().nth(1);
    if arg.as_deref() == Some("--cases") {
        return print_cases();
    }
    let dir = arg.map_or_else(|| PathBuf::from("docs/images"), PathBuf::from);
    std::fs::create_dir_all(&dir)?;
    for item in gallery_list::ITEMS {
        let symbol = gallery_list::render(item)?;
        let svg = gallery_list::with_background(&symbol.to_svg()).ok_or("svg without viewBox")?;
        std::fs::write(dir.join(format!("{}.svg", item.name)), svg)?;
        println!(
            "{:24} {:32} valid={}",
            item.name,
            item.sidc,
            symbol.is_valid()
        );
    }
    Ok(())
}

/// Prints the gallery as oracle cases (`tools/oracle/render.mjs` input).
fn print_cases() -> Result<(), Box<dyn std::error::Error>> {
    use gallery_list::Value;
    for item in gallery_list::ITEMS {
        let mut opts = serde_json::Map::new();
        for (k, v) in item.options {
            let v = match *v {
                Value::S(s) => serde_json::json!(s),
                Value::N(n) => serde_json::json!(n),
                Value::B(b) => serde_json::json!(b),
            };
            opts.insert((*k).into(), v);
        }
        let mut case = serde_json::Map::new();
        case.insert("sidc".into(), serde_json::json!(item.sidc));
        case.insert("options".into(), serde_json::Value::Object(opts));
        if item.app6 {
            case.insert("cfg".into(), serde_json::json!({ "standard": "APP6" }));
        }
        println!("{}", serde_json::Value::Object(case));
    }
    Ok(())
}
