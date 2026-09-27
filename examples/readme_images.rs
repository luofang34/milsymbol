//! Writes the README gallery (`docs/images/*.svg`) with this crate.
//! Run from the repository root: `cargo run --example readme_images`.

#[path = "../tests/gallery_list.rs"]
mod gallery_list;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().nth(1).as_deref() == Some("--cases") {
        return print_cases();
    }
    std::fs::create_dir_all("docs/images")?;
    for item in gallery_list::ITEMS {
        let symbol = gallery_list::render(item);
        std::fs::write(format!("docs/images/{}.svg", item.name), symbol.to_svg())?;
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
        let case = serde_json::Value::Object(case);
        println!("{case}");
    }
    Ok(())
}
